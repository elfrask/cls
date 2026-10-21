use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use cls_core::config::ModuleManifest;
use cls_core::frontend::ast::Module as ClsModule;

pub fn execute(args: &[String]) -> i32 {
    let config = load_config();

    let entry = args
        .iter()
        .find(|a| !a.starts_with("-"))
        .map(|s| s.as_str())
        .or_else(|| {
            config.as_ref().and_then(|c| {
                let e = &c.entry;
                if !e.is_empty() && Path::new(e).exists() {
                    Some(e.as_str())
                } else {
                    None
                }
            })
        })
        .unwrap_or_else(|| {
            eprintln!("Uso: clx build <archivo> -o <salida>");
            eprintln!("  (o ejecuta desde un proyecto con cls.json que tenga 'entry')");
            std::process::exit(1);
        })
        .to_string();

    let out = args
        .iter()
        .position(|a| a == "-o" || a == "--out")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.as_str())
        .unwrap_or("dist/app.clsapp");

    let source = match fs::read_to_string(&entry) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error al leer '{}': {}", entry, e);
            return 1;
        }
    };

    let tokens = match cls_core::frontend::Lexer::new(&source).tokenize() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Error de sintaxis: {}", e);
            return 1;
        }
    };
    let mut module = match cls_core::frontend::Parser::new(tokens).parse() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Error de parseo: {}", e);
            return 1;
        }
    };

    let base_dir = Path::new(&entry)
        .parent()
        .unwrap_or(Path::new("."))
        .to_path_buf();
    let entry_root = cls_jit::native_resolve::module_root(&base_dir, &base_dir);
    let manifest = ModuleManifest::find_in_dir(&entry_root);
    if let Err(e) =
        cls_jit::native_resolve::resolve_module_natives(&mut module, &entry_root, manifest.as_ref())
    {
        eprintln!("Error resolviendo extensiones: {}", e);
        return 1;
    }

    let mut extra_paths: Vec<PathBuf> = Vec::new();
    for m in [manifest.as_ref(), config.as_ref()].into_iter().flatten() {
        for p in &m.path_modules {
            let pb = Path::new(p);
            let abs = if pb.is_absolute() {
                pb.to_path_buf()
            } else {
                entry_root.join(pb)
            };
            if !extra_paths.iter().any(|x| x == &abs) {
                extra_paths.push(abs);
            }
        }
    }
    cls_jit::resolve::set_extra_module_paths(extra_paths);

    let mut seen = HashSet::new();
    let mut imports: Vec<(String, String, ClsModule)> = Vec::new();
    if let Err(e) = cls_jit::resolve::load_import_modules(
        &module,
        &base_dir,
        &mut seen,
        &mut imports,
        manifest.as_ref(),
    ) {
        eprintln!("Error resolviendo imports: {}", e);
        return 1;
    }

    let native_root = Path::new(out)
        .parent()
        .unwrap_or(Path::new("."))
        .join("native-libs");
    let mut native_map: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    if let Err(e) = collect_native_assets(
        &module,
        &imports,
        &entry_root,
        manifest.as_ref(),
        &native_root,
        &mut native_map,
    ) {
        eprintln!("Error empaquetando librerias nativas: {}", e);
        return 1;
    }

    if let Some(parent) = Path::new(&out).parent() {
        fs::create_dir_all(parent).ok();
    }

    let file = match fs::File::create(&out) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Error al crear '{}': {}", out, e);
            return 1;
        }
    };
    let mut zip = zip::ZipWriter::new(file);
    let options: zip::write::FileOptions<()> =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    let name = config.as_ref().map(|c| c.name.as_str()).unwrap_or("app");
    let version = config
        .as_ref()
        .map(|c| c.version.as_str())
        .unwrap_or("1.0.0");
    let mut manifest_json = serde_json::json!({
        "name": name,
        "version": version,
        "entry": "source.clsx",
        "format": "source",
    });
    if !native_map.is_empty() {
        let mut libs = serde_json::Map::new();
        for (pkg, files) in &native_map {
            libs.insert(
                pkg.clone(),
                serde_json::json!(files.iter().cloned().collect::<Vec<_>>()),
            );
        }
        manifest_json["nativeLibs"] = serde_json::Value::Object(libs);
    }

    if let Err(e) = zip.start_file("manifest.json", options) {
        eprintln!("Error empaquetando '{}': {}", out, e);
        return 1;
    }
    if let Err(e) = zip.write_all(
        serde_json::to_string_pretty(&manifest_json)
            .unwrap()
            .as_bytes(),
    ) {
        eprintln!("Error empaquetando '{}': {}", out, e);
        return 1;
    }
    if let Err(e) = zip.start_file("source.clsx", options) {
        eprintln!("Error empaquetando '{}': {}", out, e);
        return 1;
    }
    if let Err(e) = zip.write_all(source.as_bytes()) {
        eprintln!("Error empaquetando '{}': {}", out, e);
        return 1;
    }
    if let Err(e) = zip.finish() {
        eprintln!("Error empaquetando '{}': {}", out, e);
        return 1;
    }

    println!("Empaquetado: {} ({})", out, name);
    if !native_map.is_empty() {
        println!("Librerias nativas: {}", native_root.display());
    }
    0
}

fn is_native_lib(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| matches!(e.to_ascii_lowercase().as_str(), "dll" | "so" | "dylib"))
        .unwrap_or(false)
}

fn safe_component(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' {
            out.push(c);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "app".to_string()
    } else {
        out
    }
}

fn package_id(root: &Path, fallback: Option<&ModuleManifest>) -> String {
    let m = ModuleManifest::find_in_dir(root).or_else(|| fallback.cloned());
    match m {
        Some(m) => format!("{}@{}", safe_component(&m.name), safe_component(&m.version)),
        None => "app@0.0.0".to_string(),
    }
}

fn collect_native_assets(
    module: &ClsModule,
    imports: &[(String, String, ClsModule)],
    entry_root: &Path,
    entry_manifest: Option<&ModuleManifest>,
    native_root: &Path,
    native_map: &mut BTreeMap<String, BTreeSet<String>>,
) -> Result<(), String> {
    let mut assets: Vec<PathBuf> = Vec::new();
    assets.extend(
        cls_jit::native_resolve::resolved_native_libraries(module)
            .into_iter()
            .map(PathBuf::from),
    );
    for (_, _, m) in imports {
        assets.extend(
            cls_jit::native_resolve::resolved_native_libraries(m)
                .into_iter()
                .map(PathBuf::from),
        );
    }

    for asset in assets {
        let src = Path::new(&asset);
        let parent = src.parent().unwrap_or(entry_root);
        let root = cls_jit::native_resolve::module_root(parent, entry_root);
        let pkg = package_id(&root, entry_manifest);
        let dest_dir = native_root.join(&pkg);
        fs::create_dir_all(&dest_dir)
            .map_err(|e| format!("no se pudo crear '{}': {}", dest_dir.display(), e))?;

        // Copia el asset y sus vecinos nativos (dependencias co-localizadas).
        let entries = fs::read_dir(parent)
            .map_err(|e| format!("no se pudo leer '{}': {}", parent.display(), e))?;
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() || !is_native_lib(&path) {
                continue;
            }
            let filename = match path.file_name().and_then(|f| f.to_str()) {
                Some(f) => f.to_string(),
                None => continue,
            };
            let dest = dest_dir.join(&filename);
            if dest.exists() {
                let old = fs::read(&dest)
                    .map_err(|e| format!("no se pudo leer '{}': {}", dest.display(), e))?;
                let new = fs::read(&path)
                    .map_err(|e| format!("no se pudo leer '{}': {}", path.display(), e))?;
                if old != new {
                    return Err(format!(
                        "colision de libreria nativa '{}' en el paquete '{}' (contenido distinto)",
                        filename, pkg
                    ));
                }
            } else {
                fs::copy(&path, &dest).map_err(|e| {
                    format!(
                        "no se pudo copiar '{}' -> '{}': {}",
                        path.display(),
                        dest.display(),
                        e
                    )
                })?;
            }
            native_map.entry(pkg.clone()).or_default().insert(filename);
        }
    }
    Ok(())
}

fn load_config() -> Option<ModuleManifest> {
    let path = std::env::current_dir().ok()?.join("cls.json");
    if path.exists() {
        ModuleManifest::from_file(&path).ok()
    } else {
        None
    }
}

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::packages::{self, PackageSource, PackageSpec};

pub fn execute(cmd: &str, args: &[String]) -> i32 {
    match cmd {
        "add" => cmd_add(args),
        "remove" | "rm" => cmd_remove(args),
        "install" | "i" => cmd_install(args),
        _ => {
            eprintln!("Comando desconocido: clx {}", cmd);
            eprintln!("Uso: clx add <spec> | clx remove <spec> | clx install [<ruta|zip|spec>...]");
            1
        }
    }
}

// ── add ──────────────────────────────────────────────────────────────────────

fn cmd_add(args: &[String]) -> i32 {
    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!("clx add - Agregar dependencia a cls.json (sin descargar)");
        println!();
        println!("Uso: clx add <spec> [--dev]");
        println!();
        println!("  <spec>  [scope/]nombre[@version]");
        println!("          con versión -> fija; sin versión -> rango ^<última>");
        println!("  --dev   agregar a devDependencies");
        return 0;
    }
    let spec_arg = match args.iter().find(|a| !a.starts_with('-')) {
        Some(s) => s.clone(),
        None => {
            eprintln!("Uso: clx add <spec> [--dev]");
            return 1;
        }
    };
    let is_dev = args.iter().any(|a| a == "--dev");

    let spec = match PackageSpec::parse(&spec_arg) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error: {}", e);
            return 1;
        }
    };

    let manifest_path = Path::new("cls.json");
    if !manifest_path.exists() {
        eprintln!("Error: cls.json no encontrado. Ejecuta 'clx new' primero");
        return 1;
    }

    // Valor de la dependencia: exacto si se especificó versión, si no rango ^.
    let value = if let Some(v) = &spec.version {
        v.clone()
    } else {
        match resolve_latest(&spec) {
            Some(v) => format!("^{}", v),
            None => {
                eprintln!(
                    "Error: módulo '{}' no resuelto (sin registry disponible y no instalado en global).",
                    spec.id()
                );
                return 1;
            }
        }
    };

    if let Err(e) = save_dependency(manifest_path, &spec.id(), &value, is_dev) {
        eprintln!("Error: {}", e);
        return 1;
    }
    let section = if is_dev { "devDependencies" } else { "dependencies" };
    println!("\"{}\": \"{}\" agregado a {}", spec.id(), value, section);
    0
}

// ── remove ───────────────────────────────────────────────────────────────────

fn cmd_remove(args: &[String]) -> i32 {
    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!("clx remove - Quitar dependencia de cls.json");
        println!();
        println!("Uso: clx remove <spec>");
        return 0;
    }
    let spec_arg = match args.iter().find(|a| !a.starts_with('-')) {
        Some(s) => s.clone(),
        None => {
            eprintln!("Uso: clx remove <spec>");
            return 1;
        }
    };
    let spec = match PackageSpec::parse(&spec_arg) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error: {}", e);
            return 1;
        }
    };
    let manifest_path = Path::new("cls.json");
    if !manifest_path.exists() {
        eprintln!("Error: cls.json no encontrado");
        return 1;
    }

    let content = match fs::read_to_string(manifest_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error al leer '{}': {}", manifest_path.display(), e);
            return 1;
        }
    };
    let mut json: serde_json::Value = match serde_json::from_str(&content) {
        Ok(j) => j,
        Err(e) => {
            eprintln!("Error: cls.json corrupto ({})", e);
            return 1;
        }
    };

    let key = spec.id();
    let removed = json["dependencies"]
        .as_object_mut()
        .map(|d| d.remove(&key).is_some())
        .unwrap_or(false)
        || json["devDependencies"]
            .as_object_mut()
            .map(|d| d.remove(&key).is_some())
            .unwrap_or(false);

    if removed {
        if let Err(e) = fs::write(manifest_path, serde_json::to_string_pretty(&json).unwrap()) {
            eprintln!("Error al escribir cls.json: {}", e);
            return 1;
        }
        println!("\"{}\" removido de cls.json", key);
        0
    } else {
        eprintln!("'{}' no encontrado en dependencies/devDependencies", key);
        1
    }
}

// ── install ──────────────────────────────────────────────────────────────────

fn cmd_install(args: &[String]) -> i32 {
    if args.iter().any(|a| a == "-h" || a == "--help") {
        print_install_help();
        return 0;
    }

    let save = args.iter().any(|a| a == "--save" || a == "-S");
    let force = args.iter().any(|a| a == "--force" || a == "-f");
    let sources: Vec<String> = args
        .iter()
        .filter(|a| !a.starts_with('-'))
        .cloned()
        .collect();

    for a in args {
        if a.starts_with('-') && !matches!(a.as_str(), "--save" | "-S" | "--force" | "-f") {
            eprintln!("Opción desconocida: {}", a);
            print_install_help();
            return 1;
        }
    }

    if sources.is_empty() {
        return install_deps(force);
    }

    let mut failed = false;
    for src in &sources {
        let p = Path::new(src);
        let res = if p.exists() {
            install_local_file(p, save, force)
        } else {
            install_spec(src, save, force)
        };
        if res != 0 {
            failed = true;
        }
    }
    if failed {
        1
    } else {
        0
    }
}

/// Instala un paquete local (dir o `.zip`/`.clsapp`) al store global versionado.
fn install_local_file(p: &Path, save: bool, force: bool) -> i32 {
    let is_zip = p.is_file()
        && p.extension()
            .map(|e| e.eq_ignore_ascii_case("zip") || e.eq_ignore_ascii_case("clsapp"))
            .unwrap_or(false);

    let info = if p.is_dir() {
        match packages::inspect_dir(p, PackageSource::Local) {
            Some(i) => i,
            None => {
                eprintln!("Error: '{}' no es un directorio de paquete válido", p.display());
                return 1;
            }
        }
    } else if is_zip {
        match packages::inspect_zip(p) {
            Ok(i) => i,
            Err(e) => {
                eprintln!("Error en '{}': {}", p.display(), e);
                return 1;
            }
        }
    } else {
        eprintln!("Error: '{}' no es un directorio ni un archivo .zip/.clsapp", p.display());
        return 1;
    };

    if !info.manifest_ok {
        eprintln!("Error: '{}' no tiene un cls.json válido (manifiesto obligatorio)", p.display());
        for issue in &info.issues {
            eprintln!("  - {}", issue);
        }
        return 1;
    }
    if info.name.trim().is_empty() {
        eprintln!("Error: el manifiesto no declara 'name' (obligatorio)");
        return 1;
    }
    if info.version.trim().is_empty() {
        eprintln!("Error: el manifiesto no declara 'version' (obligatorio)");
        return 1;
    }
    if !packages::is_valid_semver(&info.version) {
        eprintln!(
            "Error: versión no válida (se espera semver): '{}'",
            info.version
        );
        return 1;
    }
    let entry = match &info.export_entry {
        Some(e) if !e.is_empty() => e.clone(),
        _ => {
            eprintln!(
                "Error: '{}' no declara 'export.entry' en cls.json (requerido para instalar)",
                info.name
            );
            return 1;
        }
    };
    if !is_zip && !p.join(&entry).exists() {
        eprintln!("Error: export.entry '{}' no existe en '{}'", entry, p.display());
        return 1;
    }

    let dest = packages::global_package_dir(&None, &info.name, &info.version);
    if dest.exists() {
        if !force {
            eprintln!(
                "Error: '{}@{}' ya está instalado en '{}' (usa --force para sobrescribir)",
                info.name,
                info.version,
                dest.display()
            );
            return 1;
        }
        let _ = fs::remove_dir_all(&dest);
    }
    if let Some(parent) = dest.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let res = if is_zip {
        packages::extract_zip_to(p, &dest)
    } else {
        packages::copy_dir_all(p, &dest)
    };
    if let Err(e) = res {
        eprintln!("Error instalando '{}': {}", p.display(), e);
        return 1;
    }
    println!(
        "Instalado {}@{} -> {}",
        info.name,
        info.version,
        dest.display()
    );

    if save {
        if let Err(e) = save_cwd_dependency(&info.name, &info.version) {
            eprintln!("Aviso: --save no pudo actualizar cls.json: {}", e);
        } else {
            println!("\"{}\": \"{}\" agregado a dependencies", info.name, info.version);
        }
    }
    0
}

/// Instala un paquete por spec: si ya está en global, listo; si no, requiere
/// registry (Fase B).
fn install_spec(spec_arg: &str, save: bool, force: bool) -> i32 {
    let spec = match PackageSpec::parse(spec_arg) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error: {}", e);
            return 1;
        }
    };
    let _ = force;

    match packages::find_by_spec(&spec) {
        Some(info) => {
            println!(
                "{}@{} ya está instalado ({})",
                info.id(),
                info.version,
                info.path.display()
            );
            if save {
                if let Err(e) = save_cwd_dependency(&info.id(), &info.version) {
                    eprintln!("Aviso: --save no pudo actualizar cls.json: {}", e);
                }
            }
            0
        }
        None => {
            eprintln!(
                "Error: '{}' no está instalado y el registry aún no está disponible (Fase B).",
                spec.id()
            );
            eprintln!(
                "  driver de git: {}",
                if crate::git_driver::is_available() {
                    "disponible"
                } else {
                    "no instalado (scripts/gix-driver-build)"
                }
            );
            1
        }
    }
}

/// Instala las dependencias de `cls.json`. Versiones exactas → esa; rangos →
/// última que cumpla. Instala lo que falte desde el registry (Fase B pendiente)
/// y escribe `cls.lock`.
fn install_deps(force: bool) -> i32 {
    let _ = force;
    let manifest_path = Path::new("cls.json");
    if !manifest_path.exists() {
        eprintln!("Error: cls.json no encontrado (o pasa rutas a 'clx install')");
        return 1;
    }
    let manifest = match cls_core::config::ModuleManifest::from_file(manifest_path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Error: {}", e);
            return 1;
        }
    };

    let mut deps: Vec<(String, String)> = Vec::new();
    for (k, v) in &manifest.dependencies {
        deps.push((k.clone(), v.clone()));
    }
    for (k, v) in &manifest.dev_dependencies {
        deps.push((k.clone(), v.clone()));
    }
    if deps.is_empty() {
        println!("No hay dependencias que instalar.");
        return 0;
    }

    let registry = registry_url(&manifest);
    let mut lock_pkgs: BTreeMap<String, serde_json::Value> = BTreeMap::new();
    let mut missing: Vec<String> = Vec::new();

    for (id, range) in &deps {
        let spec = match PackageSpec::parse(id) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("  {} ... spec inválido ({})", id, e);
                missing.push(id.clone());
                continue;
            }
        };
        // Buscar una versión instalada que cumpla el rango declarado.
        let found = packages::installed_versions(&spec)
            .into_iter()
            .map(|(v, _)| v)
            .filter(|v| packages::version_satisfies(range, v))
            .max_by(|a, b| packages::cmp_semver(a, b));
        match found {
            Some(v) => {
                let channel = if packages::is_prerelease(&v) { "dev" } else { "stable" };
                println!("  {} {} (instalado)", id, v);
                lock_pkgs.insert(
                    id.clone(),
                    serde_json::json!({
                        "version": v,
                        "channel": channel,
                        "resolved": "global",
                    }),
                );
            }
            None => {
                eprintln!("  {} {} ... no instalado (requiere registry, Fase B)", id, range);
                missing.push(id.clone());
            }
        }
    }

    write_lockfile(&registry, &lock_pkgs);

    if missing.is_empty() {
        0
    } else {
        eprintln!();
        eprintln!(
            "Faltan {} dependencia(s): {}",
            missing.len(),
            missing.join(", ")
        );
        1
    }
}

// ── helpers ──────────────────────────────────────────────────────────────────

/// Última versión disponible: registry (si hay red) → global (stable) → None.
fn resolve_latest(spec: &PackageSpec) -> Option<String> {
    if let Some(v) = registry_latest(spec) {
        return Some(v);
    }
    packages::latest_installed(spec, true)
}

/// Consulta `GET {registry}/packages/{id}/latest?channel=stable` (best-effort).
/// Sin servidor (Fase B) devuelve None.
fn registry_latest(spec: &PackageSpec) -> Option<String> {
    let registry = std::env::var("CLS_REGISTRY")
        .ok()
        .or_else(|| {
            let c = fs::read_to_string("cls.json").ok()?;
            let j: serde_json::Value = serde_json::from_str(&c).ok()?;
            j["registry"].as_str().map(|s| s.to_string())
        })
        .unwrap_or_else(|| "https://registry.cls-lang.org".to_string());
    let url = format!(
        "{}/packages/{}/latest?channel=stable",
        registry.trim_end_matches('/'),
        spec.id()
    );
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(2))
        .build();
    let resp = agent.get(&url).call().ok()?;
    let body = resp.into_string().ok()?;
    let j: serde_json::Value = serde_json::from_str(&body).ok()?;
    j["version"].as_str().map(|s| s.to_string())
}

fn registry_url(manifest: &cls_core::config::ModuleManifest) -> String {
    std::env::var("CLS_REGISTRY").unwrap_or_else(|_| manifest.registry.clone())
}

/// Agrega/actualiza una dependencia EXACTA en el `cls.json` del CWD.
fn save_cwd_dependency(id: &str, version: &str) -> Result<(), String> {
    save_dependency(Path::new("cls.json"), id, version, false)
}

/// Agrega/actualiza una dependencia en `manifest_path`.
fn save_dependency(
    manifest_path: &Path,
    id: &str,
    value: &str,
    is_dev: bool,
) -> Result<(), String> {
    let content = fs::read_to_string(manifest_path).map_err(|e| e.to_string())?;
    let mut json: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("cls.json corrupto: {}", e))?;
    let section = if is_dev { "devDependencies" } else { "dependencies" };
    json[section][id] = serde_json::Value::String(value.to_string());
    fs::write(manifest_path, serde_json::to_string_pretty(&json).unwrap())
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Escribe/mergea `cls.lock` con las entradas resueltas.
fn write_lockfile(registry: &str, packages: &BTreeMap<String, serde_json::Value>) {
    let lock = serde_json::json!({
        "lockfileVersion": 1,
        "registry": registry,
        "packages": packages,
    });
    if let Err(e) = fs::write("cls.lock", serde_json::to_string_pretty(&lock).unwrap()) {
        eprintln!("Aviso: no se pudo escribir cls.lock: {}", e);
    }
}

fn print_install_help() {
    println!("clx install - Instalar paquetes");
    println!();
    println!("Uso:");
    println!("  clx install                      Instala las dependencias de cls.json");
    println!("  clx install <ruta|zip>...        Instala paquetes locales (dir o .zip/.clsapp)");
    println!("  clx install <spec>...            Instala por nombre ([scope/]nombre[@versión])");
    println!();
    println!("Opciones:");
    println!("  -S, --save    Registrar la dependencia en cls.json (exacta)");
    println!("  -f, --force   Sobrescribir la versión ya instalada");
    println!();
    println!("Todo paquete se instala en ~/.cls/modules/[scope/]nombre@versión.");
    println!("Requiere 'name' y 'version' (semver) en el manifiesto del paquete.");
}

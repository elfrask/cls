use cls_runtime::{Intrinsics, Interpreter};
use cls_runtime::{VfsResolver, LocalFs, ClsLibResolver};
use cls_core::config::ModuleManifest;
use std::sync::Arc;
use std::path::Path;

pub fn execute(args: &[String]) -> i32 {
    // Help manual del subcomando
    if args.iter().take_while(|a| *a != "--").any(|a| a == "-h" || a == "--help") {
        print_help();
        return 0;
    }

    // `clx run --jit <archivo> [-- args...]` → compilación JIT
    let jit = args.iter().take_while(|a| *a != "--").any(|a| a == "--jit" || a == "-j");

    // Separar args de la app (todo después de --)
    let app_args: Vec<String> = args.iter()
        .skip_while(|a| *a != "--")
        .skip(1)
        .map(|s| s.to_string())
        .collect();

    // Entry: ignorar los flags --jit y --target <valor> al resolver el archivo
    let mut cli_args: Vec<String> = Vec::new();
    let mut skip_next = false;
    for a in args.iter().take_while(|a| *a != "--") {
        if skip_next {
            skip_next = false;
            continue;
        }
        if a == "--jit" || a == "-j" {
            continue;
        }
        if a == "--target" || a == "-t" {
            skip_next = true;
            continue;
        }
        cli_args.push(a.clone());
    }

    let config = load_config();

    let entry = resolve_entry(&cli_args, config.as_ref());

    if jit {
        let target_opt: Option<String> = {
            let a: Vec<&String> = args.iter().take_while(|a| *a != "--").collect();
            let mut t: Option<String> = None;
            let mut i = 0;
            while i < a.len() {
                if a[i] == "--target" || a[i] == "-t" {
                    if let Some(v) = a.get(i + 1) {
                        t = Some(v.to_string());
                    }
                    break;
                }
                if let Some(v) = a[i].strip_prefix("--target=") {
                    t = Some(v.to_string());
                    break;
                }
                i += 1;
            }
            t
        };
        return crate::jit::run_jit(&entry, &app_args, target_opt.as_deref());
    }

    let source = match std::fs::read_to_string(&entry) {
        Ok(s) => s,
        Err(e) => { eprintln!("Error al leer '{}': {}", entry, e); return 1; }
    };

    let mut lexer = cls_core::frontend::Lexer::new(&source);
    let tokens = match lexer.tokenize() {
        Ok(t) => t,
        Err(e) => { cls_runtime::show_syntax_error(e, &source, &entry); return 1; }
    };

    let mut parser = cls_core::frontend::Parser::new(tokens);
    let module = match parser.parse() {
        Ok(m) => m,
        Err(e) => { cls_runtime::show_syntax_error(e, &source, &entry); return 1; }
    };

    let vfs = make_vfs(config.as_ref());
    let lib_resolver = make_lib_resolver(vfs.clone());
    let native: std::sync::Arc<dyn cls_runtime::ffi::NativeBackend> =
        std::sync::Arc::new(crate::native::DynamicBackend::default());
    let entry_dir = std::path::Path::new(&entry)
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let resolver = make_desktop_resolver(vfs, lib_resolver, native.clone(), entry_dir);
    let mut interpreter = Interpreter::new(Intrinsics::desktop_defaults(app_args), resolver);
    interpreter.set_source_file(entry);
    interpreter.set_config(config);
    interpreter.set_native_backend(native.clone());
    // `clx run --target <tripla>` → simula el entorno para la directiva `when`
    if let Some(t) = args.iter()
        .take_while(|a| *a != "--")
        .position(|a| a == "--target" || a == "-t")
        .and_then(|i| args.get(i + 1))
    {
        interpreter.set_target_str(t);
    }

    if let Err(e) = interpreter.execute(&module) {
        let report = interpreter.build_error_report(e);
        cls_runtime::show_runtime_error(&report);
        return 1;
    }
    match interpreter.call_main() {
        Ok(code) => code,
        Err(e) => {
            let report = interpreter.build_error_report(e);
            cls_runtime::show_runtime_error(&report);
            1
        }
    }
}

fn print_help() {
    println!("clx run — Ejecutar un programa CLS");
    println!();
    println!("Uso: clx run [archivo] [--] [args...]");
    println!();
    println!("Opciones:");
    println!("  --jit, -j               Compilar y ejecutar con el intérprete JIT (CLS → WASM)");
    println!("  --target <tripla>, -t   Simular el entorno para la directiva 'when'");
    println!("  -h, --help              Mostrar esta ayuda");
    println!("  --                      Separar los args de la aplicación");
    println!();
    println!("Sin archivo, usa el 'entry' de cls.json (o busca main.clsx).");
}

fn resolve_entry(args: &[String], config: Option<&ModuleManifest>) -> String {
    // Solo considerar args antes de -- (después son app_args)
    let entry_arg = args.iter().take_while(|a| *a != "--").find(|a| !a.starts_with("-"));
    if let Some(e) = entry_arg {
        return e.to_string();
    }
    // Si no, usar entry de cls.json
    if let Some(cfg) = config {
        let proposed = &cfg.entry;
        if !proposed.is_empty() && Path::new(proposed).exists() {
            return proposed.clone();
        }
        // Si el entry no existe, buscar main.clsx
        let candidates = ["main.clsx", "src/main.clsx", "mod.clsx", "src/mod.clsx"];
        for c in &candidates {
            if Path::new(c).exists() {
                return c.to_string();
            }
        }
    }
    eprintln!("Uso: clx run <archivo> [--] [args...]");
    eprintln!("  (o ejecuta desde un proyecto con cls.json que tenga 'entry')");
    std::process::exit(1);
}

fn load_config() -> Option<ModuleManifest> {
    let cwd = std::env::current_dir().ok()?;
    let path = cwd.join("cls.json");
    if path.exists() {
        ModuleManifest::from_file(&path).ok()
    } else {
        None
    }
}

fn make_vfs(config: Option<&ModuleManifest>) -> Arc<VfsResolver> {
    let mut vfs = VfsResolver::new();
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    vfs.register("app", Arc::new(LocalFs::new("app", &cwd, false)));

    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(std::path::PathBuf::from)
        .ok();
    if let Some(ref h) = home {
        vfs.register("user", Arc::new(LocalFs::new("user", h, false)));
    }

    let tmp = std::env::temp_dir();
    vfs.register("tmp", Arc::new(LocalFs::new("tmp", &tmp, false)));

    if let Some(cfg) = config {
        let entry_path = Path::new(&cfg.entry);
        if let Some(parent) = entry_path.parent() {
            vfs.add_route("project", &parent.to_string_lossy()).ok();
        }
    }

    Arc::new(vfs)
}

fn make_lib_resolver(vfs: Arc<VfsResolver>) -> Arc<dyn ClsLibResolver> {
    Arc::new(DesktopLibResolver { vfs })
}

fn make_desktop_resolver(
    vfs: Arc<VfsResolver>,
    lib_resolver: Arc<dyn ClsLibResolver>,
    native: std::sync::Arc<dyn cls_runtime::ffi::NativeBackend>,
    entry_dir: std::path::PathBuf,
) -> cls_runtime::ModuleResolver {
    let mut resolver = cls_runtime::ModuleResolver::new().with_core_stdlib();
    resolver.add_internal("fs", crate::modules::fs::module(vfs));
    resolver.add_internal("http", crate::modules::http::module());
    resolver.add_internal("Lib", crate::modules::lib::module(lib_resolver));
    // Directorios donde buscar módulos de usuario instalados: ~/.cls/modules/
    let entry_dir = entry_dir.clone();
    let manifest = cls_core::config::ModuleManifest::find_in_dir(&entry_dir);
    resolver.set_external(move |path: String, _env: &mut cls_runtime::Environment| -> cls_core::error::ClsResult<Option<cls_runtime::Value>> {
        let candidates = crate::jit::module_candidates(&path, &entry_dir, manifest.as_ref());
        for candidate in candidates {
            if let Ok(source) = std::fs::read_to_string(&candidate) {
                // El nodo consigue el source; el runtime (centralizado) lo carga y recolecta exports
                let mut interp = Interpreter::new(
                    Intrinsics::empty(),
                    cls_runtime::ModuleResolver::new().with_core_stdlib(),
                );
                interp.set_native_backend(native.clone());
                let path_for_module = candidate.to_string_lossy().to_string();
                return Ok(Some(interp.load_module_source(&path_for_module, &source)?));
            }
        }
        Ok(None)
    });
    resolver
}

// ─── DesktopClsLibResolver ──────────────────────────────────────────────────
use cls_runtime::ClsLibIndex;
use cls_core::error::ClsResult;

struct DesktopLibResolver {
    vfs: Arc<VfsResolver>,
}

impl DesktopLibResolver {
    fn try_read(&self, path: &str) -> Result<Vec<u8>, ()> {
        if path.contains("://") {
            self.vfs.read_file(path).map_err(|_| ())
        } else {
            std::fs::read(path).map_err(|_| ())
        }
    }
}

impl ClsLibResolver for DesktopLibResolver {
    fn resolve(&self, name: &str) -> ClsResult<Option<Vec<u8>>> {
        if name.contains('/') || name.contains('\\') || name.ends_with(".clslib") {
            if let Ok(data) = self.try_read(name) {
                return Ok(Some(data));
            }
            return Ok(None);
        }

        let name = name.trim_end_matches(".clslib");

        if let Ok(data) = self.try_read(&format!("./libs/{}.clslib", name)) {
            return Ok(Some(data));
        }

        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_default();
        if !home.is_empty() {
            let named = format!("{}/.cls/clslibs/names/{}.clslib", home, name);
            if let Ok(data) = self.try_read(&named) {
                return Ok(Some(data));
            }

            let index_path = format!("{}/.cls/clslibs/index.json", home);
            if let Ok(index_json) = std::fs::read_to_string(&index_path) {
                if let Ok(index) = serde_json::from_str::<ClsLibIndex>(&index_json) {
                    if let Some(entry) = index.find(name) {
                        let hash_path = format!("{}/.cls/clslibs/by-hash/{}/{}.clslib", home, entry.hash, name);
                        if let Ok(data) = self.try_read(&hash_path) {
                            return Ok(Some(data));
                        }
                    }
                }
            }
        }

        Ok(None)
    }
}

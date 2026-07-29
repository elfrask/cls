use cls_runtime::{Intrinsics, Interpreter};
use cls_runtime::{VfsResolver, LocalFs, ClsLibResolver};
use crate::module_loader;
use std::sync::Arc;

pub fn execute(args: &[String]) -> i32 {
    if args.is_empty() {
        eprintln!("Uso: clx run <archivo> [--] [args...]");
        return 1;
    }
    let path = &args[0];
    let app_args: Vec<String> = args[1..].to_vec();

    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => { eprintln!("Error al leer '{}': {}", path, e); return 1; }
    };

    let mut lexer = cls_core::frontend::Lexer::new(&source);
    let tokens = match lexer.tokenize() {
        Ok(t) => t,
        Err(e) => { cls_runtime::show_syntax_error(&e, &source, path); return 1; }
    };

    let mut parser = cls_core::frontend::Parser::new(tokens);
    let module = match parser.parse() {
        Ok(m) => m,
        Err(e) => { cls_runtime::show_syntax_error(&e, &source, path); return 1; }
    };

    let vfs = make_vfs();
    let lib_resolver = make_lib_resolver(vfs.clone());
    let resolver = make_desktop_resolver(vfs, lib_resolver);
    let mut interpreter = Interpreter::new(Intrinsics::desktop_defaults(app_args), resolver);
    interpreter.set_source_file(path.to_string());

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

fn make_vfs() -> Arc<VfsResolver> {
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
    Arc::new(vfs)
}

fn make_lib_resolver(vfs: Arc<VfsResolver>) -> Arc<dyn ClsLibResolver> {
    Arc::new(DesktopLibResolver { vfs })
}

fn make_desktop_resolver(vfs: Arc<VfsResolver>, lib_resolver: Arc<dyn ClsLibResolver>) -> cls_runtime::ModuleResolver {
    let mut resolver = cls_runtime::ModuleResolver::new().with_core_stdlib();
    resolver.add_internal("fs", crate::modules::fs::module(vfs));
    resolver.add_internal("http", crate::modules::http::module());
    resolver.add_internal("Lib", crate::modules::lib::module(lib_resolver));
    resolver.set_external(|path: String, _env: &mut cls_runtime::Environment| -> cls_core::error::ClsResult<Option<cls_runtime::Value>> {
        let candidate = format!("{}.clsx", path);
        match std::fs::read_to_string(&candidate) {
            Ok(source) => Ok(Some(module_loader::load_module(&source)?)),
            Err(_) => Ok(None),
        }
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
        // Si es path directo (contiene / o termina en .clslib), intentar directo
        if name.contains('/') || name.contains('\\') || name.ends_with(".clslib") {
            if let Ok(data) = self.try_read(name) {
                return Ok(Some(data));
            }
            return Ok(None);
        }

        let name = name.trim_end_matches(".clslib");

        // 1. Local: ./libs/{name}.clslib
        if let Ok(data) = self.try_read(&format!("./libs/{}.clslib", name)) {
            return Ok(Some(data));
        }

        // 2. Global names/: ~/.cls/clslibs/names/{name}.clslib
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_default();
        if !home.is_empty() {
            let named = format!("{}/.cls/clslibs/names/{}.clslib", home, name);
            if let Ok(data) = self.try_read(&named) {
                return Ok(Some(data));
            }

            // 3. Via index.json → by-hash/
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

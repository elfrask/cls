use std::fs;
use std::path::Path;
use cls_core::config::ModuleManifest;

pub fn execute(args: &[String]) -> i32 {
    let strict = args.iter().any(|a| a == "--strict");
    let path = args.iter().find(|a| !a.starts_with("--")).map(|s| s.as_str()).unwrap_or(".");

    // Cargar config para tipos
    let mut types_config = ModuleManifest::find_and_load()
        .ok()
        .map(|m| m.compiler.types)
        .unwrap_or_else(|| cls_core::config::types::TypesConfig {
            check: true,
            strict,
            ..Default::default()
        });
    if strict {
        types_config.strict = true;
    }

    let p = Path::new(path);
    if !p.exists() {
        eprintln!("Error: '{}' no encontrado", path);
        return 1;
    }

    let files: Vec<String> = if p.is_dir() {
        scan_clsx_files(p)
    } else {
        vec![path.to_string()]
    };

    if files.is_empty() {
        eprintln!("No se encontraron archivos .clsx en '{}'", path);
        return 1;
    }

    let mut total_errors = 0;
    let mut total_warnings = 0;

    for file in &files {
        let source = match fs::read_to_string(file) {
            Ok(s) => s,
            Err(e) => { eprintln!("Error al leer '{}': {}", file, e); total_errors += 1; continue; }
        };

        let mut lexer = cls_core::frontend::Lexer::new(&source);
        let tokens = match lexer.tokenize() {
            Ok(t) => t,
            Err(e) => { super::util::show_error(&source, &e.to_string(), file); total_errors += 1; continue; }
        };

        let mut parser = cls_core::frontend::Parser::new(tokens);
        let module = match parser.parse() {
            Ok(m) => m,
            Err(e) => { super::util::show_error(&source, &e.to_string(), file); total_errors += 1; continue; }
        };

        let mut checker = cls_core::middleware::TypeChecker::new(types_config.clone());
        if let Err(e) = checker.check(&module) {
            eprintln!("Error interno en '{}': {}", file, e);
            total_errors += 1;
            continue;
        }
        let diagnostics = checker.diagnostics();

        if diagnostics.is_empty() && files.len() == 1 {
            println!("No se encontraron errores de tipo.");
            return 0;
        }

        for diag in diagnostics {
            let severity = match diag.severity {
                cls_core::error::diagnostic::Severity::Error => "ERROR",
                cls_core::error::diagnostic::Severity::Warning => "WARN",
                _ => "INFO",
            };
            let name = if files.len() > 1 {
                format!("{}:{}", file, diag.span)
            } else {
                format!("{}:{}", diag.span.start_line, diag.span.start_col)
            };
            eprintln!("[{}] {} ({})", severity, diag.message, name);
        }

        let errs = diagnostics.iter().filter(|d| matches!(d.severity, cls_core::error::diagnostic::Severity::Error)).count();
        let warns = diagnostics.len() - errs;
        total_errors += errs;
        total_warnings += warns;
    }

    if files.len() > 1 {
        let summary = if total_errors > 0 || total_warnings > 0 {
            format!("{} errores, {} advertencias en {} archivos", total_errors, total_warnings, files.len())
        } else {
            format!("Sin errores en {} archivos", files.len())
        };
        eprintln!("{}", summary);
    }

    if total_errors > 0 { 1 } else { 0 }
}

fn scan_clsx_files(dir: &Path) -> Vec<String> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // Skip hidden dirs and node-style dirs
                if let Some(name) = path.file_name() {
                    let n = name.to_string_lossy();
                    if n.starts_with('.') || n == "modules" || n == "dist" || n == "libs" {
                        continue;
                    }
                }
                files.extend(scan_clsx_files(&path));
            } else if path.extension().map(|e| e == "clsx").unwrap_or(false) {
                files.push(path.to_string_lossy().to_string());
            }
        }
    }
    files
}

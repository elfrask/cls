use std::fs;
use std::path::Path;
use std::collections::HashSet;
use cls_core::config::ModuleManifest;
use cls_core::frontend::ast::{Module, Statement};

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
        // El nodo resuelve los imports (lee los archivos) y los pasa como prelude;
        // el core registra los tipos del prelude antes de chequear el módulo principal.
        let base_dir = Path::new(file).parent().unwrap_or(Path::new(".")).to_path_buf();
        let mut seen = HashSet::new();
        let mut imports: Vec<Module> = Vec::new();
        load_import_modules(&module, &base_dir, &mut seen, &mut imports);
        if let Err(e) = checker.check_with_prelude(&module, &imports) {
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
            // Contexto de código: mostrar la línea fuente y el caret en la posición
            let line = diag.span.start_line as usize;
            let col = diag.span.start_col as usize;
            if let Some(src_line) = source.lines().nth(line.saturating_sub(1)) {
                let pad = " ".repeat(line.to_string().len());
                eprintln!("  {} | {}", line, src_line);
                let width = if diag.span.end_line == diag.span.start_line && diag.span.end_col > diag.span.start_col {
                    (diag.span.end_col - diag.span.start_col) as usize
                } else {
                    1
                };
                eprintln!("  {} | {}{}", pad, " ".repeat(col.saturating_sub(1)), "^".repeat(width));
            }
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

/// Resuelve los imports de un módulo (recursivamente) y los carga como AST.
/// El nodo consigue los archivos; el core los verifica.
fn load_import_modules(
    module: &Module,
    base_dir: &Path,
    seen: &mut HashSet<String>,
    out: &mut Vec<Module>,
) {
    for stmt in &module.statements {
        if let Statement::Import(i) = stmt {
            let candidate = base_dir.join(format!("{}.clsx", i.path));
            let key = candidate.to_string_lossy().to_string();
            if seen.insert(key) {
                if let Ok(source) = fs::read_to_string(&candidate) {
                    if let Ok(toks) = cls_core::frontend::Lexer::new(&source).tokenize() {
                        if let Ok(m) = cls_core::frontend::Parser::new(toks).parse() {
                            load_import_modules(&m, base_dir, seen, out);
                            out.push(m);
                        }
                    }
                }
            }
        }
    }
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

use std::fs;

pub fn execute(args: &[String]) -> i32 {
    if args.is_empty() {
        eprintln!("Uso: clx check <archivo> [--strict]");
        return 1;
    }
    let path = &args[0];
    let strict = args.contains(&"--strict".to_string());

    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => { eprintln!("Error al leer '{}': {}", path, e); return 1; }
    };

    let mut lexer = cls_core::frontend::Lexer::new(&source);
    let tokens = match lexer.tokenize() {
        Ok(t) => t,
        Err(e) => { super::util::show_error(&source, &e.to_string(), path); return 1; }
    };

    let mut parser = cls_core::frontend::Parser::new(tokens);
    let module = match parser.parse() {
        Ok(m) => m,
        Err(e) => { super::util::show_error(&source, &e.to_string(), path); return 1; }
    };

    let config = cls_core::config::types::TypesConfig {
        check: true,
        strict,
        ..Default::default()
    };
    let mut checker = cls_core::middleware::TypeChecker::new(config);
    if let Err(e) = checker.check(&module) {
        eprintln!("Error interno: {}", e);
        return 1;
    }
    let diagnostics = checker.diagnostics();

    if diagnostics.is_empty() {
        println!("No se encontraron errores de tipo.");
        return 0;
    }

    for diag in diagnostics {
        let severity = match diag.severity {
            cls_core::error::diagnostic::Severity::Error => "ERROR",
            cls_core::error::diagnostic::Severity::Warning => "WARN",
            _ => "INFO",
        };
        eprintln!("[{}] {} ({}:{})", severity, diag.message, diag.span.start_line, diag.span.start_col);
    }
    let errors = diagnostics.iter().filter(|d| matches!(d.severity, cls_core::error::diagnostic::Severity::Error)).count();
    if errors > 0 { 1 } else { 0 }
}

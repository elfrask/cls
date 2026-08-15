use std::fs;

pub fn execute(args: &[String]) -> i32 {
    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!("clx ast — Dump del AST");
        println!();
        println!("Uso: clx ast <archivo> [--json]");
        println!();
        println!("  <archivo>  Archivo .clsx a analizar");
        println!("  --json     Serializar el AST como JSON (default: Debug)");
        return 0;
    }
    if args.is_empty() {
        eprintln!("Uso: clx ast <archivo> [--json]");
        return 1;
    }
    let target = &args[0];
    if target.starts_with('-') {
        eprintln!("Error: '{}' no es un archivo válido (usa 'clx ast -h' para ayuda)", target);
        return 1;
    }
    let source = match fs::read_to_string(target) {
        Ok(s) => s,
        Err(e) => { eprintln!("Error al leer '{}': {}", target, e); return 1; }
    };

    let mut lexer = cls_core::frontend::Lexer::new(&source);
    let tokens = match lexer.tokenize() {
        Ok(t) => t,
        Err(e) => { cls_runtime::show_syntax_error(e, &source, target); return 1; }
    };

    let mut parser = cls_core::frontend::Parser::new(tokens);
    let module = match parser.parse() {
        Ok(m) => m,
        Err(e) => { cls_runtime::show_syntax_error(e, &source, target); return 1; }
    };

    if args.iter().any(|a| a == "--json") {
        let backend = cls_core::backend::JsonBackend::new();
        match backend.emit(&module) {
            Ok(json) => println!("{}", json),
            Err(e) => { eprintln!("Error al serializar AST: {}", e); return 1; }
        }
    } else {
        println!("{:#?}", module);
    }
    0
}

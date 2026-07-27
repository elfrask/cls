use std::fs;

pub fn execute(args: &[String]) -> i32 {
    if args.is_empty() {
        eprintln!("Uso: clx ast <archivo> [--json]");
        return 1;
    }
    let source = match fs::read_to_string(&args[0]) {
        Ok(s) => s,
        Err(e) => { eprintln!("Error al leer '{}': {}", args[0], e); return 1; }
    };

    let mut lexer = cls_core::frontend::Lexer::new(&source);
    let tokens = match lexer.tokenize() {
        Ok(t) => t,
        Err(e) => { super::util::show_error(&source, &e.to_string(), &args[0]); return 1; }
    };

    let mut parser = cls_core::frontend::Parser::new(tokens);
    let module = match parser.parse() {
        Ok(m) => m,
        Err(e) => { super::util::show_error(&source, &e.to_string(), &args[0]); return 1; }
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

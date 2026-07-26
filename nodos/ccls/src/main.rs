//! ccls — CLI principal de CLS
//! Subcomandos: run, check, build, ast
use std::env;
use std::fs;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("ccls 2.0 — CLS Language Compiler & Runner");
        println!("Uso: ccls <comando> [opciones...]");
        println!("");
        println!("Comandos:");
        println!("  run   <archivo> [args...]    Ejecutar script .ccls o .clsapp");
        println!("  check <archivo>              Verificar tipos");
        println!("  build <archivo> -o <salida>  Compilar a .clsapp");
        println!("  ast   <archivo> --json       Dump AST como JSON");
        return;
    }
    let cmd = &args[1];
    let result = match cmd.as_str() {
        "run" => cmd_run(&args[2..]),
        "check" => cmd_check(&args[2..]),
        "build" => cmd_build(&args[2..]),
        "ast" => cmd_ast(&args[2..]),
        _ => {
            eprintln!("Comando desconocido: {}. Usa 'ccls' sin argumentos para ayuda.", cmd);
            process::exit(1);
        }
    };
    process::exit(result);
}

fn cmd_run(args: &[String]) -> i32 {
    if args.is_empty() {
        eprintln!("Uso: ccls run <archivo> [args...]");
        return 1;
    }
    let path = &args[0];
    let app_args: Vec<String> = args[1..].to_vec();

    // 1. Leer el archivo
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error al leer '{}': {}", path, e);
            return 1;
        }
    };

    // 2. Tokenizar
    let mut lexer = cls_core::frontend::Lexer::new(&source);
    let tokens = match lexer.tokenize() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Error de tokenización: {}", e);
            return 1;
        }
    };

    // 3. Parsear
    let mut parser = cls_core::frontend::Parser::new(tokens);
    let module = match parser.parse() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Error de parseo: {}", e);
            return 1;
        }
    };

    // 4. Ejecutar (tree-walker)
    let mut interpreter = cls_runtime::Interpreter::new(app_args);
    if let Err(e) = interpreter.execute(&module) {
        eprintln!("Error de ejecución: {}", e);
        return 1;
    }

    // 5. Llamar main() y retornar código de salida

    // 5. Llamar main() y retornar código de salida
    match interpreter.call_main() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("Error en main(): {}", e);
            1
        }
    }
}

fn cmd_check(args: &[String]) -> i32 {
    if args.is_empty() {
        eprintln!("Uso: ccls check <archivo>");
        return 1;
    }

    let source = match fs::read_to_string(&args[0]) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error al leer '{}': {}", args[0], e);
            return 1;
        }
    };

    let mut lexer = cls_core::frontend::Lexer::new(&source);
    let tokens = match lexer.tokenize() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Error de tokenización: {}", e);
            return 1;
        }
    };

    let mut parser = cls_core::frontend::Parser::new(tokens);
    let module = match parser.parse() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Error de parseo: {}", e);
            return 1;
        }
    };

    let config = cls_core::config::types::TypesConfig::default();
    let mut checker = cls_core::middleware::TypeChecker::new(config);
    if let Err(e) = checker.check(&module) {
        eprintln!("Error interno: {}", e);
        return 1;
    };
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
        eprintln!("[{}] {} ({}:{})",
            severity,
            diag.message,
            diag.span.start_line,
            diag.span.start_col
        );
    }
    let errors = diagnostics.iter().filter(|d| matches!(d.severity, cls_core::error::diagnostic::Severity::Error)).count();
    if errors > 0 { 1 } else { 0 }
}

fn cmd_build(args: &[String]) -> i32 {
    if args.is_empty() {
        eprintln!("Uso: ccls build <archivo> [-o <salida>]");
        return 1;
    }
    // TODO: implementar compilación a .clsapp
    println!("[ccls] Build no implementado aún");
    0
}

fn cmd_ast(args: &[String]) -> i32 {
    if args.is_empty() {
        eprintln!("Uso: ccls ast <archivo> [--json]");
        return 1;
    }

    let source = match fs::read_to_string(&args[0]) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error al leer '{}': {}", args[0], e);
            return 1;
        }
    };

    let mut lexer = cls_core::frontend::Lexer::new(&source);
    let tokens = match lexer.tokenize() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Error de tokenización: {}", e);
            return 1;
        }
    };

    let mut parser = cls_core::frontend::Parser::new(tokens);
    let module = match parser.parse() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Error de parseo: {}", e);
            return 1;
        }
    };

    let use_json = args.iter().any(|a| a == "--json");
    if use_json {
        let backend = cls_core::backend::JsonBackend::new();
        match backend.emit(&module) {
            Ok(json) => println!("{}", json),
            Err(e) => {
                eprintln!("Error al serializar AST: {}", e);
                return 1;
            }
        }
    } else {
        println!("{:#?}", module);
    }
    0
}

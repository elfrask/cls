//! clxr — CLS Runtime ejecutor
//! Uso: clxr <archivo.clsx | .clsapp> [args...]
use std::env;
use std::fs;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("clxr 2.0 — CLS Runtime Executor");
        eprintln!("Uso: clxr <archivo> [args...]");
        eprintln!("  Ejecuta archivos .clsx o .clsapp directamente");
        process::exit(1);
    }

    let path = &args[1];
    let app_args: Vec<String> = args[2..].to_vec();

    if path.ends_with(".clsapp") {
        eprintln!("[clxr] .clsapp support no implementado aún");
        process::exit(1);
    }

    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => { eprintln!("Error al leer '{}': {}", path, e); process::exit(1); }
    };

    let mut lexer = cls_core::frontend::Lexer::new(&source);
    let tokens = match lexer.tokenize() {
        Ok(t) => t,
        Err(e) => { show_error(&source, &e.to_string(), path); process::exit(1); }
    };

    let mut parser = cls_core::frontend::Parser::new(tokens);
    let module = match parser.parse() {
        Ok(m) => m,
        Err(e) => { show_error(&source, &e.to_string(), path); process::exit(1); }
    };

    // Runtime ligero: solo math + json (sin fs, sin http)
    let resolver = cls_runtime::ModuleResolver::new().with_core_stdlib();
    let mut interpreter = cls_runtime::Interpreter::new(
        cls_runtime::Intrinsics::desktop_defaults(app_args),
        resolver,
    );
    interpreter.set_source_file(path.to_string());

    if let Err(e) = interpreter.execute(&module) {
        eprintln!("{}", e);
        process::exit(1);
    }
    match interpreter.call_main() {
        Ok(code) => process::exit(code),
        Err(e) => { eprintln!("{}", e); process::exit(1); }
    }
}

fn show_error(source: &str, error_msg: &str, path: &str) {
    eprintln!("Error en '{}':", path);
    eprintln!("  {}", error_msg);
    if source.is_empty() { return; }
    let line_col = error_msg.split("línea").nth(1).and_then(|s| {
        let parts: Vec<&str> = s.splitn(2, ',').collect();
        let line = parts.first()?.trim().parse::<usize>().ok()?;
        let col = parts.get(1).and_then(|c|
            c.split("columna").nth(1).and_then(|c2|
                c2.trim().trim_matches(|p| p == ')' || p == '(').parse::<usize>().ok()
            )
        ).unwrap_or(1);
        Some((line, col))
    });
    if let Some((line, col)) = line_col {
        if let Some(src_line) = source.lines().nth(line.saturating_sub(1)) {
            eprintln!("");
            eprintln!("  {} | {}", line, src_line);
            let pad = " ".repeat(line.to_string().len());
            eprintln!("  {} | {}{}", pad, " ".repeat(col.saturating_sub(1) as usize), "^");
        }
    }
}

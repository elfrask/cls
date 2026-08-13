use cls_runtime::{Intrinsics, Interpreter, ModuleResolver, Value};
use std::io::Write;

pub fn execute(_args: &[String]) -> i32 {
    println!("CLS 2.0 REPL (Ctrl+C o :salir para salir)");
    println!("");

    let resolver = ModuleResolver::new().with_core_stdlib();
    let mut interpreter = Interpreter::new(Intrinsics::desktop_defaults(vec![]), resolver);

    loop {
        let line = match read_line("> ") {
            Some(l) => l,
            None => { println!(""); break; }
        };

        let trimmed = line.trim().to_string();
        if trimmed.is_empty() { continue; }
        if matches!(trimmed.as_str(), "exit" | "quit" | ":exit" | ":quit" | ":salir" | ":q") { break; }
        if matches!(trimmed.as_str(), ":help" | ":h") {
            println!("  exit / quit / :q / :salir  Salir");
            continue;
        }

        // Detectar si es statement completo o expresión suelta
        let is_expr = !trimmed.starts_with("var ")
            && !trimmed.starts_with("const ")
            && !trimmed.starts_with("function ")
            && !trimmed.starts_with("for ")
            && !trimmed.starts_with("while ")
            && !trimmed.starts_with("if ")
            && !trimmed.starts_with("switch ")
            && !trimmed.starts_with("try ")
            && !trimmed.starts_with("import ")
            && !trimmed.starts_with("from ")
            && !trimmed.starts_with("include ")
            && !trimmed.starts_with("with ")
            && !trimmed.starts_with("loop ")
            && !trimmed.starts_with("return ")
            && !trimmed.starts_with("export ");

        let source = if is_expr {
            format!("print({});", trimmed)
        } else {
            if trimmed.ends_with(';') || trimmed.ends_with('}') {
                trimmed.clone()
            } else {
                format!("{};", trimmed)
            }
        };

        let mut lexer = cls_core::frontend::Lexer::new(&source);
        let tokens = match lexer.tokenize() {
            Ok(t) => t,
            Err(e) => { eprintln!("Error: {}", e); continue; }
        };

        let mut parser = cls_core::frontend::Parser::new(tokens);
        let module = match parser.parse() {
            Ok(m) => m,
            Err(e) => { eprintln!("Error: {}", e); continue; }
        };

        match interpreter.execute(&module) {
            Ok(Value::Void) => {}
            Ok(v) if is_expr => println!("{}", v),
            Ok(_) => {}
            Err(e) => eprintln!("Error: {}", e),
        }
    }

    println!("Adiós!");
    0
}

fn read_line(prompt: &str) -> Option<String> {
    print!("{}", prompt);
    std::io::stdout().flush().ok()?;
    let mut line = String::new();
    match std::io::stdin().read_line(&mut line) {
        Ok(0) => None,
        Ok(_) => Some(line.trim_end().to_string()),
        Err(_) => None,
    }
}

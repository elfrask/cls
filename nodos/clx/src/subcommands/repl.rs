use cls_jit::repl::{ReplResult, ReplSession};
use cls_jit::JitContext;
use std::io::Write;

pub fn execute(_args: &[String]) -> i32 {
    println!("CLS 2.0 REPL (JIT) (Ctrl+C o :salir para salir)");
    println!("");

    let ctx = JitContext {
        native_backend: std::sync::Arc::new(crate::native::DynamicBackend),
        module_index: None,
        host_intrinsics: &[],
        host_call_handler: None,
        module_source_resolver: None,
        output: None,
    };
    let mut session = match ReplSession::new() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[JIT] Error creando el REPL: {}", e);
            return 1;
        }
    };

    loop {
        let line = match read_line("> ") {
            Some(l) => l,
            None => {
                println!("");
                break;
            }
        };

        let trimmed = line.trim().to_string();
        if trimmed.is_empty() {
            continue;
        }
        if matches!(trimmed.as_str(), "exit" | "quit" | ":exit" | ":quit" | ":salir" | ":q") {
            break;
        }
        if matches!(trimmed.as_str(), ":help" | ":h") {
            println!("  exit / quit / :q / :salir  Salir");
            continue;
        }

        // Detectar si es statement completo o expresión suelta (envuelta en print).
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
            let expr = if trimmed.ends_with(';') {
                trimmed.trim_end_matches(';').to_string()
            } else {
                trimmed.clone()
            };
            format!("print({});", expr)
        } else {
            if trimmed.ends_with(';') || trimmed.ends_with('}') {
                trimmed.clone()
            } else {
                format!("{};", trimmed)
            }
        };

        match session.run_line(&source, &ctx) {
            ReplResult::Ok => {}
            ReplResult::SyntaxError | ReplResult::CompileError | ReplResult::RuntimeError => {}
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

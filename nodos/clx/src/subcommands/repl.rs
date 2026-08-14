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
        let is_expr = !is_statement_start(&trimmed) && !is_lvalue_assign(&trimmed);

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

/// ¿La línea es una declaración top-level (statement completo, sin envolver
/// en `print`)? Cubre los keywords del lenguaje, incluidas las declaraciones
/// que el walker/JIT soporta a nivel top-level (B3b).
fn is_statement_start(line: &str) -> bool {
    const KEYWORDS: &[&str] = &[
        "var ", "const ", "function ", "for ", "while ", "if ", "switch ",
        "try ", "import ", "from ", "include ", "with ", "loop ", "return ",
        "export ", "class ", "enum ", "struct ", "interface ", "alias ",
        "namespace ", "extension ", "config ", "meta ", "when ",
        // `print(...)` ya imprime: envolverlo otra vez imprime el valor void
        // del print interno ("void") como argumento del externo.
        "print(", "print (",
    ];
    KEYWORDS.iter().any(|k| line.starts_with(k))
}

/// ¿La línea es una asignación a un lvalue (`x = 5`, `arr[0] = 99`, `a.b = 1`)?
/// Como statement se ejecuta en silencio (paridad con archivo); envuelta en
/// `print` devolvería punteros o rompería (B3a). Excluye comparaciones
/// (`==`, `!=`, `<=`, `>=`) y flechas (`=>`).
fn is_lvalue_assign(line: &str) -> bool {
    let s = line.trim_end().trim_end_matches(';').trim();
    if s.is_empty() {
        return false;
    }
    let bytes = s.as_bytes();
    for (i, &c) in bytes.iter().enumerate() {
        if c != b'=' {
            continue;
        }
        let prev = if i > 0 { bytes[i - 1] } else { 0 };
        let next = bytes.get(i + 1).copied().unwrap_or(0);
        if prev == b'=' || prev == b'!' || prev == b'<' || prev == b'>' || next == b'=' {
            continue;
        }
        return is_simple_lvalue(&s[..i]);
    }
    false
}

/// Lvalue simple: `identificador (["[" ... "]"] | ["." identificador])*`
/// (sin operadores). Whitespace permitido alrededor de `.`, `[`, `]`.
fn is_simple_lvalue(lhs: &str) -> bool {
    let mut rest = lhs.trim();
    let mut expect_ident = true;
    while !rest.is_empty() {
        let b = rest.as_bytes();
        if expect_ident {
            if !(b[0].is_ascii_alphabetic() || b[0] == b'_') {
                return false;
            }
            let mut i = 1;
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                i += 1;
            }
            rest = rest[i..].trim_start();
            expect_ident = false;
        } else if rest.starts_with('.') {
            rest = rest[1..].trim_start();
            expect_ident = true;
        } else if rest.starts_with('[') {
            match rest.find(']') {
                Some(p) => rest = rest[p + 1..].trim_start(),
                None => return false,
            }
        } else {
            return false;
        }
    }
    !expect_ident
}

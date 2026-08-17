use crate::walker::value::{FunValue, Value};
use std::collections::HashMap;

/// Funciones y valores top-level inyectados por el nodo.
/// El core da defaults, el nodo los personaliza.
pub struct Intrinsics {
    pub globals: HashMap<String, Value>,
    pub args: Vec<String>,
}

impl Intrinsics {
    /// Defaults para entorno desktop (stdout/stdin)
    pub fn desktop_defaults(args: Vec<String>) -> Self {
        let mut globals = HashMap::new();

        // print
        globals.insert("print".into(), Value::Fun(FunValue::new_native(
            "print", vec!["value".into()], |vals| {
                let s: String = vals.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(" ");
                println!("{}", s);
                Ok(Value::Void)
            },
        )));

        // input
        globals.insert("input".into(), Value::Fun(FunValue::new_native(
            "input", vec!["prompt".into()], |vals| {
                let prompt = vals.first().map(|v| v.to_string()).unwrap_or_default();
                if !prompt.is_empty() { print!("{}", prompt); use std::io::Write; std::io::stdout().flush().ok(); }
                let mut line = String::new();
                std::io::stdin().read_line(&mut line).ok();
                Ok(Value::String(line.trim_end().to_string()))
            },
        )));

        Self { globals, args }
    }

    pub fn empty() -> Self {
        Self { globals: HashMap::new(), args: vec![] }
    }

    /// Agrega un valor global (función, constante, etc.)
    pub fn add(&mut self, name: &str, value: Value) -> &mut Self {
        self.globals.insert(name.into(), value);
        self
    }
}

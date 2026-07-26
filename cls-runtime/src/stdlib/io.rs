use crate::environment::Environment;
use crate::value::{FunValue, Value};
use cls_core::error::{ClsError, ClsResult};

pub fn register(env: &mut Environment) {
    let print_fn = FunValue::new_native(
        "print",
        vec!["value".to_string()],
        |args: &[Value]| -> ClsResult<Value> {
            if args.is_empty() {
                println!();
            } else {
                let output = args
                    .iter()
                    .map(|a: &Value| a.to_string())
                    .collect::<Vec<_>>()
                    .join(" ");
                println!("{}", output);
            }
            Ok(Value::Void)
        },
    );
    env.define("print", Value::Fun(print_fn));

    let input_fn = FunValue::new_native(
        "input",
        vec!["prompt".to_string()],
        |args: &[Value]| -> ClsResult<Value> {
            let prompt = args
                .first()
                .map(|a: &Value| a.to_string())
                .unwrap_or_default();
            if !prompt.is_empty() {
                print!("{}", prompt);
                use std::io::Write;
                std::io::stdout().flush().ok();
            }
            let mut line = String::new();
            std::io::stdin()
                .read_line(&mut line)
                .map_err(|e| ClsError::RuntimeError(format!("Error de entrada: {}", e)))?;
            Ok(Value::String(line.trim_end().to_string()))
        },
    );
    env.define("input", Value::Fun(input_fn));
}

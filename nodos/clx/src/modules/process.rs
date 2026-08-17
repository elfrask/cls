use cls_runtime::value::{FunValue, Value};
use cls_core::error::ClsError;
use std::collections::HashMap;

fn str_arg(a: &[Value], i: usize, fn_name: &str) -> Result<String, ClsError> {
    match a.get(i) {
        Some(Value::String(s)) => Ok(s.clone()),
        _ => Err(ClsError::RuntimeError(format!("{}: esperaba String", fn_name))),
    }
}

pub fn module(app_args: Vec<String>) -> Value {
    let mut m = HashMap::new();
    m.insert("args".into(), Value::Fun(FunValue::new_native("args", vec![], {
        let app_args = app_args.clone();
        move |_| Ok(Value::Array(app_args.iter().map(|s| Value::String(s.clone())).collect()))
    })));
    m.insert("cwd".into(), Value::Fun(FunValue::new_native("cwd", vec![], |_| {
        std::env::current_dir()
            .map(|d| Value::String(d.to_string_lossy().to_string()))
            .map_err(|e| ClsError::RuntimeError(format!("process.cwd: {}", e)))
    })));
    m.insert("env".into(), Value::Fun(FunValue::new_native("env", vec!["key".into()], |a| {
        let k = str_arg(a, 0, "process.env")?;
        Ok(Value::String(std::env::var(&k).unwrap_or_default()))
    })));
    m.insert("exit".into(), Value::Fun(FunValue::new_native("exit", vec!["code".into()], |a| {
        let code = match a.first() {
            Some(Value::Int(i)) => *i as i32,
            _ => 0,
        };
        std::process::exit(code);
    })));
    m.insert("pid".into(), Value::Fun(FunValue::new_native("pid", vec![], |_| {
        Ok(Value::Int(std::process::id() as i64))
    })));
    m.insert("platform".into(), Value::Fun(FunValue::new_native("platform", vec![], |_| {
        Ok(Value::String(std::env::consts::OS.to_string()))
    })));
    m.insert("title".into(), Value::Fun(FunValue::new_native("title", vec![], |_| {
        // Sin crates: título del proceso no portable.
        Ok(Value::String(String::new()))
    })));
    Value::Record(m)
}

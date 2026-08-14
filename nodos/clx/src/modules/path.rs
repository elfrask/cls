use cls_runtime::value::{FunValue, Value};
use cls_core::error::ClsError;
use std::collections::HashMap;

fn str_arg(a: &[Value], i: usize, fn_name: &str) -> Result<String, ClsError> {
    match a.get(i) {
        Some(Value::String(s)) => Ok(s.clone()),
        _ => Err(ClsError::RuntimeError(format!("{}: esperaba String", fn_name))),
    }
}

/// Normaliza `..`/`.` sin tocar el FS (pila de componentes). Acepta `/` y `\`.
fn normalize_path(s: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for comp in s.split(['/', '\\']) {
        match comp {
            "" | "." => {}
            ".." => {
                if let Some(last) = parts.last() {
                    if *last != ".." {
                        parts.pop();
                        continue;
                    }
                }
                parts.push(comp);
            }
            other => parts.push(other),
        }
    }
    let mut out = String::new();
    if s.starts_with('/') || s.starts_with('\\') {
        out.push(std::path::MAIN_SEPARATOR);
    }
    out.push_str(&parts.join(&std::path::MAIN_SEPARATOR.to_string()));
    if out.is_empty() {
        out.push('.');
    }
    out
}

pub fn module() -> Value {
    let mut m = HashMap::new();
    m.insert("join".into(), Value::Fun(FunValue::new_native("join", vec!["a".into(), "b".into()], |a| {
        let sa = str_arg(a, 0, "path.join")?;
        let sb = str_arg(a, 1, "path.join")?;
        Ok(Value::String(std::path::Path::new(&sa).join(&sb).to_string_lossy().to_string()))
    })));
    m.insert("basename".into(), Value::Fun(FunValue::new_native("basename", vec!["path".into()], |a| {
        let p = str_arg(a, 0, "path.basename")?;
        let base = std::path::Path::new(&p).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        Ok(Value::String(base))
    })));
    m.insert("dirname".into(), Value::Fun(FunValue::new_native("dirname", vec!["path".into()], |a| {
        let p = str_arg(a, 0, "path.dirname")?;
        let dir = std::path::Path::new(&p).parent().map(|d| d.to_string_lossy().to_string()).unwrap_or_else(|| ".".to_string());
        Ok(Value::String(dir))
    })));
    m.insert("extname".into(), Value::Fun(FunValue::new_native("extname", vec!["path".into()], |a| {
        let p = str_arg(a, 0, "path.extname")?;
        let ext = std::path::Path::new(&p).extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
        Ok(Value::String(ext))
    })));
    m.insert("resolve".into(), Value::Fun(FunValue::new_native("resolve", vec!["path".into()], |a| {
        let p = str_arg(a, 0, "path.resolve")?;
        let path = std::path::Path::new(&p);
        let resolved = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir().map(|cwd| cwd.join(path)).unwrap_or_else(|_| path.to_path_buf())
        };
        Ok(Value::String(resolved.to_string_lossy().to_string()))
    })));
    m.insert("normalize".into(), Value::Fun(FunValue::new_native("normalize", vec!["path".into()], |a| {
        let p = str_arg(a, 0, "path.normalize")?;
        Ok(Value::String(normalize_path(&p)))
    })));
    m.insert("isAbsolute".into(), Value::Fun(FunValue::new_native("isAbsolute", vec!["path".into()], |a| {
        let p = str_arg(a, 0, "path.isAbsolute")?;
        Ok(Value::Bool(std::path::Path::new(&p).is_absolute()))
    })));
    m.insert("sep".into(), Value::Fun(FunValue::new_native("sep", vec![], |_| {
        Ok(Value::String(std::path::MAIN_SEPARATOR_STR.to_string()))
    })));
    Value::Record(m)
}

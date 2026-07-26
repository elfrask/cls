use cls_runtime::value::{FunValue, Value};
use cls_core::error::{ClsError, ClsResult};
use std::collections::HashMap;
use std::fs;

pub fn module() -> Value {
    let mut m = HashMap::new();

    m.insert("readFile".into(), Value::Fun(FunValue::new_native("readFile", vec!["path".into()], |a| {
        let path = match a.first() { Some(Value::String(s)) => s.clone(), _ => return Err(ClsError::RuntimeError("fs.readFile: esperaba path".into())) };
        fs::read_to_string(&path).map(Value::String).map_err(|e| ClsError::RuntimeError(format!("fs.readFile: {}", e)))
    })));

    m.insert("writeFile".into(), Value::Fun(FunValue::new_native("writeFile", vec!["path".into(), "content".into()], |a| {
        if a.len() < 2 { return Err(ClsError::RuntimeError("fs.writeFile: esperaba 2 args".into())); }
        let path = match &a[0] { Value::String(s) => s.clone(), _ => return Err(ClsError::RuntimeError("fs.writeFile: path debe ser String".into())) };
        let content = a[1].to_string();
        fs::write(&path, &content).map(|_| Value::Void).map_err(|e| ClsError::RuntimeError(format!("fs.writeFile: {}", e)))
    })));

    m.insert("exists".into(), Value::Fun(FunValue::new_native("exists", vec!["path".into()], |a| {
        let path = match a.first() { Some(Value::String(s)) => s.clone(), _ => return Ok(Value::Bool(false)) };
        Ok(Value::Bool(std::path::Path::new(&path).exists()))
    })));

    m.insert("rm".into(), Value::Fun(FunValue::new_native("rm", vec!["path".into()], |a| {
        let path = match a.first() { Some(Value::String(s)) => s.clone(), _ => return Err(ClsError::RuntimeError("fs.rm: esperaba path".into())) };
        let p = std::path::Path::new(&path);
        (if p.is_dir() { fs::remove_dir_all(&path) } else { fs::remove_file(&path) })
            .map(|_| Value::Void).map_err(|e| ClsError::RuntimeError(format!("fs.rm: {}", e)))
    })));

    m.insert("mkdir".into(), Value::Fun(FunValue::new_native("mkdir", vec!["path".into()], |a| {
        let path = match a.first() { Some(Value::String(s)) => s.clone(), _ => return Err(ClsError::RuntimeError("fs.mkdir: esperaba path".into())) };
        fs::create_dir_all(&path).map(|_| Value::Void).map_err(|e| ClsError::RuntimeError(format!("fs.mkdir: {}", e)))
    })));

    m.insert("listDir".into(), Value::Fun(FunValue::new_native("listDir", vec!["path".into()], |a| {
        let path = match a.first() { Some(Value::String(s)) => s.clone(), Some(v) => v.to_string(), None => ".".into() };
        let entries: Vec<Value> = fs::read_dir(&path)
            .map_err(|e| ClsError::RuntimeError(format!("fs.listDir: {}", e)))?
            .filter_map(|e| e.ok()).map(|e| Value::String(e.file_name().to_string_lossy().to_string())).collect();
        Ok(Value::Array(entries))
    })));

    m.insert("cwd".into(), Value::Fun(FunValue::new_native("cwd", vec![], |_| {
        std::env::current_dir().map(|p| Value::String(p.to_string_lossy().to_string())).map_err(|e| ClsError::RuntimeError(format!("fs.cwd: {}", e)))
    })));

    Value::Record(m)
}

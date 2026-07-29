use cls_runtime::value::{FunValue, Value};
use cls_runtime::VfsResolver;
use cls_core::error::{ClsError, ClsResult};
use std::collections::HashMap;
use std::sync::Arc;

fn extract_path(args: &[Value], fn_name: &str) -> ClsResult<String> {
    match args.first() {
        Some(Value::String(s)) => Ok(s.clone()),
        _ => Err(ClsError::RuntimeError(format!("{}: esperaba path como String", fn_name)))
    }
}

fn read_file(path: &str, vfs: &VfsResolver) -> ClsResult<String> {
    if path.contains("://") {
        vfs.read_to_string(path)
    } else {
        std::fs::read_to_string(path)
            .map_err(|e| ClsError::RuntimeError(format!("fs.readFile: {}", e)))
    }
}

fn write_file(path: &str, content: &str, vfs: &VfsResolver) -> ClsResult<()> {
    if path.contains("://") {
        vfs.write_file(path, content.as_bytes())
    } else {
        std::fs::write(path, content)
            .map_err(|e| ClsError::RuntimeError(format!("fs.writeFile: {}", e)))
    }
}

fn file_exists(path: &str, vfs: &VfsResolver) -> bool {
    if path.contains("://") {
        vfs.exists(path)
    } else {
        std::path::Path::new(path).exists()
    }
}

fn remove_path(path: &str, vfs: &VfsResolver) -> ClsResult<()> {
    if path.contains("://") {
        vfs.remove(path)
    } else {
        let p = std::path::Path::new(path);
        (if p.is_dir() { std::fs::remove_dir_all(path) } else { std::fs::remove_file(path) })
            .map_err(|e| ClsError::RuntimeError(format!("fs.rm: {}", e)))
    }
}

fn make_dir(path: &str, vfs: &VfsResolver) -> ClsResult<()> {
    if path.contains("://") {
        vfs.create_dir(path)
    } else {
        std::fs::create_dir_all(path)
            .map_err(|e| ClsError::RuntimeError(format!("fs.mkdir: {}", e)))
    }
}

fn list_dir(path: &str, vfs: &VfsResolver) -> ClsResult<Vec<Value>> {
    if path.contains("://") {
        let entries = vfs.list_dir(path)?;
        Ok(entries.into_iter().map(Value::String).collect())
    } else {
        let entries: Vec<Value> = std::fs::read_dir(path)
            .map_err(|e| ClsError::RuntimeError(format!("fs.listDir: {}", e)))?
            .filter_map(|e| e.ok())
            .map(|e| Value::String(e.file_name().to_string_lossy().to_string()))
            .collect();
        Ok(entries)
    }
}

pub fn module(vfs: Arc<VfsResolver>) -> Value {
    let mut m = HashMap::new();

    m.insert("readFile".into(), Value::Fun(FunValue::new_native("readFile", vec!["path".into()], {
        let vfs = vfs.clone();
        move |a| {
            let path = extract_path(a, "fs.readFile")?;
            read_file(&path, &vfs).map(Value::String)
        }
    })));

    m.insert("writeFile".into(), Value::Fun(FunValue::new_native("writeFile", vec!["path".into(), "content".into()], {
        let vfs = vfs.clone();
        move |a| {
            if a.len() < 2 {
                return Err(ClsError::RuntimeError("fs.writeFile: esperaba 2 args".into()));
            }
            let path = match &a[0] { Value::String(s) => s.clone(), _ => return Err(ClsError::RuntimeError("fs.writeFile: path debe ser String".into())) };
            let content = a[1].to_string();
            write_file(&path, &content, &vfs).map(|_| Value::Void)
        }
    })));

    m.insert("exists".into(), Value::Fun(FunValue::new_native("exists", vec!["path".into()], {
        let vfs = vfs.clone();
        move |a| {
            let path = extract_path(a, "fs.exists")?;
            Ok(Value::Bool(file_exists(&path, &vfs)))
        }
    })));

    m.insert("rm".into(), Value::Fun(FunValue::new_native("rm", vec!["path".into()], {
        let vfs = vfs.clone();
        move |a| {
            let path = extract_path(a, "fs.rm")?;
            remove_path(&path, &vfs).map(|_| Value::Void)
        }
    })));

    m.insert("mkdir".into(), Value::Fun(FunValue::new_native("mkdir", vec!["path".into()], {
        let vfs = vfs.clone();
        move |a| {
            let path = extract_path(a, "fs.mkdir")?;
            make_dir(&path, &vfs).map(|_| Value::Void)
        }
    })));

    m.insert("listDir".into(), Value::Fun(FunValue::new_native("listDir", vec!["path".into()], {
        let vfs = vfs.clone();
        move |a| {
            let path = match a.first() {
                Some(Value::String(s)) => s.clone(),
                Some(v) => v.to_string(),
                None => ".".into()
            };
            let entries = list_dir(&path, &vfs)?;
            Ok(Value::Array(entries))
        }
    })));

    m.insert("cwd".into(), Value::Fun(FunValue::new_native("cwd", vec![], |_| {
        std::env::current_dir()
            .map(|p| Value::String(p.to_string_lossy().to_string()))
            .map_err(|e| ClsError::RuntimeError(format!("fs.cwd: {}", e)))
    })));

    Value::Record(m)
}

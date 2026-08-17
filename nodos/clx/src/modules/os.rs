use cls_runtime::value::{FunValue, Value};
use cls_core::error::ClsError;
use std::collections::HashMap;

fn str_arg(a: &[Value], i: usize, fn_name: &str) -> Result<String, ClsError> {
    match a.get(i) {
        Some(Value::String(s)) => Ok(s.clone()),
        _ => Err(ClsError::RuntimeError(format!("{}: esperaba String", fn_name))),
    }
}

pub fn module() -> Value {
    let mut m = HashMap::new();
    m.insert("platform".into(), Value::Fun(FunValue::new_native("platform", vec![], |_| {
        Ok(Value::String(std::env::consts::OS.to_string()))
    })));
    m.insert("arch".into(), Value::Fun(FunValue::new_native("arch", vec![], |_| {
        Ok(Value::String(std::env::consts::ARCH.to_string()))
    })));
    m.insert("version".into(), Value::Fun(FunValue::new_native("version", vec![], |_| {
        #[cfg(windows)]
        let v = std::env::var("OS").unwrap_or_default();
        #[cfg(not(windows))]
        let v = std::env::consts::OS.to_string();
        Ok(Value::String(v))
    })));
    m.insert("hostname".into(), Value::Fun(FunValue::new_native("hostname", vec![], |_| {
        let name = std::env::var("COMPUTERNAME")
            .or_else(|_| std::env::var("HOSTNAME"))
            .unwrap_or_default();
        Ok(Value::String(name))
    })));
    m.insert("home".into(), Value::Fun(FunValue::new_native("home", vec![], |_| {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_default();
        Ok(Value::String(home))
    })));
    m.insert("tempdir".into(), Value::Fun(FunValue::new_native("tempdir", vec![], |_| {
        Ok(Value::String(std::env::temp_dir().to_string_lossy().to_string()))
    })));
    m.insert("cpus".into(), Value::Fun(FunValue::new_native("cpus", vec![], |_| {
        Ok(Value::Int(std::thread::available_parallelism().map(|n| n.get() as i64).unwrap_or(1)))
    })));
    m.insert("pid".into(), Value::Fun(FunValue::new_native("pid", vec![], |_| {
        Ok(Value::Int(std::process::id() as i64))
    })));
    m.insert("uptime".into(), Value::Fun(FunValue::new_native("uptime", vec![], |_| {
        // Sin crates de sysinfo: uptime real no disponible portablemente.
        Ok(Value::Int(0))
    })));
    m.insert("env".into(), Value::Fun(FunValue::new_native("env", vec!["key".into()], |a| {
        let k = str_arg(a, 0, "os.env")?;
        Ok(Value::String(std::env::var(&k).unwrap_or_default()))
    })));
    m.insert("sep".into(), Value::Fun(FunValue::new_native("sep", vec![], |_| {
        Ok(Value::String(std::path::MAIN_SEPARATOR_STR.to_string()))
    })));
    m.insert("isWindows".into(), Value::Fun(FunValue::new_native("isWindows", vec![], |_| {
        Ok(Value::Bool(cfg!(windows)))
    })));
    m.insert("isUnix".into(), Value::Fun(FunValue::new_native("isUnix", vec![], |_| {
        Ok(Value::Bool(!cfg!(windows)))
    })));
    Value::Record(m)
}

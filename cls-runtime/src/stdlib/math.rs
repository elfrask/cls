use crate::value::{FunValue, Value};
use cls_core::error::{ClsError, ClsResult};
use std::collections::HashMap;

fn as_f64(v: &Value) -> ClsResult<f64> {
    match v { Value::Int(i) => Ok(*i as f64), Value::Float(f) => Ok(*f), _ => Err(ClsError::RuntimeError("Esperaba número".into())) }
}

/// Devuelve el módulo math como Value::Record
pub fn module() -> Value {
    let mut m = HashMap::new();

    m.insert("PI".into(), Value::Float(std::f64::consts::PI));
    m.insert("E".into(), Value::Float(std::f64::consts::E));

    m.insert("abs".into(), Value::Fun(FunValue::new_native("abs", vec!["x".into()], |a| {
        let v = a.first().ok_or(ClsError::RuntimeError("abs: esperaba 1 arg".into()))?;
        match v { Value::Int(i) => Ok(Value::Int(i.abs())), Value::Float(f) => Ok(Value::Float(f.abs())), _ => Err(ClsError::RuntimeError("abs: esperaba número".into())) }
    })));

    m.insert("sqrt".into(), Value::Fun(FunValue::new_native("sqrt", vec!["x".into()], |a| {
        let x = as_f64(a.first().ok_or(ClsError::RuntimeError("sqrt: esperaba 1 arg".into()))?)?;
        Ok(Value::Float(x.sqrt()))
    })));

    m.insert("pow".into(), Value::Fun(FunValue::new_native("pow", vec!["base".into(), "exp".into()], |a| {
        if a.len() < 2 { return Err(ClsError::RuntimeError("pow: esperaba 2 args".into())); }
        Ok(Value::Float(as_f64(&a[0])?.powf(as_f64(&a[1])?)))
    })));

    m.insert("min".into(), Value::Fun(FunValue::new_native("min", vec!["a".into(), "b".into()], |a| {
        let x = as_f64(a.first().ok_or(ClsError::RuntimeError("min: esperaba args".into()))?)?;
        let y = as_f64(a.get(1).ok_or(ClsError::RuntimeError("min: esperaba 2 args".into()))?)?;
        Ok(Value::Float(x.min(y)))
    })));

    m.insert("max".into(), Value::Fun(FunValue::new_native("max", vec!["a".into(), "b".into()], |a| {
        let x = as_f64(a.first().ok_or(ClsError::RuntimeError("max: esperaba args".into()))?)?;
        let y = as_f64(a.get(1).ok_or(ClsError::RuntimeError("max: esperaba 2 args".into()))?)?;
        Ok(Value::Float(x.max(y)))
    })));

    m.insert("floor".into(), Value::Fun(FunValue::new_native("floor", vec!["x".into()], |a| {
        Ok(Value::Int(as_f64(a.first().ok_or(ClsError::RuntimeError("floor: esperaba 1 arg".into()))?)? .floor() as i64))
    })));

    m.insert("ceil".into(), Value::Fun(FunValue::new_native("ceil", vec!["x".into()], |a| {
        Ok(Value::Int(as_f64(a.first().ok_or(ClsError::RuntimeError("ceil: esperaba 1 arg".into()))?)? .ceil() as i64))
    })));

    m.insert("round".into(), Value::Fun(FunValue::new_native("round", vec!["x".into()], |a| {
        Ok(Value::Int(as_f64(a.first().ok_or(ClsError::RuntimeError("round: esperaba 1 arg".into()))?)? .round() as i64))
    })));

    m.insert("random".into(), Value::Fun(FunValue::new_native("random", vec![], |_| {
        use std::time::{SystemTime, UNIX_EPOCH};
        let t = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        Ok(Value::Float((t % 1000000) as f64 / 1000000.0))
    })));

    for name in &["sin", "cos", "tan"] {
        let n = *name;
        m.insert(n.into(), Value::Fun(FunValue::new_native(n, vec!["x".into()], move |a| {
            let x = as_f64(a.first().ok_or(ClsError::RuntimeError(format!("{}: esperaba 1 arg", n)))?)?;
            let r = match n { "sin" => x.sin(), "cos" => x.cos(), _ => x.tan() };
            Ok(Value::Float(r))
        })));
    }

    m.insert("log".into(), Value::Fun(FunValue::new_native("log", vec!["x".into()], |a| {
        Ok(Value::Float(as_f64(a.first().ok_or(ClsError::RuntimeError("log: esperaba 1 arg".into()))?)? .ln()))
    })));

    m.insert("range".into(), Value::Fun(FunValue::new_native("range", vec!["start".into(), "end".into()], |a| {
        let start = match a.first() { Some(Value::Int(i)) => *i, _ => return Err(ClsError::RuntimeError("range: esperaba start (Int)".into())) };
        let end = match a.get(1) { Some(Value::Int(i)) => *i, _ => return Err(ClsError::RuntimeError("range: esperaba end (Int)".into())) };
        Ok(Value::Array((start..end).map(Value::Int).collect()))
    })));

    Value::Record(m)
}

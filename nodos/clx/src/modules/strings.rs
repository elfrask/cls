use cls_runtime::value::{FunValue, Value};
use std::collections::HashMap;

pub fn module() -> Value {
    let mut m = HashMap::new();
    m.insert("indexOf".into(), Value::Fun(FunValue::new_native("indexOf", vec!["s".into(), "sub".into()], |a| {
        let s = match a.first() { Some(Value::String(x)) => x.clone(), _ => String::new() };
        let sub = match a.get(1) { Some(Value::String(x)) => x.clone(), _ => String::new() };
        match s.find(&sub) {
            Some(idx) => Ok(Value::Int(idx as i64)),
            None => Ok(Value::Int(-1)),
        }
    })));
    m.insert("slice".into(), Value::Fun(FunValue::new_native("slice", vec!["s".into(), "start".into(), "end".into()], |a| {
        let s = match a.first() { Some(Value::String(x)) => x.clone(), _ => String::new() };
        let start = match a.get(1) { Some(Value::Int(i)) => *i, _ => 0 };
        let end = match a.get(2) { Some(Value::Int(i)) => *i, _ => -1 };
        let len = s.len() as i64;
        let start = start.max(0).min(len);
        let end = if end < 0 { len } else { end.max(start).min(len) };
        Ok(Value::String(s[start as usize..end as usize].to_string()))
    })));
    m.insert("split".into(), Value::Fun(FunValue::new_native("split", vec!["s".into(), "sep".into()], |a| {
        let s = match a.first() { Some(Value::String(x)) => x.clone(), _ => String::new() };
        let sep = match a.get(1) { Some(Value::String(x)) => x.clone(), _ => String::new() };
        let parts: Vec<Value> = if sep.is_empty() {
            vec![Value::String(s)]
        } else {
            s.split(&sep).map(|p| Value::String(p.to_string())).collect()
        };
        Ok(Value::Array(parts))
    })));
    Value::Record(m)
}

use crate::value::{FunValue, Value};
use cls_core::error::{ClsError, ClsResult};
use std::collections::HashMap;

pub fn module() -> Value {
    let mut m = HashMap::new();

    m.insert("get".into(), Value::Fun(FunValue::new_native("get", vec!["url".into()], |a| {
        let url = match a.first() { Some(Value::String(s)) => s.clone(), _ => return Err(ClsError::RuntimeError("http.get: esperaba URL".into())) };
        let body = ureq::get(&url).call().map_err(|e| ClsError::RuntimeError(format!("http.get: {}", e)))?.into_string().map_err(|e| ClsError::RuntimeError(format!("http.get: {}", e)))?;
        Ok(Value::String(body))
    })));

    m.insert("post".into(), Value::Fun(FunValue::new_native("post", vec!["url".into(), "body".into()], |a| {
        if a.len() < 2 { return Err(ClsError::RuntimeError("http.post: esperaba 2 args".into())); }
        let url = match &a[0] { Value::String(s) => s.clone(), _ => return Err(ClsError::RuntimeError("http.post: url debe ser String".into())) };
        let body_content = a[1].to_string();
        let body = ureq::post(&url).send_string(&body_content).map_err(|e| ClsError::RuntimeError(format!("http.post: {}", e)))?.into_string().map_err(|e| ClsError::RuntimeError(format!("http.post: {}", e)))?;
        Ok(Value::String(body))
    })));

    Value::Record(m)
}

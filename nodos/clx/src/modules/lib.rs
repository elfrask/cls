use cls_runtime::ClsLibResolver;
use cls_runtime::value::{FunValue, Value};
use cls_core::error::ClsError;
use std::collections::HashMap;
use std::sync::Arc;

pub fn module(resolver: Arc<dyn ClsLibResolver>) -> Value {
    let mut m = HashMap::new();

    m.insert("load".into(), Value::Fun(FunValue::new_native("load", vec!["name".into()], {
        let resolver = resolver.clone();
        move |a| {
            let name = match a.first() {
                Some(Value::String(s)) => s.clone(),
                _ => return Err(ClsError::RuntimeError("Lib.load: esperaba name como String".into()))
            };
            let bytes = resolver
                .resolve(&name)?
                .ok_or_else(|| ClsError::RuntimeError(format!("Lib.load: '{}' no encontrado", name)))?;

            match String::from_utf8(bytes) {
                Ok(text) => Ok(Value::String(text)),
                Err(_) => Ok(Value::String(format!("<binary {} bytes>", name)))
            }
        }
    })));

    Value::Record(m)
}

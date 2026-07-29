use crate::value::{FunValue, Value};
use cls_core::error::ClsError;
use std::collections::HashMap;

pub fn module() -> Value {
    let mut m = HashMap::new();

    m.insert("parse".into(), Value::Fun(FunValue::new_native("parse", vec!["text".into()], |a| {
        let text = match a.first() { Some(Value::String(s)) => s.clone(), _ => return Err(ClsError::RuntimeError("json.parse: esperaba String".into())) };
        let v: serde_json::Value = serde_json::from_str(&text).map_err(|e| ClsError::RuntimeError(format!("json.parse: {}", e)))?;
        Ok(json_to_value(&v))
    })));

    m.insert("stringify".into(), Value::Fun(FunValue::new_native("stringify", vec!["value".into()], |a| {
        let v = a.first().unwrap_or(&Value::Null);
        serde_json::to_string(&value_to_json(v)).map(Value::String).map_err(|e| ClsError::RuntimeError(format!("json.stringify: {}", e)))
    })));

    Value::Record(m)
}

fn json_to_value(v: &serde_json::Value) -> Value {
    match v {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Bool(*b),
        serde_json::Value::Number(n) => if let Some(i) = n.as_i64() { Value::Int(i) } else { Value::Float(n.as_f64().unwrap_or(0.0)) },
        serde_json::Value::String(s) => Value::String(s.clone()),
        serde_json::Value::Array(arr) => Value::Array(arr.iter().map(json_to_value).collect()),
        serde_json::Value::Object(obj) => Value::Record(obj.iter().map(|(k, v)| (k.clone(), json_to_value(v))).collect()),
    }
}

fn value_to_json(v: &Value) -> serde_json::Value {
    match v {
        Value::Null => serde_json::Value::Null,
        Value::Bool(b) => serde_json::Value::Bool(*b),
        Value::Int(i) => serde_json::Value::Number(serde_json::Number::from(*i)),
        Value::Float(f) => serde_json::value::Number::from_f64(*f).map(serde_json::Value::Number).unwrap_or(serde_json::Value::Null),
        Value::String(s) => serde_json::Value::String(s.clone()),
        Value::Array(arr) => serde_json::Value::Array(arr.iter().map(value_to_json).collect()),
        Value::Record(rec) => serde_json::Value::Object(rec.iter().map(|(k, v)| (k.clone(), value_to_json(v))).collect()),
        _ => serde_json::Value::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::FunKind;
    use cls_core::error::ClsResult;

    fn get_fn(name: &str) -> std::sync::Arc<dyn Fn(&[Value]) -> ClsResult<Value>> {
        let module = module();
        match &module {
            Value::Record(m) => match m.get(name) {
                Some(Value::Fun(f)) => match &f.kind {
                    FunKind::Native { func, .. } => func.clone(),
                    _ => panic!("not native"),
                },
                _ => panic!("fn '{}' not found", name),
            },
            _ => panic!("not a module"),
        }
    }

    #[test]
    fn test_parse_null() {
        let r = get_fn("parse")(&[Value::String("null".into())]).unwrap();
        assert_eq!(r, Value::Null);
    }

    #[test]
    fn test_parse_int() {
        let r = get_fn("parse")(&[Value::String("42".into())]).unwrap();
        assert_eq!(r, Value::Int(42));
    }

    #[test]
    fn test_parse_string() {
        let r = get_fn("parse")(&[Value::String(r#""hello""#.into())]).unwrap();
        assert_eq!(r, Value::String("hello".into()));
    }

    #[test]
    fn test_parse_object() {
        let r = get_fn("parse")(&[Value::String(r#"{"a":1,"b":"x"}"#.into())]).unwrap();
        match &r {
            Value::Record(m) => {
                assert_eq!(m.get("a"), Some(&Value::Int(1)));
                assert_eq!(m.get("b"), Some(&Value::String("x".into())));
            }
            _ => panic!("expected Record"),
        }
    }

    #[test]
    fn test_stringify() {
        let r = get_fn("stringify")(&[Value::Record({
            let mut m = HashMap::new();
            m.insert("x".into(), Value::Int(10));
            m
        })]).unwrap();
        assert_eq!(r, Value::String(r#"{"x":10}"#.into()));
    }

    #[test]
    fn test_parse_error() {
        assert!(get_fn("parse")(&[Value::String("invalid".into())]).is_err());
    }
}

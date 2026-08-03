//! Métodos de tipos primitivos vía dispatch tables estáticas.
//!
//! Los primitivos (String, Int, Array, etc.) NO se envuelven en objetos.
//! Los métodos se resuelven por una tabla global `tipo → {nombre → PrimitiveMethod}`.
//! El receiver viaja como `args[0]` (plano, sin boxing).
//!
//! Esto es compatible con compilación nativa/WASM: el tipo del receiver se
//! conoce en compile-time, por lo que el futuro compilador puede devolver la
//! dirección directa del método (monomorfización).

use std::collections::HashMap;
use std::sync::Arc;
use cls_core::error::{ClsError, ClsResult};
use crate::value::Value;

pub type MethodFn = Arc<dyn Fn(&[Value]) -> ClsResult<Value>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrimitiveType {
    String,
    Int,
    Float,
    Bool,
    Char,
    Array,
    Tuple,
    Record,
}

/// Método o propiedad computada de un tipo primitivo.
pub enum PrimitiveMethod {
    /// Método invocable: `"hola".upper()` → fn([recv, ...args])
    Method(MethodFn),
    /// Propiedad computada: `"hola".length` → fn([recv])
    Getter(MethodFn),
}

type Table = HashMap<&'static str, PrimitiveMethod>;

fn method(f: fn(&[Value]) -> ClsResult<Value>) -> PrimitiveMethod {
    PrimitiveMethod::Method(Arc::new(f))
}

fn getter(f: fn(&[Value]) -> ClsResult<Value>) -> PrimitiveMethod {
    PrimitiveMethod::Getter(Arc::new(f))
}

/// Construye todas las tablas de métodos por tipo primitivo.
pub fn build_method_tables() -> HashMap<PrimitiveType, Table> {
    let mut tables = HashMap::new();
    tables.insert(PrimitiveType::String, string_table());
    tables.insert(PrimitiveType::Array, array_table());
    tables.insert(PrimitiveType::Tuple, tuple_table());
    tables.insert(PrimitiveType::Int, number_table());
    tables.insert(PrimitiveType::Float, number_table());
    tables.insert(PrimitiveType::Bool, bool_table());
    tables.insert(PrimitiveType::Char, char_table());
    tables.insert(PrimitiveType::Record, record_table());
    tables
}

/// Convierte un `Value` al tipo esperado, o error de tipo.
fn expect_string(args: &[Value]) -> ClsResult<&str> {
    match args.first().unwrap_or(&Value::Null) {
        Value::String(s) => Ok(s),
        other => Err(ClsError::RuntimeError(format!(
            "Método de String invocado sobre {}", other.type_name()
        ))),
    }
}

fn expect_array(args: &[Value]) -> ClsResult<&Vec<Value>> {
    match args.first().unwrap_or(&Value::Null) {
        Value::Array(a) => Ok(a),
        other => Err(ClsError::RuntimeError(format!(
            "Método de Array invocado sobre {}", other.type_name()
        ))),
    }
}

fn expect_tuple(args: &[Value]) -> ClsResult<&Vec<Value>> {
    match args.first().unwrap_or(&Value::Null) {
        Value::Tuple(t) => Ok(t),
        other => Err(ClsError::RuntimeError(format!(
            "Método de Tuple invocado sobre {}", other.type_name()
        ))),
    }
}

// ---------------------------------------------------------------- String

fn string_table() -> Table {
    let mut t = Table::new();
    t.insert("upper", method(|a| {
        Ok(Value::String(expect_string(a)?.to_uppercase()))
    }));
    t.insert("lower", method(|a| {
        Ok(Value::String(expect_string(a)?.to_lowercase()))
    }));
    t.insert("trim", method(|a| {
        Ok(Value::String(expect_string(a)?.trim().to_string()))
    }));
    t.insert("contains", method(|a| {
        let needle = match a.get(1) { Some(Value::String(s)) => s.clone(), _ => return Err(ClsError::RuntimeError("contains: esperaba String".into())) };
        Ok(Value::Bool(expect_string(a)?.contains(&needle)))
    }));
    t.insert("startsWith", method(|a| {
        let needle = match a.get(1) { Some(Value::String(s)) => s.clone(), _ => return Err(ClsError::RuntimeError("startsWith: esperaba String".into())) };
        Ok(Value::Bool(expect_string(a)?.starts_with(&needle)))
    }));
    t.insert("endsWith", method(|a| {
        let needle = match a.get(1) { Some(Value::String(s)) => s.clone(), _ => return Err(ClsError::RuntimeError("endsWith: esperaba String".into())) };
        Ok(Value::Bool(expect_string(a)?.ends_with(&needle)))
    }));
    t.insert("length", getter(|a| {
        Ok(Value::Int(expect_string(a)?.chars().count() as i64))
    }));
    t.insert("isEmpty", method(|a| {
        Ok(Value::Bool(expect_string(a)?.is_empty()))
    }));
    t.insert("toString", method(|a| Ok(a.first().cloned().unwrap_or(Value::Null))));
    t
}

// ----------------------------------------------------------------- Array

fn array_table() -> Table {
    let mut t = Table::new();
    t.insert("length", getter(|a| {
        Ok(Value::Int(expect_array(a)?.len() as i64))
    }));
    // Mutadores: reciben el array, lo modifican y devuelven el array mutado.
    // evaluate_call hace write-back de la variable cuando el resultado es Array.
    t.insert("push", method(|a| {
        let value = match a.get(1).cloned() { Some(v) => v, None => return Err(ClsError::RuntimeError("push: falta el elemento".into())) };
        let mut arr = expect_array(a)?.clone();
        arr.push(value);
        Ok(Value::Array(arr))
    }));
    t.insert("pop", method(|a| {
        let mut arr = expect_array(a)?.clone();
        arr.pop();
        Ok(Value::Array(arr))
    }));
    t.insert("shift", method(|a| {
        let mut arr = expect_array(a)?.clone();
        if !arr.is_empty() { arr.remove(0); }
        Ok(Value::Array(arr))
    }));
    t.insert("unshift", method(|a| {
        let value = match a.get(1).cloned() { Some(v) => v, None => return Err(ClsError::RuntimeError("unshift: falta el elemento".into())) };
        let mut arr = expect_array(a)?.clone();
        arr.insert(0, value);
        Ok(Value::Array(arr))
    }));
    t.insert("indexOf", method(|a| {
        let target = match a.get(1) { Some(v) => v, None => return Err(ClsError::RuntimeError("indexOf: falta el elemento".into())) };
        let idx = expect_array(a)?.iter().position(|x| x == target).map(|i| i as i64).unwrap_or(-1);
        Ok(Value::Int(idx))
    }));
    t.insert("includes", method(|a| {
        let target = match a.get(1) { Some(v) => v, None => return Err(ClsError::RuntimeError("includes: falta el elemento".into())) };
        Ok(Value::Bool(expect_array(a)?.iter().any(|x| x == target)))
    }));
    t.insert("join", method(|a| {
        let sep = match a.get(1) { Some(Value::String(s)) => s.clone(), _ => ",".to_string() };
        let items: Vec<String> = expect_array(a)?.iter().map(|x| x.to_string()).collect();
        Ok(Value::String(items.join(&sep)))
    }));
    t.insert("reverse", method(|a| {
        let mut arr = expect_array(a)?.clone();
        arr.reverse();
        Ok(Value::Array(arr))
    }));
    t.insert("toString", method(|a| Ok(a.first().cloned().unwrap_or(Value::Null))));
    t
}

// ----------------------------------------------------------------- Tuple

fn tuple_table() -> Table {
    let mut t = Table::new();
    t.insert("length", getter(|a| {
        Ok(Value::Int(expect_tuple(a)?.len() as i64))
    }));
    t.insert("join", method(|a| {
        let sep = match a.get(1) { Some(Value::String(s)) => s.clone(), _ => ",".to_string() };
        let items: Vec<String> = expect_tuple(a)?.iter().map(|x| x.to_string()).collect();
        Ok(Value::String(items.join(&sep)))
    }));
    t.insert("toString", method(|a| Ok(a.first().cloned().unwrap_or(Value::Null))));
    t
}

// ------------------------------------------------------------- Number

fn number_table() -> Table {
    let mut t = Table::new();
    t.insert("toString", method(|a| {
        match a.first().unwrap_or(&Value::Null) {
            Value::Int(i) => Ok(Value::String(i.to_string())),
            Value::Float(f) => Ok(Value::String(f.to_string())),
            other => Err(ClsError::RuntimeError(format!("toString numérico sobre {}", other.type_name()))),
        }
    }));
    t.insert("abs", method(|a| {
        match a.first().unwrap_or(&Value::Null) {
            Value::Int(i) => Ok(Value::Int(i.abs())),
            Value::Float(f) => Ok(Value::Float(f.abs())),
            other => Err(ClsError::RuntimeError(format!("abs sobre {}", other.type_name()))),
        }
    }));
    t
}

// ----------------------------------------------------------------- Bool

fn bool_table() -> Table {
    let mut t = Table::new();
    t.insert("toString", method(|a| {
        match a.first().unwrap_or(&Value::Null) {
            Value::Bool(b) => Ok(Value::String(b.to_string())),
            other => Err(ClsError::RuntimeError(format!("toString bool sobre {}", other.type_name()))),
        }
    }));
    t
}

// ----------------------------------------------------------------- Char

fn char_table() -> Table {
    let mut t = Table::new();
    t.insert("toString", method(|a| {
        match a.first().unwrap_or(&Value::Null) {
            Value::Char(c) => Ok(Value::String(c.to_string())),
            other => Err(ClsError::RuntimeError(format!("toString char sobre {}", other.type_name()))),
        }
    }));
    t
}

// --------------------------------------------------------------- Record

fn record_table() -> Table {
    let mut t = Table::new();
    t.insert("length", getter(|a| {
        match a.first().unwrap_or(&Value::Null) {
            Value::Record(r) => Ok(Value::Int(r.len() as i64)),
            other => Err(ClsError::RuntimeError(format!("length sobre {}", other.type_name()))),
        }
    }));
    t.insert("size", getter(|a| {
        match a.first().unwrap_or(&Value::Null) {
            Value::Record(r) => Ok(Value::Int(r.len() as i64)),
            other => Err(ClsError::RuntimeError(format!("size sobre {}", other.type_name()))),
        }
    }));
    t.insert("keys", method(|a| {
        match a.first().unwrap_or(&Value::Null) {
            Value::Record(r) => {
                let mut keys: Vec<String> = r.keys().cloned().collect();
                keys.sort();
                Ok(Value::Array(keys.into_iter().map(Value::String).collect()))
            }
            other => Err(ClsError::RuntimeError(format!("keys sobre {}", other.type_name()))),
        }
    }));
    t.insert("values", method(|a| {
        match a.first().unwrap_or(&Value::Null) {
            Value::Record(r) => {
                let mut entries: Vec<(String, Value)> = r.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
                entries.sort_by(|a, b| a.0.cmp(&b.0));
                Ok(Value::Array(entries.into_iter().map(|(_, v)| v).collect()))
            }
            other => Err(ClsError::RuntimeError(format!("values sobre {}", other.type_name()))),
        }
    }));
    t.insert("has", method(|a| {
        match a.first().unwrap_or(&Value::Null) {
            Value::Record(r) => {
                let key = match a.get(1) { Some(Value::String(k)) => k.clone(), _ => return Err(ClsError::RuntimeError("has: esperaba String".into())) };
                Ok(Value::Bool(r.contains_key(&key)))
            }
            other => Err(ClsError::RuntimeError(format!("has sobre {}", other.type_name()))),
        }
    }));
    t.insert("toString", method(|a| Ok(a.first().cloned().unwrap_or(Value::Null))));
    t
}

/// Mapea un `Value` a su tipo primitivo para lookup en la tabla.
pub fn primitive_type_of(v: &Value) -> Option<PrimitiveType> {
    match v {
        Value::String(_) => Some(PrimitiveType::String),
        Value::Int(_) => Some(PrimitiveType::Int),
        Value::Float(_) => Some(PrimitiveType::Float),
        Value::Bool(_) => Some(PrimitiveType::Bool),
        Value::Char(_) => Some(PrimitiveType::Char),
        Value::Array(_) => Some(PrimitiveType::Array),
        Value::Tuple(_) => Some(PrimitiveType::Tuple),
        Value::Record(_) => Some(PrimitiveType::Record),
        _ => None,
    }
}

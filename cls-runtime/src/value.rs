use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex};
use cls_core::error::ClsResult;
use cls_core::frontend::ast::Block;

/// Estado de un poll de corrutina/promise
#[derive(Debug, Clone, PartialEq)]
pub enum PollState {
    Pending,
    Ready(Value),
    Rejected(String),
}

/// Contrato para corrutinas. Tanto el intérprete (scheduler de clxr)
/// como los state machines WASM (futuro) implementan esto.
/// Corre en un solo thread (el scheduler de clxr), por eso no requiere Send/Sync.
pub trait Pollable {
    fn poll(&mut self, interp: &mut crate::interpreter::Interpreter) -> PollState;
}

/// Promise — puente entre intérprete y WASM. Compartido vía Arc (como JS).
#[derive(Clone)]
pub struct Promise {
    inner: Arc<Mutex<PromiseInner>>,
}

struct PromiseInner {
    pollable: Option<Box<dyn Pollable>>,
    result: Option<PollState>,
}

impl Promise {
    pub fn new(pollable: Box<dyn Pollable>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(PromiseInner { pollable: Some(pollable), result: None })),
        }
    }

    pub fn resolved(value: Value) -> Self {
        Self {
            inner: Arc::new(Mutex::new(PromiseInner {
                pollable: None,
                result: Some(PollState::Ready(value)),
            })),
        }
    }

    pub fn rejected(msg: String) -> Self {
        Self {
            inner: Arc::new(Mutex::new(PromiseInner {
                pollable: None,
                result: Some(PollState::Rejected(msg)),
            })),
        }
    }

    /// Intenta resolver (poll). Devuelve el estado actual.
    pub fn poll(&mut self, interp: &mut crate::interpreter::Interpreter) -> PollState {
        let mut inner = self.inner.lock().unwrap();
        if let Some(ref result) = inner.result {
            return result.clone();
        }
        if let Some(ref mut pollable) = inner.pollable {
            let state = pollable.poll(interp);
            match &state {
                PollState::Ready(_) | PollState::Rejected(_) => {
                    inner.result = Some(state.clone());
                    inner.pollable = None;
                }
                PollState::Pending => {}
            }
            state
        } else {
            PollState::Pending
        }
    }
}

impl fmt::Debug for Promise {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Promise").finish()
    }
}

impl PartialEq for Promise {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}

/// Definición de un struct (almacenada en el intérprete)
#[derive(Debug, Clone)]
pub struct StructDef {
    pub name: String,
    pub fields: Vec<StructField>,
}

#[derive(Debug, Clone)]
pub struct StructField {
    pub name: String,
}

/// Instancia de un struct en runtime (Vec-based, como C/Rust)
#[derive(Debug, Clone)]
pub struct StructInstance {
    pub def_name: String,
    pub fields: Vec<Value>,
}

impl PartialEq for StructInstance {
    fn eq(&self, other: &Self) -> bool {
        self.def_name == other.def_name && self.fields == other.fields
    }
}

/// Valores runtime de CLS
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    // Primitivos
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Char(char),
    Null,
    Void,

    // Complejos
    Array(Vec<Value>),
    Record(HashMap<String, Value>),
    Fun(FunValue),
    Struct(Box<StructInstance>),
    Promise(Promise),

    // Tipos especiales
    Unknown,

    // CMX
    Cmx(Box<CmxValue>),
}

impl Value {
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Int(_) => "Int",
            Value::Float(_) => "Float",
            Value::String(_) => "String",
            Value::Bool(_) => "Bool",
            Value::Char(_) => "Char",
            Value::Null => "Null",
            Value::Void => "Void",
            Value::Array(_) => "Array",
            Value::Record(_) => "Record",
            Value::Fun(_) => "Fun",
            Value::Struct(_) => "Struct",  // nombre real via to_string()
            Value::Promise(_) => "Promise",
            Value::Unknown => "Unknown",
            Value::Cmx(_) => "Cmx",
        }
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(v) => *v,
            Value::Int(v) => *v != 0,
            Value::Float(v) => *v != 0.0,
            Value::String(v) => !v.is_empty(),
            Value::Null | Value::Void => false,
            Value::Array(v) => !v.is_empty(),
            Value::Record(v) => !v.is_empty(),
            Value::Struct(_) => true,
            Value::Promise(_) => true,
            _ => true,
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            Value::Int(v) => v.to_string(),
            Value::Float(v) => v.to_string(),
            Value::String(v) => v.clone(),
            Value::Bool(v) => v.to_string(),
            Value::Char(v) => v.to_string(),
            Value::Null => "null".to_string(),
            Value::Void => "void".to_string(),
            Value::Array(v) => {
                let items: Vec<String> = v.iter().map(|x| x.to_string()).collect();
                format!("[{}]", items.join(", "))
            }
            Value::Record(v) => {
                let entries: Vec<String> = v
                    .iter()
                    .map(|(k, val)| format!("{}: {}", k, val.to_string()))
                    .collect();
                format!("{{{}}}", entries.join(", "))
            }
            Value::Fun(f) => format!("<function {}>", f.name),
            Value::Struct(s) => {
                let def_name = &s.def_name;
                let fields: Vec<String> = s.fields.iter().map(|v| v.to_string()).collect();
                format!("{}({})", def_name, fields.join(", "))
            }
            Value::Promise(_) => "<promise>".to_string(),
            Value::Unknown => "unknown".to_string(),
            Value::Cmx(cmx) => {
                let props_str = if cmx.props.is_empty() {
                    String::new()
                } else {
                    let entries: Vec<String> = cmx.props.iter()
                        .map(|(k, v)| format!("{}=\"{}\"", k, v.to_string()))
                        .collect();
                    format!(" {}", entries.join(" "))
                };
                let children_str = if cmx.children.is_empty() {
                    " />".to_string()
                } else {
                    format!(">... ({} children)</{}>", cmx.children.len(), cmx.tag)
                };
                format!("<{}{}{}", cmx.tag, props_str, children_str)
            }
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

pub type NativeFn = Box<dyn Fn(&[Value]) -> ClsResult<Value>>;

/// Valor de función (callable)
#[derive(Clone)]
pub struct FunValue {
    pub name: String,
    pub kind: FunKind,
    pub is_async: bool,
}

impl fmt::Debug for FunValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FunValue")
            .field("name", &self.name)
            .field("kind", &self.kind)
            .finish()
    }
}

#[derive(Clone)]
pub enum FunKind {
    /// Función nativa de Rust
    Native {
        params: Vec<String>,
        func: std::sync::Arc<dyn Fn(&[Value]) -> ClsResult<Value>>,
    },
    /// Función definida por el usuario (AST)
    User {
        params: Vec<cls_core::frontend::ast::Parameter>,
        body: Block,
    },
}

impl PartialEq for FunValue {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl PartialEq for FunKind {
    fn eq(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (FunKind::Native { .. }, FunKind::Native { .. })
                | (FunKind::User { .. }, FunKind::User { .. })
        )
    }
}

impl fmt::Debug for FunKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FunKind::Native { params, .. } => f
                .debug_struct("Native")
                .field("params", params)
                .field("func", &"<native>")
                .finish(),
            FunKind::User { params, body } => f
                .debug_struct("User")
                .field("params", params)
                .field("body", body)
                .finish(),
        }
    }
}

impl FunValue {
    pub fn new_native<F>(name: &str, params: Vec<String>, func: F) -> Self
    where
        F: Fn(&[Value]) -> ClsResult<Value> + 'static,
    {
        Self {
            name: name.to_string(),
            is_async: false,
            kind: FunKind::Native {
                params,
                func: std::sync::Arc::new(func),
            },
        }
    }

    pub fn new_user(name: &str, params: Vec<cls_core::frontend::ast::Parameter>, body: Block) -> Self {
        Self {
            name: name.to_string(),
            is_async: false,
            kind: FunKind::User { params, body },
        }
    }

    pub fn new_async_user(name: &str, params: Vec<cls_core::frontend::ast::Parameter>, body: Block) -> Self {
        Self {
            name: name.to_string(),
            is_async: true,
            kind: FunKind::User { params, body },
        }
    }
}

/// Valor CMX (JSX nativo)
#[derive(Debug, Clone, PartialEq)]
pub struct CmxValue {
    pub tag: String,
    pub props: HashMap<String, Value>,
    pub children: Vec<Value>,
}

impl CmxValue {
    pub fn new(tag: String) -> Self {
        Self {
            tag,
            props: HashMap::new(),
            children: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_type_names() {
        assert_eq!(Value::Int(42).type_name(), "Int");
        assert_eq!(Value::Float(3.14).type_name(), "Float");
        assert_eq!(Value::String("hi".into()).type_name(), "String");
        assert_eq!(Value::Bool(true).type_name(), "Bool");
        assert_eq!(Value::Null.type_name(), "Null");
        assert_eq!(Value::Void.type_name(), "Void");
        assert_eq!(Value::Array(vec![]).type_name(), "Array");
        assert_eq!(Value::Record(HashMap::new()).type_name(), "Record");
    }

    #[test]
    fn test_value_truthy() {
        assert!(Value::Bool(true).is_truthy());
        assert!(!Value::Bool(false).is_truthy());
        assert!(Value::Int(5).is_truthy());
        assert!(!Value::Int(0).is_truthy());
        assert!(Value::String("a".into()).is_truthy());
        assert!(!Value::String("".into()).is_truthy());
        assert!(!Value::Null.is_truthy());
        assert!(!Value::Void.is_truthy());
        assert!(Value::Array(vec![Value::Int(1)]).is_truthy());
        assert!(!Value::Array(vec![]).is_truthy());
        assert!(!Value::Record(HashMap::new()).is_truthy());
    }

    #[test]
    fn test_value_display() {
        assert_eq!(Value::Int(42).to_string(), "42");
        assert_eq!(Value::String("hello".into()).to_string(), "hello");
        assert_eq!(Value::Bool(true).to_string(), "true");
        assert_eq!(Value::Null.to_string(), "null");
        assert_eq!(Value::Void.to_string(), "void");
        assert_eq!(Value::Array(vec![Value::Int(1), Value::Int(2)]).to_string(), "[1, 2]");
    }

    #[test]
    fn test_fun_value_native() {
        let f = FunValue::new_native("test", vec!["x".into()], |a| {
            Ok(a.first().cloned().unwrap_or(Value::Null))
        });
        assert_eq!(f.name, "test");
        match &f.kind {
            FunKind::Native { params, .. } => assert_eq!(params[0], "x"),
            _ => panic!("expected Native"),
        }
    }

    #[test]
    fn test_cmx_value_new() {
        let mut cmx = CmxValue::new("App".into());
        assert_eq!(cmx.tag, "App");
        cmx.props.insert("color".into(), Value::String("red".into()));
        assert_eq!(cmx.props.len(), 1);
    }
}

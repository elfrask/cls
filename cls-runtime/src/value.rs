use std::collections::HashMap;
use std::fmt;
use cls_core::error::ClsResult;
use cls_core::frontend::ast::Block;

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
            Value::Null => false,
            Value::Void => false,
            Value::Array(v) => !v.is_empty(),
            Value::Record(v) => !v.is_empty(),
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
            Value::Unknown => "unknown".to_string(),
            Value::Cmx(_) => "<cmx>".to_string(),
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
            kind: FunKind::Native {
                params,
                func: std::sync::Arc::new(func),
            },
        }
    }

    pub fn new_user(name: &str, params: Vec<cls_core::frontend::ast::Parameter>, body: Block) -> Self {
        Self {
            name: name.to_string(),
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

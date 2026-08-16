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
    fn poll(&mut self, interp: &mut crate::walker::interpreter::Interpreter) -> PollState;
}

/// Promise - puente entre intérprete y WASM. Compartido vía Arc (como JS).
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
    pub fn poll(&mut self, interp: &mut crate::walker::interpreter::Interpreter) -> PollState {
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
    /// Nombres de los campos (para la representación `Punto { x: 3, y: 4 }`).
    pub field_names: Vec<String>,
}

impl PartialEq for StructInstance {
    fn eq(&self, other: &Self) -> bool {
        self.def_name == other.def_name && self.fields == other.fields
    }
}

/// Definición de una clase (el valor `Class` es callable: construye instancias)
#[derive(Clone)]
pub struct ClassDef {
    pub name: String,
    pub extends: Option<String>,
    /// Cadena de ancestros: [padre, abuelo, ...] (para `is` y `super`)
    pub ancestors: Vec<String>,
    pub methods: HashMap<String, FunValue>,
    pub field_defaults: HashMap<String, Option<Value>>,
    pub ctor: Option<cls_core::frontend::ast::FunctionDecl>,
    /// Métodos marcados private
    pub private_methods: std::collections::HashSet<String>,
    /// Métodos marcados protected
    pub protected_methods: std::collections::HashSet<String>,
    /// Métodos marcados static
    pub static_methods: std::collections::HashSet<String>,
    /// Fields marcados private
    pub private_fields: std::collections::HashSet<String>,
    /// Fields marcados protected
    pub protected_fields: std::collections::HashSet<String>,
    /// Fields marcados static (viven en el ClassDef, no en la instancia)
    pub static_fields: std::collections::HashSet<String>,
    /// Fields readonly (escritura solo interna)
    pub readonly_fields: std::collections::HashSet<String>,
}

/// Definición de un enum (variantes constantes con identidad)
#[derive(Debug, Clone)]
pub struct EnumDef {
    pub name: String,
    pub variants: Vec<String>,
}

impl PartialEq for EnumDef {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

/// Valor de una variante de enum (índice = los "1-2 bytes" en nativo)
#[derive(Debug, Clone, PartialEq)]
pub struct EnumValue {
    pub def_name: String,
    pub variant: String,
    pub index: u16,
}

/// Instancia de una clase en runtime
#[derive(Clone)]
pub struct ClassInstance {
    pub class_name: String,
    pub fields: HashMap<String, Value>,
    pub methods: HashMap<String, FunValue>,
    /// Fields marcados private (desde ClassDef)
    pub private_fields: std::collections::HashSet<String>,
    /// Métodos marcados private (desde ClassDef)
    pub private_methods: std::collections::HashSet<String>,
    /// Fields marcados protected (desde ClassDef)
    pub protected_fields: std::collections::HashSet<String>,
    /// Métodos marcados protected (desde ClassDef)
    pub protected_methods: std::collections::HashSet<String>,
    /// Métodos marcados static (desde ClassDef)
    pub static_methods: std::collections::HashSet<String>,
    /// Fields readonly (escritura solo interna)
    pub readonly_fields: std::collections::HashSet<String>,
}

impl PartialEq for ClassDef {
    fn eq(&self, other: &Self) -> bool { self.name == other.name }
}

impl PartialEq for ClassInstance {
    fn eq(&self, other: &Self) -> bool {
        self.class_name == other.class_name && self.fields == other.fields
    }
}

impl fmt::Debug for ClassDef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClassDef").field("name", &self.name).finish()
    }
}

impl fmt::Debug for ClassInstance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClassInstance").field("name", &self.class_name).finish()
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
    /// Tupla: array inmutable (no admite push/pop/set)
    Tuple(Vec<Value>),
    Record(HashMap<String, Value>),
    Fun(FunValue),
    Struct(Box<StructInstance>),
    Promise(Promise),
    Class(Box<ClassDef>),
    Object(Box<ClassInstance>),
    EnumDef(Box<EnumDef>),
    Enum(Box<EnumValue>),

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
            Value::Tuple(_) => "Tuple",
            Value::Record(_) => "Record",
            Value::Fun(_) => "Fun",
            Value::Struct(_) => "Struct",  // nombre real via to_string()
            Value::Promise(_) => "Promise",
            Value::Class(_) => "Class",
            Value::Object(_) => "Object",  // nombre real via to_string()
            Value::EnumDef(_) => "Enum",
            Value::Enum(_) => "Enum",
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
            Value::Tuple(v) => !v.is_empty(),
            Value::Record(v) => !v.is_empty(),
            Value::Struct(_) => true,
            Value::Promise(_) => true,
            Value::Class(_) => true,
            Value::Object(_) => true,
            Value::EnumDef(_) => true,
            Value::Enum(_) => true,
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
                let items: Vec<String> = v.iter().map(|x| x.repr()).collect();
                format!("[{}]", items.join(", "))
            }
            Value::Tuple(v) => {
                let items: Vec<String> = v.iter().map(|x| x.repr()).collect();
                format!("({})", items.join(", "))
            }
            Value::Record(v) => {
                let mut keys: Vec<&String> = v.keys().collect();
                keys.sort();
                let entries: Vec<String> = keys
                    .iter()
                    .map(|k| format!("{}: {}", k, v[*k].repr()))
                    .collect();
                format!("{{{}}}", entries.join(", "))
            }
            Value::Fun(f) => format!("<function {}>", f.name),
            Value::Struct(s) => {
                let fields: Vec<String> = s
                    .fields
                    .iter()
                    .enumerate()
                    .map(|(i, v)| {
                        let name = s.field_names.get(i).cloned().unwrap_or_default();
                        format!("{}: {}", name, v.repr())
                    })
                    .collect();
                format!("{} {{ {} }}", s.def_name, fields.join(", "))
            }
            Value::Promise(_) => "<promise>".to_string(),
            Value::Class(c) => format!("<class {}>", c.name),
            Value::Object(o) => {
                let fields: Vec<String> = o.fields.iter()
                    .map(|(k, v)| format!("{}: {}", k, v.repr()))
                    .collect();
                format!("<{} {{{}}}>", o.class_name, fields.join(", "))
            }
            Value::Unknown => "unknown".to_string(),
            Value::EnumDef(e) => format!("<enum {}>", e.name),
            Value::Enum(e) => e.variant.clone(),
            Value::Cmx(cmx) => {
                let tag_str = cmx.tag.to_string();
                let props_str = if cmx.props.is_empty() {
                    String::new()
                } else {
                    let mut keys: Vec<&String> = cmx.props.keys().collect();
                    keys.sort();
                    let entries: Vec<String> = keys.iter()
                        .map(|k| format!("{}=\"{}\"", k, cmx.props[*k].to_string()))
                        .collect();
                    format!(" {}", entries.join(" "))
                };
                let children_str = if cmx.children.is_empty() {
                    " />".to_string()
                } else {
                    format!(">... ({} children)</{}>", cmx.children.len(), tag_str)
                };
                format!("<{}{}{}", tag_str, props_str, children_str)
            }
        }
    }

    /// Representación de impresión (`repr`): los strings van entre comillas dobles
    /// con escapes visibles (`\n`, `\t`, `\\`, `\"`); el resto igual que `to_string`.
    pub fn repr(&self) -> String {
        match self {
            Value::String(v) => {
                let escaped = v
                    .replace('\\', "\\\\")
                    .replace('"', "\\\"")
                    .replace('\n', "\\n")
                    .replace('\t', "\\t");
                format!("\"{}\"", escaped)
            }
            _ => self.to_string(),
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
        /// Entorno léxico capturado (closures). None = usa el env global actual.
        closure: Option<std::sync::Arc<std::sync::Mutex<crate::walker::environment::Environment>>>,
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
            FunKind::User { params, body, .. } => f
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
            kind: FunKind::User { params, body, closure: None },
        }
    }

    pub fn new_user_with_closure(name: &str, params: Vec<cls_core::frontend::ast::Parameter>, body: Block, closure: std::sync::Arc<std::sync::Mutex<crate::walker::environment::Environment>>) -> Self {
        Self {
            name: name.to_string(),
            is_async: false,
            kind: FunKind::User { params, body, closure: Some(closure) },
        }
    }

    pub fn new_async_user(name: &str, params: Vec<cls_core::frontend::ast::Parameter>, body: Block) -> Self {
        Self {
            name: name.to_string(),
            is_async: true,
            kind: FunKind::User { params, body, closure: None },
        }
    }

    pub fn new_async_user_with_closure(name: &str, params: Vec<cls_core::frontend::ast::Parameter>, body: Block, closure: std::sync::Arc<std::sync::Mutex<crate::walker::environment::Environment>>) -> Self {
        Self {
            name: name.to_string(),
            is_async: true,
            kind: FunKind::User { params, body, closure: Some(closure) },
        }
    }
}

/// Valor CMX (JSX nativo)
#[derive(Debug, Clone, PartialEq)]
pub struct CmxValue {
    /// El valor del tag: String para minúsculas, o la referencia (función/var/clase/etc)
    /// para mayúsculas. CMX es agnóstico - no ejecuta la referencia.
    pub tag: Value,
    pub props: HashMap<String, Value>,
    pub children: Vec<Value>,
}

impl CmxValue {
    pub fn new(tag: String) -> Self {
        Self {
            tag: Value::String(tag),
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
        assert_eq!(cmx.tag, Value::String("App".into()));
        cmx.props.insert("color".into(), Value::String("red".into()));
        assert_eq!(cmx.props.len(), 1);
    }

    #[test]
    fn tuple_type_name_truthy_display() {
        let t = Value::Tuple(vec![Value::Int(10), Value::Int(20)]);
        assert_eq!(t.type_name(), "Tuple");
        assert!(t.is_truthy());
        assert_eq!(t.to_string(), "(10, 20)");
        assert_eq!(Value::Tuple(vec![]).is_truthy(), false);
    }

    #[test]
    fn tuple_partial_eq() {
        let a = Value::Tuple(vec![Value::Int(1), Value::String("x".into())]);
        let b = Value::Tuple(vec![Value::Int(1), Value::String("x".into())]);
        let c = Value::Tuple(vec![Value::Int(1), Value::String("y".into())]);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn enum_value_display_and_eq() {
        let e1 = Value::Enum(Box::new(EnumValue { def_name: "Color".into(), variant: "Rojo".into(), index: 0 }));
        let e2 = Value::Enum(Box::new(EnumValue { def_name: "Color".into(), variant: "Rojo".into(), index: 0 }));
        let e3 = Value::Enum(Box::new(EnumValue { def_name: "Color".into(), variant: "Verde".into(), index: 1 }));
        let e4 = Value::Enum(Box::new(EnumValue { def_name: "Otro".into(), variant: "Rojo".into(), index: 0 }));
        assert_eq!(e1.type_name(), "Enum");
        assert!(e1.is_truthy());
        assert_eq!(e1.to_string(), "Rojo");
        assert_eq!(e1, e2);
        assert_ne!(e1, e3);
        assert_ne!(e1, e4); // distinto enum, misma variante
    }

    #[test]
    fn enum_def_partial_eq() {
        let a = Value::EnumDef(Box::new(EnumDef { name: "Color".into(), variants: vec!["R".into()] }));
        let b = Value::EnumDef(Box::new(EnumDef { name: "Color".into(), variants: vec!["R".into()] }));
        let c = Value::EnumDef(Box::new(EnumDef { name: "Otro".into(), variants: vec!["R".into()] }));
        assert_eq!(a.to_string(), "<enum Color>");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}

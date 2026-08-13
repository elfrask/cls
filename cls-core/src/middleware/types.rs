/// Sistema de tipos de CLS
use std::fmt;

/// Representa un tipo en el sistema de tipos CLS
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    // Primitivos
    Int,
    Float,
    String,
    Bool,
    Char,
    Any,
    Unknown,
    Null,
    Void,
    Empty,

    // Con parámetros
    Array(Box<Type>),                    // type[]
    Tuple(Vec<Type>),                    // (Int, String) heterogéneo por posición
    Record(Box<Type>, Box<Type>),       // String{Integer}
    Shape(Vec<(String, Type)>),         // {nombre: String, edad: Int} (blueprint)
    Fun(Vec<Type>, Box<Type>),          // (Int, String) -> Bool
    Union(Vec<Type>),                   // "a" | "b" | 5
    Literal(LitVal),                    // "d", 5, 1.5 (literal type)

    // Tipos acrónimos (alias)
    I32, I64, I16, I8, F32, F64, Cmx,

    // Tipos nombrados por usuario
    Named(String, Vec<Type>), // Persona, Array<String>
}

/// Valor de un literal type (anotación o const)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LitVal {
    Str(String),
    Int(i64),
    Float(u64), // bits de f64 (f64 no es Eq/Hash)
    Bool(bool),
}

impl LitVal {
    /// El tipo base del literal (sin el valor exacto)
    pub fn base_type(&self) -> Type {
        match self {
            LitVal::Str(_) => Type::String,
            LitVal::Int(_) => Type::Int,
            LitVal::Float(_) => Type::Float,
            LitVal::Bool(_) => Type::Bool,
        }
    }
}

impl Type {
    pub fn is_assignable_to(&self, other: &Type) -> bool {
        match (self, other) {
            // Any puede ser cualquier cosa
            (Type::Any, _) | (_, Type::Any) => true,

            // Tipos idénticos
            (a, b) if a == b => true,

            // Enteros a flotantes (implícito)
            (Type::Int, Type::Float) => true,

            // Alias de enteros
            (Type::I32, Type::Int)
            | (Type::I64, Type::Int)
            | (Type::I16, Type::Int)
            | (Type::I8, Type::Int) => true,

            // Arrays
            (Type::Array(a), Type::Array(b)) => a.is_assignable_to(b),

            // Tuplas: mismo largo y cada slot assignable (posición a posición)
            (Type::Tuple(a), Type::Tuple(b)) => {
                a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| x.is_assignable_to(y))
            }

            // Uniones: assignable si algún miembro del objetivo lo acepta
            (_, Type::Union(members)) => members.iter().any(|m| self.is_assignable_to(m)),
            (Type::Union(members), _) => members.iter().all(|m| m.is_assignable_to(self)),

            // Literal: assignable a otro literal idéntico, o a su tipo base
            (Type::Literal(a), Type::Literal(b)) => a == b,
            (Type::Literal(v), _) => v.base_type().is_assignable_to(other),
            (_, Type::Literal(_)) => false,

            // Records
            (Type::Record(a1, b1), Type::Record(a2, b2)) => {
                a1.is_assignable_to(a2) && b1.is_assignable_to(b2)
            }

            // Shapes: cada campo del destino debe existir en el origen con tipo
            // compatible (el origen puede tener campos extra). Estructural.
            (Type::Shape(src), Type::Shape(dst)) => {
                dst.iter().all(|(dname, dty)| {
                    src.iter()
                        .find(|(sname, _)| sname == dname)
                        .map(|(_, sty)| sty.is_assignable_to(dty))
                        .unwrap_or(false)
                })
            }
            // Shape → Record<K,V>: permitido si todos los valores son assignables a V
            // (el literal `{a:1,b:2}` se usa como diccionario homogéneo tipado).
            (Type::Shape(src), Type::Record(k2, v2)) => {
                k2.is_assignable_to(&Type::String)
                    && src.iter().all(|(_, sty)| sty.is_assignable_to(v2))
            }

            // Functions
            (Type::Fun(params_a, ret_a), Type::Fun(params_b, ret_b)) => {
                // params: contravariante, ret: covariante
                let params_match = params_a.len() == params_b.len()
                    && params_a
                        .iter()
                        .zip(params_b.iter())
                        .all(|(a, b)| b.is_assignable_to(a)); // contravariante
                params_match && ret_a.is_assignable_to(ret_b) // covariante
            }

            // Tipos nombrados: mismo nombre y parámetros compatibles
            (Type::Named(name_a, params_a), Type::Named(name_b, params_b)) => {
                name_a == name_b
                    && params_a.len() == params_b.len()
                    && params_a
                        .iter()
                        .zip(params_b.iter())
                        .all(|(a, b)| a.is_assignable_to(b))
            }

            // Default: no asignable
            _ => false,
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            Type::Int => "Int".to_string(),
            Type::Float => "Float".to_string(),
            Type::String => "String".to_string(),
            Type::Bool => "Bool".to_string(),
            Type::Char => "Char".to_string(),
            Type::Any => "Any".to_string(),
            Type::Unknown => "Unknown".to_string(),
            Type::Null => "Null".to_string(),
            Type::Void => "Void".to_string(),
            Type::Empty => "Empty".to_string(),
            Type::Array(inner) => format!("{}[]", inner.to_string()),
            Type::Tuple(ts) => format!(
                "({})",
                ts.iter().map(|t| t.to_string()).collect::<Vec<_>>().join(", ")
            ),
            Type::Union(ts) => ts.iter().map(|t| t.to_string()).collect::<Vec<_>>().join(" | "),
            Type::Literal(v) => match v {
                LitVal::Str(s) => format!("\"{}\"", s),
                LitVal::Int(i) => i.to_string(),
                LitVal::Float(bits) => format!("{}", f64::from_bits(*bits)),
                LitVal::Bool(b) => b.to_string(),
            },
            Type::Record(k, v) => format!("{}{{{}}}", k.to_string(), v.to_string()),
            Type::Shape(fields) => {
                let fields_str = fields
                    .iter()
                    .map(|(n, t)| format!("{}: {}", n, t.to_string()))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{{{}}}", fields_str)
            }
            Type::Fun(params, ret) => {
                let params_str = params
                    .iter()
                    .map(|p| p.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("fun({}): {}", params_str, ret.to_string())
            }
            Type::I32 => "i32".to_string(),
            Type::I64 => "i64".to_string(),
            Type::I16 => "i16".to_string(),
            Type::I8 => "i8".to_string(),
            Type::F32 => "f32".to_string(),
            Type::F64 => "f64".to_string(),
            Type::Cmx => "cmx".to_string(),
            Type::Named(name, params) => {
                if params.is_empty() {
                    name.clone()
                } else {
                    let params_str = params
                        .iter()
                        .map(|p| p.to_string())
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("{}<{}>", name, params_str)
                }
            }
        }
    }

    /// Si es un Shape, devuelve el tipo del campo por nombre.
    pub fn shape_field(&self, name: &str) -> Option<&Type> {
        match self {
            Type::Shape(fields) => fields
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, t)| t),
            _ => None,
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

/// Función host registrada por el NODO (intrinsic), llamable desde CLS como
/// `nombre(args)`. Se compila vía el canal genérico `env.host_call(id, ptr, n)`:
/// el host recibe el id + los args empaquetados (val, tag) y responde un valor.
///
/// El `id` lo asigna el nodo (debe ser único dentro del registro).
#[derive(Debug, Clone, PartialEq)]
pub struct HostIntrinsic {
    pub id: u32,
    pub name: String,
    /// Tipos concretos de los parámetros (el typeck los valida en la llamada).
    pub params: Vec<Type>,
    /// Tipo del retorno (`Type::Void` si no devuelve nada).
    pub ret: Type,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tuple_identity() {
        let a = Type::Tuple(vec![Type::Int, Type::String]);
        let b = Type::Tuple(vec![Type::Int, Type::String]);
        assert_eq!(a, b, "PartialEq falló para tuplas idénticas");
        assert!(a.is_assignable_to(&b), "Tuple idéntica no assignable");
    }

    #[test]
    fn literal_to_base() {
        let l = Type::Literal(LitVal::Int(1));
        assert_eq!(l, Type::Literal(LitVal::Int(1)));
        assert!(l.is_assignable_to(&Type::Int), "Literal(1) no assignable a Int");
    }

    #[test]
    fn literal_identity() {
        let a = Type::Literal(LitVal::Str("red".to_string()));
        let b = Type::Literal(LitVal::Str("red".to_string()));
        let c = Type::Literal(LitVal::Str("blue".to_string()));
        assert!(a.is_assignable_to(&b));
        assert!(!c.is_assignable_to(&b));
    }

    #[test]
    fn union_literal() {
        let u = Type::Union(vec![
            Type::Literal(LitVal::Str("red".to_string())),
            Type::Literal(LitVal::Str("blue".to_string())),
        ]);
        let red = Type::Literal(LitVal::Str("red".to_string()));
        let other = Type::Literal(LitVal::Str("green".to_string()));
        assert!(red.is_assignable_to(&u));
        assert!(!other.is_assignable_to(&u));
    }
}

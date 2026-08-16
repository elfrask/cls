//! AST - TypeKind (Fase 1: extraido de frontend/ast.rs).

use crate::frontend::ast::*;
use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TypeKind {
    // Tipos primitivos
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

    // Tipos con parámetros
    Array(Box<TypeAnnotation>),
    Tuple(Vec<TypeAnnotation>),        // (Int, String) heterogéneo
    Union(Vec<TypeAnnotation>),        // "a" | "b" | 5
    Record(Box<TypeAnnotation>, Box<TypeAnnotation>), // String{Integer}
    Shape(Vec<(String, TypeAnnotation)>), // {nombre: String, edad: Int}
    Intersection(Vec<TypeAnnotation>),  // Shape1 & Shape2 (merge de shapes)
    Fun(Vec<TypeAnnotation>, Box<TypeAnnotation>),     // (Int, String) -> Bool
    Literal(LiteralKind),              // "d", 5, true (literal type)
    Access(Box<TypeAnnotation>, TypeAccess), // T["key"] | T[0]
    Phantom(Box<TypeAnnotation>),      // !T - param que no participa en el tipo

    // Tipo nombrado (definido por usuario)
    Named(String, Vec<TypeAnnotation>), // Persona, Array<String>

    // Tipos acrónimos
    I32, I64, I16, I8, F32, F64, Cmx,
}

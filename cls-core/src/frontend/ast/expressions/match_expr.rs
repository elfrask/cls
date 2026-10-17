//! AST - MatchExpr (feature `match`, Fase F1).
//!
//! `match` es una expresión (también usable como sentencia) con 4 modos
//! (ver `agent-context/MATCH_PLAN.md` §2):
//! - `match (e) is { ... }`   -> por TIPO
//! - `match (e) in { ... }`   -> por VALOR (igualdad)
//! - `match() if { ... }`     -> por CONDICIÓN (armas = condiciones Bool)
//! - `match (e) { ... }`      -> por PATRÓN (variantes de enum / literales)

use crate::frontend::ast::*;
use crate::error::diagnostic::Span;
use serde::{Deserialize, Serialize};

/// Modo del `match` (elegido por la keyword tras el paréntesis).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatchMode {
    /// Sin keyword: patrones (variantes de enum sin payload / literales) - F1.
    Pattern,
    /// `is`: por tipo del valor.
    Type,
    /// `in`: por igualdad de valor.
    Value,
    /// `if`: condiciones Bool (sin scrutinee).
    Cond,
}

/// Un patrón de arma.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MatchPattern {
    /// Modo VALOR / PATRÓN: literal (`0`, `"x"`, `true`).
    Literal(Literal),
    /// Modo PATRÓN: variante de enum desnuda (`Rojo`) o unit con paréntesis (`s1()`).
    Ident(String),
    /// Modo TIPO: el patrón es una anotación de tipo (`String[]`, `Vector2`, `(A, B)`).
    Type(TypeAnnotation),
    /// Modo PATRÓN (F2): variante con bindings (`ok(texto)`). En F1 los bindings
    /// no se ligan (variantes unit); se parsean para no romper la gramática.
    Variant { name: String, binds: Vec<String> },
    /// Modo CONDICIÓN: el patrón ES una expresión Bool (`n1 > n2`). Variante
    /// extra respecto del plan F1: en modo condición las armas son condiciones.
    Cond(Box<Expression>),
}

/// Cuerpo de un arma: `-> expr` (F1) o `-> { }` (F4, arrowReturn).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MatchBody {
    Expr(Box<Expression>),
    Block(Block),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchArm {
    pub patterns: Vec<MatchPattern>,
    pub body: MatchBody,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchExpr {
    /// Scrutinee (None en modo condición).
    pub scrutinee: Option<Box<Expression>>,
    /// `match (nombre <- expr)` liga `nombre` al scrutinee (segmento de narrowing).
    pub binding: Option<String>,
    pub mode: MatchMode,
    pub arms: Vec<MatchArm>,
    pub default: Option<MatchBody>,
    pub span: Span,
}

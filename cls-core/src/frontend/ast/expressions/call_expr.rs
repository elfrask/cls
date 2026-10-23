//! AST - CallExpr (Fase 1: extraido de frontend/ast.rs).

use crate::frontend::ast::*;
use crate::error::diagnostic::Span;
use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallExpr {
    pub callee: Box<Expression>,
    /// Argumentos de tipo explícitos: `f<String>(...)`.
    #[serde(default)]
    pub type_args: Vec<TypeAnnotation>,
    pub args: Vec<Expression>,
    pub span: Span,
}
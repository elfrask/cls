//! AST - Parameter (Fase 1: extraido de frontend/ast.rs).

use crate::frontend::ast::*;
use crate::error::diagnostic::Span;
use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub type_ann: Option<TypeAnnotation>,
    pub default_value: Option<Expression>,
    pub span: Span,
}

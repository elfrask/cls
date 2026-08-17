//! AST - MetaDirective (Fase 1: extraido de frontend/ast.rs).

use crate::frontend::ast::*;
use crate::error::diagnostic::Span;
use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaDirective {
    pub name: String,
    pub args: Vec<Expression>,
    pub span: Span,
}

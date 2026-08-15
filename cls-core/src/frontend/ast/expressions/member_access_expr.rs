//! AST — MemberAccessExpr (Fase 1: extraido de frontend/ast.rs).

use crate::frontend::ast::*;
use crate::error::diagnostic::Span;
use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberAccessExpr {
    pub object: Box<Expression>,
    pub member: String,
    pub span: Span,
}

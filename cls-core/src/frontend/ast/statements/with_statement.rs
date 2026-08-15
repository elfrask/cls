//! AST — WithStatement (Fase 1: extraido de frontend/ast.rs).

use crate::frontend::ast::*;
use crate::error::diagnostic::Span;
use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithStatement {
    pub name: String,
    pub value: Expression,
    pub block: Block,
    pub span: Span,
}

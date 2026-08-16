//! AST - ForEachStatement (Fase 1: extraido de frontend/ast.rs).

use crate::frontend::ast::*;
use crate::error::diagnostic::Span;
use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForEachStatement {
    pub item_name: String,
    pub index_name: Option<String>,
    pub iterable: Expression,
    pub block: Block,
    pub span: Span,
}

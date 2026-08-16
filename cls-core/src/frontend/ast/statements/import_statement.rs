//! AST - ImportStatement (Fase 1: extraido de frontend/ast.rs).

use crate::error::diagnostic::Span;
use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportStatement {
    pub path: String,
    pub alias: Option<String>,
    pub span: Span,
}

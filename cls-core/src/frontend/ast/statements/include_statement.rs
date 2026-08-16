//! AST - IncludeStatement (Fase 1: extraido de frontend/ast.rs).

use crate::error::diagnostic::Span;
use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncludeStatement {
    pub path: String,
    pub span: Span,
}

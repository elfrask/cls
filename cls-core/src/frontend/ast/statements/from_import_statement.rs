//! AST - FromImportStatement (Fase 1: extraido de frontend/ast.rs).

use crate::frontend::ast::*;
use crate::error::diagnostic::Span;
use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FromImportStatement {
    pub path: String,
    pub names: Vec<ImportName>,
    pub span: Span,
}

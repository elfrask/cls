//! AST — ConfigDirective (Fase 1: extraido de frontend/ast.rs).

use crate::error::diagnostic::Span;
use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigDirective {
    pub key: String,
    pub value: String,
    pub span: Span,
}

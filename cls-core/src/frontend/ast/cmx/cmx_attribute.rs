//! AST — CmxAttribute (Fase 1: extraido de frontend/ast.rs).

use crate::frontend::ast::*;
use crate::error::diagnostic::Span;
use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CmxAttribute {
    pub name: String,
    pub value: Option<CmxAttributeValue>,
    pub span: Span,
}

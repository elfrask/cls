//! AST - element (Fase 1: extraido de frontend/ast.rs).

use crate::frontend::ast::*;
use crate::error::diagnostic::Span;
use serde::{Deserialize, Serialize};


/// CMX Element (JSX nativo)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CmxElement {
    pub tag: String,
    pub attributes: Vec<CmxAttribute>,
    pub children: Vec<CmxChild>,
    pub span: Span,
}

//! AST — SignatureDecl (Fase 1: extraido de frontend/ast.rs).

use crate::frontend::ast::*;
use crate::error::diagnostic::Span;
use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureDecl {
    pub name: String,
    pub params: Vec<Parameter>,
    pub return_type: Option<TypeAnnotation>,
    pub span: Span,
}

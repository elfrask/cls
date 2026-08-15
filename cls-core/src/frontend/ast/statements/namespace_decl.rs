//! AST — NamespaceDecl (Fase 1: extraido de frontend/ast.rs).

use crate::frontend::ast::*;
use crate::error::diagnostic::Span;
use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamespaceDecl {
    pub name: String,
    pub body: Vec<Statement>,
    pub span: Span,
}

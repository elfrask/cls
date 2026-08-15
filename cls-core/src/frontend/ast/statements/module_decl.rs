//! AST — ModuleDecl (Fase 1: extraido de frontend/ast.rs).

use crate::frontend::ast::*;
use crate::error::diagnostic::Span;
use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleDecl {
    pub name: String,
    pub body: Vec<Statement>,
    pub span: Span,
}

//! AST — CatchClause (Fase 1: extraido de frontend/ast.rs).

use crate::frontend::ast::*;
use crate::error::diagnostic::Span;
use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatchClause {
    pub param_name: String,
    pub param_type: Option<TypeAnnotation>,
    pub block: Block,
    pub span: Span,
}

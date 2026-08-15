//! AST — SwitchStatement (Fase 1: extraido de frontend/ast.rs).

use crate::frontend::ast::*;
use crate::error::diagnostic::Span;
use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwitchStatement {
    pub value: Expression,
    pub cases: Vec<CaseClause>,
    pub default: Option<Block>,
    pub span: Span,
}

//! AST — WhenBranch (Fase 1: extraido de frontend/ast.rs).

use crate::frontend::ast::*;
use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhenBranch {
    pub cond: TargetCond,
    pub block: Block,
}

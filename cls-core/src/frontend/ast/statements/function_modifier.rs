//! AST - FunctionModifier (Fase 1: extraido de frontend/ast.rs).

use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FunctionModifier {
    Async,
    Sync,
    Static,
    Export,
    Private,
    Public,
    Global,
}

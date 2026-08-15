//! AST — NativeDecl (Fase 1: extraido de frontend/ast.rs).

use crate::frontend::ast::*;
use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NativeDecl {
    Function(FunctionDecl),
    Structure(StructureDecl),
    Var(VarDecl),
}

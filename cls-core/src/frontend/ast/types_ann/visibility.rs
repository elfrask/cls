//! AST - Visibility (Fase 1: extraido de frontend/ast.rs).

use serde::{Deserialize, Serialize};


/// Visibilidad de miembros
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Visibility {
    Public,
    Private,
    Protected,
    Export,
    Default,
}


impl Default for Visibility {
    fn default() -> Self {
        Visibility::Default
    }
}

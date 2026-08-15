//! AST — TypeAccess (Fase 1: extraido de frontend/ast.rs).

use serde::{Deserialize, Serialize};


/// Acceso a un miembro/posición de un tipo (compile-time)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TypeAccess {
    Key(String),  // T["field"]
    Index(usize), // T[0]
}

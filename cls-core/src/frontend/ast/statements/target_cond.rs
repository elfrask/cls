//! AST — TargetCond (Fase 1: extraido de frontend/ast.rs).

use serde::{Deserialize, Serialize};


/// Condición de la directiva `when` (selección por entorno).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TargetCond {
    Any,
    Os(String),
    Arch(String),
    Abi(String),
    Platform(String),
    Target(String),
    Not(Box<TargetCond>),
    And(Box<TargetCond>, Box<TargetCond>),
    Or(Box<TargetCond>, Box<TargetCond>),
}

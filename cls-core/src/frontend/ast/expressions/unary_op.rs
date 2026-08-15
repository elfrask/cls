//! AST — UnaryOp (Fase 1: extraido de frontend/ast.rs).

use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UnaryOp {
    Negate,     // -x
    Not,        // !x
    BitwiseNot, // ~x
    TypeOf,     // typeof x
    PostInc,    // x++
    PostDec,    // x--
    PreInc,     // ++x
    PreDec,     // --x
}

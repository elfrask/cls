//! AST - IfExpr (feature F3: familia ternario).
//!
//! `if` como **expresión** (bloques + `<-`):
//! ```clx
//! var mayor = if (10 > 5) { * <- true } else { * <- false }
//! ```
//!
//! Es una expresión pura (no usa `?:`). Sus ramas son **ternary-targets**: el
//! valor lo entregan con `* <- expr` (arrowReturn, F4). Requiere `else`.
//! El ternario legacy `if (c) then (a) else (b)` se conserva (ver
//! `ConditionalExpr`).

use crate::frontend::ast::*;
use crate::error::diagnostic::Span;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfExpr {
    pub condition: Box<Expression>,
    pub then_block: Block,
    pub else_block: Block,
    pub span: Span,
}

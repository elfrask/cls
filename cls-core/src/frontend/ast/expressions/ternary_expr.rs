//! AST - TernaryExpr (feature F3: familia ternario).
//!
//! `?:` estilo JS (solo expresión, sin `<-`):
//! ```clx
//! var mayor = 10 > 5 ? true : false;
//! ```
//! Precedencia **la más baja** (por encima de la asignación) y **asociativa a
//! derecha** (`a ? b : c ? d : e` == `a ? b : (c ? d : e)`).
//!
//! Es un nodo propio (no reusa `ConditionalExpr`, que es el ternario legacy
//! `if (c) then (a) else (b)`): así el display, el traversal y el emisor
//! distinguen ambos y el legacy queda intacto.

use crate::frontend::ast::*;
use crate::error::diagnostic::Span;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TernaryExpr {
    pub condition: Box<Expression>,
    pub then_expr: Box<Expression>,
    pub else_expr: Box<Expression>,
    pub span: Span,
}

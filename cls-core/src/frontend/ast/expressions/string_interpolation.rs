//! AST - StringInterpolation (Fase 1: extraido de frontend/ast.rs).

use crate::frontend::ast::*;
use crate::error::diagnostic::Span;
use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StringInterpolation {
    pub parts: Vec<InterpolationPart>,
    pub span: Span,
}

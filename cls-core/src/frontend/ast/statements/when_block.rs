//! AST — WhenBlock (Fase 1: extraido de frontend/ast.rs).

use crate::frontend::ast::*;
use crate::error::diagnostic::Span;
use serde::{Deserialize, Serialize};


/// Directiva multi-entorno: todas las ramas se compilan; en runtime (o en build
/// para AOT) se selecciona la que coincide con el target del entorno actual.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhenBlock {
    pub branches: Vec<WhenBranch>,
    pub span: Span,
}

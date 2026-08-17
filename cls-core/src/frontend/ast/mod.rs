//! AST del lenguaje (Fase 1: extraido de frontend/ast.rs).

pub mod cmx;
pub mod display;
pub mod expressions;
pub mod statements;
pub mod types_ann;

pub use cmx::*;
pub use display::*;
pub use expressions::*;
pub use statements::*;
pub use types_ann::*;

use crate::error::diagnostic::Span;
use serde::{Deserialize, Serialize};


/// Módulo/Archivo CLS completo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Module {
    pub statements: Vec<Statement>,
    pub span: Span,
}

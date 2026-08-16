//! AST - TypeAnnotation (Fase 1: extraido de frontend/ast.rs).

use crate::frontend::ast::*;
use crate::error::diagnostic::Span;
use serde::{Deserialize, Serialize};


/// Anotación de tipo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeAnnotation {
    pub kind: TypeKind,
    pub span: Span,
}

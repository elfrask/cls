//! AST — TypeParam (Fase 1: extraido de frontend/ast.rs).

use crate::frontend::ast::*;
use crate::error::diagnostic::Span;
use serde::{Deserialize, Serialize};


/// Parámetro de tipo genérico (con default opcional)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeParam {
    pub name: String,
    pub default: Option<TypeAnnotation>,
    pub span: Span,
}

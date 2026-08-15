//! AST — InterfaceField (Fase 1: extraido de frontend/ast.rs).

use crate::frontend::ast::*;
use crate::error::diagnostic::Span;
use serde::{Deserialize, Serialize};


/// Campo tipado de una interface (shape): `nombre: tipo`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceField {
    pub name: String,
    pub type_ann: TypeAnnotation,
    pub span: Span,
}

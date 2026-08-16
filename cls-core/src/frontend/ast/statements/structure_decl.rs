//! AST - StructureDecl (Fase 1: extraido de frontend/ast.rs).

use crate::frontend::ast::*;
use crate::error::diagnostic::Span;
use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructureDecl {
    pub name: String,
    pub fields: Vec<FieldDecl>,
    pub span: Span,
    /// Visibilidad (export -> disponible en módulos importados)
    #[serde(default)]
    pub visibility: Visibility,
}

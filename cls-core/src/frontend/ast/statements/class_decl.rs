//! AST - ClassDecl (Fase 1: extraido de frontend/ast.rs).

use crate::frontend::ast::*;
use crate::error::diagnostic::Span;
use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassDecl {
    pub name: String,
    pub extends: Option<String>,
    pub implements: Vec<String>,
    pub body: Vec<ClassMember>,
    pub span: Span,
    /// Parámetros de tipo genérico `<T>` (compile-time)
    #[serde(default)]
    pub type_params: Vec<TypeParam>,
    /// Visibilidad (export -> disponible en módulos importados)
    #[serde(default)]
    pub visibility: Visibility,
}

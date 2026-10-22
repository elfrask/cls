//! AST - InterfaceDecl (Fase 1: extraido de frontend/ast.rs).

use crate::frontend::ast::*;
use crate::error::diagnostic::Span;
use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceDecl {
    pub name: String,
    #[serde(default)]
    pub type_params: Vec<TypeParam>,
    #[serde(default)]
    pub fields: Vec<InterfaceField>,
    /// Interfaces base (`interface Hijo extends Base, Otra`). Los campos y
    /// métodos heredados se resuelven en el typeck.
    #[serde(default)]
    pub extends: Vec<TypeAnnotation>,
    pub signatures: Vec<SignatureDecl>,
    pub span: Span,
    /// Visibilidad (export -> disponible en módulos importados)
    #[serde(default)]
    pub visibility: Visibility,
}

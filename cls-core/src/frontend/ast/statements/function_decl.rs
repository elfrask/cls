//! AST - FunctionDecl (Fase 1: extraido de frontend/ast.rs).

use crate::frontend::ast::*;
use crate::error::diagnostic::Span;
use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDecl {
    pub name: String,
    pub params: Vec<Parameter>,
    pub return_type: Option<TypeAnnotation>,
    pub body: Block,
    pub visibility: Visibility,
    pub modifiers: Vec<FunctionModifier>,
    pub span: Span,
    /// Parámetros de tipo genérico `<T, U>` (compile-time)
    #[serde(default)]
    pub type_params: Vec<TypeParam>,
    /// Función nativa (sin cuerpo, declarada en `extension` o como símbolo del SO)
    #[serde(default)]
    pub is_native: bool,
}

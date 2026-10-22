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
    /// Marca de declaración nativa/FFI (`extension`). `None` = función normal;
    /// `Some` = la función es un import nativo (sin body) equivalente a una
    /// `function` del lenguaje.
    #[serde(default)]
    pub native: Option<NativeInfo>,
}

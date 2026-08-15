//! AST — VarDecl (Fase 1: extraido de frontend/ast.rs).

use crate::frontend::ast::*;
use crate::error::diagnostic::Span;
use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VarDecl {
    pub name: String,
    pub type_ann: Option<TypeAnnotation>,
    pub value: Option<Expression>,
    pub visibility: Visibility,
    pub span: Span,
    /// Variable estática (miembro de clase static)
    #[serde(default)]
    pub is_static: bool,
    /// Variable de solo lectura: escritura solo interna (readonly)
    #[serde(default)]
    pub is_readonly: bool,
    /// REPL: el inicializador SOLO puebla el string pool (data segment), no se
    /// ejecuta en `__init_globals`. El valor llega por transferencia de estado
    /// entre instancias (el hoist conserva la expresión para que los strings
    /// mantengan los mismos offsets del pool y los punteros previos sigan
    /// siendo válidos).
    #[serde(default)]
    pub pool_only: bool,
    /// REPL: seed del string pool SIN global WASM. El inicializador se interna
    /// en el pool (seed), pero la declaración no crea un `__g_N` ni se registra
    /// en los globals de usuario (los índices de los vars de usuario deben
    /// mantenerse estables entre sesiones para la transferencia de estado).
    #[serde(default)]
    pub pool_seed: bool,
}

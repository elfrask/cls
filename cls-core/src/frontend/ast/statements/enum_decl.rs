//! AST — EnumDecl (Fase 1: extraido de frontend/ast.rs).

use crate::frontend::ast::*;
use crate::error::diagnostic::Span;
use serde::{Deserialize, Serialize};


/// Declaración de enum: `enum Color { Rojo, Verde, Azul };`
/// Las variantes son constantes con identidad única (índice dentro del enum).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumDecl {
    pub name: String,
    pub variants: Vec<String>,
    pub span: Span,
    /// Visibilidad (export → disponible en módulos importados)
    pub visibility: Visibility,
}

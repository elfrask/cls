//! AST — ExtensionDecl (Fase 1: extraido de frontend/ast.rs).

use crate::frontend::ast::*;
use crate::error::diagnostic::Span;
use serde::{Deserialize, Serialize};


/// Declaración nativa (`extension "lib" { ... }`) — símbolos de librerías del SO.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionDecl {
    pub library: String,
    /// Tipo de extensión: `extension "lib" as <kind>` (default `C`).
    pub kind: ExtensionKind,
    pub declarations: Vec<NativeDecl>,
    pub span: Span,
}

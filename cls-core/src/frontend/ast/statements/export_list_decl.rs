//! AST - ExportListDecl (`export { ... }`).

use crate::error::diagnostic::Span;
use serde::{Deserialize, Serialize};


/// Referencia a un símbolo exportable dentro de `export { ... }`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExportRef {
    /// Nombre simple: declaración local, binding de `from-import` o export directo.
    Name(String),
    /// Miembro de un módulo importado con namespace: `ns::member` (requiere `as`).
    NamespaceMember(String, String),
    /// Todo el contenido de un módulo importado con namespace: `ns::*` (plano).
    NamespaceAll(String),
}

/// Un elemento de `export { ... }`: `ref` o `ref as exported`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportSpec {
    pub reference: ExportRef,
    /// Nombre exportado (`as`); `None` = el nombre del propio `ref`.
    #[serde(default)]
    pub exported: Option<String>,
}

/// `export { a, b as c } [from "path"];`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportListDecl {
    pub specs: Vec<ExportSpec>,
    /// Reexport: `export { a } from "path"` resuelve los `ref` contra ese módulo.
    #[serde(default)]
    pub from: Option<String>,
    pub span: Span,
}

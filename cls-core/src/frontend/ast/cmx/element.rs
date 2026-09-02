//! AST - element (Fase 1: extraido de frontend/ast.rs).

use crate::frontend::ast::*;
use crate::error::diagnostic::Span;
use serde::{Deserialize, Serialize};


/// CMX Element (JSX nativo)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CmxElement {
    pub tag: String,
    pub attributes: Vec<CmxAttribute>,
    /// Spreads de props `{...expr}` en el tag (REST_SPREAD_PLAN): se evalúan
    /// antes que los attributes; los atributos nombrados tienen prioridad.
    pub spreads: Vec<Expression>,
    pub children: Vec<CmxChild>,
    pub span: Span,
}

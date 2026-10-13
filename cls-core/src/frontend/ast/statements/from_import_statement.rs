//! AST - FromImportStatement (Fase 1: extraido de frontend/ast.rs).

use crate::frontend::ast::*;
use crate::error::diagnostic::Span;
use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FromImportStatement {
    pub path: String,
    /// Símbolos nombrados (`a`, `b as c`).
    pub names: Vec<ImportName>,
    /// `from "m" import *;` — importa **todos** los exports del módulo al scope
    /// actual (reemplaza el antiguo `include`).
    #[serde(default)]
    pub import_all: bool,
    /// `from "m" import * as ns;` — binding del namespace (acceso `ns::sym`).
    #[serde(default)]
    pub namespace: Option<String>,
    pub span: Span,
}

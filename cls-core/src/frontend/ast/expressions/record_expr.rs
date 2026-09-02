//! AST - RecordExpr (Fase 1: extraido de frontend/ast.rs).

use crate::frontend::ast::*;
use crate::error::diagnostic::Span;
use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordExpr {
    pub entries: Vec<(String, Expression)>,
    /// Spreads `{...expr, key: val}` — Fase 2 de REST_SPREAD_PLAN. Se evalúan
    /// antes que entries; los campos de entries tienen prioridad sobre los del
    /// spread (el último set gana).
    pub spreads: Vec<Expression>,
    pub span: Span,
}

//! AST - CmxChild (Fase 1: extraido de frontend/ast.rs).

use crate::frontend::ast::*;
use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CmxChild {
    Text(String),
    Expression(Box<Expression>),
    Element(Box<CmxElement>),
}

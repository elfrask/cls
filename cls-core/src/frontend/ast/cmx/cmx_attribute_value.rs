//! AST - CmxAttributeValue (Fase 1: extraido de frontend/ast.rs).

use crate::frontend::ast::*;
use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CmxAttributeValue {
    String(String),
    Expression(Box<Expression>),
    Shorthand(String), // {value} -> value={value}
}

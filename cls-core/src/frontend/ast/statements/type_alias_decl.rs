//! AST - TypeAliasDecl (Fase 1: extraido de frontend/ast.rs).

use crate::frontend::ast::*;
use crate::error::diagnostic::Span;
use serde::{Deserialize, Serialize};


/// Alias de tipo (compile-time): `alias Vec3 = (Int, Int, Int);`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeAliasDecl {
    pub name: String,
    pub type_params: Vec<TypeParam>,
    pub type_ann: TypeAnnotation,
    pub span: Span,
}

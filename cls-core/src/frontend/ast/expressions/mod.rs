//! AST - expressions (Fase 1: extraido de frontend/ast.rs).

mod array_expr;
mod arrow_function_expr;
mod assignment_expr;
mod binary_expr;
mod call_expr;
mod conditional_expr;
mod index_expr;
mod interpolation_part;
mod literal;
mod literal_kind;
mod member_access_expr;
mod record_expr;
mod string_interpolation;
mod tuple_expr;
mod unary_expr;
mod unary_op;

pub use array_expr::*;
pub use arrow_function_expr::*;
pub use assignment_expr::*;
pub use binary_expr::*;
pub use call_expr::*;
pub use conditional_expr::*;
pub use index_expr::*;
pub use interpolation_part::*;
pub use literal::*;
pub use literal_kind::*;
pub use member_access_expr::*;
pub use record_expr::*;
pub use string_interpolation::*;
pub use tuple_expr::*;
pub use unary_expr::*;
pub use unary_op::*;

use super::cmx::*;
use crate::error::diagnostic::Span;
use serde::{Deserialize, Serialize};



/// Expresiones
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Expression {
    // Literales
    Literal(Literal),

    // Identificadores
    Identifier(String, Span),

    // Operaciones binarias
    Binary(BinaryExpr),

    // Operaciones unarias
    Unary(UnaryExpr),

    // Llamadas
    Call(CallExpr),

    // Acceso a miembro
    MemberAccess(MemberAccessExpr),

    // Indexado
    Index(IndexExpr),

    // Arrays
    Array(ArrayExpr),

    // Tuplas (arrays inmutables)
    Tuple(TupleExpr),

    // Records/Objects
    Record(RecordExpr),

    // Funciones flecha
    ArrowFunction(ArrowFunctionExpr),

    // If como expresión
    Conditional(ConditionalExpr),

    // Asignación
    Assignment(AssignmentExpr),

    // CMX (JSX)
    Cmx(CmxElement),

    // Paréntesis
    Parenthesized(Box<Expression>, Span),

    // Interpolación de strings
    StringInterpolation(StringInterpolation),

    // Namespace access
    NamespaceAccess(String, String, Span), // name::identifier

    // Await: espera una expresion
    Await(Box<Expression>, Span),
}

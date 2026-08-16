//! Helpers sueltos del backend WASM (Fase 1: extraido de wasm/mod.rs).

use crate::error::Span;
use crate::frontend::ast::*;
use crate::middleware::types::{LitVal, Type};
/// Declaraci�n sint�tica de `main` no-op (modo librería): `(i64 args) -> i64`,
/// devuelve 0. Permite instanciar un módulo sin `main` (solo exports).
pub(super) fn noop_main_decl() -> FunctionDecl {
    let span = Span::new(1, 1, 1, 1);
    FunctionDecl {
        name: "main".to_string(),
        params: vec![],
        return_type: None,
        body: Block {
            statements: vec![Statement::Return(Some(Expression::Literal(Literal {
                kind: LiteralKind::Int(0),
                span,
            })))],
            span,
        },
        visibility: Visibility::Private,
        modifiers: vec![],
        span,
        type_params: vec![],
        is_native: false,
    }
}
/// Tipo CLS de un literal (fallback cuando el type map no lo tiene).
/// `math.range(...)` (posiblemente entre paréntesis) -> devuelve un array.
pub(super) fn is_math_range_call(expr: &Expression) -> bool {
    let inner = match expr {
        Expression::Parenthesized(e, _) => &**e,
        e => e,
    };
    if let Expression::Call(c) = inner {
        if let Expression::MemberAccess(m) = &*c.callee {
            if let Expression::Identifier(obj, _) = &*m.object {
                return obj == "math" && m.member == "range";
            }
        }
    }
    false
}

pub(super) fn cmx_literal_type(e: &Expression) -> Option<Type> {
    if let Expression::Literal(l) = e {
        return Some(match &l.kind {
            LiteralKind::Int(_) => Type::Int,
            LiteralKind::Float(_) => Type::Float,
            LiteralKind::String(_) => Type::String,
            LiteralKind::Bool(_) => Type::Bool,
            LiteralKind::Char(_) => Type::Char,
            _ => Type::Any,
        });
    }
    None
}

/// Tipo runtime de una unión (monomórfica) -> el tipo base de sus miembros.
pub(super) fn union_base(t: &Type) -> Type {
    if let Type::Union(members) = t {
        if members
            .iter()
            .all(|m| matches!(m, Type::String | Type::Literal(LitVal::Str(_))))
        {
            return Type::String;
        }
        if members
            .iter()
            .all(|m| matches!(m, Type::Int | Type::Literal(LitVal::Int(_))))
        {
            return Type::Int;
        }
        if members.iter().all(|m| {
            matches!(
                m,
                Type::Float | Type::F32 | Type::F64 | Type::Literal(LitVal::Float(_))
            )
        }) {
            return Type::Float;
        }
        if members
            .iter()
            .all(|m| matches!(m, Type::Bool | Type::Literal(LitVal::Bool(_))))
        {
            return Type::Bool;
        }
    }
    t.clone()
}



pub(super) fn type_name_str(t: &Type) -> &'static str {
    match t {
        Type::Int | Type::I8 | Type::I16 | Type::I32 | Type::I64 => "Int",
        Type::Float | Type::F32 | Type::F64 => "Float",
        Type::String => "String",
        Type::Bool => "Bool",
        Type::Char => "Char",
        Type::Array(_) => "Array",
        _ => "Any",
    }
}

pub(super) fn statement_display(stmt: &Statement) -> String {
    format!("{}", stmt)
}

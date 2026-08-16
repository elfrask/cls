//! TypeChecker â€” helpers libres (Fase 1: extraido de middleware/typeck.rs).

use super::*;

pub fn expr_span(expr: &Expression) -> Span {
    match expr {
        Expression::Literal(l) => l.span,
        Expression::Identifier(_, s) => *s,
        Expression::Binary(b) => b.span,
        Expression::Unary(u) => u.span,
        Expression::Call(c) => c.span,
        Expression::MemberAccess(m) => m.span,
        Expression::Index(i) => i.span,
        Expression::Array(a) => a.span,
        Expression::Tuple(t) => t.span,
        Expression::Record(r) => r.span,
        Expression::ArrowFunction(a) => a.span,
        Expression::Conditional(c) => c.span,
        Expression::Assignment(a) => a.span,
        Expression::Cmx(c) => c.span,
        Expression::Parenthesized(_, s) => *s,
        Expression::StringInterpolation(s) => s.span,
        Expression::NamespaceAccess(_, _, s) => *s,
        Expression::Await(_, s) => *s,
    }
}

pub(crate) fn builtin_type_name(name: &str) -> Option<Type> {
    match name {
        "String" => Some(Type::String),
        "Int" => Some(Type::Int),
        "Float" => Some(Type::Float),
        "Bool" => Some(Type::Bool),
        "Char" => Some(Type::Char),
        "Array" => Some(Type::Array(Box::new(Type::Any))),
        "Tuple" => Some(Type::Tuple(vec![])),
        "Record" => Some(Type::Record(Box::new(Type::Any), Box::new(Type::Any))),
        "Cmx" => Some(Type::Cmx),
        "Null" => Some(Type::Null),
        "Void" => Some(Type::Void),
        _ => None,
    }
}

pub(crate) fn module_arity(mod_name: &str, member: &str) -> Option<usize> {
    match mod_name {
        "os" => match member {
            "platform" | "arch" | "version" | "hostname" | "home" | "tempdir"
            | "cpus" | "pid" | "uptime" | "sep" | "isWindows" | "isUnix" => Some(0),
            "env" => Some(1),
            _ => None,
        },
        "path" => match member {
            "join" => Some(2),
            "basename" | "dirname" | "extname" | "resolve" | "normalize"
            | "isAbsolute" => Some(1),
            "sep" => Some(0),
            _ => None,
        },
        "process" => match member {
            "args" | "cwd" | "pid" | "platform" | "title" => Some(0),
            "env" => Some(1),
            "exit" => Some(1),
            _ => None,
        },
        "time" => match member {
            "now" | "seconds" | "iso" | "date" | "clock" | "year" | "month"
            | "day" | "hour" | "minute" | "second" => Some(0),
            "sleep" => Some(1),
            _ => None,
        },
        "random" => match member {
            "random" | "uuid" => Some(0),
            "int" | "float" => Some(2),
            _ => None,
        },
        _ => None,
    }
}

pub fn expr_short_display(expr: &Expression) -> String {
    crate::frontend::ast::expr_display(expr)
}

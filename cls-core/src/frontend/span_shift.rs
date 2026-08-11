//! Desplazamiento de spans de un AST completo.
//!
//! Cuando se fusionan módulos importados en un solo WASM, sus coordenadas
//! (línea/col) pueden colisionar con las del módulo principal (el `Span` no
//! incluye el archivo). Para evitarlo, cada módulo importado se desplaza con un
//! offset de línea único antes de pasarlo al typeck y al backend.

use super::ast::*;
use crate::error::diagnostic::Span;

pub fn shift_module(m: &mut Module, offset: u32) {
    for stmt in &mut m.statements {
        shift_stmt(stmt, offset);
    }
    m.span = shift_span(m.span.clone(), offset);
}

fn shift_span(mut s: Span, offset: u32) -> Span {
    s.start_line += offset;
    s.end_line += offset;
    s
}

fn shift_type(t: &mut TypeAnnotation, offset: u32) {
    t.span = shift_span(t.span.clone(), offset);
    match &mut t.kind {
        TypeKind::Array(inner) => shift_type(inner, offset),
        TypeKind::Tuple(v) | TypeKind::Union(v) => {
            for x in v {
                shift_type(x, offset);
            }
        }
        TypeKind::Record(a, b) => {
            shift_type(a, offset);
            shift_type(b, offset);
        }
        TypeKind::Shape(fields) => {
            for (_, ft) in fields {
                shift_type(ft, offset);
            }
        }
        TypeKind::Intersection(v) => {
            for x in v {
                shift_type(x, offset);
            }
        }
        TypeKind::Fun(params, ret) => {
            for p in params {
                shift_type(p, offset);
            }
            shift_type(ret, offset);
        }
        TypeKind::Access(inner, _) => shift_type(inner, offset),
        TypeKind::Phantom(inner) => shift_type(inner, offset),
        _ => {}
    }
}

fn shift_literal(l: &mut Literal, offset: u32) {
    l.span = shift_span(l.span.clone(), offset);
}

pub fn shift_expr(e: &mut Expression, offset: u32) {
    match e {
        Expression::Literal(l) => shift_literal(l, offset),
        Expression::Identifier(_, s) => *s = shift_span(s.clone(), offset),
        Expression::Binary(b) => {
            b.span = shift_span(b.span.clone(), offset);
            shift_expr(&mut b.left, offset);
            shift_expr(&mut b.right, offset);
        }
        Expression::Unary(u) => {
            u.span = shift_span(u.span.clone(), offset);
            shift_expr(&mut u.operand, offset);
        }
        Expression::Call(c) => {
            c.span = shift_span(c.span.clone(), offset);
            shift_expr(&mut c.callee, offset);
            for a in &mut c.args {
                shift_expr(a, offset);
            }
        }
        Expression::MemberAccess(m) => {
            m.span = shift_span(m.span.clone(), offset);
            shift_expr(&mut m.object, offset);
        }
        Expression::Index(i) => {
            i.span = shift_span(i.span.clone(), offset);
            shift_expr(&mut i.object, offset);
            shift_expr(&mut i.index, offset);
        }
        Expression::Array(a) => {
            a.span = shift_span(a.span.clone(), offset);
            for el in &mut a.elements {
                shift_expr(el, offset);
            }
        }
        Expression::Tuple(t) => {
            t.span = shift_span(t.span.clone(), offset);
            for el in &mut t.elements {
                shift_expr(el, offset);
            }
        }
        Expression::Record(r) => {
            r.span = shift_span(r.span.clone(), offset);
            for (_, v) in &mut r.entries {
                shift_expr(v, offset);
            }
        }
        Expression::ArrowFunction(a) => {
            a.span = shift_span(a.span.clone(), offset);
            for p in &mut a.params {
                p.span = shift_span(p.span.clone(), offset);
                if let Some(ann) = &mut p.type_ann {
                    shift_type(ann, offset);
                }
                if let Some(d) = &mut p.default_value {
                    shift_expr(d, offset);
                }
            }
            if let Some(ret) = &mut a.return_type {
                shift_type(ret, offset);
            }
            shift_block(&mut a.body, offset);
        }
        Expression::Conditional(c) => {
            c.span = shift_span(c.span.clone(), offset);
            shift_expr(&mut c.condition, offset);
            shift_expr(&mut c.then_expr, offset);
            shift_expr(&mut c.else_expr, offset);
        }
        Expression::Assignment(a) => {
            a.span = shift_span(a.span.clone(), offset);
            shift_expr(&mut a.target, offset);
            shift_expr(&mut a.value, offset);
        }
        Expression::Parenthesized(inner, s) => {
            *s = shift_span(s.clone(), offset);
            shift_expr(inner, offset);
        }
        Expression::StringInterpolation(s) => {
            s.span = shift_span(s.span.clone(), offset);
            for part in &mut s.parts {
                if let InterpolationPart::Expr(e) = part {
                    shift_expr(e, offset);
                }
            }
        }
        Expression::Cmx(c) => shift_cmx(c, offset),
        Expression::NamespaceAccess(_, _, s) => {
            *s = shift_span(s.clone(), offset);
        }
        Expression::Await(inner, s) => {
            *s = shift_span(s.clone(), offset);
            shift_expr(inner, offset);
        }
    }
}

fn shift_cmx(c: &mut CmxElement, offset: u32) {
    c.span = shift_span(c.span.clone(), offset);
    for attr in &mut c.attributes {
        if let Some(CmxAttributeValue::Expression(e)) = &mut attr.value {
            shift_expr(e, offset);
        }
    }
    for child in &mut c.children {
        match child {
            CmxChild::Expression(e) => shift_expr(e, offset),
            CmxChild::Element(el) => shift_cmx(el, offset),
            _ => {}
        }
    }
}

fn shift_block(b: &mut Block, offset: u32) {
    b.span = shift_span(b.span.clone(), offset);
    for st in &mut b.statements {
        shift_stmt(st, offset);
    }
}

fn shift_function(f: &mut FunctionDecl, offset: u32) {
    f.span = shift_span(f.span.clone(), offset);
    for p in &mut f.params {
        p.span = shift_span(p.span.clone(), offset);
        if let Some(ann) = &mut p.type_ann {
            shift_type(ann, offset);
        }
        if let Some(d) = &mut p.default_value {
            shift_expr(d, offset);
        }
    }
    if let Some(ret) = &mut f.return_type {
        shift_type(ret, offset);
    }
    shift_block(&mut f.body, offset);
    for tp in &mut f.type_params {
        tp.span = shift_span(tp.span.clone(), offset);
    }
}

pub fn shift_stmt(s: &mut Statement, offset: u32) {
    match s {
        Statement::VarDecl(v) | Statement::ConstDecl(v) => {
            v.span = shift_span(v.span.clone(), offset);
            if let Some(ann) = &mut v.type_ann {
                shift_type(ann, offset);
            }
            if let Some(val) = &mut v.value {
                shift_expr(val, offset);
            }
        }
        Statement::FunctionDecl(f) => shift_function(f, offset),
        Statement::If(i) => {
            i.span = shift_span(i.span.clone(), offset);
            shift_expr(&mut i.condition, offset);
            shift_block(&mut i.then_block, offset);
            for e in &mut i.elif_branches {
                shift_expr(&mut e.condition, offset);
                shift_block(&mut e.block, offset);
            }
            if let Some(eb) = &mut i.else_block {
                shift_block(eb, offset);
            }
        }
        Statement::While(w) => {
            w.span = shift_span(w.span.clone(), offset);
            shift_expr(&mut w.condition, offset);
            shift_block(&mut w.block, offset);
        }
        Statement::Loop(b) => shift_block(b, offset),
        Statement::For(f) => {
            f.span = shift_span(f.span.clone(), offset);
            if let Some(init) = &mut f.init {
                shift_stmt(init, offset);
            }
            if let Some(c) = &mut f.condition {
                shift_expr(c, offset);
            }
            if let Some(u) = &mut f.update {
                shift_expr(u, offset);
            }
            shift_block(&mut f.block, offset);
        }
        Statement::ForEach(fe) => {
            fe.span = shift_span(fe.span.clone(), offset);
            shift_expr(&mut fe.iterable, offset);
            shift_block(&mut fe.block, offset);
        }
        Statement::Switch(s) => {
            s.span = shift_span(s.span.clone(), offset);
            shift_expr(&mut s.value, offset);
            for c in &mut s.cases {
                if let CasePattern::Literal(l) = &mut c.pattern {
                    shift_literal(l, offset);
                }
                shift_block(&mut c.block, offset);
            }
            if let Some(d) = &mut s.default {
                shift_block(d, offset);
            }
        }
        Statement::Try(t) => {
            t.span = shift_span(t.span.clone(), offset);
            shift_block(&mut t.try_block, offset);
            for c in &mut t.catch_clauses {
                c.span = shift_span(c.span.clone(), offset);
                if let Some(pt) = &mut c.param_type {
                    shift_type(pt, offset);
                }
                shift_block(&mut c.block, offset);
            }
            if let Some(f) = &mut t.finally_block {
                shift_block(f, offset);
            }
        }
        Statement::With(w) => {
            w.span = shift_span(w.span.clone(), offset);
            shift_expr(&mut w.value, offset);
            shift_block(&mut w.block, offset);
        }
        Statement::Return(Some(e)) => shift_expr(e, offset),
        Statement::Return(None) => {}
        Statement::Break | Statement::Continue => {}
        Statement::ClassDecl(c) => {
            c.span = shift_span(c.span.clone(), offset);
            for m in &mut c.body {
                match m {
                    ClassMember::Method(f) | ClassMember::Constructor(f) => shift_function(f, offset),
                    ClassMember::Property(v) => {
                        v.span = shift_span(v.span.clone(), offset);
                        if let Some(ann) = &mut v.type_ann {
                            shift_type(ann, offset);
                        }
                        if let Some(val) = &mut v.value {
                            shift_expr(val, offset);
                        }
                    }
                }
            }
        }
        Statement::StructureDecl(st) => {
            st.span = shift_span(st.span.clone(), offset);
            for f in &mut st.fields {
                f.span = shift_span(f.span.clone(), offset);
                shift_type(&mut f.type_ann, offset);
                if let Some(d) = &mut f.default_value {
                    shift_expr(d, offset);
                }
            }
        }
        Statement::InterfaceDecl(i) => {
            i.span = shift_span(i.span.clone(), offset);
            for f in &mut i.fields {
                shift_type(&mut f.type_ann, offset);
            }
        }
        Statement::ModuleDecl(m) => {
            m.span = shift_span(m.span.clone(), offset);
            for st in &mut m.body {
                shift_stmt(st, offset);
            }
        }
        Statement::NamespaceDecl(n) => {
            n.span = shift_span(n.span.clone(), offset);
            for st in &mut n.body {
                shift_stmt(st, offset);
            }
        }
        Statement::TypeAlias(t) => {
            t.span = shift_span(t.span.clone(), offset);
            shift_type(&mut t.type_ann, offset);
        }
        Statement::EnumDecl(e) => {
            e.span = shift_span(e.span.clone(), offset);
        }
        Statement::Import(_) | Statement::FromImport(_) | Statement::Include(_) => {}
        Statement::Extension(_) => {}
        Statement::When(w) => {
            for b in &mut w.branches {
                shift_block(&mut b.block, offset);
            }
        }
        Statement::Expression(e) => shift_expr(e, offset),
        Statement::Config(_) | Statement::Meta(_) => {}
        Statement::Cmx(c) => shift_cmx(c, offset),
    }
}

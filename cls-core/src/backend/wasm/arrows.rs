//! Recolectores de arrow functions y variables libres (Fase 1: extraido de wasm/mod.rs).
//!
//! `collect_arrows_*` encuentra las arrow functions anidadas en un modulo;
//! `collect_free_vars_*` calcula los identifiers libres del body de una arrow
//! (para construir el handle con sus captures).

use super::*;
/// Motor de emisión a nivel de módulo.
pub(crate) fn collect_arrows_in_block(block: &Block, out: &mut Vec<ArrowFunctionExpr>) {
    for stmt in &block.statements {
        collect_arrows_in_stmt(stmt, out);
    }
}

pub(crate) fn collect_arrows_in_stmt(stmt: &Statement, out: &mut Vec<ArrowFunctionExpr>) {
    match stmt {
        Statement::VarDecl(v) | Statement::ConstDecl(v) => {
            if let Some(val) = &v.value {
                collect_arrows_in_expr(val, out);
            }
        }
        Statement::Expression(e) => collect_arrows_in_expr(e, out),
        Statement::Return(Some(e)) => collect_arrows_in_expr(e, out),
        Statement::If(i) => {
            collect_arrows_in_expr(&i.condition, out);
            collect_arrows_in_block(&i.then_block, out);
            for e in &i.elif_branches {
                collect_arrows_in_expr(&e.condition, out);
                collect_arrows_in_block(&e.block, out);
            }
            if let Some(eb) = &i.else_block {
                collect_arrows_in_block(eb, out);
            }
        }
        Statement::While(w) => {
            collect_arrows_in_expr(&w.condition, out);
            collect_arrows_in_block(&w.block, out);
        }
        Statement::For(f) => {
            if let Some(init) = &f.init {
                collect_arrows_in_stmt(init, out);
            }
            if let Some(cond) = &f.condition {
                collect_arrows_in_expr(cond, out);
            }
            if let Some(upd) = &f.update {
                collect_arrows_in_expr(upd, out);
            }
            collect_arrows_in_block(&f.block, out);
        }
        Statement::ForEach(fe) => {
            collect_arrows_in_expr(&fe.iterable, out);
            collect_arrows_in_block(&fe.block, out);
        }
        Statement::Switch(s) => {
            collect_arrows_in_expr(&s.value, out);
            for c in &s.cases {
                collect_arrows_in_block(&c.block, out);
            }
            if let Some(d) = &s.default {
                collect_arrows_in_block(d, out);
            }
        }
        Statement::With(w) => {
            collect_arrows_in_expr(&w.value, out);
            collect_arrows_in_block(&w.block, out);
        }
        Statement::Loop(b) => collect_arrows_in_block(b, out),
        _ => {}
    }
}

pub(crate) fn collect_arrows_in_expr(expr: &Expression, out: &mut Vec<ArrowFunctionExpr>) {
    match expr {
        Expression::ArrowFunction(a) => {
            out.push((*a).clone());
            collect_arrows_in_block(&a.body, out);
        }
        Expression::Call(c) => {
            collect_arrows_in_expr(&c.callee, out);
            for arg in &c.args {
                collect_arrows_in_expr(arg, out);
            }
        }
        Expression::MemberAccess(m) => collect_arrows_in_expr(&m.object, out),
        Expression::Index(i) => {
            collect_arrows_in_expr(&i.object, out);
            collect_arrows_in_expr(&i.index, out);
        }
        Expression::Array(a) => {
            for el in &a.elements {
                collect_arrows_in_expr(el, out);
            }
        }
        Expression::Tuple(t) => {
            for el in &t.elements {
                collect_arrows_in_expr(el, out);
            }
        }
        Expression::Record(r) => {
            for (_, v) in &r.entries {
                collect_arrows_in_expr(v, out);
            }
        }
        Expression::Binary(b) => {
            collect_arrows_in_expr(&b.left, out);
            collect_arrows_in_expr(&b.right, out);
        }
        Expression::Unary(u) => collect_arrows_in_expr(&u.operand, out),
        Expression::Conditional(c) => {
            collect_arrows_in_expr(&c.condition, out);
            collect_arrows_in_expr(&c.then_expr, out);
            collect_arrows_in_expr(&c.else_expr, out);
        }
        Expression::Assignment(a) => {
            collect_arrows_in_expr(&a.target, out);
            collect_arrows_in_expr(&a.value, out);
        }
        Expression::Parenthesized(e, _) => collect_arrows_in_expr(e, out),
        Expression::StringInterpolation(s) => {
            for part in &s.parts {
                if let InterpolationPart::Expr(e) = part {
                    collect_arrows_in_expr(e, out);
                }
            }
        }
        Expression::Cmx(c) => {
            for attr in &c.attributes {
                if let Some(CmxAttributeValue::Expression(e)) = &attr.value {
                    collect_arrows_in_expr(e, out);
                }
            }
            for child in &c.children {
                match child {
                    CmxChild::Expression(e) => collect_arrows_in_expr(e, out),
                    CmxChild::Element(el) => {
                        collect_arrows_in_expr(&Expression::Cmx((**el).clone()), out)
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
}

/// Recolecta los identifiers libres del body de una arrow (closures).
/// `locals` acumula params + variables declaradas dentro; `free` acumula los
/// identifiers que se usan pero no son params ni declarados localmente.
pub(crate) fn collect_free_vars_in_block(block: &Block, locals: &mut Vec<String>, free: &mut Vec<String>) {
    for stmt in &block.statements {
        collect_free_vars_in_stmt(stmt, locals, free);
    }
}

pub(crate) fn collect_free_vars_in_stmt(stmt: &Statement, locals: &mut Vec<String>, free: &mut Vec<String>) {
    match stmt {
        Statement::VarDecl(v) | Statement::ConstDecl(v) => {
            if let Some(val) = &v.value {
                collect_free_vars_in_expr(val, locals, free);
            }
            locals.push(v.name.clone());
        }
        Statement::Expression(e) => collect_free_vars_in_expr(e, locals, free),
        Statement::Return(Some(e)) => collect_free_vars_in_expr(e, locals, free),
        Statement::If(i) => {
            collect_free_vars_in_expr(&i.condition, locals, free);
            collect_free_vars_in_block(&i.then_block, locals, free);
            for e in &i.elif_branches {
                collect_free_vars_in_expr(&e.condition, locals, free);
                collect_free_vars_in_block(&e.block, locals, free);
            }
            if let Some(eb) = &i.else_block {
                collect_free_vars_in_block(eb, locals, free);
            }
        }
        Statement::While(w) => {
            collect_free_vars_in_expr(&w.condition, locals, free);
            collect_free_vars_in_block(&w.block, locals, free);
        }
        Statement::For(f) => {
            if let Some(init) = &f.init {
                collect_free_vars_in_stmt(init, locals, free);
            }
            if let Some(cond) = &f.condition {
                collect_free_vars_in_expr(cond, locals, free);
            }
            if let Some(upd) = &f.update {
                collect_free_vars_in_expr(upd, locals, free);
            }
            collect_free_vars_in_block(&f.block, locals, free);
        }
        Statement::ForEach(fe) => {
            collect_free_vars_in_expr(&fe.iterable, locals, free);
            locals.push(fe.item_name.clone());
            if let Some(iname) = &fe.index_name {
                locals.push(iname.clone());
            }
            collect_free_vars_in_block(&fe.block, locals, free);
        }
        Statement::Switch(s) => {
            collect_free_vars_in_expr(&s.value, locals, free);
            for c in &s.cases {
                collect_free_vars_in_block(&c.block, locals, free);
            }
            if let Some(d) = &s.default {
                collect_free_vars_in_block(d, locals, free);
            }
        }
        Statement::With(w) => {
            collect_free_vars_in_expr(&w.value, locals, free);
            locals.push(w.name.clone());
            collect_free_vars_in_block(&w.block, locals, free);
        }
        Statement::Loop(b) => collect_free_vars_in_block(b, locals, free),
        _ => {}
    }
}

pub(crate) fn collect_free_vars_in_expr(expr: &Expression, locals: &mut Vec<String>, free: &mut Vec<String>) {
    match expr {
        Expression::Identifier(name, _) => {
            if !locals.contains(name) && !free.contains(name) {
                free.push(name.clone());
            }
        }
        Expression::Call(c) => {
            collect_free_vars_in_expr(&c.callee, locals, free);
            for arg in &c.args {
                collect_free_vars_in_expr(arg, locals, free);
            }
        }
        Expression::MemberAccess(m) => collect_free_vars_in_expr(&m.object, locals, free),
        Expression::Index(i) => {
            collect_free_vars_in_expr(&i.object, locals, free);
            collect_free_vars_in_expr(&i.index, locals, free);
        }
        Expression::Array(a) => {
            for el in &a.elements {
                collect_free_vars_in_expr(el, locals, free);
            }
        }
        Expression::Tuple(t) => {
            for el in &t.elements {
                collect_free_vars_in_expr(el, locals, free);
            }
        }
        Expression::Record(r) => {
            for (_, v) in &r.entries {
                collect_free_vars_in_expr(v, locals, free);
            }
        }
        Expression::Binary(b) => {
            collect_free_vars_in_expr(&b.left, locals, free);
            collect_free_vars_in_expr(&b.right, locals, free);
        }
        Expression::Unary(u) => collect_free_vars_in_expr(&u.operand, locals, free),
        Expression::Conditional(c) => {
            collect_free_vars_in_expr(&c.condition, locals, free);
            collect_free_vars_in_expr(&c.then_expr, locals, free);
            collect_free_vars_in_expr(&c.else_expr, locals, free);
        }
        Expression::Assignment(a) => {
            collect_free_vars_in_expr(&a.target, locals, free);
            collect_free_vars_in_expr(&a.value, locals, free);
        }
        Expression::Parenthesized(e, _) => collect_free_vars_in_expr(e, locals, free),
        Expression::StringInterpolation(s) => {
            for part in &s.parts {
                if let InterpolationPart::Expr(e) = part {
                    collect_free_vars_in_expr(e, locals, free);
                }
            }
        }
        Expression::Cmx(c) => {
            for attr in &c.attributes {
                if let Some(CmxAttributeValue::Expression(e)) = &attr.value {
                    collect_free_vars_in_expr(e, locals, free);
                }
            }
            for child in &c.children {
                match child {
                    CmxChild::Expression(e) => collect_free_vars_in_expr(e, locals, free),
                    CmxChild::Element(el) => {
                        collect_free_vars_in_expr(&Expression::Cmx((**el).clone()), locals, free)
                    }
                    _ => {}
                }
            }
        }
        Expression::ArrowFunction(a) => {
            // Arrow anidada: sus variables libres también son libres para la arrow
            // externa (el padre debe capturarlas para construir el handle interno).
            // Los params de la arrow interna se excluyen.
            let mut inner_locals: Vec<String> = a.params.iter().map(|p| p.name.clone()).collect();
            inner_locals.extend(locals.iter().cloned());
            collect_free_vars_in_block(&a.body, &mut inner_locals, free);
        }
        _ => {}
    }
}
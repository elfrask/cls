//! flow.rs (Fase 1: extraido de cls-core/src/middleware/typeck/statements.rs).

use super::*;
use crate::frontend::token::Operator;

impl TypeChecker {



    pub(crate) fn check_if(&mut self, i: &IfStatement) -> Type {
        let cond = self.check_expression(&i.condition);
        if !cond.is_assignable_to(&Type::Bool) {
            self.warn(
                &format!("Condición if debe ser Bool, encontró {}", cond),
                i.span.clone(),
            );
        }
        // Narrowing por `is`: si la condición es `x is Tipo`, estrechar `x` al
        // tipo dentro del bloque (scope anidado, se restaura al salir). Réplica
        // del narrowing de TypeScript. Ver plan tipo-callable-is-narrowing-spread.md.
        let narrowed = self.narrowed_var(&i.condition);
        if let Some((name, ty)) = &narrowed {
            self.push_scope();
            self.define(name, ty.clone());
        }
        self.check_block(&i.then_block);
        if narrowed.is_some() {
            self.pop_scope();
        }
        for elif in &i.elif_branches {
            let en = self.narrowed_var(&elif.condition);
            if let Some((name, ty)) = &en {
                self.push_scope();
                self.define(name, ty.clone());
            }
            self.check_expression(&elif.condition);
            self.check_block(&elif.block);
            if en.is_some() {
                self.pop_scope();
            }
        }
        if let Some(else_block) = &i.else_block {
            self.check_block(else_block);
        }
        Type::Void
    }

    /// Si `cond` es `Identifier(x) is Tipo` (builtin/Callable), devuelve
    /// `(x, tipo_estrecho)` para narrowing dentro del bloque.
    fn narrowed_var(&self, cond: &Expression) -> Option<(String, Type)> {
        if let Expression::Binary(b) = cond {
            if b.op == Operator::Is {
                if let Expression::Identifier(name, _) = &*b.left {
                    if let Expression::Identifier(right, _) = &*b.right {
                        if let Some(t) = crate::middleware::typeck::helpers::builtin_type_name(right) {
                            return Some((name.clone(), t));
                        }
                        if right == "Callable" {
                            return Some((name.clone(), Type::Callable));
                        }
                    }
                }
            }
        }
        None
    }



    pub(crate) fn check_while(&mut self, w: &WhileStatement) -> Type {
        let cond = self.check_expression(&w.condition);
        if !cond.is_assignable_to(&Type::Bool) {
            self.warn(
                &format!("Condición while debe ser Bool, encontró {}", cond),
                w.span.clone(),
            );
        }
        self.check_block(&w.block);
        Type::Void
    }



    pub(crate) fn check_for(&mut self, f: &ForStatement) -> Type {
        self.push_scope();
        if let Some(init) = &f.init {
            self.check_statement(init);
        }
        if let Some(cond) = &f.condition {
            self.check_expression(cond);
        }
        if let Some(upd) = &f.update {
            self.check_expression(upd);
        }
        self.check_block(&f.block);
        self.pop_scope();
        Type::Void
    }



    pub(crate) fn check_foreach(&mut self, fe: &ForEachStatement) -> Type {
        let iter_ty = self.check_expression(&fe.iterable);
        let item_ty = match &iter_ty {
            Type::Array(e) => (**e).clone(),
            Type::Tuple(s) => s.first().cloned().unwrap_or(Type::Any),
            Type::Named(n, _) if self.enums.contains(n) => iter_ty.clone(),
            // Magic methods __iter/__next: el item es el tipo del elemento del
            // array devuelto por __iter, o el retorno de __next del iterador.
            Type::Named(_, _) => {
                match self.named_magic_ret(&iter_ty, "__iter") {
                    Some(Type::Array(e)) => (*e).clone(),
                    Some(Type::Named(itn, _)) => self
                        .class_members
                        .get(&itn)
                        .and_then(|im| im.get("__next"))
                        .cloned()
                        .unwrap_or(Type::Any),
                    _ => Type::Any,
                }
            }
            _ => Type::Any,
        };
        self.push_scope();
        self.define(&fe.item_name, item_ty);
        if let Some(idx_name) = &fe.index_name {
            self.define(idx_name, Type::Int);
        }
        self.check_block(&fe.block);
        self.pop_scope();
        Type::Void
    }



    pub(crate) fn check_switch(&mut self, s: &SwitchStatement) -> Type {
        self.check_expression(&s.value);
        for case in &s.cases {
            self.check_block(&case.block);
        }
        if let Some(default) = &s.default {
            self.check_block(default);
        }
        Type::Void
    }



    pub(crate) fn check_try(&mut self, t: &TryStatement) -> Type {
        self.check_block(&t.try_block);
        for catch in &t.catch_clauses {
            self.push_scope();
            let err_type = catch.param_type.as_ref()
                .map(|ta| self.resolve_type_annotation(ta))
                .unwrap_or(Type::String);
            self.define(&catch.param_name, err_type);
            self.check_block(&catch.block);
            self.pop_scope();
        }
        if let Some(finally) = &t.finally_block {
            self.check_block(finally);
        }
        Type::Void
    }



    pub(crate) fn check_with(&mut self, w: &WithStatement) -> Type {
        self.check_expression(&w.value);
        self.push_scope();
        self.define(&w.name, Type::Any);
        self.check_block(&w.block);
        self.pop_scope();
        Type::Void
    }

}
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

    /// Si `cond` es `Identifier(x) is Tipo` (builtin/Callable/clase), devuelve
    /// `(x, tipo_estrecho)` para narrowing dentro del bloque. Acepta la clase
    /// cualificada por el alias de import (`x is mod::Clase`), estrechando al
    /// nombre local de la clase.
    fn narrowed_var(&self, cond: &Expression) -> Option<(String, Type)> {
        if let Expression::Binary(b) = cond {
            if b.op == Operator::Is {
                if let Expression::Identifier(name, _) = &*b.left {
                    match &*b.right {
                        Expression::Identifier(right, _) => {
                            if let Some(t) = crate::middleware::typeck::helpers::builtin_type_name(right) {
                                return Some((name.clone(), t));
                            }
                            if right == "Callable" {
                                return Some((name.clone(), Type::Callable));
                            }
                            // Clase de usuario: `x is Derived` estrecha `x` a `Derived`
                            // (acceso a campos/métodos de la subclase dentro del bloque).
                            if self.class_members.contains_key(right.as_str()) {
                                return Some((name.clone(), Type::Named(right.clone(), vec![])));
                            }
                        }
                        // Clase importada cualificada `mod::Clase`: el nombre local
                        // de la clase es el miembro tras `::`.
                        Expression::NamespaceAccess(_ns, member, _) => {
                            if self.class_members.contains_key(member.as_str()) {
                                return Some((name.clone(), Type::Named(member.clone(), vec![])));
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        None
    }



    /// Chequeo del `match` (feature F1). Reglas:
    /// - `match` vacío (sin armas) -> error de compilación.
    /// - Modo TIPO: los patrones son tipos; si el scrutinee es un nombre, se
    ///   estrecha a ese tipo (o a la unión si hay varios) dentro del arma.
    /// - Modo PATRÓN: variantes de enum (sin payload en F1) / literales.
    /// - Modo VALOR / CONDICIÓN: igualdad / condiciones Bool.
    /// - El tipo del `match` es el de la PRIMERA arma; las demás deben ser
    ///   asignables.
    pub(crate) fn check_match(&mut self, m: &MatchExpr) -> Type {
        if m.arms.is_empty() {
            self.error(
                "`match` vacío: se requiere al menos un arma (`patrón -> expr`)",
                m.span.clone(),
            );
            return Type::Unknown;
        }

        let scrut_ty = m.scrutinee.as_ref().map(|s| self.check_expression(s));

        // Nombre a estrechar dentro de las armas: el binding (`nombre <- expr`) o
        // el propio scrutinee si es un identificador.
        let narrow_name: Option<String> = m.binding.clone().or_else(|| match &m.scrutinee {
            Some(s) => match &**s {
                Expression::Identifier(n, _) => Some(n.clone()),
                _ => None,
            },
            None => None,
        });

        let mut result: Option<Type> = None;
        for arm in &m.arms {
            self.push_scope();
            if let (Some(name), Some(base)) = (&narrow_name, &scrut_ty) {
                if let Some(narrowed) = self.match_arm_narrow(m.mode, &arm.patterns, base) {
                    self.define(name, narrowed);
                }
            }
            self.check_match_patterns(m.mode, &arm.patterns, scrut_ty.as_ref());
            let bt = match &arm.body {
                MatchBody::Expr(e) => self.check_expression(e),
                MatchBody::Block(b) => {
                    self.check_block(b);
                    Type::Void
                }
            };
            self.pop_scope();
            match &result {
                None => result = Some(bt),
                Some(rt) => {
                    if !bt.is_assignable_to(rt) {
                        let msg = format!(
                            "Tipo de arma `match` ({}) no asignable al tipo de la primera arma ({})",
                            bt, rt
                        );
                        if self.config.strict {
                            self.error(&msg, arm.span.clone());
                        } else {
                            self.warn(&msg, arm.span.clone());
                        }
                    }
                }
            }
        }

        if let Some(def) = &m.default {
            let dt = match def {
                MatchBody::Expr(e) => self.check_expression(e),
                MatchBody::Block(b) => {
                    self.check_block(b);
                    Type::Void
                }
            };
            if let Some(rt) = &result {
                if !dt.is_assignable_to(rt) {
                    let msg = format!(
                        "Tipo de `default` de `match` ({}) no asignable al tipo de la primera arma ({})",
                        dt, rt
                    );
                    if self.config.strict {
                        self.error(&msg, m.span.clone());
                    } else {
                        self.warn(&msg, m.span.clone());
                    }
                }
            }
        }

        result.unwrap_or(Type::Void)
    }

    /// Tipo estrechado del scrutinee dentro de un arma.
    /// - Modo TIPO: la unión de los tipos de sus patrones (o el único tipo).
    /// - Modo PATRÓN: el enum del scrutinee (sin cambio).
    /// - Modo VALOR / CONDICIÓN: sin cambio.
    fn match_arm_narrow(
        &mut self,
        mode: MatchMode,
        patterns: &[MatchPattern],
        base: &Type,
    ) -> Option<Type> {
        match mode {
            MatchMode::Type => {
                let mut types = Vec::new();
                for p in patterns {
                    if let MatchPattern::Type(ta) = p {
                        types.push(self.resolve_type_annotation(ta));
                    }
                }
                match types.len() {
                    0 => None,
                    1 => Some(types.remove(0)),
                    _ => Some(Type::Union(types)),
                }
            }
            MatchMode::Pattern => Some(base.clone()),
            MatchMode::Value | MatchMode::Cond => None,
        }
    }

    /// Valida los patrones de un arma y registra sus tipos/expresiones.
    fn check_match_patterns(
        &mut self,
        mode: MatchMode,
        patterns: &[MatchPattern],
        scrut_ty: Option<&Type>,
    ) {
        for p in patterns {
            match p {
                MatchPattern::Literal(l) => {
                    self.check_literal(l);
                }
                MatchPattern::Type(ta) => {
                    self.resolve_type_annotation(ta);
                }
                MatchPattern::Cond(e) => {
                    let t = self.check_expression(e);
                    if !t.is_assignable_to(&Type::Bool) {
                        self.warn(
                            &format!("Condición de arma `match` debe ser Bool, encontró {}", t),
                            expr_span(e),
                        );
                    }
                }
                MatchPattern::Variant { binds, .. } => {
                    // F1: variantes unit; los bindings de payload llegan en F2.
                    for b in binds {
                        self.define(b, Type::Any);
                    }
                }
                MatchPattern::Ident(_) => {
                    // Variante de enum desnuda. La validación de variantes con el
                    // enum del scrutinee es F2; en F1 el emisor comprueba el índice.
                    let _ = (mode, scrut_ty);
                }
            }
        }
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
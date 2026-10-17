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

        // F4: este `match` es un match-target para `match <-`/`match* <-`/...
        self.match_yield_stack.push(None);

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
        // Esquema del enum del scrutinee (modo PATRÓN, F2): variante -> params.
        // Se resuelve una vez; `None` = el scrutinee no es un enum conocido (o es
        // un enum importado sin esquema registrado).
        let enum_schema: Option<(String, Vec<(String, Vec<Type>)>)> =
            if m.mode == MatchMode::Pattern {
                match scrut_ty.as_ref() {
                    Some(Type::Named(n, _)) => match self.enum_schema(n) {
                        Some(s) => Some((n.clone(), s)),
                        // Enum importado sin esquema registrado: el emisor lo
                        // resuelve por sufijo; no forzamos error (paridad F1).
                        None => None,
                    },
                    // Tipos claramente no-enum -> error claro. `Any`/`Unknown`/
                    // `Union`/`Named` (clase/struct/import) no se fuerzan (el
                    // emisor decide).
                    Some(other) if is_obviously_non_enum(other) => {
                        self.error(
                            &format!(
                                "`match` en modo patrón requiere un enum como scrutinee, encontró `{}`",
                                other
                            ),
                            m.span.clone(),
                        );
                        None
                    }
                    _ => None,
                }
            } else {
                None
            };
        for arm in &m.arms {
            self.push_scope();
            if let (Some(name), Some(base)) = (&narrow_name, &scrut_ty) {
                if let Some(narrowed) = self.match_arm_narrow(m.mode, &arm.patterns, base) {
                    self.define(name, narrowed);
                }
            }
            self.check_match_patterns(m.mode, &arm.patterns, scrut_ty.as_ref(), enum_schema.as_ref(), &arm.span);
            // `Some(t)` = el arma entrega un valor (expr o bloque con `* <-`).
            // `None` = arma-bloque que NO entrega a este `match` (p.ej. salta
            // afuera con `match* <-` o cae sin `<-`): no constriñe el tipo.
            let bt_opt = match &arm.body {
                MatchBody::Expr(e) => Some(self.check_expression(e)),
                MatchBody::Block(b) => self.check_match_arm_block(b),
            };
            self.pop_scope();
            if let Some(bt) = bt_opt {
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
        }

        if let Some(def) = &m.default {
            let dt_opt = match def {
                MatchBody::Expr(e) => Some(self.check_expression(e)),
                MatchBody::Block(b) => self.check_match_arm_block(b),
            };
            if let (Some(dt), Some(rt)) = (dt_opt, &result) {
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

        let my = self.match_yield_stack.pop().flatten();
        match result {
            Some(t) if !matches!(t, Type::Void) => t,
            _ => my.unwrap_or(Type::Void),
        }
    }

    /// Chequea el bloque de un arma de `match` como ternary-target (F4).
    /// Devuelve `Some(tipo)` si el bloque entrega un valor con `* <-` a su
    /// propio arma; `None` si no (salta afuera o cae sin `<-`).
    fn check_match_arm_block(&mut self, b: &Block) -> Option<Type> {
        self.ternary_yield_stack.push(None);
        self.check_block(b);
        self.ternary_yield_stack.pop().flatten()
    }


    /// `if (c) { ... } else { ... }` como expresión (F3). El tipo es el del
    /// `* <-` que reciban sus ramas (ternary-targets). Requiere `else` (lo
    /// valida el parser).
    pub(crate) fn check_if_expr(&mut self, e: &IfExpr) -> Type {
        let cond = self.check_expression(&e.condition);
        if !cond.is_assignable_to(&Type::Bool) {
            self.warn(
                &format!("Condición de expresión `if` debe ser Bool, encontró {}", cond),
                e.span.clone(),
            );
        }
        self.ternary_yield_stack.push(None);
        self.check_block(&e.then_block);
        let then_y = self.ternary_yield_stack.pop().flatten();
        self.ternary_yield_stack.push(None);
        self.check_block(&e.else_block);
        let else_y = self.ternary_yield_stack.pop().flatten();
        match (then_y, else_y) {
            (Some(t), _) => t,
            (None, Some(t)) => t,
            (None, None) => Type::Void,
        }
    }

    /// `* <- expr` / `match <- expr` (F4): chequea el valor y registra su tipo
    /// en el target léxico correspondiente (para tipar `if`-expresiones y
    /// `match`). No valida existencia del target (lo hace el emisor).
    pub(crate) fn check_arrow_return(&mut self, a: &ArrowReturn) -> Type {
        let vt = self.check_expression(&a.value);
        if a.is_match {
            if let Some(slot) = self.match_yield_stack.iter_mut().rev().nth(a.stars) {
                if slot.is_none() {
                    *slot = Some(vt.clone());
                }
            }
        } else if a.stars >= 1 {
            if let Some(slot) = self.ternary_yield_stack.iter_mut().rev().nth(a.stars - 1) {
                if slot.is_none() {
                    *slot = Some(vt.clone());
                }
            }
        }
        vt
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
    /// `enum_schema`: (nombre, variante -> params) del enum del scrutinee, si se
    /// conoce. En modo PATRÓN valida variante/aridad/tipos y liga los binds.
    fn check_match_patterns(
        &mut self,
        mode: MatchMode,
        patterns: &[MatchPattern],
        scrut_ty: Option<&Type>,
        enum_schema: Option<&(String, Vec<(String, Vec<Type>)>)>,
        arm_span: &Span,
    ) {
        // F2: a lo sumo un patrón con binds por arma (el payload ligado tiene un
        // único origen; varias alternativas con binds serían ambiguas).
        let with_binds = patterns
            .iter()
            .filter(|p| matches!(p, MatchPattern::Variant { binds, .. } if !binds.is_empty()))
            .count();
        if with_binds > 1 {
            self.error(
                "Un arma de `match` solo puede tener un patrón con bindings de payload",
                arm_span.clone(),
            );
        }
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
                MatchPattern::Variant { name, binds } => {
                    // Validación estática contra el esquema del enum (F2).
                    if let Some((enum_name, variants)) = enum_schema {
                        match variants.iter().find(|(v, _)| v == name) {
                            Some((_, params)) => {
                                if params.len() != binds.len() {
                                    self.error(
                                        &format!(
                                            "La variante `{}` del enum `{}` espera {} binding(s), recibió {}",
                                            name,
                                            enum_name,
                                            params.len(),
                                            binds.len()
                                        ),
                                        arm_span.clone(),
                                    );
                                }
                                for (i, b) in binds.iter().enumerate() {
                                    if b == "_" {
                                        continue;
                                    }
                                    let bt = params.get(i).cloned().unwrap_or(Type::Any);
                                    self.define(b, bt);
                                }
                            }
                            None => {
                                self.error(
                                    &format!("La variante `{}` no existe en el enum `{}`", name, enum_name),
                                    arm_span.clone(),
                                );
                            }
                        }
                    } else {
                        // Esquema desconocido: ligar Any (paridad F1).
                        for b in binds {
                            if b != "_" {
                                self.define(b, Type::Any);
                            }
                        }
                    }
                }
                MatchPattern::Ident(name) => {
                    // Variante unit desnuda: validar contra el esquema (F2).
                    if let Some((enum_name, variants)) = enum_schema {
                        match variants.iter().find(|(v, _)| v == name) {
                            Some((_, params)) => {
                                if !params.is_empty() {
                                    self.error(
                                        &format!(
                                            "La variante `{}` del enum `{}` tiene payload; usá `{}(...)` en el patrón",
                                            name, enum_name, name
                                        ),
                                        arm_span.clone(),
                                    );
                                }
                            }
                            None => {
                                self.error(
                                    &format!("La variante `{}` no existe en el enum `{}`", name, enum_name),
                                    arm_span.clone(),
                                );
                            }
                        }
                    }
                    let _ = (mode, scrut_ty);
                }
            }
        }
    }

    /// Esquema del enum `name` (exacto o por sufijo `mod::name`).
    pub(crate) fn enum_schema(&self, name: &str) -> Option<Vec<(String, Vec<Type>)>> {
        if let Some(s) = self.enum_defs.get(name) {
            return Some(s.clone());
        }
        let suffix = format!("::{}", name);
        self.enum_defs
            .iter()
            .find(|(k, _)| k.ends_with(&suffix))
            .map(|(_, v)| v.clone())
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

    /// Devuelve (enum, variante) si `e` demuestra una construcción de variante:
    /// una llamada directa `E::v(...)`, un `E::v` sin payload, o un identificador
    /// cuyo último valor registrado fue una construcción. Base de la
    /// provabilidad del destructuring-let (F2).
    pub(crate) fn construction_variant(&self, e: &Expression) -> Option<(String, String)> {
        match e {
            Expression::Call(c) => match &*c.callee {
                Expression::NamespaceAccess(ns, m, _) if self.enum_schema(ns).is_some() => {
                    Some((ns.clone(), m.clone()))
                }
                _ => None,
            },
            Expression::NamespaceAccess(ns, m, _) if self.enum_schema(ns).is_some() => {
                Some((ns.clone(), m.clone()))
            }
            Expression::Identifier(n, _) => self.var_variant.get(n).cloned(),
            _ => None,
        }
    }

    /// `var Enum::variant(a, b) = valor;` (F2, estático). Valida forma, variante,
    /// aridad y tipos; exige que la variante sea **demostrable** (construcción
    /// directa de la MISMA variante o variable que la guarda). `_` ignora slots.
    pub(crate) fn check_destructure_let(&mut self, d: &DestructureLet) {
        let vt = self.check_expression(&d.value);
        let Some(schema) = self.enum_schema(&d.enum_name) else {
            self.error(
                &format!(
                    "destructuring-let: `{}` no es un enum conocido",
                    d.enum_name
                ),
                d.span.clone(),
            );
            return;
        };
        let Some((_, params)) = schema.iter().find(|(v, _)| v == &d.variant) else {
            self.error(
                &format!(
                    "La variante `{}` no existe en el enum `{}`",
                    d.variant, d.enum_name
                ),
                d.span.clone(),
            );
            for b in &d.binds {
                if b != "_" {
                    self.define(b, Type::Any);
                }
            }
            return;
        };
        if params.len() != d.binds.len() {
            self.error(
                &format!(
                    "La variante `{}` del enum `{}` espera {} binding(s), recibió {}",
                    d.variant,
                    d.enum_name,
                    params.len(),
                    d.binds.len()
                ),
                d.span.clone(),
            );
        }
        // Provabilidad: la variante debe ser demostrable estáticamente.
        let same_enum = |a: &str, b: &str| {
            a == b
                || a.ends_with(&format!("::{}", b))
                || b.ends_with(&format!("::{}", a))
        };
        match self.construction_variant(&d.value) {
            Some((e, v)) if same_enum(&e, &d.enum_name) && v == d.variant => {}
            _ => {
                self.error(
                    &format!(
                        "No se puede probar que el valor sea `{}::{}` (destructuring-let estático): \
                         inicializalo con una construcción de la misma variante \
                         (`var x = {}::{}(...); var {}::{}(...) = x;`)",
                        d.enum_name, d.variant, d.enum_name, d.variant, d.enum_name, d.variant
                    ),
                    d.span.clone(),
                );
            }
        }
        // Definir los binds con los tipos de los params.
        let _ = vt;
        for (i, b) in d.binds.iter().enumerate() {
            if b == "_" {
                continue;
            }
            let bt = params.get(i).cloned().unwrap_or(Type::Any);
            self.define(b, bt);
        }
    }
}

/// Es el tipo claramente no-enum? (para el error de scrutinee del modo PATRON).
fn is_obviously_non_enum(t: &Type) -> bool {
    matches!(
        t,
        Type::Int
            | Type::I8
            | Type::I16
            | Type::I32
            | Type::I64
            | Type::Float
            | Type::F32
            | Type::F64
            | Type::String
            | Type::Bool
            | Type::Char
            | Type::Array(_)
            | Type::Tuple(_)
            | Type::Record(_, _)
            | Type::Fun(_, _)
            | Type::Literal(_)
            | Type::Null
            | Type::Void
            | Type::Empty
    )
}

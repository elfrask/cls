//! containers.rs (Fase 1: extraido de cls-core/src/middleware/typeck/expressions.rs).

use super::*;

impl TypeChecker {



    pub(crate) fn check_index(&mut self, idx: &IndexExpr) -> Type {
        let obj = self.check_expression(&idx.object);
        let index_type = self.check_expression(&idx.index);
        match obj {
            Type::Array(inner) => *inner,
            Type::Record(_k, v) => *v,
            // JSON / Value / Cmx: el índice (clave u offset) devuelve un `Value`
            // dinámico; el tag runtime viaja con el valor.
            Type::Json | Type::Value | Type::Cmx => Type::Value,
            // Shape: índice literal con clave conocida -> tipo del campo; clave
            // desconocida -> error (la estructura del record es fija).
            Type::Shape(fields) => {
                match idx.index.as_ref() {
                    Expression::Literal(l) if matches!(l.kind, LiteralKind::String(_)) => {
                        let k = match &l.kind { LiteralKind::String(s) => s.clone(), _ => String::new() };
                        fields.iter()
                            .find(|(n, _)| *n == k)
                            .map(|(_, t)| t.clone())
                            .unwrap_or_else(|| self.error(
                                &format!("El record no tiene el campo '{}'", k),
                                idx.span.clone(),
                            ))
                    }
                    _ => Type::Any,
                }
            }
            // Tupla: índice literal -> slot exacto; dinámico -> unión de slots
            Type::Tuple(ts) => {
                match idx.index.as_ref() {
                    Expression::Literal(l) if matches!(l.kind, LiteralKind::Int(_)) => {
                        let i = match &l.kind { LiteralKind::Int(n) => *n as usize, _ => 0 };
                        ts.get(i).cloned().unwrap_or(Type::Any)
                    }
                    _ => {
                        if ts.is_empty() { Type::Any }
                        else { Type::Union(ts.clone()) }
                    }
                }
            }
            Type::Union(us) => Type::Union(
                us.iter().map(|u| match u {
                    Type::Array(inner) => (**inner).clone(),
                    Type::Tuple(ts) => ts.first().cloned().unwrap_or(Type::Any),
                    _ => Type::Any,
                }).collect(),
            ),
            _ => {
                // Magic method __get: clase con __get -> tipo de su retorno.
                if let Some(ret) = self.named_magic_ret(&obj, "__get") {
                    return ret;
                }
                let _ = index_type;
                Type::Any
            }
        }
    }



    pub(crate) fn check_array(&mut self, arr: &ArrayExpr) -> Type {
        let mut elem_type = Type::Any;
        for elem in &arr.elements {
            let t = self.check_expression(elem);
            if matches!(elem_type, Type::Any) {
                elem_type = t;
            } else if !t.is_assignable_to(&elem_type) && !elem_type.is_assignable_to(&t) {
                // Array heterogéneo: en CLS tipado no se permite mezclar tipos
                // incompatibles en un array literal (paridad con el JIT, que no
                // puede emitir layouts mixtos). El walker lo tolera; el JIT no.
                self.error(
                    &format!(
                        "Array heterogéneo: los elementos son de tipos incompatibles \
                         ({} y {}). Usa `Record<String, any>` o un array homogéneo.",
                        elem_type, t
                    ),
                    arr.span.clone(),
                );
                elem_type = t;
            } else if !t.is_assignable_to(&elem_type) && elem_type.is_assignable_to(&t) {
                // Compatible por promoción: `[1, 2.0]` -> el array es de Float
                // (el Int se promueve en emisión). íšltimo tipo más específico.
                elem_type = t;
            }
        }
        Type::Array(Box::new(elem_type))
    }



    pub(crate) fn check_tuple(&mut self, tup: &TupleExpr) -> Type {
        let types: Vec<Type> = tup.elements.iter()
            .map(|e| self.check_expression(e))
            .collect();
        Type::Tuple(types)
    }



    pub(crate) fn check_record(&mut self, rec: &RecordExpr) -> Type {
        // Spread `{...expr, ...}`: el tipo resultante es Record<String, Value>
        // (campos dinámicos — no se conocen estáticamente). Los spreads se
        // chequean; las entries extra se evalúan y el conjunto tipa dinámico.
        // Ver REST_SPREAD_PLAN Fase 2.
        if !rec.spreads.is_empty() {
            for s in &rec.spreads {
                self.check_expression(s);
            }
            for (_, expr) in &rec.entries {
                self.check_expression(expr);
            }
            let r = Type::Record(Box::new(Type::String), Box::new(Type::Value));
            self.types_by_span.insert(rec.span.clone(), r.clone());
            return r;
        }
        let mut fields: Vec<(String, Type)> = Vec::new();
        // Si el span ya tiene un tipo esperado (p.ej. `var d: Record<K,V> = {...}`
        // o `return {...}` con función tipada Record), propagarlo: el literal
        // interno con valor Record hereda el tipo del valor esperado.
        let expected = self.types_by_span.get(&rec.span).cloned();
        let expected_value = match &expected {
            Some(Type::Record(_, v)) => Some((**v).clone()),
            _ => None,
        };
        for (key, expr) in &rec.entries {
            if let (Some(ev), Expression::Record(inner)) = (&expected_value, expr) {
                if matches!(ev, Type::Record(_, _)) || matches!(ev, Type::Shape(_)) {
                    self.types_by_span.insert(inner.span.clone(), ev.clone());
                }
            }
            let t = self.check_expression(expr);
            fields.push((key.clone(), t));
        }
        // Re-insertar el tipo esperado del contexto (Record<K,V>): `check_expression`
        // de los valores puede haber sobreescrito el span con Shape (inferencia).
        if let Some(exp) = &expected {
            if matches!(exp, Type::Record(_, _)) {
                self.types_by_span.insert(rec.span.clone(), exp.clone());
            }
        }
        Type::Shape(fields)
    }



    pub(crate) fn check_arrow_function(&mut self, arrow: &ArrowFunctionExpr) -> Type {
        let param_types: Vec<Type> = arrow.params.iter()
            .map(|p| p.type_ann.as_ref()
                .map(|ta| self.resolve_type_annotation(ta))
                .unwrap_or(Type::Any))
            .collect();

        // Chequear params y body PRIMERO: así las variables declaradas dentro
        // del body (p.ej. `var inner = () -> ...`) quedan tipadas antes de
        // inferir el retorno (necesario para arrow-de-arrow con captura).
        // El retorno de la arrow se INFIERE del body: no debe validarse contra
        // el `current_return_type` de la función que la contiene.
        self.push_scope();
        let prev_return = self.current_return_type.take();
        for (param, typ) in arrow.params.iter().zip(param_types.iter()) {
            self.define(&param.name, typ.clone());
        }
        self.check_block(&arrow.body);
        self.current_return_type = prev_return;

        // Inferir el retorno del primer `return expr` del body. Leer del type map
        // (ya registrado por check_block) para no depender del scope actual.
        let return_type = arrow.return_type.as_ref()
            .map(|ta| self.resolve_type_annotation(ta))
            .unwrap_or_else(|| {
                let mut t = Type::Any;
                for stmt in &arrow.body.statements {
                    if let Statement::Return(Some(e)) = stmt {
                        let sp = expr_span(e);
                        if let Some(ty) = self.types_by_span.get(&sp) {
                            t = ty.clone();
                        } else {
                            t = self.check_expression(e);
                        }
                        break;
                    }
                }
                t
            });
        self.pop_scope();

        Type::Fun(param_types, Box::new(return_type))
    }



    pub(crate) fn check_conditional(&mut self, cond: &ConditionalExpr) -> Type {
        self.check_expression(&cond.condition);
        let then_type = self.check_expression(&cond.then_expr);
        let else_type = self.check_expression(&cond.else_expr);

        if then_type.is_assignable_to(&else_type) {
            then_type
        } else if else_type.is_assignable_to(&then_type) {
            else_type
        } else {
            Type::Any
        }
    }



    pub(crate) fn check_assignment(&mut self, assign: &AssignmentExpr) -> Type {
        let left = self.check_expression(&assign.target);
        let right = self.check_expression(&assign.value);

        if !right.is_assignable_to(&left) {
            let msg = format!("Tipo {} no asignable a {}", right, left);
            if self.config.strict {
                self.error(&msg, assign.span.clone());
            } else {
                self.warn(&msg, assign.span.clone());
            }
        }

        left
    }

}
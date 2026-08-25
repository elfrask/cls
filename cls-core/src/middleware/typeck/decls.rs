//! decls.rs (Fase 1: extraido de cls-core/src/middleware/typeck/statements.rs).

use super::*;

impl TypeChecker {



    pub(crate) fn check_var_decl(&mut self, var: &VarDecl, is_const: bool) -> Type {
        // Record literal con anotación (p.ej. `var d: Record<String, Int> = {a:1}`
        // o `var p: Persona = {nombre: "Ana", edad: 30}`): registrar el tipo
        // anotado en el span del literal ANTES de chequearlo, para que el backend
        // lo emita como dict (Record) o shape según lo que pida la anotación y
        // los literales internos hereden el tipo esperado (records anidados).
        let annotated = var.type_ann.as_ref().map(|t| self.resolve_type_annotation(t));
        if let (Some(declared), Some(Expression::Record(rec))) = (&annotated, &var.value) {
            if matches!(declared, Type::Record(_, _)) || matches!(declared, Type::Shape(_)) {
                self.types_by_span.insert(rec.span.clone(), declared.clone());
            }
        }
        let mut inferred = var.value.as_ref()
            .map(|e| self.check_expression(e))
            .unwrap_or(Type::Null);

        // const infiere literal type para literales (nunca muta)
        if is_const {
            if let Some(Expression::Literal(lit)) = &var.value {
                inferred = self.literal_type(&lit.kind);
            }
        }

        // Para verificar assignability, usar el literal type si el valor es un literal
        let check_type = if let Some(Expression::Literal(lit)) = &var.value {
            self.literal_type(&lit.kind)
        } else {
            inferred.clone()
        };

        let declared = annotated.unwrap_or_else(|| inferred.clone());

        if self.config.strict && var.value.is_some() && !check_type.is_assignable_to(&declared) {
            return self.error(
                &format!("No se puede asignar {} a {}", inferred, declared),
                var.span.clone(),
            );
        }

        if self.config.null_safety && matches!(inferred, Type::Null) && !matches!(declared, Type::Any) {
            self.warn(
                &format!("Posible null asignado a {} '{}'", declared, var.name),
                var.span.clone(),
            );
        }

        // Variable duplicada en el scope actual -> error (paridad con el
        // resolver: no se puede redeclarar una variable en el mismo bloque).
        if let Some(scope) = self.scopes.last() {
            if scope.contains_key(&var.name) {
                return self.error(
                    &format!("El nombre '{}' ya está declarado en este scope", var.name),
                    var.span.clone(),
                );
            }
        }
        self.define(&var.name, declared.clone());
        // Registrar el tipo declarado en el span de la declaración (REPL con
        // estado persistente: los hoists quedan sin init y el backend necesita
        // el tipo vía type map en el span del VarDecl).
        self.types_by_span.insert(var.span.clone(), declared.clone());
        // Array literal vacío con anotación (p.ej. `const out: int[] = []`):
        // registrar el tipo anotado en el span del literal para que el backend
        // sepa el tipo del elemento (check_array infiere Any, sin elementos).
        if let Some(Expression::Array(arr)) = &var.value {
            if arr.elements.is_empty() {
                if let Some(declared_arr) = var.type_ann.as_ref().map(|t| self.resolve_type_annotation(t)) {
                    self.types_by_span.insert(arr.span.clone(), declared_arr);
                }
            }
        }
        declared
    }



    pub(crate) fn check_function_decl(&mut self, func: &FunctionDecl) -> Type {
        // Scope con type params como placeholders (Named) para genéricos
        self.push_scope();
        for tp in &func.type_params {
            self.define(&tp.name, Type::Named(tp.name.clone(), vec![]));
        }

        let return_type = func.return_type.as_ref()
            .map(|t| self.resolve_type_annotation(t))
            .unwrap_or(Type::Void);

        let param_types: Vec<(String, Type)> = func.params.iter()
            .map(|p| {
                let t = p.type_ann.as_ref()
                    .map(|ta| self.resolve_type_annotation(ta))
                    .unwrap_or(Type::Any);
                (p.name.clone(), t)
            })
            .collect();

        let fn_type = Type::Fun(
            param_types.iter().map(|(_, t)| t.clone()).collect(),
            Box::new(return_type.clone()),
        );

        // Verificar cuerpo con params y placeholders en scope
        let prev_return = self.current_return_type.replace(return_type.clone());
        let prev_fn_span = self.current_fn_span.clone();
        self.current_fn_span = func.span.clone();
        for (name, typ) in &param_types {
            self.define(name, typ.clone());
        }
        // Registrar la función ANTES de chequear el cuerpo -> permite recursión
        self.define(&func.name, fn_type.clone());
        self.check_block(&func.body);
        self.pop_scope();
        self.current_return_type = prev_return;
        self.current_fn_span = prev_fn_span;

        self.define_decl(&func.name, fn_type, &func.span);
        return_type
    }

}
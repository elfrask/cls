//! TypeChecker - check_expression y chequeos de expresiones (Fase 1: extraido de middleware/typeck.rs).

use super::*;

impl TypeChecker {



    pub(crate) fn check_expression(&mut self, expr: &Expression) -> Type {
        let span = expr_span(expr);
        let t = match expr {
            Expression::Literal(l) => self.check_literal(l),
            Expression::Identifier(name, span) => {
                // Primero: ¿es una variable local declarada (incluso si se llama
                // `fs`, `math`, `json`)? El scope local gana sobre los módulos
                // internos del nodo (json/math/fs/http).
                if let Some(t) = self.lookup(name) {
                    t.clone()
                } else if matches!(
                    name.as_str(),
                    "json" | "math" | "fs" | "http" | "Lib" | "async" | "os" | "path"
                        | "process" | "time" | "random"
                ) {
                    // Módulos internos del nodo: no son variables, pero se aceptan
                    // como namespace (el backend los resuelve).
                    Type::Any
                } else {
                    self.lookup(name)
                        .cloned()
                        .unwrap_or_else(|| {
                            if self.config.no_implicit_any {
                                self.error(
                                    &format!("Variable no definida: {}", name),
                                    span.clone(),
                                )
                            } else {
                                Type::Any
                            }
                        })
                }
            }
            Expression::Binary(b) => self.check_binary(b),
            Expression::Unary(u) => self.check_unary(u),
            Expression::Call(c) => self.check_call(c),
            Expression::MemberAccess(m) => self.check_member_access(m),
            Expression::Index(i) => self.check_index(i),
            Expression::Array(a) => self.check_array(a),
            Expression::Tuple(t) => self.check_tuple(t),
            Expression::Record(r) => self.check_record(r),
            Expression::ArrowFunction(a) => self.check_arrow_function(a),
            Expression::Conditional(c) => self.check_conditional(c),
            Expression::Assignment(a) => self.check_assignment(a),
            Expression::Parenthesized(inner, _) => self.check_expression(inner),
            Expression::StringInterpolation(s) => {
                for part in &s.parts {
                    if let InterpolationPart::Expr(e) = part {
                        self.check_expression(e);
                    }
                }
                Type::String
            }
            Expression::Cmx(c) => {
                // Chequear las subexpresiones internas (attrs y children) para que
                // sus spans queden en el type map (el emisor las evalúa).
                for attr in &c.attributes {
                    if let Some(CmxAttributeValue::Expression(expr)) = &attr.value {
                        self.check_expression(expr);
                    }
                }
                for child in &c.children {
                    match child {
                        CmxChild::Expression(expr) => {
                            self.check_expression(expr);
                        }
                        CmxChild::Element(el) => {
                            self.check_expression(&Expression::Cmx((**el).clone()));
                        }
                        _ => {}
                    }
                }
                Type::Cmx
            }
            Expression::NamespaceAccess(ns, name, span) => {
                // `x::miembro` de un módulo importado -> tipo del export.
                match self.module_member_type(ns, name) {
                    Some(t) => t,
                    None => {
                        let available = self.module_export_names(ns);
                        let hint = if available.is_empty() {
                            "el módulo no exporta ningún símbolo (usa `export` en cada declaración)".to_string()
                        } else {
                            format!("el módulo exporta: {}", available.join(", "))
                        };
                        self.error(
                            &format!(
                                "'{}' no existe o no se exporta en '{}' ({})",
                                name, ns, hint
                            ),
                            span.clone(),
                        )
                    }
                }
            }
            Expression::Await(expr, _) => self.check_expression(expr),
        };
        if self.config.check {
            // Un literal de record anotado como Record<K,V> (var/return) registra
            // el tipo esperado en su span ANTES de chequearse; la inferencia aquí
            // produce Shape. Mantener el Record anotado (el backend lo emite como
            // dict con keys - necesario para el marshalling del binding).
            let prev = self.types_by_span.get(&span).cloned();
            if matches!(&prev, Some(Type::Record(_, _))) && matches!(&t, Type::Shape(_)) {
                self.types_by_span.insert(span, prev.unwrap());
            } else {
                self.types_by_span.insert(span, t.clone());
            }
        }
        t
    }



    pub(crate) fn check_literal(&mut self, lit: &Literal) -> Type {
        match &lit.kind {
            LiteralKind::Int(_) => Type::Int,
            LiteralKind::Float(_) => Type::Float,
            LiteralKind::String(_) => Type::String,
            LiteralKind::Bool(_) => Type::Bool,
            LiteralKind::Char(_) => Type::Char,
            LiteralKind::Null => Type::Null,
            LiteralKind::Unknown => Type::Unknown,
        }
    }

}

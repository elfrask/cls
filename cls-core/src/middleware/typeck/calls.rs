//! calls.rs (Fase 1: extraido de cls-core/src/middleware/typeck/expressions.rs).

use super::*;

impl TypeChecker {



    pub(crate) fn check_call(&mut self, call: &CallExpr) -> Type {
        let callee_type = self.check_expression(&call.callee);

        // Métodos de primitivos (callee MemberAccess): el tipo del miembro ES el
        // resultado (`.join(sep)` -> String, `.contains(x)` -> Bool, ...).
        if let Expression::MemberAccess(m) = &*call.callee {
            // `app.tag()`: el tag de un Cmx es invocable si es mayúscula (handle
            // de función). El runtime despacha por tag-bit; aquí permitimos la
            // llamada y tipamos el retorno como Value (dinámico). Réplica del
            // patrón JSON/Value (ver plan completar-tipo-cmx.md).
            let obj_ty0 = self.check_expression(&m.object);
            if matches!(obj_ty0, Type::Cmx) && m.member == "tag" {
                for arg in &call.args {
                    self.check_expression(arg);
                }
                return Type::Value;
            }
            // Array.map(f) -> Array(retorno de f)
            let obj_ty = obj_ty0;
            if matches!(&obj_ty, Type::Array(_)) && m.member == "map" {
                for arg in &call.args {
                    self.check_expression(arg);
                }
                if let Some(arg0) = call.args.first() {
                    if let Type::Fun(_, ret) = self.check_expression(arg0) {
                        return Type::Array(ret);
                    }
                }
                return obj_ty;
            }
            for arg in &call.args {
                self.check_expression(arg);
            }
            // `math.abs` devuelve el tipo del primer argumento (int->Int, float->Float);
            // `math.pow` SIEMPRE devuelve Float (el walker usa `powf` incondicional
            // y el emisor emite MathPow f64). Paridad con module_call_ret del backend.
            if let Expression::Identifier(obj, _) = &*m.object {
                // Validar aridad de los módulos internos del nodo (os/path/process/
                // time/random): el emisor accede a c.args[i] y un índice fuera de
                // rango paniquea. Error de tipo claro aquí, antes de emitir.
                if matches!(obj.as_str(), "os" | "path" | "process" | "time" | "random") {
                    if let Some(arity) = module_arity(obj.as_str(), m.member.as_str()) {
                        if call.args.len() != arity {
                            self.error(
                                &format!(
                                    "{}.{} esperaba {} argumento(s), recibió {}",
                                    obj,
                                    m.member,
                                    arity,
                                    call.args.len()
                                ),
                                call.span.clone(),
                            );
                        }
                    }
                }
                if obj == "math" {
                    if m.member == "pow" {
                        return Type::Float;
                    }
                    if m.member == "abs" {
                        if let Some(arg0) = call.args.first() {
                            let at = self.check_expression(arg0);
                            if matches!(at, Type::Float | Type::F32 | Type::F64) {
                                return Type::Float;
                            }
                        }
                        return Type::Int;
                    }
                }
            }
            // Llamar una función como valor (`app.tag()`, `f()`): el resultado es
            // su retorno, no el tipo de la función.
            return match callee_type {
                Type::Fun(_, ret) => *ret,
                t => t,
            };
        }

        // Verificar args y recolectar tipos (para inferir genéricos)
        let arg_types: Vec<Type> = call.args.iter()
            .map(|a| self.check_expression(a))
            .collect();

        match callee_type {
            Type::Fun(params, ret) => {
                // print es variádico; no validar arity
                let is_print = matches!(&*call.callee, Expression::Identifier(n, _) if n == "print");
                if self.config.strict && !is_print && params.len() != call.args.len() {
                    self.warn(
                        &format!(
                            "Función espera {} args, recibió {}",
                            params.len(),
                            call.args.len()
                        ),
                        call.span.clone(),
                    );
                }
                // Inferir genéricos desde los args: param Named("T") -> arg
                let mut bindings = HashMap::new();
                for (param, arg) in params.iter().zip(arg_types.iter()) {
                    if let Type::Named(n, ps) = param {
                        if ps.is_empty() && !matches!(arg, Type::Any) {
                            bindings.entry(n.clone()).or_insert_with(|| arg.clone());
                        }
                    }
                }
                // Validar que cada argumento sea asignable a su parámetro
                // (firma conocida). No aplica a print (variádico) ni a los
                // métodos de primitivos (MemberAccess ya retornó arriba).
                for (i, (param, arg_ty)) in params.iter().zip(arg_types.iter()).enumerate() {
                    let param_subst = self.substitute(param, &bindings);
                    // Sin firma útil (Any/huecos/genérico sin binding) -> no validar.
                    if matches!(param_subst, Type::Any | Type::Unknown)
                        || matches!(arg_ty, Type::Any | Type::Unknown)
                        || self.has_unbound_generic(&param_subst, &bindings)
                    {
                        continue;
                    }
                    // El tipo del literal se usa como literal type para respetar
                    // uniones de literales y promociones implícitas (int->float).
                    let arg_check = match &call.args[i] {
                        Expression::Literal(l) => self.literal_type(&l.kind),
                        _ => arg_ty.clone(),
                    };
                    if !arg_check.is_assignable_to(&param_subst) {
                        let msg = format!(
                            "Se esperaba {}, recibió {} en el argumento {}",
                            param_subst,
                            arg_ty,
                            i + 1
                        );
                        let span = expr_span(&call.args[i]);
                        if self.config.strict {
                            self.error(&msg, span);
                        } else {
                            self.warn(&msg, span);
                        }
                    }
                }
                self.substitute(&ret, &bindings)
            }
            Type::Named(_, _) => {
                // ¿Constructor de clase/struct? - el callee es el NOMBRE de la
                // clase (Identifier) o `lib::Clase` (NamespaceAccess) -> devuelve
                // el tipo de la clase.
                let is_ctor = match &*call.callee {
                    Expression::Identifier(n, _) => {
                        self.class_members.contains_key(n)
                            || self.struct_members.contains_key(n)
                    }
                    Expression::NamespaceAccess(_, member, _) => {
                        self.class_members.contains_key(member)
                            || self.struct_members.contains_key(member)
                    }
                    _ => false,
                };
                if is_ctor {
                    return callee_type.clone();
                }
                // Objeto callable (magic __call): el callee es una expresión cuyo
                // tipo es una clase con __call -> tipo del retorno del __call.
                // Aridad validada contra la firma declarada (M3: args extra
                // producían basura de memoria).
                if let Some(ret) = self.named_magic_ret(&callee_type, "__call") {
                    if let Some(params) = self.magic_params_for(&callee_type, "__call") {
                        if params.len() != call.args.len() {
                            self.error(
                                &format!(
                                    "'__call' de '{}' esperaba {} argumento(s), recibió {}",
                                    match &callee_type {
                                        Type::Named(cn, _) => cn.as_str(),
                                        _ => "?",
                                    },
                                    params.len(),
                                    call.args.len()
                                ),
                                call.span.clone(),
                            );
                        }
                    }
                    return ret;
                }
                callee_type.clone()
            }
            Type::Any => Type::Any,
            _ => self.error(
                &format!("No se puede llamar como función: {}", callee_type),
                call.span.clone(),
            ),
        }
    }

}
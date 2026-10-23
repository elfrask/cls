//! calls.rs (Fase 1: extraido de cls-core/src/middleware/typeck/expressions.rs).

use super::*;

impl TypeChecker {



    /// Construcción de una variante de enum (F2): valida variante, aridad y
    /// tipos de los args contra el esquema. Devuelve `Named(enum)`.
    fn check_enum_construct(
        &mut self,
        enum_name: &str,
        member: &str,
        schema: &[(String, Vec<Type>)],
        args: &[Expression],
        span: &Span,
    ) -> Type {
        let named = Type::Named(enum_name.to_string(), vec![]);
        let Some((_, params)) = schema.iter().find(|(v, _)| v == member) else {
            self.error(
                &format!("La variante `{}` no existe en el enum `{}`", member, enum_name),
                span.clone(),
            );
            for a in args {
                self.check_expression(a);
            }
            return named;
        };
        if params.len() != args.len() {
            self.error(
                &format!(
                    "La variante `{}` del enum `{}` espera {} argumento(s), recibió {}",
                    member,
                    enum_name,
                    params.len(),
                    args.len()
                ),
                span.clone(),
            );
        }
        for (i, a) in args.iter().enumerate() {
            let at = self.check_expression(a);
            if let Some(pt) = params.get(i) {
                if self.config.strict && !at.is_assignable_to(pt) {
                    self.error(
                        &format!(
                            "Argumento {} de `{}::{}` no asignable a `{}` (encontró `{}`)",
                            i + 1,
                            enum_name,
                            member,
                            pt,
                            at
                        ),
                        expr_span(a),
                    );
                }
            }
        }
        named
    }

    /// Tipo de un `X::v` (variante de enum como valor). Valida que la variante
    /// exista; si tiene payload, no puede usarse desnuda (falta la llamada).
    pub(crate) fn check_enum_variant_value(
        &mut self,
        enum_name: &str,
        member: &str,
        span: &Span,
    ) -> Type {
        let named = Type::Named(enum_name.to_string(), vec![]);
        if let Some(schema) = self.enum_schema(enum_name) {
            match schema.iter().find(|(v, _)| v == member) {
                Some((_, params)) => {
                    if !params.is_empty() {
                        self.error(
                            &format!(
                                "La variante `{}` del enum `{}` tiene payload; construila con `{}::{}(...)`",
                                member, enum_name, enum_name, member
                            ),
                            span.clone(),
                        );
                    }
                }
                None => {
                    self.error(
                        &format!("La variante `{}` no existe en el enum `{}`", member, enum_name),
                        span.clone(),
                    );
                }
            }
        }
        named
    }

    pub(crate) fn check_call(&mut self, call: &CallExpr) -> Type {
        // Construcción de variante de enum con payload (F2): `X::v(args)`.
        // Validación estática de variante/aridad/tipos; el tipo es `Named(X)`.
        if let Expression::NamespaceAccess(ns, member, span) = &*call.callee {
            if let Some(schema) = self.enum_schema(ns) {
                return self.check_enum_construct(ns, member, &schema, &call.args, span);
            }
        }
        let callee_type = self.check_expression(&call.callee);

        // Métodos de primitivos (callee MemberAccess): el tipo del miembro ES el
        // resultado (`.join(sep)` -> String, `.contains(x)` -> Bool, ...).
        if let Expression::MemberAccess(m) = &*call.callee {
            // `app.tag()`: el tag de un Cmx es invocable si es mayúscula (handle
            // de función). El runtime despacha por tag-bit; aquí permitimos la
            // llamada y tipamos el retorno como Value (dinámico). Réplica del
            // patrón JSON/Value (ver plan completar-tipo-cmx.md).
            // Métodos de CMX (rework): los definidos en la clase core `CMX`
            // (frontend/builtins) + los nativos que quedan (toString/cloneWith).
            let obj_ty0 = self.check_expression(&m.object);
            if matches!(obj_ty0, Type::Cmx) {
                for arg in &call.args {
                    self.check_expression(arg);
                }
                if let Some(ms) = self.class_members.get("CMX") {
                    if let Some(ret) = ms.get(m.member.as_str()) {
                        return ret.clone();
                    }
                }
                match m.member.as_str() {
                    "tagCallable" => return Type::Value,
                    "toString" | "__str" | "__repr" => return Type::String,
                    "cloneWith" => return Type::Cmx,
                    "tryCallTag" => return Type::Value,
                    _ => {}
                }
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
            // Aridad de los métodos de String (el emisor accede a c.args[i] y
            // un índice fuera de rango paniquea; error de tipo claro aquí).
            if matches!(&obj_ty, Type::String) {
                let arity = match m.member.as_str() {
                    "slice" => Some(2usize),
                    "indexOf" | "split" | "contains" | "startsWith" | "endsWith" => Some(1),
                    "upper" | "lower" | "trim" | "isEmpty" | "toString" | "length" => Some(0),
                    _ => None,
                };
                if let Some(a) = arity {
                    if call.args.len() != a {
                        self.error(
                            &format!(
                                "String.{} esperaba {} argumento(s), recibió {}",
                                m.member,
                                a,
                                call.args.len()
                            ),
                            call.span.clone(),
                        );
                    }
                }
            }
            // `math.abs` devuelve el tipo del primer argumento (int->Int, float->Float);
            // `math.pow` SIEMPRE devuelve Float (el walker usa `powf` incondicional
            // y el emisor emite MathPow f64). Paridad con module_call_ret del backend.
            if let Expression::Identifier(obj, _) = &*m.object {
                // Resolver alias de import (`import "math" as m` -> `m` es math).
                let canonical = self
                    .import_aliases
                    .get(obj)
                    .cloned()
                    .unwrap_or_else(|| obj.clone());
                let obj = &canonical;
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
            // su retorno, no el tipo de la función. Un `Callable` no expone su
            // firma en compile-time -> resultado dinámico (`Any`, tag int).
            return match callee_type {
                Type::Fun(_, ret) => *ret,
                Type::Callable => Type::Value,
                t => t,
            };
        }

        // Verificar args y recolectar tipos (para inferir genéricos)
        let arg_types: Vec<Type> = call.args.iter()
            .map(|a| self.check_expression(a))
            .collect();

        // B2: conversiones/formatos intrínsecos (`str`/`toString`/`int`/`float`)
        // sobre una unión de builtins. El emisor elige el despacho por el tipo
        // estático (`__intr_str_int`, `__intr_str_float`, `emit_to_int`, …);
        // una unión mezcla representaciones (int vs float) y no es discriminable
        // sin tag/boxing. Antes: WASM inválido (`expected i64, found f64`) o
        // conversión silenciosa incorrecta. Rechazo estático claro.
        if let Expression::Identifier(name, _) = &*call.callee {
            if matches!(name.as_str(), "str" | "toString" | "int" | "float") {
                if let Some(arg0) = arg_types.first() {
                    let tagged_str = matches!(name.as_str(), "str" | "toString")
                        && arg0.is_tagged_builtin_union();
                    if self.union_has_undiscriminable_builtin(arg0) && !tagged_str {
                        self.error(
                            &format!(
                                "`{}(…)` sobre una unión de builtins ({}) no es discriminable en \
                                 runtime: usá un valor con tipo concreto",
                                name, arg0
                            ),
                            call.span.clone(),
                        );
                    }
                }
            }
        }

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
                // Activar sólo los type params del callee: así un tipo
                // desconocido en la firma no se confunde con un genérico.
                // Si el callee es una variable (`var f = id; f(3.5)`), no hay
                // nombre de función: se infieren candidatos desde su firma.
                let callee_name = match &*call.callee {
                    Expression::Identifier(n, _) => Some(n.clone()),
                    _ => None,
                };
                let known_fn = callee_name
                    .as_ref()
                    .is_some_and(|n| self.fn_type_params.contains_key(n));
                let mut callee_type_params = callee_name
                    .as_ref()
                    .and_then(|n| self.fn_type_params.get(n).cloned())
                    .unwrap_or_default();
                if !known_fn {
                    for p in &params {
                        self.collect_type_param_candidates(p, &mut callee_type_params);
                    }
                    self.collect_type_param_candidates(&ret, &mut callee_type_params);
                }
                let prev_type_params = self.active_type_params.len();
                for tp in callee_type_params {
                    if !self.active_type_params.contains(&tp) {
                        self.active_type_params.push(tp);
                    }
                }
                // Inferir genéricos desde los args unificando la forma completa
                // del parámetro (incluye Shape/Array/Record/Tuplas y campos de
                // interface como `ObjType<T>`).
                let mut bindings = HashMap::new();
                for (param, arg) in params.iter().zip(arg_types.iter()) {
                    self.infer_generic_bindings(param, arg, &mut bindings);
                }

                // Validar que cada argumento sea asignable a su parámetro
                // (firma conocida). No aplica a print (variádico) ni a los
                // métodos de primitivos (MemberAccess ya retornó arriba).
                for (i, (param, arg_ty)) in params.iter().zip(arg_types.iter()).enumerate() {
                    let param_subst = self.substitute(param, &bindings);
                    // Un genérico no inferido no exime de validar la forma
                    // externa: `ObjType<T>` -> `Shape{value:Value}`; así
                    // `whatType("abc")` da error en vez de WASM inválido.
                    let param_check = self.erase_unbound_generics(&param_subst, &bindings);
                    if matches!(param_check, Type::Value | Type::Unknown)
                        || matches!(arg_ty, Type::Value | Type::Unknown)
                    {
                        continue;
                    }
                    // El tipo del literal se usa como literal type para respetar
                    // uniones de literales y promociones implícitas (int->float).
                    let arg_check = match &call.args[i] {
                        Expression::Literal(l) => self.literal_type(&l.kind),
                        _ => arg_ty.clone(),
                    };
                    if !self.is_assignable(&arg_check, &param_check) {
                        let msg = format!(
                            "Tipo de parámetro inválido: se esperaba {}, se recibió {} en el argumento {}",
                            param_subst,
                            arg_ty,
                            i + 1
                        );
                        // No es un warning: un mismatch deja el WASM con una
                        // representación incorrecta (p. ej. f64 en un param
                        // i64). Debe bloquear la compilación.
                        self.error(&msg, expr_span(&call.args[i]));
                    }
                }
                self.active_type_params.truncate(prev_type_params);
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
                    let ctor_name = match &*call.callee {
                        Expression::Identifier(n, _) => n.as_str(),
                        Expression::NamespaceAccess(_, member, _) => member.as_str(),
                        _ => "",
                    };
                    if let Some(params) = self
                        .magic_params
                        .get(ctor_name)
                        .and_then(|members| members.get("main"))
                    {
                        let total = params.len();
                        let required = self
                            .ctor_required
                            .get(ctor_name)
                            .copied()
                            .unwrap_or(total);
                        if call.args.len() < required || call.args.len() > total {
                            let expected = if required == total {
                                format!("{}", total)
                            } else {
                                format!("entre {} y {}", required, total)
                            };
                            self.error(
                                &format!(
                                    "El constructor de '{}' espera {} argumento(s), recibió {}",
                                    ctor_name,
                                    expected,
                                    call.args.len()
                                ),
                                call.span.clone(),
                            );
                        }
                    } else if self.class_members.contains_key(ctor_name) && !call.args.is_empty() {
                        // La clase no declara constructor: solo `Clase()`.
                        self.error(
                            &format!(
                                "La clase '{}' no declara constructor, pero recibió {} argumento(s)",
                                ctor_name,
                                call.args.len()
                            ),
                            call.span.clone(),
                        );
                    }
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
            Type::Value => Type::Value,
            // Un valor `Callable` es invocable en runtime por tag-bit (handle de
            // función simple o closure); su firma/retorno no se conoce en
            // compile-time -> resultado dinámico (`Any`, tag int). Cubre
            // `Record<String,Callable>["k"](...)`, `Callable[i](...)` y campos
            // `var f: Callable` (`me.f(...)`).
            Type::Callable => Type::Value,
            _ => self.error(
                &format!("No se puede llamar como función: {}", callee_type),
                call.span.clone(),
            ),
        }
    }

}
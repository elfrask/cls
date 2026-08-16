//! TypeChecker â€” check_expression y chequeos de expresiones (Fase 1: extraido de middleware/typeck.rs).

use super::*;

impl TypeChecker {



    pub(crate) fn check_expression(&mut self, expr: &Expression) -> Type {
        let span = expr_span(expr);
        let t = match expr {
            Expression::Literal(l) => self.check_literal(l),
            Expression::Identifier(name, span) => {
                // Primero: Â¿es una variable local declarada (incluso si se llama
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
                // sus spans queden en el type map (el emisor las evalíºa).
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
                // `x::miembro` de un módulo importado â†’ tipo del export.
                match self.module_member_type(ns, name) {
                    Some(t) => t,
                    None => {
                        let available = self.module_export_names(ns);
                        let hint = if available.is_empty() {
                            "el módulo no exporta ningíºn sí­mbolo (usa `export` en cada declaración)".to_string()
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
            // el tipo esperado en su span ANTES de chequearse; la inferencia aquí­
            // produce Shape. Mantener el Record anotado (el backend lo emite como
            // dict con keys â€” necesario para el marshalling del binding).
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


    pub(crate) fn check_binary(&mut self, bin: &BinaryExpr) -> Type {
        use crate::frontend::token::Operator;

        // `is` con tipo builtin (`v is String`): el right es un nombre de tipo, no
        // una variable. Se registra el tipo del nombre en el span para el backend.
        let is_builtin_is = if bin.op == Operator::Is {
            match &*bin.right {
                Expression::Identifier(n, _) => builtin_type_name(n).is_some(),
                _ => false,
            }
        } else {
            false
        };

        let left = self.check_expression(&bin.left);
        let right = if is_builtin_is {
            if let Expression::Identifier(n, sp) = &*bin.right {
                let t = builtin_type_name(n).unwrap();
                self.types_by_span.insert(sp.clone(), t.clone());
                t
            } else {
                self.check_expression(&bin.right)
            }
        } else {
            self.check_expression(&bin.right)
        };

        match bin.op {
            Operator::Plus => {
                let is_str_l = matches!(left, Type::String);
                let is_str_r = matches!(right, Type::String);
                let is_num_l = matches!(left, Type::Int | Type::Float | Type::I32 | Type::I64);
                let is_num_r = matches!(right, Type::Int | Type::Float | Type::I32 | Type::I64);

                if is_str_l && is_str_r {
                    return Type::String;
                }
                if is_num_l && is_num_r {
                    if matches!(left, Type::Float) || matches!(right, Type::Float) {
                        return Type::Float;
                    }
                    return Type::Int;
                }
                // Int + Float â†’ Float
                if is_num_l && matches!(right, Type::Float) {
                    return Type::Float;
                }
                if matches!(left, Type::Float) && is_num_r {
                    return Type::Float;
                }
                // Magic method __add: clase con __add (left primero, luego right).
                // El operando se valida contra el parí¡metro del magic (M1: tipos
                // incompatibles producí­an basura de memoria).
                if self.named_magic_ret(&left, "__add").is_some() {
                    self.validate_magic_binary_operand(&left, &right, "__add", bin.span.clone());
                    return self.named_magic_ret(&left, "__add").unwrap();
                }
                if self.named_magic_ret(&right, "__add").is_some() {
                    self.validate_magic_binary_operand(&right, &left, "__add", bin.span.clone());
                    return self.named_magic_ret(&right, "__add").unwrap();
                }
                self.error(
                    &format!(
                        "Operador + no soportado entre {} y {} (en `{}`)",
                        left,
                        right,
                        format!(
                            "{} + {}",
                            expr_short_display(&bin.left),
                            expr_short_display(&bin.right)
                        )
                    ),
                    bin.span.clone(),
                )
            }
            Operator::Minus | Operator::Star | Operator::Slash | Operator::Percent | Operator::StarStar => {
                let l_ok = matches!(left, Type::Int | Type::Float | Type::I32 | Type::I64);
                let r_ok = matches!(right, Type::Int | Type::Float | Type::I32 | Type::I64);
                if l_ok && r_ok {
                    if matches!(left, Type::Float) || matches!(right, Type::Float) {
                        return Type::Float;
                    }
                    return Type::Int;
                }
                // Magic methods aritméticos: __sub/__mul/__div/__mod/__pow.
                let magic = match bin.op {
                    Operator::Minus => "__sub",
                    Operator::Star => "__mul",
                    Operator::Slash => "__div",
                    Operator::Percent => "__mod",
                    _ => "__pow",
                };
                // El operando se valida contra el parí¡metro del magic (M1).
                if self.named_magic_ret(&left, magic).is_some() {
                    self.validate_magic_binary_operand(&left, &right, magic, bin.span.clone());
                    return self.named_magic_ret(&left, magic).unwrap();
                }
                if self.named_magic_ret(&right, magic).is_some() {
                    self.validate_magic_binary_operand(&right, &left, magic, bin.span.clone());
                    return self.named_magic_ret(&right, magic).unwrap();
                }
                self.error(
                    &format!(
                        "Operador requiere tipos numéricos, encontró {} y {} (en `{} {} {}`)",
                        left,
                        right,
                        expr_short_display(&bin.left),
                        bin.op,
                        expr_short_display(&bin.right)
                    ),
                    bin.span.clone(),
                )
            }
            // Operadores bit a bit: exigen enteros y devuelven Int.
            Operator::Caret | Operator::ShiftLeft | Operator::ShiftRight => {
                let l_ok = matches!(left, Type::Int | Type::I32 | Type::I64 | Type::I8 | Type::I16);
                let r_ok = matches!(right, Type::Int | Type::I32 | Type::I64 | Type::I8 | Type::I16);
                if l_ok && r_ok {
                    return Type::Int;
                }
                self.error(
                    &format!("Operador bit a bit requiere enteros, encontró {} y {}", left, right),
                    bin.span.clone(),
                )
            }
            Operator::StrictEqual | Operator::NotEqual
            | Operator::LessThan | Operator::LessEqual
            | Operator::GreaterThan | Operator::GreaterEqual
            | Operator::In | Operator::Is => {
                // Validar los operandos del dispatch mí¡gico (M1/M4): tipos
                // incompatibles producí­an basura de memoria o WASM inví¡lido.
                match bin.op {
                    Operator::StrictEqual | Operator::NotEqual => {
                        if self.named_magic_ret(&left, "__equals").is_some() {
                            self.validate_magic_binary_operand(&left, &right, "__equals", bin.span.clone());
                        } else if self.named_magic_ret(&right, "__equals").is_some() {
                            self.validate_magic_binary_operand(&right, &left, "__equals", bin.span.clone());
                        }
                    }
                    Operator::LessThan | Operator::LessEqual
                    | Operator::GreaterThan | Operator::GreaterEqual => {
                        if self.named_magic_ret(&left, "__compare").is_some() {
                            self.validate_magic_binary_operand(&left, &right, "__compare", bin.span.clone());
                        } else if self.named_magic_ret(&right, "__compare").is_some() {
                            self.validate_magic_binary_operand(&right, &left, "__compare", bin.span.clone());
                        }
                    }
                    Operator::In => {
                        if self.named_magic_ret(&right, "__contains").is_some() {
                            self.validate_magic_binary_operand(&right, &left, "__contains", bin.span.clone());
                        }
                    }
                    _ => {}
                }
                Type::Bool
            }
            Operator::And | Operator::Or => {
                if !left.is_assignable_to(&Type::Bool) || !right.is_assignable_to(&Type::Bool) {
                    self.warn("Operador lógico requiere Bool", bin.span.clone());
                }
                Type::Bool
            }
            _ => Type::Any,
        }
    }


    pub(crate) fn check_unary(&mut self, un: &UnaryExpr) -> Type {
        let operand = self.check_expression(&un.operand);
        match un.op {
            UnaryOp::Negate => operand,
            UnaryOp::Not => Type::Bool,
            UnaryOp::BitwiseNot => Type::Int,
            UnaryOp::TypeOf => Type::String,
            UnaryOp::PostInc | UnaryOp::PreInc => Type::Int,
            UnaryOp::PostDec | UnaryOp::PreDec => Type::Int,
        }
    }


    pub(crate) fn check_call(&mut self, call: &CallExpr) -> Type {
        let callee_type = self.check_expression(&call.callee);

        // Métodos de primitivos (callee MemberAccess): el tipo del miembro ES el
        // resultado (`.join(sep)` â†’ String, `.contains(x)` â†’ Bool, ...).
        if let Expression::MemberAccess(m) = &*call.callee {
            // Array.map(f) â†’ Array(retorno de f)
            let obj_ty = self.check_expression(&m.object);
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
            // `math.abs` devuelve el tipo del primer argumento (intâ†’Int, floatâ†’Float);
            // `math.pow` SIEMPRE devuelve Float (el walker usa `powf` incondicional
            // y el emisor emite MathPow f64). Paridad con module_call_ret del backend.
            if let Expression::Identifier(obj, _) = &*m.object {
                // Validar aridad de los módulos internos del nodo (os/path/process/
                // time/random): el emisor accede a c.args[i] y un í­ndice fuera de
                // rango paniquea. Error de tipo claro aquí­, antes de emitir.
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
                // print es varií¡dico; no validar arity
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
                // Inferir genéricos desde los args: param Named("T") â†’ arg
                let mut bindings = HashMap::new();
                for (param, arg) in params.iter().zip(arg_types.iter()) {
                    if let Type::Named(n, ps) = param {
                        if ps.is_empty() && !matches!(arg, Type::Any) {
                            bindings.entry(n.clone()).or_insert_with(|| arg.clone());
                        }
                    }
                }
                // Validar que cada argumento sea asignable a su parí¡metro
                // (firma conocida). No aplica a print (varií¡dico) ni a los
                // métodos de primitivos (MemberAccess ya retornó arriba).
                for (i, (param, arg_ty)) in params.iter().zip(arg_types.iter()).enumerate() {
                    let param_subst = self.substitute(param, &bindings);
                    // Sin firma íºtil (Any/huecos/genérico sin binding) â†’ no validar.
                    if matches!(param_subst, Type::Any | Type::Unknown)
                        || matches!(arg_ty, Type::Any | Type::Unknown)
                        || self.has_unbound_generic(&param_subst, &bindings)
                    {
                        continue;
                    }
                    // El tipo del literal se usa como literal type para respetar
                    // uniones de literales y promociones implí­citas (intâ†’float).
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
                // Â¿Constructor de clase/struct? â€” el callee es el NOMBRE de la
                // clase (Identifier) â†’ devuelve el tipo de la clase.
                if let Expression::Identifier(n, _) = &*call.callee {
                    if self.class_members.contains_key(n) || self.struct_members.contains_key(n) {
                        return callee_type.clone();
                    }
                }
                // Objeto callable (magic __call): el callee es una expresión cuyo
                // tipo es una clase con __call â†’ tipo del retorno del __call.
                // Aridad validada contra la firma declarada (M3: args extra
                // producí­an basura de memoria).
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


    pub(crate) fn check_member_access(&mut self, member: &MemberAccessExpr) -> Type {
        // Módulos internos del nodo (resueltos por nombre en el JIT): se manejan
        // ANTES de evaluar el object (que no estí¡ definido como variable).
        if let Expression::Identifier(name, _) = &*member.object {
            if self.enums.contains(name) {
                return Type::Named(name.clone(), vec![]);
            }
            if name == "http" {
                return match member.member.as_str() {
                    "get" | "post" => Type::String,
                    _ => Type::Any,
                };
            }
            if name == "fs" {
                return match member.member.as_str() {
                    "exists" => Type::Bool,
                    "cwd" | "readFile" => Type::String,
                    "listDir" => Type::Array(Box::new(Type::String)),
                    _ => Type::Any,
                };
            }
            if name == "json" {
                return match member.member.as_str() {
                    // parse devuelve un Record<String, any> (para acceso por
                    // í­ndice obj["k"] y print). El layout del host es compatible.
                    "parse" => Type::Record(Box::new(Type::String), Box::new(Type::Any)),
                    "stringify" => Type::String,
                    _ => Type::Any,
                };
            }
            if name == "math" {
                return match member.member.as_str() {
                    "range" => Type::Array(Box::new(Type::Int)),
                    "random" => Type::Float,
                    "sqrt" | "floor" | "ceil" | "round" | "sin" | "cos" | "tan"
                    | "log" | "pow" | "min" | "max" => Type::Float,
                    "abs" => Type::Int,
                    _ => Type::Any,
                };
            }
            if name == "os" {
                return match member.member.as_str() {
                    "platform" | "arch" | "version" | "hostname" | "home"
                    | "tempdir" | "env" | "sep" => Type::String,
                    "cpus" | "pid" | "uptime" => Type::Int,
                    "isWindows" | "isUnix" => Type::Bool,
                    _ => Type::Any,
                };
            }
            if name == "path" {
                return match member.member.as_str() {
                    "join" | "basename" | "dirname" | "extname" | "resolve"
                    | "normalize" | "sep" => Type::String,
                    "isAbsolute" => Type::Bool,
                    _ => Type::Any,
                };
            }
            if name == "process" {
                return match member.member.as_str() {
                    "args" => Type::Array(Box::new(Type::String)),
                    "cwd" | "env" | "platform" | "title" => Type::String,
                    "pid" => Type::Int,
                    "exit" => Type::Void,
                    _ => Type::Any,
                };
            }
            if name == "time" {
                return match member.member.as_str() {
                    "iso" | "date" | "clock" => Type::String,
                    "now" | "seconds" | "year" | "month" | "day" | "hour"
                    | "minute" | "second" => Type::Int,
                    "sleep" => Type::Void,
                    _ => Type::Any,
                };
            }
            if name == "random" {
                return match member.member.as_str() {
                    "random" | "float" => Type::Float,
                    "int" => Type::Int,
                    "uuid" => Type::String,
                    _ => Type::Any,
                };
            }
        }
        let obj_type = self.check_expression(&member.object);
        // Color.Rojo â†’ el tipo del enum (si member.object es un nombre de enum)
        // Métodos/getters de primitivos (sin boxing): tipo conocido por miembro.
        match obj_type {
            Type::String => match member.member.as_str() {
                "length" => Type::Int,
                "upper" | "lower" | "trim" | "toString" => Type::String,
                "contains" | "startsWith" | "endsWith" | "isEmpty" => Type::Bool,
                _ => Type::Any,
            },
            Type::Array(elem) => match member.member.as_str() {
                "length" => Type::Int,
                "join" | "toString" => Type::String,
                "includes" | "isEmpty" => Type::Bool,
                "indexOf" => Type::Int,
                "push" | "pop" | "shift" | "unshift" | "reverse" => Type::Array(elem.clone()),
                _ => Type::Any,
            },
            Type::Tuple(_) => match member.member.as_str() {
                "length" => Type::Int,
                "join" | "toString" => Type::String,
                _ => Type::Any,
            },
            Type::Record(k, _) => match member.member.as_str() {
                "length" | "size" => Type::Int,
                "has" => Type::Bool,
                "keys" => Type::Array(k.clone()),
                "values" => Type::Array(Box::new(Type::Any)),
                "toString" => Type::String,
                _ => Type::Any,
            },
            Type::Shape(fields) => {
                match member.member.as_str() {
                    "length" | "size" => Type::Int,
                    "keys" => Type::Array(Box::new(Type::String)),
                    "values" => Type::Array(Box::new(Type::Any)),
                    "has" => Type::Bool,
                    "toString" => Type::String,
                    name => fields.iter()
                        .find(|(n, _)| *n == name)
                        .map(|(_, t)| t.clone())
                        .unwrap_or_else(|| self.error(
                            &format!("El record no tiene el campo '{}'", name),
                            member.span.clone(),
                        )),
                }
            }
            Type::Cmx => match member.member.as_str() {
                "tag" => Type::Fun(vec![Type::Any], Box::new(Type::String)),
                "props" => Type::Record(Box::new(Type::String), Box::new(Type::Any)),
                "children" => Type::Array(Box::new(Type::Cmx)),
                _ => Type::Any,
            },
            Type::Int | Type::Float => match member.member.as_str() {
                "toString" => Type::String,
                "abs" => obj_type,
                _ => Type::Any,
            },
            Type::Bool | Type::Char => match member.member.as_str() {
                "toString" => Type::String,
                _ => Type::Any,
            },
            Type::Named(name, _) => {
                if let Some(members) = self.class_members.get(name.as_str()) {
                    if let Some(t) = members.get(&member.member) {
                        return t.clone();
                    }
                }
                // Campo de structure: `p.campo` â†’ tipo anotado del campo.
                if let Some(members) = self.struct_members.get(name.as_str()) {
                    if let Some(t) = members.get(&member.member) {
                        return t.clone();
                    }
                }
                // `Color.Rojo` / `lib::Color.Rojo` â†’ la variante de enum es
                // del mismo tipo (identidad con nombre del enum).
                if self.enums.contains(name.as_str()) {
                    return Type::Named(name.clone(), vec![]);
                }
                // Módulo/namespace importado: `x::miembro`.
                if let Some(t) = self.module_member_type(name.as_str(), &member.member) {
                    return t;
                }
                Type::Any
            }
            _ => Type::Any,
        }
    }


    pub(crate) fn check_index(&mut self, idx: &IndexExpr) -> Type {
        let obj = self.check_expression(&idx.object);
        let index_type = self.check_expression(&idx.index);
        match obj {
            Type::Array(inner) => *inner,
            Type::Record(_k, v) => *v,
            // Shape: í­ndice literal con clave conocida â†’ tipo del campo; clave
            // desconocida â†’ error (la estructura del record es fija).
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
            // Tupla: í­ndice literal â†’ slot exacto; diní¡mico â†’ unión de slots
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
                // Magic method __get: clase con __get â†’ tipo de su retorno.
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
                // Compatible por promoción: `[1, 2.0]` â†’ el array es de Float
                // (el Int se promueve en emisión). íšltimo tipo mí¡s especí­fico.
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

        // Chequear params y body PRIMERO: así­ las variables declaradas dentro
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
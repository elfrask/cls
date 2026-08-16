//! TypeChecker - check_statement y chequeos de statements (Fase 1: extraido de middleware/typeck.rs).

use super::*;

impl TypeChecker {



    pub(crate) fn check_statement(&mut self, stmt: &Statement) -> Type {
        match stmt {
            Statement::VarDecl(v) => self.check_var_decl(v, false),
            Statement::ConstDecl(v) => self.check_var_decl(v, true),
            Statement::FunctionDecl(f) => self.check_function_decl(f),
            Statement::If(i) => self.check_if(i),
            Statement::While(w) => self.check_while(w),
            Statement::Loop(b) => {
                self.check_block(b);
                Type::Void
            }
            Statement::For(f) => self.check_for(f),
            Statement::ForEach(fe) => self.check_foreach(fe),
            Statement::Switch(s) => self.check_switch(s),
            Statement::Try(t) => self.check_try(t),
            Statement::With(w) => self.check_with(w),
            Statement::Return(expr) => {
                // Literal de record en return con función declarada como
                // Record<K,V> (o Shape): registrar el tipo esperado en el span
                // del literal ANTES de chequearlo, para que el backend lo emita
                // como dict (con keys) y no como shape contiguo sin claves.
                if let (Some(expected), Some(Expression::Record(rec))) =
                    (&self.current_return_type, expr.as_ref())
                {
                    if matches!(expected, Type::Record(_, _)) || matches!(expected, Type::Shape(_)) {
                        self.types_by_span.insert(rec.span.clone(), expected.clone());
                    }
                }
                let ret_type = expr.as_ref()
                    .map(|e| self.check_expression(e))
                    .unwrap_or(Type::Void);
                // Verificar que el tipo de retorno concuerde
                if let Some(expected) = &self.current_return_type {
                    if !ret_type.is_assignable_to(expected) {
                        let msg = format!(
                            "Tipo de retorno {} no coincide con el declarado {}",
                            ret_type, expected
                        );
                        let span = expr.as_ref()
                            .map(|e| expr_span(e))
                            .unwrap_or_else(|| self.current_fn_span.clone());
                        // null como centinela (p.ej. __next -> int con `return null`)
                        // se permite con null_safety: warn, no bloquea (paridad walker).
                        if self.config.null_safety && matches!(ret_type, Type::Null) {
                            self.warn(&msg, span);
                        } else if self.config.strict {
                            self.error(&msg, span);
                        } else {
                            self.warn(&msg, span);
                        }
                    }
                }
                ret_type
            }
            Statement::Break(_) => Type::Void,
            Statement::Continue(_) => Type::Void,
            Statement::Expression(e) => self.check_expression(e),
            Statement::ClassDecl(c) => self.check_class(c),
            Statement::StructureDecl(s) => {
                self.define(&s.name, Type::Named(s.name.clone(), vec![]));
                let members: HashMap<String, Type> = s.fields.iter()
                    .map(|f| {
                        let t = self.resolve_type_annotation(&f.type_ann);
                        (f.name.clone(), t)
                    })
                    .collect();
                self.struct_members.insert(s.name.clone(), members);
                Type::Void
            }
            Statement::InterfaceDecl(i) => {
                self.define(&i.name, Type::Named(i.name.clone(), vec![]));
                let fields: HashMap<String, TypeAnnotation> = i.fields.iter()
                    .map(|f| (f.name.clone(), f.type_ann.clone()))
                    .collect();
                let signatures: HashMap<String, SignatureDecl> = i.signatures.iter()
                    .map(|s| (s.name.clone(), s.clone()))
                    .collect();
                self.interfaces.insert(i.name.clone(), InterfaceInfo {
                    type_params: i.type_params.clone(),
                    fields,
                    field_order: i.fields.iter().map(|f| f.name.clone()).collect(),
                    signatures,
                    signature_order: i.signatures.iter().map(|s| s.name.clone()).collect(),
                });
                if !self.config.strict {
                    self.warn(&format!("interface '{}' solo tiene efecto en type-checker", i.name), i.span);
                }
                Type::Void
            }            Statement::TypeAlias(t) => {
                self.check_type_alias(t);
                Type::Void
            }
            Statement::EnumDecl(e) => {
                self.define(&e.name, Type::Named(e.name.clone(), vec![]));
                self.enums.insert(e.name.clone());
                Type::Void
            }
            Statement::ModuleDecl(m) => {
                self.define(&m.name, Type::Named(m.name.clone(), vec![]));
                self.push_scope();
                for stmt in &m.body {
                    self.check_statement(stmt);
                }
                self.pop_scope();
                Type::Void
            }
            Statement::NamespaceDecl(n) => {
                self.define(&n.name, Type::Named(n.name.clone(), vec![]));
                Type::Void
            }
            Statement::Import(imp) => self.check_import(imp),
            Statement::FromImport(fi) => self.check_from_import(fi),
            Statement::Include(inc) => self.check_include(inc),
            Statement::When(w) => {
                // Cada rama se chequea en su propio scope (símbolos condicionales).
                for branch in &w.branches {
                    self.push_scope();
                    self.check_block(&branch.block);
                    self.pop_scope();
                }
                Type::Void
            }
            Statement::Extension(e) => {
                // Funciones/structs/variables nativas se registran como símbolos.
                for decl in &e.declarations {
                    match decl {
                        NativeDecl::Function(f) => {
                            let mut param_tys = Vec::new();
                            for p in &f.params {
                                let t = p.type_ann.as_ref()
                                    .map(|ta| self.resolve_type_annotation(ta))
                                    .unwrap_or(Type::Any);
                                param_tys.push(t);
                            }
                            let ret = f.return_type.as_ref()
                                .map(|ta| self.resolve_type_annotation(ta))
                                .unwrap_or(Type::Void);
                            self.define(&f.name, Type::Fun(param_tys, Box::new(ret)));
                        }
                        NativeDecl::Structure(s) => {
                            self.define(&s.name, Type::Named(s.name.clone(), vec![]));
                        }
                        NativeDecl::Var(v) => {
                            let t = v.type_ann.as_ref()
                                .map(|ta| self.resolve_type_annotation(ta))
                                .unwrap_or(Type::Any);
                            self.define(&v.name, t);
                        }
                    }
                }
                Type::Void
            }
            Statement::Config(_) | Statement::Meta(_) => Type::Void,
            Statement::Cmx(c) => {
                self.check_expression(&Expression::Cmx(c.clone()));
                Type::Cmx
            }
        }
    }


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

        self.define(&func.name, fn_type);
        return_type
    }


    pub(crate) fn check_if(&mut self, i: &IfStatement) -> Type {
        let cond = self.check_expression(&i.condition);
        if !cond.is_assignable_to(&Type::Bool) {
            self.warn(
                &format!("Condición if debe ser Bool, encontró {}", cond),
                i.span.clone(),
            );
        }
        self.check_block(&i.then_block);
        for elif in &i.elif_branches {
            self.check_expression(&elif.condition);
            self.check_block(&elif.block);
        }
        if let Some(else_block) = &i.else_block {
            self.check_block(else_block);
        }
        Type::Void
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


    pub(crate) fn check_class(&mut self, c: &ClassDecl) -> Type {
        let class_type = Type::Named(c.name.clone(), vec![]);
        self.define(&c.name, class_type.clone());
        self.push_scope();
        self.define("me", class_type.clone());
        self.define("super", class_type.clone());
        // Type params de la clase como placeholders (para fields/methods genéricos)
        for tp in &c.type_params {
            self.define(&tp.name, Type::Named(tp.name.clone(), vec![]));
        }
        // 1. pasada: recolectar los tipos de los miembros ANTES de chequear los
        // bodies, para que `me.campo`/`me.metodo()` resuelvan dentro del check.
        let mut members: HashMap<String, Type> = HashMap::new();
        let mut params_map: HashMap<String, Vec<Type>> = HashMap::new();
        if let Some(parent) = &c.extends {
            if let Some(pm) = self.class_members.get(parent) {
                members.extend(pm.clone());
            }
            if let Some(pp) = self.magic_params.get(parent) {
                params_map.extend(pp.clone());
            }
        }
        for member in &c.body {
            match member {
                ClassMember::Method(f) | ClassMember::Constructor(f) => {
                    members.insert(
                        f.name.clone(),
                        f.return_type
                            .as_ref()
                            .map(|t| self.resolve_type_annotation(t))
                            .unwrap_or(Type::Void),
                    );
                    params_map.insert(
                        f.name.clone(),
                        f.params
                            .iter()
                            .map(|p| {
                                p.type_ann
                                    .as_ref()
                                    .map(|t| self.resolve_type_annotation(t))
                                    .unwrap_or(Type::Any)
                            })
                            .collect(),
                    );
                }
                ClassMember::Property(v) => {
                    members.insert(
                        v.name.clone(),
                        v.type_ann
                            .as_ref()
                            .map(|t| self.resolve_type_annotation(t))
                            .unwrap_or(Type::Any),
                    );
                }
            }
        }
        self.class_members.insert(c.name.clone(), members);
        self.magic_params.insert(c.name.clone(), params_map);
        if let Some(parent) = &c.extends {
            self.class_parents.insert(c.name.clone(), parent.clone());
        }
        // 2. pasada: chequear los bodies.
        for member in &c.body {
            match member {
                ClassMember::Method(f) | ClassMember::Constructor(f) => {
                    self.check_function_decl(f);
                }
                ClassMember::Property(v) => {
                    self.check_var_decl(v, false);
                }
            }
        }
        // 3. pasada: verificar conformidad con las interfaces `implements`.
        for iface in &c.implements {
            self.check_implements(&c.name, iface, c.span.clone());
        }
        self.pop_scope();
        class_type
    }


    /// Verifica que la clase provea los campos y métodos que exige la interface.
    pub(crate) fn check_implements(&mut self, class_name: &str, iface_name: &str, span: Span) {
        let info = match self.interfaces.get(iface_name) {
            Some(i) => i.clone(),
            None => {
                self.error(
                    &format!(
                        "La clase '{}' implementa la interface '{}', que no está definida",
                        class_name, iface_name
                    ),
                    span,
                );
                return;
            }
        };
        let bind = self.interface_bindings(&info, &[]);
        let member_types: HashMap<String, Type> = self.class_members
            .get(class_name)
            .cloned()
            .unwrap_or_default();
        for fname in &info.field_order {
            let Some(ta) = info.fields.get(fname) else { continue };
            let required = self.resolve_annotation_with(ta, &bind);
            let ok = member_types
                .get(fname)
                .map(|provided| provided.is_assignable_to(&required))
                .unwrap_or(false);
            if !ok {
                self.error(
                    &format!(
                        "La clase '{}' no implementa el campo '{}: {}' exigido por la interface '{}'",
                        class_name, fname, required, iface_name
                    ),
                    span.clone(),
                );
            }
        }
        for (sig_name, sig) in &info.signatures {
            let required_fun = self.signature_type(sig, &bind);
            match member_types.get(sig_name) {
                None => {
                    self.error(
                        &format!(
                            "La clase '{}' no implementa el método '{}' exigido por la interface '{}'",
                            class_name, sig_name, iface_name
                        ),
                        span.clone(),
                    );
                }
                Some(ret) => {
                    if let Type::Fun(_, req_ret) = &required_fun {
                        if !ret.is_assignable_to(req_ret) {
                            self.error(
                                &format!(
                                    "El método '{}' de '{}' devuelve {}, la interface '{}' exige {}",
                                    sig_name, class_name, ret, iface_name, req_ret
                                ),
                                span.clone(),
                            );
                        }
                    }
                }
            }
        }
    }


    pub(crate) fn check_block(&mut self, block: &Block) {
        for stmt in &block.statements {
            self.check_statement(stmt);
        }
    }

}
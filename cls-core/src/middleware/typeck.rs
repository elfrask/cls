use crate::error::{ClsResult, Diagnostic, Span};
use crate::frontend::ast::*;
use crate::middleware::types::{Type, LitVal};
use crate::config::types::TypesConfig;
use std::collections::HashMap;

/// Definición compile-time de una interface (shapes con genéricos).
#[derive(Clone)]
struct InterfaceInfo {
    type_params: Vec<TypeParam>,
    fields: HashMap<String, TypeAnnotation>,
    /// Orden de declaración de los campos (para offsets deterministas del shape).
    field_order: Vec<String>,
    signatures: HashMap<String, SignatureDecl>,
    /// Orden de declaración de los métodos (para offsets deterministas del shape).
    signature_order: Vec<String>,
}

/// Type checker configurable de CLS
pub struct TypeChecker {
    config: TypesConfig,
    diagnostics: Vec<Diagnostic>,
    scopes: Vec<HashMap<String, Type>>,
    current_return_type: Option<Type>,
    /// Span de la función actual (para errores de `return` sin span propio).
    current_fn_span: Span,
    interfaces: HashMap<String, InterfaceInfo>,
    enums: std::collections::HashSet<String>,
    /// Mapa Span → Type de TODAS las expresiones visitadas (para backends).
    /// Se llena solo cuando `config.check` es true.
    types_by_span: HashMap<Span, Type>,
    /// Miembros de cada clase: nombre → tipo del campo o del retorno del método.
    class_members: HashMap<String, HashMap<String, Type>>,
    /// Campos de cada structure: nombre → tipo. Para tipar `p.campo` (member access).
    struct_members: HashMap<String, HashMap<String, Type>>,
    /// Módulos importados (prelude) — para resolver símbolos de `import`/`from`/`include`.
    /// Cada entrada: (path del import, módulo parseado).
    prelude: Vec<(String, Module)>,
    /// Alias de `import "path" as x` → path (para `x::miembro`).
    import_aliases: HashMap<String, String>,
}

impl TypeChecker {
    pub fn new(config: TypesConfig) -> Self {
        let mut tc = Self {
            config,
            diagnostics: Vec::new(),
            scopes: vec![HashMap::new()],
            current_return_type: None,
    current_fn_span: Span::new(1, 1, 1, 1),
            interfaces: HashMap::new(),
            enums: std::collections::HashSet::new(),
            types_by_span: HashMap::new(),
            class_members: HashMap::new(),
        struct_members: HashMap::new(),
            prelude: Vec::new(),
            import_aliases: HashMap::new(),
        };
        // Registrar funciones built-in (core intrinsics)
        tc.define("print", Type::Fun(vec![Type::Any], Box::new(Type::Void)));
        tc.define("input", Type::Fun(vec![Type::String], Box::new(Type::String)));
        tc.define("args", Type::Array(Box::new(Type::String)));
        tc.define("toString", Type::Fun(vec![Type::Any], Box::new(Type::String)));
        tc.define("int", Type::Fun(vec![Type::Any], Box::new(Type::Int)));
        tc.define("float", Type::Fun(vec![Type::Any], Box::new(Type::Float)));
        tc.define("str", Type::Fun(vec![Type::Any], Box::new(Type::String)));
        tc.define("bool", Type::Fun(vec![Type::Any], Box::new(Type::Bool)));
        tc.define("len", Type::Fun(vec![Type::Any], Box::new(Type::Int)));
        tc.define("type", Type::Fun(vec![Type::Any], Box::new(Type::String)));
        tc.define("now", Type::Fun(vec![], Box::new(Type::Int)));
        tc.define("exit", Type::Fun(vec![Type::Int], Box::new(Type::Void)));
        tc.define("sleep", Type::Fun(vec![Type::Int], Box::new(Type::Void)));
        tc.define("throw", Type::Fun(vec![Type::Any], Box::new(Type::Unknown)));
        tc
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    pub fn check(&mut self, module: &Module) -> ClsResult<()> {
        if !self.config.check {
            return Ok(());
        }
        // Pre-registrar firmas de funciones top-level (uso antes de definición).
        for stmt in &module.statements {
            if let Statement::FunctionDecl(f) = stmt {
                self.define_function_signature(f);
            }
        }
        for stmt in &module.statements {
            self.check_statement(stmt);
        }
        // No fallar si hay errores; reportar como diagnóstico
        Ok(())
    }

    fn define_function_signature(&mut self, f: &FunctionDecl) {
        let param_tys: Vec<Type> = f.params.iter()
            .map(|p| p.type_ann.as_ref().map(|t| self.resolve_type_annotation(t)).unwrap_or(Type::Any))
            .collect();
        let ret = f.return_type.as_ref()
            .map(|t| self.resolve_type_annotation(t))
            .unwrap_or(Type::Void);
        self.define(&f.name, Type::Fun(param_tys, Box::new(ret)));
    }

    /// Chequea un módulo con un prelude de módulos importados.
    /// Los tipos (enum/class/alias/interface) del prelude se registran primero,
    /// para que el módulo principal pueda usarlos en anotaciones.
    pub fn check_with_prelude(&mut self, module: &Module, prelude: &[(String, Module)]) -> ClsResult<()> {        if !self.config.check {
            return Ok(());
        }
        self.prelude = prelude.to_vec();
        // Pre-registrar firmas de funciones top-level de cada módulo del prelude
        // (para soportar recursión y uso antes de definición dentro del módulo).
        for (_path, m) in prelude {
            for stmt in &m.statements {
                if let Statement::FunctionDecl(f) = stmt {
                    self.define_function_signature(f);
                }
            }
        }
        for (_path, m) in prelude {
            for stmt in &m.statements {
                self.check_statement(stmt);
            }
        }
        for stmt in &module.statements {
            self.check_statement(stmt);
        }
        Ok(())
    }

    /// Registra las firmas de las funciones host del NODO (intrinsics) en el
    /// scope global: las llamadas a esos nombres se tipan contra la firma y el
    /// emisor las compila vía el canal `env.host_call`.
    pub fn register_host_intrinsics(&mut self, intrinsics: &[crate::middleware::types::HostIntrinsic]) {
        for i in intrinsics {
            self.define(&i.name, Type::Fun(i.params.clone(), Box::new(i.ret.clone())));
        }
    }

    fn error(&mut self, msg: &str, span: Span) -> Type {
        self.diagnostics.push(Diagnostic::error(msg, span));
        Type::Unknown
    }

    fn warn(&mut self, msg: &str, span: Span) {
        self.diagnostics.push(Diagnostic::warning(msg, span));
    }

    fn define(&mut self, name: &str, typ: Type) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), typ);
        }
    }

    fn lookup(&self, name: &str) -> Option<&Type> {
        self.scopes.iter().rev().find_map(|s| s.get(name))
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }


    fn check_statement(&mut self, stmt: &Statement) -> Type {
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
                        if self.config.strict {
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

    fn check_var_decl(&mut self, var: &VarDecl, is_const: bool) -> Type {
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

    fn check_function_decl(&mut self, func: &FunctionDecl) -> Type {
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
        // Registrar la función ANTES de chequear el cuerpo → permite recursión
        self.define(&func.name, fn_type.clone());
        self.check_block(&func.body);
        self.pop_scope();
        self.current_return_type = prev_return;
        self.current_fn_span = prev_fn_span;

        self.define(&func.name, fn_type);
        return_type
    }

    fn check_if(&mut self, i: &IfStatement) -> Type {
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

    fn check_while(&mut self, w: &WhileStatement) -> Type {
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

    fn check_for(&mut self, f: &ForStatement) -> Type {
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

    fn check_foreach(&mut self, fe: &ForEachStatement) -> Type {
        let iter_ty = self.check_expression(&fe.iterable);
        let item_ty = match &iter_ty {
            Type::Array(e) => (**e).clone(),
            Type::Tuple(s) => s.first().cloned().unwrap_or(Type::Any),
            Type::Named(n, _) if self.enums.contains(n) => iter_ty.clone(),
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

    fn check_switch(&mut self, s: &SwitchStatement) -> Type {
        self.check_expression(&s.value);
        for case in &s.cases {
            self.check_block(&case.block);
        }
        if let Some(default) = &s.default {
            self.check_block(default);
        }
        Type::Void
    }

    fn check_try(&mut self, t: &TryStatement) -> Type {
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

    fn check_with(&mut self, w: &WithStatement) -> Type {
        self.check_expression(&w.value);
        self.push_scope();
        self.define(&w.name, Type::Any);
        self.check_block(&w.block);
        self.pop_scope();
        Type::Void
    }

    fn check_class(&mut self, c: &ClassDecl) -> Type {
        let class_type = Type::Named(c.name.clone(), vec![]);
        self.define(&c.name, class_type.clone());
        self.push_scope();
        self.define("me", class_type.clone());
        self.define("super", class_type.clone());
        // Type params de la clase como placeholders (para fields/methods genéricos)
        for tp in &c.type_params {
            self.define(&tp.name, Type::Named(tp.name.clone(), vec![]));
        }
        // 1ª pasada: recolectar los tipos de los miembros ANTES de chequear los
        // bodies, para que `me.campo`/`me.metodo()` resuelvan dentro del check.
        let mut members: HashMap<String, Type> = HashMap::new();
        if let Some(parent) = &c.extends {
            if let Some(pm) = self.class_members.get(parent) {
                members.extend(pm.clone());
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
        // 2ª pasada: chequear los bodies.
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
        // 3ª pasada: verificar conformidad con las interfaces `implements`.
        for iface in &c.implements {
            self.check_implements(&c.name, iface, c.span.clone());
        }
        self.pop_scope();
        class_type
    }

    /// Verifica que la clase provea los campos y métodos que exige la interface.
    fn check_implements(&mut self, class_name: &str, iface_name: &str, span: Span) {
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

    fn check_block(&mut self, block: &Block) {
        for stmt in &block.statements {
            self.check_statement(stmt);
        }
    }


    fn check_expression(&mut self, expr: &Expression) -> Type {
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
                // `x::miembro` de un módulo importado → tipo del export.
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
            // dict con keys — necesario para el marshalling del binding).
            let prev = self.types_by_span.get(&span).cloned();
            if matches!(&prev, Some(Type::Record(_, _))) && matches!(&t, Type::Shape(_)) {
                self.types_by_span.insert(span, prev.unwrap());
            } else {
                self.types_by_span.insert(span, t.clone());
            }
        }
        t
    }

    /// Mapa de tipos por span de todas las expresiones visitadas.
    pub fn type_map(&self) -> &HashMap<Span, Type> {
        &self.types_by_span
    }

    fn check_literal(&mut self, lit: &Literal) -> Type {
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

    fn check_binary(&mut self, bin: &BinaryExpr) -> Type {
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
                // Int + Float → Float
                if is_num_l && matches!(right, Type::Float) {
                    return Type::Float;
                }
                if matches!(left, Type::Float) && is_num_r {
                    return Type::Float;
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

    fn check_unary(&mut self, un: &UnaryExpr) -> Type {
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

    fn check_call(&mut self, call: &CallExpr) -> Type {
        let callee_type = self.check_expression(&call.callee);

        // Métodos de primitivos (callee MemberAccess): el tipo del miembro ES el
        // resultado (`.join(sep)` → String, `.contains(x)` → Bool, ...).
        if let Expression::MemberAccess(m) = &*call.callee {
            // Array.map(f) → Array(retorno de f)
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
            // `math.abs` devuelve el tipo del primer argumento (int→Int, float→Float);
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
                // Inferir genéricos desde los args: param Named("T") → arg
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
                    // Sin firma útil (Any/huecos/genérico sin binding) → no validar.
                    if matches!(param_subst, Type::Any | Type::Unknown)
                        || matches!(arg_ty, Type::Any | Type::Unknown)
                        || self.has_unbound_generic(&param_subst, &bindings)
                    {
                        continue;
                    }
                    // El tipo del literal se usa como literal type para respetar
                    // uniones de literales y promociones implícitas (int→float).
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
                // Struct/Class constructor — devuelve el tipo
                callee_type.clone()
            }
            Type::Any => Type::Any,
            _ => self.error(
                &format!("No se puede llamar como función: {}", callee_type),
                call.span.clone(),
            ),
        }
    }

    /// ¿El tipo aún contiene un type param genérico sin binding (Named sin args
    /// que no está en bindings)? Si sí, la firma no está completamente resuelta
    /// y el argumento no se puede validar de forma fiable (p.ej. `T[]`).
    fn has_unbound_generic(&self, ty: &Type, bindings: &HashMap<String, Type>) -> bool {
        match ty {
            Type::Named(n, ps) => {
                if ps.is_empty() {
                    !bindings.contains_key(n)
                } else {
                    ps.iter().any(|p| self.has_unbound_generic(p, bindings))
                }
            }
            Type::Array(inner) => self.has_unbound_generic(inner, bindings),
            Type::Tuple(ts) => ts.iter().any(|t| self.has_unbound_generic(t, bindings)),
            Type::Record(k, v) => {
                self.has_unbound_generic(k, bindings) || self.has_unbound_generic(v, bindings)
            }
            Type::Shape(fields) => fields.iter().any(|(_, t)| self.has_unbound_generic(t, bindings)),
            Type::Fun(ps, r) => {
                ps.iter().any(|p| self.has_unbound_generic(p, bindings))
                    || self.has_unbound_generic(r, bindings)
            }
            Type::Union(ts) => ts.iter().any(|t| self.has_unbound_generic(t, bindings)),
            _ => false,
        }
    }

    /// Sustituye type params (Named sin args) por sus bindings en un tipo.
    fn substitute(&self, ty: &Type, bindings: &HashMap<String, Type>) -> Type {
        match ty {
            Type::Named(n, params) => {
                if params.is_empty() {
                    if let Some(b) = bindings.get(n) {
                        b.clone()
                    } else {
                        ty.clone()
                    }
                } else {
                    Type::Named(
                        n.clone(),
                        params.iter().map(|p| self.substitute(p, bindings)).collect(),
                    )
                }
            }
            Type::Array(inner) => Type::Array(Box::new(self.substitute(inner, bindings))),
            Type::Tuple(ts) => Type::Tuple(ts.iter().map(|t| self.substitute(t, bindings)).collect()),
            Type::Union(ts) => Type::Union(ts.iter().map(|t| self.substitute(t, bindings)).collect()),
            Type::Record(k, v) => Type::Record(
                Box::new(self.substitute(k, bindings)),
                Box::new(self.substitute(v, bindings)),
            ),
            Type::Fun(ps, r) => Type::Fun(
                ps.iter().map(|p| self.substitute(p, bindings)).collect(),
                Box::new(self.substitute(r, bindings)),
            ),
            _ => ty.clone(),
        }
    }

    fn check_member_access(&mut self, member: &MemberAccessExpr) -> Type {
        // Módulos internos del nodo (resueltos por nombre en el JIT): se manejan
        // ANTES de evaluar el object (que no está definido como variable).
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
                    // índice obj["k"] y print). El layout del host es compatible.
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
        // Color.Rojo → el tipo del enum (si member.object es un nombre de enum)
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
                // Campo de structure: `p.campo` → tipo anotado del campo.
                if let Some(members) = self.struct_members.get(name.as_str()) {
                    if let Some(t) = members.get(&member.member) {
                        return t.clone();
                    }
                }
                // `Color.Rojo` / `lib::Color.Rojo` → la variante de enum es
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

    /// `import "path" as x` → define el alias como módulo (acceso `x::f`).
    fn check_import(&mut self, imp: &ImportStatement) -> Type {
        // `import "math"`/`import "json"` (internals del nodo) → namespace.
        let alias = imp.alias.as_deref().unwrap_or(&imp.path);
        self.import_aliases.insert(alias.to_string(), imp.path.clone());
        self.define(alias, Type::Named(alias.to_string(), vec![]));
        Type::Void
    }

    /// `from "path" import a as fa, b` → define cada nombre en el scope actual.
    fn check_from_import(&mut self, fi: &FromImportStatement) -> Type {
        for im in &fi.names {
            if let Some(t) = self.find_export_type(&fi.path, &im.name) {
                let local = im.alias.as_deref().unwrap_or(&im.name);
                self.define(local, t);
            } else {
                let available = self.module_export_names(&fi.path);
                let hint = if available.is_empty() {
                    format!(
                        "El módulo '{}' no exporta ningún símbolo (usa `export` en cada declaración).",
                        fi.path
                    )
                } else {
                    format!(
                        "El módulo '{}' exporta: {}",
                        fi.path,
                        available.join(", ")
                    )
                };
                self.error(
                    &format!(
                        "'{}' no se exporta en el módulo '{}'. {}",
                        im.name, fi.path, hint
                    ),
                    fi.span.clone(),
                );
            }
        }
        Type::Void
    }

    /// Nombres de los símbolos exportados de un módulo del prelude.
    fn module_export_names(&self, path: &str) -> Vec<String> {
        let mut names = Vec::new();
        if let Some(m) = self.find_prelude_module(path) {
            for stmt in &m.statements {
                match stmt {
                    Statement::FunctionDecl(f) if f.visibility == Visibility::Export => {
                        names.push(f.name.clone());
                    }
                    Statement::VarDecl(v) | Statement::ConstDecl(v)
                        if v.visibility == Visibility::Export =>
                    {
                        names.push(v.name.clone());
                    }
                    Statement::EnumDecl(e) if e.visibility == Visibility::Export => {
                        names.push(e.name.clone());
                    }
                    _ => {}
                }
            }
        }
        names
    }

    /// `include "path"` → define TODOS los exports en el scope actual.
    fn check_include(&mut self, inc: &IncludeStatement) -> Type {
        let m = match self.find_prelude_module(&inc.path) {
            Some(m) => m.clone(),
            None => return Type::Void,
        };
        for stmt in &m.statements {
            match stmt {
                Statement::FunctionDecl(f) if f.visibility == Visibility::Export => {
                    let t = self.function_decl_type(f);
                    self.define(&f.name, t);
                }
                Statement::VarDecl(v) | Statement::ConstDecl(v)
                    if v.visibility == Visibility::Export =>
                {
                    let t = v.type_ann.as_ref()
                        .map(|ta| self.resolve_type_annotation(ta))
                        .or_else(|| v.value.as_ref().map(|val| self.infer_literal_type(val)))
                        .unwrap_or(Type::Any);
                    self.define(&v.name, t);
                }
                Statement::EnumDecl(e) if e.visibility == Visibility::Export => {
                    self.define(&e.name, Type::Named(e.name.clone(), vec![]));
                }
                _ => {}
            }
        }
        Type::Void
    }

    /// Tipo de un export por nombre en el módulo del prelude (path).
    fn find_export_type(&mut self, path: &str, name: &str) -> Option<Type> {
        let m = self.find_prelude_module(path)?.clone();
        for stmt in &m.statements {
            match stmt {
                Statement::FunctionDecl(f)
                    if f.visibility == Visibility::Export && f.name == name =>
                {
                    return Some(self.function_decl_type(f));
                }
                Statement::VarDecl(v) | Statement::ConstDecl(v)
                    if v.visibility == Visibility::Export && v.name == name =>
                {
                    let t = v.type_ann.as_ref()
                        .map(|ta| self.resolve_type_annotation(ta))
                        .or_else(|| v.value.as_ref().map(|val| self.infer_literal_type(val)))
                        .unwrap_or(Type::Any);
                    return Some(t);
                }
                Statement::EnumDecl(e) if e.visibility == Visibility::Export && e.name == name => {
                    return Some(Type::Named(e.name.clone(), vec![]));
                }
                _ => {}
            }
        }
        None
    }

    /// Busca un módulo del prelude cuyo path coincida con el import.
    fn find_prelude_module(&self, path: &str) -> Option<&Module> {
        self.prelude.iter().find(|(p, _)| p == path).map(|(_, m)| m)
    }

    /// Tipo de un valor simple (literal/identificador) para exports sin anotación.
    fn infer_literal_type(&mut self, val: &Expression) -> Type {
        match val {
            Expression::Literal(l) => match &l.kind {
                LiteralKind::Int(_) => Type::Int,
                LiteralKind::Float(_) => Type::Float,
                LiteralKind::String(_) => Type::String,
                LiteralKind::Bool(_) => Type::Bool,
                LiteralKind::Char(_) => Type::Char,
                LiteralKind::Null => Type::Null,
                _ => Type::Any,
            },
            Expression::Array(_) => Type::Array(Box::new(Type::Any)),
            Expression::Identifier(_, _) => Type::Any,
            _ => Type::Any,
        }
    }

    /// Tipo de una función a partir de su declaración.
    fn function_decl_type(&mut self, f: &FunctionDecl) -> Type {        let params: Vec<Type> = f.params.iter()
            .map(|p| p.type_ann.as_ref()
                .map(|ta| self.resolve_type_annotation(ta))
                .unwrap_or(Type::Any))
            .collect();
        let ret = f.return_type.as_ref()
            .map(|ta| self.resolve_type_annotation(ta))
            .unwrap_or(Type::Void);
        Type::Fun(params, Box::new(ret))
    }

    /// Tipo de `x::miembro` cuando `x` es un módulo importado.
    fn module_member_type(&mut self, module_alias: &str, member: &str) -> Option<Type> {
        let path = self
            .import_aliases
            .get(module_alias)
            .cloned()
            .unwrap_or_else(|| module_alias.to_string());
        self.find_export_type(&path, member)
    }

    fn check_index(&mut self, idx: &IndexExpr) -> Type {
        let obj = self.check_expression(&idx.object);
        let index_type = self.check_expression(&idx.index);
        match obj {
            Type::Array(inner) => *inner,
            Type::Record(_k, v) => *v,
            // Shape: índice literal con clave conocida → tipo del campo; clave
            // desconocida → error (la estructura del record es fija).
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
            // Tupla: índice literal → slot exacto; dinámico → unión de slots
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
                let _ = index_type;
                Type::Any
            }
        }
    }

    fn check_array(&mut self, arr: &ArrayExpr) -> Type {
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
                // Compatible por promoción: `[1, 2.0]` → el array es de Float
                // (el Int se promueve en emisión). Último tipo más específico.
                elem_type = t;
            }
        }
        Type::Array(Box::new(elem_type))
    }

    fn check_tuple(&mut self, tup: &TupleExpr) -> Type {
        let types: Vec<Type> = tup.elements.iter()
            .map(|e| self.check_expression(e))
            .collect();
        Type::Tuple(types)
    }

    fn check_record(&mut self, rec: &RecordExpr) -> Type {
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

    fn check_arrow_function(&mut self, arrow: &ArrowFunctionExpr) -> Type {
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

    fn check_conditional(&mut self, cond: &ConditionalExpr) -> Type {
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

    fn check_assignment(&mut self, assign: &AssignmentExpr) -> Type {
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

    // ═══════════════════════════════════════════
    // Type resolution
    // ═══════════════════════════════════════════

    pub fn resolve_type_annotation(&mut self, ann: &TypeAnnotation) -> Type {
        self.resolve_annotation_with(ann, &HashMap::new())
    }

    /// Resuelve una anotación bajo un contexto de type params (bindings T→tipo).
    fn resolve_annotation_with(
        &mut self,
        ann: &TypeAnnotation,
        bindings: &HashMap<String, Type>,
    ) -> Type {
        match &ann.kind {
            TypeKind::Int => Type::Int,
            TypeKind::Float => Type::Float,
            TypeKind::String => Type::String,
            TypeKind::Bool => Type::Bool,
            TypeKind::Char => Type::Char,
            TypeKind::Any => Type::Any,
            TypeKind::Unknown => Type::Unknown,
            TypeKind::Null => Type::Null,
            TypeKind::Void => Type::Void,
            TypeKind::Empty => Type::Empty,
            TypeKind::Array(inner) => {
                Type::Array(Box::new(self.resolve_annotation_with(inner, bindings)))
            }
            TypeKind::Tuple(types) => Type::Tuple(
                types.iter()
                    .map(|t| self.resolve_annotation_with(t, bindings))
                    .collect(),
            ),
            TypeKind::Union(types) => Type::Union(
                types.iter()
                    .map(|t| self.resolve_annotation_with(t, bindings))
                    .collect(),
            ),
            TypeKind::Literal(lit) => self.literal_type(lit),
            TypeKind::Access(base, access) => {
                self.resolve_type_access(base, access, bindings)
            }
            // Phantom: !T se resuelve SIN sustituir type params (no unifica)
            TypeKind::Phantom(inner) => self.resolve_annotation_with(inner, &HashMap::new()),
            TypeKind::Record(k, v) => {
                Type::Record(
                    Box::new(self.resolve_annotation_with(k, bindings)),
                    Box::new(self.resolve_annotation_with(v, bindings)),
                )
            }
            TypeKind::Shape(fields) => {
                Type::Shape(
                    fields.iter()
                        .map(|(n, ta)| (n.clone(), self.resolve_annotation_with(ta, bindings)))
                        .collect(),
                )
            }
            TypeKind::Intersection(members) => {
                // Merge de shapes: campos de todos los miembros (los no-shape se
                // ignoran o resuelven a Any). Conflicto de tipo = error.
                let mut out: Vec<(String, Type)> = Vec::new();
                for m in members {
                    let t = self.resolve_annotation_with(m, bindings);
                    if let Type::Shape(fields) = t {
                        for (n, ty) in fields {
                            if let Some((_, existing)) = out.iter_mut().find(|(en, _)| *en == n) {
                                if *existing != ty {
                                    return self.error(
                                        &format!("Campo '{}' con tipos incompatibles en la conjunción de shapes", n),
                                        ann.span.clone(),
                                    );
                                }
                            } else {
                                out.push((n, ty));
                            }
                        }
                    }
                }
                Type::Shape(out)
            }
            TypeKind::Fun(params, ret) => {
                let param_types: Vec<Type> = params.iter()
                    .map(|p| self.resolve_annotation_with(p, bindings))
                    .collect();
                Type::Fun(param_types, Box::new(self.resolve_annotation_with(ret, bindings)))
            }
            TypeKind::I32 => Type::I32,
            TypeKind::I64 => Type::I64,
            TypeKind::I16 => Type::I16,
            TypeKind::I8 => Type::I8,
            TypeKind::F32 => Type::F32,
            TypeKind::F64 => Type::F64,
            TypeKind::Cmx => Type::Cmx,
            TypeKind::Named(name, params) => {
                // Type param (T, U) del contexto genérico
                if let Some(t) = bindings.get(name) {
                    return t.clone();
                }
                let param_types: Vec<Type> = params.iter()
                    .map(|p| self.resolve_annotation_with(p, bindings))
                    .collect();
                // Si es un nombre conocido, mapearlo
                match name.as_str() {
                    "Integer" => Type::Int,
                    "Float" => Type::Float,
                    "Character" => Type::Char,
                    "Boolean" => Type::Bool,
                    // Record<K, V> → diccionario tipado
                    "Record" if param_types.len() == 2 => Type::Record(
                        Box::new(param_types[0].clone()),
                        Box::new(param_types[1].clone()),
                    ),
                    name if self.interfaces.contains_key(name) => {
                        let info = self.interfaces[name].clone();
                        let bind = self.interface_bindings(&info, &param_types);
                        let mut fields: Vec<(String, Type)> = info
                            .field_order
                            .iter()
                            .filter_map(|fn_| info.fields.get(fn_).map(|ta| (fn_.clone(), self.resolve_annotation_with(ta, &bind))))
                            .collect();
                        for name_sig in &info.signature_order {
                            if let Some(sig) = info.signatures.get(name_sig) {
                                fields.push((name_sig.clone(), self.signature_type(sig, &bind)));
                            }
                        }
                        Type::Shape(fields)
                    }
                    _ => {
                        self.lookup(name)
                            .cloned()
                            .unwrap_or(Type::Named(name.clone(), param_types))
                    }
                }
            }
        }
    }

    /// Convierte un literal AST a un literal type (o su tipo base).
    fn literal_type(&self, lit: &LiteralKind) -> Type {
        match lit {
            LiteralKind::String(s) => Type::Literal(LitVal::Str(s.clone())),
            LiteralKind::Int(i) => Type::Literal(LitVal::Int(*i)),
            LiteralKind::Float(f) => Type::Literal(LitVal::Float(f.to_bits())),
            LiteralKind::Bool(b) => Type::Literal(LitVal::Bool(*b)),
            _ => Type::Any,
        }
    }

    /// Resuelve un acceso a tipo: `T["field"]` o `T[0]`.
    fn resolve_type_access(
        &mut self,
        base: &TypeAnnotation,
        access: &TypeAccess,
        bindings: &HashMap<String, Type>,
    ) -> Type {
        // Caso interface nombrada (con args opcionales): resolver miembros con genéricos
        if let TypeKind::Named(name, arg_anns) = &base.kind {
            if let Some(info) = self.interfaces.get(name).cloned() {
                let arg_types: Vec<Type> = arg_anns.iter()
                    .map(|a| self.resolve_annotation_with(a, bindings))
                    .collect();
                let b = self.interface_bindings(&info, &arg_types);
                match access {
                    TypeAccess::Key(key) => {
                        if let Some(ta) = info.fields.get(key) {
                            return self.resolve_annotation_with(ta, &b);
                        }
                        if let Some(sig) = info.signatures.get(key) {
                            return self.signature_type(sig, &b);
                        }
                        return self.error(
                            &format!("Interface '{}' no tiene miembro '{}'", name, key),
                            base.span.clone(),
                        );
                    }
                    TypeAccess::Index(i) => {
                        let order = self.interface_member_types(&info, &b);
                        return order.get(*i).cloned().unwrap_or_else(|| self.error(
                            &format!("Index '{}' fuera de rango en interface '{}'", i, name),
                            base.span.clone(),
                        ));
                    }
                }
            }
        }

        // Fallback: resolver el tipo base y aplicar sobre tipos compuestos
        let base_type = self.resolve_annotation_with(base, bindings);
        match access {
            TypeAccess::Key(key) => match &base_type {
                Type::Record(_, v) => (**v).clone(),
                Type::Shape(fields) => fields.iter()
                    .find(|(n, _)| n == key)
                    .map(|(_, t)| t.clone())
                    .unwrap_or(Type::Any),
                _ => Type::Any,
            },
            TypeAccess::Index(i) => match base_type {
                Type::Tuple(ts) => ts.get(*i).cloned().unwrap_or(Type::Any),
                Type::Array(inner) => *inner,
                Type::Union(ts) => ts.get(*i).cloned().unwrap_or(Type::Any),
                _ => Type::Any,
            },
        }
    }

    /// Construye bindings T→tipo para los type params de una interface.
    fn interface_bindings(&mut self, info: &InterfaceInfo, args: &[Type]) -> HashMap<String, Type> {
        let mut bindings = HashMap::new();
        for (i, tp) in info.type_params.iter().enumerate() {
            if let Some(arg) = args.get(i) {
                bindings.insert(tp.name.clone(), arg.clone());
            } else if let Some(default) = &tp.default {
                let resolved = self.resolve_annotation_with(default, &bindings);
                bindings.insert(tp.name.clone(), resolved);
            } else {
                bindings.insert(tp.name.clone(), Type::Any);
            }
        }
        bindings
    }

    /// Tipos de los campos de una interface en orden de declaración (para acceso
    /// por índice y offsets deterministas del shape). NO itera el HashMap: usa
    /// `field_order` (el orden en que se declararon los campos).
    fn interface_member_types(&mut self, info: &InterfaceInfo, bindings: &HashMap<String, Type>) -> Vec<Type> {
        info.field_order.iter()
            .filter_map(|name| info.fields.get(name).map(|ta| self.resolve_annotation_with(ta, bindings)))
            .collect()
    }

    /// Tipo `fun(params) -> ret` de una signature, con genéricos aplicados.
    fn signature_type(&mut self, sig: &SignatureDecl, bindings: &HashMap<String, Type>) -> Type {
        let params: Vec<Type> = sig.params.iter()
            .map(|p| p.type_ann.as_ref()
                .map(|ta| self.resolve_annotation_with(ta, bindings))
                .unwrap_or(Type::Any))
            .collect();
        let ret = sig.return_type.as_ref()
            .map(|ta| self.resolve_annotation_with(ta, bindings))
            .unwrap_or(Type::Void);
        Type::Fun(params, Box::new(ret))
    }

    /// Registra y define un alias de tipo (compile-time).
    fn check_type_alias(&mut self, alias: &TypeAliasDecl) {
        let type_ann = alias.type_ann.clone();
        let resolved = self.resolve_annotation_with(&type_ann, &HashMap::new());
        self.define(&alias.name, resolved);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::{Lexer, Parser};
    use crate::error::Diagnostic;
    use crate::error::diagnostic::Severity;

    /// Parsea y chequea un source, devolviendo los diagnostics.
    fn check_source(src: &str, strict: bool) -> Vec<Diagnostic> {
        let toks = Lexer::new(src).tokenize().expect("tokenize");
        let module = Parser::new(toks).parse().expect("parse");
        let config = TypesConfig { check: true, strict, ..Default::default() };
        let mut tc = TypeChecker::new(config);
        tc.check(&module).expect("check no debe fallar");
        tc.diagnostics().to_vec()
    }

    fn count_errors(diags: &[Diagnostic]) -> usize {
        diags.iter().filter(|d| matches!(d.severity, Severity::Error)).count()
    }

    #[test]
    fn tuple_valid() {
        let d = check_source("function f() { var a: (Int, String) = (1, \"x\"); };", true);
        assert_eq!(count_errors(&d), 0, "tupla valida: {:?}", d);
    }

    #[test]
    fn tuple_invalid_slot() {
        let d = check_source("function f() { var a: (Int, String) = (1, 2); };", true);
        assert_eq!(count_errors(&d), 1, "slot 2 es Int no String: {:?}", d);
    }

    #[test]
    fn union_literal_valid() {
        let src = "alias Color = \"red\" | \"green\"; function f() { var c: Color = \"red\"; };";
        let d = check_source(src, true);
        assert_eq!(count_errors(&d), 0, "{:?}", d);
    }

    #[test]
    fn union_literal_invalid() {
        let src = "alias Color = \"red\" | \"green\"; function f() { var c: Color = \"purple\"; };";
        let d = check_source(src, true);
        assert_eq!(count_errors(&d), 1, "purple no esta en la union: {:?}", d);
    }

    #[test]
    fn alias_function_type() {
        let src = "alias Fn = (Int) -> Int; function f() { };";
        let d = check_source(src, true);
        assert_eq!(count_errors(&d), 0, "alias de funcion: {:?}", d);
    }

    #[test]
    fn interface_extract_default() {
        let src = "interface H<T=Int> { num: T, }; function f() { var n: H[\"num\"] = 1; };";
        let d = check_source(src, true);
        assert_eq!(count_errors(&d), 0, "H[\"num\"] con default Int: {:?}", d);
    }

    #[test]
    fn interface_extract_with_arg() {
        let src = "interface H<T=Int> { num: T, }; function f() { var s: H<String>[\"num\"] = \"x\"; };";
        let d = check_source(src, true);
        assert_eq!(count_errors(&d), 0, "H<String>[\"num\"] es String: {:?}", d);
    }

    #[test]
    fn generic_function() {
        let src = "function id<T>(x: T) -> T { return x; }; function f() { var g: Int = id(5); var h: String = id(\"a\"); };";
        let d = check_source(src, true);
        assert_eq!(count_errors(&d), 0, "genericos: {:?}", d);
    }

    #[test]
    fn phantom_not_substituted() {
        let src = "interface M<T> { real: T, ghost: !T, }; function f() { var r: M<String>[\"real\"] = \"ok\"; var g: M<String>[\"ghost\"]; };";
        let d = check_source(src, true);
        assert_eq!(count_errors(&d), 0, "phantom: {:?}", d);
    }

    #[test]
    fn enum_typed_ok() {
        let src = "enum Color { Rojo, Verde, }; function f() { var c: Color = Color.Rojo; };";
        let d = check_source(src, true);
        assert_eq!(count_errors(&d), 0, "enum: {:?}", d);
    }

    #[test]
    fn enum_typed_wrong() {
        let src = "enum Color { Rojo, Verde, }; function f() { var c: Color = 5; };";
        let d = check_source(src, true);
        assert_eq!(count_errors(&d), 1, "Int no es Color: {:?}", d);
    }

    #[test]
    fn record_typed() {
        let src = "function f() { var d: Record<String, Int> = {a: 1}; };";
        let d = check_source(src, true);
        assert_eq!(count_errors(&d), 0, "record: {:?}", d);
    }

    #[test]
    fn tuple_dynamic_index_union() {
        // índice dinámico sobre tupla → unión; no debe dar error en estricto
        let src = "function f() { var a: (Int, String) = (1, \"x\"); var i = 0; var v = a[i]; };";
        let d = check_source(src, true);
        assert_eq!(count_errors(&d), 0, "indice dinamico: {:?}", d);
    }

    #[test]
    fn tuple_access_by_literal() {
        // t[1] con índice literal → slot exacto
        let src = "function f() { var a: (Int, String) = (1, \"x\"); var n: Int = a[0]; };";
        let d = check_source(src, true);
        assert_eq!(count_errors(&d), 0, "indice literal: {:?}", d);
    }

    #[test]
    fn call_arg_type_mismatch() {
        // Tarea 1: arg Int a param String → error en estricto (con firma conocida)
        let src = "function f(x: String) -> String { return x; }; function g() { var y = f(123); };";
        let d = check_source(src, true);
        assert_eq!(count_errors(&d), 1, "Int a param String: {:?}", d);
    }

    #[test]
    fn call_arg_type_ok() {
        let src = "function f(x: String) -> String { return x; }; function g() { var y = f(\"ok\"); };";
        let d = check_source(src, true);
        assert_eq!(count_errors(&d), 0, "String a param String: {:?}", d);
    }

    #[test]
    fn call_arg_promotion_int_to_float() {
        // int → float es asignable; no debe dar error
        let src = "function f(x: Float) -> Float { return x; }; function g() { var y = f(5); };";
        let d = check_source(src, true);
        assert_eq!(count_errors(&d), 0, "int a param Float: {:?}", d);
    }

    #[test]
    fn generic_array_param_no_false_positive() {
        // T[] sin binding (param anidado en contenedor) → no validar (sin falso positivo)
        let src = "function first<T>(a: T[]) -> T { return a[0]; }; function g() { var y = first([1,2,3]); };";
        let d = check_source(src, true);
        assert_eq!(count_errors(&d), 0, "T[] no debe false-positivar: {:?}", d);
    }

    #[test]
    fn implements_missing_member_errors() {
        let src = "interface I { num: Int, }; class A implements I { var num: String = \"x\"; };";
        let d = check_source(src, true);
        assert_eq!(count_errors(&d), 1, "campo con tipo incompatibble: {:?}", d);
    }

    #[test]
    fn implements_ok() {
        let src = "interface I { num: Int, }; class A implements I { var num: Int = 1; };";
        let d = check_source(src, true);
        assert_eq!(count_errors(&d), 0, "conformidad ok: {:?}", d);
    }

    #[test]
    fn implements_unknown_interface_errors() {
        let src = "class A implements NoExiste { var num: Int = 1; };";
        let d = check_source(src, true);
        assert_eq!(count_errors(&d), 1, "interface no definida: {:?}", d);
    }
}

/// Devuelve el `Span` de una expresión (cada variante lo lleva).
pub fn expr_span(expr: &Expression) -> Span {
    match expr {
        Expression::Literal(l) => l.span,
        Expression::Identifier(_, s) => *s,
        Expression::Binary(b) => b.span,
        Expression::Unary(u) => u.span,
        Expression::Call(c) => c.span,
        Expression::MemberAccess(m) => m.span,
        Expression::Index(i) => i.span,
        Expression::Array(a) => a.span,
        Expression::Tuple(t) => t.span,
        Expression::Record(r) => r.span,
        Expression::ArrowFunction(a) => a.span,
        Expression::Conditional(c) => c.span,
        Expression::Assignment(a) => a.span,
        Expression::Cmx(c) => c.span,
        Expression::Parenthesized(_, s) => *s,
        Expression::StringInterpolation(s) => s.span,
        Expression::NamespaceAccess(_, _, s) => *s,
        Expression::Await(_, s) => *s,
    }
}

/// Nombre de tipo builtin → `Type` (para `v is Tipo`). `None` si no es builtin.
fn builtin_type_name(name: &str) -> Option<Type> {
    match name {
        "String" => Some(Type::String),
        "Int" => Some(Type::Int),
        "Float" => Some(Type::Float),
        "Bool" => Some(Type::Bool),
        "Char" => Some(Type::Char),
        "Array" => Some(Type::Array(Box::new(Type::Any))),
        "Tuple" => Some(Type::Tuple(vec![])),
        "Record" => Some(Type::Record(Box::new(Type::Any), Box::new(Type::Any))),
        "Cmx" => Some(Type::Cmx),
        "Null" => Some(Type::Null),
        "Void" => Some(Type::Void),
        _ => None,
    }
}

/// Aridad esperada de un miembro de los módulos internos del nodo desktop
/// (os/path/process/time/random). `None` si el miembro no existe o es libre.
fn module_arity(mod_name: &str, member: &str) -> Option<usize> {
    match mod_name {
        "os" => match member {
            "platform" | "arch" | "version" | "hostname" | "home" | "tempdir"
            | "cpus" | "pid" | "uptime" | "sep" | "isWindows" | "isUnix" => Some(0),
            "env" => Some(1),
            _ => None,
        },
        "path" => match member {
            "join" => Some(2),
            "basename" | "dirname" | "extname" | "resolve" | "normalize"
            | "isAbsolute" => Some(1),
            "sep" => Some(0),
            _ => None,
        },
        "process" => match member {
            "args" | "cwd" | "pid" | "platform" | "title" => Some(0),
            "env" => Some(1),
            "exit" => Some(1),
            _ => None,
        },
        "time" => match member {
            "now" | "seconds" | "iso" | "date" | "clock" | "year" | "month"
            | "day" | "hour" | "minute" | "second" => Some(0),
            "sleep" => Some(1),
            _ => None,
        },
        "random" => match member {
            "random" | "uuid" => Some(0),
            "int" | "float" => Some(2),
            _ => None,
        },
        _ => None,
    }
}

/// Formatea una expresión como texto CLS corto y legible (para mensajes de error).
/// NO usa Debug del AST — el usuario debe poder leer qué falló.
pub fn expr_short_display(expr: &Expression) -> String {
    crate::frontend::ast::expr_display(expr)
}


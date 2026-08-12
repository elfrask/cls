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
}

/// Type checker configurable de CLS
pub struct TypeChecker {
    config: TypesConfig,
    diagnostics: Vec<Diagnostic>,
    scopes: Vec<HashMap<String, Type>>,
    current_return_type: Option<Type>,
    interfaces: HashMap<String, InterfaceInfo>,
    enums: std::collections::HashSet<String>,
    /// Mapa Span → Type de TODAS las expresiones visitadas (para backends).
    /// Se llena solo cuando `config.check` es true.
    types_by_span: HashMap<Span, Type>,
    /// Miembros de cada clase: nombre → tipo del campo o del retorno del método.
    class_members: HashMap<String, HashMap<String, Type>>,
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
            interfaces: HashMap::new(),
            enums: std::collections::HashSet::new(),
            types_by_span: HashMap::new(),
            class_members: HashMap::new(),
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
    pub fn check_with_prelude(&mut self, module: &Module, prelude: &[(String, Module)]) -> ClsResult<()> {
        if !self.config.check {
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
                let ret_type = expr.as_ref()
                    .map(|e| self.check_expression(e))
                    .unwrap_or(Type::Void);
                // Verificar que el tipo de retorno concuerde
                if let Some(expected) = &self.current_return_type {
                    if !ret_type.is_assignable_to(expected) {
                        self.warn(
                            &format!(
                                "Tipo de retorno {} no coincide con el declarado {}",
                                ret_type, expected
                            ),
                            Span::new(1, 1, 1, 1),
                        );
                    }
                }
                ret_type
            }
            Statement::Break => Type::Void,
            Statement::Continue => Type::Void,
            Statement::Expression(e) => self.check_expression(e),
            Statement::ClassDecl(c) => self.check_class(c),
            Statement::StructureDecl(s) => {
                self.define(&s.name, Type::Named(s.name.clone(), vec![]));
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

        let declared = var.type_ann.as_ref()
            .map(|t| self.resolve_type_annotation(t))
            .unwrap_or_else(|| inferred.clone());

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
        // Record literal con anotación (p.ej. `var d: Record<String, Int> = {a:1}`
        // o `var p: Persona = {nombre: "Ana", edad: 30}`): registrar el tipo
        // anotado en el span del literal para que el backend lo emita como dict
        // (Record) o shape según lo que pida la anotación (offsets consistentes).
        if let Some(Expression::Record(rec)) = &var.value {
            if let Some(declared_rec) = var.type_ann.as_ref().map(|t| self.resolve_type_annotation(t)) {
                if matches!(declared_rec, Type::Record(_, _)) | matches!(declared_rec, Type::Shape(_)) {
                    self.types_by_span.insert(rec.span.clone(), declared_rec);
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
        for (name, typ) in &param_types {
            self.define(name, typ.clone());
        }
        // Registrar la función ANTES de chequear el cuerpo → permite recursión
        self.define(&func.name, fn_type.clone());
        self.check_block(&func.body);
        self.pop_scope();
        self.current_return_type = prev_return;

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
        self.pop_scope();
        class_type
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
                self.lookup(name)
                    .cloned()
                    .unwrap_or_else(|| {
                        if self.config.no_implicit_any {
                            self.error(&format!("Variable no definida: {}", name), span.clone())
                        } else {
                            Type::Any
                        }
                    })
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
            self.types_by_span.insert(span, t.clone());
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
        let obj_type = self.check_expression(&member.object);
        // Color.Rojo → el tipo del enum (si member.object es un nombre de enum)
        if let Expression::Identifier(name, _) = &*member.object {
            if self.enums.contains(name) {
                return Type::Named(name.clone(), vec![]);
            }
            // Módulos internos del nodo (resueltos por nombre en el JIT).
            if name == "http" {
                return match member.member.as_str() {
                    "get" | "post" => Type::String,
                    _ => Type::Any,
                };
            }
            if name == "fs" {
                return match member.member.as_str() {
                    "exists" => Type::Bool,
                    "cwd" | "readFile" | "listDir" => Type::String,
                    _ => Type::Any,
                };
            }
            if name == "json" {
                return match member.member.as_str() {
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
        }
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
        for (key, expr) in &rec.entries {
            let t = self.check_expression(expr);
            fields.push((key.clone(), t));
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
        self.push_scope();
        for (param, typ) in arrow.params.iter().zip(param_types.iter()) {
            self.define(&param.name, typ.clone());
        }
        self.check_block(&arrow.body);

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
            self.warn(
                &format!("Tipo {} no asignable a {}", right, left),
                assign.span.clone(),
            );
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
                        for (name_sig, sig) in &info.signatures {
                            fields.push((name_sig.clone(), self.signature_type(sig, &bind)));
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

    /// Tipos de los campos de una interface en orden (para acceso por índice).
    fn interface_member_types(&mut self, info: &InterfaceInfo, bindings: &HashMap<String, Type>) -> Vec<Type> {
        info.fields.iter()
            .map(|(name, ta)| {
                let t = self.resolve_annotation_with(ta, bindings);
                (name.clone(), t)
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|(_, t)| t)
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

/// Formatea una expresión como texto CLS corto y legible (para mensajes de error).
/// NO usa Debug del AST — el usuario debe poder leer qué falló.
pub fn expr_short_display(expr: &Expression) -> String {
    use crate::frontend::token::Operator;
    match expr {
        Expression::Literal(l) => match &l.kind {
            LiteralKind::Int(v) => v.to_string(),
            LiteralKind::Float(v) => v.to_string(),
            LiteralKind::String(s) => format!("\"{}\"", s),
            LiteralKind::Bool(b) => b.to_string(),
            LiteralKind::Char(c) => format!("'{}'", c),
            LiteralKind::Null => "null".to_string(),
            LiteralKind::Unknown => "?".to_string(),
        },
        Expression::Identifier(name, _) => name.clone(),
        Expression::Binary(b) => {
            let op = match b.op {
                Operator::Plus => "+",
                Operator::Minus => "-",
                Operator::Star => "*",
                Operator::Slash => "/",
                Operator::Percent => "%",
                Operator::StarStar => "**",
                Operator::StrictEqual => "==",
                Operator::NotEqual => "!=",
                Operator::LessThan => "<",
                Operator::LessEqual => "<=",
                Operator::GreaterThan => ">",
                Operator::GreaterEqual => ">=",
                Operator::And => "&&",
                Operator::Or => "||",
                Operator::In => "in",
                Operator::Is => "is",
                Operator::Caret => "^",
                Operator::ShiftLeft => "<<",
                Operator::ShiftRight => ">>",
                _ => "?",
            };
            format!(
                "({} {} {})",
                expr_short_display(&b.left),
                op,
                expr_short_display(&b.right)
            )
        }
        Expression::Unary(u) => {
            let op = match u.op {
                crate::frontend::ast::UnaryOp::Negate => "-",
                crate::frontend::ast::UnaryOp::Not => "!",
                crate::frontend::ast::UnaryOp::BitwiseNot => "~",
                crate::frontend::ast::UnaryOp::TypeOf => "typeof ",
                _ => "",
            };
            format!("{}{}", op, expr_short_display(&u.operand))
        }
        Expression::Call(c) => format!(
            "{}({})",
            expr_short_display(&c.callee),
            c.args.iter().map(expr_short_display).collect::<Vec<_>>().join(", ")
        ),
        Expression::MemberAccess(m) => format!("{}.{}", expr_short_display(&m.object), m.member),
        Expression::Index(i) => format!(
            "{}[{}]",
            expr_short_display(&i.object),
            expr_short_display(&i.index)
        ),
        Expression::Array(a) => format!(
            "[{}]",
            a.elements.iter().map(expr_short_display).collect::<Vec<_>>().join(", ")
        ),
        Expression::Tuple(t) => format!(
            "({})",
            t.elements.iter().map(expr_short_display).collect::<Vec<_>>().join(", ")
        ),
        Expression::Record(r) => format!(
            "{{{}}}",
            r.entries.iter().map(|(k, v)| format!("{}: {}", k, expr_short_display(v))).collect::<Vec<_>>().join(", ")
        ),
        Expression::ArrowFunction(_) => "fn(...)".to_string(),
        Expression::Conditional(c) => format!(
            "({} ? {} : {})",
            expr_short_display(&c.condition),
            expr_short_display(&c.then_expr),
            expr_short_display(&c.else_expr)
        ),
        Expression::Assignment(a) => format!(
            "{} = {}",
            expr_short_display(&a.target),
            expr_short_display(&a.value)
        ),
        Expression::Parenthesized(inner, _) => format!("({})", expr_short_display(inner)),
        Expression::StringInterpolation(s) => {
            let mut out = String::from("\"");
            for part in &s.parts {
                match part {
                    InterpolationPart::Text(t) => out.push_str(t),
                    InterpolationPart::Expr(e) => {
                        out.push_str("${");
                        out.push_str(&expr_short_display(e));
                        out.push('}');
                    }
                }
            }
            out.push('"');
            out
        }
        Expression::Cmx(c) => format!("<{} />", c.tag),
        Expression::NamespaceAccess(ns, name, _) => format!("{}::{}", ns, name),
        Expression::Await(inner, _) => format!("await {}", expr_short_display(inner)),
    }
}


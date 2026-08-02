use crate::error::{ClsResult, Diagnostic, Span};
use crate::frontend::ast::*;
use crate::middleware::types::{Type, LitVal};
use crate::config::types::TypesConfig;
use std::collections::HashMap;

/// DefiniciÃ³n compile-time de una interface (shapes con genÃ©ricos).
#[derive(Clone)]
struct InterfaceInfo {
    type_params: Vec<TypeParam>,
    fields: HashMap<String, TypeAnnotation>,
    signatures: HashMap<String, SignatureDecl>,
}

/// Type checker configurable de CLS
pub struct TypeChecker {
    config: TypesConfig,
    diagnostics: Vec<Diagnostic>,
    scopes: Vec<HashMap<String, Type>>,
    current_return_type: Option<Type>,
    interfaces: HashMap<String, InterfaceInfo>,
}

impl TypeChecker {
    pub fn new(config: TypesConfig) -> Self {
        let mut tc = Self {
            config,
            diagnostics: Vec::new(),
            scopes: vec![HashMap::new()],
            current_return_type: None,
            interfaces: HashMap::new(),
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
        for stmt in &module.statements {
            self.check_statement(stmt);
        }
        // No fallar si hay errores; reportar como diagnÃ³stico
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

    // â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
    // Statements
    // â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

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
                    signatures,
                });
                if !self.config.strict {
                    self.warn(&format!("interface '{}' solo tiene efecto en type-checker", i.name), i.span);
                }
                Type::Void
            }
            Statement::TypeAlias(t) => {
                self.check_type_alias(t);
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
            Statement::Import(_) | Statement::FromImport(_) | Statement::Include(_) => Type::Any,
            Statement::Config(_) | Statement::Meta(_) => Type::Void,
            Statement::Cmx(_) => Type::Cmx,
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

        if self.config.strict && !check_type.is_assignable_to(&declared) {
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
        declared
    }

    fn check_function_decl(&mut self, func: &FunctionDecl) -> Type {
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

        self.define(&func.name, fn_type);

        // Verificar cuerpo con tipo de retorno esperado
        let prev_return = self.current_return_type.replace(return_type.clone());
        self.push_scope();
        for (name, typ) in &param_types {
            self.define(name, typ.clone());
        }
        self.check_block(&func.body);
        self.pop_scope();
        self.current_return_type = prev_return;

        return_type
    }

    fn check_if(&mut self, i: &IfStatement) -> Type {
        let cond = self.check_expression(&i.condition);
        if !cond.is_assignable_to(&Type::Bool) {
            self.warn(
                &format!("CondiciÃ³n if debe ser Bool, encontrÃ³ {}", cond),
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
                &format!("CondiciÃ³n while debe ser Bool, encontrÃ³ {}", cond),
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
        if let Some(_cond) = &f.condition {
            // check condition
        }
        if let Some(_upd) = &f.update {
            // check update
        }
        self.check_block(&f.block);
        self.pop_scope();
        Type::Void
    }

    fn check_foreach(&mut self, fe: &ForEachStatement) -> Type {
        let _iter = self.check_expression(&fe.iterable);
        self.push_scope();
        self.define(&fe.item_name, Type::Any);
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
                .unwrap_or(Type::Any);
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

    // â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
    // Expressions
    // â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

    fn check_expression(&mut self, expr: &Expression) -> Type {
        match expr {
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
            Expression::StringInterpolation(_) => Type::String,
            Expression::Cmx(_) => Type::Cmx,
            Expression::NamespaceAccess(_, _, span) => {
                self.error("Namespace access sin tipo", span.clone())
            }
            Expression::Await(expr, _) => self.check_expression(expr),
        }
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

        let left = self.check_expression(&bin.left);
        let right = self.check_expression(&bin.right);

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
                self.error(
                    &format!("Operador + no soportado entre {} y {}", left, right),
                    bin.span.clone(),
                )
            }
            Operator::Minus | Operator::Star | Operator::Slash | Operator::Percent => {
                let l_ok = matches!(left, Type::Int | Type::Float | Type::I32 | Type::I64);
                let r_ok = matches!(right, Type::Int | Type::Float | Type::I32 | Type::I64);
                if l_ok && r_ok {
                    if matches!(left, Type::Float) || matches!(right, Type::Float) {
                        return Type::Float;
                    }
                    return Type::Int;
                }
                self.error(
                    &format!("Operador requiere tipos numÃ©ricos, encontrÃ³ {} y {}", left, right),
                    bin.span.clone(),
                )
            }
            Operator::StrictEqual | Operator::NotEqual
            | Operator::LessThan | Operator::LessEqual
            | Operator::GreaterThan | Operator::GreaterEqual => {
                Type::Bool
            }
            Operator::And | Operator::Or => {
                if !left.is_assignable_to(&Type::Bool) || !right.is_assignable_to(&Type::Bool) {
                    self.warn("Operador lÃ³gico requiere Bool", bin.span.clone());
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

        // Verificar args
        for arg in &call.args {
            self.check_expression(arg);
        }

        match callee_type {
            Type::Fun(params, ret) => {
                if self.config.strict && params.len() != call.args.len() {
                    self.warn(
                        &format!(
                            "FunciÃ³n espera {} args, recibiÃ³ {}",
                            params.len(),
                            call.args.len()
                        ),
                        call.span.clone(),
                    );
                }
                *ret
            }
            Type::Named(_, _) => {
                // Struct constructor â€” devuelve el tipo del struct
                callee_type.clone()
            }
            Type::Any => Type::Any,
            _ => self.error(
                &format!("No se puede llamar como funciÃ³n: {}", callee_type),
                call.span.clone(),
            ),
        }
    }

    fn check_member_access(&mut self, member: &MemberAccessExpr) -> Type {
        self.check_expression(&member.object);
        Type::Any // No podemos saber el tipo del miembro en tiempo de compilaciÃ³n
    }

    fn check_index(&mut self, idx: &IndexExpr) -> Type {
        let obj = self.check_expression(&idx.object);
        let index_type = self.check_expression(&idx.index);
        match obj {
            Type::Array(inner) => *inner,
            Type::Record(k, v) => *v,
            // Tupla: Ã­ndice literal â†’ slot exacto; dinÃ¡mico â†’ uniÃ³n de slots
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
        let mut value_type = Type::Any;
        for (_, expr) in &rec.entries {
            let t = self.check_expression(expr);
            if matches!(value_type, Type::Any) {
                value_type = t;
            }
        }
        Type::Record(Box::new(Type::String), Box::new(value_type))
    }

    fn check_arrow_function(&mut self, arrow: &ArrowFunctionExpr) -> Type {
        let param_types: Vec<Type> = arrow.params.iter()
            .map(|p| p.type_ann.as_ref()
                .map(|ta| self.resolve_type_annotation(ta))
                .unwrap_or(Type::Any))
            .collect();

        let return_type = arrow.return_type.as_ref()
            .map(|ta| self.resolve_type_annotation(ta))
            .unwrap_or(Type::Any);

        self.push_scope();
        for (param, typ) in arrow.params.iter().zip(param_types.iter()) {
            self.define(&param.name, typ.clone());
        }
        self.check_block(&arrow.body);
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

    // â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
    // Type resolution
    // â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

    pub fn resolve_type_annotation(&mut self, ann: &TypeAnnotation) -> Type {
        self.resolve_annotation_with(ann, &HashMap::new())
    }

    /// Resuelve una anotaciÃ³n bajo un contexto de type params (bindings Tâ†’tipo).
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
            TypeKind::Record(k, v) => {
                Type::Record(
                    Box::new(self.resolve_annotation_with(k, bindings)),
                    Box::new(self.resolve_annotation_with(v, bindings)),
                )
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
                // Type param (T, U) del contexto genÃ©rico
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
                    // Record<K, V> â†’ diccionario tipado
                    "Record" if param_types.len() == 2 => Type::Record(
                        Box::new(param_types[0].clone()),
                        Box::new(param_types[1].clone()),
                    ),
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
        // Caso interface nombrada (con args opcionales): resolver miembros con genÃ©ricos
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
            TypeAccess::Key(key) => match base_type {
                Type::Record(_, v) => *v,
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

    /// Construye bindings Tâ†’tipo para los type params de una interface.
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

    /// Tipos de los campos de una interface en orden (para acceso por Ã­ndice).
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

    /// Tipo `fun(params) -> ret` de una signature, con genÃ©ricos aplicados.
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


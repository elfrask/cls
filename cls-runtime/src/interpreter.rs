use crate::environment::Environment;
use crate::intrinsics::Intrinsics;
use crate::resolver::ModuleResolver;
use crate::value::{FunValue, Value, StructDef, StructField, StructInstance, Pollable, PollState, Promise, ClassDef, ClassInstance};
use cls_core::config::ModuleManifest;
use cls_core::error::{ClsError, ClsResult, Diagnostic, Span, StackFrame};
use cls_core::frontend::ast::*;
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
enum Flow { Normal, Return(Value), Break, Continue }

/// Corrutina de una función async. Guarda el cuerpo y params para ejecutarlos
/// en el poll (por ahora síncrono hasta el primer await; luego suspensible).
pub struct CoroutineTask {
    name: String,
    body: Block,
    params: Vec<cls_core::frontend::ast::Parameter>,
    args: Vec<Value>,
}

impl CoroutineTask {
    pub fn new(
        name: &str,
        body: Block,
        params: Vec<cls_core::frontend::ast::Parameter>,
        args: Vec<Value>,
    ) -> Self {
        Self { name: name.to_string(), body, params, args }
    }
}

impl Pollable for CoroutineTask {
    fn poll(&mut self, interp: &mut Interpreter) -> PollState {
        match interp.run_async_body(&self.body, &self.params, &self.args) {
            Ok(v) => PollState::Ready(v),
            Err(e) => PollState::Rejected(e.to_string()),
        }
    }
}

/// Intérprete tree-walker de CLS
/// Ejecuta el AST directamente, sin compilación intermedia
pub struct Interpreter {
    env: Environment,
    resolver: ModuleResolver,
    diagnostics: Vec<Diagnostic>,
    args: Vec<String>,
    exports: HashSet<String>,
    source_file: String,
    import_trace: Vec<ImportFrame>,
    call_stack: Vec<StackFrame>,
    flow: Flow,
    config: Option<ModuleManifest>,
    structs: HashMap<String, StructDef>,
    classes: HashMap<String, ClassDef>,
    self_stack: Vec<Value>,
}

/// Frame de importación para trace de errores
#[derive(Debug, Clone)]
pub struct ImportFrame {
    pub source_file: String,
    pub module_name: String,
    pub line: u32,
    pub col: u32,
}

impl Interpreter {
    pub fn new(intrinsics: Intrinsics, resolver: ModuleResolver) -> Self {
        let args = intrinsics.args.clone();
        let mut interpreter = Self {
            env: Environment::new(),
            resolver,
            diagnostics: Vec::new(),
            args,
            exports: HashSet::new(),
            source_file: String::new(),
            import_trace: Vec::new(),
            call_stack: Vec::new(),
            flow: Flow::Normal,
            config: None,
            structs: HashMap::new(),
            classes: HashMap::new(),
            self_stack: Vec::new(),
        };
        interpreter.register_intrinsics(intrinsics);
        interpreter
    }

    pub fn set_config(&mut self, config: Option<ModuleManifest>) {
        self.config = config;
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    fn register_intrinsics(&mut self, intrinsics: Intrinsics) {
        // Registra los globales del nodo (print, input, etc.)
        for (name, value) in &intrinsics.globals {
            self.env.define(name, value.clone());
        }

        // Intrínsecos del core
        self.register_core_intrinsics();
    }

    fn register_core_intrinsics(&mut self) {
        // now() → timestamp en ms
        self.env.define("now", Value::Fun(FunValue::new_native(
            "now", vec![],
            |_| {
                use std::time::{SystemTime, UNIX_EPOCH};
                let t = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;
                Ok(Value::Int(t))
            },
        )));

        // exit(code)
        self.env.define("exit", Value::Fun(FunValue::new_native(
            "exit", vec!["code".into()],
            |a| {
                let code = match a.first() {
                    Some(Value::Int(i)) => *i as i32,
                    _ => 0,
                };
                std::process::exit(code);
            },
        )));

        // toString(val)
        self.env.define("toString", Value::Fun(FunValue::new_native(
            "toString", vec!["val".into()],
            |a| {
                let s = a.first().map(|v| v.to_string()).unwrap_or_default();
                Ok(Value::String(s))
            },
        )));

        // int(val)
        self.env.define("int", Value::Fun(FunValue::new_native(
            "int", vec!["val".into()],
            |a| {
                let v = a.first().ok_or(ClsError::RuntimeError("int: esperaba 1 arg".into()))?;
                match v {
                    Value::Int(_) => Ok(v.clone()),
                    Value::Float(f) => Ok(Value::Int(*f as i64)),
                    Value::String(s) => s.parse::<i64>()
                        .map(Value::Int)
                        .map_err(|_| ClsError::RuntimeError(format!("int: no se puede convertir '{}'", s))),
                    _ => Err(ClsError::RuntimeError(format!("int: no se puede convertir {}", v.type_name()))),
                }
            },
        )));

        // str(val)
        self.env.define("str", Value::Fun(FunValue::new_native(
            "str", vec!["val".into()],
            |a| {
                Ok(Value::String(a.first().map(|v| v.to_string()).unwrap_or_default()))
            },
        )));

        // float(val)
        self.env.define("float", Value::Fun(FunValue::new_native(
            "float", vec!["val".into()],
            |a| {
                let v = a.first().ok_or(ClsError::RuntimeError("float: esperaba 1 arg".into()))?;
                match v {
                    Value::Float(_) => Ok(v.clone()),
                    Value::Int(i) => Ok(Value::Float(*i as f64)),
                    Value::String(s) => s.parse::<f64>()
                        .map(Value::Float)
                        .map_err(|_| ClsError::RuntimeError(format!("float: no se puede convertir '{}'", s))),
                    _ => Err(ClsError::RuntimeError(format!("float: no se puede convertir {}", v.type_name()))),
                }
            },
        )));

        // bool(val)
        self.env.define("bool", Value::Fun(FunValue::new_native(
            "bool", vec!["val".into()],
            |a| {
                let v = a.first().unwrap_or(&Value::Null);
                Ok(Value::Bool(v.is_truthy()))
            },
        )));

        // len(val) - longitud de array, string o record
        self.env.define("len", Value::Fun(FunValue::new_native(
            "len", vec!["val".into()],
            |a| {
                let v = a.first().ok_or(ClsError::RuntimeError("len: esperaba 1 arg".into()))?;
                match v {
                    Value::Array(arr) => Ok(Value::Int(arr.len() as i64)),
                    Value::String(s) => Ok(Value::Int(s.len() as i64)),
                    Value::Record(r) => Ok(Value::Int(r.len() as i64)),
                    _ => Err(ClsError::RuntimeError(format!("len: no aplicable a {}", v.type_name()))),
                }
            },
        )));

        // type(val)
        self.env.define("type", Value::Fun(FunValue::new_native(
            "type", vec!["val".into()],
            |a| {
                let name = a.first().map(|v| v.type_name()).unwrap_or("Unknown");
                Ok(Value::String(name.to_string()))
            },
        )));

        // sleep(ms)
        self.env.define("sleep", Value::Fun(FunValue::new_native(
            "sleep", vec!["ms".into()],
            |a| {
                let ms = match a.first() { Some(Value::Int(i)) => *i as u64, _ => 0 };
                std::thread::sleep(std::time::Duration::from_millis(ms));
                Ok(Value::Void)
            },
        )));

        // throw(msg) — lanza un error de runtime intencional
        self.env.define("throw", Value::Fun(FunValue::new_native(
            "throw", vec!["msg".into()],
            |a| {
                let msg = match a.first() {
                    Some(Value::String(s)) => s.clone(),
                    Some(v) => v.to_string(),
                    None => "error".to_string(),
                };
                Err(ClsError::RuntimeError(msg))
            },
        )));

        // push(arr, val), pop(arr)
        self.env.define("push", Value::Fun(FunValue::new_native(
            "push", vec!["arr".into(), "val".into()],
            |a| {
                if a.len() < 2 { return Err(ClsError::RuntimeError("push: esperaba 2 args".into())); }
                // No podemos modificar self.env desde aquí, así que esto es limitado
                // El usuario debe usar arr.push(val) en el futuro
                Err(ClsError::RuntimeError("push: usa arr.push(val) en su lugar".into()))
            },
        )));
    }

    /// Establece el archivo fuente actual (para trace de errores)
    pub fn set_source_file(&mut self, path: String) {
        self.source_file = path;
    }

    /// Obtiene el trace de importaciones para errores
    pub fn get_import_trace(&self) -> &[ImportFrame] {
        &self.import_trace
    }

    /// Construye un ErrorReport desde un error de runtime
    pub fn build_error_report(&self, error: ClsError) -> crate::error_report::ErrorReport {
        crate::error_report::ErrorReport::from_runtime(
            error,
            self.call_stack.clone(),
            &self.import_trace,
            &self.source_file,
        )
    }

    /// Crea un RuntimeError con línea/columna embebidos
    fn err_at(&self, msg: impl Into<String>, span: &Span) -> ClsError {
        ClsError::RuntimeError(format!(
            "{} (línea {}, columna {})",
            msg.into(),
            span.start_line,
            span.start_col
        ))
    }

    /// Ejecuta un módulo completo
    pub fn execute(&mut self, module: &Module) -> ClsResult<Value> {
        let mut result = Value::Void;
        for stmt in &module.statements {
            result = self.execute_statement(stmt)?;
        }
        Ok(result)
    }

    fn execute_statement(&mut self, stmt: &Statement) -> ClsResult<Value> {
        match stmt {
            Statement::VarDecl(var) => self.execute_var_decl(var),
            Statement::ConstDecl(var) => self.execute_const_decl(var),
            Statement::FunctionDecl(func) => self.execute_function_decl(func),
            Statement::If(if_stmt) => self.execute_if(if_stmt),
            Statement::While(while_stmt) => self.execute_while(while_stmt),
            Statement::Loop(block) => self.execute_loop(block),
            Statement::For(for_stmt) => self.execute_for(for_stmt),
            Statement::ForEach(for_each) => self.execute_for_each(for_each),
            Statement::Switch(switch) => self.execute_switch(switch),
            Statement::Try(try_stmt) => self.execute_try(try_stmt),
            Statement::With(with_stmt) => self.execute_with(with_stmt),
            Statement::Return(expr) => {
                let val = if let Some(expr) = expr {
                    self.evaluate_expression(expr)?
                } else {
                    Value::Void
                };
                self.flow = Flow::Return(val.clone());
                Ok(val)
            }
            Statement::Break => {
                self.flow = Flow::Break;
                Ok(Value::Void)
            }
            Statement::Continue => {
                self.flow = Flow::Continue;
                Ok(Value::Void)
            }
            Statement::Expression(expr) => self.evaluate_expression(expr),
            Statement::ClassDecl(class) => self.execute_class_decl(class),
            Statement::StructureDecl(structure) => self.execute_structure_decl(structure),
            Statement::InterfaceDecl(interface) => self.execute_interface_decl(interface),
            Statement::ModuleDecl(module) => self.execute_module_decl(module),
            Statement::NamespaceDecl(ns) => self.execute_namespace_decl(ns),
            Statement::Import(import) => self.execute_import(import),
            Statement::FromImport(from_import) => self.execute_from_import(from_import),
            Statement::Include(include) => self.execute_include(include),
            Statement::Config(_) | Statement::Meta(_) => Ok(Value::Void),
            Statement::Cmx(cmx) => self.evaluate_cmx(cmx),
        }
    }

    fn execute_var_decl(&mut self, var: &VarDecl) -> ClsResult<Value> {
        let value = if let Some(expr) = &var.value {
            self.evaluate_expression(expr)?
        } else {
            Value::Null
        };
        self.env.define(&var.name, value.clone());
        if matches!(var.visibility, Visibility::Export) {
            self.exports.insert(var.name.clone());
        }
        Ok(value)
    }

    fn execute_const_decl(&mut self, var: &VarDecl) -> ClsResult<Value> {
        let value = if let Some(expr) = &var.value {
            self.evaluate_expression(expr)?
        } else {
            Value::Null
        };
        self.env.define_const(&var.name, value.clone());
        if matches!(var.visibility, Visibility::Export) {
            self.exports.insert(var.name.clone());
        }
        Ok(value)
    }

    fn execute_function_decl(&mut self, func: &FunctionDecl) -> ClsResult<Value> {
        let is_async = func.modifiers.iter().any(|m| matches!(m, FunctionModifier::Async));
        // Capturar entorno léxico (closures) para module/namespace/scope
        let closure = Arc::new(Mutex::new(self.env.clone()));
        let fun_val = if is_async {
            Value::Fun(FunValue::new_async_user_with_closure(&func.name, func.params.clone(), func.body.clone(), closure.clone()))
        } else {
            Value::Fun(FunValue::new_user_with_closure(&func.name, func.params.clone(), func.body.clone(), closure.clone()))
        };
        // Insertar la función en su propio closure para permitir recursión
        closure.lock().unwrap().define(&func.name, fun_val.clone());
        self.env.define(&func.name, fun_val);
        if matches!(func.visibility, Visibility::Export) {
            self.exports.insert(func.name.clone());
        }
        Ok(Value::Void)
    }

    fn execute_if(&mut self, if_stmt: &IfStatement) -> ClsResult<Value> {
        let condition = self.evaluate_expression(&if_stmt.condition)?;
        if condition.is_truthy() {
            return self.execute_block(&if_stmt.then_block);
        }
        for elif in &if_stmt.elif_branches {
            let cond = self.evaluate_expression(&elif.condition)?;
            if cond.is_truthy() {
                return self.execute_block(&elif.block);
            }
        }
        if let Some(else_block) = &if_stmt.else_block {
            return self.execute_block(else_block);
        }
        Ok(Value::Void)
    }

    fn execute_while(&mut self, while_stmt: &WhileStatement) -> ClsResult<Value> {
        loop {
            self.flow = Flow::Normal;
            let condition = self.evaluate_expression(&while_stmt.condition)?;
            if !condition.is_truthy() { break; }
            self.execute_block(&while_stmt.block)?;
            match std::mem::replace(&mut self.flow, Flow::Normal) {
                Flow::Break => break,
                Flow::Continue => continue,
                Flow::Return(v) => { self.flow = Flow::Return(v); break; }
                _ => continue,
            }
        }
        Ok(Value::Void)
    }

    fn execute_loop(&mut self, block: &Block) -> ClsResult<Value> {
        loop {
            self.flow = Flow::Normal;
            self.execute_block(block)?;
            match std::mem::replace(&mut self.flow, Flow::Normal) {
                Flow::Break => break,
                Flow::Continue => continue,
                Flow::Return(v) => { self.flow = Flow::Return(v); break; }
                Flow::Normal => continue,
            }
        }
        Ok(Value::Void)
    }

    fn execute_for(&mut self, for_stmt: &ForStatement) -> ClsResult<Value> {
        if let Some(init) = &for_stmt.init {
            self.execute_statement(init)?;
        }
        loop {
            self.flow = Flow::Normal;
            if let Some(cond) = &for_stmt.condition {
                let cond_val = self.evaluate_expression(cond)?;
                if !cond_val.is_truthy() { break; }
            }
            self.execute_block(&for_stmt.block)?;
            let flow = std::mem::replace(&mut self.flow, Flow::Normal);
            if matches!(flow, Flow::Return(_)) { self.flow = flow; break; }
            if matches!(flow, Flow::Break) { break; }
            if let Some(update) = &for_stmt.update {
                self.evaluate_expression(update)?;
            }
        }
        Ok(Value::Void)
    }

    fn execute_for_each(&mut self, for_each: &ForEachStatement) -> ClsResult<Value> {
        let iterable = self.evaluate_expression(&for_each.iterable)?;
        match iterable {
            Value::Array(arr) => {
                for (idx, item) in arr.iter().enumerate() {
                    self.env.push_scope();
                    self.env.define(&for_each.item_name, item.clone());
                    if let Some(idx_name) = &for_each.index_name {
                        self.env.define(idx_name, Value::Int(idx as i64));
                    }
                    self.execute_block(&for_each.block)?;
                    self.env.pop_scope();
                }
                Ok(Value::Void)
            }
            _ => Err(self.err_at(format!("No se puede iterar sobre: {:?}", iterable), &for_each.span)),
        }
    }

    fn execute_switch(&mut self, switch: &SwitchStatement) -> ClsResult<Value> {
        let value = self.evaluate_expression(&switch.value)?;
        for case in &switch.cases {
            let pattern_val = match &case.pattern {
                CasePattern::Literal(lit) => self.evaluate_literal(lit)?,
                CasePattern::Identifier(_) => continue, // TODO
                CasePattern::Default => continue,
            };
            if value == pattern_val {
                return self.execute_block(&case.block);
            }
        }
        if let Some(default) = &switch.default {
            return self.execute_block(default);
        }
        Ok(Value::Void)
    }

    fn execute_try(&mut self, try_stmt: &TryStatement) -> ClsResult<Value> {
        match self.execute_block(&try_stmt.try_block) {
            Ok(v) => Ok(v),
            Err(e) => {
                for catch in &try_stmt.catch_clauses {
                    self.env.push_scope();
                    // TODO: crear objeto de error
                    let err_val = Value::String(e.to_string());
                    self.env.define(&catch.param_name, err_val);
                    let result = self.execute_block(&catch.block);
                    self.env.pop_scope();
                    return result;
                }
                if let Some(finally) = &try_stmt.finally_block {
                    self.execute_block(finally)?;
                }
                Err(e)
            }
        }
    }

    fn execute_with(&mut self, with_stmt: &WithStatement) -> ClsResult<Value> {
        let value = self.evaluate_expression(&with_stmt.value)?;
        self.env.push_scope();
        self.env.define(&with_stmt.name, value);
        let result = self.execute_block(&with_stmt.block);
        self.env.pop_scope();
        result
    }

    fn execute_class_decl(&mut self, class: &ClassDecl) -> ClsResult<Value> {
        let mut methods: HashMap<String, FunValue> = HashMap::new();
        let mut field_defaults: HashMap<String, Option<Value>> = HashMap::new();
        let mut ctor: Option<FunctionDecl> = None;

        for member in &class.body {
            match member {
                ClassMember::Method(f) => {
                    let fun = FunValue::new_user(&f.name, f.params.clone(), f.body.clone());
                    methods.insert(f.name.clone(), fun);
                }
                ClassMember::Property(v) => {
                    let default = if let Some(expr) = &v.value {
                        Some(self.evaluate_expression(expr)?)
                    } else {
                        None
                    };
                    field_defaults.insert(v.name.clone(), default);
                }
                ClassMember::Constructor(f) => {
                    ctor = Some(f.clone());
                }
            }
        }

        let mut def = ClassDef {
            name: class.name.clone(),
            extends: class.extends.clone(),
            methods,
            field_defaults,
            ctor,
        };

        // Herencia: copiar métodos y fields del padre (si ya está registrado)
        if let Some(parent_name) = &class.extends {
            if let Some(parent) = self.classes.get(parent_name) {
                for (k, v) in &parent.methods {
                    def.methods.entry(k.clone()).or_insert_with(|| v.clone());
                }
                for (k, v) in &parent.field_defaults {
                    def.field_defaults.entry(k.clone()).or_insert_with(|| v.clone());
                }
                if def.ctor.is_none() {
                    def.ctor = parent.ctor.clone();
                }
            }
        }

        self.classes.insert(class.name.clone(), def.clone());
        self.env.define(&class.name, Value::Class(Box::new(def)));
        Ok(Value::Void)
    }

    fn execute_structure_decl(&mut self, structure: &StructureDecl) -> ClsResult<Value> {
        let fields: Vec<StructField> = structure.fields.iter()
            .map(|f| StructField { name: f.name.clone() })
            .collect();

        let struct_name = structure.name.clone();
        let def = StructDef {
            name: struct_name.clone(),
            fields: fields.clone(),
        };
        self.structs.insert(struct_name.clone(), def);

        let field_names: Vec<String> = fields.iter().map(|f| f.name.clone()).collect();
        let closure_fields = fields.clone();
        let closure_name = struct_name.clone();
        let fn_display = struct_name.clone();
        let constructor = FunValue::new_native(
            &fn_display,
            field_names,
            move |a| {
                let values: Vec<Value> = a.to_vec();
                if values.len() != closure_fields.len() {
                    return Err(ClsError::RuntimeError(format!(
                        "{}: esperaba {} argumentos, se recibieron {}",
                        closure_name, closure_fields.len(), values.len()
                    )));
                }
                Ok(Value::Struct(Box::new(StructInstance {
                    def_name: closure_name.clone(),
                    fields: values,
                })))
            },
        );
        self.env.define(&struct_name, Value::Fun(constructor));

        Ok(Value::Void)
    }

    fn execute_interface_decl(&mut self, _interface: &InterfaceDecl) -> ClsResult<Value> {
        // Las interfaces solo tienen impacto en type-checker
        Ok(Value::Void)
    }

    fn execute_module_decl(&mut self, module: &ModuleDecl) -> ClsResult<Value> {
        // Ejecutar el body en un entorno aislado
        let saved_env = std::mem::replace(&mut self.env, Environment::new());
        for stmt in &module.body {
            self.execute_statement(stmt)?;
        }
        let entries = self.env.all();
        self.env = saved_env;
        // Registrar el módulo como Record en el scope actual
        self.env.define(&module.name, Value::Record(entries));
        Ok(Value::Void)
    }

    fn execute_namespace_decl(&mut self, ns: &NamespaceDecl) -> ClsResult<Value> {
        // Namespace es igual que module en runtime: un Record aislado
        let saved_env = std::mem::replace(&mut self.env, Environment::new());
        for stmt in &ns.body {
            self.execute_statement(stmt)?;
        }
        let entries = self.env.all();
        self.env = saved_env;
        self.env.define(&ns.name, Value::Record(entries));
        Ok(Value::Void)
    }

    fn execute_import(&mut self, import: &ImportStatement) -> ClsResult<Value> {
        // Registrar frame para trace de errores
        self.import_trace.push(ImportFrame {
            source_file: self.source_file.clone(),
            module_name: import.path.clone(),
            line: import.span.start_line,
            col: import.span.start_col,
        });

        let result = self.resolver.resolve(&import.path, &mut self.env);

        match result {
            Ok(module) => {
                let alias = import.alias.as_deref().unwrap_or(&import.path);
                self.env.define(alias, module);
                Ok(Value::Void)
            }
            Err(e) => {
                // Dejar que el error se propague tal cual (el trace ya tiene la info)
                Err(e)
            }
        }
    }

    fn execute_from_import(&mut self, fi: &FromImportStatement) -> ClsResult<Value> {
        let module = self.resolver.resolve(&fi.path, &mut self.env)?;
        match module {
            Value::Record(entries) => {
                for im in &fi.names {
                    let alias = im.alias.as_deref().unwrap_or(&im.name);
                    if let Some(val) = entries.get(&im.name) {
                        self.env.define(alias, val.clone());
                    } else {
                        return Err(self.err_at(format!("'{}' no existe en el módulo '{}'", im.name, fi.path), &fi.span));
                    }
                }
            }
            _ => return Err(self.err_at(format!("'{}' no es un módulo (no tiene exports)", fi.path), &fi.span)),
        }
        Ok(Value::Void)
    }

    fn execute_include(&mut self, include: &IncludeStatement) -> ClsResult<Value> {
        let module = self.resolver.resolve(&include.path, &mut self.env)?;
        match module {
            Value::Record(entries) => {
                for (name, val) in &entries {
                    self.env.define(name, val.clone());
                }
            }
            _ => return Err(self.err_at(format!("'{}' no es un módulo", include.path), &include.span)),
        }
        Ok(Value::Void)
    }

    fn execute_block(&mut self, block: &Block) -> ClsResult<Value> {
        self.env.push_scope();
        let mut result = Value::Void;
        for stmt in &block.statements {
            result = self.execute_statement(stmt)?;
            if !matches!(self.flow, Flow::Normal) { break; }
        }
        self.env.pop_scope();
        Ok(result)
    }

    /// Ejecuta el cuerpo de una corrutina (async fn) con params bindeados.
    fn run_async_body(&mut self, body: &Block, params: &[cls_core::frontend::ast::Parameter], args: &[Value]) -> ClsResult<Value> {
        self.env.push_scope();
        for (i, param) in params.iter().enumerate() {
            let arg_val = if i < args.len() {
                args[i].clone()
            } else if let Some(default) = &param.default_value {
                self.evaluate_expression(default)?
            } else {
                Value::Null
            };
            self.env.define(&param.name, arg_val);
        }
        let result = self.execute_block(body);
        let result = match std::mem::replace(&mut self.flow, Flow::Normal) {
            Flow::Return(val) => Ok(val),
            _ => result,
        };
        self.env.pop_scope();
        result
    }

    fn evaluate_expression(&mut self, expr: &Expression) -> ClsResult<Value> {
        match expr {
            Expression::Literal(lit) => self.evaluate_literal(lit),
            Expression::Identifier(name, span) => {
                if name == "me" {
                    return Ok(self.self_stack.last().cloned().unwrap_or(Value::Null));
                }
                self.env.get(name).cloned().ok_or_else(|| {
                    self.err_at(format!("Variable no definida: {}", name), span)
                })
            }
            Expression::Binary(bin) => self.evaluate_binary(bin),
            Expression::Unary(un) => self.evaluate_unary(un),
            Expression::Call(call) => self.evaluate_call(call),
            Expression::MemberAccess(member) => self.evaluate_member_access(member),
            Expression::Index(idx) => self.evaluate_index(idx),
            Expression::Array(arr) => self.evaluate_array(arr),
            Expression::Record(rec) => self.evaluate_record(rec),
            Expression::ArrowFunction(arrow) => self.evaluate_arrow_function(arrow),
            Expression::Conditional(cond) => self.evaluate_conditional(cond),
            Expression::Assignment(assign) => self.evaluate_assignment(assign),
            Expression::Parenthesized(inner, _) => self.evaluate_expression(inner),
            Expression::StringInterpolation(interp) => self.evaluate_string_interpolation(interp),
            Expression::Cmx(cmx) => self.evaluate_cmx(cmx),
            Expression::NamespaceAccess(ns, name, span) => self.evaluate_namespace_access(ns, name, span),
            Expression::Await(expr, _) => self.evaluate_await(expr),
        }
    }

    fn evaluate_await(&mut self, expr: &Expression) -> ClsResult<Value> {
        let value = self.evaluate_expression(expr)?;
        match value {
            Value::Promise(mut p) => {
                match p.poll(self) {
                    PollState::Ready(v) => Ok(v),
                    PollState::Rejected(e) => Err(ClsError::RuntimeError(e)),
                    PollState::Pending => Ok(Value::Void),
                }
            }
            other => Ok(other),
        }
    }

    fn evaluate_literal(&mut self, lit: &Literal) -> ClsResult<Value> {
        Ok(match &lit.kind {
            LiteralKind::Int(v) => Value::Int(*v),
            LiteralKind::Float(v) => Value::Float(*v),
            LiteralKind::String(v) => Value::String(v.clone()),
            LiteralKind::Bool(v) => Value::Bool(*v),
            LiteralKind::Char(v) => Value::Char(*v),
            LiteralKind::Null => Value::Null,
            LiteralKind::Unknown => Value::Unknown,
        })
    }

    fn evaluate_binary(&mut self, bin: &BinaryExpr) -> ClsResult<Value> {
        use cls_core::frontend::token::Operator;

        let left = self.evaluate_expression(&bin.left)?;

        // Short-circuit para lógicos: evaluar right solo si hace falta
        match bin.op {
            Operator::And => {
                if !left.is_truthy() { return Ok(Value::Bool(false)); }
                let right = self.evaluate_expression(&bin.right)?;
                return Ok(Value::Bool(right.is_truthy()));
            }
            Operator::Or => {
                if left.is_truthy() { return Ok(Value::Bool(true)); }
                let right = self.evaluate_expression(&bin.right)?;
                return Ok(Value::Bool(right.is_truthy()));
            }
            _ => {}
        }

        let right = self.evaluate_expression(&bin.right)?;
        self.evaluate_binary_values(left, bin.op, right, &bin.span)
    }

    fn evaluate_unary(&mut self, un: &UnaryExpr) -> ClsResult<Value> {
        let operand = self.evaluate_expression(&un.operand)?;
        match un.op {
            UnaryOp::Negate => match operand {
                Value::Int(v) => Ok(Value::Int(-v)),
                Value::Float(v) => Ok(Value::Float(-v)),
                _ => Err(self.err_at(format!("No se puede negar: {:?}", operand), &un.span)),
            },
            UnaryOp::Not => {
                let val = operand.is_truthy();
                Ok(Value::Bool(!val))
            }
            UnaryOp::BitwiseNot => match operand {
                Value::Int(v) => Ok(Value::Int(!v)),
                _ => Err(self.err_at(format!("No se puede aplicar ~: {:?}", operand), &un.span)),
            },
            UnaryOp::TypeOf => Ok(Value::String(operand.type_name().to_string())),
            UnaryOp::PostInc | UnaryOp::PreInc | UnaryOp::PostDec | UnaryOp::PreDec => {
                let name = match &*un.operand {
                    Expression::Identifier(n, _) => n.clone(),
                    _ => return Err(self.err_at("++/-- requiere identificador", &un.span)),
                };
                let delta: i64 = match un.op { UnaryOp::PostInc | UnaryOp::PreInc => 1, _ => -1 };
                if self.env.is_const(&name) {
                    return Err(self.err_at(format!("No se puede modificar la constante '{}'", name), &un.span));
                }
                let new_val = match &operand {
                    Value::Int(v) => Value::Int(v + delta),
                    Value::Float(f) => Value::Float(f + delta as f64),
                    _ => return Err(self.err_at("++/-- solo aplica a números", &un.span)),
                };
                self.env.set(&name, new_val);
                match un.op {
                    UnaryOp::PreInc | UnaryOp::PreDec => self.env.get(&name).cloned()
                        .ok_or_else(|| self.err_at(format!("Variable no definida: {}", name), &un.span)),
                    UnaryOp::PostInc | UnaryOp::PostDec => Ok(operand),
                    _ => unreachable!(),
                }
            }
        }
    }

    fn evaluate_call(&mut self, call: &CallExpr) -> ClsResult<Value> {
        // Detectar obj.method(args) para pasar `me`
        if let Expression::MemberAccess(member) = &*call.callee {
            let obj = self.evaluate_expression(&member.object)?;
            let mut args = Vec::new();
            for arg in &call.args {
                args.push(self.evaluate_expression(arg)?);
            }
            match obj {
                Value::Object(obj_val) => {
                    let (result, mutated) = self.call_method(obj_val, &member.member, args, &call.span)?;
                    // Re-insertar el objeto mutado en el env si es una variable
                    if let Expression::Identifier(name, _) = &*member.object {
                        self.env.set(name, Value::Object(Box::new(mutated)));
                    }
                    return Ok(result);
                }
                other => {
                    // Resolver miembro y llamar como función
                    let callee = match other {
                        Value::Record(rec) => rec.get(&member.member).cloned()
                            .ok_or_else(|| self.err_at(format!("Miembro '{}' no encontrado", member.member), &call.span))?,
                        _ => {
                            let member_access = MemberAccessExpr {
                                object: Box::new(Expression::Literal(Literal { kind: LiteralKind::Null, span: call.span })),
                                member: member.member.clone(),
                                span: member.span,
                            };
                            return self.evaluate_member_access(&member_access).and_then(|c| self.call_function_value(c, args, &call.span));
                        }
                    };
                    return self.call_function_value(callee, args, &call.span);
                }
            }
        }

        let callee = self.evaluate_expression(&call.callee)?;
        let mut args = Vec::new();
        for arg in &call.args {
            args.push(self.evaluate_expression(arg)?);
        }

        self.call_function_value(callee, args, &call.span)
    }

    #[allow(dead_code)]
    fn evaluate_member_call(&mut self, call: &CallExpr) -> ClsResult<Value> {
        // TODO: método de objeto/clase
        Err(self.err_at("Member call no implementado", &call.span))
    }

    /// Instancia una clase: crea el objeto, corre el constructor con `me`
    fn instantiate_class(&mut self, def: Box<ClassDef>, args: Vec<Value>) -> ClsResult<Value> {
        let mut fields = HashMap::new();
        for (k, v) in &def.field_defaults {
            fields.insert(k.clone(), v.clone().unwrap_or(Value::Null));
        }
        let instance = ClassInstance {
            class_name: def.name.clone(),
            fields,
            methods: def.methods.clone(),
        };

        if let Some(ctor) = &def.ctor {
            self.self_stack.push(Value::Object(Box::new(instance.clone())));
            let result = self.run_async_body(&ctor.body, &ctor.params, &args);
            // Recuperar la instancia mutada por el constructor
            let instance = match self.self_stack.pop() {
                Some(Value::Object(mutated)) => *mutated,
                _ => instance,
            };
            let _ = result?;
            return Ok(Value::Object(Box::new(instance)));
        }

        Ok(Value::Object(Box::new(instance)))
    }

    /// Llama un método de un objeto con `me` = objeto.
    /// Devuelve el resultado y el objeto (posiblemente mutado por `me.field = ...`).
    fn call_method(&mut self, obj: Box<ClassInstance>, name: &str, args: Vec<Value>, span: &Span) -> ClsResult<(Value, ClassInstance)> {
        let method = obj.methods.get(name).cloned()
            .ok_or_else(|| self.err_at(format!("Método '{}' no encontrado en '{}'", name, obj.class_name), span))?;
        let original = obj.clone();
        self.self_stack.push(Value::Object(obj));
        let result = self.call_function_value(Value::Fun(method), args, span);
        let mutated = match self.self_stack.pop() {
            Some(Value::Object(o)) => *o,
            _ => *original,
        };
        result.map(|v| (v, mutated))
    }

    /// Ejecuta un valor de función con argumentos ya evaluados
    fn call_function_value(&mut self, callee: Value, args: Vec<Value>, call_span: &Span) -> ClsResult<Value> {
        let fn_name = match &callee {
            Value::Fun(f) => f.name.clone(),
            _ => "<unknown>".to_string(),
        };
        let is_async = match &callee {
            Value::Fun(f) => f.is_async,
            _ => false,
        };
        let current_frame = StackFrame::new(&fn_name, Some(*call_span), &self.source_file);
        self.call_stack.push(current_frame.clone());

        let result = match callee {
            Value::Fun(fun) => match &fun.kind {
                crate::value::FunKind::Native { params: _, func } => {
                    func(&args)
                }
                crate::value::FunKind::User { params, body, closure } => {
                    if is_async {
                        // Función async: crear Promise lazy, NO ejecutar el cuerpo aún
                        let task = CoroutineTask::new(&fun.name, body.clone(), params.clone(), args);
                        Ok(Value::Promise(Promise::new(Box::new(task))))
                    } else {
                        // Si hay closure (definida en module/namespace), usarlo como base
                        let saved = if let Some(c) = closure {
                            let base = c.lock().unwrap().clone();
                            Some(std::mem::replace(&mut self.env, base))
                        } else {
                            None
                        };
                        self.env.push_scope();
                        for (i, param) in params.iter().enumerate() {
                            let arg_val = if i < args.len() {
                                args[i].clone()
                            } else if let Some(default) = &param.default_value {
                                self.evaluate_expression(default)?
                            } else {
                                Value::Null
                            };
                            self.env.define(&param.name, arg_val);
                        }
                        let result = self.execute_block(body);
                        let result = match std::mem::replace(&mut self.flow, Flow::Normal) {
                            Flow::Return(val) => Ok(val),
                            _ => result,
                        };
                        self.env.pop_scope();
                        if let Some(s) = saved { self.env = s; }
                        result
                    }
                }
            },
            Value::Class(def) => self.instantiate_class(def, args),
            _ => Err(self.err_at("No se puede llamar", call_span)),
        };

        self.call_stack.pop();
        result.map_err(|e| {
            let err_msg = e.to_string();
            // Si el error ya tiene call stack, no duplicar
            if err_msg.contains("\n  Call stack:") {
                return e;
            }
            let stack_trace: Vec<String> = self.call_stack.iter()
                .map(|f| {
                    if let Some(s) = &f.span {
                        format!("{} ({}:{})", f.function, f.source_file, s)
                    } else {
                        f.function.clone()
                    }
                })
                .collect();
            let trace_str = stack_trace.join("\n  → ");
            let span_display = if let Some(s) = &current_frame.span {
                format!(" (línea {}, columna {})", s.start_line, s.start_col)
            } else {
                String::new()
            };
            ClsError::RuntimeError(format!(
                "{}\n  Call stack:\n  → {} → {}{}",
                err_msg, trace_str, current_frame.function, span_display
            ))
        })
    }

    /// Carga un archivo .ccls como módulo, devolviendo solo lo exportado.
    pub fn load_module_source(&mut self, module_name: &str, source: &str) -> ClsResult<Value> {
        // Compilar el módulo externo
        let mut lexer = cls_core::frontend::Lexer::new(source);
        let tokens = lexer.tokenize().map_err(|e| {
            ClsError::RuntimeError(format!("Tokenización de '{}': {}", module_name, e))
        })?;
        let mut parser = cls_core::frontend::Parser::new(tokens);
        let module = parser.parse().map_err(|e| {
            ClsError::RuntimeError(format!("Parseo de '{}': {}", module_name, e))
        })?;

        // Guardar estado actual (scope global, exports, resolver)
        let saved_exports = std::mem::take(&mut self.exports);
        let saved_env = std::mem::replace(&mut self.env, Environment::new());
        let saved_resolver = std::mem::replace(
            &mut self.resolver,
            ModuleResolver::new().with_core_stdlib(),
        );

        // Ejecutar el módulo en scope aislado
        for stmt in &module.statements {
            self.execute_statement(stmt)?;
        }

        // Recolectar solo exportados
        let mut entries = std::collections::HashMap::new();
        for name in &self.exports {
            if let Some(val) = self.env.get(name) {
                entries.insert(name.clone(), val.clone());
            }
        }

        // Restaurar estado
        self.exports = saved_exports;
        self.env = saved_env;
        self.resolver = saved_resolver;

        Ok(Value::Record(entries))
    }

    /// Llama a main() con los args del CLI.
    /// Retorna el código de salida (i32).
    pub fn call_main(&mut self) -> ClsResult<i32> {
        if let Some(main_val) = self.env.get("main").cloned() {
            let args_val = Value::Array(
                self.args.iter().map(|a| Value::String(a.clone())).collect(),
            );
            let dummy_span = Span::new(0, 0, 0, 0);
            match self.call_function_value(main_val, vec![args_val], &dummy_span) {
                Ok(Value::Int(code)) => Ok(code as i32),
                Ok(_) => Ok(0),
                Err(e) => Err(e),
            }
        } else {
            Ok(0)
        }
    }

    fn evaluate_member_access(&mut self, member: &MemberAccessExpr) -> ClsResult<Value> {
        let object = self.evaluate_expression(&member.object)?;
        match object {
            Value::Record(rec) => {
                rec.get(&member.member).cloned().ok_or_else(|| {
                    self.err_at(format!("Miembro no encontrado: '{}'", member.member), &member.span)
                })
            }
            Value::Struct(inst) => {
                let def = self.structs.get(&inst.def_name).ok_or_else(|| {
                    self.err_at(format!("Struct '{}' no definido", inst.def_name), &member.span)
                })?;
                let idx = def.fields.iter().position(|f| f.name == member.member).ok_or_else(|| {
                    self.err_at(format!("Campo '{}' no encontrado en struct '{}'", member.member, inst.def_name), &member.span)
                })?;
                Ok(inst.fields[idx].clone())
            }
            Value::Cmx(ref cmx) => match member.member.as_str() {
                "tag" => Ok(cmx.tag.clone()),
                "props" => Ok(Value::Record(cmx.props.clone())),
                "children" => Ok(Value::Array(cmx.children.clone())),
                name => cmx.props.get(name).cloned().ok_or_else(|| {
                    self.err_at(format!("Propiedad CMX no encontrada: '{}'", name), &member.span)
                }),
            },
            Value::Object(obj) => {
                // 1. field
                if let Some(v) = obj.fields.get(&member.member) {
                    return Ok(v.clone());
                }
                // 2. method
                if let Some(m) = obj.methods.get(&member.member) {
                    return Ok(Value::Fun(m.clone()));
                }
                Err(self.err_at(format!("Miembro '{}' no encontrado en '{}'", member.member, obj.class_name), &member.span))
            }
            Value::Class(class) => {
                // Acceso a clase: métodos estáticos o constructor name
                if let Some(m) = class.methods.get(&member.member) {
                    return Ok(Value::Fun(m.clone()));
                }
                Err(self.err_at(format!("Miembro '{}' no encontrado en clase '{}'", member.member, class.name), &member.span))
            }
            _ => Err(self.err_at(format!("No se puede acceder a miembro en: {:?}", object), &member.span)),
        }
    }

    fn evaluate_index(&mut self, idx: &IndexExpr) -> ClsResult<Value> {
        let object = self.evaluate_expression(&idx.object)?;
        let index = self.evaluate_expression(&idx.index)?;
        match (object, index) {
            (Value::Array(arr), Value::Int(i)) => {
                if i < 0 || i >= arr.len() as i64 {
                    Err(self.err_at(format!("Índice fuera de rango: {}", i), &idx.span))
                } else {
                    Ok(arr[i as usize].clone())
                }
            }
            (Value::Record(rec), Value::String(key)) => {
                rec.get(&key).cloned().ok_or_else(|| {
                    self.err_at(format!("Key no encontrada: {}", key), &idx.span)
                })
            }
            _ => Err(self.err_at("Indexado no soportado", &idx.span)),
        }
    }

    fn evaluate_array(&mut self, arr: &ArrayExpr) -> ClsResult<Value> {
        let mut elements = Vec::new();
        for elem in &arr.elements {
            elements.push(self.evaluate_expression(elem)?);
        }
        Ok(Value::Array(elements))
    }

    fn evaluate_record(&mut self, rec: &RecordExpr) -> ClsResult<Value> {
        let mut entries = std::collections::HashMap::new();
        for (key, expr) in &rec.entries {
            entries.insert(key.clone(), self.evaluate_expression(expr)?);
        }
        Ok(Value::Record(entries))
    }

    fn evaluate_arrow_function(&mut self, arrow: &ArrowFunctionExpr) -> ClsResult<Value> {
        // body ya es un Block (multi-statement o Return implícito del parser)
        Ok(Value::Fun(FunValue::new_user("<anonymous>", arrow.params.clone(), *arrow.body.clone())))
    }

    fn evaluate_conditional(&mut self, cond: &ConditionalExpr) -> ClsResult<Value> {
        let condition = self.evaluate_expression(&cond.condition)?;
        if condition.is_truthy() {
            self.evaluate_expression(&cond.then_expr)
        } else {
            self.evaluate_expression(&cond.else_expr)
        }
    }

    fn evaluate_assignment(&mut self, assign: &AssignmentExpr) -> ClsResult<Value> {
        use cls_core::frontend::token::Operator;

        let value = self.evaluate_expression(&assign.value)?;

        // Operadores compuestos: target += val  →  target = target + val
        let new_value = if assign.op != Operator::Equal {
            let current = self.read_target(&assign.target, &assign.span)?;
            self.apply_compound(current, assign.op, value, &assign.span)?
        } else {
            value
        };

        self.write_target(&assign.target, new_value.clone(), &assign.span)?;
        Ok(new_value)
    }

    /// Lee el valor actual de un target (Identifier | MemberAccess | Index)
    fn read_target(&mut self, target: &Expression, span: &Span) -> ClsResult<Value> {
        match target {
            Expression::Identifier(name, _) => {
                if name == "me" {
                    Ok(self.self_stack.last().cloned().unwrap_or(Value::Null))
                } else {
                    self.env.get(name).cloned()
                        .ok_or_else(|| self.err_at(format!("Variable no definida: {}", name), span))
                }
            }
            Expression::MemberAccess(member) => self.evaluate_member_access(member),
            Expression::Index(idx) => self.evaluate_index(idx),
            _ => Err(self.err_at("Target de asignación no soportado", span)),
        }
    }

    /// Escribe un valor en un target (Identifier | MemberAccess | Index)
    fn write_target(&mut self, target: &Expression, value: Value, span: &Span) -> ClsResult<()> {
        match target {
            Expression::Identifier(name, span) => {
                if self.env.is_const(name) {
                    return Err(self.err_at(format!("No se puede reasignar la constante '{}'", name), span));
                }
                self.env.set(name, value);
                Ok(())
            }
            Expression::MemberAccess(member) => {
                // Caso especial: me.field = value → mutar self_stack
                if let Expression::Identifier(obj_name, _) = &*member.object {
                    if obj_name == "me" {
                        if let Some(Value::Object(obj)) = self.self_stack.last_mut() {
                            obj.fields.insert(member.member.clone(), value);
                            return Ok(());
                        }
                    }
                }
                let mut object = self.evaluate_expression(&member.object)?;
                match object {
                    Value::Struct(mut inst) => {
                        let def = self.structs.get(&inst.def_name).ok_or_else(|| {
                            self.err_at(format!("Struct '{}' no definido", inst.def_name), span)
                        })?;
                        let idx = def.fields.iter().position(|f| f.name == member.member).ok_or_else(|| {
                            self.err_at(format!("Campo '{}' no encontrado", member.member), span)
                        })?;
                        inst.fields[idx] = value;
                        if let Expression::Identifier(name, _) = &*member.object {
                            self.env.set(name, Value::Struct(inst));
                        }
                        Ok(())
                    }
                    Value::Object(mut obj) => {
                        obj.fields.insert(member.member.clone(), value);
                        if let Expression::Identifier(name, _) = &*member.object {
                            self.env.set(name, Value::Object(obj));
                        }
                        Ok(())
                    }
                    Value::Record(ref mut rec) => {
                        rec.insert(member.member.clone(), value);
                        Ok(())
                    }
                    _ => Err(self.err_at("No se puede asignar a este miembro", span)),
                }
            }
            Expression::Index(idx) => {
                let object = self.evaluate_expression(&idx.object)?;
                let index = self.evaluate_expression(&idx.index)?;
                match (object, index) {
                    (Value::Array(mut arr), Value::Int(i)) => {
                        if i < 0 || i >= arr.len() as i64 {
                            Err(self.err_at(format!("Índice fuera de rango: {}", i), span))
                        } else {
                            arr[i as usize] = value;
                            if let Expression::Identifier(name, _) = &*idx.object {
                                self.env.set(name, Value::Array(arr));
                            }
                            Ok(())
                        }
                    }
                    (Value::Record(mut rec), Value::String(key)) => {
                        rec.insert(key, value);
                        if let Expression::Identifier(name, _) = &*idx.object {
                            self.env.set(name, Value::Record(rec));
                        }
                        Ok(())
                    }
                    _ => Err(self.err_at("Indexado no soportado para asignación", span)),
                }
            }
            _ => Err(self.err_at("Target de asignación no soportado", span)),
        }
    }

    /// Aplica un operador compuesto: current OP value
    fn apply_compound(&mut self, current: Value, op: cls_core::frontend::token::Operator, value: Value, span: &Span) -> ClsResult<Value> {
        use cls_core::frontend::token::Operator;
        // Convertir operador compuesto a su base: += → +, -= → -, etc.
        let base_op = match op {
            Operator::PlusEqual => Operator::Plus,
            Operator::MinusEqual => Operator::Minus,
            Operator::StarEqual => Operator::Star,
            Operator::SlashEqual => Operator::Slash,
            _ => op,
        };
        self.evaluate_binary_values(current, base_op, value, span)
    }

    /// Evalúa un binario con valores ya resueltos (para operadores compuestos)
    fn evaluate_binary_values(&mut self, left: Value, op: cls_core::frontend::token::Operator, right: Value, span: &Span) -> ClsResult<Value> {
        use cls_core::frontend::token::Operator;

        // Short-circuit para lógicos
        match op {
            Operator::And => return Ok(Value::Bool(left.is_truthy() && right.is_truthy())),
            Operator::Or => return Ok(Value::Bool(left.is_truthy() || right.is_truthy())),
            _ => {}
        }

        match (&op, &left, &right) {
            (Operator::Plus, Value::Int(a), Value::Int(b)) => Ok(Value::Int(a + b)),
            (Operator::Plus, Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
            (Operator::Plus, Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 + b)),
            (Operator::Plus, Value::Float(a), Value::Int(b)) => Ok(Value::Float(a + *b as f64)),
            (Operator::Plus, Value::String(a), Value::String(b)) => Ok(Value::String(format!("{}{}", a, b))),
            (Operator::Minus, a, b) => num_op(a, b, |x, y| x - y, |x, y| x - y),
            (Operator::Star, a, b) => num_op(a, b, |x, y| x * y, |x, y| x * y),
            (Operator::Slash, Value::Int(x), Value::Int(y)) => {
                if *y == 0 { Err(self.err_at("División por cero", span)) }
                else { Ok(Value::Int(x / y)) }
            }
            (Operator::Slash, a, b) => match (as_f64(a), as_f64(b)) {
                (Some(m), Some(n)) => {
                    if n == 0.0 { Err(self.err_at("División por cero", span)) }
                    else { Ok(Value::Float(m / n)) }
                }
                _ => unsupported_span(&op, a, b, span),
            },
            (Operator::Percent, Value::Int(a), Value::Int(b)) => {
                if *b == 0 { Err(self.err_at("Módulo por cero", span)) }
                else { Ok(Value::Int(a % b)) }
            }
            (Operator::StarStar, a, b) => match (as_f64(a), as_f64(b)) {
                (Some(x), Some(y)) => Ok(Value::Float(x.powf(y))),
                _ => unsupported_span(&op, a, b, span),
            },
            (Operator::Caret, Value::Int(a), Value::Int(b)) => Ok(Value::Int(a ^ b)),
            (Operator::ShiftLeft, Value::Int(a), Value::Int(b)) => Ok(Value::Int(a.wrapping_shl(*b as u32))),
            (Operator::ShiftRight, Value::Int(a), Value::Int(b)) => Ok(Value::Int(a.wrapping_shr(*b as u32))),
            // Comparación de strings (antes de la numérica genérica)
            (Operator::LessThan, Value::String(a), Value::String(b)) => Ok(Value::Bool(a < b)),
            (Operator::LessEqual, Value::String(a), Value::String(b)) => Ok(Value::Bool(a <= b)),
            (Operator::GreaterThan, Value::String(a), Value::String(b)) => Ok(Value::Bool(a > b)),
            (Operator::GreaterEqual, Value::String(a), Value::String(b)) => Ok(Value::Bool(a >= b)),
            // Comparación numérica (Int/Float mixto)
            (Operator::LessThan, a, b) => cmp_num(a, b, |x, y| x < y),
            (Operator::LessEqual, a, b) => cmp_num(a, b, |x, y| x <= y),
            (Operator::GreaterThan, a, b) => cmp_num(a, b, |x, y| x > y),
            (Operator::GreaterEqual, a, b) => cmp_num(a, b, |x, y| x >= y),
            (Operator::StrictEqual, a, b) => Ok(Value::Bool(a == b)),
            (Operator::NotEqual, a, b) => Ok(Value::Bool(a != b)),
            _ => unsupported_span(&op, &left, &right, span),
        }
    }

    fn evaluate_string_interpolation(&mut self, interp: &StringInterpolation) -> ClsResult<Value> {
        let mut result = String::new();
        for part in &interp.parts {
            match part {
                InterpolationPart::Text(s) => result.push_str(s),
                InterpolationPart::Expr(expr) => {
                    let val = self.evaluate_expression(expr)?;
                    result.push_str(&val.to_string());
                }
            }
        }
        Ok(Value::String(result))
    }

    fn evaluate_cmx(&mut self, cmx: &CmxElement) -> ClsResult<Value> {
        // Tag mayúscula → guardar la referencia (función, var, clase, etc) SIN ejecutarla.
        // Tag minúscula → guardar como String. CMX es agnóstico.
        let tag = if cmx.tag.starts_with(|c: char| c.is_uppercase()) {
            match self.env.get(&cmx.tag) {
                Some(val) => val.clone(),          // guardar la referencia
                None => Value::String(cmx.tag.clone()),  // no encontrada → string
            }
        } else {
            Value::String(cmx.tag.clone())
        };

        let mut props = std::collections::HashMap::new();
        for attr in &cmx.attributes {
            let val = self.eval_cmx_attr(attr)?;
            props.insert(attr.name.clone(), val);
        }
        let mut children = Vec::new();
        for child in &cmx.children {
            children.push(match child {
                CmxChild::Text(s) => Value::String(s.clone()),
                CmxChild::Expression(expr) => self.evaluate_expression(expr)?,
                CmxChild::Element(el) => self.evaluate_cmx(el)?,
            });
        }
        Ok(Value::Cmx(Box::new(crate::value::CmxValue { tag, props, children })))
    }

    fn eval_cmx_attr(&mut self, attr: &CmxAttribute) -> ClsResult<Value> {
        match &attr.value {
            Some(CmxAttributeValue::String(s)) => Ok(Value::String(s.clone())),
            Some(CmxAttributeValue::Expression(expr)) => self.evaluate_expression(expr),
            Some(CmxAttributeValue::Shorthand(name)) => {
                Ok(self.env.get(name).cloned().unwrap_or(Value::Null))
            }
            None => Ok(Value::Bool(true)),
        }
    }

    fn evaluate_namespace_access(&mut self, ns: &str, name: &str, span: &Span) -> ClsResult<Value> {
        let ns_val = self.env.get(ns).cloned()
            .ok_or_else(|| self.err_at(format!("Namespace '{}' no definido", ns), span))?;
        match ns_val {
            Value::Record(rec) => rec.get(name).cloned()
                .ok_or_else(|| self.err_at(format!("'{}' no existe en '{}'", name, ns), span)),
            Value::Object(obj) => {
                if let Some(v) = obj.fields.get(name) {
                    Ok(v.clone())
                } else if let Some(m) = obj.methods.get(name) {
                    Ok(Value::Fun(m.clone()))
                } else {
                    Err(self.err_at(format!("'{}' no existe en '{}'", name, ns), span))
                }
            }
            other => Err(self.err_at(format!("'{}' no es un namespace/modulo", ns), span)),
        }
    }
}

// ─── Helpers de operaciones binarias ─────────────────────────────────────

fn as_f64(v: &Value) -> Option<f64> {
    match v {
        Value::Int(i) => Some(*i as f64),
        Value::Float(f) => Some(*f),
        _ => None,
    }
}

/// Operación numérica: Int si ambos Int, Float si hay algún Float
fn num_op<F, G>(a: &Value, b: &Value, int_fn: F, float_fn: G) -> ClsResult<Value>
where F: Fn(i64, i64) -> i64, G: Fn(f64, f64) -> f64
{
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => Ok(Value::Int(int_fn(*x, *y))),
        (x, y) => match (as_f64(x), as_f64(y)) {
            (Some(m), Some(n)) => Ok(Value::Float(float_fn(m, n))),
            _ => Err(ClsError::RuntimeError(format!(
                "Operación no soportada: {} y {}", a.type_name(), b.type_name()
            ))),
        }
    }
}

/// Comparación numérica mixta (Int/Float)
fn cmp_num<F>(a: &Value, b: &Value, cmp: F) -> ClsResult<Value>
where F: Fn(f64, f64) -> bool
{
    match (as_f64(a), as_f64(b)) {
        (Some(x), Some(y)) => Ok(Value::Bool(cmp(x, y))),
        _ => Err(ClsError::RuntimeError(format!(
            "Comparación no soportada: {} y {}", a.type_name(), b.type_name()
        ))),
    }
}

fn unsupported_span(op: &cls_core::frontend::token::Operator, a: &Value, b: &Value, span: &Span) -> ClsResult<Value> {
    Err(ClsError::RuntimeError(format!(
        "Operación no soportada: {} {} {} (línea {}, columna {})",
        a.type_name(), op, b.type_name(),
        span.start_line, span.start_col
    )))
}

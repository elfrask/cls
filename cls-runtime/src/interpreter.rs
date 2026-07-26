use crate::environment::Environment;
use crate::intrinsics::Intrinsics;
use crate::resolver::ModuleResolver;
use crate::value::{FunValue, Value};
use cls_core::error::{ClsError, ClsResult, Diagnostic};
use cls_core::frontend::ast::*;
use std::collections::HashSet;

/// Intérprete tree-walker de CLS
/// Ejecuta el AST directamente, sin compilación intermedia
pub struct Interpreter {
    env: Environment,
    resolver: ModuleResolver,
    diagnostics: Vec<Diagnostic>,
    args: Vec<String>,
    exports: HashSet<String>,  // nombres exportados
    source_file: String,        // archivo actual (para trace de errores)
    import_trace: Vec<ImportFrame>,  // trazado de imports
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
        };
        interpreter.register_intrinsics(intrinsics);
        interpreter
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
            Statement::ConstDecl(var) => self.execute_var_decl(var),
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
                if let Some(expr) = expr {
                    let val = self.evaluate_expression(expr)?;
                    // TODO: return from function (necesita control de flujo)
                    Ok(val)
                } else {
                    Ok(Value::Void)
                }
            }
            Statement::Break => Ok(Value::Void), // TODO: control de flujo
            Statement::Continue => Ok(Value::Void), // TODO: control de flujo
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

    fn execute_function_decl(&mut self, func: &FunctionDecl) -> ClsResult<Value> {
        let fun_val = Value::Fun(FunValue::new_user(
            &func.name,
            func.params.clone(),
            func.body.clone(),
        ));
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
            let condition = self.evaluate_expression(&while_stmt.condition)?;
            if !condition.is_truthy() {
                break;
            }
            self.execute_block(&while_stmt.block)?;
        }
        Ok(Value::Void)
    }

    fn execute_loop(&mut self, block: &Block) -> ClsResult<Value> {
        loop {
            self.execute_block(block)?;
            // TODO: break/continue
            break;
        }
        Ok(Value::Void)
    }

    fn execute_for(&mut self, for_stmt: &ForStatement) -> ClsResult<Value> {
        if let Some(init) = &for_stmt.init {
            self.execute_statement(init)?;
        }
        loop {
            if let Some(cond) = &for_stmt.condition {
                let cond_val = self.evaluate_expression(cond)?;
                if !cond_val.is_truthy() {
                    break;
                }
            }
            self.execute_block(&for_stmt.block)?;
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
            _ => Err(ClsError::RuntimeError(format!(
                "No se puede iterar sobre: {:?}",
                iterable
            ))),
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

    fn execute_class_decl(&mut self, _class: &ClassDecl) -> ClsResult<Value> {
        // TODO: registrar clase en el entorno
        Ok(Value::Void)
    }

    fn execute_structure_decl(&mut self, _structure: &StructureDecl) -> ClsResult<Value> {
        // TODO: registrar estructura
        Ok(Value::Void)
    }

    fn execute_interface_decl(&mut self, _interface: &InterfaceDecl) -> ClsResult<Value> {
        // TODO: registrar interfaz
        Ok(Value::Void)
    }

    fn execute_module_decl(&mut self, _module: &ModuleDecl) -> ClsResult<Value> {
        // TODO: registrar módulo
        Ok(Value::Void)
    }

    fn execute_namespace_decl(&mut self, _ns: &NamespaceDecl) -> ClsResult<Value> {
        // TODO: registrar namespace
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
                        return Err(ClsError::RuntimeError(format!(
                            "'{}' no existe en el módulo '{}'", im.name, fi.path
                        )));
                    }
                }
            }
            _ => return Err(ClsError::RuntimeError(format!(
                "'{}' no es un módulo (no tiene exports)", fi.path
            ))),
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
            _ => return Err(ClsError::RuntimeError(format!(
                "'{}' no es un módulo", include.path
            ))),
        }
        Ok(Value::Void)
    }

    fn execute_block(&mut self, block: &Block) -> ClsResult<Value> {
        self.env.push_scope();
        let mut result = Value::Void;
        for stmt in &block.statements {
            result = self.execute_statement(stmt)?;
        }
        self.env.pop_scope();
        Ok(result)
    }

    fn evaluate_expression(&mut self, expr: &Expression) -> ClsResult<Value> {
        match expr {
            Expression::Literal(lit) => self.evaluate_literal(lit),
            Expression::Identifier(name, _) => {
                self.env.get(name).cloned().ok_or_else(|| {
                    ClsError::RuntimeError(format!("Variable no definida: {}", name))
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
            Expression::NamespaceAccess(ns, name, _) => self.evaluate_namespace_access(ns, name),
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
        let right = self.evaluate_expression(&bin.right)?;

        match (&bin.op, &left, &right) {
            // Aritméticos
            (Operator::Plus, Value::Int(a), Value::Int(b)) => Ok(Value::Int(a + b)),
            (Operator::Plus, Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
            (Operator::Plus, Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 + b)),
            (Operator::Plus, Value::Float(a), Value::Int(b)) => Ok(Value::Float(a + *b as f64)),
            (Operator::Plus, Value::String(a), Value::String(b)) => {
                Ok(Value::String(format!("{}{}", a, b)))
            }
            (Operator::Minus, Value::Int(a), Value::Int(b)) => Ok(Value::Int(a - b)),
            (Operator::Minus, Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
            (Operator::Minus, Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 - b)),
            (Operator::Minus, Value::Float(a), Value::Int(b)) => Ok(Value::Float(a - *b as f64)),
            (Operator::Star, Value::Int(a), Value::Int(b)) => Ok(Value::Int(a * b)),
            (Operator::Star, Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
            (Operator::Star, Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 * b)),
            (Operator::Star, Value::Float(a), Value::Int(b)) => Ok(Value::Float(a * *b as f64)),
            (Operator::Slash, Value::Int(a), Value::Int(b)) => {
                if *b == 0 { Err(ClsError::RuntimeError("División por cero".into())) }
                else { Ok(Value::Int(a / b)) }
            }
            (Operator::Slash, Value::Float(a), Value::Float(b)) => {
                if *b == 0.0 { Err(ClsError::RuntimeError("División por cero".into())) }
                else { Ok(Value::Float(a / b)) }
            }
            (Operator::Slash, Value::Int(a), Value::Float(b)) => {
                if *b == 0.0 { Err(ClsError::RuntimeError("División por cero".into())) }
                else { Ok(Value::Float(*a as f64 / b)) }
            }
            (Operator::Slash, Value::Float(a), Value::Int(b)) => {
                if *b == 0 { Err(ClsError::RuntimeError("División por cero".into())) }
                else { Ok(Value::Float(a / *b as f64)) }
            }
            (Operator::Percent, Value::Int(a), Value::Int(b)) => {
                if *b == 0 { Err(ClsError::RuntimeError("Módulo por cero".into())) }
                else { Ok(Value::Int(a % b)) }
            }

            // Comparación
            (Operator::StrictEqual, a, b) => Ok(Value::Bool(a == b)),
            (Operator::NotEqual, a, b) => Ok(Value::Bool(a != b)),
            (Operator::LessThan, Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a < b)),
            (Operator::LessThan, Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a < b)),
            (Operator::LessEqual, Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a <= b)),
            (Operator::LessEqual, Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a <= b)),
            (Operator::GreaterThan, Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a > b)),
            (Operator::GreaterThan, Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a > b)),
            (Operator::GreaterEqual, Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a >= b)),
            (Operator::GreaterEqual, Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a >= b)),

            // Lógicos
            (Operator::And, a, b) => {
                let a_val = a.is_truthy();
                let b_val = b.is_truthy();
                Ok(Value::Bool(a_val && b_val))
            }
            (Operator::Or, a, b) => {
                let a_val = a.is_truthy();
                let b_val = b.is_truthy();
                Ok(Value::Bool(a_val || b_val))
            }

            _ => Err(ClsError::RuntimeError(format!(
                "Operación no soportada: {:?} {:?} {:?}",
                left, bin.op, right
            ))),
        }
    }

    fn evaluate_unary(&mut self, un: &UnaryExpr) -> ClsResult<Value> {
        let operand = self.evaluate_expression(&un.operand)?;
        match un.op {
            UnaryOp::Negate => match operand {
                Value::Int(v) => Ok(Value::Int(-v)),
                Value::Float(v) => Ok(Value::Float(-v)),
                _ => Err(ClsError::RuntimeError(format!(
                    "No se puede negar: {:?}",
                    operand
                ))),
            },
            UnaryOp::Not => {
                let val = operand.is_truthy();
                Ok(Value::Bool(!val))
            }
            UnaryOp::BitwiseNot => match operand {
                Value::Int(v) => Ok(Value::Int(!v)),
                _ => Err(ClsError::RuntimeError(format!(
                    "No se puede aplicar ~: {:?}",
                    operand
                ))),
            },
            UnaryOp::TypeOf => Ok(Value::String(operand.type_name().to_string())),
        }
    }

    fn evaluate_call(&mut self, call: &CallExpr) -> ClsResult<Value> {
        let callee = self.evaluate_expression(&call.callee)?;
        let mut args = Vec::new();
        for arg in &call.args {
            args.push(self.evaluate_expression(arg)?);
        }

        self.call_function_value(callee, args)
    }

    fn evaluate_member_call(&mut self, _call: &CallExpr) -> ClsResult<Value> {
        // TODO: método de objeto/clase
        Err(ClsError::RuntimeError("Member call no implementado".to_string()))
    }

    /// Ejecuta un valor de función con argumentos ya evaluados
    fn call_function_value(&mut self, callee: Value, args: Vec<Value>) -> ClsResult<Value> {
        match callee {
            Value::Fun(fun) => match &fun.kind {
                crate::value::FunKind::Native { params: _, func } => {
                    func(&args)
                }
                crate::value::FunKind::User { params, body } => {
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
                    self.env.pop_scope();
                    // Si el return fue con valor, propagarlo
                    result
                }
            },
            _ => Err(ClsError::RuntimeError(format!(
                "No se puede llamar: {:?}", callee
            ))),
        }
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
            match self.call_function_value(main_val, vec![args_val]) {
                Ok(Value::Int(code)) => Ok(code as i32),
                Ok(_) => Ok(0),
                Err(e) => {
                    eprintln!("Error en main(): {}", e);
                    Ok(1)
                }
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
                    ClsError::RuntimeError(format!("Miembro no encontrado: {}", member.member))
                })
            }
            _ => Err(ClsError::RuntimeError(format!(
                "No se puede acceder a miembro en: {:?}",
                object
            ))),
        }
    }

    fn evaluate_index(&mut self, idx: &IndexExpr) -> ClsResult<Value> {
        let object = self.evaluate_expression(&idx.object)?;
        let index = self.evaluate_expression(&idx.index)?;
        match (object, index) {
            (Value::Array(arr), Value::Int(i)) => {
                if i < 0 || i >= arr.len() as i64 {
                    Err(ClsError::RuntimeError(format!(
                        "Índice fuera de rango: {}",
                        i
                    )))
                } else {
                    Ok(arr[i as usize].clone())
                }
            }
            (Value::Record(rec), Value::String(key)) => {
                rec.get(&key).cloned().ok_or_else(|| {
                    ClsError::RuntimeError(format!("Key no encontrada: {}", key))
                })
            }
            _ => Err(ClsError::RuntimeError("Indexado no soportado".to_string())),
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
        let block = match &*arrow.body {
            Statement::Expression(expr) => {
                let stmt = Statement::Return(Some(expr.clone()));
                Block {
                    statements: vec![stmt],
                    span: arrow.span.clone(),
                }
            }
            stmt => {
                // Si es otro tipo de statement, lo envolvemos en un bloque
                Block {
                    statements: vec![stmt.clone()],
                    span: arrow.span.clone(),
                }
            }
        };
        Ok(Value::Fun(FunValue::new_user("<anonymous>", arrow.params.clone(), block)))
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
        let value = self.evaluate_expression(&assign.value)?;
        match &*assign.target {
            Expression::Identifier(name, _) => {
                self.env.set(name, value.clone());
                Ok(value)
            }
            _ => Err(ClsError::RuntimeError(
                "Target de asignación no soportado".to_string(),
            )),
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

    fn evaluate_cmx(&mut self, _cmx: &CmxElement) -> ClsResult<Value> {
        // TODO: crear estructura CmxValue
        Ok(Value::Cmx(Box::new(crate::value::CmxValue::new(
            "placeholder".to_string(),
        ))))
    }

    fn evaluate_namespace_access(&mut self, ns: &str, name: &str) -> ClsResult<Value> {
        // TODO: resolver namespace::name
        Err(ClsError::RuntimeError(format!(
            "Namespace access no implementado: {}::{}",
            ns, name
        )))
    }
}

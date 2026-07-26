use crate::environment::Environment;
use crate::value::{FunValue, Value};
use crate::stdlib;
use cls_core::error::{ClsError, ClsResult, Diagnostic};
use cls_core::frontend::ast::*;

/// Intérprete tree-walker de CLS
/// Ejecuta el AST directamente, sin compilación intermedia
pub struct Interpreter {
    env: Environment,
    diagnostics: Vec<Diagnostic>,
    args: Vec<String>,
}

impl Interpreter {
    pub fn new(args: Vec<String>) -> Self {
        let mut interpreter = Self {
            env: Environment::new(),
            diagnostics: Vec::new(),
            args,
        };
        interpreter.register_stdlib();
        interpreter
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Registra funciones de la biblioteca estándar
    fn register_stdlib(&mut self) {
        stdlib::io::register(&mut self.env);
        stdlib::math::register(&mut self.env);
        stdlib::fs::register(&mut self.env);
        // args como variable global
        let args_array = Value::Array(
            self.args.iter().map(|a| Value::String(a.clone())).collect(),
        );
        self.env.define("args", args_array);
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
        Ok(value)
    }

    fn execute_function_decl(&mut self, func: &FunctionDecl) -> ClsResult<Value> {
        let fun_val = Value::Fun(FunValue::new_user(
            &func.name,
            func.params.clone(),
            func.body.clone(),
        ));
        self.env.define(&func.name, fun_val);
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

    fn execute_import(&mut self, _import: &ImportStatement) -> ClsResult<Value> {
        // TODO: cargar módulo externo
        Ok(Value::Void)
    }

    fn execute_from_import(&mut self, _from_import: &FromImportStatement) -> ClsResult<Value> {
        // TODO: cargar símbolos específicos
        Ok(Value::Void)
    }

    fn execute_include(&mut self, _include: &IncludeStatement) -> ClsResult<Value> {
        // TODO: incluir módulo completo
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
            (Operator::Star, Value::Int(a), Value::Int(b)) => Ok(Value::Int(a * b)),
            (Operator::Star, Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
            (Operator::Slash, Value::Int(a), Value::Int(b)) => {
                if *b == 0 {
                    Err(ClsError::RuntimeError("División por cero".to_string()))
                } else {
                    Ok(Value::Int(a / b))
                }
            }
            (Operator::Slash, Value::Float(a), Value::Float(b)) => {
                if *b == 0.0 {
                    Err(ClsError::RuntimeError("División por cero".to_string()))
                } else {
                    Ok(Value::Float(a / b))
                }
            }
            (Operator::Percent, Value::Int(a), Value::Int(b)) => {
                if *b == 0 {
                    Err(ClsError::RuntimeError("Módulo por cero".to_string()))
                } else {
                    Ok(Value::Int(a % b))
                }
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

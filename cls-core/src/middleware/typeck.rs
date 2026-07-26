use crate::error::{ClsError, ClsResult, Diagnostic};
use crate::frontend::ast::*;
use crate::middleware::types::Type;
use crate::config::types::TypesConfig;

/// Type checker configurable de CLS
pub struct TypeChecker {
    config: TypesConfig,
    diagnostics: Vec<Diagnostic>,
    // TODO: symbol table, scopes, etc.
}

impl TypeChecker {
    pub fn new(config: TypesConfig) -> Self {
        Self {
            config,
            diagnostics: Vec::new(),
        }
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Verifica un módulo completo
    pub fn check(&mut self, module: &Module) -> ClsResult<()> {
        if !self.config.check {
            // Modo dinámico: todo es Any, no hay chequeo
            return Ok(());
        }

        for stmt in &module.statements {
            self.check_statement(stmt)?;
        }

        Ok(())
    }

    fn check_statement(&mut self, stmt: &Statement) -> ClsResult<Type> {
        match stmt {
            Statement::VarDecl(var) => self.check_var_decl(var),
            Statement::ConstDecl(var) => self.check_var_decl(var),
            Statement::FunctionDecl(func) => self.check_function_decl(func),
            Statement::Return(expr) => {
                if let Some(expr) = expr {
                    self.check_expression(expr)?;
                }
                Ok(Type::Void)
            }
            Statement::Expression(expr) => self.check_expression(expr),
            _ => Ok(Type::Void), // TODO: implementar otros statements
        }
    }

    fn check_var_decl(&mut self, var: &VarDecl) -> ClsResult<Type> {
        let inferred = if let Some(value) = &var.value {
            self.check_expression(value)?
        } else {
            Type::Any
        };

        if let Some(type_ann) = &var.type_ann {
            let declared = self.resolve_type_annotation(type_ann)?;
            if self.config.strict && !inferred.is_assignable_to(&declared) {
                return Err(ClsError::TypeError(format!(
                    "No se puede asignar {} a {}",
                    inferred, declared
                )));
            }
            return Ok(declared);
        }

        Ok(inferred)
    }

    fn check_function_decl(&mut self, func: &FunctionDecl) -> ClsResult<Type> {
        // TODO: crear scope para parámetros
        // TODO: verificar que el body retorne el tipo correcto
        let return_type = func.return_type.as_ref()
            .map(|t| self.resolve_type_annotation(t))
            .transpose()?
            .unwrap_or(Type::Any);
        Ok(return_type)
    }

    fn check_expression(&mut self, expr: &Expression) -> ClsResult<Type> {
        match expr {
            Expression::Literal(lit) => Ok(match &lit.kind {
                LiteralKind::Int(_) => Type::Int,
                LiteralKind::Float(_) => Type::Float,
                LiteralKind::String(_) => Type::String,
                LiteralKind::Bool(_) => Type::Bool,
                LiteralKind::Char(_) => Type::Char,
                LiteralKind::Null => Type::Null,
                LiteralKind::Unknown => Type::Unknown,
            }),
            Expression::Identifier(name, _) => {
                // TODO: lookup en symbol table
                Ok(Type::Any)
            }
            Expression::Binary(bin) => {
                let left = self.check_expression(&bin.left)?;
                let right = self.check_expression(&bin.right)?;
                self.check_binary_op(&left, &bin.op, &right)
            }
            Expression::Unary(un) => {
                let operand = self.check_expression(&un.operand)?;
                self.check_unary_op(&un.op, &operand)
            }
            Expression::Call(call) => {
                let callee = self.check_expression(&call.callee)?;
                // TODO: verificar que callee sea Fun y que los args coincidan
                Ok(Type::Any)
            }
            _ => Ok(Type::Any), // TODO: implementar otros expressions
        }
    }

    fn check_binary_op(&self, left: &Type, op: &crate::frontend::token::Operator, right: &Type) -> ClsResult<Type> {
        use crate::frontend::token::Operator;
        match op {
            Operator::Plus | Operator::Minus | Operator::Star | Operator::Slash | Operator::Percent => {
                if left.is_assignable_to(&Type::Float) && right.is_assignable_to(&Type::Float) {
                    Ok(Type::Float)
                } else if left.is_assignable_to(&Type::Int) && right.is_assignable_to(&Type::Int) {
                    Ok(Type::Int)
                } else {
                    Err(ClsError::TypeError(format!(
                        "Operador {} no soportado para {} y {}",
                        op, left, right
                    )))
                }
            }
            Operator::StrictEqual | Operator::NotEqual
            | Operator::LessThan | Operator::LessEqual
            | Operator::GreaterThan | Operator::GreaterEqual => {
                if left.is_assignable_to(right) || right.is_assignable_to(left) {
                    Ok(Type::Bool)
                } else {
                    Err(ClsError::TypeError(format!(
                        "No se puede comparar {} con {}",
                        left, right
                    )))
                }
            }
            Operator::And | Operator::Or => {
                if left.is_assignable_to(&Type::Bool) && right.is_assignable_to(&Type::Bool) {
                    Ok(Type::Bool)
                } else {
                    Err(ClsError::TypeError(format!(
                        "Operador lógico {} requiere Bool, encontró {} y {}",
                        op, left, right
                    )))
                }
            }
            _ => Ok(Type::Any), // TODO: otros operadores
        }
    }

    fn check_unary_op(&self, op: &crate::frontend::ast::UnaryOp, operand: &Type) -> ClsResult<Type> {
        match op {
            crate::frontend::ast::UnaryOp::Negate => {
                if operand.is_assignable_to(&Type::Float) {
                    Ok(operand.clone())
                } else {
                    Err(ClsError::TypeError(format!(
                        "No se puede negar tipo {}",
                        operand
                    )))
                }
            }
            crate::frontend::ast::UnaryOp::Not => {
                if operand.is_assignable_to(&Type::Bool) {
                    Ok(Type::Bool)
                } else {
                    Err(ClsError::TypeError(format!(
                        "No se puede aplicar ! a tipo {}",
                        operand
                    )))
                }
            }
            _ => Ok(operand.clone()), // TODO: otros operadores
        }
    }

    fn resolve_type_annotation(&self, ann: &TypeAnnotation) -> ClsResult<Type> {
        match &ann.kind {
            TypeKind::Int => Ok(Type::Int),
            TypeKind::Float => Ok(Type::Float),
            TypeKind::String => Ok(Type::String),
            TypeKind::Bool => Ok(Type::Bool),
            TypeKind::Char => Ok(Type::Char),
            TypeKind::Any => Ok(Type::Any),
            TypeKind::Unknown => Ok(Type::Unknown),
            TypeKind::Null => Ok(Type::Null),
            TypeKind::Void => Ok(Type::Void),
            TypeKind::Empty => Ok(Type::Empty),
            TypeKind::Array(inner) => {
                let inner_type = self.resolve_type_annotation(inner)?;
                Ok(Type::Array(Box::new(inner_type)))
            }
            TypeKind::Record(k, v) => {
                let key_type = self.resolve_type_annotation(k)?;
                let value_type = self.resolve_type_annotation(v)?;
                Ok(Type::Record(Box::new(key_type), Box::new(value_type)))
            }
            TypeKind::Fun(params, ret) => {
                let param_types = params
                    .iter()
                    .map(|p| self.resolve_type_annotation(p))
                    .collect::<ClsResult<Vec<_>>>()?;
                let ret_type = self.resolve_type_annotation(ret)?;
                Ok(Type::Fun(param_types, Box::new(ret_type)))
            }
            TypeKind::I32 => Ok(Type::I32),
            TypeKind::I64 => Ok(Type::I64),
            TypeKind::I16 => Ok(Type::I16),
            TypeKind::I8 => Ok(Type::I8),
            TypeKind::F32 => Ok(Type::F32),
            TypeKind::F64 => Ok(Type::F64),
            TypeKind::Cmx => Ok(Type::Cmx),
            TypeKind::Named(name, params) => {
                let param_types = params
                    .iter()
                    .map(|p| self.resolve_type_annotation(p))
                    .collect::<ClsResult<Vec<_>>>()?;
                Ok(Type::Named(name.clone(), param_types))
            }
        }
    }
}

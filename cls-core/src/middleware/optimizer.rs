use crate::frontend::ast::*;

/// Optimizador de AST de CLS
pub struct Optimizer;

impl Optimizer {
    pub fn new() -> Self {
        Self
    }

    /// Optimiza un módulo completo
    pub fn optimize(&self, module: &mut Module) {
        for stmt in &mut module.statements {
            self.optimize_statement(stmt);
        }
    }

    fn optimize_statement(&self, stmt: &mut Statement) {
        match stmt {
            Statement::Expression(expr) => {
                self.optimize_expression(expr);
            }
            Statement::VarDecl(var) => {
                if let Some(value) = &mut var.value {
                    self.optimize_expression(value);
                }
            }
            Statement::FunctionDecl(func) => {
                self.optimize_block(&mut func.body);
            }
            Statement::If(if_stmt) => {
                self.optimize_expression(&mut if_stmt.condition);
                self.optimize_block(&mut if_stmt.then_block);
                for elif in &mut if_stmt.elif_branches {
                    self.optimize_expression(&mut elif.condition);
                    self.optimize_block(&mut elif.block);
                }
                if let Some(else_block) = &mut if_stmt.else_block {
                    self.optimize_block(else_block);
                }
            }
            Statement::While(while_stmt) => {
                self.optimize_expression(&mut while_stmt.condition);
                self.optimize_block(&mut while_stmt.block);
            }
            Statement::Loop(block) => {
                self.optimize_block(block);
            }
            Statement::For(for_stmt) => {
                if let Some(init) = &mut for_stmt.init {
                    self.optimize_statement(init);
                }
                if let Some(cond) = &mut for_stmt.condition {
                    self.optimize_expression(cond);
                }
                if let Some(update) = &mut for_stmt.update {
                    self.optimize_expression(update);
                }
                self.optimize_block(&mut for_stmt.block);
            }
            Statement::ForEach(for_each) => {
                self.optimize_expression(&mut for_each.iterable);
                self.optimize_block(&mut for_each.block);
            }
            Statement::Switch(switch) => {
                self.optimize_expression(&mut switch.value);
                for case in &mut switch.cases {
                    self.optimize_block(&mut case.block);
                }
                if let Some(default) = &mut switch.default {
                    self.optimize_block(default);
                }
            }
            Statement::Try(try_stmt) => {
                self.optimize_block(&mut try_stmt.try_block);
                for catch in &mut try_stmt.catch_clauses {
                    self.optimize_block(&mut catch.block);
                }
                if let Some(finally) = &mut try_stmt.finally_block {
                    self.optimize_block(finally);
                }
            }
            Statement::With(with_stmt) => {
                self.optimize_expression(&mut with_stmt.value);
                self.optimize_block(&mut with_stmt.block);
            }
            Statement::Return(expr) => {
                if let Some(expr) = expr {
                    self.optimize_expression(expr);
                }
            }
            _ => {}
        }
    }

    fn optimize_block(&self, block: &mut Block) {
        for stmt in &mut block.statements {
            self.optimize_statement(stmt);
        }
    }

    fn optimize_expression(&self, expr: &mut Expression) {
        match expr {
            Expression::Binary(bin) => {
                self.optimize_expression(&mut bin.left);
                self.optimize_expression(&mut bin.right);
                // TODO: constant folding
                // if let (Expression::Literal(a), Expression::Literal(b)) = (&*bin.left, &*bin.right) {
                //     // evaluar constante
                // }
            }
            Expression::Unary(un) => {
                self.optimize_expression(&mut un.operand);
            }
            Expression::Call(call) => {
                self.optimize_expression(&mut call.callee);
                for arg in &mut call.args {
                    self.optimize_expression(arg);
                }
            }
            Expression::MemberAccess(member) => {
                self.optimize_expression(&mut member.object);
            }
            Expression::Index(idx) => {
                self.optimize_expression(&mut idx.object);
                self.optimize_expression(&mut idx.index);
            }
            Expression::Array(arr) => {
                for elem in &mut arr.elements {
                    self.optimize_expression(elem);
                }
            }
            Expression::Record(rec) => {
                for (_, value) in &mut rec.entries {
                    self.optimize_expression(value);
                }
            }
            Expression::ArrowFunction(arrow) => {
                self.optimize_statement(&mut arrow.body);
            }
            Expression::Conditional(cond) => {
                self.optimize_expression(&mut cond.condition);
                self.optimize_expression(&mut cond.then_expr);
                self.optimize_expression(&mut cond.else_expr);
            }
            Expression::Assignment(assign) => {
                self.optimize_expression(&mut assign.target);
                self.optimize_expression(&mut assign.value);
            }
            Expression::Parenthesized(inner, _) => {
                self.optimize_expression(inner);
            }
            Expression::StringInterpolation(interp) => {
                for part in &mut interp.parts {
                    if let InterpolationPart::Expr(expr) = part {
                        self.optimize_expression(expr);
                    }
                }
            }
            _ => {}
        }
    }
}

use crate::frontend::ast::*;

/// Trait para implementar el patrón Visitor sobre el AST
pub trait AstVisitor<T> {
    fn visit_module(&mut self, module: &Module) -> T;
    fn visit_statement(&mut self, stmt: &Statement) -> T;
    fn visit_expression(&mut self, expr: &Expression) -> T;
    fn visit_block(&mut self, block: &Block) -> T;
}

/// Función de utilidad para recorrer el AST
pub fn walk_module<V, T>(visitor: &mut V, module: &Module) -> T
where
    V: AstVisitor<T>,
{
    visitor.visit_module(module)
}

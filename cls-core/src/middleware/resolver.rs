use crate::error::{ClsError, ClsResult, Diagnostic};
use crate::frontend::ast::*;
use std::collections::HashMap;

/// Resolver de nombres y scopes
pub struct NameResolver {
    scopes: Vec<Scope>,
    diagnostics: Vec<Diagnostic>,
}

impl NameResolver {
    pub fn new() -> Self {
        Self {
            scopes: vec![Scope::global()],
            diagnostics: Vec::new(),
        }
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    pub fn resolve(&mut self, module: &Module) -> ClsResult<()> {
        for stmt in &module.statements {
            self.resolve_statement(stmt)?;
        }
        Ok(())
    }

    fn resolve_statement(&mut self, stmt: &Statement) -> ClsResult<()> {
        match stmt {
            Statement::VarDecl(var) => {
                self.define(var.name.clone(), SymbolKind::Variable);
                if let Some(value) = &var.value {
                    self.resolve_expression(value)?;
                }
            }
            Statement::ConstDecl(var) => {
                self.define(var.name.clone(), SymbolKind::Variable);
                if let Some(value) = &var.value {
                    self.resolve_expression(value)?;
                }
            }
            Statement::FunctionDecl(func) => {
                self.define(func.name.clone(), SymbolKind::Function);
                self.push_scope();
                for param in &func.params {
                    self.define(param.name.clone(), SymbolKind::Parameter);
                }
                self.resolve_block(&func.body)?;
                self.pop_scope();
            }
            Statement::ClassDecl(class) => {
                self.define(class.name.clone(), SymbolKind::Class);
                self.push_scope();
                for member in &class.body {
                    match member {
                        ClassMember::Method(func) => {
                            self.define(func.name.clone(), SymbolKind::Function);
                        }
                        ClassMember::Property(var) => {
                            self.define(var.name.clone(), SymbolKind::Variable);
                        }
                        ClassMember::Constructor(func) => {
                            self.define(func.name.clone(), SymbolKind::Constructor);
                        }
                    }
                }
                self.pop_scope();
            }
            Statement::ModuleDecl(module) => {
                self.define(module.name.clone(), SymbolKind::Module);
                self.push_scope();
                for stmt in &module.body {
                    self.resolve_statement(stmt)?;
                }
                self.pop_scope();
            }
            Statement::NamespaceDecl(ns) => {
                self.define(ns.name.clone(), SymbolKind::Namespace);
                self.push_scope();
                for stmt in &ns.body {
                    self.resolve_statement(stmt)?;
                }
                self.pop_scope();
            }
            Statement::Expression(expr) => {
                self.resolve_expression(expr)?;
            }
            Statement::If(if_stmt) => {
                self.resolve_expression(&if_stmt.condition)?;
                self.resolve_block(&if_stmt.then_block)?;
                for elif in &if_stmt.elif_branches {
                    self.resolve_expression(&elif.condition)?;
                    self.resolve_block(&elif.block)?;
                }
                if let Some(else_block) = &if_stmt.else_block {
                    self.resolve_block(else_block)?;
                }
            }
            Statement::While(while_stmt) => {
                self.resolve_expression(&while_stmt.condition)?;
                self.resolve_block(&while_stmt.block)?;
            }
            Statement::Loop(block) => {
                self.resolve_block(block)?;
            }
            Statement::For(for_stmt) => {
                if let Some(init) = &for_stmt.init {
                    self.resolve_statement(init)?;
                }
                if let Some(cond) = &for_stmt.condition {
                    self.resolve_expression(cond)?;
                }
                if let Some(update) = &for_stmt.update {
                    self.resolve_expression(update)?;
                }
                self.resolve_block(&for_stmt.block)?;
            }
            Statement::ForEach(for_each) => {
                self.resolve_expression(&for_each.iterable)?;
                self.push_scope();
                self.define(for_each.item_name.clone(), SymbolKind::Variable);
                if let Some(idx) = &for_each.index_name {
                    self.define(idx.clone(), SymbolKind::Variable);
                }
                self.resolve_block(&for_each.block)?;
                self.pop_scope();
            }
            Statement::Switch(switch) => {
                self.resolve_expression(&switch.value)?;
                for case in &switch.cases {
                    self.resolve_block(&case.block)?;
                }
                if let Some(default) = &switch.default {
                    self.resolve_block(default)?;
                }
            }
            Statement::Try(try_stmt) => {
                self.resolve_block(&try_stmt.try_block)?;
                for catch in &try_stmt.catch_clauses {
                    self.push_scope();
                    self.define(catch.param_name.clone(), SymbolKind::Variable);
                    self.resolve_block(&catch.block)?;
                    self.pop_scope();
                }
                if let Some(finally) = &try_stmt.finally_block {
                    self.resolve_block(finally)?;
                }
            }
            Statement::With(with_stmt) => {
                self.resolve_expression(&with_stmt.value)?;
                self.push_scope();
                self.define(with_stmt.name.clone(), SymbolKind::Variable);
                self.resolve_block(&with_stmt.block)?;
                self.pop_scope();
            }
            Statement::Return(expr) => {
                if let Some(expr) = expr {
                    self.resolve_expression(expr)?;
                }
            }
            Statement::Break | Statement::Continue => {}
            Statement::Import(import) => {
                // TODO: cargar módulo y registrar símbolos importados
                if let Some(alias) = &import.alias {
                    self.define(alias.clone(), SymbolKind::Module);
                }
            }
            Statement::FromImport(from_import) => {
                for name in &from_import.names {
                    let alias = name.alias.as_ref().unwrap_or(&name.name);
                    self.define(alias.clone(), SymbolKind::Variable);
                }
            }
            Statement::Include(include) => {
                // TODO: procesar include
            }
            Statement::StructureDecl(structure) => {
                self.define(structure.name.clone(), SymbolKind::Structure);
            }
            Statement::InterfaceDecl(interface) => {
                self.define(interface.name.clone(), SymbolKind::Interface);
            }
            Statement::Config(_) | Statement::Meta(_) => {}
            Statement::Cmx(_) => {}
        }
        Ok(())
    }

    fn resolve_block(&mut self, block: &Block) -> ClsResult<()> {
        self.push_scope();
        for stmt in &block.statements {
            self.resolve_statement(stmt)?;
        }
        self.pop_scope();
        Ok(())
    }

    fn resolve_expression(&mut self, expr: &Expression) -> ClsResult<()> {
        match expr {
            Expression::Identifier(name, _) => {
                if !self.lookup(name) {
                    return Err(ClsError::SyntaxError(format!(
                        "Variable no definida: {}",
                        name
                    )));
                }
            }
            Expression::Literal(_) => {}
            Expression::Binary(bin) => {
                self.resolve_expression(&bin.left)?;
                self.resolve_expression(&bin.right)?;
            }
            Expression::Unary(un) => {
                self.resolve_expression(&un.operand)?;
            }
            Expression::Call(call) => {
                self.resolve_expression(&call.callee)?;
                for arg in &call.args {
                    self.resolve_expression(arg)?;
                }
            }
            Expression::MemberAccess(member) => {
                self.resolve_expression(&member.object)?;
            }
            Expression::Index(idx) => {
                self.resolve_expression(&idx.object)?;
                self.resolve_expression(&idx.index)?;
            }
            Expression::Array(arr) => {
                for elem in &arr.elements {
                    self.resolve_expression(elem)?;
                }
            }
            Expression::Record(rec) => {
                for (_, value) in &rec.entries {
                    self.resolve_expression(value)?;
                }
            }
            Expression::ArrowFunction(arrow) => {
                self.push_scope();
                for param in &arrow.params {
                    self.define(param.name.clone(), SymbolKind::Parameter);
                }
                self.resolve_statement(&arrow.body)?;
                self.pop_scope();
            }
            Expression::Conditional(cond) => {
                self.resolve_expression(&cond.condition)?;
                self.resolve_expression(&cond.then_expr)?;
                self.resolve_expression(&cond.else_expr)?;
            }
            Expression::Assignment(assign) => {
                self.resolve_expression(&assign.target)?;
                self.resolve_expression(&assign.value)?;
            }
            Expression::Parenthesized(inner, _) => {
                self.resolve_expression(inner)?;
            }
            Expression::StringInterpolation(interp) => {
                for part in &interp.parts {
                    if let InterpolationPart::Expr(expr) = part {
                        self.resolve_expression(expr)?;
                    }
                }
            }
            Expression::Cmx(_) => {}
            Expression::NamespaceAccess(ns, name, _) => {
                if !self.lookup(ns) {
                    return Err(ClsError::SyntaxError(format!(
                        "Namespace no definido: {}",
                        ns
                    )));
                }
            }
        }
        Ok(())
    }

    fn define(&mut self, name: String, kind: SymbolKind) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.define(name, kind);
        }
    }

    fn lookup(&self, name: &str) -> bool {
        self.scopes.iter().rev().any(|s| s.contains(name))
    }

    fn push_scope(&mut self) {
        self.scopes.push(Scope::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }
}

struct Scope {
    symbols: HashMap<String, SymbolKind>,
}

impl Scope {
    fn new() -> Self {
        Self {
            symbols: HashMap::new(),
        }
    }

    fn global() -> Self {
        let mut scope = Self::new();
        // TODO: registrar funciones y tipos intrínsecos
        scope
    }

    fn define(&mut self, name: String, kind: SymbolKind) {
        self.symbols.insert(name, kind);
    }

    fn contains(&self, name: &str) -> bool {
        self.symbols.contains_key(name)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum SymbolKind {
    Variable,
    Function,
    Parameter,
    Class,
    Structure,
    Interface,
    Module,
    Namespace,
    Constructor,
}

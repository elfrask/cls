use crate::error::{ClsError, ClsResult, Diagnostic};
use crate::frontend::ast::*;
use std::collections::HashMap;

/// Resolver de nombres y scopes
pub struct NameResolver {
    scopes: Vec<Scope>,
    diagnostics: Vec<Diagnostic>,
    /// Target del entorno actual. Usado para evaluar las directivas `when`
    /// en compile-time: solo la rama que matchea este target se procesa
    /// (mismo comportamiento que el emisor WASM en `effective_statements`).
    target: Target,
}

impl NameResolver {
    pub fn new() -> Self {
        Self {
            scopes: vec![Scope::global()],
            diagnostics: Vec::new(),
            target: Target::host(),
        }
    }

    /// Construye un resolver para un target especifico (usado por
    /// `clx check --target <tripla>` para simular el entorno).
    pub fn with_target(target: Target) -> Self {
        Self {
            scopes: vec![Scope::global()],
            diagnostics: Vec::new(),
            target,
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
            Statement::TypeAlias(t) => {
                self.define(t.name.clone(), SymbolKind::Type);
            }
            Statement::EnumDecl(e) => {
                self.define(e.name.clone(), SymbolKind::Enum);
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
            Statement::Break(_) | Statement::Continue(_) => {}
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
            Statement::Include(_include) => {
                // TODO: procesar include
            }
            Statement::When(w) => {
                // `when` es compile-time: solo se procesa la rama que matchea
                // el target del host. Antes del fix (dev-2) se iteraban TODAS
                // las ramas y se poppeaban los scopes, lo que hacia que las
                // declaraciones dentro de `when` (extension/structure/enum)
                // se perdieran al salir del scope. Esto invalidaba el patron
                // canonico `when (os: windows) { extension "ws2_32.dll" as C
                // { ... } }`. Ver extension-when.md y decision 002.
                for branch in &w.branches {
                    if self.target.matches(&branch.cond) {
                        self.resolve_block(&branch.block)?;
                        break;
                    }
                }
            }
            Statement::StructureDecl(structure) => {
                self.define(structure.name.clone(), SymbolKind::Structure);
            }
            Statement::Extension(ext) => {
                for decl in &ext.declarations {
                    match decl {
                        NativeDecl::Function(f) => self.define(f.name.clone(), SymbolKind::Function),
                        NativeDecl::Structure(s) => self.define(s.name.clone(), SymbolKind::Structure),
                        NativeDecl::Var(v) => self.define(v.name.clone(), SymbolKind::Variable),
                    }
                }
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
            Expression::Identifier(name, span) => {
                if !self.lookup(name) {
                    return Err(ClsError::SyntaxError(format!(
                        "Variable no definida: {} (línea {}, columna {})",
                        name, span.start_line, span.start_col
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
            Expression::Tuple(tup) => {
                for elem in &tup.elements {
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
                self.resolve_block(&arrow.body)?;
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
            Expression::NamespaceAccess(ns, _name, _) => {
                if !self.lookup(ns) {
                    return Err(ClsError::SyntaxError(format!(
                        "Namespace no definido: {}",
                        ns
                    )));
                }
            }
            Expression::Await(expr, _) => {
                self.resolve_expression(expr)?;
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
        for name in &["print", "input", "toString", "int", "float", "str", "bool",
                       "len", "type", "now", "exit", "sleep", "throw", "args"] {
            scope.symbols.insert(name.to_string(), SymbolKind::Function);
        }
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
    Type,
    Enum,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::{Lexer, Parser};

    fn resolve(src: &str) -> Vec<Diagnostic> {
        let toks = Lexer::new(src).tokenize().expect("tokenize");
        let module = Parser::new(toks).parse().expect("parse");
        let mut r = NameResolver::new();
        r.resolve(&module).expect("resolve");
        r.diagnostics().to_vec()
    }

    /// Cuenta errores: el resolver devuelve Err (SyntaxError) o registra diagnostics.
    fn error_count(src: &str) -> usize {
        let toks = Lexer::new(src).tokenize().expect("tokenize");
        let module = Parser::new(toks).parse().expect("parse");
        let mut r = NameResolver::new();
        match r.resolve(&module) {
            Ok(()) => r.diagnostics().len(),
            Err(_) => 1,
        }
    }

    #[test]
    fn defined_variables_resolve() {
        let d = resolve("function f() { var x = 1; return x; };");
        assert!(d.is_empty(), "x definida: {:?}", d);
    }

    #[test]
    fn undefined_variable_errors() {
        assert_eq!(error_count("function f() { return y; };"), 1);
    }

    #[test]
    fn params_are_in_scope() {
        let d = resolve("function f(a: int, b: int) -> int { return a + b; };");
        assert!(d.is_empty(), "params en scope: {:?}", d);
    }

    #[test]
    fn class_and_enum_definitions() {
        let d = resolve("class A { var f: int; }; enum E { X, Y, };");
        assert!(d.is_empty(), "clase y enum definidos: {:?}", d);
    }
}

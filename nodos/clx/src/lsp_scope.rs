//! Símbolos visibles en una posición del archivo actual.
//!
//! El completion de un LSP normal no alcanza con listar las declaraciones de
//! nivel superior: dentro de una función también tienen que aparecer sus
//! parámetros y las variables locales de los bloques que contienen al cursor.
//! Este módulo recorre el AST y devuelve primero lo local y después el scope
//! superior.

use std::collections::HashSet;

use cls_core::error::Span;
use cls_core::frontend::ast::*;
use cls_core::frontend::{Lexer, Parser};
use tower_lsp::lsp_types::{Position, SymbolKind};

use crate::lsp_index::{parse_source, symbols_from_statements, type_ann_to_string, LspSymbol};
use crate::lsp_text::{line_text, utf16_col_to_char_index};

/// Símbolos visibles en `position`, priorizando el scope local.
pub fn local_symbols_at(source: &str, position: Position) -> Vec<LspSymbol> {
    let mut collector = Collector::new(source, position);
    if let Some(module) = parse_module(source) {
        collector.walk_top(&module.statements);
    }

    // Los locales primero; el scope de nivel superior queda como fallback.
    let mut out = collector.locals;
    out.extend(parse_source(source));
    out
}

fn parse_module(source: &str) -> Option<Module> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().ok()?;
    let mut parser = Parser::new(tokens);
    parser.parse().ok()
}

fn class_member_span(member: &ClassMember) -> Option<Span> {
    match member {
        ClassMember::Method(function) | ClassMember::Constructor(function) => Some(function.span),
        ClassMember::Property(property) => Some(property.span),
    }
}

fn statement_start_line(statement: &Statement) -> Option<u32> {
    statement_span(statement).map(|span| span.start_line)
}

fn statement_end_line(statement: &Statement) -> Option<u32> {
    statement_span(statement).map(|span| span.end_line)
}

fn statement_span(statement: &Statement) -> Option<Span> {
    Some(match statement {
        Statement::VarDecl(value) | Statement::ConstDecl(value) => value.span,
        Statement::FunctionDecl(value) => value.span,
        Statement::If(value) => value.span,
        Statement::While(value) => value.span,
        Statement::Loop(block) => block.span,
        Statement::For(value) => value.span,
        Statement::ForEach(value) => value.span,
        Statement::Switch(value) => value.span,
        Statement::Try(value) => value.span,
        Statement::With(value) => value.span,
        Statement::ClassDecl(value) => value.span,
        Statement::StructureDecl(value) => value.span,
        Statement::InterfaceDecl(value) => value.span,
        Statement::ModuleDecl(value) => value.span,
        Statement::NamespaceDecl(value) => value.span,
        Statement::TypeAlias(value) => value.span,
        Statement::EnumDecl(value) => value.span,
        Statement::DestructureLet(value) => value.span,
        Statement::DestructureClass(value) => value.span,
        Statement::Import(value) => value.span,
        Statement::FromImport(value) => value.span,
        Statement::ExportList(value) => value.span,
        Statement::ArrowReturn(value) => value.span,
        Statement::Break(span) | Statement::Continue(span) => *span,
        Statement::When(value) => value.span,
        _ => return None,
    })
}

struct Collector<'a> {
    source: &'a str,
    line: u32,
    column: usize,
    locals: Vec<LspSymbol>,
    seen: HashSet<String>,
}

impl<'a> Collector<'a> {
    fn new(source: &'a str, position: Position) -> Self {
        let column = line_text(source, position.line)
            .map(|line| utf16_col_to_char_index(line, position.character))
            .unwrap_or(0);
        Self {
            source,
            line: position.line,
            column,
            locals: Vec::new(),
            seen: HashSet::new(),
        }
    }

    fn walk_top(&mut self, statements: &[Statement]) {
        for statement in statements {
            self.visit_statement(statement);
        }
    }

    fn walk_statements(&mut self, statements: &[Statement], span: Span) {
        let start_line = statements
            .first()
            .and_then(statement_start_line)
            .unwrap_or(span.start_line);
        let end_line = statements
            .last()
            .and_then(statement_end_line)
            .unwrap_or(span.end_line)
            .max(span.end_line);
        if start_line == 0 || end_line == 0 {
            return;
        }
        if self.line < start_line.saturating_sub(1) || self.line > end_line.saturating_sub(1) {
            return;
        }
        for statement in statements {
            self.visit_statement(statement);
        }
    }

    fn walk_block(&mut self, block: &Block) {
        let start_line = block
            .statements
            .first()
            .and_then(statement_start_line)
            .unwrap_or(block.span.start_line);
        self.walk_block_from(block, start_line);
    }

    fn walk_block_from(&mut self, block: &Block, start_line: u32) {
        let end_line = block
            .statements
            .last()
            .and_then(statement_end_line)
            .unwrap_or(block.span.end_line)
            .max(block.span.end_line);
        if start_line == 0 || end_line == 0 {
            return;
        }
        if self.line < start_line.saturating_sub(1) || self.line > end_line.saturating_sub(1) {
            return;
        }
        for statement in &block.statements {
            self.visit_statement(statement);
        }
    }

    /// `Block.span` del parser apunta al cierre `}`; para saber si el cursor
    /// está dentro del bloque usamos también las líneas de sus sentencias.
    fn block_contains(&self, block: &Block) -> bool {
        let start_line = block
            .statements
            .first()
            .and_then(statement_start_line)
            .unwrap_or(block.span.start_line);
        let end_line = block
            .statements
            .last()
            .and_then(statement_end_line)
            .unwrap_or(block.span.end_line)
            .max(block.span.end_line);

        if start_line == 0 || end_line == 0 {
            return self.contains(&block.span);
        }
        self.line >= start_line.saturating_sub(1) && self.line <= end_line.saturating_sub(1)
    }

    /// `FunctionDecl.span` y `Block.span` del parser son reducidos; el rango
    /// real va desde la declaración hasta el cierre del cuerpo.
    fn function_contains(&self, function: &FunctionDecl) -> bool {
        let start_line = function.span.start_line;
        let end_line = function
            .body
            .statements
            .last()
            .and_then(statement_end_line)
            .unwrap_or(function.body.span.end_line)
            .max(function.body.span.end_line)
            .max(function.span.end_line);
        if start_line == 0 || end_line == 0 {
            return false;
        }
        self.line >= start_line.saturating_sub(1) && self.line <= end_line.saturating_sub(1)
    }

    fn class_contains(&self, class: &ClassDecl) -> bool {
        // El span de la clase apunta al cierre; el inicio real es el primer
        // miembro (o el encabezado si la clase está vacía).
        let first_member = class
            .body
            .iter()
            .filter_map(class_member_span)
            .map(|span| span.start_line)
            .min()
            .unwrap_or(class.span.start_line);
        let start_line = if class.span.start_line == 0 {
            first_member
        } else {
            class.span.start_line.min(first_member)
        };
        let end_line = class
            .body
            .iter()
            .filter_map(class_member_span)
            .map(|span| span.end_line)
            .max()
            .unwrap_or(class.span.end_line)
            .max(class.span.end_line);
        if start_line == 0 || end_line == 0 {
            return false;
        }
        self.line >= start_line.saturating_sub(1) && self.line <= end_line.saturating_sub(1)
    }

    fn visit_statement(&mut self, statement: &Statement) {
        match statement {
            Statement::VarDecl(_) | Statement::ConstDecl(_) => {
                self.push_declaration(statement, true);
            }
            Statement::FunctionDecl(function) => {
                self.push_declaration(statement, false);
                self.walk_function(function);
            }
            Statement::ClassDecl(class) => {
                self.push_declaration(statement, false);
                self.walk_class(class);
            }
            Statement::StructureDecl(_)
            | Statement::InterfaceDecl(_)
            | Statement::TypeAlias(_)
            | Statement::EnumDecl(_) => {
                self.push_declaration(statement, false);
            }
            Statement::ModuleDecl(module) => {
                self.push_declaration(statement, false);
                self.walk_statements(&module.body, module.span);
            }
            Statement::NamespaceDecl(namespace) => {
                self.push_declaration(statement, false);
                self.walk_statements(&namespace.body, namespace.span);
            }
            Statement::DestructureLet(destructure) => {
                for bind in &destructure.binds {
                    if bind != "_" {
                        self.push_binding(
                            bind,
                            None,
                            format!("{}::{}", destructure.enum_name, destructure.variant),
                            destructure.span,
                            true,
                        );
                    }
                }
            }
            Statement::DestructureClass(destructure) => {
                for bind in &destructure.binds {
                    if bind != "_" {
                        self.push_binding(
                            bind,
                            Some(destructure.class_name.clone()),
                            format!("{} destructure", destructure.class_name),
                            destructure.span,
                            true,
                        );
                    }
                }
            }
            Statement::If(if_statement) => {
                self.walk_block(&if_statement.then_block);
                for branch in &if_statement.elif_branches {
                    self.walk_block(&branch.block);
                }
                if let Some(else_block) = &if_statement.else_block {
                    self.walk_block(else_block);
                }
            }
            Statement::While(while_statement) => self.walk_block(&while_statement.block),
            Statement::Loop(block) => self.walk_block(block),
            Statement::For(for_statement) => {
                if self.block_contains(&for_statement.block) {
                    if let Some(init) = &for_statement.init {
                        self.visit_statement(init);
                    }
                    self.walk_block(&for_statement.block);
                }
            }
            Statement::ForEach(for_each) => {
                if self.block_contains(&for_each.block) {
                    self.push_binding(
                        &for_each.item_name,
                        None,
                        "for-each item".to_string(),
                        for_each.span,
                        true,
                    );
                    if let Some(index) = &for_each.index_name {
                        self.push_binding(
                            index,
                            None,
                            "for-each index".to_string(),
                            for_each.span,
                            true,
                        );
                    }
                    self.walk_block(&for_each.block);
                }
            }
            Statement::Switch(switch) => {
                for case in &switch.cases {
                    self.walk_block(&case.block);
                }
                if let Some(default) = &switch.default {
                    self.walk_block(default);
                }
            }
            Statement::Try(try_statement) => {
                self.walk_block(&try_statement.try_block);
                for catch in &try_statement.catch_clauses {
                    if self.block_contains(&catch.block) {
                        self.push_binding(
                            &catch.param_name,
                            catch
                                .param_type
                                .as_ref()
                                .map(|a| type_ann_to_string(Some(a))),
                            "catch".to_string(),
                            catch.span,
                            true,
                        );
                        self.walk_block(&catch.block);
                    }
                }
                if let Some(finally) = &try_statement.finally_block {
                    self.walk_block(finally);
                }
            }
            Statement::With(with_statement) => {
                if self.block_contains(&with_statement.block) {
                    self.push_binding(
                        &with_statement.name,
                        None,
                        "with".to_string(),
                        with_statement.span,
                        true,
                    );
                    self.walk_block(&with_statement.block);
                }
            }
            Statement::When(when) => {
                for branch in &when.branches {
                    self.walk_block(&branch.block);
                }
            }
            Statement::Return(Some(expression)) | Statement::Expression(expression) => {
                self.walk_expression(expression);
            }
            Statement::ArrowReturn(arrow) => self.walk_expression(&arrow.value),
            _ => {}
        }
    }

    fn walk_function(&mut self, function: &FunctionDecl) {
        if !self.function_contains(function) {
            return;
        }
        for parameter in &function.params {
            self.push_parameter(parameter);
        }
        // El cuerpo puede empezar con un `if` cuyo span apunta al cierre;
        // usamos el inicio de la declaración de la función como cota.
        self.walk_block_from(&function.body, function.span.start_line);
    }

    fn walk_class(&mut self, class: &ClassDecl) {
        if !self.class_contains(class) {
            return;
        }
        for member in &class.body {
            match member {
                ClassMember::Method(function) | ClassMember::Constructor(function) => {
                    if !self.function_contains(function) {
                        continue;
                    }
                    self.push_binding(
                        "me",
                        Some(class.name.clone()),
                        format!("me: {}", class.name),
                        class.span,
                        false,
                    );
                    if let Some(parent) = &class.extends {
                        self.push_binding(
                            "super",
                            Some(parent.clone()),
                            format!("super: {}", parent),
                            class.span,
                            false,
                        );
                    }
                    for parameter in &function.params {
                        self.push_parameter(parameter);
                    }
                    self.walk_block_from(&function.body, function.span.start_line);
                }
                ClassMember::Property(_) => {}
            }
        }
    }

    fn walk_expression(&mut self, expression: &Expression) {
        match expression {
            Expression::ArrowFunction(arrow) => {
                if !self.block_contains(&arrow.body) {
                    return;
                }
                for parameter in &arrow.params {
                    self.push_parameter(parameter);
                }
                self.walk_block(&arrow.body);
            }
            Expression::Call(call) => {
                self.walk_expression(&call.callee);
                for argument in &call.args {
                    self.walk_expression(argument);
                }
            }
            Expression::Binary(binary) => {
                self.walk_expression(&binary.left);
                self.walk_expression(&binary.right);
            }
            Expression::Unary(unary) => self.walk_expression(&unary.operand),
            Expression::Assignment(assignment) => {
                self.walk_expression(&assignment.target);
                self.walk_expression(&assignment.value);
            }
            Expression::MemberAccess(access) => self.walk_expression(&access.object),
            Expression::Index(index) => {
                self.walk_expression(&index.object);
                self.walk_expression(&index.index);
            }
            Expression::Conditional(conditional) => {
                self.walk_expression(&conditional.condition);
                self.walk_expression(&conditional.then_expr);
                self.walk_expression(&conditional.else_expr);
            }
            Expression::Ternary(ternary) => {
                self.walk_expression(&ternary.condition);
                self.walk_expression(&ternary.then_expr);
                self.walk_expression(&ternary.else_expr);
            }
            Expression::Array(array) => {
                for element in &array.elements {
                    self.walk_expression(element);
                }
            }
            Expression::Tuple(tuple) => {
                for element in &tuple.elements {
                    self.walk_expression(element);
                }
            }
            Expression::Record(record) => {
                for (_, value) in &record.entries {
                    self.walk_expression(value);
                }
                for spread in &record.spreads {
                    self.walk_expression(spread);
                }
            }
            Expression::Parenthesized(inner, _) => self.walk_expression(inner),
            Expression::Await(inner, _) => self.walk_expression(inner),
            Expression::Spread(inner, _) => self.walk_expression(inner),
            Expression::IfExpr(if_expr) => {
                self.walk_block(&if_expr.then_block);
                self.walk_block(&if_expr.else_block);
            }
            _ => {}
        }
    }

    fn push_declaration(&mut self, statement: &Statement, before_only: bool) {
        for symbol in symbols_from_statements(std::slice::from_ref(statement), self.source) {
            self.push(symbol, before_only);
        }
    }

    fn push_parameter(&mut self, parameter: &Parameter) {
        let ty = parameter
            .type_ann
            .as_ref()
            .map(|annotation| type_ann_to_string(Some(annotation)));
        let detail = match &ty {
            Some(ty) => format!("{}: {}", parameter.name, ty),
            None => parameter.name.clone(),
        };
        self.push(
            LspSymbol {
                name: parameter.name.clone(),
                kind: SymbolKind::VARIABLE,
                detail,
                doc: String::new(),
                span: parameter.span,
                ty,
                children: Vec::new(),
            },
            false,
        );
    }

    fn push_binding(
        &mut self,
        name: &str,
        ty: Option<String>,
        detail: String,
        span: Span,
        before_only: bool,
    ) {
        self.push(
            LspSymbol {
                name: name.to_string(),
                kind: SymbolKind::VARIABLE,
                detail,
                doc: String::new(),
                span,
                ty,
                children: Vec::new(),
            },
            before_only,
        );
    }

    fn push(&mut self, symbol: LspSymbol, before_only: bool) {
        if before_only && !self.starts_before(&symbol.span) {
            return;
        }
        if self.seen.insert(symbol.name.clone()) {
            self.locals.push(symbol);
        }
    }

    fn starts_before(&self, span: &Span) -> bool {
        let start_line = span.start_line.saturating_sub(1);
        if start_line < self.line {
            return true;
        }
        start_line == self.line && (span.start_col.saturating_sub(1) as usize) <= self.column
    }

    fn contains(&self, span: &Span) -> bool {
        if span.start_line == 0 || span.end_line == 0 {
            return false;
        }
        let start_line = span.start_line.saturating_sub(1);
        let end_line = span.end_line.saturating_sub(1);
        if self.line < start_line || self.line > end_line {
            return false;
        }
        if self.line == start_line && self.column < span.start_col.saturating_sub(1) as usize {
            return false;
        }
        if self.line == end_line && self.column > span.end_col as usize {
            return false;
        }
        true
    }
}

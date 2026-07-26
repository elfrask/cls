use crate::error::{ClsError, ClsResult, Diagnostic};
use crate::error::diagnostic::Span;
use crate::frontend::ast::*;
use crate::frontend::token::{Keyword, Operator, Symbol, Token};
use std::iter::Peekable;
use std::vec::IntoIter;

/// Parser recursive descent de CLS
/// Convierte tokens en un AST
pub struct Parser {
    tokens: Peekable<IntoIter<Token>>,
    current_token: Token,
    diagnostics: Vec<Diagnostic>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        let mut iter = tokens.into_iter().peekable();
        let first = iter.next().unwrap_or(Token::EOF);
        Self {
            tokens: iter,
            current_token: first,
            diagnostics: Vec::new(),
        }
    }

    /// Obtiene los diagnósticos (errores/warnings)
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Parsea todo el módulo
    pub fn parse(&mut self) -> ClsResult<Module> {
        let mut statements = Vec::new();

        while !self.is_eof() {
            match self.parse_statement() {
                Ok(stmt) => statements.push(stmt),
                Err(e) => {
                    // Intentar recuperarse del error
                    self.recover();
                    // Por ahora retornamos el error
                    return Err(e);
                }
            }
        }

        Ok(Module {
            statements,
            span: Span::new(1, 1, 1, 1),
        })
    }

    fn parse_statement(&mut self) -> ClsResult<Statement> {
        // Saltar newlines vacíos
        while matches!(self.current_token, Token::Newline) {
            self.advance();
        }

        if self.is_eof() {
            return Err(ClsError::SyntaxError("EOF inesperado".to_string()));
        }

        // #config directive
        if self.check_directive() {
            return self.parse_config_directive();
        }

        // Comentario de línea (ya manejado por lexer, pero por si acaso)
        match &self.current_token {
            // Configuración inline
            Token::Keyword(Keyword::Config) => self.parse_config(),

            // Declaraciones de variables
            Token::Keyword(Keyword::Var) => self.parse_var_decl(),
            Token::Keyword(Keyword::Const) => self.parse_const_decl(),

            // Funciones
            Token::Keyword(Keyword::Function) => self.parse_function_decl(),
            Token::Keyword(Keyword::Void) => self.parse_void_function(),
            Token::Keyword(Keyword::Method) => self.parse_method_decl(),

            // Control de flujo
            Token::Keyword(Keyword::If) => self.parse_if_statement(),
            Token::Keyword(Keyword::While) => self.parse_while_statement(),
            Token::Keyword(Keyword::Loop) => self.parse_loop_statement(),
            Token::Keyword(Keyword::For) => self.parse_for_statement(),
            Token::Keyword(Keyword::Switch) => self.parse_switch_statement(),
            Token::Keyword(Keyword::Try) => self.parse_try_statement(),
            Token::Keyword(Keyword::With) => self.parse_with_statement(),
            Token::Keyword(Keyword::Return) => self.parse_return_statement(),
            Token::Keyword(Keyword::Break) => self.parse_break(),
            Token::Keyword(Keyword::Continue) => self.parse_continue(),

            // Clases y estructuras
            Token::Keyword(Keyword::Class) => self.parse_class_decl(),
            Token::Keyword(Keyword::Structure) => self.parse_structure_decl(),
            Token::Keyword(Keyword::Interface) => self.parse_interface_decl(),
            Token::Keyword(Keyword::Module) => self.parse_module_decl(),
            Token::Keyword(Keyword::Namespace) => self.parse_namespace_decl(),

            // Imports
            Token::Keyword(Keyword::Import) => self.parse_import(),
            Token::Keyword(Keyword::From) => self.parse_from_import(),
            Token::Keyword(Keyword::Include) => self.parse_include(),

            // Modifiers
            Token::Keyword(Keyword::Public) | Token::Keyword(Keyword::Private) => {
                self.parse_visibility_modifier()
            }

            // CMX (JSX)
            Token::Cmx(_) => self.parse_cmx(),

            // Default: expresión
            _ => {
                let expr = self.parse_expression()?;
                self.consume_symbol(Symbol::Semicolon);
                Ok(Statement::Expression(expr))
            }
        }
    }

    fn check_directive(&self) -> bool {
        // Verificar si es #config, #define, etc.
        false // TODO: implementar
    }

    fn parse_config_directive(&mut self) -> ClsResult<Statement> {
        // TODO: implementar #config(...)
        Err(ClsError::SyntaxError("Config directives no implementados aún".to_string()))
    }

    fn parse_config(&mut self) -> ClsResult<Statement> {
        // TODO: implementar
        Err(ClsError::SyntaxError("Config no implementado aún".to_string()))
    }

    fn parse_var_decl(&mut self) -> ClsResult<Statement> {
        self.expect_keyword(Keyword::Var)?;
        let name = self.expect_identifier()?;
        
        let type_ann = if self.consume_operator(Operator::Colon) {
            Some(self.parse_type_annotation()?)
        } else {
            None
        };

        let value = if self.consume_operator(Operator::Equal) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        self.consume_symbol(Symbol::Semicolon);

        Ok(Statement::VarDecl(VarDecl {
            name,
            type_ann,
            value,
            visibility: Visibility::Default,
            span: Span::new(0, 0, 0, 0),
        }))
    }

    fn parse_const_decl(&mut self) -> ClsResult<Statement> {
        self.expect_keyword(Keyword::Const)?;
        let name = self.expect_identifier()?;
        
        let type_ann = if self.consume_operator(Operator::Colon) {
            Some(self.parse_type_annotation()?)
        } else {
            None
        };

        let value = if self.consume_operator(Operator::Equal) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        self.consume_symbol(Symbol::Semicolon);

        Ok(Statement::ConstDecl(VarDecl {
            name,
            type_ann,
            value,
            visibility: Visibility::Default,
            span: Span::new(0, 0, 0, 0),
        }))
    }

    fn parse_function_decl(&mut self) -> ClsResult<Statement> {
        self.expect_keyword(Keyword::Function)?;
        let name = self.expect_identifier()?;
        
        // Parámetros
        self.expect_symbol(Symbol::LParen)?;
        let params = self.parse_parameters()?;
        self.expect_symbol(Symbol::RParen)?;

        // Return type
        let return_type = if self.consume_operator(Operator::Arrow) {
            Some(self.parse_type_annotation()?)
        } else {
            None
        };

        // Body
        let body = self.parse_block()?;

        self.consume_symbol(Symbol::Semicolon);

        Ok(Statement::FunctionDecl(FunctionDecl {
            name,
            params,
            return_type,
            body,
            visibility: Visibility::Default,
            modifiers: Vec::new(),
            span: Span::new(0, 0, 0, 0),
        }))
    }

    fn parse_void_function(&mut self) -> ClsResult<Statement> {
        self.expect_keyword(Keyword::Void)?;
        let name = self.expect_identifier()?;
        
        self.expect_symbol(Symbol::LParen)?;
        let params = self.parse_parameters()?;
        self.expect_symbol(Symbol::RParen)?;
        
        let body = self.parse_block()?;
        self.consume_symbol(Symbol::Semicolon);

        Ok(Statement::FunctionDecl(FunctionDecl {
            name,
            params,
            return_type: Some(TypeAnnotation {
                kind: TypeKind::Void,
                span: Span::new(0, 0, 0, 0),
            }),
            body,
            visibility: Visibility::Default,
            modifiers: Vec::new(),
            span: Span::new(0, 0, 0, 0),
        }))
    }

    fn parse_method_decl(&mut self) -> ClsResult<Statement> {
        // Similar a function pero es un método de clase
        self.parse_function_decl()
    }

    fn parse_parameters(&mut self) -> ClsResult<Vec<Parameter>> {
        let mut params = Vec::new();

        if !self.check_symbol(Symbol::RParen) {
            loop {
                let name = self.expect_identifier()?;
                
                let type_ann = if self.consume_operator(Operator::Colon) {
                    Some(self.parse_type_annotation()?)
                } else {
                    None
                };

                let default_value = if self.consume_operator(Operator::Equal) {
                    Some(self.parse_expression()?)
                } else {
                    None
                };

                params.push(Parameter {
                    name,
                    type_ann,
                    default_value,
                    span: Span::new(0, 0, 0, 0),
                });

                if !self.consume_symbol(Symbol::Comma) {
                    break;
                }
            }
        }

        Ok(params)
    }

    fn parse_type_annotation(&mut self) -> ClsResult<TypeAnnotation> {
        // Fun types: fun(params...) -> ReturnType
        if let Token::Identifier(ref s) = &self.current_token {
            if s == "fun" && self.lookahead_is(1, Symbol::LParen) {
                return self.parse_fun_type();
            }
        }

        let mut kind = self.parse_base_type()?;
        let span = Span::new(0, 0, 0, 0);

        // Postfix: [] para arrays (sin llaves para evitar ambigüedad con bloques)
        loop {
            if self.consume_symbol(Symbol::LBracket) {
                if !self.consume_symbol(Symbol::RBracket) {
                    return Err(ClsError::SyntaxError("Esperaba ']' en tipo array".to_string()));
                }
                kind = TypeKind::Array(Box::new(TypeAnnotation {
                    kind: kind.clone(),
                    span: span.clone(),
                }));
            } else {
                break;
            }
        }

        Ok(TypeAnnotation { kind, span })
    }

    fn parse_base_type(&mut self) -> ClsResult<TypeKind> {
        // Paréntesis
        if self.consume_symbol(Symbol::LParen) {
            let inner = self.parse_type_annotation()?;
            self.expect_symbol(Symbol::RParen)?;
            return Ok(inner.kind);
        }

        // Identificador o acrónimo
        let name = self.expect_identifier()?;
        let name_str = name.as_str();

        // Acrónimos
        match name_str {
            "int" | "Integer" => return Ok(TypeKind::Int),
            "str" | "String" => return Ok(TypeKind::String),
            "float" | "Float" => return Ok(TypeKind::Float),
            "bool" | "Boolean" => return Ok(TypeKind::Bool),
            "char" | "Character" => return Ok(TypeKind::Char),
            "any" | "Any" => return Ok(TypeKind::Any),
            "unknown" => return Ok(TypeKind::Unknown),
            "null" => return Ok(TypeKind::Null),
            "Void" | "void" => return Ok(TypeKind::Void),
            "cmx" | "Cmx" => return Ok(TypeKind::Cmx),
            "i32" => return Ok(TypeKind::I32),
            "i64" => return Ok(TypeKind::I64),
            "i16" => return Ok(TypeKind::I16),
            "i8" => return Ok(TypeKind::I8),
            "f32" => return Ok(TypeKind::F32),
            "f64" => return Ok(TypeKind::F64),
            _ => {}
        }

        // Tipo nombrado (puede tener parámetros genéricos <T, U>)
        let mut type_params = Vec::new();
        if !self.is_eof() && matches!(self.current_token, Token::Operator(Operator::LessThan)) {
            // Consumir '<'
            self.advance(); // consume '<'
            loop {
                type_params.push(self.parse_type_annotation()?);
                if !self.consume_symbol(Symbol::Comma) {
                    break;
                }
            }
            // Consumir '>' (que puede ser Operator::GreaterThan)
            if matches!(self.current_token, Token::Operator(Operator::GreaterThan)
                | Token::Operator(Operator::ShiftRight))
            {
                self.advance();
            } else {
                return Err(ClsError::SyntaxError(
                    "Esperaba '>' para cerrar parámetros genéricos".to_string(),
                ));
            }
        }

        Ok(TypeKind::Named(name, type_params))
    }

    fn parse_fun_type(&mut self) -> ClsResult<TypeAnnotation> {
        // "fun" ya fue verificado
        let ident = self.expect_identifier()?;
        assert!(ident == "fun", "parse_fun_type called without 'fun'");

        self.expect_symbol(Symbol::LParen)?;

        let mut params = Vec::new();
        if !self.check_symbol(Symbol::RParen) {
            loop {
                params.push(self.parse_type_annotation()?);
                if !self.consume_symbol(Symbol::Comma) {
                    break;
                }
            }
        }
        self.expect_symbol(Symbol::RParen)?;

        self.expect_operator(Operator::Colon)?;
        let return_type = self.parse_type_annotation()?;

        Ok(TypeAnnotation {
            kind: TypeKind::Fun(params, Box::new(return_type)),
            span: Span::new(0, 0, 0, 0),
        })
    }

    fn lookahead_is(&self, offset: usize, expected: Symbol) -> bool {
        // Lookahead básico: verificar si el token a N posiciones es el símbolo esperado
        // Por ahora esto es una aproximación simple
        // TODO: implementar lookahead real con peekable tokens
        false
    }

    fn parse_if_statement(&mut self) -> ClsResult<Statement> {
        self.expect_keyword(Keyword::If)?;
        self.expect_symbol(Symbol::LParen)?;
        let condition = self.parse_expression()?;
        self.expect_symbol(Symbol::RParen)?;
        
        let then_block = self.parse_block()?;
        
        let mut elif_branches = Vec::new();
        while self.consume_keyword(Keyword::Elif) {
            self.expect_symbol(Symbol::LParen)?;
            let elif_cond = self.parse_expression()?;
            self.expect_symbol(Symbol::RParen)?;
            let elif_block = self.parse_block()?;
            elif_branches.push(ElifBranch {
                condition: elif_cond,
                block: elif_block,
                span: Span::new(0, 0, 0, 0),
            });
        }
        
        let else_block = if self.consume_keyword(Keyword::Else) {
            Some(self.parse_block()?)
        } else {
            None
        };
        
        self.consume_symbol(Symbol::Semicolon);
        
        Ok(Statement::If(IfStatement {
            condition,
            then_block,
            elif_branches,
            else_block,
            span: Span::new(0, 0, 0, 0),
        }))
    }

    fn parse_while_statement(&mut self) -> ClsResult<Statement> {
        self.expect_keyword(Keyword::While)?;
        self.expect_symbol(Symbol::LParen)?;
        let condition = self.parse_expression()?;
        self.expect_symbol(Symbol::RParen)?;
        let block = self.parse_block()?;
        self.consume_symbol(Symbol::Semicolon);
        
        Ok(Statement::While(WhileStatement {
            condition,
            block,
            span: Span::new(0, 0, 0, 0),
        }))
    }

    fn parse_loop_statement(&mut self) -> ClsResult<Statement> {
        self.expect_keyword(Keyword::Loop)?;
        let block = self.parse_block()?;
        self.consume_symbol(Symbol::Semicolon);
        
        Ok(Statement::Loop(block))
    }

    fn parse_for_statement(&mut self) -> ClsResult<Statement> {
        self.expect_keyword(Keyword::For)?;
        
        if self.consume_keyword(Keyword::Each) {
            return self.parse_for_each_statement();
        }
        
        // For tradicional
        self.expect_symbol(Symbol::LParen)?;
        
        let init = if self.check_operator(Operator::Equal) {
            // sin init
            None
        } else if self.consume_keyword(Keyword::Var) {
            let stmt = self.parse_var_decl()?;
            Some(Box::new(stmt))
        } else {
            let expr = self.parse_expression()?;
            Some(Box::new(Statement::Expression(expr)))
        };
        
        self.expect_symbol(Symbol::Semicolon)?;
        
        let condition = if self.consume_symbol(Symbol::Semicolon) {
            None
        } else {
            let cond = self.parse_expression()?;
            self.expect_symbol(Symbol::Semicolon)?;
            Some(cond)
        };
        
        let update = if self.check_symbol(Symbol::RParen) {
            None
        } else {
            let upd = self.parse_expression()?;
            Some(upd)
        };
        
        self.expect_symbol(Symbol::RParen)?;
        let block = self.parse_block()?;
        self.consume_symbol(Symbol::Semicolon);
        
        Ok(Statement::For(ForStatement {
            init,
            condition,
            update,
            block,
            span: Span::new(0, 0, 0, 0),
        }))
    }

    fn parse_for_each_statement(&mut self) -> ClsResult<Statement> {
        // Ya se consumió "for each"
        let item_name = self.expect_identifier()?;
        
        let index_name = if self.consume_keyword(Keyword::And) {
            Some(self.expect_identifier()?)
        } else {
            None
        };
        
        self.expect_keyword(Keyword::In)?;
        self.expect_symbol(Symbol::LParen)?;
        let iterable = self.parse_expression()?;
        self.expect_symbol(Symbol::RParen)?;
        let block = self.parse_block()?;
        self.consume_symbol(Symbol::Semicolon);
        
        Ok(Statement::ForEach(ForEachStatement {
            item_name,
            index_name,
            iterable,
            block,
            span: Span::new(0, 0, 0, 0),
        }))
    }

    fn parse_switch_statement(&mut self) -> ClsResult<Statement> {
        self.expect_keyword(Keyword::Switch)?;
        self.expect_symbol(Symbol::LParen)?;
        let value = self.parse_expression()?;
        self.expect_symbol(Symbol::RParen)?;
        self.expect_symbol(Symbol::LBrace)?;
        
        let mut cases = Vec::new();
        let mut default = None;
        
        while !self.check_symbol(Symbol::RBrace) && !self.is_eof() {
            self.skip_newlines();
            
            if self.consume_keyword(Keyword::Case) {
                if self.consume_keyword(Keyword::Default) {
                    default = Some(self.parse_block()?);
                } else {
                    let pattern = self.parse_case_pattern()?;
                    let block = self.parse_block()?;
                    cases.push(CaseClause {
                        pattern,
                        block,
                        span: Span::new(0, 0, 0, 0),
                    });
                }
                self.consume_symbol(Symbol::Semicolon);
            } else {
                break;
            }
        }
        
        self.expect_symbol(Symbol::RBrace)?;
        self.consume_symbol(Symbol::Semicolon);
        
        Ok(Statement::Switch(SwitchStatement {
            value,
            cases,
            default,
            span: Span::new(0, 0, 0, 0),
        }))
    }

    fn parse_case_pattern(&mut self) -> ClsResult<CasePattern> {
        self.expect_symbol(Symbol::LParen)?;
        let expr = self.parse_expression()?;
        self.expect_symbol(Symbol::RParen)?;
        
        match expr {
            Expression::Literal(lit) => Ok(CasePattern::Literal(lit)),
            Expression::Identifier(name, _) => Ok(CasePattern::Identifier(name)),
            _ => Err(ClsError::SyntaxError("Pattern de case inválido".to_string())),
        }
    }

    fn parse_try_statement(&mut self) -> ClsResult<Statement> {
        self.expect_keyword(Keyword::Try)?;
        let try_block = self.parse_block()?;
        
        let mut catch_clauses = Vec::new();
        while self.consume_keyword(Keyword::Catch) {
            self.expect_symbol(Symbol::LParen)?;
            let param_name = self.expect_identifier()?;
            let param_type = if self.consume_operator(Operator::Colon) {
                Some(self.parse_type_annotation()?)
            } else {
                None
            };
            self.expect_symbol(Symbol::RParen)?;
            let block = self.parse_block()?;
            catch_clauses.push(CatchClause {
                param_name,
                param_type,
                block,
                span: Span::new(0, 0, 0, 0),
            });
            self.consume_symbol(Symbol::Semicolon);
        }
        
        let finally_block = if self.consume_keyword(Keyword::Finally) {
            Some(self.parse_block()?)
        } else {
            None
        };
        
        self.consume_symbol(Symbol::Semicolon);
        
        Ok(Statement::Try(TryStatement {
            try_block,
            catch_clauses,
            finally_block,
            span: Span::new(0, 0, 0, 0),
        }))
    }

    fn parse_with_statement(&mut self) -> ClsResult<Statement> {
        self.expect_keyword(Keyword::With)?;
        let name = self.expect_identifier()?;
        self.expect_keyword(Keyword::In)?;
        self.expect_symbol(Symbol::LParen)?;
        let value = self.parse_expression()?;
        self.expect_symbol(Symbol::RParen)?;
        let block = self.parse_block()?;
        self.consume_symbol(Symbol::Semicolon);
        
        Ok(Statement::With(WithStatement {
            name,
            value,
            block,
            span: Span::new(0, 0, 0, 0),
        }))
    }

    fn parse_return_statement(&mut self) -> ClsResult<Statement> {
        self.expect_keyword(Keyword::Return)?;
        
        let value = if self.check_symbol(Symbol::Semicolon) || self.is_eof() {
            None
        } else {
            Some(self.parse_expression()?)
        };
        
        self.consume_symbol(Symbol::Semicolon);
        
        Ok(Statement::Return(value))
    }

    fn parse_break(&mut self) -> ClsResult<Statement> {
        self.expect_keyword(Keyword::Break)?;
        self.consume_symbol(Symbol::Semicolon);
        Ok(Statement::Break)
    }

    fn parse_continue(&mut self) -> ClsResult<Statement> {
        self.expect_keyword(Keyword::Continue)?;
        self.consume_symbol(Symbol::Semicolon);
        Ok(Statement::Continue)
    }

    fn parse_class_decl(&mut self) -> ClsResult<Statement> {
        self.expect_keyword(Keyword::Class)?;
        let name = self.expect_identifier()?;
        
        // Herencia
        let extends = if self.consume_symbol(Symbol::LParen) {
            let parent = self.expect_identifier()?;
            self.expect_symbol(Symbol::RParen)?;
            Some(parent)
        } else {
            None
        };
        
        // Implements (opcional)
        let implements = Vec::new(); // TODO
        
        self.expect_symbol(Symbol::LBrace)?;
        
        let mut body = Vec::new();
        while !self.check_symbol(Symbol::RBrace) && !self.is_eof() {
            self.skip_newlines();
            let member = self.parse_class_member()?;
            body.push(member);
        }
        
        self.expect_symbol(Symbol::RBrace)?;
        self.consume_symbol(Symbol::Semicolon);
        
        Ok(Statement::ClassDecl(ClassDecl {
            name,
            extends,
            implements,
            body,
            span: Span::new(0, 0, 0, 0),
        }))
    }

    fn parse_class_member(&mut self) -> ClsResult<ClassMember> {
        // Verificar si es un método o una propiedad
        let is_method = self.check_keyword(Keyword::Function) || self.check_keyword(Keyword::Void);
        
        if is_method {
            let stmt = self.parse_function_decl()?;
            match stmt {
                Statement::FunctionDecl(func) => {
                    if func.name == "main" {
                        Ok(ClassMember::Constructor(func))
                    } else {
                        Ok(ClassMember::Method(func))
                    }
                }
                _ => unreachable!(),
            }
        } else {
            // Es una propiedad
            let stmt = self.parse_var_decl()?;
            match stmt {
                Statement::VarDecl(var) => Ok(ClassMember::Property(var)),
                _ => unreachable!(),
            }
        }
    }

    fn parse_structure_decl(&mut self) -> ClsResult<Statement> {
        self.expect_keyword(Keyword::Structure)?;
        let name = self.expect_identifier()?;
        
        self.expect_symbol(Symbol::LBrace)?;
        
        let mut fields = Vec::new();
        while !self.check_symbol(Symbol::RBrace) && !self.is_eof() {
            self.skip_newlines();
            
            let field_name = self.expect_identifier()?;
            self.expect_operator(Operator::Colon)?;
            let type_ann = self.parse_type_annotation()?;
            
            let default_value = if self.consume_operator(Operator::Equal) {
                Some(self.parse_expression()?)
            } else {
                None
            };
            
            fields.push(FieldDecl {
                name: field_name,
                type_ann,
                default_value,
                span: Span::new(0, 0, 0, 0),
            });
            
            self.consume_symbol(Symbol::Comma);
        }
        
        self.expect_symbol(Symbol::RBrace)?;
        self.consume_symbol(Symbol::Semicolon);
        
        Ok(Statement::StructureDecl(StructureDecl {
            name,
            fields,
            span: Span::new(0, 0, 0, 0),
        }))
    }

    fn parse_interface_decl(&mut self) -> ClsResult<Statement> {
        self.expect_keyword(Keyword::Interface)?;
        let name = self.expect_identifier()?;
        self.expect_symbol(Symbol::LParen)?;
        self.expect_symbol(Symbol::RParen)?;
        
        self.expect_symbol(Symbol::LBrace)?;
        
        let mut signatures = Vec::new();
        while !self.check_symbol(Symbol::RBrace) && !self.is_eof() {
            self.skip_newlines();
            
            let sig_name = self.expect_identifier()?;
            self.expect_symbol(Symbol::LParen)?;
            let params = self.parse_parameters()?;
            self.expect_symbol(Symbol::RParen)?;
            
            let return_type = if self.consume_operator(Operator::Colon) {
                Some(self.parse_type_annotation()?)
            } else {
                None
            };
            
            signatures.push(SignatureDecl {
                name: sig_name,
                params,
                return_type,
                span: Span::new(0, 0, 0, 0),
            });
            
            self.consume_symbol(Symbol::Comma);
        }
        
        self.expect_symbol(Symbol::RBrace)?;
        self.consume_symbol(Symbol::Semicolon);
        
        Ok(Statement::InterfaceDecl(InterfaceDecl {
            name,
            signatures,
            span: Span::new(0, 0, 0, 0),
        }))
    }

    fn parse_module_decl(&mut self) -> ClsResult<Statement> {
        self.expect_keyword(Keyword::Module)?;
        let name = self.expect_identifier()?;
        
        self.expect_symbol(Symbol::LBrace)?;
        
        let mut body = Vec::new();
        while !self.check_symbol(Symbol::RBrace) && !self.is_eof() {
            self.skip_newlines();
            let stmt = self.parse_statement()?;
            body.push(stmt);
        }
        
        self.expect_symbol(Symbol::RBrace)?;
        self.consume_symbol(Symbol::Semicolon);
        
        Ok(Statement::ModuleDecl(ModuleDecl {
            name,
            body,
            span: Span::new(0, 0, 0, 0),
        }))
    }

    fn parse_namespace_decl(&mut self) -> ClsResult<Statement> {
        self.expect_keyword(Keyword::Namespace)?;
        let name = self.expect_identifier()?;
        
        self.expect_symbol(Symbol::LBrace)?;
        
        let mut body = Vec::new();
        while !self.check_symbol(Symbol::RBrace) && !self.is_eof() {
            self.skip_newlines();
            let stmt = self.parse_statement()?;
            body.push(stmt);
        }
        
        self.expect_symbol(Symbol::RBrace)?;
        self.consume_symbol(Symbol::Semicolon);
        
        Ok(Statement::NamespaceDecl(NamespaceDecl {
            name,
            body,
            span: Span::new(0, 0, 0, 0),
        }))
    }

    fn parse_import(&mut self) -> ClsResult<Statement> {
        self.expect_keyword(Keyword::Import)?;
        let path = self.expect_string()?;
        
        let alias = if self.consume_keyword(Keyword::As) {
            Some(self.expect_identifier()?)
        } else {
            None
        };
        
        self.consume_symbol(Symbol::Semicolon);
        
        Ok(Statement::Import(ImportStatement {
            path,
            alias,
            span: Span::new(0, 0, 0, 0),
        }))
    }

    fn parse_from_import(&mut self) -> ClsResult<Statement> {
        self.expect_keyword(Keyword::From)?;
        let path = self.expect_string()?;
        self.expect_keyword(Keyword::Import)?;
        
        let mut names = Vec::new();
        loop {
            let name = self.expect_identifier()?;
            let alias = if self.consume_keyword(Keyword::As) {
                Some(self.expect_identifier()?)
            } else {
                None
            };
            names.push(ImportName { name, alias });
            
            if !self.consume_symbol(Symbol::Comma) {
                break;
            }
        }
        
        self.consume_symbol(Symbol::Semicolon);
        
        Ok(Statement::FromImport(FromImportStatement {
            path,
            names,
            span: Span::new(0, 0, 0, 0),
        }))
    }

    fn parse_include(&mut self) -> ClsResult<Statement> {
        self.expect_keyword(Keyword::Include)?;
        let path = self.expect_string()?;
        self.consume_symbol(Symbol::Semicolon);
        
        Ok(Statement::Include(IncludeStatement {
            path,
            span: Span::new(0, 0, 0, 0),
        }))
    }

    fn parse_visibility_modifier(&mut self) -> ClsResult<Statement> {
        // public/private func/var/etc
        let visibility = match self.current_token {
            Token::Keyword(Keyword::Public) => Visibility::Public,
            Token::Keyword(Keyword::Private) => Visibility::Private,
            _ => Visibility::Default,
        };
        
        self.advance(); // consume public/private
        
        // Luego parsear la declaración
        let mut stmt = self.parse_statement()?;
        
        // Aplicar visibilidad
        if let Statement::FunctionDecl(ref mut f) = stmt {
            f.visibility = visibility;
        } else if let Statement::VarDecl(ref mut v) = stmt {
            v.visibility = visibility;
        }
        
        Ok(stmt)
    }

    fn parse_cmx(&mut self) -> ClsResult<Statement> {
        // TODO: implementar CMX parser
        Err(ClsError::SyntaxError("CMX parser no implementado aún".to_string()))
    }

    fn parse_block(&mut self) -> ClsResult<Block> {
        self.expect_symbol(Symbol::LBrace)?;
        
        let mut statements = Vec::new();
        while !self.check_symbol(Symbol::RBrace) && !self.is_eof() {
            self.skip_newlines();
            self.skip_semicolons();
            if self.check_symbol(Symbol::RBrace) {
                break;
            }
            let stmt = self.parse_statement()?;
            statements.push(stmt);
        }
        
        self.expect_symbol(Symbol::RBrace)?;
        self.skip_semicolons();
        
        Ok(Block {
            statements,
            span: Span::new(0, 0, 0, 0),
        })
    }

    // ═══════════════════════════════════════════
    // PARSER DE EXPRESIONES (recursive descent con precedencia)
    // ═══════════════════════════════════════════

    fn parse_expression(&mut self) -> ClsResult<Expression> {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> ClsResult<Expression> {
        let expr = self.parse_conditional()?;
        
        if let Some(op) = self.check_assignment_operator() {
            self.advance();
            let value = self.parse_assignment()?;
            return Ok(Expression::Assignment(AssignmentExpr {
                target: Box::new(expr),
                op,
                value: Box::new(value),
                span: Span::new(0, 0, 0, 0),
            }));
        }
        
        Ok(expr)
    }

    fn check_assignment_operator(&self) -> Option<Operator> {
        match self.current_token {
            Token::Operator(Operator::Equal) => Some(Operator::Equal),
            Token::Operator(Operator::PlusEqual) => Some(Operator::PlusEqual),
            Token::Operator(Operator::MinusEqual) => Some(Operator::MinusEqual),
            Token::Operator(Operator::StarEqual) => Some(Operator::StarEqual),
            Token::Operator(Operator::SlashEqual) => Some(Operator::SlashEqual),
            _ => None,
        }
    }

    fn parse_conditional(&mut self) -> ClsResult<Expression> {
        let condition = self.parse_logical_or()?;
        
        if self.consume_keyword(Keyword::If) {
            self.expect_symbol(Symbol::LParen)?;
            let cond = self.parse_expression()?;
            self.expect_symbol(Symbol::RParen)?;
            self.expect_keyword(Keyword::Then)?;
            self.expect_symbol(Symbol::LParen)?;
            let then_expr = self.parse_expression()?;
            self.expect_symbol(Symbol::RParen)?;
            self.expect_keyword(Keyword::Else)?;
            self.expect_symbol(Symbol::LParen)?;
            let else_expr = self.parse_expression()?;
            self.expect_symbol(Symbol::RParen)?;
            
            return Ok(Expression::Conditional(ConditionalExpr {
                condition: Box::new(cond),
                then_expr: Box::new(then_expr),
                else_expr: Box::new(else_expr),
                span: Span::new(0, 0, 0, 0),
            }));
        }
        
        Ok(condition)
    }

    fn parse_logical_or(&mut self) -> ClsResult<Expression> {
        let mut expr = self.parse_logical_and()?;
        
        while self.check_operator(Operator::Or) {
            self.advance();
            let right = self.parse_logical_and()?;
            expr = Expression::Binary(BinaryExpr {
                left: Box::new(expr),
                op: Operator::Or,
                right: Box::new(right),
                span: Span::new(0, 0, 0, 0),
            });
        }
        
        Ok(expr)
    }

    fn parse_logical_and(&mut self) -> ClsResult<Expression> {
        let mut expr = self.parse_equality()?;
        
        while self.check_operator(Operator::And) {
            self.advance();
            let right = self.parse_equality()?;
            expr = Expression::Binary(BinaryExpr {
                left: Box::new(expr),
                op: Operator::And,
                right: Box::new(right),
                span: Span::new(0, 0, 0, 0),
            });
        }
        
        Ok(expr)
    }

    fn parse_equality(&mut self) -> ClsResult<Expression> {
        let mut expr = self.parse_comparison()?;
        
        while let Token::Operator(op) = &self.current_token {
            let op = match op {
                Operator::StrictEqual | Operator::NotEqual => op.clone(),
                _ => break,
            };
            self.advance();
            let right = self.parse_comparison()?;
            expr = Expression::Binary(BinaryExpr {
                left: Box::new(expr),
                op,
                right: Box::new(right),
                span: Span::new(0, 0, 0, 0),
            });
        }
        
        Ok(expr)
    }

    fn parse_comparison(&mut self) -> ClsResult<Expression> {
        let mut expr = self.parse_term()?;
        
        while let Token::Operator(op) = &self.current_token {
            let op = match op {
                Operator::LessThan
                | Operator::LessEqual
                | Operator::GreaterThan
                | Operator::GreaterEqual => op.clone(),
                _ => break,
            };
            self.advance();
            let right = self.parse_term()?;
            expr = Expression::Binary(BinaryExpr {
                left: Box::new(expr),
                op,
                right: Box::new(right),
                span: Span::new(0, 0, 0, 0),
            });
        }
        
        Ok(expr)
    }

    fn parse_term(&mut self) -> ClsResult<Expression> {
        let mut expr = self.parse_factor()?;
        
        while let Token::Operator(op) = &self.current_token {
            let op = match op {
                Operator::Plus | Operator::Minus => op.clone(),
                _ => break,
            };
            self.advance();
            let right = self.parse_factor()?;
            expr = Expression::Binary(BinaryExpr {
                left: Box::new(expr),
                op,
                right: Box::new(right),
                span: Span::new(0, 0, 0, 0),
            });
        }
        
        Ok(expr)
    }

    fn parse_factor(&mut self) -> ClsResult<Expression> {
        let mut expr = self.parse_unary()?;
        
        while let Token::Operator(op) = &self.current_token {
            let op = match op {
                Operator::Star
                | Operator::Slash
                | Operator::Percent
                | Operator::StarStar => op.clone(),
                _ => break,
            };
            self.advance();
            let right = self.parse_unary()?;
            expr = Expression::Binary(BinaryExpr {
                left: Box::new(expr),
                op,
                right: Box::new(right),
                span: Span::new(0, 0, 0, 0),
            });
        }
        
        Ok(expr)
    }

    fn parse_unary(&mut self) -> ClsResult<Expression> {
        if let Token::Operator(op) = &self.current_token {
            let op = match op {
                Operator::Minus => UnaryOp::Negate,
                Operator::Not => UnaryOp::Not,
                Operator::Tilde => UnaryOp::BitwiseNot,
                _ => return self.parse_call(),
            };
            self.advance();
            let operand = self.parse_unary()?;
            return Ok(Expression::Unary(UnaryExpr {
                op,
                operand: Box::new(operand),
                span: Span::new(0, 0, 0, 0),
            }));
        }
        
        self.parse_call()
    }

    fn parse_call(&mut self) -> ClsResult<Expression> {
        let mut expr = self.parse_primary()?;
        
        loop {
            match self.current_token {
                // Llamada de función
                Token::Symbol(Symbol::LParen) => {
                    self.advance();
                    let args = self.parse_call_args()?;
                    self.expect_symbol(Symbol::RParen)?;
                    expr = Expression::Call(CallExpr {
                        callee: Box::new(expr),
                        args,
                        span: Span::new(0, 0, 0, 0),
                    });
                }
                // Acceso a miembro
                Token::Symbol(Symbol::Dot) => {
                    self.advance();
                    let member = self.expect_identifier()?;
                    expr = Expression::MemberAccess(MemberAccessExpr {
                        object: Box::new(expr),
                        member,
                        span: Span::new(0, 0, 0, 0),
                    });
                }
                // Namespace access
                Token::Operator(Operator::ColonColon) => {
                    self.advance();
                    let member = self.expect_identifier()?;
                    expr = Expression::NamespaceAccess(
                        match expr {
                            Expression::Identifier(name, _) => name,
                            _ => return Err(ClsError::SyntaxError("Esperaba identificador".to_string())),
                        },
                        member,
                        Span::new(0, 0, 0, 0),
                    );
                }
                // Indexado
                Token::Symbol(Symbol::LBracket) => {
                    self.advance();
                    let index = self.parse_expression()?;
                    self.expect_symbol(Symbol::RBracket)?;
                    expr = Expression::Index(IndexExpr {
                        object: Box::new(expr),
                        index: Box::new(index),
                        span: Span::new(0, 0, 0, 0),
                    });
                }
                _ => break,
            }
        }
        
        Ok(expr)
    }

    fn parse_call_args(&mut self) -> ClsResult<Vec<Expression>> {
        let mut args = Vec::new();
        
        if !self.check_symbol(Symbol::RParen) {
            loop {
                args.push(self.parse_expression()?);
                if !self.consume_symbol(Symbol::Comma) {
                    break;
                }
            }
        }
        
        Ok(args)
    }

    fn parse_primary(&mut self) -> ClsResult<Expression> {
        match &self.current_token {
            Token::IntLiteral(v) => {
                let v = *v;
                self.advance();
                Ok(Expression::Literal(Literal {
                    kind: LiteralKind::Int(v),
                    span: Span::new(0, 0, 0, 0),
                }))
            }
            Token::FloatLiteral(v) => {
                let v = *v;
                self.advance();
                Ok(Expression::Literal(Literal {
                    kind: LiteralKind::Float(v),
                    span: Span::new(0, 0, 0, 0),
                }))
            }
            Token::StringLiteral(v) => {
                let v = v.clone();
                self.advance();
                Ok(Expression::Literal(Literal {
                    kind: LiteralKind::String(v),
                    span: Span::new(0, 0, 0, 0),
                }))
            }
            Token::BoolLiteral(v) => {
                let v = *v;
                self.advance();
                Ok(Expression::Literal(Literal {
                    kind: LiteralKind::Bool(v),
                    span: Span::new(0, 0, 0, 0),
                }))
            }
            Token::CharLiteral(v) => {
                let v = *v;
                self.advance();
                Ok(Expression::Literal(Literal {
                    kind: LiteralKind::Char(v),
                    span: Span::new(0, 0, 0, 0),
                }))
            }
            Token::Identifier(name) => {
                let name = name.clone();
                self.advance();
                
                // Verificar si es una función flecha
                if self.check_symbol(Symbol::LParen) {
                    // Podría ser una llamada o una función flecha
                    // Por ahora devolvemos el identificador y que parse_call lo maneje
                    return Ok(Expression::Identifier(name, Span::new(0, 0, 0, 0)));
                }
                
                Ok(Expression::Identifier(name, Span::new(0, 0, 0, 0)))
            }
            Token::Keyword(Keyword::Me) => {
                self.advance();
                Ok(Expression::Identifier("me".to_string(), Span::new(0, 0, 0, 0)))
            }
            Token::Keyword(Keyword::True) => {
                self.advance();
                Ok(Expression::Literal(Literal {
                    kind: LiteralKind::Bool(true),
                    span: Span::new(0, 0, 0, 0),
                }))
            }
            Token::Keyword(Keyword::False) => {
                self.advance();
                Ok(Expression::Literal(Literal {
                    kind: LiteralKind::Bool(false),
                    span: Span::new(0, 0, 0, 0),
                }))
            }
            Token::Symbol(Symbol::LParen) => {
                self.advance();
                
                // Función flecha: (params) -> type { body }
                // O paréntesis: (expr)
                
                // Verificar si es función flecha
                if self.is_arrow_function() {
                    return self.parse_arrow_function();
                }
                
                // Paréntesis
                let expr = self.parse_expression()?;
                self.expect_symbol(Symbol::RParen)?;
                Ok(Expression::Parenthesized(Box::new(expr), Span::new(0, 0, 0, 0)))
            }
            Token::Symbol(Symbol::LBracket) => {
                self.advance();
                let mut elements = Vec::new();
                if !self.check_symbol(Symbol::RBracket) {
                    loop {
                        elements.push(self.parse_expression()?);
                        if !self.consume_symbol(Symbol::Comma) {
                            break;
                        }
                    }
                }
                self.expect_symbol(Symbol::RBracket)?;
                Ok(Expression::Array(ArrayExpr {
                    elements,
                    span: Span::new(0, 0, 0, 0),
                }))
            }
            Token::Symbol(Symbol::LBrace) => {
                self.advance();
                let mut entries = Vec::new();
                if !self.check_symbol(Symbol::RBrace) {
                    loop {
                        let key = match &self.current_token {
                            Token::Identifier(name) => name.clone(),
                            Token::StringLiteral(s) => s.clone(),
                            _ => return Err(ClsError::SyntaxError("Key de record inválida".to_string())),
                        };
                        self.advance();
                        self.expect_operator(Operator::Colon)?;
                        let value = self.parse_expression()?;
                        entries.push((key, value));
                        if !self.consume_symbol(Symbol::Comma) {
                            break;
                        }
                    }
                }
                self.expect_symbol(Symbol::RBrace)?;
                Ok(Expression::Record(RecordExpr {
                    entries,
                    span: Span::new(0, 0, 0, 0),
                }))
            }
            _ => Err(ClsError::SyntaxError(format!(
                "Token inesperado: {:?}",
                self.current_token
            ))),
        }
    }

    fn is_arrow_function(&self) -> bool {
        // Mirar adelante para ver si hay -> 
        // Esto es una aproximación simple
        false // TODO: implementar look-ahead real
    }

    fn parse_arrow_function(&mut self) -> ClsResult<Expression> {
        // TODO: implementar
        Err(ClsError::SyntaxError("Arrow functions no implementadas aún".to_string()))
    }

    // ═══════════════════════════════════════════
    // HELPERS
    // ═══════════════════════════════════════════

    fn advance(&mut self) {
        if let Some(next) = self.tokens.next() {
            self.current_token = next;
        } else {
            self.current_token = Token::EOF;
        }
    }

    fn is_eof(&self) -> bool {
        matches!(self.current_token, Token::EOF)
    }

    fn check_symbol(&self, symbol: Symbol) -> bool {
        matches!(&self.current_token, Token::Symbol(s) if s == &symbol)
    }

    fn check_keyword(&self, keyword: Keyword) -> bool {
        matches!(&self.current_token, Token::Keyword(k) if k == &keyword)
    }

    fn check_operator(&self, op: Operator) -> bool {
        matches!(&self.current_token, Token::Operator(o) if o == &op)
    }

    fn consume_symbol(&mut self, symbol: Symbol) -> bool {
        if self.check_symbol(symbol) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn consume_keyword(&mut self, keyword: Keyword) -> bool {
        if self.check_keyword(keyword) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn consume_operator(&mut self, op: Operator) -> bool {
        if self.check_operator(op) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect_symbol(&mut self, symbol: Symbol) -> ClsResult<()> {
        if self.consume_symbol(symbol) {
            Ok(())
        } else {
            Err(ClsError::SyntaxError(format!(
                "Esperaba símbolo {:?}, encontró {:?}",
                symbol, self.current_token
            )))
        }
    }

    fn expect_keyword(&mut self, keyword: Keyword) -> ClsResult<()> {
        if self.consume_keyword(keyword) {
            Ok(())
        } else {
            Err(ClsError::SyntaxError(format!(
                "Esperaba keyword {:?}, encontró {:?}",
                keyword, self.current_token
            )))
        }
    }

    fn expect_operator(&mut self, op: Operator) -> ClsResult<()> {
        if self.consume_operator(op) {
            Ok(())
        } else {
            Err(ClsError::SyntaxError(format!(
                "Esperaba operador {:?}, encontró {:?}",
                op, self.current_token
            )))
        }
    }

    fn expect_identifier(&mut self) -> ClsResult<String> {
        match &self.current_token {
            Token::Identifier(name) => {
                let name = name.clone();
                self.advance();
                Ok(name)
            }
            _ => Err(ClsError::SyntaxError(format!(
                "Esperaba identificador, encontró {:?}",
                self.current_token
            ))),
        }
    }

    fn expect_string(&mut self) -> ClsResult<String> {
        match &self.current_token {
            Token::StringLiteral(s) => {
                let s = s.clone();
                self.advance();
                Ok(s)
            }
            _ => Err(ClsError::SyntaxError(format!(
                "Esperaba string, encontró {:?}",
                self.current_token
            ))),
        }
    }

    fn skip_newlines(&mut self) {
        while matches!(self.current_token, Token::Newline) {
            self.advance();
        }
    }

    fn skip_semicolons(&mut self) {
        while self.consume_symbol(Symbol::Semicolon) {
            // saltar todos los ; consecutivos
        }
    }

    fn recover(&mut self) {
        // Estrategia de recuperación de errores: saltar hasta el siguiente ; o }
        while !self.is_eof() {
            if self.check_symbol(Symbol::Semicolon) || self.check_symbol(Symbol::RBrace) {
                self.advance();
                break;
            }
            self.advance();
        }
    }
}

use crate::error::{ClsError, ClsResult, Diagnostic};
use crate::error::diagnostic::Span;
use crate::frontend::ast::*;
use crate::frontend::token::{CmxToken, Keyword, Operator, Symbol, Token, SpannedToken};
use crate::middleware::typeck::expr_span;
use std::iter::Peekable;
use std::vec::IntoIter;

/// Parser recursive descent de CLS
/// Convierte tokens en un AST
pub struct Parser {
    tokens: Peekable<IntoIter<SpannedToken>>,
    current_token: Token,
    current_span: Span,
    diagnostics: Vec<Diagnostic>,
}

impl Parser {
    pub fn new(tokens: Vec<SpannedToken>) -> Self {
        let mut iter = tokens.into_iter().peekable();
        let first = iter.next().unwrap_or(SpannedToken::new(Token::EOF, Span::new(1, 1, 1, 1)));
        let span = first.span.clone();
        Self {
            tokens: iter,
            current_token: first.token,
            current_span: span,
            diagnostics: Vec::new(),
        }
    }

    /// Obtiene el span del token actual
    fn span(&self) -> Span {
        self.current_span.clone()
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
            return Err(self.syntax_err("EOF inesperado"));
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
            Token::Keyword(Keyword::Let) => self.parse_var_decl(), // let es alias de var

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
            Token::Keyword(Keyword::Alias) => self.parse_alias_decl(),
            Token::Keyword(Keyword::Enum) => self.parse_enum_decl(),

            // Imports
            Token::Keyword(Keyword::Import) => self.parse_import(),
            Token::Keyword(Keyword::From) => self.parse_from_import(),
            Token::Keyword(Keyword::Include) => self.parse_include(),

            // Nativo (FFI)
            Token::Keyword(Keyword::Extension) => self.parse_extension_decl(),

            // Directiva multi-entorno
            Token::Keyword(Keyword::When) => self.parse_when(),
            Token::Keyword(Keyword::Default) => self.parse_default_when(),

            // Modifiers
            Token::Keyword(Keyword::Public)
            | Token::Keyword(Keyword::Private)
            | Token::Keyword(Keyword::Export) => {
                self.parse_visibility_modifier()
            }
            Token::Keyword(Keyword::Async) => self.parse_async_function(),

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
        false // TODO(dev2): implementar
    }

    fn parse_config_directive(&mut self) -> ClsResult<Statement> {
        // TODO(dev2): implementar #config(...)
        Err(self.syntax_err("Config directives no implementados aún".to_string()))
    }

    fn parse_config(&mut self) -> ClsResult<Statement> {
        // TODO(dev2): implementar
        Err(self.syntax_err("Config no implementado aún".to_string()))
    }

    fn parse_var_decl(&mut self) -> ClsResult<Statement> {
        // Acepta var, let, o const
        let decl_span = self.span();
        if !(self.consume_keyword(Keyword::Var)
            || self.consume_keyword(Keyword::Let)
            || self.consume_keyword(Keyword::Const))
        {
            return Err(self.syntax_err("Esperaba 'var', 'let' o 'const'"));
        }
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
            span: decl_span,
            is_static: false,
            is_readonly: false,
            pool_only: false,
            pool_seed: false,
        }))
    }

    fn parse_const_decl(&mut self) -> ClsResult<Statement> {
        let decl_span = self.span();
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
            span: decl_span,
            is_static: false,
            is_readonly: false,
            pool_only: false,
            pool_seed: false,
        }))
    }

    fn parse_function_decl(&mut self) -> ClsResult<Statement> {
        // Span del inicio de la función (token `function`): apuntar aquí en vez
        // de `self.span()` tras el `;` (que apunta al token siguiente -> frames
        // de call stack con línea off-by-one).
        let fn_span = self.span();
        self.expect_keyword(Keyword::Function)?;
        let name = self.expect_identifier()?;
        let type_params = self.parse_type_params()?;
        
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

        // Body (o declaración nativa si hay ';' sin '{')
        let (body, is_native) = if self.check_symbol(Symbol::Semicolon) {
            self.advance(); // consume ';'
            (Block { statements: Vec::new(), span: self.span() }, true)
        } else {
            (self.parse_block()?, false)
        };

        self.consume_symbol(Symbol::Semicolon);

        Ok(Statement::FunctionDecl(FunctionDecl {
            name,
            params,
            return_type,
            body,
            visibility: Visibility::Default,
            modifiers: Vec::new(),
            span: fn_span,
            type_params,
            is_native,
        }))
    }

    fn parse_async_function(&mut self) -> ClsResult<Statement> {
        self.advance(); // consume "async"
        let mut stmt = self.parse_function_decl()?;
        if let Statement::FunctionDecl(ref mut f) = stmt {
            f.modifiers.push(FunctionModifier::Async);
        }
        Ok(stmt)
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
                span: self.span(),
            }),
            body,
            visibility: Visibility::Default,
            modifiers: Vec::new(),
            span: self.span(),
            type_params: Vec::new(),
            is_native: false,
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
                    span: self.span(),
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
            if s == "fun" && self.lookahead_is(0, Symbol::LParen) {
                return self.parse_fun_type();
            }
        }

        let mut kind = self.parse_base_type()?;
        let span = self.span();

        // Postfix: [] para arrays, ["key"]/[n] para acceso a tipos
        loop {
            if self.consume_symbol(Symbol::LBracket) {
                if self.consume_symbol(Symbol::RBracket) {
                    kind = TypeKind::Array(Box::new(TypeAnnotation {
                        kind: kind.clone(),
                        span: span.clone(),
                    }));
                } else {
                    let access = match &self.current_token {
                        Token::StringLiteral(s) => {
                            let k = s.clone();
                            self.advance();
                            TypeAccess::Key(k)
                        }
                        Token::IntLiteral(i) => {
                            let n = *i as usize;
                            self.advance();
                            TypeAccess::Index(n)
                        }
                        _ => return Err(self.syntax_err(
                            "Esperaba clave \"name\" o índice numérico en acceso a tipo",
                        )),
                    };
                    self.expect_symbol(Symbol::RBracket)?;
                    kind = TypeKind::Access(Box::new(TypeAnnotation {
                        kind: kind.clone(),
                        span: span.clone(),
                    }), access);
                }
            } else {
                break;
            }
        }

        // Unión: tipo | tipo | tipo
        if self.check_operator(Operator::Or) {
            let mut members = vec![TypeAnnotation {
                kind: kind.clone(),
                span: span.clone(),
            }];
            while self.consume_operator(Operator::Or) {
                let m_kind = self.parse_base_type()?;
                members.push(TypeAnnotation {
                    kind: m_kind,
                    span: self.span(),
                });
            }
            kind = TypeKind::Union(members);
        }

        // Conjunción: shape1 & shape2 (merge de campos) u otros tipos
        if self.check_operator(Operator::And) {
            let mut members = vec![TypeAnnotation {
                kind: kind.clone(),
                span: span.clone(),
            }];
            while self.consume_operator(Operator::And) {
                let m_kind = self.parse_base_type()?;
                members.push(TypeAnnotation {
                    kind: m_kind,
                    span: self.span(),
                });
            }
            kind = TypeKind::Intersection(members);
        }

        Ok(TypeAnnotation { kind, span })
    }

    fn parse_base_type(&mut self) -> ClsResult<TypeKind> {
        // Shape literal: `{ nombre: String, edad: Int }` (blueprint de record)
        if matches!(&self.current_token, Token::Symbol(Symbol::LBrace)) {
            self.advance();
            let mut fields = Vec::new();
            if !self.check_symbol(Symbol::RBrace) {
                loop {
                    self.skip_newlines();
                    let name = match &self.current_token {
                        Token::Identifier(n) => n.clone(),
                        _ => return Err(self.syntax_err("Nombre de campo inválido en shape")),
                    };
                    self.advance();
                    self.expect_operator(Operator::Colon)?;
                    let ty = self.parse_type_annotation()?;
                    fields.push((name, ty));
                    if !self.consume_symbol(Symbol::Comma) {
                        break;
                    }
                }
            }
            self.expect_symbol(Symbol::RBrace)?;
            return Ok(TypeKind::Shape(fields));
        }
        // Literales de tipo: "d", 5, 1.5, true
        match &self.current_token {
            Token::StringLiteral(v) => {
                let v = v.clone();
                self.advance();
                return Ok(TypeKind::Literal(LiteralKind::String(v)));
            }
            Token::IntLiteral(v) => {
                let v = *v;
                self.advance();
                return Ok(TypeKind::Literal(LiteralKind::Int(v)));
            }
            Token::FloatLiteral(v) => {
                let v = *v;
                self.advance();
                return Ok(TypeKind::Literal(LiteralKind::Float(v)));
            }
            Token::BoolLiteral(v) => {
                let v = *v;
                self.advance();
                return Ok(TypeKind::Literal(LiteralKind::Bool(v)));
            }
            _ => {}
        }

        // Phantom: !T - el type param no participa en el tipo (no se unifica)
        if self.consume_operator(Operator::Not) {
            let inner = self.parse_base_type()?;
            return Ok(TypeKind::Phantom(Box::new(TypeAnnotation {
                kind: inner,
                span: self.span(),
            })));
        }

        // Paréntesis: (Int) agrupación | (Int, String) tupla
        if self.consume_symbol(Symbol::LParen) {
            let first = self.parse_type_annotation()?;
            let mut elems = vec![first];
            let mut is_tuple = false;
            if self.consume_symbol(Symbol::Comma) {
                is_tuple = true;
                loop {
                    elems.push(self.parse_type_annotation()?);
                    if !self.consume_symbol(Symbol::Comma) {
                        break;
                    }
                }
            }
            self.expect_symbol(Symbol::RParen)?;
            // (Int) -> Int | (Int, String) -> Bool -> tipo funcion
            if self.consume_operator(Operator::Arrow) {
                let ret = self.parse_type_annotation()?;
                return Ok(TypeKind::Fun(elems, Box::new(ret)));
            }
            if is_tuple {
                return Ok(TypeKind::Tuple(elems));
            }
            return Ok(elems.remove(0).kind);
        }

        // Identificador o acrónimo
        let name = self.expect_identifier()?;
        let name_str = name.as_str();

        // Acrónimos
        match name_str {
            "int" | "Int" | "Integer" => return Ok(TypeKind::Int),
            "str" | "String" => return Ok(TypeKind::String),
            "float" | "Float" => return Ok(TypeKind::Float),
            "bool" | "Bool" | "Boolean" => return Ok(TypeKind::Bool),
            "char" | "Char" | "Character" => return Ok(TypeKind::Char),
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
        // `<T>` simple tokenizado como CMX: reinterpretarlo como un arg genérico
        if let Some(tag) = self.consume_cmx_simple_tag() {
            type_params.push(TypeAnnotation {
                kind: TypeKind::Named(tag, vec![]),
                span: self.span(),
            });
        } else if !self.is_eof() && matches!(self.current_token, Token::Operator(Operator::LessThan)) {
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
                return Err(self.syntax_err(
                    "Esperaba '>' para cerrar parámetros genéricos",
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

        self.consume_operator(Operator::Colon);
        self.expect_operator(Operator::Arrow)?;
        let return_type = self.parse_type_annotation()?;

        Ok(TypeAnnotation {
            kind: TypeKind::Fun(params, Box::new(return_type)),
            span: self.span(),
        })
    }

    fn lookahead_is(&self, offset: usize, expected: Symbol) -> bool {
        self.tokens.clone().nth(offset)
            .map_or(false, |t| matches!(&t.token, Token::Symbol(s) if *s == expected))
    }

    /// Si el token actual es un `OpenTag` CMX no-self-closing (`<T`), devuelve el
    /// tag. Se usa para reinterpretar genéricos de tipo que el lexer tokenizó
    /// como CMX (ambiguos con tags).
    fn cmx_as_simple_tag(&self) -> Option<String> {
        if let Token::Cmx(CmxToken::OpenTag { name, is_self_closing }) = &self.current_token {
            if !*is_self_closing {
                return Some(name.clone());
            }
        }
        None
    }

    /// Consume `<T>` simple (OpenTag + CloseTag) como un tag de genérico.
    fn consume_cmx_simple_tag(&mut self) -> Option<String> {
        let name = self.cmx_as_simple_tag()?;
        self.advance();
        if let Token::Cmx(CmxToken::CloseTag { name: close }) = &self.current_token {
            if close == &name {
                self.advance();
            }
        }
        Some(name)
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
                span: self.span(),
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
            span: self.span(),
        }))
    }

    fn parse_while_statement(&mut self) -> ClsResult<Statement> {
        self.expect_keyword(Keyword::While)?;
        self.expect_symbol(Symbol::LParen)?;
        // Condición vacía = true (bucle infinito)
        let condition = if self.check_symbol(Symbol::RParen) {
            Expression::Literal(Literal {
                kind: LiteralKind::Bool(true),
                span: self.span(),
            })
        } else {
            self.parse_expression()?
        };
        self.expect_symbol(Symbol::RParen)?;
        let block = self.parse_block()?;
        self.consume_symbol(Symbol::Semicolon);
        
        Ok(Statement::While(WhileStatement {
            condition,
            block,
            span: self.span(),
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
        
        let (init, init_has_semicolon) = if self.check_operator(Operator::Equal) {
            // sin init
            (None, false)
        } else if self.check_keyword(Keyword::Var)
            || self.check_keyword(Keyword::Let)
            || self.check_keyword(Keyword::Const)
        {
            // parse_var_decl ya consume el ';'
            let stmt = self.parse_var_decl()?;
            (Some(Box::new(stmt)), true)
        } else {
            let expr = self.parse_expression()?;
            (Some(Box::new(Statement::Expression(expr))), false)
        };
        
        if !init_has_semicolon {
            self.expect_symbol(Symbol::Semicolon)?;
        }
        
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
            span: self.span(),
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
            span: self.span(),
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
                        span: self.span(),
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
            span: self.span(),
        }))
    }

    fn parse_case_pattern(&mut self) -> ClsResult<CasePattern> {
        self.expect_symbol(Symbol::LParen)?;
        let expr = self.parse_expression()?;
        self.expect_symbol(Symbol::RParen)?;
        
        match expr {
            Expression::Literal(lit) => Ok(CasePattern::Literal(lit)),
            Expression::Identifier(name, _) => Ok(CasePattern::Identifier(name)),
            _ => Err(self.syntax_err("Pattern de case inválido".to_string())),
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
                span: self.span(),
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
            span: self.span(),
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
            span: self.span(),
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
        let span = self.span();
        self.expect_keyword(Keyword::Break)?;
        self.consume_symbol(Symbol::Semicolon);
        Ok(Statement::Break(span))
    }

    fn parse_continue(&mut self) -> ClsResult<Statement> {
        let span = self.span();
        self.expect_keyword(Keyword::Continue)?;
        self.consume_symbol(Symbol::Semicolon);
        Ok(Statement::Continue(span))
    }

    fn parse_class_decl(&mut self) -> ClsResult<Statement> {
        self.expect_keyword(Keyword::Class)?;
        let name = self.expect_identifier()?;
        let type_params = self.parse_type_params()?;
        
        // Herencia: `class Hijo: Padre` (principal), `extends Base` o `(Base)` (alias)
        let extends = if self.consume_operator(Operator::Colon) {
            let parent = self.expect_identifier()?;
            Some(parent)
        } else if self.check_keyword(Keyword::Extends) {
            self.advance();
            let parent = self.expect_identifier()?;
            Some(parent)
        } else if self.consume_symbol(Symbol::LParen) {
            let parent = self.expect_identifier()?;
            self.expect_symbol(Symbol::RParen)?;
            Some(parent)
        } else {
            None
        };
        
        // Implements (opcional): `class A implements I, J { ... }`
        let mut implements = Vec::new();
        if matches!(&self.current_token, Token::Identifier(w) if w == "implements") {
            self.advance();
            loop {
                implements.push(self.expect_identifier()?);
                if !self.consume_symbol(Symbol::Comma) {
                    break;
                }
            }
        }
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
            span: self.span(),
            type_params,
            visibility: Visibility::Default,
        }))
    }

    fn parse_class_member(&mut self) -> ClsResult<ClassMember> {
        // Detectar modificadores: public / private / protected / static / readonly
        let mut is_public = false;
        let mut is_private = false;
        let mut is_protected = false;
        let mut is_static = false;
        let mut is_readonly = false;
        loop {
            match self.current_token {
                Token::Keyword(Keyword::Public) => { self.advance(); is_public = true; }
                Token::Keyword(Keyword::Private) => { self.advance(); is_private = true; }
                Token::Keyword(Keyword::Protected) => { self.advance(); is_protected = true; }
                Token::Keyword(Keyword::Static) => { self.advance(); is_static = true; }
                Token::Keyword(Keyword::Readonly) => { self.advance(); is_readonly = true; }
                _ => break,
            }
        }

        // Verificar si es un método o una propiedad
        let is_method = self.check_keyword(Keyword::Function) || self.check_keyword(Keyword::Void);
        
        if is_method {
            let mut stmt = self.parse_function_decl()?;
            if let Statement::FunctionDecl(ref mut func) = stmt {
                if is_public { func.visibility = Visibility::Public; }
                if is_private { func.visibility = Visibility::Private; }
                if is_protected { func.visibility = Visibility::Protected; }
                if is_static { func.modifiers.push(FunctionModifier::Static); }
            }
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
            let mut stmt = self.parse_var_decl()?;
            if let Statement::VarDecl(ref mut var) = stmt {
                if is_public { var.visibility = Visibility::Public; }
                if is_private { var.visibility = Visibility::Private; }
                if is_protected { var.visibility = Visibility::Protected; }
                var.is_static = is_static;
                var.is_readonly = is_readonly;
            }
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
                span: self.span(),
            });
            
            self.consume_symbol(Symbol::Comma);
        }
        
        self.expect_symbol(Symbol::RBrace)?;
        self.consume_symbol(Symbol::Semicolon);
        
        Ok(Statement::StructureDecl(StructureDecl {
            name,
            fields,
            span: self.span(),
            visibility: Visibility::Default,
        }))
    }

    fn parse_interface_decl(&mut self) -> ClsResult<Statement> {
        self.expect_keyword(Keyword::Interface)?;
        let name = self.expect_identifier()?;
        let type_params = self.parse_type_params()?;

        // Compatibilidad: interface Name () { ... } (el () vacío del formato clsi)
        if self.check_symbol(Symbol::LParen) {
            self.advance();
            self.expect_symbol(Symbol::RParen)?;
        }
        
        self.expect_symbol(Symbol::LBrace)?;
        
        let mut fields = Vec::new();
        let mut signatures = Vec::new();
        while !self.check_symbol(Symbol::RBrace) && !self.is_eof() {
            self.skip_newlines();
            
            let member_name = self.expect_identifier()?;
            if self.consume_symbol(Symbol::LParen) {
                // Método: nombre(params) : Tipo
                let params = self.parse_parameters()?;
                self.expect_symbol(Symbol::RParen)?;
                let return_type = if self.consume_operator(Operator::Colon) {
                    Some(self.parse_type_annotation()?)
                } else {
                    None
                };
                signatures.push(SignatureDecl {
                    name: member_name,
                    params,
                    return_type,
                    span: self.span(),
                });
            } else {
                // Campo (shape): nombre : Tipo
                self.expect_operator(Operator::Colon)?;
                let type_ann = self.parse_type_annotation()?;
                fields.push(InterfaceField {
                    name: member_name,
                    type_ann,
                    span: self.span(),
                });
            }
            
            self.consume_symbol(Symbol::Comma);
        }
        
        self.expect_symbol(Symbol::RBrace)?;
        self.consume_symbol(Symbol::Semicolon);
        
        Ok(Statement::InterfaceDecl(InterfaceDecl {
            name,
            type_params,
            fields,
            signatures,
            span: self.span(),
            visibility: Visibility::Default,
        }))
    }

    /// Parsea parámetros de tipo `<T=Int, U>` (genéricos de tipo, compile-time).
    fn parse_type_params(&mut self) -> ClsResult<Vec<TypeParam>> {
        // `<T>` simple puede tokenizarse como CMX: reinterpretarlo
        if let Some(tag) = self.consume_cmx_simple_tag() {
            return Ok(vec![TypeParam {
                name: tag,
                default: None,
                span: self.span(),
            }]);
        }
        let mut params = Vec::new();
        if !matches!(self.current_token, Token::Operator(Operator::LessThan)) {
            return Ok(params);
        }
        self.advance(); // '<'
        loop {
            let name = self.expect_identifier()?;
            let default = if self.consume_operator(Operator::Equal) {
                Some(self.parse_type_annotation()?)
            } else {
                None
            };
            params.push(TypeParam {
                name,
                default,
                span: self.span(),
            });
            if !self.consume_symbol(Symbol::Comma) {
                break;
            }
        }
        if matches!(self.current_token, Token::Operator(Operator::GreaterThan)
            | Token::Operator(Operator::ShiftRight))
        {
            self.advance();
        } else {
            return Err(self.syntax_err("Esperaba '>' en parámetros de tipo".to_string()));
        }
        Ok(params)
    }

    /// `alias <Name>[<T=Int>] = <tipo>;` - alias de tipos (compile-time).
    fn parse_alias_decl(&mut self) -> ClsResult<Statement> {
        self.expect_keyword(Keyword::Alias)?;
        let name = self.expect_identifier()?;
        let type_params = self.parse_type_params()?;
        self.expect_operator(Operator::Equal)?;
        let type_ann = self.parse_type_annotation()?;
        self.consume_symbol(Symbol::Semicolon);
        Ok(Statement::TypeAlias(TypeAliasDecl {
            name,
            type_params,
            type_ann,
            span: self.span(),
            visibility: Visibility::Default,
        }))
    }

    /// `enum Nombre { Var1, Var2, Var3, };` - variantes constantes con identidad.
    fn parse_enum_decl(&mut self) -> ClsResult<Statement> {
        self.expect_keyword(Keyword::Enum)?;
        let name = self.expect_identifier()?;
        self.expect_symbol(Symbol::LBrace)?;

        let mut variants = Vec::new();
        while !self.check_symbol(Symbol::RBrace) && !self.is_eof() {
            self.skip_newlines();
            variants.push(self.expect_identifier()?);
            self.consume_symbol(Symbol::Comma);
        }
        self.expect_symbol(Symbol::RBrace)?;
        self.consume_symbol(Symbol::Semicolon);

        Ok(Statement::EnumDecl(EnumDecl {
            name,
            variants,
            span: self.span(),
            visibility: Visibility::Default,
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
            span: self.span(),
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
            span: self.span(),
        }))
    }

    fn parse_import(&mut self) -> ClsResult<Statement> {
        let import_span = self.span();  // span de 'import'
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
            span: import_span,
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
            span: self.span(),
        }))
    }

    fn parse_include(&mut self) -> ClsResult<Statement> {
        self.expect_keyword(Keyword::Include)?;
        let path = self.expect_string()?;
        self.consume_symbol(Symbol::Semicolon);
        
        Ok(Statement::Include(IncludeStatement {
            path,
            span: self.span(),
        }))
    }

    /// `extension "<lib>" [as <Tipo>] { function ...; structure ...; var ...; }` - FFI nativo.
    fn parse_extension_decl(&mut self) -> ClsResult<Statement> {
        self.expect_keyword(Keyword::Extension)?;
        let library = self.expect_string()?;
        let kind = if self.consume_keyword(Keyword::As) {
            let name = self.expect_identifier()?;
            ExtensionKind::from_name(&name)
        } else {
            ExtensionKind::C
        };
        self.expect_symbol(Symbol::LBrace)?;
        let mut declarations = Vec::new();
        self.skip_newlines();
        while !self.check_symbol(Symbol::RBrace) && !self.is_eof() {
            self.skip_newlines();
            let mut is_export = false;
            if self.consume_keyword(Keyword::Export) {
                is_export = true;
            }
            match self.current_token {
                Token::Keyword(Keyword::Function) => {
                    let stmt = self.parse_function_decl()?;
                    if let Statement::FunctionDecl(mut f) = stmt {
                        if is_export { f.visibility = Visibility::Export; }
                        declarations.push(NativeDecl::Function(f));
                    }
                }
                Token::Keyword(Keyword::Structure) => {
                    let stmt = self.parse_structure_decl()?;
                    if let Statement::StructureDecl(mut s) = stmt {
                        if is_export { s.visibility = Visibility::Export; }
                        declarations.push(NativeDecl::Structure(s));
                    }
                }
                Token::Keyword(Keyword::Var) | Token::Keyword(Keyword::Let) => {
                    let stmt = self.parse_var_decl()?;
                    if let Statement::VarDecl(mut v) = stmt {
                        if is_export { v.visibility = Visibility::Export; }
                        declarations.push(NativeDecl::Var(v));
                    }
                }
                _ => {
                    return Err(self.syntax_err(
                        "En extension solo se permiten declaraciones function, structure o var",
                    ))
                }
            }
            self.skip_newlines();
        }
        self.expect_symbol(Symbol::RBrace)?;
        self.consume_symbol(Symbol::Semicolon);
        Ok(Statement::Extension(ExtensionDecl {
            library,
            kind,
            declarations,
            span: self.span(),
        }))
    }

    /// Directiva multi-entorno: `when <cond> { declaraciones }`.
    fn parse_when(&mut self) -> ClsResult<Statement> {
        self.expect_keyword(Keyword::When)?;
        let cond = self.parse_target_cond()?;
        let block = self.parse_block()?;
        self.consume_symbol(Symbol::Semicolon);
        Ok(Statement::When(WhenBlock {
            branches: vec![WhenBranch { cond, block }],
            span: self.span(),
        }))
    }

    /// `default { declaraciones }` = rama que siempre matchea.
    fn parse_default_when(&mut self) -> ClsResult<Statement> {
        self.expect_keyword(Keyword::Default)?;
        let block = self.parse_block()?;
        self.consume_symbol(Symbol::Semicolon);
        Ok(Statement::When(WhenBlock {
            branches: vec![WhenBranch { cond: TargetCond::Any, block }],
            span: self.span(),
        }))
    }

    // ── Condiciones de target (precedencia: or < and < not < átomo) ─────────

    fn parse_target_cond(&mut self) -> ClsResult<TargetCond> {
        self.parse_target_or()
    }

    fn parse_target_or(&mut self) -> ClsResult<TargetCond> {
        let mut left = self.parse_target_and()?;
        while self.consume_keyword(Keyword::Or) {
            let right = self.parse_target_and()?;
            left = TargetCond::Or(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_target_and(&mut self) -> ClsResult<TargetCond> {
        let mut left = self.parse_target_not()?;
        while self.consume_keyword(Keyword::And) {
            let right = self.parse_target_not()?;
            left = TargetCond::And(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_target_not(&mut self) -> ClsResult<TargetCond> {
        if self.consume_keyword(Keyword::Not) || self.consume_operator(Operator::Not) {
            let inner = self.parse_target_not()?;
            return Ok(TargetCond::Not(Box::new(inner)));
        }
        self.parse_target_atom()
    }

    fn parse_target_atom(&mut self) -> ClsResult<TargetCond> {
        if self.consume_symbol(Symbol::LParen) {
            let inner = self.parse_target_cond()?;
            self.expect_symbol(Symbol::RParen)?;
            return Ok(inner);
        }
        // Prefijos: os: / arch: / abi: / platform: / target:
        if let Token::Identifier(name) = &self.current_token {
            let field = name.clone();
            if matches!(field.as_str(), "os" | "arch" | "abi" | "platform" | "target") {
                self.advance();
                if !self.consume_operator(Operator::Colon) {
                    return Err(self.syntax_err(format!("Esperaba ':' tras '{}' en la condición de 'when'", field)));
                }
                if field == "target" {
                    return Ok(TargetCond::Target(self.expect_string()?));
                }
                let v = self.expect_target_name()?;
                return Ok(match field.as_str() {
                    "os" => TargetCond::Os(v),
                    "arch" => TargetCond::Arch(v),
                    "abi" => TargetCond::Abi(v),
                    "platform" => TargetCond::Platform(v),
                    _ => TargetCond::Any,
                });
            }
        }
        // Nombre simple: SO conocido -> Os; tripla (contiene '-') -> Target; arch -> Arch.
        if let Token::Identifier(name) = &self.current_token {
            let name = name.clone();
            self.advance();
            let lower = name.to_lowercase();
            const OSES: &[&str] = &["windows", "linux", "macos", "none", "bare-metal", "freebsd"];
            const ARCHES: &[&str] = &["x86_64", "arm64", "aarch64", "arm", "riscv32", "riscv64", "avr"];
            if OSES.contains(&lower.as_str()) {
                return Ok(TargetCond::Os(lower));
            }
            if name.contains('-') {
                if lower == "cls-arch" {
                    return Ok(TargetCond::Arch("cls-arch".to_string()));
                }
                return Ok(TargetCond::Target(name));
            }
            if ARCHES.contains(&lower.as_str()) {
                return Ok(TargetCond::Arch(if lower == "aarch64" { "arm64".to_string() } else { lower }));
            }
            return Err(self.syntax_err(format!(
                "Condición de 'when' inválida: '{}' (usa os:, arch:, abi:, platform:, target: o una tripla)",
                name
            )));
        }
        Err(self.syntax_err("Condición de 'when' inválida"))
    }

    /// Lee un identificador que puede contener guiones (`cls-arch`, `bare-metal`).
    fn expect_target_name(&mut self) -> ClsResult<String> {
        let mut name = self.expect_identifier()?;
        while self.consume_operator(Operator::Minus) {
            let part = self.expect_identifier()?;
            name.push('-');
            name.push_str(&part);
        }
        Ok(name)
    }

    fn parse_visibility_modifier(&mut self) -> ClsResult<Statement> {
        // public/private/export/static func/var/etc
        let visibility = match self.current_token {
            Token::Keyword(Keyword::Public) => Visibility::Public,
            Token::Keyword(Keyword::Private) => Visibility::Private,
            Token::Keyword(Keyword::Export) => Visibility::Export,
            _ => Visibility::Default,
        };
        
        self.advance(); // consume modifier
        
        // Luego parsear la declaración
        let mut stmt = self.parse_statement()?;
        
        // Aplicar visibilidad
        match &mut stmt {
            Statement::FunctionDecl(ref mut f) => f.visibility = visibility,
            Statement::VarDecl(ref mut v) | Statement::ConstDecl(ref mut v) => v.visibility = visibility,
            Statement::ClassDecl(ref mut c) => c.visibility = visibility,
            Statement::EnumDecl(ref mut e) => e.visibility = visibility,
            Statement::StructureDecl(ref mut s) => s.visibility = visibility,
            Statement::InterfaceDecl(ref mut i) => i.visibility = visibility,
            Statement::TypeAlias(ref mut t) => t.visibility = visibility,
            _ => {}
        }
        
        Ok(stmt)
    }

    fn parse_cmx(&mut self) -> ClsResult<Statement> {
        let element = self.parse_cmx_element()?;
        Ok(Statement::Cmx(element))
    }

    fn parse_cmx_element(&mut self) -> ClsResult<CmxElement> {
        let (tag_name, is_self_closing) = match &self.current_token {
            Token::Cmx(CmxToken::OpenTag { name, is_self_closing }) => (name.clone(), *is_self_closing),
            _ => return Err(self.syntax_err("Se esperaba OpenTag CMX")),
        };
        let tag_span = self.span();
        self.advance();

        let mut attributes = Vec::new();
        loop {
            let is_attr_str = matches!(&self.current_token, Token::Cmx(CmxToken::AttrString { .. }));
            let is_attr_expr = matches!(&self.current_token, Token::Cmx(CmxToken::AttrExpr { .. }));
            if is_attr_str {
                if let Token::Cmx(CmxToken::AttrString { name, value }) = &self.current_token {
                    attributes.push(CmxAttribute { name: name.clone(), value: Some(CmxAttributeValue::String(value.clone())), span: self.span() });
                }
                self.advance();
            } else if is_attr_expr {
                if let Token::Cmx(CmxToken::AttrExpr { name }) = &self.current_token {
                    let expr_name = name.clone();
                    self.advance();
                    let expr = self.parse_expression()?;
                    attributes.push(CmxAttribute { name: expr_name, value: Some(CmxAttributeValue::Expression(Box::new(expr))), span: self.span() });
                }
            } else { break; }
        }

        if is_self_closing { return Ok(CmxElement { tag: tag_name, attributes, children: vec![], span: tag_span }); }

        let mut children = Vec::new();
        loop {
            let is_text = matches!(&self.current_token, Token::Cmx(CmxToken::Text { .. }));
            let is_open = matches!(&self.current_token, Token::Cmx(CmxToken::OpenTag { .. }));
            let is_close = matches!(&self.current_token, Token::Cmx(CmxToken::CloseTag { .. }));
            
            if is_text {
                if let Token::Cmx(CmxToken::Text { content }) = &self.current_token {
                    let text = content.clone();
                    self.advance();
                    children.push(CmxChild::Text(text));
                }
            } else if is_open {
                if let Token::Cmx(CmxToken::OpenTag { .. }) = &self.current_token {
                    let el = self.parse_cmx_element()?;
                    children.push(CmxChild::Element(Box::new(el)));
                }
            } else if matches!(&self.current_token, Token::Symbol(Symbol::LBrace)) {
                // Interpolación en el cuerpo: `{expr}` -> CmxChild::Expression.
                self.advance();
                let expr = self.parse_expression()?;
                self.expect_symbol(Symbol::RBrace)?;
                children.push(CmxChild::Expression(Box::new(expr)));
            } else if is_close {
                self.advance(); break;
            } else { break; }
        }
        Ok(CmxElement { tag: tag_name, attributes, children, span: tag_span })
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
            span: self.span(),
        })
    }

    // ========================================================================
    // PARSER DE EXPRESIONES (recursive descent con precedencia)
    // ========================================================================

    fn parse_expression(&mut self) -> ClsResult<Expression> {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> ClsResult<Expression> {
        // await expresion
        if self.check_keyword(Keyword::Await) {
            let span = self.span();
            self.advance();
            let expr = self.parse_assignment()?;
            return Ok(Expression::Await(Box::new(expr), span));
        }

        let expr = self.parse_conditional()?;
        
        if let Some(op) = self.check_assignment_operator() {
            self.advance();
            let value = self.parse_assignment()?;
            // Span mergeado del target y el valor: `self.span()` (token actual)
            // colisiona con el span del RHS y contamina el type map del emisor.
            let assign_span = expr_span(&expr).merge(&expr_span(&value));
            return Ok(Expression::Assignment(AssignmentExpr {
                target: Box::new(expr),
                op,
                value: Box::new(value),
                span: assign_span,
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
            Token::Operator(Operator::PercentEqual) => Some(Operator::PercentEqual),
            _ => None,
        }
    }

    fn parse_conditional(&mut self) -> ClsResult<Expression> {
        self.parse_logical_or()
    }

    fn parse_logical_or(&mut self) -> ClsResult<Expression> {
        let mut expr = self.parse_logical_and()?;
        
        while self.check_operator(Operator::Or) {
            self.advance();
            let right = self.parse_logical_and()?;
            let bin_span = expr_span(&expr).merge(&expr_span(&right));
            expr = Expression::Binary(BinaryExpr {
                left: Box::new(expr),
                op: Operator::Or,
                right: Box::new(right),
                span: bin_span,
            });
        }
        
        Ok(expr)
    }

    fn parse_logical_and(&mut self) -> ClsResult<Expression> {
        let mut expr = self.parse_equality()?;
        
        while self.check_operator(Operator::And) {
            self.advance();
            let right = self.parse_equality()?;
            let bin_span = expr_span(&expr).merge(&expr_span(&right));
            expr = Expression::Binary(BinaryExpr {
                left: Box::new(expr),
                op: Operator::And,
                right: Box::new(right),
                span: bin_span,
            });
        }
        
        Ok(expr)
    }

    fn parse_equality(&mut self) -> ClsResult<Expression> {
        let mut expr = self.parse_xor()?;
        
        loop {
            // '==' y '!='
            if let Token::Operator(op) = &self.current_token {
                match op {
                    Operator::StrictEqual | Operator::NotEqual => {
                        let op = op.clone();
                        self.advance();
                        let right = self.parse_xor()?;
                        // Span mergeado de ambos operandos: `self.span()` (token
                        // actual) colisiona con el span del operando derecho y
                        // contamina el type map del emisor.
                        let bin_span = expr_span(&expr).merge(&expr_span(&right));
                        expr = Expression::Binary(BinaryExpr {
                            left: Box::new(expr),
                            op,
                            right: Box::new(right),
                            span: bin_span,
                        });
                        continue;
                    }
                    _ => {}
                }
            }
            // 'is' keyword: obj is Clase
            if self.check_keyword(Keyword::Is) {
                self.advance();
                let right = self.parse_xor()?;
                let bin_span = expr_span(&expr).merge(&expr_span(&right));
                expr = Expression::Binary(BinaryExpr {
                    left: Box::new(expr),
                    op: Operator::Is,
                    right: Box::new(right),
                    span: bin_span,
                });
                continue;
            }
            // 'in' keyword: x in coleccion
            if self.check_keyword(Keyword::In) {
                self.advance();
                let right = self.parse_xor()?;
                let bin_span = expr_span(&expr).merge(&expr_span(&right));
                expr = Expression::Binary(BinaryExpr {
                    left: Box::new(expr),
                    op: Operator::In,
                    right: Box::new(right),
                    span: bin_span,
                });
                continue;
            }
            break;
        }
        
        Ok(expr)
    }

    fn parse_xor(&mut self) -> ClsResult<Expression> {
        let mut expr = self.parse_comparison()?;
        
        while let Token::Operator(op) = &self.current_token {
            let op = match op {
                Operator::Caret => op.clone(),
                _ => break,
            };
            self.advance();
            let right = self.parse_comparison()?;
            let bin_span = expr_span(&expr).merge(&expr_span(&right));
            expr = Expression::Binary(BinaryExpr {
                left: Box::new(expr),
                op,
                right: Box::new(right),
                span: bin_span,
            });
        }
        
        Ok(expr)
    }

    fn parse_comparison(&mut self) -> ClsResult<Expression> {
        let mut expr = self.parse_shift()?;
        
        while let Token::Operator(op) = &self.current_token {
            let op = match op {
                Operator::LessThan
                | Operator::LessEqual
                | Operator::GreaterThan
                | Operator::GreaterEqual => op.clone(),
                _ => break,
            };
            self.advance();
            let right = self.parse_shift()?;
            let bin_span = expr_span(&expr).merge(&expr_span(&right));
            expr = Expression::Binary(BinaryExpr {
                left: Box::new(expr),
                op,
                right: Box::new(right),
                span: bin_span,
            });
        }
        
        Ok(expr)
    }

    fn parse_shift(&mut self) -> ClsResult<Expression> {
        let mut expr = self.parse_term()?;
        
        while let Token::Operator(op) = &self.current_token {
            let op = match op {
                Operator::ShiftLeft | Operator::ShiftRight => op.clone(),
                _ => break,
            };
            self.advance();
            let right = self.parse_term()?;
            let bin_span = expr_span(&expr).merge(&expr_span(&right));
            expr = Expression::Binary(BinaryExpr {
                left: Box::new(expr),
                op,
                right: Box::new(right),
                span: bin_span,
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
            let bin_span = expr_span(&expr).merge(&expr_span(&right));
            expr = Expression::Binary(BinaryExpr {
                left: Box::new(expr),
                op,
                right: Box::new(right),
                span: bin_span,
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
            let bin_span = expr_span(&expr).merge(&expr_span(&right));
            expr = Expression::Binary(BinaryExpr {
                left: Box::new(expr),
                op,
                right: Box::new(right),
                span: bin_span,
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
            // Span del operador: el unario usa el span del operador (no el del
            // token siguiente) para que NO colisione con el span del operando
            // (el typeck registra ambos tipos por span; si colisionan, el tipo
            // del unario pisa al del operando y el dispatch de magic methods
            // no encuentra la clase).
            let op_span = self.current_span.clone();
            self.advance();
            let operand = self.parse_unary()?;
            return Ok(Expression::Unary(UnaryExpr {
                op,
                operand: Box::new(operand),
                span: op_span,
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
                        span: self.span(),
                    });
                }
                // Acceso a miembro
                Token::Symbol(Symbol::Dot) => {
                    self.advance();
                    let member = self.expect_identifier()?;
                    expr = Expression::MemberAccess(MemberAccessExpr {
                        object: Box::new(expr),
                        member,
                        span: self.span(),
                    });
                }
                // Namespace access
                Token::Operator(Operator::ColonColon) => {
                    self.advance();
                    let member = self.expect_identifier()?;
                    expr = Expression::NamespaceAccess(
                        match expr {
                            Expression::Identifier(name, _) => name,
                            _ => return Err(self.syntax_err("Esperaba identificador")),
                        },
                        member,
                        self.span(),
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
                        span: self.span(),
                    });
                }
                // Postfix ++ y --
                Token::Operator(Operator::PlusPlus) => {
                    self.advance();
                    expr = Expression::Unary(UnaryExpr {
                        op: UnaryOp::PostInc,
                        operand: Box::new(expr),
                        span: self.span(),
                    });
                }
                Token::Operator(Operator::MinusMinus) => {
                    self.advance();
                    expr = Expression::Unary(UnaryExpr {
                        op: UnaryOp::PostDec,
                        operand: Box::new(expr),
                        span: self.span(),
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
                    span: self.span(),
                }))
            }
            Token::FloatLiteral(v) => {
                let v = *v;
                self.advance();
                Ok(Expression::Literal(Literal {
                    kind: LiteralKind::Float(v),
                    span: self.span(),
                }))
            }
            Token::StringLiteral(v) => {
                let v = v.clone();
                let lit_span = self.span();
                self.advance();
                if v.contains('$') {
                    self.parse_string_interpolation(&v, &lit_span)
                } else {
                    Ok(Expression::Literal(Literal {
                        kind: LiteralKind::String(v),
                        span: self.span(),
                    }))
                }
            }
            Token::BoolLiteral(v) => {
                let v = *v;
                self.advance();
                Ok(Expression::Literal(Literal {
                    kind: LiteralKind::Bool(v),
                    span: self.span(),
                }))
            }
            Token::CharLiteral(v) => {
                let v = *v;
                self.advance();
                Ok(Expression::Literal(Literal {
                    kind: LiteralKind::Char(v),
                    span: self.span(),
                }))
            }
            Token::Identifier(name) => {
                let name = name.clone();
                self.advance();

                // `null` como literal (antes era un Identifier no resuelto).
                if name == "null" {
                    return Ok(Expression::Literal(Literal {
                        kind: LiteralKind::Null,
                        span: self.span(),
                    }));
                }

                // Verificar si es una función flecha
                if self.check_symbol(Symbol::LParen) {
                    // Podría ser una llamada o una función flecha
                    // Por ahora devolvemos el identificador y que parse_call lo maneje
                    return Ok(Expression::Identifier(name, self.span()));
                }
                
                Ok(Expression::Identifier(name, self.span()))
            }
            Token::Keyword(Keyword::Me) => {
                self.advance();
                Ok(Expression::Identifier("me".to_string(), self.span()))
            }
            Token::Keyword(Keyword::Super) => {
                self.advance();
                Ok(Expression::Identifier("super".to_string(), self.span()))
            }
            Token::Keyword(Keyword::True) => {
                self.advance();
                Ok(Expression::Literal(Literal {
                    kind: LiteralKind::Bool(true),
                    span: self.span(),
                }))
            }
            Token::Keyword(Keyword::False) => {
                self.advance();
                Ok(Expression::Literal(Literal {
                    kind: LiteralKind::Bool(false),
                    span: self.span(),
                }))
            }
            Token::Symbol(Symbol::LParen) => {
                self.advance();
                let is_arrow = self.is_arrow_function()
                    || matches!(&self.current_token, Token::Symbol(Symbol::RParen))
                        && self.tokens.clone().next()
                            .map_or(false, |t| matches!(&t.token, Token::Operator(Operator::Arrow)));
                if is_arrow { return self.parse_arrow_function(); }
                
                // Tupla vacía: ()
                if self.consume_symbol(Symbol::RParen) {
                    return Ok(Expression::Tuple(TupleExpr {
                        elements: Vec::new(),
                        span: self.span(),
                    }));
                }
                // Paréntesis o tupla: (a) | (a, b, c)
                let first = self.parse_expression()?;
                if self.consume_symbol(Symbol::Comma) {
                    let mut elements = vec![first];
                    loop {
                        elements.push(self.parse_expression()?);
                        if !self.consume_symbol(Symbol::Comma) {
                            break;
                        }
                    }
                    self.expect_symbol(Symbol::RParen)?;
                    Ok(Expression::Tuple(TupleExpr {
                        elements,
                        span: self.span(),
                    }))
                } else {
                    self.expect_symbol(Symbol::RParen)?;
                    Ok(Expression::Parenthesized(Box::new(first), self.span()))
                }
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
                    span: self.span(),
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
                            _ => return Err(self.syntax_err("Key de record inválida")),
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
                    span: self.span(),
                }))
            }
            // Ternario: if (cond) then (a) else (b)
            Token::Keyword(Keyword::If) => {
                self.advance();
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
                Ok(Expression::Conditional(ConditionalExpr {
                    condition: Box::new(cond),
                    then_expr: Box::new(then_expr),
                    else_expr: Box::new(else_expr),
                    span: self.span(),
                }))
            }
            // CMX (JSX nativo)
            Token::Cmx(CmxToken::OpenTag { .. }) => {
                let element = self.parse_cmx_element()?;
                Ok(Expression::Cmx(element))
            }
            _ => Err(self.syntax_err(format!(
                "Token inesperado: {}",
                self.current_token
            )))
        }
    }

    fn is_arrow_function(&self) -> bool {
        // Buscar -> mirando adelante sin consumir tokens
        // Clonamos el iterador para explorar
        let mut cursor = self.tokens.clone();
        let mut depth = 1;
        while let Some(next) = cursor.peek() {
            match &next.token {
                Token::Symbol(Symbol::LParen) => depth += 1,
                Token::Symbol(Symbol::RParen) => {
                    depth -= 1;
                    if depth == 0 {
                        // Despues del ) cerrado, ver si viene ->
                        cursor.next();
                        if let Some(after) = cursor.peek() {
                            return matches!(&after.token, Token::Operator(Operator::Arrow));
                        }
                        break;
                    }
                }
                _ => {}
            }
            cursor.next();
        }
        false
    }

    /// Parsea una string con interpolación: "Hello, $name!" o "${a + b}"
    fn parse_string_interpolation(&self, s: &str, lit_span: &Span) -> ClsResult<Expression> {

        let chars: Vec<char> = s.chars().collect();
        let mut parts: Vec<InterpolationPart> = Vec::new();
        let mut i = 0;

        while i < chars.len() {
            if chars[i] == '$' && i + 1 < chars.len() {
                if chars[i + 1] == '{' {
                    // ${expr} - parsear expresión
                    let mut depth = 1;
                    let mut expr_str = String::new();
                    let mut j = i + 2;
                    while j < chars.len() && depth > 0 {
                        match chars[j] {
                            '{' => depth += 1,
                            '}' => depth -= 1,
                            _ => {}
                        }
                        if depth > 0 {
                            expr_str.push(chars[j]);
                        }
                        j += 1;
                    }
                    if depth != 0 {
                        return Err(self.syntax_err(
                            "${} sin cerrar",
                        ));
                    }
                    // Parsear la expresión interna
                    let mut expr = parse_expr_from_str(&expr_str)?;
                    // Reubicar los spans del expr (parseado en línea 1, cols
                    // relativas) a la posición real dentro del literal, para que
                    // cada interpolación tenga spans únicos en el type map.
                    let dl = lit_span.start_line.saturating_sub(1);
                    let dc = lit_span.start_col as i64 + i as i64 + 2;
                    crate::frontend::span_shift::shift_expr_xy(&mut expr, dl, dc);
                    parts.push(InterpolationPart::Expr(expr));
                    i = j; // saltar }
                } else if chars[i + 1].is_alphabetic() || chars[i + 1] == '_' {
                    // $var - variable lookup
                    let mut var_name = String::new();
                    let mut j = i + 1;
                    while j < chars.len() && (chars[j].is_alphanumeric() || chars[j] == '_') {
                        var_name.push(chars[j]);
                        j += 1;
                    }
                    // Span único por literal + offset de columna (el span NO debe
                    // colisionar con otra interpolación: el typeck los usa como clave).
                    let start_col = lit_span.start_col + i as u32 + 1;
                    let end_col = lit_span.start_col + j as u32 + 1;
                    parts.push(InterpolationPart::Expr(
                        Expression::Identifier(var_name, Span::new(lit_span.start_line, start_col, lit_span.start_line, end_col))
                    ));
                    i = j;
                } else {
                    // $ literal, no es interpolación
                    parts.push(InterpolationPart::Text("$".to_string()));
                    i += 1;
                }
            } else {
                // Texto normal
                let mut text = String::new();
                while i < chars.len() && chars[i] != '$' {
                    text.push(chars[i]);
                    i += 1;
                }
                parts.push(InterpolationPart::Text(text));
            }
        }

        Ok(Expression::StringInterpolation(StringInterpolation {
            parts,
            span: Span::new(0, 0, 0, 0),
        }))
    }

    fn parse_arrow_function(&mut self) -> ClsResult<Expression> {
        // Ya se consumio el ( inicial
        let params = self.parse_parameters()?;
        self.expect_symbol(Symbol::RParen)?;
        self.expect_operator(Operator::Arrow)?;
        // Determinar si hay tipo de retorno: si despues de -> viene un
        // identificador seguido de { , es return_type { body }
        // si viene cualquier otra cosa, es el body directamente
        let has_return_type = matches!(&self.current_token, Token::Identifier(_))
            && self.tokens.peek().map(|t| matches!(&t.token, Token::Symbol(Symbol::LBrace))).unwrap_or(false);
        let return_type = if has_return_type {
            Some(self.parse_type_annotation()?)
        } else {
            None
        };
        let body = if matches!(self.current_token, Token::Symbol(Symbol::LBrace)) {
            self.parse_block()?
        } else {
            let expr = self.parse_expression()?;
            self.consume_symbol(Symbol::Semicolon);
            // (x) -> expr  equivale a  (x) -> { return expr; }
            Block {
                statements: vec![Statement::Return(Some(expr))],
                span: self.span(),
            }
        };
        Ok(Expression::ArrowFunction(ArrowFunctionExpr {
            params,
            return_type,
            body: Box::new(body),
            span: self.span(),
        }))
    }

    // ==============================================================
    // HELPERS
    // ──────────────────────────────────────────────────────────────────────────────────────

    fn advance(&mut self) {
        if let Some(next) = self.tokens.next() {
            self.current_span = next.span;
            self.current_token = next.token;
        } else {
            self.current_span = self.span();
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

    /// Fábrica centralizada: error de sintaxis con span del token actual.
    fn syntax_err(&self, msg: impl Into<String>) -> ClsError {
        let s = self.span();
        ClsError::syntax_at(&msg.into(), &s)
    }

    fn expect_symbol(&mut self, symbol: Symbol) -> ClsResult<()> {
        if self.consume_symbol(symbol) {
            Ok(())
        } else {
            Err(self.syntax_err(format!(
                "Esperaba símbolo {}, encontró {}",
                symbol, self.current_token
            )))
        }
    }

    fn expect_keyword(&mut self, keyword: Keyword) -> ClsResult<()> {
        if self.consume_keyword(keyword) {
            Ok(())
        } else {
            Err(self.syntax_err(format!(
                "Esperaba keyword {}, encontró {}",
                keyword, self.current_token
            )))
        }
    }

    fn expect_operator(&mut self, op: Operator) -> ClsResult<()> {
        if self.consume_operator(op) {
            Ok(())
        } else {
            Err(self.syntax_err(format!(
                "Esperaba operador {}, encontró {}",
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
            _ => Err(self.syntax_err(format!(
                "Esperaba identificador, encontró {}",
                self.current_token
            )))
        }
    }

    fn expect_string(&mut self) -> ClsResult<String> {
        match &self.current_token {
            Token::StringLiteral(s) => {
                let s = s.clone();
                self.advance();
                Ok(s)
            }
            _ => Err(self.syntax_err(format!(
                "Esperaba string, encontró {}",
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

/// Parsea una expresión desde un string (para interpolación ${expr})
fn parse_expr_from_str(expr_str: &str) -> ClsResult<Expression> {
    let source = format!("({})", expr_str);
    let mut lexer = crate::frontend::Lexer::new(&source);
    let tokens = lexer.tokenize().map_err(|e| {
        ClsError::SyntaxError(format!("Error en interpolación: {}", e))
    })?;
    let mut parser = Parser::new(tokens);
    let expr = parser.parse_expression().map_err(|e| {
        ClsError::SyntaxError(format!("Error en expresión interpolada: {}", e))
    })?;
    Ok(expr)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::ast::Visibility;
    use crate::frontend::lexer::Lexer;

    fn parse(source: &str) -> Module {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        parser.parse().unwrap()
    }

    fn parse_expr(source: &str) -> Expression {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        parser.parse_expression().unwrap()
    }

    #[test]
    fn test_parse_empty() { let m = parse(""); assert!(m.statements.is_empty()); }

    #[test]
    fn test_parse_var_decl() {
        let m = parse("var x = 42");
        assert!(matches!(&m.statements[0], Statement::VarDecl(v) if v.name == "x"));
    }

    #[test]
    fn test_parse_function_decl() {
        let m = parse("function add(a: int, b: int) -> int { return a + b; };");
        assert!(matches!(&m.statements[0], Statement::FunctionDecl(f) if f.name == "add" && f.params.len() == 2));
    }

    #[test]
    fn test_parse_if() { let m = parse("if (true) { var x = 1; };"); assert!(matches!(&m.statements[0], Statement::If(_))); }

    #[test]
    fn test_parse_while() { let m = parse("while (true) { break; };"); assert!(matches!(&m.statements[0], Statement::While(_))); }

    #[test]
    fn test_parse_return() {
        let m = parse("function f() -> int { return 42; };");
        assert!(matches!(&m.statements[0], Statement::FunctionDecl(f) if matches!(&f.body.statements[0], Statement::Return(_))));
    }

    #[test]
    fn test_parse_import() { let m = parse(r#"import "math" as math;"#); assert!(matches!(&m.statements[0], Statement::Import(_))); }

    #[test]
    fn test_parse_from_import() { let m = parse(r#"from "math" import abs, PI;"#); assert!(matches!(&m.statements[0], Statement::FromImport(_))); }

    #[test]
    fn test_parse_int() { assert!(matches!(parse_expr("42"), Expression::Literal(l) if matches!(l.kind, LiteralKind::Int(42)))); }

    #[test]
    fn test_parse_string() { assert!(matches!(parse_expr(r#""hello""#), Expression::Literal(l) if matches!(l.kind, LiteralKind::String(ref s) if s == "hello"))); }

    #[test]
    fn test_parse_binary() { assert!(matches!(parse_expr("1+2"), Expression::Binary(_))); }

    #[test]
    fn test_parse_call() { assert!(matches!(parse_expr("f(1)"), Expression::Call(c) if c.args.len() == 1)); }

    #[test]
    fn test_parse_record() { assert!(matches!(parse_expr(r#"{"a":1}"#), Expression::Record(_))); }

    #[test]
    fn test_parse_array() { assert!(matches!(parse_expr("[1,2,3]"), Expression::Array(a) if a.elements.len() == 3)); }

    #[test]
    fn test_parse_arrow() { assert!(matches!(parse_expr("(x)->x*2"), Expression::ArrowFunction(_))); }

    #[test]
    fn test_parse_member() { assert!(matches!(parse_expr("a.b"), Expression::MemberAccess(_))); }

    #[test]
    fn test_parse_index() { assert!(matches!(parse_expr("a[0]"), Expression::Index(_))); }

    #[test]
    fn test_parse_unary() { assert!(matches!(parse_expr("-5"), Expression::Unary(u) if u.op == UnaryOp::Negate)); }

    #[test]
    fn test_parse_interpolation() { assert!(matches!(parse_expr("`a $b`"), Expression::StringInterpolation(_))); }

    #[test]
    fn test_parse_assignment() { assert!(matches!(parse_expr("x=10"), Expression::Assignment(_))); }

    #[test]
    fn test_parse_export() {
        let m = parse("export function f() -> int {};");
        assert!(matches!(&m.statements[0], Statement::FunctionDecl(f) if f.visibility == Visibility::Export));
    }

    #[test]
    fn test_parse_error_syntax() {
        let mut l = Lexer::new("var x = ;");
        let t = l.tokenize().unwrap();
        assert!(Parser::new(t).parse().is_err());
    }
}

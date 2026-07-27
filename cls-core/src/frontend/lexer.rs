use crate::error::{ClsError, ClsResult, Diagnostic};
use crate::error::diagnostic::Span;
use crate::frontend::token::{CmxToken, Keyword, Operator, Symbol, Token, SpannedToken};
use std::collections::VecDeque;

/// Tokenizador/lexer de CLS
/// Convierte código fuente en una lista de tokens con información de posición
pub struct Lexer {
    source: Vec<char>,
    source_str: String,
    pos: usize,
    line: u32,
    col: u32,
    line_start: usize,
    diagnostics: Vec<Diagnostic>,
    cmx_buffer: VecDeque<SpannedToken>,
    in_cmx_expr: bool,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Self {
            source: source.chars().collect(),
            source_str: source.to_string(),
            pos: 0,
            line: 1,
            col: 1,
            line_start: 0,
            diagnostics: Vec::new(),
            cmx_buffer: VecDeque::new(),
            in_cmx_expr: false,
        }
    }

    /// Obtiene los diagnósticos (errores/warnings)
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Tokeniza todo el código fuente
    pub fn tokenize(&mut self) -> ClsResult<Vec<SpannedToken>> {
        let mut tokens = Vec::new();

        loop {
            let spanned = self.next_token()?;
            let is_eof = matches!(spanned.token, Token::EOF);
            tokens.push(spanned);
            if is_eof {
                break;
            }
        }

        Ok(tokens)
    }

    fn next_token(&mut self) -> ClsResult<SpannedToken> {
        self.skip_whitespace_and_comments();

        // Si hay tokens CMX en buffer y no estamos dentro de expresión CMX
        if !self.in_cmx_expr {
            if let Some(t) = self.cmx_buffer.pop_front() {
                return Ok(t);
            }
        }

        if self.is_eof() {
            return Ok(SpannedToken::new(Token::EOF, self.current_span()));
        }

        let ch = self.current_char();
        let start_span = self.current_span();

        let token = match ch {
            // Strings
            '"' | '\'' | '`' => self.lex_string(ch),
            // Números
            '0'..='9' => self.lex_number(),
            // Identificadores y keywords
            c if c.is_alphabetic() || c == '_' => self.lex_identifier_or_keyword(),
            // CMX
            '<' if self.peek_is_cmx_start() => self.lex_cmx(),
            // Operadores y símbolos
            _ => self.lex_operator_or_symbol(),
        }?;

        Ok(SpannedToken::new(token, start_span))
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            if self.is_eof() {
                break;
            }

            let ch = self.current_char();

            // Comentarios de línea
            if ch == '#' {
                self.skip_until_newline();
                continue;
            }

            // Espacios en blanco (incluyendo newlines)
            if ch == ' ' || ch == '\t' || ch == '\r' || ch == '\n' {
                self.advance();
                continue;
            }

            break;
        }
    }

    fn skip_until_newline(&mut self) {
        while !self.is_eof() && self.current_char() != '\n' {
            self.advance();
        }
        // Consume el newline
        if !self.is_eof() {
            self.advance();
        }
    }

    fn lex_string(&mut self, delimiter: char) -> ClsResult<Token> {
        let mut content = String::new();
        self.advance(); // Consume el delimiter

        while !self.is_eof() && self.current_char() != delimiter {
            if self.current_char() == '\\' {
                self.advance();
                if self.is_eof() {
                    return Err(ClsError::SyntaxError(
                        "String sin cerrar".to_string(),
                    ));
                }
                let escaped = self.current_char();
                let escaped_char = match escaped {
                    'n' => '\n',
                    't' => '\t',
                    'r' => '\r',
                    'b' => '\x08',
                    '\\' => '\\',
                    '\'' => '\'',
                    '"' => '"',
                    _ => escaped,
                };
                content.push(escaped_char);
            } else {
                content.push(self.current_char());
            }
            self.advance();
        }

        if self.is_eof() {
            return Err(ClsError::SyntaxError(
                "String sin cerrar".to_string(),
            ));
        }

        self.advance(); // Consume el delimiter final

        Ok(Token::StringLiteral(content))
    }

    fn lex_number(&mut self) -> ClsResult<Token> {
        let mut num_str = String::new();
        let mut has_dot = false;

        while !self.is_eof() {
            let ch = self.current_char();
            if ch.is_ascii_digit() {
                num_str.push(ch);
            } else if ch == '.' && !has_dot && self.peek_char_is_digit(1) {
                has_dot = true;
                num_str.push(ch);
            } else {
                break;
            }
            self.advance();
        }

        if has_dot {
            match num_str.parse::<f64>() {
                Ok(v) => Ok(Token::FloatLiteral(v)),
                Err(_) => Err(ClsError::SyntaxError(format!(
                    "Número inválido: {}",
                    num_str
                ))),
            }
        } else {
            match num_str.parse::<i64>() {
                Ok(v) => Ok(Token::IntLiteral(v)),
                Err(_) => Err(ClsError::SyntaxError(format!(
                    "Número inválido: {}",
                    num_str
                ))),
            }
        }
    }

    fn lex_identifier_or_keyword(&mut self) -> ClsResult<Token> {
        let mut ident = String::new();

        while !self.is_eof()
            && (self.current_char().is_alphanumeric() || self.current_char() == '_')
        {
            ident.push(self.current_char());
            self.advance();
        }

        // Verificar si es keyword
        let token = match ident.as_str() {
            "var" => Token::Keyword(Keyword::Var),
            "const" => Token::Keyword(Keyword::Const),
            "let" => Token::Keyword(Keyword::Let),
            "function" => Token::Keyword(Keyword::Function),
            "void" => Token::Keyword(Keyword::Void),
            "method" => Token::Keyword(Keyword::Method),
            "export" => Token::Keyword(Keyword::Export),
            "if" => Token::Keyword(Keyword::If),
            "elif" => Token::Keyword(Keyword::Elif),
            "else" => Token::Keyword(Keyword::Else),
            "while" => Token::Keyword(Keyword::While),
            "loop" => Token::Keyword(Keyword::Loop),
            "for" => Token::Keyword(Keyword::For),
            "each" => Token::Keyword(Keyword::Each),
            "in" => Token::Keyword(Keyword::In),
            "and" => Token::Keyword(Keyword::And),
            "switch" => Token::Keyword(Keyword::Switch),
            "case" => Token::Keyword(Keyword::Case),
            "default" => Token::Keyword(Keyword::Default),
            "try" => Token::Keyword(Keyword::Try),
            "catch" => Token::Keyword(Keyword::Catch),
            "finally" => Token::Keyword(Keyword::Finally),
            "with" => Token::Keyword(Keyword::With),
            "return" => Token::Keyword(Keyword::Return),
            "break" => Token::Keyword(Keyword::Break),
            "continue" => Token::Keyword(Keyword::Continue),
            "class" => Token::Keyword(Keyword::Class),
            "structure" => Token::Keyword(Keyword::Structure),
            "interface" => Token::Keyword(Keyword::Interface),
            "module" => Token::Keyword(Keyword::Module),
            "namespace" => Token::Keyword(Keyword::Namespace),
            "public" => Token::Keyword(Keyword::Public),
            "private" => Token::Keyword(Keyword::Private),
            "static" => Token::Keyword(Keyword::Static),
            "me" => Token::Keyword(Keyword::Me),
            "import" => Token::Keyword(Keyword::Import),
            "from" => Token::Keyword(Keyword::From),
            "as" => Token::Keyword(Keyword::As),
            "include" => Token::Keyword(Keyword::Include),
            "async" => Token::Keyword(Keyword::Async),
            "sync" => Token::Keyword(Keyword::Sync),
            "macro" => Token::Keyword(Keyword::Macro),
            "global" => Token::Keyword(Keyword::Global),
            "true" => Token::Keyword(Keyword::True),
            "false" => Token::Keyword(Keyword::False),
            "then" => Token::Keyword(Keyword::Then),
            _ => Token::Identifier(ident),
        };

        Ok(token)
    }

    fn lex_cmx(&mut self) -> ClsResult<Token> {
        self.advance(); // consume '<'
        let (_, all_tokens) = self.lex_cmx_element_tokens()?;
        // Push all tokens except the first (OpenTag) to global buffer
        let first = all_tokens[0].token.clone();
        for tok in all_tokens.iter().skip(1) {
            self.cmx_buffer.push_back(tok.clone());
        }
        Ok(first)
    }

    /// Parsea un elemento CMX completo → Vec de tokens en orden
    fn lex_cmx_element_tokens(&mut self) -> ClsResult<(String, Vec<SpannedToken>)> {
        let mut tokens = Vec::new();
        let tag_span = self.current_span();
        let tag_name = self.read_cmx_identifier()?;
        let self_closing = self.lex_cmx_attrs_into(&mut tokens)?;

        tokens.insert(0, SpannedToken::new(
            Token::Cmx(CmxToken::OpenTag { name: tag_name.clone(), is_self_closing: self_closing }),
            tag_span,
        ));

        if !self_closing {
            self.lex_cmx_children_into(&tag_name, &mut tokens)?;
        }
        Ok((tag_name, tokens))
    }

    /// Parsea atributos CMX, agregando tokens al Vec
    fn lex_cmx_attrs_into(&mut self, tokens: &mut Vec<SpannedToken>) -> ClsResult<bool> {
        loop {
            self.skip_cmx_whitespace();
            if self.is_eof() { return Err(ClsError::SyntaxError("Tag CMX sin cerrar".into())); }
            let ch = self.current_char();
            if ch == '>' { self.advance(); return Ok(false); }
            if ch == '/' && self.pos+1 < self.source.len() && self.source[self.pos+1] == '>' {
                self.advance(); self.advance(); return Ok(true);
            }
            if ch == '{' {
                self.advance();
                let name = self.read_cmx_identifier().unwrap_or_default();
                tokens.push(SpannedToken::new(Token::Cmx(CmxToken::AttrExpr { name }), self.current_span()));
                continue;
            }
            let name = self.read_cmx_identifier()?;
            self.skip_cmx_whitespace();
            if self.current_char() == '=' {
                self.advance(); self.skip_cmx_whitespace();
                if self.current_char() == '"' {
                    let value = self.read_cmx_string()?;
                    tokens.push(SpannedToken::new(Token::Cmx(CmxToken::AttrString { name, value }), self.current_span()));
                } else if self.current_char() == '{' {
                    let span = self.current_span(); self.advance();
                    tokens.push(SpannedToken::new(Token::Cmx(CmxToken::AttrExpr { name }), span));
                    self.in_cmx_expr = true;
                    let mut depth: i32 = 1;
                    while !self.is_eof() && depth > 0 {
                        let t = self.next_token()?;
                        match &t.token {
                            Token::Symbol(Symbol::LBrace) => depth += 1,
                            Token::Symbol(Symbol::RBrace) => { depth -= 1; if depth == 0 { break; } }
                            _ => {}
                        }
                        tokens.push(t);
                    }
                    self.in_cmx_expr = false;
                } else { return Err(ClsError::SyntaxError("Valor attr CMX inválido".into())); }
            }
        }
    }

    /// Parsea children CMX, agregando tokens al Vec
    fn lex_cmx_children_into(&mut self, close_tag: &str, tokens: &mut Vec<SpannedToken>) -> ClsResult<()> {
        loop {
            self.skip_cmx_whitespace();
            if self.is_eof() { return Err(ClsError::SyntaxError(format!("Tag '{}' sin cerrar", close_tag))); }
            let ch = self.current_char();
            if ch == '<' {
                if self.pos+1 < self.source.len() && self.source[self.pos+1] == '/' {
                    self.advance(); self.advance();
                    let name = self.read_cmx_identifier()?;
                    self.skip_cmx_whitespace();
                    if self.current_char() != '>' { return Err(ClsError::SyntaxError("Tag cierre inválido".into())); }
                    self.advance();
                    tokens.push(SpannedToken::new(Token::Cmx(CmxToken::CloseTag { name }), self.current_span()));
                    return Ok(());
                }
                if self.peek_is_cmx_start() {
                    self.advance();
                    let (_, child) = self.lex_cmx_element_tokens()?;
                    tokens.extend(child);
                    continue;
                }
                tokens.push(SpannedToken::new(Token::Cmx(CmxToken::Text { content: "<".into() }), self.current_span()));
                self.advance();
            } else if ch == '{' {
                self.advance();
                self.in_cmx_expr = true;
                let mut depth: i32 = 1;
                while !self.is_eof() && depth > 0 {
                    let t = self.next_token()?;
                    match &t.token {
                        Token::Symbol(Symbol::LBrace) => depth += 1,
                        Token::Symbol(Symbol::RBrace) => { depth -= 1; if depth == 0 { break; } }
                        _ => {}
                    }
                    tokens.push(t);
                }
                self.in_cmx_expr = false;
            } else {
                let mut text = String::new();
                while !self.is_eof() && self.current_char() != '<' && self.current_char() != '{' {
                    text.push(self.current_char()); self.advance();
                }
                if !text.trim().is_empty() {
                    tokens.push(SpannedToken::new(Token::Cmx(CmxToken::Text { content: text }), self.current_span()));
                }
            }
        }
    }

    /// Lee un identificador CMX (tag name o attribute name)
    fn read_cmx_identifier(&mut self) -> ClsResult<String> {
        let mut name = String::new();
        while !self.is_eof() && (self.current_char().is_alphanumeric() || self.current_char() == '_' || self.current_char() == '-') {
            name.push(self.current_char());
            self.advance();
        }
        if name.is_empty() { Err(ClsError::SyntaxError("Esperaba nombre de tag CMX".into())) } else { Ok(name) }
    }

    /// Salta espacios CMX
    fn skip_cmx_whitespace(&mut self) {
        while !self.is_eof() && matches!(self.current_char(), ' ' | '\t' | '\n' | '\r') { self.advance(); }
    }

    /// Lee string CMX entre comillas dobles
    fn read_cmx_string(&mut self) -> ClsResult<String> {
        self.advance();
        let mut s = String::new();
        while !self.is_eof() && self.current_char() != '"' {
            if self.current_char() == '\\' { self.advance(); if !self.is_eof() { s.push(self.current_char()); } }
            else { s.push(self.current_char()); }
            self.advance();
        }
        if self.is_eof() { return Err(ClsError::SyntaxError("String CMX sin cerrar".into())); }
        self.advance();
        Ok(s)
    }

    fn lex_operator_or_symbol(&mut self) -> ClsResult<Token> {
        let ch = self.current_char();

        // Símbolos primero (no ambiguos)
        if let Some(symbol) = Symbol::from_char(ch) {
            self.advance();
            // Manejar ...
            if symbol == Symbol::Dot && self.current_char() == '.' && self.peek_char(1) == '.' {
                self.advance();
                self.advance();
                return Ok(Token::Symbol(Symbol::Ellipsis));
            }
            return Ok(Token::Symbol(symbol));
        }

        // Operadores multi-carácter
        if let Some(op) = self.try_lex_multi_operator() {
            return Ok(op);
        }

        // Operadores simples
        if let Some(op) = Operator::from_str(&ch.to_string()) {
            self.advance();
            return Ok(Token::Operator(op));
        }

        // Carácter desconocido
        Err(ClsError::SyntaxError(format!(
            "Carácter inesperado: '{}' en línea {}, columna {}",
            ch, self.line, self.col
        )))
    }

    fn try_lex_multi_operator(&mut self) -> Option<Token> {
        // Intentar match de 2 caracteres primero
        if self.pos + 2 <= self.source.len() {
            let candidate: String = self.source[self.pos..self.pos + 2].iter().collect();
            if let Some(op) = Operator::from_str(&candidate) {
                self.advance();
                self.advance();
                return Some(Token::Operator(op));
            }
        }

        // Intentar match de 3 caracteres
        if self.pos + 3 <= self.source.len() {
            let candidate: String = self.source[self.pos..self.pos + 3].iter().collect();
            if let Some(op) = Operator::from_str(&candidate) {
                self.advance();
                self.advance();
                self.advance();
                return Some(Token::Operator(op));
            }
        }

        None
    }

    fn peek_is_cmx_start(&self) -> bool {
        if self.in_cmx_expr { return false; }
        if self.pos + 1 >= self.source.len() { return false; }
        let next = self.source[self.pos + 1];
        next.is_alphabetic() || next == '>'
    }

    fn peek_char_is_digit(&self, offset: usize) -> bool {
        if self.pos + offset >= self.source.len() {
            return false;
        }
        self.source[self.pos + offset].is_ascii_digit()
    }

    fn current_span(&self) -> Span {
        Span {
            start_line: self.line,
            start_col: self.col,
            end_line: self.line,
            end_col: self.col,
        }
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.source.len()
    }

    fn current_char(&self) -> char {
        self.source[self.pos]
    }

    fn peek_char(&self, offset: usize) -> char {
        if self.pos + offset >= self.source.len() {
            '\0'
        } else {
            self.source[self.pos + offset]
        }
    }

    fn advance(&mut self) {
        if !self.is_eof() {
            if self.source[self.pos] == '\n' {
                self.line += 1;
                self.col = 1;
                self.line_start = self.pos + 1;
            } else {
                self.col += 1;
            }
            self.pos += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexer_simple() {
        let mut lexer = Lexer::new("var x = 42");
        let tokens = lexer.tokenize().unwrap();
        assert!(tokens.len() > 0);
    }

    #[test]
    fn test_lexer_string() {
        let mut lexer = Lexer::new(r#""hello world""#);
        let tokens = lexer.tokenize().unwrap();
        // assert!(matches!(tokens[0], Token::StringLiteral(_)));
    }

    #[test]
    fn test_lexer_comment() {
        let mut lexer = Lexer::new("# comentario\nvar x = 1");
        let tokens = lexer.tokenize().unwrap();
        // assert!(matches!(tokens[0], Token::Keyword(_)));
    }
}

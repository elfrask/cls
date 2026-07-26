use crate::error::diagnostic::Span;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Todos los tipos de tokens del lenguaje CLS
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Literales
    IntLiteral(i64),
    FloatLiteral(f64),
    StringLiteral(String),
    BoolLiteral(bool),
    CharLiteral(char),

    // Identificadores y keywords
    Identifier(String),
    Keyword(Keyword),

    // Operadores (incluye multi-carácter)
    Operator(Operator),

    // Símbolos
    Symbol(Symbol),

    // CMX (JSX nativo)
    Cmx(CmxToken),

    // Especiales
    Newline,
    EOF,
}

impl Token {
    pub fn span(&self) -> Span {
        match self {
            Token::IntLiteral(_) => Span::new(0, 0, 0, 0),
            Token::FloatLiteral(_) => Span::new(0, 0, 0, 0),
            Token::StringLiteral(_) => Span::new(0, 0, 0, 0),
            Token::BoolLiteral(_) => Span::new(0, 0, 0, 0),
            Token::CharLiteral(_) => Span::new(0, 0, 0, 0),
            Token::Identifier(_) => Span::new(0, 0, 0, 0),
            Token::Keyword(_) => Span::new(0, 0, 0, 0),
            Token::Operator(_) => Span::new(0, 0, 0, 0),
            Token::Symbol(_) => Span::new(0, 0, 0, 0),
            Token::Cmx(_) => Span::new(0, 0, 0, 0),
            Token::Newline => Span::new(0, 0, 0, 0),
            Token::EOF => Span::new(0, 0, 0, 0),
        }
    }
}

/// Keywords del lenguaje CLS
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Keyword {
    // Declaraciones
    Var,
    Const,
    Let,

    // Funciones
    Function,
    Void,
    Method,
    Export,

    // Control de flujo
    If,
    Elif,
    Else,
    While,
    Loop,
    For,
    Each,
    In,
    And,      // para "for each x and i in"
    Switch,
    Case,
    Default,
    Try,
    Catch,
    Finally,
    With,
    Return,
    Break,
    Continue,

    // Clases y estructuras
    Class,
    Structure,
    Interface,
    Module,
    Namespace,
    Public,
    Private,
    Static,
    Me,

    // Imports
    Import,
    From,
    As,
    Include,

    // Tipos especiales
    Async,
    Sync,
    Macro,
    Global,

    // Configuración
    Config,
    Then,
    True,
    False,
}

impl fmt::Display for Operator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Operadores de CLS
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Operator {
    // Aritméticos
    Plus,       // +
    Minus,      // -
    Star,       // *
    Slash,      // /
    Percent,    // %
    StarStar,   // **

    // Comparación
    Equal,          // =
    NotEqual,       // !=
    LessThan,       // <
    GreaterThan,    // >
    LessEqual,      // <=
    GreaterEqual,   // >=
    StrictEqual,    // ==

    // Lógicos
    And,        // &
    Or,         // |
    Not,        // !
    Question,   // ?

    // Asignación compuesta
    PlusEqual,      // +=
    MinusEqual,     // -=
    StarEqual,      // *=
    SlashEqual,     // /=

    // Incremento/Decremento
    PlusPlus,       // ++
    MinusMinus,     // --

    // Tipo / Namespace
    Arrow,          // ->
    ColonColon,     // ::
    DotDot,         // ..
    Colon,          // :

    // Otros
    At,             // @
    Pipe,           // |
    Tilde,          // ~
    Caret,          // ^
    ShiftLeft,      // <<
    ShiftRight,     // >>
    Backslash,      // \
}

impl Operator {
    /// Operadores de un solo carácter para el lexer
    pub const SINGLE_CHARS: &[char] = &[
        '+', '-', '*', '/', '%', '=', '<', '>', '!', '&', '|', '^', '~', '?', ':', '@', '#', '\\',
    ];

    /// Operadores multi-carácter para el lexer
    pub const MULTI: &[&str] = &[
        "==", "!=", ">=", "<=", "&&", "||", "++", "--", "**", "+=", "-=", "*=", "/=", "=>",
        "->", "::", "..", "<<", ">>", "//",
    ];

    /// Obtiene el operador desde un string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "+" => Some(Self::Plus),
            "-" => Some(Self::Minus),
            "*" => Some(Self::Star),
            "/" => Some(Self::Slash),
            "%" => Some(Self::Percent),
            "**" => Some(Self::StarStar),
            "=" => Some(Self::Equal),
            "!=" => Some(Self::NotEqual),
            "<" => Some(Self::LessThan),
            ">" => Some(Self::GreaterThan),
            "<=" => Some(Self::LessEqual),
            ">=" => Some(Self::GreaterEqual),
            "==" => Some(Self::StrictEqual),
            "&" => Some(Self::And),
            "|" => Some(Self::Or),
            "!" => Some(Self::Not),
            "?" => Some(Self::Question),
            "+=" => Some(Self::PlusEqual),
            "-=" => Some(Self::MinusEqual),
            "*=" => Some(Self::StarEqual),
            "/=" => Some(Self::SlashEqual),
            "++" => Some(Self::PlusPlus),
            "--" => Some(Self::MinusMinus),
            "->" => Some(Self::Arrow),
            "::" => Some(Self::ColonColon),
            ".." => Some(Self::DotDot),
            ":" => Some(Self::Colon),
            "@" => Some(Self::At),
            "~" => Some(Self::Tilde),
            "^" => Some(Self::Caret),
            "<<" => Some(Self::ShiftLeft),
            ">>" => Some(Self::ShiftRight),
            "\\" => Some(Self::Backslash),
            _ => None,
        }
    }
}

/// Símbolos estructurales
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Symbol {
    LParen,     // (
    RParen,     // )
    LBracket,   // [
    RBracket,   // ]
    LBrace,     // {
    RBrace,     // }
    Comma,      // ,
    Dot,        // .
    Semicolon,  // ;
    Ellipsis,   // ...
}

impl Symbol {
    pub fn from_char(c: char) -> Option<Self> {
        match c {
            '(' => Some(Self::LParen),
            ')' => Some(Self::RParen),
            '[' => Some(Self::LBracket),
            ']' => Some(Self::RBracket),
            '{' => Some(Self::LBrace),
            '}' => Some(Self::RBrace),
            ',' => Some(Self::Comma),
            '.' => Some(Self::Dot),
            ';' => Some(Self::Semicolon),
            _ => None,
        }
    }
}

/// Token CMX (JSX nativo)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CmxToken {
    // Tag de apertura
    OpenTag { name: String, is_self_closing: bool },
    // Tag de cierre
    CloseTag { name: String },
    // Texto entre tags
    Text { content: String },
    // Inicio de expresión dentro de CMX
    ExpressionStart,
    // Fin de expresión dentro de CMX
    ExpressionEnd,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::IntLiteral(v) => write!(f, "Int({})", v),
            Token::FloatLiteral(v) => write!(f, "Float({})", v),
            Token::StringLiteral(v) => write!(f, "String({:?})", v),
            Token::BoolLiteral(v) => write!(f, "Bool({})", v),
            Token::CharLiteral(v) => write!(f, "Char({:?})", v),
            Token::Identifier(v) => write!(f, "Ident({})", v),
            Token::Keyword(k) => write!(f, "Keyword({:?})", k),
            Token::Operator(o) => write!(f, "Op({:?})", o),
            Token::Symbol(s) => write!(f, "Sym({:?})", s),
            Token::Cmx(c) => write!(f, "Cmx({:?})", c),
            Token::Newline => write!(f, "Newline"),
            Token::EOF => write!(f, "EOF"),
        }
    }
}

/// Token con información de span (línea, columna)
#[derive(Debug, Clone)]
pub struct SpannedToken {
    pub token: Token,
    pub span: Span,
}

impl SpannedToken {
    pub fn new(token: Token, span: Span) -> Self {
        Self { token, span }
    }
}

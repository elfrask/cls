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
    Alias,
    Enum,
    Public,
    Private,
    Protected,
    Static,
    Extends,
    Is,
    Super,
    Readonly,
    Me,

    // Imports
    Import,
    From,
    As,
    Include,

    // Tipos especiales
    Async,
    Await,
    Sync,
    Macro,
    Global,

    // Configuración
    Config,
    Then,
    True,
    False,
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
    Is,             // is (instancia de)

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
            "&&" => Some(Self::And),
            "|" => Some(Self::Or),
            "||" => Some(Self::Or),
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
    /// Tag de apertura: <Tag> o <Tag />
    OpenTag { name: String, is_self_closing: bool },
    /// Tag de cierre: </Tag>
    CloseTag { name: String },
    /// Texto entre tags
    Text { content: String },
    /// Atributo string: name="value"
    AttrString { name: String, value: String },
    /// Inicio de atributo expresión: name={ → el parser lee expresión hasta ExprEnd
    AttrExpr { name: String },
    /// Fin de expresión dentro de CMX ({ → ... → })
    ExprEnd,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::IntLiteral(v) => write!(f, "{}", v),
            Token::FloatLiteral(v) => write!(f, "{}", v),
            Token::StringLiteral(v) => write!(f, "\"{}\"", v),
            Token::BoolLiteral(v) => write!(f, "{}", v),
            Token::CharLiteral(v) => write!(f, "'{}'", v),
            Token::Identifier(v) => write!(f, "{}", v),
            Token::Keyword(k) => write!(f, "{}", k),
            Token::Operator(o) => write!(f, "{}", o),
            Token::Symbol(s) => write!(f, "{}", s),
            Token::Cmx(_) => write!(f, "<cmx>"),
            Token::Newline => write!(f, "\\n"),
            Token::EOF => write!(f, "<eof>"),
        }
    }
}

// ─── Display impls legibles ───

impl fmt::Display for CmxToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CmxToken::OpenTag { name, is_self_closing } => {
                if *is_self_closing { write!(f, "<{}/>", name) } else { write!(f, "<{}>", name) }
            }
            CmxToken::CloseTag { name } => write!(f, "</{}>", name),
            CmxToken::Text { content } => write!(f, "text({})", content),
            CmxToken::AttrString { name, value } => write!(f, "{}=\"{}\"", name, value),
            CmxToken::AttrExpr { name } => write!(f, "{}={{...}}", name),
            CmxToken::ExprEnd => write!(f, "}}"),
        }
    }
}

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Symbol::LParen => write!(f, "'('"),
            Symbol::RParen => write!(f, "')'"),
            Symbol::LBracket => write!(f, "'['"),
            Symbol::RBracket => write!(f, "']'"),
            Symbol::LBrace => write!(f, "'{{'"),
            Symbol::RBrace => write!(f, "'}}'"),
            Symbol::Comma => write!(f, "','"),
            Symbol::Dot => write!(f, "'.'"),
            Symbol::Semicolon => write!(f, "';'"),
            Symbol::Ellipsis => write!(f, "'...'"),
        }
    }
}

impl fmt::Display for Keyword {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Keyword::Var => "var",
            Keyword::Const => "const",
            Keyword::Let => "let",
            Keyword::Function => "function",
            Keyword::Void => "void",
            Keyword::Method => "method",
            Keyword::Export => "export",
            Keyword::If => "if",
            Keyword::Elif => "elif",
            Keyword::Else => "else",
            Keyword::While => "while",
            Keyword::Loop => "loop",
            Keyword::For => "for",
            Keyword::Each => "each",
            Keyword::In => "in",
            Keyword::And => "and",
            Keyword::Switch => "switch",
            Keyword::Case => "case",
            Keyword::Default => "default",
            Keyword::Try => "try",
            Keyword::Catch => "catch",
            Keyword::Finally => "finally",
            Keyword::With => "with",
            Keyword::Return => "return",
            Keyword::Break => "break",
            Keyword::Continue => "continue",
            Keyword::Class => "class",
            Keyword::Structure => "structure",
            Keyword::Interface => "interface",
            Keyword::Module => "module",
            Keyword::Namespace => "namespace",
            Keyword::Alias => "alias",
            Keyword::Enum => "enum",
            Keyword::Public => "public",
            Keyword::Private => "private",
            Keyword::Protected => "protected",
            Keyword::Static => "static",
            Keyword::Extends => "extends",
            Keyword::Is => "is",
            Keyword::Super => "super",
            Keyword::Readonly => "readonly",
            Keyword::Me => "me",
            Keyword::Import => "import",
            Keyword::From => "from",
            Keyword::As => "as",
            Keyword::Include => "include",
            Keyword::Async => "async",
            Keyword::Await => "await",
            Keyword::Sync => "sync",
            Keyword::Macro => "macro",
            Keyword::Global => "global",
            Keyword::Config => "config",
            Keyword::Then => "then",
            Keyword::True => "true",
            Keyword::False => "false",
        };
        write!(f, "'{}'", s)
    }
}

impl fmt::Display for Operator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Operator::Plus => "+",
            Operator::Minus => "-",
            Operator::Star => "*",
            Operator::Slash => "/",
            Operator::Percent => "%",
            Operator::StarStar => "**",
            Operator::Equal => "=",
            Operator::NotEqual => "!=",
            Operator::LessThan => "<",
            Operator::GreaterThan => ">",
            Operator::LessEqual => "<=",
            Operator::GreaterEqual => ">=",
            Operator::StrictEqual => "==",
            Operator::Is => "is",
            Operator::And => "&",
            Operator::Or => "|",
            Operator::Not => "!",
            Operator::Question => "?",
            Operator::PlusEqual => "+=",
            Operator::MinusEqual => "-=",
            Operator::StarEqual => "*=",
            Operator::SlashEqual => "/=",
            Operator::PlusPlus => "++",
            Operator::MinusMinus => "--",
            Operator::Arrow => "->",
            Operator::ColonColon => "::",
            Operator::DotDot => "..",
            Operator::Colon => ":",
            Operator::At => "@",
            Operator::Pipe => "|",
            Operator::Tilde => "~",
            Operator::Caret => "^",
            Operator::ShiftLeft => "<<",
            Operator::ShiftRight => ">>",
            Operator::Backslash => "\\",
        };
        write!(f, "'{}'", s)
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

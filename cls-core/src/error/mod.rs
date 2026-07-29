pub mod diagnostic;

pub use diagnostic::{Diagnostic, Span};
use thiserror::Error;

/// Resultado de operaciones CLS
pub type ClsResult<T> = Result<T, ClsError>;

/// Frame individual en el call stack (función + ubicación)
#[derive(Debug, Clone)]
pub struct StackFrame {
    pub function: String,
    pub span: Option<Span>,
    pub source_file: String,
}

impl StackFrame {
    pub fn new(function: &str, span: Option<Span>, source_file: &str) -> Self {
        Self {
            function: function.to_string(),
            span,
            source_file: source_file.to_string(),
        }
    }
}

#[derive(Error, Debug)]
pub enum ClsError {
    #[error("Error de compilación: {0}")]
    CompileError(String),

    #[error("Error de runtime: {0}")]
    RuntimeError(String),

    #[error("Error de tipo: {0}")]
    TypeError(String),

    #[error("Error de sintaxis: {0}")]
    SyntaxError(String),

    #[error("Error de IO: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Error de configuración: {0}")]
    ConfigError(String),
}

impl ClsError {
    pub fn syntax_at(msg: &str, span: &Span) -> Self {
        ClsError::SyntaxError(format!(
            "{} (línea {}, columna {})",
            msg, span.start_line, span.start_col
        ))
    }

    pub fn with_span(msg: &str, span: &Span) -> Self {
        ClsError::SyntaxError(format!(
            "{} en línea {}, columna {}",
            msg, span.start_line, span.start_col
        ))
    }

    /// Extrae línea/columna del mensaje (para variantes viejas que incrustan span en el string)
    pub fn extract_line_col(msg: &str) -> Option<(usize, usize)> {
        let rest = msg.split("línea").nth(1)?;
        let line_str: String = rest.chars().take_while(|c| c.is_ascii_digit() || c.is_whitespace()).collect();
        let line = line_str.trim().parse::<usize>().ok()?;
        // Buscar "columna " después de la línea
        let after = rest.split("columna").nth(1)?;
        let col_str: String = after.chars().take_while(|c| c.is_ascii_digit() || c.is_whitespace()).collect();
        let col = col_str.trim().parse::<usize>().ok()?;
        Some((line, col))
    }
}

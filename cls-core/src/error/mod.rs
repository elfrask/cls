pub mod diagnostic;

pub use diagnostic::{Diagnostic, Span};
use thiserror::Error;

/// Resultado de operaciones CLS
pub type ClsResult<T> = Result<T, ClsError>;

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

    #[error("Error de parseo de configuración: {0}")]
    ConfigError(String),
}

impl ClsError {
    /// Construye un SyntaxError con información de span
    pub fn syntax_at(msg: &str, span: &Span) -> Self {
        ClsError::SyntaxError(format!(
            "{} (línea {}, columna {})",
            msg, span.start_line, span.start_col
        ))
    }

    /// Construye un error con mensaje y span
    pub fn with_span(msg: &str, span: &Span) -> Self {
        ClsError::SyntaxError(format!(
            "{} en línea {}, columna {}",
            msg, span.start_line, span.start_col
        ))
    }
}

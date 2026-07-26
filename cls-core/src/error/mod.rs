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

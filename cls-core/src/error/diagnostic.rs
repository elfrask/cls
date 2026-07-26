use serde::{Deserialize, Serialize};
use std::fmt;

/// Representa un diagnóstico (error/warning) con contexto de fuente
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub message: String,
    pub span: Span,
    pub severity: Severity,
    pub source_file: Option<String>,
}

impl Diagnostic {
    pub fn error(message: &str, span: Span) -> Self {
        Self {
            message: message.to_string(),
            span,
            severity: Severity::Error,
            source_file: None,
        }
    }

    pub fn warning(message: &str, span: Span) -> Self {
        Self {
            message: message.to_string(),
            span,
            severity: Severity::Warning,
            source_file: None,
        }
    }
}

/// Ubicación en el código fuente (archivo, línea, columna, longitud)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Span {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

impl Span {
    pub fn new(start_line: u32, start_col: u32, end_line: u32, end_col: u32) -> Self {
        Self {
            start_line,
            start_col,
            end_line,
            end_col,
        }
    }

    pub fn merge(&self, other: &Span) -> Span {
        Span {
            start_line: self.start_line.min(other.start_line),
            start_col: self.start_col.min(other.start_col),
            end_line: self.end_line.max(other.end_line),
            end_col: self.end_col.max(other.end_col),
        }
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.start_line, self.start_col)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
    Hint,
}

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

    /// Error de sintaxis con span estructurado (no incrustado en el texto).
    #[error("Error de sintaxis: {0}")]
    SyntaxErrorAt(String, Span),

    /// Error de compilación con span estructurado (p.ej. el JIT: "no soportado").
    #[error("Error de compilación: {0}")]
    CompileErrorAt(String, Span),

    #[error("Error de IO: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Error de configuración: {0}")]
    ConfigError(String),
}

// `std::io::Error` no es `Clone`; se reconstruye conservando kind y mensaje.
impl Clone for ClsError {
    fn clone(&self) -> Self {
        match self {
            ClsError::CompileError(m) => ClsError::CompileError(m.clone()),
            ClsError::RuntimeError(m) => ClsError::RuntimeError(m.clone()),
            ClsError::TypeError(m) => ClsError::TypeError(m.clone()),
            ClsError::SyntaxError(m) => ClsError::SyntaxError(m.clone()),
            ClsError::SyntaxErrorAt(m, s) => ClsError::SyntaxErrorAt(m.clone(), s.clone()),
            ClsError::CompileErrorAt(m, s) => ClsError::CompileErrorAt(m.clone(), s.clone()),
            ClsError::IoError(e) => ClsError::IoError(std::io::Error::new(e.kind(), e.to_string())),
            ClsError::ConfigError(m) => ClsError::ConfigError(m.clone()),
        }
    }
}

impl ClsError {
    /// Fábrica centralizada: error de sintaxis con span estructurado.
    /// El mensaje queda limpio (sin "(línea N, columna M)" incrustado);
    /// la ubicación vive en el `Span` para que el formateador la use directa.
    pub fn syntax_at(msg: &str, span: &Span) -> Self {
        ClsError::SyntaxErrorAt(msg.to_string(), span.clone())
    }

    /// Alias de `syntax_at` (mensaje limpio + span).
    pub fn with_span(msg: &str, span: &Span) -> Self {
        ClsError::SyntaxErrorAt(msg.to_string(), span.clone())
    }

    /// Fábrica: error de compilación con span estructurado (JIT y backend).
    pub fn compile_at(msg: &str, span: &Span) -> Self {
        ClsError::CompileErrorAt(msg.to_string(), span.clone())
    }

    /// Extrae línea/columna del mensaje (para variantes legacy que incrustan span en el string)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_new_and_display() {
        let s = Span::new(1, 5, 3, 20);
        assert_eq!(s.start_line, 1);
        assert_eq!(s.start_col, 5);
        assert_eq!(s.end_line, 3);
        assert_eq!(s.end_col, 20);
        assert_eq!(s.to_string(), "1:5");
    }

    #[test]
    fn test_span_merge() {
        let a = Span::new(1, 1, 2, 10);
        let b = Span::new(3, 5, 4, 15);
        let merged = a.merge(&b);
        assert_eq!(merged.start_line, 1);
        assert_eq!(merged.start_col, 1);
        assert_eq!(merged.end_line, 4);
        assert_eq!(merged.end_col, 15);
    }

    #[test]
    fn test_stack_frame_new() {
        let span = Span::new(2, 10, 2, 20);
        let f = StackFrame::new("foo", Some(span), "test.clsx");
        assert_eq!(f.function, "foo");
        assert!(f.span.is_some());
        assert_eq!(f.source_file, "test.clsx");
    }

    #[test]
    fn test_stack_frame_no_span() {
        let f = StackFrame::new("bar", None, "lib.clsx");
        assert_eq!(f.function, "bar");
        assert!(f.span.is_none());
    }

    #[test]
    fn test_cls_error_display() {
        let err = ClsError::RuntimeError("test error".into());
        let msg = err.to_string();
        assert!(msg.contains("test error"));
        assert!(msg.contains("Error de runtime"));

        let err = ClsError::SyntaxError("bad token".into());
        assert!(err.to_string().contains("Error de sintaxis"));
    }

    #[test]
    fn test_syntax_at() {
        let span = Span::new(3, 7, 3, 7);
        let err = ClsError::syntax_at("error", &span);
        match &err {
            ClsError::SyntaxErrorAt(msg, s) => {
                assert_eq!(msg, "error");
                assert_eq!(s.start_line, 3);
                assert_eq!(s.start_col, 7);
            }
            _ => panic!("esperaba SyntaxErrorAt"),
        }
    }

    #[test]
    fn test_extract_line_col() {
        let msg = "Error de runtime: División por cero (línea 2, columna 38)";
        let (line, col) = ClsError::extract_line_col(msg).unwrap();
        assert_eq!(line, 2);
        assert_eq!(col, 38);
    }

    #[test]
    fn test_extract_line_col_no_match() {
        let msg = "Error de runtime: algo salió mal";
        assert!(ClsError::extract_line_col(msg).is_none());
    }

    #[test]
    fn test_cls_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: ClsError = io_err.into();
        assert!(matches!(err, ClsError::IoError(_)));
    }
}

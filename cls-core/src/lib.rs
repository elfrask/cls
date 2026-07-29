//! CLS Core Library
//!
//! El compilador de CLS: frontend (lexer/parser) → middleware (análisis) → backend (salida).
//! También compila a WASM para ser usado desde cualquier lenguaje host.

pub mod config;
pub mod frontend;
pub mod middleware;
pub mod backend;
pub mod error;

/// Punto de entrada principal de la librería
pub use error::{ClsError, ClsResult, StackFrame, Span};

/// Versión del compilador CLS
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

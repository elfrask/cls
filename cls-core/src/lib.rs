//! CLS Core Library
//!
//! El compilador de CLS: frontend (lexer/parser) -> middleware (análisis) -> backend (salida).
//! También compila a WASM para ser usado desde cualquier lenguaje host.

pub mod config;
pub mod frontend;
pub mod middleware;
pub mod backend;
pub mod error;
pub mod ansi;

/// Punto de entrada principal de la librería
pub use error::{ClsError, ClsResult, StackFrame, Span};

/// Versión del compilador CLS
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
/// Hash de los fuentes del backend WASM (+ internals) — el caché CLS->WASM de
/// cls-jit lo incluye para invalidar .wasm viejos al cambiar el emisor.
pub const BACKEND_HASH: &str = env!("CLS_BACKEND_HASH");

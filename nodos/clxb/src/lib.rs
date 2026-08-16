//! clxb - nodo de bindings de CLS (embedding).
//!
//! Compila CLS -> WASM (motor `cls-jit`) y expone la ejecución embebida:
//! `run_main`, `call` a `export function`, `eval`, captura de `print` y el SDK
//! de nodo (intrinsics vía `env.host_call`, resolver de módulos). La capa C
//! (`clsb_v1_*`) vive en `capi.rs` (F2b).

pub mod capi;
pub mod engine;
pub mod value;

pub use engine::{ClsEngine, ClsModule};
pub use value::ClsValue;

/// Error de embedding con el trace completo formateado.
#[derive(Debug, Clone)]
pub struct ClsError {
    /// Mensaje limpio.
    pub message: String,
    /// Trace completo (AGENTS.md: obligatorio en runtime).
    pub trace: String,
}

impl ClsError {
    pub fn new(msg: String) -> Self {
        Self {
            message: msg.clone(),
            trace: msg,
        }
    }
}

impl std::fmt::Display for ClsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.trace)
    }
}

impl std::error::Error for ClsError {}

impl From<cls_core::error::ClsError> for ClsError {
    fn from(e: cls_core::error::ClsError) -> Self {
        let mut msg = e.to_string();
        // Anclar línea/col si el error las incrusta (fallback).
        if let Some((l, c)) = cls_core::error::ClsError::extract_line_col(&msg) {
            msg = format!("{} (línea {}, columna {})", msg, l, c);
        }
        Self::new(msg)
    }
}

impl From<String> for ClsError {
    fn from(msg: String) -> Self {
        Self::new(msg)
    }
}

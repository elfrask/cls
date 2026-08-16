//! `cls-internals`: módulos internos e intrinsics de CLS precompilados a WASM.
//!
//! Fase 2 del plan de rendimiento (ver `agent-context/plan-performance/`).
//! Este crate NO se integra todavía con el core (cls-core/cls-jit): expone el
//! binario WASM compilado (`INTERNALS_WASM`) y la firma de cada función interna
//! (`abi::INTERNALS_FUNCTIONS`). La Fase 3 decidirá cómo el emisor las linkea
//! dentro del módulo CLS (fusión de secciones o import de módulo interno).
//!
//! Las funciones internas viven en `wasm/` (sub-crate que compila a
//! `wasm32-unknown-unknown`); `build.rs` lo compila y lo embebe aquí.

pub mod abi;

pub use abi::{InternalsFn, INTERNALS_FUNCTIONS};

/// Bytes del módulo WASM con todas las funciones internas (`__intr_*`),
/// exportadas con prefijo `__intr_<area>_<op>` (ver `abi::INTERNALS_FUNCTIONS`).
pub static INTERNALS_WASM: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/internals.wasm"));

/// Nombre del prefijo de export de las funciones internas.
pub const INTR_PREFIX: &str = "__intr_";

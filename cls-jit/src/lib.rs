//! Motor JIT reusable de CLS: CLS → WASM → wasmtime (Cranelift).
//!
//! Este crate centraliza el motor de ejecución JIT que antes vivía en el nodo
//! `clx` (`nodos/clx/src/jit.rs`). Lo usan:
//!
//! - `clx run --jit` (CLI de desarrollo) — el nodo inyecta el backend nativo y
//!   los hooks del índice de módulos.
//! - El futuro nodo de bindings (`nodos/clxb`) — mismo motor, otra frontera.
//!
//! El motor es **agnóstico al nodo**: el nodo provee el backend de extensiones
//! nativas (`NativeBackend`) y los hooks opcionales (`ModuleIndexHook`) vía
//! [`JitContext`]. Todo lo demás (parseo, typeck, span_shift, flatten, emisión
//! WASM, host functions, ejecución) es responsabilidad del motor.

pub mod engine;
pub mod error;
pub mod flatten;
pub mod resolve;
pub mod timing;

pub use engine::run_jit;
pub use resolve::{cache_dir, load_import_modules, module_candidates};

use std::path::Path;
use std::sync::Arc;

/// Hook del índice de módulos del nodo (caché del workspace, INFORMATIVO).
///
/// El motor escribe el índice tras compilar un módulo (para inspeccionar desde
/// disco qué módulos participan y sus hashes). El nodo decide dónde y cómo;
/// el motor solo lo invoca si el nodo lo provee.
pub trait ModuleIndexHook {
    /// Raíz del workspace del entry (el primer dir con `cls.json`).
    fn workspace_root(&self, entry: &Path) -> std::path::PathBuf;
    /// Escribe el índice de integridad de los módulos del workspace.
    fn write_module_index(&self, entry: &Path, extra: &[std::path::PathBuf]);
}

/// Contexto que el NODO inyecta al motor JIT.
///
/// El motor es agnóstico a la plataforma; el nodo provee:
/// - `native_backend`: backend de extensiones nativas (`extension "lib" as C`),
///   el nodo desktop usa `DynamicBackend` (libloading).
/// - `module_index`: hooks opcionales del índice de módulos del workspace.
pub struct JitContext<'a> {
    /// Backend de extensiones nativas (`cls_runtime::ffi::NativeBackend`).
    pub native_backend: Arc<dyn cls_runtime::ffi::NativeBackend>,
    /// Hooks del índice de módulos del nodo (opcional).
    pub module_index: Option<&'a dyn ModuleIndexHook>,
}

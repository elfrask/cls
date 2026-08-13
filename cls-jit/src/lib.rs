//! Motor JIT reusable de CLS: CLS → WASM → runtime (wasmtime desktop, wasmi web).
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

pub mod compile;
pub mod engine;
pub mod error;
pub mod flatten;
pub mod host;
pub mod resolve;
pub mod state;
pub mod timing;

#[cfg(feature = "wasmi-runtime")]
pub mod wasmi_rt;
pub mod wasmtime_rt;

pub use compile::{compile_file, compile_source, parse_clx_exports, CompileOptions, CompiledModule, ExportSig};
pub use engine::{run_jit, run_jit_with};
pub use host::{HostCallArg, HostCallHandler, HostCallResult, ModuleSourceResolver, OutputSink};
pub use resolve::{cache_dir, load_import_modules, module_candidates};

use std::path::Path;
use std::sync::Arc;

/// Runtime de ejecución del WASM.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RuntimeKind {
    /// wasmtime (Cranelift): excepciones CLS completas (tag + try_table + caret).
    Wasmtime,
    /// wasmi (intérprete puro, para wasm32/navegador): sin exception-handling;
    /// los errores de runtime son traps y `try/catch`/`throw` no se compilan.
    Wasmi,
}

impl RuntimeKind {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            RuntimeKind::Wasmtime => "wasmtime",
            RuntimeKind::Wasmi => "wasmi",
        }
    }
}

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
/// - `host_intrinsics`: funciones host del nodo (nombres → firmas) que el
///   script puede llamar como `nombre(args)`; se compilan vía `env.host_call`.
/// - `host_call_handler`: despacha las llamadas del canal `env.host_call`.
pub struct JitContext<'a> {
    /// Backend de extensiones nativas (`cls_runtime::ffi::NativeBackend`).
    pub native_backend: Arc<dyn cls_runtime::ffi::NativeBackend>,
    /// Hooks del índice de módulos del nodo (opcional).
    pub module_index: Option<&'a dyn ModuleIndexHook>,
    /// Funciones host del nodo (intrinsics): el typeck las tipa y el backend
    /// emite las llamadas vía `env.host_call(id, ptr, n)`.
    pub host_intrinsics: &'a [cls_core::middleware::types::HostIntrinsic],
    /// Handler del canal `env.host_call` (sin handler → 0 + warning).
    pub host_call_handler: Option<Arc<dyn HostCallHandler>>,
    /// Resolver de módulos del nodo: provee sources que no están en disco.
    pub module_source_resolver: Option<&'a dyn ModuleSourceResolver>,
    /// Destino de `print` (sin él → stdout).
    pub output: Option<Arc<dyn OutputSink>>,
}

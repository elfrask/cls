//! Motor JIT: delegación al crate `cls-jit` (motor reusable).
//!
//! `clx run --jit <archivo> [-- args...]` compila el archivo con el motor
//! centralizado de `cls-jit` (CLS → WASM → wasmtime), inyectando el backend
//! nativo del nodo (`DynamicBackend`) y los hooks del índice de módulos del
//! workspace. El motor vive en `cls-jit` para que el nodo de bindings lo
//! reutilice sin duplicar.

use std::sync::Arc;

pub use cls_jit::{cache_dir, load_import_modules, module_candidates};

/// Hooks del nodo al índice de módulos (caché del workspace, INFORMATIVO).
struct ClxModuleIndexHook;

impl cls_jit::ModuleIndexHook for ClxModuleIndexHook {
    fn workspace_root(&self, entry: &std::path::Path) -> std::path::PathBuf {
        crate::module_index::workspace_root(entry)
    }

    fn write_module_index(&self, entry: &std::path::Path, extra: &[std::path::PathBuf]) {
        let _ = crate::module_index::write_module_index(entry, extra);
    }
}

/// Ejecuta un programa CLS con el JIT (CLS → WASM → wasmtime).
/// Devuelve el exit code (0 = OK, 1 = error).
pub fn run_jit(entry: &str, app_args: &[String], target_str: Option<&str>) -> i32 {
    let ctx = cls_jit::JitContext {
        native_backend: Arc::new(crate::native::DynamicBackend),
        module_index: Some(&ClxModuleIndexHook),
        host_intrinsics: &[],
        host_call_handler: None,
        module_source_resolver: None,
        output: None,
    };
    // `CLS_JIT_RUNTIME=wasmi` → ejecutar con wasmi (intérprete puro, sin
    // excepciones CLS). Útil para validar el runtime del navegador desde el CLI.
    let runtime = match std::env::var("CLS_JIT_RUNTIME").as_deref() {
        Ok("wasmi") => cls_jit::RuntimeKind::Wasmi,
        _ => cls_jit::RuntimeKind::Wasmtime,
    };
    cls_jit::run_jit_with(entry, app_args, target_str, &ctx, runtime)
}

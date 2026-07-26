use crate::error::ClsResult;
use crate::frontend::ast::Module;

/// Backend que compila AST a WASM (placeholder)
pub struct WasmBackend;

impl WasmBackend {
    pub fn new() -> Self {
        Self
    }

    /// Compila el AST a WASM (por ahora placeholder)
    pub fn emit(&self, _module: &Module) -> ClsResult<Vec<u8>> {
        // TODO: implementar codegen a WASM
        Err(crate::error::ClsError::CompileError(
            "WASM codegen no implementado aún".to_string(),
        ))
    }
}

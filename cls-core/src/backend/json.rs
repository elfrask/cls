use crate::error::ClsResult;
use crate::frontend::ast::Module;
use serde_json;

/// Backend que serializa el AST a JSON
pub struct JsonBackend;

impl JsonBackend {
    pub fn new() -> Self {
        Self
    }

    /// Serializa el AST a JSON
    pub fn emit(&self, module: &Module) -> ClsResult<String> {
        serde_json::to_string_pretty(module)
            .map_err(|e| crate::error::ClsError::CompileError(e.to_string()))
    }
}

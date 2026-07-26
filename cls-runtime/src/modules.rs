use crate::error::ClsResult;
use crate::value::Value;
use std::collections::HashMap;

/// Gestor de módulos WASM y .clsapp
pub struct ModuleManager {
    loaded: HashMap<String, Value>,
}

impl ModuleManager {
    pub fn new() -> Self {
        Self {
            loaded: HashMap::new(),
        }
    }

    pub fn load_app(&mut self, _path: &str) -> ClsResult<()> {
        // TODO: cargar .clsapp y ejecutar
        Ok(())
    }
}

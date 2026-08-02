use crate::value::Value;
use crate::environment::Environment;
use cls_core::error::{ClsError, ClsResult};
use std::collections::HashMap;

/// Resolvedor de módulos — configurado por el nodo.
///
/// Orden: cache → internals → external hook → error.
pub struct ModuleResolver {
    /// Módulos internos (Map de nombre → módulo)
    internals: HashMap<String, Value>,
    /// Hook externo: (path, env) → Ok(Some(module)) | Ok(None) | Err(error)
    external: Option<Box<dyn Fn(String, &mut Environment) -> ClsResult<Option<Value>>>>,
    /// Caché de módulos ya importados
    cache: HashMap<String, Value>,
}

impl ModuleResolver {
    pub fn new() -> Self {
        Self {
            internals: HashMap::new(),
            external: None,
            cache: HashMap::new(),
        }
    }

    pub fn with_core_stdlib(mut self) -> Self {
        self.internals.insert("math".into(), crate::stdlib::math::module());
        self.internals.insert("json".into(), crate::stdlib::json::module());
        self.internals.insert("async".into(), crate::stdlib::async_::module());
        self
    }

    pub fn add_internal(&mut self, name: &str, module: Value) {
        self.internals.insert(name.into(), module);
    }

    /// Hook para módulos externos (.ccls, .clsapp, etc.)
    /// Debe retornar Ok(Some(module)) si se encontró, Ok(None) si no, Err si hubo error.
    pub fn set_external<F>(&mut self, resolver: F)
    where
        F: Fn(String, &mut Environment) -> ClsResult<Option<Value>> + 'static,
    {
        self.external = Some(Box::new(resolver));
    }

    /// Resuelve un módulo: cache → internals → external → error
    pub fn resolve(&mut self, path: &str, env: &mut Environment) -> ClsResult<Value> {
        // 1. Caché
        if let Some(m) = self.cache.get(path) {
            return Ok(m.clone());
        }

        // 2. Internals
        if let Some(m) = self.internals.get(path) {
            self.cache.insert(path.into(), m.clone());
            return Ok(m.clone());
        }

        // 3. External hook
        if let Some(ref hook) = self.external {
            match hook(path.to_string(), env) {
                Ok(Some(m)) => {
                    self.cache.insert(path.into(), m.clone());
                    return Ok(m);
                }
                Ok(None) => {} // No encontrado, continuar
                Err(e) => {
                    return Err(ClsError::RuntimeError(format!(
                        "Error en '{}': {}", path, e
                    )));
                }
            }
        }

        // 4. No encontrado
        Err(ClsError::RuntimeError(format!(
            "Módulo '{}' no encontrado", path
        )))
    }
}

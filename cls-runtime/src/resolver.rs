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
    /// Hook externo: (path, env) → Option<module>
    external: Option<Box<dyn Fn(String, &mut Environment) -> Option<Value>>>,
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

    /// Agrega las stdlib core (math, json)
    pub fn with_core_stdlib(mut self) -> Self {
        self.internals.insert("math".into(), crate::stdlib::math::module());
        self.internals.insert("json".into(), crate::stdlib::json::module());
        self
    }

    /// Agrega/sobrescribe un módulo interno
    pub fn add_internal(&mut self, name: &str, module: Value) {
        self.internals.insert(name.into(), module);
    }

    /// Hook para módulos externos (.ccls, .clsapp, etc.)
    pub fn set_external<F>(&mut self, resolver: F)
    where
        F: Fn(String, &mut Environment) -> Option<Value> + 'static,
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
            if let Some(m) = hook(path.to_string(), env) {
                self.cache.insert(path.into(), m.clone());
                return Ok(m);
            }
        }

        // 4. No encontrado
        Err(ClsError::RuntimeError(format!(
            "Módulo '{}' no encontrado", path
        )))
    }
}

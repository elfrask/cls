use crate::walker::value::Value;
use crate::walker::environment::Environment;
use cls_core::error::{ClsError, ClsResult};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Directorio global de módulos de usuario: `~/.cls/modules/`.
/// Aquí viven los módulos instalados por el usuario (no del registry).
pub fn user_modules_dir() -> Option<PathBuf> {
    let base = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(PathBuf::from)
        .ok()?;
    Some(base.join(".cls").join("modules"))
}

/// Directorio global del registry (módulos globales instalados vía `clx install`).
pub fn global_modules_dir() -> Option<PathBuf> {
    user_modules_dir()
}

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
    /// Directorio base para resolver imports relativos (dir del archivo que
    /// importa, o del proyecto/cls.json). Lo actualiza el runtime al ejecutar.
    base_dir: Option<PathBuf>,
}

impl ModuleResolver {
    pub fn new() -> Self {
        Self {
            internals: HashMap::new(),
            external: None,
            cache: HashMap::new(),
            base_dir: None,
        }
    }

    pub fn with_core_stdlib(mut self) -> Self {
        self.internals.insert("math".into(), crate::walker::stdlib::math::module());
        self.internals.insert("json".into(), crate::walker::stdlib::json::module());
        self.internals.insert("async".into(), crate::walker::stdlib::async_::module());
        self
    }

    pub fn add_internal(&mut self, name: &str, module: Value) {
        self.internals.insert(name.into(), module);
    }

    /// Directorio base actual (dir del archivo que importa).
    pub fn base_dir(&self) -> Option<&Path> {
        self.base_dir.as_deref()
    }

    /// Actualiza el directorio base (dir del archivo fuente actual).
    pub fn set_base_dir(&mut self, dir: PathBuf) {
        self.base_dir = Some(dir);
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

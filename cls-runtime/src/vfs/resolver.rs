use std::collections::HashMap;
use std::sync::Arc;

use cls_core::error::{ClsError, ClsResult};

use super::protocol::VfsProtocol;

/// Resolvedor de protocolos VFS.
/// Registra protocolos (res, app, user, tmp, rutas personalizadas) y resuelve paths.
pub struct VfsResolver {
    protocols: HashMap<String, Arc<dyn VfsProtocol>>,
    /// Rutas personalizadas definidas en module.clsconfig -> routes
    custom_routes: HashMap<String, String>,
    /// Protocolos reservados que no pueden ser sobrescritos
    reserved: Vec<String>,
}

impl VfsResolver {
    pub fn new() -> Self {
        Self {
            protocols: HashMap::new(),
            custom_routes: HashMap::new(),
            reserved: vec!["res".into(), "app".into(), "user".into(), "tmp".into()],
        }
    }

    /// Registra un protocolo
    pub fn register(&mut self, name: &str, proto: Arc<dyn VfsProtocol>) {
        self.protocols.insert(name.to_string(), proto);
    }

    /// Agrega una ruta personalizada (de module.clsconfig -> routes)
    pub fn add_route(&mut self, name: &str, target: &str) -> ClsResult<()> {
        if self.reserved.contains(&name.to_string()) {
            return Err(ClsError::RuntimeError(format!(
                "No se puede sobrescribir el protocolo reservado: {}", name
            )));
        }
        self.custom_routes.insert(name.to_string(), target.to_string());
        Ok(())
    }

    /// Resuelve un path como "app://config.json" o "./file.txt"
    /// Devuelve (protocol_impl, path_resuelto_sin_protocolo)
    pub fn resolve(&self, path: &str) -> ClsResult<(Arc<dyn VfsProtocol>, String)> {
        // Si tiene protocolo: proto://ruta
        if let Some((proto, rest)) = path.split_once("://") {
            // Buscar ruta personalizada primero
            if let Some(target) = self.custom_routes.get(proto) {
                // Si la ruta personalizada apunta a otro protocolo
                if target.contains("://") {
                    return self.resolve(&format!("{}{}", target, rest));
                }
                // Si es relativa, delegar a app://
                if let Some(app) = self.protocols.get("app") {
                    let full_path = format!("{}/{}", target.trim_end_matches('/'), rest.trim_start_matches('/'));
                    return Ok((app.clone(), full_path));
                }
            }

            // Buscar protocolo registrado
            if let Some(proto_impl) = self.protocols.get(proto) {
                return Ok((proto_impl.clone(), rest.to_string()));
            }

            return Err(ClsError::RuntimeError(format!("Protocolo no soportado: {}", proto)));
        }

        // Sin protocolo: relativo al cwd (usar app://)
        if let Some(app) = self.protocols.get("app") {
            return Ok((app.clone(), path.to_string()));
        }

        Err(ClsError::RuntimeError("Ningun protocolo disponible".to_string()))
    }

    /// Lee un archivo usando el protocolo adecuado
    pub fn read_file(&self, path: &str) -> ClsResult<Vec<u8>> {
        let (proto, resolved) = self.resolve(path)?;
        proto.read(&resolved)
    }

    pub fn read_to_string(&self, path: &str) -> ClsResult<String> {
        let (proto, resolved) = self.resolve(path)?;
        proto.read_to_string(&resolved)
    }

    pub fn write_file(&self, path: &str, data: &[u8]) -> ClsResult<()> {
        let (proto, resolved) = self.resolve(path)?;
        proto.write(&resolved, data)
    }

    pub fn exists(&self, path: &str) -> bool {
        if let Ok((proto, resolved)) = self.resolve(path) {
            proto.exists(&resolved)
        } else {
            false
        }
    }

    pub fn list_dir(&self, path: &str) -> ClsResult<Vec<String>> {
        let (proto, resolved) = self.resolve(path)?;
        proto.list_dir(&resolved)
    }

    pub fn remove(&self, path: &str) -> ClsResult<()> {
        let (proto, resolved) = self.resolve(path)?;
        if proto.is_read_only() {
            return Err(ClsError::RuntimeError(format!("{} es read-only", proto.name())));
        }
        let full = std::path::Path::new(&resolved);
        if full.exists() && full.is_dir() {
            std::fs::remove_dir_all(full)
                .map_err(|e| ClsError::RuntimeError(format!("rm: {}", e)))?;
        } else if full.exists() {
            std::fs::remove_file(full)
                .map_err(|e| ClsError::RuntimeError(format!("rm: {}", e)))?;
        }
        Ok(())
    }

    pub fn create_dir(&self, path: &str) -> ClsResult<()> {
        let (proto, resolved) = self.resolve(path)?;
        if proto.is_read_only() {
            return Err(ClsError::RuntimeError(format!("{} es read-only", proto.name())));
        }
        std::fs::create_dir_all(std::path::Path::new(&resolved))
            .map_err(|e| ClsError::RuntimeError(format!("mkdir: {}", e)))?;
        Ok(())
    }
}

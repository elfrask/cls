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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vfs::protocol::LocalFs;
    use std::sync::Arc;

    fn test_resolver() -> VfsResolver {
        let mut v = VfsResolver::new();
        let tmp = std::env::temp_dir().join("cls-vfs-test");
        std::fs::create_dir_all(&tmp).ok();
        v.register("app", Arc::new(LocalFs::new("app", &tmp, false)));
        v
    }

    #[test]
    fn test_resolve_noproto() {
        let v = test_resolver();
        let (proto, path) = v.resolve("file.txt").unwrap();
        assert_eq!(proto.name(), "app");
        assert_eq!(path, "file.txt");
    }

    #[test]
    fn test_resolve_proto() {
        let v = test_resolver();
        let (proto, path) = v.resolve("app://sub/file.txt").unwrap();
        assert_eq!(proto.name(), "app");
        assert_eq!(path, "sub/file.txt");
    }

    #[test]
    fn test_resolve_unknown_proto() {
        let v = test_resolver();
        assert!(v.resolve("bad://file.txt").is_err());
    }

    #[test]
    fn test_write_and_read() {
        let v = test_resolver();
        v.write_file("test.txt", b"hello vfs").unwrap();
        let data = v.read_file("test.txt").unwrap();
        assert_eq!(data, b"hello vfs");
    }

    #[test]
    fn test_read_to_string() {
        let v = test_resolver();
        v.write_file("greeting.txt", b"hola mundo").unwrap();
        let s = v.read_to_string("greeting.txt").unwrap();
        assert_eq!(s, "hola mundo");
    }

    #[test]
    fn test_exists() {
        let v = test_resolver();
        v.write_file("exists_test.txt", b"").unwrap();
        assert!(v.exists("exists_test.txt"));
        assert!(!v.exists("no_such_file.txt"));
    }

    #[test]
    fn test_list_dir() {
        let v = test_resolver();
        v.write_file("dir_list_a.txt", b"").unwrap();
        v.write_file("dir_list_b.txt", b"").unwrap();
        let entries = v.list_dir(".").unwrap();
        assert!(entries.contains(&"dir_list_a.txt".to_string()));
        assert!(entries.contains(&"dir_list_b.txt".to_string()));
    }

    #[test]
    fn test_proto_write_and_read() {
        let v = test_resolver();
        v.write_file("app://proto_test.txt", b"proto").unwrap();
        let data = v.read_to_string("app://proto_test.txt").unwrap();
        assert_eq!(data, "proto");
    }

    #[test]
    fn test_custom_route() {
        let mut v = test_resolver();
        v.add_route("data", "app://custom/").unwrap();
        let (proto, path) = v.resolve("data://config.json").unwrap();
        assert_eq!(proto.name(), "app");
        assert_eq!(path, "custom/config.json");
    }

    #[test]
    fn test_readonly_deny_write() {
        let mut v = VfsResolver::new();
        let tmp = std::env::temp_dir().join("cls-vfs-readonly-test");
        std::fs::create_dir_all(&tmp).ok();
        v.register("ro", Arc::new(LocalFs::new("ro", &tmp, true)));
        assert!(v.write_file("ro://x.txt", b"data").is_err());
    }
}

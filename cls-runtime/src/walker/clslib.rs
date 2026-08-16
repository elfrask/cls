use std::collections::HashMap;

use cls_core::error::ClsResult;
use serde::{Deserialize, Serialize};

/// Computa el hash SHA-256 de un slice de bytes (no hace I/O).
pub fn compute_hash_bytes(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

/// Índice de .clslib (index.json)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClsLibIndex {
    pub version: u32,
    pub libraries: HashMap<String, ClsLibEntry>,
}

/// Entrada individual en el índice
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClsLibEntry {
    pub hash: String,
    pub path: String,
    pub size: u64,
    pub added: String,
    pub sources: Vec<String>,
}

impl ClsLibIndex {
    pub fn new() -> Self {
        Self { version: 1, libraries: HashMap::new() }
    }

    pub fn find(&self, name: &str) -> Option<&ClsLibEntry> {
        self.libraries.get(name)
    }

    pub fn register(&mut self, name: &str, hash: &str, path: &str, size: u64, source: &str) {
        let entry = ClsLibEntry {
            hash: hash.to_string(),
            path: path.to_string(),
            size,
            added: format!("{:?}", std::time::SystemTime::now()),
            sources: vec![source.to_string()],
        };
        self.libraries.insert(name.to_string(), entry);
    }
}

impl Default for ClsLibIndex {
    fn default() -> Self {
        Self::new()
    }
}

/// Resolvedor de .clslib - cada nodo implementa con su propio VFS/filesystem.
/// Nunca usa std::fs; los nodos inyectan la lógica de I/O.
pub trait ClsLibResolver: Send + Sync {
    /// Resuelve un nombre de librería (sin extensión, ej: "foo") y devuelve
    /// el contenido binario del .clslib si se encuentra.
    fn resolve(&self, name: &str) -> ClsResult<Option<Vec<u8>>>;
}

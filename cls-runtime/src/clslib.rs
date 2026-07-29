use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::io::Read;

use cls_core::error::{ClsError, ClsResult};
use serde::{Deserialize, Serialize};

/// Computa el hash SHA-256 de un archivo
pub fn compute_hash(path: &Path) -> ClsResult<String> {
    use sha2::{Digest, Sha256};
    let mut file = fs::File::open(path)
        .map_err(|e| ClsError::RuntimeError(format!("Hash: no se puede abrir {}: {}", path.display(), e)))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = file.read(&mut buf)
            .map_err(|e| ClsError::RuntimeError(format!("Hash: error leyendo: {}", e)))?;
        if n == 0 { break; }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClsLibIndex {
    pub version: u32,
    pub libraries: HashMap<String, ClsLibEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClsLibEntry {
    pub hash: String,
    pub path: String,
    pub size: u64,
    pub added: String,
    pub sources: Vec<String>,
}

impl ClsLibIndex {
    pub fn load_or_create(path: &Path) -> Self {
        if path.exists() {
            let content = fs::read_to_string(path).unwrap_or_default();
            serde_json::from_str(&content).unwrap_or_else(|_| Self::new())
        } else {
            Self::new()
        }
    }

    pub fn new() -> Self {
        Self { version: 1, libraries: HashMap::new() }
    }

    pub fn save(&self, path: &Path) -> ClsResult<()> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| ClsError::RuntimeError(format!("index.json: {}", e)))?;
        fs::write(path, content)
            .map_err(|e| ClsError::RuntimeError(format!("index.json: {}", e)))?;
        Ok(())
    }

    /// Busca un .clslib por nombre en el index
    pub fn find(&self, name: &str) -> Option<&ClsLibEntry> {
        self.libraries.get(name)
    }

    /// Registra un .clslib en el index
    pub fn register(&mut self, name: &str, hash: &str, lib_path: &Path, source: &str) {
        let path_rel = lib_path.to_string_lossy().to_string();
        let size = fs::metadata(lib_path).map(|m| m.len()).unwrap_or(0);
        let added = format!("{:?}", std::time::SystemTime::now());

        let entry = ClsLibEntry {
            hash: hash.to_string(),
            path: path_rel,
            size,
            added,
            sources: vec![source.to_string()],
        };

        self.libraries.insert(name.to_string(), entry);
    }
}

/// Registro global de .clslib (~/.cls/clslibs/)
pub struct ClsLibRegistry {
    pub base: PathBuf,          // ~/.cls/clslibs/
    pub by_hash: PathBuf,       // by-hash/
    pub names: PathBuf,         // names/
    pub index_path: PathBuf,    // index.json
    pub index: ClsLibIndex,
}

impl ClsLibRegistry {
    pub fn open(base: &Path) -> Self {
        let base = base.to_path_buf();
        let by_hash = base.join("by-hash");
        let names = base.join("names");
        let index_path = base.join("index.json");

        fs::create_dir_all(&by_hash).ok();
        fs::create_dir_all(&names).ok();

        let index = ClsLibIndex::load_or_create(&index_path);

        Self { base, by_hash, names, index_path, index }
    }

    /// Instala un .clslib desde un path
    pub fn install(&mut self, name: &str, source: &Path, origin: &str) -> ClsResult<()> {
        let hash = compute_hash(source)?;
        let hash_dir = self.by_hash.join(&hash);
        fs::create_dir_all(&hash_dir).ok();

        let target = hash_dir.join(format!("{}.clslib", name));
        fs::copy(source, &target)
            .map_err(|e| ClsError::RuntimeError(format!("ClsLib install: {}", e)))?;

        // Actualizar index
        self.index.register(name, &hash, &target, origin);
        self.index.save(&self.index_path)?;

        // Crear symlink en names/
        let symlink = self.names.join(format!("{}.clslib", name));
        if symlink.exists() { fs::remove_file(&symlink).ok(); }
        #[cfg(not(target_os = "windows"))]
        std::os::unix::fs::symlink(&target, &symlink).ok();
        #[cfg(target_os = "windows")]
        fs::copy(&target, &symlink).ok();

        Ok(())
    }

    /// Encuentra el path de un .clslib por nombre
    pub fn find(&self, name: &str) -> Option<PathBuf> {
        let name = name.trim_end_matches(".clslib");
        // Primero buscar en names/
        let named = self.names.join(format!("{}.clslib", name));
        if named.exists() {
            return Some(named);
        }
        // Luego buscar en by-hash via index
        if let Some(entry) = self.index.find(name) {
            let hash_path = self.by_hash.join(&entry.hash).join(format!("{}.clslib", name));
            if hash_path.exists() {
                return Some(hash_path);
            }
        }
        None
    }
}

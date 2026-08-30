use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use cls_core::error::{ClsError, ClsResult};

/// Trait que todo protocolo VFS debe implementar
pub trait VfsProtocol: Send + Sync {
    /// Lee un archivo como bytes
    fn read(&self, path: &str) -> ClsResult<Vec<u8>>;

    /// Lee un archivo como string
    fn read_to_string(&self, path: &str) -> ClsResult<String> {
        let bytes = self.read(path)?;
        String::from_utf8(bytes).map_err(|e| ClsError::RuntimeError(format!("UTF-8 invalido: {}", e)))
    }

    /// Escribe un archivo (solo RW protocols)
    fn write(&self, path: &str, data: &[u8]) -> ClsResult<()>;

    /// Verifica si existe
    fn exists(&self, path: &str) -> bool;

    /// Lista un directorio
    fn list_dir(&self, path: &str) -> ClsResult<Vec<String>>;

    /// Nombre del protocolo (res, app, user, tmp, etc.)
    fn name(&self) -> &str;

    /// Si es read-only (como res://)
    fn is_read_only(&self) -> bool;
}

/// Implementacion de filesystem local (app://, user://, tmp://)
pub struct LocalFs {
    name: String,
    base: PathBuf,
    read_only: bool,
}

impl LocalFs {
    pub fn new(name: &str, base: &Path, read_only: bool) -> Self {
        Self {
            name: name.to_string(),
            base: base.to_path_buf(),
            read_only,
        }
    }

    fn resolve(&self, path: &str) -> ClsResult<PathBuf> {
        crate::vfs::security::resolve_safe(path, &self.base)
    }
}

impl VfsProtocol for LocalFs {
    fn name(&self) -> &str { &self.name }
    fn is_read_only(&self) -> bool { self.read_only }

    fn read(&self, path: &str) -> ClsResult<Vec<u8>> {
        let full = self.resolve(path)?;
        fs::read(&full).map_err(|e| ClsError::RuntimeError(format!("{}: {}: {}", self.name, path, e)))
    }

    fn write(&self, path: &str, data: &[u8]) -> ClsResult<()> {
        if self.read_only {
            return Err(ClsError::RuntimeError(format!("{} es read-only", self.name)));
        }
        let full = self.resolve(path)?;
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent).ok();
        }
        fs::write(&full, data).map_err(|e| ClsError::RuntimeError(format!("{}: {}: {}", self.name, path, e)))
    }

    fn exists(&self, path: &str) -> bool {
        self.resolve(path).map(|p| p.exists()).unwrap_or(false)
    }

    fn list_dir(&self, path: &str) -> ClsResult<Vec<String>> {
        let full = self.resolve(path)?;
        let entries = fs::read_dir(&full)
            .map_err(|e| ClsError::RuntimeError(format!("{}: {}: {}", self.name, path, e)))?;
        let mut result = Vec::new();
        for entry in entries.flatten() {
            result.push(entry.file_name().to_string_lossy().to_string());
        }
        Ok(result)
    }
}

/// Implementacion de filesystem zip (res:// dentro de .clsapp)
pub struct ZipFs {
    name: String,
    archive: Option<std::sync::Mutex<zip::ZipArchive<std::fs::File>>>,
    /// Cache LRU para archivos frecuentes
    cache: Option<std::sync::Mutex<std::collections::HashMap<String, Vec<u8>>>>,
}

impl ZipFs {
    /// Crea un ZipFs desde un archivo .clsapp
    pub fn open(name: &str, path: &Path) -> ClsResult<Self> {
        let file = fs::File::open(path)
            .map_err(|e| ClsError::RuntimeError(format!("No se puede abrir {}: {}", path.display(), e)))?;
        let archive = zip::ZipArchive::new(file)
            .map_err(|e| ClsError::RuntimeError(format!("Zip invalido: {}", e)))?;
        Ok(Self {
            name: name.to_string(),
            archive: Some(std::sync::Mutex::new(archive)),
            cache: Some(std::sync::Mutex::new(std::collections::HashMap::new())),
        })
    }

    /// Crea un ZipFs vacio (placeholder, modo desarrollo)
    pub fn empty() -> Self {
        Self { name: "res".to_string(), archive: None, cache: None }
    }

    fn read_from_zip(&self, path: &str) -> ClsResult<Vec<u8>> {
        if let Some(ref archive) = self.archive {
            let mut zip = archive.lock().unwrap();
            let mut file = zip.by_name(path)
                .map_err(|_| ClsError::RuntimeError(format!("res://{} no encontrado", path)))?;
            let mut buf = Vec::new();
            file.read_to_end(&mut buf)
                .map_err(|e| ClsError::RuntimeError(format!("res://{}: {}", path, e)))?;
            Ok(buf)
        } else {
            Err(ClsError::RuntimeError("res:// no disponible".to_string()))
        }
    }
}

impl VfsProtocol for ZipFs {
    fn name(&self) -> &str { &self.name }
    fn is_read_only(&self) -> bool { true }

    fn read(&self, path: &str) -> ClsResult<Vec<u8>> {
        // Intentar cache primero
        if let Some(ref cache) = self.cache {
            if let Some(data) = cache.lock().unwrap().get(path) {
                return Ok(data.clone());
            }
        }

        let data = self.read_from_zip(path)?;

        // Cachear
        if let Some(ref cache) = self.cache {
            let mut c = cache.lock().unwrap();
            if c.len() < 256 {
                c.insert(path.to_string(), data.clone());
            }
        }

        Ok(data)
    }

    fn write(&self, _path: &str, _data: &[u8]) -> ClsResult<()> {
        Err(ClsError::RuntimeError("res:// es read-only".to_string()))
    }

    fn exists(&self, path: &str) -> bool {
        if let Some(ref archive) = self.archive {
            archive.lock().unwrap().by_name(path).is_ok()
        } else {
            false
        }
    }

    fn list_dir(&self, _path: &str) -> ClsResult<Vec<String>> {
        Err(ClsError::RuntimeError("res:// listDir no implementado".to_string()))
    }
}

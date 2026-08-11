use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use super::types::*;

/// Manifiesto único de proyecto CLS (cls.json).
/// Fusiona los metadatos de proyecto + compilador + intérprete en un solo archivo.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleManifest {
    /// Nombre del proyecto
    pub name: String,

    /// Versión (semver)
    pub version: String,

    #[serde(default)]
    pub description: String,

    #[serde(default)]
    pub authors: Vec<String>,

    #[serde(default)]
    pub license: String,

    /// Registry URL para dependencias
    #[serde(default = "default_registry")]
    pub registry: String,

    /// Punto de entrada principal
    #[serde(default = "default_entry")]
    pub entry: String,

    /// Configuración del proyecto
    #[serde(default)]
    pub project: ProjectConfig,

    /// Configuración del compilador
    #[serde(default)]
    pub compiler: CompilerConfig,

    /// Configuración del intérprete
    #[serde(default)]
    pub interpreter: InterpreterConfig,

    /// Dependencias del proyecto
    #[serde(default)]
    pub dependencies: HashMap<String, String>,

    /// Dependencias de desarrollo (camelCase para compat con formato anterior)
    #[serde(default, rename = "devDependencies")]
    pub dev_dependencies: HashMap<String, String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lockfile_version: Option<u32>,
}

fn default_registry() -> String {
    "https://registry.cls-lang.org".to_string()
}

fn default_entry() -> String {
    "src/main.clsx".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectConfig {
    #[serde(default = "default_source")]
    pub source_dir: String,

    #[serde(default = "default_out")]
    pub out_dir: String,

    #[serde(default = "default_target")]
    pub target: String,
}

fn default_source() -> String { "src".to_string() }
fn default_out() -> String { "dist".to_string() }
fn default_target() -> String { "executable".to_string() }

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            source_dir: default_source(),
            out_dir: default_out(),
            target: default_target(),
        }
    }
}

impl ModuleManifest {
    /// Carga el manifiesto desde `cls.json`
    pub fn from_file(path: &Path) -> crate::error::ClsResult<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| crate::error::ClsError::ConfigError(e.to_string()))?;
        let manifest: Self = serde_json::from_str(&content)
            .map_err(|e| crate::error::ClsError::ConfigError(
                format!("Error en cls.json: {} (en {})", e, path.display())
            ))?;
        Ok(manifest)
    }

    /// Guarda el manifiesto en `cls.json`
    pub fn save(&self, path: &Path) -> crate::error::ClsResult<()> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| crate::error::ClsError::ConfigError(e.to_string()))?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Crea un manifiesto por defecto para un proyecto nuevo
    pub fn default_for(name: &str) -> Self {
        Self {
            name: name.to_string(),
            version: "0.1.0".to_string(),
            description: String::new(),
            authors: Vec::new(),
            license: "MIT".to_string(),
            registry: default_registry(),
            entry: default_entry(),
            project: ProjectConfig::default(),
            compiler: CompilerConfig::default(),
            interpreter: InterpreterConfig::default(),
            dependencies: HashMap::new(),
            dev_dependencies: HashMap::new(),
            lockfile_version: None,
        }
    }

    /// Busca cls.json desde el directorio actual hacia arriba (o usa defaults)
    pub fn find_and_load() -> crate::error::ClsResult<Self> {
        let cwd = std::env::current_dir().unwrap_or_default();
        let path = cwd.join("cls.json");
        if path.exists() {
            Self::from_file(&path)
        } else {
            // Devolver un manifiesto por defecto con nombre del directorio
            let name = cwd.file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "project".to_string());
            Ok(Self::default_for(&name))
        }
    }

    /// Busca el cls.json de un proyecto desde un directorio hacia arriba.
    pub fn find_in_dir(start: &Path) -> Option<Self> {
        let mut dir = Some(start.to_path_buf());
        while let Some(d) = dir {
            let candidate = d.join("cls.json");
            if candidate.exists() {
                if let Ok(m) = Self::from_file(&candidate) {
                    return Some(m);
                }
            }
            dir = d.parent().map(|p| p.to_path_buf());
        }
        None
    }

    /// La versión exacta de una dependencia declarada (nombre → semver).
    /// Devuelve el rango declarado (p.ej. `^1.2.0`); la resolución exacta del
    /// semver se hace contra el almacén global. Si no está declarada → None.
    pub fn dependency_version(&self, name: &str) -> Option<&str> {
        self.dependencies
            .get(name)
            .map(|s| s.as_str())
            .or_else(|| self.dev_dependencies.get(name).map(|s| s.as_str()))
    }

    /// Nombres de módulos en el workspace `{base}/modules/{name}/mod.clsx`.
    /// Devuelve true si un módulo está declarado como dependencia (para priorizarlo
    /// frente a los globales sin versión).
    pub fn is_dependency(&self, name: &str) -> bool {
        self.dependencies.contains_key(name) || self.dev_dependencies.contains_key(name)
    }
}

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use super::types::*;

/// Manifiesto completo de un módulo/proyecto CLS
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleManifest {
    /// Nombre del módulo
    pub name: String,

    /// Versión (semver)
    pub version: String,

    /// Descripción
    #[serde(default)]
    pub description: String,

    /// Autores
    #[serde(default)]
    pub authors: Vec<String>,

    /// Licencia (SPDX)
    #[serde(default)]
    pub license: String,

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

    /// Dependencias de desarrollo
    #[serde(default)]
    pub dev_dependencies: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// Punto de entrada principal
    #[serde(default = "default_entry")]
    pub entry: String,

    /// Directorio del código fuente
    #[serde(default = "default_source")]
    pub source_dir: String,

    /// Directorio de salida
    #[serde(default = "default_out")]
    pub out_dir: String,

    /// Target: "executable", "library", "dynamic-lib"
    #[serde(default = "default_target")]
    pub target: String,
}

fn default_entry() -> String {
    "src/main.ccls".to_string()
}

fn default_source() -> String {
    "src".to_string()
}

fn default_out() -> String {
    "dist".to_string()
}

fn default_target() -> String {
    "executable".to_string()
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            entry: default_entry(),
            source_dir: default_source(),
            out_dir: default_out(),
            target: default_target(),
        }
    }
}

impl ModuleManifest {
    /// Carga un manifiesto desde un archivo `module.clsconfig`
    pub fn from_file(path: &Path) -> crate::error::ClsResult<Self> {
        let content = std::fs::read_to_string(path)?;
        let manifest: Self = serde_json::from_str(&content)
            .map_err(|e| crate::error::ClsError::ConfigError(e.to_string()))?;
        Ok(manifest)
    }

    /// Guarda el manifiesto en un archivo
    pub fn save(&self, path: &Path) -> crate::error::ClsResult<()> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| crate::error::ClsError::ConfigError(e.to_string()))?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Crea un manifiesto por defecto
    pub fn default_for(name: &str) -> Self {
        Self {
            name: name.to_string(),
            version: "0.1.0".to_string(),
            description: String::new(),
            authors: Vec::new(),
            license: "MIT".to_string(),
            project: ProjectConfig::default(),
            compiler: CompilerConfig::default(),
            interpreter: InterpreterConfig::default(),
            dependencies: HashMap::new(),
            dev_dependencies: HashMap::new(),
        }
    }
}

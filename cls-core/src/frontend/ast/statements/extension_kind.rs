//! AST - ExtensionKind (Fase 1: extraido de frontend/ast.rs).

use serde::{Deserialize, Serialize};


/// Tipo de extensión (backend nativo). Enum fijo para los conocidos (rendimiento)
/// + `Custom` para tipos futuros sin tocar el core.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExtensionKind {
    C,
    Python,
    Wasm,
    Js,
    Wasi,
    Custom(String),
}


impl ExtensionKind {
    pub fn from_name(s: &str) -> Self {
        match s {
            "C" | "c" => ExtensionKind::C,
            "Python" | "python" => ExtensionKind::Python,
            "Wasm" | "wasm" => ExtensionKind::Wasm,
            "Js" | "js" | "JS" => ExtensionKind::Js,
            "Wasi" | "wasi" => ExtensionKind::Wasi,
            other => ExtensionKind::Custom(other.to_string()),
        }
    }

    pub fn name(&self) -> String {
        match self {
            ExtensionKind::C => "C".to_string(),
            ExtensionKind::Python => "Python".to_string(),
            ExtensionKind::Wasm => "Wasm".to_string(),
            ExtensionKind::Js => "Js".to_string(),
            ExtensionKind::Wasi => "Wasi".to_string(),
            ExtensionKind::Custom(s) => s.clone(),
        }
    }
}

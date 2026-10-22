//! Snapshot de configuración del proyecto expuesto al lenguaje por el intrinsic
//! `config()` (solo lectura).
//!
//! El **nodo** (`clx`) arma el snapshot desde el `cls.json` del proyecto del
//! entry + los flags de CLI y lo fija aquí antes de ejecutar. El host function
//! `env.config_record` lo lee y lo materializa como `Record<String, String>` en
//! la memoria lineal del módulo (una sola vez; ver [`crate::host`]).
//!
//! Análogo a [`crate::limits`]: un `OnceLock` global, idempotente, por proceso.

use std::sync::OnceLock;

static CONFIG: OnceLock<Vec<(String, String)>> = OnceLock::new();

/// Fija el snapshot de configuración (claves planas con puntos, valores String).
/// Idempotente: la primera llamada gana (el proceso ejecuta un solo programa).
pub fn set_config(pairs: Vec<(String, String)>) {
    let _ = CONFIG.set(pairs);
}

/// Snapshot de configuración vigente (vacío si el nodo no lo fijó).
pub fn config() -> &'static [(String, String)] {
    CONFIG.get().map(|v| v.as_slice()).unwrap_or(&[])
}

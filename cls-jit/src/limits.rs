//! Límites de recursos del runtime (sandbox): memoria lineal y tiempo de
//! ejecución.
//!
//! Los fija el **nodo** antes de ejecutar, desde:
//!   1. Env `CLS_MEMORY_LIMIT` / `CLS_TIME_LIMIT` (mayor prioridad).
//!   2. `cls.json` -> `interpreter.runtime.memoryLimit`.
//!   3. Default: **1 GiB** de memoria, sin límite de tiempo.
//!
//! La memoria se aplica como `StoreLimits` de wasmtime (limita el crecimiento
//! de las memorias lineales). `0` = sin límite.

use std::sync::OnceLock;

/// Límite de memoria por defecto: 1 GiB.
pub const DEFAULT_MEMORY_LIMIT: u64 = 1024 * 1024 * 1024;

static MEMORY_LIMIT: OnceLock<u64> = OnceLock::new();
static TIME_LIMIT: OnceLock<u64> = OnceLock::new();

/// Fija el límite de memoria (bytes; `0` = sin límite). Idempotente.
pub fn set_memory_limit(bytes: u64) {
    let _ = MEMORY_LIMIT.set(bytes);
}

/// Límite de memoria vigente (bytes; `0` = sin límite).
pub fn memory_limit() -> u64 {
    MEMORY_LIMIT.get().copied().unwrap_or(DEFAULT_MEMORY_LIMIT)
}

/// Fija el límite de tiempo de ejecución (ms; `0` = sin límite). Idempotente.
pub fn set_time_limit(ms: u64) {
    let _ = TIME_LIMIT.set(ms);
}

/// Límite de tiempo vigente (ms; `0` = sin límite).
pub fn time_limit() -> u64 {
    TIME_LIMIT.get().copied().unwrap_or(0)
}

/// Parsea un tamaño tipo `"512MB"`, `"1GB"`, `"1024KB"`, `"1GiB"`, `"500"`.
/// Acepta sufijos decimales (KB/MB/GB) y binarios (KiB/MiB/GiB). Devuelve bytes.
pub fn parse_size(s: &str) -> Option<u64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let (num, unit) = match s.find(|c: char| !(c.is_ascii_digit() || c == '.')) {
        Some(idx) => (&s[..idx], &s[idx..]),
        None => (s, ""),
    };
    let n: f64 = num.parse().ok()?;
    if !n.is_finite() || n < 0.0 {
        return None;
    }
    let mult = match unit.trim().to_ascii_uppercase().as_str() {
        "" | "B" => 1f64,
        "K" | "KB" => 1_000f64,
        "KI" | "KIB" => 1_024f64,
        "M" | "MB" => 1_000_000f64,
        "MI" | "MIB" => 1_024f64 * 1_024f64,
        "G" | "GB" => 1_000_000_000f64,
        "GI" | "GIB" => 1_024f64 * 1_024f64 * 1_024f64,
        _ => return None,
    };
    Some((n * mult) as u64)
}

/// Lee `interpreter.runtime.memoryLimit` del `cls.json` del proyecto del entry
/// (buscando hacia arriba desde su directorio). `None` si no lo declara.
pub fn manifest_memory_limit(entry: &str) -> Option<u64> {
    let dir = std::path::Path::new(entry).parent()?;
    let m = cls_core::config::ModuleManifest::find_in_dir(dir)?;
    parse_size(&m.interpreter.runtime.memory_limit)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_sizes() {
        assert_eq!(parse_size("500"), Some(500));
        assert_eq!(parse_size("512MB"), Some(512_000_000));
        assert_eq!(parse_size("1GB"), Some(1_000_000_000));
        assert_eq!(parse_size("2GiB"), Some(2 * 1024 * 1024 * 1024));
        assert_eq!(parse_size("1024KB"), Some(1_024_000));
        assert_eq!(parse_size("64KIB"), Some(65_536));
        assert_eq!(parse_size("1.5GB"), Some(1_500_000_000));
        assert_eq!(parse_size(""), None);
        assert_eq!(parse_size("abc"), None);
        assert_eq!(parse_size("10XB"), None);
    }

    #[test]
    fn default_is_one_gib() {
        assert_eq!(DEFAULT_MEMORY_LIMIT, 1_073_741_824);
    }
}

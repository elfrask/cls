//! Indexación de módulos para el caché del JIT.
//!
//! El caché CLS->WASM en `~/.cache/cls/` usa una clave hash (`cache_key` en
//! `jit.rs`). Esa clave ya hashea el source del entry y de TODOS los módulos
//! importados (locales y globales de `~/.cls`), de modo que editar cualquier
//! `.clsx` del grafo importado invalida el caché y editar uno NO importado no
//! lo invalida.
//!
//! Este módulo construye un **índice de integridad** del workspace (un hash de
//! contenido de todos los `*.clsx` del proyecto más los módulos globales
//! importados). Es INFORMATIVO: el JIT no lo lee en el HIT (la invalidación ya
//! la cubre `cache_key`); sirve para inspeccionar desde disco qué módulos
//! participan en la compilación y sus hashes.

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

/// Raíz del workspace: el primer dir con `cls.json` subiendo desde `entry`,
/// o el dir del entry si no hay proyecto.
///
/// Se canonicaliza el dir inicial a ABSOLUTO: sin esto, el ascenso atraviesa el
/// componente vacío (`""`) y `"cls.json"` relativo al CWD podía casar un
/// `cls.json` ajeno (raíz del repo) en vez de detenerse en la raíz del sistema.
pub fn workspace_root(entry: &Path) -> PathBuf {
    let start = entry
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    let start = std::fs::canonicalize(&start).unwrap_or(start);
    let mut dir = Some(start.clone());
    while let Some(d) = dir {
        if d.join("cls.json").exists() {
            return d;
        }
        dir = d.parent().map(|p| p.to_path_buf());
    }
    start
}

/// Directorio de caché del workspace: `[workspace]/.cls-cache/`.
pub fn workspace_cache_dir(entry: &Path) -> PathBuf {
    workspace_root(entry).join(".cls-cache")
}

/// Ruta del índice de módulos del workspace.
pub fn module_index_path(entry: &Path) -> PathBuf {
    workspace_cache_dir(entry).join("module-index.json")
}

/// Hash de contenido de un archivo (SHA-256, hexadecimal).
fn file_hash(path: &Path) -> Option<String> {
    let data = std::fs::read(path).ok()?;
    Some(hex_of(&Sha256::digest(&data)))
}

/// Hash simple de un string (SHA-256 hex).
pub fn sha256_hex(s: &str) -> String {
    hex_of(&Sha256::digest(s.as_bytes()))
}

fn hex_of(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

// ── Recorrido del workspace ──────────────────────────────────────────────────

/// Recolecta todos los `*.clsx` bajo un directorio (recursivo), saltando `.cls-cache`.
fn collect_clsx_files(dir: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') || name == "target" || name == "node_modules" {
                continue;
            }
            if path.is_dir() {
                collect_clsx_files(&path, out);
            } else if path.extension().map(|e| e == "clsx").unwrap_or(false) {
                out.push(path);
            }
        }
    }
}

/// Escribe el índice de módulos del workspace en `[workspace]/.cls-cache/module-index.json`.
/// Devuelve la ruta escrita.
///
/// `extra_modules`: rutas de módulos importados fuera del workspace (p.ej.
/// globales `~/.cls/modules/...`). El JIT no lee este índice en el HIT (la
/// invalidación la cubre `cache_key` hasheando los sources importados); es un
/// artefacto informativo para inspección.
pub fn write_module_index(entry: &Path, extra_modules: &[PathBuf]) -> PathBuf {
    let dir = workspace_cache_dir(entry);
    let _ = std::fs::create_dir_all(&dir);
    let root = workspace_root(entry);
    let mut files = Vec::new();
    collect_clsx_files(&root, &mut files);
    let mut rel: Vec<String> = files
        .iter()
        .filter_map(|f| f.strip_prefix(&root).ok().map(|r| r.to_string_lossy().into_owned()))
        .collect();
    rel.sort();
    let mut entries = Vec::new();
    for r in &rel {
        entries.push(format!(
            "  \"{}\": \"{}\"",
            r.replace('\\', "/"),
            file_hash(&root.join(r)).unwrap_or_default()
        ));
    }
    for p in extra_modules {
        entries.push(format!(
            "  \"{}\": \"{}\"",
            p.display().to_string().replace('\\', "/"),
            file_hash(p).unwrap_or_default()
        ));
    }
    // Hash de integridad del índice (para verificar que el archivo no está
    // corrupto/truncado sin tener que re-scanear el workspace).
    let body = entries.join(",\n");
    let digest = sha256_hex(&body);
    let json = format!(
        "{{\n  \"version\": 1,\n  \"modules\": {{\n{}\n  }},\n  \"hash\": \"{}\"\n}}\n",
        body, digest
    );
    let path = module_index_path(entry);
    let _ = std::fs::write(&path, json);
    path
}

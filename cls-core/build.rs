//! Hash de los fuentes del backend WASM (+ internals si existen) para el caché
//! CLS->WASM de cls-jit. `cache_key` lo incluye: cualquier cambio del emisor
//! invalida los .wasm cacheados (el caché viejo confundió el debug del shadow
//! stack y del dead-flow — HANDOFF-FASE3 Paso 0).

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let mut h = DefaultHasher::new();
    let mut n = 0u32;
    hash_dir(&Path::new("src/backend/wasm"), &mut h, &mut n);
    // cls-internals (Fase 2): si el crate interno cambia, el módulo embebido en
    // el core no cambia por sí solo, pero el hash anticipa su integración.
    let internals = Path::new("../cls-internals/src");
    if internals.exists() {
        hash_dir(internals, &mut h, &mut n);
    }
    h.finish().hash(&mut h);
    println!("cargo:rustc-env=CLS_BACKEND_HASH={:016x}", h.finish());
    println!("cargo:rerun-if-changed=src/backend/wasm");
    if internals.exists() {
        println!("cargo:rerun-if-changed=../cls-internals/src");
    }
}

fn hash_dir(dir: &Path, h: &mut DefaultHasher, n: &mut u32) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut files: Vec<_> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().map_or(false, |x| x == "rs"))
        .collect();
    files.sort();
    for f in files {
        if let Ok(bytes) = std::fs::read(&f) {
            f.to_string_lossy().to_string().hash(h);
            bytes.hash(h);
            *n += 1;
        }
    }
}

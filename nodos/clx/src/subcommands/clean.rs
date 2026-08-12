use std::fs;

/// `clx clean` — limpia el caché de compilación CLS→WASM (`~/.cache/cls/`).
pub fn execute(args: &[String]) -> i32 {
    let all = args.iter().any(|a| a == "--all");
    let dir = crate::jit::cache_dir();

    if !dir.exists() {
        if !all {
            eprintln!("Caché vacía (no existe {})", dir.display());
        }
        return 0;
    }

    let mut removed = 0usize;
    let mut bytes = 0u64;
    match fs::read_dir(&dir) {
        Ok(entries) => {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Ok(meta) = fs::metadata(&path) {
                        bytes += meta.len();
                    }
                    if fs::remove_file(&path).is_ok() {
                        removed += 1;
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Error al leer el caché '{}': {}", dir.display(), e);
            return 1;
        }
    }

    if all {
        // `--all` elimina también el directorio y subcarpetas.
        if let Err(e) = fs::remove_dir_all(&dir) {
            eprintln!("Error al eliminar '{}': {}", dir.display(), e);
            return 1;
        }
    }

    // Limpiar los índices de módulos del workspace ([cwd]/.cls-cache/ si existe).
    let cwd = std::env::current_dir().unwrap_or_default();
    let ws_cache = cwd.join(".cls-cache");
    if ws_cache.exists() {
        let _ = fs::remove_dir_all(&ws_cache);
    }

    println!(
        "Caché limpiada: {} archivo(s), {} eliminados de '{}'",
        removed,
        fmt_bytes(bytes),
        dir.display()
    );
    0
}

fn fmt_bytes(n: u64) -> String {
    if n >= 1024 * 1024 {
        format!("{:.1} MB", n as f64 / (1024.0 * 1024.0))
    } else if n >= 1024 {
        format!("{:.1} KB", n as f64 / 1024.0)
    } else {
        format!("{} B", n)
    }
}

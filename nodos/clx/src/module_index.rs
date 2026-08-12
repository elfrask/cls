//! Indexación de módulos para el caché del JIT.
//!
//! El caché CLS→WASM en `~/.cache/cls/` usa una clave hash del entry. Pero si un
//! módulo importado cambia (`.clsx` de un proyecto), el hash del entry no cambia
//! y el caché sirve un binario viejo (stale cache).
//!
//! Este módulo construye un **índice de integridad**: un hash de contenido de
//! todos los `*.clsx` del workspace (y de los módulos globales importados). El
//! JIT lo incluye en la clave del caché, de modo que si cualquier módulo cambia,
//! la clave cambia y se recompila.

use std::path::{Path, PathBuf};

/// Raíz del workspace: el primer dir con `cls.json` subiendo desde `entry`,
/// o el dir del entry si no hay proyecto.
pub fn workspace_root(entry: &Path) -> PathBuf {
    let mut dir = Some(
        entry
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from(".")),
    );
    while let Some(d) = dir {
        if d.join("cls.json").exists() {
            return d;
        }
        dir = d.parent().map(|p| p.to_path_buf());
    }
    entry
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
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
    use std::io::Write;
    let mut hasher = sha256_new();
    let _ = hasher.write_all(&data);
    Some(hex_of(&hasher.finish()))
}

// ── SHA-256 mínimo (sin dependencia externa) ─────────────────────────────────

struct Sha256 {
    h: [u32; 8],
    len: u64,
    buf: Vec<u8>,
}

impl Sha256 {
    fn finish(mut self) -> Vec<u8> {
        // padding
        let bit_len = self.len * 8;
        self.buf.push(0x80);
        while self.buf.len() % 64 != 56 {
            self.buf.push(0);
        }
        for i in (0..8).rev() {
            self.buf.push(((bit_len >> (i * 8)) & 0xff) as u8);
        }
        let mut h = self.h;
        for chunk in self.buf.chunks(64) {
            let mut w = [0u32; 64];
            for i in 0..16 {
                w[i] = u32::from_be_bytes([
                    chunk[i * 4],
                    chunk[i * 4 + 1],
                    chunk[i * 4 + 2],
                    chunk[i * 4 + 3],
                ]);
            }
            for i in 16..64 {
                let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
                let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
                w[i] = w[i - 16]
                    .wrapping_add(s0)
                    .wrapping_add(w[i - 7])
                    .wrapping_add(s1);
            }
            let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
            for i in 0..64 {
                let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
                let ch = (e & f) ^ (!e & g);
                let t1 = hh
                    .wrapping_add(s1)
                    .wrapping_add(ch)
                    .wrapping_add(K[i])
                    .wrapping_add(w[i]);
                let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
                let maj = (a & b) ^ (a & c) ^ (b & c);
                let t2 = s0.wrapping_add(maj);
                hh = g;
                g = f;
                f = e;
                e = d.wrapping_add(t1);
                d = c;
                c = b;
                b = a;
                a = t1.wrapping_add(t2);
            }
            h[0] = h[0].wrapping_add(a);
            h[1] = h[1].wrapping_add(b);
            h[2] = h[2].wrapping_add(c);
            h[3] = h[3].wrapping_add(d);
            h[4] = h[4].wrapping_add(e);
            h[5] = h[5].wrapping_add(f);
            h[6] = h[6].wrapping_add(g);
            h[7] = h[7].wrapping_add(hh);
        }
        h.iter()
            .flat_map(|x| x.to_be_bytes())
            .collect()
    }
}

const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

fn sha256_new() -> Sha256 {
    Sha256 {
        h: [
            0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
        ],
        len: 0,
        buf: Vec::new(),
    }
}

impl std::io::Write for Sha256 {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        self.len += data.len() as u64;
        self.buf.extend_from_slice(data);
        Ok(data.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
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

/// Hash de integridad del workspace: combina el hash de todos los `*.clsx` del
/// proyecto (orden estable por ruta relativa) más los módulos globales explícitos.
///
/// `extra_modules`: rutas de módulos importados fuera del workspace (p.ej.
/// globales `~/.cls/modules/...`) cuyos cambios también deben invalidar el caché.
pub fn workspace_integrity_hash(entry: &Path, extra_modules: &[PathBuf]) -> String {
    let root = workspace_root(entry);
    let mut files = Vec::new();
    collect_clsx_files(&root, &mut files);
    // Rutas relativas estables.
    let mut rel: Vec<String> = files
        .iter()
        .filter_map(|f| f.strip_prefix(&root).ok().map(|r| r.to_string_lossy().into_owned()))
        .collect();
    rel.sort();

    let mut all = Vec::new();
    for r in &rel {
        all.push(format!("{}={}", r, file_hash(&root.join(r)).unwrap_or_default()));
    }
    // Módulos extra (globales) — hash por ruta absoluta estable.
    let mut extra: Vec<String> = extra_modules
        .iter()
        .filter_map(|p| file_hash(p).map(|h| format!("{}={}", p.display(), h)))
        .collect();
    extra.sort();
    all.extend(extra);

    let joined = all.join(";");
    // Re-hashear el joined para una clave corta.
    sha256_hex(&joined)
}

/// Hash simple de un string (SHA-256 hex).
pub fn sha256_hex(s: &str) -> String {
    use std::io::Write;
    let mut hasher = sha256_new();
    let _ = hasher.write_all(s.as_bytes());
    hex_of(&hasher.finish())
}

/// Escribe el índice de módulos del workspace en `[workspace]/.cls-cache/module-index.json`.
/// Devuelve la ruta escrita.
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
    let json = format!(
        "{{\n  \"version\": 1,\n  \"modules\": {{\n{}\n  }}\n}}\n",
        entries.join(",\n")
    );
    let path = module_index_path(entry);
    let _ = std::fs::write(&path, json);
    path
}

/// Limpia el índice de módulos del workspace (para `clx clean`).
pub fn clear_workspace_cache(entry: &Path) {
    let dir = workspace_cache_dir(entry);
    if dir.exists() {
        let _ = std::fs::remove_dir_all(&dir);
    }
}

/// ¿Existe un índice previo? (para saber si escribir o no).
pub fn index_exists(entry: &Path) -> bool {
    module_index_path(entry).exists()
}

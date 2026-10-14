//! Carga dinámica del driver de git (`clxgix.dll`/`libclxgix.so`/`.dylib`).
//!
//! El driver enlaza `gix` (pesado) y vive **fuera** del binario `clx`. Aquí solo
//! se localiza y se invoca su ABI C cuando hace falta descargar un paquete del
//! registry (Fase B). Ver `drivers/gix-driver/`.

// El fetch se usa recién en la Fase B (registry); hasta entonces el loader solo
// reporta disponibilidad.
#![allow(dead_code)]

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::path::{Path, PathBuf};

use libloading::{Library, Symbol};

/// Debe coincidir con `ABI_VERSION` del driver.
const ABI_VERSION: c_int = 1;

fn candidate_names() -> &'static [&'static str] {
    &[
        "clxgix.dll",
        "gix.dll",
        "libclxgix.so",
        "libclxgix.dylib",
    ]
}

/// Rutas donde se busca el driver, en orden.
fn search_paths() -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = Vec::new();
    if let Ok(p) = std::env::var("CLS_GIX_DRIVER") {
        if !p.is_empty() {
            v.push(PathBuf::from(p));
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for n in candidate_names() {
                v.push(dir.join(n));
            }
            for n in candidate_names() {
                v.push(dir.join("drivers").join(n));
            }
        }
    }
    if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
        let d = PathBuf::from(home).join(".cls").join("drivers");
        for n in candidate_names() {
            v.push(d.join(n));
        }
    }
    v
}

/// Primera ruta existente del driver, si la hay.
pub fn find_driver() -> Option<PathBuf> {
    search_paths().into_iter().find(|p| p.is_file())
}

/// ¿El driver está disponible para cargarse?
pub fn is_available() -> bool {
    find_driver().is_some()
}

type AbiVersionFn = unsafe extern "C" fn() -> c_int;
type FetchFn = unsafe extern "C" fn(
    *const c_char,
    *const c_char,
    *const c_char,
    *const c_char,
    *const c_char,
    *mut c_char,
    usize,
) -> c_int;

/// Clona `repo_url`@`commit` en `work_dir` y copia `subdir` (vacío = raíz) a
/// `out_dir`. Devuelve error claro si el driver no está o falla.
pub fn fetch(
    repo_url: &str,
    commit: &str,
    work_dir: &Path,
    out_dir: &Path,
    subdir: &str,
) -> Result<(), String> {
    let path = find_driver().ok_or_else(|| {
        "driver de git no disponible: compilá 'drivers/gix-driver' (scripts/gix-driver-build) \
         o definí CLS_GIX_DRIVER"
            .to_string()
    })?;

    let cstr = |s: &str| CString::new(s).map_err(|e| format!("string inválido: {}", e));

    unsafe {
        let lib = Library::new(&path)
            .map_err(|e| format!("no se pudo cargar '{}': {}", path.display(), e))?;

        let abi: Symbol<AbiVersionFn> = lib
            .get(b"clx_git_abi_version\0")
            .map_err(|e| format!("ABI ausente en '{}': {}", path.display(), e))?;
        let ver = abi();
        if ver != ABI_VERSION {
            return Err(format!(
                "ABI del driver incompatible: {} (esperado {})",
                ver, ABI_VERSION
            ));
        }

        let fetch: Symbol<FetchFn> = lib
            .get(b"clx_git_fetch\0")
            .map_err(|e| format!("símbolo clx_git_fetch ausente: {}", e))?;

        let c_url = cstr(repo_url)?;
        let c_commit = cstr(commit)?;
        let c_work = CString::new(work_dir.to_string_lossy().as_bytes())
            .map_err(|e| e.to_string())?;
        let c_out = CString::new(out_dir.to_string_lossy().as_bytes()).map_err(|e| e.to_string())?;
        let c_sub = cstr(subdir)?;

        let mut err = vec![0 as c_char; 1024];
        let rc = fetch(
            c_url.as_ptr(),
            c_commit.as_ptr(),
            c_work.as_ptr(),
            c_out.as_ptr(),
            c_sub.as_ptr(),
            err.as_mut_ptr(),
            err.len(),
        );
        if rc == 0 {
            Ok(())
        } else {
            let msg = CStr::from_ptr(err.as_ptr()).to_string_lossy().into_owned();
            Err(if msg.is_empty() {
                format!("el driver devolvió error {}", rc)
            } else {
                msg
            })
        }
    }
}

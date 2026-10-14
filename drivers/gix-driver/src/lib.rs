//! Driver de git para `clx`, basado en `gix` (gitoxide).
//!
//! Se compila como **cdylib** (`clxgix.dll` / `libclxgix.so` / `.dylib`) y `clx`
//! lo **carga dinámicamente** (`libloading`) solo cuando necesita descargar un
//! paquete del registry. Así el binario `clx` NO enlaza `gix` ni sus ~50 deps.
//!
//! ABI C (ver `nodos/clx/src/git_driver.rs`):
//!   int clx_git_abi_version(void);
//!   int clx_git_fetch(repo_url, commit, work_dir, out_dir, subdir,
//!                     err_buf, err_len);
//!
//! `clx_git_fetch` clona el repo (checkout detached del `commit`) en `work_dir`
//! y copia `work_dir/[subdir]` a `out_dir` (excluyendo `.git`). Devuelve 0 OK,
//! <0 error (mensaje UTF-8 en `err_buf`).

use std::ffi::CStr;
use std::os::raw::{c_char, c_int};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

/// Versión del ABI que expone este driver. Debe coincidir con la del loader.
const ABI_VERSION: c_int = 1;

#[no_mangle]
pub extern "C" fn clx_git_abi_version() -> c_int {
    ABI_VERSION
}

/// Ver `lib.rs` (cabecera) para el contrato. Devuelve 0 OK, -1 error.
///
/// # Safety
/// Los punteros deben ser C strings válidos (o null) y `err_buf` tener `err_len`
/// bytes. `work_dir` y `out_dir` deben serrutas UTF-8.
#[no_mangle]
pub unsafe extern "C" fn clx_git_fetch(
    repo_url: *const c_char,
    commit: *const c_char,
    work_dir: *const c_char,
    out_dir: *const c_char,
    subdir: *const c_char,
    err_buf: *mut c_char,
    err_len: usize,
) -> c_int {
    match fetch_impl(repo_url, commit, work_dir, out_dir, subdir) {
        Ok(()) => 0,
        Err(e) => {
            write_err(err_buf, err_len, &e);
            -1
        }
    }
}

unsafe fn cstr_or_empty(p: *const c_char) -> String {
    if p.is_null() {
        String::new()
    } else {
        CStr::from_ptr(p).to_string_lossy().into_owned()
    }
}

unsafe fn write_err(err_buf: *mut c_char, err_len: usize, msg: &str) {
    if err_buf.is_null() || err_len == 0 {
        return;
    }
    let bytes = msg.as_bytes();
    let n = bytes.len().min(err_len - 1);
    std::ptr::copy_nonoverlapping(bytes.as_ptr() as *const c_char, err_buf, n);
    *err_buf.add(n) = 0;
}

unsafe fn fetch_impl(
    repo_url: *const c_char,
    commit: *const c_char,
    work_dir: *const c_char,
    out_dir: *const c_char,
    subdir: *const c_char,
) -> Result<(), String> {
    let repo_url = cstr_or_empty(repo_url);
    let commit = cstr_or_empty(commit);
    let work_dir = PathBuf::from(cstr_or_empty(work_dir));
    let out_dir = PathBuf::from(cstr_or_empty(out_dir));
    let subdir = cstr_or_empty(subdir);

    if repo_url.is_empty() {
        return Err("repo_url vacío".into());
    }
    if commit.is_empty() {
        return Err("commit vacío".into());
    }
    if work_dir.as_os_str().is_empty() {
        return Err("work_dir vacío".into());
    }
    if out_dir.as_os_str().is_empty() {
        return Err("out_dir vacío".into());
    }

    clone_at(&repo_url, &commit, &work_dir)?;

    let src = if subdir.is_empty() {
        work_dir.clone()
    } else {
        work_dir.join(&subdir)
    };
    if !src.exists() {
        return Err(format!("subdir '{}' no existe en el repo", subdir));
    }
    copy_tree_excluding_git(&src, &out_dir)?;
    Ok(())
}

/// Clona `repo_url` y hace checkout detached de `commit` en `dest`.
fn clone_at(repo_url: &str, commit: &str, dest: &Path) -> Result<(), String> {
    use gix::create::{Kind, Options as CreateOptions};

    let prepare = gix::clone::PrepareFetch::new(
        repo_url,
        dest,
        Kind::WithWorktree,
        CreateOptions::default(),
        gix::open::Options::default(),
    )
    .map_err(|e| format!("prepare clone: {}", e))?;

    // `with_revision`: fetch SOLO de esa revisión + checkout con HEAD detached.
    let mut prepare = prepare
        .with_revision(Some(commit.to_string()))
        .map_err(|e| format!("set revision '{}': {}", commit, e))?;

    let interrupt = AtomicBool::new(false);
    let (mut checkout, _fetch) = prepare
        .fetch_then_checkout(gix::progress::Discard, &interrupt)
        .map_err(|e| format!("fetch '{}': {}", repo_url, e))?;
    let (_repo, _co) = checkout
        .main_worktree(gix::progress::Discard, &interrupt)
        .map_err(|e| format!("checkout '{}': {}", commit, e))?;
    Ok(())
}

/// Copia recursivamente `src` → `dst`, excluyendo `.git`.
fn copy_tree_excluding_git(src: &Path, dst: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dst).map_err(|e| format!("crear '{}': {}", dst.display(), e))?;
    for entry in std::fs::read_dir(src).map_err(|e| format!("leer '{}': {}", src.display(), e))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name == ".git" {
            continue;
        }
        let from = entry.path();
        let to = dst.join(&name);
        let ft = entry.file_type().map_err(|e| e.to_string())?;
        if ft.is_dir() {
            copy_tree_excluding_git(&from, &to)?;
        } else if ft.is_file() {
            std::fs::copy(&from, &to)
                .map_err(|e| format!("copiar '{}' -> '{}': {}", from.display(), to.display(), e))?;
        }
        // symlinks: se omiten (evita escapar del sandbox).
    }
    Ok(())
}

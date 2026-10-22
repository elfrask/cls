//! Resolucion de librerias nativas (`extension`) relativa al paquete.
//!
//! El import de modulos y la libreria nativa son dos capas distintas: el
//! resolver de modulos ya elige `foo@version`; aqui resolvemos el asset nativo
//! dentro de la raiz de ESE paquete. `pathModulesNatives` tiene alcance por
//! manifiesto (proyecto o modulo), nunca global.

use cls_core::config::ModuleManifest;
use cls_core::error::{ClsError, ClsResult};
use cls_core::frontend::ast::{Module as ClsModule, NativeInfo, Statement, TargetCond};
use std::path::{Path, PathBuf};

/// Raiz del paquete: el ancestro mas cercano con `cls.json`. Si no hay
/// manifiesto, usa `fallback`.
pub fn module_root(start: &Path, fallback: &Path) -> PathBuf {
    let mut dir = Some(start.to_path_buf());
    while let Some(d) = dir {
        if d.join("cls.json").is_file() {
            return d;
        }
        dir = d.parent().map(|p| p.to_path_buf());
    }
    fallback.to_path_buf()
}

fn is_path_like(library: &str) -> bool {
    library.starts_with("./")
        || library.starts_with("../")
        || Path::new(library).is_absolute()
        || library.contains('/')
        || library.contains('\\')
}

fn native_roots(root: &Path, manifest: Option<&ModuleManifest>) -> Vec<PathBuf> {
    let mut roots = vec![root.to_path_buf()];
    if let Some(m) = manifest {
        for p in &m.path_modules_natives {
            let pb = Path::new(p);
            let candidate = if pb.is_absolute() {
                pb.to_path_buf()
            } else {
                root.join(pb)
            };
            if !roots.iter().any(|r| r == &candidate) {
                roots.push(candidate);
            }
        }
    }
    roots
}

fn resolve_one(library: &str, roots: &[PathBuf]) -> ClsResult<Option<PathBuf>> {
    let p = Path::new(library);
    if p.is_absolute() {
        if p.is_file() {
            return Ok(Some(
                std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf()),
            ));
        }
        return Err(ClsError::CompileError(format!(
            "Libreria nativa '{}' no encontrada (ruta absoluta)",
            library
        )));
    }

    let mut tried: Vec<PathBuf> = Vec::new();
    for root in roots {
        let candidate = root.join(library);
        tried.push(candidate.clone());
        if candidate.is_file() {
            let resolved = std::fs::canonicalize(&candidate).unwrap_or(candidate);
            return Ok(Some(resolved));
        }
    }

    if is_path_like(library) {
        let list = tried
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(ClsError::CompileError(format!(
            "Libreria nativa '{}' no encontrada. Buscado en: {}",
            library, list
        )));
    }

    // Nombre simple sin asset local: lo resuelve el loader del SO.
    Ok(None)
}

fn resolve_info(info: &mut NativeInfo, roots: &[PathBuf]) -> ClsResult<()> {
    if info.resolved_library.is_some() {
        return Ok(());
    }
    if let Some(path) = resolve_one(&info.library, roots)? {
        info.resolved_library = Some(path.to_string_lossy().to_string());
    }
    Ok(())
}

fn current_os() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    }
}

fn cond_matches(cond: &TargetCond) -> bool {
    match cond {
        TargetCond::Any => true,
        TargetCond::Os(os) => os.eq_ignore_ascii_case(current_os()),
        TargetCond::Arch(arch) => arch.eq_ignore_ascii_case(std::env::consts::ARCH),
        // El emisor elige la rama final de plataforma/abi/target.
        TargetCond::Abi(_) | TargetCond::Platform(_) | TargetCond::Target(_) => true,
        TargetCond::Not(c) => !cond_matches(c),
        TargetCond::And(a, b) => cond_matches(a) && cond_matches(b),
        TargetCond::Or(a, b) => cond_matches(a) || cond_matches(b),
    }
}

fn resolve_statement(stmt: &mut Statement, roots: &[PathBuf]) -> ClsResult<()> {
    match stmt {
        Statement::FunctionDecl(f) => {
            if let Some(info) = &mut f.native {
                resolve_info(info, roots)?;
            }
        }
        Statement::StructureDecl(s) => {
            if let Some(info) = &mut s.native {
                resolve_info(info, roots)?;
            }
        }
        Statement::VarDecl(v) => {
            if let Some(info) = &mut v.native {
                resolve_info(info, roots)?;
            }
        }
        Statement::When(w) => {
            for branch in &mut w.branches {
                if !cond_matches(&branch.cond) {
                    continue;
                }
                for inner in &mut branch.block.statements {
                    resolve_statement(inner, roots)?;
                }
            }
        }
        _ => {}
    }
    Ok(())
}

/// Resuelve las extensiones del modulo contra su propia raiz/manifiesto.
pub fn resolve_module_natives(
    module: &mut ClsModule,
    root: &Path,
    manifest: Option<&ModuleManifest>,
) -> ClsResult<()> {
    let roots = native_roots(root, manifest);
    for stmt in &mut module.statements {
        resolve_statement(stmt, &roots)?;
    }
    Ok(())
}

/// Rutas resueltas de un modulo (para cache/build).
pub fn resolved_native_libraries(module: &ClsModule) -> Vec<String> {
    let mut out = Vec::new();
    fn walk(stmt: &Statement, out: &mut Vec<String>) {
        match stmt {
            Statement::FunctionDecl(f) => {
                if let Some(i) = &f.native {
                    if let Some(r) = &i.resolved_library {
                        out.push(r.clone());
                    }
                }
            }
            Statement::StructureDecl(s) => {
                if let Some(i) = &s.native {
                    if let Some(r) = &i.resolved_library {
                        out.push(r.clone());
                    }
                }
            }
            Statement::VarDecl(v) => {
                if let Some(i) = &v.native {
                    if let Some(r) = &i.resolved_library {
                        out.push(r.clone());
                    }
                }
            }
            Statement::When(w) => {
                for b in &w.branches {
                    if !cond_matches(&b.cond) {
                        continue;
                    }
                    for s in &b.block.statements {
                        walk(s, out);
                    }
                }
            }
            _ => {}
        }
    }
    for stmt in &module.statements {
        walk(stmt, &mut out);
    }
    out.sort();
    out.dedup();
    out
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("cls_native_{}_{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn resolves_relative_asset_from_module_root() {
        let root = temp_dir("relative");
        fs::create_dir_all(root.join("native")).unwrap();
        fs::write(root.join("native/foo.dll"), b"x").unwrap();
        let got = resolve_one("./native/foo.dll", &[root.clone()])
            .unwrap()
            .unwrap();
        assert_eq!(got, fs::canonicalize(root.join("native/foo.dll")).unwrap());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn resolves_from_path_modules_natives() {
        let root = temp_dir("paths");
        fs::create_dir_all(root.join("shared")).unwrap();
        fs::write(root.join("shared/bar.dll"), b"x").unwrap();
        let mut m = ModuleManifest::default_for("t");
        m.path_modules_natives = vec!["shared".to_string()];
        let roots = native_roots(&root, Some(&m));
        let got = resolve_one("./bar.dll", &roots).unwrap().unwrap();
        assert_eq!(got, fs::canonicalize(root.join("shared/bar.dll")).unwrap());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn leaves_system_names_alone() {
        let root = temp_dir("system");
        let got = resolve_one("bcrypt.dll", &[root.clone()]).unwrap();
        assert!(got.is_none());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn missing_relative_is_an_error() {
        let root = temp_dir("missing");
        assert!(resolve_one("./nope.dll", &[root.clone()]).is_err());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn module_root_walks_to_manifest() {
        let root = temp_dir("root");
        fs::write(root.join("cls.json"), b"{}").unwrap();
        fs::create_dir_all(root.join("src")).unwrap();
        assert_eq!(module_root(&root.join("src"), &root), root);
        let _ = fs::remove_dir_all(root);
    }
}

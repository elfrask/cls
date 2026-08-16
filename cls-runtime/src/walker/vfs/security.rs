use std::path::{Component, Path, PathBuf};

use cls_core::error::{ClsError, ClsResult};

/// Resuelve un path de forma segura evitando path traversal (chroot jail).
/// No permite rutas absolutas ni navegacion fuera del base.
pub fn resolve_safe(path: &str, base: &Path) -> ClsResult<PathBuf> {
    // Normalizar el path: colapsar . y .. segun las reglas
    let mut result = base.to_path_buf();

    for component in Path::new(path).components() {
        match component {
            Component::ParentDir => {
                // Verificar que no escape del jail
                if result == base {
                    return Err(ClsError::RuntimeError(format!(
                        "Path traversal detectado: '{}' intenta salir de '{}'",
                        path, base.display()
                    )));
                }
                result.pop();
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(ClsError::RuntimeError(format!(
                    "Ruta absoluta no permitida en protocolo: '{}' contiene '/' o raiz",
                    path
                )));
            }
            Component::CurDir => {
                // . es ignorado
            }
            _ => {
                result.push(component);
            }
        }
    }

    // Verificar que el resultado sigue dentro del base
    if !result.starts_with(base) {
        return Err(ClsError::RuntimeError(format!(
            "Path traversal detectado: '{}' resolvio a '{}' fuera de '{}'",
            path, result.display(), base.display()
        )));
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_path() {
        let base = Path::new("/app/data");
        let result = resolve_safe("config.json", base).unwrap();
        assert_eq!(result, Path::new("/app/data/config.json"));
    }

    #[test]
    fn test_nested_path() {
        let base = Path::new("/app/data");
        let result = resolve_safe("logs/error.log", base).unwrap();
        assert_eq!(result, Path::new("/app/data/logs/error.log"));
    }

    #[test]
    fn test_path_traversal_blocked() {
        let base = Path::new("/app/data");
        let result = resolve_safe("../../../etc/passwd", base);
        assert!(result.is_err());
    }

    #[test]
    fn test_absolute_path_blocked() {
        let base = Path::new("/app/data");
        let result = resolve_safe("/etc/passwd", base);
        assert!(result.is_err());
    }

    #[test]
    fn test_dot_dot_ok_within_base() {
        let base = Path::new("/app/data");
        let result = resolve_safe("sub/../other/file.txt", base).unwrap();
        assert_eq!(result, Path::new("/app/data/other/file.txt"));
    }
}

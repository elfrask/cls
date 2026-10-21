//! Caché CLS->WASM en disco y resolución de imports de módulos de usuario.

use cls_core::frontend::ast::Module as ClsModule;
use std::path::PathBuf;
use std::sync::OnceLock;

/// Rutas adicionales de búsqueda de módulos. Las configura el nodo (CLI
/// `--add-path-module=<path>` o `cls.json` -> `pathModules`) antes de compilar.
/// Ver `examples/patrones.md` §11.
static EXTRA_MODULE_PATHS: OnceLock<Vec<PathBuf>> = OnceLock::new();

/// Fija las rutas adicionales de búsqueda de módulos. Idempotente: el proceso
/// compila una sola vez, así que la primera llamada gana.
pub fn set_extra_module_paths(paths: Vec<PathBuf>) {
    let _ = EXTRA_MODULE_PATHS.set(paths);
}

/// Rutas adicionales de búsqueda de módulos (vacío si no se configuraron).
pub fn extra_module_paths() -> &'static [PathBuf] {
    EXTRA_MODULE_PATHS
        .get()
        .map(|v| v.as_slice())
        .unwrap_or(&[])
}

/// Directorio del caché de compilación: `~/.cache/cls/` (HOME o USERPROFILE).
pub fn cache_dir() -> std::path::PathBuf {
    let base = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| std::env::temp_dir().to_string_lossy().into_owned());
    std::path::PathBuf::from(base).join(".cache").join("cls")
}

/// Escribe bytes a un archivo de forma atómica: a un temporal y `rename`.
/// Un fallo a mitad no deja un `.wasm` corrupto en el caché (el temporal queda
/// huérfano pero el destino anterior permanece intacto).
pub(crate) fn atomic_write(path: &std::path::Path, data: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension(format!("tmp{}", std::process::id()));
    std::fs::write(&tmp, data)?;
    std::fs::rename(&tmp, path)
}

/// Clave del caché: hash del fuente + versión del compilador + target + los
/// **sources de los módulos importados** (locales y globales de ~/.cls) + el
/// runtime (wasmtime y wasmi emiten bytes distintos: el tag de excepciones).
/// Editar un .clsx del grafo importado invalida el caché; editar uno NO
/// importado NO lo invalida (evita sobre-invalidación al tocar archivos no
/// relacionados).
pub(crate) fn cache_key(
    source: &str,
    target_str: Option<&str>,
    entry: &std::path::Path,
    module_sources: &[String],
    runtime: &str,
    trace_calls: bool,
    memory_max_pages: Option<u64>,
) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    source.hash(&mut h);
    cls_core::VERSION.hash(&mut h);
    // Hash de los fuentes del backend WASM: editar el emisor invalida los .wasm
    // cacheados (sin esto, módulos viejos se reutilizan y confunden el debug —
    // HANDOFF-FASE3 Paso 0).
    cls_core::BACKEND_HASH.hash(&mut h);
    target_str.unwrap_or("").hash(&mut h);
    runtime.hash(&mut h);
    // El shadow call stack (trace_calls) cambia el WASM: incluirlo en la key.
    trace_calls.hash(&mut h);
    // El máximo de memoria (páginas) cambia la sección memory del WASM.
    memory_max_pages.hash(&mut h);
    // Integridad de los módulos importados: se hashean los SOURCES de los módulos
    // resueltos (locales del proyecto Y globales de ~/.cls). Así editar cualquier
    // .clsx importado invalida el caché aunque esté fuera del workspace.
    for ms in module_sources {
        ms.hash(&mut h);
    }
    // El entry forma parte del `source`; los imports del grafo están en
    // `module_sources`. NO se barre el workspace completo (sobre-invalidación).
    let _ = entry;
    h.finish()
}

/// Compara dos versiones semver ("1.2.0" vs "1.10.0"). Retorna Ordering.
fn cmp_semver(a: &str, b: &str) -> std::cmp::Ordering {
    let av: Vec<u64> = a
        .trim_start_matches(['^', '~', '>', '=', '<'])
        .split('.')
        .filter_map(|p| p.parse().ok())
        .collect();
    let bv: Vec<u64> = b
        .split('.')
        .filter_map(|p| p.parse().ok())
        .collect();
    for i in 0..3 {
        let x = av.get(i).copied().unwrap_or(0);
        let y = bv.get(i).copied().unwrap_or(0);
        if x != y {
            return x.cmp(&y);
        }
    }
    std::cmp::Ordering::Equal
}

/// ¿El rango semver declarado (p.ej. `^1.2.0`, `~1.2`, `>=1.0`) acepta la versión?
fn semver_matches(range: &str, version: &str) -> bool {
    let range = range.trim();
    if range.starts_with('^') {
        // ^1.2.0 -> >=1.2.0 y major == 1
        let min = &range[1..];
        if cmp_semver(version, min) == std::cmp::Ordering::Less {
            return false;
        }
        let parts = min.split('.').collect::<Vec<_>>();
        if parts.is_empty() {
            return false;
        }
        let major = parts[0].parse::<u64>().unwrap_or(0);
        let vmajor = version.split('.').next().and_then(|p| p.parse::<u64>().ok()).unwrap_or(0);
        vmajor == major
    } else if range.starts_with('~') {
        let min = &range[1..];
        let mut mv = min.split('.').collect::<Vec<_>>();
        if mv.len() >= 2 {
            mv.truncate(2);
        }
        let upper = format!("{}.999.999", mv.join("."));
        cmp_semver(version, min) != std::cmp::Ordering::Less
            && cmp_semver(version, &upper) != std::cmp::Ordering::Greater
    } else if range.starts_with('>') || range.starts_with('=') {
        let min = range.trim_start_matches(['>', '=', ' ']);
        if range.starts_with(">=") {
            cmp_semver(version, min) != std::cmp::Ordering::Less
        } else if range.starts_with('>') {
            cmp_semver(version, min) == std::cmp::Ordering::Greater
        } else {
            cmp_semver(version, min) == std::cmp::Ordering::Equal
        }
    } else if let Some((lo, hi)) = range.split_once(" - ") {
        cmp_semver(version, lo) != std::cmp::Ordering::Less
            && cmp_semver(version, hi) != std::cmp::Ordering::Greater
    } else {
        // Versión exacta o sin prefijo.
        cmp_semver(version, range) == std::cmp::Ordering::Equal
    }
}

/// Raíz del proyecto: sube desde `start` hasta encontrar `cls.json` o un dir
/// con `modules/`. Si no encuentra, devuelve `start`.
fn project_root(start: &std::path::Path) -> std::path::PathBuf {
    let mut dir = Some(start.to_path_buf());
    while let Some(d) = dir {
        if d.join("cls.json").exists() || d.join("modules").is_dir() {
            return d;
        }
        dir = d.parent().map(|p| p.to_path_buf());
    }
    start.to_path_buf()
}

/// `scope/nombre` de un paquete con scope (exactamente un `/`, sin `.`/`..`,
/// sin extensión). `None` si es un path explícito o un nombre plano.
fn scoped_parts(name: &str) -> Option<(&str, &str)> {
    if name.contains('\\') {
        return None;
    }
    let (a, b) = name.split_once('/')?;
    if a.is_empty() || b.is_empty() || b.contains('/') {
        return None;
    }
    if a == "." || a == ".." || a.starts_with('.') {
        return None;
    }
    if std::path::Path::new(b).extension().is_some() {
        return None;
    }
    Some((a, b))
}

/// Candidatos de archivo para un import de módulo, en orden de búsqueda.
///
/// Orden (RESOLVERS.md):
///   1. {base_dir}/{path}.clsx            (junto al archivo que importa)
///   2. {proyecto}/modules/{name}/mod.clsx (módulos del workspace)
///   2b. rutas extra (--add-path-module / cls.json pathModules): {ruta}/{name}/
///   3. {cwd}/{path}.clsx                 (relativo al CWD)
///   4. {cwd}/modules/{name}/mod.clsx     (módulos del proyecto)
///   5. {home}/.cls/modules/{name}@{version}/mod.clsx (globales usuario, versionado)
///   6. {home}/.cls/modules/{name}/mod.clsx            (globales sin versión)
///
/// Si `manifest` declara `dependencies[name]`, la búsqueda global filtra por el
/// rango semver declarado (prioriza la versión instalada que lo cumpla).
pub fn module_candidates(
    path: &str,
    base_dir: &std::path::Path,
    manifest: Option<&cls_core::config::ModuleManifest>,
) -> Vec<std::path::PathBuf> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    // Migracion dev-2 (Fase 7): antes se llamaba `cls_runtime::user_modules_dir()`
    // (parte del walker eliminado). La logica es trivial: el path a
    // `~/.cls/modules` del usuario actual. Se inlinea para evitar
    // agregar una API publica a `cls-runtime` solo por esto.
    let user_modules: Option<std::path::PathBuf> = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(|h| std::path::PathBuf::from(h).join(".cls").join("modules"));
    let name = path.trim_start_matches(['/', '\\']).trim();
    let proj = project_root(base_dir);
    let extras = extra_module_paths();
    let mut candidates: Vec<std::path::PathBuf> = Vec::new();
    // Paquete con scope `scope/nombre`: layout global anidado
    // `~/.cls/modules/{scope}/{nombre}@{version}/` (filtrado por el rango de
    // `dependencies[{scope/name}]`). Se resuelve ANTES del path directo.
    if let Some((scope, pkg)) = scoped_parts(name) {
        if let Some(ref um) = user_modules {
            let declared = manifest.and_then(|m| m.dependency_version(name).map(|s| s.to_string()));
            let scope_dir = um.join(scope);
            if let Ok(entries) = std::fs::read_dir(&scope_dir) {
                let mut versions: Vec<(String, std::path::PathBuf)> = entries
                    .flatten()
                    .filter(|e| e.path().is_dir())
                    .filter_map(|e| {
                        let n = e.file_name().to_string_lossy().to_string();
                        n.strip_prefix(&format!("{}@", pkg))
                            .map(|ver| (ver.to_string(), e.path()))
                    })
                    .collect();
                versions.sort_by(|a, b| cmp_semver(&a.0, &b.0).reverse());
                for (ver, p) in versions {
                    if let Some(ref d) = declared {
                        if !semver_matches(d, &ver) {
                            continue;
                        }
                    }
                    push_module_dir_candidates(&p, &mut candidates);
                }
            }
            if declared.is_none() {
                push_module_dir_candidates(&scope_dir.join(pkg), &mut candidates);
            }
        }
        // Fallbacks (módulos propios o path relativo).
        push_module_dir_candidates(&proj.join("modules").join(scope).join(pkg), &mut candidates);
        for e in extras {
            push_module_dir_candidates(&e.join(scope).join(pkg), &mut candidates);
        }
        push_module_dir_candidates(&cwd.join("modules").join(scope).join(pkg), &mut candidates);
        candidates.push(base_dir.join(format!("{}.clsx", name)));
        return candidates;
    }
    if name.contains('/') || name.contains('\\') || std::path::Path::new(name).is_absolute() {
        // Path explícito: directo. Agregar `.clsx` si no lo trae.
        let p = std::path::Path::new(&path);
        let with_ext = if p.extension().is_some() {
            p.to_path_buf()
        } else {
            std::path::PathBuf::from(format!("{}.clsx", path))
        };
        if with_ext.is_absolute() {
            candidates.push(with_ext);
        } else {
            candidates.push(base_dir.join(with_ext));
        }
    } else {
        // Módulo por nombre: buscar en orden.
        candidates.push(base_dir.join(format!("{}.clsx", name)));
        push_module_dir_candidates(&proj.join("modules").join(name), &mut candidates);
        for e in extras {
            push_module_dir_candidates(&e.join(name), &mut candidates);
            candidates.push(e.join(format!("{}.clsx", name)));
        }
        candidates.push(cwd.join(format!("{}.clsx", name)));
        push_module_dir_candidates(&cwd.join("modules").join(name), &mut candidates);
        if let Some(ref um) = user_modules {
            let declared = manifest.and_then(|m| m.dependency_version(name).map(|s| s.to_string()));
            // Versiones instaladas (nombre@versión): ordenar desc y filtrar por rango.
            if let Ok(entries) = std::fs::read_dir(um) {
                let mut versions: Vec<(String, std::path::PathBuf)> = entries
                    .flatten()
                    .filter(|e| e.path().is_dir())
                    .filter_map(|e| {
                        let n = e.file_name().to_string_lossy().to_string();
                        let v = n.strip_prefix(&format!("{}@", name));
                        v.map(|ver| (ver.to_string(), e.path()))
                    })
                    .collect();
                versions.sort_by(|a, b| cmp_semver(&a.0, &b.0).reverse());
                for (ver, p) in versions {
                    if let Some(ref d) = declared {
                        if !semver_matches(d, &ver) {
                            continue;
                        }
                    }
                    push_module_dir_candidates(&p, &mut candidates);
                }
            }
            // Sin versión (fallback): solo si no hay rango declarado que exigir.
            if declared.is_none() {
                push_module_dir_candidates(&um.join(name), &mut candidates);
            }
        }
    }
    candidates
}

/// Candidatos de un directorio de módulo `{root}/modules/{name}` (o global
/// `~/.cls/modules/{name}[@ver]`): primero el `export.entry` de su `cls.json`
/// (aislamiento de módulo), luego `mod.clsx`.
fn push_module_dir_candidates(dir: &std::path::Path, candidates: &mut Vec<std::path::PathBuf>) {
    if let Some(entry) = module_manifest_entry(dir) {
        candidates.push(entry);
    }
    candidates.push(dir.join("mod.clsx"));
}

/// `dir/cls.json` con `export.entry` -> `dir/<entry>` (si el archivo existe).
fn module_manifest_entry(dir: &std::path::Path) -> Option<std::path::PathBuf> {
    let manifest_path = dir.join("cls.json");
    if !manifest_path.exists() {
        return None;
    }
    let m = cls_core::config::ModuleManifest::from_file(&manifest_path).ok()?;
    let entry = m.export_config?.entry;
    if entry.is_empty() {
        return None;
    }
    let p = dir.join(&entry);
    if p.exists() {
        Some(p)
    } else {
        None
    }
}

/// ¿`dir/cls.json` describe un **módulo** (tiene sección `export`)? Si no,
/// el módulo no tiene aislamiento propio.
fn module_has_export(dir: &std::path::Path) -> bool {
    let manifest_path = dir.join("cls.json");
    if !manifest_path.exists() {
        return false;
    }
    cls_core::config::ModuleManifest::from_file(&manifest_path)
        .map(|m| m.export_config.is_some())
        .unwrap_or(false)
}

/// Resuelve los imports de un módulo (recursivamente) y los carga como AST.
/// Cada entrada del resultado: (path del import, source, módulo parseado).
/// Si un import no se puede resolver, devuelve un error claro con los
/// candidatos probados (no se queda en silencio).
pub fn load_import_modules(
    module: &ClsModule,
    base_dir: &std::path::Path,
    seen: &mut std::collections::HashSet<String>,
    out: &mut Vec<(String, String, ClsModule)>,
    manifest: Option<&cls_core::config::ModuleManifest>,
) -> cls_core::error::ClsResult<()> {
    load_import_modules_hooked(module, base_dir, seen, out, manifest, None)
}

/// Igual que [`load_import_modules`] pero con un hook del nodo: si un import no
/// resuelve en disco, el hook provee el source (módulos en memoria/VFS).
pub fn load_import_modules_hooked(
    module: &ClsModule,
    base_dir: &std::path::Path,
    seen: &mut std::collections::HashSet<String>,
    out: &mut Vec<(String, String, ClsModule)>,
    manifest: Option<&cls_core::config::ModuleManifest>,
    hook: Option<&dyn crate::host::ModuleSourceResolver>,
) -> cls_core::error::ClsResult<()> {
    use cls_core::error::ClsError;
    // Módulos internos del core/nodo: NO se resuelven como archivos.
    // `net` eliminado en dev-2 (era código muerto: los host_net_* nunca se
    // usaron desde ningún .clsx; los sockets deben venir de `extension`+`when`
    // en el .clsx del usuario, ver docs/lenguaje/extension.md).
    const INTERNALS: &[&str] = &[
        "math", "json", "fs", "http", "Lib", "async", "os", "path", "process", "time", "random", "strings",
    ];
    for stmt in &module.statements {
        let import = match stmt {
            Statement::Import(i) => Some((i.path.clone(), i.span.clone())),
            Statement::FromImport(fi) => Some((fi.path.clone(), fi.span.clone())),
            Statement::ExportList(el) => el.from.clone().map(|p| (p, el.span.clone())),
            _ => None,
        };
        if let Some((path, span)) = import {
            if INTERNALS.contains(&path.as_str()) {
                continue;
            }
            let candidates = module_candidates(&path, base_dir, manifest);
            let mut found = false;
            for candidate in &candidates {
                let key = candidate.to_string_lossy().to_string();
                if seen.contains(&key) {
                    // Ya cargado por otro import: no duplicar en `out`, pero el
                    // módulo SÍ está resuelto (import doble con sintaxis distinta).
                    found = true;
                    continue;
                }
                // Solo seguimos al siguiente candidato si el archivo NO existe.
                let source = match std::fs::read_to_string(candidate) {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                // El archivo EXISTE: si no lexea/parsea, es un error real del
                // módulo (no un "módulo no encontrado"). Lo propagamos con el
                // path y la causa, en vez de silenciarlo y reportar el error
                // genérico de resolución (que desorienta).
                let toks = cls_core::frontend::Lexer::new(&source)
                    .tokenize()
                    .map_err(|e| {
                        ClsError::compile_at(
                            &format!(
                                "El módulo '{}' ({}) tiene un error de lexing: {}",
                                path,
                                candidate.display(),
                                e
                            ),
                            &span,
                        )
                    })?;
                let mut m = cls_core::frontend::Parser::new(toks).parse().map_err(|e| {
                    ClsError::compile_at(
                        &format!(
                            "El módulo '{}' ({}) tiene un error de sintaxis: {}",
                            path,
                            candidate.display(),
                            e
                        ),
                        &span,
                    )
                })?;
                seen.insert(key);
                // Aislamiento de módulo: si el directorio del módulo tiene su
                // propio `cls.json` con sección `export`, sus imports internos
                // se resuelven desde esa raíz (no desde el proyecto principal).
                let pkg_start = candidate.parent().unwrap_or(base_dir);
                let pkg_root = crate::native_resolve::module_root(pkg_start, base_dir);
                let pkg_manifest = cls_core::config::ModuleManifest::find_in_dir(&pkg_root);
                crate::native_resolve::resolve_module_natives(
                    &mut m,
                    &pkg_root,
                    pkg_manifest.as_ref(),
                )
                .map_err(|e| {
                    ClsError::compile_at(
                        &format!(
                            "El modulo '{}' ({}) tiene una extension nativa invalida: {}",
                            path,
                            candidate.display(),
                            e
                        ),
                        &span,
                    )
                })?;
                let (mod_base, mod_manifest) = match candidate.parent() {
                    Some(d) if module_has_export(d) => {
                        let mb = d.to_path_buf();
                        let mm = cls_core::config::ModuleManifest::find_in_dir(&mb);
                        (mb, mm)
                    }
                    _ => (base_dir.to_path_buf(), manifest.cloned()),
                };
                load_import_modules_hooked(&m, &mod_base, seen, out, mod_manifest.as_ref(), hook)?;
                out.push((path.clone(), source, m));
                found = true;
                break;
            }
            // Hook del nodo: el import no está en disco -> el nodo provee el source.
            if !found {
                if let Some(h) = hook {
                    if let Some(source) = h.resolve_source(&path, base_dir) {
                        let toks = cls_core::frontend::Lexer::new(&source)
                            .tokenize()
                            .map_err(|e| {
                                ClsError::compile_at(
                                    &format!("El módulo '{}' (hook del nodo) tiene un error de lexing: {}", path, e),
                                    &span,
                                )
                            })?;
                        let mut m = cls_core::frontend::Parser::new(toks).parse().map_err(|e| {
                            ClsError::compile_at(
                                &format!("El módulo '{}' (hook del nodo) tiene un error de sintaxis: {}", path, e),
                                &span,
                            )
                        })?;
                        let hook_root = crate::native_resolve::module_root(base_dir, base_dir);
                        crate::native_resolve::resolve_module_natives(
                            &mut m, &hook_root, manifest
                        )?;
                        let key = format!("hook:{}", path);
                        seen.insert(key);
                        load_import_modules_hooked(&m, base_dir, seen, out, manifest, hook)?;
                        out.push((path.clone(), source, m));
                        found = true;
                    }
                }
            }
            if !found {
                // El módulo no se resolvió: error claro con los candidatos.
                let tried = candidates
                    .iter()
                    .map(|c| format!("  - {}", c.display()))
                    .collect::<Vec<_>>()
                    .join("\n");
                return Err(ClsError::compile_at(
                    &format!(
                        "No se pudo resolver el módulo '{}'.\nSe buscó en:\n{}",
                        path, tried
                    ),
                    &span,
                ));
            }
        }
    }
    Ok(())
}

use cls_core::frontend::ast::Statement;

//! Descubrimiento e inspección de paquetes CLS.
//!
//! Paquetes **locales**: `{proyecto}/modules/{nombre}/` (un paquete por dir).
//! Paquetes **globales**: `~/.cls/modules/{nombre}@{version}/` (o `{nombre}/`).
//! Paquetes **archivados**: `.zip`/`.clsapp` que contienen un `cls.json`.
//!
//! Un *módulo* es un paquete cuyo `cls.json` declara la sección `export`
//! (`export.entry` obligatorio). Un *proyecto* declara `entry`.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use cls_core::config::ModuleManifest;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PackageSource {
    Local,
    Global,
    Archive,
}

impl PackageSource {
    pub fn label(&self) -> &'static str {
        match self {
            PackageSource::Local => "local",
            PackageSource::Global => "global",
            PackageSource::Archive => "zip",
        }
    }
}

/// Información inspeccionada de un paquete.
pub struct PackageInfo {
    pub name: String,
    /// Scope (`usuario` en `usuario/foo`), si aplica.
    pub scope: Option<String>,
    pub version: String,
    pub description: String,
    pub license: String,
    pub authors: Vec<String>,
    pub target: String,
    pub entry: String,
    pub export_entry: Option<String>,
    pub export_types: Option<String>,
    pub dependencies: Vec<String>,
    pub path: PathBuf,
    pub source: PackageSource,
    pub manifest_ok: bool,
    /// Problemas detectados (auditoría). Vacío = sano.
    pub issues: Vec<String>,
}

impl PackageInfo {
    /// Es un *módulo* si declara `export.entry`.
    pub fn is_module(&self) -> bool {
        self.export_entry.is_some()
    }

    /// Etiqueta corta: `module` / `project`.
    pub fn kind(&self) -> &'static str {
        if self.is_module() {
            "module"
        } else {
            "project"
        }
    }

    /// Identificador sin versión: `[scope/]name`.
    pub fn id(&self) -> String {
        match &self.scope {
            Some(s) => format!("{}/{}", s, self.name),
            None => self.name.clone(),
        }
    }

    /// `[scope/]nombre@version` (o sin `@version` si no hay versión).
    pub fn full_name(&self) -> String {
        if self.version.is_empty() {
            self.id()
        } else {
            format!("{}@{}", self.id(), self.version)
        }
    }
}

/// `~/.cls/modules` del usuario (HOME o USERPROFILE).
pub fn global_modules_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(|h| PathBuf::from(h).join(".cls").join("modules"))
}

/// `{cwd}/modules` del proyecto actual.
pub fn local_modules_dir() -> PathBuf {
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("modules")
}

/// Separa `nombre@version` (si el dir no tiene manifiesto).
fn split_name_version(dir_name: &str) -> (String, String) {
    match dir_name.split_once('@') {
        Some((n, v)) => (n.to_string(), v.to_string()),
        None => (dir_name.to_string(), String::new()),
    }
}

// ── Spec de paquete y semver ─────────────────────────────────────────────────

/// Spec de paquete: `[scope/]name[@version]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageSpec {
    pub scope: Option<String>,
    pub name: String,
    pub version: Option<String>,
}

impl PackageSpec {
    /// Parsea `[scope/]name[@version]`. Valida charset de nombre/scope y, si hay
    /// versión, su forma semver.
    pub fn parse(s: &str) -> Result<PackageSpec, String> {
        let s = s.trim();
        if s.is_empty() {
            return Err("spec de paquete vacío".to_string());
        }
        if s.starts_with('-') {
            return Err(format!("'{}' no es un nombre de paquete válido", s));
        }
        let (path_part, version) = match s.split_once('@') {
            Some((p, v)) => (p.to_string(), Some(v.to_string())),
            None => (s.to_string(), None),
        };
        let (scope, name) = match path_part.split_once('/') {
            Some((sc, n)) => (Some(sc.to_string()), n.to_string()),
            None => (None, path_part.clone()),
        };
        if !valid_pkg_id(&name) {
            return Err(format!("nombre de paquete inválido: '{}'", name));
        }
        if let Some(sc) = &scope {
            if !valid_pkg_id(sc) {
                return Err(format!("scope inválido: '{}'", sc));
            }
        }
        if let Some(v) = &version {
            if !is_valid_semver(v) {
                return Err(format!("versión no válida (se espera semver): '{}'", v));
            }
        }
        Ok(PackageSpec {
            scope,
            name,
            version,
        })
    }

    /// Identificador sin versión: `[scope/]name`.
    pub fn id(&self) -> String {
        match &self.scope {
            Some(s) => format!("{}/{}", s, self.name),
            None => self.name.clone(),
        }
    }
}

/// Valida el charset de un nombre/scope de paquete.
fn valid_pkg_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
}

/// Forma semver laxa: `x[.y[.z]][-pre][+build]`, con `x` numérico.
pub fn is_valid_semver(v: &str) -> bool {
    let v = v.trim();
    if v.is_empty() {
        return false;
    }
    let core = v.split(['-', '+']).next().unwrap_or("");
    let parts: Vec<&str> = core.split('.').collect();
    if parts.is_empty() || parts.len() > 3 {
        return false;
    }
    parts.iter().all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
}

/// ¿La versión es prerelease (canal `dev`)? `1.2.0-dev.3` → true.
pub fn is_prerelease(v: &str) -> bool {
    v.contains('-')
}

/// Compara versiones semver por sus componentes numéricos (ignora prefijos de
/// rango y prerelease).
pub fn cmp_semver(a: &str, b: &str) -> std::cmp::Ordering {
    let nums = |s: &str| -> Vec<u64> {
        s.trim_start_matches(['^', '~', '>', '=', '<', ' '])
            .split(['-', '+'])
            .next()
            .unwrap_or("")
            .split('.')
            .filter_map(|p| p.trim().parse().ok())
            .collect()
    };
    let av = nums(a);
    let bv = nums(b);
    for i in 0..3 {
        let x = av.get(i).copied().unwrap_or(0);
        let y = bv.get(i).copied().unwrap_or(0);
        if x != y {
            return x.cmp(&y);
        }
    }
    std::cmp::Ordering::Equal
}

/// ¿La versión cumple el rango declarado? Port de `semver_matches` del resolver:
/// `^1.2.0`, `~1.2`, `>=1.0`, `1.0 - 2.0`, exacta, etc.
pub fn version_satisfies(range: &str, version: &str) -> bool {
    use std::cmp::Ordering;
    let range = range.trim();
    if range.is_empty() || range == "*" {
        return true;
    }
    if let Some(min) = range.strip_prefix('^') {
        if cmp_semver(version, min) == Ordering::Less {
            return false;
        }
        let major = min.split('.').next().unwrap_or("0").parse::<u64>().unwrap_or(0);
        let vmajor = version
            .split('.')
            .next()
            .and_then(|p| p.parse::<u64>().ok())
            .unwrap_or(0);
        return vmajor == major;
    }
    if let Some(min) = range.strip_prefix('~') {
        let mut mv: Vec<&str> = min.split('.').collect();
        if mv.len() >= 2 {
            mv.truncate(2);
        }
        let upper = format!("{}.999.999", mv.join("."));
        return cmp_semver(version, min) != Ordering::Less
            && cmp_semver(version, &upper) != Ordering::Greater;
    }
    if range.starts_with(">=") {
        return cmp_semver(version, range.trim_start_matches(['>', '=', ' '])) != Ordering::Less;
    }
    if let Some(hi) = range.strip_prefix('>') {
        return cmp_semver(version, hi.trim()) == Ordering::Greater;
    }
    if let Some((lo, hi)) = range.split_once(" - ") {
        return cmp_semver(version, lo) != Ordering::Less
            && cmp_semver(version, hi) != Ordering::Greater;
    }
    cmp_semver(version, range) == Ordering::Equal
}

/// Directorio global de un paquete versionado, con layout anidado de scope:
/// `~/.cls/modules/[scope/]name@version/`.
pub fn global_package_dir(scope: &Option<String>, name: &str, version: &str) -> PathBuf {
    let base = global_modules_dir().unwrap_or_else(|| PathBuf::from(".cls-modules"));
    match scope {
        Some(s) => base.join(s).join(format!("{}@{}", name, version)),
        None => base.join(format!("{}@{}", name, version)),
    }
}

/// Versiones instaladas de un paquete: `(version, dir)` (global), ordenadas.
pub fn installed_versions(spec: &PackageSpec) -> Vec<(String, PathBuf)> {
    let Some(base) = global_modules_dir() else {
        return Vec::new();
    };
    let parent = match &spec.scope {
        Some(s) => base.join(s),
        None => base.clone(),
    };
    let prefix = format!("{}@", spec.name);
    let mut out = Vec::new();
    if let Ok(rd) = fs::read_dir(&parent) {
        for e in rd.flatten() {
            let p = e.path();
            if !p.is_dir() {
                continue;
            }
            let dn = e.file_name().to_string_lossy().to_string();
            if let Some(ver) = dn.strip_prefix(&prefix) {
                out.push((ver.to_string(), p));
            }
        }
    }
    out.sort_by(|a, b| cmp_semver(&a.0, &b.0));
    out
}

/// Última versión instalada de un paquete. Con `only_stable`, ignora prereleases
/// (canal `dev`).
pub fn latest_installed(spec: &PackageSpec, only_stable: bool) -> Option<String> {
    installed_versions(spec)
        .into_iter()
        .map(|(v, _)| v)
        .filter(|v| !only_stable || !is_prerelease(v))
        .max_by(|a, b| cmp_semver(a, b))
}

/// Inspecciona un directorio de paquete (con `cls.json` en su raíz).
pub fn inspect_dir(dir: &Path, source: PackageSource) -> Option<PackageInfo> {
    inspect_dir_scoped(dir, source, None)
}

/// Igual que [`inspect_dir`] pero fijando el `scope` (paquetes anidados).
pub fn inspect_dir_scoped(
    dir: &Path,
    source: PackageSource,
    scope: Option<String>,
) -> Option<PackageInfo> {
    if !dir.is_dir() {
        return None;
    }
    let manifest_path = dir.join("cls.json");
    let dir_name = dir
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let (fallback_name, fallback_ver) = split_name_version(&dir_name);

    let mut issues: Vec<String> = Vec::new();
    let manifest = if manifest_path.exists() {
        match ModuleManifest::from_file(&manifest_path) {
            Ok(m) => Some(m),
            Err(e) => {
                issues.push(format!("cls.json inválido: {}", e));
                None
            }
        }
    } else {
        issues.push("falta cls.json (manifiesto obligatorio)".to_string());
        None
    };

    let mut info = PackageInfo {
        name: manifest
            .as_ref()
            .map(|m| m.name.clone())
            .filter(|s| !s.is_empty())
            .unwrap_or(fallback_name),
        scope,
        version: manifest
            .as_ref()
            .map(|m| m.version.clone())
            .filter(|s| !s.is_empty())
            .unwrap_or(fallback_ver),
        description: manifest.as_ref().map(|m| m.description.clone()).unwrap_or_default(),
        license: manifest.as_ref().map(|m| m.license.clone()).unwrap_or_default(),
        authors: manifest.as_ref().map(|m| m.authors.clone()).unwrap_or_default(),
        target: manifest.as_ref().map(|m| m.project.target.clone()).unwrap_or_default(),
        entry: manifest.as_ref().map(|m| m.entry.clone()).unwrap_or_default(),
        export_entry: manifest
            .as_ref()
            .and_then(|m| m.export_config.as_ref().map(|e| e.entry.clone())),
        export_types: manifest
            .as_ref()
            .and_then(|m| m.export_config.as_ref().and_then(|e| e.types.clone())),
        dependencies: manifest
            .as_ref()
            .map(|m| {
                let mut d: Vec<String> = m.dependencies.keys().cloned().collect();
                d.extend(m.dev_dependencies.keys().cloned());
                d
            })
            .unwrap_or_default(),
        path: dir.to_path_buf(),
        source,
        manifest_ok: manifest.is_some(),
        issues,
    };
    info.dependencies.sort();

    // Coherencia de rutas declaradas.
    if let Some(e) = info.export_entry.clone() {
        if !dir.join(&e).exists() {
            info.issues.push(format!("export.entry '{}' no existe", e));
        }
    }
    if let Some(t) = info.export_types.clone() {
        if !dir.join(&t).exists() {
            info.issues.push(format!("export.types '{}' no existe", t));
        }
    }
    if !info.is_module() && !info.entry.is_empty() && !dir.join(&info.entry).exists() {
        info.issues.push(format!("entry '{}' no existe", info.entry));
    }
    Some(info)
}

/// Lista los paquetes de un directorio de módulos. Soporta el layout **anidado**
/// de scopes (`{dir}/{scope}/{name}@{version}/`): un subdir que no es paquete
/// (sin `cls.json`/`mod.clsx` ni `@`) se trata como scope y se recorre.
pub fn list_dir(dir: &Path, source: PackageSource) -> Vec<PackageInfo> {
    let mut out = Vec::new();
    list_dir_into(dir, source, &mut out);
    out.sort_by(|a, b| a.id().cmp(&b.id()).then(cmp_semver(&a.version, &b.version)));
    out
}

fn list_dir_into(dir: &Path, source: PackageSource, out: &mut Vec<PackageInfo>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if !p.is_dir() {
            continue;
        }
        let dn = e.file_name().to_string_lossy().to_string();
        if dn.starts_with('.') {
            continue;
        }
        let looks_package =
            p.join("cls.json").exists() || p.join("mod.clsx").exists() || dn.contains('@');
        if looks_package {
            if let Some(i) = inspect_dir(&p, source) {
                out.push(i);
            }
        } else {
            // Posible scope: sus hijos son paquetes con ese scope.
            let mut children = Vec::new();
            list_dir_into(&p, source, &mut children);
            for mut c in children {
                if c.scope.is_none() {
                    c.scope = Some(dn.clone());
                }
                out.push(c);
            }
        }
    }
}

/// Todos los paquetes locales (`{cwd}/modules/*`).
pub fn local_packages() -> Vec<PackageInfo> {
    list_dir(&local_modules_dir(), PackageSource::Local)
}

/// Todos los paquetes globales (`~/.cls/modules/*`).
pub fn global_packages() -> Vec<PackageInfo> {
    match global_modules_dir() {
        Some(d) => list_dir(&d, PackageSource::Global),
        None => Vec::new(),
    }
}

/// Busca un paquete por nombre (compatibilidad): parsea `[scope/]name[@ver]`.
pub fn find_by_name(name: &str) -> Option<PackageInfo> {
    let spec = PackageSpec::parse(name).ok()?;
    find_by_spec(&spec)
}

/// Busca un paquete por spec: primero local (`./modules`, solo unscoped), luego
/// global (`~/.cls/modules/[scope/]name[@version]`).
pub fn find_by_spec(spec: &PackageSpec) -> Option<PackageInfo> {
    if spec.scope.is_none() {
        let local = local_modules_dir().join(&spec.name);
        if local.is_dir() {
            return inspect_dir(&local, PackageSource::Local);
        }
    }
    let base = global_modules_dir()?;
    let pkg_dir = match &spec.scope {
        Some(s) => base.join(s).join(&spec.name),
        None => base.join(&spec.name),
    };
    // Sin versión: dir exacto (`name/`, sin `@`).
    if pkg_dir.is_dir() {
        return inspect_dir_scoped(&pkg_dir, PackageSource::Global, spec.scope.clone());
    }
    // Con versiones: `name@version` / `scope/name@version`.
    let parent = pkg_dir.parent()?;
    let prefix = format!("{}@", spec.name);
    let mut matches: Vec<PackageInfo> = Vec::new();
    if let Ok(rd) = fs::read_dir(parent) {
        for e in rd.flatten() {
            let p = e.path();
            if !p.is_dir() {
                continue;
            }
            let dn = e.file_name().to_string_lossy().to_string();
            let Some(ver) = dn.strip_prefix(&prefix) else { continue };
            if let Some(want) = &spec.version {
                if ver != want {
                    continue;
                }
            }
            if let Some(i) =
                inspect_dir_scoped(&p, PackageSource::Global, spec.scope.clone())
            {
                matches.push(i);
            }
        }
    }
    matches.sort_by(|a, b| cmp_semver(&a.version, &b.version));
    matches.pop()
}

// ── Paquetes archivados (.zip / .clsapp) ─────────────────────────────────────

/// Lee el `cls.json` de un zip y devuelve la info del paquete. No extrae nada.
pub fn inspect_zip(path: &Path) -> Result<PackageInfo, String> {
    let file = fs::File::open(path).map_err(|e| format!("no se pudo abrir '{}': {}", path.display(), e))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("zip inválido: {}", e))?;

    // Preferir el cls.json de menor profundidad (raíz del paquete dentro del zip).
    let mut best: Option<(usize, String)> = None;
    for i in 0..zip.len() {
        let name = zip
            .by_index(i)
            .map_err(|e| format!("zip corrupto: {}", e))?
            .name()
            .to_string();
        if name.ends_with("cls.json") {
            let depth = name.matches('/').count();
            if best.as_ref().map(|(d, _)| depth < *d).unwrap_or(true) {
                best = Some((depth, name));
            }
        }
    }
    let (depth, manifest_name) = best.ok_or_else(|| "el zip no contiene cls.json".to_string())?;
    let prefix = manifest_name
        .rsplit_once('/')
        .map(|(d, _)| format!("{}/", d))
        .unwrap_or_default();

    let mut content = String::new();
    zip.by_name(&manifest_name)
        .map_err(|e| format!("no se pudo leer cls.json: {}", e))?
        .read_to_string(&mut content)
        .map_err(|e| format!("cls.json no es UTF-8: {}", e))?;
    let _ = depth;
    let manifest: ModuleManifest =
        serde_json::from_str(&content).map_err(|e| format!("cls.json inválido: {}", e))?;
    let _ = prefix;

    Ok(info_from_manifest(manifest, path.to_path_buf(), PackageSource::Archive))
}

/// Extrae un zip (la subcarpeta que contiene `cls.json`) en `dest`.
pub fn extract_zip_to(path: &Path, dest: &Path) -> Result<(), String> {
    let file = fs::File::open(path).map_err(|e| format!("no se pudo abrir '{}': {}", path.display(), e))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("zip inválido: {}", e))?;

    // Subcarpeta raíz: la del cls.json de menor profundidad.
    let mut manifest_entry: Option<(usize, String)> = None;
    for i in 0..zip.len() {
        let name = zip
            .by_index(i)
            .map_err(|e| format!("zip corrupto: {}", e))?
            .name()
            .to_string();
        if name.ends_with("cls.json") {
            let depth = name.matches('/').count();
            if manifest_entry.as_ref().map(|(d, _)| depth < *d).unwrap_or(true) {
                manifest_entry = Some((depth, name));
            }
        }
    }
    let (_, manifest_name) = manifest_entry.ok_or_else(|| "el zip no contiene cls.json".to_string())?;
    let prefix = manifest_name
        .rsplit_once('/')
        .map(|(d, _)| format!("{}/", d))
        .unwrap_or_default();

    fs::create_dir_all(dest).map_err(|e| format!("no se pudo crear '{}': {}", dest.display(), e))?;

    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| format!("zip corrupto: {}", e))?;
        let raw = entry.name().to_string();
        if !raw.starts_with(&prefix) {
            continue;
        }
        let rel = &raw[prefix.len()..];
        if rel.is_empty() {
            continue;
        }
        // Seguridad: rechazar rutas con componentes `..` o absolutas.
        let rel_path = Path::new(rel);
        if rel_path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir | std::path::Component::RootDir))
        {
            return Err(format!("entrada zip insegura: '{}'", raw));
        }
        let out = dest.join(rel_path);
        if raw.ends_with('/') || entry.is_dir() {
            fs::create_dir_all(&out).map_err(|e| format!("no se pudo crear '{}': {}", out.display(), e))?;
            continue;
        }
        if let Some(parent) = out.parent() {
            fs::create_dir_all(parent).ok();
        }
        let mut buf = Vec::new();
        entry
            .read_to_end(&mut buf)
            .map_err(|e| format!("no se pudo leer '{}': {}", raw, e))?;
        fs::write(&out, &buf).map_err(|e| format!("no se pudo escribir '{}': {}", out.display(), e))?;
    }
    Ok(())
}

/// Construye un `PackageInfo` a partir de un manifiesto ya parseado.
fn info_from_manifest(manifest: ModuleManifest, path: PathBuf, source: PackageSource) -> PackageInfo {
    let export_entry = manifest.export_config.as_ref().map(|e| e.entry.clone());
    PackageInfo {
        name: manifest.name.clone(),
        scope: None,
        version: manifest.version.clone(),
        description: manifest.description.clone(),
        license: manifest.license.clone(),
        authors: manifest.authors.clone(),
        target: manifest.project.target.clone(),
        entry: manifest.entry.clone(),
        export_entry,
        export_types: manifest.export_config.as_ref().and_then(|e| e.types.clone()),
        dependencies: {
            let mut d: Vec<String> = manifest.dependencies.keys().cloned().collect();
            d.extend(manifest.dev_dependencies.keys().cloned());
            d.sort();
            d
        },
        path,
        source,
        manifest_ok: true,
        issues: Vec::new(),
    }
}

// ── Utilidades de copia ──────────────────────────────────────────────────────

/// Copia recursiva de un directorio (sin seguir symlinks; ignora `.git`/`.cls-cache`).
pub fn copy_dir_all(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| format!("no se pudo crear '{}': {}", dst.display(), e))?;
    for entry in fs::read_dir(src).map_err(|e| format!("no se pudo leer '{}': {}", src.display(), e))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name == ".git" || name == ".cls-cache" || name == "target" {
            continue;
        }
        let from = entry.path();
        let to = dst.join(&name);
        if from.is_dir() {
            copy_dir_all(&from, &to)?;
        } else {
            fs::copy(&from, &to).map_err(|e| {
                format!("no se pudo copiar '{}' -> '{}': {}", from.display(), to.display(), e)
            })?;
        }
    }
    Ok(())
}

// ── Glob (mínimo: `*`, `**`, `?`) ────────────────────────────────────────────

/// ¿`text` (relativo, con `/`) matchea el patrón glob `pat`?
/// Soporta `*` (cualquier cosa salvo `/`), `**` (cualquier cosa incl. `/`) y `?`.
pub fn glob_match(pat: &str, text: &str) -> bool {
    let p: Vec<char> = pat.chars().collect();
    let t: Vec<char> = text.chars().collect();
    fn rec(p: &[char], t: &[char]) -> bool {
        if p.is_empty() {
            return t.is_empty();
        }
        match p[0] {
            '*' => {
                if p.len() >= 2 && p[1] == '*' {
                    let rest = if p.len() >= 3 && p[2] == '/' { &p[3..] } else { &p[2..] };
                    for i in 0..=t.len() {
                        if rec(rest, &t[i..]) {
                            return true;
                        }
                    }
                    false
                } else {
                    let rest = &p[1..];
                    if rec(rest, t) {
                        return true;
                    }
                    let mut i = 0;
                    while i < t.len() && t[i] != '/' {
                        i += 1;
                        if rec(rest, &t[i..]) {
                            return true;
                        }
                    }
                    false
                }
            }
            '?' => !t.is_empty() && t[0] != '/' && rec(&p[1..], &t[1..]),
            c => !t.is_empty() && t[0] == c && rec(&p[1..], &t[1..]),
        }
    }
    rec(&p, &t)
}

/// Recolecta recursivamente los `.clsx` bajo `base` (relativos, con `/`).
pub fn collect_clsx(base: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        if let Ok(rd) = fs::read_dir(dir) {
            for e in rd.flatten() {
                let p = e.path();
                let name = e.file_name().to_string_lossy().to_string();
                if name.starts_with('.') || name == "target" || name == "modules" {
                    continue;
                }
                if p.is_dir() {
                    walk(&p, out);
                } else if p.extension().map(|x| x == "clsx").unwrap_or(false) {
                    out.push(p);
                }
            }
        }
    }
    walk(base, &mut out);
    out
}

/// Expande una lista de patrones glob (relativos a `base`) a archivos `.clsx`.
/// Un patrón sin metacaracteres se trata como ruta literal.
pub fn expand_globs(base: &Path, patterns: &[String]) -> Vec<PathBuf> {
    let files = collect_clsx(base);
    let mut result: Vec<PathBuf> = Vec::new();
    for pat in patterns {
        let norm = pat.replace('\\', "/");
        let is_glob = norm.contains('*') || norm.contains('?') || norm.contains('[');
        if !is_glob {
            let literal = base.join(&norm);
            if literal.is_file() {
                result.push(literal);
            }
            continue;
        }
        for f in &files {
            let rel = f
                .strip_prefix(base)
                .map(|r| r.to_string_lossy().replace('\\', "/"))
                .unwrap_or_default();
            if glob_match(&norm, &rel) {
                result.push(f.clone());
            }
        }
    }
    result.sort();
    result.dedup();
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_spec_forms() {
        let s = PackageSpec::parse("foo").unwrap();
        assert_eq!(s.scope, None);
        assert_eq!(s.name, "foo");
        assert_eq!(s.version, None);
        assert_eq!(s.id(), "foo");

        let s = PackageSpec::parse("foo@1.2.3").unwrap();
        assert_eq!(s.version.as_deref(), Some("1.2.3"));

        let s = PackageSpec::parse("usuario/foo").unwrap();
        assert_eq!(s.scope.as_deref(), Some("usuario"));
        assert_eq!(s.name, "foo");
        assert_eq!(s.id(), "usuario/foo");

        let s = PackageSpec::parse("usuario/foo@2.0.0-dev.1").unwrap();
        assert_eq!(s.id(), "usuario/foo");
        assert!(is_prerelease(s.version.as_deref().unwrap()));
    }

    #[test]
    fn parse_spec_rejects() {
        assert!(PackageSpec::parse("").is_err());
        assert!(PackageSpec::parse("--x").is_err());
        assert!(PackageSpec::parse("foo@nope").is_err());
        assert!(PackageSpec::parse("a/b/c").is_err());
        assert!(PackageSpec::parse("bad name").is_err());
    }

    #[test]
    fn semver_compare_and_ranges() {
        assert_eq!(cmp_semver("1.2.0", "1.10.0"), std::cmp::Ordering::Less);
        assert_eq!(cmp_semver("2.0.0", "2.0.0"), std::cmp::Ordering::Equal);
        assert!(version_satisfies("^1.2.0", "1.9.0"));
        assert!(!version_satisfies("^1.2.0", "2.0.0"));
        assert!(!version_satisfies("^1.2.0", "1.1.0"));
        assert!(version_satisfies("~1.2.0", "1.2.9"));
        assert!(!version_satisfies("~1.2.0", "1.3.0"));
        assert!(version_satisfies("1.0.0", "1.0.0"));
        assert!(!version_satisfies("1.0.0", "1.0.1"));
        assert!(version_satisfies(">=1.0", "1.5.0"));
        assert!(version_satisfies("*", "9.9.9"));
    }

    #[test]
    fn valid_semver_shapes() {
        assert!(is_valid_semver("1.0.0"));
        assert!(is_valid_semver("1.2.3-dev.1"));
        assert!(is_valid_semver("1.2.3+build"));
        assert!(!is_valid_semver(""));
        assert!(!is_valid_semver("v1.0.0"));
        assert!(!is_valid_semver("1.x.0"));
    }

    #[test]
    fn glob_matching() {
        assert!(glob_match("**/*.clsx", "tests/a.clsx"));
        assert!(glob_match("**/*.clsx", "a.clsx"));
        assert!(glob_match("tests/*.clsx", "tests/b.clsx"));
        assert!(!glob_match("tests/*.clsx", "tests/sub/b.clsx"));
        assert!(glob_match("tests/?.clsx", "tests/a.clsx"));
    }
}

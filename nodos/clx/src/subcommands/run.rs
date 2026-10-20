//! `clx run` - Ejecuta un programa CLS usando el motor JIT.
//!
//! Migracion dev-2 (Fase 7): se elimino el path `--ast-walker` (tree-walker
//! deprecado). El binario es 100% JIT ahora. El walker completo se borro
//! de `cls-runtime` en esta fase.

use cls_core::config::ModuleManifest;
use std::path::Path;

pub fn execute(args: &[String]) -> i32 {
    // Help manual del subcomando
    if args.iter().take_while(|a| *a != "--").any(|a| a == "-h" || a == "--help") {
        print_help();
        return 0;
    }

    // Separar args de la app (todo después de --)
    let app_args: Vec<String> = args.iter()
        .skip_while(|a| *a != "--")
        .skip(1)
        .map(|s| s.to_string())
        .collect();

    // Entry: ignorar flags --jit/-j, --ast-walker (deprecado) y --target <valor>
    // al resolver el archivo. `--ast-walker` se acepta silenciosamente por
    // compatibilidad hacia atras (se elimino en Fase 7).
    let mut cli_args: Vec<String> = Vec::new();
    let mut skip_next = false;
    for a in args.iter().take_while(|a| *a != "--") {
        if skip_next {
            skip_next = false;
            continue;
        }
        if a == "--jit" || a == "-j" || a == "--ast-walker" {
            continue;
        }
        if a == "--target" || a == "-t" {
            skip_next = true;
            continue;
        }
        if a.starts_with("--add-path-module=") {
            continue;
        }
        if a == "--add-path-module" {
            skip_next = true;
            continue;
        }
        cli_args.push(a.clone());
    }

    let config = load_config();
    let entry = resolve_entry(&cli_args, config.as_ref());

    // Rutas adicionales de búsqueda de módulos: `cls.json` -> `pathModules`
    // (CWD y proyecto del entry) + `--add-path-module=<path>` (repetible).
    // Ver examples/patrones.md §11.
    let extra_paths = collect_extra_paths(&entry, config.as_ref(), args);
    if !extra_paths.is_empty() {
        cls_jit::set_extra_module_paths(extra_paths);
    }

    // `--target <tripla>` -> simula el entorno para la directiva `when`
    let target_opt: Option<String> = {
        let a: Vec<&String> = args.iter().take_while(|a| *a != "--").collect();
        let mut t: Option<String> = None;
        let mut i = 0;
        while i < a.len() {
            if a[i] == "--target" || a[i] == "-t" {
                if let Some(v) = a.get(i + 1) {
                    t = Some(v.to_string());
                }
                break;
            }
            if let Some(v) = a[i].strip_prefix("--target=") {
                t = Some(v.to_string());
                break;
            }
            i += 1;
        }
        t
    };

    crate::jit::run_jit(&entry, &app_args, target_opt.as_deref())
}

fn print_help() {
    println!("clx run - Ejecutar un programa CLS");
    println!();
    println!("Uso: clx run [archivo] [--] [args...]");
    println!();
    println!("Opciones:");
    println!("  --jit, -j               (obsoleto) El JIT ya es el intérprete por defecto");
    println!("  --target <tripla>, -t   Simular el entorno para la directiva 'when'");
    println!("  --add-path-module <dir> Ruta adicional de búsqueda de módulos (repetible)");
    println!("  -h, --help              Mostrar esta ayuda");
    println!("  --                      Separar los args de la aplicación");
    println!();
    println!("Sin archivo, usa el 'entry' de cls.json (o busca main.clsx).");
    println!();
    println!("El JIT (CLS -> WASM) es el intérprete. El AST-walker se elimino");
    println!("en dev-2 (Fase 7).");
}

fn resolve_entry(args: &[String], config: Option<&ModuleManifest>) -> String {
    // Solo considerar args antes de -- (después son app_args)
    let entry_arg = args.iter().take_while(|a| *a != "--").find(|a| !a.starts_with("-"));
    if let Some(e) = entry_arg {
        return e.to_string();
    }
    // Si no, usar entry de cls.json
    if let Some(cfg) = config {
        let proposed = &cfg.entry;
        if !proposed.is_empty() && Path::new(proposed).exists() {
            return proposed.clone();
        }
        // Si el entry no existe, buscar main.clsx
        let candidates = ["main.clsx", "src/main.clsx", "mod.clsx", "src/mod.clsx"];
        for c in &candidates {
            if Path::new(c).exists() {
                return c.to_string();
            }
        }
    }
    eprintln!("Uso: clx run <archivo> [--] [args...]");
    eprintln!("  (o ejecuta desde un proyecto con cls.json que tenga 'entry')");
    std::process::exit(1);
}

/// Rutas adicionales de búsqueda de módulos, en orden y sin duplicados:
///   1. `pathModules` del `cls.json` del CWD,
///   2. `pathModules` del `cls.json` del proyecto del entry (si es otro),
///   3. cada `--add-path-module=<path>` (o `--add-path-module <path>`).
/// Las relativas se resuelven contra el directorio que las declara (los flags,
/// contra el CWD). Ver `examples/patrones.md` §11.
pub(crate) fn collect_extra_paths(
    entry: &str,
    config: Option<&ModuleManifest>,
    args: &[String],
) -> Vec<std::path::PathBuf> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let mut out: Vec<std::path::PathBuf> = Vec::new();

    // 1) cls.json del CWD (el que `load_config` ya cargó).
    if let Some(cfg) = config {
        push_manifest_paths(&cfg.path_modules, &cwd, &mut out);
    }
    // 2) cls.json del proyecto del entry (si difiere del CWD).
    if let Some(dir) = find_manifest_dir(Path::new(entry)) {
        if dir != cwd {
            if let Ok(m) = ModuleManifest::from_file(&dir.join("cls.json")) {
                push_manifest_paths(&m.path_modules, &dir, &mut out);
            }
        }
    }
    // 3) Flags `--add-path-module` (relativos al CWD).
    let a: Vec<&String> = args.iter().take_while(|a| *a != "--").collect();
    let mut i = 0;
    while i < a.len() {
        if let Some(v) = a[i].strip_prefix("--add-path-module=") {
            push_unique(resolve_relative(v, &cwd), &mut out);
        } else if a[i] == "--add-path-module" {
            if let Some(v) = a.get(i + 1) {
                push_unique(resolve_relative(v.as_str(), &cwd), &mut out);
            }
            i += 1;
        }
        i += 1;
    }
    out
}

/// Resuelve `p` contra `base` si es relativa.
fn resolve_relative(p: &str, base: &Path) -> std::path::PathBuf {
    let pb = Path::new(p);
    if pb.is_absolute() {
        pb.to_path_buf()
    } else {
        base.join(pb)
    }
}

/// Agrega las rutas de `pathModules` (relativas a `base`), sin duplicados.
fn push_manifest_paths(declared: &[String], base: &Path, out: &mut Vec<std::path::PathBuf>) {
    for p in declared {
        push_unique(resolve_relative(p, base), out);
    }
}

/// `out.push(p)` si no estaba ya.
fn push_unique(p: std::path::PathBuf, out: &mut Vec<std::path::PathBuf>) {
    let key = p.to_string_lossy().to_string();
    if !out.iter().any(|q| q.to_string_lossy() == key) {
        out.push(p);
    }
}

/// Directorio del `cls.json` propio o del ancestro más cercano de `start`.
fn find_manifest_dir(start: &Path) -> Option<std::path::PathBuf> {
    let mut dir = if start.is_dir() {
        Some(start.to_path_buf())
    } else {
        start.parent().map(|p| p.to_path_buf())
    };
    while let Some(d) = dir {
        if d.join("cls.json").exists() {
            return Some(d);
        }
        dir = d.parent().map(|p| p.to_path_buf());
    }
    None
}

fn load_config() -> Option<ModuleManifest> {
    let cwd = std::env::current_dir().ok()?;
    let path = cwd.join("cls.json");
    if path.exists() {
        ModuleManifest::from_file(&path).ok()
    } else {
        None
    }
}

//! `clx test` - Descubre y ejecuta los tests del proyecto.
//!
//! Los archivos se toman de `testEntry` en `cls.json` (array de rutas o globs)
//! o de los argumentos posicionales. Cada archivo se ejecuta con el JIT; su
//! exit code determina pass/fail.
//!
//! NOTA: el soporte de lenguaje para tests (`test(...)`, aserciones y nombres
//! reservados) es la siguiente fase. Por ahora un archivo de test es un
//! programa CLS normal (con `main`).

use std::path::Path;

use cls_core::config::ModuleManifest;

use crate::packages;

pub fn execute(args: &[String]) -> i32 {
    if args.iter().take_while(|a| *a != "--").any(|a| a == "-h" || a == "--help") {
        print_help();
        return 0;
    }

    // Flag --list: solo descubrir, no ejecutar.
    let list_only = args.iter().any(|a| a == "--list");

    // --filter <substr> (o --filter=<substr>)
    let filter: Option<String> = {
        let mut f = None;
        let mut i = 0;
        while i < args.len() {
            if args[i] == "--filter" {
                f = args.get(i + 1).cloned();
                break;
            }
            if let Some(v) = args[i].strip_prefix("--filter=") {
                f = Some(v.to_string());
                break;
            }
            i += 1;
        }
        f
    };

    let cwd = std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());

    // Patrones: args posicionales > testEntry del manifiesto.
    let mut patterns: Vec<String> = args
        .iter()
        .take_while(|a| *a != "--")
        .filter(|a| !a.starts_with('-'))
        .cloned()
        .collect();

    let mut from_manifest = false;
    if patterns.is_empty() {
        let manifest_path = cwd.join("cls.json");
        if manifest_path.exists() {
            match ModuleManifest::from_file(&manifest_path) {
                Ok(m) => {
                    patterns = m.test_entry.clone();
                    from_manifest = !patterns.is_empty();
                }
                Err(e) => {
                    eprintln!("Error: cls.json inválido: {}", e);
                    return 1;
                }
            }
        }
    }

    if patterns.is_empty() {
        eprintln!("No se encontraron tests.");
        eprintln!("Define archivos o globs en 'testEntry' de cls.json, p.ej.:");
        eprintln!("  \"testEntry\": [\"tests/**/*.clsx\"]");
        eprintln!("o pasa rutas: clx test tests/foo.clsx");
        return 1;
    }

    let mut files = packages::expand_globs(&cwd, &patterns);
    if let Some(f) = &filter {
        files.retain(|p| p.to_string_lossy().contains(f.as_str()));
    }

    if files.is_empty() {
        eprintln!(
            "No se resolvió ningún archivo de test desde {} patrón(es){}.",
            patterns.len(),
            if from_manifest { " (testEntry)" } else { "" }
        );
        for p in &patterns {
            eprintln!("  - {}", p);
        }
        return 1;
    }

    if list_only {
        println!("Tests descubiertos ({}):", files.len());
        for f in &files {
            println!("  {}", f.display());
        }
        return 0;
    }

    println!("Ejecutando {} test(s):", files.len());
    println!();

    let mut passed = 0usize;
    let mut failed = 0usize;

    for f in &files {
        let entry = f.to_string_lossy().to_string();
        println!("  {}", rel_display(&cwd, f));
        use std::io::Write;
        std::io::stdout().flush().ok();
        let code = crate::jit::run_jit(&entry, &[], None);
        if code == 0 {
            println!("    -> ok");
            passed += 1;
        } else {
            println!("    -> FAIL (exit {})", code);
            failed += 1;
        }
    }

    println!();
    println!("Resultado: {} PASS, {} FAIL, {} total", passed, failed, files.len());
    if failed == 0 {
        0
    } else {
        1
    }
}

fn rel_display(cwd: &Path, f: &Path) -> String {
    f.strip_prefix(cwd)
        .map(|r| r.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| f.display().to_string())
}

fn print_help() {
    println!("clx test - Ejecuta los tests del proyecto");
    println!();
    println!("Uso: clx test [archivos|globs...] [--filter <substr>] [--list]");
    println!();
    println!("Sin argumentos, usa 'testEntry' de cls.json (array de rutas o globs).");
    println!("Cada archivo se ejecuta con el JIT; su exit code determina pass/fail.");
    println!();
    println!("  --filter <substr>   Solo ejecutar tests cuya ruta contenga <substr>");
    println!("  --list              Listar los tests descubiertos sin ejecutarlos");
    println!();
    println!("Pendiente (fase siguiente): intrinsics de test (`test(...)`, aserciones,");
    println!("nombres reservados). Hoy un test es un programa CLS normal (con main).");
}

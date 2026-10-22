use std::fs;
use std::path::Path;
use cls_core::config::ModuleManifest;

fn print_help() {
    println!("clx new - Crear proyecto CLS");
    println!();
    println!("Uso: clx new <nombre> [opciones]");
    println!();
    println!("Proyecto:");
    println!("  --lib                    Crear una libreria (target library, sin main)");
    println!("  --entry <ruta>           Entry point (default: <source-dir>/main.clsx)");
    println!("  --target <tipo>          executable | library (default: executable)");
    println!("  --source-dir <dir>       Directorio de fuentes (default: src)");
    println!("  --out-dir <dir>          Directorio de salida (default: dist)");
    println!("  --test-entry <glob>      Patron de tests (repetible; default: vacio)");
    println!("  --path-module <dir>      Ruta extra de modulos (repetible)");
    println!("  --path-module-native <dir>  Ruta de librerias nativas (repetible)");
    println!();
    println!("Metadata:");
    println!("  --version <semver>       Version del proyecto (default: 0.1.0)");
    println!("  --description <texto>    Descripcion");
    println!("  --author <nombre>        Autor (repetible)");
    println!("  --license <id>           Licencia (default: MIT)");
    println!("  --registry <url>         Registry de dependencias");
    println!();
    println!("Tipos y compilador:");
    println!("  --strict                 Activar tipado estricto");
    println!("  --no-strict              Desactivar tipado estricto");
    println!("  --no-check               Desactivar chequeo de tipos");
    println!("  --no-source-maps         Desactivar source maps");
    println!("  --feature-async          Habilitar feature async");
    println!("  --feature-macros         Habilitar feature macros");
    println!("  --feature-experimental   Habilitar feature experimental");
    println!();
    println!("Runtime / sandbox:");
    println!("  --memory-limit <size>    Limite de memoria (default: 1GiB)");
    println!("  --stack-size <size>      Stack (default: 8MB)");
    println!("  --max-execution-time <ms>  Timeout de ejecucion (0 = sin limite)");
    println!("  --max-output-bytes <n>   Limite de salida (0 = sin limite)");
    println!("  --max-file-size <n>      Limite de archivo (0 = sin limite)");
    println!("  --gc                     Habilitar GC");
    println!("  --gc-threshold <size>    Umbral de GC (default: 64MB)");
    println!("  --allow-fs               Permitir filesystem");
    println!("  --allow-net              Permitir red");
    println!("  --allow-process          Permitir procesos");
    println!("  --allow-env              Permitir variables de entorno");
    println!();
    println!("  -h, --help               Muestra esta ayuda");
}

fn matches_flag(arg: &str, flag: &str) -> bool {
    arg == flag || arg.starts_with(&format!("{}=", flag))
}

fn take_value(args: &[String], i: &mut usize, flag: &str) -> Result<String, String> {
    let arg = &args[*i];
    if let Some(v) = arg.strip_prefix(&format!("{}=", flag)) {
        return Ok(v.to_string());
    }
    *i += 1;
    args.get(*i)
        .cloned()
        .ok_or_else(|| format!("falta valor para {}", flag))
}

fn parse_u64(value: &str, flag: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|_| format!("{} espera un entero no negativo (recibi '{}')", flag, value))
}

pub fn execute(args: &[String]) -> i32 {
    if args.iter().any(|a| a == "-h" || a == "--help") {
        print_help();
        return 0;
    }

    let mut i = 0usize;
    macro_rules! value {
        ($flag:expr) => {
            match take_value(args, &mut i, $flag) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    return 1;
                }
            }
        };
    }

    let mut name: Option<String> = None;
    let mut is_lib = false;
    let mut version = "0.1.0".to_string();
    let mut description = String::new();
    let mut authors: Vec<String> = Vec::new();
    let mut license = "MIT".to_string();
    let mut registry = "https://registry.cls-lang.org".to_string();
    let mut entry: Option<String> = None;
    let mut target: Option<String> = None;
    let mut source_dir = "src".to_string();
    let mut out_dir = "dist".to_string();
    let mut test_entries: Vec<String> = Vec::new();
    let mut path_modules: Vec<String> = Vec::new();
    let mut path_natives: Vec<String> = Vec::new();

    let mut check = true;
    let mut strict = false;
    let mut source_maps = true;
    let mut feature_async = false;
    let mut feature_macros = false;
    let mut feature_experimental = false;

    let mut memory_limit = "1GiB".to_string();
    let mut stack_size = "8MB".to_string();
    let mut max_execution_time = 0u64;
    let mut max_output_bytes = 0u64;
    let mut max_file_size = 0u64;
    let mut gc_enabled = false;
    let mut gc_threshold = "64MB".to_string();
    let mut allow_fs = false;
    let mut allow_net = false;
    let mut allow_process = false;
    let mut allow_env = false;

    while i < args.len() {
        let arg = args[i].as_str();

        if arg == "--lib" {
            is_lib = true;
            i += 1;
            continue;
        }
        if arg == "--strict" {
            strict = true;
            i += 1;
            continue;
        }
        if arg == "--no-strict" {
            strict = false;
            i += 1;
            continue;
        }
        if arg == "--no-check" {
            check = false;
            i += 1;
            continue;
        }
        if arg == "--no-source-maps" {
            source_maps = false;
            i += 1;
            continue;
        }
        if arg == "--feature-async" {
            feature_async = true;
            i += 1;
            continue;
        }
        if arg == "--feature-macros" {
            feature_macros = true;
            i += 1;
            continue;
        }
        if arg == "--feature-experimental" {
            feature_experimental = true;
            i += 1;
            continue;
        }
        if arg == "--gc" {
            gc_enabled = true;
            i += 1;
            continue;
        }
        if arg == "--allow-fs" {
            allow_fs = true;
            i += 1;
            continue;
        }
        if arg == "--allow-net" {
            allow_net = true;
            i += 1;
            continue;
        }
        if arg == "--allow-process" {
            allow_process = true;
            i += 1;
            continue;
        }
        if arg == "--allow-env" {
            allow_env = true;
            i += 1;
            continue;
        }

        if matches_flag(arg, "--version") {
            version = value!("--version");
            i += 1;
            continue;
        }
        if matches_flag(arg, "--description") {
            description = value!("--description");
            i += 1;
            continue;
        }
        if matches_flag(arg, "--author") {
            authors.push(value!("--author"));
            i += 1;
            continue;
        }
        if matches_flag(arg, "--license") {
            license = value!("--license");
            i += 1;
            continue;
        }
        if matches_flag(arg, "--registry") {
            registry = value!("--registry");
            i += 1;
            continue;
        }
        if matches_flag(arg, "--entry") {
            entry = Some(value!("--entry"));
            i += 1;
            continue;
        }
        if matches_flag(arg, "--target") {
            target = Some(value!("--target"));
            i += 1;
            continue;
        }
        if matches_flag(arg, "--source-dir") {
            source_dir = value!("--source-dir");
            i += 1;
            continue;
        }
        if matches_flag(arg, "--out-dir") {
            out_dir = value!("--out-dir");
            i += 1;
            continue;
        }
        if matches_flag(arg, "--test-entry") {
            test_entries.push(value!("--test-entry"));
            i += 1;
            continue;
        }
        if matches_flag(arg, "--path-module") {
            path_modules.push(value!("--path-module"));
            i += 1;
            continue;
        }
        if matches_flag(arg, "--path-module-native") {
            path_natives.push(value!("--path-module-native"));
            i += 1;
            continue;
        }
        if matches_flag(arg, "--memory-limit") {
            memory_limit = value!("--memory-limit");
            i += 1;
            continue;
        }
        if matches_flag(arg, "--stack-size") {
            stack_size = value!("--stack-size");
            i += 1;
            continue;
        }
        if matches_flag(arg, "--gc-threshold") {
            gc_threshold = value!("--gc-threshold");
            i += 1;
            continue;
        }
        if matches_flag(arg, "--max-execution-time") {
            match parse_u64(&value!("--max-execution-time"), "--max-execution-time") {
                Ok(v) => max_execution_time = v,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    return 1;
                }
            }
            i += 1;
            continue;
        }
        if matches_flag(arg, "--max-output-bytes") {
            match parse_u64(&value!("--max-output-bytes"), "--max-output-bytes") {
                Ok(v) => max_output_bytes = v,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    return 1;
                }
            }
            i += 1;
            continue;
        }
        if matches_flag(arg, "--max-file-size") {
            match parse_u64(&value!("--max-file-size"), "--max-file-size") {
                Ok(v) => max_file_size = v,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    return 1;
                }
            }
            i += 1;
            continue;
        }

        if arg.starts_with('-') {
            eprintln!("Error: opcion desconocida '{}' (usa 'clx new -h')", arg);
            return 1;
        }
        if name.is_some() {
            eprintln!("Error: argumento extra '{}'", arg);
            return 1;
        }
        name = Some(arg.to_string());
        i += 1;
    }

    let name = match name {
        Some(n) => n,
        None => {
            eprintln!("Uso: clx new <nombre> [opciones]");
            eprintln!("Usa 'clx new -h' para ver todas las opciones.");
            return 1;
        }
    };
    if name.starts_with('-') {
        eprintln!(
            "Error: '{}' no es un nombre de proyecto valido (los nombres no empiezan con '-'; usa 'clx new -h' para ayuda)",
            name
        );
        return 1;
    }

    if let Some(ref t) = target {
        if t != "executable" && t != "library" {
            eprintln!("Error: --target espera 'executable' o 'library' (recibi '{}')", t);
            return 1;
        }
    }
    if is_lib {
        target = Some("library".to_string());
    }
    let target = target.unwrap_or_else(|| "executable".to_string());
    let is_lib = target == "library";

    let entry = match entry {
        Some(e) => e,
        None if is_lib => String::new(),
        None => format!("{}/main.clsx", source_dir),
    };
    if !is_lib && entry.is_empty() {
        eprintln!("Error: un proyecto executable necesita --entry");
        return 1;
    }

    let dir = Path::new(&name);
    if dir.exists() {
        eprintln!("Error: el directorio '{}' ya existe", name);
        return 1;
    }

    let src_dir = dir.join(&source_dir);
    let mod_dir = dir.join("modules");
    if let Err(e) = fs::create_dir_all(&src_dir) {
        eprintln!("Error al crear '{}': {}", src_dir.display(), e);
        return 1;
    }
    if let Err(e) = fs::create_dir_all(&mod_dir) {
        eprintln!("Error al crear '{}': {}", mod_dir.display(), e);
        return 1;
    }

    let mut manifest = ModuleManifest::default_for(&name);
    manifest.version = version;
    manifest.description = description;
    manifest.authors = authors;
    manifest.license = license;
    manifest.registry = registry;
    manifest.entry = entry.clone();
    manifest.project.source_dir = source_dir;
    manifest.project.out_dir = out_dir;
    manifest.project.target = target.clone();
    manifest.compiler.types.check = check;
    manifest.compiler.types.strict = strict && check;
    manifest.compiler.source_maps = source_maps;
    manifest.compiler.features.async_ = feature_async;
    manifest.compiler.features.macros = feature_macros;
    manifest.compiler.features.experimental = feature_experimental;
    manifest.interpreter.runtime.memory_limit = memory_limit;
    manifest.interpreter.runtime.stack_size = stack_size;
    manifest.interpreter.runtime.gc.enabled = gc_enabled;
    manifest.interpreter.runtime.gc.threshold = gc_threshold;
    manifest.interpreter.sandbox.allow_fs = allow_fs;
    manifest.interpreter.sandbox.allow_net = allow_net;
    manifest.interpreter.sandbox.allow_process = allow_process;
    manifest.interpreter.sandbox.allow_env = allow_env;
    manifest.interpreter.sandbox.max_execution_time = max_execution_time;
    manifest.interpreter.sandbox.max_output_bytes = max_output_bytes;
    manifest.interpreter.sandbox.max_file_size = max_file_size;
    manifest.test_entry = test_entries;
    manifest.path_modules = path_modules;
    manifest.path_modules_natives = path_natives;

    if let Err(e) = manifest.save(&dir.join("cls.json")) {
        eprintln!("Error al escribir '{}': {}", dir.join("cls.json").display(), e);
        return 1;
    }

    if !is_lib {
        let entry_path = dir.join(&entry);
        if let Some(parent) = entry_path.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                eprintln!("Error al crear '{}': {}", parent.display(), e);
                return 1;
            }
        }
        let main_content = r#"function main(args: String[]) -> int {
    print("Hello from CLS!");
    return 0;
}
"#;
        if let Err(e) = fs::write(&entry_path, main_content) {
            eprintln!("Error al escribir '{}': {}", entry_path.display(), e);
            return 1;
        }
    }

    if let Err(e) = fs::write(
        dir.join(".gitignore"),
        "modules/\ndist/\n.cls-types\n.cls-cache/\n",
    ) {
        eprintln!("Error al escribir '{}': {}", dir.join(".gitignore").display(), e);
        return 1;
    }

    println!("Proyecto '{}' creado", name);
    println!("  entry: {}", if is_lib { "(libreria)" } else { entry.as_str() });
    println!("  target: {}", target);
    println!("  cls.json: completo (compilador, runtime, sandbox y paths)");
    0
}

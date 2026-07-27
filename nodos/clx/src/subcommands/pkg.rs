use std::fs;
use std::path::Path;

pub fn execute(cmd: &str, args: &[String]) -> i32 {
    match cmd {
        "add" => cmd_add(args),
        "remove" | "rm" => cmd_remove(args),
        "install" | "i" => cmd_install(args),
        _ => {
            eprintln!("Comando desconocido: clx {}", cmd);
            eprintln!("Uso: clx add <paquete> | clx remove <paquete> | clx install");
            1
        }
    }
}

fn cmd_add(args: &[String]) -> i32 {
    if args.is_empty() {
        eprintln!("Uso: clx add <paquete> [--dev]");
        return 1;
    }
    let pkg = &args[0];
    let is_dev = args.iter().any(|a| a == "--dev");

    let manifest_path = "cls.json";
    if !Path::new(manifest_path).exists() {
        eprintln!("Error: cls.json no encontrado. Ejecuta 'clx new' primero");
        return 1;
    }

    let content = fs::read_to_string(manifest_path).unwrap();
    let mut json: serde_json::Value = serde_json::from_str(&content).unwrap();

    let section = if is_dev { "devDependencies" } else { "dependencies" };
    json[section][pkg] = serde_json::Value::String("^1.0.0".to_string());

    fs::write(manifest_path, serde_json::to_string_pretty(&json).unwrap()).unwrap();
    println!("✅ '{}' agregado a {}", pkg, section);
    0
}

fn cmd_remove(args: &[String]) -> i32 {
    if args.is_empty() {
        eprintln!("Uso: clx remove <paquete>");
        return 1;
    }
    let pkg = &args[0];
    let manifest_path = "cls.json";

    if !Path::new(manifest_path).exists() {
        eprintln!("Error: cls.json no encontrado");
        return 1;
    }

    let content = fs::read_to_string(manifest_path).unwrap();
    let mut json: serde_json::Value = serde_json::from_str(&content).unwrap();

    let removed = json["dependencies"].as_object_mut().map(|d| d.remove(pkg).is_some()).unwrap_or(false)
        || json["devDependencies"].as_object_mut().map(|d| d.remove(pkg).is_some()).unwrap_or(false);

    if removed {
        fs::write(manifest_path, serde_json::to_string_pretty(&json).unwrap()).unwrap();
        println!("✅ '{}' removido", pkg);
    } else {
        eprintln!("'{}' no encontrado en dependencias", pkg);
        return 1;
    }
    0
}

fn cmd_install(_args: &[String]) -> i32 {
    let manifest_path = "cls.json";
    if !Path::new(manifest_path).exists() {
        eprintln!("Error: cls.json no encontrado");
        return 1;
    }

    let content = fs::read_to_string(manifest_path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();

    // Registry: env var > cls.json > default
    let registry = std::env::var("CLS_REGISTRY")
        .or_else(|_| json["registry"].as_str().map(|s| s.to_string()).ok_or(()))
        .unwrap_or_else(|_| "https://registry.cls-lang.org".to_string());

    eprintln!("[clx install] No implementado aún (registry: {})", registry);
    0
}

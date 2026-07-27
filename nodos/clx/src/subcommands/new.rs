use std::fs;
use std::path::Path;

pub fn execute(args: &[String]) -> i32 {
    if args.is_empty() {
        eprintln!("Uso: clx new <nombre> [--lib]");
        return 1;
    }
    let name = &args[0];
    let is_lib = args.iter().any(|a| a == "--lib");

    let dir = Path::new(name);
    if dir.exists() {
        eprintln!("Error: el directorio '{}' ya existe", name);
        return 1;
    }

    // Crear estructura de directorios
    let src_dir = dir.join("src");
    let mod_dir = dir.join("modules");
    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&mod_dir).unwrap();

    // cls.json
    let registry = std::env::var("CLS_REGISTRY")
        .unwrap_or_else(|_| "https://registry.cls-lang.org".to_string());
    let manifest = serde_json::json!({
        "name": name,
        "version": "0.1.0",
        "entry": if is_lib { "" } else { "src/main.clsx" },
        "description": "",
        "authors": [],
        "license": "MIT",
        "registry": registry,
        "project": {
            "sourceDir": "src",
            "outDir": "dist",
            "target": if is_lib { "library" } else { "executable" }
        },
        "dependencies": {},
        "devDependencies": {}
    });
    fs::write(dir.join("cls.json"), serde_json::to_string_pretty(&manifest).unwrap()).unwrap();

    // main.clsx (solo si es binario)
    if !is_lib {
        let main_content = r#"function main(args: String[]) -> int {
    print("Hello from CLS!");
    return 0;
}
"#;
        fs::write(src_dir.join("main.clsx"), main_content).unwrap();
    }

    // .gitignore
    fs::write(dir.join(".gitignore"), "modules/\ndist/\n").unwrap();

    println!("✅ Proyecto '{}' creado", name);
    0
}

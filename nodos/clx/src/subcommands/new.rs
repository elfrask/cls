use std::fs;
use std::path::Path;
use cls_core::config::ModuleManifest;

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

    let src_dir = dir.join("src");
    let mod_dir = dir.join("modules");
    if let Err(e) = fs::create_dir_all(&src_dir) {
        eprintln!("Error al crear '{}': {}", src_dir.display(), e);
        return 1;
    }
    if let Err(e) = fs::create_dir_all(&mod_dir) {
        eprintln!("Error al crear '{}': {}", mod_dir.display(), e);
        return 1;
    }

    // cls.json via ModuleManifest
    let mut manifest = ModuleManifest::default_for(name);
    manifest.entry = if is_lib { String::new() } else { "src/main.clsx".to_string() };
    manifest.project.target = if is_lib { "library".to_string() } else { "executable".to_string() };
    if let Err(e) = manifest.save(&dir.join("cls.json")) {
        eprintln!("Error al escribir '{}': {}", dir.join("cls.json").display(), e);
        return 1;
    }

    // main.clsx
    if !is_lib {
        let main_content = r#"function main(args: String[]) -> int {
    print("Hello from CLS!");
    return 0;
}
"#;
        if let Err(e) = fs::write(src_dir.join("main.clsx"), main_content) {
            eprintln!("Error al escribir '{}': {}", src_dir.join("main.clsx").display(), e);
            return 1;
        }
    }

    // .gitignore
    if let Err(e) = fs::write(dir.join(".gitignore"), "modules/\ndist/\n.cls-types\n") {
        eprintln!("Error al escribir '{}': {}", dir.join(".gitignore").display(), e);
        return 1;
    }

    println!("Proyecto '{}' creado", name);
    0
}

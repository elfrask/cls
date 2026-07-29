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
    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&mod_dir).unwrap();

    // cls.json via ModuleManifest
    let mut manifest = ModuleManifest::default_for(name);
    manifest.entry = if is_lib { String::new() } else { "src/main.clsx".to_string() };
    manifest.project.target = if is_lib { "library".to_string() } else { "executable".to_string() };
    manifest.save(&dir.join("cls.json")).unwrap();

    // main.clsx
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

    println!("Proyecto '{}' creado", name);
    0
}

use std::fs;
use std::io::Write;
use std::path::Path;
use cls_core::config::ModuleManifest;

pub fn execute(args: &[String]) -> i32 {
    // Cargar config
    let config = load_config();

    // Entry: arg > config
    let entry = args.iter().find(|a| !a.starts_with("-")).map(|s| s.as_str())
        .or_else(|| config.as_ref().and_then(|c| {
            let e = &c.entry;
            if !e.is_empty() && Path::new(e).exists() { Some(e.as_str()) } else { None }
        }))
        .unwrap_or_else(|| {
            eprintln!("Uso: clx build <archivo> -o <salida>");
            eprintln!("  (o ejecuta desde un proyecto con cls.json que tenga 'entry')");
            std::process::exit(1);
        }).to_string();

    let out = args.iter()
        .position(|a| a == "-o" || a == "--out")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.as_str())
        .unwrap_or("dist/app.clsapp");

    let source = match fs::read_to_string(&entry) {
        Ok(s) => s,
        Err(e) => { eprintln!("Error al leer '{}': {}", entry, e); return 1; }
    };

    // Verificar que el código compila
    let mut lexer = cls_core::frontend::Lexer::new(&source);
    let tokens = match lexer.tokenize() {
        Ok(t) => t,
        Err(e) => { eprintln!("Error de sintaxis: {}", e); return 1; }
    };
    let mut parser = cls_core::frontend::Parser::new(tokens);
    if let Err(e) = parser.parse() {
        eprintln!("Error de parseo: {}", e);
        return 1;
    }

    // Crear directorio de salida
    if let Some(parent) = Path::new(&out).parent() {
        fs::create_dir_all(parent).ok();
    }

    // Crear .clsapp (zip)
    //
    // PROVISIONAL (deuda 7.3): empaqueta el source .clsx crudo en el zip hasta que
    // exista un formato estable de AST serializado. Ver AGENTS.md → "Sistema de
    // módulos": el plan es empaquetar AST serializado (módulos fuente como .ast) y,
    // más adelante, .clbin (WASM) dentro del .clsapp/.clslib. NO implementar ahora:
    // no hay formato estable y el runtime clxr aún espera source.
    let file = match fs::File::create(&out) {
        Ok(f) => f,
        Err(e) => { eprintln!("Error al crear '{}': {}", out, e); return 1; }
    };
    let mut zip = zip::ZipWriter::new(file);
    let options: zip::write::FileOptions<()> = zip::write::FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    // Manifest con datos de cls.json
    let name = config.as_ref().map(|c| &c.name).map(|s| s.as_str()).unwrap_or("app");
    let version = config.as_ref().map(|c| &c.version).map(|s| s.as_str()).unwrap_or("1.0.0");
    let manifest = serde_json::json!({
        "name": name,
        "version": version,
        "entry": "source.clsx",
        "format": "source"
    });

    if let Err(e) = zip.start_file("manifest.json", options) {
        eprintln!("Error empaquetando '{}': {}", out, e);
        return 1;
    }
    if let Err(e) = zip.write_all(serde_json::to_string_pretty(&manifest).unwrap().as_bytes()) {
        eprintln!("Error empaquetando '{}': {}", out, e);
        return 1;
    }

    // Código fuente
    if let Err(e) = zip.start_file("source.clsx", options) {
        eprintln!("Error empaquetando '{}': {}", out, e);
        return 1;
    }
    if let Err(e) = zip.write_all(source.as_bytes()) {
        eprintln!("Error empaquetando '{}': {}", out, e);
        return 1;
    }

    if let Err(e) = zip.finish() {
        eprintln!("Error empaquetando '{}': {}", out, e);
        return 1;
    }

    println!("Empaquetado: {} ({})", out, name);
    0
}

fn load_config() -> Option<ModuleManifest> {
    let path = std::env::current_dir().ok()?.join("cls.json");
    if path.exists() { ModuleManifest::from_file(&path).ok() } else { None }
}

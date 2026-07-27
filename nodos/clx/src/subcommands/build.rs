use std::fs;
use std::io::Write;
use std::path::Path;

pub fn execute(args: &[String]) -> i32 {
    if args.is_empty() {
        eprintln!("Uso: clx build <archivo> -o <salida>");
        return 1;
    }

    let entry = &args[0];
    let out = args.iter()
        .position(|a| a == "-o" || a == "--out")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.as_str())
        .unwrap_or("dist/app.clsapp");

    let source = match fs::read_to_string(entry) {
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
    if let Some(parent) = Path::new(out).parent() {
        fs::create_dir_all(parent).ok();
    }

    // Crear .clsapp (zip)
    let file = fs::File::create(out).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    let options: zip::write::FileOptions<()> = zip::write::FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    // Manifest
    let manifest = serde_json::json!({
        "name": "app",
        "version": "1.0.0",
        "entry": "source.clsx",
        "format": "source"
    });
    zip.start_file("manifest.json", options).unwrap();
    zip.write_all(serde_json::to_string_pretty(&manifest).unwrap().as_bytes()).unwrap();

    // Código fuente
    zip.start_file("source.clsx", options).unwrap();
    zip.write_all(source.as_bytes()).unwrap();

    zip.finish().unwrap();

    println!("✅ Empaquetado: {}", out);
    0
}

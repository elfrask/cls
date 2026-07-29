use std::env;
use std::fs;
use std::io::Read;
use std::process;
use std::sync::Arc;

use cls_runtime::{VfsResolver, LocalFs, ZipFs};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("clxr 2.0 — CLS Runtime Executor");
        eprintln!("Uso: clxr <archivo> [args...]");
        eprintln!("  .clsx  → ejecucion directa");
        eprintln!("  .clsapp → extrae y ejecuta (formato zip)");
        process::exit(1);
    }

    let path = &args[1];
    let app_args: Vec<String> = args[2..].to_vec();

    // Construir VFS
    let (source, _vfs) = load_source_and_vfs(path);

    let mut lexer = cls_core::frontend::Lexer::new(&source);
    let tokens = match lexer.tokenize() {
        Ok(t) => t,
        Err(e) => { show_error(&source, &e.to_string(), path); process::exit(1); }
    };

    let mut parser = cls_core::frontend::Parser::new(tokens);
    let module = match parser.parse() {
        Ok(m) => m,
        Err(e) => { show_error(&source, &e.to_string(), path); process::exit(1); }
    };

    let resolver = cls_runtime::ModuleResolver::new().with_core_stdlib();
    let mut interpreter = cls_runtime::Interpreter::new(
        cls_runtime::Intrinsics::desktop_defaults(app_args),
        resolver,
    );
    interpreter.set_source_file(path.to_string());

    if let Err(e) = interpreter.execute(&module) {
        eprintln!("{}", e);
        process::exit(1);
    }
    match interpreter.call_main() {
        Ok(code) => process::exit(code),
        Err(e) => { eprintln!("{}", e); process::exit(1); }
    }
}

fn load_source_and_vfs(path: &str) -> (String, Arc<VfsResolver>) {
    let mut vfs = make_base_vfs();

    if path.ends_with(".clsapp") {
        match open_clsapp_vfs(path, &mut vfs) {
            Ok(source) => return (source, Arc::new(vfs)),
            Err(e) => { eprintln!("Error al cargar '{}': {}", path, e); process::exit(1); }
        }
    }

    match fs::read_to_string(path) {
        Ok(source) => (source, Arc::new(vfs)),
        Err(e) => { eprintln!("Error al leer '{}': {}", path, e); process::exit(1); }
    }
}

fn make_base_vfs() -> VfsResolver {
    let mut vfs = VfsResolver::new();
    let cwd = env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    vfs.register("app", Arc::new(LocalFs::new("app", &cwd, false)));

    let home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .map(std::path::PathBuf::from)
        .ok();
    if let Some(ref h) = home {
        vfs.register("user", Arc::new(LocalFs::new("user", h, false)));
    }

    let tmp = env::temp_dir();
    vfs.register("tmp", Arc::new(LocalFs::new("tmp", &tmp, false)));

    vfs
}

fn open_clsapp_vfs(path: &str, vfs: &mut VfsResolver) -> Result<String, String> {
    let path_buf = std::path::PathBuf::from(path);
    let vfs_path = path_buf.to_owned();

    // Registrar res:// desde el .clsapp
    let zipfs = ZipFs::open("res", &vfs_path)
        .map_err(|e| format!("Zip invalido: {}", e))?;
    vfs.register("res", Arc::new(zipfs));

    // Leer source desde el zip
    let file = fs::File::open(path).map_err(|e| format!("No se puede abrir: {}", e))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("Zip invalido: {}", e))?;

    let entry = if let Ok(mut mf) = archive.by_name("manifest.json") {
        let mut content = String::new();
        mf.read_to_string(&mut content).ok();
        let json: serde_json::Value = serde_json::from_str(&content).unwrap_or_default();
        json["entry"].as_str().unwrap_or("source.clsx").to_string()
    } else {
        "source.clsx".to_string()
    };

    let mut source = String::new();
    archive
        .by_name(&entry)
        .map_err(|e| format!("Entry '{}' no encontrado: {}", entry, e))?
        .read_to_string(&mut source)
        .map_err(|e| format!("Error leyendo '{}': {}", entry, e))?;

    Ok(source)
}

fn show_error(source: &str, error_msg: &str, path: &str) {
    eprintln!("Error en '{}':", path);
    eprintln!("  {}", error_msg);
    if source.is_empty() { return; }
    let line_col = error_msg.split("linea").nth(1).and_then(|s| {
        let parts: Vec<&str> = s.splitn(2, ',').collect();
        let line = parts.first()?.trim().parse::<usize>().ok()?;
        let col = parts.get(1).and_then(|c|
            c.split("columna").nth(1).and_then(|c2|
                c2.trim().trim_matches(|p| p == ')' || p == '(').parse::<usize>().ok()
            )
        ).unwrap_or(1);
        Some((line, col))
    });
    if let Some((line, col)) = line_col {
        if let Some(src_line) = source.lines().nth(line.saturating_sub(1)) {
            eprintln!("");
            eprintln!("  {} | {}", line, src_line);
            let pad = " ".repeat(line.to_string().len());
            if col > 1 {
                eprintln!("  {} | {}{}", pad, " ".repeat(col.saturating_sub(1)), "^");
            } else {
                eprintln!("  {} | ^", pad);
            }
        }
    }
}

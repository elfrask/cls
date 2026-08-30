//! `clxr` - CLS Runtime Executor.
//!
//! Ejecuta un archivo `.clsx` o `.clsapp` usando el motor JIT de
//! `cls-jit` (CLS -> WASM -> wasmtime). Mismo path que `clx run`,
//! pero como binario independiente con un solo argumento posicional.
//!
//! Migracion dev-2 (Fase 7): se elimino el path `--ast-walker`
//! (tree-walker deprecado). El runtime ahora es 100% JIT.
//! El walker completo (`cls-runtime/src/walker/`) se borro en esta
//! misma fase; los simbolos `ModuleResolver`, `Interpreter` y
//! `Intrinsics` del walker ya no existen.

use std::env;
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::process;
use std::sync::Arc;

use cls_jit::{JitContext, RuntimeKind};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("clxr 2.0 - CLS Runtime Executor");
        eprintln!("Uso: clxr <archivo> [args...]");
        eprintln!("  .clsx           -> ejecucion JIT");
        eprintln!("  .clsapp         -> extrae y ejecuta via JIT");
        process::exit(1);
    }

    // `args[0]` es el path del binario: lo saltamos.
    // Flags `--*` se ignoran silenciosamente (compatibilidad hacia atras
    // con el flag `--ast-walker` eliminado en Fase 7).
    let positional: Vec<String> = args.iter()
        .skip(1)
        .filter(|a| !a.starts_with("--"))
        .cloned()
        .collect();

    if positional.is_empty() {
        eprintln!("Error: se requiere un archivo de entrada");
        process::exit(1);
    }

    let path = positional[0].clone();
    let app_args: Vec<String> = positional[1..].to_vec();

    // Si es .clsapp, extraer a temporal y usar el entry como .clsx.
    let resolved_path = match resolve_entry(&path) {
        Ok(p) => p,
        Err(e) => { eprintln!("Error: {}", e); process::exit(1); }
    };

    run_jit(&resolved_path, &app_args);
}

/// Resuelve el entry: si es .clsapp, extrae a un dir temporal y devuelve
/// el path al .clsx interno. Si es .clsx, lo devuelve tal cual.
fn resolve_entry(path: &str) -> Result<PathBuf, String> {
    if !path.ends_with(".clsapp") {
        return Ok(PathBuf::from(path));
    }
    // .clsapp: extraer
    let file = fs::File::open(path).map_err(|e| format!("No se puede abrir '{}': {}", path, e))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("Zip invalido: {}", e))?;

    // Determinar el entry del manifest.json o default a source.clsx
    let entry_name = if let Ok(mut mf) = archive.by_name("manifest.json") {
        let mut content = String::new();
        mf.read_to_string(&mut content).ok();
        let json: serde_json::Value = serde_json::from_str(&content).unwrap_or_default();
        json["entry"].as_str().unwrap_or("source.clsx").to_string()
    } else {
        "source.clsx".to_string()
    };

    // Extraer a un dir temporal del sistema (el SO lo limpia)
    let temp_dir = std::env::temp_dir().join(format!("clxr-{}", std::process::id()));
    fs::create_dir_all(&temp_dir).map_err(|e| format!("No se puede crear temp dir: {}", e))?;
    let entry_path = temp_dir.join(&entry_name);

    let mut entry_file = archive
        .by_name(&entry_name)
        .map_err(|e| format!("Entry '{}' no encontrado: {}", entry_name, e))?;
    let mut content = String::new();
    entry_file
        .read_to_string(&mut content)
        .map_err(|e| format!("Error leyendo '{}': {}", entry_name, e))?;
    fs::write(&entry_path, content).map_err(|e| format!("No se puede escribir entry: {}", e))?;

    Ok(entry_path)
}

/// Ejecuta el archivo con el JIT.
fn run_jit(entry: &std::path::Path, app_args: &[String]) -> ! {
    let ctx = JitContext {
        native_backend: Arc::new(cls_runtime::DynamicBackend),
        module_index: None,
        host_intrinsics: &[],
        host_call_handler: None,
        module_source_resolver: None,
        output: None,
    };
    let entry_str = entry.to_string_lossy();
    let exit_code = cls_jit::run_jit_with_opts(
        &entry_str, app_args, None, &ctx,
        RuntimeKind::Wasmtime, true,
    );
    process::exit(exit_code);
}

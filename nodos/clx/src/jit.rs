//! Motor JIT: CLS → WASM → wasmtime (Cranelift).
//!
//! `clx run --jit <archivo> [-- args...]` compila el archivo con el backend WASM
//! de cls-core (usando el type map del TypeChecker) y lo ejecuta en wasmtime.

use cls_core::config::types::TypesConfig;
use cls_core::error::{ClsError, Span};
use cls_core::middleware::TypeChecker;
use cls_runtime::error_report::{format_error, ErrorFormat, ErrorReport};
use wasmtime::{Caller, Engine, Linker, Memory, Module, Store, Val};
use std::time::Instant;

const HOST: &str = "env";

/// Muestra un error CLS con el formato estricto (trace numerado + línea + caret),
/// igual que el walker (error_report). El nodo decide el formato (Console).
fn show_cls_error(error: &ClsError, entry: &str, source: Option<&str>) {
    // Reconstruir el error por tipo (ClsError no es Clone por IoError).
    let reconstructed: ClsError = match error {
        ClsError::SyntaxErrorAt(m, s) => ClsError::syntax_at(m, s),
        ClsError::CompileErrorAt(m, s) => ClsError::compile_at(m, s),
        ClsError::CompileError(m) => ClsError::CompileError(m.clone()),
        ClsError::TypeError(m) => ClsError::TypeError(m.clone()),
        ClsError::RuntimeError(m) => ClsError::RuntimeError(m.clone()),
        ClsError::SyntaxError(m) => ClsError::SyntaxError(m.clone()),
        ClsError::IoError(e) => ClsError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())),
        ClsError::ConfigError(m) => ClsError::ConfigError(m.clone()),
    };
    let span = match &reconstructed {
        ClsError::SyntaxErrorAt(_, s) | ClsError::CompileErrorAt(_, s) => Some(s.clone()),
        _ => ClsError::extract_line_col(&reconstructed.to_string())
            .map(|(l, c)| Span::new(l as u32, c as u32, l as u32, c as u32)),
    };
    let report = ErrorReport {
        error: reconstructed,
        span,
        stack: vec![],
        import_trace: vec![],
        source_file: entry.to_string(),
        source: source.map(|s| s.to_string()),
    };
    eprintln!("{}", format_error(&report, &ErrorFormat::Console));
}

/// `CLS_JIT_TIMING=1` → imprime el tiempo de cada fase del pipeline a stderr.
fn jit_timing() -> bool {
    std::env::var("CLS_JIT_TIMING")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

fn tick(timing: bool, label: &str, start: Instant) -> Instant {
    if timing {
        eprintln!(
            "[JIT-TIMING] {:<26} {:>12.2} ms",
            label,
            start.elapsed().as_secs_f64() * 1000.0
        );
    }
    Instant::now()
}

/// Estado del host: separador de argumentos en `print` y archivo fuente (para errores).
struct HostState {
    first_in_line: bool,
    source_file: String,
}

impl Default for HostState {
    fn default() -> Self {
        Self {
            first_in_line: true,
            source_file: String::new(),
        }
    }
}

/// Directorio del caché de compilación: `~/.cache/cls/` (HOME o USERPROFILE).
fn cache_dir() -> std::path::PathBuf {
    let base = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| std::env::temp_dir().to_string_lossy().into_owned());
    std::path::PathBuf::from(base).join(".cache").join("cls")
}

/// Clave del caché: hash del fuente + versión del compilador + target.
fn cache_key(source: &str, target_str: Option<&str>) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    source.hash(&mut h);
    cls_core::VERSION.hash(&mut h);
    target_str.unwrap_or("").hash(&mut h);
    h.finish()
}

pub fn run_jit(entry: &str, app_args: &[String], target_str: Option<&str>) -> i32 {
    let timing = jit_timing();
    let mut t = Instant::now();

    let source = match std::fs::read_to_string(entry) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error al leer '{}': {}", entry, e);
            return 1;
        }
    };
    t = tick(timing, "lectura", t);

    // Caché CLS→WASM en disco: si el fuente no cambió (misma versión + target),
    // saltamos lexer/parser/typeck/emisión y cargamos el .wasm directamente.
    let key = cache_key(&source, target_str);
    let cache_path = cache_dir().join(format!("{:016x}.wasm", key));
    if let Ok(cached) = std::fs::read(&cache_path) {
        if timing {
            eprintln!("[JIT-TIMING] caché CLS→WASM: HIT ({} bytes)", cached.len());
        }
        return run_wasm(&cached, entry, app_args, timing, t, None);
    }
    if timing {
        eprintln!("[JIT-TIMING] caché CLS→WASM: miss");
    }

    // Parseo
    let mut lexer = cls_core::frontend::Lexer::new(&source);
    let tokens = match lexer.tokenize() {
        Ok(tk) => tk,
        Err(e) => {
            cls_runtime::show_syntax_error(e, &source, entry);
            return 1;
        }
    };
    t = tick(timing, "lexer", t);

    let mut parser = cls_core::frontend::Parser::new(tokens);
    let module = match parser.parse() {
        Ok(m) => m,
        Err(e) => {
            cls_runtime::show_syntax_error(e, &source, entry);
            return 1;
        }
    };
    t = tick(timing, "parser", t);

    // Type checker: llena el mapa Span → Type (requerido por el backend).
    let types_config = TypesConfig {
        check: true,
        strict: true,
        no_implicit_any: true,
        null_safety: true,
    };
    let mut checker = TypeChecker::new(types_config);
    if let Err(e) = checker.check(&module) {
        show_cls_error(&e, entry, Some(&source));
        return 1;
    }
    t = tick(timing, "typecheck", t);

    // Emitir WASM (target para la directiva `when`).
    let target = match target_str {
        Some(tt) => cls_core::frontend::ast::Target::parse(tt),
        None => cls_core::frontend::ast::Target::host(),
    };
    let type_map = checker.type_map().clone();
    t = tick(timing, "type_map.clone", t);

    let backend = cls_core::backend::wasm::WasmBackend::with_target(type_map, target);
    let wasm_bytes = match backend.emit(&module) {
        Ok(b) => b,
        Err(e) => {
            show_cls_error(&e, entry, Some(&source));
            return 1;
        }
    };
    if std::env::var("CLS_DUMP_WAT").is_ok() {
        if let Ok(wat) = wasmprinter::print_bytes(&wasm_bytes) {
            eprintln!("--- WAT DEBUG ---\n{}", wat);
        }
    }
    t = tick(timing, "emit WASM", t);
    if timing {
        eprintln!("[JIT-TIMING] WASM size: {} bytes", wasm_bytes.len());
    }

    // Guardar en caché SOLO si run_wasm valida y ejecuta bien (evita cachear WASM
    // inválido si el emisor tiene un bug). run_wasm recibe el path para escribirlo
    // tras una validación exitosa de Module::new.
    run_wasm(&wasm_bytes, entry, app_args, timing, t, Some(cache_path))
}

/// Config de caché de wasmtime (WASM→nativo): crea el TOML por defecto si falta.
fn wasmtime_cache_config() -> Option<wasmtime::Config> {
    let dir = cache_dir();
    let config_path = dir.join("wasmtime-cache.toml");
    if !config_path.exists() {
        let cache_dir = dir.join("wasmtime").to_string_lossy().replace('\\', "/");
        let toml = format!("[cache]\nenabled = true\ndirectory = \"{}\"\n", cache_dir);
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(&config_path, toml);
    }
    let mut config = wasmtime::Config::new();
    if config.cache_config_load(&config_path).is_ok() {
        Some(config)
    } else {
        None
    }
}

fn run_wasm(
    wasm_bytes: &[u8],
    entry: &str,
    app_args: &[String],
    timing: bool,
    mut t: Instant,
    cache_path: Option<std::path::PathBuf>,
) -> i32 {
    let engine = match wasmtime_cache_config() {
        Some(config) => match Engine::new(&config) {
            Ok(e) => e,
            Err(e) => {
                eprintln!("[JIT] Error creando engine wasmtime: {}", e);
                return 1;
            }
        },
        None => Engine::default(),
    };
    t = tick(timing, "Engine::default", t);

    let module = match Module::new(&engine, wasm_bytes) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("[JIT] Módulo WASM inválido para '{}':\n{:?}", entry, e);
            if let Ok(wat) = wasmprinter::print_bytes(wasm_bytes) {
                eprintln!("--- WAT ---\n{}", wat);
            }
            // No dejar WASM inválido en el caché.
            if let Some(p) = &cache_path {
                let _ = std::fs::remove_file(p);
            }
            return 1;
        }
    };
    t = tick(timing, "Module::new (Cranelift)", t);

    // El WASM es válido: persistirlo en el caché CLS→WASM (fallo silencioso).
    if let Some(p) = &cache_path {
        let _ = std::fs::create_dir_all(cache_dir())
            .and_then(|_| std::fs::write(p, wasm_bytes));
    }

    let mut store = Store::new(
        &engine,
        HostState {
            first_in_line: true,
            source_file: entry.to_string(),
        },
    );
    let mut linker = Linker::new(&engine);
    t = tick(timing, "Store+Linker", t);

    if let Err(e) = register_host_functions(&mut linker) {
        eprintln!("[JIT] Error registrando funciones host: {}", e);
        return 1;
    }

    // Extensiones: hosts genéricos `env.<sym>__<sig>@<lib>` → DynamicBackend.
    if let Err(e) = register_native_hosts(&mut linker, &module) {
        eprintln!("[JIT] Error registrando hosts de extensiones: {}", e);
        return 1;
    }
    t = tick(timing, "register hosts", t);

    let instance = match linker.instantiate(&mut store, &module) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("[JIT] Error de instanciación para '{}': {}", entry, e);
            return 1;
        }
    };
    t = tick(timing, "instantiate", t);

    // Escribir los args de la app en la memoria y llamar main(ptr).
    let alloc = match instance.get_typed_func::<i64, i64>(&mut store, "alloc") {
        Ok(f) => f,
        Err(e) => {
            eprintln!("[JIT] Export 'alloc' no disponible: {}", e);
            return 1;
        }
    };
    let memory = match instance.get_memory(&mut store, "memory") {
        Some(m) => m,
        None => {
            eprintln!("[JIT] Export 'memory' no disponible");
            return 1;
        }
    };

    let args_ptr = match write_args(&mut store, &memory, &alloc, app_args) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("[JIT] Error escribiendo args: {}", e);
            return 1;
        }
    };
    t = tick(timing, "write_args", t);

    let main = match instance.get_typed_func::<i64, i64>(&mut store, "main") {
        Ok(f) => f,
        Err(e) => {
            eprintln!("[JIT] Export 'main' no disponible: {}", e);
            return 1;
        }
    };

    let result = match main.call(&mut store, args_ptr) {
        Ok(code) => code as i32,
        Err(e) => {
            show_cls_error(
                &ClsError::RuntimeError(format!("Trap WASM: {}", e)),
                entry,
                None,
            );
            1
        }
    };
    tick(timing, "ejecución main", t);
    result
}

/// Escribe los args como Array<String> en memoria y devuelve el ptr.
fn write_args(
    store: &mut Store<HostState>,
    memory: &Memory,
    alloc: &wasmtime::TypedFunc<i64, i64>,
    app_args: &[String],
) -> Result<i64, String> {
    let n = app_args.len() as i64;
    // Layout de array actual: [cap:i64][len:i64][elems...] (header 16 bytes).
    let array_ptr = alloc.call(&mut *store, n * 8 + 16).map_err(|e| e.to_string())?;
    memory
        .write(&mut *store, array_ptr as usize, &n.to_le_bytes())
        .map_err(|e| e.to_string())?;
    memory
        .write(&mut *store, (array_ptr as usize) + 8, &n.to_le_bytes())
        .map_err(|e| e.to_string())?;
    for (i, arg) in app_args.iter().enumerate() {
        let sptr = alloc
            .call(&mut *store, arg.len() as i64)
            .map_err(|e| e.to_string())?;
        memory
            .write(&mut *store, sptr as usize, arg.as_bytes())
            .map_err(|e| e.to_string())?;
        let packed = (sptr << 32) | (arg.len() as i64);
        memory
            .write(&mut *store, (array_ptr as usize) + 16 + i * 8, &packed.to_le_bytes())
            .map_err(|e| e.to_string())?;
    }
    Ok(array_ptr)
}

/// Resuelve la memoria del módulo desde el caller del host function.
fn caller_memory<'a>(caller: &'a mut Caller<'_, HostState>) -> Option<Memory> {
    caller.get_export("memory").and_then(|e| e.into_memory())
}

/// Llama al allocator exportado del módulo desde un host function.
fn caller_alloc(caller: &mut Caller<'_, HostState>, n: i64) -> Option<i64> {
    let func = caller.get_export("alloc").and_then(|e| e.into_func())?;
    let mut results = [Val::I64(0)];
    func.call(caller, &[Val::I64(n)], &mut results).ok()?;
    results[0].i64()
}

/// Escribe un string en la memoria del módulo (via alloc) y lo empaqueta.
fn caller_write_str(caller: &mut Caller<'_, HostState>, s: &str) -> Option<i64> {
    let len = s.len() as i64;
    let ptr = caller_alloc(caller, len)?;
    let memory = caller_memory(caller)?;
    let data = memory.data_mut(caller);
    let start = ptr as usize;
    if start + s.len() <= data.len() {
        data[start..start + s.len()].copy_from_slice(s.as_bytes());
    }
    Some((ptr << 32) | len)
}

/// Lee un string empaquetado (ptr<<32|len) de la memoria.
fn caller_read_str(caller: &mut Caller<'_, HostState>, packed: i64) -> String {
    let ptr = (packed >> 32) as usize;
    let len = (packed & 0xffff_ffff) as usize;
    if let Some(memory) = caller.get_export("memory").and_then(|e| e.into_memory()) {
        let data = memory.data(&mut *caller);
        if ptr + len <= data.len() {
            return String::from_utf8_lossy(&data[ptr..ptr + len]).into_owned();
        }
    }
    String::new()
}

fn print_arg(caller: &mut Caller<'_, HostState>, value: &str) {
    let state = caller.data_mut();
    if !state.first_in_line {
        print!(" ");
    }
    print!("{}", value);
    state.first_in_line = false;
}

fn format_float(v: f64) -> String {
    format!("{}", v)
}

fn register_host_functions(linker: &mut Linker<HostState>) -> Result<(), String> {
    linker
        .func_wrap(HOST, "print_int", |mut caller: Caller<'_, HostState>, v: i64| {
            print_arg(&mut caller, &v.to_string());
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "print_float", |mut caller: Caller<'_, HostState>, v: f64| {
            print_arg(&mut caller, &format_float(v));
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "print_bool", |mut caller: Caller<'_, HostState>, v: i32| {
            print_arg(&mut caller, if v != 0 { "true" } else { "false" });
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "print_char", |mut caller: Caller<'_, HostState>, v: i32| {
            let c = char::from_u32(v as u32).unwrap_or('?');
            print_arg(&mut caller, &c.to_string());
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "print_str", |mut caller: Caller<'_, HostState>, v: i64| {
            let s = caller_read_str(&mut caller, v);
            print_arg(&mut caller, &s);
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "print_end", |mut caller: Caller<'_, HostState>| {
            println!();
            caller.data_mut().first_in_line = true;
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "now", |_: Caller<'_, HostState>| -> i64 {
            use std::time::{SystemTime, UNIX_EPOCH};
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0)
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "exit", |_: Caller<'_, HostState>, code: i64| -> () {
            std::process::exit(code as i32);
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "sleep", |_: Caller<'_, HostState>, ms: i64| {
            if ms > 0 {
                std::thread::sleep(std::time::Duration::from_millis(ms as u64));
            }
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "trap", |mut caller: Caller<'_, HostState>, msg: i64, span: i64| -> () {
            let s = caller_read_str(&mut caller, msg);
            let file = caller.data().source_file.clone();
            let line = ((span >> 32) & 0xffff_ffff) as u32;
            let col = (span & 0xffff_ffff) as u32;
            let err = ClsError::RuntimeError(s);
            let span_s = if line > 0 {
                Some(Span::new(line, col, line, col))
            } else {
                None
            };
            let report = ErrorReport {
                error: err,
                span: span_s,
                stack: vec![],
                import_trace: vec![],
                source_file: file,
                source: None,
            };
            eprintln!("{}", format_error(&report, &ErrorFormat::Console));
            std::process::exit(1);
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "parse_int", |mut caller: Caller<'_, HostState>, v: i64| -> i64 {
            let s = caller_read_str(&mut caller, v);
            s.trim().parse::<i64>().unwrap_or(0)
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "parse_float", |mut caller: Caller<'_, HostState>, v: i64| -> f64 {
            let s = caller_read_str(&mut caller, v);
            s.trim().parse::<f64>().unwrap_or(0.0)
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "parse_bool", |mut caller: Caller<'_, HostState>, v: i64| -> i32 {
            let s = caller_read_str(&mut caller, v);
            let t = s.trim();
            if t == "true" || t == "1" {
                1
            } else {
                0
            }
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(
            HOST,
            "str_concat",
            |mut caller: Caller<'_, HostState>, a: i64, b: i64| -> i64 {
                let sa = caller_read_str(&mut caller, a);
                let sb = caller_read_str(&mut caller, b);
                let out = format!("{}{}", sa, sb);
                caller_write_str(&mut caller, &out).unwrap_or(0)
            },
        )
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "str_int", |mut caller: Caller<'_, HostState>, v: i64| -> i64 {
            caller_write_str(&mut caller, &v.to_string()).unwrap_or(0)
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "str_float", |mut caller: Caller<'_, HostState>, v: f64| -> i64 {
            caller_write_str(&mut caller, &format_float(v)).unwrap_or(0)
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "str_bool", |mut caller: Caller<'_, HostState>, v: i32| -> i64 {
            caller_write_str(&mut caller, if v != 0 { "true" } else { "false" })
                .unwrap_or(0)
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "str_char", |mut caller: Caller<'_, HostState>, v: i32| -> i64 {
            let c = char::from_u32(v as u32).unwrap_or('?');
            caller_write_str(&mut caller, &c.to_string()).unwrap_or(0)
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "pow_num", |_: Caller<'_, HostState>, a: i64, b: i64| -> i64 {
            if b == 0 {
                1
            } else {
                (a as f64).powi(b as i32) as i64
            }
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "fmod", |_: Caller<'_, HostState>, a: f64, b: f64| -> f64 {
            a % b
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "input", |mut caller: Caller<'_, HostState>| -> i64 {
            let mut line = String::new();
            let _ = std::io::stdin().read_line(&mut line);
            let line = line.trim_end_matches(['\r', '\n']);
            caller_write_str(&mut caller, line).unwrap_or(0)
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "str_upper", |mut caller: Caller<'_, HostState>, v: i64| -> i64 {
            let s = caller_read_str(&mut caller, v);
            caller_write_str(&mut caller, &s.to_uppercase()).unwrap_or(0)
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "str_lower", |mut caller: Caller<'_, HostState>, v: i64| -> i64 {
            let s = caller_read_str(&mut caller, v);
            caller_write_str(&mut caller, &s.to_lowercase()).unwrap_or(0)
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "str_trim", |mut caller: Caller<'_, HostState>, v: i64| -> i64 {
            let s = caller_read_str(&mut caller, v);
            caller_write_str(&mut caller, s.trim()).unwrap_or(0)
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "str_contains", |mut caller: Caller<'_, HostState>, a: i64, b: i64| -> i32 {
            let sa = caller_read_str(&mut caller, a);
            let sb = caller_read_str(&mut caller, b);
            if sa.contains(&sb) { 1 } else { 0 }
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "str_starts_with", |mut caller: Caller<'_, HostState>, a: i64, b: i64| -> i32 {
            let sa = caller_read_str(&mut caller, a);
            let sb = caller_read_str(&mut caller, b);
            if sa.starts_with(&sb) { 1 } else { 0 }
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "str_ends_with", |mut caller: Caller<'_, HostState>, a: i64, b: i64| -> i32 {
            let sa = caller_read_str(&mut caller, a);
            let sb = caller_read_str(&mut caller, b);
            if sa.ends_with(&sb) { 1 } else { 0 }
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "str_is_empty", |mut caller: Caller<'_, HostState>, v: i64| -> i32 {
            let s = caller_read_str(&mut caller, v);
            if s.is_empty() { 1 } else { 0 }
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "str_repr", |mut caller: Caller<'_, HostState>, v: i64| -> i64 {
            let s = caller_read_str(&mut caller, v);
            let escaped = s
                .replace('\\', "\\\\")
                .replace('"', "\\\"")
                .replace('\n', "\\n")
                .replace('\t', "\\t");
            caller_write_str(&mut caller, &format!("\"{}\"", escaped)).unwrap_or(0)
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "str_length", |mut caller: Caller<'_, HostState>, v: i64| -> i64 {
            let s = caller_read_str(&mut caller, v);
            s.len() as i64
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "int_abs", |_: Caller<'_, HostState>, v: i64| -> i64 {
            v.abs()
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "float_abs", |_: Caller<'_, HostState>, v: f64| -> f64 {
            v.abs()
        })
        .map_err(|e| e.to_string())?;

    // ── Métodos de Array (layout [cap:i64][len:i64][elem...]) ───────────────

    linker
        .func_wrap(HOST, "arr_push", |mut caller: Caller<'_, HostState>, ptr: i64, val: i64, es: i64| -> i64 {
            let p = ptr as usize;
            let len = arr_len(&mut caller, p);
            let cap = arr_cap(&mut caller, p);
            let new_p = if len + 1 > cap {
                arr_realloc(&mut caller, p, ((cap * 2 + 1).max(len + 1)) as usize, es as usize)
            } else {
                p
            };
            arr_set(&mut caller, new_p, len as usize, es as usize, val);
            arr_write_i64(&mut caller, new_p + 8, len + 1);
            new_p as i64
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "arr_pop", |mut caller: Caller<'_, HostState>, ptr: i64, es: i64| -> i64 {
            let p = ptr as usize;
            let len = arr_len(&mut caller, p);
            if len <= 0 {
                return p as i64;
            }
            arr_write_i64(&mut caller, p + 8, len - 1);
            p as i64
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "arr_shift", |mut caller: Caller<'_, HostState>, ptr: i64, es: i64| -> i64 {
            let p = ptr as usize;
            let es = es as usize;
            let len = arr_len(&mut caller, p);
            if len <= 0 {
                return p as i64;
            }
            for i in 0..(len - 1) as usize {
                let e = arr_elem(&mut caller, p, i + 1, es);
                arr_set(&mut caller, p, i, es, e);
            }
            arr_write_i64(&mut caller, p + 8, len - 1);
            p as i64
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "arr_unshift", |mut caller: Caller<'_, HostState>, ptr: i64, val: i64, es: i64| -> i64 {
            let p = ptr as usize;
            let es = es as usize;
            let len = arr_len(&mut caller, p);
            let cap = arr_cap(&mut caller, p);
            let new_p = if len + 1 > cap {
                arr_realloc(&mut caller, p, ((cap * 2 + 1).max(len + 1)) as usize, es)
            } else {
                p
            };
            for i in (0..len as usize).rev() {
                let e = arr_elem(&mut caller, new_p, i, es);
                arr_set(&mut caller, new_p, i + 1, es, e);
            }
            arr_set(&mut caller, new_p, 0, es, val);
            arr_write_i64(&mut caller, new_p + 8, len + 1);
            new_p as i64
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "arr_reverse", |mut caller: Caller<'_, HostState>, ptr: i64, es: i64| -> i64 {
            let p = ptr as usize;
            let es = es as usize;
            let len = arr_len(&mut caller, p);
            for i in 0..(len as usize / 2) {
                let a = arr_elem(&mut caller, p, i, es);
                let b = arr_elem(&mut caller, p, (len as usize) - 1 - i, es);
                arr_set(&mut caller, p, i, es, b);
                arr_set(&mut caller, p, (len as usize) - 1 - i, es, a);
            }
            p as i64
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "arr_to_string", |mut caller: Caller<'_, HostState>, ptr: i64, es: i64, kind: i64| -> i64 {
            let s = arr_to_string(&mut caller, ptr, es, kind);
            caller_write_str(&mut caller, &s).unwrap_or(0)
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "arr_index_of", |mut caller: Caller<'_, HostState>, ptr: i64, needle: i64, es: i64| -> i64 {
            let p = ptr as usize;
            let len = arr_len(&mut caller, p);
            for i in 0..len as usize {
                if arr_elem(&mut caller, p, i, es as usize) == needle {
                    return i as i64;
                }
            }
            -1
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "arr_includes", |mut caller: Caller<'_, HostState>, ptr: i64, needle: i64, es: i64| -> i32 {
            let p = ptr as usize;
            let len = arr_len(&mut caller, p);
            for i in 0..len as usize {
                if arr_elem(&mut caller, p, i, es as usize) == needle {
                    return 1;
                }
            }
            0
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(
            HOST,
            "arr_join",
            |mut caller: Caller<'_, HostState>, ptr: i64, sep: i64, es: i64, kind: i64| -> i64 {
                let p = ptr as usize;
                let es = es as usize;
                let len = arr_len(&mut caller, p);
                let separator = caller_read_str(&mut caller, sep);
                let mut out = String::new();
                for i in 0..len as usize {
                    if i > 0 {
                        out.push_str(&separator);
                    }
                    let e = arr_elem(&mut caller, p, i, es);
                    match kind {
                        1 => out.push_str(&caller_read_str(&mut caller, e)),
                        2 => out.push_str(&format_float(f64::from_bits(e as u64))),
                        3 => out.push_str(if e != 0 { "true" } else { "false" }),
                        4 => out.push(char::from_u32(e as u32).unwrap_or('?')),
                        _ => out.push_str(&e.to_string()),
                    }
                }
                caller_write_str(&mut caller, &out).unwrap_or(0)
            },
        )
        .map_err(|e| e.to_string())?;

    register_stdlib_hosts(linker)?;
    register_record_hosts(linker)?;

    Ok(())
}

/// Registra hosts para las extensiones (`env.<sym>__<sig>@<lib>`) que delegan en
/// `DynamicBackend` (libloading). `sig` = ret+params: i=int, f=float, b=bool, c=char, s=string, v=void.
fn register_native_hosts(linker: &mut Linker<HostState>, module: &wasmtime::Module) -> Result<usize, String> {
    use cls_runtime::ffi::{NativeBackend, NativeType};
    use cls_runtime::Value;
    let backend = &crate::native::DynamicBackend;
    let mut count = 0;
    let imports: Vec<(String, String)> = module
        .imports()
        .map(|it| (it.module().to_string(), it.name().to_string()))
        .collect();
    for (m, n) in imports {
        if m != "env" {
            continue;
        }
        let (rest, lib) = match n.split_once('@') {
            Some(x) => x,
            None => continue,
        };
        let (sym, sig) = match rest.split_once("__") {
            Some(x) => x,
            None => continue,
        };
        let sym = sym.to_string();
        let lib = lib.to_string();
        let sig = sig.to_string();
        let name = n.clone();
        let native_type = |c: char| match c {
            'f' => NativeType::Float,
            'b' => NativeType::Bool,
            'c' => NativeType::CInt,
            's' => NativeType::CString,
            _ => NativeType::Int,
        };
        let ret_to_i64 = |v: Result<Value, cls_core::error::ClsError>| -> i64 {
            match v {
                Ok(Value::Int(n)) => n,
                Ok(Value::Float(f)) => f as i64,
                Ok(Value::Bool(b)) => {
                    if b {
                        1
                    } else {
                        0
                    }
                }
                Ok(Value::Char(ch)) => ch as i64,
                Ok(_) => 0,
                Err(_) => 0,
            }
        };
        let ret_to_f64 = |v: Result<Value, cls_core::error::ClsError>| -> f64 {
            match v {
                Ok(Value::Float(f)) => f,
                Ok(Value::Int(n)) => n as f64,
                Ok(_) => 0.0,
                Err(_) => 0.0,
            }
        };
        let params: Vec<char> = sig.chars().skip(1).collect();
        let rcode = sig.chars().next().unwrap_or('i');
        let lib2 = lib.clone();
        let sym2 = sym.clone();
        match params.as_slice() {
            [] => {
                let ret = rcode;
                let lib3 = lib2.clone();
                let sym3 = sym2.clone();
                linker
                    .func_wrap(HOST, &name, move |_: Caller<'_, HostState>| -> i64 {
                        let _ = &lib3;
                        let _ = &sym3;
                        let r = backend.call_function(&lib3, &sym3, &[], &[], native_type(ret));
                        ret_to_i64(r)
                    })
                    .map_err(|e| e.to_string())?;
            }
            [p0] if *p0 == 'f' => {
                let ret = rcode;
                let lib3 = lib2.clone();
                let sym3 = sym2.clone();
                linker
                    .func_wrap(HOST, &name, move |_: Caller<'_, HostState>, a: f64| -> f64 {
                        let r = backend.call_function(
                            &lib3,
                            &sym3,
                            &[Value::Float(a)],
                            &[NativeType::Float],
                            native_type(ret),
                        );
                        ret_to_f64(r)
                    })
                    .map_err(|e| e.to_string())?;
            }
            [p0] => {
                let p0 = *p0;
                let ret = rcode;
                let lib3 = lib2.clone();
                let sym3 = sym2.clone();
                linker
                    .func_wrap(HOST, &name, move |mut caller: Caller<'_, HostState>, a: i64| -> i64 {
                        let arg = match p0 {
                            's' => Value::String(caller_read_str(&mut caller, a)),
                            _ => Value::Int(a),
                        };
                        let r = backend.call_function(&lib3, &sym3, &[arg], &[native_type(p0)], native_type(ret));
                        match r {
                            Ok(Value::String(s)) => caller_write_str(&mut caller, &s).unwrap_or(0),
                            Ok(v) => match v {
                                Value::Float(f) => f as i64,
                                Value::Int(n) => n,
                                _ => 0,
                            },
                            Err(_) => 0,
                        }
                    })
                    .map_err(|e| e.to_string())?;
            }
            [p0, p1] if *p0 == 'f' || *p1 == 'f' => {
                let p0 = *p0;
                let p1 = *p1;
                let ret = rcode;
                let lib3 = lib2.clone();
                let sym3 = sym2.clone();
                if p0 == 'f' && p1 == 'f' {
                    linker
                        .func_wrap(HOST, &name, move |_: Caller<'_, HostState>, a: f64, b: f64| -> f64 {
                            let r = backend.call_function(
                                &lib3,
                                &sym3,
                                &[Value::Float(a), Value::Float(b)],
                                &[NativeType::Float, NativeType::Float],
                                native_type(ret),
                            );
                            ret_to_f64(r)
                        })
                        .map_err(|e| e.to_string())?;
                } else if p0 == 'f' {
                    let lib4 = lib2.clone();
                    let sym4 = sym2.clone();
                    linker
                        .func_wrap(HOST, &name, move |_: Caller<'_, HostState>, a: f64, b: i64| -> f64 {
                            let r = backend.call_function(
                                &lib4,
                                &sym4,
                                &[Value::Float(a), Value::Int(b)],
                                &[NativeType::Float, NativeType::Int],
                                native_type(ret),
                            );
                            ret_to_f64(r)
                        })
                        .map_err(|e| e.to_string())?;
                } else {
                    let lib4 = lib2.clone();
                    let sym4 = sym2.clone();
                    linker
                        .func_wrap(HOST, &name, move |_: Caller<'_, HostState>, a: i64, b: f64| -> f64 {
                            let r = backend.call_function(
                                &lib4,
                                &sym4,
                                &[Value::Int(a), Value::Float(b)],
                                &[NativeType::Int, NativeType::Float],
                                native_type(ret),
                            );
                            ret_to_f64(r)
                        })
                        .map_err(|e| e.to_string())?;
                }
            }
            [p0, p1] => {
                let p0 = *p0;
                let p1 = *p1;
                let ret = rcode;
                let lib3 = lib2.clone();
                let sym3 = sym2.clone();
                linker
                    .func_wrap(HOST, &name, move |mut caller: Caller<'_, HostState>, a: i64, b: i64| -> i64 {
                        let arg0 = match p0 {
                            's' => Value::String(caller_read_str(&mut caller, a)),
                            _ => Value::Int(a),
                        };
                        let arg1 = match p1 {
                            's' => Value::String(caller_read_str(&mut caller, b)),
                            _ => Value::Int(b),
                        };
                        let r = backend.call_function(
                            &lib3,
                            &sym3,
                            &[arg0, arg1],
                            &[native_type(p0), native_type(p1)],
                            native_type(ret),
                        );
                        ret_to_i64(r)
                    })
                    .map_err(|e| e.to_string())?;
            }
            _ => {
                return Err(format!(
                    "[JIT] Extensión '{}': el JIT soporta natives de hasta 2 argumentos por ahora",
                    n
                ))
            }
        }
        count += 1;
    }
    Ok(count)
}

// ── Helpers de memoria de arrays ────────────────────────────────────────────

fn arr_read_i64(caller: &mut Caller<'_, HostState>, addr: usize) -> i64 {
    if let Some(mem) = caller.get_export("memory").and_then(|e| e.into_memory()) {
        let data = mem.data(&mut *caller);
        if addr + 8 <= data.len() {
            return i64::from_le_bytes(data[addr..addr + 8].try_into().unwrap());
        }
    }
    0
}

fn arr_write_i64(caller: &mut Caller<'_, HostState>, addr: usize, v: i64) {
    if let Some(mem) = caller.get_export("memory").and_then(|e| e.into_memory()) {
        let data = mem.data_mut(caller);
        if addr + 8 <= data.len() {
            data[addr..addr + 8].copy_from_slice(&v.to_le_bytes());
        }
    }
}

fn arr_len(caller: &mut Caller<'_, HostState>, ptr: usize) -> i64 {
    arr_read_i64(caller, ptr + 8)
}

fn arr_cap(caller: &mut Caller<'_, HostState>, ptr: usize) -> i64 {
    arr_read_i64(caller, ptr)
}

fn arr_elem(caller: &mut Caller<'_, HostState>, ptr: usize, idx: usize, es: usize) -> i64 {
    let addr = ptr + 16 + idx * es;
    if es == 4 {
        if let Some(mem) = caller.get_export("memory").and_then(|e| e.into_memory()) {
            let data = mem.data(&mut *caller);
            if addr + 4 <= data.len() {
                return i32::from_le_bytes(data[addr..addr + 4].try_into().unwrap()) as i64;
            }
        }
        0
    } else {
        arr_read_i64(caller, addr)
    }
}

fn arr_set(caller: &mut Caller<'_, HostState>, ptr: usize, idx: usize, es: usize, v: i64) {
    let addr = ptr + 16 + idx * es;
    if es == 4 {
        if let Some(mem) = caller.get_export("memory").and_then(|e| e.into_memory()) {
            let data = mem.data_mut(caller);
            if addr + 4 <= data.len() {
                data[addr..addr + 4].copy_from_slice(&(v as i32).to_le_bytes());
            }
        }
    } else {
        arr_write_i64(caller, addr, v);
    }
}

fn arr_realloc(caller: &mut Caller<'_, HostState>, ptr: usize, new_cap: usize, es: usize) -> usize {
    let len = arr_len(caller, ptr) as usize;
    let size = (new_cap * es + 16) as i64;
    let new_ptr = caller_alloc(caller, size).unwrap_or(0) as usize;
    arr_write_i64(caller, new_ptr, new_cap as i64);
    arr_write_i64(caller, new_ptr + 8, len as i64);
    for i in 0..len {
        let e = arr_elem(caller, ptr, i, es);
        arr_set(caller, new_ptr, i, es, e);
    }
    new_ptr
}

static RNG_STATE: std::sync::Mutex<u64> = std::sync::Mutex::new(0x9E37_79B9_7F4A_7C15);

/// Registra los hosts de stdlib (math, json, fs).
fn register_stdlib_hosts(linker: &mut Linker<HostState>) -> Result<(), String> {
    linker
        .func_wrap(HOST, "math_sqrt", |_: Caller<'_, HostState>, v: f64| -> f64 { v.sqrt() })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "math_pow", |_: Caller<'_, HostState>, a: f64, b: f64| -> f64 { a.powf(b) })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "math_min", |_: Caller<'_, HostState>, a: f64, b: f64| -> f64 { a.min(b) })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "math_max", |_: Caller<'_, HostState>, a: f64, b: f64| -> f64 { a.max(b) })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "math_floor", |_: Caller<'_, HostState>, v: f64| -> f64 { v.floor() })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "math_ceil", |_: Caller<'_, HostState>, v: f64| -> f64 { v.ceil() })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "math_round", |_: Caller<'_, HostState>, v: f64| -> f64 { v.round() })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "math_random", |_: Caller<'_, HostState>| -> f64 {
            let mut s = RNG_STATE.lock().unwrap();
            *s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (*s >> 11) as f64 / (1u64 << 53) as f64
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "math_sin", |_: Caller<'_, HostState>, v: f64| -> f64 { v.sin() })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "math_cos", |_: Caller<'_, HostState>, v: f64| -> f64 { v.cos() })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "math_tan", |_: Caller<'_, HostState>, v: f64| -> f64 { v.tan() })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "math_log", |_: Caller<'_, HostState>, v: f64| -> f64 { v.ln() })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "math_range", |mut caller: Caller<'_, HostState>, a: i64, b: i64| -> i64 {
            let n = (b - a).max(0);
            let size = (n * 8 + 16) as i64;
            let ptr = caller_alloc(&mut caller, size).unwrap_or(0) as usize;
            arr_write_i64(&mut caller, ptr, n);
            arr_write_i64(&mut caller, ptr + 8, n);
            for i in 0..n {
                arr_set(&mut caller, ptr, i as usize, 8, a + i);
            }
            ptr as i64
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "json_stringify", |mut caller: Caller<'_, HostState>, v: i64, kind: i64| -> i64 {
            match kind {
                1 => {
                    let mut out = String::new();
                    json_serialize_record(&mut caller, v, &mut out);
                    caller_write_str(&mut caller, &out).unwrap_or(0)
                }
                2 => {
                    let mut out = String::new();
                    json_serialize_array(&mut caller, v, &mut out);
                    caller_write_str(&mut caller, &out).unwrap_or(0)
                }
                _ => v,
            }
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "json_parse", |mut caller: Caller<'_, HostState>, s: i64| -> i64 {
            let text = caller_read_str(&mut caller, s);
            match serde_json::from_str::<serde_json::Value>(&text) {
                Ok(v) => json_build(&mut caller, &v).0,
                Err(_) => 0,
            }
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "fs_exists", |mut caller: Caller<'_, HostState>, p: i64| -> i32 {
            let s = caller_read_str(&mut caller, p);
            if std::path::Path::new(&s).exists() {
                1
            } else {
                0
            }
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "fs_cwd", |mut caller: Caller<'_, HostState>| -> i64 {
            let cwd = std::env::current_dir()
                .map(|d| d.to_string_lossy().into_owned())
                .unwrap_or_default();
            caller_write_str(&mut caller, &cwd).unwrap_or(0)
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "fs_read_file", |mut caller: Caller<'_, HostState>, p: i64| -> i64 {
            let s = caller_read_str(&mut caller, p);
            match std::fs::read_to_string(&s) {
                Ok(contents) => caller_write_str(&mut caller, &contents).unwrap_or(0),
                Err(_) => 0,
            }
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "fs_write_file", |mut caller: Caller<'_, HostState>, p: i64, d: i64| -> i64 {
            let path = caller_read_str(&mut caller, p);
            let data = caller_read_str(&mut caller, d);
            let _ = std::fs::write(&path, data);
            0
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "fs_list_dir", |mut caller: Caller<'_, HostState>, p: i64| -> i64 {
            let s = caller_read_str(&mut caller, p);
            let joined = std::fs::read_dir(&s)
                .map(|rd| {
                    rd.filter_map(|e| e.ok().map(|e| e.file_name().to_string_lossy().into_owned()))
                        .collect::<Vec<_>>()
                        .join(",")
                })
                .unwrap_or_default();
            caller_write_str(&mut caller, &joined).unwrap_or(0)
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "fs_mkdir", |mut caller: Caller<'_, HostState>, p: i64| -> i64 {
            let s = caller_read_str(&mut caller, p);
            let _ = std::fs::create_dir_all(&s);
            0
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "fs_rm", |mut caller: Caller<'_, HostState>, p: i64| -> i64 {
            let s = caller_read_str(&mut caller, p);
            let _ = std::fs::remove_file(&s);
            0
        })
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// tags de tipo para JSON: 0=int, 1=string, 2=float, 3=bool, 4=char, 5=array, 6=record.
fn json_build(caller: &mut Caller<'_, HostState>, v: &serde_json::Value) -> (i64, i64) {
    match v {
        serde_json::Value::Null => (0, 0),
        serde_json::Value::Bool(b) => ((if *b { 1 } else { 0 }), 3),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                (i, 0)
            } else if let Some(f) = n.as_f64() {
                (f.to_bits() as i64, 2)
            } else {
                (0, 0)
            }
        }
        serde_json::Value::String(s) => (caller_write_str(caller, s).unwrap_or(0), 1),
        serde_json::Value::Array(items) => {
            let n = items.len();
            let ptr = caller_alloc(caller, (n * 8 + 16) as i64).unwrap_or(0) as usize;
            arr_write_i64(caller, ptr, n as i64);
            arr_write_i64(caller, ptr + 8, n as i64);
            for (i, it) in items.iter().enumerate() {
                let (val, _) = json_build(caller, it);
                arr_set(caller, ptr, i, 8, val);
            }
            (ptr as i64, 5)
        }
        serde_json::Value::Object(map) => {
            let n = map.len();
            let ptr = caller_alloc(caller, (n * 24 + 16) as i64).unwrap_or(0) as usize;
            arr_write_i64(caller, ptr, n as i64);
            arr_write_i64(caller, ptr + 8, n as i64);
            let mut i = 0;
            for (k, val) in map {
                let key = caller_write_str(caller, k).unwrap_or(0);
                let (vv, tag) = json_build(caller, val);
                arr_write_i64(caller, ptr + 16 + i * 24, key);
                arr_write_i64(caller, ptr + 16 + i * 24 + 8, vv);
                arr_write_i64(caller, ptr + 16 + i * 24 + 16, tag);
                i += 1;
            }
            (ptr as i64, 6)
        }
    }
}

fn json_serialize_val(caller: &mut Caller<'_, HostState>, val: i64, tag: i64, out: &mut String) {
    match tag {
        1 => {
            out.push('"');
            out.push_str(&json_escape(&caller_read_str(caller, val)));
            out.push('"');
        }
        2 => out.push_str(&format_float(f64::from_bits(val as u64))),
        3 => out.push_str(if val != 0 { "true" } else { "false" }),
        4 => out.push(char::from_u32(val as u32).unwrap_or('?')),
        5 => json_serialize_array(caller, val, out),
        6 => json_serialize_record(caller, val, out),
        _ => out.push_str(&val.to_string()),
    }
}

fn json_serialize_record(caller: &mut Caller<'_, HostState>, ptr: i64, out: &mut String) {
    let p = ptr as usize;
    let len = arr_len(caller, p);
    out.push('{');
    for i in 0..len as usize {
        if i > 0 {
            out.push(',');
        }
        let key = arr_read_i64(caller, p + 16 + i * 24);
        let val = arr_read_i64(caller, p + 16 + i * 24 + 8);
        let tag = arr_read_i64(caller, p + 16 + i * 24 + 16);
        out.push('"');
        out.push_str(&json_escape(&caller_read_str(caller, key)));
        out.push_str("\":");
        json_serialize_val(caller, val, tag, out);
    }
    out.push('}');
}

fn json_serialize_array(caller: &mut Caller<'_, HostState>, ptr: i64, out: &mut String) {
    let p = ptr as usize;
    let len = arr_len(caller, p);
    out.push('[');
    for i in 0..len as usize {
        if i > 0 {
            out.push(',');
        }
        let val = arr_elem(caller, p, i, 8);
        json_serialize_val(caller, val, 0, out);
    }
    out.push(']');
}

fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
}
fn register_record_hosts(linker: &mut Linker<HostState>) -> Result<(), String> {
    linker
        .func_wrap(HOST, "record_new", |mut caller: Caller<'_, HostState>, cap: i64| -> i64 {
            let size = cap * 24 + 16;
            let ptr = caller_alloc(&mut caller, size).unwrap_or(0) as usize;
            arr_write_i64(&mut caller, ptr, cap);
            arr_write_i64(&mut caller, ptr + 8, 0);
            ptr as i64
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "record_set", |mut caller: Caller<'_, HostState>, ptr: i64, key: i64, val: i64, tag: i64| -> i64 {
            let p = ptr as usize;
            let len = arr_len(&mut caller, p) as usize;
            let cap = arr_cap(&mut caller, p) as usize;
            let k = caller_read_str(&mut caller, key);
            for i in 0..len {
                let ki = arr_read_i64(&mut caller, p + 16 + i * 24);
                if caller_read_str(&mut caller, ki) == k {
                    arr_write_i64(&mut caller, p + 16 + i * 24 + 8, val);
                    arr_write_i64(&mut caller, p + 16 + i * 24 + 16, tag);
                    return p as i64;
                }
            }
            let mut new_p = p;
            let mut new_cap = cap;
            if len >= cap {
                new_cap = if cap == 0 { 4 } else { cap * 2 };
                let size = (new_cap * 24 + 16) as i64;
                let np = caller_alloc(&mut caller, size).unwrap_or(0) as usize;
                arr_write_i64(&mut caller, np, new_cap as i64);
                arr_write_i64(&mut caller, np + 8, len as i64);
                for i in 0..len {
                    let kk = arr_read_i64(&mut caller, p + 16 + i * 24);
                    let vv = arr_read_i64(&mut caller, p + 16 + i * 24 + 8);
                    let tt = arr_read_i64(&mut caller, p + 16 + i * 24 + 16);
                    arr_write_i64(&mut caller, np + 16 + i * 24, kk);
                    arr_write_i64(&mut caller, np + 16 + i * 24 + 8, vv);
                    arr_write_i64(&mut caller, np + 16 + i * 24 + 16, tt);
                }
                new_p = np;
            }
            arr_write_i64(&mut caller, new_p + 16 + len * 24, key);
            arr_write_i64(&mut caller, new_p + 16 + len * 24 + 8, val);
            arr_write_i64(&mut caller, new_p + 16 + len * 24 + 16, tag);
            arr_write_i64(&mut caller, new_p + 8, (len + 1) as i64);
            new_p as i64
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "record_get", |mut caller: Caller<'_, HostState>, ptr: i64, key: i64| -> i64 {
            let p = ptr as usize;
            let len = arr_len(&mut caller, p) as usize;
            let k = caller_read_str(&mut caller, key);
            for i in 0..len {
                let ki = arr_read_i64(&mut caller, p + 16 + i * 24);
                if caller_read_str(&mut caller, ki) == k {
                    return arr_read_i64(&mut caller, p + 16 + i * 24 + 8);
                }
            }
            0
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "record_has", |mut caller: Caller<'_, HostState>, ptr: i64, key: i64| -> i32 {
            let p = ptr as usize;
            let len = arr_len(&mut caller, p) as usize;
            let k = caller_read_str(&mut caller, key);
            for i in 0..len {
                let ki = arr_read_i64(&mut caller, p + 16 + i * 24);
                if caller_read_str(&mut caller, ki) == k {
                    return 1;
                }
            }
            0
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "record_tag", |mut caller: Caller<'_, HostState>, ptr: i64, key: i64| -> i64 {
            let p = ptr as usize;
            let len = arr_len(&mut caller, p) as usize;
            let k = caller_read_str(&mut caller, key);
            for i in 0..len {
                let ki = arr_read_i64(&mut caller, p + 16 + i * 24);
                if caller_read_str(&mut caller, ki) == k {
                    return arr_read_i64(&mut caller, p + 16 + i * 24 + 16);
                }
            }
            0
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "record_len", |mut caller: Caller<'_, HostState>, ptr: i64| -> i64 {
            arr_len(&mut caller, ptr as usize)
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "record_keys", |mut caller: Caller<'_, HostState>, ptr: i64| -> i64 {
            let p = ptr as usize;
            let len = arr_len(&mut caller, p) as usize;
            let size = (len * 8 + 16) as i64;
            let out = caller_alloc(&mut caller, size).unwrap_or(0) as usize;
            arr_write_i64(&mut caller, out, len as i64);
            arr_write_i64(&mut caller, out + 8, len as i64);
            for i in 0..len {
                let ki = arr_read_i64(&mut caller, p + 16 + i * 24);
                arr_set(&mut caller, out, i, 8, ki);
            }
            out as i64
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "record_values", |mut caller: Caller<'_, HostState>, ptr: i64| -> i64 {
            let p = ptr as usize;
            let len = arr_len(&mut caller, p) as usize;
            let size = (len * 8 + 16) as i64;
            let out = caller_alloc(&mut caller, size).unwrap_or(0) as usize;
            arr_write_i64(&mut caller, out, len as i64);
            arr_write_i64(&mut caller, out + 8, len as i64);
            for i in 0..len {
                let vi = arr_read_i64(&mut caller, p + 16 + i * 24 + 8);
                arr_set(&mut caller, out, i, 8, vi);
            }
            out as i64
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "record_to_string", |mut caller: Caller<'_, HostState>, ptr: i64| -> i64 {
            let s = record_to_string(&mut caller, ptr);
            caller_write_str(&mut caller, &s).unwrap_or(0)
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "http_get", |mut caller: Caller<'_, HostState>, url: i64| -> i64 {
            let u = caller_read_str(&mut caller, url);
            match ureq::get(&u).call() {
                Ok(resp) => match resp.into_string() {
                    Ok(body) => caller_write_str(&mut caller, &body).unwrap_or(0),
                    Err(_) => 0,
                },
                Err(_) => 0,
            }
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "http_post", |mut caller: Caller<'_, HostState>, url: i64, data: i64| -> i64 {
            let u = caller_read_str(&mut caller, url);
            let d = caller_read_str(&mut caller, data);
            match ureq::post(&u).send_string(&d) {
                Ok(resp) => match resp.into_string() {
                    Ok(body) => caller_write_str(&mut caller, &body).unwrap_or(0),
                    Err(_) => 0,
                },
                Err(_) => 0,
            }
        })
        .map_err(|e| e.to_string())?;
    // CMX: layout [tag:i64][props_ptr:i64][children_ptr:i64].
    linker
        .func_wrap(HOST, "cmx_new", |mut caller: Caller<'_, HostState>, tag: i64, kind: i64| -> i64 {
            // layout: [tag][props_ptr][children_ptr][kind] (kind 0=elemento, 1=texto)
            let ptr = caller_alloc(&mut caller, 32).unwrap_or(0) as usize;
            arr_write_i64(&mut caller, ptr, tag);
            arr_write_i64(&mut caller, ptr + 8, 0);
            arr_write_i64(&mut caller, ptr + 16, 0);
            arr_write_i64(&mut caller, ptr + 24, kind);
            ptr as i64
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "cmx_set_prop", |mut caller: Caller<'_, HostState>, ptr: i64, key: i64, val: i64, tag: i64| -> i64 {
            let p = ptr as usize;
            let mut props = arr_read_i64(&mut caller, p + 8) as usize;
            if props == 0 {
                let np = caller_alloc(&mut caller, (4 * 24 + 16) as i64).unwrap_or(0) as usize;
                arr_write_i64(&mut caller, np, 4);
                arr_write_i64(&mut caller, np + 8, 0);
                arr_write_i64(&mut caller, p + 8, np as i64);
                props = np;
            }
            let pr = props as i64;
            let len = arr_len(&mut caller, props) as usize;
            let cap = arr_cap(&mut caller, props) as usize;
            let k = caller_read_str(&mut caller, key);
            for i in 0..len {
                let ki = arr_read_i64(&mut caller, props + 16 + i * 24);
                if caller_read_str(&mut caller, ki) == k {
                    arr_write_i64(&mut caller, props + 16 + i * 24 + 8, val);
                    arr_write_i64(&mut caller, props + 16 + i * 24 + 16, tag);
                    return pr;
                }
            }
            if len + 1 > cap {
                // realloc copiando [key, val, tag]
                let new_cap = if cap == 0 { 4 } else { cap * 2 };
                let new_cap = new_cap.max(len + 1);
                let np = caller_alloc(&mut caller, (new_cap * 24 + 16) as i64).unwrap_or(0) as usize;
                arr_write_i64(&mut caller, np, new_cap as i64);
                arr_write_i64(&mut caller, np + 8, len as i64);
                for i in 0..len {
                    let kk = arr_read_i64(&mut caller, props + 16 + i * 24);
                    let vv = arr_read_i64(&mut caller, props + 16 + i * 24 + 8);
                    let tt = arr_read_i64(&mut caller, props + 16 + i * 24 + 16);
                    arr_write_i64(&mut caller, np + 16 + i * 24, kk);
                    arr_write_i64(&mut caller, np + 16 + i * 24 + 8, vv);
                    arr_write_i64(&mut caller, np + 16 + i * 24 + 16, tt);
                }
                props = np;
                arr_write_i64(&mut caller, p + 8, props as i64);
            }
            arr_write_i64(&mut caller, props + 16 + len * 24, key);
            arr_write_i64(&mut caller, props + 16 + len * 24 + 8, val);
            arr_write_i64(&mut caller, props + 16 + len * 24 + 16, tag);
            arr_write_i64(&mut caller, props + 8, (len + 1) as i64);
            arr_read_i64(&mut caller, p + 8)
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "cmx_add_child", |mut caller: Caller<'_, HostState>, ptr: i64, val: i64, tag: i64| -> i64 {
            let p = ptr as usize;
            let mut children = arr_read_i64(&mut caller, p + 16) as usize;
            if children == 0 {
                let nc = caller_alloc(&mut caller, (4 * 16 + 16) as i64).unwrap_or(0) as usize;
                arr_write_i64(&mut caller, nc, 4);
                arr_write_i64(&mut caller, nc + 8, 0);
                arr_write_i64(&mut caller, p + 16, nc as i64);
                children = nc;
            }
            let len = arr_len(&mut caller, children) as usize;
            let cap = arr_cap(&mut caller, children) as usize;
            if len + 1 > cap {
                // realloc copiando [val, tag]
                let new_cap = if cap == 0 { 4 } else { cap * 2 };
                let new_cap = new_cap.max(len + 1);
                let np = caller_alloc(&mut caller, (new_cap * 16 + 16) as i64).unwrap_or(0) as usize;
                arr_write_i64(&mut caller, np, new_cap as i64);
                arr_write_i64(&mut caller, np + 8, len as i64);
                for i in 0..len {
                    let v = arr_read_i64(&mut caller, children + 16 + i * 16);
                    let t = arr_read_i64(&mut caller, children + 16 + i * 16 + 8);
                    arr_write_i64(&mut caller, np + 16 + i * 16, v);
                    arr_write_i64(&mut caller, np + 16 + i * 16 + 8, t);
                }
                children = np;
                arr_write_i64(&mut caller, p + 16, children as i64);
            }
            arr_write_i64(&mut caller, children + 16 + len * 16, val);
            arr_write_i64(&mut caller, children + 16 + len * 16 + 8, tag);
            arr_write_i64(&mut caller, children + 8, (len + 1) as i64);
            arr_read_i64(&mut caller, p + 16)
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "cmx_to_string", |mut caller: Caller<'_, HostState>, ptr: i64| -> i64 {
            let s = cmx_format(&mut caller, ptr as usize);
            caller_write_str(&mut caller, &s).unwrap_or(0)
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "print_any", |mut caller: Caller<'_, HostState>, val: i64, tag: i64| {
            let s = fmt_val_to_string(&mut caller, val, tag);
            print_arg(&mut caller, &s);
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "fn_handle", |mut caller: Caller<'_, HostState>, table_idx: i64, nombre: i64| -> i64 {
            // Handle de función: [tabla_idx][capturas=0][nombre] (24 bytes).
            let ptr = caller_alloc(&mut caller, 24).unwrap_or(0) as usize;
            arr_write_i64(&mut caller, ptr, table_idx);
            arr_write_i64(&mut caller, ptr + 8, 0);
            arr_write_i64(&mut caller, ptr + 16, nombre);
            ptr as i64
        })
        .map_err(|e| e.to_string())?;
    linker
        .func_wrap(HOST, "fn_to_string", |mut caller: Caller<'_, HostState>, handle: i64| -> i64 {
            let nombre = arr_read_i64(&mut caller, (handle as usize) + 16);
            let s = caller_read_str(&mut caller, nombre);
            caller_write_str(&mut caller, &s).unwrap_or(0)
        })
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Tipo de un tag: compuesto (`tipo<<8 | kind`) o legacy (0-5, arr_kind_code).
fn tag_type(tag: i64) -> i32 {
    if tag >= 256 {
        (tag >> 8) as i32
    } else {
        tag as i32
    }
}

/// Formatea un valor según su tag. Tag = `tipo<<8 | kind` (kind solo para arrays).
/// tipo: 0=int,1=string,2=float,3=bool,4=char,5=cmx,6=array,7=record.
fn fmt_val_to_string(caller: &mut Caller<'_, HostState>, val: i64, tag: i64) -> String {
    let t = tag_type(tag);
    let kind = (tag & 0xff) as i32;
    match t {
        1 => caller_read_str(caller, val),
        2 => format_float(f64::from_bits(val as u64)),
        3 => {
            if val != 0 {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        4 => char::from_u32(val as u32).unwrap_or('?').to_string(),
        5 => cmx_format(caller, val as usize),
        6 => {
            // Array: es=16 si es de Cmx (kind 5), si no 8.
            let es = if kind == 5 { 16 } else { 8 };
            arr_to_string(caller, val, es, kind as i64)
        }
        7 => record_to_string(caller, val),
        _ => val.to_string(),
    }
}

/// `[e1, e2, ...]` — kind 5 = array de Cmx (entradas `[val, tag]` stride 16).
fn arr_to_string(caller: &mut Caller<'_, HostState>, ptr: i64, es: i64, kind: i64) -> String {
    let p = ptr as usize;
    let es = es as usize;
    let len = arr_len(caller, p);
    let mut out = String::from("[");
    for i in 0..len as usize {
        if i > 0 {
            out.push_str(", ");
        }
        let e = arr_elem(caller, p, i, es);
        match kind {
            1 => {
                out.push('"');
                out.push_str(&json_escape(&caller_read_str(caller, e)));
                out.push('"');
            }
            2 => out.push_str(&format_float(f64::from_bits(e as u64))),
            3 => out.push_str(if e != 0 { "true" } else { "false" }),
            4 => out.push(char::from_u32(e as u32).unwrap_or('?')),
            5 => {
                // entrada [val, tag] stride 16
                let tg = arr_read_i64(caller, p + 16 + i * 16 + 8);
                let tv = tag_type(tg);
                if tv == 1 {
                    out.push('"');
                    out.push_str(&json_escape(&caller_read_str(caller, e)));
                    out.push('"');
                } else if tv == 5 {
                    // CmxValue: texto (kind 1) → comillas; elemento → plano
                    let ck = arr_read_i64(caller, (e as usize) + 24);
                    if ck == 1 {
                        let ctag = arr_read_i64(caller, e as usize);
                        out.push('"');
                        out.push_str(&json_escape(&caller_read_str(caller, ctag)));
                        out.push('"');
                    } else {
                        out.push_str(&cmx_format(caller, e as usize));
                    }
                } else {
                    out.push_str(&fmt_val_to_string(caller, e, tg));
                }
            }
            _ => out.push_str(&e.to_string()),
        }
    }
    out.push(']');
    out
}

/// `{k: v, ...}` — formatea cada valor por su tag (claves ordenadas, como el walker).
fn record_to_string(caller: &mut Caller<'_, HostState>, ptr: i64) -> String {
    let p = ptr as usize;
    let len = arr_len(caller, p);
    let mut entries: Vec<(String, i64, i64)> = Vec::with_capacity(len as usize);
    for i in 0..len as usize {
        let key = arr_read_i64(caller, p + 16 + i * 24);
        let val = arr_read_i64(caller, p + 16 + i * 24 + 8);
        let tag = arr_read_i64(caller, p + 16 + i * 24 + 16);
        entries.push((caller_read_str(caller, key), val, tag));
    }
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    let mut out = String::from("{");
    for (i, (key, val, tag)) in entries.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push_str(key);
        out.push_str(": ");
        let t = tag_type(*tag);
        if t == 1 {
            out.push('"');
            out.push_str(&json_escape(&caller_read_str(caller, *val)));
            out.push('"');
        } else {
            out.push_str(&fmt_val_to_string(caller, *val, *tag));
        }
    }
    out.push('}');
    out
}

/// Formatea un CmxValue. Un CmxValue de "texto" (kind=1) se muestra plano (el tag
/// sin `<.../>`), para paridad con el walker (los Text children del cuerpo).
fn cmx_format(caller: &mut Caller<'_, HostState>, p: usize) -> String {
    let tag = arr_read_i64(caller, p);
    let props = arr_read_i64(caller, p + 8) as usize;
    let children = arr_read_i64(caller, p + 16) as usize;
    let kind = arr_read_i64(caller, p + 24);
    let nprops = if props != 0 { arr_len(caller, props) as usize } else { 0 };
    let nchild = if children != 0 { arr_len(caller, children) as usize } else { 0 };
    if kind == 1 {
        return caller_read_str(caller, tag);
    }
    let mut out = String::from("<");
    out.push_str(&caller_read_str(caller, tag));
    let mut prop_entries: Vec<(String, i64, i64)> = Vec::with_capacity(nprops);
    for i in 0..nprops {
        let key = arr_read_i64(caller, props + 16 + i * 24);
        let val = arr_read_i64(caller, props + 16 + i * 24 + 8);
        let t = arr_read_i64(caller, props + 16 + i * 24 + 16);
        prop_entries.push((caller_read_str(caller, key), val, t));
    }
    prop_entries.sort_by(|a, b| a.0.cmp(&b.0));
    for (key, val, t) in prop_entries {
        out.push(' ');
        out.push_str(&key);
        out.push_str("=\"");
        let tv = tag_type(t);
        if tv == 1 {
            out.push_str(&caller_read_str(caller, val));
        } else {
            out.push_str(&fmt_val_to_string(caller, val, t));
        }
        out.push('"');
    }
    if nchild == 0 {
        out.push_str(" />");
    } else {
        out.push_str(">... (");
        out.push_str(&nchild.to_string());
        out.push_str(" children)</");
        out.push_str(&caller_read_str(caller, tag));
        out.push('>');
    }
    out
}

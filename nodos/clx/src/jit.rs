//! Motor JIT: CLS → WASM → wasmtime (Cranelift).
//!
//! `clx run --jit <archivo> [-- args...]` compila el archivo con el backend WASM
//! de cls-core (usando el type map del TypeChecker) y lo ejecuta en wasmtime.

use cls_core::config::types::TypesConfig;
use cls_core::middleware::TypeChecker;
use wasmtime::{Caller, Engine, Linker, Memory, Module, Store, Val};

const HOST: &str = "env";

/// Estado del host: separador de argumentos en `print`.
struct HostState {
    first_in_line: bool,
}

impl Default for HostState {
    fn default() -> Self {
        Self { first_in_line: true }
    }
}

pub fn run_jit(entry: &str, app_args: &[String]) -> i32 {
    let source = match std::fs::read_to_string(entry) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error al leer '{}': {}", entry, e);
            return 1;
        }
    };

    // Parseo
    let mut lexer = cls_core::frontend::Lexer::new(&source);
    let tokens = match lexer.tokenize() {
        Ok(t) => t,
        Err(e) => {
            cls_runtime::show_syntax_error(e, &source, entry);
            return 1;
        }
    };
    let mut parser = cls_core::frontend::Parser::new(tokens);
    let module = match parser.parse() {
        Ok(m) => m,
        Err(e) => {
            cls_runtime::show_syntax_error(e, &source, entry);
            return 1;
        }
    };

    // Type checker: llena el mapa Span → Type (requerido por el backend).
    let types_config = TypesConfig {
        check: true,
        strict: true,
        no_implicit_any: true,
        null_safety: true,
    };
    let mut checker = TypeChecker::new(types_config);
    if let Err(e) = checker.check(&module) {
        eprintln!("Error interno del type checker en '{}': {}", entry, e);
        return 1;
    }

    // Emitir WASM.
    let backend = cls_core::backend::wasm::WasmBackend::new(checker.type_map().clone());
    let wasm_bytes = match backend.emit(&module) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("[JIT] Error de compilación en '{}': {}", entry, e);
            return 1;
        }
    };

    run_wasm(&wasm_bytes, entry, app_args)
}

fn run_wasm(wasm_bytes: &[u8], entry: &str, app_args: &[String]) -> i32 {
    let engine = Engine::default();
    let module = match Module::new(&engine, wasm_bytes) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("[JIT] Módulo WASM inválido para '{}':\n{:?}", entry, e);
            return 1;
        }
    };
    let mut store = Store::new(&engine, HostState::default());
    let mut linker = Linker::new(&engine);

    if let Err(e) = register_host_functions(&mut linker) {
        eprintln!("[JIT] Error registrando funciones host: {}", e);
        return 1;
    }

    let instance = match linker.instantiate(&mut store, &module) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("[JIT] Error de instanciación para '{}': {}", entry, e);
            return 1;
        }
    };

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

    let main = match instance.get_typed_func::<i64, i64>(&mut store, "main") {
        Ok(f) => f,
        Err(e) => {
            eprintln!("[JIT] Export 'main' no disponible: {}", e);
            return 1;
        }
    };

    match main.call(&mut store, args_ptr) {
        Ok(code) => code as i32,
        Err(e) => {
            eprintln!("[JIT] Error en ejecución: {}", e);
            1
        }
    }
}

/// Escribe los args como Array<String> en memoria y devuelve el ptr.
fn write_args(
    store: &mut Store<HostState>,
    memory: &Memory,
    alloc: &wasmtime::TypedFunc<i64, i64>,
    app_args: &[String],
) -> Result<i64, String> {
    let n = app_args.len() as i64;
    let array_ptr = alloc.call(&mut *store, n * 8 + 8).map_err(|e| e.to_string())?;
    memory
        .write(&mut *store, array_ptr as usize, &n.to_le_bytes())
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
            .write(&mut *store, (array_ptr as usize) + 8 + i * 8, &packed.to_le_bytes())
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
        .func_wrap(HOST, "trap", |_: Caller<'_, HostState>, msg: i64| -> () {
            eprintln!("[JIT] Error de runtime: mensaje {}", msg);
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

    Ok(())
}

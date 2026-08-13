//! Orquestación del motor: pipeline CLS → WASM y despacho al runtime.
//!
//! El pipeline (lectura → lexer → parser → typeck → imports → span_shift →
//! flatten → emisión) es compartido por todos los runtimes. La ejecución del
//! WASM vive en [`crate::wasmtime_rt`] (desktop) y [`crate::wasmi_rt`] (web).

use cls_core::config::types::TypesConfig;
use cls_core::error::{ClsError, Span};
use cls_core::frontend::ast::Module as ClsModule;
use cls_core::middleware::TypeChecker;
use std::time::Instant;

use crate::error::{show_cls_error, show_type_diag};
use crate::flatten::flatten_imports;
use crate::resolve::{cache_dir, cache_key, load_import_modules_hooked};
use crate::timing::{jit_timing, tick};
use crate::{JitContext, RuntimeKind};

/// Ejecuta un programa CLS con el JIT (CLS → WASM → wasmtime).
/// Devuelve el exit code del proceso (0 = OK, 1 = error).
pub fn run_jit(entry: &str, app_args: &[String], target_str: Option<&str>, ctx: &JitContext) -> i32 {
    run_jit_with(entry, app_args, target_str, ctx, RuntimeKind::Wasmtime)
}

/// Ejecuta con un runtime explícito (wasmtime o wasmi).
///
/// Diferencias por runtime:
/// - **wasmtime**: excepciones CLS (tag + try_table), errores de runtime con
///   payload (msg, span) → caret exacto.
/// - **wasmi** (sin propuesta de exception-handling): el backend emite sin tag;
///   los `try/catch`/`throw` fallan en compilación con error claro y los errores
///   de runtime (div 0, índice) son traps (`unreachable`) con el shadow call
///   stack pero sin caret del span CLS.
pub fn run_jit_with(
    entry: &str,
    app_args: &[String],
    target_str: Option<&str>,
    ctx: &JitContext,
    runtime: RuntimeKind,
) -> i32 {
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

    let entry_path = std::path::Path::new(entry);

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
    // Funciones host del nodo (intrinsics): el typeck las registra en el scope
    // global para tipar las llamadas (el emisor las compila vía env.host_call).
    checker.register_host_intrinsics(ctx.host_intrinsics);
    let base_dir = std::path::Path::new(entry)
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .to_path_buf();
    let mut seen = std::collections::HashSet::new();
    let mut imports: Vec<(String, String, ClsModule)> = Vec::new();
    let manifest = cls_core::config::ModuleManifest::find_in_dir(&base_dir);
    if let Err(e) = load_import_modules_hooked(
        &module,
        &base_dir,
        &mut seen,
        &mut imports,
        manifest.as_ref(),
        ctx.module_source_resolver,
    ) {
        show_cls_error(&e, entry, Some(&source));
        return 1;
    }
    // Clave de caché: source + versión + target + sources de módulos + runtime
    // (wasmtime y wasmi emiten bytes distintos: el tag de excepciones).
    let module_sources: Vec<String> = imports.iter().map(|(_, src, _)| src.clone()).collect();
    let key = cache_key(&source, target_str, entry_path, &module_sources, runtime.as_str());
    let cache_path = cache_dir().join(format!("{:016x}.wasm", key));
    if let Ok(cached) = std::fs::read(&cache_path) {
        if timing {
            eprintln!("[JIT-TIMING] caché CLS→WASM: HIT ({} bytes)", cached.len());
        }
        return run_wasm_dispatch(&cached, entry, app_args, timing, t, None, &imports, ctx, runtime);
    }
    if timing {
        eprintln!("[JIT-TIMING] caché CLS→WASM: miss");
    }
    // Desplazar los spans de cada módulo importado con un offset de línea único.
    for (i, (_path, _src, m)) in imports.iter_mut().enumerate() {
        let offset = 100000u32 * (i as u32 + 1);
        cls_core::frontend::span_shift::shift_module(m, offset);
    }
    // Prelude para el checker: solo (path, module).
    let prelude: Vec<(String, ClsModule)> = imports
        .iter()
        .map(|(p, _, m)| (p.clone(), m.clone()))
        .collect();
    if let Err(e) = checker.check_with_prelude(&module, &prelude) {
        show_cls_error(&e, entry, Some(&source));
        return 1;
    }
    let has_type_errors = checker.diagnostics().iter().any(|d| {
        matches!(d.severity, cls_core::error::diagnostic::Severity::Error)
    });
    if has_type_errors {
        for diag in checker.diagnostics() {
            if matches!(diag.severity, cls_core::error::diagnostic::Severity::Error) {
                show_type_diag(diag, &source, entry, &imports);
                return 1;
            }
        }
    }
    t = tick(timing, "typecheck", t);

    // Emitir WASM (target para la directiva `when`).
    let target = match target_str {
        Some(tt) => cls_core::frontend::ast::Target::parse(tt),
        None => cls_core::frontend::ast::Target::host(),
    };
    let type_map = checker.type_map();

    let prelude_for_flatten: Vec<(String, ClsModule)> = imports
        .iter()
        .map(|(p, _, m)| (p.clone(), m.clone()))
        .collect();
    let merged = flatten_imports(&module, &prelude_for_flatten);

    // wasmtime → excepciones CLS; wasmi → modo sin excepciones (sin tag).
    // El modo app exige main; los intrinsics del nodo viajan al backend.
    let exceptions = matches!(runtime, RuntimeKind::Wasmtime);
    let opts = cls_core::backend::wasm::WasmBackendOptions {
        exceptions,
        require_main: true,
        intrinsics: ctx.host_intrinsics.to_vec(),
    };
    let backend = cls_core::backend::wasm::WasmBackend::with_options(type_map, target, opts);
    let wasm_bytes = match backend.emit(&merged) {
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

    run_wasm_dispatch(&wasm_bytes, entry, app_args, timing, t, Some(cache_path), &imports, ctx, runtime)
}

fn run_wasm_dispatch(
    wasm_bytes: &[u8],
    entry: &str,
    app_args: &[String],
    timing: bool,
    t: Instant,
    cache_path: Option<std::path::PathBuf>,
    imports: &[(String, String, ClsModule)],
    ctx: &JitContext,
    runtime: RuntimeKind,
) -> i32 {
    match runtime {
        RuntimeKind::Wasmtime => crate::wasmtime_rt::run_wasm_wasmtime(
            wasm_bytes, entry, app_args, timing, t, cache_path, imports, ctx,
        ),
        #[cfg(feature = "wasmi-runtime")]
        RuntimeKind::Wasmi => crate::wasmi_rt::run_wasm_wasmi(
            wasm_bytes, entry, app_args, timing, t, cache_path, imports, ctx,
        ),
        #[cfg(not(feature = "wasmi-runtime"))]
        RuntimeKind::Wasmi => {
            eprintln!(
                "[JIT] runtime wasmi no disponible: compila cls-jit con la feature 'wasmi-runtime'"
            );
            1
        }
    }
}

/// Desempaqueta `(line<<32)|col` en un Span.
pub(crate) fn unpack_span(packed: i64) -> Span {
    let line = ((packed >> 32) & 0xffff_ffff) as u32;
    let col = (packed & 0xffff_ffff) as u32;
    Span::new(line, col, line, col)
}

/// Offsets de línea de los módulos importados: `(100000*(i+1), path, source)`.
pub(crate) fn module_offsets(modules: &[(String, String, ClsModule)]) -> Vec<(u32, String, String)> {
    modules
        .iter()
        .enumerate()
        .map(|(i, (path, src, _m))| (100000u32 * (i as u32 + 1), path.clone(), src.clone()))
        .collect()
}

/// Resuelve el archivo y source reales de un span de runtime (puede estar
/// desplazado por pertenecer a un módulo importado). Devuelve (archivo, source,
/// span de-shifteado). Para el entry, `source` es `None` → el formateador lee del
/// archivo en disco (evita perder la línea+caret del error del archivo principal).
pub(crate) fn resolve_runtime_span(
    span: Span,
    entry: &str,
    modules: &[(u32, String, String)],
) -> (String, Option<String>, Span) {
    let raw_line = span.start_line;
    if raw_line >= 100000 {
        let idx = (raw_line / 100000) as usize;
        let offset = idx * 100000;
        let real_line = raw_line - offset as u32;
        if let Some((off, file, source)) = modules.iter().find(|(o, _, _)| *o == offset as u32) {
            let _ = off;
            let real = Span::new(real_line, span.start_col, real_line, span.end_col);
            return (file.clone(), Some(source.clone()), real);
        }
    }
    (entry.to_string(), None, span)
}

/// Índice de integridad de los módulos del workspace (INFORMATIVO). El hook lo
/// provee el nodo; si el nodo no lo registra, se omite. Se pasan las rutas
/// ABSOLUTAS de los módulos resueltos que caen FUERA del workspace (globales
/// de ~/.cls).
pub(crate) fn maybe_write_module_index(
    entry: &str,
    modules: &[(String, String, ClsModule)],
    ctx: &JitContext,
) {
    if let Some(hook) = ctx.module_index {
        let base_dir = std::path::Path::new(entry)
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .to_path_buf();
        let manifest = cls_core::config::ModuleManifest::find_in_dir(&base_dir);
        let root = hook.workspace_root(std::path::Path::new(entry));
        let mut extra: Vec<std::path::PathBuf> = Vec::new();
        for (import_path, _src, _m) in modules {
            for cand in crate::resolve::module_candidates(import_path, &base_dir, manifest.as_ref()) {
                if cand.is_file() {
                    if !cand.starts_with(&root) {
                        extra.push(cand);
                    }
                    break;
                }
            }
        }
        hook.write_module_index(std::path::Path::new(entry), &extra);
    }
}

/// Construye el reporte de un error de runtime como string (trace completo).
/// Recibe las piezas que cada runtime extrae de su error:
/// - `msg`: mensaje de la excepción CLS (vacío si el error fue un trap del host).
/// - `span`: span de la excepción CLS (puede ser None).
/// - `call_stack`: shadow call stack `(nombre, span)` acumulado por fn_enter.
/// - `pending_call_site`: span del call site pendiente (si el backend lo emitió).
/// - `trap_text`: root cause del error del runtime (para traps).
pub fn build_error_string(    msg: String,
    span: Option<Span>,
    call_stack: Vec<(String, Span)>,
    pending_call_site: Option<Span>,
    trap_text: String,
    entry: &str,
    modules: &[(u32, String, String)],
) -> String {
    use cls_runtime::error_report::{format_error, ErrorFormat, ErrorReport};

    // Shadow call stack: frames (nombre, span) en orden de ejecución
    // (main → outer → inner). El frame del error es el último. Cada span
    // se de-shiftea (los módulos importados tienen offset 100000*n).
    let stack: Vec<cls_core::error::StackFrame> = call_stack
        .iter()
        .map(|(name, sp)| {
            let (file, _source, real) = resolve_runtime_span(*sp, entry, modules);
            cls_core::error::StackFrame::new(name, Some(real), &file)
        })
        .collect();
    let mut error_span = span;
    let short = trap_text;
    // Stack overflow: mensaje limpio y solo los frames del tope de la
    // recursión (no los ~1000 acumulados).
    let is_stack_overflow =
        short.contains("call stack exhausted") || short.contains("stack overflow");
    let from_call_site = error_span.is_none() && pending_call_site.is_some();
    let trap_msg = if is_stack_overflow {
        "stack overflow".to_string()
    } else if from_call_site {
        // Error de conversión de host (`int("x")` inválido): el span ya
        // apunta al call site y el mensaje es del usuario (sin prefijo).
        short
    } else if short.starts_with("Trap") {
        format!("Trap WASM: {}", short.trim_start_matches("Trap "))
    } else {
        format!("Trap WASM: {}", short)
    };
    let fallback_msg = if msg.is_empty() { trap_msg } else { msg };
    let mut stack = stack;
    if is_stack_overflow {
        // Solo el punto de recursión + los frames más recientes.
        let keep = stack.len().saturating_sub(3);
        stack.drain(..keep);
    }
    if error_span.is_none() {
        error_span = pending_call_site.or_else(|| stack.last().and_then(|f| f.span));
    }
    if let Some(span) = error_span {
        // De-shiftear el span si pertenece a un módulo importado y resolver el
        // archivo/source real (para el caret correcto).
        let (file, source, real_span) = resolve_runtime_span(span, entry, modules);
        let report = ErrorReport {
            error: ClsError::RuntimeError(fallback_msg),
            span: Some(real_span),
            stack,
            import_trace: vec![],
            source_file: file,
            source,
        };
        format_error(&report, &ErrorFormat::Console)
    } else {
        let report = ErrorReport {
            error: ClsError::RuntimeError(fallback_msg),
            span: stack.last().and_then(|f| f.span),
            stack,
            import_trace: vec![],
            source_file: entry.to_string(),
            source: None,
        };
        format_error(&report, &ErrorFormat::Console)
    }
}

/// Reporta un error de runtime con el trazo completo (AGENTS.md: obligatorio).
pub(crate) fn finish_run_error(
    msg: String,
    span: Option<Span>,
    call_stack: Vec<(String, Span)>,
    pending_call_site: Option<Span>,
    trap_text: String,
    entry: &str,
    modules: &[(u32, String, String)],
) -> i32 {
    let text = build_error_string(
        msg,
        span,
        call_stack,
        pending_call_site,
        trap_text,
        entry,
        modules,
    );
    eprintln!("{}", text);
    1
}

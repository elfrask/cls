//! Motor JIT: delegación al crate `cls-jit` (motor reusable).
//!
//! `clx run --jit <archivo> [-- args...]` compila el archivo con el motor
//! centralizado de `cls-jit` (CLS -> WASM -> wasmtime), inyectando el backend
//! nativo del nodo (`DynamicBackend`) y los hooks del índice de módulos del
//! workspace. El motor vive en `cls-jit` para que el nodo de bindings lo
//! reutilice sin duplicar.

use cls_core::config::ModuleManifest;
use std::path::Path;
use std::sync::Arc;

pub use cls_jit::{cache_dir, load_import_modules};

/// Hooks del nodo al índice de módulos (caché del workspace, INFORMATIVO).
struct ClxModuleIndexHook;

impl cls_jit::ModuleIndexHook for ClxModuleIndexHook {
    fn workspace_root(&self, entry: &std::path::Path) -> std::path::PathBuf {
        crate::module_index::workspace_root(entry)
    }

    fn write_module_index(&self, entry: &std::path::Path, extra: &[std::path::PathBuf]) {
        let _ = crate::module_index::write_module_index(entry, extra);
    }
}

/// Ejecuta un programa CLS con el JIT (CLS -> WASM -> wasmtime).
/// Devuelve el exit code (0 = OK, 1 = error).
pub fn run_jit(entry: &str, app_args: &[String], target_str: Option<&str>) -> i32 {
    let ctx = cls_jit::JitContext {
        native_backend: Arc::new(cls_runtime::DynamicBackend),
        module_index: Some(&ClxModuleIndexHook),
        host_intrinsics: &[],
        host_call_handler: None,
        module_source_resolver: None,
        output: None,
    };
    // `CLS_JIT_RUNTIME=wasmi` -> ejecutar con wasmi (intérprete puro, sin
    // excepciones CLS). Útil para validar el runtime del navegador desde el CLI.
    let runtime = match std::env::var("CLS_JIT_RUNTIME").as_deref() {
        Ok("wasmi") => cls_jit::RuntimeKind::Wasmi,
        _ => cls_jit::RuntimeKind::Wasmtime,
    };
    // `CLS_JIT_TRACE=0` -> omitir el shadow call stack (pierde el trace de
    // errores de runtime a cambio de menos código WASM; `clx run --release`).
    let trace_calls = !matches!(std::env::var("CLS_JIT_TRACE").as_deref(), Ok("0"));

    // Manifiesto del proyecto del entry (para límites + snapshot de config).
    let manifest = Path::new(entry)
        .parent()
        .and_then(ModuleManifest::find_in_dir);

    // Sandbox: límite de memoria. Prioridad: env `CLS_MEMORY_LIMIT` >
    // `cls.json` -> `interpreter.runtime.memoryLimit` > default 1 GiB.
    let mem = std::env::var("CLS_MEMORY_LIMIT")
        .ok()
        .and_then(|s| cls_jit::limits::parse_size(&s))
        .or_else(|| {
            manifest
                .as_ref()
                .and_then(|m| cls_jit::limits::parse_size(&m.interpreter.runtime.memory_limit))
        })
        .unwrap_or(cls_jit::limits::DEFAULT_MEMORY_LIMIT);
    cls_jit::limits::set_memory_limit(mem);

    // Sandbox: límite de tiempo. Prioridad: env `CLS_TIME_LIMIT` >
    // `cls.json` -> `interpreter.sandbox.maxExecutionTime` (si > 0) > sin límite.
    // El default del campo es `0` = sin límite, para no cortar corridas largas
    // legítimas; el timeout solo se activa si el proyecto lo declara.
    let time = std::env::var("CLS_TIME_LIMIT")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .or_else(|| {
            manifest
                .as_ref()
                .map(|m| m.interpreter.sandbox.max_execution_time)
                .filter(|t| *t > 0)
        })
        .unwrap_or(0);
    if time > 0 {
        cls_jit::limits::set_time_limit(time);
    }

    // Snapshot de config (solo lectura) expuesto al lenguaje por `config()`.
    cls_jit::runtime_config::set_config(build_config_snapshot(
        target_str,
        runtime,
        trace_calls,
        manifest.as_ref(),
        mem,
        cls_jit::limits::time_limit(),
    ));

    cls_jit::run_jit_with_opts(entry, app_args, target_str, &ctx, runtime, trace_calls)
}

/// Arma el snapshot `(clave, valor)` que el intrinsic `config()` expone al
/// lenguaje. Todas las claves son String (planas, con puntos); los booleanos
/// viajan como `"true"`/`"false"` y las listas de paths unidas por `;`.
fn build_config_snapshot(
    target_str: Option<&str>,
    runtime: cls_jit::RuntimeKind,
    trace_calls: bool,
    manifest: Option<&ModuleManifest>,
    memory_limit_bytes: u64,
    time_limit_ms: u64,
) -> Vec<(String, String)> {
    let m = manifest.cloned().unwrap_or_else(|| ModuleManifest::default_for("project"));
    let sb = &m.interpreter.sandbox;
    let bool_s = |b: bool| if b { "true" } else { "false" }.to_string();
    let join = |v: &[String]| v.join(";");

    let mut out: Vec<(String, String)> = Vec::new();
    macro_rules! push {
        ($k:expr, $v:expr) => {
            out.push(($k.to_string(), $v))
        };
    }

    // project
    push!("project.name", m.name.clone());
    push!("project.version", m.version.clone());
    push!("project.entry", m.entry.clone());
    push!("project.description", m.description.clone());
    push!("project.license", m.license.clone());
    push!("project.registry", m.registry.clone());
    push!("project.sourceDir", m.project.source_dir.clone());
    push!("project.outDir", m.project.out_dir.clone());
    push!("project.target", m.project.target.clone());

    // compiler
    push!("compiler.targetArchitecture", m.compiler.target_architecture.clone());
    push!("compiler.optimizationLevel", m.compiler.optimization_level.clone());
    push!("compiler.types.strict", bool_s(m.compiler.types.strict));
    push!("compiler.types.check", bool_s(m.compiler.types.check));

    // interpreter
    push!("interpreter.mode", m.interpreter.mode.clone());
    push!("interpreter.runtime.memoryLimit", m.interpreter.runtime.memory_limit.clone());
    push!("interpreter.runtime.stackSize", m.interpreter.runtime.stack_size.clone());

    // sandbox
    push!("sandbox.allowFs", bool_s(sb.allow_fs));
    push!("sandbox.allowNet", bool_s(sb.allow_net));
    push!("sandbox.allowProcess", bool_s(sb.allow_process));
    push!("sandbox.allowEnv", bool_s(sb.allow_env));
    push!("sandbox.maxExecutionTime", sb.max_execution_time.to_string());
    push!("sandbox.maxOutputBytes", sb.max_output_bytes.to_string());
    push!("sandbox.maxFileSize", sb.max_file_size.to_string());
    push!("sandbox.readPaths", join(&sb.read_paths));
    push!("sandbox.writePaths", join(&sb.write_paths));
    push!("sandbox.allowedHosts", join(&sb.allowed_hosts));

    // runtime (efectivo)
    push!("runtime.target", target_str.unwrap_or("host").to_string());
    push!("runtime.memoryLimitBytes", memory_limit_bytes.to_string());
    push!("runtime.timeLimitMs", time_limit_ms.to_string());
    let runtime_name = match runtime {
        cls_jit::RuntimeKind::Wasmtime => "wasmtime",
        cls_jit::RuntimeKind::Wasmi => "wasmi",
    };
    push!("runtime.runtime", runtime_name.to_string());
    push!("runtime.traceCalls", bool_s(trace_calls));
    let paths: Vec<String> = cls_jit::extra_module_paths()
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    push!("runtime.pathModules", join(&paths));

    out
}

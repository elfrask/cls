# Arquitectura

CLS es un workspace Rust (edición 2021) con 4 crates de librería y 3 nodos.

## Crates

| Crate | Rol |
|---|---|
| `cls-core` | Frontend (lexer/parser/AST), middleware (typeck/resolver/optimizer), backend (`wasm/`, `json`), config, error, `ansi` |
| `cls-runtime` | Tree-walker deprecado (`walker/`), `Value`, stdlib core, VFS, `error_report`, `.clslib`/`ClsLibIndex`, FFI (`extension`) |
| `cls-jit` | Motor JIT reusable: compile/engine/flatten/host/repl/resolve/state + `wasmtime_rt` / `wasmi_rt` |
| `cls-internals` | Módulos internos e intrinsics **precompilados a WASM** (`cls-internals/wasm/` → `wasm32-unknown-unknown`, embebido con `include_bytes!`); se fusionan dentro del módulo CLS en la emisión (cero imports) |
| `nodos/clx` | CLI de desarrollo + LSP + `maptype` + backend nativo + módulos desktop (`fs`, `http`, `Lib`, `os`, `path`, `process`, `time`, `random`) |
| `nodos/clxb` | Bindings C (`clsb`) - motor de embedding (compile, call, run_main, eval) |
| `nodos/clxr` | Runtime ligero (solo `cls-core` + `cls-runtime`) |

## Pipeline del compilador

```
.clsx -> Lexer -> Parser -> AST -> TypeChecker -> WasmBackend -> WASM -> wasmtime/wasmi
```

El JIT (`cls-jit/src/engine.rs`) orquesta: lectura -> lexer -> parser ->
imports (recursivos) -> caché (`~/.cache/cls/`) -> desplazamiento de spans
(`span_shift`, offset `100000 * n` por módulo) -> typecheck estricto ->
flatten -> emisión WASM -> ejecución (ver `runtime/jit.md`).

### `cls-core` (`cls-core/src/lib.rs`)

Módulos expuestos: `config`, `frontend`, `middleware`, `backend`, `error`,
`ansi`.

- `frontend/`: `lexer.rs`, `parser.rs`, `token.rs`, `span_shift.rs` +
  `ast/` (71 archivos por área: `expressions/`, `statements/`, `cmx/`,
  `types_ann/`, `display.rs`; re-exports en `ast/mod.rs`).
- `middleware/`: `types.rs` (`Type`), `typeck/` (carpeta con 14 archivos:
  `statements.rs`, `expressions.rs`, `types.rs`, `binary.rs`, `calls.rs`,
  `classes.rs`, `containers.rs`, `decls.rs`, `flow.rs`, `helpers.rs`,
  `magics.rs`, `member.rs`, `modules.rs`, `tests.rs` + `mod.rs` con el core
  `TypeChecker`), `resolver.rs` (`NameResolver`), `optimizer.rs` (`Optimizer`).
- `backend/`: `wasm/` (carpeta con feature `wasm-backend`, usa
  `wasm-encoder`): `engine/` (`mod`, `emit`, `functions`, `globals`,
  `metadata`, `fusion`) + `emitter/` (14 archivos por área: statements,
  expressions, binary, calls, classes, strings, member, containers, foreach,
  module_calls, primitives, print, assignment, mod) + `layout.rs`, `types.rs`,
  `host_fn.rs`, `helpers.rs`. También `json.rs` (`JsonBackend`) y `visitor.rs`
  (`AstVisitor`).

**El typeck es la fuente de verdad del emisor**: produce el type map
`Span -> Type` (`types_by_span`) que el `WasmBackend` consume por referencia
sin clonar. El JIT corre el checker con `strict: true`,
`no_implicit_any: true` y `null_safety: true`.

### `WasmBackend` (`cls-core/src/backend/wasm/`)

```rust
pub struct WasmBackendOptions {
    pub exceptions: bool,      // tag + try_table (wasmtime) vs modo sin excepciones (wasmi)
    pub require_main: bool,    // true = debe existir main(args) ; false = modo librería (main no-op)
    pub intrinsics: Vec<HostIntrinsic>, // intrinsics del nodo vía env.host_call
    pub trace_calls: bool,     // true = shadow call stack en memoria lineal (default); false = CLS_JIT_TRACE=0 (pierde el trace de errores)
}
```

Constructores:

| Constructor | Modo |
|---|---|
| `new(types)` | Default (target host) |
| `with_target(types, target)` | Target explícito (para `when`) |
| `with_options(types, target, opts)` | Opciones completas |
| `without_exceptions(types, target)` | Sin excepciones WASM (wasmi) |
| `library_mode(types, target)` | Sin `main` obligatorio (librería) |
| `library_without_exceptions(...)` | Librería + sin excepciones (browser) |

### `cls-jit` (motor agnóstico al nodo)

`cls-jit/src/lib.rs` expone: `compile`, `engine`, `error`, `flatten`, `host`,
`repl`, `resolve`, `state`, `timing`, `wasmtime_rt`, `wasmi_rt` (feature
`wasmi-runtime`).

- `compile.rs` - `compile_file`/`compile_source`/`CompiledModule`/`ExportSig`.
- `engine.rs` - `run_jit`/`run_jit_with`; `RuntimeKind { Wasmtime, Wasmi }`.
- `repl.rs` - `ReplSession`: REPL JIT con estado persistente (cada línea
  compila un módulo nuevo; globals + heap se transfieren entre instancias).
- `host.rs` + `wasmtime_rt.rs` - cuerpos genéricos de las host functions
  `env.*` y `register_host_functions` en el `Linker`; canal
  `env.host_call(id, ptr, n)` para intrinsics del nodo (`HostCallHandler`).
- `resolve.rs` - `cache_dir()`, `load_import_modules`, `module_candidates`.
- El nodo inyecta `JitContext { native_backend, module_index, host_intrinsics,
  host_call_handler, module_source_resolver, output }`.

## Nodos

| Nodo | Dependencias | Notas |
|---|---|---|
| `clx` | `cls-core` (wasm-backend), `cls-runtime`, `cls-jit` (wasmi-runtime), `tokio`/`tower-lsp` (LSP), `libloading` (extensiones dinámicas) | `cargo build --bin clx` |
| `clxr` | `cls-core`, `cls-runtime`, `zip` | Walker deprecado; no lleva JIT |
| `clxb` | core+runtime+jit; lib name `clsb`, `crate-type = ["rlib","cdylib"]` | `cargo build -p clxb` |

## Reglas de arquitectura

1. **Core/runtime agnósticos al entorno** - el nodo inyecta resolvers de
   módulos, internals (`fs`, `http`, ...), VFS y backends nativos. El
   runtime centraliza la carga/ejecución/exports
   (`Interpreter::load_module_source`).
2. **Errores runtime con trace completo** - `build_error_report` ->
   `error_report.rs` (formatos Plain/Console/Html/Json; el nodo elige).
   Typecheck de un solo nivel (ver `runtime/errores.md`).
3. **Colores centralizados** - `cls_core::ansi`.
4. **Rendimiento** - el JIT no boxea ni usa dispatch dinámico en runtime;
   los métodos de primitivos se compilan a llamadas directas a internals/host
   functions; el typeck es la fuente de tipos para la emisión. Las internals
   viven **fusionadas dentro del módulo CLS** (cero imports de internals).
5. **Sin paridad con el walker** - el walker (`cls-runtime/src/walker/`)
   está deprecado (se elimina tras 2.0-dev1) y solo sirve de referencia
   sintáctica.

## Configuración

`cls-core/src/config/`: `manifest.rs` (`ModuleManifest`, camelCase) y
`types.rs` (`TypesConfig`, `CompilerConfig`, `FeaturesConfig`,
`WarningsConfig`, `InterpreterConfig`, `RuntimeMemoryConfig`, `GcConfig`,
`SandboxConfig`). Ver `guia/configuracion.md`.
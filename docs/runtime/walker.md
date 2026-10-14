# El tree-walker (ELIMINADO)

> **ELIMINADO en CLS 2.0-dev-2.** El tree-walker (`cls-runtime/src/walker/`)
> fue un intérprete de referencia sintáctica creado para validar el lenguaje en
> las fases iniciales. Se eliminó por completo: el **JIT** (`clx run`, CLS →
> WASM → wasmtime) es el único intérprete.

## Qué pasó con sus partes

| Componente | Destino |
|---|---|
| `Interpreter` / `Environment` / `execute_*` | Eliminados |
| `Value` | Se conserva en `cls-runtime/src/value.rs` (lo usa el FFI nativo) |
| `ImportFrame` | Se movió a `cls-core/src/error.rs` |
| `load_module_source` (walker) | La carga de módulos la hace el JIT (`flatten.rs`) |
| `nodos/clxr` | Reescrito a JIT-only |
| `clx run --ast-walker` | Se acepta pero **se ignora silenciosamente** (cae al JIT; compatibilidad de scripts) |

## Referencias

- El intérprete actual: [runtime/jit.md](jit.md).
- Los scripts de auditoría comparan el JIT contra oracles `.expect`
  (`examples/test/jit-test/`).

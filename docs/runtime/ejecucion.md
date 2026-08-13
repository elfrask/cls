# Ejecución: walker y JIT

CLS tiene **dos ejecutores**:

| Ejecutor | Comando | Modelo | Estado |
|----------|---------|--------|--------|
| **JIT** (objetivo) | `clx run` (default) | CLS → WASM → wasmtime (Cranelift) | **intérprete objetivo** |
| **Tree-walker** | `clx run --ast-walker` | AST evaluado directo | **DEPRECADO**; solo referencia sintáctica; se eliminará tras CLS 2.0-dev1 |

El JIT compila el AST a un binario WASM con el backend de `cls-core` y lo ejecuta
en wasmtime. El walker recorre el AST y lo evalúa paso a paso. `clx run` usa el
JIT por defecto desde la deprecación del walker; `--jit`/`-j` se aceptan por
compatibilidad sin efecto. `clx run --ast-walker` imprime una advertencia de
deprecación en stderr y ejecuta con el walker.

## Pipeline JIT

```
fuente (String)
  → Lexer → Vec<Token>
  → Parser → Module (AST)
  → Resolver de imports (nodo) → prelude
  → TypeChecker (Span → Type)
  → Backend WASM → Vec<u8>
  → wasmtime (Cranelift) → ejecutar main → exit code
```

Detalle: `agent-context/JIT_COMPILATION.md`, `agent-context/audit/user-use/pipeline.md`.

## Pipeline walker (referencia)

```
fuente (String)
  → Lexer   → Vec<Token>
  → Parser  → Module (AST)
  → Interpreter::execute(&Module) → Value
```

## El `Interpreter` (walker)

El `Interpreter` mantiene el estado de la ejecución:

- **`env`** — el entorno (scopes de variables).
- **`call_stack`** — la pila de llamadas (para el trazo de errores).
- **`import_trace`** — los imports en curso (para el trazo de errores).
- **`flow`** — la señal de flujo (`Normal`, `Return`, `Break`, `Continue`).
- **`self_stack`** — frames `me`/`super` durante llamadas a métodos.
- **`classes`**, **`structs`**, **`method_tables`** — tipos definidos y métodos
  de primitivos.

### Ciclo de vida

1. `Interpreter::new(Intrinsics, ModuleResolver)` — define intrinsics globales
   (`print`, `input`, `type`, `len`, `toString`, etc.) y módulos internos.
2. `interpreter.execute(&module)` — ejecuta las sentencias de nivel superior.
3. `interpreter.call_main()` — llama `main` con los args y devuelve el exit code.

### Cargas de módulos (walker)

`Interpreter::load_module_source(nombre, source)` compila un módulo, lo ejecuta
en un scope aislado, recolecta SOLO los símbolos `export` y devuelve un record.
En el JIT, la carga es distinta: el nodo resuelve los `.clsx` a AST y el backend
los fusiona en un solo WASM (ver `agent-context/audit/user-use/modules.md`).

## Configuración del runtime

La config (modo, límites, sandbox) se pasa con `interpreter.set_config(...)`.

## El JIT (intérprete objetivo)

- Implementación: `nodos/clx/src/jit.rs` (`run_jit`) + `cls-core/src/backend/wasm.rs`.
- Requiere `clx check` interno (typeck) para emitir.
- Errores de runtime: los traps WASM se reportan con el span; el call stack
  completo está en proceso (ver `agent-context/audit/TODO.md`).

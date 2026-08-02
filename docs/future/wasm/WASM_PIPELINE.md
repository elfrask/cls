# Compilación WASM (`.clbin`)

## Objetivo

Compilar código CLS a **WebAssembly** (`.clbin`), el formato de distribución portable
equivalente al `.jar` de Java. Se ejecuta en `clxr` o en cualquier host con runtime WASM.

```
.clsx → Lexer → Parser → TypeChecker → Optimizer → WASM backend → .clbin
```

## Estado actual

- `WasmBackend` existe como placeholder en `cls-core/src/backend/wasm.rs`
- `emit()` devuelve `"WASM codegen no implementado aún"`
- No hay dependencias WASM en los Cargo.toml (candidatos: `walrus`, `wasm-encoder`)

## Diseño del backend

### Enfoque propuesto: IR intermedio → WASM

1. **AST tipado** → IR lineal (instrucciones 3-address, como MIR de Rust)
2. IR → **WASM** (vía `walrus` o `wasm-encoder`)
3. WASM empaquetado en `.clbin`

### Decisiones clave

| Decisión | Opción |
|----------|--------|
| Valores | `int` → `i64`, `float` → `f64`, sin boxear |
| Arrays | `Array<T>` → memoria lineal + longitud |
| `structure` | Layout plano (struct WASM con offsets fijos) |
| Strings | Estilo Rust: `(ptr, len)` en memoria lineal |
| Records | Tabla hash o linear scan (depende del tamaño) |
| GC | **No en el binario** — memoria lineal gestionada por el host o arena |

### Funciones CLS → WASM

```clx
function add(a: int, b: int) -> int { return a + b; }
```

```wasm
(func $add (param $a i64) (param $b i64) (result i64)
  local.get $a
  local.get $b
  i64.add)
```

### Magic methods en WASM

- `__add__`, `__equals__`, `__toString__`, etc.
- Se resuelven en **compile-time** → despacho de tabla de funciones
- No hay lookup de strings en runtime para métodos mágicos

## Pipeline del backend

1. **Analizar**: AST tipado (ya existe `TypeChecker`)
2. **Bajar**: AST → IR lineal
3. **Emitir**: IR → WASM (módulo)
4. **Empaquetar**: WASM + imports/exports → `.clbin`

## Empaquetado `.clbin`

```
.clbin (zip)
├── module.wasm       # el binario WASM
├── manifest.json     # nombre, versión, entry, exports
└── (opcional)        # source map para debugging
```

## Ejecución en `clxr`

`clxr` usa un runtime WASM para ejecutar `.clbin`:
- Candidatos: `wasmtime` (con Cranelift JIT) o `wasmer`
- Exports de CLS se mapean a funciones WASM
- `main` → `_start` o export `main`

## Dependencias a agregar

```toml
# cls-runtime o clx
wasmtime = "16"      # o wasmer
# cls-core (para emitir)
walrus = "0.21"      # o wasm-encoder
```

## Verificación

- `clx build --target wasm` → genera `.clbin`
- `clxr app.clbin` → ejecuta
- Test: los examples actuales compilan y corren igual

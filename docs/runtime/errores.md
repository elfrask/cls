# Sistema de errores

## Regla central

- **Runtime y compilación** (`clx run`, `clxr`, build): **siempre** el trazo
  completo - import_trace + call stack numerado con código fuente por frame +
  el frame del error con caret. Prohibido mostrar solo el mensaje.
- **Typecheck** (`clx check`): errores de **un solo nivel** (archivo
  checado) - `file:línea:col` + línea fuente + caret, sin trace de imports.

## Encabezados y labels

| Tipo | Encabezado | Label |
|---|---|---|
| Sintaxis | `Error en '<file>':` | `[Error de Sintaxis]` |
| Compilación | `Error de Compilación:` | `[Error de Compilación]` |
| Runtime | `Error de ejecución:` | `[Runtime Error]` |
| Tipo | `Error de ejecución:` (via runtime) | `[Error de Tipo]` |

En consola el encabezado y el mensaje `Error:` van en rojo brillante; los
números de la traza en cian; `->` y la función en amarillo; el label en
magenta brillante; el caret del frame del error en rojo (los demás en gris).

## Ejemplo real de salida (runtime)

```
Error de ejecución:

1. En main.clsx:3:14 [Runtime Error]
  3 |     return y / 0;
    |             ^
  Error: División por cero
```

## Formato y construcción

Todo vive en `cls-runtime/src/error_report.rs`:

- `ErrorReport { error, span, stack, import_trace, source_file, source }`.
- `enum ErrorFormat { Plain, Console, Html, Json }` - lo elige el **nodo**.
- `trait ErrorFormatter` + `PlainFormatter` (sin decoradores),
  `ConsoleFormatter` (ANSI de `cls_core::ansi`), `HtmlFormatter`
  (`<pre class="cls-error">`), `JsonFormatter`.
- `format_error(report, format) -> String` produce el string; el nodo decide
  formato y destino.
- `show_runtime_error` / `show_syntax_error` = wrappers que imprimen por
  stderr en formato `Console` (compatibilidad).

`JsonFormatter` emite:

```json
{
  "error": "...",
  "message": "...",
  "file": "main.clsx",
  "span": { "line": 3, "col": 14, "end_line": 3, "end_col": 14 },
  "stack": [ { "function": "main", "file": "main.clsx", "span": {...} } ],
  "imports": [ { "module": "lib", "file": "lib.clsx", "line": 1, "col": 1 } ]
}
```

Decisiones internas:

- `trace_entry` -> `collect_trace` número import_trace y call stack junto con
  el frame del error; cada entrada lee su línea del source (de `source` en
  memoria o del archivo) para mostrar código + caret.
- Tabulaciones -> 4 espacios en el caret (`caret_for`).
- `clean_error_msg` quita el prefijo `Error de X: ` y el `Call stack:`
  embebido de mensajes legacy.

## `ClsError`

`cls-core/src/error/mod.rs`:

```rust
pub enum ClsError {
    CompileError(String),        // "Error de compilación: {0}"
    RuntimeError(String),        // "Error de runtime: {0}"
    TypeError(String),           // "Error de tipo: {0}"
    SyntaxError(String),         // "Error de sintaxis: {0}"
    SyntaxErrorAt(String, Span), // span estructurado (no incrustado)
    CompileErrorAt(String, Span),
    IoError(std::io::Error),
    ConfigError(String),
}
```

Fábricas:

- `ClsError::syntax_at(msg, span)` -> `SyntaxErrorAt` (mensaje limpio, la
  ubicación vive en el `Span`).
- `ClsError::with_span(msg, span)` - alias de `syntax_at`.
- `ClsError::compile_at(msg, span)` -> `CompileErrorAt` (JIT y backend).
- `extract_line_col(msg)` - fallback que extrae `(línea, columna)` de
  mensajes legacy que incrustan el span en el string; lo usa `error_span`
  cuando el error no trae span estructurado.

Parser y lexer usan `self.syntax_err(msg)`; el JIT usa `compile_at` para
"El JIT (subconjunto WASM) aún no soporta...".

## Errores del JIT

- **wasmtime** (default): el backend emite excepciones WASM (tag +
  `try_table`); `throw` y errores de runtime llevan payload
  `(mensaje, span empaquetado)` que el host desempaca para renderizar el
  caret exacto.
- **wasmi** (`CLS_JIT_RUNTIME=wasmi`): sin excepciones - `try/catch`/`throw`
  fallan en compilación y los errores son traps con el mensaje, **sin caret**.
- El call stack CLS vive en la **memoria lineal** del módulo: frames de 12
  bytes (`name_idx:u32, line:u32, col:u32`) escritos por `fn_enter`/`fn_exit`/
  `call_site` como stores WASM inline (0 host calls), en la región
  `[SHADOW_STACK_BASE .. SHADOW_STACK_BASE + 12*1000)`. El host lee la región
  solo en el trap (`read_shadow_trace`) y resuelve `idx → nombre` contra la
  tabla de strings; el stack overflow se detecta y reporta como
  `stack overflow` limpio con los últimos 3 frames.

### Flag `trace_calls` (`CLS_JIT_TRACE=0`)

El shadow call stack se emite como stores WASM en la memoria lineal del módulo
(frames de 12 bytes con el nombre y el call site del llamador). Por defecto está
activo (`trace_calls: true` en `WasmBackendOptions`). Desactivarlo **pierde el
trace de errores de runtime** (el reporte solo muestra el frame del error, sin
`-> main -> outer -> ...`) a cambio de algo menos de código WASM:

```
CLS_JIT_TRACE=0 clx run programa.clsx
```

- Es una opción para releases que no necesitan traza; en desarrollo se
  recomienda dejarlo activo (default).
- La caché CLS→WASM distingue el flag (misma fuente con/ sin trace genera
  módulos distintos).
- `compile_file`/`run_jit_with_opts` aceptan el flag programáticamente
  (`CompileOptions.trace_calls`).

## Typecheck (`clx check`)

Formato de diagnóstico:

```
[ERROR] mensaje (file:line:col)
  line | código fuente
       | ^^^ (caret, ancho del span)
```

- Con `--strict` las asignaciones incompatibles son ERROR (con span real).
- Un archivo sin problemas imprime en verde
  `No se encontraron errores de tipo.`; un directorio imprime el resumen
  `N errores, M advertencias en K archivos` (en `stderr`).
- Los spans de módulos importados se desplazan (offset `100000 * n`) y cada
  diagnóstico se renderiza contra el archivo real del módulo.
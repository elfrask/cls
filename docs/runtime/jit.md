# El JIT (intérprete objetivo)

`clx run` compila CLS a **WASM** y lo ejecuta con **wasmtime** (Cranelift).
Es el intérprete objetivo del proyecto; el tree-walker (`runtime/walker.md`)
está deprecado.

## Pipeline

`cls-jit/src/engine.rs` orquesta el pipeline completo:

1. **Lectura** — se lee el entry desde disco.
2. **Lexer** — `cls_core::frontend::Lexer`.
3. **Parser** — `cls_core::frontend::Parser` (AST).
4. **Imports** — se resuelven `import` / `from import` / `include`
   recursivamente (`load_import_modules_hooked`).
5. **Caché** — se calcula la clave de los fuentes; si el `.wasm` existe en
   `~/.cache/cls/`, se salta directo a la ejecución.
6. **Desplazamiento de spans** — cada módulo importado se fusiona al módulo
   principal con un offset de línea único (`100000 * n`).
7. **Typecheck estricto** — `TypeChecker` con `strict: true`,
   `no_implicit_any: true` y `null_safety: true`. Los errores de tipo abortan
   con diagnóstico + caret.
8. **Flatten** — los imports se aplanan en el módulo único.
9. **Emisión** — `WasmBackend` (`cls-core/src/backend/wasm.rs`) genera el
   binario WASM.
10. **Ejecución** — instancia el módulo en wasmtime (o wasmi) y llama a
    `main(args)`.

## Caché

- Ubicación: `~/.cache/cls/` (HOME/USERPROFILE).
- Clave: hash de la fuente del entry + versión de `cls-core` + target +
  runtime + **fuentes de todos los módulos importados**. Editar cualquier
  `.clsx` del grafo invalida el caché.
- Escritura atómica (archivo temporal + rename); solo se persiste si el
  binario valida.
- `clx clean` vacía el caché. El workspace adicionalmente registra un índice
  de integridad informativo en `[workspace]/.cls-cache/module-index.json`
  (hashes SHA-256 de cada `.clsx`; el JIT no lo usa para invalidar).

## Representación de tipos

| CLS | WASM |
|---|---|
| `Int` | `i64` |
| `Float` | `f64` |
| `Bool` | `i32` (0/1) |
| `Char` | `i32` (codepoint) |
| `String` | `i64` = `(ptr << 32) | len` en la memoria lineal |
| `Array<T>` | `i64` = puntero a `[cap][len][elems...]` |
| `Record`/`Shape`/`Tuple`/`Cmx`/clases/structs/enums | `i64` = puntero |

Memoria: bump allocator (sin free), heap inicial 1 MB tras el string pool,
`memory.grow` según demanda. Los strings de la fuente se internan en un pool
en el data segment.

## Host functions

El módulo WASM importa ~105 funciones `env.*` implementadas en el host
(`cls-jit/src/host.rs` + `wasmtime_rt.rs`):

- Impresión y conversiones (`print_*`, `parse_*`, `str_*`).
- `now`, `exit`, `sleep`, `trap`.
- Arrays y records (`arr_*`, `record_*`).
- Math (`math_sqrt`, `math_pow`, `math_range`, ...), JSON (`json_parse`,
  `json_stringify`).
- Módulos desktop: `fs_*`, `http_*`, `os_*`, `path_*`, `process_*`,
  `time_*`, `random_*`.
- CMX (`cmx_*`) y funciones como valor (`fn_*`).
- `host_call(id, ptr, n)` — canal genérico para intrinsics del nodo.

Los métodos de primitivos (`"hola".upper()`, `arr.push(x)`, ...) se compilan
a llamadas directas a estas host functions (sin objetos ni boxing).

## Excepciones y errores

- **wasmtime** (default): el backend emite una sección de excepciones WASM
  (tag + `try_table`). `throw(msg)` y errores de runtime llevan payload
  `(mensaje, span empaquetado)` que el host desempaca para renderizar el
  caret exacto.
- **wasmi** (`CLS_JIT_RUNTIME=wasmi`): intérprete puro, **sin soporte de
  excepciones** — `try/catch`/`throw` fallan en compilación y los errores son
  traps con el mensaje, sin caret.
- El call stack CLS se mantiene con un "shadow stack" (`fn_enter`/`fn_exit`)
  de hasta 1000 frames; el formateador del runtime lo muestra numerado con
  código fuente por frame (ver `runtime/errores.md`). El stack overflow se
  detecta y reporta limpio.
- Errores de tipo del typeck estricto abortan antes de emitir.

## Límites del subconjunto JIT

El emisor soporta: literales, aritmética, comparaciones, lógicos,
asignaciones (incl. compuestas e `++`/`--`), ternario, `if/elif/else`,
`while`, `loop`, `for`, `for each`, `switch`, `with`, `try/catch` (solo
wasmtime), `return`/`break`/`continue`, arrays, tuplas, records (dinámicos y
con shape), strings e interpolación, CMX, enums, structs, clases (herencia,
visibilidad, static, `super`, `is`, magic `__toString`/`__type`/`__toJson`),
arrow functions con capturas, funciones como valor, `when` (compile-time),
módulos aplanados, `extension` (nativa, ≤ 4 args) y `main`.

Errores explícitos del emisor ("El JIT (subconjunto WASM) aún no soporta...")
para lo que no entra en el subset: tipos `Any`/`Unknown` sin anotar,
parámetros sin anotación de tipo, arrays vacíos sin anotación, `%=` sobre
floats, índices dinámicos en records con shape, `await` (el `async` del
walker no está compilado). El subset se compone de tipos homogéneos: los
arrays heterogéneos se promueven a `Float`, y las uniones colapsan a su tipo
base o a `i64` genérico.

## Debugging

- `CLS_DUMP_WAT=1` — vierte el WAT del módulo a stderr (o lo incluye en el
  error si el módulo no valida).
- `CLS_JIT_TIMING=1` — tiempos por fase (lectura, lexer, parser, imports,
  caché, typeck, flatten, emisión, ejecución).
- `clx ast --json` — inspeccionar el AST previo a la emisión.

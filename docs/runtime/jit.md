# El JIT (intérprete objetivo)

`clx run` compila CLS a **WASM** y lo ejecuta con **wasmtime** (Cranelift).
Es el intérprete objetivo del proyecto; el tree-walker (`runtime/walker.md`)
está deprecado.

## Pipeline

`cls-jit/src/engine.rs` orquesta el pipeline completo:

1. **Lectura** - se lee el entry desde disco.
2. **Lexer** - `cls_core::frontend::Lexer`.
3. **Parser** - `cls_core::frontend::Parser` (AST).
4. **Imports** - se resuelven `import` / `from import` / `include`
   recursivamente (`load_import_modules_hooked`).
5. **Caché** - se calcula la clave de los fuentes; si el `.wasm` existe en
   `~/.cache/cls/`, se salta directo a la ejecución.
6. **Desplazamiento de spans** - cada módulo importado se fusiona al módulo
   principal con un offset de línea único (`100000 * n`).
7. **Typecheck estricto** - `TypeChecker` con `strict: true`,
   `no_implicit_any: true` y `null_safety: true`. Los errores de tipo abortan
   con diagnóstico + caret.
8. **Flatten** - los imports se aplanan en el módulo único.
9. **Emisión** - `WasmBackend` (carpeta `cls-core/src/backend/wasm/`:
   `engine/` y `emitter/`) genera el binario WASM, **fusionando** dentro del
   módulo las internals precompiladas de `cls-internals`.
10. **Ejecución** - instancia el módulo en wasmtime (o wasmi) y llama a
    `main(args)`.

## Caché

- Ubicación: `~/.cache/cls/` (HOME/USERPROFILE).
- Clave: hash de la fuente del entry + versión de `cls-core` + target +
  runtime + **fuentes de todos los módulos importados** + `BACKEND_HASH`
  (build.rs de `cls-core` hashea `src/backend/wasm/` y las fuentes de
  `cls-internals`). Editar cualquier `.clsx` del grafo **o el emisor** invalida
  el caché. El flag `trace_calls` también forma parte de la clave (misma fuente
  con/sin trace genera módulos distintos).
- Escritura atómica (archivo temporal + rename); solo se persiste si el
  binario valida.
- `clx clean` vacía el caché. El workspace adicionalmente registra un índice
  de integridad informativo en `[workspace]/.cls-cache/module-index.json`
  (hashes SHA-256 de cada `.clsx`; el JIT no lo usa para invalidar).

## Internals fusionadas (`cls-internals`)

Los arrays, strings, records, math y conversiones ya **no se importan del
host**: viven precompilados a WASM en el crate `cls-internals` (sub-crate
`cls-internals/wasm/` → `wasm32-unknown-unknown`, embebido con
`include_bytes!`) y el emisor los **fusiona dentro del módulo CLS**
(`engine/fusion.rs`), compartiendo la memoria lineal:

- El emisor llama a `__intr_<area>_<op>` por nombre (`func_indexes`); si una
  internals no está presente cae al **host fallback** con la misma firma.
- El re-mapeo es solo de **índices WASM** (types/funcs/globals/tabla), no de
  direcciones; el import `__cls_alloc` de internals se resuelve al `__alloc`
  del CLS (misma firma `(i64) -> i64`).
- Las direcciones internas del sub-crate quedan **intactas** (ventana fija).
- Verificado: el WAT del módulo no importa `env.arr_*`/`str_*`/`record_*`/
  `math_*`/`parse_*` (paridad 02-strings, 03-tuplas, 04-arrays, 06/09-records,
  suites 25+21 PASS, wasmtime y wasmi).

### Layout de la memoria lineal (`layout.rs`)

```
[0 .. INTERNALS_WINDOW_END)        ventana de internals (1.11MB, direcciones intactas)
[STRING_DATA_BASE .. +512KB)       string pool del CLS (data segment)
[STRING_TABLE_BASE .. +256KB)      tabla de strings (offset, len por entrada)
[HEAP_START ..]                    heap bump (allocator compartido)
[SHADOW_STACK_BASE .. +12*1000)    shadow call stack (trace de errores)
```

Memoria mínima 32 páginas (2MB); `memory.grow` según demanda.

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

Memoria: bump allocator (sin free) en `HEAP_START` (tras la ventana de
internals + string pool + tabla), `memory.grow` según demanda. Los strings de
la fuente se internan en un pool en el data segment (tabla de índices en
`STRING_TABLE_BASE`, datos en `STRING_DATA_BASE`).

## Host functions

El módulo WASM importa **solo las host functions de I/O, errores y nodo**
(~77 imports `env.*`, implementadas en `cls-jit/src/host.rs` +
`wasmtime_rt.rs`/`wasmi_rt.rs`):

- Impresión (`print_*`) y errores (`trap`, `exit`).
- `now`, `sleep`, `host_call(id, ptr, n)` - canal genérico para intrinsics del
  nodo.
- Módulos desktop: `fs_*`, `http_*`, `os_*`, `path_*`, `process_*`, `time_*`,
  `random_*`.
- CMX (`cmx_*`) y funciones como valor (`fn_*`).

Los arrays/strings/records/math/conversiones se **fusionan** desde
`cls-internals` (ver arriba); el `HostFn` correspondiente queda como fallback
con la misma firma. Los métodos de primitivos (`"hola".upper()`,
`arr.push(x)`, ...) se compilan a llamadas directas a internals u host (sin
objetos ni boxing).

## Excepciones y errores

- **wasmtime** (default): el backend emite una sección de excepciones WASM
  (tag + `try_table`). `throw(msg)` y errores de runtime llevan payload
  `(mensaje, span empaquetado)` que el host desempaca para renderizar el
  caret exacto.
- **wasmi** (`CLS_JIT_RUNTIME=wasmi`): intérprete puro, **sin soporte de
  excepciones** - `try/catch`/`throw` fallan en compilación y los errores son
  traps con el mensaje, sin caret.
- El call stack CLS se mantiene en la **memoria lineal** del módulo: frames de
  12 bytes (`name_idx:u32, line:u32, col:u32`) escritos por `fn_enter`/
  `fn_exit`/`call_site` como **stores WASM inline (0 host calls)** en la región
  `[SHADOW_STACK_BASE .. SHADOW_STACK_BASE + 12*1000)`. El host lee la región
  **solo en el trap** (`read_shadow_trace` en wasmtime/wasmi/repl) y resuelve
  `idx → nombre` contra la tabla de strings; el formateador del runtime lo
  muestra numerado con código fuente por frame (ver `runtime/errores.md`). El
  stack overflow se detecta y reporta limpio. `CLS_JIT_TRACE=0` omite el
  shadow stack (pierde el trace, menos código WASM).
- **Dead-flow**: el emisor cierra las funciones con `unreachable` tras
  `return`/`break`/`if` con todas las ramas terminadas y un default de retorno
  para flujo vivo sin `return` (cranelift exige stack balanceado; test
  `examples/audit/test-features/jit-test/units/deadflow.clsx`).
- Errores de tipo del typeck estricto abortan antes de emitir.

## Límites del subconjunto JIT

El emisor soporta: literales, aritmética, comparaciones, lógicos,
asignaciones (incl. compuestas e `++`/`--`), ternario, `if/elif/else`,
`while`, `loop`, `for`, `for each`, `switch`, `with`, `try/catch` (solo
wasmtime), `return`/`break`/`continue`, arrays, tuplas, records (dinámicos y
con shape), strings e interpolación, CMX, enums, structs, clases (herencia,
visibilidad, static, `super`, `is`, **magic methods completos 24/24**:
`__toString`/`__repr`/`__type`/`__toJson`/`__len`/`__int`/`__float`/`__bool`/
`__call`/`__iter`/`__next`/`__get`/`__set`/`__contains`/`__equals`/
`__compare`/`__add`/`__sub`/`__mul`/`__div`/`__mod`/`__pow`/`__neg`/`__not`),
arrow functions con capturas, funciones como valor, `when` (compile-time),
módulos aplanados, `extension` (nativa, ≤ 4 args) y `main`.

**FFI estructurado** (`CRecord`/`CArray`/`CStruct`): los valores viajan como
puntero al layout de la memoria lineal (`array [cap][len][elems*8]`, `record
[cap][len][(key,val,tag)*24]`, `struct` contiguo). El wrapper del JIT traduce el
offset wasm a la dirección host (`ffi_wasm_to_host`/`ffi_host_to_wasm`) para que
el DLL lea/escriba el layout zero-copy; el backend del nodo pasa el ptr como
`Value::Int`. Ver `docs/lenguaje/extension.md` y `examples/jit-examples/`.

Los magic methods se despachan por la **vtable de la clase** con la firma
declarada del método (`call_indirect`): el emisor busca el magic en el tipo
estático del objeto y emite `me` + args + dispatch. Los magics deben **anotar
su retorno** (el JIT no puede tipar `Any`); la iteración usa el protocolo
`__iter` -> Array u objeto iterador con `__next` hasta `null`. Verificado en
`examples/audit/test-features/tests/jit-magic-all.clsx`.

**Truthiness de condiciones** (paridad walker): `if`/`while`/`for`/`elif`
coaccionan la condición a bool - numéricos `!= 0`, strings no vacíos
(`len != 0`), arrays/records/tuplas con elementos (`len` del header), shapes y
objetos siempre verdaderos; tipos sin definir (`Any`) dan error de compilación
claro ("la condición debe ser Bool"). El intrínseco `bool(x)` usa la misma
semántica (`bool(cmx)`, `bool(record)`, ... válidos).

Errores explícitos del emisor ("El JIT (subconjunto WASM) aún no soporta...")
para lo que no entra en el subset: tipos `Any`/`Unknown` sin anotar,
parámetros sin anotación de tipo, arrays vacíos sin anotación, `%=` sobre
floats, índices dinámicos en records con shape, `await` (el `async` del
walker no está compilado). El subset se compone de tipos homogéneos: los
arrays heterogéneos se promueven a `Float`, y las uniones colapsan a su tipo
base o a `i64` genérico.

## Debugging

- `CLS_DUMP_WAT=1` - vierte el WAT del módulo a stderr (o lo incluye en el
  error si el módulo no valida).
- `CLS_JIT_TIMING=1` - tiempos por fase (lectura, lexer, parser, imports,
  caché, typeck, flatten, emisión, ejecución).
- `clx ast --json` - inspeccionar el AST previo a la emisión.

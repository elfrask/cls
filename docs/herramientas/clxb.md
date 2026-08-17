# Bindings C de CLS (`clxb`)

`clxb` es el nodo de **embedding**: expone una ABI C versionada (`clsb_v1_*`)
para embeber CLS en programas escritos en C/C++. Internamente usa el JIT
(CLS -> WASM -> wasmtime), el mismo motor de `clx run`.

## Compilación

```sh
cargo build -p clxb
```

Produce la librería compartida en `target/debug/`:

| Plataforma | Artefacto |
|---|---|
| Windows | `clsb.dll` (+ import lib) |
| Linux | `libclsb.so` |
| macOS | `libclsb.dylib` |

El header es `nodos/clxb/include/clsb.h` (espejo de `nodos/clxb/src/capi.rs`).
Compilar el harness de ejemplo:

```sh
gcc harness.c -I ../include -L ../../target/debug -lclsb -o harness.exe
# copiar clsb.dll junto al exe o agregar el dir al PATH
```

## Tipos opacos

| Tipo | Notas |
|---|---|
| `clsb_engine` | Motor; **single-thread por handle** (no compartir entre threads). |
| `clsb_module` | Módulo compilado (modo librería: `main` opcional). |
| `clsb_error` | Error con trace; las cadenas viven mientras el error. |
| `clsb_config` | `{ enable_fs, enable_http }` - reservado para sandbox. |
| `clsb_status` | `CLSB_OK` = 0; distinto de 0 = error. |

### `clsb_value`

```c
typedef struct clsb_value {
    int32_t tag;
    int64_t bits;              /* int | bits de float | bool 0/1 | char */
    const char* text;          /* CLSB_STRING (owned) */
    struct clsb_value* items;  /* CLSB_ARRAY (owned, n elems) */
    const char** keys;         /* CLSB_RECORD (owned, n claves) */
    struct clsb_value* vals;   /* CLSB_RECORD (owned, n valores) */
    size_t n;                  /* ARRAY: elems · RECORD: entradas */
} clsb_value;
```

Kinds (`tag`, códigos de la custom section `clx:exports`):

| Constante | Valor | Constante | Valor |
|---|---|---|---|
| `CLSB_INT` | 0 | `CLSB_STRING` | 4 |
| `CLSB_FLOAT` | 1 | `CLSB_ARRAY` | 5 |
| `CLSB_BOOL` | 2 | `CLSB_RECORD` | 6 |
| `CLSB_CHAR` | 3 | `CLSB_NULL` | 12 |

## Funciones de la ABI (27 exports)

### Ciclo de vida

```c
clsb_engine* clsb_engine_new(const clsb_config* cfg);   /* cfg puede ser NULL */
void clsb_engine_free(clsb_engine* e);
```

### Compilación

```c
clsb_module* clsb_compile_source(clsb_engine* e, const char* source,
                                 const char* name, const char* base_dir,
                                 clsb_error** err);
clsb_module* clsb_compile_file(clsb_engine* e, const char* path,
                               clsb_error** err);
void clsb_module_free(clsb_module* m);
```

### Ejecución

```c
int64_t clsb_run_main(clsb_module* m, const clsb_value* args, size_t args_len,
                      clsb_error** err);   /* -> exit code de main; -1 si error */
clsb_status clsb_call(clsb_module* m, const char* name,
                      const clsb_value* args, size_t args_len,
                      clsb_value* out, clsb_error** err);
clsb_status clsb_eval(clsb_engine* e, const char* source,
                      clsb_value* out, clsb_error** err);
```

`clsb_call` invoca funciones `export function` del módulo. `clsb_eval` compila
y evalúa un source en caliente.

### SDK de nodo

```c
clsb_status clsb_set_output(clsb_engine* e, clsb_output_cb cb, void* ud);
clsb_status clsb_set_resolver(clsb_engine* e, clsb_resolver_cb cb, void* ud);
clsb_status clsb_register_host_function(clsb_engine* e, const char* name,
                                        const char* sig, clsb_host_fn cb,
                                        void* ud);
```

### Valores

```c
clsb_value clsb_value_null(void);
clsb_value clsb_value_int(int64_t v);
clsb_value clsb_value_float(double v);
clsb_value clsb_value_bool(int v);
clsb_value clsb_value_char(uint32_t v);
clsb_value clsb_value_string(const char* s);
clsb_value clsb_value_array(size_t n);
clsb_value clsb_value_record(size_t n);
void clsb_value_free(clsb_value* v);
```

Los slots de array/record se llenan con setters que toman ownership:
`clsb_value_set_text(v, s)`, `clsb_value_array_set(v, i, val)` y
`clsb_value_record_set(v, i, key, val)`.

### Errores y versión

```c
void clsb_error_free(clsb_error* e);
const char* clsb_error_trace(const clsb_error* e);
const char* clsb_error_message(const clsb_error* e);
const char* clsb_version(void);   /* "clsb 2.0-dev1" (estático) */
```

> **`clsb_error_message` devuelve el trace completo**, igual que
> `clsb_error_trace` (no un mensaje limpio).

## Callbacks

| Callback | Firma | Contrato |
|---|---|---|
| `clsb_output_cb` | `(ud, text, is_end)` | Captura `print` del script: `is_end = 0` (valor) o `1` (fin de línea). |
| `clsb_resolver_cb` | `(ud, path, base_dir, buf, buf_len) -> size_t` | Resuelve `import` no encontrado en disco: escribe el source en `buf` y devuelve su longitud; `0` = no lo conoce. Buffer interno reutilizable de 1 MB. |
| `clsb_host_fn` | `(ud, id, args, args_len, out) -> int` | Función host del nodo; escribe el resultado en `out` (con los constructores). Devuelve `0` = ok. |

Firma de registro de funciones host (códigos): `i` = int, `f` = float,
`b` = bool, `c` = char, `s` = string, `v` = void. Ejemplo: `"i(i,i)"` = función
que recibe dos ints y devuelve int.

## Ciclo típico (del harness)

```c
#include <clsb.h>

static void on_output(void* ud, const char* text, int is_end) { ... }
static int host_duplicar(void* ud, uint32_t id, const clsb_value* args,
                         size_t n, clsb_value* out) {
    *out = clsb_value_int(args[0].bits * 2);
    return 0;
}

int main(void) {
    clsb_error* err = NULL;
    clsb_engine* engine = clsb_engine_new(NULL);
    clsb_module* m = clsb_compile_source(engine,
        "export function suma(a: int, b: int) -> int { return a + b; }\n"
        "function main(args: String[]) -> int { return 0; }\n",
        "harness", ".", &err);

    clsb_value args[2] = { clsb_value_int(20), clsb_value_int(22) };
    clsb_value out = clsb_value_null();
    clsb_status st = clsb_call(m, "suma", args, 2, &out, &err);
    /* out.bits == 42, out.tag == CLSB_INT */

    clsb_set_output(engine, on_output, NULL);
    int64_t code = clsb_run_main(m, NULL, 0, &err);

    clsb_register_host_function(engine, "duplicar", "i(i)", host_duplicar, NULL);

    clsb_value_free(&out);
    clsb_value_free(&args[0]); clsb_value_free(&args[1]);
    clsb_module_free(m);
    clsb_engine_free(engine);
    return 0;
}
```

## Sandbox

`clsb_engine_new` con `enable_fs = 0` **y** `enable_http = 0` (o `cfg = NULL`)
activa el sandbox:

- Los módulos del nodo desktop - `fs`, `http`, `os`, `path`, `process`,
  `time`, `random` - **no se registran**: los imports desconocidos se definen
  como traps (la instanciación es OK; acceder a ellos da error de runtime).
- Solo queda el core: print/math/json/strings.

`exit()`/`trap()` **no matan el proceso del host**: lanzan traps WASM con
mensajes codificados (`__clsb_exit__:<code>` / `__clsb_trap__:<msg>`).
`clsb_run_main` traduce el exit a su código de retorno; llamar `exit()` desde
`clsb_call` devuelve el error `"exit() llamado dentro de una función exportada
(no aplicable)"`.

## Límites

- **Tipos soportados en `clsb_call`**: int, float, bool, char, string, array,
  record, void y null.
- **Extensiones nativas (`extension`) no soportadas**: el backend del engine es
  un dummy (`NoNative`) y devuelve un error claro.
- `clsb_run_main` convierte cada arg a `String` antes de llamar `main`.

## Memoria

- El host **recibe copias** de los valores y errores; los libera con
  `clsb_value_free` (recursivo) y `clsb_error_free`.
- Las cadenas de `clsb_error_trace`/`clsb_error_message` viven mientras el
  `clsb_error`; `clsb_version` es estático y vive toda la vida del proceso.
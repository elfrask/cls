# Bindings de Python (`clsb`)

Paquete Python que embeber CLS vía la ABI C de `clxb` (ver `herramientas/clxb.md`).
Permite compilar source CLS, llamar funciones `export` y registrar funciones
host desde Python.

## Requisitos

- Python >= 3.9.
- La librería nativa `clsb` compilada (`cargo build -p clxb` → `target/debug/clsb.dll`).
- Instalar el paquete: `pip install ./bindings/python` (o agregar la carpeta al `PYTHONPATH`).

La librería nativa se busca en orden:

1. `CLS_LIB_PATH` (variable de entorno, apunta al **directorio** del binario).
2. `clsb/bin/` (junto al paquete).
3. Rutas del sistema (PATH).

## Uso básico

```python
import clsb

engine = clsb.Engine()
engine.set_output(print)                      # print del script → Python
engine.register_host_function("duplicar", "i(i)", lambda fid, a: a[0] * 2)

module = engine.compile_source(
    'export function suma(a: int, b: int) -> int { return a + b; }'
)
print(module.call("suma", 20, 22))            # 42
engine.eval('export function hola() -> String { return "hi"; }')  # "hi"
```

## API

### `Engine`

| Método | Descripción |
|---|---|
| `Engine()` | Crea el motor (sandbox activo por defecto). |
| `set_output(cb)` | `cb(linea)` recibe cada línea que el script imprime con `print`. |
| `set_resolver(cb)` | `cb(path, base_dir) -> str | None` resuelve `import "x"` no encontrado en disco (devuelve el source). |
| `register_host_function(name, sig, fn)` | Registra una función host. `sig` = `ret(params)` con `i/f/b/c/s/v` (ej. `"i(i,i)"`). El callback recibe `(id, args)` y devuelve el valor. |
| `compile_source(source, name="module", base_dir=".")` | Compila source en memoria → `Module`. |
| `compile_file(path)` | Compila desde archivo → `Module`. |
| `eval(source)` | Compila y llama al primer export (o `main`) con 0 args. |

### `Module`

| Método | Descripción |
|---|---|
| `run_main(args=())` | Ejecuta `main(args)`; devuelve el exit code (`int`). |
| `call(name, *args)` | Llama a una función `export` con conversión automática de valores. |

### `ClsError`

Excepción con `message` y `trace` (el trace completo del error CLS).

## Conversión automática de valores

Python → CLS: `int → Int` · `float → Float` · `bool → Bool` · `str → String` ·
`list/tuple → Array` · `dict → Record` · `None → null`. CLS → Python: la
conversión inversa; los `Record` se devuelven como `dict` y los `Array` como
`list`.

## Constantes de tipos

`clsb.CLSB_INT`, `CLSB_FLOAT`, `CLSB_BOOL`, `CLSB_CHAR`, `CLSB_STRING`,
`CLSB_ARRAY`, `CLSB_RECORD`, `CLSB_NULL` (para usuarios avanzados que
inspeccionen `clsb_value` crudos).

## Tests

```sh
$env:CLS_LIB_PATH = "<repo>/target/debug"
python -m unittest tests.test_bindings -v
```

Los tests cubren: `call` con escalares (int/float/bool/string), arrays
(incluido vacío), retorno de `Record`, `run_main`, `eval`, captura de `print`
(varias líneas, con tipos mixtos), host functions, resolver de imports, trace
de errores, errores de sintaxis y `None` ↔ `null`.

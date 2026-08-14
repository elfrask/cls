# Embedding CLS en Python (`clsb`)

El paquete `clsb` embebe el motor CLS (JIT/WASM) en Python vía la ABI C
`clsb_v1_*`. Permite compilar código CLS, llamar funciones exportadas,
ejecutar `main`, evaluar snippets y construir un nodo SDK (resolver, host
functions, captura de `print`).

> Binding en desarrollo (Fase 3 de `agent-context/BINDINGS_PLAN.md`).
> El intérprete objetivo es el JIT; los scripts se compilan a WASM y se
> ejecutan con wasmtime.

## Instalación

```bash
pip install clsb            # wheel con la librería nativa incluida
# o desde el repo (requiere clsb.dll/.so/.dylib en CLS_LIB_PATH o clsb/bin/)
```

La librería se busca en orden: `CLS_LIB_PATH` > `clsb/bin/` (wheel) > PATH.

## Uso básico

```python
import clsb

engine = clsb.Engine()
engine.set_output(print)                    # print del script → Python

module = engine.compile_source(
    'export function suma(a: int, b: int) -> int { return a + b; }'
)
print(module.call("suma", 20, 22))          # 42

engine.eval('export function hola() -> String { return "hi"; }')  # "hi"
```

## Conversión de valores

| CLS | Python |
|-----|--------|
| `int` | `int` |
| `float` | `float` |
| `bool` | `bool` |
| `String` | `str` |
| `Array<T>` | `list` |
| `Record<K,V>` | `dict` |
| `null` | `None` |

## SDK de nodo

```python
def resolver(path, base_dir):
    if path == "virt":
        return 'export function v() -> int { return 9; };'
    return None

def triple(fid, args):
    return args[0] * 3

engine = clsb.Engine()
engine.set_resolver(resolver)               # import "virt" → source
engine.register_host_function("triple", "i(i)", triple)  # CLS→host
engine.compile_source('export function usa() -> int { return triple(5); };')
```

## Sandbox

Por defecto el embedding **no expone** `fs`, `http`, `os`, `path`, `process`,
`time` ni `random` (solo core: `math`, `json`, primitivos). Intentar usarlos
produce un error de runtime. El `exit(n)` de un script devuelve `n` como exit
code de `run_main` sin matar el proceso de Python.

## API

- `Engine()` — motor (un hilo por engine).
- `engine.set_output(cb)` / `set_resolver(cb)` / `register_host_function(name, sig, fn)`.
- `engine.compile_source(src, name?, base_dir?) -> Module` / `compile_file(path)`.
- `engine.eval(src) -> Any`.
- `Module.call(name, *args) -> Any` / `Module.run_main(args?) -> int`.
- `clsb.ClsError` — excepción con `.message` y `.trace` (trace completo).

Ver `bindings/python/` (código) y `bindings/python/tests/test_bindings.py` (tests).

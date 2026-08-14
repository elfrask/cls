# clsb — bindings de CLS para Python

Bindings oficiales de CLS para Python (embedding): compilar código CLS, llamar
funciones exportadas, ejecutar `main`, evaluar snippets y construir un nodo SDK
(resolver, host functions, captura de `print`).

## Instalación

```bash
pip install clsb
```

La librería nativa se busca en `CLS_LIB_PATH`, en `clsb/bin/` (wheel) o en el
PATH. Si la DLL no se encuentra, configura:

```bash
# Windows
set CLS_LIB_PATH=C:\ruta\a\clsb.dll
# Linux/macOS
export CLS_LIB_PATH=/ruta/a/libclsb.so
```

## Ejemplo mínimo

```python
import clsb

engine = clsb.Engine()
module = engine.compile_source(
    'export function suma(a: int, b: int) -> int { return a + b; }'
)
print(module.call("suma", 20, 22))   # 42
```

## Documentación

- Guía de uso y API: `docs/embedding/python.md`
- Plan general de bindings (Fase 3): `agent-context/BINDINGS_PLAN.md`
- Tests: `tests/test_bindings.py`

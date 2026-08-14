"""clsb — bindings de CLS para Python (embedding vía ABI C).

```python
import clsb

engine = clsb.Engine()
engine.set_output(print)                      # print del script → Python
engine.register_host_function("duplicar", "i(i)", lambda id, a: a[0] * 2)

module = engine.compile_source(
    'export function suma(a: int, b: int) -> int { return a + b; }'
)
print(module.call("suma", 20, 22))            # 42
engine.eval('export function hola() -> String { return "hi"; }')  # "hi"
```

Conversión automática de valores:
`int → int` · `float → float` · `bool → bool` · `str → String` ·
`list → Array` · `dict → Record` · `None → null` (y viceversa en retornos).
"""

from __future__ import annotations

import ctypes
from typing import Any, Callable, Optional

from . import _lib
from ._lib import ClsbValue, lib as _lib_handle

__all__ = ["Engine", "Module", "ClsError", "lib", "CLSB_INT", "CLSB_FLOAT",
           "CLSB_BOOL", "CLSB_CHAR", "CLSB_STRING", "CLSB_ARRAY", "CLSB_RECORD",
           "CLSB_NULL"]

# Re-export de los kinds (para usuarios avanzados).
CLSB_INT = _lib.CLSB_INT
CLSB_FLOAT = _lib.CLSB_FLOAT
CLSB_BOOL = _lib.CLSB_BOOL
CLSB_CHAR = _lib.CLSB_CHAR
CLSB_STRING = _lib.CLSB_STRING
CLSB_ARRAY = _lib.CLSB_ARRAY
CLSB_RECORD = _lib.CLSB_RECORD
CLSB_NULL = _lib.CLSB_NULL

from ._lib import lib  # noqa: E402  (carga la librería si se importa)


class ClsError(Exception):
    """Error de CLS con el trace completo."""

    def __init__(self, message: str, trace: str):
        super().__init__(trace)
        self.message = message
        self.trace = trace


def _take_error(err_ptr: Any) -> ClsError:
    """Convierte un clsb_error* (por puntero) en ClsError y lo libera."""
    p = err_ptr.value
    if not p:
        return ClsError("error desconocido", "error desconocido")
    l = _lib_handle()
    trace = l.clsb_error_trace(p) or b""
    msg = l.clsb_error_message(p) or trace
    l.clsb_error_free(p)
    err_ptr.value = None
    return ClsError(msg.decode("utf-8", "replace"), trace.decode("utf-8", "replace"))


# ── marshalling Python ↔ clsb_value ─────────────────────────────────────────

def to_value(x: Any) -> ClsbValue:
    """Convierte un valor Python a clsb_value (memoria del runtime; liberar)."""
    l = _lib_handle()
    if x is None:
        return l.clsb_value_null()
    if isinstance(x, bool):
        return l.clsb_value_bool(1 if x else 0)
    if isinstance(x, int):
        return l.clsb_value_int(x)
    if isinstance(x, float):
        return l.clsb_value_float(x)
    if isinstance(x, str):
        return l.clsb_value_string(x.encode("utf-8"))
    if isinstance(x, (list, tuple)):
        v = l.clsb_value_array(len(x))
        for i, item in enumerate(x):
            child = to_value(item)
            l.clsb_value_array_set(ctypes.byref(v), i, child)
        return v
    if isinstance(x, dict):
        v = l.clsb_value_record(len(x))
        for i, (k, val) in enumerate(x.items()):
            child = to_value(val)
            l.clsb_value_record_set(
                ctypes.byref(v), i, str(k).encode("utf-8"), child
            )
        return v
    raise TypeError(f"tipo no soportado por el binding: {type(x).__name__}")


def from_value(v: ClsbValue) -> Any:
    """Convierte un clsb_value a valor Python (y libera el C recursivamente).

    Los hijos de array/record se LEEN sin liberar (los libera el free recursivo
    del padre): liberarlos aquí además del padre sería doble-free (crash heap).
    """
    l = _lib_handle()
    tag = v.tag
    try:
        if tag == CLSB_INT:
            return v.bits
        if tag == CLSB_FLOAT:
            return ctypes.c_double.from_buffer_copy(ctypes.c_int64(v.bits)).value
        if tag == CLSB_BOOL:
            return v.bits != 0
        if tag == CLSB_CHAR:
            return chr(v.bits) if 0 <= v.bits <= 0x10FFFF else "?"
        if tag == CLSB_STRING:
            return (v.text or b"").decode("utf-8", "replace")
        if tag == CLSB_ARRAY:
            return [_read_value_no_free(v.items[i]) for i in range(v.n)]
        if tag == CLSB_RECORD:
            out: dict[str, Any] = {}
            for i in range(v.n):
                key = (v.keys[i] or b"").decode("utf-8", "replace")
                out[key] = _read_value_no_free(v.vals[i])
            return out
        return None
    finally:
        l.clsb_value_free(ctypes.byref(v))


def _read_value_no_free(v: ClsbValue) -> Any:
    """Lee un clsb_value sin liberarlo (los hijos los libera el padre)."""
    l = _lib_handle()
    tag = v.tag
    if tag == CLSB_INT:
        return v.bits
    if tag == CLSB_FLOAT:
        return ctypes.c_double.from_buffer_copy(ctypes.c_int64(v.bits)).value
    if tag == CLSB_BOOL:
        return v.bits != 0
    if tag == CLSB_CHAR:
        return chr(v.bits) if 0 <= v.bits <= 0x10FFFF else "?"
    if tag == CLSB_STRING:
        return (v.text or b"").decode("utf-8", "replace")
    if tag == CLSB_ARRAY:
        return [_read_value_no_free(v.items[i]) for i in range(v.n)]
    if tag == CLSB_RECORD:
        out: dict[str, Any] = {}
        for i in range(v.n):
            key = (v.keys[i] or b"").decode("utf-8", "replace")
            out[key] = _read_value_no_free(v.vals[i])
        return out
    return None


# ── callbacks ───────────────────────────────────────────────────────────────

class _OutputBridge:
    """Acumula las partes de print y emite líneas al callback Python."""

    def __init__(self, cb: Callable[[str], None]):
        self._cb = cb
        self._buf: list[str] = []

    def _call(self, ud, text: bytes | None, is_end: int) -> None:
        if is_end:
            self._cb("".join(self._buf))
            self._buf = []
        elif text:
            self._buf.append(text.decode("utf-8", "replace"))


class _ResolverBridge:
    def __init__(self, cb: Callable[[str, str], Optional[str]]):
        self._cb = cb

    def _call(self, ud, path: bytes, base: bytes, buf, buf_len: int) -> int:
        try:
            src = self._cb(path.decode("utf-8", "replace"),
                           base.decode("utf-8", "replace"))
        except Exception:
            return 0
        if src is None:
            return 0
        data = src.encode("utf-8")
        if len(data) > buf_len:
            return 0
        ctypes.memmove(buf, data, len(data))
        return len(data)


class _HostFnBridge:
    def __init__(self, cb: Callable[..., Any]):
        self._cb = cb

    def _call(self, ud, fid: int, args_ptr, args_len: int, out_ptr) -> int:
        try:
            args = [_read_value_no_free(args_ptr[i]) for i in range(args_len)]
            result = self._cb(fid, args)
            out = to_value(result)
            out_ptr[0] = out
            return 0
        except Exception as e:
            # Error del callback → la llamada CLS recibe 0 (documentado);
            # el usuario puede re-lanzar en su callback si quiere.
            import sys
            print(f"clsb: host function {fid} falló: {e}", file=sys.stderr)
            return 1


# ── API ─────────────────────────────────────────────────────────────────────

class Engine:
    """Motor de embedding de CLS."""

    def __init__(self):
        self._l = _lib_handle()
        self._h = self._l.clsb_engine_new(None)
        if not self._h:
            raise ClsError("no se pudo crear el engine", "clsb_engine_new falló")
        self._output_cb = None
        self._resolver_cb = None
        self._host_cbs: list = []

    def __del__(self):
        try:
            if getattr(self, "_h", None):
                self._l.clsb_engine_free(self._h)
        except Exception:
            pass

    def set_output(self, cb: Callable[[str], None]) -> None:
        """Captura el `print` del script: cb recibe cada línea."""
        bridge = _OutputBridge(cb)
        self._output_cb = _lib.OUTPUT_CB(bridge._call)
        self._l.clsb_set_output(self._h, self._output_cb, None)

    def set_resolver(self, cb: Callable[[str, str], Optional[str]]) -> None:
        """Resuelve `import "x"` no encontrado en disco: cb(path, base_dir) → source."""
        bridge = _ResolverBridge(cb)
        self._resolver_cb = _lib.RESOLVER_CB(bridge._call)
        self._l.clsb_set_resolver(self._h, self._resolver_cb, None)

    def register_host_function(
        self, name: str, sig: str, fn: Callable[..., Any]
    ) -> None:
        """Registra una función host del nodo. `sig` = ret(params) con
        i=int f=float b=bool c=char s=string v=void (ej. `"i(i,i)"`).
        El callback recibe (id, args) y devuelve el valor."""
        bridge = _HostFnBridge(fn)
        cb = _lib.HOST_FN_CB(bridge._call)
        self._host_cbs.append(cb)  # mantener viva la referencia
        rc = self._l.clsb_register_host_function(
            self._h, name.encode("utf-8"), sig.encode("utf-8"), cb, None
        )
        if rc != 0:
            raise ClsError(f"no se pudo registrar '{name}'", f"sig inválida: {sig}")

    def compile_source(
        self, source: str, name: str = "module", base_dir: str = "."
    ) -> "Module":
        err = ctypes.c_void_p()
        h = self._l.clsb_compile_source(
            self._h, source.encode("utf-8"), name.encode("utf-8"),
            base_dir.encode("utf-8"), ctypes.byref(err),
        )
        if not h:
            raise _take_error(err)
        return Module(self, h)

    def compile_file(self, path: str) -> "Module":
        err = ctypes.c_void_p()
        h = self._l.clsb_compile_file(
            self._h, str(path).encode("utf-8"), ctypes.byref(err)
        )
        if not h:
            raise _take_error(err)
        return Module(self, h)

    def eval(self, source: str) -> Any:
        """Compila y llama al primer export (o main) con 0 args."""
        err = ctypes.c_void_p()
        out = ClsbValue()
        rc = self._l.clsb_eval(
            self._h, source.encode("utf-8"), ctypes.byref(out), ctypes.byref(err)
        )
        if rc != 0:
            raise _take_error(err)
        return from_value(out)


class Module:
    """Módulo CLS compilado e instanciado."""

    def __init__(self, engine: Engine, handle: int):
        self._engine = engine
        self._l = engine._l
        self._h = handle

    def __del__(self):
        try:
            if getattr(self, "_h", None):
                self._l.clsb_module_free(self._h)
        except Exception:
            pass

    def run_main(self, args: list[str] | tuple[str, ...] = ()) -> int:
        """Ejecuta `main(args)` y devuelve el exit code."""
        vals = [to_value(a) for a in args]
        arr = (ClsbValue * max(len(vals), 1))()
        for i, v in enumerate(vals):
            arr[i] = v
        err = ctypes.c_void_p()
        code = self._l.clsb_run_main(
            self._h, arr, len(vals), ctypes.byref(err)
        )
        for v in vals:
            self._l.clsb_value_free(ctypes.byref(v))
        if code == -1 and err.value:
            raise _take_error(err)
        return int(code)

    def call(self, name: str, *args: Any) -> Any:
        """Llama a una función exportada con conversión automática."""
        vals = [to_value(a) for a in args]
        arr = (ClsbValue * max(len(vals), 1))()
        for i, v in enumerate(vals):
            arr[i] = v
        err = ctypes.c_void_p()
        out = ClsbValue()
        rc = self._l.clsb_call(
            self._h, name.encode("utf-8"), arr, len(vals),
            ctypes.byref(out), ctypes.byref(err),
        )
        for v in vals:
            self._l.clsb_value_free(ctypes.byref(v))
        if rc != 0:
            raise _take_error(err)
        return from_value(out)

"""Carga de la librería nativa `clxb` y definición del ABI C (`clsb_v1_*`).

La librería se busca en orden:
1. `CLS_LIB_PATH` (variable de entorno)
2. `clsb/bin/` (junto a este paquete — wheels la llevan)
3. Rutas del sistema (PATH, rutas estándar)

Los valores de `clsb_value` se construyen SIEMPRE con los constructores de la
librería (copian con el runtime) y se liberan con `clsb_value_free`.
"""

from __future__ import annotations

import ctypes
import os
import sys
from pathlib import Path

# ── kinds (códigos de la custom section clx:exports) ────────────────────────
CLSB_INT = 0
CLSB_FLOAT = 1
CLSB_BOOL = 2
CLSB_CHAR = 3
CLSB_STRING = 4
CLSB_ARRAY = 5
CLSB_RECORD = 6
CLSB_NULL = 12


class ClsbConfig(ctypes.Structure):
    _fields_ = [
        ("enable_fs", ctypes.c_int),
        ("enable_http", ctypes.c_int),
    ]


class ClsbValue(ctypes.Structure):
    pass


ClsbValue._fields_ = [
    ("tag", ctypes.c_int32),
    ("bits", ctypes.c_int64),
    ("text", ctypes.c_char_p),
    ("items", ctypes.POINTER(ClsbValue)),
    ("keys", ctypes.POINTER(ctypes.c_char_p)),
    ("vals", ctypes.POINTER(ClsbValue)),
    ("n", ctypes.c_size_t),
]

P_VALUE = ctypes.POINTER(ClsbValue)
P_ERROR = ctypes.c_void_p

# ── callbacks ────────────────────────────────────────────────────────────────
OUTPUT_CB = ctypes.CFUNCTYPE(None, ctypes.c_void_p, ctypes.c_char_p, ctypes.c_int)
RESOLVER_CB = ctypes.CFUNCTYPE(
    ctypes.c_size_t,
    ctypes.c_void_p,
    ctypes.c_char_p,
    ctypes.c_char_p,
    ctypes.POINTER(ctypes.c_char),
    ctypes.c_size_t,
)
HOST_FN_CB = ctypes.CFUNCTYPE(
    ctypes.c_int,
    ctypes.c_void_p,
    ctypes.c_uint32,
    P_VALUE,
    ctypes.c_size_t,
    P_VALUE,
)


class ClsLibraryError(RuntimeError):
    """No se pudo cargar la librería nativa de CLS."""


def _candidates() -> list[Path]:
    names = {
        "win32": ["clsb.dll"],
        "darwin": ["libclsb.dylib", "clsb.dylib"],
    }.get(sys.platform, ["libclsb.so", "clsb.so"])

    cands: list[Path] = []
    env = os.environ.get("CLS_LIB_PATH")
    if env:
        cands.extend(Path(env) / n for n in names)
    pkg_dir = Path(__file__).resolve().parent
    cands.extend(pkg_dir / "bin" / n for n in names)
    for n in names:
        cands.append(Path(n))
    return cands


_lib: ctypes.CDLL | None = None


def lib() -> ctypes.CDLL:
    """Devuelve la librería cargada (singleton)."""
    global _lib
    if _lib is not None:
        return _lib
    last: Exception | None = None
    for cand in _candidates():
        if not cand.exists():
            continue
        try:
            _lib = ctypes.CDLL(str(cand))
            _configure(_lib)
            return _lib
        except OSError as e:
            last = e
            continue
    raise ClsLibraryError(
        "No se encontró la librería de CLS (clsb.dll/.so/.dylib). "
        "Configura CLS_LIB_PATH o coloca el binario en clsb/bin/. "
        f"Buscado: {[str(c) for c in _candidates()]} (último error: {last})"
    )


def _configure(l: ctypes.CDLL) -> None:
    l.clsb_engine_new.restype = ctypes.c_void_p
    l.clsb_engine_new.argtypes = [ctypes.POINTER(ClsbConfig)]
    l.clsb_engine_free.argtypes = [ctypes.c_void_p]

    l.clsb_compile_source.restype = ctypes.c_void_p
    l.clsb_compile_source.argtypes = [
        ctypes.c_void_p, ctypes.c_char_p, ctypes.c_char_p, ctypes.c_char_p,
        ctypes.POINTER(ctypes.c_void_p),
    ]
    l.clsb_compile_file.restype = ctypes.c_void_p
    l.clsb_compile_file.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.POINTER(ctypes.c_void_p)]
    l.clsb_module_free.argtypes = [ctypes.c_void_p]

    l.clsb_run_main.restype = ctypes.c_int64
    l.clsb_run_main.argtypes = [ctypes.c_void_p, P_VALUE, ctypes.c_size_t, ctypes.POINTER(ctypes.c_void_p)]
    l.clsb_call.restype = ctypes.c_int
    l.clsb_call.argtypes = [
        ctypes.c_void_p, ctypes.c_char_p, P_VALUE, ctypes.c_size_t,
        P_VALUE, ctypes.POINTER(ctypes.c_void_p),
    ]
    l.clsb_eval.restype = ctypes.c_int
    l.clsb_eval.argtypes = [ctypes.c_void_p, ctypes.c_char_p, P_VALUE, ctypes.POINTER(ctypes.c_void_p)]

    l.clsb_set_output.restype = ctypes.c_int
    l.clsb_set_output.argtypes = [ctypes.c_void_p, OUTPUT_CB, ctypes.c_void_p]
    l.clsb_set_resolver.restype = ctypes.c_int
    l.clsb_set_resolver.argtypes = [ctypes.c_void_p, RESOLVER_CB, ctypes.c_void_p]
    l.clsb_register_host_function.restype = ctypes.c_int
    l.clsb_register_host_function.argtypes = [
        ctypes.c_void_p, ctypes.c_char_p, ctypes.c_char_p, HOST_FN_CB, ctypes.c_void_p,
    ]

    l.clsb_value_null.restype = ClsbValue
    l.clsb_value_int.restype = ClsbValue
    l.clsb_value_int.argtypes = [ctypes.c_int64]
    l.clsb_value_float.restype = ClsbValue
    l.clsb_value_float.argtypes = [ctypes.c_double]
    l.clsb_value_bool.restype = ClsbValue
    l.clsb_value_bool.argtypes = [ctypes.c_int]
    l.clsb_value_char.restype = ClsbValue
    l.clsb_value_char.argtypes = [ctypes.c_uint32]
    l.clsb_value_string.restype = ClsbValue
    l.clsb_value_string.argtypes = [ctypes.c_char_p]
    l.clsb_value_array.restype = ClsbValue
    l.clsb_value_array.argtypes = [ctypes.c_size_t]
    l.clsb_value_record.restype = ClsbValue
    l.clsb_value_record.argtypes = [ctypes.c_size_t]
    l.clsb_value_free.argtypes = [P_VALUE]
    l.clsb_value_set_text.argtypes = [P_VALUE, ctypes.c_char_p]
    l.clsb_value_array_set.argtypes = [P_VALUE, ctypes.c_size_t, ClsbValue]
    l.clsb_value_record_set.argtypes = [P_VALUE, ctypes.c_size_t, ctypes.c_char_p, ClsbValue]

    l.clsb_error_free.argtypes = [ctypes.c_void_p]
    l.clsb_error_trace.restype = ctypes.c_char_p
    l.clsb_error_trace.argtypes = [ctypes.c_void_p]
    l.clsb_error_message.restype = ctypes.c_char_p
    l.clsb_error_message.argtypes = [ctypes.c_void_p]
    l.clsb_version.restype = ctypes.c_char_p

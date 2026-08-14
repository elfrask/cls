"""Tipos estáticos del binding clsb (para mypy/type checkers)."""

from typing import Any, Callable, Optional, Union

__all__ = ["Engine", "Module", "ClsError", "lib"]

Value = Union[int, float, bool, str, list, dict, None]

CLSB_INT: int
CLSB_FLOAT: int
CLSB_BOOL: int
CLSB_CHAR: int
CLSB_STRING: int
CLSB_ARRAY: int
CLSB_RECORD: int
CLSB_NULL: int


class ClsError(Exception):
    message: str
    trace: str

    def __init__(self, message: str, trace: str) -> None: ...


class Engine:
    def __init__(self) -> None: ...
    def set_output(self, cb: Callable[[str], None]) -> None: ...
    def set_resolver(self, cb: Callable[[str, str], Optional[str]]) -> None: ...
    def register_host_function(self, name: str, sig: str, fn: Callable[..., Any]) -> None: ...
    def compile_source(self, source: str, name: str = "module", base_dir: str = ".") -> "Module": ...
    def compile_file(self, path: str) -> "Module": ...
    def eval(self, source: str) -> Any: ...


class Module:
    def run_main(self, args: Optional[list[str]] = None) -> int: ...
    def call(self, name: str, *args: Any) -> Any: ...


def lib() -> Any: ...

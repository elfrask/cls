"""Tests del binding Python de CLS (F3) - paridad con el harness C.

Correr:
    $env:CLS_LIB_PATH = "<repo>/target/debug"
    python -m unittest tests.test_bindings -v
"""

from __future__ import annotations

import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

import clsb  # noqa: E402

SRC = """
export function suma(a: int, b: int) -> int { return a + b; }
export function saludo(n: String) -> String { return "hola " + n; }
export function doble_f(x: float) -> float { return x * 2.0; }
export function mayor(a: int, b: int) -> bool { return a > b; }
export function total(ns: int[]) -> int { var t: int = 0; for each n in (ns) { t += n; } return t; }
export function datos() -> Record<String, String> { var d: Record<String, String> = {a: "1", b: "2"}; return d; }
export function datos_anidado() -> Record<String, int[]> { var d: Record<String, int[]> = {x: [1, 2, 3]}; return d; }
function main(args: String[]) -> int { print("main:", args[0]); return 0; }
"""


class TestBindings(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.engine = clsb.Engine()
        cls.module = cls.engine.compile_source(SRC, "tests")

    def test_call_escalares(self):
        m = self.module
        self.assertEqual(m.call("suma", 20, 22), 42)
        self.assertEqual(m.call("doble_f", 2.5), 5.0)
        self.assertIs(m.call("mayor", 5, 3), True)
        self.assertEqual(m.call("saludo", "mundo"), "hola mundo")

    def test_call_arrays(self):
        self.assertEqual(self.module.call("total", [1, 2, 3]), 6)
        self.assertEqual(self.module.call("total", []), 0)

    def test_call_record_retorno(self):
        self.assertEqual(self.module.call("datos"), {"a": "1", "b": "2"})

    def test_call_record_con_array_anidado(self):
        self.assertEqual(self.module.call("datos_anidado"), {"x": [1, 2, 3]})

    def test_run_main(self):
        code = self.module.run_main(["hola"])
        self.assertEqual(code, 0)

    def test_eval(self):
        self.assertEqual(self.engine.eval('export function siete() -> int { return 7; };'), 7)

    def test_output_capturado(self):
        engine = clsb.Engine()
        lines: list[str] = []
        engine.set_output(lambda line: lines.append(line))
        module = engine.compile_source(
            'function main(args: String[]) -> int { print("a", 1, 2.5); print("segunda"); return 0; };'
        )
        module.run_main([])
        self.assertEqual(lines, ["a 1 2.5", "segunda"])

    def test_host_function(self):
        engine = clsb.Engine()
        engine.register_host_function("duplicar", "i(i)", lambda fid, args: args[0] * 2)
        module = engine.compile_source(
            'export function usa() -> int { return duplicar(21); };'
        )
        self.assertEqual(module.call("usa"), 42)

    def test_resolver(self):
        engine = clsb.Engine()
        engine.set_resolver(lambda path, base: None if path != "virt" else 'export function v() -> int { return 9; };')
        module = engine.compile_source(
            'import "virt" as v; export function usa() -> int { return v::v(); };'
        )
        self.assertEqual(module.call("usa"), 9)

    def test_error_trace(self):
        with self.assertRaises(clsb.ClsError) as ctx:
            self.module.call("no_existe")
        self.assertIn("no_existe", ctx.exception.trace)

    def test_error_sintaxis(self):
        with self.assertRaises(clsb.ClsError):
            self.engine.compile_source("function main( {")

    def test_none_y_null(self):
        m = self.engine.compile_source(
            'export function nada() { return; }'
        )
        self.assertIsNone(m.call("nada"))


if __name__ == "__main__":
    unittest.main()

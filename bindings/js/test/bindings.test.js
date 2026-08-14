"use strict";

const { test } = require("node:test");
const assert = require("node:assert");
const clsb = require("../src/index.js");

test("version", () => {
  assert.ok(clsb.version().includes("clsb"));
});

test("call escalares y string", () => {
  const e = new clsb.Engine();
  const m = e.compileSource(
    "export function suma(a: int, b: int) -> int { return a + b; }"
  );
  assert.strictEqual(m.call("suma", 20, 22), 42);
  const m2 = e.compileSource(
    'export function hola() -> String { return "hi"; }'
  );
  assert.strictEqual(m2.call("hola"), "hi");
  e.dispose();
});

test("eval", () => {
  const e = new clsb.Engine();
  assert.strictEqual(e.eval("export function x() -> int { return 7; }"), 7);
  e.dispose();
});

test("run_main con args", () => {
  const e = new clsb.Engine();
  const m = e.compileSource(
    "function main(args: String[]) -> int { print(args.length); return args.length; }"
  );
  assert.strictEqual(m.runMain(["a", "b", "c"]), 3);
  e.dispose();
});

test("print capturado", () => {
  const e = new clsb.Engine();
  const lines = [];
  e.setOutput((line) => lines.push(line));
  const m = e.compileSource(
    'function main(args: String[]) -> int { print("hola", 42); print("mundo"); return 0; }'
  );
  m.runMain([]);
  assert.deepStrictEqual(lines, ["hola 42", "mundo"]);
  e.dispose();
});

test("arrays y records", () => {
  const e = new clsb.Engine();
  const m = e.compileSource(
    "export function join_strs(xs: String[]) -> String { var s: String = \"\"; for each x in (xs) { s += x; } return s; }"
  );
  assert.strictEqual(m.call("join_strs", ["uno", "dos", "tres"]), "unodostres");
  const m2 = e.compileSource(
    "export function datos() -> Record<String, int> { return {a: 1, b: 2}; }"
  );
  assert.deepStrictEqual(m2.call("datos"), { a: 1, b: 2 });
  e.dispose();
});

test("host functions multiples", () => {
  const e = new clsb.Engine();
  e.registerHostFunction("alpha", "i(i)", (id, args) => args[0] * 3);
  e.registerHostFunction("beta", "i(i)", (id, args) => args[0] * 5);
  const m = e.compileSource(
    "export function usa() -> int { return alpha(2) + beta(2); }"
  );
  assert.strictEqual(m.call("usa"), 16);
  e.dispose();
});

test("resolver virtual", () => {
  const e = new clsb.Engine();
  e.setResolver((path) =>
    path === "virt" ? "export function v() -> int { return 9; }" : null
  );
  const m = e.compileSource(
    'import "virt" as v; export function usa() -> int { return v::v(); }'
  );
  assert.strictEqual(m.call("usa"), 9);
  e.dispose();
});

test("error con trace", () => {
  const e = new clsb.Engine();
  assert.throws(
    () => e.compileSource("function main( {"),
    (err) => err instanceof clsb.ClsError
  );
  e.dispose();
});

test("sandbox: fs bloqueado", () => {
  const e = new clsb.Engine();
  const m = e.compileSource(
    "function main(args: String[]) -> int { var c = fs.cwd(); print(c); return 0; }"
  );
  assert.throws(() => m.runMain([]), clsb.ClsError);
  e.dispose();
});

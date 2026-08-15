"use strict";
const clsb = require("../src/index.js");

let pass = 0, fail = 0;
function check(label, cond, detail) {
  if (cond) { pass++; console.log("ok:   " + label); }
  else { fail++; console.log("FAIL: " + label + (detail ? " -> " + detail : "")); }
}

function tryRun(label, fn, expectError) {
  try {
    const r = fn();
    check(label, expectError === false || expectError === undefined, "sin error, resultado=" + JSON.stringify(r));
  } catch (e) {
    check(label, expectError === true, "error=" + String(e).slice(0, 150));
  }
}

// host fns multiples
{
  const e = new clsb.Engine();
  e.registerHostFunction("alpha", "i(i)", (id, args) => args[0] * 3);
  e.registerHostFunction("beta", "i(i)", (id, args) => args[0] * 5);
  const m = e.compileSource("export function usa() -> int { return alpha(2) + beta(2); }");
  check("host fns multiples -> 16", m.call("usa") === 16);
  e.dispose();
}
// host fn string
{
  const e = new clsb.Engine();
  e.registerHostFunction("saluda", "s(s)", (id, args) => "hola " + args[0]);
  const m = e.compileSource('export function f() -> String { return saluda("mundo"); }');
  check("host fn string -> hola mundo", m.call("f") === "hola mundo");
  e.dispose();
}
// record con literal
{
  const e = new clsb.Engine();
  const m = e.compileSource("export function datos() -> Record<String, int> { return {a: 1, b: 2}; }");
  const r = m.call("datos");
  check("record literal -> {a:1,b:2}", JSON.stringify(r) === JSON.stringify({ a: 1, b: 2 }), JSON.stringify(r));
  e.dispose();
}
// resolver
{
  const e = new clsb.Engine();
  e.setResolver((path) => path === "virt" ? "export function v() -> int { return 9; }" : null);
  const m = e.compileSource('import "virt" as v; export function usa() -> int { return v::v(); }');
  check("resolver virtual -> 9", m.call("usa") === 9);
  e.dispose();
}
// div por cero: debe dar error con mensaje (no "Trap WASM")
{
  const e = new clsb.Engine();
  const m = e.compileSource("export function div(a: int, b: int) -> int { return a / b; }");
  tryRun("div por cero -> error", () => m.call("div", 10, 0), true);
  e.dispose();
}
// sandbox fs
{
  const e = new clsb.Engine();
  const m = e.compileSource("function main(args: String[]) -> int { var c = fs.cwd(); print(c); return 0; };");
  tryRun("sandbox fs.cwd -> error", () => m.runMain([]), true);
  e.dispose();
}
// exit no debe matar el proceso
{
  const e = new clsb.Engine();
  const m = e.compileSource("function main(args: String[]) -> int { exit(7); return 0; };");
  tryRun("exit(7) controlado -> error o codigo", () => m.runMain([]), true);
  e.dispose();
}
// error runtime en modulo importado con trace
{
  const e = new clsb.Engine();
  const m = e.compileSource("function main(args: String[]) -> int { var x = 1 / 0; return 0; };");
  tryRun("runtime error en main -> error", () => m.runMain([]), true);
  e.dispose();
}
// aridad incorrecta en modulo interno: no debe crashear el proceso
{
  const e = new clsb.Engine();
  tryRun("random.float() aridad -> error", () => e.eval("export function f() -> float { return random.float(); }"), false);
  e.dispose();
}
// typecheck TS
{
  const ts = require("typescript");
  const fs = require("fs");
  const src = fs.readFileSync("types/types.test.ts", "utf8");
  const res = ts.transpileModule(src, { compilerOptions: { noEmit: true, strict: true } });
  check("tsc transpile sin error", !res.diagnostics || res.diagnostics.length === 0);
}
console.log(`\n${pass} checks, ${fail} fails`);
process.exit(fail === 0 ? 0 : 1);

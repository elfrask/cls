// Typecheck del paquete: valida la API del binding con los tipos TS.
import {
  Engine,
  Module,
  ClsError,
  ClsValue,
  EngineOptions,
  version,
  CLSB_INT,
} from "../src/index.js";
const opts: EngineOptions = { fs: false, http: false };
const e: Engine = new Engine(opts);
const v: string = version();
const v2: string = version();

e.setOutput((line: string) => void line);
e.setResolver((path: string, baseDir: string) => (path === "m" ? "export function f() -> int { return 1; }" : null));
e.registerHostFunction("mul", "i(i)", (id: number, args: ClsValue[]) => (args[0] as number) * 2);

const m: Module = e.compileSource("export function f() -> int { return 1; }");
const r: ClsValue = m.call("f");
const code: number = m.runMain(["a"]);
m.dispose();

const ev: ClsValue = e.eval("1 + 1");
const c: number = CLSB_INT;
const err = new ClsError("m", "t");
const isErr: Error = err;
e.dispose();

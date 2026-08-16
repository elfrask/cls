# Embedding CLS en Node.js (`@cls-embed/node`)

El paquete `@cls-embed/node` embebe el motor CLS (JIT/WASM) en Node.js vía la
ABI C `clsb_v1_*` (con [koffi](https://koffi.dev)). Permite compilar código
CLS, llamar funciones exportadas, ejecutar `main`, evaluar snippets y
construir un nodo SDK (resolver, host functions, captura de `print`).

> Binding funcional (Fase 4 de `agent-context/BINDINGS_PLAN.md`): suite de
> tests 10/10 y typecheck TS verificados. El intérprete objetivo es el JIT;
> los scripts se compilan a WASM y se ejecutan con wasmtime.

## Instalación

```bash
npm install @cls-embed/node        # paquete con la librería nativa incluida
# o desde el repo (requiere clsb.dll/.so/.dylib en CLS_LIB_PATH o bindings/js/lib/)
```

La librería se busca en orden: `CLS_LIB_PATH` > `lib/` (junto al paquete) > PATH.

## Uso básico

```js
const clsb = require("@cls-embed/node");

const engine = new clsb.Engine();
engine.setOutput(console.log);      // print del script -> Node

const module = engine.compileSource(
  "export function suma(a: int, b: int) -> int { return a + b; }"
);
console.log(module.call("suma", 20, 22));   // 42

engine.eval('export function hola() -> String { return "hi"; }');  // "hi"
```

## Conversión de valores

| CLS | JS |
|-----|-----|
| `int` | `number` (integer) |
| `float` | `number` |
| `bool` | `boolean` |
| `String` | `string` |
| `Array<T>` | `Array<ClsValue>` |
| `Record<K,V>` | `object` (claves string) |
| `null` | `null` |

## SDK de nodo

```js
engine.setResolver((path, baseDir) => {
  if (path === "virt") return 'export function v() -> int { return 9; };';
  return null;                        // -> error "módulo no encontrado"
});

engine.registerHostFunction("triple", "i(i)", (id, args) => args[0] * 3);
engine.compileSource("export function usa() -> int { return triple(5); };");
```

## Sandbox

Por defecto el embedding **no expone** `fs`, `http`, `os`, `path`, `process`,
`time` ni `random` (solo core: `math`, `json`, primitivos). Intentar usarlos
produce un error de runtime (trap WASM). Para habilitar módulos del sistema:

```js
const engine = new clsb.Engine({ fs: true, http: true });
```

El `exit(n)` de un script devuelve `n` como exit code de `run_main` sin matar
el proceso de Node.

## API

- `new Engine(opts?)` - motor (un hilo por engine). `opts.fs` / `opts.http`
  (`boolean`, default `false`) habilitan los módulos del sistema.
- `engine.version` - versión de la ABI (`clsb_version`).
- `engine.setOutput(cb(line))` / `engine.setResolver(cb(path, baseDir) -> string|null)`
  / `engine.registerHostFunction(name, sig, fn(id, args) -> ClsValue)`.
- `engine.compileSource(src, name?, baseDir?) -> Module` / `engine.compileFile(path)`.
- `engine.eval(src) -> ClsValue`.
- `Module.call(name, ...args) -> ClsValue` / `Module.runMain(args?) -> int`.
- `Module.dispose()` / `Engine.dispose()` - libera los handles nativos.
- `clsb.ClsError` - error con `.message` y `.trace` (trace completo).
- Constantes de kind: `CLSB_INT`, `CLSB_FLOAT`, `CLSB_BOOL`, `CLSB_CHAR`,
  `CLSB_STRING`, `CLSB_ARRAY`, `CLSB_RECORD`, `CLSB_NULL`.

## Tests

```bash
npm test          # suite de integración (node --test)
npm run typecheck # valida el .d.ts con tsc --noEmit
```

Ver `bindings/js/` (código), `bindings/js/test/bindings.test.js` (tests) y
`bindings/js/types/types.test.ts` (typecheck).

# `examples/lang-features/` — matriz de features del lenguaje CLS 2.0

Prueba de **cobertura del lenguaje completo** (ver `examples/patrones.md` §5.6).
Ejercita cada eje del lenguaje con salida **determinista** y la compara contra un
**golden** (`expected.txt`), de modo que cualquier **regresión** rompa el runner.
Además, cada **repro de bug** vive en `findings/` y se corre en cada ejecución.

- **Motor**: JIT (`clx run`, default). No hay tree-walker.
- **Autocontención**: solo CLS + FFI + `print`/`input` + los módulos de
  `/modules` (`check`, `bytes`, `prng`). **No** se usan los `host_call`
  `fs`/`http`/`json`/`strings` (§2.7). El shim FFI propio es `ffi/langffi_shim.c`.

## Estructura

```
examples/lang-features/
├── cls.json                     # entry + pathModules:["../../modules"]
├── run.ps1                      # orquestador: matriz + negativos + findings
├── expected.txt                 # golden (salida completa de la matriz)
├── src/
│   ├── main.clsx                # corre todas las secciones y resume
│   ├── sections/
│   │   ├── control.clsx         # if/elif/else, switch, while, loop, for, each, with, try
│   │   ├── operators.clsx       # aritmeticos, bit a bit, comparacion, logicos, ++/--, spread, in, is
│   │   ├── types.clsx           # alias, uniones, const, tuplas, records, interfaces, extraccion, genericos
│   │   ├── oop.clsx             # clases, herencia, super, visibilidad, me, is, magics
│   │   ├── enums.clsx           # variantes, ==, is, iteracion
│   │   ├── structs.clsx         # structure, instanciacion posicional, campos
│   │   ├── functions.clsx       # arrow, closures, callable, recursion, genericos
│   │   ├── strings.clsx         # interpolacion, templates, metodos, Char buffers
│   │   ├── cmx.clsx             # tags, props, children, spread, is Callable
│   │   ├── when.clsx            # when compile-time (os/arch/platform/abi, or/not, default)
│   │   ├── modules.clsx         # import as, from import as, re-export, /modules
│   │   ├── errors.clsx          # throw/catch, div por cero, indice fuera de rango, propagacion
│   │   └── extension.clsx       # FFI (when + extension)
│   └── lib/
│       ├── util.clsx            # modulo local (export function/var/const/class/enum)
│       └── index.clsx           # barrel (re-export ns::sym as + from)
├── ffi/
│   ├── langffi_shim.c           # shim C minimo (add/mul/clamp/strlen/half/hypot)
│   ├── langffi_shim.dll         # compilado por run.ps1 (gcc)
│   └── build.ps1
├── negative/                    # accesos invalidos que DEBEN rechazarse
└── findings/                    # solo GAPs documentados (los repros resueltos
                                 #   se promovieron a src/sections + expected.txt)
```

## Cómo correr

Desde la **raíz del repo** (el path del `.dll` FFI se resuelve relativo al CWD):

```powershell
target\debug\clx.exe run examples\lang-features\src\main.clsx --add-path-module=modules
powershell -File examples\lang-features\run.ps1
```

`run.ps1` (`-Update` regenera `expected.txt`):

1. compila `ffi/langffi_shim.dll` si falta o está viejo (requiere `gcc`).
2. corre la **matriz** y compara byte-a-byte con `expected.txt`.
3. corre `negative/*.clsx` — cada uno **debe** ser rechazado (exit ≠ 0).
4. corre `findings/*.clsx` — cada uno **debe** fallar (exit ≠ 0 = bug presente).
5. **exit ≠ 0** si la matriz diverge, si un negativo no se rechaza o si quedan
   findings abiertos.

## Matriz de features

`OK` = verificado por la matriz (`expected.txt`). `GAP` = límite del
lenguaje/JIT documentado en `findings/gaps.txt` (no es regresión). No hay features
en `FAIL`.

| Eje | Feature | Estado | Detalle |
|---|---|---|---|
| control | `if`/`elif`/`else` | OK | |
| control | `switch`/`case`/`case default` | OK | |
| control | `while` | OK | |
| control | `loop` + `break` | OK | |
| control | `for` clásico (`++`, `continue`, `break`) | OK | |
| control | `for each` / `for each … and idx` | OK | |
| control | `with … in (…)` | OK (parcial) | liga el valor; ver GAP (solo passthrough) |
| control | `try`/`catch`/`finally` (normal, tras catch, propagado, break, continue) | OK | |
| control | `finally` con `return` dentro del `try` | OK | `ctrl.try.finally.return` (el finally corre; el valor se evalúa antes) |
| operators | `+ - * / % **` | OK | |
| operators | bit a bit `& \| ^ ~ << >>` | OK | |
| operators | comparación / lógicos | OK | |
| operators | unarios `- ~`, postfix `++`/`--` | OK | |
| operators | prefijo `++x` / `--x` | OK | `op.preinc.*` / `op.predec.*` |
| operators | asignación compuesta `+= -= *= /= %=` | OK | |
| operators | spread `...` (array y record) | OK | |
| operators | `is` (básico) | OK | |
| operators | `is` con narrowing | OK | `oop.narrow.*` (incluye clases de usuario) |
| operators | `in` (String y clase `__contains`) | OK | |
| operators | `in` sobre array/record | OK | `op.in.arr.*` / `op.in.rec.*` |
| types | `alias`, uniones, `const` (literal), `let` | OK | |
| types | tuplas heterogéneas (índice literal / dinámico homogéneo) | OK | |
| types | tupla heterogénea con índice dinámico | **GAP** | `findings/06` (slots sin tag) |
| types | `Record<K,V>`, shapes, intersección | OK | |
| types | interfaces (genéricas) + extracción `T["k"]` | OK | |
| types | genéricos `function id<T>` / `class Caja<T>` | OK (parcial) | retorno `T` no se sustituye; ver GAP |
| types | phantom `!T` | OK | `types.phantom.fn` / `types.phantom.class` |
| oop | `class` + constructor `main` + `me` | OK | |
| oop | herencia `:` y `extends` | OK | |
| oop | `super` (método, campo, `super.main`) | OK | |
| oop | visibilidad `public/private/protected/static/readonly` (válidos) | OK | |
| oop | accesos inválidos (private/protected/readonly/unknown) | OK (rechazados) | `negative/` |
| oop | upcast `var b: Base = derivado` | OK | `oop.upcast.*` (dispatch dinámico + covarianza de arrays) |
| oop | `is` (clase/herencia/struct/enum) | OK | |
| oop | magic methods (aritmética, unarios, conversión, igualdad, order, `__len`, `__get`/`__set`, `__contains`, `__iter`/`__next`, `__call`, `__repr`/`__toString`, `__type`) | OK | `__toJson` excluido (requiere `json`, host_call prohibido) |
| enums | variantes, `Enum.Var`, `==`, `is`, iteración (con y sin índice) | OK | |
| match | 4 modos (valor/tipo/condición/patrón) + enums con payload | OK | `match.*` (F1/F2) |
| match | unión temporal `(A, B)` + acceso a miembro sobre unión | OK | `match.union.*` (F5) |
| match | familia ternario `?:` / `if`-expresión + arrowReturn `<-` | OK | `tern.*` (F3/F4) |
| structs | declaración, instanciación posicional, campos, escritura, `is` | OK | |
| functions | genéricas, arrow `->` (expr/bloque/sin args), defaults | OK | |
| functions | closures (captura/re-captura/mutación) | OK | |
| functions | función como valor / `Callable` / narrowing `is Callable` | OK | |
| functions | recursión, llamada indirecta | OK | `fn.indirect.order*` (orden plano-antes-de-indirecta) |
| functions | arrow fat `(x) => ...` / `x => ...` | OK | `fn.fat.*` (alias `=>` de `->`; anotado e inferido) |
| functions | local capturada vs global homónimo | OK | `fn.local.shadow` / `fn.closure.mutate` (la local no muta el global) |
| strings | interpolación `$x` / `${expr}` / templates backtick | OK | |
| strings | métodos (`upper/lower/trim/split/slice/indexOf/contains/startsWith/endsWith/isEmpty/length`) | OK | |
| strings | Char buffers (`toChar`/`char`/`ord`/`join`) | OK | |
| strings | índice de String fuera de rango | OK | `str.index.oob` (clamping `""`, documentado en datos.md) |
| cmx | tags minúscula/mayúscula, props (string/expr/bool/int), children, self-closing | OK | |
| cmx | spread de props + `app.tag is Callable` + invocación | OK | |
| cmx | shorthand `{nombre}` | OK | `cmx.shorthand` |
| cmx | `str()`/`toString()`/`bool()` sobre `Value` | OK | `cmx.any.str/tostring/bool` |
| cmx | comparar prop bool con `true` | OK | `cmx.prop.bool` / `.nefalse` |
| cmx | child de texto `==` su string | OK | `cmx.text.value` |
| when | `os:`, `arch:`, `platform:`, `abi:`, `or`/`not`, `default` | OK | host Windows / `arch: cls-arch` |
| when | `target:` triple sin comillas | OK | `when.target.triple` |
| modules | `import "x" as ns` (`ns::sym`) | OK | |
| modules | `from "x" import a as b, c` | OK | |
| modules | re-export barrel (`ns::sym as`, `from`) | OK | |
| modules | const/var/función transversal | OK (parcial) | `from-import` de var = snapshot; `ns::var` es live |
| modules | `module`/`namespace` | **GAP** | `findings/17` (no cableado en typeck/emisor) |
| modules | `/modules` compartidos (`bytes`, `prng`) | OK | |
| errors | `throw`/`catch`, div por cero, índice fuera de rango (array), propagación/aniñado | OK | |
| extension | FFI `when` + `extension` (CInt/CDouble/CString) | OK | shim propio |
| misc | `static function` | OK | |

## Gaps / no soportado (no son regresiones de un plan, son límites del lenguaje/JIT)

| Feature | Estado | Nota |
|---|---|---|
| `async` / `await` | no ejecutable | el parser lo acepta; el JIT no compila `await` (docs/lenguaje/funciones.md) |
| `macro`, `global`, `sync` | keywords reservados | sin implementación documentada |
| `match` F6 (`__match`/`__unpack` de clases) | pendiente | ver `agent-context/MATCH_PLAN.md` §4-F6 |
| acceso a miembro sobre unión de builtins (`int \| float`) | GAP | requiere boxing; `str(v)` sí funciona (ver docs/lenguaje/tipos.md) |
| rest params `function f(...xs)` / spread en llamadas `f(...args)` | no soportado | `call_indirect` exige aridad estática |
| `with` | **semántica pobre** | liga el valor como `Any`; `o.x`/aritmética fallan o devuelven basura; solo passthrough/print |
| inferencia de retorno de genéricos en clases | parcial | `caja.obtener()` tipa `T`; requiere `int(...)`/asignación tipada |
| `__toJson` | no probado | solo se dispara con `json.stringify` (host_call prohibido por autocontención) |
| índice dinámico de tupla **heterogénea** | GAP | slots sin tag; el literal y el dinámico homogéneo sí funcionan (`findings/06`) |
| `module` / `namespace` | GAP | el parser acepta la sintaxis; el typeck/emisor no cablean `NS::sym` (`findings/17`) |
| `is` narrowing sobre `Callable` | OK | sí funciona (igual que el narrowing de clases, ya soportado) |

## Findings (bugs abiertos)

Ver `findings/README.md`. Estado tras el endurecimiento de la matriz: **0 bugs
abiertos, 2 GAP** documentados (`findings/gaps.txt`). Los **15 repros resueltos**
(01–05, 07–16) fueron **promovidos a la matriz dorada** (`src/sections/` +
`expected.txt`) y **retirados** de `findings/`; ya no queda ningún `[FIX?]`.
`run.ps1` cuenta los abiertos como FAIL; los GAP no cuentan.

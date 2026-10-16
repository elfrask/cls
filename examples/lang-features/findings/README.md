# `findings/` — repros mínimos de regresiones (matriz de features)

Cada archivo `.clsx` de esta carpeta es un **repro mínimo** de un bug detectado al
construir `examples/lang-features/`. El contrato es:

- `clx run <repro> --add-path-module=modules` **debe fallar** (exit ≠ 0) mientras
  el bug exista.
- Si un repro pasa a **exit 0**, el bug fue arreglado → **promoverlo a la matriz**
  (`src/sections/` + `expected.txt`) y **retirar** el repro de esta carpeta.

`run.ps1` corre todos y reporta `[BUG]` (abierto) / `[FIX?]` (resuelto, pendiente
de promover) / `[GAP]` (documentado, `gaps.txt`). Estado **tras esta sesión de
endurecimiento**:

> **0 findings abiertos, 0 pendientes de promover, 2 GAP.**
> Los **15 repros resueltos** (01–05, 07–16) fueron **promovidos a la matriz**
> (`src/sections/` secciones `control`/`operators`/`types`/`oop`/`functions`/
> `strings`/`cmx`/`when`) y **retirados** de `findings/`; su cobertura ahora es
> parte del golden determinista (`expected.txt`). Solo quedan los **GAP** (06 y
> 17), que no son regresiones.

Comando general (desde la raíz del repo):

```powershell
target\debug\clx.exe run examples\lang-features\findings\GN-nombre.clsx --add-path-module=modules
```

---

## Historial de hallazgos (todos cerrados o GAP)

Esta tabla conserva la causa raíz de cada hallazgo y **dónde** quedó su cobertura
en la matriz tras la promoción.

| # | Feature | Estado | Causa / nota | Cobertura en la matriz |
|---|---------|--------|--------------|------------------------|
| 01 | `finally` con `return` | **PROMOVIDO** (reinterpretado) | El `finally` SÍ corría; la expectativa original (`tRF`) era incorrecta. Semántica estándar: el valor de retorno se evalúa ANTES del `finally` → `tR`, y el `finally` corre (efecto lateral). | `control`: `ctrl.try.finally.return`, `.side` |
| 02 | prefijo `++x` / `--x` | **PROMOVIDO** | `parser.rs parse_unary`: faltaba mapear `PlusPlus`/`MinusMinus` a `PreInc`/`PreDec` (el emisor/typeck ya los soportaban). | `operators`: `op.preinc.*`, `op.predec.*` |
| 03 | `in` sobre array/record | **PROMOVIDO** | `binary.rs emit_binary` (`In`): faltaba despachar Array → `__intr_arr_includes` y Record/Shape → `__intr_record_has`. | `operators`: `op.in.arr.*`, `op.in.rec.*` |
| 04 | `is` narrowing (clases) | **PROMOVIDO** | `typeck/flow.rs narrowed_var`: no estrechaba a clases de usuario; ahora `x is Clase` define `x: Named(Clase)`. | `oop`: `oop.narrow.*` |
| 05 | upcast `var b: Base = d` | **PROMOVIDO** | `typeck/classes.rs is_assignable`: asignabilidad que consulta `class_parents` (herencia) + covarianza de arrays/tuplas. | `oop`: `oop.upcast.dispatch`, `.field`, `.array.dispatch` |
| 06 | índice dinámico de tupla heterogénea | **GAP** | Las tuplas son slots crudos de 8B **sin tag**: recuperar el tipo del slot en runtime exigiría taggear/boxear (prohibido por rendimiento). Índice literal y dinámico homogéneo sí funcionan. Ver `gaps.txt`. | — (GAP, no se cubre) |
| 07 | phantom `!T` | **PROMOVIDO** | `engine/functions.rs resolve_annotation_type`: `!T` (compile-time only) se mapea a `Any` (i64 en la firma WASM). | `types`: `types.phantom.fn`, `.class` |
| 08 | flecha `=>` (fat arrow) | **PROMOVIDO** | Se agregó el token `Operator::FatArrow` y el parser acepta `=>` como alias de `->`, incluida la forma sin paréntesis `x => expr`. | `functions`: `fn.fat.expr/block/multi/noargs/infer[.paren]` |
| 09 | `indirect call type mismatch` | **PROMOVIDO** | La arrow tomaba el **span del token siguiente** y el type map lo pisaba con el tipo de la sentencia siguiente (`var f = suma;`), dando aridad errónea. Ahora el span de la arrow es el del token de flecha. | `functions`: `fn.indirect.order*` |
| 10 | CMX shorthand `{nombre}` | **PROMOVIDO** | Se agregó `CmxToken::AttrShorthand`; el lexer consume el `}` de cierre y el parser emite `CmxAttributeValue::Shorthand`. | `cmx`: `cmx.shorthand` |
| 11 | `str()`/`toString()`/`bool()` sobre `Any` | **PROMOVIDO** | `host_any_member` no tenía brazo para tag 5 (CmxValue): `AnyMember(cmx, "props")` devolvía `(0,0)`. Ahora resuelve props/children/kind/tag. | `cmx`: `cmx.any.str/tostring/bool` |
| 12 | prop bool de CMX `== true` | **PROMOVIDO** | `binary.rs` (`==`/`!=`): el tipo común de comparación ahora alinea i32/i64 (i64 side si algún lado es i64), en vez de emitir `i64.eq` con un i32. | `cmx`: `cmx.prop.bool`, `.nefalse` |
| 13 | child de texto CMX `== "Hola"` | **PROMOVIDO** | Los children de texto/expresión String se normalizan como CmxValue de texto (kind=1); `==` entre Cmx y String se resuelve formateando el Cmx a string. | `cmx`: `cmx.text.value` |
| 14 | `when (target: <tripla>)` sin comillas | **PROMOVIDO** | `parser.rs parse_target_atom`: `target:` acepta string O tripla desnuda (`x86_64-windows-msvc`). | `when`: `when.target.triple` |
| 15 | local capturada pisa un global homónimo | **PROMOVIDO** | `emitter/mod.rs emit_ident_load/-store/-ptr`: el orden de resolución ponía `globals` antes que capturas/promoted/locales; una local que sombrea al global ahora gana. | `functions`: `fn.local.shadow`, `fn.closure.mutate` |
| 16 | String fuera de rango | **PROMOVIDO** (no era bug) | `docs/lenguaje/datos.md` define el **clamping** (`s[99]` → `""`) y el oráculo `str-index.clsx` lo codifica. No se alinea con Array: es decisión de diseño. | `strings`: `str.index.oob`, `str.index.inrange` |
| 17 | `module` / `namespace` | **GAP** | El parser acepta la sintaxis pero el typeck/emisor no cablean `NS::sym` / `NS.member`. Ver `gaps.txt`. | — (GAP, no se cubre) |

---

## GAPs (no son regresiones)

Listados en `gaps.txt`; `run.ps1` los reporta como `[GAP]` y no los cuenta como
abiertos.

### 06 — índice dinámico de tupla heterogénea
`var t: (int, String, float); var i = 2; t[i]` no puede recuperar el tag del slot
en runtime: la representación de tuplas es un bloque de slots de 8 bytes sin tag
(igual que Array, pero heterogéneo por posición). El índice **literal** y el
dinámico **homogéneo** funcionan. Solución completa = taggear cada slot (boxing),
prohibido por las reglas de rendimiento del JIT.

> **Nota (dev de pruebas):** este GAP queda **explícitamente fuera** de la matriz
> dorada. La cobertura sí cubre los casos soportados (`types.tuple.lit0/lit1`,
> `types.tuple.dyn.int` homogéneo, `types.tuple.join`). No agregar un caso de
> índice dinámico heterogéneo: requeriría boxing (prohibido) y la promesa de
> `docs/lenguaje/tipos.md` (`a[i] -> Int|String`) no es alcanzable sin él.

### 17 — `module` / `namespace`
`namespace NS { export function hi() ... }` + `NS::hi()`: el parser acepta la
sintaxis, pero el typeck no registra los símbolos del namespace y el emisor no
resuelve `NS::miembro`. Es una feature de scoping no implementada (no una
regresión de un plan).

---

## Corrección de expectativas (no eran bugs)

- **01**: `finally` + `return`. El `finally` corría; el valor de retorno se
  evalúa antes (semántica estándar). La expectativa `tRF` era un error de cálculo.
- **16**: String fuera de rango devuelve `""` por diseño (clamping documentado en
  `datos.md`), no lanza. Consistente con el oráculo `str-index.clsx`.

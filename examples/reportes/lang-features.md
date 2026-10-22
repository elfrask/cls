# Reporte de regresiones — prueba: matriz de features (`examples/lang-features/`)

> Cierre por prueba (`examples/patrones.md` §5.2.10 / §5.6). Prueba nueva motivada
> por una observación real: **los tests usaban muy pocas features del lenguaje**
> (`switch`, `loop`, `for`, `for each`, `with`, `when`, `is`, spread, enums,
> genéricos, magics, CMX…). Esta matriz ejercita el lenguaje **completo**.

- **Roles:** dev-pruebas → dev-correcciones → Juez (gate).
- **Veredicto del Juez:** **APROBADO** (reprodujo todo; verificó **independientemente**
  las 2 expectativas reinterpretadas; sin falso verde).

---

## Resultado

- `examples/lang-features/run.ps1` → **exit 0**: matriz == golden, **6/6 negativos**
  rechazados, **0 findings abiertos** (eran 17), **2 GAP** documentados.
- Golden: **322 líneas / 290 aserciones / 65 filas de features / 13 secciones**.
- Cobertura: control (`if/elif/else`, `switch/case/default`, `while`, `loop`,
  `for`, `for each` (+idx), `with`, `try/catch/finally`), operadores (arith/bit a
  bit/comparación/unarios/`++/--`/compuestos/spread/`is`/`in`), tipos (alias,
  uniones, literales, tuplas, `Record<K,V>`, shapes, interfaces genéricas,
  extracción `T["k"]`, genéricos, phantom `!T`), OOP (herencia `:`/`extends`,
  `super`, visibilidad, `static`, `readonly`, `me`, magics, `is`), enums
  (iterables), structs, funciones (genéricas, arrow `->` y **fat arrow `=>`**,
  closures, `Callable`, recursión), strings (interpolación/templates/métodos/Char),
  CMX, `when` multi-entorno, `extension` FFI, módulos.

## Regresiones encontradas y solución

| ID | Área | Síntoma | Causa raíz | Solución | Estado |
|----|------|---------|------------|----------|--------|
| **02** | Parser `/unary` | `++y;`/`--y;` (prefijo) → error de sintaxis | `parse_unary` no mapeaba `PlusPlus/MinusMinus` a `PreInc/PreDec` (emisor/typeck ya los tenían) | Mapeo en `parser.rs` | ✅ |
| **03** | Emisor `in` | `in` sobre **array/record** daba mal (`9 in [1,2,3]`→true, `"x" in {x:1}`→false) | Arm `In` solo cubría String/`__contains` | `Array→__intr_arr_includes`, `Record/Shape→__intr_record_has` (`binary.rs`) | ✅ |
| **04** | Typeck narrowing | `if (b is Derived) { b.extra }` → error de tipo | `narrowed_var` no definía `Named(Clase)` | `typeck/flow.rs` | ✅ |
| **05** | Typeck asignabilidad | `var b: Base = derivado;` → "No se puede asignar" | `is_assignable` no consultaba `class_parents` | `typeck/classes.rs` (+ covarianza de arrays/tuplas) | ✅ |
| **07** | Typeck/JIT phantom | `!T` → "no soportado por el JIT" | `resolve_annotation_type` no manejaba `Phantom` | `Phantom(_) → Any` (`engine/functions.rs`) | ✅ |
| **08** | Parser fat arrow | `(x) => …`, `x => …` no parseaban | Token/parse de `=>` inexistente en arrow | `Operator::FatArrow` + `parse_arrow_tail`/`is_arrow_function` (`token.rs`,`parser.rs`) | ✅ |
| **09** | Emisor `call_indirect` | Llamada indirecta trapeaba según el **orden** de una función local plana | La arrow usaba `self.span()` (lookahead) → el type map se pisaba con el tipo de la sentencia siguiente | Span de la arrow = token de flecha | ✅ |
| **10** | CMX lexer/parser | CMX shorthand `{nombre}` → `Token inesperado '<'` | (fix de CMX shorthand) | Lexer/parser CMX | ✅ |
| **11** | Host `any_member` | `str()/toString()/bool()` sobre `Any`/CMX → `"0"`/`false` | Faltaba el brazo **tag 5 (CmxValue)** en `host_any_member` | Brazo 5 (props/children/kind/tag) en `cls-jit/src/host.rs` | ✅ |
| **12** | Emisor `==`/`!=` | `prop bool CMX == true` → WASM inválido (`i64 vs i32`) | Comparación no alineaba i32/i64 | Extensión del i32 al tipo común (`binary.rs`) | ✅ |
| **13** | Emisor/runtime CMX | `<p>Hola</p>.children[0] == "Hola"` → false | Child texto no se exponía como CmxValue de texto | `containers.rs` + `emit_cmx_string_eq` | ✅ |
| **14** | Parser `when` | `when (target: x86_64-windows-msvc)` → "Esperaba string" | Parser exigía string | Parse de tripla sin comillas | ✅ |
| **15** | Emisor/scope | Local capturada por closure **mutaba un global homónimo** | Orden de resolución ponía `globals` antes de capturas/promoted/locales | Reordenado (`emitter/mod.rs`) | ✅ |
| **01** | `finally`+`return` | Repro esperaba `"tRF"` | **No era bug**: el `finally` **sí corre**; el valor se evalúa antes (semántica JS/Java/C#) | Repro corregido + promovido a la matriz (`ctrl.try.finally.return`) | ✅ (reinterpretado) |
| **16** | String OOB | `s[99]` no lanzaba | **No era bug**: `docs/lenguaje/datos.md` define **clamping**→`""` (Array sí lanza) | Documentado + promovido (`str.index.oob`) | ✅ (reinterpretado) |

## GAPs (features no implementadas; no son regresiones)

- **06 — tupla heterogénea con índice dinámico**: `t[i]` devuelve el bit-pattern
  (sin tag). Arreglarlo exige **boxing** (prohibido). Nota de excepción añadida a
  `docs/lenguaje/tipos.md`.
- **17 — `module`/`namespace`**: `NS::hi()` no existe/soportado (keywords presentes
  sin cablear en typeck/emisor).
- (Menor) instanciar una **interface con `!T`** (sustitución restante en campos) no
  está cubierto; el fix de 07 cubre `!T` en firmas de función/clase.

## Extra (necesario para que los fixes se vean)

- `cls-core/build.rs`: `CLS_BACKEND_HASH` ahora hashea también `src/frontend` y
  `src/middleware`. Sin esto el caché `~/.cache/cls` servía WASM **stale** y
  ocultaba los fixes de parser/typeck.

## Verificación (al cierre)

- `lang-features/run.ps1`: **OK** (0 abiertos, 2 GAP). El Juez forzó mutaciones (golden corrupto, `.clsx` válido en `negative/`) y el runner **las detecta**.
- `cargo test --workspace`: **0 fallos** · `run-tests.ps1`: **57 PASS** · `run-availible.ps1`: **24 PASS / 1 SKIP**.
- `conformance`: **2302 + fuzz 20000** · `brainfuck`: **OK** · `clsh`: **22/25** · `sqlite-orm`: **OK**.
- Juez: verificó **independientemente** 01 (`finally` corre, valor antes; semántica estándar) y 16 (clamping documentado), los 15 fixes, los 2 GAP, `git diff cls-internals/` vacío, sin `host_calls` nuevos, sin punteros → **APROBADO**.

## Endurecimiento (§5.6)

Tras la aprobación, se **promovieron los 15 fixes a la matriz dorada** (36 aserciones
nuevas; golden 254→290 checks) y se **limpiaron comentarios/workarounds obsoletos**
(`functions.clsx`, `cmx.clsx`) que evitaban features ya funcionales. Regla nueva en
`patrones.md §5.6`: **uso obligatorio del lenguaje completo**.

## Archivos tocados (por las correcciones)

`cls-core/src/frontend/{parser,token,lexer}.rs`,
`cls-core/src/middleware/typeck/{flow,classes,calls,decls}.rs`,
`cls-core/src/backend/wasm/{engine/functions,emitter/binary,emitter/containers,emitter/mod}.rs`,
`cls-jit/src/host.rs`,
`cls-core/build.rs`,
`docs/lenguaje/tipos.md`.

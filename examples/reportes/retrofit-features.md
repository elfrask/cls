# Reporte — retrofit de features (§5.6) de los tests existentes

> Cierre por prueba/tanda (`examples/patrones.md` §5.6). Motivado por la
> observación de que los tests usaban un subconjunto mínimo del lenguaje.

- **Alcance:** refactorizar `examples/{brainfuck,conformance,clsh,sqlite-orm}` +
  los módulos que consumen (`orm`, `sqlite`, `io`, `bytes`, `check`, `prng`,
  `config`) para **usar el lenguaje completo** — **sin cambiar resultados**.
- **Roles:** dev-pruebas → dev-correcciones → Juez (gate).
- **Veredicto del Juez:** **APROBADO** (tras un primer **RECHAZO** acotado a
  §5.6 que se corrigió: workaround + docs falsas).

---

## Resultado

Los 4 tests siguen dando **exactamente lo mismo** pero ahora usan features ricas:

| Test | Antes | Ahora |
|---|---|---|
| brainfuck | `while` + 8 `if` para opcodes | **`enum Op`** + **`switch`**; `for`, `for each … and idx` |
| conformance | `if/elif` sobre `op` | **`switch`** (dispatch_binop/unop/…), **`loop`+break** (collatz/gcd), **`for`/`for each`**, `bool` |
| clsh | `K_*` ints, `Any[]`, `while(true)` | **`enum Tok`**, **`structure`** (Redir/Dollar/Unesc), tupla heterogénea, **`switch`** sobre char, **`loop`**, `for`/`for each` |
| sqlite-orm (+módulos) | `while` + if-chains | **`switch`** (cellOf/bind), **`for each`(+idx)**, **spread** (orm.update), **magic `__len`** |

Verificación: brainfuck **OK byte a byte** · conformance **2302 + fuzz 20000** ·
clsh **22/25** (mismas 3 D1–D3) · sqlite-orm **OK** · lang-features **OK**.

## Regresiones encontradas al usar más features (y solución)

| ID | Feature | Síntoma | Causa raíz | Solución | Estado |
|----|---------|---------|------------|----------|--------|
| **F1** | `switch` sobre `String` | Solo matcheaba literales internados (mismo puntero); strings construidas caían al `default` | `emit_switch` comparaba el i64 empaquetado (`i64.eq`) | Comparar **contenido** (`__intr_str_eq`); por tipo: `i64.eq`/`i32.eq`/`f64.eq` (`emitter/statements.rs`) | ✅ |
| **F2** | Closure en `for each` | Capturaba la variable del loop → valor **rancio** (`seen=0,0,0`) | Con la variable promovida a heap, `for each` hacía `local.set` del valor crudo en el slot-ptr | **Binding por iteración** (`__alloc` por iteración) en `emitter/foreach.rs` | ✅ |
| **F3** | `const` literal entero | `const N = 5; N * 2` → error ("requiere numérico") | `Type::Literal(5)` no operaba como su base | Literales operan como base en aritmética/bit a bit/comparación (`middleware/types.rs`, `typeck/binary.rs`) | ✅ |
| **F4** | `enum` importado | `Enum.Var == Enum.Var` fallaba entre módulos/alias | `def_id` distinto por alias al aplanar | Identidad por **definición** (span) (`engine/emit.rs`) | ✅ |
| **F5** | Closure que devuelve `Bool` | `var f = () -> (k == 2);` → WASM inválido (`expected i64, found i32`) | Arrows normalizaban la firma a `-> i64` y usaban prologue `f64.reinterpret`; el caller usaba el tipo real | La arrow usa la **firma natural** (`engine/emit.rs`, `engine/functions.rs`) | ✅ |

## Cumplimiento §5.6 (Juez)

- **RECHAZO inicial** (acotado): `clsh/lexer.clsx` mantenía un workaround
  `switch (ord(c.toChar()[0]))` que **evitaba** `switch` sobre `String` (ya
  funcional) + docs falsas (§10 decía que comparaba el puntero) + comentarios
  obsoletos sobre identidad de enum.
- **Corregido**: `isOpByte` despacha con `switch (c) { case (";") … }`; §10
  marcada RESUELTO; comentarios de enum actualizados (identidad por definición).
- **APROBADO** tras la corrección.

## Verificación (al cierre)

- 4 runners: brainfuck **OK** · conformance **2302/2302 + 20000/20000** · clsh **22/25** · sqlite-orm **OK**.
- Suites: `cargo test --workspace` **0** · `run-tests.ps1` **57 PASS** · `run-availible.ps1` **24/1 SKIP** · `lang-features/run.ps1` **OK**.
- Juez: A/B contra el binario previo (confirmó que F1–F5 eran bugs), verificó que `switch` sobre Int/Float/Bool/Char/enum **no** se rompió, `git diff cls-internals/` **vacío**, sin `host_calls`, sin punteros, oráculos/`.expect`/`expected.txt` **intactos**.

## Gaps / observaciones

- **`for each` sobre `String`** no soportado (iterar por índice).
- **`Char` sin literal** propio (`char(n)`).
- **`with`** es passthrough parcial (liga `Any`).
- Keywords sin ejecución: `async/await`, `macro`, `global`, `sync`, `match`.

## Archivos tocados (por las correcciones)

`cls-core/src/backend/wasm/emitter/{statements,foreach}.rs`,
`cls-core/src/backend/wasm/engine/{emit,functions}.rs`,
`cls-core/src/middleware/{types.rs,typeck/binary.rs}`,
+ los tests y módulos retrofiteados.

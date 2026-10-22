# Findings — prueba #6 (DNS)

Repros mínimos de bugs del lenguaje encontrados al construir la prueba.

**ESTADO: los 3 findings (F1/F2/F3) están RESUELTOS en el compilador/JIT/typeck.**
La causa raíz se corrigió en Rust (sin `host_calls` nuevos, sin exponer punteros,
sin trucos) y se quitaron los workarounds de `modules/dns`, `modules/net` y
`examples/dns`.

Todos se reproducen desde la raíz del repo con `target\debug\clx.exe` (JIT).

---

## F1 — Parámetros array se pasan por valor (mutación no llega al llamador) — RESUELTO

- **Repro:** `01-array-param-by-value.clsx`
- **Comando:** `clx run examples\dns\findings\01-array-param-by-value.clsx`
- **Esperado/obtenido (ahora):** `x.length=3` ✅
- **Síntoma original:** `a.push(v)` dentro de una función mutaba una **copia** del
  array; el array del llamador no cambiaba (`x.length=2`). La mutación *sí*
  llegaba mientras `push` no reasignara el buffer; se perdía cuando `push`
  crecía el array (realloc) y devolvía un puntero nuevo.
- **Causa raíz:** los arrays viajan como puntero al layout `[cap][len][elems]`
  (semántica de referencia), pero al crecer `push`/`unshift` reubicaban el buffer
  a una dirección nueva: el write-back solo actualizaba al *receiver* del mutador
  (el parámetro local del callee), dejando al llamador apuntando al buffer viejo.
- **Fix (Rust):**
  - `cls-internals/wasm/src/arrays.rs`: al crecer, el buffer viejo **no se
    descarta**: se marca como *forward* (`cap = FWD_MAGIC`, `len = <ptr nuevo>`)
    y `__intr_arr_*` resuelven la cadena con `arr_resolve` (nueva
    `__intr_arr_deref`, idempotente). La **identidad del array es estable**, así
    que cualquier alias (incluida la variable del llamador) ve la versión crecida.
  - `cls-internals/src/abi.rs`: se registra `__intr_arr_deref`.
  - `cls-core/src/backend/wasm/emitter/expressions.rs` (`emit_expression`):
    resuelve el forward de todo valor leído de tipo `Array` (punto único por el
    que pasa toda lectura) — transparente, ABI-compatible (los arrays siguen
    pasándose como `i64`; sin convención by-ref, sin romper llamadas dinámicas).
- **Evidencia:** `x.length=3`; arrays de `String`/clase/anidados mutados en
  funciones y aliasing (`cp = xs; xs.push(99)` → `cp` ve el crecimiento) probados
  aparte.

## F2 — Corrupción de strings en auto-concatenación `x = x + <string>` — RESUELTO

- **Repro:** `02-string-self-concat.clsx`
- **Comando:** `clx run examples\dns\findings\02-string-self-concat.clsx`
- **Esperado/obtenido (ahora):** `selfcat ... wrongA=0` ✅
- **Síntoma original:** `out = ""; out = out + a` (con `a` de ≥ ~decenas de bytes)
  conservaba la longitud pero **corrompía bytes** (`wrongA=47`, variable).
- **Causa raíz:** en el *slow path* de `__intr_str_append` (cuando el string
  viejo no tiene header de capacidad, p.ej. un literal) la capacidad se
  calculaba a partir de la longitud **vieja** (`so.len()`), ignorando la pieza:
  con `old = ""` y una pieza de 100 B alocaba 16 B y escribía 100 → desbordaba
  el buffer y el contenido correcto se pisaba con las alocaciones siguientes.
- **Fix (Rust):**
  - `cls-internals/wasm/src/strings.rs`: la capacidad del slow path cubre la
    **longitud total** (`so.len() + piece.len()`).
  - Endurecido el header de capacidad: `[MAGIC:i64][cap:i64][bytes]` con MAGIC
    exacto de 64 bits (`read_cap`/`write_cap`/`alloc_with_cap`). Antes bastaba el
    bit 63 del byte pegado al literal, que se activaba tras un string que termina
    en byte multibyte → falso positivo que escribía **dentro del pool**.
- **Evidencia:** repro `wrongA=0`; test de 3200 B (`0123456789ABCDEF` ×200) y
  multibyte (`áéíóúñ` ×100, 1200 B) **byte a byte `==`** contra el literal
  esperado (antes: corrupción).

## F3 — `is` con clase cualificada de otro módulo no soportado — RESUELTO

- **Repro:** `03-is-qualified-class.clsx` (+ módulo `isq_mod/`)
- **Comando:** `clx run examples\dns\findings\03-is-qualified-class.clsx --add-path-module=examples\dns\findings`
- **Esperado/obtenido (ahora):** `who=B` · `is MB, extra=2` ✅
- **Síntoma original:** error de compilación `'is' con 'm::MB': se esperaba un
  enum o structure en el JIT`.
- **Causa raíz (doble):**
  1. **Emisor**: `is` solo miraba clases por `Identifier`; con
     `mod::Clase` (NamespaceAccess) caía al path enum/struct. Además, al aplanar
     los imports, cada clase aparecía **duplicada** (`Clase` sin prefijar y
     `mod::Clase`), con `class_id` y `extends` sin resolver en el clon
     prefijado → layouts distintos (`extra` a distinto offset).
  2. **Typeck**: el *narrowing* de `if (x is ...)` solo aceptaba `Identifier` a la
     derecha, así que `x.campo` del subtipo no typecheckeaba con la clase
     cualificada.
- **Fix (Rust):**
  - `cls-core/src/backend/wasm/engine/emit.rs`: la **identidad de clase es su
    DEFINICIÓN** (`class_id_by_span`, mismo criterio que `def_id_by_span` de los
    enums) → `mod::Clase` y `Clase` comparten `class_id`; y `resolve_parent_key`
    resuelve el `extends` del clon prefijado contra `mod::Padre` (mismo módulo)
    de modo que el layout/ancestors coinciden.
  - `cls-core/src/backend/wasm/emitter/binary.rs`: `is` acepta
    `Identifier` **y** `NamespaceAccess`, y recolecta las subclases por
    `class_id` de definición (`ancestor_class_id`).
  - `cls-core/src/backend/wasm/emitter/mod.rs`: helpers
    `resolve_is_class_key` / `ancestor_class_id`.
  - `cls-core/src/middleware/typeck/flow.rs` (`narrowed_var`): estrecha también
    con `mod::Clase` (al nombre local de la clase).
- **Evidencia:** `who=B` + `is MB, extra=2`; las aserciones `is dns::RrA` /
  `dns::RrAAAA` / `dns::RrName` / `dns::RrSOA` / `dns::RrTXT` de
  `examples/dns/src/main.clsx` pasan.

---

## Deudas / limitaciones (no bugs)

- **Ruta del `.dll` FFI relativa al CWD**: los shims `modules/net` y `modules/io`
  se resuelven desde el CWD; hay que correr desde la raíz del repo.
- **`nslookup` en Windows no admite puerto no estándar** (no soporta `-port` ni
  `set port`), así que el oráculo CLI opcional se marca **SKIP**; el oráculo
  Python (independiente) cubre el eje.

## Workarounds quitados

- `modules/dns/mod.clsx`: `pushU16`/`pushU32`/`pushAll` ahora **mutan in-place**
  (sin `-> int[]` ni `return`); `appendStr` eliminado (se usa `+` normal en
  `RrTXT.render`); accesores virtuales de `Rr` (`ipText`/`nameTarget`/`mxPref`/
  `txtParts`/`soaMname`/`soaSerial`) eliminados.
- `examples/dns/src/main.clsx`: call sites `x = dns::pushU16(x, v)` →
  `dns::pushU16(x, v)`; `joinLines` usa `+`; las aserciones `live` usan
  `is` + narrowing (`r1 is dns::RrA` → `r1.ip`), con `check::fail` en el `else`.
- `modules/net/mod.clsx`: `toChars`/`toBytes` siguen siendo conversores
  `int[]↔Char[]` (devuelven un array nuevo por diseño; no dependen del bug).

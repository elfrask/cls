# Regresiones de CLS halladas por la prueba #5 (SQLite ORM)

Repros mínimos y evidencia de bugs del lenguaje/compilador JIT detectados al
construir `examples/sqlite-orm/`.

**Todos los comandos se ejecutan desde la raíz del repo** (`CWD = raíz`).

Resumen:

| # | Área | Efecto | Archivo(s) |
|---|---|---|---|
| 01 | `const` en módulo importado | valor **0** al leerse dentro de ese módulo | `01-const-*.clsx` |
| 02 | Control de flujo `elif` | ejecuta el `else` de más | `02-elif-chain.clsx` |
| 03 | Strings / FFI `CString` | `CString: el argumento contiene NUL` (corrupción) | `src/repro-string-*.clsx` |
| 04 | Shadow stack (trace on) | trap + **panic** del reporter de errores | `run` de `src/main.clsx` |
| 05 | `Callable` no invocable | `No se puede llamar como función: Callable` | `05-callable-not-callable.clsx` |
| 06 | Callback C→CLS | no expresable (sin *function pointers*) | `06-ffi-callback-c-to-cls.clsx` |
| 07 | Gaps de FFI/módulos | ruta `.dll` relativa al CWD; aislamiento de imports | (notas) |

> **Estado: los findings 01–05 fueron RESUELTOS** por el desarrollador de
> correcciones (ver `examples/reportes/05-sqlite-orm.md`). El **06** queda como
> **feature faltante documentada** (callbacks C→CLS). `src/main.clsx` corre con el
> shadow stack **por defecto** (ya no requiere `CLS_JIT_TRACE=0`).

---

## 1. `export const` de un módulo importado vale 0 dentro de ese módulo

- **Archivos:** `01-const-mod.clsx`, `01-const-imported-module.clsx`.
- **Feature:** constantes de módulo (`export const`) + resolución cross-module.
- **Repro:**
  ```
  target\debug\clx.exe run examples\sqlite-orm\findings\01-const-imported-module.clsx
  ```
- **Esperado:** `getKind()=2`, `KIND=2`, `mkBox().kind=2`.
- **Obtenido:**
  ```
  cross getKind()=0      (esperado 2)
  cross KIND=2           (esperado 2)
  intra mkBox().kind=0   (esperado 2)   <-- BUG
  intra mkBoxLit().kind=2
  ```
- **Causa (análisis):** dentro de un módulo **no-entry**, un `export const` se
  lee como `0` cuando se referencia desde una función de ese mismo módulo
  (`getKind()` devuelve 0; `Box(KIND)` guarda 0). El acceso **desde otro módulo**
  (`m::KIND`) sí da el valor correcto (2). Un literal (`Box(2)`) también. Es un
  problema de inicialización/orden de los globales aplanados de módulos
  importados.
- **Impacto:** cualquier `const` de librería usado internamente. **Workaround
  usado en la prueba:** las fábricas de `Cell` en `driver.clsx` usan literales
  en vez de `KIND_*`.

---

## 2. Cadena `if / elif / elif / ... / else` ejecuta el `else` de más

- **Archivo:** `02-elif-chain.clsx`.
- **Feature:** control de flujo `elif` (cuerpos que **no** hacen `return`).
- **Repro:**
  ```
  target\debug\clx.exe run examples\sqlite-orm\findings\02-elif-chain.clsx
  ```
- **Esperado:** `f(1)=A f(2)=B f(3)=C f(4)=D`.
- **Obtenido:**
  ```
  f(1)=A  (esperado A)
  f(2)=D  (esperado B; BUG si D)   <-- BUG
  f(3)=C  (esperado C)
  f(4)=D  (esperado D)
  ```
- **Causa (análisis):** con **≥2 `elif` + `else`** y cuerpos que no salen de la
  rama (`return`), la primera `elif` (y en cadenas más largas varias) cae además
  en el `else`. Con **1 solo `elif`** no ocurre; **sin `else`** tampoco; con
  cuerpos que hacen `return` tampoco (el `return` enmascara el fall-through). El
  fall-through es visible con asignación/`push`.
- **Impacto:** semántica de control de flujo incorrecta en código idiomático.
  **Workaround usado:** en `sqlite_driver.materialize` se usa `if/else` anidado
  con `return` en vez de la cadena `elif`.
- **Repro extendido (para el fixer):** con 4 ramas, `t=2` y `t=3` caen al `else`
  (`whichPush(2)=B,E`, `whichPush(3)=C,E`); con 3 ramas sólo `t=2`
  (`chain3push(2)=BD`).

---

## 3. Corrupción de strings / FFI: `CString: el argumento contiene NUL`

- **Archivos:** `src/repro-string-inline.clsx` (pasa), `src/repro-string-var.clsx`
  (falla). Ambos insertan 1000 filas por el ORM (bulk) y difieren sólo en si el
  nombre se construye inline o en una variable local.
- **Feature:** paso de `String` CLS a un parámetro `CString` de la extensión,
  desde una `Cell` retenida en un literal de array, a través de varias capas.
- **Repro:**
  ```
  target\debug\clx.exe run examples\sqlite-orm\src\repro-string-var.clsx
  target\debug\clx.exe run examples\sqlite-orm\src\repro-string-inline.clsx
  ```
- **Obtenido:** `repro-string-var` **falla 6/6** con
  ```
  Trap WASM: Extensión 'o_bind_text' (...sqlite_shim.dll): Error de runtime:
  CString: el argumento contiene NUL
  ```
  `repro-string-inline` **pasa 6/6** (`var temp build ok=1000`). El patrón se
  **invierte** al cambiar la aridad de la fila: con 3 columnas
  (`[int, text, float]`) el **inline** falla (`src/pReadSize`, ya retirado;
  reproducido también por `testBatch` original con `insertBatch(Cell[][])`) y el
  **var** pasa (patrón de `pBulk`).
- **Causa (análisis):** un `String` que viaja como argumento `CString` puede
  llegar al wrapper con un byte `NUL` dentro de su longitud. No se reproduce con
  una llamada FFI directa (`o_bind_text(0,1,"x"+str(i))` en bucle pasa), sólo a
  través de la cadena `Cell`→`Cell[]`→`ORM`→`driver`→`bind`→`o_bind_text`, y
  depende de la forma del literal/aridad. Apunta a **lifetime/copias de strings
  temporales** en el emisor (la `Cell` retiene el string; al leerse más tarde su
  buffer está corrupto).
- **Impacto:** alto (corrupción silenciosa de datos). **Workaround usado:**
  `main.clsx` inserta por streaming (una `Cell[]` fresca por fila, sin `Cell[][]`
  anidado) y `run.ps1` es estable 10/10.
- **Repro puro-CLS relacionado** (retener strings en arrays anidados):
  `nestedCheck(1000)` en un test interno dio `bad=1` (una cadena corrupta); a
  mayor escala dio 0 — es no determinista.

---

## 4. Shadow stack (`CLS_JIT_TRACE` on) → trap + panic del reporter

- **Archivo/comando:** `src/main.clsx` (lectura de 10k filas vía ORM).
- **Feature:** shadow call stack en memoria lineal (default on).
- **Repro:**
  ```
  target\debug\clx.exe run examples\sqlite-orm\src\main.clsx
  ```
- **Obtenido (trace ON, default):** falla **5/5** con
  ```
  thread 'main' panicked at cls-jit\src\engine.rs:356:38:
  attempt to multiply with overflow
  ```
  La pila: `read_shadow_trace` → `read_shadow_stack` → `name_at`
  (`string_table_base + idx*8` con `idx` corrupto). Con
  `CLS_JIT_TRACE=0` el **mismo programa pasa** (todas las categorías OK).
- **Causa (análisis):** se dispara un trap en `materialize` (lectura de 30k
  celdas); al construir el trace, `__shadow_ptr` está corrupto y `name_at`
  multiplica con overflow → panic de Rust. Dos problemas:
  (a) el trap en sí (`wasm trap: interrupt`, visto en un repro intermedio); y
  (b) el **reporter de errores paniquea** en vez de mostrar el error — un trap
  bajo trace on puede tumbar el proceso antes de imprimir nada.
- **Impacto:** cualquier programa con carga semejante de función/lectura +
  shadow stack podía crashear con un panic ilegible.
- **Estado:** ✅ **RESUELTO**. Causa raíz: el shadow stack y sus slots estaban
  **dentro del rango de crecimiento del heap** (el bump-allocator los pisaba al
  crecer) → `__shadow_ptr`/`name_idx` corruptos. Fix: se movió la región del
  shadow stack **fuera del heap** (`layout.rs`), y el reporter ahora **acota**
  índices (`read_shadow_stack`/`name_at` con `checked_*` + límite de frames) →
  **no puede paniquear**. Verificado con trace on (default), error de runtime y
  trace profundo.

---

## 5. Un valor tipado como `Callable` no se puede invocar

- **Archivo:** `05-callable-not-callable.clsx`.
- **Feature:** funciones como valor / `Callable`.
- **Repro:**
  ```
  target\debug\clx.exe run examples\sqlite-orm\findings\05-callable-not-callable.clsx
  ```
- **Obtenido:** error de typeck
  ```
  [ERROR] No se puede llamar como función: Callable (05...:12:26)
  ```
  Afecta a `Record<String, Callable>["k"](...)`, `Callable[i](...)` (p. ej.
  `var arr: Callable[] = [add]; arr[0](2,3)`) y campos `var f: Callable`.
- **Lo que SÍ funciona:** un **record literal** con funciones inferidas,
  `var r = { add: add }; r["add"](2,3)` (el tipo de `add` es `Fun(int,int)->int`,
  no `Callable`). Y pasar el record a una función tipada como
  `Record<String, Callable>` compila, pero el typeck marca el resultado de la
  llamada como `Callable` (error de retorno).
- **Impacto:** bloquea el patrón "driver = Record de funciones".
  **Workaround usado:** `Driver` es una **clase base** y los backends heredan
  (el despacho dinámico por clase sí funciona: `class Impl: Base` + `d.m(...)`).

---

## 6. Callback C→CLS (`sqlite3_exec`) — bloqueo (no expresable)

- **Archivo:** `06-ffi-callback-c-to-cls.clsx`.
- **Feature:** FFI inverso (C llama a CLS).
- **Repro:**
  ```
  target\debug\clx.exe run examples\sqlite-orm\findings\06-ffi-callback-c-to-cls.clsx
  ```
- **Observado:** `Callable` como parámetro nativo **compila** (se emite como un
  entero: firma `iisi`); al no existir el símbolo en el shim, el runtime informa
  `Símbolo nativo 'o_exec_cb' no encontrado`. Es decir, el **handle de función
  cruza como entero**, pero **no hay mecanismo** para que C lo invoque (no hay
  *trampoline* host, ni un tipo `CFunc(...)`). `sqlite3_exec` con callback **no
  es implementable hoy** sin exponer punteros o tocar el host.
- **Impacto:** el punto más frágil del FFI (callbacks C→CLS) queda **bloqueado**;
  se reporta como gap, no como crash.

---

## 7. Gaps / limitaciones (rodeables, documentadas)

- **Ruta del `.dll` FFI relativa al CWD**, no al archivo `.clsx` (ya documentado
  en `docs/lenguaje/extension.md`). Obliga a correr desde la raíz del repo.
- **Aislamiento de imports con path**: `import "../src/orm"` desde `findings/`
  **no** aísla los imports internos de `orm.clsx` (`import "driver"` se resuelve
  contra el dir del **entry**, no contra el dir del módulo) ⇒ los repros que usan
  el ORM deben vivir en `src/`. Sin `cls.json` propio, la resolución es
  "relativa al entry".
- **Tipos dinámicos:** `Any` no está soportado por el JIT ("se requiere tipo
  concreto") y un array literal heterogéneo (`[1,"dos"]` anotado `Value[]`) es
  rechazado ("Array heterogéneo... Usa `Record<String, any>`"). El JIT exige
  anotación de tipo en **todos** los parámetros de función.
- **Herencia cross-module**: `class SqliteDriver: drv::Driver` **no parsea**
  ("Esperaba '{', encontré '::'"); hay que usar el nombre **no cualificado**
  (`: Driver`) aunque el módulo se importe con alias (el aplanado deja el nombre
  global). En cambio, una anotación de tipo sí acepta `drv::Driver`. Incoherencia
  parser/typeck/emitter.
- **`bf`-style**: `CFloat` (f32) no soportado por el dispatcher (usar `CDouble`),
  ya documentado.

---

## Entorno

- Windows, `gcc` 15.2.0 (MinGW-w64 x64), Python 3.13.9, SQLite 3.50.4
  (`C:\Program Files\Python313\DLLs\sqlite3.dll`).
- `target\debug\clx.exe` (JIT). Sin `sqlite3` CLI en el sistema (se usó el
  módulo `sqlite3` de Python como oráculo).

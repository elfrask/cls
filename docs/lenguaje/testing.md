# Testing en CLS — estado y propuestas

> Este documento es **una propuesta**. La CLI (`clx test`) ya existe como MVP
> (descubre archivos y los ejecuta con el JIT), pero el soporte de **lenguaje**
> (intrínsecos, nombres reservados, aserciones) está pendiente por decisión
> explícita. Aquí se listan las opciones a elegir.

## Estado actual (MVP)

- `cls.json` acepta `"testEntry": ["tests/**/*.clsx"]` (array; rutas o globs).
- `clx test [archivos|globs] [--filter <s>] [--list]` descubre y ejecuta cada
  archivo con el JIT; el **exit code** del archivo decide pass/fail.
- Hoy un archivo de test es un programa CLS normal (necesita `main`). Lo que
  falta es que el archivo pueda declarar `test(...)` en vez de `main`.

---

## 1. ¿Cómo entra un archivo de test? (reservados vs. módulo)

Tres opciones, de menos a más intrusiva:

| Opción | Cómo se escribe | Pro | Contra |
|--------|-----------------|-----|--------|
| A. Módulo `test` | `import "test" as t; t.test("suma", fn(){...});` | No reserva nada; cero colisión | Verboso; hay que importar |
| B. Nombres reservados | `test("suma", fn(){...});` en un archivo bajo `testEntry` | Limpio; el runner los detecta | `test`/`assert` pasan a ser reservados (rompe vars con esos nombres) |
| C. Azúcar de sintaxis | `test "suma" { ... };` o `@test function suma(){}` | Lo más expresivo | Cambia el parser; más superficie |

**Propuesta (recomendada): B + A.** Nombres reservados **solo dentro de archivos
listados en `testEntry`** (el compilador predefine un prelude de test), y un
módulo `test` explícito para quienes prefieran no depender del prelude.

---

## 2. Funciones de test (intrínsecos propuestos)

### Registro y estructura
- `test(nombre, fn)` — registra un caso.
- `describe(nombre, fn)` — agrupa casos (anidables).
- `skip(nombre, fn)` / `todo(nombre)` — marcar pendientes.
- `only(nombre, fn)` — ejecutar solo ese (útil en desarrollo).
- Hooks: `before_all(fn)`, `after_all(fn)`, `before_each(fn)`, `after_each(fn)`.

### Aserciones
- `assert(cond, msg?)` — falla si `cond` es falso.
- `assert_eq(actual, esperado, msg?)` / `assert_ne(...)`.
- `assert_throws(fn, msg_esperado?)` — el bloque debe lanzar.
- `assert_close(a, b, eps)` — comparación float.
- `fail(msg)` — falla incondicional.

### Alternativa fluent (TypeScript/Jest style)
- `expect(x).toBe(y)`, `.toEqual(y)`, `.toBeTruthy()`, `.toContain(x)`,
  `.toThrow()`.
- Requiere un tipo `Expectation` con métodos encadenables (más trabajo de
  typeck/emitter). Se puede dejar como **fase 2**.

### Utilidades
- `snapshot(valor)` — snapshot testing (compara contra un `.snap`).
- `bench(nombre, fn)` — micro-benchmark (mide ms/n, no pass/fail).
- `mock(fn)` — stub/spy (fase futura; probablemente módulo `mock`).

---

## 3. Intrínsecos de host (canal runtime ↔ runner)

El runner (`clx test`) necesita recibir los resultados por test, no solo el
exit code. Propuesta de intrínsecos internos `__test_*` (mismo patrón que
`__intr_*`):

- `__test_register(name_ptr, fn_handle, file_ptr)` — el `test(...)` del prelude
  lo llama; el runtime arma la lista.
- `__test_begin(name)` / `__test_end(name, ok)` — delimitan cada caso.
- `__test_fail(msg, span)` — una aserción fallida.
- `__test_note(msg)` — log de diagnóstico.
- Salida por el canal host ya existente (stdout/`output`), en formato
  estructurado (TAP/JSON) que `clx test` parsea.

Flujo: `clx test` compila el archivo con una **entrada sintética** (`__cls_test_main`)
que recorre los tests registrados; el host recolecta `begin/end/fail` y arma el
reporte.

---

## 4. Descubrimiento y CLI

- `testEntry` (ya implementado): array de rutas/globs.
- Convención sugerida además: `**/*.test.clsx` y `tests/**/*.clsx`.
- Flags propuestos para `clx test`:
  - `--filter <substr>` / `--grep <regex>` (regex requiere motor).
  - `--format console|tap|json|junit` (para CI).
  - `--bail` — parar en el primer fail.
  - `--jobs N` — paralelismo (aislar el estado del runtime por proceso).
  - `--watch` — re-ejecutar al cambiar (ya hay watcher en `clx maptype`).
  - `--coverage` (futuro).
- Exit codes: `0` todo ok, `1` fallos, `2` error de compilación.

---

## 5. Semántica y detalles

- **Aislamiento**: cada archivo de test = un módulo compilado independiente
  (evita estado compartido). Los `describe` anidados comparten scope.
- **Reservados**: `test`, `describe`, `it`, `expect`, `assert`, `assert_eq`,
  `assert_ne`, `assert_throws`, `fail`, `skip`, `only`, `todo`, `before_*`,
  `after_*`, `bench`, `mock`, `snapshot`. Todos predefinidos **solo** en modo
  test para no romper código de producción.
- **Spans**: un `assert` fallido debe reportar archivo:línea + mensaje, como los
  errores de runtime (reusar `show_runtime_error`/formato de trace).
- **Exit del test runner**: un test pasa si termina sin `fail`/excepción; un
  `throw` no capturado = test fallido (no crash del proceso).

---

## 6. Roadmap sugerido (por fases)

1. **F1 — Mínimo viable**: `test()`, `assert()`, `assert_eq()`, `fail()`, y el
   canal `__test_*`. `clx test` reporta por caso (no solo exit code).
2. **F2 — Estructura**: `describe`, hooks, `skip`/`only`, `testEntry` globs
   (listo), `--filter`/`--list` (listo), salida TAP/JSON.
3. **F3 — Aserciones ricas**: `assert_throws`, `assert_close`, `expect(...)`.
4. **F4 — Avanzado**: `snapshot`, `bench`, `mock`, `--jobs`, `--watch`,
   `--coverage`.

---

## 7. Preguntas abiertas (a decidir)

1. ¿Nombres reservados en modo test (B) o módulo `test` (A)? ¿O ambos?
2. ¿Sintaxis de bloque (`test "x" { }`) o función (`test("x", fn(){})`)?
3. ¿Aserciones funcionales (`assert_eq`) o fluent (`expect().toBe()`)?
4. ¿Un archivo de test declara su propio `main`-equivalente o el runner lo
   sintetiza desde los `test()` registrados? (propuesta: lo sintetiza).
5. ¿`testEntry` reemplaza los argumentos posicionales o se combinan? (hoy:
   posicionales > `testEntry`).

# Reporte de regresiones — prueba #5: SQLite ORM

> Cierre por prueba: **qué regresiones se encontraron** y **qué soluciones se
> aplicaron** (`examples/patrones.md` §5.2.10).

- **Prueba:** #5 SQLite ORM (`examples/sqlite-orm/`) — ORM agnóstico al motor con
  driver SQLite (FFI), comparado **byte a byte** contra **Python `sqlite3`**.
- **Roles:** dev-pruebas → dev-correcciones → Juez (gate).
- **Veredicto del Juez:** **APROBADO** (reprodujo todo, forzó divergencias para
  descartar falso verde).

---

## Resultado

- **ORM == oráculo Python byte a byte**: `CHECK` 1582 B idénticos; `dump` **553090 B idénticos**; `PRAGMA integrity_check = ok`.
- CRUD, tipos (int/real/text UTF-8 con acentos+emoji/blob/NULL), **batch 10k**,
  transacciones (BEGIN/COMMIT/ROLLBACK), orden/paginación, columna `CArray` de 10k.
- Arquitectura escalable: `driver.clsx` (contrato) + `orm.clsx` (SQL genérico) +
  `sqlite_driver.clsx`; un backend MySQL/Postgres = otra subclase de `Driver`.
  El acceso a SQLite va por un **shim C** (sin punteros en el lado CLS).

## Regresiones encontradas y solución

| ID | Feature | Síntoma | Causa raíz | Solución | Estado |
|----|---------|---------|------------|----------|--------|
| **R1** | Módulos / globales | `export const` de un módulo importado valía **0** dentro de ese mismo módulo (`getKind()=0`) | Al declarar un global `mod::NAME` no se registraba el alias sin cualificar `NAME` (las funciones sí lo hacían) → identificador sin resolver → local 0 | Registrar alias del global (`emit.rs`), mismo criterio que las funciones | ✅ |
| **R2** | Control de flujo `if/elif` | Cadena con **≥2 `elif` + `else`** ejecutaba el `else` de más (`f(2)=D`) | Cada `elif` se cerraba secuencialmente (ramas hermanas, no anidadas) → caía al `else` | Emitir cada `elif` **dentro del `else`** anterior (`emitter/statements.rs::emit_if`) | ✅ |
| **R3** | Strings + FFI | `CString: el argumento contiene NUL` al pasar strings temporales retenidos en `Cell[]` por FFI | **Misma causa que R4** (buffer de strings pisado por el shadow stack) | Fix de layout (R4) | ✅ |
| **R4** | Shadow stack / reporter | Trap + **panic** del reporter (`attempt to multiply with overflow` en `name_at`) con el shadow stack por defecto | El shadow stack y sus slots (`PENDING_CALL_SLOT_ADDR`/`ERROR_SLOT_ADDR`/frames) vivían **dentro del rango de crecimiento del heap**: el bump-allocator los pisaba → `__shadow_ptr`/`name_idx` corruptos. Además el reporter no acotaba índices | (a) mover la región del shadow stack **fuera del heap** (`layout.rs`); (b) `read_shadow_stack`/`name_at` con `checked_*` + límite de frames → **el reporter no puede paniquear** (`engine.rs`) | ✅ |
| **R5** | `Callable` | Un valor `Callable` **no se podía invocar** (`No se puede llamar como función: Callable`) | El typeck no tenía arm para `Type::Callable` y el emisor no ruteaba a `call_indirect` | `Type::Callable` en llamada → `Any` (despacho dinámico por tag) (`typeck/calls.rs`, `emitter/calls.rs`) | ✅ |
| **R6** | FFI inverso | Callback **C→CLS** (`sqlite3_exec`) no expresable | No hay *function pointers*/trampoline host ni tipo `CFunc(...)` | **Feature faltante** documentada (`docs/lenguaje/extension.md`, «Callbacks C→CLS»). No bloquea | 📝 |

## Gaps / bloqueos

- **R6** (arriba): callbacks C→CLS requieren *trampoline* host — feature pendiente.
- **Ruta del `.dll` FFI relativa al CWD** (documentado).
- **Aislamiento de imports**: `import "driver"` se resuelve contra el dir del **entry**, no del módulo sin `cls.json` propio.
- **Herencia cross-module** (`: drv::Driver`) no parsea (hay que usar el nombre no cualificado).
- `Any` no soportado (JIT exige tipo concreto); `CFloat` no soportado.

## Observaciones del Juez (atendidas / anotadas)

- ~~`run.ps1` forzaba `CLS_JIT_TRACE=0`~~ → **removido**: ahora corre con el shadow stack por defecto.
- Docs de `sqlite-orm` actualizadas (findings RESUELTOS).
- **Preexistente**: `throw "boom"` (sin paréntesis) es un **no-op silencioso** (hay que usar `throw("boom")`) — laxitud del parser, fuera de alcance.

## Verificación (al cierre)

- `examples/sqlite-orm/run.ps1`: **OK** (dump 553090 B / CHECK 1582 B byte a byte, integridad ok), **con shadow stack por defecto**. El Juez forzó divergencias (alterar el oráculo) y el runner las detecta.
- `cargo test --workspace`: **145 passed / 0 failed** (incl. `clxb`).
- `run-tests.ps1`: **56 PASS / 0 FAIL** · `run-availible.ps1`: **24 PASS / 1 SKIP**.
- `conformance/run.ps1`: **2302/2302** + fuzz **20000/20000**.
- `brainfuck/run.ps1`: **OK** · `clsh/run.ps1`: **22/25** (D1–D3 arquitectura).
- Juez: verificación independiente (anti-falso-verde, reporter sin panic con trace profundo de 303 frames, REPL/`.clsapp` intactos tras el cambio de layout) → **APROBADO**.

## Archivos tocados (por las correcciones)

`cls-core/src/backend/wasm/engine/emit.rs` (R1),
`cls-core/src/backend/wasm/emitter/statements.rs` (R2),
`cls-core/src/backend/wasm/layout.rs` + `cls-core/src/backend/wasm/mod.rs` + `cls-jit/src/engine.rs` (R4/R3),
`cls-core/src/middleware/typeck/calls.rs` + `cls-core/src/backend/wasm/emitter/calls.rs` (R5),
`docs/lenguaje/extension.md` (R6, documentación).

# Reporte de regresiones — prueba #6: DNS resolver / authoritative

> Cierre por prueba: **qué regresiones se encontraron** y **qué soluciones
> (workarounds) se aplicaron** (`examples/patrones.md` §5.2.10). Rol: **dev de
> pruebas** (documenta los bugs y rodea solo lo inevitable) + **dev de
> correcciones** (arregla la causa raíz en el compilador/JIT/typeck).

- **Prueba:** #6 DNS (`examples/dns/`) — codec + transporte UDP/TCP diferencial
  contra un **oráculo Python independiente** (puro, sin `dnslib`).
- **Autocontención:** CLS puro + FFI a C. Sin `host_calls`, sin punteros, sin red
  externa (servidor autoritativo local `127.0.0.1:15353`).
- **Módulos extraídos a `/modules`:** `net` (transporte UDP/TCP FFI) y `dns`
  (codec DNS RFC 1035 en CLS puro). La prueba es consumidor (§11.5).

---

## Resultado

- **`examples/dns/run.ps1` → `RESULTADO: OK`** (exit 0). Números exactos:
  - **Offline (parser diferencial)**: `cls_offline.txt` vs `offline_expected.txt`
    → **360 líneas idénticas** (13 vectores de casos límite + 8 respuestas
    `answer()` por round-trip).
  - **Builder**: `cls_queries.hex` vs `queries_expected.txt` → **8 líneas byte a
    byte idénticas**.
  - **Live (UDP + retry TCP)**: `cls_live.txt` vs `live_expected.txt` →
    **173 líneas idénticas**; queries recibidas por el server
    (`out/queries.log`) → **10 idénticas**.
  - **nslookup** (oráculo CLI opcional): **SKIP** — Windows `nslookup` no admite
    puerto no estándar (no soporta `-port` ni `set port`).
- Cubre: A/AAAA (RFC 5952)/CNAME+chase/MX/TXT (multi-string)/NS/SOA, **compresión
  de nombres** (punteros `0xC0`, cadenas puntero→puntero, punteros a RDATA),
  **NXDOMAIN**, **TC + retry TCP**, flags raros, nombre largo, **endianness y
  parseo binario** (`int[]`).

## Regresiones encontradas — RESUELTAS (findings del lenguaje)

Los 3 findings quedaron **RESUELTOS**; los workarounds de `modules/dns`,
`modules/net` y `examples/dns` se quitaron.

| ID | Feature | Síntoma original | Repro | Causa raíz / fix | Workaround quitado |
|----|---------|------------------|-------|------------------|--------------------|
| **F1** | Arrays como argumento | Mutar un array **parámetro** no afectaba al array del llamador cuando `push` crecía el buffer (pass-by-value). | `findings/01-array-param-by-value.clsx` | `push`/`unshift` reubicaban el buffer y el write-back solo tocaba al receiver. **Fix:** el buffer viejo queda como *forward* (`FWD_MAGIC`) + `arr_resolve` idempotente en `__intr_arr_*` + `__intr_arr_deref` resuelto en `emit_expression` → identidad de array estable (semántica de referencia). `cls-internals/wasm/src/arrays.rs`, `emitter/expressions.rs`, `abi.rs`. | Helpers que **devuelven** el array (`out = pushU16(out, v)`) → mutadores in-place. |
| **F2** | Strings (concatenación) | `out = ""; out = out + a` con `a` largo **corrompía bytes** (longitud ok, contenido mal). También al acumular strings largos en bucles. | `findings/02-string-self-concat.clsx` | El slow path de `__intr_str_append` calculaba la capacidad con la longitud **vieja** (ignoraba la pieza) → sub-alocaba y desbordaba. **Fix:** capacidad = longitud total; header de capacidad `[MAGIC64][cap]` para no confundir literales del pool con buffers con slack. `cls-internals/wasm/src/strings.rs`. | Concatenar **1 byte a la vez** (`appendStr`) → `+` normal. |
| **F3** | `is` / módulos | `x is mod::Clase` (clase **importada**) → error `'is' con 'mod::Clase': se esperaba un enum o structure en el JIT`. | `findings/03-is-qualified-class.clsx` (+ `isq_mod/`) | La clase importada aparecía duplicada con `class_id`/`extends` inconsistentes y `is` solo aceptaba `Identifier`. **Fix:** identidad de clase por definición (`class_id_by_span`), `resolve_parent_key`, `is` acepta `NamespaceAccess`, narrowing del typeck acepta clase cualificada. `engine/emit.rs`, `emitter/binary.rs`, `emitter/mod.rs`, `typeck/flow.rs`. | Accesores **virtuales** (`Rr.ipText()`, `nameTarget()`, ...) → `is` + narrowing (`r1 is dns::RrA` → `r1.ip`). |

> Detalle (comandos, causa raíz por archivo, evidencia) en
> `examples/dns/findings/README.md`.
> **F2** era el más severo: afectaba a cualquier builder de strings largos
> (volcado hex, `join`, render) y solo se detectaba por comparación **byte a
> byte** con el oráculo.

## Deudas / limitaciones (no bugs)

- **Ruta del `.dll` FFI relativa al CWD** (documentado; correr desde la raíz).
- **`nslookup` Windows** sin soporte de puerto no estándar (oráculo CLI → SKIP).

## Verificación (al cierre)

- `examples/dns/run.ps1`: **OK** (360/8/173/10 líneas byte a byte; ver arriba).
- **Sin regresiones** en las suites existentes:
  - `cargo test --workspace`: **0 failed**.
  - `examples/test/jit-test/run-tests.ps1`: **57 PASS / 0 FAIL**.
  - `examples/test/jit-test/run-availible.ps1`: **24 PASS / 1 SKIP**.
  - `examples/conformance/run.ps1`: **2302/2302** + fuzz **20000/20000**.
  - `examples/brainfuck/run.ps1`: **OK**. `examples/lang-features/run.ps1`: **OK**
    (6/6 negativos, 2 GAP). `examples/sqlite-orm/run.ps1`: **OK** (1582 B / 553090 B /
    integrity ok). `examples/clsh/run.ps1`: **22/25** (3 divergencias de arquitectura,
    igual que antes).

## Archivos creados

- `modules/net/` — `net_shim.c`, `build.ps1`, `mod.clsx`, `cls.json`, `README.md`.
- `modules/dns/` — `mod.clsx`, `cls.json`, `README.md`.
- `examples/dns/` — `cls.json`, `run.ps1`, `README.md`, `src/main.clsx`,
  `src/probe_udp.clsx`, `oracle/{dns_indep,zone,server,vector}.py`,
  `oracle/zone.json`, `tests/README.md`, `findings/*` + `findings/README.md`.
- `modules/README.md` — tabla actualizada (`net`, `dns`).

> **Fix de lenguaje (Rust) en esta ronda**: los findings F1–F3 se resolvieron en
> `cls-core` (emitter/engine/typeck) y `cls-internals` (arrays/strings), sin
> `host_calls` nuevos, sin exponer punteros y sin trucos. Los workarounds se
> quitaron de `modules/dns`, `modules/net` y `examples/dns`.

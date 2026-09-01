# Estado del runtime — WASI, async y límites del JIT

> **Propósito**: documentar el estado REAL de capacidades que suelen darse por
> sentadas pero que en CLS 2.0 (JIT) tienen restricciones. Un desarrollador debe
> leer esto antes de diseñar una feature de red/I/O/concurrencia.
>
> **Regla**: esta documentación solo cubre lo implementado y accesible hoy
> (ver convención en `docs/README.md`). Si algo no está, es porque no existe.

## WASI — NO implementado

- El kind `Wasi` existe en la gramática de `extension` (ver
  `docs/lenguaje/extension.md`) pero **NO está implementado** en el JIT.
- Declarar `extension "..." as Wasi` hoy falla o se ignora; solo el kind `C`
  está funcional.
- **File I/O**: NO se hace vía WASI. Se usa el módulo desktop `fs` del nodo
  (`import "fs" as fs`), que es un host call del nodo (ver
  `docs/stdlib/desktop.md`).
- **Consecuencia para features nuevas**: no diseñar nada que dependa de
  `wasi_snapshot_preview1`. Lo que necesite I/O de archivos usa `fs`; lo que
  necesite red/sockets usa `extension` + `when` (patrón de
  `docs/lenguaje/extension-when.md`).

## Async / concurrencia — NO existe en el JIT

- **No hay `async`/`await` en el JIT** (solo el walker deprecado lo tenía
  planeado; ver `agent-context/ASYNC_PLAN.md`).
- El módulo `async` del core (mencionado en `docs/stdlib/core.md`) NO está
  implementado en el JIT.
- Un programa CLS es **bloqueante**: una llamada a socket/lectura bloquea hasta
  completar.
- **Servidor HTTP**: single-thread por request. La concurrencia (threads por
  request vía `extension` a `pthread_create`, o multiplexación con
  `select`/`poll`) es trabajo futuro — documentado como limitación en el
  framework (`docs/desarrollo/minilaravel.md`), NO implementado en v1.

## Límites del JIT relevantes para features nuevas

| Capacidad | Estado | Dónde |
|---|---|---|
| FFI a librerías nativas (`extension as C`) | ✅ Implementado, hasta 16 args por llamada (decisión 002) | `docs/lenguaje/extension.md` |
| Sockets TCP portables | ✅ Patrón `when (os:)` + `extension` documentado con ejemplo completo | `docs/lenguaje/extension-when.md` |
| File I/O | ✅ Vía módulo desktop `fs` (host call del nodo) | `docs/stdlib/desktop.md` |
| WASI | ❌ Kind declarado, NO implementado | — |
| async/await | ❌ No existe en el JIT | `agent-context/ASYNC_PLAN.md` |
| Threads | ❌ No hay API de threads en el lenguaje; posible vía `extension` a `pthread_create` (no documentado como API) | — |
| HTTP cliente | ✅ `import "http" as http` (`get`/`post`) | `docs/stdlib/desktop.md` |
| HTTP servidor | 🟡 Framework en `.clsx` puro (especificación delegada) | `docs/desarrollo/minilaravel.md` |

## Por qué (restricción arquitectónica)

El host es deliberadamente delgado: solo hace lo que WASM/WASI no puede (FFI a
DLLs/SOs vía `extension`, I/O de archivos vía `fs`, I/O básico). Cualquier
feature de red/HTTP/TCP/sockets se implementa como `.clsx` puro usando `when` +
`extension` — ver la restricción permanente en
`agent-context/dev2/09-REMAINING-TASKS-FASE7.md`. PRs que toquen `host_net_*`
o libloading para sockets se rechazan.

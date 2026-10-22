# `negative/` — accesos inválidos que DEBEN ser rechazados

Cada archivo de esta carpeta ejercita un acceso **inválido** que el typeck/emisor
debe rechazar en **compile-time**. `run.ps1` los ejecuta y exige **exit ≠ 0**;
si alguno compila (exit 0), es un fallo de la matriz (control de acceso
perdido).

| Archivo | Qué prueba | Error esperado |
|---|---|---|
| `neg-private-field.clsx` | leer `private` desde fuera de la clase | acceso privado |
| `neg-private-method.clsx` | llamar `private` desde fuera | acceso privado |
| `neg-protected-field.clsx` | leer `protected` desde fuera (sin subclase) | acceso protegido |
| `neg-readonly-write.clsx` | escribir `readonly` desde fuera | escritura readonly |
| `neg-unknown-member.clsx` | campo inexistente | miembro no existe |
| `neg-tuple-write.clsx` | escribir un índice de tupla (inmutable) | tupla inmutable |

Los accesos **válidos** equivalentes se prueban en
`src/sections/oop.clsx` (public desde fuera, private vía método público,
protected vía subclase, static vía clase, readonly lectura).

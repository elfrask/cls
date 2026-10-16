# `/modules` — módulos compartidos de CLS

Módulos **CLS puro** reutilizables entre los proyectos de prueba de `examples/`.
Son los ladrillos de la **stdlib autocontenida**: se apoyan solo en
intrínsecos primitivos, `print`/`input` y **FFI a C** (ver
`examples/patrones.md` §2 y §11).

## Reglas

1. **CLS puro + FFI.** Nada de `host_calls` ni intrínsecos nuevos. Si falta una
   capacidad, se resuelve con FFI (`extension`), no desde el nodo.
2. **Sin dependencias externas** fuera de CLS + FFI.
3. **Un módulo = una carpeta** en `modules/<nombre>/` con su `cls.json`.
4. **No copiar/pegar** módulos entre proyectos: se referencian (ver abajo).
5. Si un módulo reemplaza a un `host_call` (`fs`, `http`, `json`, `strings`),
   se anota en la tabla de deuda de `examples/patrones.md` §2.7.

## Estructura de un módulo

```
modules/<nombre>/
├── cls.json          # { "name": "...", "version": "...", "export": { "entry": "mod.clsx" } }
└── mod.clsx          # API pública (export function/var/const/class/enum/structure)
```

Si el módulo tiene más de un archivo, `export.entry` apunta al index y sus
imports internos se resuelven desde la raíz del módulo (aislamiento).

## Cómo consumirlo desde un proyecto

Como `modules/` está fuera de cada proyecto (`examples/<prueba>/`), hay que
indicar la ruta de búsqueda con **una** de estas dos formas (o ambas):

**Flag de CLI** (relativa al CWD):

```bash
clx run examples/brainfuck/src/main.clsx --add-path-module=modules
clx run app.clsx --add-path-module=modules --add-path-module=libs
```

**`cls.json` del proyecto** (`pathModules`, relativa a la raíz del proyecto):

```json
{
  "name": "brainfuck",
  "entry": "src/main.clsx",
  "pathModules": ["../../modules"]
}
```

Luego, en el código:

```clx
from "str" import *;                 # usa modules/str/
import "sha256" as sha;              # usa modules/sha256/  -> sha::hash(...)
```

## Módulos

| Módulo | Estado | Reemplaza a | Notas |
|--------|--------|-------------|-------|
| `diag` | Activo | — | CLS puro. Diagnóstico para runners de prueba: `hexByte`, `bytesToHex`. Usado por `examples/brainfuck`. |
| `config` | Activo | — | CLS puro. Introspección de la configuración del host **solo lectura**: `all`, `get`, `getBool`, `getInt`, `sandboxEnabled`. Envuelve el intrinsic `config()` con caché perezosa (una sola llamada al host). |
| `orm` | Activo | — | CLS puro. Capa ORM **agnóstica al motor** + contrato de driver (`Cell`, `ResultSet`, `Driver` base) y `Orm` (CRUD, query builder, transacciones, bulk). Backends enchufables. Extraído de `examples/sqlite-orm`. |
| `sqlite` | Activo | — | **FFI a C**. Backend SQLite (`SqliteDriver: Driver`) sobre el shim `sqlite_shim.dll` (sin punteros; handles enteros). Consume `orm`. Requiere `powershell -File modules/sqlite/build.ps1`. |

## Excepción sancionada: `config()`

`config` es un **intrinsic de introspección autorizado** (excepción explícita a
la regla 5.1.6 de no tocar el host): no añade una *feature* del lenguaje, solo
**lee parámetros del host** (`cls.json` + flags de CLI) y los expone como un
`Record<String, String>` plano de solo lectura. El módulo `config/` es la capa
CLS idiomática sobre él (caché perezosa + helpers tipados). Ver
`docs/guia/configuracion.md`.


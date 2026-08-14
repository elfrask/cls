# Runtime ligero (`clxr`)

`clxr` es el runtime de ejecución de apps del nodo liviano: **sin herramientas
de desarrollo**. Ejecuta código `.clsx` directamente o apps empaquetadas
`.clsapp`.

## Uso

```
clxr <archivo.clsx|app.clsapp> [args...]
```

Sin argumentos imprime la ayuda:

```
clxr 2.0 — CLS Runtime Executor
Uso: clxr <archivo> [args...]
  .clsx  → ejecucion directa
  .clsapp → extrae y ejecuta (formato zip)
```

## Entradas

### `.clsx` — ejecución directa

El archivo se lee como texto y se ejecuta. Es lo mismo que el walker de
`clx run`, sin los flags.

### `.clsapp` — app empaquetada

Es un **zip** con un `manifest.json`:

1. Se abre el zip (vía `ZipFs`) y se registra en el VFS como `res://`.
2. Se lee `manifest.json` → campo `entry` (default: `source.clsx`).
3. Se ejecuta el source del entry desde el zip.

Si el zip es inválido o el entry no existe, error y exit 1.

## Motor

- **Tree-walker** (`Interpreter`), no el JIT, con la **stdlib core**
  (`math`, `json`, `async`). No carga los módulos del nodo desktop
  (`fs`, `http`, `os`, `path`, `process`, `time`, `random`, `Lib`).
- Intrinsics desktop default (`print`, `input`).

## Argumentos

Los args tras el archivo se pasan a `main(args: String[])` y quedan
disponibles para el runtime:

```clx
function main(args: String[]) -> int {
    print("hola", args[0]);
    return 0;
}
```

```sh
clxr app.clsx mundo   # imprime "hola mundo"
```

## VFS base

Se prepara un VFS de tres raíces para cualquier I/O por protocolo
(ver `runtime/vfs.md`):

| Raíz | Destino |
|---|---|
| `app://` | directorio de trabajo actual (CWD) |
| `user://` | `$HOME` / `$USERPROFILE` |
| `tmp://` | directorio temporal del sistema |

Nota: aunque el VFS se construye, el resolver de `clxr` solo expone el core;
los módulos desktop (que usan el VFS) no están disponibles.

## Errores y exit code

- Errores de sintaxis: `show_syntax_error` (línea + caret).
- Errores de runtime: `show_runtime_error` con el **trace completo**
  (import trace + call stack numerado con código por frame).
- El **exit code del proceso es el de `main`** (si `main` no devuelve `int`,
  se usa `0`; si no hay `main`, `0`). Los errores devuelven `1`.

Fuente: `nodos/clxr/src/main.rs`.
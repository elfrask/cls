# Language Server (LSP)

`clx lsp` inicia un servidor de lenguaje [LSP] sobre **stdin/stdout**
(tower-lsp). No soporta TCP; el cliente lo debe lanzar como proceso y hablar
por los pipes estándar.

```
clx lsp            # servidor activo; log "[clx lsp] ready" en stderr
clx lsp --silent   # silencia el log de arranque (-s también funciona)
```

El servidor se identifica como `clx-lsp 2.0.0`.

## Capacidades

| Capacidad | Detalle |
|---|---|
| Text document sync | `FULL` — el documento completo se envía en cada cambio. |
| Completion | Trigger chars: `.`, `"`, `/`. |
| Hover | Docs de las type definitions (`.clsi`). |
| Go-to-definition | Símbolos declarados en el documento actual. |
| Document symbols | Funciones, variables y parámetros (flat). |
| Diagnostics | Publicados con `source: "clx"`; se limpian al cerrar el documento. |

## Diagnósticos

Pipeline por documento abierto o cambiado (`did_open` / `did_change`):

1. **Lexer** → errores de sintaxis.
2. **Parser** → errores de sintaxis/estructurales.
3. **NameResolver** → errores de nombres.
4. **TypeChecker** → errores y warnings de tipos.

La configuración de tipos se toma de `cls.json` del workspace
(`compiler.types`); si no existe, se usa el default
(`check: true, strict: false`).

## Autocompletado

- **Tras `.`** (miembros): miembros del módulo importado o del módulo por
  nombre (desde las type definitions), exports **en vivo** de `math`, `json`,
  `fs` y `http`, y campos de `structure`s del documento.
- **Sin `.`**: símbolos del scope del documento (detalle `scope`), símbolos de
  otros documentos abiertos (detalle `open-document`), keywords hardcodeadas
  (`var`, `function`, `if`, `while`, `for`, `return`, `import`, `from`, `as`,
  `export`, `structure`, `interface`, `true`, `false`, `null`, `break`,
  `continue`, `loop`, `switch`), intrinsics de `core.clsi` (con firma y doc),
  módulos de las type definitions y archivos `.clsx` del workspace
  (detalle `workspace/<ruta>`).

## Hover

Busca la palabra bajo el cursor en las type definitions y devuelve markdown con
la firma y los tags documentados del `.clsi`:

- `@description` → párrafo.
- `@params nombre desc` → ítem de lista.
- `@return ...` → flecha `→`.
- `@deprecated ...` → texto tachado.

Si la palabra no está en las definiciones, devuelve el nombre entre backticks.

## Type definitions

- **Builtins embebidos** (12): `core`, `math`, `json`, `fs`, `http`, `Lib`,
  `async`, `os`, `path`, `process`, `time`, `random` — los `.clsi` de
  `cls-runtime/clsi/` se distribuyen dentro del binario (`include_str!`).
- **Override de usuario**: los `.clsi` del directorio `clsi/` del workspace
  tienen prioridad y se cargan además de los builtins.

Fuente: `nodos/clx/src/lsp.rs` y `nodos/clx/src/type_defs.rs`.

[LSP]: https://microsoft.github.io/language-server-protocol/
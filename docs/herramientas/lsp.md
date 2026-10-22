# Language Server (LSP)

`clx lsp` inicia un servidor de lenguaje [LSP] sobre **stdin/stdout**
(tower-lsp). No soporta TCP; el cliente lo debe lanzar como proceso y hablar
por los pipes estándar.

```
clx lsp            # servidor activo; log "[clx lsp] ready" en stderr
clx lsp --silent   # silencia el log de arranque (-s también funciona)
```

El servidor se identifica como `clx-lsp` con la versión del binario.

## Capacidades

| Capacidad | Detalle |
|---|---|
| Text document sync | `FULL` - el documento completo se envía en cada cambio. |
| Completion | Trigger chars: `.`, `:`, `"`. También funciona con `Ctrl+Space` después de un `.`. |
| Hover | Símbolos locales, imports y type definitions (`.clsi`). |
| Go-to-definition | Símbolos locales, imports relativos/módulos y búsqueda única en el workspace. |
| Document symbols | Jerárquico: funciones, variables, clases, structs, interfaces, enums y sus miembros. |
| Diagnostics | Publicados con `source: "clx"`; se limpian al cerrar el documento. |

## Diagnósticos

Pipeline por documento abierto, cambiado o guardado (`did_open`,
`did_change`, `did_save`):

1. **Lexer** -> errores de sintaxis.
2. **Parser** -> errores de sintaxis/estructurales.
3. **NameResolver** -> errores de nombres.
4. **TypeChecker** -> errores y warnings de tipos.

La configuración de tipos se toma de `cls.json` del workspace
(`compiler.types`); si no existe, se usa el default
(`check: true, strict: false`).

## Autocompletado

- **Tras `.` o `::`** (miembros): resuelve cadenas como `obj.campo.`,
  `modulo.miembro.` y `namespace::` usando imports, variables locales,
  structs/clases y las type definitions.
- **Sin `.`**: símbolos del documento actual (detalle `scope`), símbolos de
  otros documentos abiertos (detalle `open-document`), keywords, intrinsics de
  `core.clsi`, módulos builtin y archivos `.clsx`/`.clsi` del workspace.
- Los módulos del workspace se indexan recursivamente y se identifican por
  nombre de archivo (o por el directorio si el archivo es `mod.clsx`).

## Hover

Busca la palabra bajo el cursor en:

1. Símbolos del documento actual.
2. Imports y archivos del workspace.
3. Type definitions (`.clsi`).

Devuelve markdown con la firma y los tags documentados:

- `@description` -> párrafo.
- `@params nombre desc` -> ítem de lista.
- `@return ...` -> flecha `->`.
- `@deprecated ...` -> texto tachado.

## Go-to-definition

- Símbolos declarados en el documento actual.
- Símbolos importados con `import "..." as alias` o `from "..." import x`.
- Símbolos únicos encontrados en el workspace.
- Los builtins embebidos no tienen archivo físico; se documentan vía hover.

## Document symbols

El outline del editor muestra la jerarquía real:

- Funciones y variables.
- Clases con propiedades y métodos.
- Structs e interfaces con campos/firmas.
- Enums con variantes.
- Módulos y namespaces anidados.

## Workspace index

El servidor mantiene un índice de archivos `.clsx` y `.clsi` del workspace:

- Excluye `target/`, `node_modules/`, `.git/`, `.cls-cache/`, `.cls-types/`,
  `out/` y `dist/`.
- Se reconstruye al recibir `workspace/didChangeWatchedFiles` para cambios en
  `.clsx`, `.clsi` o `cls.json`.
- También se actualiza al guardar (`did_save`).

## Type definitions

- **Builtins embebidos** (14): `core`, `math`, `json`, `fs`, `http`, `strings`,
  `Lib`, `async`, `os`, `path`, `process`, `time`, `random`, `types` - los
  `.clsi` de `cls-runtime/clsi/` se distribuyen dentro del binario
  (`include_str!`).
- **Override de usuario**: los `.clsi` del directorio `clsi/` del workspace
  tienen prioridad y se cargan además de los builtins.

## VS Code

La extensión `ccls-lang` lanza `clx lsp --silent` y registra `.clsx` y `.clsi`
como lenguaje `clx`. Settings:

| Setting | Default | Descripción |
|---|---|---|
| `cls.lsp.enabled` | `true` | Inicia el servidor LSP. |
| `cls.lsp.serverPath` | `""` | Ruta al ejecutable `clx`; vacío = PATH, `target/debug`, `target/release` o `~/.cargo/bin`. |
| `cls.lsp.trace.server` | `"off"` | Traza LSP (`off`, `messages`, `verbose`). |
| `cls.types.useStaticTypes` | `true` | Type maps estáticos como fallback si el LSP está desactivado. |

Fuente: `nodos/clx/src/lsp.rs`, `lsp_index.rs`, `lsp_text.rs` y
`type_defs.rs`.

[LSP]: https://microsoft.github.io/language-server-protocol/

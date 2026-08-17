# Generador de type maps (`clx maptype`)

`clx maptype` extrae un **type map** (`.type.json`) con las declaraciones
públicas de cada archivo `.clsx`/`.clsi` sin ejecutar el código: analiza el AST
y los comentarios de documentación `# @tag`.

## Uso

```
clx maptype [path] -o <dir> [--watch|-w]
```

| Opción | Default | Descripción |
|---|---|---|
| `path` | `.` | Archivo `.clsx`/`.clsi` o directorio a procesar (recursivo). |
| `-o` / `--out` | `./.cls-types` | Directorio de salida. |
| `-w` / `--watch` | - | Modo vigilancia: polling cada 2s, regenera si el mtime cambió. |

En modo directorio se respeta la estructura relativa al CWD: cada
`sub/dir/arch.clsx` produce `sub/dir/arch.type.json` bajo la salida. Se omiten
directorios ocultos (empiezan con `.`) y `modules`, `dist`, `libs`, `target`.

En modo archivo, si la salida es un directorio el mapa se escribe como
`{stem}.type.json` dentro; si no, en la ruta exacta. Una entrada que no sea
`.clsx`/`.clsi` es un error (exit 1).

## Formato

Cada archivo genera un JSON:

```json
{
  "source": "src/mod.clsx",
  "entries": [
    {
      "name": "suma",
      "kind": "function",
      "line": 1, "col": 1, "end_line": 1, "end_col": 40,
      "doc": "Suma dos números",
      "version": null,
      "deprecated": null,
      "signature": "suma(a: int, b: int) -> int",
      "params": [ { "name": "a", "type_": "int", "doc": null } ],
      "return_type": "int",
      "return_doc": null,
      "fields": [],
      "members": [],
      "type_": null,
      "value": null
    }
  ]
}
```

### Campos de `TypeEntry`

| Campo | Contenido |
|---|---|
| `name` | Nombre de la declaración. |
| `kind` | Ver tipos de entrada abajo. |
| `line`/`col`/`end_line`/`end_col` | Span de la declaración (1-indexed). |
| `doc` | Último `@description` sobre la declaración. |
| `version` | Valor de `@version`, si existe. |
| `deprecated` | Valor de `@deprecated`, si existe. |
| `signature` | Firma textual (funciones e imports). |
| `params` | `{ name, type_, doc }` por parámetro. |
| `return_type` | Tipo de retorno (funciones). |
| `return_doc` | Texto de `@return`. |
| `fields` | `{ name, type_ }` - campos de `structure` y propiedades de `class`. |
| `members` | Métodos de `interface`/`class`; funciones y variables de `module`/`namespace`. |
| `type_` | Anotación de tipo (variables/constantes). |
| `value` | Siempre `null` (reservado). |

### Kinds

| Kind | Declaración |
|---|---|
| `function` | `function name(...)` |
| `async function` | `async function name(...)` |
| `variable` | `var name` |
| `constant` | `const name` |
| `structure` | `structure name` |
| `interface` | `interface name` |
| `import` | `import "x" as y` / `from "x" import y` (una entrada por símbolo importado) |
| `class` | `class name` (`signature` = `class X extends Y` si hereda) |
| `module` | `module name` |
| `namespace` | `namespace name` |

## Documentación

Los comentarios `# @tag` **antes** de la declaración (el `# @title` del header
del módulo corta el bloque):

| Tag | Uso |
|---|---|
| `# @title` | Título del módulo; separa el header de la documentación de funciones. |
| `# @description texto` | Descripción (se usa la última antes de la declaración). |
| `# @params nombre desc` | Doc de un parámetro. |
| `# @return tipo desc` | Doc del retorno (se descarta la primera palabra como tipo). |
| `# @version N` | Versión de la entrada. |
| `# @deprecated msg` | Marcado de deprecación. |

## Salida

En modo directorio (stderr):

```
Generando type maps desde 'X' -> 'Y'...
  <in> -> <out> (N entradas)
Completado.
```

En modo watch se imprime además `Watch mode activo (polling cada 2s)...` y se
regeneran solo los archivos cuyo mtime cambió.

Fuente: `nodos/clx/src/subcommands/maptype.rs`.
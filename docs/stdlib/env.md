# Módulo estándar `env`

`modules/env` carga archivos `.env` sin modificar el entorno del proceso. Está
pensado para configuración de aplicaciones y se distribuye con los módulos
estándar de CLS.

```clx
import "env" as env;

var values = env.load(".env");
var name = values.get("APP_NAME");
var port = values.get_int("PORT");
var debug = values.get_bool("DEBUG");
```

## API

| Símbolo | Firma | Descripción |
|---|---|---|
| `load` | `load(path: String) -> Env` | Lee y parsea el archivo; si no existe devuelve un `Env` vacío. |
| `load_default` | `load_default() -> Env` | Equivale a `load(".env")`. |
| `parse` | `parse(text: String, path: String) -> Env` | Parsea texto; útil para tests. |
| `merge` | `merge(base: Env, override: Env) -> Env` | Combina dos mapas; gana `override`. |

## `Env`

| Método | Descripción |
|---|---|
| `get(name) -> String` | Valor o `""`. |
| `require(name) -> String` | Valor o error de runtime. |
| `has(name) -> bool` | Indica si existe la clave. |
| `get_int(name) -> int` | Conversión entera. |
| `get_bool(name) -> bool` | `1`, `true`, `yes` u `on` son verdaderos. |
| `set(name, value) -> Env` | Agrega/actualiza en memoria. |
| `all() -> Record<String, String>` | Copia del mapa cargado. |

## Formato

- `KEY=value`
- `export KEY=value`
- Comillas simples o dobles: `KEY="texto con # y espacios"`
- Comentarios desde `#` en líneas no citadas.
- Líneas vacías ignoradas.
- Las rutas son relativas al CWD del proceso.

## Relación con `system_env`

`env` no escribe variables del sistema. Si una aplicación necesita priorizar el
entorno real, debe consultarlo explícitamente con `system_env` y combinar el
resultado con `Env`:

```clx
var e = env.load(".env");
var db = system_env("APP_DB");
if (db == "") { db = e.get("APP_DB"); };
```

Ver también `stdlib/core.md` y `guia/configuracion.md`.

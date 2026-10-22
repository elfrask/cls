# env - variables de entorno desde `.env`

Modulo estandar de carga de `.env`. No modifica el entorno del proceso; devuelve
un objeto `Env` con los valores, y las variables del sistema se siguen leyendo
con `system_env`.

```clx
import "env" as env;

var e = env.load(".env");
var name = e.get("APP_NAME");
var port = e.get_int("PORT");
var debug = e.get_bool("DEBUG");
```

## API

| Simbolo | Descripcion |
|---|---|
| `load(path)` | Lee y parsea el archivo; si no existe devuelve `Env` vacio |
| `load_default()` | `load(".env")` |
| `parse(text, path)` | Parsea texto (util para tests) |
| `merge(base, override)` | Combina dos `Env`, gana `override` |
| `Env.get(name)` | Valor o `""` |
| `Env.require(name)` | Valor o error |
| `Env.has(name)` | Existencia |
| `Env.get_int` / `Env.get_bool` | Conversion |
| `Env.set` / `Env.all` | Escritura/lectura del mapa |

Soporta `KEY=value`, comillas simples/dobles, `export KEY=...` y comentarios
con `#` en lineas no citadas.

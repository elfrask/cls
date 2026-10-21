# `sqlite` — backend SQLite del ORM (FFI a C)

Módulo **CLS + FFI** que implementa el contrato de driver de `modules/orm`
sobre la API C de SQLite, a través del shim `sqlite_shim.c`
(`sqlite_shim.dll`). Es la mitad específica de motor de la prueba #5
(`examples/sqlite-orm`), extraída a `/modules` (ver `examples/patrones.md` §11).

No usa `host_calls`: el acceso a SQLite es **FFI puro**. El shim envuelve la API
*pointer-heavy* de SQLite (`sqlite3*`, `sqlite3_stmt*`, `char**`) en funciones
que solo reciben/devuelven escalares y strings; CLS referencia conexiones,
sentencias y result sets por **handles enteros** (nunca ve punteros).

## API

| Símbolo | Descripción |
|---|---|
| `SqliteDriver: Driver` | backend: `connect`/`close`/`exec`/`query`/`queryBind`/`prepare`/`bind`/`step`/`resetStmt`/`finalizeStmt`/`lastId`/`changes`/`integrity`/`version` |
| `removeFile(path) -> int` | borra un archivo (via `remove` de la libc) |

Uso:

```clx
import "orm" as orm;
import "sqlite" as sq;

var driver = sq::SqliteDriver();
driver.connect("app.db");
var o = orm::Orm(driver);
```

## Compilar / consumir

```powershell
# Compila sqlite_shim.dll y copia sqlite3.dll junto al shim (desde cualquier CWD)
powershell -File modules\sqlite\build.ps1
```

`build.ps1`:

1. localiza `sqlite3.dll` en el runtime de Python
   (`sys.prefix\DLLs\sqlite3.dll`) y lo **copia** a `modules/sqlite/`;
2. exporta `CLS_SQLITE_PATH` en la sesión actual;
3. compila `sqlite_shim.dll` desde `sqlite_shim.c` si falta o está viejo
   (requiere `gcc` de MinGW-w64 x64 en `PATH`).

El shim se **autolocaliza** (`GetModuleFileNameW`) y busca `sqlite3.dll` primero
junto a él, luego en `%CLS_SQLITE_PATH%` y por último en `PATH`, resolviendo los
símbolos con `GetProcAddress` (sin dependencia de import estática). Por eso la
copia de `sqlite3.dll` junto al shim basta; `CLS_SQLITE_PATH` es respaldo.

> El shim llama `sqlite3_initialize()` antes de la primera operación: el build de
> SQLite de Python se compila con `SQLITE_OMIT_AUTOINIT`.

## Limitaciones conocidas

- **La ruta del `.dll` FFI se resuelve contra la raíz del paquete**, no contra el CWD.
  `extension "./sqlite_shim.dll"` se busca en el `cls.json` del módulo.
- **Callbacks C→CLS** (`sqlite3_exec` con callback): no expresables hoy (no hay
  *function pointers* en el FFI). Ver `examples/sqlite-orm/findings/06`.

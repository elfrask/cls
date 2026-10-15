# Prueba #5 — SQLite ORM (CLS + FFI a C, oráculo externo)

Prueba diferencial del lenguaje **CLS 2.0** (JIT). Construye un **ORM** cuyo
núcleo es **agnóstico al motor**, con un **driver SQLite** enchufable, y valida
todo el resultado contra un **oráculo externo independiente**: el módulo
`sqlite3` de Python (la misma `libsqlite3` del sistema).

Reglas que respeta (ver `examples/patrones.md` §2, §5, §11):

- **Autocontención**: código CLS puro + **FFI a C**. **No** usa
  `fs`/`http`/`json`/`strings` ni ningún `host_call` nuevo.
- **Sin punteros expuestos al lenguaje**: la API C de SQLite es *pointer-heavy*
  (`sqlite3*`, `sqlite3_stmt*`, `char**`); un **shim C** la envuelve en
  funciones que sólo reciben/devuelven escalares (enteros/doubles) y strings.
  CLS referencia conexiones, sentencias y result sets por **handles enteros**.
- **Oráculo externo obligatorio**: Python `sqlite3`.
- **Sin trucos**: no se toca el compilador ni nada fuera de `examples/sqlite-orm/`.

---

## Arquitectura

```
examples/sqlite-orm/
├── cls.json                 # entry + pathModules
├── run.ps1                  # compila shim, corre CLS + Python, compara byte a byte
├── src/
│   ├── main.clsx            # runner: carga de trabajo + protocolo CHECK
│   ├── orm.clsx             # ORM genérico (CRUD, query builder, transacciones)
│   ├── driver.clsx          # contrato de driver (Cell, ResultSet, Driver base)
│   └── sqlite_driver.clsx   # backend SQLite (usa el shim por FFI)
├── oracle/
│   ├── sqlite_shim.c        # shim C "sin punteros" sobre la API de SQLite
│   ├── sqlite_shim.dll      # (build)
│   ├── oracle.py            # oráculo Python (construye py.db, dump canónico)
│   └── sqlite3.dll          # (build) copia del runtime de Python
├── tests/README.md          # los casos viven en src/main.clsx (ver abajo)
├── out/                     # cls.db, py.db, dumps y CHECKs (artefactos)
└── findings/                # repros mínimos de regresiones del lenguaje
```

### Capas

1. **`driver.clsx`** — el contrato. Define:
   - `Cell`: valor neutro al motor `{kind, i, f, s}` donde `kind` es
     `1 NULL, 2 INT, 3 FLOAT, 4 TEXT, 5 BLOB` (el BLOB viaja como **hex**).
   - `ResultSet`: filas × columnas materializadas como `Cell[]`.
   - `Driver` (**clase base**): los métodos que todo backend debe implementar
     (ver *Contrato* abajo).

2. **`orm.clsx`** — la capa genérica. **No contiene SQL específico de SQLite**:
   emite SQL estándar (`CREATE TABLE`, `INSERT ... VALUES (?, ...)`,
   `SELECT ... WHERE ... ORDER BY ... LIMIT ... OFFSET ...`, `BEGIN/COMMIT/
   ROLLBACK`) y delega la ejecución en un `Driver`. Usa *binding* de parámetros
   (`?`), nunca interpola valores en el texto.

3. **`sqlite_driver.clsx`** — `SqliteDriver: Driver`. Traduce cada `Cell` a
   `o_bind_*` del shim y materializa los `ResultSet` desde el shim.

4. **`oracle/sqlite_shim.c`** — shim C sobre `sqlite3.dll`. Se **autolocaliza**
   (`GetModuleFileNameW`) y carga `sqlite3.dll` desde `%CLS_SQLITE_PATH%`, su
   propio directorio o el PATH, resolviendo símbolos con `GetProcAddress`
   (sin dependencia de import estática). Nota: el build de SQLite de Python se
   compila con `SQLITE_OMIT_AUTOINIT`, por lo que el shim llama
   `sqlite3_initialize()` antes de la primera operación.

5. **`oracle/oracle.py`** — oráculo externo. Reproduce la misma carga de trabajo
   con la stdlib `sqlite3` y produce:
   - `run <db>`: construye la base y emite las mismas líneas `CHECK k=v` que CLS
     (valida la **lectura** vía ORM).
   - `dump <db>`: volcado canónico del contenido (filas por `rowid`, UTF-8 crudo,
     blobs en hex) para comparar **byte a byte**.

### Cómo valida (`run.ps1`)

1. CLS construye `out/cls.db` a través del ORM y emite `CHECK k=v`.
2. Python construye `out/py.db` con las mismas operaciones y emite los mismos
   `CHECK k=v`.
3. Se comparan **byte a byte**: (a) los CHECK de CLS vs Python, (b) `dump(cls.db)`
   vs `dump(py.db)`, y (c) `PRAGMA integrity_check(cls.db) == ok`.

Si algo diverge, `run.ps1` sale con código != 0.

---

## Contrato de driver (para agregar MySQL/Postgres)

Un backend nuevo se implementa **heredando de `Driver`** y sobrescribiendo:

| Método | Firma | Debe hacer |
|---|---|---|
| `connect` | `(path: String) -> int` | abrir conexión; devolver handle (>=0 ok) |
| `close` | `() -> int` | cerrar |
| `exec` | `(sql: String) -> int` | SQL sin resultados (DDL/DML/TX); 0 = ok |
| `query` | `(sql: String) -> ResultSet` | `SELECT` sin parámetros |
| `queryBind` | `(sql: String, params: Cell[]) -> ResultSet` | `SELECT` con `?` |
| `prepare` | `(sql: String) -> int` | sentencia preparada (handle) |
| `bind` | `(stmt: int, idx: int, cell: Cell) -> int` | enlazar parámetro `idx` (1-based) |
| `step` | `(stmt: int) -> int` | 1 fila / 0 fin / <0 error |
| `resetStmt` | `(stmt: int) -> int` | reutilizar tras `step` |
| `finalizeStmt` | `(stmt: int) -> int` | liberar |
| `lastId` | `() -> int` | last insert id |
| `changes` | `() -> int` | filas afectadas del último DML |
| `integrity` | `() -> String` | chequeo de integridad del motor |
| `version` | `() -> String` | versión del motor |

El ORM sólo usa estos métodos; para un backend MySQL/Postgres basta con crear
`mysql_driver.clsx` (`class MysqlDriver: Driver { ... }`) y pasarlo a
`Orm(driver)` en `main.clsx`. El SQL que emite `orm.clsx` es estándar.

> **Nota (regresión)**: el patrón alternativo "driver = `Record<String, Callable>`"
> **no es viable hoy** porque CLS no permite invocar un valor tipado como
> `Callable` (ver `findings/05`). Por eso el driver es una **clase base**.

---

## Limitaciones conocidas (documentadas, no bugs de la prueba)

- **Ruta del `.dll` FFI = relativa al CWD**, no al archivo `.clsx`. Por eso hay
  que correr desde la **raíz del repo**. `run.ps1` lo hace; para ejecuciones
  manuales, `cd` a la raíz primero.
- `sqlite3.dll` no se versiona: `run.ps1` lo copia desde el runtime de Python a
  `oracle/` (y exporta `CLS_SQLITE_PATH`). Para ejecución manual, tener
  `oracle/sqlite3.dll` o `CLS_SQLITE_PATH` apuntando a la DLL.
- **C→CLS callback** (`sqlite3_exec` con callback): **no es expresable** hoy
  (no hay *function pointers* en el FFI). Ver `findings/06`.

---

## Cómo correr

```powershell
# Desde la raíz del repo
target\debug\clx.exe run examples\sqlite-orm\src\main.clsx --add-path-module=modules
powershell -File examples\sqlite-orm\run.ps1
```

> `main.clsx` corre con el **shadow stack por defecto** (trace on). El bug que
> lo rompía (trap + panic del reporter) quedó **resuelto** — ver `findings/04`.

---

## Cobertura

| Categoría | Dónde | Qué verifica |
|---|---|---|
| CRUD | `testCrud` | CREATE/INSERT single/SELECT todos/por id/WHERE/UPDATE/DELETE |
| Tipos | `testTypes` | INTEGER, REAL, TEXT con **UTF-8 (acentos + emoji)**, BLOB, NULL |
| Batch 10k | `testBatch` | 10.000 filas vía ORM (bulk preparado en 1 TX) + lectura de 10k |
| Transacciones | `testTx` | `BEGIN`/`ROLLBACK` (atomicidad) y `BEGIN`/`COMMIT` |
| Orden/paginación | `testOrder` | `ORDER BY v DESC, id ASC` + `LIMIT 3 OFFSET 2` |
| Integridad | `main` | `PRAGMA integrity_check` == `ok` |
| FFI CArray | shim `o_column_int` | marshalling `CArray` (int64) — ver `findings/` |
| Callback C→CLS | `findings/06` | bloqueo reportado (no expresable) |

El resultado de la última corrida y el **reporte de regresiones** están en
`findings/README.md`.

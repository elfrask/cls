# Prueba #5 — SQLite ORM (CLS + FFI a C, oráculo externo)

Prueba diferencial del lenguaje **CLS 2.0** (JIT). Construye un **ORM** cuyo
núcleo es **agnóstico al motor**, con un **driver SQLite** enchufable, y valida
todo el resultado contra un **oráculo externo independiente**: el módulo
`sqlite3` de Python (la misma `libsqlite3` del sistema).

La prueba es un **consumidor** de los módulos compartidos de `/modules`
(ver `examples/patrones.md` §11.5):

- **`modules/orm`** — capa genérica + contrato de driver (`Cell`, `ResultSet`,
  `Driver` base, `Orm`). Ver `modules/orm/README.md`.
- **`modules/sqlite`** — backend FFI (`SqliteDriver: Driver` + shim C). Ver
  `modules/sqlite/README.md`.

Reglas que respeta (ver `examples/patrones.md` §2, §5, §11):

- **Autocontención**: código CLS puro + **FFI a C**. **No** usa
  `fs`/`http`/`json`/`strings` ni ningún `host_call` nuevo.
- **Sin punteros expuestos al lenguaje**: el shim C envuelve la API de SQLite;
  CLS referencia conexiones, sentencias y result sets por **handles enteros**.
- **Oráculo externo obligatorio**: Python `sqlite3`.
- **Sin trucos**: no se toca el compilador; el runner vive en `src/main.clsx` y
  los módulos en `/modules`.

---

## Arquitectura

```
examples/sqlite-orm/            # consumidor
├── cls.json                    # entry + pathModules: ["../../modules"]
├── run.ps1                     # llama a modules/sqlite/build.ps1, corre CLS + Python, compara
├── src/
│   ├── main.clsx               # runner: carga de trabajo + protocolo CHECK
│   ├── repro-string-inline.clsx# repros de la prueba (usan orm + sqlite)
│   └── repro-string-var.clsx
├── oracle/
│   ├── oracle.py               # oráculo Python (construye py.db, dump canónico)
│   └── sqlite3.dll             # (build) copia del runtime de Python
├── tests/README.md
├── out/                        # cls.db, py.db, dumps y CHECKs (artefactos)
└── findings/                   # repros mínimos / reporte de regresiones

modules/orm/                    # módulo compartido (capa genérica)
├── cls.json
├── mod.clsx                    # consts KIND_*, Cell, ResultSet, Driver, Orm
└── README.md

modules/sqlite/                 # módulo compartido (backend FFI)
├── cls.json
├── mod.clsx                    # SqliteDriver: Driver  (import "orm")
├── sqlite_shim.c               # shim C "sin punteros" (movido desde oracle/)
├── build.ps1                   # compila el shim + copia sqlite3.dll
└── README.md
```

### Capas

1. **`modules/orm`** — el contrato y la capa genérica. Define:
   - `Cell`: valor neutro al motor `{kind, i, f, s}` donde `kind` es
     `1 NULL, 2 INT, 3 FLOAT, 4 TEXT, 5 BLOB` (el BLOB viaja como **hex**).
   - `ResultSet`: filas × columnas materializadas como `Cell[]`.
   - `Driver` (**clase base**): los métodos que todo backend debe implementar
     (ver *Contrato* abajo).
   - `Orm`: emite SQL estándar y delega en un `Driver`.

2. **`modules/sqlite`** — `SqliteDriver: Driver`. Traduce cada `Cell` a
   `o_bind_*` del shim y materializa los `ResultSet` desde el shim. Importa la
   capa genérica (`import "orm" as orm;`).

3. **`modules/sqlite/sqlite_shim.c`** — shim C sobre `sqlite3.dll`. Se
   **autolocaliza** (`GetModuleFileNameW`) y carga `sqlite3.dll` desde
   `%CLS_SQLITE_PATH%`, su propio directorio o el PATH, resolviendo símbolos con
   `GetProcAddress` (sin dependencia de import estática). Nota: el build de
   SQLite de Python se compila con `SQLITE_OMIT_AUTOINIT`, por lo que el shim
   llama `sqlite3_initialize()` antes de la primera operación.

4. **`src/main.clsx`** — el consumidor: importa `orm` y `sqlite`, construye
   `out/cls.db` y emite el protocolo `CHECK k=v`.

5. **`oracle/oracle.py`** — oráculo externo. Reproduce la misma carga de trabajo
   con la stdlib `sqlite3` y produce:
   - `run <db>`: construye la base y emite las mismas líneas `CHECK k=v` que CLS
     (valida la **lectura** vía ORM).
   - `dump <db>`: volcado canónico del contenido (filas por `rowid`, UTF-8 crudo,
     blobs en hex) para comparar **byte a byte**.

### Cómo valida (`run.ps1`)

1. `modules/sqlite/build.ps1` compila `sqlite_shim.dll` y copia `sqlite3.dll`.
2. CLS construye `out/cls.db` a través del ORM y emite `CHECK k=v`.
3. Python construye `out/py.db` con las mismas operaciones y emite los mismos
   `CHECK k=v`.
4. Se comparan **byte a byte**: (a) los CHECK de CLS vs Python, (b) `dump(cls.db)`
   vs `dump(py.db)`, y (c) `PRAGMA integrity_check(cls.db) == ok`.

Si algo diverge, `run.ps1` sale con código != 0.

---

## Contrato de driver

Vive en `modules/orm`; ver `modules/orm/README.md` para la tabla completa y cómo
agregar un backend MySQL/Postgres (subclase de `Driver`, heredada con el nombre
**no cualificado** `Driver`; en anotaciones se usa `orm::Cell`/`orm::ResultSet`).

El ORM sólo usa esos métodos; un backend nuevo basta con pasarlo a
`Orm(driver)` en `src/main.clsx`. El SQL que emite `Orm` es estándar.

---

## Limitaciones conocidas (documentadas, no bugs de la prueba)

- **Ruta del `.dll` FFI = relativa al CWD**, no al archivo `.clsx`. La extensión
  del módulo `sqlite` apunta a `modules/sqlite/sqlite_shim.dll`, así que hay que
  correr desde la **raíz del repo**. `run.ps1` lo hace; para ejecuciones
  manuales, `cd` a la raíz primero. Ver `modules/sqlite/README.md`.
- `sqlite3.dll` no se versiona: `modules/sqlite/build.ps1` lo copia desde el
  runtime de Python a `modules/sqlite/` (y exporta `CLS_SQLITE_PATH` en la
  sesión). El shim también lo localiza junto a él.
- **C→CLS callback** (`sqlite3_exec` con callback): **no es expresable** hoy
  (no hay *function pointers* en el FFI). Ver `findings/06`.

---

## Cómo correr

```powershell
# Desde la raíz del repo
powershell -File examples\sqlite-orm\run.ps1

# Equivalente manual (el runner requiere pathModules -> modules):
target\debug\clx.exe run examples\sqlite-orm\src\main.clsx
```

> `main.clsx` corre con el **shadow stack por defecto** (trace on). El bug que lo
> rompía (trap + panic del reporter) quedó **resuelto** — ver `findings/04`.

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

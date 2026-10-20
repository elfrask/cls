# `orm` — capa ORM genérica + contrato de driver

Módulo **CLS puro** (sin FFI, sin `host_calls`) con la capa genérica de un ORM
**agnóstico al motor** y el **contrato de driver** que implementan los backends.
Es la mitad reutilizable de la prueba #5 (`examples/sqlite-orm`), extraída a
`/modules` (ver `examples/patrones.md` §11).

- Emite **SQL estándar** (`CREATE TABLE`, `INSERT ... VALUES (?, ...)`,
  `UPDATE ... SET ... WHERE ...`, `DELETE`, `SELECT ... WHERE ... ORDER BY ...
  LIMIT ... OFFSET ...`, `BEGIN`/`COMMIT`/`ROLLBACK`) y delega la ejecución en un
  `Driver`. **Nunca** interpola valores en el texto: usa *binding* `?`.
- No conoce ningún motor. El backend (SQLite en `modules/sqlite`; MySQL/Postgres
  a futuro) es una subclase de `Driver`.

## API

Valores neutros al motor (el typeck del JIT no permite arrays heterogéneos ni
`Any`, así que viajan taggeados en `Cell`):

| Símbolo | Descripción |
|---|---|
| `KIND_NULL`/`KIND_INT`/`KIND_FLOAT`/`KIND_TEXT`/`KIND_BLOB` | tags `1..5` |
| `Cell` | `{kind, i, f, s}`; el BLOB viaja como **hex** en `s` |
| `cNull` / `cInt` / `cFloat` / `cText` / `cBlob` | fábricas de `Cell` |
| `ResultSet` | filas × columnas materializadas (`nrows`, `ncols`, `cols`, `cells`) + `at`/`getInt`/`getFloat`/`getText`/`colIndex` |
| `Driver` | clase base del backend (ver contrato) |
| `Orm` | capa genérica (CRUD, query builder, transacciones, bulk) |

Uso:

```clx
import "orm" as orm;
import "sqlite" as sq;

var driver = sq::SqliteDriver();
driver.connect("app.db");
var o = orm::Orm(driver);
o.createTable("users", ["id INTEGER PRIMARY KEY", "name TEXT"]);
o.insert("users", ["id", "name"], [orm::cInt(1), orm::cText("Ana")]);
var rs = o.selectAll("users", "id");
```

Métodos principales de `Orm`: `exec`, `query`, `queryBind`, `begin`/`commit`/`rollback`,
`createTable`, `dropTable`, `insert`, `insertBatch`, `prepareInsert`/`bulkRow`/
`finishInsert` (streaming), `runDml`, `selectAll`, `selectWhere`, `findById`,
`update`, `deleteWhere`, `count`, `countWhere`, `integrity`, `engineVersion`,
`lastId`, `changes`.

## Contrato de driver (para agregar MySQL/Postgres)

Un backend nuevo **hereda de `Driver`** y sobrescribe:

| Método | Firma | Debe hacer |
|---|---|---|
| `connect` | `(path: String) -> int` | abrir conexión; devolver handle (>= 0 ok) |
| `close` | `() -> int` | cerrar |
| `exec` | `(sql: String) -> int` | SQL sin resultados (DDL/DML/TX); 0 = ok |
| `query` | `(sql: String) -> ResultSet` | `SELECT` sin parámetros |
| `queryBind` | `(sql: String, params: Cell[]) -> ResultSet` | `SELECT` con `?` |
| `prepare` | `(sql: String) -> int` | sentencia preparada (handle) |
| `bind` | `(stmt: int, idx: int, cell: Cell) -> int` | enlazar parámetro `idx` (1-based) |
| `step` | `(stmt: int) -> int` | 1 fila / 0 fin / < 0 error |
| `resetStmt` | `(stmt: int) -> int` | reutilizar tras `step` |
| `finalizeStmt` | `(stmt: int) -> int` | liberar |
| `lastId` | `() -> int` | last insert id |
| `changes` | `() -> int` | filas afectadas del último DML |
| `integrity` | `() -> String` | chequeo de integridad del motor |
| `version` | `() -> String` | versión del motor |

Para un backend nuevo: crear `modules/<motor>/mod.clsx` con
`class <Motor>Driver: Driver { ... }` (heredar con el nombre **no cualificado**
`Driver`; en anotaciones sí se usa `orm::Cell`, `orm::ResultSet`) y consumirlo
pasándolo a `Orm(driver)`. El SQL que emite `Orm` es estándar.

> **Nota**: el patrón alternativo «driver = `Record<String, Callable>`» no es
> viable hoy porque CLS no permite invocar un valor tipado como `Callable` (ver
> `examples/sqlite-orm/findings/05`). Por eso el driver es una **clase base** y el
> despacho es polimorfismo por clase.

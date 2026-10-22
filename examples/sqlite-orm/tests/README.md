# tests/

Los casos de prueba de esta suite viven en **`src/main.clsx`** (runner único que
construye `out/cls.db` a través del ORM y emite el protocolo `CHECK k=v`). El
oráculo externo (`oracle/oracle.py`) reproduce la misma carga de trabajo y
`run.ps1` compara **byte a byte** ambas bases y las líneas `CHECK`.

El ORM ya no vive en `src/`: es un módulo compartido (`modules/orm`) más su
backend FFI (`modules/sqlite`), referenciados con `import "orm"`/`import
"sqlite"` y resueltos vía `"pathModules": ["../../modules"]` de `cls.json` (ver
`examples/patrones.md` §11). Los repros que usan el ORM quedan en `src/repro-*`.

Categorías cubiertas por `src/main.clsx`: CRUD, tipos (INTEGER/REAL/TEXT UTF-8/
BLOB/NULL), batch 10k, transacciones, orden/paginación, integridad y el
marshalling `CArray`.

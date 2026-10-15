# tests/

Los casos de prueba de esta suite viven en **`src/main.clsx`** (runner único que
construye `out/cls.db` a través del ORM y emite el protocolo `CHECK k=v`). El
oráculo externo (`oracle/oracle.py`) reproduce la misma carga de trabajo y
`run.ps1` compara **byte a byte** ambas bases y las líneas `CHECK`.

No hay `tests/*.clsx` separados porque necesitan importar el ORM de `src/`, y el
resolutor de módulos sin `cls.json` propio resuelve los imports internos
relativos al **entry** (por eso los repros que usan el ORM también viven en
`src/repro-*.clsx`). Ver `findings/README.md` §7.

Categorías cubiertas por `src/main.clsx`: CRUD, tipos (INTEGER/REAL/TEXT UTF-8/
BLOB/NULL), batch 10k, transacciones, orden/paginación, integridad y el
marshalling `CArray`.

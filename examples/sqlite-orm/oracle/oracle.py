#!/usr/bin/env python3
# oracle.py - Oraculo externo de la prueba #5 (SQLite ORM).
#
# Independiente de CLS: usa el modulo `sqlite3` de Python (la libsqlite3 del
# sistema) para
#   run <db>     : construye la base de referencia y emite las mismas lineas
#                  `CHECK k=v` que el runner CLS (verifica la LECTURA via ORM).
#   dump <db>    : imprime un volcado canonico (byte a byte comparable) del
#                  contenido de una base.
#   integrity <db>: imprime `PRAGMA integrity_check`.
#
# Uso (desde la raiz del repo):
#   python examples/sqlite-orm/oracle/oracle.py run       examples/sqlite-orm/out/py.db
#   python examples/sqlite-orm/oracle/oracle.py dump      examples/sqlite-orm/out/cls.db
#
# Determinista: no usa floats aleatorios ni fechas.

import sqlite3
import sys

# Salida en UTF-8 (el dump incluye emoji/acentos; la consola Windows usa cp1252).
try:
    sys.stdout.reconfigure(encoding="utf-8", newline="\n")
except Exception:
    pass

# --- mismo mapeo de "kind" que el ORM (driver.clsx) ---
K_NULL, K_INT, K_FLOAT, K_TEXT, K_BLOB = 1, 2, 3, 4, 5

def kind_of(v):
    if v is None:
        return K_NULL
    if isinstance(v, bytes):
        return K_BLOB
    if isinstance(v, float):
        return K_FLOAT
    if isinstance(v, int):
        return K_INT
    return K_TEXT

def q(ident):
    return '"' + ident.replace('"', '""') + '"'

def connect(path):
    conn = sqlite3.connect(path)
    conn.isolation_level = None  # control manual de BEGIN/COMMIT/ROLLBACK
    return conn

def emit(k, v):
    print("CHECK {}={}".format(k, v))

# ----------------------------------------------------------------------
# run: reproduce la carga de trabajo y emite los CHECK.
# ----------------------------------------------------------------------
def run(path):
    conn = connect(path)
    c = conn.cursor()
    print("ENGINE " + sqlite3.sqlite_version)

    # --- CRUD: users ---
    c.execute("DROP TABLE IF EXISTS users")
    c.execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, age INTEGER)")
    c.executemany("INSERT INTO users (id, name, age) VALUES (?, ?, ?)",
                  [(1, "Ana", 30), (2, "Luis", 25), (3, "Mia", 41)])
    emit("users.count", c.execute("SELECT COUNT(*) FROM users").fetchone()[0])
    row = c.execute("SELECT * FROM users WHERE id = ?", (2,)).fetchone()
    emit("users.find2.rows", 1)
    emit("users.find2.age", row[2])
    emit("users.gt30", c.execute("SELECT COUNT(*) FROM users WHERE age > ?", (30,)).fetchone()[0])
    c.execute("UPDATE users SET age = ? WHERE id = ?", (26, 2))
    emit("users.updated", c.rowcount)
    emit("users.age2", c.execute("SELECT age FROM users WHERE id = ?", (2,)).fetchone()[0])
    c.execute("DELETE FROM users WHERE id = ?", (3,))
    emit("users.deleted", c.rowcount)
    emit("users.count2", c.execute("SELECT COUNT(*) FROM users").fetchone()[0])

    # --- tipos ---
    c.execute("DROP TABLE IF EXISTS types")
    c.execute("CREATE TABLE types (id INTEGER, i INTEGER, r REAL, t TEXT, b BLOB, n INTEGER)")
    c.execute("INSERT INTO types (id,i,r,t,b,n) VALUES (?,?,?,?,?,?)",
              (1, 42, 3.5, "acentos: \u00e1\u00e9\u00ed\u00f3\u00fa \u00f1 emoji \U0001F600",
               bytes.fromhex("00ff10"), 100))
    c.execute("INSERT INTO types (id,i,r,t,b,n) VALUES (?,?,?,?,?,?)",
              (2, -7, -0.125, "", b"", 0))
    c.execute("INSERT INTO types (id,i,r,t,b,n) VALUES (?,?,?,?,?,?)",
              (3, None, None, None, None, None))
    c.execute("INSERT INTO types (id,i,r,t,b,n) VALUES (?,?,?,?,?,?)",
              (4, 0, 0.0, "x", bytes.fromhex("ff"), None))
    emit("types.count", c.execute("SELECT COUNT(*) FROM types").fetchone()[0])
    cur = c.execute("SELECT * FROM types ORDER BY id")
    ncols = len(cur.description)
    rows = cur.fetchall()
    emit("types.ncols", ncols)
    emit("types.r0.ki", kind_of(rows[0][1]))
    emit("types.r0.kt", kind_of(rows[0][3]))
    emit("types.r0.kb", kind_of(rows[0][4]))
    emit("types.r0.kn", kind_of(rows[0][5]))
    emit("types.r1.tlen", len(rows[1][3].encode("utf-8")))
    emit("types.r2.ki", kind_of(rows[2][1]))
    emit("types.r3.kn", kind_of(rows[3][5]))
    emit("types.r0.i", rows[0][1])
    emit("types.r0.bloblen", len(rows[0][4].hex()))

    # --- batch 10k ---
    c.execute("DROP TABLE IF EXISTS batch")
    c.execute("CREATE TABLE batch (id INTEGER, name TEXT, score REAL)")
    c.execute("BEGIN")
    ins = "INSERT INTO batch (id, name, score) VALUES (?, ?, ?)"
    for k in range(1, 10001):
        nm = "user-" + str(k)
        if k % 97 == 0:
            nm = "u-" + str(k) + "-\u00f1\u00e1\U0001F600"
        c.execute(ins, (k, nm, k * 0.5))
    c.execute("COMMIT")
    cnt, sm = c.execute("SELECT COUNT(*), SUM(id) FROM batch").fetchone()
    emit("batch.count", cnt)
    emit("batch.idsum", sm)
    fname = c.execute("SELECT name FROM batch WHERE id = ?", (1,)).fetchone()[0]
    emit("batch.first.namelen", len(fname.encode("utf-8")))
    emit("batch.all.nrows", c.execute("SELECT COUNT(*) FROM batch").fetchone()[0])
    emit("batch.all.lastid", c.execute("SELECT id FROM batch ORDER BY id DESC LIMIT 1").fetchone()[0])
    # FFI CArray: misma columna id como lista de enteros.
    ids = [r[0] for r in c.execute("SELECT id FROM batch ORDER BY id")]
    emit("batch.carraylen", len(ids))
    emit("batch.carraysum", sum(ids))

    # --- transacciones ---
    c.execute("DROP TABLE IF EXISTS tx")
    c.execute("CREATE TABLE tx (id INTEGER, v TEXT)")
    c.execute("BEGIN")
    c.execute("INSERT INTO tx (id,v) VALUES (?,?)", (1, "a"))
    c.execute("INSERT INTO tx (id,v) VALUES (?,?)", (2, "b"))
    c.execute("ROLLBACK")
    emit("tx.after_rollback", c.execute("SELECT COUNT(*) FROM tx").fetchone()[0])
    c.execute("BEGIN")
    c.execute("INSERT INTO tx (id,v) VALUES (?,?)", (3, "c"))
    c.execute("INSERT INTO tx (id,v) VALUES (?,?)", (4, "d"))
    c.execute("COMMIT")
    emit("tx.after_commit", c.execute("SELECT COUNT(*) FROM tx").fetchone()[0])
    trows = c.execute("SELECT id FROM tx ORDER BY id").fetchall()
    emit("tx.row0.id", trows[0][0])
    emit("tx.row1.id", trows[1][0])

    # --- orden / paginacion ---
    c.execute("DROP TABLE IF EXISTS ord")
    c.execute("CREATE TABLE ord (id INTEGER, v INTEGER)")
    c.execute("BEGIN")
    for i in range(10):
        c.execute("INSERT INTO ord (id,v) VALUES (?,?)", (i, (i * 7) % 10))
    c.execute("COMMIT")
    page = c.execute("SELECT id FROM ord WHERE 1 = 1 ORDER BY v DESC, id ASC LIMIT 3 OFFSET 2").fetchall()
    emit("ord.rows", len(page))
    emit("ord.page1", ",".join(str(r[0]) for r in page))

    emit("integrity", c.execute("PRAGMA integrity_check").fetchone()[0])
    conn.close()

# ----------------------------------------------------------------------
# dump: volcado canonico (filas por rowid, UTF-8 crudo, blobs en hex).
# ----------------------------------------------------------------------
def dump(path):
    conn = connect(path)
    c = conn.cursor()
    out = []
    tables = [r[0] for r in c.execute(
        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name")]
    for t in tables:
        cols = [r[1] for r in c.execute("PRAGMA table_info(%s)" % q(t))]
        out.append("T %s %s" % (t, ",".join(cols)))
        for row in c.execute("SELECT * FROM %s ORDER BY rowid" % q(t)):
            parts = []
            for v in row:
                if v is None:
                    parts.append("N")
                elif isinstance(v, bytes):
                    parts.append("B" + v.hex())
                elif isinstance(v, float):
                    parts.append("F" + repr(v))
                elif isinstance(v, int):
                    parts.append("I" + str(v))
                else:
                    parts.append("T" + v)
            out.append("R " + "|".join(parts))
    conn.close()
    sys.stdout.write("\n".join(out) + "\n")

def integrity(path):
    conn = connect(path)
    print(conn.execute("PRAGMA integrity_check").fetchone()[0])
    conn.close()

def main(argv):
    if len(argv) < 3:
        print("uso: oracle.py {run|dump|integrity} <db>", file=sys.stderr)
        return 2
    cmd, path = argv[1], argv[2]
    if cmd == "run":
        run(path)
    elif cmd == "dump":
        dump(path)
    elif cmd == "integrity":
        integrity(path)
    else:
        print("comando desconocido: " + cmd, file=sys.stderr)
        return 2
    return 0

if __name__ == "__main__":
    sys.exit(main(sys.argv))

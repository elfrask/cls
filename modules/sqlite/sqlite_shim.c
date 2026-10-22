/*
 * sqlite_shim.c - Puente FFI "sin punteros" entre CLS y la API C de SQLite.
 *
 * REGLA (examples/patrones.md §2.2 y el enunciado de la prueba #5): el codigo
 * CLS NUNCA manipula punteros. La API de SQLite es pointer-heavy
 * (`sqlite3*`, `sqlite3_stmt*`, `char**`), asi que este shim la envuelve en
 * funciones cuyos parametros/retornos son SOLO escalares (int64/double) o
 * strings (char*, que el FFI de CLS copia a la memoria del modulo). Las
 * conexiones, sentencias y conjuntos de resultados viven dentro del shim y se
 * referencian por un indice entero (handle).
 *
 * Este shim es FFI legitimo (no expone punteros al lenguaje): NO es un
 * `host_call`, NO es una feature del host. Es la "puerta de escape a C".
 *
 * Carga de sqlite3.dll: el shim se autolocaliza (GetModuleFileNameW) y busca
 * `sqlite3.dll` en su propio directorio, luego en la ruta de busqueda estandar
 * y en `%CLS_SQLITE_PATH%`. Se resuelven los simbolos por GetProcAddress, de
 * modo que el .dll no queda con una dependencia de import estatica (robusto
 * frente al orden de busqueda de DLLs de Windows).
 *
 * Compilacion (Windows, MinGW-w64 x64):
 *   gcc -shared -O2 -o modules/sqlite/sqlite_shim.dll modules/sqlite/sqlite_shim.c
 * (o `powershell -File modules/sqlite/build.ps1`, que ademas copia sqlite3.dll).
 */

#ifndef _WIN32
#ifndef _GNU_SOURCE
#define _GNU_SOURCE
#endif
#endif

#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <stdio.h>

#ifdef _WIN32
#include <windows.h>
#define EXPORT __declspec(dllexport)
#else
#include <dlfcn.h>
#define EXPORT __attribute__((visibility("default")))
#endif

/* ------------------------------------------------------------------ */
/* Subconjunto de la ABI de SQLite que usamos (sin incluir sqlite3.h, que  */
/* no viene con el runtime de Python). Constantes y prototipos minimos.    */
/* ------------------------------------------------------------------ */
#define SQLITE_OK 0
#define SQLITE_ROW 100
#define SQLITE_DONE 101
#define SQLITE_OPEN_READONLY 0x00000001
#define SQLITE_OPEN_READWRITE 0x00000002
#define SQLITE_OPEN_CREATE 0x00000004

#define SQLITE_INTEGER 1
#define SQLITE_FLOAT 2
#define SQLITE_TEXT 3
#define SQLITE_BLOB 4
#define SQLITE_NULL 5

typedef struct sqlite3 sqlite3;
typedef struct sqlite3_stmt sqlite3_stmt;

/* Punteros a funcion resueltos dinamicamente. */
static int (*p_open_v2)(const char *, sqlite3 **, int, const char *);
static int (*p_close)(sqlite3 *);
static int (*p_exec)(sqlite3 *, const char *, void *, void *);
static int64_t (*p_last_insert_rowid)(sqlite3 *);
static int (*p_changes)(sqlite3 *);
static const char *(*p_errmsg)(sqlite3 *);
static int (*p_prepare_v2)(sqlite3 *, const char *, int, sqlite3_stmt **, const char **);
static int (*p_bind_int64)(sqlite3_stmt *, int, int64_t);
static int (*p_bind_double)(sqlite3_stmt *, int, double);
static int (*p_bind_text)(sqlite3_stmt *, int, const char *, int, void *);
static int (*p_bind_null)(sqlite3_stmt *, int);
static int (*p_bind_blob)(sqlite3_stmt *, int, const void *, int, void *);
static int (*p_initialize)(void);
static int (*p_step)(sqlite3_stmt *);
static int (*p_reset)(sqlite3_stmt *);
static int (*p_finalize)(sqlite3_stmt *);
static int (*p_column_count)(sqlite3_stmt *);
static int (*p_column_type)(sqlite3_stmt *, int);
static int64_t (*p_column_int64)(sqlite3_stmt *, int);
static double (*p_column_double)(sqlite3_stmt *, int);
static const unsigned char *(*p_column_text)(sqlite3_stmt *, int);
static const void *(*p_column_blob)(sqlite3_stmt *, int);
static int (*p_column_bytes)(sqlite3_stmt *, int);
static const char *(*p_column_name)(sqlite3_stmt *, int);
static const char *(*p_libversion)(void);

#ifdef _WIN32
static HMODULE g_lib = NULL;
#else
static void *g_lib = NULL;
#endif

/* Error global del shim (fallo de carga, etc.). */
static char g_error[1024];

static void set_error(const char *msg) {
    strncpy(g_error, msg ? msg : "", sizeof(g_error) - 1);
    g_error[sizeof(g_error) - 1] = '\0';
}

#ifdef _WIN32
static void *sym(const char *name) {
    return (void *)GetProcAddress(g_lib, name);
}
static int load_lib_at(const char *path) {
    if (!path || !path[0]) return 0;
    HMODULE h = LoadLibraryA(path);
    if (!h) return 0;
    if (g_lib) FreeLibrary(g_lib);
    g_lib = h;
    return 1;
}
/* Directorio del propio shim (para encontrar sqlite3.dll junto a el). */
static int self_dir(char *out, size_t cap) {
    HMODULE m = NULL;
    if (!GetModuleHandleExA(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS |
                GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            (LPCSTR)(void *)&self_dir, &m)) {
        return 0;
    }
    char buf[MAX_PATH];
    DWORD n = GetModuleFileNameA(m, buf, (DWORD)sizeof(buf));
    if (n == 0 || n >= sizeof(buf)) return 0;
    char *slash = strrchr(buf, '\\');
    if (!slash) return 0;
    *slash = '\0';
    strncpy(out, buf, cap - 1);
    out[cap - 1] = '\0';
    return 1;
}
#else
static void *sym(const char *name) { return dlsym(g_lib, name); }
static int load_lib_at(const char *path) {
    if (!path || !path[0]) return 0;
    void *h = dlopen(path, RTLD_NOW | RTLD_LOCAL);
    if (!h) return 0;
    if (g_lib) dlclose(g_lib);
    g_lib = h;
    return 1;
}
/* Directorio del propio shim (para encontrar libsqlite3 junto a el). */
static int self_dir(char *out, size_t cap) {
    Dl_info info;
    if (!dladdr((void *)&self_dir, &info) || !info.dli_fname) return 0;
    strncpy(out, info.dli_fname, cap - 1);
    out[cap - 1] = '\0';
    char *slash = strrchr(out, '/');
    if (!slash) return 0;
    *slash = '\0';
    return 1;
}
#endif

/* Carga sqlite3.dll: candidatos en orden. Devuelve 1 si ok. */
static int ensure_loaded(void) {
    if (g_lib) return 1;

    char path[2048];
    char dir[2048];

#ifdef _WIN32
    /* 1. Variable de entorno del runner. */
    const char *env = getenv("CLS_SQLITE_PATH");
    if (load_lib_at(env)) goto resolved;
    /* 2. Junto al shim. */
    if (self_dir(dir, sizeof(dir))) {
        snprintf(path, sizeof(path), "%s\\sqlite3.dll", dir);
        if (load_lib_at(path)) goto resolved;
    }
    /* 3. Busqueda estandar (CWD / PATH). */
    if (load_lib_at("sqlite3.dll")) goto resolved;
#else
    /* 1. Variable de entorno del runner. */
    const char *env = getenv("CLS_SQLITE_PATH");
    if (load_lib_at(env)) goto resolved;
    /* 2. Junto al shim. */
    if (self_dir(dir, sizeof(dir))) {
#ifdef __APPLE__
        snprintf(path, sizeof(path), "%s/libsqlite3.dylib", dir);
#else
        snprintf(path, sizeof(path), "%s/libsqlite3.so", dir);
#endif
        if (load_lib_at(path)) goto resolved;
    }
    /* 3. Carga estandar del sistema. */
#ifdef __APPLE__
    if (load_lib_at("libsqlite3.dylib")) goto resolved;
#else
    if (load_lib_at("libsqlite3.so.0")) goto resolved;
    if (load_lib_at("libsqlite3.so")) goto resolved;
#endif
#endif
    set_error("no se pudo cargar SQLite (probado CLS_SQLITE_PATH, "
              "directorio del shim y la carga estandar del sistema)");
    return 0;

resolved:
    p_open_v2 = (int (*)(const char *, sqlite3 **, int, const char *))sym("sqlite3_open_v2");
    p_close = (int (*)(sqlite3 *))sym("sqlite3_close");
    p_exec = (int (*)(sqlite3 *, const char *, void *, void *))sym("sqlite3_exec");
    p_last_insert_rowid = (int64_t (*)(sqlite3 *))sym("sqlite3_last_insert_rowid");
    p_changes = (int (*)(sqlite3 *))sym("sqlite3_changes");
    p_errmsg = (const char *(*)(sqlite3 *))sym("sqlite3_errmsg");
    p_prepare_v2 = (int (*)(sqlite3 *, const char *, int, sqlite3_stmt **, const char **))sym("sqlite3_prepare_v2");
    p_bind_int64 = (int (*)(sqlite3_stmt *, int, int64_t))sym("sqlite3_bind_int64");
    p_bind_double = (int (*)(sqlite3_stmt *, int, double))sym("sqlite3_bind_double");
    p_bind_text = (int (*)(sqlite3_stmt *, int, const char *, int, void *))sym("sqlite3_bind_text");
    p_bind_null = (int (*)(sqlite3_stmt *, int))sym("sqlite3_bind_null");
    p_bind_blob = (int (*)(sqlite3_stmt *, int, const void *, int, void *))sym("sqlite3_bind_blob");
    p_initialize = (int (*)(void))sym("sqlite3_initialize");
    p_step = (int (*)(sqlite3_stmt *))sym("sqlite3_step");
    p_reset = (int (*)(sqlite3_stmt *))sym("sqlite3_reset");
    p_finalize = (int (*)(sqlite3_stmt *))sym("sqlite3_finalize");
    p_column_count = (int (*)(sqlite3_stmt *))sym("sqlite3_column_count");
    p_column_type = (int (*)(sqlite3_stmt *, int))sym("sqlite3_column_type");
    p_column_int64 = (int64_t (*)(sqlite3_stmt *, int))sym("sqlite3_column_int64");
    p_column_double = (double (*)(sqlite3_stmt *, int))sym("sqlite3_column_double");
    p_column_text = (const unsigned char *(*)(sqlite3_stmt *, int))sym("sqlite3_column_text");
    p_column_blob = (const void *(*)(sqlite3_stmt *, int))sym("sqlite3_column_blob");
    p_column_bytes = (int (*)(sqlite3_stmt *, int))sym("sqlite3_column_bytes");
    p_column_name = (const char *(*)(sqlite3_stmt *, int))sym("sqlite3_column_name");
    p_libversion = (const char *(*)(void))sym("sqlite3_libversion");

    if (!p_open_v2 || !p_exec || !p_prepare_v2 || !p_step || !p_column_count) {
        set_error("sqlite3.dll no exporta los simbolos esperados");
#ifdef _WIN32
        FreeLibrary(g_lib);
#endif
        g_lib = NULL;
        return 0;
    }
    /* El build de sqlite3 de Python se compila con SQLITE_OMIT_AUTOINIT:
     * hay que inicializar explicitamente antes de la primera llamada. */
    if (p_initialize) p_initialize();
    return 1;
}

/* ------------------------------------------------------------------ */
/* Registro de handles: conexiones, sentencias y result sets.             */
/* ------------------------------------------------------------------ */
#define MAX_CONN 64
#define MAX_STMT 64
#define MAX_RES 64

typedef struct Conn { sqlite3 *db; } Conn;
static Conn g_conns[MAX_CONN];
static sqlite3_stmt *g_stmts[MAX_STMT];
static Conn *g_conn_of_stmt[MAX_STMT];

typedef struct Cell {
    int type;        /* SQLITE_* */
    int64_t i;
    double d;
    char *s;         /* text (utf8, nul-term) o hex de blob */
} Cell;

typedef struct Result {
    int ncols;
    int64_t nrows;
    Cell *cells;          /* nrows*ncols */
    char **colnames;
} Result;
static Result *g_results[MAX_RES];

static int64_t materialize_stmt(sqlite3_stmt *st);

static int slot_alloc(void *arr, size_t n, size_t elemsize) {
    unsigned char *p = (unsigned char *)arr;
    for (size_t k = 0; k < n; k++) {
        void *slot;
        memcpy(&slot, p + k * elemsize, sizeof(void *));
        if (!slot) return (int)k;
    }
    return -1;
}

/* ------------------------------------------------------------------ */
/* API publica (sin punteros).                                           */
/* ------------------------------------------------------------------ */

EXPORT int64_t sq_init(void) {
    if (!ensure_loaded()) return -1;
    return 0;
}

/* version de sqlite (dummy param por workaround del typeck de CLS). */
EXPORT const char *sq_version(const char *_d) {
    (void)_d;
    if (!ensure_loaded()) return g_error;
    return p_libversion ? p_libversion() : "?";
}

EXPORT const char *sq_error(const char *_d) {
    (void)_d;
    return g_error;
}

EXPORT int64_t sq_open(const char *path) {
    if (!ensure_loaded()) return -1;
    int idx = slot_alloc(g_conns, MAX_CONN, sizeof(Conn));
    if (idx < 0) { set_error("sin slots de conexion"); return -2; }
    sqlite3 *db = NULL;
    int rc = p_open_v2(path, &db, SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE, NULL);
    if (rc != SQLITE_OK || !db) {
        char tmp[512];
        snprintf(tmp, sizeof(tmp), "sqlite3_open_v2 rc=%d", rc);
        set_error(tmp);
        if (db) p_close(db);
        return -3;
    }
    g_conns[idx].db = db;
    return (int64_t)idx;
}

EXPORT int64_t sq_close(int64_t h) {
    if (h < 0 || h >= MAX_CONN || !g_conns[h].db) return -1;
    int rc = p_close(g_conns[h].db);
    g_conns[h].db = NULL;
    return (int64_t)rc;
}

EXPORT int64_t sq_exec(int64_t h, const char *sql) {
    if (h < 0 || h >= MAX_CONN || !g_conns[h].db) return -1;
    return (int64_t)p_exec(g_conns[h].db, sql, NULL, NULL);
}

EXPORT int64_t sq_last_id(int64_t h) {
    if (h < 0 || h >= MAX_CONN || !g_conns[h].db) return -1;
    return p_last_insert_rowid(g_conns[h].db);
}

EXPORT int64_t sq_changes(int64_t h) {
    if (h < 0 || h >= MAX_CONN || !g_conns[h].db) return -1;
    return (int64_t)p_changes(g_conns[h].db);
}

EXPORT const char *sq_errmsg(int64_t h) {
    if (h < 0 || h >= MAX_CONN || !g_conns[h].db) return "";
    return p_errmsg(g_conns[h].db);
}

/* ---------------- prepared statements ---------------- */
EXPORT int64_t sq_prepare(int64_t h, const char *sql) {
    if (h < 0 || h >= MAX_CONN || !g_conns[h].db) return -1;
    int idx = slot_alloc(g_stmts, MAX_STMT, sizeof(sqlite3_stmt *));
    if (idx < 0) { set_error("sin slots de sentencia"); return -2; }
    sqlite3_stmt *st = NULL;
    int rc = p_prepare_v2(g_conns[h].db, sql, -1, &st, NULL);
    if (rc != SQLITE_OK || !st) {
        char tmp[512];
        snprintf(tmp, sizeof(tmp), "prepare rc=%d: %s", rc, p_errmsg(g_conns[h].db));
        set_error(tmp);
        return -3;
    }
    g_stmts[idx] = st;
    g_conn_of_stmt[idx] = &g_conns[h];
    return (int64_t)idx;
}

static int stmt_ok(int64_t s) { return s >= 0 && s < MAX_STMT && g_stmts[s]; }

/* Convierte hex -> bytes; devuelve buffer malloc'd y longitud. */
static unsigned char *hex_to_bytes(const char *hex, int *out_len) {
    size_t n = strlen(hex);
    if (n % 2 != 0) { *out_len = -1; return NULL; }
    size_t blen = n / 2;
    unsigned char *buf = (unsigned char *)malloc(blen > 0 ? blen : 1);
    if (!buf) { *out_len = -1; return NULL; }
    for (size_t k = 0; k < blen; k++) {
        unsigned int byte;
        char pair[3] = { hex[2 * k], hex[2 * k + 1], 0 };
        if (sscanf(pair, "%2x", &byte) != 1) { free(buf); *out_len = -1; return NULL; }
        buf[k] = (unsigned char)byte;
    }
    *out_len = (int)blen;
    return buf;
}

EXPORT int64_t sq_bind_int(int64_t s, int64_t idx, int64_t v) {
    if (!stmt_ok(s)) return -1;
    return (int64_t)p_bind_int64(g_stmts[s], (int)idx, v);
}
EXPORT int64_t sq_bind_double(int64_t s, int64_t idx, double v) {
    if (!stmt_ok(s)) return -1;
    return (int64_t)p_bind_double(g_stmts[s], (int)idx, v);
}
EXPORT int64_t sq_bind_text(int64_t s, int64_t idx, const char *v) {
    if (!stmt_ok(s)) return -1;
    if (!v) return (int64_t)p_bind_null(g_stmts[s], (int)idx);
    /* SQLITE_TRANSIENT = (void*)-1: sqlite copia el buffer. */
    return (int64_t)p_bind_text(g_stmts[s], (int)idx, v, -1, (void *)-1);
}
EXPORT int64_t sq_bind_null(int64_t s, int64_t idx) {
    if (!stmt_ok(s)) return -1;
    return (int64_t)p_bind_null(g_stmts[s], (int)idx);
}
EXPORT int64_t sq_bind_blob_hex(int64_t s, int64_t idx, const char *hex) {
    if (!stmt_ok(s)) return -1;
    int blen = 0;
    unsigned char *buf = hex_to_bytes(hex, &blen);
    if (blen < 0) return -99;
    int rc = p_bind_blob(g_stmts[s], (int)idx, buf, blen, (void *)-1);
    free(buf);
    return (int64_t)rc;
}
EXPORT int64_t sq_step(int64_t s) {
    if (!stmt_ok(s)) return -1;
    return (int64_t)p_step(g_stmts[s]);
}
EXPORT int64_t sq_reset(int64_t s) {
    if (!stmt_ok(s)) return -1;
    return (int64_t)p_reset(g_stmts[s]);
}
EXPORT int64_t sq_finalize(int64_t s) {
    if (!stmt_ok(s)) return -1;
    int rc = p_finalize(g_stmts[s]);
    g_stmts[s] = NULL;
    g_conn_of_stmt[s] = NULL;
    return (int64_t)rc;
}

/* ---------------- query -> result set materializado ---------------- */
static char *bytes_to_hex(const unsigned char *b, int n) {
    char *out = (char *)malloc((size_t)n * 2 + 1);
    if (!out) return NULL;
    static const char hx[] = "0123456789abcdef";
    for (int k = 0; k < n; k++) {
        out[2 * k] = hx[(b[k] >> 4) & 0xF];
        out[2 * k + 1] = hx[b[k] & 0xF];
    }
    out[2 * n] = '\0';
    return out;
}

EXPORT int64_t sq_query(int64_t h, const char *sql) {
    if (h < 0 || h >= MAX_CONN || !g_conns[h].db) return -1;
    sqlite3_stmt *st = NULL;
    int rc = p_prepare_v2(g_conns[h].db, sql, -1, &st, NULL);
    if (rc != SQLITE_OK || !st) {
        char tmp[512];
        snprintf(tmp, sizeof(tmp), "query prepare rc=%d: %s", rc, p_errmsg(g_conns[h].db));
        set_error(tmp);
        return -3;
    }
    int64_t idx = materialize_stmt(st);
    p_finalize(st);
    return idx;
}

/* Materializa un sqlite3_stmt* ya preparado (y con binds hechos) en un result
 * set. Compartido por sq_query y sq_result_from_stmt. */
static int64_t materialize_stmt(sqlite3_stmt *st) {
    int idx = slot_alloc(g_results, MAX_RES, sizeof(Result *));
    if (idx < 0) { set_error("sin slots de resultado"); return -2; }

    int ncols = p_column_count(st);
    Result *r = (Result *)calloc(1, sizeof(Result));
    r->ncols = ncols;
    r->colnames = (char **)calloc((size_t)(ncols > 0 ? ncols : 1), sizeof(char *));
    for (int c = 0; c < ncols; c++) {
        const char *nm = p_column_name(st, c);
        r->colnames[c] = strdup(nm ? nm : "");
    }
    size_t cap = 16;
    r->cells = (Cell *)calloc(cap * (size_t)ncols, sizeof(Cell));
    int64_t nrows = 0;

    int rc;
    while ((rc = p_step(st)) == SQLITE_ROW) {
        if ((size_t)(nrows + 1) > cap) {
            size_t ncap = cap * 2;
            Cell *nc = (Cell *)realloc(r->cells, ncap * (size_t)ncols * sizeof(Cell));
            if (!nc) break;
            memset(nc + cap * (size_t)ncols, 0, (ncap - cap) * (size_t)ncols * sizeof(Cell));
            r->cells = nc;
            cap = ncap;
        }
        for (int c = 0; c < ncols; c++) {
            Cell *cell = &r->cells[(size_t)nrows * ncols + c];
            int t = p_column_type(st, c);
            cell->type = t;
            if (t == SQLITE_INTEGER) {
                cell->i = p_column_int64(st, c);
            } else if (t == SQLITE_FLOAT) {
                cell->d = p_column_double(st, c);
            } else if (t == SQLITE_TEXT) {
                const unsigned char *txt = p_column_text(st, c);
                int nb = p_column_bytes(st, c);
                cell->s = (char *)malloc((size_t)nb + 1);
                if (txt) memcpy(cell->s, txt, (size_t)nb);
                cell->s[nb] = '\0';
            } else if (t == SQLITE_BLOB) {
                const void *bl = p_column_blob(st, c);
                int nb = p_column_bytes(st, c);
                cell->s = bytes_to_hex((const unsigned char *)bl, nb);
            }
        }
        nrows++;
    }
    r->nrows = nrows;
    g_results[idx] = r;
    return (int64_t)idx;
}

/* Materializa el stmt preparado en `s` (result set), sin finalizarlo. */
EXPORT int64_t sq_result_from_stmt(int64_t s) {
    if (!stmt_ok(s)) return -1;
    return materialize_stmt(g_stmts[s]);
}

/* Borra un archivo (FILE* remove de la libc). Permite empezar de cero sin fs. */
EXPORT int64_t sq_remove(const char *path) {
    return (int64_t)remove(path);
}

static Result *res_ok(int64_t r) {
    if (r < 0 || r >= MAX_RES || !g_results[r]) return NULL;
    return g_results[r];
}

EXPORT int64_t sq_nrows(int64_t r) {
    Result *p = res_ok(r);
    return p ? p->nrows : -1;
}
EXPORT int64_t sq_ncols(int64_t r) {
    Result *p = res_ok(r);
    return p ? p->ncols : -1;
}
EXPORT const char *sq_colname(int64_t r, int64_t c) {
    Result *p = res_ok(r);
    if (!p || c < 0 || c >= p->ncols) return "";
    return p->colnames[c];
}
EXPORT int64_t sq_cell_type(int64_t r, int64_t i, int64_t c) {
    Result *p = res_ok(r);
    if (!p || i < 0 || i >= p->nrows || c < 0 || c >= p->ncols) return -1;
    return (int64_t)p->cells[(size_t)i * p->ncols + c].type;
}
EXPORT int64_t sq_cell_int(int64_t r, int64_t i, int64_t c) {
    Result *p = res_ok(r);
    if (!p || i < 0 || i >= p->nrows || c < 0 || c >= p->ncols) return 0;
    return p->cells[(size_t)i * p->ncols + c].i;
}
EXPORT double sq_cell_double(int64_t r, int64_t i, int64_t c) {
    Result *p = res_ok(r);
    if (!p || i < 0 || i >= p->nrows || c < 0 || c >= p->ncols) return 0.0;
    return p->cells[(size_t)i * p->ncols + c].d;
}
EXPORT const char *sq_cell_text(int64_t r, int64_t i, int64_t c) {
    Result *p = res_ok(r);
    if (!p || i < 0 || i >= p->nrows || c < 0 || c >= p->ncols) return "";
    Cell *cell = &p->cells[(size_t)i * p->ncols + c];
    return cell->s ? cell->s : "";
}
EXPORT const char *sq_cell_blob_hex(int64_t r, int64_t i, int64_t c) {
    Result *p = res_ok(r);
    if (!p || i < 0 || i >= p->nrows || c < 0 || c >= p->ncols) return "";
    Cell *cell = &p->cells[(size_t)i * p->ncols + c];
    return (cell->type == SQLITE_BLOB && cell->s) ? cell->s : "";
}
EXPORT int64_t sq_free_result(int64_t r) {
    Result *p = res_ok(r);
    if (!p) return -1;
    for (int64_t k = 0; k < p->nrows * p->ncols; k++) free(p->cells[k].s);
    free(p->cells);
    for (int c = 0; c < p->ncols; c++) free(p->colnames[c]);
    free(p->colnames);
    free(p);
    g_results[r] = NULL;
    return 0;
}

/* Columna entera materializada como CArray i64 [cap][len][elems*8].
 * (Prueba explicita del marshalling CArray del FFI.) */
EXPORT int64_t sq_column_int(int64_t r, int64_t c) {
    Result *p = res_ok(r);
    if (!p || c < 0 || c >= p->ncols) return 0;
    int64_t n = 0;
    for (int64_t i = 0; i < p->nrows; i++) {
        if (p->cells[(size_t)i * p->ncols + c].type == SQLITE_INTEGER ||
            p->cells[(size_t)i * p->ncols + c].type == SQLITE_NULL) n++;
    }
    unsigned char *arr = (unsigned char *)malloc(16 + (size_t)n * 8);
    if (!arr) return 0;
    int64_t cap = n, len = n;
    memcpy(arr, &cap, 8);
    memcpy(arr + 8, &len, 8);
    int64_t w = 0;
    for (int64_t i = 0; i < p->nrows; i++) {
        Cell *cell = &p->cells[(size_t)i * p->ncols + c];
        if (cell->type == SQLITE_INTEGER) {
            memcpy(arr + 16 + (size_t)w * 8, &cell->i, 8);
            w++;
        } else if (cell->type == SQLITE_NULL) {
            int64_t z = 0;
            memcpy(arr + 16 + (size_t)w * 8, &z, 8);
            w++;
        }
    }
    return (int64_t)(uintptr_t)arr;
}

/* PRAGMA integrity_check -> primera fila, primera columna (CString). */
EXPORT const char *sq_integrity(int64_t h) {
    static char buf[256];
    if (h < 0 || h >= MAX_CONN || !g_conns[h].db) return "";
    sqlite3_stmt *st = NULL;
    if (p_prepare_v2(g_conns[h].db, "PRAGMA integrity_check", -1, &st, NULL) != SQLITE_OK)
        return "";
    if (p_step(st) == SQLITE_ROW) {
        const unsigned char *txt = p_column_text(st, 0);
        if (txt) {
            strncpy(buf, (const char *)txt, sizeof(buf) - 1);
            buf[sizeof(buf) - 1] = '\0';
        } else {
            buf[0] = '\0';
        }
    } else {
        buf[0] = '\0';
    }
    p_finalize(st);
    return buf;
}

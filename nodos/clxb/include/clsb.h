/* clsb.h — ABI C del nodo de bindings de CLS (embedding).
 *
 * Generado a mano (espejo de nodos/clxb/src/capi.rs). El binario:
 *   Windows: clsb.dll · Linux: libclsb.so · macOS: libclsb.dylib
 *
 * Contrato de memoria:
 *   - clsb_value* devueltos → clsb_value_free (recursivo).
 *   - clsb_error* → clsb_error_free; los strings viven mientras el error.
 *   - clsb_version → estático.
 *   - Un handle no se comparte entre threads (single-thread por handle).
 */
#ifndef CLSB_H
#define CLSB_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef int clsb_status;
#define CLSB_OK 0

/* kinds de valor (códigos de la custom section clx:exports) */
#define CLSB_INT    0
#define CLSB_FLOAT  1
#define CLSB_BOOL   2
#define CLSB_CHAR   3
#define CLSB_STRING 4
#define CLSB_ARRAY  5
#define CLSB_RECORD 6
#define CLSB_NULL   12

typedef struct clsb_engine clsb_engine;
typedef struct clsb_module clsb_module;
typedef struct clsb_error  clsb_error;

typedef struct clsb_config {
    int enable_fs;    /* reservado (sandbox futuro) */
    int enable_http;  /* reservado */
} clsb_config;

typedef struct clsb_value {
    int32_t tag;
    int64_t bits;              /* int | bits de float | bool 0/1 | char */
    const char* text;          /* CLSB_STRING (owned) */
    struct clsb_value* items;  /* CLSB_ARRAY (owned, n elems) */
    const char** keys;         /* CLSB_RECORD (owned, n claves) */
    struct clsb_value* vals;   /* CLSB_RECORD (owned, n valores) */
    size_t n;                  /* ARRAY: elems · RECORD: entradas */
} clsb_value;

/* callbacks */
typedef void (*clsb_output_cb)(void* ud, const char* text, int is_end);
typedef size_t (*clsb_resolver_cb)(void* ud, const char* path,
                                   const char* base_dir, char* buf, size_t buf_len);
typedef int (*clsb_host_fn)(void* ud, uint32_t id,
                            const clsb_value* args, size_t args_len,
                            clsb_value* out);

/* ciclo de vida */
clsb_engine* clsb_engine_new(const clsb_config* cfg);
void clsb_engine_free(clsb_engine* e);

/* compilación (modo librería: main opcional) */
clsb_module* clsb_compile_source(clsb_engine* e, const char* source,
                                 const char* name, const char* base_dir,
                                 clsb_error** err);
clsb_module* clsb_compile_file(clsb_engine* e, const char* path,
                               clsb_error** err);
void clsb_module_free(clsb_module* m);

/* ejecución */
int64_t clsb_run_main(clsb_module* m, const clsb_value* args, size_t args_len,
                      clsb_error** err);
clsb_status clsb_call(clsb_module* m, const char* name,
                      const clsb_value* args, size_t args_len,
                      clsb_value* out, clsb_error** err);
clsb_status clsb_eval(clsb_engine* e, const char* source,
                      clsb_value* out, clsb_error** err);

/* SDK de nodo */
clsb_status clsb_set_output(clsb_engine* e, clsb_output_cb cb, void* ud);
clsb_status clsb_set_resolver(clsb_engine* e, clsb_resolver_cb cb, void* ud);
clsb_status clsb_register_host_function(clsb_engine* e, const char* name,
                                        const char* sig, clsb_host_fn cb,
                                        void* ud);

/* valores */
clsb_value clsb_value_null(void);
clsb_value clsb_value_int(int64_t v);
clsb_value clsb_value_float(double v);
clsb_value clsb_value_bool(int v);
clsb_value clsb_value_char(uint32_t v);
clsb_value clsb_value_string(const char* s);
clsb_value clsb_value_array(size_t n);
clsb_value clsb_value_record(size_t n);
void clsb_value_free(clsb_value* v);

/* errores y versión */
void clsb_error_free(clsb_error* e);
const char* clsb_error_trace(const clsb_error* e);
const char* clsb_error_message(const clsb_error* e);
const char* clsb_version(void);

#ifdef __cplusplus
}
#endif

#endif /* CLSB_H */

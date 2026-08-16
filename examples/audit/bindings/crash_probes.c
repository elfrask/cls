/* crash_probes.c - pruebas que pueden crashear, cada una en un subproceso
 * (el caller corre cada caso por separado y captura el exit code).
 *
 * Uso: crash_probes.exe <caso>
 *   double_free    - clsb_value_free dos veces sobre el mismo valor
 *   null_err_compile - clsb_compile_source con err_out=NULL (y source inválido)
 *   null_err_call  - clsb_call con err_out=NULL (y export inexistente)
 */
#include <clsb.h>
#include <stdio.h>
#include <string.h>

static int host_dummy(void* ud, uint32_t id, const clsb_value* args, size_t n, clsb_value* out) {
    (void)ud; (void)id; (void)args; (void)n;
    *out = clsb_value_int(1);
    return 0;
}

int main(int argc, char** argv) {
    setvbuf(stdout, NULL, _IONBF, 0);
    if (argc < 2) { printf("uso: crash_probes <caso>\n"); return 0; }
    const char* caso = argv[1];

    if (strcmp(caso, "double_free") == 0) {
        clsb_value v = clsb_value_string("doble free");
        clsb_value_free(&v);
        printf("primer free OK\n");
        clsb_value_free(&v);
        printf("segundo free OK (sin crash)\n");
        return 0;
    }

    if (strcmp(caso, "null_err_compile") == 0) {
        clsb_engine* e = clsb_engine_new(NULL);
        /* err_out = NULL con error de sintaxis: el runtime escribe en NULL */
        clsb_module* m = clsb_compile_source(e, "function main( {", "bad", ".", NULL);
        printf("compile con err NULL -> module=%p (sin crash)\n", (void*)m);
        clsb_engine_free(e);
        return 0;
    }

    if (strcmp(caso, "null_err_call") == 0) {
        clsb_engine* e = clsb_engine_new(NULL);
        clsb_error* err = NULL;
        clsb_module* m = clsb_compile_source(e, "export function x() -> int { return 1; }", "m", ".", &err);
        clsb_value out = clsb_value_null();
        clsb_status st = clsb_call(m, "no_existe", NULL, 0, &out, NULL);
        printf("call con err NULL -> st=%d (sin crash)\n", st);
        clsb_module_free(m);
        clsb_engine_free(e);
        return 0;
    }

    if (strcmp(caso, "null_name_call") == 0) {
        /* name = NULL en clsb_call: CStr::from_ptr(NULL) = UB */
        clsb_engine* e = clsb_engine_new(NULL);
        clsb_error* err = NULL;
        clsb_module* m = clsb_compile_source(e, "export function x() -> int { return 1; }", "m", ".", &err);
        clsb_value out = clsb_value_null();
        clsb_status st = clsb_call(m, NULL, NULL, 0, &out, &err);
        printf("call con name=NULL -> st=%d (sin crash)\n", st);
        clsb_module_free(m);
        clsb_engine_free(e);
        return 0;
    }

    printf("caso desconocido: %s\n", caso);
    return 0;
}

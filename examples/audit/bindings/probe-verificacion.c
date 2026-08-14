#include <clsb.h>
#include <stdio.h>
#include <string.h>

static int checks = 0;
static int fails = 0;
static void check(int ok, const char* what) { checks++; if (!ok) { fails++; printf("FAIL: %s\n", what); } else { printf("ok:   %s\n", what); } }

static void on_output(void* ud, const char* text, int is_end) { (void)ud; if (is_end) printf("[print] %s\n", text ? text : ""); }

int main(void) {
    clsb_engine* engine = clsb_engine_new(NULL);
    clsb_error* err = NULL;

    /* Probe 1: sandbox — el script puede usar fs del host? */
    clsb_set_output(engine, on_output, NULL);
    clsb_module* m = clsb_compile_source(engine,
        "function main(args: String[]) -> int { print(fs.cwd()); return 0; };",
        "sandbox", ".", &err);
    if (m) {
        printf("--- fs.cwd desde script (sandbox) ---\n");
        clsb_run_main(m, NULL, 0, &err);
        check(1, "script con fs.cwd ejecutado (deberia estar bloqueado en sandbox)");
    } else {
        check(err != NULL, "compile fallo (sandbox bloqueado): trace presente");
        printf("trace: %s\n", err ? clsb_error_trace(err) : "");
    }
    if (err) { clsb_error_free(err); err = NULL; }
    if (m) clsb_module_free(m);

    /* Probe 2: exit() mata el proceso host? */
    printf("--- exit(7) desde script ---\n");
    m = clsb_compile_source(engine,
        "function main(args: String[]) -> int { exit(7); return 0; };",
        "exitprobe", ".", &err);
    if (m) {
        int64_t code = clsb_run_main(m, NULL, 0, &err);
        printf("run_main devolvio code=%lld (si llego aqui, exit NO mato el proceso)\n", (long long)code);
    }
    if (err) clsb_error_free(err);
    if (m) clsb_module_free(m);

    clsb_engine_free(engine);
    printf("\n%d checks, %d fails\n", checks, fails);
    return fails == 0 ? 0 : 1;
}

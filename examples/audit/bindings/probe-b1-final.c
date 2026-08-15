#include <clsb.h>
#include <stdio.h>
#include <string.h>

static void on_output(void* ud, const char* text, int is_end) { (void)ud; if (is_end) printf("[print] %s\n", text ? text : ""); }

int main(void) {
    /* B1: sandbox — engine con config NULL (sandbox ON) */
    clsb_engine* engine = clsb_engine_new(NULL);
    clsb_error* err = NULL;
    clsb_set_output(engine, on_output, NULL);

    clsb_module* m = clsb_compile_source(engine,
        "function main(args: String[]) -> int { var c = fs.cwd(); print(c); return 0; };",
        "sandbox", ".", &err);
    if (!m) { printf("B1: compile fallo (sandbox en compile): %s\n", err ? clsb_error_message(err) : "?"); return 1; }
    int64_t code = clsb_run_main(m, NULL, 0, &err);
    if (err) {
        printf("B1 OK: fs.cwd bloqueado en runtime, err: %s\n", clsb_error_message(err));
        clsb_error_free(err);
    } else {
        printf("B1 FALLA: fs.cwd se ejecuto, run_main code=%lld\n", (long long)code);
    }
    clsb_module_free(m);

    /* B1b: enable_fs=1 expone fs */
    clsb_config cfg; memset(&cfg, 0, sizeof(cfg)); cfg.enable_fs = 1;
    clsb_engine* e2 = clsb_engine_new(&cfg);
    err = NULL;
    m = clsb_compile_source(e2,
        "function main(args: String[]) -> int { print(fs.cwd()); return 0; };",
        "sandbox2", ".", &err);
    if (!m) { printf("B1b: compile fallo con fs habilitado: %s\n", err ? clsb_error_message(err) : "?"); return 1; }
    err = NULL;
    code = clsb_run_main(m, NULL, 0, &err);
    if (err) { printf("B1b FALLA: fs.cwd fallo con enable_fs=1: %s\n", clsb_error_message(err)); }
    else { printf("B1b OK: fs.cwd accesible con enable_fs=1 (code=%lld)\n", (long long)code); }
    clsb_module_free(m);
    clsb_engine_free(e2);
    clsb_engine_free(engine);

    /* run_main sin main */
    clsb_engine* e3 = clsb_engine_new(NULL);
    err = NULL;
    m = clsb_compile_source(e3, "export function x() -> int { return 1; }", "nomain", ".", &err);
    if (m) {
        err = NULL;
        code = clsb_run_main(m, NULL, 0, &err);
        printf("run_main sin main: code=%lld err=%s\n", (long long)code, err ? clsb_error_message(err) : "(sin error)");
        if (err) clsb_error_free(err);
        clsb_module_free(m);
    } else {
        printf("compile sin main fallo: %s\n", err ? clsb_error_message(err) : "?");
    }
    clsb_engine_free(e3);
    return 0;
}

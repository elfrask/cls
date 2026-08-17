#include <clsb.h>
#include <stdio.h>

static void on_output(void* ud, const char* text, int is_end) { (void)ud; if (is_end) printf("[print] %s\n", text ? text : ""); }

int main(void) {
    clsb_engine* engine = clsb_engine_new(NULL);
    clsb_error* err = NULL;
    clsb_set_output(engine, on_output, NULL);
    clsb_module* m = clsb_compile_source(engine,
        "export function div(a: int, b: int) -> int { return a / b; }",
        "divtest", ".", &err);
    clsb_value out = clsb_value_null();
    clsb_status st = clsb_call(m, "div", NULL, 0, &out, &err);
    /* sin args: deberia dar error de aridad */
    printf("call div sin args: st=%d err=%s\n", st, err ? clsb_error_message(err) : "(sin error)");
    if (err) { clsb_error_free(err); err = NULL; }

    /* div por cero */
    clsb_value args[2] = { clsb_value_int(10), clsb_value_int(0) };
    st = clsb_call(m, "div", args, 2, &out, &err);
    printf("call div(10,0): st=%d\n", st);
    if (err) {
        const char* msg = clsb_error_message(err);
        const char* trace = clsb_error_trace(err);
        printf("--- message ---\n%s\n--- trace ---\n%s\n", msg, trace);
        clsb_error_free(err);
    }
    clsb_value_free(&args[0]); clsb_value_free(&args[1]);
    clsb_value_free(&out);
    clsb_module_free(m);
    clsb_engine_free(engine);
    return 0;
}

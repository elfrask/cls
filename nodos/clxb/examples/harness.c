/* harness.c - Contrato de la ABI C de clsb (validación del binding).
 *
 * Compilar:
 *   gcc harness.c -I ../include -L ../../target/debug -lclsb -o harness.exe
 *   (copiar clsb.dll junto al exe o agregar el dir al PATH)
 *
 * Ejecutar: harness.exe  -> imprime OK por cada prueba (o FAIL + detalle).
 */
#include <clsb.h>
#include <stdio.h>
#include <string.h>

static int checks = 0;
static int fails = 0;

static void check(int ok, const char* what) {
    checks++;
    if (!ok) {
        fails++;
        printf("FAIL: %s\n", what);
    } else {
        printf("ok:   %s\n", what);
    }
}

/* Captura de print del script */
static char out_buf[4096];
static size_t out_len = 0;
static void on_output(void* ud, const char* text, int is_end) {
    (void)ud;
    if (is_end) {
        out_buf[out_len] = '\0';
        out_len = 0;
    } else if (text && out_len + strlen(text) < sizeof(out_buf) - 1) {
        out_len += (size_t)snprintf(out_buf + out_len, sizeof(out_buf) - out_len, "%s", text);
    }
}

/* Función host del nodo: duplicar(i) -> i */
static int host_duplicar(void* ud, uint32_t id, const clsb_value* args,
                         size_t n, clsb_value* out) {
    (void)ud; (void)id; (void)n;
    *out = clsb_value_int(args[0].bits * 2);
    return 0;
}

int main(void) {
    const char* src =
        "export function suma(a: int, b: int) -> int { return a + b; }\n"
        "export function saludo(n: String) -> String { return \"hola \" + n; }\n"
        "export function doble_f(x: float) -> float { return x * 2.0; }\n"
        "export function total(ns: int[]) -> int { var t: int = 0; for each n in (ns) { t += n; } return t; }\n"
        "function main(args: String[]) -> int { print(\"main:\", args[0]); return 0; }\n";

    clsb_engine* engine = clsb_engine_new(NULL);
    check(engine != NULL, "engine_new");

    clsb_error* err = NULL;
    clsb_module* m = clsb_compile_source(engine, src, "harness", ".", &err);
    check(m != NULL && err == NULL, "compile_source");

    /* call: suma(20, 22) -> 42 */
    clsb_value args[2] = { clsb_value_int(20), clsb_value_int(22) };
    clsb_value out = clsb_value_null();
    clsb_status st = clsb_call(m, "suma", args, 2, &out, &err);
    check(st == CLSB_OK && out.tag == CLSB_INT && out.bits == 42, "call suma -> 42");
    clsb_value_free(&out);
    clsb_value_free(&args[0]); clsb_value_free(&args[1]);

    /* call: saludo("mundo") -> "hola mundo" */
    clsb_value s = clsb_value_string("mundo");
    st = clsb_call(m, "saludo", &s, 1, &out, &err);
    check(st == CLSB_OK && out.tag == CLSB_STRING && strcmp(out.text, "hola mundo") == 0,
          "call saludo -> 'hola mundo'");
    clsb_value_free(&s);
    clsb_value_free(&out);

    /* call: doble_f(2.5) -> 5.0 */
    clsb_value f = clsb_value_float(2.5);
    st = clsb_call(m, "doble_f", &f, 1, &out, &err);
    check(st == CLSB_OK && out.tag == CLSB_FLOAT &&
          ((double)out.bits == 0 || out.bits != 0), "call doble_f (float)");
    if (st == CLSB_OK) {
        double v;
        memcpy(&v, &out.bits, sizeof(v));
        check(v == 5.0, "call doble_f -> 5.0");
    }
    clsb_value_free(&f);
    clsb_value_free(&out);

    /* call: total([1,2,3]) -> 6 */
    clsb_value arr = clsb_value_array(3);
    arr.items[0] = clsb_value_int(1);
    arr.items[1] = clsb_value_int(2);
    arr.items[2] = clsb_value_int(3);
    st = clsb_call(m, "total", &arr, 1, &out, &err);
    check(st == CLSB_OK && out.tag == CLSB_INT && out.bits == 6, "call total([1,2,3]) -> 6");
    clsb_value_free(&arr);
    clsb_value_free(&out);

    /* run_main(["hola"]) -> exit 0 + print capturado */
    clsb_value marg = clsb_value_string("hola");
    clsb_set_output(engine, on_output, NULL);
    int64_t code = clsb_run_main(m, &marg, 1, &err);
    check(code == 0 && strcmp(out_buf, "main: hola") == 0, "run_main + print capturado");
    clsb_value_free(&marg);

    /* eval */
    st = clsb_eval(engine, "export function siete() -> int { return 7; };", &out, &err);
    check(st == CLSB_OK && out.tag == CLSB_INT && out.bits == 7, "eval -> 7");
    clsb_value_free(&out);

    /* host function del nodo */
    clsb_register_host_function(engine, "duplicar", "i(i)", host_duplicar, NULL);
    st = clsb_eval(engine,
                   "export function usa() -> int { return duplicar(21); };",
                   &out, &err);
    check(st == CLSB_OK && out.tag == CLSB_INT && out.bits == 42, "host fn duplicar -> 42");
    clsb_value_free(&out);

    /* error: export inexistente -> trace con mensaje */
    err = NULL;
    st = clsb_call(m, "no_existe", NULL, 0, &out, &err);
    check(st != CLSB_OK && err != NULL && clsb_error_trace(err) != NULL,
          "call inexistente -> error con trace");
    if (err) clsb_error_free(err);

    /* error de sintaxis en compile */
    err = NULL;
    clsb_module* bad = clsb_compile_source(engine, "function main( {", "bad", ".", &err);
    check(bad == NULL && err != NULL && clsb_error_trace(err) != NULL,
          "source inválido -> error de sintaxis");
    if (bad) clsb_module_free(bad);
    if (err) clsb_error_free(err);

    /* version */
    check(strlen(clsb_version()) > 0, "version no vacía");

    clsb_module_free(m);
    clsb_engine_free(engine);

    printf("\n%d checks, %d fails\n", checks, fails);
    return fails == 0 ? 0 : 1;
}

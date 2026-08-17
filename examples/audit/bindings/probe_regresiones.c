#include <clsb.h>
#include <stdio.h>
#include <string.h>

static int checks = 0, fails = 0;
static void check(int ok, const char* what) {
    checks++;
    if (!ok) { fails++; printf("FAIL: %s\n", what); } else printf("ok:   %s\n", what);
}

static int host_calls = 0;
static int host_triple_i(void* ud, uint32_t id, const clsb_value* args, size_t n, clsb_value* out) {
    (void)ud; (void)id; (void)n;
    host_calls++;
    printf("    [cb i(i)] id=%u n=%zu bits=%lld\n", id, n, (long long)args[0].bits);
    *out = clsb_value_int(args[0].bits * 3);
    return 0;
}
static int host_triple_f(void* ud, uint32_t id, const clsb_value* args, size_t n, clsb_value* out) {
    (void)ud; (void)id; (void)n;
    double v; memcpy(&v, &args[0].bits, sizeof(v));
    *out = clsb_value_float(v * 3.0);
    return 0;
}
static int host_greet_s(void* ud, uint32_t id, const clsb_value* args, size_t n, clsb_value* out) {
    (void)ud; (void)id; (void)args; (void)n;
    *out = clsb_value_string("saludo desde C");
    return 0;
}

static void show(const char* lbl, clsb_status st, const clsb_value* out, clsb_error* err) {
    printf("  %s: st=%d tag=%d bits=%lld text=%s\n", lbl, st, out->tag,
           (long long)out->bits, out->text ? out->text : "(null)");
    if (err) { printf("  [err] %s\n", clsb_error_trace(err)); clsb_error_free(err); }
}

int main(void) {
    setvbuf(stdout, NULL, _IONBF, 0);
    clsb_engine* e = clsb_engine_new(NULL);
    clsb_status rc;
    rc = clsb_register_host_function(e, "triplicar_i", "i(i)", host_triple_i, NULL);
    rc |= clsb_register_host_function(e, "triplicar_f", "f(f)", host_triple_f, NULL);
    rc |= clsb_register_host_function(e, "greet", "s()", host_greet_s, NULL);
    check(rc == 0, "registro de host fns");

    clsb_error* err = NULL;
    /* caso 1: clsb_call a usa_i */
    clsb_module* m = clsb_compile_source(e,
        "export function usa_i() -> int { return triplicar_i(14); }\n"
        "export function usa_f(x: float) -> float { return triplicar_f(x); }\n"
        "export function usa_s() -> String { return greet(); }\n",
        "p", ".", &err);
    check(m != NULL, "compile host module");
    if (err) { printf("  [compile] %s\n", clsb_error_trace(err)); clsb_error_free(err); }

    clsb_value out = clsb_value_null();
    clsb_status st = clsb_call(m, "usa_i", NULL, 0, &out, &err);
    check(st == CLSB_OK && out.tag == CLSB_INT && out.bits == 42, "clsb_call usa_i -> 42");
    show("call usa_i", st, &out, err);
    clsb_value_free(&out); out = clsb_value_null();

    clsb_value f = clsb_value_float(2.0);
    st = clsb_call(m, "usa_f", &f, 1, &out, &err);
    double fv = 0; if (st == CLSB_OK && out.tag == CLSB_FLOAT) memcpy(&fv, &out.bits, sizeof(fv));
    check(st == CLSB_OK && out.tag == CLSB_FLOAT && fv == 6.0, "clsb_call usa_f -> 6.0");
    show("call usa_f", st, &out, err);
    clsb_value_free(&out); out = clsb_value_null();
    clsb_value_free(&f);

    st = clsb_call(m, "usa_s", NULL, 0, &out, &err);
    check(st == CLSB_OK && out.tag == CLSB_STRING && out.text &&
          strcmp(out.text, "saludo desde C") == 0, "clsb_call usa_s -> string");
    show("call usa_s", st, &out, err);
    clsb_value_free(&out); out = clsb_value_null();

    /* caso 2: clsb_eval con la misma fn */
    st = clsb_eval(e, "export function usa2() -> int { return triplicar_i(14); };", &out, &err);
    check(st == CLSB_OK && out.tag == CLSB_INT && out.bits == 42, "clsb_eval usa2 -> 42");
    show("eval usa2", st, &out, err);
    clsb_value_free(&out); out = clsb_value_null();

    st = clsb_eval(e, "export function usa3() -> String { return greet(); };", &out, &err);
    check(st == CLSB_OK && out.tag == CLSB_STRING && out.text &&
          strcmp(out.text, "saludo desde C") == 0, "clsb_eval usa3 -> string");
    show("eval usa3", st, &out, err);
    clsb_value_free(&out); out = clsb_value_null();

    /* caso 3: record return */
    clsb_module* mr = clsb_compile_source(e,
        "export function frec() -> Record<String, int> { return { \"a\": 1, \"b\": 2 }; }\n",
        "r", ".", &err);
    check(mr != NULL, "compile record module");
    if (err) { printf("  [compile] %s\n", clsb_error_trace(err)); clsb_error_free(err); }
    st = clsb_call(mr, "frec", NULL, 0, &out, &err);
    int rok = st == CLSB_OK && out.tag == CLSB_RECORD && out.n == 2 && out.keys && out.vals &&
              out.keys[0] && strcmp(out.keys[0], "a") == 0 && out.vals[0].bits == 1;
    check(rok, "frec -> Record{a:1,b:2} via call");
    printf("  frec: st=%d tag=%d n=%zu k0=%s v0=%lld\n", st, out.tag, out.n,
           (out.keys && out.keys[0]) ? out.keys[0] : "(null)",
           out.vals ? (long long)out.vals[0].bits : -1);
    clsb_value_free(&out); out = clsb_value_null();

    /* caso 4: array de strings param */
    clsb_module* ma = clsb_compile_source(e,
        "export function join_strs(xs: String[]) -> String { var s: String = \"\"; for each x in (xs) { s += x; } return s; }\n",
        "a", ".", &err);
    check(ma != NULL, "compile array module");
    if (err) { printf("  [compile] %s\n", clsb_error_trace(err)); clsb_error_free(err); }
    clsb_value arrs = clsb_value_array(3);
    arrs.items[0] = clsb_value_string("uno");
    arrs.items[1] = clsb_value_string("dos");
    arrs.items[2] = clsb_value_string("tres");
    st = clsb_call(ma, "join_strs", &arrs, 1, &out, &err);
    check(st == CLSB_OK && out.tag == CLSB_STRING && out.text &&
          strcmp(out.text, "unodostres") == 0, "join_strs -> 'unodostres'");
    show("call join_strs", st, &out, err);
    clsb_value_free(&out); out = clsb_value_null();
    clsb_value_free(&arrs);

    /* caso 5: array de ints (control) */
    clsb_module* mi = clsb_compile_source(e,
        "export function total(xs: int[]) -> int { var t: int = 0; for each n in (xs) { t += n; } return t; }\n",
        "i", ".", &err);
    clsb_value arri = clsb_value_array(3);
    arri.items[0] = clsb_value_int(10);
    arri.items[1] = clsb_value_int(20);
    arri.items[2] = clsb_value_int(30);
    st = clsb_call(mi, "total", &arri, 1, &out, &err);
    check(st == CLSB_OK && out.tag == CLSB_INT && out.bits == 60, "total([10,20,30]) -> 60");
    show("call total", st, &out, err);
    clsb_value_free(&out); out = clsb_value_null();
    clsb_value_free(&arri);

    clsb_module_free(mi);
    clsb_module_free(ma);
    clsb_module_free(mr);
    clsb_module_free(m);
    clsb_engine_free(e);
    printf("\n%d checks, %d fails\n", checks, fails);
    return fails == 0 ? 0 : 1;
}

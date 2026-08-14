#include <clsb.h>
#include <stdio.h>

static int host_char_x(void* ud, uint32_t id, const clsb_value* args, size_t n, clsb_value* out) {
    (void)ud; (void)id; (void)args; (void)n;
    *out = clsb_value_char('X');
    return 0;
}

int main(void) {
    clsb_engine* e = clsb_engine_new(NULL);
    clsb_status rc = clsb_register_host_function(e, "obtenerX", "c()", host_char_x, NULL);
    printf("register rc=%d\n", rc);
    clsb_error* err = NULL;
    clsb_module* m = clsb_compile_source(e,
        "export function fint() -> int { return -42; }\n"
        "export function ffloat() -> float { return 3.5; }\n"
        "export function fbool() -> bool { return true; }\n"
        "export function fchar() -> char { return obtenerX(); }\n"
        "export function fstr() -> String { return \"hola bindings\"; }\n"
        "export function farr() -> int[] { return [1, 2, 3]; }\n"
        "export function frec() -> Record<String, int> { return { \"a\": 1, \"b\": 2 }; }\n"
        "export function echo_str(s: String) -> String { return \"[\" + s + \"]\"; }\n"
        "export function sum_ints(xs: int[]) -> int { var t: int = 0; for each n in (xs) { t += n; } return t; }\n"
        "export function join_strs(xs: String[]) -> String { var s: String = \"\"; for each x in (xs) { s += x; } return s; }\n"
        "export function get_rec(d: Record<String, int>, k: String) -> int { return d[k]; }\n"
        "export function add_f(a: float, b: float) -> float { return a + b; }\n"
        "export function and_b(a: bool, b: bool) -> bool { return a && b; }\n"
        "export function echo_c(c: char) -> char { return c; }\n"
        "export function mayus(s: String) -> String { return s.upper(); }\n"
        "export function div(a: int, b: int) -> int { return a / b; }\n"
        "export function suma(a: int, b: int) -> int { return a + b; }\n",
        "p", ".", &err);
    printf("module=%p\n", (void*)m);
    if (err) { printf("ERR: %s\n", clsb_error_trace(err)); clsb_error_free(err); return 1; }
    clsb_value out = clsb_value_null();
    clsb_status st = clsb_call(m, "fchar", NULL, 0, &out, &err);
    printf("call st=%d tag=%d bits=%lld\n", st, out.tag, (long long)out.bits);
    if (err) { printf("ERR: %s\n", clsb_error_trace(err)); clsb_error_free(err); }
    clsb_value_free(&out);
    clsb_module_free(m);
    clsb_engine_free(e);
    return 0;
}

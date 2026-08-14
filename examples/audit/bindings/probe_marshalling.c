#include <clsb.h>
#include <stdio.h>
#include <string.h>

static void show(const char* lbl, clsb_status st, const clsb_value* out, clsb_error* err) {
    printf("  %s: st=%d tag=%d n=%zu bits=%lld text=%s\n", lbl, st, out->tag, out->n,
           (long long)out->bits, out->text ? out->text : "(null)");
    if (out->tag == CLSB_RECORD && out->keys && out->vals) {
        for (size_t i = 0; i < out->n && i < 4; i++) {
            printf("    rec[%zu]: key='%s' val_bits=%lld\n", i,
                   out->keys[i] ? out->keys[i] : "(null)",
                   (long long)out->vals[i].bits);
        }
    }
    if (err) { printf("  [err] %s\n", clsb_error_trace(err)); clsb_error_free(err); }
}

int main(void) {
    setvbuf(stdout, NULL, _IONBF, 0);
    clsb_engine* e = clsb_engine_new(NULL);
    clsb_error* err = NULL;
    clsb_value out = clsb_value_null();

    clsb_module* m = clsb_compile_source(e,
        "export function frec_lit() -> Record<String, int> { return { \"a\": 1, \"b\": 2 }; }\n"
        "export function frec_var() -> Record<String, int> { var d: Record<String, int> = { \"a\": 1, \"b\": 2 }; return d; }\n"
        "export function rec_nested() -> Record<String, Record<String, int> > { var d: Record<String, Record<String, int> > = { \"k\": { \"x\": 9 } }; return d; }\n"
        "export function slen(xs: String[]) -> int { return xs.length; }\n"
        "export function sjoin(xs: String[]) -> String { var s: String = \"\"; for each x in (xs) { s += x; } return s; }\n"
        "export function total(xs: int[]) -> int { var t: int = 0; for each n in (xs) { t += n; } return t; }\n",
        "r", ".", &err);
    if (!m) { printf("compile fail: %s\n", clsb_error_trace(err)); return 1; }

    clsb_status st = clsb_call(m, "frec_lit", NULL, 0, &out, &err);
    printf("[1] frec_lit (literal en return):\n"); show("frec_lit", st, &out, err);
    clsb_value_free(&out); out = clsb_value_null();

    st = clsb_call(m, "frec_var", NULL, 0, &out, &err);
    printf("[2] frec_var (var tipada):\n"); show("frec_var", st, &out, err);
    clsb_value_free(&out); out = clsb_value_null();

    st = clsb_call(m, "rec_nested", NULL, 0, &out, &err);
    printf("[3] rec_nested:\n"); show("rec_nested", st, &out, err);
    clsb_value_free(&out); out = clsb_value_null();

    clsb_value arrs = clsb_value_array(3);
    arrs.items[0] = clsb_value_string("uno");
    arrs.items[1] = clsb_value_string("dos");
    arrs.items[2] = clsb_value_string("tres");
    st = clsb_call(m, "slen", &arrs, 1, &out, &err);
    printf("[4] slen (xs.length de String[]):\n"); show("slen", st, &out, err);
    clsb_value_free(&out); out = clsb_value_null();

    st = clsb_call(m, "sjoin", &arrs, 1, &out, &err);
    printf("[5] sjoin (for-each + concat de String[]):\n"); show("sjoin", st, &out, err);
    clsb_value_free(&out); out = clsb_value_null();
    clsb_value_free(&arrs);

    clsb_value arri = clsb_value_array(3);
    arri.items[0] = clsb_value_int(10);
    arri.items[1] = clsb_value_int(20);
    arri.items[2] = clsb_value_int(30);
    st = clsb_call(m, "total", &arri, 1, &out, &err);
    printf("[6] total (int[] control):\n"); show("total", st, &out, err);
    clsb_value_free(&out); out = clsb_value_null();
    clsb_value_free(&arri);

    clsb_module_free(m);
    clsb_engine_free(e);
    return 0;
}

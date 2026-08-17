#include <clsb.h>
#include <stdio.h>

static int host_triple_i(void* ud, uint32_t id, const clsb_value* args, size_t n, clsb_value* out) {
    (void)ud; (void)id; (void)n;
    *out = clsb_value_int(args[0].bits * 3);
    return 0;
}

int main(void) {
    setvbuf(stdout, NULL, _IONBF, 0);
    clsb_engine* e = clsb_engine_new(NULL);
    clsb_status rc = clsb_register_host_function(e, "triplicar_i", "i(i)", host_triple_i, NULL);
    printf("register rc=%d\n", rc);
    clsb_error* err = NULL;
    clsb_value out = clsb_value_null();
    clsb_status st = clsb_eval(e,
        "export function usa() -> int { return triplicar_i(21); };",
        &out, &err);
    printf("eval st=%d tag=%d bits=%lld\n", st, out.tag, (long long)out.bits);
    if (err) { printf("ERR: %s\n", clsb_error_trace(err)); clsb_error_free(err); }
    clsb_value_free(&out);
    clsb_engine_free(e);
    return 0;
}

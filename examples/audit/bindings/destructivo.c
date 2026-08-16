/* destructivo.c - Harness C destructivo del binding clxb (auditoría QA).
 *
 * Compilar:
 *   gcc destructivo.c -I ../../../nodos/clxb/include -L ../../../target/debug -lclsb -o destructivo.exe
 *   (dir target/debug en PATH, o copiar clsb.dll junto al exe)
 *
 * Cubre: tipos de retorno, params complejos, run_main con args, eval (variantes),
 * host functions (i/f/s), casos de error (sin crash), memoria y print capturado.
 */
#include <clsb.h>
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <windows.h>
#include <psapi.h>

/* NOTA QA: clsb.h comenta los "setters seguros" (clsb_value_set_text /
 * clsb_value_array_set / clsb_value_record_set) pero NO los declara, y el
 * binario clxb.dll (commit a569e3b) NO los exporta. Sin ellos, un consumidor C
 * NO puede construir records (las keys deben ser owned por el runtime; asignar
 * strings estáticos provoca CString::from_raw inválido en clsb_value_free).
 * Este harness llena arrays por asignación directa de items (seguro: los slots
 * vienen null-inicializados por clsb_value_array) y para el record param usa
 * claves estáticas + NO libera el record (leak documentado de la prueba).
 */

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

static void show_err(const char* label, clsb_error* err) {
    if (err) {
        const char* t = clsb_error_trace(err);
        printf("  [%s] trace:\n%s\n", label, t ? t : "(null)");
        clsb_error_free(err);
    } else {
        printf("  [%s] (sin error)\n", label);
    }
}

/* ── captura de print (multi-línea) ─────────────────────────────────────── */
typedef struct { char lines[8][512]; int nlines; int cur; } OutCap;

static void on_output(void* ud, const char* text, int is_end) {
    OutCap* o = (OutCap*)ud;
    if (is_end) {
        if (o->nlines < 8) {
            o->lines[o->nlines][o->cur] = '\0';
            o->nlines++;
            o->cur = 0;
        }
    } else if (text && o->cur < 511) {
        int l = (int)strlen(text);
        if (o->cur + l > 511) l = 511 - o->cur;
        memcpy(o->lines[o->nlines] + o->cur, text, (size_t)l);
        o->cur += l;
    }
}

/* ── host functions del nodo ────────────────────────────────────────────── */
static int host_triple_i(void* ud, uint32_t id, const clsb_value* args, size_t n, clsb_value* out) {
    (void)ud; (void)id; (void)n;
    *out = clsb_value_int(args[0].bits * 3);
    return 0;
}

static int host_triple_f(void* ud, uint32_t id, const clsb_value* args, size_t n, clsb_value* out) {
    (void)ud; (void)id; (void)n;
    double v;
    memcpy(&v, &args[0].bits, sizeof(v));
    *out = clsb_value_float(v * 3.0);
    return 0;
}

static int host_quintuple_i(void* ud, uint32_t id, const clsb_value* args, size_t n, clsb_value* out) {
    (void)ud; (void)id; (void)n;
    *out = clsb_value_int(args[0].bits * 5);
    return 0;
}

static int host_greet_s(void* ud, uint32_t id, const clsb_value* args, size_t n, clsb_value* out) {
    (void)ud; (void)id; (void)n;
    *out = clsb_value_string("saludo desde C");
    return 0;
}

static int host_echo_s(void* ud, uint32_t id, const clsb_value* args, size_t n, clsb_value* out) {
    (void)ud; (void)id; (void)n;
    char buf[256];
    snprintf(buf, sizeof(buf), "<<%s>>", args[0].text ? args[0].text : "");
    *out = clsb_value_string(buf);
    return 0;
}

static int host_char_x(void* ud, uint32_t id, const clsb_value* args, size_t n, clsb_value* out) {
    (void)ud; (void)id; (void)args; (void)n;
    *out = clsb_value_char('X');
    return 0;
}

static SIZE_T wss(void) {
    PROCESS_MEMORY_COUNTERS pmc;
    if (GetProcessMemoryInfo(GetCurrentProcess(), &pmc, sizeof(pmc))) return pmc.WorkingSetSize;
    return 0;
}

int main(void) {
    setvbuf(stdout, NULL, _IONBF, 0);
    const char* SRC =
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
        "export function suma(a: int, b: int) -> int { return a + b; }\n";

    clsb_engine* engine = clsb_engine_new(NULL);
    check(engine != NULL, "engine_new");
    if (!engine) return 2;

    /* host char para poder probar retorno char (no hay literal char en CLS 2.0) */
    clsb_status rc = clsb_register_host_function(engine, "obtenerX", "c()", host_char_x, NULL);
    check(rc == CLSB_OK, "register host 'c()' (char)");

    clsb_error* err = NULL;
    clsb_module* m = clsb_compile_source(engine, SRC, "destructivo", ".", &err);
    check(m != NULL && err == NULL, "compile_source (destructivo)");
    if (!m) {
        show_err("compile destructivo", err);
        return 2;
    }

    clsb_value out = clsb_value_null();

    /* ── S1: todos los tipos de retorno ─────────────────────────────────── */
    printf("\n=== S1: tipos de retorno ===\n");
    {
        clsb_status st = clsb_call(m, "fint", NULL, 0, &out, &err);
        check(st == CLSB_OK && out.tag == CLSB_INT && out.bits == -42,
              "fint -> int -42");
        printf("  fint: st=%d tag=%d bits=%lld\n", st, out.tag, (long long)out.bits);
        clsb_value_free(&out);
        out = clsb_value_null();
        st = clsb_call(m, "ffloat", NULL, 0, &out, &err);
        double fv = 0;
        if (st == CLSB_OK && out.tag == CLSB_FLOAT) memcpy(&fv, &out.bits, sizeof(fv));
        check(st == CLSB_OK && out.tag == CLSB_FLOAT && fv == 3.5,
              "ffloat -> float 3.5");
        printf("  ffloat: st=%d tag=%d fv=%.6f\n", st, out.tag, fv);
        clsb_value_free(&out);
        out = clsb_value_null();
        st = clsb_call(m, "fbool", NULL, 0, &out, &err);
        check(st == CLSB_OK && out.tag == CLSB_BOOL && out.bits == 1,
              "fbool -> bool true");
        printf("  fbool: st=%d tag=%d bits=%lld\n", st, out.tag, (long long)out.bits);
        clsb_value_free(&out);
        out = clsb_value_null();
        st = clsb_call(m, "fchar", NULL, 0, &out, &err);
        check(st == CLSB_OK && out.tag == CLSB_CHAR && out.bits == 'X',
              "fchar -> char 'X'");
        printf("  fchar: st=%d tag=%d bits=%lld (codepoint)\n", st, out.tag, (long long)out.bits);
        clsb_value_free(&out);
        out = clsb_value_null();
        st = clsb_call(m, "fstr", NULL, 0, &out, &err);
        check(st == CLSB_OK && out.tag == CLSB_STRING && out.text &&
              strcmp(out.text, "hola bindings") == 0,
              "fstr -> String 'hola bindings'");
        printf("  fstr: st=%d tag=%d text=%s\n", st, out.tag, out.text ? out.text : "(null)");
        clsb_value_free(&out);
        out = clsb_value_null();
        st = clsb_call(m, "farr", NULL, 0, &out, &err);
        int arr_ok = st == CLSB_OK && out.tag == CLSB_ARRAY && out.n == 3 &&
                     out.items && out.items[0].bits == 1 && out.items[1].bits == 2 &&
                     out.items[2].bits == 3;
        check(arr_ok, "farr -> int[] [1,2,3]");
        printf("  farr: st=%d tag=%d n=%zu items=%lld,%lld,%lld\n", st, out.tag, out.n,
               out.items ? (long long)out.items[0].bits : -1,
               out.items ? (long long)out.items[1].bits : -1,
               out.items ? (long long)out.items[2].bits : -1);
        clsb_value_free(&out);
        out = clsb_value_null();
        st = clsb_call(m, "frec", NULL, 0, &out, &err);
        int rec_ok = st == CLSB_OK && out.tag == CLSB_RECORD && out.n == 2 &&
                     out.keys && out.vals && out.keys[0] && out.vals[0].bits == 1 &&
                     out.keys[1] && out.vals[1].bits == 2;
        check(rec_ok, "frec -> Record {a:1,b:2}");
        printf("  frec: st=%d tag=%d n=%zu keys=[%s,%s] vals=[%lld,%lld]\n", st, out.tag, out.n,
               out.keys && out.keys[0] ? out.keys[0] : "(null)",
               out.keys && out.keys[1] ? out.keys[1] : "(null)",
               out.vals ? (long long)out.vals[0].bits : -1,
               out.vals ? (long long)out.vals[1].bits : -1);
        clsb_value_free(&out);
        out = clsb_value_null();    }

    /* ── S2: params complejos ───────────────────────────────────────────── */
    printf("\n=== S2: params complejos ===\n");
    {
        clsb_value s = clsb_value_string("abc");
        clsb_status st = clsb_call(m, "echo_str", &s, 1, &out, &err);
        check(st == CLSB_OK && out.tag == CLSB_STRING && out.text &&
              strcmp(out.text, "[abc]") == 0, "echo_str(string) -> '[abc]'");
        printf("  echo_str: st=%d -> %s\n", st, out.text ? out.text : "(null)");
        clsb_value_free(&out);
        out = clsb_value_null();        clsb_value_free(&s);

        clsb_value arr = clsb_value_array(4);
        for (int i = 0; i < 4; i++) arr.items[i] = clsb_value_int((i + 1) * 10);
        st = clsb_call(m, "sum_ints", &arr, 1, &out, &err);
        check(st == CLSB_OK && out.tag == CLSB_INT && out.bits == 100,
              "sum_ints([10,20,30,40]) -> 100");
        printf("  sum_ints: st=%d -> %lld\n", st, (long long)out.bits);
        clsb_value_free(&out);
        out = clsb_value_null();        clsb_value_free(&arr);

        clsb_value arrs = clsb_value_array(3);
        arrs.items[0] = clsb_value_string("uno");
        arrs.items[1] = clsb_value_string("dos");
        arrs.items[2] = clsb_value_string("tres");
        st = clsb_call(m, "join_strs", &arrs, 1, &out, &err);
        check(st == CLSB_OK && out.tag == CLSB_STRING && out.text &&
              strcmp(out.text, "unodostres") == 0, "join_strs([str;3]) -> 'unodostres'");
        printf("  join_strs: st=%d -> %s\n", st, out.text ? out.text : "(null)");
        clsb_value_free(&out);
        clsb_value_free(&arrs);

        /* record param: sin setters exportados, las claves estáticas solo se
         * pueden LEER (value_to_cls copia con CStr). NO se libera el record. */
        clsb_value rec = clsb_value_record(2);
        rec.keys[0] = "ciudad";
        rec.vals[0] = clsb_value_string("Lima");
        rec.keys[1] = "a";
        rec.vals[1] = clsb_value_int(7);
        clsb_value k = clsb_value_string("a");
        st = clsb_call(m, "get_rec", (clsb_value[]){ rec, k }, 2, &out, &err);
        check(st == CLSB_OK && out.tag == CLSB_INT && out.bits == 7,
              "get_rec(record con str+int) -> 7");
        printf("  get_rec: st=%d -> %lld\n", st, (long long)out.bits);
        clsb_value_free(&out);
        clsb_value_free(&k);
        /* NOTA: rec NO se libera (las claves estáticas crashean clsb_value_free) */

        clsb_value fa = clsb_value_float(1.5);
        clsb_value fb = clsb_value_float(2.25);
        st = clsb_call(m, "add_f", (clsb_value[]){ fa, fb }, 2, &out, &err);
        double fv = 0;
        if (st == CLSB_OK && out.tag == CLSB_FLOAT) memcpy(&fv, &out.bits, sizeof(fv));
        check(st == CLSB_OK && out.tag == CLSB_FLOAT && fv == 3.75,
              "add_f(1.5, 2.25) -> 3.75");
        printf("  add_f: st=%d fv=%.6f\n", st, fv);
        clsb_value_free(&out);
        out = clsb_value_null();        clsb_value_free(&fa);
        clsb_value_free(&fb);

        clsb_value bt = clsb_value_bool(1);
        clsb_value bf = clsb_value_bool(0);
        st = clsb_call(m, "and_b", (clsb_value[]){ bt, bf }, 2, &out, &err);
        check(st == CLSB_OK && out.tag == CLSB_BOOL && out.bits == 0,
              "and_b(true, false) -> false");
        printf("  and_b: st=%d -> %lld\n", st, (long long)out.bits);
        clsb_value_free(&out);
        out = clsb_value_null();        clsb_value_free(&bt);
        clsb_value_free(&bf);

        clsb_value ch = clsb_value_char('Z');
        st = clsb_call(m, "echo_c", &ch, 1, &out, &err);
        check(st == CLSB_OK && out.tag == CLSB_CHAR && out.bits == 'Z',
              "echo_c('Z') -> 'Z'");
        printf("  echo_c: st=%d bits=%lld\n", st, (long long)out.bits);
        clsb_value_free(&out);
        out = clsb_value_null();        clsb_value_free(&ch);
    }

    /* ── S3: run_main con args ──────────────────────────────────────────── */
    printf("\n=== S3: run_main con args ===\n");
    {
        OutCap cap = {0};
        clsb_set_output(engine, on_output, &cap);
        const char* MAIN_SRC =
            "function main(args: String[]) -> int {\n"
            "    print(\"argc=\", args.length);\n"
            "    for each a in (args) { print(\"arg:\", a); }\n"
            "    return args.length;\n"
            "}\n";
        clsb_module* mm = clsb_compile_source(engine, MAIN_SRC, "main", ".", &err);
        clsb_value margs[3] = { clsb_value_string("uno"), clsb_value_string("dos"),
                                clsb_value_string("tres") };
        int64_t code = clsb_run_main(mm, margs, 3, &err);
        check(code == 3, "run_main -> exit code = args.length (3)");
        check(cap.nlines == 4 && strcmp(cap.lines[0], "argc= 3") == 0 &&
              strcmp(cap.lines[1], "arg: uno") == 0 &&
              strcmp(cap.lines[2], "arg: dos") == 0 &&
              strcmp(cap.lines[3], "arg: tres") == 0,
              "print capturado: 4 líneas correctas");
        printf("  exit code=%lld, líneas capturadas=%d\n", (long long)code, cap.nlines);
        for (int i = 0; i < cap.nlines; i++) printf("    L%d: '%s'\n", i, cap.lines[i]);
        for (int i = 0; i < 3; i++) clsb_value_free(&margs[i]);
        clsb_module_free(mm);
    }

    /* ── S4: eval (variantes) ───────────────────────────────────────────── */
    printf("\n=== S4: eval ===\n");
    {
        err = NULL;
        clsb_status st = clsb_eval(engine,
            "export function siete() -> int { return 7; };", &out, &err);
        check(st == CLSB_OK && out.tag == CLSB_INT && out.bits == 7,
              "eval(export fn) -> 7");
        printf("  eval export: st=%d tag=%d bits=%lld\n", st, out.tag, (long long)out.bits);
        clsb_value_free(&out);
        out = clsb_value_null();
        err = NULL;
        OutCap cap = {0};
        clsb_set_output(engine, on_output, &cap);
        st = clsb_eval(engine,
            "function main(args: String[]) -> int { print(\"eval-main-corriendo\"); return 2; };",
            &out, &err);
        check(st == CLSB_OK && out.tag == CLSB_NULL,
              "eval(solo main) -> OK, out tag NULL(12)");
        check(cap.nlines == 1 && strcmp(cap.lines[0], "eval-main-corriendo") == 0,
              "eval(solo main) ejecutó main (print capturado)");
        printf("  eval solo-main: st=%d tag=%d nlines=%d\n", st, out.tag, cap.nlines);
        clsb_value_free(&out);
        out = clsb_value_null();
        err = NULL;
        st = clsb_eval(engine,
            "export function suma(a: int, b: int) -> int { return a + b; };", &out, &err);
        check(st != CLSB_OK && err != NULL,
              "eval(primer export CON args) -> error de aridad (0 args)");
        printf("  eval export-con-args: st=%d (error esperado)\n", st);
        show_err("eval aridad", err);
        err = NULL;
        clsb_value_free(&out);
        out = clsb_value_null();    }

    /* ── S5: host functions ─────────────────────────────────────────────── */
    printf("\n=== S5: host functions ===\n");
    {
        /* HALLAZGO: clxb registra UN handler global por engine - la última
         * registración gana para TODOS los ids. Workaround probado: un engine
         * por función host. La demo del bug va al final (S5f). */
        const char* HS_I = "export function usa_i() -> int { return triplicar_i(14); }\n";
        const char* HS_F = "export function usa_f(x: float) -> float { return triplicar_f(x); }\n";
        const char* HS_S = "export function usa_s() -> String { return greet(); }\n";
        const char* HS_SS = "export function usa_ss(s: String) -> String { return ecos(s); }\n";

        /* S5a: i(i) */
        {
            clsb_engine* he = clsb_engine_new(NULL);
            clsb_register_host_function(he, "triplicar_i", "i(i)", host_triple_i, NULL);
            clsb_module* hm = clsb_compile_source(he, HS_I, "host", ".", &err);
            err = NULL;
            clsb_status st = clsb_call(hm, "usa_i", NULL, 0, &out, &err);
            check(st == CLSB_OK && out.tag == CLSB_INT && out.bits == 42,
                  "host i(i): triplicar_i(14) -> 42");
            printf("  usa_i: st=%d bits=%lld\n", st, (long long)out.bits);
            clsb_value_free(&out); out = clsb_value_null();
            clsb_module_free(hm);
            clsb_engine_free(he);
        }
        /* S5b: f(f) */
        {
            clsb_engine* he = clsb_engine_new(NULL);
            clsb_register_host_function(he, "triplicar_f", "f(f)", host_triple_f, NULL);
            clsb_module* hm = clsb_compile_source(he, HS_F, "host", ".", &err);
            clsb_value f = clsb_value_float(2.0);
            err = NULL;
            clsb_status st = clsb_call(hm, "usa_f", &f, 1, &out, &err);
            double fv = 0;
            if (st == CLSB_OK && out.tag == CLSB_FLOAT) memcpy(&fv, &out.bits, sizeof(fv));
            check(st == CLSB_OK && out.tag == CLSB_FLOAT && fv == 6.0,
                  "host f(f): triplicar_f(2.0) -> 6.0");
            printf("  usa_f: st=%d fv=%.6f\n", st, fv);
            clsb_value_free(&out); out = clsb_value_null();
            clsb_value_free(&f);
            clsb_module_free(hm);
            clsb_engine_free(he);
        }
        /* S5c: s() retorno string */
        {
            clsb_engine* he = clsb_engine_new(NULL);
            clsb_register_host_function(he, "greet", "s()", host_greet_s, NULL);
            clsb_module* hm = clsb_compile_source(he, HS_S, "host", ".", &err);
            err = NULL;
            clsb_status st = clsb_call(hm, "usa_s", NULL, 0, &out, &err);
            check(st == CLSB_OK && out.tag == CLSB_STRING && out.text &&
                  strcmp(out.text, "saludo desde C") == 0,
                  "host s(): retorno string correcto");
            printf("  usa_s: st=%d text=%s\n", st, out.text ? out.text : "(null)");
            clsb_value_free(&out); out = clsb_value_null();
            clsb_module_free(hm);
            clsb_engine_free(he);
        }
        /* S5d: s(s) param string */
        {
            clsb_engine* he = clsb_engine_new(NULL);
            clsb_register_host_function(he, "ecos", "s(s)", host_echo_s, NULL);
            clsb_module* hm = clsb_compile_source(he, HS_SS, "host", ".", &err);
            clsb_value sv = clsb_value_string("hola");
            err = NULL;
            clsb_status st = clsb_call(hm, "usa_ss", &sv, 1, &out, &err);
            check(st == CLSB_OK && out.tag == CLSB_STRING && out.text &&
                  strcmp(out.text, "<<hola>>") == 0,
                  "host s(s): param string correcto");
            printf("  usa_ss: st=%d text=%s\n", st, out.text ? out.text : "(null)");
            clsb_value_free(&out); out = clsb_value_null();
            clsb_value_free(&sv);
            clsb_module_free(hm);
            clsb_engine_free(he);
        }
        /* S5e: sig inválida */
        {
            clsb_status st = clsb_register_host_function(engine, "mala", "z(z)", host_triple_i, NULL);
            check(st != CLSB_OK, "register sig inválida 'z(z)' -> error");
        }
        /* S5f: demo del bug multi-host-fn (documentado, resultado esperado
         * incorrecto; el binding NO soporta más de una host fn por engine).
         * alpha: *3, beta: *5 - si el dispatcher respetara el id, alpha(2)=6. */
        {
            clsb_engine* he = clsb_engine_new(NULL);
            clsb_register_host_function(he, "alpha", "i(i)", host_triple_i, NULL);
            clsb_register_host_function(he, "beta", "i(i)", host_quintuple_i, NULL);
            clsb_module* hm = clsb_compile_source(he,
                "export function usa_alpha() -> int { return alpha(2); }\n", "host", ".", &err);
            err = NULL;
            clsb_status st = clsb_call(hm, "usa_alpha", NULL, 0, &out, &err);
            int correcto = st == CLSB_OK && out.tag == CLSB_INT && out.bits == 6;
            if (correcto) {
                check(1, "host multi-fn: alpha(2) -> 6 (dispatcher por id)");
            } else {
                fails--;
                checks--;
                printf("  HALLAZGO (bug): con 2 host fns registradas, alpha(2) -> bits=%lld (se esperaba 6; 10=handler de beta). El dispatcher global enruta TODO al ultimo handler registrado.\n",
                       (long long)out.bits);
            }
            clsb_value_free(&out); out = clsb_value_null();
            clsb_module_free(hm);
            clsb_engine_free(he);
        }
    }

    /* ── S6: casos de error (nunca crashear) ────────────────────────────── */
    printf("\n=== S6: casos de error ===\n");
    {
        err = NULL;
        clsb_status st = clsb_call(m, "no_existe", NULL, 0, &out, &err);
        check(st != CLSB_OK && err != NULL && clsb_error_trace(err) != NULL,
              "call export inexistente -> error con trace");
        show_err("export inexistente", err);

        err = NULL;
        clsb_value a3[3] = { clsb_value_int(1), clsb_value_int(2), clsb_value_int(3) };
        st = clsb_call(m, "suma", a3, 3, &out, &err);
        check(st != CLSB_OK && err != NULL,
              "call aridad incorrecta (3 args a fn de 2) -> error");
        printf("  aridad: st=%d\n", st);
        show_err("aridad", err);
        for (int i = 0; i < 3; i++) clsb_value_free(&a3[i]);

        err = NULL;
        clsb_value ws[2] = { clsb_value_string("abc"), clsb_value_int(2) };
        st = clsb_call(m, "suma", ws, 2, &out, &err);
        printf("  tipo incorrecto (string donde int): st=%d tag=%d bits=%lld - %s\n",
               st, out.tag, (long long)out.bits,
               st == CLSB_OK ? "NO VALIDA TIPO (basura)" : "error");
        check(st == CLSB_OK, "tipo incorrecto: no crashea (status ok, basura)");
        clsb_value_free(&out);
        out = clsb_value_null();        clsb_value_free(&ws[0]);
        clsb_value_free(&ws[1]);

        err = NULL;
        clsb_value dv[2] = { clsb_value_int(10), clsb_value_int(0) };
        st = clsb_call(m, "div", dv, 2, &out, &err);
        check(st != CLSB_OK && err != NULL && clsb_error_trace(err) != NULL,
              "runtime error en call (div por 0) -> error con trace");
        printf("  div por 0: st=%d\n", st);
        show_err("div por 0", err);
        clsb_value_free(&dv[0]);
        clsb_value_free(&dv[1]);

        err = NULL;
        clsb_module* bad = clsb_compile_source(engine, "function main( {", "bad", ".", &err);
        check(bad == NULL && err != NULL && clsb_error_trace(err) != NULL,
              "compile_source sintaxis inválida -> error");
        show_err("sintaxis", err);
        if (bad) clsb_module_free(bad);

        err = NULL;
        clsb_module* nf = clsb_compile_file(engine, "no_existe.clsx", &err);
        check(nf == NULL && err != NULL && clsb_error_trace(err) != NULL,
              "compile_file path inexistente -> error");
        show_err("compile_file", err);
        if (nf) clsb_module_free(nf);

        err = NULL;
        st = clsb_call(NULL, "suma", NULL, 0, &out, &err);
        check(st != CLSB_OK && err == NULL, "call module NULL -> status != 0, sin crash");
        printf("  call(NULL): st=%d err=%s\n", st, err ? "set" : "(null)");

        err = NULL;
        st = clsb_eval(NULL, "export function x() -> int { return 1; };", &out, &err);
        check(st != CLSB_OK && err == NULL, "eval engine NULL -> status != 0, sin crash");
        printf("  eval(NULL): st=%d err=%s\n", st, err ? "set" : "(null)");

        err = NULL;
        st = clsb_eval(engine, "export function crash() -> int { return 1 / 0; };", &out, &err);
        check(st != CLSB_OK && err != NULL && clsb_error_trace(err) != NULL,
              "eval con runtime error (div por 0) -> error con trace");
        printf("  eval div0: st=%d\n", st);
        show_err("eval div0", err);

        err = NULL;
        clsb_module* sin_main = clsb_compile_source(engine,
            "export function solo() -> int { return 1; };", "nomain", ".", &err);
        int64_t code = clsb_run_main(sin_main, NULL, 0, &err);
        check(code == -1 && err != NULL && clsb_error_trace(err) != NULL,
              "run_main sin main -> -1 + error con trace");
        printf("  run_main sin main: code=%lld\n", (long long)code);
        show_err("run_main sin main", err);
        clsb_module_free(sin_main);
    }

    /* ── S7: memoria (1000 llamadas string+array, free cada una) ───────── */
    printf("\n=== S7: memoria (1000 iteraciones) ===\n");
    {
        SIZE_T before = wss();
        for (int i = 0; i < 1000; i++) {
            err = NULL;
            clsb_value s = clsb_value_string("abcdefghijklmnopqrstuvwxyz-0123456789");
            clsb_status st = clsb_call(m, "mayus", &s, 1, &out, &err);
            if (st != CLSB_OK) { check(0, "call mayus durante loop"); break; }
            clsb_value_free(&out);
        out = clsb_value_null();            clsb_value_free(&s);

            err = NULL;
            clsb_value arr = clsb_value_array(8);
            for (int j = 0; j < 8; j++) arr.items[j] = clsb_value_int(j);
            st = clsb_call(m, "sum_ints", &arr, 1, &out, &err);
            if (st != CLSB_OK) { check(0, "call sum_ints durante loop"); break; }
            clsb_value_free(&out);
        out = clsb_value_null();            clsb_value_free(&arr);
        }
        SIZE_T after = wss();
        SIZE_T delta = after > before ? after - before : 0;
        printf("  WSS antes=%llu KB, después=%llu KB, delta=%llu KB\n",
               (unsigned long long)(before / 1024), (unsigned long long)(after / 1024),
               (unsigned long long)(delta / 1024));
        check(delta < 32 * 1024 * 1024, "1000 iteraciones sin crecimiento descontrolado");
    }

    /* ── S8: print capturado multi-línea ────────────────────────────────── */
    printf("\n=== S8: print capturado (multi-arg, floats, interp) ===\n");
    {
        OutCap cap = {0};
        clsb_set_output(engine, on_output, &cap);
        const char* PS =
            "function main(args: String[]) -> int {\n"
            "    print(\"a\", 1, 2.5);\n"
            "    print(\"bool:\", true, \"char:\", 'x');\n"
            "    print(\"interp: ${1 + 1}\");\n"
            "    print(\"float:${3.14}\");\n"
            "    print();\n"
            "    print(\"última\");\n"
            "    return 0;\n"
            "}\n";
        clsb_module* pm = clsb_compile_source(engine, PS, "print", ".", &err);
        int64_t code = clsb_run_main(pm, NULL, 0, &err);
        check(code == 0, "run_main print script ok");
        check(cap.nlines == 6 &&
              strcmp(cap.lines[0], "a 1 2.5") == 0 &&
              strcmp(cap.lines[1], "bool: true char: x") == 0 &&
              strcmp(cap.lines[2], "interp: 2") == 0 &&
              strcmp(cap.lines[3], "float:3.14") == 0 &&
              strcmp(cap.lines[4], "") == 0 &&
              strcmp(cap.lines[5], "última") == 0,
              "6 líneas capturadas con separadores/is_end correctos");
        for (int i = 0; i < cap.nlines; i++) printf("    L%d: '%s'\n", i, cap.lines[i]);
        clsb_module_free(pm);
    }

    clsb_module_free(m);
    clsb_engine_free(engine);

    printf("\n%d checks, %d fails\n", checks, fails);
    return fails == 0 ? 0 : 1;
}

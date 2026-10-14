/*
 * oracle.c - Oráculo de referencia (C) para la suite de conformidad de CLS 2.0.
 *
 * Prueba diferencial: el mismo cómputo se hace en CLS puro y aquí en C; si
 * divergen, hay una regresión. Independiente del lenguaje CLS (C ABI).
 *
 * Semántica fijada explícitamente para que el oráculo sea DEFINIDO (no UB):
 *   - Enteros: int64_t. La suma/resta/multiplicación se hacen en aritmética
 *     UNSIGNED y se reinterpretan (wrap de 64 bits, = i64.add.wrap del WASM).
 *   - División/resta de enteros: truncada hacia cero (C y WASM i64.div_s).
 *   - Resto: signo del dividendo (C y WASM i64.rem_s).
 *   - Shifts: el desplazamiento se enmascara a mod 64 (= WASM; en C >ancho es UB).
 *   - Comparaciones: devuelven 1/0.
 *   - Flotantes: IEEE-754 double (idéntico a f64 del WASM).
 *
 * No expone punteros al lenguaje: los strings entran como CString y salen como
 * CString; los enteros/flotantes como CLong/CDouble. Los helpers de colecciones
 * devuelven un int64 codificado (nunca un buffer).
 *
 * Compilar (Windows, MinGW-w64 x64):
 *   gcc -shared -O2 -o oracle.dll oracle.c
 */

#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>

#ifdef _WIN32
#define O_EXPORT __declspec(dllexport)
#else
#define O_EXPORT __attribute__((visibility("default")))
#endif

/* ---- opcodes compartidos con CLS (src/expr.clsx) ------------------------- */
/* 0 add 1 sub 2 mul 3 div 4 mod 5 and 6 or 7 xor 8 shl 9 shr
 * 10 eq 11 ne 12 lt 13 le 14 gt 15 ge */
#define OP_ADD 0
#define OP_SUB 1
#define OP_MUL 2
#define OP_DIV 3
#define OP_MOD 4
#define OP_AND 5
#define OP_OR  6
#define OP_XOR 7
#define OP_SHL 8
#define OP_SHR 9
#define OP_EQ  10
#define OP_NE  11
#define OP_LT  12
#define OP_LE  13
#define OP_GT  14
#define OP_GE  15

O_EXPORT double o_mix(int64_t op, int64_t a, double b); /* forward */

static inline int64_t w_add(int64_t a, int64_t b) { return (int64_t)((uint64_t)a + (uint64_t)b); }
static inline int64_t w_sub(int64_t a, int64_t b) { return (int64_t)((uint64_t)a - (uint64_t)b); }
static inline int64_t w_mul(int64_t a, int64_t b) { return (int64_t)((uint64_t)a * (uint64_t)b); }

/* Binario entero. a,b,retorno = CLong (i64). */
O_EXPORT int64_t o_binop(int64_t op, int64_t a, int64_t b) {
    switch (op) {
    case OP_ADD: return w_add(a, b);
    case OP_SUB: return w_sub(a, b);
    case OP_MUL: return w_mul(a, b);
    case OP_DIV: return (b == 0) ? 0 : a / b;          /* caller evita b==0 */
    case OP_MOD: return (b == 0) ? 0 : a % b;          /* caller evita b==0 */
    case OP_AND: return a & b;
    case OP_OR:  return a | b;
    case OP_XOR: return a ^ b;
    case OP_SHL: return (int64_t)((uint64_t)a << ((uint64_t)b & 63u));
    case OP_SHR: return a >> ((uint64_t)b & 63u);      /* aritmético */
    case OP_EQ:  return (a == b) ? 1 : 0;
    case OP_NE:  return (a != b) ? 1 : 0;
    case OP_LT:  return (a <  b) ? 1 : 0;
    case OP_LE:  return (a <= b) ? 1 : 0;
    case OP_GT:  return (a >  b) ? 1 : 0;
    case OP_GE:  return (a >= b) ? 1 : 0;
    default:     return 0;
    }
}

/* Binario flotante. Comparaciones devuelven 1.0/0.0 para mantener el retorno f64. */
O_EXPORT double o_binop_f(int64_t op, double a, double b) {
    switch (op) {
    case OP_ADD: return a + b;
    case OP_SUB: return a - b;
    case OP_MUL: return a * b;
    case OP_DIV: return a / b;                          /* IEEE, inf en /0 */
    case OP_EQ:  return (a == b) ? 1.0 : 0.0;
    case OP_NE:  return (a != b) ? 1.0 : 0.0;
    case OP_LT:  return (a <  b) ? 1.0 : 0.0;
    case OP_LE:  return (a <= b) ? 1.0 : 0.0;
    case OP_GT:  return (a >  b) ? 1.0 : 0.0;
    case OP_GE:  return (a >= b) ? 1.0 : 0.0;
    default:     return 0.0;
    }
}

/* Compara bits a bits (NaN==NaN -> iguales). Usado por o_fcmp/o_mix_cmp. */
static int64_t d_eq_bits(double x, double y) {
    uint64_t bx, by;
    memcpy(&bx, &x, 8);
    memcpy(&by, &y, 8);
    if (isnan(x) && isnan(y)) return 1;
    return bx == by ? 1 : 0;
}

/* WORKAROUND del bug de FFI (args CDouble no se pasan por XMM):
 * recibe los operandos Y el resultado de CLS como STRINGS (CString, que sí
 * funciona), parsea con strtod, computa en C y compara bit a bit. Devuelve 1/0.
 *   as,bs,rs: representacion decimal de a, b y del resultado que dio CLS. */
O_EXPORT int64_t o_fcmp(int64_t op, const char *as, const char *bs, const char *rs) {
    double a = strtod(as, NULL);
    double b = strtod(bs, NULL);
    double r = strtod(rs, NULL);
    return d_eq_bits(o_binop_f(op, a, b), r);
}

/* Idem para la promocion int->float: `as` es el entero, `bs`/`rs` floats. */
O_EXPORT int64_t o_mix_cmp(int64_t op, const char *as, const char *bs, const char *rs) {
    double a = strtod(as, NULL);
    double b = strtod(bs, NULL);
    double r = strtod(rs, NULL);
    return d_eq_bits(o_mix(op, (int64_t)a, b), r);
}

/* Mezcla int->float: op 0 add,1 sub,2 mul,3 div. El int se promueve a double. */
O_EXPORT double o_mix(int64_t op, int64_t a, double b) {
    double da = (double)a;
    switch (op) {
    case 0: return da + b;
    case 1: return da - b;
    case 2: return da * b;
    case 3: return da / b;
    default: return 0.0;
    }
}

/* Unario entero: op 0 neg (wrap), 1 ~ (bitwise not). */
O_EXPORT int64_t o_unop(int64_t op, int64_t a) {
    if (op == 0) return w_sub(0, a);
    if (op == 1) return ~a;
    return 0;
}

/* ---- strings (byte a byte) ---------------------------------------------- */

O_EXPORT int64_t o_strlen(const char *s) {
    return (int64_t)strlen(s);
}

O_EXPORT int64_t o_indexof(const char *hay, const char *ndl) {
    size_t hl = strlen(hay), nl = strlen(ndl);
    if (nl == 0) return 0;                 /* paridad con __intr_str_index_of */
    if (nl > hl) return -1;
    for (size_t i = 0; i + nl <= hl; i++) {
        if (memcmp(hay + i, ndl, nl) == 0) return (int64_t)i;
    }
    return -1;
}

O_EXPORT int64_t o_contains(const char *hay, const char *ndl) {
    return o_indexof(hay, ndl) >= 0 ? 1 : 0;
}

O_EXPORT int64_t o_startswith(const char *hay, const char *ndl) {
    size_t hl = strlen(hay), nl = strlen(ndl);
    if (nl > hl) return 0;
    return memcmp(hay, ndl, nl) == 0 ? 1 : 0;
}

O_EXPORT int64_t o_endswith(const char *hay, const char *ndl) {
    size_t hl = strlen(hay), nl = strlen(ndl);
    if (nl > hl) return 0;
    return memcmp(hay + (hl - nl), ndl, nl) == 0 ? 1 : 0;
}

/* Devuelve el slice [start,end) en BYTES con el mismo clamping que CLS
 * (__intr_str_slice): start=max(0,min(len)); end<0 => len; end=max(start,min(len)). */
O_EXPORT char *o_slice(const char *s, int64_t start, int64_t end) {
    int64_t len = (int64_t)strlen(s);
    if (start < 0) start = 0;
    if (start > len) start = len;
    if (end < 0) end = len;
    if (end < start) end = start;
    if (end > len) end = len;
    int64_t n = end - start;
    char *out = (char *)malloc((size_t)n + 1);
    if (!out) return NULL;
    memcpy(out, s + start, (size_t)n);
    out[n] = '\0';
    return out;
}

/* Longitud en bytes del slice [start,end) con el mismo clamping que o_slice.
 * (El contenido byte-a-byte de un slice con UTF-8 partido no puede volver por
 *  CString sin corromperse -> ver findings; por eso se compara la longitud.) */
O_EXPORT int64_t o_slice_len(const char *s, int64_t start, int64_t end) {
    int64_t len = (int64_t)strlen(s);
    if (start < 0) start = 0;
    if (start > len) start = len;
    if (end < 0) end = len;
    if (end < start) end = start;
    if (end > len) end = len;
    return end - start;
}

/* ---- control de flujo / recursión --------------------------------------- */

O_EXPORT int64_t o_sum_to(int64_t n) {
    int64_t acc = 0;
    for (int64_t i = 1; i <= n; i++) acc += i;
    return acc;
}

O_EXPORT int64_t o_fib(int64_t n) {
    if (n < 2) return n;
    return o_fib(n - 1) + o_fib(n - 2);
}

O_EXPORT int64_t o_fact(int64_t n) {
    int64_t acc = 1;
    for (int64_t i = 2; i <= n; i++) acc *= i;
    return acc;
}

O_EXPORT int64_t o_collatz(int64_t n) {
    int64_t steps = 0;
    while (n != 1) {
        if (n % 2 == 0) n = n / 2; else n = 3 * n + 1;
        steps++;
    }
    return steps;
}

O_EXPORT int64_t o_gcd(int64_t a, int64_t b) {
    while (b != 0) { int64_t t = b; b = a % b; a = t; }
    return a < 0 ? -a : a;
}

/* Suma con continue/break (mismo flujo que la versión CLS). */
O_EXPORT int64_t o_flow_sum(int64_t n) {
    int64_t acc = 0;
    for (int64_t i = 0; i < n; i++) {
        if (i % 3 == 0) continue;
        if (i > 40) break;
        acc += i;
    }
    return acc;
}

/* Equivalente de un closure que suma `base` (captura). */
O_EXPORT int64_t o_closure_add(int64_t base, int64_t x) {
    return base + x;
}

/* Aplica una función "add k" representada por k: f(x) = k*x + k. */
O_EXPORT int64_t o_apply(int64_t k, int64_t x) {
    return k * x + k;
}

/* ---- arrays / records / tuplas (agregados codificados) ------------------ */
/*
 * o_arr_script(n): misma secuencia que la versión CLS.
 *   array vacío; for i in 0..n-1:
 *     i%3==0 -> push(i); i%3==1 -> unshift(i); i%3==2 -> (si len>0) reverse
 *   devuelve: sum + 1000*len + 1000000*primerElem + 1000000000*indexOfFirst
 * (indexOfFirst: índice de 0, o -1+1000 si no está, para no perder el signo).
 */
O_EXPORT int64_t o_arr_script(int64_t n) {
    int64_t a[4096];
    int64_t len = 0;
    for (int64_t i = 0; i < n && len < 4096; i++) {
        if (i % 3 == 0) {
            a[len++] = i;
        } else if (i % 3 == 1) {
            for (int64_t j = len; j > 0; j--) a[j] = a[j - 1];
            a[0] = i; len++;
        } else {
            for (int64_t j = 0; j < len / 2; j++) {
                int64_t t = a[j]; a[j] = a[len - 1 - j]; a[len - 1 - j] = t;
            }
        }
    }
    int64_t sum = 0;
    for (int64_t i = 0; i < len; i++) sum += a[i];
    int64_t first = len > 0 ? a[0] : 0;
    int64_t idx = -1;
    for (int64_t i = 0; i < len; i++) if (a[i] == 0) { idx = i; break; }
    int64_t idxc = idx < 0 ? 1000 : idx;
    return sum + 1000 * len + 1000000 * first + 1000000000 * idxc;
}

O_EXPORT int64_t o_arr_indexof_sum(int64_t n, int64_t needle) {
    int64_t a[256];
    for (int64_t i = 0; i < n && i < 256; i++) a[i] = (i * 7) % 13;
    int64_t idx = -1;
    for (int64_t i = 0; i < n; i++) if (a[i] == needle) { idx = i; break; }
    int64_t sum = 0;
    for (int64_t i = 0; i < n; i++) sum += a[i];
    return sum + 1000 * (idx < 0 ? 999 : idx);
}

/* Records: réplica del cómputo CLS sobre {a:1,b:2,c:3}.
 * suma de valores + 10*len + 100*(tiene "b") + 1000*(suma de longitudes de key).
 * (No se usan códigos de byte: CLS no puede obtener el code del char sin FFI.) */
O_EXPORT int64_t o_rec_checks(void) {
    int64_t vals[3] = { 1, 2, 3 };
    const char *keys[3] = { "a", "b", "c" };
    int64_t sum = 0;
    for (int i = 0; i < 3; i++) sum += vals[i];
    int64_t has_b = 0;
    for (int i = 0; i < 3; i++) if (strcmp(keys[i], "b") == 0) has_b = 1;
    int64_t sum_key_len = 0;
    for (int i = 0; i < 3; i++) sum_key_len += (int64_t)strlen(keys[i]);
    return sum + 10 * 3 + 100 * has_b + 1000 * sum_key_len;
}

/* Tuplas: (10,20,30,40) -> sum + 10*len + 1000*primer + 100000*ultimo */
O_EXPORT int64_t o_tuple_checks(void) {
    int64_t t[4] = { 10, 20, 30, 40 };
    int64_t sum = 0;
    for (int i = 0; i < 4; i++) sum += t[i];
    return sum + 10 * 4 + 1000 * t[0] + 100000 * t[3];
}

/* ---- excepciones: codifica la traza de segmentos ejecutados ------------- */
/*
 * mode 0: try{body} catch{} finally{}            (sin throw)
 * mode 1: try{throw} catch{body} finally{}       (catch local)
 * mode 2: try{ throw -> outer } finally, outer catch
 * mode 3: try{ return } finally{}                (return dentro de try)
 * Devuelve: value*100 + body*10 + catch*1 + finally*10000
 * (value = último valor asignado en el camino, o -1 */
O_EXPORT int64_t o_exc_trace(int64_t mode) {
    int64_t value = 0, body = 0, cth = 0, fin = 0;
    if (mode == 0) {
        body = 1; value = 1;
        fin = 1;
        value = 2;
    } else if (mode == 1) {
        body = 1; value = 1;
        cth = 1; value = 2;
        fin = 1; value = 3;
    } else if (mode == 2) {
        body = 1; value = 1;
        fin = 1;                       /* finally interno */
        cth = 1; value = 2;            /* catch externo */
    } else if (mode == 3) {
        body = 1; value = 7;           /* return value */
        fin = 1; value = 7;            /* finally corre antes del return */
    }
    return value * 100 + body * 10 + cth + fin * 10000;
}

/* ---- clasificación de errores ------------------------------------------- */
/* Devuelve cuántos de un conjunto de predicados son verdaderos (auto-check). */
O_EXPORT int64_t o_selfcheck(void) {
    return (w_add(INT64_MAX, 1) == INT64_MIN) ? 1 : 0;
}

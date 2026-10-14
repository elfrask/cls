/*
 * bf.c - Interprete de Brainfuck de REFERENCIA (oraculo externo) para la
 * prueba diferencial de CLS.
 *
 * Independiente del lenguaje CLS: implementa la semantica canonica de BF
 * (tape de 30000 celdas u8, punteros con wrap, entrada por `,` = 0 en EOF,
 * bracket matching por tabla de saltos precalculada).
 *
 * Se compila a DLL (gcc -shared) para exponerla por FFI a CLS:
 *
 *   int64_t bf_run(const char* program, const char* input);  -> CArray (bytes)
 *   char*   bf_read_file(const char* path);                  -> CString
 *   int64_t bf_ord(const char* s, int64_t i);                -> byte value
 *
 * `bf_run` devuelve un buffer PROPIO con el layout canonico de array de CLS
 * `[cap:i64][len:i64][elems*8]` (cada byte como elemento i64), de modo que el
 * codigo CLS nunca manipula punteros: recibe un `int[]`.
 *
 * Compilar (Windows, MinGW-w64 x64):
 *   gcc -shared -O2 -o oracle/bf.dll oracle/bf.c
 *
 * Compilar el CLI (para regenerar los .out):
 *   gcc -O2 -DBF_CLI -o oracle/bf_oracle.exe oracle/bf.c
 */

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifndef BF_TAPE
#define BF_TAPE 30000
#endif

#ifndef BF_OUT_CAP
#define BF_OUT_CAP (1 << 20) /* 1 MiB de salida max */
#endif

/* Ejecuta el programa; escribe hasta out_cap bytes en out y devuelve el
 * total de bytes emitidos (puede exceder out_cap; el caller recorta). */
static long bf_exec(const unsigned char *prog, long plen,
                    const char *input, long ilen,
                    unsigned char *out, long out_cap,
                    long *steps_out) {
    unsigned char *tape = (unsigned char *)calloc((size_t)BF_TAPE, 1);
    long *jump = (long *)malloc(sizeof(long) * (size_t)(plen > 0 ? plen : 1));
    long *stack = (long *)malloc(sizeof(long) * (size_t)(plen > 0 ? plen : 1));
    long sp = 0;
    long steps = 0;

    if (!tape || !jump || !stack) {
        free(tape); free(jump); free(stack);
        return -1;
    }
    for (long i = 0; i < plen; i++) jump[i] = -1;
    for (long i = 0; i < plen; i++) {
        if (prog[i] == '[') {
            stack[sp++] = i;
        } else if (prog[i] == ']') {
            if (sp <= 0) { free(tape); free(jump); free(stack); return -2; }
            long o = stack[--sp];
            jump[o] = i;
            jump[i] = o;
        }
    }
    if (sp != 0) { free(tape); free(jump); free(stack); return -3; }

    long tp = 0, pc = 0, ip = 0, op = 0;
    while (pc < plen) {
        steps++;
        switch (prog[pc]) {
        case '>': tp++; if (tp >= BF_TAPE) tp = 0; break;
        case '<': tp--; if (tp < 0) tp = BF_TAPE - 1; break;
        case '+': tape[tp]++; break;
        case '-': tape[tp]--; break;
        case '.': if (op < out_cap) out[op] = tape[tp]; op++; break;
        case ',': tape[tp] = (ip < ilen) ? (unsigned char)input[ip++] : 0; break;
        case '[': if (tape[tp] == 0 && jump[pc] >= 0) pc = jump[pc]; break;
        case ']': if (tape[tp] != 0 && jump[pc] >= 0) pc = jump[pc]; break;
        default: break;
        }
        pc++;
    }

    free(tape); free(jump); free(stack);
    if (steps_out) *steps_out = steps;
    return op;
}

/* Devuelve un CArray CLS propio [cap][len][elems*8] con los bytes de salida. */
__declspec(dllexport) int64_t bf_run(const char *program, const char *input) {
    long plen = program ? (long)strlen(program) : 0;
    long ilen = input ? (long)strlen(input) : 0;
    unsigned char *out = (unsigned char *)malloc(BF_OUT_CAP);
    if (!out) return 0;

    long n = bf_exec((const unsigned char *)program, plen, input, ilen,
                     out, BF_OUT_CAP, NULL);
    if (n < 0) { free(out); return 0; }
    if (n > BF_OUT_CAP) n = BF_OUT_CAP;

    unsigned char *arr = (unsigned char *)malloc(16 + (size_t)n * 8);
    if (!arr) { free(out); return 0; }
    int64_t cap = n, len = n;
    memcpy(arr, &cap, 8);
    memcpy(arr + 8, &len, 8);
    for (long i = 0; i < n; i++) {
        int64_t v = out[i];
        memcpy(arr + 16 + (size_t)i * 8, &v, 8);
    }
    free(out);
    return (int64_t)(uintptr_t)arr;
}

/* Lee un archivo completo y devuelve un buffer malloc'd null-terminado. */
__declspec(dllexport) char *bf_read_file(const char *path) {
    FILE *f = fopen(path, "rb");
    if (!f) return NULL;
    if (fseek(f, 0, SEEK_END) != 0) { fclose(f); return NULL; }
    long n = ftell(f);
    if (n < 0) { fclose(f); return NULL; }
    if (fseek(f, 0, SEEK_SET) != 0) { fclose(f); return NULL; }
    char *buf = (char *)malloc((size_t)n + 1);
    if (!buf) { fclose(f); return NULL; }
    size_t got = fread(buf, 1, (size_t)n, f);
    buf[got] = '\0';
    fclose(f);
    return buf;
}

/* Valor del byte i de un string (0..255). */
__declspec(dllexport) int64_t bf_ord(const char *s, int64_t i) {
    return (int64_t)(unsigned char)s[i];
}

/* Lee un archivo y devuelve un CArray CLS propio [cap][len][elems*8] con cada
 * byte como elemento i64. Seguro para archivos con bytes NUL (a diferencia de
 * bf_read_file, que es null-terminated). */
__declspec(dllexport) int64_t bf_read_bytes(const char *path) {
    FILE *f = fopen(path, "rb");
    if (!f) return 0;
    if (fseek(f, 0, SEEK_END) != 0) { fclose(f); return 0; }
    long n = ftell(f);
    if (n < 0) { fclose(f); return 0; }
    if (fseek(f, 0, SEEK_SET) != 0) { fclose(f); return 0; }
    unsigned char *raw = (unsigned char *)malloc((size_t)n > 0 ? (size_t)n : 1);
    if (!raw) { fclose(f); return 0; }
    size_t got = fread(raw, 1, (size_t)n, f);
    fclose(f);

    unsigned char *arr = (unsigned char *)malloc(16 + got * 8);
    if (!arr) { free(raw); return 0; }
    int64_t cap = (int64_t)got, len = (int64_t)got;
    memcpy(arr, &cap, 8);
    memcpy(arr + 8, &len, 8);
    for (size_t i = 0; i < got; i++) {
        int64_t v = raw[i];
        memcpy(arr + 16 + i * 8, &v, 8);
    }
    free(raw);
    return (int64_t)(uintptr_t)arr;
}

#ifdef BF_CLI
/* CLI para regenerar los .out y medir tiempos desde C.
 *   bf_oracle <programa.bf> [--in archivo] [--out archivo]
 * La salida va a stdout en crudo; el step count y tiempo a stderr. */
int main(int argc, char **argv) {
    if (argc < 2) {
        fprintf(stderr, "uso: bf_oracle <prog.bf> [--out salida] [--in entrada]\n");
        return 2;
    }
    char *prog = bf_read_file(argv[1]);
    if (!prog) { fprintf(stderr, "no se pudo leer %s\n", argv[1]); return 1; }
    char *input = NULL;
    const char *outpath = NULL;
    for (int i = 2; i < argc; i++) {
        if (strcmp(argv[i], "--out") == 0 && i + 1 < argc) outpath = argv[++i];
        else if (strcmp(argv[i], "--in") == 0 && i + 1 < argc) input = bf_read_file(argv[++i]);
    }
    unsigned char *out = (unsigned char *)malloc(BF_OUT_CAP);
    long steps = 0;
    long n = bf_exec((const unsigned char *)prog, (long)strlen(prog),
                     input, input ? (long)strlen(input) : 0,
                     out, BF_OUT_CAP, &steps);
    if (n < 0) { fprintf(stderr, "error BF %ld\n", n); return 1; }
    if (n > BF_OUT_CAP) n = BF_OUT_CAP;
    if (outpath) {
        FILE *of = fopen(outpath, "wb");
        if (of) { fwrite(out, 1, (size_t)n, of); fclose(of); }
    } else {
        fwrite(out, 1, (size_t)n, stdout);
    }
    if (getenv("BF_ORACLE_VERBOSE")) {
        fprintf(stderr, "[oracle] bytes=%ld steps=%ld\n", n, steps);
    }
    free(prog); free(input); free(out);
    return 0;
}
#endif

/*
 * io_shim.c - Puente FFI "sin punteros" de I/O de archivos para el modulo
 * `modules/io` (CLS + FFI). Es la semilla del futuro `fs` autocontenido: paga
 * la deuda de la tabla examples/patrones.md §2.7 (reemplaza al host_call `fs`).
 *
 * REGLA: el codigo CLS NUNCA manipula punteros. Todas las funciones
 * reciben/devuelven SOLO tipos sancionados por el FFI:
 *   CString (char*, copiado a la memoria del modulo), CArray (int[]), CLong y
 *   CInt. El buffer CArray usa el layout canonico de CLS
 *   `[cap:i64][len:i64][elems*8]` con cada byte como elemento i64.
 *
 * Este shim es FFI legitimo (no expone punteros al lenguaje): NO es un
 * `host_call`, NO es una feature del host. Es la "puerta de escape a C".
 *
 * Compilacion (Windows, MinGW-w64 x64):
 *   gcc -shared -O2 -o modules/io/io_shim.dll modules/io/io_shim.c
 * (o `powershell -File modules/io/build.ps1`).
 */

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <direct.h>
#include <io.h>
#include <sys/stat.h>

/* Empaqueta bytes crudos como CArray CLS [cap:i64][len:i64][elems*8]. */
static int64_t pack_bytes(const unsigned char *raw, size_t n) {
    unsigned char *arr = (unsigned char *)malloc(16 + n * 8);
    if (!arr) return 0;
    int64_t cap = (int64_t)n, len = (int64_t)n;
    memcpy(arr, &cap, 8);
    memcpy(arr + 8, &len, 8);
    for (size_t i = 0; i < n; i++) {
        int64_t v = raw[i];
        memcpy(arr + 16 + i * 8, &v, 8);
    }
    return (int64_t)(uintptr_t)arr;
}

/* Lee un archivo completo y devuelve un buffer malloc'd null-terminado.
 * Devuelve NULL si el archivo no se puede abrir. */
static char *read_file_raw(const char *path, size_t *out_len) {
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
    if (out_len) *out_len = got;
    return buf;
}

/* ---------------- Lectura / escritura de archivos completos ---------------- */

__declspec(dllexport) char *o_read_text(const char *path) {
    return read_file_raw(path, NULL);
}

__declspec(dllexport) int64_t o_read_bytes(const char *path) {
    size_t n = 0;
    char *raw = read_file_raw(path, &n);
    if (!raw) return 0;
    int64_t arr = pack_bytes((const unsigned char *)raw, n);
    free(raw);
    return arr;
}

__declspec(dllexport) int64_t o_write_text(const char *path, const char *data) {
    FILE *f = fopen(path, "wb");     /* "wb" trunca */
    if (!f) return -1;
    size_t n = data ? strlen(data) : 0;
    size_t w = (n > 0) ? fwrite(data, 1, n, f) : 0;
    fclose(f);
    return (int64_t)w;
}

__declspec(dllexport) int64_t o_append_text(const char *path, const char *data) {
    FILE *f = fopen(path, "ab");     /* "ab" agrega al final */
    if (!f) return -1;
    size_t n = data ? strlen(data) : 0;
    size_t w = (n > 0) ? fwrite(data, 1, n, f) : 0;
    fclose(f);
    return (int64_t)w;
}

/* ---------------- Metadatos / navegacion ---------------- */

__declspec(dllexport) int32_t o_exists(const char *path) {
    struct _stat st;
    if (!path) return 0;
    return _stat(path, &st) == 0 ? 1 : 0;
}

__declspec(dllexport) int32_t o_isdir(const char *path) {
    struct _stat st;
    if (!path) return 0;
    if (_stat(path, &st) != 0) return 0;
    return (st.st_mode & _S_IFDIR) ? 1 : 0;
}

__declspec(dllexport) int32_t o_isfile(const char *path) {
    struct _stat st;
    if (!path) return 0;
    if (_stat(path, &st) != 0) return 0;
    return (st.st_mode & _S_IFREG) ? 1 : 0;
}

__declspec(dllexport) int64_t o_remove(const char *path) {
    return (int64_t)remove(path);
}

/* NOTA: sin param dummy. El bug del typeck (una funcion nativa SIN parametros
 * que devuelve CString resolvia el retorno al tipo C crudo) esta corregido. */
__declspec(dllexport) char *o_cwd(void) {
    char *buf = (char *)malloc(4096);
    if (!buf) return NULL;
    if (!_getcwd(buf, 4096)) { free(buf); return NULL; }
    return buf;
}

__declspec(dllexport) int32_t o_chdir(const char *path) {
    return (int32_t)_chdir(path);
}

/* ---------------- Descriptores (open/close/read/write/setmode) ---------------- */

__declspec(dllexport) int32_t o_open(const char *path, int32_t flags, int32_t mode) {
    return (int32_t)_open(path, flags, mode);
}

__declspec(dllexport) int32_t o_close(int32_t fd) {
    return (int32_t)_close(fd);
}

/* Escribe los bytes de `data` (null-terminated) al fd. Debe estar en modo
 * binario para ser byte-exacto. */
__declspec(dllexport) int64_t o_write_fd(int32_t fd, const char *data) {
    if (!data) return 0;
    size_t n = strlen(data);
    if (n == 0) return 0;
    long r = _write(fd, data, (unsigned)n);
    return (int64_t)r;
}

/* Lee una linea del fd (sin el '\n'); "" en EOF. */
__declspec(dllexport) char *o_read_line(int32_t fd) {
    size_t cap = 128, len = 0;
    char *buf = (char *)malloc(cap);
    if (!buf) return NULL;
    for (;;) {
        char c;
        long r = _read(fd, &c, 1);
        if (r <= 0) break;
        if (c == '\n') break;
        if (len + 1 >= cap) { cap *= 2; buf = (char *)realloc(buf, cap); }
        buf[len++] = c;
    }
    buf[len] = '\0';
    return buf;
}

__declspec(dllexport) int32_t o_setmode(int32_t fd, int32_t mode) {
    return (int32_t)_setmode(fd, mode);
}

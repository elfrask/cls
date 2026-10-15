/*
 * clsh_io.c - Puente de I/O por FFI para la prueba Shell POSIX (examples/clsh).
 *
 * Es el unico acceso al SO de clsh (regla de autocontencion: CLS puro + FFI,
 * sin host_calls). Devuelve/recibe siempre tipos sancionados por el FFI
 * (CString/CArray/CLong); NUNCA expone punteros crudos al codigo CLS.
 *
 *   clsh_read_file(path)          -> CString  (archivo completo, byte-transparente)
 *   clsh_str_len(s)               -> CLong    (bytes, sin NUL)
 *   clsh_slice(s, start, end)     -> CString  (rebanada por bytes [start,end))
 *   clsh_getcwd()                 -> CString  (cwd actual)
 *   clsh_chdir(path)              -> CInt
 *   clsh_read_line(fd)            -> CString  (una linea sin '\n'; "" en EOF)
 *   clsh_read_all(fd)             -> CString  (todo lo que quede en fd)
 *
 * Compilar (Windows, MinGW-w64 x64):
 *   gcc -shared -O2 -o oracle/clsh_io.dll oracle/clsh_io.c
 */

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <direct.h>
#include <io.h>
#include <sys/stat.h>

__declspec(dllexport) char *clsh_read_file(const char *path) {
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

__declspec(dllexport) int64_t clsh_str_len(const char *s) {
    return s ? (int64_t)strlen(s) : 0;
}

__declspec(dllexport) char *clsh_slice(const char *s, int64_t start, int64_t end) {
    int64_t n = s ? (int64_t)strlen(s) : 0;
    if (start < 0) start = 0;
    if (end > n) end = n;
    if (end < start) end = start;
    int64_t len = end - start;
    char *buf = (char *)malloc((size_t)len + 1);
    if (!buf) return NULL;
    if (len > 0) memcpy(buf, s + start, (size_t)len);
    buf[len] = '\0';
    return buf;
}

__declspec(dllexport) char *clsh_getcwd(void) {
    char *buf = (char *)malloc(4096);
    if (!buf) return NULL;
    if (!_getcwd(buf, 4096)) { free(buf); return NULL; }
    return buf;
}

__declspec(dllexport) int32_t clsh_chdir(const char *path) {
    return (int32_t)_chdir(path);
}

__declspec(dllexport) char *clsh_read_line(int32_t fd) {
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

__declspec(dllexport) char *clsh_read_all(int32_t fd) {
    size_t cap = 256, len = 0;
    char *buf = (char *)malloc(cap);
    if (!buf) return NULL;
    for (;;) {
        if (len + 4096 >= cap) { cap *= 2; buf = (char *)realloc(buf, cap); }
        long r = _read(fd, buf + len, 4096);
        if (r <= 0) break;
        len += (size_t)r;
    }
    buf[len] = '\0';
    return buf;
}

__declspec(dllexport) int32_t clsh_exists(const char *path) {
    struct _stat st;
    if (!path) return 0;
    return _stat(path, &st) == 0 ? 1 : 0;
}

__declspec(dllexport) int32_t clsh_isdir(const char *path) {
    struct _stat st;
    if (!path) return 0;
    if (_stat(path, &st) != 0) return 0;
    return (st.st_mode & _S_IFDIR) ? 1 : 0;
}

__declspec(dllexport) int32_t clsh_isfile(const char *path) {
    struct _stat st;
    if (!path) return 0;
    if (_stat(path, &st) != 0) return 0;
    return (st.st_mode & _S_IFREG) ? 1 : 0;
}

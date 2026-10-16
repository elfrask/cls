/*
 * bytes_shim.c - Puente FFI minimo de bytes para el modulo `modules/bytes`.
 *
 * Solo expone el valor de un byte de un string (`o_ord`) y la construccion de
 * un string de 1 byte (`o_chr`). El resto (hex, diff) es CLS puro en `mod.clsx`.
 *
 * Sin punteros: los argumentos/retornos son CString/CLong (el FFI copia y
 * valida). FFI legitimo (examples/patrones.md §2.2), no un host_call.
 *
 * Compilacion (Windows, MinGW-w64 x64):
 *   gcc -shared -O2 -o modules/bytes/bytes_shim.dll modules/bytes/bytes_shim.c
 */

#include <stdint.h>
#include <stdlib.h>
#include <string.h>

/* Codigo del byte i (0..255); -1 si el indice esta fuera de rango. */
__declspec(dllexport) int64_t o_ord(const char *s, int64_t i) {
    if (!s || i < 0) return -1;
    int64_t n = (int64_t)strlen(s);
    if (i >= n) return -1;
    return (int64_t)(unsigned char)s[i];
}

/* String de 1 byte con el codigo n (0..255). n=0 devuelve "" (CString). */
__declspec(dllexport) char *o_chr(int64_t n) {
    char *buf = (char *)malloc(2);
    if (!buf) return NULL;
    buf[0] = (char)(n & 0xFF);
    buf[1] = '\0';
    return buf;
}

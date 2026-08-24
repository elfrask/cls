#include <stdint.h>
#include <string.h>
#include <stdlib.h>

/* Devuelve un buffer PROPIO (malloc) con un array CLS: [cap][len][elems*8]. */
__declspec(dllexport) int64_t make_array_own(void) {
    unsigned char *p = (unsigned char *)malloc(16 + 3 * 8);
    int64_t cap = 3, len = 3;
    memcpy(p, &cap, 8);
    memcpy(p + 8, &len, 8);
    for (int i = 0; i < 3; i++) {
        int64_t v = 100 + i;
        memcpy(p + 16 + i * 8, &v, 8);
    }
    return (int64_t)(uintptr_t)p;
}

/* Devuelve un buffer PROPIO con un record CLS: [cap][len][(k,v,t)*24].
 * Las keys se empaquetan con OFFSET relativo al buffer (bajo, < 1MB) en los
 * bits altos: (offset<<32)|len. El JIT resuelve offset contra el buffer host. */
__declspec(dllexport) int64_t make_record_own(void) {
    unsigned char *p = (unsigned char *)malloc(16 + 2 * 24 + 2);
    int64_t cap = 2, len = 2;
    memcpy(p, &cap, 8);
    memcpy(p + 8, &len, 8);
    /* keys en p[64] ("a") y p[65] ("b") -> offset 64 y 65 */
    p[64] = 'a';
    p[65] = 'b';
    /* key "a" = (64<<32)|1 */
    int64_t k1 = ((int64_t)64 << 32) | 1;
    memcpy(p + 16, &k1, 8);
    int64_t v = 7; memcpy(p + 24, &v, 8);
    int64_t t = 0; memcpy(p + 32, &t, 8);
    /* key "b" = (65<<32)|1 */
    int64_t k2 = ((int64_t)65 << 32) | 1;
    memcpy(p + 40, &k2, 8);
    v = 8; memcpy(p + 48, &v, 8);
    t = 0; memcpy(p + 56, &t, 8);
    return (int64_t)(uintptr_t)p;
}

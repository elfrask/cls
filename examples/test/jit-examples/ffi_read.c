#include <stdint.h>
#include <string.h>

/* Lee el layout de un array CLS: [cap][len][elems*8]. */
__declspec(dllexport) int64_t array_len_probe(const void *arr) {
    const unsigned char *p = (const unsigned char *)arr;
    int64_t cap, len;
    memcpy(&cap, p, 8);
    memcpy(&len, p + 8, 8);
    if (len < 0 || len > 100000000) return -2;
    return len;
}

__declspec(dllexport) int64_t array_elem_probe(const void *arr, int64_t i) {
    const unsigned char *p = (const unsigned char *)arr;
    int64_t v;
    memcpy(&v, p + 16 + i * 8, 8);
    return v;
}

/* Lee un record CLS: [cap][len][(key,val,tag)*24]; suma los valores int. */
__declspec(dllexport) int64_t sum_record_ints(const void *rec) {
    const unsigned char *p = (const unsigned char *)rec;
    int64_t len;
    memcpy(&len, p + 8, 8);
    if (len < 0 || len > 100000000) return -2;
    int64_t sum = 0;
    for (int64_t i = 0; i < len; i++) {
        const unsigned char *e = p + 16 + i * 24;
        int64_t tag;
        memcpy(&tag, e + 16, 8);
        if (tag == 0) {
            int64_t v;
            memcpy(&v, e + 8, 8);
            sum += v;
        }
    }
    return sum;
}

/* Recibe un record CLS (host ptr), suma 1 al primer valor int encontrado,
 * y devuelve el mismo ptr (in-place en la memoria del módulo). */
__declspec(dllexport) int64_t bump_first_int(void *rec) {
    unsigned char *p = (unsigned char *)rec;
    int64_t len;
    memcpy(&len, p + 8, 8);
    if (len <= 0 || len > 1000000) return (int64_t)(uintptr_t)p;
    int64_t v;
    memcpy(&v, p + 16 + 0 * 24 + 8, 8);
    v += 1;
    memcpy(p + 16 + 0 * 24 + 8, &v, 8);
    return (int64_t)(uintptr_t)p;
}

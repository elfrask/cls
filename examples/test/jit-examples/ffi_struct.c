#include <stdint.h>
#include <string.h>
#include <stdlib.h>

/* Devuelve un buffer PROPIO con un struct CLS Punto {x,y}: layout CLS de
 * struct = [def_id:i64][len:i64][campos contiguos]. x en +16, y en +24. */
__declspec(dllexport) int64_t make_punto_own(void) {
    unsigned char *p = (unsigned char *)malloc(32);
    int64_t def = 0, nfields = 2;
    memcpy(p, &def, 8);
    memcpy(p + 8, &nfields, 8);
    int64_t x = 3; memcpy(p + 16, &x, 8);
    int64_t y = 5; memcpy(p + 24, &y, 8);
    return (int64_t)(uintptr_t)p;
}

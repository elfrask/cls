/* langffi_shim.c - shim C minimo para el eje FFI de examples/lang-features.
 * Sin punteros expuestos al lenguaje: solo enteros, doubles y CString.
 * Compilar: gcc -shared -O2 -o langffi_shim.dll langffi_shim.c  (ver build.ps1) */
#include <string.h>

int lf_add(int a, int b) { return a + b; }
int lf_mul(int a, int b) { return a * b; }
int lf_clamp(int v, int lo, int hi) {
    if (v < lo) return lo;
    if (v > hi) return hi;
    return v;
}
int lf_strlen(const char* s) { return (int)strlen(s); }
double lf_half(double x) { return x / 2.0; }
double lf_hypot(double x, double y) { return __builtin_sqrt(x * x + y * y); }

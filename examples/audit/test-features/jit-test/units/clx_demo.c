#include <string.h>
__declspec(dllexport) int add(int a, int b) { return a + b; }
__declspec(dllexport) double mul(double a, double b) { return a * b; }
__declspec(dllexport) const char* greet(const char* n) { return "hola"; }

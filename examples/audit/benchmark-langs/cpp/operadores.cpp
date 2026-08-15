#include <chrono>
#include <iostream>

double now_ms() {
    return std::chrono::duration<double, std::milli>(
        std::chrono::high_resolution_clock::now().time_since_epoch()).count();
}

int main() {
    const int N = 10000000;
    double t0, t1;
    long long s = 0;

    t0 = now_ms();
    for (int i = 0; i < N; i++) { s = s + i; asm volatile("" : "+r"(s)); }
    t1 = now_ms();
    std::cout << "op_add_ms: " << (t1 - t0) << "\n";

    t0 = now_ms();
    s = 1000000;
    for (int i = 0; i < N; i++) { s = s - 1; asm volatile("" : "+r"(s)); }
    t1 = now_ms();
    std::cout << "op_sub_ms: " << (t1 - t0) << "\n";

    t0 = now_ms();
    s = 1;
    for (int i = 0; i < N; i++) { s = s * 2; asm volatile("" : "+r"(s)); }
    t1 = now_ms();
    std::cout << "op_mul_ms: " << (t1 - t0) << "\n";

    t0 = now_ms();
    s = 1000000000LL;
    for (int i = 0; i < N; i++) { s = s / 2; asm volatile("" : "+r"(s)); }
    t1 = now_ms();
    std::cout << "op_div_ms: " << (t1 - t0) << "\n";

    t0 = now_ms();
    s = 999999;
    for (int i = 0; i < N; i++) { s = s % 2; asm volatile("" : "+r"(s)); }
    t1 = now_ms();
    std::cout << "op_mod_ms: " << (t1 - t0) << "\n";

    t0 = now_ms();
    bool b = true;
    for (int i = 0; i < N; i++) { b = i > 0; asm volatile("" : "+r"(b)); }
    t1 = now_ms();
    std::cout << "op_cmp_ms: " << (t1 - t0) << "\n";

    std::cout << "op_sanity: " << s << "\n";
    return 0;
}

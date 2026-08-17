#include <chrono>
#include <iostream>
#include <vector>
#include <string>
#include <cmath>

int64_t fib(int64_t n) { return n < 2 ? n : fib(n - 1) + fib(n - 2); }
int64_t cuadrado(int64_t x) { return x * x; }

double now_ms() {
    return std::chrono::duration<double, std::milli>(
        std::chrono::high_resolution_clock::now().time_since_epoch()).count();
}

int main() {
    const int N_ARITH = 20000000;
    double t0 = now_ms();
    long long sum = 0;
    for (int i = 0; i < N_ARITH; i++) {
        sum += i; sum -= 1; sum *= 2; sum /= 2; sum %= 1000000;
        if (sum < 0) sum = 0;
    }
    double t1 = now_ms();
    std::cout << "arith_result: " << sum << "\n";
    std::cout << "arith_ms: " << (t1 - t0) << "\n";

    t0 = now_ms();
    long long r = fib(30);
    t1 = now_ms();
    std::cout << "fib_result: " << r << "\n";
    std::cout << "fib_ms: " << (t1 - t0) << "\n";

    const int N_ARR = 100000;
    t0 = now_ms();
    std::vector<int> arr;
    arr.reserve(N_ARR);
    for (int i = 0; i < N_ARR; i++) arr.push_back(i);
    long long asum = 0;
    for (int x : arr) asum += x;
    t1 = now_ms();
    std::cout << "arr_len: " << arr.size() << "\n";
    std::cout << "arr_sum: " << asum << "\n";
    std::cout << "arr_ms: " << (t1 - t0) << "\n";

    const int N_STR = 10000;
    t0 = now_ms();
    std::string s;
    for (int i = 0; i < N_STR; i++) s += "x";
    t1 = now_ms();
    std::cout << "str_len: " << s.size() << "\n";
    std::cout << "str_ms: " << (t1 - t0) << "\n";

    const int N_MATH = 200000;
    t0 = now_ms();
    double acc = 0.0;
    for (int i = 0; i < N_MATH; i++) {
        acc += std::sqrt((double)(i + 1));
        acc += std::sin((double)i);
    }
    t1 = now_ms();
    std::cout << "math_result: " << acc << "\n";
    std::cout << "math_ms: " << (t1 - t0) << "\n";

    const int N_CALL = 1000000;
    t0 = now_ms();
    long long csum = 0;
    for (int i = 0; i < N_CALL; i++) csum += cuadrado(i);
    t1 = now_ms();
    std::cout << "call_result: " << csum << "\n";
    std::cout << "call_ms: " << (t1 - t0) << "\n";
    return 0;
}

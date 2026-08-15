import time, math

def fib(n):
    return n if n < 2 else fib(n - 1) + fib(n - 2)

def cuadrado(x):
    return x * x

def now_ms():
    return time.perf_counter() * 1000.0

# arith
N = 20_000_000
t0 = now_ms()
s = 0
for i in range(N):
    s += i; s -= 1; s *= 2; s //= 2; s %= 1_000_000
    if s < 0: s = 0
t1 = now_ms()
print("arith_result:", s)
print("arith_ms:", round(t1 - t0, 3))

# fib
t0 = now_ms()
r = fib(30)
t1 = now_ms()
print("fib_result:", r)
print("fib_ms:", round(t1 - t0, 3))

# array
N = 100_000
t0 = now_ms()
arr = [i for i in range(N)]
asum = sum(arr)
t1 = now_ms()
print("arr_len:", len(arr))
print("arr_sum:", asum)
print("arr_ms:", round(t1 - t0, 3))

# string
N = 10_000
t0 = now_ms()
s = ""
for i in range(N):
    s += "x"
t1 = now_ms()
print("str_len:", len(s))
print("str_ms:", round(t1 - t0, 3))

# math
N = 200_000
t0 = now_ms()
acc = 0.0
for i in range(N):
    acc += math.sqrt(i + 1)
    acc += math.sin(i)
t1 = now_ms()
print("math_result:", acc)
print("math_ms:", round(t1 - t0, 3))

# calls
N = 1_000_000
t0 = now_ms()
csum = 0
for i in range(N):
    csum += cuadrado(i)
t1 = now_ms()
print("call_result:", csum)
print("call_ms:", round(t1 - t0, 3))

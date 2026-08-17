# benchmark.py - Micro-benchmark de Python.
# Mide las MISMAS 5 cargas de trabajo que benchmark.clsx y benchmark.js.
#
# Cómo correr:
#   python benchmark.py

import time

N_FIB = 26
N_PRIMOS = 10000
N_COLLATZ = 5000
N_ARRAY = 5000
N_STRING = 5000


def fib(n):
    if n < 2:
        return n
    return fib(n - 1) + fib(n - 2)


def es_primo(n):
    if n < 2:
        return False
    d = 2
    while d * d <= n:
        if n % d == 0:
            return False
        d += 1
    return True


def collatz_pasos(n):
    pasos = 0
    while n > 1:
        if n % 2 == 0:
            n = n // 2
        else:
            n = 3 * n + 1
        pasos += 1
    return pasos


def cronometrar():
    return time.perf_counter()


print("=== Benchmark Python ===")

inicio = cronometrar()
r = fib(N_FIB)
print(f"fib({N_FIB}) = {r}  ms: {(cronometrar() - inicio) * 1000:.2f}")

inicio = cronometrar()
primos = 0
for i in range(2, N_PRIMOS + 1):
    if es_primo(i):
        primos += 1
print(f"primos({N_PRIMOS}) = {primos}  ms: {(cronometrar() - inicio) * 1000:.2f}")

inicio = cronometrar()
mejor = 0
mejor_n = 0
for j in range(1, N_COLLATZ + 1):
    p = collatz_pasos(j)
    if p > mejor:
        mejor = p
        mejor_n = j
print(f"collatz({N_COLLATZ}) mejor={mejor_n} pasos={mejor}  ms: {(cronometrar() - inicio) * 1000:.2f}")

inicio = cronometrar()
arr = []
for k in range(N_ARRAY):
    arr.append(k)
suma = 0
for v in arr:
    suma += v
print(f"array({N_ARRAY}) suma = {suma}  ms: {(cronometrar() - inicio) * 1000:.2f}")

inicio = cronometrar()
s = ""
for m in range(N_STRING):
    s = s + "a"
print(f"string({N_STRING}) len = {len(s)}  ms: {(cronometrar() - inicio) * 1000:.2f}")

print("=== Fin benchmark Python ===")

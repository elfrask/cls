import time

def now_ms():
    return time.perf_counter() * 1000.0

N = 2_000_000
s = 0

t0 = now_ms()
for i in range(N):
    s = s + i
t1 = now_ms()
print("op_add_ms:", round(t1 - t0, 3))

t0 = now_ms()
s = 1_000_000
for i in range(N):
    s = s - 1
t1 = now_ms()
print("op_sub_ms:", round(t1 - t0, 3))

t0 = now_ms()
s = 1
for i in range(N):
    s = s * 2
t1 = now_ms()
print("op_mul_ms:", round(t1 - t0, 3))

t0 = now_ms()
s = 1_000_000_000
for i in range(N):
    s = s // 2
t1 = now_ms()
print("op_div_ms:", round(t1 - t0, 3))

t0 = now_ms()
s = 999_999
for i in range(N):
    s = s % 2
t1 = now_ms()
print("op_mod_ms:", round(t1 - t0, 3))

t0 = now_ms()
b = True
for i in range(N):
    b = i > 0
t1 = now_ms()
print("op_cmp_ms:", round(t1 - t0, 3))

print("op_sanity:", s)

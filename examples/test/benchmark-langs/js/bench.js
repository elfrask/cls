"use strict";

function fib(n) {
    return n < 2 ? n : fib(n - 1) + fib(n - 2);
}
function cuadrado(x) {
    return x * x;
}
const now_ms = () => performance.now();

// arith
const N_ARITH = 20000000;
let t0 = now_ms();
let sum = 0;
for (let i = 0; i < N_ARITH; i++) {
    sum += i; sum -= 1; sum *= 2; sum = Math.floor(sum / 2); sum %= 1000000;
    if (sum < 0) sum = 0;
}
let t1 = now_ms();
console.log("arith_result:", sum);
console.log("arith_ms:", (t1 - t0).toFixed(3));

// fib
t0 = now_ms();
let r = fib(30);
t1 = now_ms();
console.log("fib_result:", r);
console.log("fib_ms:", (t1 - t0).toFixed(3));

// array
const N_ARR = 100000;
t0 = now_ms();
const arr = [];
for (let i = 0; i < N_ARR; i++) arr.push(i);
let asum = 0;
for (const x of arr) asum += x;
t1 = now_ms();
console.log("arr_len:", arr.length);
console.log("arr_sum:", asum);
console.log("arr_ms:", (t1 - t0).toFixed(3));

// string
const N_STR = 10000;
t0 = now_ms();
let s = "";
for (let i = 0; i < N_STR; i++) s += "x";
t1 = now_ms();
console.log("str_len:", s.length);
console.log("str_ms:", (t1 - t0).toFixed(3));

// math
const N_MATH = 200000;
t0 = now_ms();
let acc = 0.0;
for (let i = 0; i < N_MATH; i++) {
    acc += Math.sqrt(i + 1);
    acc += Math.sin(i);
}
t1 = now_ms();
console.log("math_result:", acc);
console.log("math_ms:", (t1 - t0).toFixed(3));

// calls
const N_CALL = 1000000;
t0 = now_ms();
let csum = 0;
for (let i = 0; i < N_CALL; i++) csum += cuadrado(i);
t1 = now_ms();
console.log("call_result:", csum);
console.log("call_ms:", (t1 - t0).toFixed(3));

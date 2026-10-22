"use strict";
const N = 10000000;
let s = 0;
const now_ms = () => performance.now();

let t0 = now_ms();
for (let i = 0; i < N; i++) { s = s + i; }
let t1 = now_ms();
console.log("op_add_ms:", (t1 - t0).toFixed(3));

t0 = now_ms();
s = 1000000;
for (let i = 0; i < N; i++) { s = s - 1; }
t1 = now_ms();
console.log("op_sub_ms:", (t1 - t0).toFixed(3));

t0 = now_ms();
s = 1;
for (let i = 0; i < N; i++) { s = s * 2; }
t1 = now_ms();
console.log("op_mul_ms:", (t1 - t0).toFixed(3));

t0 = now_ms();
s = 1000000000;
for (let i = 0; i < N; i++) { s = Math.floor(s / 2); }
t1 = now_ms();
console.log("op_div_ms:", (t1 - t0).toFixed(3));

t0 = now_ms();
s = 999999;
for (let i = 0; i < N; i++) { s = s % 2; }
t1 = now_ms();
console.log("op_mod_ms:", (t1 - t0).toFixed(3));

t0 = now_ms();
let b = true;
for (let i = 0; i < N; i++) { b = i > 0; }
t1 = now_ms();
console.log("op_cmp_ms:", (t1 - t0).toFixed(3));

console.log("op_sanity:", s);

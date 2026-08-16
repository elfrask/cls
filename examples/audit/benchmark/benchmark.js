// benchmark.js - Micro-benchmark de Node.js.
// Mide las MISMAS 5 cargas de trabajo que benchmark.clsx y benchmark.py.
//
// Cómo correr:
//   node benchmark.js

const N_FIB = 26;
const N_PRIMOS = 10000;
const N_COLLATZ = 5000;
const N_ARRAY = 5000;
const N_STRING = 5000;

function fib(n) {
  if (n < 2) return n;
  return fib(n - 1) + fib(n - 2);
}

function esPrimo(n) {
  if (n < 2) return false;
  for (let d = 2; d * d <= n; d++) {
    if (n % d === 0) return false;
  }
  return true;
}

function collatzPasos(n) {
  let pasos = 0;
  while (n > 1) {
    if (n % 2 === 0) {
      n = n / 2;
    } else {
      n = 3 * n + 1;
    }
    pasos++;
  }
  return pasos;
}

console.log("=== Benchmark Node ===");

let inicio = performance.now();
let r = fib(N_FIB);
console.log(`fib(${N_FIB}) = ${r}  ms: ${(performance.now() - inicio).toFixed(2)}`);

inicio = performance.now();
let primos = 0;
for (let i = 2; i <= N_PRIMOS; i++) {
  if (esPrimo(i)) primos++;
}
console.log(`primos(${N_PRIMOS}) = ${primos}  ms: ${(performance.now() - inicio).toFixed(2)}`);

inicio = performance.now();
let mejor = 0;
let mejorN = 0;
for (let j = 1; j <= N_COLLATZ; j++) {
  const p = collatzPasos(j);
  if (p > mejor) {
    mejor = p;
    mejorN = j;
  }
}
console.log(`collatz(${N_COLLATZ}) mejor=${mejorN} pasos=${mejor}  ms: ${(performance.now() - inicio).toFixed(2)}`);

inicio = performance.now();
const arr = [];
for (let k = 0; k < N_ARRAY; k++) arr.push(k);
let suma = 0;
for (const v of arr) suma += v;
console.log(`array(${N_ARRAY}) suma = ${suma}  ms: ${(performance.now() - inicio).toFixed(2)}`);

inicio = performance.now();
let s = "";
for (let m = 0; m < N_STRING; m++) s = s + "a";
console.log(`string(${N_STRING}) len = ${s.length}  ms: ${(performance.now() - inicio).toFixed(2)}`);

console.log("=== Fin benchmark Node ===");

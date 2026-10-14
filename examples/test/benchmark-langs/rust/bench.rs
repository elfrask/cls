use std::time::Instant;

fn fib(n: i64) -> i64 {
    if n < 2 { n } else { fib(n - 1) + fib(n - 2) }
}
#[inline(always)]
fn cuadrado(x: i64) -> i64 { x * x }

fn main() {
    let n_arith = 20_000_000i64;
    let t0 = Instant::now();
    let mut sum: i64 = 0;
    let mut i: i64 = 0;
    while i < n_arith {
        sum += i; sum -= 1; sum *= 2; sum /= 2; sum %= 1_000_000;
        if sum < 0 { sum = 0; }
        i += 1;
    }
    let t1 = Instant::now();
    println!("arith_result: {}", sum);
    println!("arith_ms: {}", t1.duration_since(t0).as_secs_f64() * 1000.0);

    let t0 = Instant::now();
    let r = fib(30);
    let t1 = Instant::now();
    println!("fib_result: {}", r);
    println!("fib_ms: {}", t1.duration_since(t0).as_secs_f64() * 1000.0);

    let n_arr = 100_000usize;
    let t0 = Instant::now();
    let mut arr: Vec<i64> = Vec::with_capacity(n_arr);
    for i in 0..n_arr { arr.push(i as i64); }
    let mut asum: i64 = 0;
    for x in &arr { asum += x; }
    let t1 = Instant::now();
    println!("arr_len: {}", arr.len());
    println!("arr_sum: {}", asum);
    println!("arr_ms: {}", t1.duration_since(t0).as_secs_f64() * 1000.0);

    let n_str = 10_000usize;
    let t0 = Instant::now();
    let mut s = String::new();
    for _ in 0..n_str { s.push('x'); }
    let t1 = Instant::now();
    println!("str_len: {}", s.len());
    println!("str_ms: {}", t1.duration_since(t0).as_secs_f64() * 1000.0);

    let n_math = 200_000i64;
    let t0 = Instant::now();
    let mut acc: f64 = 0.0;
    for i in 0..n_math {
        acc += ((i + 1) as f64).sqrt();
        acc += (i as f64).sin();
    }
    let t1 = Instant::now();
    println!("math_result: {}", acc);
    println!("math_ms: {}", t1.duration_since(t0).as_secs_f64() * 1000.0);

    let n_call = 1_000_000i64;
    let t0 = Instant::now();
    let mut csum: i64 = 0;
    for i in 0..n_call { csum += cuadrado(i); }
    let t1 = Instant::now();
    println!("call_result: {}", csum);
    println!("call_ms: {}", t1.duration_since(t0).as_secs_f64() * 1000.0);
}

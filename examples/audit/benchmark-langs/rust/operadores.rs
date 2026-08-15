use std::time::Instant;

fn main() {
    let n = 10_000_000i64;
    let mut s: i64 = 0;
    let t0 = Instant::now();
    for i in 0..n { s += i; std::hint::black_box(&mut s); }
    let t1 = Instant::now();
    println!("op_add_ms: {}", t1.duration_since(t0).as_secs_f64() * 1000.0);

    let t0 = Instant::now();
    s = 1_000_000;
    for _ in 0..n { s -= 1; std::hint::black_box(&mut s); }
    let t1 = Instant::now();
    println!("op_sub_ms: {}", t1.duration_since(t0).as_secs_f64() * 1000.0);

    let t0 = Instant::now();
    s = 1;
    for _ in 0..n { s *= 2; std::hint::black_box(&mut s); }
    let t1 = Instant::now();
    println!("op_mul_ms: {}", t1.duration_since(t0).as_secs_f64() * 1000.0);

    let t0 = Instant::now();
    s = 1_000_000_000;
    for _ in 0..n { s /= 2; std::hint::black_box(&mut s); }
    let t1 = Instant::now();
    println!("op_div_ms: {}", t1.duration_since(t0).as_secs_f64() * 1000.0);

    let t0 = Instant::now();
    s = 999_999;
    for _ in 0..n { s %= 2; std::hint::black_box(&mut s); }
    let t1 = Instant::now();
    println!("op_mod_ms: {}", t1.duration_since(t0).as_secs_f64() * 1000.0);

    let t0 = Instant::now();
    let mut b = true;
    for i in 0..n { b = i > 0; std::hint::black_box(&mut b); }
    let t1 = Instant::now();
    println!("op_cmp_ms: {}", t1.duration_since(t0).as_secs_f64() * 1000.0);

    println!("op_sanity: {}", s);
}

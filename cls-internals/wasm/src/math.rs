//! stdlib `math` en WASM (paridad con `cls-jit/src/host.rs` — `host_math_*`).
//!
//! - `sqrt/floor/ceil/round/min/max/abs` son instrucciones nativas (bit-exactas).
//! - `sin/cos/tan/log/pow` usan el libm de Rust para wasm32 (determinista).
//! - `random` queda en el host (entropía del SO); `range` se porta (alloc local).

use crate::mem;

#[no_mangle]
pub extern "C" fn __intr_math_sqrt(v: f64) -> f64 {
    libm::sqrt(v)
}

#[no_mangle]
pub extern "C" fn __intr_math_pow(a: f64, b: f64) -> f64 {
    libm::pow(a, b)
}

#[no_mangle]
pub extern "C" fn __intr_math_min(a: f64, b: f64) -> f64 {
    a.min(b)
}

#[no_mangle]
pub extern "C" fn __intr_math_max(a: f64, b: f64) -> f64 {
    a.max(b)
}

#[no_mangle]
pub extern "C" fn __intr_math_floor(v: f64) -> f64 {
    libm::floor(v)
}

#[no_mangle]
pub extern "C" fn __intr_math_ceil(v: f64) -> f64 {
    libm::ceil(v)
}

#[no_mangle]
pub extern "C" fn __intr_math_round(v: f64) -> f64 {
    libm::round(v)
}

#[no_mangle]
pub extern "C" fn __intr_math_sin(v: f64) -> f64 {
    libm::sin(v)
}

#[no_mangle]
pub extern "C" fn __intr_math_cos(v: f64) -> f64 {
    libm::cos(v)
}

#[no_mangle]
pub extern "C" fn __intr_math_tan(v: f64) -> f64 {
    libm::tan(v)
}

#[no_mangle]
pub extern "C" fn __intr_math_log(v: f64) -> f64 {
    libm::log(v)
}

#[no_mangle]
pub extern "C" fn __intr_math_fmod(a: f64, b: f64) -> f64 {
    libm::fmod(a, b)
}

/// `abs` entero: `i64.abs` NO existe como instrucción WASM (solo `f64.abs` →
/// `F64Abs` inline). Paridad con `host_int_abs` del host.
#[no_mangle]
pub extern "C" fn __intr_int_abs(v: i64) -> i64 {
    if v < 0 { -v } else { v }
}

#[no_mangle]
pub extern "C" fn __intr_pow_num(a: i64, b: i64) -> i64 {
    if b == 0 {
        1
    } else {
        libm::pow(a as f64, b as f64) as i64
    }
}

/// `math.range(a, b)` → array de i64 `[a, a+1, ..., b-1]`.
#[no_mangle]
pub extern "C" fn __intr_math_range(a: i64, b: i64) -> i64 {
    unsafe {
        let n = (b - a).max(0);
        let size = (n * 8 + 16) as i64;
        let ptr = mem::alloc(size) as usize;
        mem::write_i64(ptr, n);
        mem::write_i64(ptr + 8, n);
        for i in 0..n {
            mem::write_i64(ptr + 16 + (i as usize) * 8, a + i);
        }
        ptr as i64
    }
}

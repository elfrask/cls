//! stdlib `math` en WASM (paridad con `cls-jit/src/host.rs` — `host_math_*`).
//!
//! - `sqrt/floor/ceil/round/min/max/abs` son instrucciones nativas (bit-exactas).
//! - `sin/cos/tan/log/pow` usan el libm de Rust para wasm32 (determinista).
//!   **Decisión pendiente (Fase 2 §Riesgos)**: minimax inline (~1e-12) vs libm
//!   del SO. Aquí: libm de Rust (paridad cercana con el host; puede diferir en
//!   los últimos bits vs libm del sistema).
//! - `random` queda en el host (entropía del SO); `range` se porta (alloc local).

use crate::mem;

#[no_mangle]
pub extern "C" fn __intr_math_sqrt(v: f64) -> f64 {
    v.sqrt()
}

#[no_mangle]
pub extern "C" fn __intr_math_pow(a: f64, b: f64) -> f64 {
    a.powf(b)
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
    v.floor()
}

#[no_mangle]
pub extern "C" fn __intr_math_ceil(v: f64) -> f64 {
    v.ceil()
}

#[no_mangle]
pub extern "C" fn __intr_math_round(v: f64) -> f64 {
    v.round()
}

#[no_mangle]
pub extern "C" fn __intr_math_sin(v: f64) -> f64 {
    v.sin()
}

#[no_mangle]
pub extern "C" fn __intr_math_cos(v: f64) -> f64 {
    v.cos()
}

#[no_mangle]
pub extern "C" fn __intr_math_tan(v: f64) -> f64 {
    v.tan()
}

#[no_mangle]
pub extern "C" fn __intr_math_log(v: f64) -> f64 {
    v.ln()
}

#[no_mangle]
pub extern "C" fn __intr_math_fmod(a: f64, b: f64) -> f64 {
    a % b
}

#[no_mangle]
pub extern "C" fn __intr_pow_num(a: i64, b: i64) -> i64 {
    if b == 0 {
        1
    } else {
        (a as f64).powi(b as i32) as i64
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

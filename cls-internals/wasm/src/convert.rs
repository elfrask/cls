//! Conversiones de primitivos (paridad con `cls-jit/src/host.rs` — `host_parse_*`).
//!
//! Los parseos fallidos no trapean (el WASM no tiene canal de error): se
//! devuelve 0 y se marca `__intr_parse_error` (0 = ok, 1 = error). La Fase 3
//! decide cómo el emisor convierte el error en trap CLS (con el mensaje
//! original desde el texto).

use crate::mem;

/// Flag de error del último parse (`0` = ok, `1` = falló).
/// Se lee vía `__intr_parse_error_get()` (los `static mut` se materializan en
/// la memoria lineal, no como global WASM con valor).
static mut __intr_parse_error: i32 = 0;

/// Lee el flag de error del último parse.
#[no_mangle]
pub extern "C" fn __intr_parse_error_get() -> i32 {
    unsafe { __intr_parse_error }
}

#[no_mangle]
pub extern "C" fn __intr_parse_int(v: i64) -> i64 {
    unsafe {
        let s = mem::read_str(v);
        match s.trim().parse::<i64>() {
            Ok(x) => {
                __intr_parse_error = 0;
                x
            }
            Err(_) => {
                __intr_parse_error = 1;
                0
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn __intr_parse_float(v: i64) -> f64 {
    unsafe {
        let s = mem::read_str(v);
        match s.trim().parse::<f64>() {
            Ok(x) => {
                __intr_parse_error = 0;
                x
            }
            Err(_) => {
                __intr_parse_error = 1;
                0.0
            }
        }
    }
}

/// Truthiness de string (paridad walker): vacío → false, no vacío → true.
#[no_mangle]
pub extern "C" fn __intr_parse_bool(v: i64) -> i32 {
    unsafe {
        let s = mem::read_str(v);
        if s.is_empty() {
            0
        } else {
            1
        }
    }
}

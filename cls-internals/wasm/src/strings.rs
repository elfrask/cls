//! Operaciones de strings (paridad con `cls-jit/src/host.rs` — `host_str_*`).
//! Strings empaquetadas `(ptr<<32)|len`; las de salida se escriben en la
//! memoria lineal (bump del allocator del módulo).

use alloc::format;
use alloc::string::ToString;
use crate::mem;

#[no_mangle]
pub extern "C" fn __intr_str_concat(a: i64, b: i64) -> i64 {
    unsafe {
        let sa = mem::read_str(a);
        let sb = mem::read_str(b);
        mem::write_str(&format!("{}{}", sa, sb))
    }
}

#[no_mangle]
pub extern "C" fn __intr_str_int(v: i64) -> i64 {
    unsafe { mem::write_str(&v.to_string()) }
}

#[no_mangle]
pub extern "C" fn __intr_str_float(v: f64) -> i64 {
    unsafe { mem::write_str(&crate::fmt::format_float(v)) }
}

#[no_mangle]
pub extern "C" fn __intr_str_bool(v: i32) -> i64 {
    unsafe { mem::write_str(if v != 0 { "true" } else { "false" }) }
}

#[no_mangle]
pub extern "C" fn __intr_str_char(v: i32) -> i64 {
    unsafe {
        let c = char::from_u32(v as u32).unwrap_or('?');
        mem::write_str(&c.to_string())
    }
}

#[no_mangle]
pub extern "C" fn __intr_str_upper(v: i64) -> i64 {
    unsafe {
        let s = mem::read_str(v);
        mem::write_str(&s.to_uppercase())
    }
}

#[no_mangle]
pub extern "C" fn __intr_str_lower(v: i64) -> i64 {
    unsafe {
        let s = mem::read_str(v);
        mem::write_str(&s.to_lowercase())
    }
}

#[no_mangle]
pub extern "C" fn __intr_str_trim(v: i64) -> i64 {
    unsafe {
        let s = mem::read_str(v);
        mem::write_str(s.trim())
    }
}

#[no_mangle]
pub extern "C" fn __intr_str_contains(a: i64, b: i64) -> i32 {
    unsafe {
        let sa = mem::read_str(a);
        let sb = mem::read_str(b);
        if sa.contains(&sb) {
            1
        } else {
            0
        }
    }
}

#[no_mangle]
pub extern "C" fn __intr_str_starts_with(a: i64, b: i64) -> i32 {
    unsafe {
        let sa = mem::read_str(a);
        let sb = mem::read_str(b);
        if sa.starts_with(&sb) {
            1
        } else {
            0
        }
    }
}

#[no_mangle]
pub extern "C" fn __intr_str_ends_with(a: i64, b: i64) -> i32 {
    unsafe {
        let sa = mem::read_str(a);
        let sb = mem::read_str(b);
        if sa.ends_with(&sb) {
            1
        } else {
            0
        }
    }
}

#[no_mangle]
pub extern "C" fn __intr_str_is_empty(v: i64) -> i32 {
    unsafe {
        let s = mem::read_str(v);
        if s.is_empty() {
            1
        } else {
            0
        }
    }
}

#[no_mangle]
pub extern "C" fn __intr_str_repr(v: i64) -> i64 {
    unsafe {
        let s = mem::read_str(v);
        let escaped = s
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\t', "\\t");
        mem::write_str(&format!("\"{}\"", escaped))
    }
}

#[no_mangle]
pub extern "C" fn __intr_str_length(v: i64) -> i64 {
    unsafe {
        let s = mem::read_str(v);
        s.len() as i64
    }
}

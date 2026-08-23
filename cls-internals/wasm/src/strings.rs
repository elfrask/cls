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

/// Igualdad de strings por CONTENIDO (no por puntero). `a`, `b` = packed.
/// Devuelve 1 si son iguales, 0 si no. Si comparten el mismo puntero -> true.
#[no_mangle]
pub extern "C" fn __intr_str_eq(a: i64, b: i64) -> i32 {
    unsafe {
        let a_ptr = (a >> 32) as usize;
        let a_len = (a & 0xffff_ffff) as usize;
        let b_ptr = (b >> 32) as usize;
        let b_len = (b & 0xffff_ffff) as usize;
        if a_ptr == b_ptr {
            return 1;
        }
        if a_len != b_len {
            return 0;
        }
        let sa = core::slice::from_raw_parts(a_ptr as *const u8, a_len);
        let sb = core::slice::from_raw_parts(b_ptr as *const u8, b_len);
        if sa == sb { 1 } else { 0 }
    }
}

/// Convierte un valor dinámico `(val, tag)` a string según su TAG (para
/// `str(any)` sobre valores leídos de records/JSON). Tags runtime:
/// 0=int 1=string 2=float 3=bool 4=char 5=cmx 6=array 7=record 12=null.
#[no_mangle]
pub extern "C" fn __intr_any_to_string(val: i64, tag: i64) -> i64 {
    unsafe {
        let s = crate::fmt::fmt_val_to_string(val, tag);
        mem::write_str(&s)
    }
}

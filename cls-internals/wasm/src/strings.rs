//! Operaciones de strings (paridad con `cls-jit/src/host.rs` — `host_str_*`).
//! Strings empaquetadas `(ptr<<32)|len`; las de salida se escriben en la
//! memoria lineal (bump del allocator del módulo).

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
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

/// `str.indexOf(s, sub) -> int`. Índice de la primera ocurrencia de `sub` en
/// `s` (bytes), o -1 si no está. Paridad con `host_str_index_of`.
#[no_mangle]
pub extern "C" fn __intr_str_index_of(s: i64, sub: i64) -> i64 {
    unsafe {
        let s = mem::read_str(s);
        let sub = mem::read_str(sub);
        match s.find(&sub) {
            Some(idx) => idx as i64,
            None => -1,
        }
    }
}

/// `str.slice(s, start, end) -> String`. Substring de `s` desde `start`
/// (inclusive) hasta `end` (exclusive), en bytes. `end < 0` = hasta el final.
/// Paridad con `host_str_slice`.
#[no_mangle]
pub extern "C" fn __intr_str_slice(s: i64, start: i64, end: i64) -> i64 {
    unsafe {
        let s = mem::read_str(s);
        let len = s.len() as i64;
        let start = start.max(0).min(len);
        let end = if end < 0 {
            len
        } else {
            end.max(start).min(len)
        };
        mem::write_str(&s[start as usize..end as usize])
    }
}

/// `str.split(s, sep) -> String[]`. Divide `s` por el separador `sep`.
/// Devuelve un array `[cap:i64][len:i64][elems*8]` (strings packed).
/// Paridad con `host_str_split`.
#[no_mangle]
pub extern "C" fn __intr_str_split(s: i64, sep: i64) -> i64 {
    unsafe {
        let s = mem::read_str(s);
        let sep = mem::read_str(sep);
        let parts: Vec<String> = if sep.is_empty() {
            vec![s]
        } else {
            s.split(&sep).map(|p| p.to_string()).collect()
        };
        let n = parts.len() as i64;
        let array_ptr = mem::alloc(n * 8 + 16);
        if array_ptr == 0 {
            return 0;
        }
        mem::write_i64(array_ptr, n);
        mem::write_i64(array_ptr + 8, n);
        for (i, part) in parts.iter().enumerate() {
            let sp = mem::write_str(part);
            mem::write_i64(array_ptr + 16 + i * 8, sp);
        }
        array_ptr as i64
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

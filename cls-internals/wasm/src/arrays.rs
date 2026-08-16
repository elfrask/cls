//! Operaciones de arrays en la memoria lineal (paridad con
//! `cls-jit/src/host.rs` — `host_arr_*`).
//! Layout: `[cap:i64][len:i64][elems*es]` con `es` = 4 (i32) u 8 (i64/f64).

use alloc::string::{String, ToString};
use crate::mem;

pub(crate) unsafe fn arr_len(ptr: usize) -> i64 {
    mem::read_i64(ptr + 8)
}

unsafe fn arr_cap(ptr: usize) -> i64 {
    mem::read_i64(ptr)
}

pub(crate) unsafe fn arr_elem(ptr: usize, idx: usize, es: usize) -> i64 {
    let addr = ptr + 16 + idx * es;
    if es == 4 {
        mem::read_i32(addr) as i64
    } else {
        mem::read_i64(addr)
    }
}

unsafe fn arr_set(ptr: usize, idx: usize, es: usize, v: i64) {
    let addr = ptr + 16 + idx * es;
    if es == 4 {
        mem::write_i32(addr, v as i32);
    } else {
        mem::write_i64(addr, v);
    }
}

unsafe fn arr_realloc(ptr: usize, new_cap: usize, es: usize) -> usize {
    let len = arr_len(ptr) as usize;
    let size = (new_cap * es + 16) as i64;
    let new_ptr = mem::alloc(size);
    mem::write_i64(new_ptr, new_cap as i64);
    mem::write_i64(new_ptr + 8, len as i64);
    for i in 0..len {
        let e = arr_elem(ptr, i, es);
        arr_set(new_ptr, i, es, e);
    }
    new_ptr
}

#[no_mangle]
pub extern "C" fn __intr_arr_push(ptr: i64, val: i64, es: i64) -> i64 {
    unsafe {
        let p = ptr as usize;
        let len = arr_len(p);
        let cap = arr_cap(p);
        let new_p = if len + 1 > cap {
            arr_realloc(p, ((cap * 2 + 1).max(len + 1)) as usize, es as usize)
        } else {
            p
        };
        arr_set(new_p, len as usize, es as usize, val);
        mem::write_i64(new_p + 8, len + 1);
        new_p as i64
    }
}

/// Realloc de array: copia a un bloque de `new_cap` (para el `push` inline del
/// emisor: el caso común sin realloc se emite como store + len++, solo el
/// crecimiento llama acá). Paridad con `arr_realloc` del host.
#[no_mangle]
pub extern "C" fn __intr_arr_realloc(ptr: i64, new_cap: i64, es: i64) -> i64 {
    unsafe {
        arr_realloc(ptr as usize, new_cap as usize, es as usize) as i64
    }
}

#[no_mangle]
pub extern "C" fn __intr_arr_pop(ptr: i64, _es: i64) -> i64 {
    unsafe {
        let p = ptr as usize;
        let len = arr_len(p);
        if len <= 0 {
            return p as i64;
        }
        mem::write_i64(p + 8, len - 1);
        p as i64
    }
}

#[no_mangle]
pub extern "C" fn __intr_arr_shift(ptr: i64, es: i64) -> i64 {
    unsafe {
        let p = ptr as usize;
        let es = es as usize;
        let len = arr_len(p);
        if len <= 0 {
            return p as i64;
        }
        for i in 0..(len - 1) as usize {
            let e = arr_elem(p, i + 1, es);
            arr_set(p, i, es, e);
        }
        mem::write_i64(p + 8, len - 1);
        p as i64
    }
}

#[no_mangle]
pub extern "C" fn __intr_arr_unshift(ptr: i64, val: i64, es: i64) -> i64 {
    unsafe {
        let p = ptr as usize;
        let es = es as usize;
        let len = arr_len(p);
        let cap = arr_cap(p);
        let new_p = if len + 1 > cap {
            arr_realloc(p, ((cap * 2 + 1).max(len + 1)) as usize, es)
        } else {
            p
        };
        for i in (0..len as usize).rev() {
            let e = arr_elem(new_p, i, es);
            arr_set(new_p, i + 1, es, e);
        }
        arr_set(new_p, 0, es, val);
        mem::write_i64(new_p + 8, len + 1);
        new_p as i64
    }
}

#[no_mangle]
pub extern "C" fn __intr_arr_reverse(ptr: i64, es: i64) -> i64 {
    unsafe {
        let p = ptr as usize;
        let es = es as usize;
        let len = arr_len(p);
        for i in 0..(len as usize / 2) {
            let a = arr_elem(p, i, es);
            let b = arr_elem(p, (len as usize) - 1 - i, es);
            arr_set(p, i, es, b);
            arr_set(p, (len as usize) - 1 - i, es, a);
        }
        p as i64
    }
}

#[no_mangle]
pub extern "C" fn __intr_arr_index_of(ptr: i64, needle: i64, es: i64) -> i64 {
    unsafe {
        let p = ptr as usize;
        let len = arr_len(p);
        for i in 0..len as usize {
            if arr_elem(p, i, es as usize) == needle {
                return i as i64;
            }
        }
        -1
    }
}

#[no_mangle]
pub extern "C" fn __intr_arr_includes(ptr: i64, needle: i64, es: i64) -> i32 {
    unsafe {
        let p = ptr as usize;
        let len = arr_len(p);
        for i in 0..len as usize {
            if arr_elem(p, i, es as usize) == needle {
                return 1;
            }
        }
        0
    }
}

#[no_mangle]
pub extern "C" fn __intr_arr_join(ptr: i64, sep: i64, es: i64, kind: i64) -> i64 {
    unsafe {
        let p = ptr as usize;
        let es = es as usize;
        let len = arr_len(p);
        let separator = mem::read_str(sep);
        let mut out = String::new();
        for i in 0..len as usize {
            if i > 0 {
                out.push_str(&separator);
            }
            let e = arr_elem(p, i, es);
            match kind {
                1 => out.push_str(&mem::read_str(e)),
                2 => out.push_str(&crate::fmt::format_float(f64::from_bits(e as u64))),
                3 => out.push_str(if e != 0 { "true" } else { "false" }),
                4 => out.push(char::from_u32(e as u32).unwrap_or('?')),
                _ => out.push_str(&e.to_string()),
            }
        }
        mem::write_str(&out)
    }
}

#[no_mangle]
pub extern "C" fn __intr_arr_to_string(ptr: i64, es: i64, kind: i64) -> i64 {
    unsafe {
        let s = crate::fmt::arr_to_string(ptr, es, kind);
        mem::write_str(&s)
    }
}

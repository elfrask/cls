//! Operaciones de arrays en la memoria lineal (paridad con
//! `cls-jit/src/host.rs` — `host_arr_*`).
//! Layout: `[cap:i64][len:i64][elems*es]` con `es` = 4 (i32) u 8 (i64/f64).

use alloc::string::ToString;
use alloc::vec::Vec;
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

/// Magic de "buffer reubicado": cuando `push`/`unshift` crecen un array, el
/// buffer viejo NO se libera (bump allocator) sino que se convierte en un
/// **forward**: `cap = FWD_MAGIC`, `len = <ptr del buffer nuevo>`. Todos los
/// accesos resuelven la cadena con `arr_resolve`, así que cualquier alias que
/// todavía apunte al buffer viejo (p.ej. la variable del llamador cuando el
/// mutador se llamó sobre un parámetro) ve la versión crecida == semántica de
/// referencia para arrays.
const FWD_MAGIC: i64 = i64::MIN;

/// Sigue la cadena de forwards hasta el buffer real. `ptr == 0` o un buffer
/// normal se devuelven tal cual. Idempotente (resolver un ptr ya resuelto es
/// no-op), por lo que puede aplicarse sin miedo en cualquier acceso a array.
pub(crate) unsafe fn arr_resolve(ptr: usize) -> usize {
    let mut p = ptr;
    let mut guard = 0u32;
    while p != 0 && mem::read_i64(p) == FWD_MAGIC {
        p = mem::read_i64(p + 8) as usize;
        guard += 1;
        if guard > 1_000_000 {
            break;
        }
    }
    p
}

/// Marca `old` como forward hacia `new`.
unsafe fn arr_mark_forward(old: usize, new: usize) {
    if old == new {
        return;
    }
    mem::write_i64(old, FWD_MAGIC);
    mem::write_i64(old + 8, new as i64);
}

/// `__intr_arr_deref(ptr) -> ptr` — resuelve la cadena de forwards (idempotente).
#[no_mangle]
pub extern "C" fn __intr_arr_deref(ptr: i64) -> i64 {
    unsafe { arr_resolve(ptr as usize) as i64 }
}

#[no_mangle]
pub extern "C" fn __intr_arr_push(ptr: i64, val: i64, es: i64) -> i64 {
    unsafe {
        let p = arr_resolve(ptr as usize);
        let len = arr_len(p);
        let cap = arr_cap(p);
        let new_p = if len + 1 > cap {
            let np = arr_realloc(p, ((cap * 2 + 1).max(len + 1)) as usize, es as usize);
            arr_mark_forward(p, np);
            np
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
        let p = arr_resolve(ptr as usize);
        let np = arr_realloc(p, new_cap as usize, es as usize);
        arr_mark_forward(p, np);
        np as i64
    }
}

#[no_mangle]
pub extern "C" fn __intr_arr_pop(ptr: i64, _es: i64) -> i64 {
    unsafe {
        let p = arr_resolve(ptr as usize);
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
        let p = arr_resolve(ptr as usize);
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
        let p = arr_resolve(ptr as usize);
        let es = es as usize;
        let len = arr_len(p);
        let cap = arr_cap(p);
        let new_p = if len + 1 > cap {
            let np = arr_realloc(p, ((cap * 2 + 1).max(len + 1)) as usize, es);
            arr_mark_forward(p, np);
            np
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
        let p = arr_resolve(ptr as usize);
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
        let p = arr_resolve(ptr as usize);
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
        let p = arr_resolve(ptr as usize);
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
        let p = arr_resolve(ptr as usize);
        let es = es as usize;
        let len = arr_len(p);
        let separator = mem::read_bytes(sep);
        let mut out: Vec<u8> = Vec::new();
        for i in 0..len as usize {
            if i > 0 {
                out.extend_from_slice(separator);
            }
            let e = arr_elem(p, i, es);
            match kind {
                1 => out.extend_from_slice(&mem::read_bytes(e)),
                2 => out.extend_from_slice(crate::fmt::format_float(f64::from_bits(e as u64)).as_bytes()),
                3 => out.extend_from_slice(if e != 0 { b"true" } else { b"false" }),
                // 4 = char: unidad de 1 byte (byte-transparente; permite
                // reconstruir un String byte a byte con join("")).
                4 => out.push((e & 0xff) as u8),
                _ => out.extend_from_slice(e.to_string().as_bytes()),
            }
        }
        mem::write_bytes(&out)
    }
}

#[no_mangle]
pub extern "C" fn __intr_arr_to_string(ptr: i64, es: i64, kind: i64) -> i64 {
    unsafe {
        let s = crate::fmt::arr_to_string(arr_resolve(ptr as usize) as i64, es, kind);
        mem::write_str(&s)
    }
}

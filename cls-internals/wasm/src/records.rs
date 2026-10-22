//! Operaciones de records (paridad con `cls-jit/src/host.rs` — `host_record_*`).
//! Layout: `[cap:i64][len:i64][(key:packed,val:i64,tag:i64)*24]`.

use crate::mem;

unsafe fn rec_len(ptr: usize) -> i64 {
    mem::read_i64(ptr + 8)
}

/// Fast path de interning (dev-2): si dos keys tienen el MISMO i64 empaquetado
/// `(ptr<<32)|len`, son el mismo string — comparación de enteros en vez de
/// leer+memcmp. Las keys literales van interned al pool, así que las búsquedas
/// con key literal golpean este camino casi siempre.
#[inline]
unsafe fn key_matches(stored_packed: i64, lookup_packed: i64) -> bool {
    if stored_packed == lookup_packed {
        return true;
    }
    mem::read_str(stored_packed) == mem::read_str(lookup_packed)
}

#[no_mangle]
pub extern "C" fn __intr_record_new(cap: i64) -> i64 {
    unsafe {
        let size = cap * 24 + 16;
        let ptr = mem::alloc(size) as usize;
        mem::write_i64(ptr, cap);
        mem::write_i64(ptr + 8, 0);
        ptr as i64
    }
}

#[no_mangle]
pub extern "C" fn __intr_record_set(ptr: i64, key: i64, val: i64, tag: i64) -> i64 {
    unsafe {
        let p = ptr as usize;
        let len = rec_len(p) as usize;
        let cap = mem::read_i64(p) as usize;
        for i in 0..len {
            let ki = mem::read_i64(p + 16 + i * 24);
            if key_matches(ki, key) {
                mem::write_i64(p + 16 + i * 24 + 8, val);
                mem::write_i64(p + 16 + i * 24 + 16, tag);
                return p as i64;
            }
        }
        let mut new_p = p;
        if len >= cap {
            let new_cap = if cap == 0 { 4 } else { cap * 2 };
            let size = (new_cap * 24 + 16) as i64;
            let np = mem::alloc(size) as usize;
            mem::write_i64(np, new_cap as i64);
            mem::write_i64(np + 8, len as i64);
            for i in 0..len {
                let kk = mem::read_i64(p + 16 + i * 24);
                let vv = mem::read_i64(p + 16 + i * 24 + 8);
                let tt = mem::read_i64(p + 16 + i * 24 + 16);
                mem::write_i64(np + 16 + i * 24, kk);
                mem::write_i64(np + 16 + i * 24 + 8, vv);
                mem::write_i64(np + 16 + i * 24 + 16, tt);
            }
            new_p = np;
        }
        mem::write_i64(new_p + 16 + len * 24, key);
        mem::write_i64(new_p + 16 + len * 24 + 8, val);
        mem::write_i64(new_p + 16 + len * 24 + 16, tag);
        mem::write_i64(new_p + 8, (len + 1) as i64);
        new_p as i64
    }
}

#[no_mangle]
pub extern "C" fn __intr_record_get(ptr: i64, key: i64) -> i64 {
    unsafe {
        let p = ptr as usize;
        let len = rec_len(p) as usize;
        for i in 0..len {
            let ki = mem::read_i64(p + 16 + i * 24);
            if key_matches(ki, key) {
                return mem::read_i64(p + 16 + i * 24 + 8);
            }
        }
        0
    }
}

#[no_mangle]
pub extern "C" fn __intr_record_has(ptr: i64, key: i64) -> i32 {
    unsafe {
        let p = ptr as usize;
        let len = rec_len(p) as usize;
        for i in 0..len {
            let ki = mem::read_i64(p + 16 + i * 24);
            if key_matches(ki, key) {
                return 1;
            }
        }
        0
    }
}

#[no_mangle]
pub extern "C" fn __intr_record_tag(ptr: i64, key: i64) -> i64 {
    unsafe {
        let p = ptr as usize;
        let len = rec_len(p) as usize;
        for i in 0..len {
            let ki = mem::read_i64(p + 16 + i * 24);
            if key_matches(ki, key) {
                return mem::read_i64(p + 16 + i * 24 + 16);
            }
        }
        0
    }
}

#[no_mangle]
pub extern "C" fn __intr_record_len(ptr: i64) -> i64 {
    unsafe { rec_len(ptr as usize) }
}

#[no_mangle]
pub extern "C" fn __intr_record_keys(ptr: i64) -> i64 {
    unsafe {
        let p = ptr as usize;
        let len = rec_len(p) as usize;
        let size = (len * 8 + 16) as i64;
        let out = mem::alloc(size) as usize;
        mem::write_i64(out, len as i64);
        mem::write_i64(out + 8, len as i64);
        for i in 0..len {
            let ki = mem::read_i64(p + 16 + i * 24);
            mem::write_i64(out + 16 + i * 8, ki);
        }
        out as i64
    }
}

#[no_mangle]
pub extern "C" fn __intr_record_values(ptr: i64) -> i64 {
    unsafe {
        let p = ptr as usize;
        let len = rec_len(p) as usize;
        let size = (len * 8 + 16) as i64;
        let out = mem::alloc(size) as usize;
        mem::write_i64(out, len as i64);
        mem::write_i64(out + 8, len as i64);
        for i in 0..len {
            let vi = mem::read_i64(p + 16 + i * 24 + 8);
            mem::write_i64(out + 16 + i * 8, vi);
        }
        out as i64
    }
}

#[no_mangle]
pub extern "C" fn __intr_record_to_string(ptr: i64) -> i64 {
    unsafe {
        let s = crate::fmt::record_to_string(ptr);
        mem::write_str(&s)
    }
}

/// Spread de records `{...src}`: copia los campos de `src` a `dst` (los campos
/// existentes en dst con la misma key se sobrescriben — el último set gana).
/// Devuelve `dst` (que puede reallocarse al crecer). REST_SPREAD_PLAN Fase 2.
#[no_mangle]
pub extern "C" fn __intr_record_merge(dst: i64, src: i64) -> i64 {
    unsafe {
        let s = src as usize;
        let n = rec_len(s) as usize;
        let mut d = dst;
        for i in 0..n {
            let key = mem::read_i64(s + 16 + i * 24);
            let val = mem::read_i64(s + 16 + i * 24 + 8);
            let tag = mem::read_i64(s + 16 + i * 24 + 16);
            // record_set puede reallocar dst -> usar el ptr retornado.
            d = __intr_record_set(d, key, val, tag);
        }
        d
    }
}

//! Operaciones de strings (paridad con `cls-jit/src/host.rs` — `host_str_*`).
//! Strings empaquetadas `(ptr<<32)|len`; las de salida se escriben en la
//! memoria lineal (bump del allocator del módulo).

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use crate::allocator;
use crate::fmt;
use crate::mem;

#[no_mangle]
pub extern "C" fn __intr_str_concat(a: i64, b: i64) -> i64 {
    unsafe {
        let sa = mem::read_bytes(a);
        let sb = mem::read_bytes(b);
        let total = sa.len() + sb.len();
        let ptr = crate::allocator::bump_alloc(total);
        if ptr == 0 {
            return 0;
        }
        core::ptr::copy_nonoverlapping(sa.as_ptr(), ptr as *mut u8, sa.len());
        core::ptr::copy_nonoverlapping(sb.as_ptr(), (ptr + sa.len()) as *mut u8, sb.len());
        ((ptr as i64) << 32) | (total as i64)
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

/// `Char.toString()` — String de **1 byte** con el valor crudo del `Char`
/// (`v & 0xFF`), byte-transparente (modelo `bytes`/`str`: `Char` es una unidad
/// de 1 byte para buffers, NO un codepoint Unicode).
#[no_mangle]
pub extern "C" fn __intr_str_char(v: i32) -> i64 {
    unsafe { mem::write_bytes(&[(v & 0xff) as u8]) }
}

/// `String.toChar() -> Char[]` — los **bytes** del string como array de `Char`
/// (cada uno 0..255). Layout de array `[cap:i64][len:i64][i32 elems]`
/// (elementos de 4 bytes; `arr_kind_code(Char) = 4`).
#[no_mangle]
pub extern "C" fn __intr_str_to_chars(s: i64) -> i64 {
    unsafe {
        let bytes = mem::read_bytes(s);
        let n = bytes.len();
        let ptr = mem::alloc(n as i64 * 4 + 16);
        if ptr == 0 {
            return 0;
        }
        mem::write_i64(ptr, n as i64);
        mem::write_i64(ptr + 8, n as i64);
        for (i, &b) in bytes.iter().enumerate() {
            mem::write_i32(ptr + 16 + i * 4, b as i32);
        }
        ptr as i64
    }
}

/// `s[i]` — byte `i` del string como String de 1 byte (semántica de bytes,
/// igual que `length`/`slice`/`indexOf`). Fuera de rango devuelve string vacío.
#[no_mangle]
pub extern "C" fn __intr_str_char_at(s: i64, i: i64) -> i64 {
    unsafe {
        let ptr = (s >> 32) as usize;
        let len = (s & 0xffff_ffff) as usize;
        if i < 0 || i as usize >= len {
            return 0; // string vacío (ptr=0, len=0)
        }
        let b = core::ptr::read((ptr as *const u8).add(i as usize));
        let out = crate::allocator::bump_alloc(1);
        if out == 0 {
            return 0;
        }
        core::ptr::write(out as *mut u8, b);
        ((out as i64) << 32) | 1
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
        let hay = mem::read_bytes(a);
        let needle = mem::read_bytes(b);
        if needle.is_empty() || hay.windows(needle.len()).any(|w| w == needle) {
            1
        } else {
            0
        }
    }
}

#[no_mangle]
pub extern "C" fn __intr_str_starts_with(a: i64, b: i64) -> i32 {
    unsafe {
        let hay = mem::read_bytes(a);
        let needle = mem::read_bytes(b);
        if hay.starts_with(needle) {
            1
        } else {
            0
        }
    }
}

#[no_mangle]
pub extern "C" fn __intr_str_ends_with(a: i64, b: i64) -> i32 {
    unsafe {
        let hay = mem::read_bytes(a);
        let needle = mem::read_bytes(b);
        if hay.ends_with(needle) {
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
    (v & 0xffff_ffff) as i64
}

/// `str.indexOf(s, sub) -> int`. Índice de la primera ocurrencia de `sub` en
/// `s` (bytes), o -1 si no está. Paridad con `host_str_index_of`.
#[no_mangle]
pub extern "C" fn __intr_str_index_of(s: i64, sub: i64) -> i64 {
    unsafe {
        let hay = mem::read_bytes(s);
        let needle = mem::read_bytes(sub);
        if needle.is_empty() {
            return 0;
        }
        if needle.len() > hay.len() {
            return -1;
        }
        for i in 0..=(hay.len() - needle.len()) {
            if &hay[i..i + needle.len()] == needle {
                return i as i64;
            }
        }
        -1
    }
}

/// `str.slice(s, start, end) -> String`. Substring de `s` desde `start`
/// (inclusive) hasta `end` (exclusive), en bytes. `end < 0` = hasta el final.
/// Paridad con `host_str_slice`.
#[no_mangle]
pub extern "C" fn __intr_str_slice(s: i64, start: i64, end: i64) -> i64 {
    unsafe {
        let bytes = mem::read_bytes(s);
        let len = bytes.len() as i64;
        let start = start.max(0).min(len);
        let end = if end < 0 {
            len
        } else {
            end.max(start).min(len)
        };
        mem::write_bytes(&bytes[start as usize..end as usize])
    }
}

/// `str.split(s, sep) -> String[]`. Divide `s` por el separador `sep`.
/// Devuelve un array `[cap:i64][len:i64][elems*8]` (strings packed).
/// Paridad con `host_str_split`.
#[no_mangle]
pub extern "C" fn __intr_str_split(s: i64, sep: i64) -> i64 {
    unsafe {
        let hay = mem::read_bytes(s);
        let needle = mem::read_bytes(sep);
        let mut parts: Vec<&[u8]> = Vec::new();
        if needle.is_empty() {
            parts.push(hay);
        } else {
            let mut start = 0usize;
            let mut i = 0usize;
            while i + needle.len() <= hay.len() {
                if &hay[i..i + needle.len()] == needle {
                    parts.push(&hay[start..i]);
                    i += needle.len();
                    start = i;
                } else {
                    i += 1;
                }
            }
            parts.push(&hay[start..]);
        }
        let n = parts.len() as i64;
        let array_ptr = mem::alloc(n * 8 + 16);
        if array_ptr == 0 {
            return 0;
        }
        mem::write_i64(array_ptr, n);
        mem::write_i64(array_ptr + 8, n);
        for (i, part) in parts.iter().enumerate() {
            let sp = mem::write_bytes(part);
            mem::write_i64(array_ptr + 16 + i * 8, sp);
        }
        array_ptr as i64
    }
}

/// Magic del header de capacidad para appends in-place: `cap | MAGIC` guardado
/// en los 8 bytes ANTERIORES al contenido (`ptr-8`). El ptr público sigue
/// apuntando directo a los bytes — el layout `(ptr<<32)|len` no cambia.
const APPEND_MAGIC: i64 = i64::MIN; // bit63 seteado: imposible en una cap real

#[inline]
unsafe fn read_cap(ptr: usize) -> Option<usize> {
    if ptr < 8 {
        return None;
    }
    let hdr = mem::read_i64(ptr - 8);
    if hdr & APPEND_MAGIC == APPEND_MAGIC {
        Some((hdr & i64::MAX) as usize)
    } else {
        None
    }
}

#[inline]
unsafe fn write_cap(ptr: usize, cap: usize) {
    mem::write_i64(ptr - 8, (cap as i64) | APPEND_MAGIC);
}

unsafe fn alloc_with_cap(bytes: usize) -> usize {
    // [cap|MAGIC][bytes...] — devolvemos ptr a los bytes.
    let block = crate::allocator::bump_alloc(bytes + 8);
    if block == 0 {
        return 0;
    }
    (block + 8) as usize
}

/// Concatenación normal (sin slack): usada por el emisor para el PRIMER
/// `s = a + b` — aloca con slack x2 y escribe el header, habilitando que los
/// appends SUBSIGUIENTES sobre `s` sean in-place (cero alocación/copia).
#[no_mangle]
pub extern "C" fn __intr_str_concat_slack(a: i64, b: i64) -> i64 {
    unsafe {
        let sa = mem::read_bytes(a);
        let sb = mem::read_bytes(b);
        let total = sa.len() + sb.len();
        let cap = (total * 2).max(total + 16);
        let ptr = alloc_with_cap(cap);
        if ptr == 0 {
            return 0;
        }
        core::ptr::copy_nonoverlapping(sa.as_ptr(), ptr as *mut u8, sa.len());
        core::ptr::copy_nonoverlapping(sb.as_ptr(), (ptr + sa.len()) as *mut u8, sb.len());
        write_cap(ptr, cap);
        ((ptr as i64) << 32) | (total as i64)
    }
}

/// `s += pieza` / `s = s + x`: si el string viejo tiene header de capacidad
/// (viene de concat_slack o de un append previo) Y hay slack suficiente,
/// escribe la pieza IN-PLACE (cero alocación, cero copia del contenido).
/// Si no, degrada a concat_slack (re-aloca con slack x2, amortizado O(1)).
///
/// `v_tag`: tag runtime de la pieza (0=int 1=string 2=float 3=bool 4=char) para
/// convertir antes de escribir. Devuelve el nuevo packed de `s`.
#[no_mangle]
pub extern "C" fn __intr_str_append(old: i64, v_val: i64, v_tag: i64) -> i64 {
    unsafe {
        let piece: Vec<u8> = match v_tag {
            1 => mem::read_bytes(v_val).to_vec(),
            // 4 = char: unidad de 1 byte (byte-transparente, no codepoint).
            4 => vec![(v_val & 0xff) as u8],
            _ => crate::fmt::fmt_val_to_string(v_val, v_tag).into_bytes(),
        };
        let old_ptr = (old >> 32) as usize;
        let old_len = (old & 0xffff_ffff) as usize;
        let total = old_len + piece.len();

        // Fast path in-place: header válido y slack suficiente.
        if let Some(cap) = read_cap(old_ptr) {
            if old_len + piece.len() <= cap {
                core::ptr::copy_nonoverlapping(
                    piece.as_ptr(),
                    (old_ptr + old_len) as *mut u8,
                    piece.len(),
                );
                return ((old_ptr as i64) << 32) | (total as i64);
            }
            // Slack insuficiente: re-aloca con crecimiento x2 (amortizado O(1)),
            // copiando el contenido viejo UNA vez más.
            let new_cap = (cap * 2).max(total + 16);
            let nptr = alloc_with_cap(new_cap);
            if nptr == 0 {
                return 0;
            }
            core::ptr::copy_nonoverlapping(old_ptr as *const u8, nptr as *mut u8, old_len);
            core::ptr::copy_nonoverlapping(
                piece.as_ptr(),
                (nptr + old_len) as *mut u8,
                piece.len(),
            );
            write_cap(nptr, new_cap);
            return ((nptr as i64) << 32) | (total as i64);
        }

        // Slow path: `old` viene sin slack (literal/json/host) → concat con slack
        // para habilitar appends futuros. Copia los bytes crudos de `old`.
        let so = mem::read_bytes(old);
        let cap = (so.len() * 2).max(so.len() + 16);
        let ptr = alloc_with_cap(cap);
        if ptr == 0 {
            return 0;
        }
        core::ptr::copy_nonoverlapping(so.as_ptr(), ptr as *mut u8, so.len());
        core::ptr::copy_nonoverlapping(piece.as_ptr(), (ptr + so.len()) as *mut u8, piece.len());
        write_cap(ptr, cap);
        ((ptr as i64) << 32) | (total as i64)
    }
}

/// Igualdad de strings por CONTENIDO (no por puntero). `a`, `b` = packed.
/// Devuelve 1 si son iguales, 0 si no. Compara la LONGITUD primero: el atajo de
/// puntero solo es válido con longitudes iguales (dos literales pueden compartir
/// ptr si uno es vacío — el pool no avanza por un literal de 0 bytes).
#[no_mangle]
pub extern "C" fn __intr_str_eq(a: i64, b: i64) -> i32 {
    unsafe {
        let a_ptr = (a >> 32) as usize;
        let a_len = (a & 0xffff_ffff) as usize;
        let b_ptr = (b >> 32) as usize;
        let b_len = (b & 0xffff_ffff) as usize;
        if a_len != b_len {
            return 0;
        }
        if a_ptr == b_ptr {
            return 1;
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

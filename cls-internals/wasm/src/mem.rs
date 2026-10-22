//! Acceso a la memoria lineal del módulo (la misma que compartirá el módulo
//! CLS en la Fase 3). Layouts:
//! - string: `(ptr<<32)|len`
//! - array: `[cap:i64][len:i64][elems*es]`
//! - record: `[cap:i64][len:i64][(key,val,tag)*24]`
//!
//! La alocación usa el bump allocator de `crate::allocator` (global `heap_ptr`
//! compartido con el módulo CLS tras la fusión — mismo layout que `__alloc`).

use alloc::string::String;
use crate::allocator::bump_alloc;

/// Lee i64 (sin alinear).
pub unsafe fn read_i64(addr: usize) -> i64 {
    core::ptr::read_unaligned(addr as *const i64)
}

/// Escribe i64 (sin alinear).
pub unsafe fn write_i64(addr: usize, v: i64) {
    core::ptr::write_unaligned(addr as *mut i64, v);
}

/// Lee i32 (sin alinear).
pub unsafe fn read_i32(addr: usize) -> i32 {
    core::ptr::read_unaligned(addr as *const i32)
}

/// Escribe i32 (sin alinear).
pub unsafe fn write_i32(addr: usize, v: i32) {
    core::ptr::write_unaligned(addr as *mut i32, v);
}

/// Lee los **bytes crudos** de un string empaquetado `(ptr<<32)|len` de la
/// memoria lineal, SIN decodificar UTF-8 (semántica de bytes del lenguaje:
/// `length`/`slice`/`indexOf`/`s[i]` operan sobre bytes, no sobre chars).
pub unsafe fn read_bytes(packed: i64) -> &'static [u8] {
    let ptr = (packed >> 32) as usize;
    let len = (packed & 0xffff_ffff) as usize;
    if len == 0 || ptr == 0 {
        return &[];
    }
    core::slice::from_raw_parts(ptr as *const u8, len)
}

/// Lee un string empaquetado y lo decodifica a `String` (lossy). Usar solo para
/// operaciones intrinsecamente de texto (upper/lower/trim/repr), que necesitan
/// UTF-8 válido; para operaciones byte-a-byte usar `read_bytes`.
pub unsafe fn read_str(packed: i64) -> String {
    let bytes = read_bytes(packed);
    if bytes.is_empty() {
        return String::new();
    }
    String::from_utf8_lossy(bytes).into_owned()
}

/// Aloca + escribe un string en la memoria lineal y lo empaqueta.
pub unsafe fn write_str(s: &str) -> i64 {
    write_bytes(s.as_bytes())
}

/// Aloca + escribe **bytes crudos** en la memoria lineal y los empaqueta.
pub unsafe fn write_bytes(b: &[u8]) -> i64 {
    let ptr = bump_alloc(b.len());
    if ptr == 0 {
        return 0;
    }
    core::ptr::copy_nonoverlapping(b.as_ptr(), ptr as *mut u8, b.len());
    ((ptr as i64) << 32) | (b.len() as i64)
}

/// Aloca `n` bytes (bump) y devuelve el offset (0 si n <= 0).
pub unsafe fn alloc(n: i64) -> usize {
    bump_alloc(n.max(0) as usize)
}

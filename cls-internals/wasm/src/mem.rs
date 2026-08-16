//! Acceso a la memoria lineal del módulo (la misma que compartirá el módulo
//! CLS en la Fase 3). Layouts:
//! - string: `(ptr<<32)|len`
//! - array: `[cap:i64][len:i64][elems*es]`
//! - record: `[cap:i64][len:i64][(key,val,tag)*24]`

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

/// Lee un string empaquetado `(ptr<<32)|len` de la memoria lineal.
pub unsafe fn read_str(packed: i64) -> String {
    let ptr = (packed >> 32) as usize;
    let len = (packed & 0xffff_ffff) as usize;
    if len == 0 {
        return String::new();
    }
    let bytes = core::slice::from_raw_parts(ptr as *const u8, len);
    String::from_utf8_lossy(bytes).into_owned()
}

/// Aloca + escribe un string en la memoria lineal y lo empaqueta.
pub unsafe fn write_str(s: &str) -> i64 {
    let mut v: Vec<u8> = s.as_bytes().to_vec();
    let ptr = v.as_mut_ptr() as i64;
    core::mem::forget(v);
    (ptr << 32) | (s.len() as i64)
}

/// Aloca `n` bytes con capacity exacta y devuelve el offset (0 si n <= 0).
pub unsafe fn alloc(n: i64) -> usize {
    if n <= 0 {
        return 0;
    }
    let v: Vec<u8> = vec![0u8; n as usize];
    let ptr = v.as_ptr() as usize;
    core::mem::forget(v);
    ptr
}

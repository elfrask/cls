//! ABI C versionado `clsb_v1_*` del nodo de bindings (embedding).
//!
//! - Handles opacos: `clsb_engine*`, `clsb_module*`, `clsb_error*`.
//! - `clsb_value`: representación C de un valor CLS (el host construye con los
//!   constructores `clsb_value_*` y libera con `clsb_value_free`).
//! - Callbacks: output (print), resolver de módulos y funciones host.
//!
//! Contrato de memoria: los valores/errores devueltos los libera el host
//! (`clsb_value_free` / `clsb_error_free`); las cadenas de `clsb_error_trace` y
//! `clsb_version` viven mientras el error/el proceso.

#![allow(non_camel_case_types)]

use std::ffi::{c_char, c_void, CStr, CString};
use std::os::raw::c_int;
use std::sync::Mutex;

use crate::{ClsEngine, ClsError, ClsModule, ClsValue};
use cls_core::middleware::types::Type;

// ── Tipos opacos y status ───────────────────────────────────────────────────

pub struct clsb_engine {
    inner: Mutex<ClsEngine>,
}

pub struct clsb_module {
    inner: ClsModule,
}

pub struct clsb_error {
    trace_c: CString,
}

/// Códigos de estado (0 = ok, distinto de 0 = error).
pub type clsb_status = c_int;
pub const CLSB_OK: clsb_status = 0;

#[repr(C)]
pub struct clsb_config {
    /// Reservado (sandbox futuro). Debe ser 0.
    pub enable_fs: c_int,
    pub enable_http: c_int,
}

// ── clsb_value ──────────────────────────────────────────────────────────────

/// Kind CLS (códigos de la custom section `clx:exports`).
pub const CLSB_INT: i32 = 0;
pub const CLSB_FLOAT: i32 = 1;
pub const CLSB_BOOL: i32 = 2;
pub const CLSB_CHAR: i32 = 3;
pub const CLSB_STRING: i32 = 4;
pub const CLSB_ARRAY: i32 = 5;
pub const CLSB_RECORD: i32 = 6;
pub const CLSB_NULL: i32 = 12;

#[repr(C)]
pub struct clsb_value {
    pub tag: i32,
    /// int / bits de float / bool (0|1) / char (codepoint).
    pub bits: i64,
    /// tag STRING: buffer UTF-8 owned (libera `clsb_value_free`).
    pub text: *const c_char,
    /// tag ARRAY: elems owned.
    pub items: *mut clsb_value,
    /// tag RECORD: claves owned + valores owned.
    pub keys: *mut *const c_char,
    pub vals: *mut clsb_value,
    /// Cantidad de elems (ARRAY) o entradas (RECORD).
    pub n: usize,
}

impl clsb_value {
    fn null() -> Self {
        Self {
            tag: CLSB_NULL,
            bits: 0,
            text: std::ptr::null(),
            items: std::ptr::null_mut(),
            keys: std::ptr::null_mut(),
            vals: std::ptr::null_mut(),
            n: 0,
        }
    }
}

// ── Callbacks ───────────────────────────────────────────────────────────────

/// `print` del script: `is_end = 0` (valor) o `1` (fin de línea).
pub type clsb_output_cb = unsafe extern "C" fn(ud: *mut c_void, text: *const c_char, is_end: c_int);

/// Resolver de módulos del nodo: escribe el source en `buf` (hasta `buf_len`)
/// y devuelve la longitud (0 = no lo conoce).
pub type clsb_resolver_cb =
    unsafe extern "C" fn(ud: *mut c_void, path: *const c_char, base_dir: *const c_char, buf: *mut c_char, buf_len: usize) -> usize;

/// Función host del nodo: recibe args (const clsb_value*) y escribe el
/// resultado en `out` (con los constructores). Devuelve 0 = ok.
pub type clsb_host_fn = unsafe extern "C" fn(
    ud: *mut c_void,
    id: u32,
    args: *const clsb_value,
    args_len: usize,
    out: *mut clsb_value,
) -> c_int;

// ── Adapters a los traits del motor ─────────────────────────────────────────

struct CSink {
    cb: clsb_output_cb,
    ud: *mut c_void,
}

// El ABI C es single-thread por handle (documentado); los punteros de callback
// solo se usan dentro de las llamadas del motor en el mismo hilo.
unsafe impl Send for CSink {}
unsafe impl Sync for CSink {}

impl cls_jit::OutputSink for CSink {
    fn write(&self, s: &str) {
        let c = CString::new(s).unwrap_or_default();
        unsafe { (self.cb)(self.ud, c.as_ptr(), 0) }
    }
    fn end_line(&self) {
        unsafe { (self.cb)(self.ud, std::ptr::null(), 1) }
    }
}

struct CResolver {
    cb: clsb_resolver_cb,
    ud: *mut c_void,
    buf: Mutex<Vec<u8>>,
}

unsafe impl Send for CResolver {}
unsafe impl Sync for CResolver {}

impl cls_jit::ModuleSourceResolver for CResolver {
    fn resolve_source(&self, path: &str, base_dir: &std::path::Path) -> Option<String> {
        let mut buf = self.buf.lock().unwrap();
        buf.resize(1 << 20, 0); // 1MB reusable
        let path_c = CString::new(path).ok()?;
        let base_c = CString::new(base_dir.to_string_lossy().as_ref()).ok()?;
        let len = unsafe {
            (self.cb)(
                self.ud,
                path_c.as_ptr(),
                base_c.as_ptr(),
                buf.as_mut_ptr() as *mut c_char,
                buf.len(),
            )
        };
        if len == 0 {
            return None;
        }
        let end = len.min(buf.len());
        let src = String::from_utf8_lossy(&buf[..end]).into_owned();
        Some(src)
    }
}

struct CHostCall {
    cb: clsb_host_fn,
    ud: *mut c_void,
}

unsafe impl Send for CHostCall {}
unsafe impl Sync for CHostCall {}

impl cls_jit::HostCallHandler for CHostCall {
    fn call(&self, id: u32, args: &[cls_jit::HostCallArg]) -> Result<cls_jit::HostCallResult, String> {
        // Convertir args a clsb_value temporales.
        let mut tmp: Vec<clsb_value> = Vec::with_capacity(args.len());
        for a in args {
            let mut v = clsb_value::null();
            v.tag = a.tag as i32;
            v.bits = a.bits;
            if let Some(t) = &a.text {
                if let Ok(c) = CString::new(t.as_str()) {
                    v.text = c.into_raw();
                }
            }
            tmp.push(v);
        }
        let mut out = clsb_value::null();
        let rc = unsafe { (self.cb)(self.ud, id, tmp.as_ptr(), tmp.len(), &mut out) };
        // Liberar los temporales (sin tocar `out`).
        for v in &tmp {
            unsafe { clsb_value_free_inner(v) };
        }
        if rc != 0 {
            return Err("la función host devolvió error".into());
        }
        let result = unsafe { value_to_result(&out) };
        unsafe { clsb_value_free_inner(&out) };
        Ok(result)
    }
}

// ── Constructores de valor ──────────────────────────────────────────────────

#[no_mangle]
pub extern "C" fn clsb_value_null() -> clsb_value {
    clsb_value::null()
}

#[no_mangle]
pub extern "C" fn clsb_value_int(v: i64) -> clsb_value {
    let mut x = clsb_value::null();
    x.tag = CLSB_INT;
    x.bits = v;
    x
}

#[no_mangle]
pub extern "C" fn clsb_value_float(v: f64) -> clsb_value {
    let mut x = clsb_value::null();
    x.tag = CLSB_FLOAT;
    x.bits = v.to_bits() as i64;
    x
}

#[no_mangle]
pub extern "C" fn clsb_value_bool(v: c_int) -> clsb_value {
    let mut x = clsb_value::null();
    x.tag = CLSB_BOOL;
    x.bits = if v != 0 { 1 } else { 0 };
    x
}

#[no_mangle]
pub extern "C" fn clsb_value_char(v: u32) -> clsb_value {
    let mut x = clsb_value::null();
    x.tag = CLSB_CHAR;
    x.bits = v as i64;
    x
}

#[no_mangle]
pub extern "C" fn clsb_value_string(s: *const c_char) -> clsb_value {
    let mut x = clsb_value::null();
    x.tag = CLSB_STRING;
    if !s.is_null() {
        if let Ok(c) = unsafe { CStr::from_ptr(s) }.to_owned().into_string() {
            if let Ok(cs) = CString::new(c) {
                x.text = cs.into_raw();
            }
        }
    }
    x
}

#[no_mangle]
pub extern "C" fn clsb_value_array(n: usize) -> clsb_value {
    let mut x = clsb_value::null();
    x.tag = CLSB_ARRAY;
    x.n = n;
    if n > 0 {
        let mut items = Vec::with_capacity(n);
        for _ in 0..n {
            items.push(clsb_value::null());
        }
        x.items = Box::into_raw(items.into_boxed_slice()) as *mut clsb_value;
    }
    x
}

#[no_mangle]
pub extern "C" fn clsb_value_record(n: usize) -> clsb_value {
    let mut x = clsb_value::null();
    x.tag = CLSB_RECORD;
    x.n = n;
    if n > 0 {
        let mut keys: Vec<*const c_char> = Vec::with_capacity(n);
        let mut vals = Vec::with_capacity(n);
        for _ in 0..n {
            keys.push(std::ptr::null());
            vals.push(clsb_value::null());
        }
        x.keys = Box::into_raw(keys.into_boxed_slice()) as *mut *const c_char;
        x.vals = Box::into_raw(vals.into_boxed_slice()) as *mut clsb_value;
    }
    x
}

/// Libera un clsb_value (recursivo). Seguro solo sobre valores construidos por
/// los constructores.
#[no_mangle]
pub unsafe extern "C" fn clsb_value_free(v: *mut clsb_value) {
    if !v.is_null() {
        unsafe { clsb_value_free_inner(&*v) };
    }
}

unsafe fn clsb_value_free_inner(v: &clsb_value) {
    if !v.text.is_null() {
        unsafe { drop(CString::from_raw(v.text as *mut c_char)) };
    }
    if !v.items.is_null() {
        let n = v.n;
        let slice = unsafe { std::slice::from_raw_parts_mut(v.items, n) };
        for item in slice.iter() {
            unsafe { clsb_value_free_inner(item) };
        }
        unsafe { drop(Box::from_raw(slice as *mut [clsb_value])) };
    }
    if !v.keys.is_null() {
        let n = v.n;
        let keys = unsafe { std::slice::from_raw_parts_mut(v.keys, n) };
        for k in keys.iter() {
            if !k.is_null() {
                unsafe { drop(CString::from_raw(*k as *mut c_char)) };
            }
        }
        unsafe { drop(Box::from_raw(keys as *mut [*const c_char])) };
    }
    if !v.vals.is_null() {
        let n = v.n;
        let slice = unsafe { std::slice::from_raw_parts_mut(v.vals, n) };
        for item in slice.iter() {
            unsafe { clsb_value_free_inner(item) };
        }
        unsafe { drop(Box::from_raw(slice as *mut [clsb_value])) };
    }
}

// ── Conversiones C ↔ Rust ───────────────────────────────────────────────────

unsafe fn value_to_cls(v: &clsb_value) -> ClsValue {
    match v.tag {
        CLSB_INT => ClsValue::Int(v.bits),
        CLSB_FLOAT => ClsValue::Float(f64::from_bits(v.bits as u64)),
        CLSB_BOOL => ClsValue::Bool(v.bits != 0),
        CLSB_CHAR => ClsValue::Char(char::from_u32(v.bits as u32).unwrap_or('?')),
        CLSB_STRING => {
            if v.text.is_null() {
                ClsValue::Str(String::new())
            } else {
                let s = unsafe { CStr::from_ptr(v.text) }.to_string_lossy().into_owned();
                ClsValue::Str(s)
            }
        }
        CLSB_ARRAY => {
            let mut items = Vec::with_capacity(v.n);
            if !v.items.is_null() {
                for i in 0..v.n {
                    items.push(unsafe { value_to_cls(&*v.items.add(i)) });
                }
            }
            ClsValue::Array(items)
        }
        CLSB_RECORD => {
            let mut entries = Vec::with_capacity(v.n);
            if !v.keys.is_null() && !v.vals.is_null() {
                for i in 0..v.n {
                    let k = unsafe { *v.keys.add(i) };
                    let key = if k.is_null() {
                        String::new()
                    } else {
                        unsafe { CStr::from_ptr(k) }.to_string_lossy().into_owned()
                    };
                    let val = unsafe { value_to_cls(&*v.vals.add(i)) };
                    entries.push((key, val));
                }
            }
            ClsValue::Record(entries)
        }
        _ => ClsValue::Null,
    }
}

fn cls_to_value(v: ClsValue) -> clsb_value {
    match v {
        ClsValue::Null => clsb_value::null(),
        ClsValue::Int(n) => clsb_value_int(n),
        ClsValue::Float(f) => clsb_value_float(f),
        ClsValue::Bool(b) => clsb_value_bool(if b { 1 } else { 0 }),
        ClsValue::Char(c) => clsb_value_char(c as u32),
        ClsValue::Str(s) => {
            let mut x = clsb_value::null();
            x.tag = CLSB_STRING;
            if let Ok(cs) = CString::new(s) {
                x.text = cs.into_raw();
            }
            x
        }
        ClsValue::Array(items) => {
            let x = clsb_value_array(items.len());
            for (i, item) in items.iter().enumerate() {
                unsafe { *x.items.add(i) = cls_to_value(item.clone()) }; // ok
            }
            x
        }
        ClsValue::Record(entries) => {
            let x = clsb_value_record(entries.len());
            for (i, (k, val)) in entries.iter().enumerate() {
                if let Ok(cs) = CString::new(k.as_str()) {
                    unsafe { *x.keys.add(i) = cs.into_raw() };
                }
                unsafe { *x.vals.add(i) = cls_to_value(val.clone()) };
            }
            x
        }
    }
}

/// Convierte un clsb_value (de un host fn) al resultado del canal host_call.
unsafe fn value_to_result(v: &clsb_value) -> cls_jit::HostCallResult {
    let mut result = cls_jit::HostCallResult {
        tag: v.tag as i64,
        bits: v.bits,
        text: None,
    };
    if v.tag == CLSB_STRING && !v.text.is_null() {
        result.text = Some(unsafe { CStr::from_ptr(v.text) }.to_string_lossy().into_owned());
    }
    result
}

// ── Ciclo de vida del engine ────────────────────────────────────────────────

/// `cfg` puede ser NULL (opciones reservadas para el sandbox futuro).
#[no_mangle]
pub extern "C" fn clsb_engine_new(_cfg: *const clsb_config) -> *mut clsb_engine {
    Box::into_raw(Box::new(clsb_engine {
        inner: Mutex::new(ClsEngine::new()),
    }))
}

#[no_mangle]
pub extern "C" fn clsb_engine_free(e: *mut clsb_engine) {
    if !e.is_null() {
        unsafe { drop(Box::from_raw(e)) };
    }
}

/// Devuelve un clsb_error (owned; liberar con `clsb_error_free`).
unsafe fn error_ptr(e: ClsError) -> *mut clsb_error {
    let trace_c = CString::new(e.trace.clone()).unwrap_or_default();
    Box::into_raw(Box::new(clsb_error { trace_c }))
}

// ── Compilación y ejecución ─────────────────────────────────────────────────

#[no_mangle]
pub extern "C" fn clsb_compile_source(
    e: *mut clsb_engine,
    source: *const c_char,
    name: *const c_char,
    base_dir: *const c_char,
    err_out: *mut *mut clsb_error,
) -> *mut clsb_module {
    let e = match unsafe { e.as_ref() } {
        Some(x) => x,
        None => return std::ptr::null_mut(),
    };
    let source = unsafe { CStr::from_ptr(source) }.to_string_lossy().into_owned();
    let name = if name.is_null() {
        "module".to_string()
    } else {
        unsafe { CStr::from_ptr(name) }.to_string_lossy().into_owned()
    };
    let base = if base_dir.is_null() {
        std::path::PathBuf::from(".")
    } else {
        std::path::PathBuf::from(unsafe { CStr::from_ptr(base_dir) }.to_string_lossy().into_owned())
    };
    let engine = e.inner.lock().unwrap();
    match engine.compile_source(&source, &name, &base) {
        Ok(m) => Box::into_raw(Box::new(clsb_module { inner: m })),
        Err(err) => {
            unsafe { *err_out = error_ptr(err) };
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn clsb_compile_file(
    e: *mut clsb_engine,
    path: *const c_char,
    err_out: *mut *mut clsb_error,
) -> *mut clsb_module {
    let e = match unsafe { e.as_ref() } {
        Some(x) => x,
        None => return std::ptr::null_mut(),
    };
    let path = unsafe { CStr::from_ptr(path) }.to_string_lossy().into_owned();
    let engine = e.inner.lock().unwrap();
    match engine.compile_file(&path) {
        Ok(m) => Box::into_raw(Box::new(clsb_module { inner: m })),
        Err(err) => {
            unsafe { *err_out = error_ptr(err) };
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn clsb_module_free(m: *mut clsb_module) {
    if !m.is_null() {
        unsafe { drop(Box::from_raw(m)) };
    }
}

#[no_mangle]
pub extern "C" fn clsb_run_main(
    m: *mut clsb_module,
    args: *const clsb_value,
    args_len: usize,
    err_out: *mut *mut clsb_error,
) -> i64 {
    let m = match unsafe { m.as_mut() } {
        Some(x) => x,
        None => return -1,
    };
    // main toma String[]: convertir cada valor a String.
    let mut strs: Vec<String> = Vec::with_capacity(args_len);
    if !args.is_null() {
        for i in 0..args_len {
            let v = unsafe { &*args.add(i) };
            let s = match unsafe { value_to_cls(v) } {
                ClsValue::Str(s) => s,
                _ => String::new(),
            };
            strs.push(s);
        }
    }
    match m.inner.run_main(&strs) {
        Ok(code) => code,
        Err(err) => {
            unsafe { *err_out = error_ptr(err) };
            -1
        }
    }
}

#[no_mangle]
pub extern "C" fn clsb_call(
    m: *mut clsb_module,
    name: *const c_char,
    args: *const clsb_value,
    args_len: usize,
    out: *mut clsb_value,
    err_out: *mut *mut clsb_error,
) -> clsb_status {
    let m = match unsafe { m.as_mut() } {
        Some(x) => x,
        None => return 1,
    };
    let name = unsafe { CStr::from_ptr(name) }.to_string_lossy().into_owned();
    let mut vals = Vec::with_capacity(args_len);
    if !args.is_null() {
        for i in 0..args_len {
            vals.push(unsafe { value_to_cls(&*args.add(i)) });
        }
    }
    match m.inner.call(&name, &vals) {
        Ok(v) => {
            if !out.is_null() {
                unsafe { *out = cls_to_value(v) };
            }
            CLSB_OK
        }
        Err(err) => {
            unsafe { *err_out = error_ptr(err) };
            1
        }
    }
}

#[no_mangle]
pub extern "C" fn clsb_eval(
    e: *mut clsb_engine,
    source: *const c_char,
    out: *mut clsb_value,
    err_out: *mut *mut clsb_error,
) -> clsb_status {
    let e = match unsafe { e.as_ref() } {
        Some(x) => x,
        None => return 1,
    };
    let source = unsafe { CStr::from_ptr(source) }.to_string_lossy().into_owned();
    let engine = e.inner.lock().unwrap();
    match engine.eval(&source) {
        Ok(v) => {
            if !out.is_null() {
                unsafe { *out = cls_to_value(v) };
            }
            CLSB_OK
        }
        Err(err) => {
            unsafe { *err_out = error_ptr(err) };
            1
        }
    }
}

// ── SDK de nodo ─────────────────────────────────────────────────────────────

/// Registra la captura de `print` del script.
#[no_mangle]
pub extern "C" fn clsb_set_output(
    e: *mut clsb_engine,
    cb: clsb_output_cb,
    ud: *mut c_void,
) -> clsb_status {
    let e = match unsafe { e.as_ref() } {
        Some(x) => x,
        None => return 1,
    };
    let mut engine = e.inner.lock().unwrap();
    engine.set_output(std::sync::Arc::new(CSink { cb, ud }));
    CLSB_OK
}

/// Registra el resolver de módulos del nodo (import "x" no resuelto en disco).
#[no_mangle]
pub extern "C" fn clsb_set_resolver(
    e: *mut clsb_engine,
    cb: clsb_resolver_cb,
    ud: *mut c_void,
) -> clsb_status {
    let e = match unsafe { e.as_ref() } {
        Some(x) => x,
        None => return 1,
    };
    let resolver = CResolver {
        cb,
        ud,
        buf: Mutex::new(Vec::new()),
    };
    let mut engine = e.inner.lock().unwrap();
    engine.set_module_resolver(std::sync::Arc::new(resolver));
    CLSB_OK
}

/// Registra una función host del nodo: `sig` = ret(params) con códigos
/// `i`=int `f`=float `b`=bool `c`=char `s`=string `v`=void (ej. `"i(i,i)"`).
#[no_mangle]
pub extern "C" fn clsb_register_host_function(
    e: *mut clsb_engine,
    name: *const c_char,
    sig: *const c_char,
    cb: clsb_host_fn,
    ud: *mut c_void,
) -> clsb_status {
    let e = match unsafe { e.as_ref() } {
        Some(x) => x,
        None => return 1,
    };
    let name = unsafe { CStr::from_ptr(name) }.to_string_lossy().into_owned();
    let sig = unsafe { CStr::from_ptr(sig) }.to_string_lossy().into_owned();
    let (params, ret) = match parse_sig(&sig) {
        Ok(p) => p,
        Err(_) => return 1,
    };
    let handler = std::sync::Arc::new(CHostCall { cb, ud });
    let mut engine = e.inner.lock().unwrap();
    engine.register_host_function(&name, params, ret, handler);
    CLSB_OK
}

fn parse_sig(sig: &str) -> Result<(Vec<Type>, Type), ()> {
    let s = sig.trim();
    let open = s.find('(').ok_or(())?;
    let close = s.rfind(')').ok_or(())?;
    let ret_code = s[..open].trim().chars().next().unwrap_or('v');
    let params_src = &s[open + 1..close];
    let mut params = Vec::new();
    for c in params_src.chars().filter(|c| !c.is_whitespace()) {
        params.push(code_type(c).ok_or(())?);
    }
    let ret = code_type(ret_code).ok_or(())?;
    Ok((params, ret))
}

fn code_type(c: char) -> Option<Type> {
    match c {
        'i' => Some(Type::Int),
        'f' => Some(Type::Float),
        'b' => Some(Type::Bool),
        'c' => Some(Type::Char),
        's' => Some(Type::String),
        'v' => Some(Type::Void),
        _ => None,
    }
}

// ── Errores y versión ───────────────────────────────────────────────────────

#[no_mangle]
pub extern "C" fn clsb_error_free(e: *mut clsb_error) {
    if !e.is_null() {
        unsafe { drop(Box::from_raw(e)) };
    }
}

/// Trace completo (vive mientras el clsb_error).
#[no_mangle]
pub extern "C" fn clsb_error_trace(e: *const clsb_error) -> *const c_char {
    match unsafe { e.as_ref() } {
        Some(x) => x.trace_c.as_ptr(),
        None => std::ptr::null(),
    }
}

/// Mensaje limpio (vive mientras el clsb_error).
#[no_mangle]
pub extern "C" fn clsb_error_message(e: *const clsb_error) -> *const c_char {
    match unsafe { e.as_ref() } {
        Some(x) => x.trace_c.as_ptr(),
        None => std::ptr::null(),
    }
}

#[no_mangle]
pub extern "C" fn clsb_version() -> *const c_char {
    b"clsb 2.0-dev1\0".as_ptr() as *const c_char
}

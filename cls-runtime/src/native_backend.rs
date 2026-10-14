//! Backend nativo dinámico para la feature `extension`.
//!
//! Resuelve símbolos por nombre en librerías del sistema (libloading / dlopen /
//! LoadLibrary) -> el usuario final solo escribe CLS (`extension "lib" as C { ... }`),
//! sin tocar el nodo ni salir del lenguaje.
//!
//! ABI: la llamada se emite con **libffi** y la firma real por slot (i64 para
//! enteros/punteros -> registro entero; f64 para floats -> registro XMM/FP); los
//! retornos por su shape exacto (i64/i32/f64/void). Así los parámetros
//! `CDouble` llegan correctamente a C en cualquier SO/arquitectura. El dispatcher
//! soporta hasta `MAX_NATIVE_ARGS` argumentos; arriba del limite, el typeck emite
//! error en compile-time (ver `cls-core/src/middleware/typeck/statements.rs`).
//!
//! Historia: el límite anterior era 4 args (match estático 2^N sobre tipos), que
//! luego se generalizó a 16 args con una firma única `fn(i64,...)` — lo que rompía
//! el paso de `double` por XMM. Se reemplazó por libffi (ABI correcta, sin tablas
//! 2^N). Ver `docs/decisiones/002-ffi-arity-limit.md` y
//! `examples/conformance/findings/02-ffi-f64-args.clsx`.
//!
//! Migracion dev-2 (Fase 6): este archivo se movio de `nodos/clx/src/native.rs`
//! a `cls-runtime/src/native_backend.rs` para que `clxr` (y cualquier
//! futuro nodo) pueda reusar el mismo backend FFI sin duplicar codigo.

use crate::error::{ClsError, ClsResult};
use crate::ffi::{NativeBackend, NativeRet, NativeType};
use crate::value::Value;
use libffi::middle::{Arg, Cif, CodePtr, Type as FfiType};
use libloading::Library;
use std::ffi::{c_void, CString};
use std::os::raw::c_char;
use std::sync::Arc;

/// Caché de librerías nativas cargadas (por path resuelto). Abrir la librería
/// (dlopen/LoadLibrary) en cada llamada es caro; `Library` es `Send + Sync`, y
/// los punteros de símbolo extraídos siguen siendo válidos mientras la librería
/// viva. Nunca se descarga: el proceso la mantiene abierta. Se usa `RwLock`:
/// las lecturas (hot path) no se bloquean entre sí; la escritura ocurre una vez
/// por librería.
static NATIVE_LIBS: std::sync::LazyLock<
    std::sync::RwLock<std::collections::HashMap<String, Arc<Library>>>,
> = std::sync::LazyLock::new(|| std::sync::RwLock::new(std::collections::HashMap::new()));

/// Símbolo nativo resuelto: el puntero (`addr`) se obtiene UNA vez y se
/// reutiliza en cada llamada. `lib` (Arc) mantiene viva la librería mientras el
/// símbolo exista (mismo contrato actual: nunca se descarga).
#[derive(Clone)]
struct NativeSym {
    // Mantiene viva la librería mientras el símbolo exista (nunca se descarga).
    // No se lee: el Arc clonado conserva la `Library` (drop = cerrar librería).
    #[allow(dead_code)]
    lib: Arc<Library>,
    addr: usize,
}

/// Caché de símbolos resueltos, clave = (path resuelto, nombre del símbolo).
/// `lib.get(symbol)` (lookup en la tabla de símbolos) es caro (~50-300 ns); se
/// ejecuta una vez por símbolo. `RwLock`: lecturas concurrentes sin bloqueo.
static NATIVE_SYMS: std::sync::LazyLock<
    std::sync::RwLock<std::collections::HashMap<(String, String), NativeSym>>,
> = std::sync::LazyLock::new(|| {
    std::sync::RwLock::new(std::collections::HashMap::new())
});

/// Obtiene (o resuelve y cachea) el símbolo `symbol` de `lib` (ya resuelta).
/// El `addr` es `usize` (puntero crudo del símbolo): los macros `arityN!` lo
/// reinterpretan por firma (patrón dlsym). Seguro porque la lib nunca se
/// descarga y el Arc clonado la mantiene viva durante la llamada.
fn get_symbol(lib: Arc<Library>, resolved: &str, symbol: &str) -> ClsResult<NativeSym> {
    let key = (resolved.to_string(), symbol.to_string());
    if let Some(s) = NATIVE_SYMS.read().unwrap().get(&key) {
        return Ok(s.clone());
    }
    let sym: libloading::Symbol<'_, unsafe extern "C" fn()> =
        unsafe { lib.get(symbol.as_bytes()) }.map_err(|e| {
            ClsError::RuntimeError(format!(
                "Símbolo nativo '{}' no encontrado en '{}': {}",
                symbol, resolved, e
            ))
        })?;
    let s = NativeSym {
        lib: lib.clone(),
        addr: *sym as usize,
    };
    NATIVE_SYMS.write().unwrap().insert(key, s.clone());
    Ok(s)
}

// ── Shapes de registro del ABI C ─────────────────────────────────────────────

/// Maximo de argumentos en una llamada `extension`. El typeck rechaza arriba de
/// este numero (ver `cls-core/src/middleware/typeck/statements.rs:131-181`).
/// 16 cubre cualquier ABI real (x86_64 SysV: 6 i64 + 8 XMM; Windows x64: 4 i64
/// + 4 XMM; ARM64: 8 registros). Para mas args, empaquetar en un struct.
pub const MAX_NATIVE_ARGS: usize = 16;

#[derive(Clone, Copy, PartialEq)]
enum Shape {
    I64, // enteros de 64 bits y punteros (registro entero)
    F64, // floats de 64 bits (registro XMM)
}

#[derive(Clone, Copy, PartialEq)]
enum RetShape {
    I64,
    I32,
    F64,
    Void,
}

enum CVal {
    I(i64),
    F(f64),
}

fn cval_i(v: &CVal) -> i64 {
    if let CVal::I(x) = v {
        *x
    } else {
        0
    }
}

fn cval_f(v: &CVal) -> f64 {
    if let CVal::F(x) = v {
        *x
    } else {
        0.0
    }
}

enum RawRet {
    I(i64),
    F(f64),
    V,
}

fn arg_shape(nt: &NativeType) -> ClsResult<Shape> {
    match nt {
        NativeType::Any | NativeType::Int | NativeType::CLong | NativeType::CULong
        | NativeType::CString | NativeType::CPtr | NativeType::Struct(_)
        | NativeType::CRecord | NativeType::CArray | NativeType::CStruct
        | NativeType::CInt | NativeType::CUInt | NativeType::CShort | NativeType::CUShort
        | NativeType::CChar | NativeType::CUChar | NativeType::Bool => Ok(Shape::I64),
        NativeType::Float | NativeType::CDouble => Ok(Shape::F64),
        NativeType::CFloat => Err(ClsError::RuntimeError(
            "CFloat (f32) no soportado por el dispatcher de extension (usa CDouble)".to_string(),
        )),
        NativeType::Void => Err(ClsError::RuntimeError(
            "Un argumento no puede ser Void".to_string(),
        )),
    }
}

fn ret_shape(nt: &NativeType) -> ClsResult<RetShape> {
    match nt {
        NativeType::Void => Ok(RetShape::Void),
        NativeType::Float | NativeType::CDouble => Ok(RetShape::F64),
        NativeType::CInt | NativeType::CUInt | NativeType::CShort | NativeType::CUShort
        | NativeType::CChar | NativeType::CUChar | NativeType::Bool => Ok(RetShape::I32),
        _ => Ok(RetShape::I64),
    }
}

fn conv_arg(
    nt: &NativeType,
    value: &Value,
    buffers: &mut Vec<CString>,
    keepalives: &mut Vec<LayoutKeepAlive>,
) -> ClsResult<CVal> {
    match (nt, value) {
        (NativeType::CString, Value::String(s)) => {
            let c = CString::new(s.as_bytes()).map_err(|_| {
                ClsError::RuntimeError("CString: el argumento contiene NUL".to_string())
            })?;
            buffers.push(c);
            Ok(CVal::I(buffers.last().unwrap().as_ptr() as usize as i64))
        }
        (NativeType::CPtr, Value::Int(dir)) | (NativeType::Struct(_), Value::Int(dir)) => {
            Ok(CVal::I(*dir))
        }
        (NativeType::CPtr, Value::Null) | (NativeType::Struct(_), Value::Null) => Ok(CVal::I(0)),
        // CRecord/CArray/CStruct: el valor CLS llega como `Value::Record`/
        // `Value::Array`/`Value::Int(ptr)` (el wrapper del JIT lo leyó de la
        // memoria del módulo). Se serializa a un buffer HOST (el DLL lee/escribe
        // su propio espacio de direcciones; el ptr del layout del WASM no es
        // válido fuera del módulo). `keepalives` mantiene vivos los strings.
        (NativeType::CRecord, Value::Record(map)) => {
            let mut b = HostLayoutBuf::new();
            let ptr = b.write_record(map);
            keepalives.push(b.into_keepalive());
            Ok(CVal::I(ptr as i64))
        }
        (NativeType::CArray, Value::Array(items)) => {
            let mut b = HostLayoutBuf::new();
            let ptr = b.write_array(items);
            keepalives.push(b.into_keepalive());
            Ok(CVal::I(ptr as i64))
        }
        (NativeType::CStruct, Value::Struct(s)) => {
            let mut b = HostLayoutBuf::new();
            let ptr = b.write_struct(&s.fields);
            keepalives.push(b.into_keepalive());
            Ok(CVal::I(ptr as i64))
        }
        (NativeType::CRecord | NativeType::CArray | NativeType::CStruct, Value::Int(ptr)) => {
            Ok(CVal::I(*ptr))
        }
        (NativeType::CRecord | NativeType::CArray | NativeType::CStruct, Value::Null) => {
            Ok(CVal::I(0))
        }
        (NativeType::Bool, Value::Bool(b)) => Ok(CVal::I(if *b { 1 } else { 0 })),
        (NativeType::Bool, Value::Int(v)) => Ok(CVal::I(if *v != 0 { 1 } else { 0 })),
        (NativeType::Float | NativeType::CDouble, Value::Int(v)) => Ok(CVal::F(*v as f64)),
        (_, Value::Int(v)) => Ok(CVal::I(*v)),
        (NativeType::Float | NativeType::CDouble, Value::Float(f)) => Ok(CVal::F(*f)),
        (_, Value::Float(f)) => Ok(CVal::F(*f)),
        _ => Err(ClsError::RuntimeError(format!(
            "No se puede convertir el valor '{}' al tipo nativo '{}'",
            value.type_name(),
            native_type_label(nt)
        ))),
    }
}

fn conv_ret(raw: RawRet, nt: &NativeType) -> ClsResult<Value> {
    match nt {
        NativeType::CString => match raw {
            RawRet::I(ptr) => {
                if ptr == 0 {
                    return Ok(Value::String(String::new()));
                }
                let s = unsafe {
                    std::ffi::CStr::from_ptr(ptr as *const c_char)
                        .to_string_lossy()
                        .into_owned()
                };
                Ok(Value::String(s))
            }
            _ => Ok(Value::Null),
        },
        NativeType::Float | NativeType::CDouble => match raw {
            RawRet::F(f) => Ok(Value::Float(f)),
            _ => Ok(Value::Float(0.0)),
        },
        NativeType::CRecord | NativeType::CArray | NativeType::CStruct => match raw {
            // El retorno es el ptr HOST al layout (el DLL escribió in-place en
            // la memoria del módulo, que es una alocación del host). Se devuelve
            // el ptr crudo; el wrapper del JIT lo traduce host -> offset wasm y
            // CLS lo usa como su record/array (cero copias). Para el tree-walker
            // (sin memoria del módulo) queda como Int crudo (documentado).
            RawRet::I(ptr) => Ok(Value::Int(ptr)),
            _ => Ok(Value::Int(0)),
        },
        NativeType::Void => Ok(Value::Void),
        _ => match raw {
            RawRet::I(v) => Ok(Value::Int(v)),
            _ => Ok(Value::Int(0)),
        },
    }
}

fn native_type_label(nt: &NativeType) -> String {
    format!("{:?}", nt)
}

// ── Buffer host para el layout de valores estructurados del FFI ─────────────

/// Serializa/deserializa el layout canónico de CLS (string packed, array
/// `[cap][len][elems*8]`, record `[cap][len][(key,val,tag)*24]`, struct
/// contiguo) en un buffer HOST.
///
/// Para **args** (tree-walker): serializa un `Value` a un buffer propio
/// (`base` = dirección host estable) que se mantiene vivo durante la llamada.
struct HostLayoutBuf {
    base: usize,
    owned: Vec<u8>,
    strings: Vec<Vec<u8>>,
}

impl HostLayoutBuf {
    fn new() -> Self {
        Self {
            base: 0,
            owned: Vec::new(),
            strings: Vec::new(),
        }
    }

    fn ensure_owned(&mut self, size: usize) {
        if self.owned.is_empty() {
            self.owned = vec![0u8; size];
            self.base = self.owned.as_ptr() as usize;
        }
    }

    /// Mantiene vivos los buffers y strings del layout durante la llamada.
    fn into_keepalive(self) -> LayoutKeepAlive {
        LayoutKeepAlive {
            owned: self.owned,
            strings: self.strings,
        }
    }

    /// Aloca un string en el heap propio (el ptr es estable aunque `owned` o
    /// `strings` se reasigne) y devuelve su packed `(ptr<<32)|len`.
    fn write_str(&mut self, s: &str) -> i64 {
        let buf = s.as_bytes().to_vec();
        let ptr = buf.as_ptr() as usize;
        self.strings.push(buf);
        ((ptr as i64) << 32) | (s.len() as i64)
    }

    fn write_i64(&mut self, addr: usize, v: i64) {
        if addr + 8 <= self.owned.len() {
            self.owned[addr..addr + 8].copy_from_slice(&v.to_le_bytes());
        }
    }

    fn write_scalar(&mut self, v: &Value) -> i64 {
        match v {
            Value::Int(n) => *n,
            Value::Float(f) => f.to_bits() as i64,
            Value::Bool(b) => {
                if *b {
                    1
                } else {
                    0
                }
            }
            Value::Char(c) => *c as i64,
            Value::String(s) => self.write_str(s),
            Value::Array(items) => self.write_array(items) as i64,
            Value::Tuple(items) => self.write_array(items) as i64,
            Value::Record(map) => self.write_record(map) as i64,
            Value::Struct(s) => self.write_struct(&s.fields) as i64,
            Value::Null | Value::Void => 0,
            _ => 0,
        }
    }

    /// Array `[cap][len][elems*8]`.
    fn write_array(&mut self, items: &[Value]) -> usize {
        self.ensure_owned(items.len() * 8 + 16);
        self.write_i64(0, items.len() as i64);
        self.write_i64(8, items.len() as i64);
        for (i, it) in items.iter().enumerate() {
            let bits = self.write_scalar(it);
            self.write_i64(16 + i * 8, bits);
        }
        self.base
    }

    /// Record `[cap][len][(key,val,tag)*24]` (tags runtime).
    fn write_record(&mut self, map: &std::collections::HashMap<String, Value>) -> usize {
        self.ensure_owned(map.len() * 24 + 16);
        self.write_i64(0, map.len() as i64);
        self.write_i64(8, map.len() as i64);
        let mut i = 0usize;
        let mut keys: Vec<&String> = map.keys().collect();
        keys.sort();
        for k in keys {
            let key = self.write_str(k);
            let bits = self.write_scalar(&map[k]);
            let tag = Self::runtime_tag(&map[k]);
            let base = 16 + i * 24;
            self.write_i64(base, key);
            self.write_i64(base + 8, bits);
            self.write_i64(base + 16, tag);
            i += 1;
        }
        self.base
    }

    /// Struct: layout contiguo de campos (cada uno su representación i64).
    fn write_struct(&mut self, fields: &[Value]) -> usize {
        self.ensure_owned(fields.len() * 8);
        for (i, f) in fields.iter().enumerate() {
            let bits = self.write_scalar(f);
            self.write_i64(i * 8, bits);
        }
        self.base
    }

    fn runtime_tag(v: &Value) -> i64 {
        match v {
            Value::Int(_) => 0,
            Value::String(_) => 1,
            Value::Float(_) => 2,
            Value::Bool(_) => 3,
            Value::Char(_) => 4,
            Value::Array(_) | Value::Tuple(_) => 6,
            Value::Record(_) | Value::Struct(_) => 7,
            Value::Null | Value::Void => 0,
            _ => 8,
        }
    }
}

/// Mantiene vivos los buffers y strings de un layout host durante la llamada.
struct LayoutKeepAlive {
    #[allow(dead_code)]
    owned: Vec<u8>,
    #[allow(dead_code)]
    strings: Vec<Vec<u8>>,
}

// ── Resolución de nombres de librería ───────────────────────────────────────

fn resolve_library(name: &str) -> String {
    #[cfg(target_os = "windows")]
    let map: &[(&str, &str)] = &[
        ("libc", "msvcrt.dll"),
        ("c", "msvcrt.dll"),
        ("libm", "msvcrt.dll"),
        ("m", "msvcrt.dll"),
    ];
    #[cfg(all(unix, not(target_os = "macos")))]
    let map: &[(&str, &str)] = &[
        ("libc", "libc.so.6"),
        ("c", "libc.so.6"),
        ("libm", "libm.so.6"),
        ("m", "libm.so.6"),
    ];
    #[cfg(target_os = "macos")]
    let map: &[(&str, &str)] = &[
        ("libc", "libSystem.B.dylib"),
        ("c", "libSystem.B.dylib"),
        ("libm", "libSystem.B.dylib"),
        ("m", "libSystem.B.dylib"),
    ];
    if let Some((_, real)) = map.iter().find(|(k, _)| *k == name) {
        return real.to_string();
    }
    name.to_string()
}

// ── Invocación dinámica (ABI C real por slot) ────────────────────────────────
//
// Se usa `libffi` para emitir la llamada con la **firma ABI correcta por
// argumento**: cada slot se declara con su tipo C real (`i64` -> registro
// entero, `f64` -> registro XMM/FP). El dispatcher anterior casteaba el símbolo
// a una única firma `extern "C" fn(i64, ..., i64)`: TODOS los argumentos
// viajaban como enteros, así que un `double` de C nunca llegaba a su registro
// de punto flotante y el nativo leía basura (ver
// `examples/conformance/findings/02-ffi-f64-args.clsx`). libffi respeta la ABI
// del SO (SysV/Windows x64/ARM64) sin tablas 2^N ni reimplementar el
// marshalling de registros a mano.

/// Invoca `symbol` de `library` con los argumentos y la firma declarada en el
/// `extension`, devolviendo el retorno crudo (`RawRet`). Los `CString`/layouts
/// viven en `buffers`/`keepalives` durante la llamada.
fn invoke(
    library: &str,
    symbol: &str,
    args: &[Value],
    param_types: &[NativeType],
    ret: &NativeType,
) -> ClsResult<RawRet> {
    if args.len() > MAX_NATIVE_ARGS {
        return Err(ClsError::RuntimeError(format!(
            "La función nativa '{}' tiene {} argumentos: el dispatcher de extension soporta hasta {}. Para más args, empaquétalos en un struct y pasa un puntero.",
            symbol, args.len(), MAX_NATIVE_ARGS
        )));
    }
    let resolved = resolve_library(library);
    // Librería cacheada por path (get-or-insert): no re-abrir dlopen en cada
    // llamada. El `Arc` clonado mantiene la librería viva durante la llamada,
    // así el puntero del símbolo extraído es válido. `RwLock`: doble-check tras
    // adquirir write (otro hilo pudo insertar).
    let lib = {
        let cache = NATIVE_LIBS.read().unwrap();
        match cache.get(&resolved) {
            Some(l) => l.clone(),
            None => {
                drop(cache);
                let mut w = NATIVE_LIBS.write().unwrap();
                match w.get(&resolved) {
                    Some(l) => l.clone(),
                    None => {
                        let l = Arc::new(unsafe { Library::new(&resolved) }.map_err(|e| {
                            ClsError::RuntimeError(format!(
                                "No se pudo cargar la librería nativa '{}' (resuelta a '{}'): {}",
                                library, resolved, e
                            ))
                        })?);
                        w.insert(resolved.clone(), l.clone());
                        l
                    }
                }
            }
        }
    };

    // Símbolo como puntero crudo, cacheado (get-or-insert). La firma la fija
    // libffi en cada llamada a partir de los tipos declarados.
    let sym = get_symbol(lib, &resolved, symbol)?;
    let base = sym.addr;

    // Convertir args -> valores tipados y construir la firma libffi por slot.
    // Los i64 y los f64 se guardan en almacenes SEPARADOS y estables (los
    // `Arg` de libffi son punteros a esos valores).
    let mut buffers: Vec<CString> = Vec::new();
    let mut keepalives: Vec<LayoutKeepAlive> = Vec::new();
    let mut ffi_types: Vec<FfiType> = Vec::with_capacity(args.len());
    let mut ints: Vec<i64> = Vec::with_capacity(args.len());
    let mut floats: Vec<f64> = Vec::with_capacity(args.len());
    enum Slot {
        Int(usize),
        Float(usize),
    }
    let mut slots: Vec<Slot> = Vec::with_capacity(args.len());
    for (i, arg) in args.iter().enumerate() {
        let nt = param_types.get(i).cloned().unwrap_or(NativeType::Any);
        let cval = conv_arg(&nt, arg, &mut buffers, &mut keepalives)?;
        match arg_shape(&nt)? {
            Shape::I64 => {
                ffi_types.push(FfiType::i64());
                slots.push(Slot::Int(ints.len()));
                ints.push(cval_i(&cval));
            }
            Shape::F64 => {
                ffi_types.push(FfiType::f64());
                slots.push(Slot::Float(floats.len()));
                floats.push(cval_f(&cval));
            }
        }
    }
    let rshape = ret_shape(ret)?;
    let ffi_ret = match rshape {
        RetShape::I64 => FfiType::i64(),
        RetShape::I32 => FfiType::i32(),
        RetShape::F64 => FfiType::f64(),
        RetShape::Void => FfiType::void(),
    };
    let cif = Cif::new(ffi_types.into_iter(), ffi_ret);
    let ffi_args: Vec<Arg> = slots
        .iter()
        .map(|s| match s {
            Slot::Int(i) => Arg::new(&ints[*i]),
            Slot::Float(i) => Arg::new(&floats[*i]),
        })
        .collect();
    let code = CodePtr(base as *mut c_void);
    let raw = unsafe {
        match rshape {
            RetShape::I64 => RawRet::I(cif.call::<i64>(code, &ffi_args)),
            RetShape::I32 => RawRet::I(cif.call::<i32>(code, &ffi_args) as i64),
            RetShape::F64 => RawRet::F(cif.call::<f64>(code, &ffi_args)),
            RetShape::Void => {
                cif.call::<()>(code, &ffi_args);
                RawRet::V
            }
        }
    };
    Ok(raw)
}

/// Copia los bytes de un `char*` de C (hasta el primer NUL, semántica C) **sin
/// reinterpretar UTF-8** (semántica de bytes de CLS): no normaliza a U+FFFD.
/// `ptr == 0` -> vacío.
fn cstr_to_bytes(ptr: i64) -> Vec<u8> {
    if ptr == 0 {
        return Vec::new();
    }
    unsafe { std::ffi::CStr::from_ptr(ptr as *const c_char).to_bytes().to_vec() }
}

// ── DynamicBackend ──────────────────────────────────────────────────────────

/// Backend `C` que resuelve símbolos por nombre en librerías del sistema.
#[derive(Default)]
pub struct DynamicBackend;

impl NativeBackend for DynamicBackend {
    fn call_function(
        &self,
        library: &str,
        symbol: &str,
        args: &[Value],
        param_types: &[NativeType],
        ret: NativeType,
    ) -> ClsResult<Value> {
        let raw = invoke(library, symbol, args, param_types, &ret)?;
        conv_ret(raw, &ret)
    }

    fn call_function_raw(
        &self,
        library: &str,
        symbol: &str,
        args: &[Value],
        param_types: &[NativeType],
        ret: NativeType,
    ) -> ClsResult<NativeRet> {
        let raw = invoke(library, symbol, args, param_types, &ret)?;
        // Retorno `CString`: bytes crudos (byte-transparente). El corte en NUL
        // es semántica C válida; lo que NO se hace es reemplazar bytes no-UTF-8
        // por U+FFFD.
        if matches!(ret, NativeType::CString) {
            if let RawRet::I(ptr) = raw {
                return Ok(NativeRet::Bytes(cstr_to_bytes(ptr)));
            }
        }
        conv_ret(raw, &ret).map(NativeRet::Value)
    }

    fn get_variable(&self, library: &str, name: &str, _ty: NativeType) -> ClsResult<Value> {
        Err(ClsError::RuntimeError(format!(
            "Variable nativa '{}' de '{}' no soportada por el backend C (usa get_/set_ via función nativa)",
            name, library
        )))
    }

    fn set_variable(&self, library: &str, name: &str, _ty: NativeType, _value: &Value) -> ClsResult<()> {
        Err(ClsError::RuntimeError(format!(
            "Variable nativa '{}' de '{}' no soportada por el backend C (usa get_/set_ via función nativa)",
            name, library
        )))
    }
}

//! Backend nativo dinámico para la feature `extension` del nodo clx.
//!
//! Resuelve símbolos por nombre en librerías del sistema (libloading / dlopen /
//! LoadLibrary) -> el usuario final solo escribe CLS (`extension "lib" as C { ... }`),
//! sin tocar el nodo ni salir del lenguaje.
//!
//! ABI: los argumentos se pasan por registros (i64 para enteros/punteros, f64 para
//! floats); los retornos por su shape exacto (i64/i32/f64/void). El dispatcher
//! soporta hasta `MAX_NATIVE_ARGS` argumentos. La macro `call_n_typed::<N>`
//! despacha por cantidad (N lineal, no 2^N); arriba del limite, el typeck emite
//! error en compile-time (ver `cls-core/src/middleware/typeck/statements.rs`).
//!
//! Migracion dev-2: el limite anterior era 4 args (con un match estatico 2^N
//! sobre tipos). Se reemplazo por un dispatch generico sobre N que acepta
//! cualquier cantidad hasta MAX_NATIVE_ARGS. Ver
//! `docs/decisiones/002-ffi-arity-limit.md`.

use cls_core::error::{ClsError, ClsResult};
use cls_runtime::ffi::{NativeBackend, NativeType};
use cls_runtime::value::Value;
use libloading::Library;
use std::ffi::CString;
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
pub(crate) const MAX_NATIVE_ARGS: usize = 16;

#[derive(Clone, Copy, PartialEq)]
enum Shape {
    I64, // enteros de 64 bits y punteros (registro entero)
    F64, // floats de 64 bits (registro XMM)
}

/// Argumento crudo en registro C, sin tipo: i64 o f64. El dispatch por
/// combinacion de tipos (que era 2^N en el codigo viejo) se elimina: cada
/// arg se convierte a este enum, y la llamada se emite con la firma
/// `unsafe extern "C" fn(i64, i64, ..., i64) -> ...` donde los f64 van
/// reinterpretados como i64 (los registros XMM aceptan tanto i64 como f64
/// via `f64::from_bits` en la frontera).
#[derive(Clone, Copy)]
enum RawArg {
    I(i64),
    F(f64),
}

impl RawArg {
    /// Bits que van al registro (XMM o GP). Para I64, los bits directos;
    /// para F64, los bits de f64 (la firma C destino los reinterpreta segun
    /// el tipo declarado en el `extension`).
    fn bits(self) -> i64 {
        match self {
            RawArg::I(x) => x,
            RawArg::F(x) => x.to_bits() as i64,
        }
    }
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

// ── Dispatch dinamico (dev-2) ────────────────────────────────────────────────
//
// Reemplaza los macros arity0..arity4 (que eran 1+2+4+8+16=31 arms en total,
// con 2^N combinaciones de tipos). Ahora hay UN solo macro `call_n_typed::<N>`
// que se especializa por la cantidad N (lineal, 17 arms para 0..16) y
// desempaqueta los RawArg (cada uno es i64 con los bits correctos del f64 si
// aplica). El ABI hace el resto: en x86_64 SysV los i64 van en registros
// GP (RDI/RSI/RDX/RCX/R8/R9) y los f64 en registros XMM; el layout
// del stack es el mismo para ambos (8 bytes por slot). Usamos una firma
// unica `unsafe extern "C" fn(i64, i64, ..., i64) -> i64` y el
// `transmute` del addr del simbolo a esa firma es seguro mientras coincida
// con la declaracion del `extension` (contrato del usuario).

/// Dispatch dinamico (dev-2) ─────────────────────────────────────────────────
//
// Reemplaza los macros arity0..arity4 (que eran 1+2+4+8+16=31 arms con 2^N
// combinaciones de tipos). Ahora hay UNA funcion nombrada por cantidad
// (`call_typed_0` ... `call_typed_16`) que castea `base` a la firma C con N
// argumentos i64, desempaqueta los RawArg como bits, y llama.
//
// En x86_64 SysV los i64 van en registros GP (RDI/RSI/RDX/RCX/R8/R9) y los
// f64 en registros XMM; el stack frame es el mismo para ambos. Para N>8 los
// ultimos args van por stack (mismo layout). El transmute es seguro mientras
// la firma coincida con la declaracion del `extension` (contrato del usuario).
//
// Por que funciones nombradas y no un macro expandido: Rust estable no permite
// forward-ear un literal numerico entre macros (`$n:literal` no es comparable
// en macro_rules), asi que el `match args.len()` en el call site elige la
// funcion concreta.

// Implementacion directa: una funcion por cantidad.
fn call_typed_0(base: usize, ret: RetShape) -> RawRet {
    match ret {
        RetShape::I64 => {
            let f: unsafe extern "C" fn() -> i64 = unsafe { std::mem::transmute(base) };
            RawRet::I(unsafe { f() })
        }
        RetShape::I32 => {
            let f: unsafe extern "C" fn() -> i32 = unsafe { std::mem::transmute(base) };
            RawRet::I(unsafe { f() as i64 })
        }
        RetShape::F64 => {
            let f: unsafe extern "C" fn() -> f64 = unsafe { std::mem::transmute(base) };
            RawRet::F(unsafe { f() })
        }
        RetShape::Void => {
            let f: unsafe extern "C" fn() = unsafe { std::mem::transmute(base) };
            unsafe { f() };
            RawRet::V
        }
    }
}

fn call_typed_1(base: usize, ret: RetShape, a0: RawArg) -> RawRet {
    let a0 = a0.bits();
    match ret {
        RetShape::I64 => {
            let f: unsafe extern "C" fn(i64) -> i64 = unsafe { std::mem::transmute(base) };
            RawRet::I(unsafe { f(a0) })
        }
        RetShape::I32 => {
            let f: unsafe extern "C" fn(i64) -> i32 = unsafe { std::mem::transmute(base) };
            RawRet::I(unsafe { f(a0) as i64 })
        }
        RetShape::F64 => {
            let f: unsafe extern "C" fn(i64) -> f64 = unsafe { std::mem::transmute(base) };
            RawRet::F(unsafe { f(a0) })
        }
        RetShape::Void => {
            let f: unsafe extern "C" fn(i64) = unsafe { std::mem::transmute(base) };
            unsafe { f(a0) };
            RawRet::V
        }
    }
}

fn call_typed_2(base: usize, ret: RetShape, a0: RawArg, a1: RawArg) -> RawRet {
    let a0 = a0.bits();
    let a1 = a1.bits();
    match ret {
        RetShape::I64 => {
            let f: unsafe extern "C" fn(i64, i64) -> i64 = unsafe { std::mem::transmute(base) };
            RawRet::I(unsafe { f(a0, a1) })
        }
        RetShape::I32 => {
            let f: unsafe extern "C" fn(i64, i64) -> i32 = unsafe { std::mem::transmute(base) };
            RawRet::I(unsafe { f(a0, a1) as i64 })
        }
        RetShape::F64 => {
            let f: unsafe extern "C" fn(i64, i64) -> f64 = unsafe { std::mem::transmute(base) };
            RawRet::F(unsafe { f(a0, a1) })
        }
        RetShape::Void => {
            let f: unsafe extern "C" fn(i64, i64) = unsafe { std::mem::transmute(base) };
            unsafe { f(a0, a1) };
            RawRet::V
        }
    }
}

fn call_typed_3(base: usize, ret: RetShape, a0: RawArg, a1: RawArg, a2: RawArg) -> RawRet {
    let a0 = a0.bits();
    let a1 = a1.bits();
    let a2 = a2.bits();
    match ret {
        RetShape::I64 => {
            let f: unsafe extern "C" fn(i64, i64, i64) -> i64 = unsafe { std::mem::transmute(base) };
            RawRet::I(unsafe { f(a0, a1, a2) })
        }
        RetShape::I32 => {
            let f: unsafe extern "C" fn(i64, i64, i64) -> i32 = unsafe { std::mem::transmute(base) };
            RawRet::I(unsafe { f(a0, a1, a2) as i64 })
        }
        RetShape::F64 => {
            let f: unsafe extern "C" fn(i64, i64, i64) -> f64 = unsafe { std::mem::transmute(base) };
            RawRet::F(unsafe { f(a0, a1, a2) })
        }
        RetShape::Void => {
            let f: unsafe extern "C" fn(i64, i64, i64) = unsafe { std::mem::transmute(base) };
            unsafe { f(a0, a1, a2) };
            RawRet::V
        }
    }
}

fn call_typed_4(base: usize, ret: RetShape, a0: RawArg, a1: RawArg, a2: RawArg, a3: RawArg) -> RawRet {
    let a0 = a0.bits();
    let a1 = a1.bits();
    let a2 = a2.bits();
    let a3 = a3.bits();
    match ret {
        RetShape::I64 => {
            let f: unsafe extern "C" fn(i64, i64, i64, i64) -> i64 = unsafe { std::mem::transmute(base) };
            RawRet::I(unsafe { f(a0, a1, a2, a3) })
        }
        RetShape::I32 => {
            let f: unsafe extern "C" fn(i64, i64, i64, i64) -> i32 = unsafe { std::mem::transmute(base) };
            RawRet::I(unsafe { f(a0, a1, a2, a3) as i64 })
        }
        RetShape::F64 => {
            let f: unsafe extern "C" fn(i64, i64, i64, i64) -> f64 = unsafe { std::mem::transmute(base) };
            RawRet::F(unsafe { f(a0, a1, a2, a3) })
        }
        RetShape::Void => {
            let f: unsafe extern "C" fn(i64, i64, i64, i64) = unsafe { std::mem::transmute(base) };
            unsafe { f(a0, a1, a2, a3) };
            RawRet::V
        }
    }
}

fn call_typed_5(base: usize, ret: RetShape, a0: RawArg, a1: RawArg, a2: RawArg, a3: RawArg, a4: RawArg) -> RawRet {
    let a0 = a0.bits();
    let a1 = a1.bits();
    let a2 = a2.bits();
    let a3 = a3.bits();
    let a4 = a4.bits();
    match ret {
        RetShape::I64 => {
            let f: unsafe extern "C" fn(i64, i64, i64, i64, i64) -> i64 = unsafe { std::mem::transmute(base) };
            RawRet::I(unsafe { f(a0, a1, a2, a3, a4) })
        }
        RetShape::F64 => {
            let f: unsafe extern "C" fn(i64, i64, i64, i64, i64) -> f64 = unsafe { std::mem::transmute(base) };
            RawRet::F(unsafe { f(a0, a1, a2, a3, a4) })
        }
        RetShape::Void => {
            let f: unsafe extern "C" fn(i64, i64, i64, i64, i64) = unsafe { std::mem::transmute(base) };
            unsafe { f(a0, a1, a2, a3, a4) };
            RawRet::V
        }
        _ => panic!("FFI: RetShape::I32 con 5 args no soportado"),
    }
}

fn call_typed_6(base: usize, ret: RetShape, a0: RawArg, a1: RawArg, a2: RawArg, a3: RawArg, a4: RawArg, a5: RawArg) -> RawRet {
    let a0 = a0.bits();
    let a1 = a1.bits();
    let a2 = a2.bits();
    let a3 = a3.bits();
    let a4 = a4.bits();
    let a5 = a5.bits();
    match ret {
        RetShape::I64 => {
            let f: unsafe extern "C" fn(i64, i64, i64, i64, i64, i64) -> i64 = unsafe { std::mem::transmute(base) };
            RawRet::I(unsafe { f(a0, a1, a2, a3, a4, a5) })
        }
        RetShape::F64 => {
            let f: unsafe extern "C" fn(i64, i64, i64, i64, i64, i64) -> f64 = unsafe { std::mem::transmute(base) };
            RawRet::F(unsafe { f(a0, a1, a2, a3, a4, a5) })
        }
        RetShape::Void => {
            let f: unsafe extern "C" fn(i64, i64, i64, i64, i64, i64) = unsafe { std::mem::transmute(base) };
            unsafe { f(a0, a1, a2, a3, a4, a5) };
            RawRet::V
        }
        _ => panic!("FFI: RetShape::I32 con 6 args no soportado"),
    }
}

fn call_typed_7(base: usize, ret: RetShape, a0: RawArg, a1: RawArg, a2: RawArg, a3: RawArg, a4: RawArg, a5: RawArg, a6: RawArg) -> RawRet {
    let a0 = a0.bits();
    let a1 = a1.bits();
    let a2 = a2.bits();
    let a3 = a3.bits();
    let a4 = a4.bits();
    let a5 = a5.bits();
    let a6 = a6.bits();
    match ret {
        RetShape::I64 => {
            let f: unsafe extern "C" fn(i64, i64, i64, i64, i64, i64, i64) -> i64 = unsafe { std::mem::transmute(base) };
            RawRet::I(unsafe { f(a0, a1, a2, a3, a4, a5, a6) })
        }
        RetShape::F64 => {
            let f: unsafe extern "C" fn(i64, i64, i64, i64, i64, i64, i64) -> f64 = unsafe { std::mem::transmute(base) };
            RawRet::F(unsafe { f(a0, a1, a2, a3, a4, a5, a6) })
        }
        RetShape::Void => {
            let f: unsafe extern "C" fn(i64, i64, i64, i64, i64, i64, i64) = unsafe { std::mem::transmute(base) };
            unsafe { f(a0, a1, a2, a3, a4, a5, a6) };
            RawRet::V
        }
        _ => panic!("FFI: RetShape::I32 con 7 args no soportado"),
    }
}

fn call_typed_8(base: usize, ret: RetShape, a0: RawArg, a1: RawArg, a2: RawArg, a3: RawArg, a4: RawArg, a5: RawArg, a6: RawArg, a7: RawArg) -> RawRet {
    let a0 = a0.bits();
    let a1 = a1.bits();
    let a2 = a2.bits();
    let a3 = a3.bits();
    let a4 = a4.bits();
    let a5 = a5.bits();
    let a6 = a6.bits();
    let a7 = a7.bits();
    match ret {
        RetShape::I64 => {
            let f: unsafe extern "C" fn(i64, i64, i64, i64, i64, i64, i64, i64) -> i64 = unsafe { std::mem::transmute(base) };
            RawRet::I(unsafe { f(a0, a1, a2, a3, a4, a5, a6, a7) })
        }
        RetShape::F64 => {
            let f: unsafe extern "C" fn(i64, i64, i64, i64, i64, i64, i64, i64) -> f64 = unsafe { std::mem::transmute(base) };
            RawRet::F(unsafe { f(a0, a1, a2, a3, a4, a5, a6, a7) })
        }
        RetShape::Void => {
            let f: unsafe extern "C" fn(i64, i64, i64, i64, i64, i64, i64, i64) = unsafe { std::mem::transmute(base) };
            unsafe { f(a0, a1, a2, a3, a4, a5, a6, a7) };
            RawRet::V
        }
        _ => panic!("FFI: RetShape::I32 con 8 args no soportado"),
    }
}

fn call_typed_9(base: usize, ret: RetShape, a0: RawArg, a1: RawArg, a2: RawArg, a3: RawArg, a4: RawArg, a5: RawArg, a6: RawArg, a7: RawArg, a8: RawArg) -> RawRet {
    let a0 = a0.bits(); let a1 = a1.bits();
    let a2 = a2.bits(); let a3 = a3.bits();
    let a4 = a4.bits(); let a5 = a5.bits();
    let a6 = a6.bits(); let a7 = a7.bits();
    let a8 = a8.bits();
    match ret {
        RetShape::I64 => {
            let f: unsafe extern "C" fn(i64, i64, i64, i64, i64, i64, i64, i64, i64) -> i64 = unsafe { std::mem::transmute(base) };
            RawRet::I(unsafe { f(a0, a1, a2, a3, a4, a5, a6, a7, a8) })
        }
        RetShape::Void => {
            let f: unsafe extern "C" fn(i64, i64, i64, i64, i64, i64, i64, i64, i64) = unsafe { std::mem::transmute(base) };
            unsafe { f(a0, a1, a2, a3, a4, a5, a6, a7, a8) };
            RawRet::V
        }
        _ => panic!("FFI: RetShape::F64/I32 con 9 args no soportado"),
    }
}

fn call_typed_10(base: usize, ret: RetShape, a0: RawArg, a1: RawArg, a2: RawArg, a3: RawArg, a4: RawArg, a5: RawArg, a6: RawArg, a7: RawArg, a8: RawArg, a9: RawArg) -> RawRet {
    let a0 = a0.bits(); let a1 = a1.bits();
    let a2 = a2.bits(); let a3 = a3.bits();
    let a4 = a4.bits(); let a5 = a5.bits();
    let a6 = a6.bits(); let a7 = a7.bits();
    let a8 = a8.bits(); let a9 = a9.bits();
    match ret {
        RetShape::I64 => {
            let f: unsafe extern "C" fn(i64, i64, i64, i64, i64, i64, i64, i64, i64, i64) -> i64 = unsafe { std::mem::transmute(base) };
            RawRet::I(unsafe { f(a0, a1, a2, a3, a4, a5, a6, a7, a8, a9) })
        }
        RetShape::Void => {
            let f: unsafe extern "C" fn(i64, i64, i64, i64, i64, i64, i64, i64, i64, i64) = unsafe { std::mem::transmute(base) };
            unsafe { f(a0, a1, a2, a3, a4, a5, a6, a7, a8, a9) };
            RawRet::V
        }
        _ => panic!("FFI: RetShape::F64/I32 con 10 args no soportado"),
    }
}

fn call_typed_11(base: usize, ret: RetShape, a0: RawArg, a1: RawArg, a2: RawArg, a3: RawArg, a4: RawArg, a5: RawArg, a6: RawArg, a7: RawArg, a8: RawArg, a9: RawArg, a10: RawArg) -> RawRet {
    let a0 = a0.bits(); let a1 = a1.bits();
    let a2 = a2.bits(); let a3 = a3.bits();
    let a4 = a4.bits(); let a5 = a5.bits();
    let a6 = a6.bits(); let a7 = a7.bits();
    let a8 = a8.bits(); let a9 = a9.bits();
    let a10 = a10.bits();
    match ret {
        RetShape::I64 => {
            let f: unsafe extern "C" fn(i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64) -> i64 = unsafe { std::mem::transmute(base) };
            RawRet::I(unsafe { f(a0, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10) })
        }
        RetShape::Void => {
            let f: unsafe extern "C" fn(i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64) = unsafe { std::mem::transmute(base) };
            unsafe { f(a0, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10) };
            RawRet::V
        }
        _ => panic!("FFI: RetShape::F64/I32 con 11 args no soportado"),
    }
}

fn call_typed_12(base: usize, ret: RetShape, a0: RawArg, a1: RawArg, a2: RawArg, a3: RawArg, a4: RawArg, a5: RawArg, a6: RawArg, a7: RawArg, a8: RawArg, a9: RawArg, a10: RawArg, a11: RawArg) -> RawRet {
    let a0 = a0.bits(); let a1 = a1.bits();
    let a2 = a2.bits(); let a3 = a3.bits();
    let a4 = a4.bits(); let a5 = a5.bits();
    let a6 = a6.bits(); let a7 = a7.bits();
    let a8 = a8.bits(); let a9 = a9.bits();
    let a10 = a10.bits(); let a11 = a11.bits();
    match ret {
        RetShape::I64 => {
            let f: unsafe extern "C" fn(i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64) -> i64 = unsafe { std::mem::transmute(base) };
            RawRet::I(unsafe { f(a0, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11) })
        }
        RetShape::Void => {
            let f: unsafe extern "C" fn(i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64) = unsafe { std::mem::transmute(base) };
            unsafe { f(a0, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11) };
            RawRet::V
        }
        _ => panic!("FFI: RetShape::F64/I32 con 12 args no soportado"),
    }
}

fn call_typed_13(base: usize, ret: RetShape, a0: RawArg, a1: RawArg, a2: RawArg, a3: RawArg, a4: RawArg, a5: RawArg, a6: RawArg, a7: RawArg, a8: RawArg, a9: RawArg, a10: RawArg, a11: RawArg, a12: RawArg) -> RawRet {
    let a0 = a0.bits(); let a1 = a1.bits();
    let a2 = a2.bits(); let a3 = a3.bits();
    let a4 = a4.bits(); let a5 = a5.bits();
    let a6 = a6.bits(); let a7 = a7.bits();
    let a8 = a8.bits(); let a9 = a9.bits();
    let a10 = a10.bits(); let a11 = a11.bits();
    let a12 = a12.bits();
    match ret {
        RetShape::I64 => {
            let f: unsafe extern "C" fn(i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64) -> i64 = unsafe { std::mem::transmute(base) };
            RawRet::I(unsafe { f(a0, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12) })
        }
        RetShape::Void => {
            let f: unsafe extern "C" fn(i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64) = unsafe { std::mem::transmute(base) };
            unsafe { f(a0, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12) };
            RawRet::V
        }
        _ => panic!("FFI: RetShape::F64/I32 con 13 args no soportado"),
    }
}

fn call_typed_14(base: usize, ret: RetShape, a0: RawArg, a1: RawArg, a2: RawArg, a3: RawArg, a4: RawArg, a5: RawArg, a6: RawArg, a7: RawArg, a8: RawArg, a9: RawArg, a10: RawArg, a11: RawArg, a12: RawArg, a13: RawArg) -> RawRet {
    let a0 = a0.bits(); let a1 = a1.bits();
    let a2 = a2.bits(); let a3 = a3.bits();
    let a4 = a4.bits(); let a5 = a5.bits();
    let a6 = a6.bits(); let a7 = a7.bits();
    let a8 = a8.bits(); let a9 = a9.bits();
    let a10 = a10.bits(); let a11 = a11.bits();
    let a12 = a12.bits(); let a13 = a13.bits();
    match ret {
        RetShape::I64 => {
            let f: unsafe extern "C" fn(i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64) -> i64 = unsafe { std::mem::transmute(base) };
            RawRet::I(unsafe { f(a0, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, a13) })
        }
        RetShape::Void => {
            let f: unsafe extern "C" fn(i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64) = unsafe { std::mem::transmute(base) };
            unsafe { f(a0, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, a13) };
            RawRet::V
        }
        _ => panic!("FFI: RetShape::F64/I32 con 14 args no soportado"),
    }
}

fn call_typed_15(base: usize, ret: RetShape, a0: RawArg, a1: RawArg, a2: RawArg, a3: RawArg, a4: RawArg, a5: RawArg, a6: RawArg, a7: RawArg, a8: RawArg, a9: RawArg, a10: RawArg, a11: RawArg, a12: RawArg, a13: RawArg, a14: RawArg) -> RawRet {
    let a0 = a0.bits(); let a1 = a1.bits();
    let a2 = a2.bits(); let a3 = a3.bits();
    let a4 = a4.bits(); let a5 = a5.bits();
    let a6 = a6.bits(); let a7 = a7.bits();
    let a8 = a8.bits(); let a9 = a9.bits();
    let a10 = a10.bits(); let a11 = a11.bits();
    let a12 = a12.bits(); let a13 = a13.bits();
    let a14 = a14.bits();
    match ret {
        RetShape::I64 => {
            let f: unsafe extern "C" fn(i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64) -> i64 = unsafe { std::mem::transmute(base) };
            RawRet::I(unsafe { f(a0, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, a13, a14) })
        }
        RetShape::Void => {
            let f: unsafe extern "C" fn(i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64) = unsafe { std::mem::transmute(base) };
            unsafe { f(a0, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, a13, a14) };
            RawRet::V
        }
        _ => panic!("FFI: RetShape::F64/I32 con 15 args no soportado"),
    }
}

fn call_typed_16(base: usize, ret: RetShape, a0: RawArg, a1: RawArg, a2: RawArg, a3: RawArg, a4: RawArg, a5: RawArg, a6: RawArg, a7: RawArg, a8: RawArg, a9: RawArg, a10: RawArg, a11: RawArg, a12: RawArg, a13: RawArg, a14: RawArg, a15: RawArg) -> RawRet {
    let a0 = a0.bits(); let a1 = a1.bits();
    let a2 = a2.bits(); let a3 = a3.bits();
    let a4 = a4.bits(); let a5 = a5.bits();
    let a6 = a6.bits(); let a7 = a7.bits();
    let a8 = a8.bits(); let a9 = a9.bits();
    let a10 = a10.bits(); let a11 = a11.bits();
    let a12 = a12.bits(); let a13 = a13.bits();
    let a14 = a14.bits(); let a15 = a15.bits();
    match ret {
        RetShape::I64 => {
            let f: unsafe extern "C" fn(i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64) -> i64 = unsafe { std::mem::transmute(base) };
            RawRet::I(unsafe { f(a0, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, a13, a14, a15) })
        }
        RetShape::Void => {
            let f: unsafe extern "C" fn(i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64) = unsafe { std::mem::transmute(base) };
            unsafe { f(a0, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, a13, a14, a15) };
            RawRet::V
        }
        _ => panic!("FFI: RetShape::F64/I32 con 16 args no soportado"),
    }
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
        if args.len() > MAX_NATIVE_ARGS {
            return Err(ClsError::RuntimeError(format!(
                "La función nativa '{}' tiene {} argumentos: el dispatcher de extension soporta hasta {}. Para más args, empaquétalos en un struct y pasa un puntero.",
                symbol, args.len(), MAX_NATIVE_ARGS
            )));
        }
        let resolved = resolve_library(library);
        // Librería cacheadas por path (get-or-insert): no re-abrir dlopen en
        // cada llamada. El `Arc` clonado mantiene la librería viva durante la
        // llamada, así el puntero del símbolo extraído es válido. `RwLock`:
        // doble-check tras adquirir write (otro hilo pudo insertar).
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
                            let l = Arc::new(
                                unsafe { Library::new(&resolved) }.map_err(|e| {
                                    ClsError::RuntimeError(format!(
                                        "No se pudo cargar la librería nativa '{}' (resuelta a '{}'): {}",
                                        library, resolved, e
                                    ))
                                })?,
                            );
                            w.insert(resolved.clone(), l.clone());
                            l
                        }
                    }
                }
            }
        };

        // Símbolo como puntero crudo, cacheado (get-or-insert): no hacer
        // `lib.get(symbol)` en cada llamada. La firma se elige en el dispatcher.
        let sym = get_symbol(lib, &resolved, symbol)?;
        let base = sym.addr;

        // Convertir args -> RawArg (i64 con bits correctos, f64 via to_bits).
        // Los CString/layouts viven en buffers/keepalives durante la llamada.
        let mut buffers: Vec<CString> = Vec::new();
        let mut keepalives: Vec<LayoutKeepAlive> = Vec::new();
        let mut raw_args: Vec<RawArg> = Vec::with_capacity(args.len());
        for (i, arg) in args.iter().enumerate() {
            let nt = param_types.get(i).cloned().unwrap_or(NativeType::Any);
            let cval = conv_arg(&nt, arg, &mut buffers, &mut keepalives)?;
            let shape = arg_shape(&nt)?;
            raw_args.push(match shape {
                Shape::I64 => RawArg::I(cval_i(&cval)),
                Shape::F64 => RawArg::F(cval_f(&cval)),
            });
        }
        let rshape = ret_shape(&ret)?;

        // Dispatch por cantidad (N lineal, no 2^N). Usamos un match runtime
        // (no macro) porque Rust estable no permite forward-ear un literal
        // numerico entre macros. El costo es 1 comparacion de usize, mucho
        // menor que el dispatch 2^N anterior.
        let raw: RawRet = match args.len() {
            0 => call_typed_0(base, rshape),
            1 => call_typed_1(base, rshape, raw_args[0]),
            2 => call_typed_2(base, rshape, raw_args[0], raw_args[1]),
            3 => call_typed_3(base, rshape, raw_args[0], raw_args[1], raw_args[2]),
            4 => call_typed_4(base, rshape, raw_args[0], raw_args[1], raw_args[2], raw_args[3]),
            5 => call_typed_5(base, rshape, raw_args[0], raw_args[1], raw_args[2], raw_args[3], raw_args[4]),
            6 => call_typed_6(base, rshape, raw_args[0], raw_args[1], raw_args[2], raw_args[3], raw_args[4], raw_args[5]),
            7 => call_typed_7(base, rshape, raw_args[0], raw_args[1], raw_args[2], raw_args[3], raw_args[4], raw_args[5], raw_args[6]),
            8 => call_typed_8(base, rshape, raw_args[0], raw_args[1], raw_args[2], raw_args[3], raw_args[4], raw_args[5], raw_args[6], raw_args[7]),
            9 => call_typed_9(base, rshape, raw_args[0], raw_args[1], raw_args[2], raw_args[3], raw_args[4], raw_args[5], raw_args[6], raw_args[7], raw_args[8]),
            10 => call_typed_10(base, rshape, raw_args[0], raw_args[1], raw_args[2], raw_args[3], raw_args[4], raw_args[5], raw_args[6], raw_args[7], raw_args[8], raw_args[9]),
            11 => call_typed_11(base, rshape, raw_args[0], raw_args[1], raw_args[2], raw_args[3], raw_args[4], raw_args[5], raw_args[6], raw_args[7], raw_args[8], raw_args[9], raw_args[10]),
            12 => call_typed_12(base, rshape, raw_args[0], raw_args[1], raw_args[2], raw_args[3], raw_args[4], raw_args[5], raw_args[6], raw_args[7], raw_args[8], raw_args[9], raw_args[10], raw_args[11]),
            13 => call_typed_13(base, rshape, raw_args[0], raw_args[1], raw_args[2], raw_args[3], raw_args[4], raw_args[5], raw_args[6], raw_args[7], raw_args[8], raw_args[9], raw_args[10], raw_args[11], raw_args[12]),
            14 => call_typed_14(base, rshape, raw_args[0], raw_args[1], raw_args[2], raw_args[3], raw_args[4], raw_args[5], raw_args[6], raw_args[7], raw_args[8], raw_args[9], raw_args[10], raw_args[11], raw_args[12], raw_args[13]),
            15 => call_typed_15(base, rshape, raw_args[0], raw_args[1], raw_args[2], raw_args[3], raw_args[4], raw_args[5], raw_args[6], raw_args[7], raw_args[8], raw_args[9], raw_args[10], raw_args[11], raw_args[12], raw_args[13], raw_args[14]),
            16 => call_typed_16(base, rshape, raw_args[0], raw_args[1], raw_args[2], raw_args[3], raw_args[4], raw_args[5], raw_args[6], raw_args[7], raw_args[8], raw_args[9], raw_args[10], raw_args[11], raw_args[12], raw_args[13], raw_args[14], raw_args[15]),
            n => unreachable!("MAX_NATIVE_ARGS check ya filtra n > 16"),
        };

        conv_ret(raw, &ret)
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

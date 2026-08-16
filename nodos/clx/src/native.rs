//! Backend nativo dinámico para la feature `extension` del nodo clx.
//!
//! Resuelve símbolos por nombre en librerías del sistema (libloading / dlopen /
//! LoadLibrary) -> el usuario final solo escribe CLS (`extension "lib" as C { ... }`),
//! sin tocar el nodo ni salir del lenguaje.
//!
//! ABI: los argumentos se pasan por registros (i64 para enteros/punteros, f64 para
//! floats); los retornos por su shape exacto (i64/i32/f64/void). El dispatcher
//! (macros `arityN!`) cubre hasta 4 argumentos; se amplía extendiendo los macros.

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
/// viva. Nunca se descarga: el proceso la mantiene abierta.
static NATIVE_LIBS: std::sync::LazyLock<
    std::sync::Mutex<std::collections::HashMap<String, Arc<Library>>>,
> = std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));

// ── Shapes de registro del ABI C ─────────────────────────────────────────────

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

fn conv_arg(nt: &NativeType, value: &Value, buffers: &mut Vec<CString>) -> ClsResult<CVal> {
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

// ── Dispatcher de firmas (hasta 4 argumentos) ───────────────────────────────

/// Genera un arm que castea el símbolo a la firma dada y la llama.
///
/// El `transmute` es el patrón dlsym: el símbolo se obtiene como un puntero a
/// `unsafe extern "C" fn()` genérico (la firma real no se conoce hasta el
/// dispatch por arity + shapes) y se reinterpreta por cada firma concreta. Rust
/// no permite castear directamente entre tipos de fn pointer distintos, así que
/// se reinterpreta el usize del símbolo. Es seguro mientras la firma elegida
/// coincida con el símbolo real (contrato del `extension "lib" as C { ... }`)
/// y la librería siga viva (la cachea `NATIVE_LIBS`).
macro_rules! emit_arm {
    ($base:expr, $ret:expr; [$($t:ty, $v:expr),*]) => {
        match $ret {
            RetShape::I64 => {
                let f: unsafe extern "C" fn($($t),*) -> i64 = unsafe { std::mem::transmute($base) };
                RawRet::I(unsafe { f($($v),*) })
            }
            RetShape::I32 => {
                let f: unsafe extern "C" fn($($t),*) -> i32 = unsafe { std::mem::transmute($base) };
                RawRet::I(unsafe { f($($v),*) } as i64)
            }
            RetShape::F64 => {
                let f: unsafe extern "C" fn($($t),*) -> f64 = unsafe { std::mem::transmute($base) };
                RawRet::F(unsafe { f($($v),*) })
            }
            RetShape::Void => {
                let f: unsafe extern "C" fn($($t),*) = unsafe { std::mem::transmute($base) };
                unsafe { f($($v),*) };
                RawRet::V
            }
        }
    };
}

macro_rules! arity0 {
    ($base:expr, $ret:expr) => {
        emit_arm!($base, $ret; [])
    };
}

macro_rules! arity1 {
    ($base:expr, $vals:expr, $ret:expr, $pts:expr) => {
        match ($pts[0], $ret) {
            (Shape::I64, r) => emit_arm!($base, r; [i64, cval_i(&$vals[0])]),
            (Shape::F64, r) => emit_arm!($base, r; [f64, cval_f(&$vals[0])]),
        }
    };
}

macro_rules! arity2 {
    ($base:expr, $vals:expr, $ret:expr, $pts:expr) => {
        match ($pts[0], $pts[1], $ret) {
            (Shape::I64, Shape::I64, r) => emit_arm!($base, r; [i64, cval_i(&$vals[0]), i64, cval_i(&$vals[1])]),
            (Shape::I64, Shape::F64, r) => emit_arm!($base, r; [i64, cval_i(&$vals[0]), f64, cval_f(&$vals[1])]),
            (Shape::F64, Shape::I64, r) => emit_arm!($base, r; [f64, cval_f(&$vals[0]), i64, cval_i(&$vals[1])]),
            (Shape::F64, Shape::F64, r) => emit_arm!($base, r; [f64, cval_f(&$vals[0]), f64, cval_f(&$vals[1])]),
        }
    };
}

macro_rules! arity3 {
    ($base:expr, $vals:expr, $ret:expr, $pts:expr) => {
        match ($pts[0], $pts[1], $pts[2], $ret) {
            (Shape::I64, Shape::I64, Shape::I64, r) => emit_arm!($base, r; [i64, cval_i(&$vals[0]), i64, cval_i(&$vals[1]), i64, cval_i(&$vals[2])]),
            (Shape::I64, Shape::I64, Shape::F64, r) => emit_arm!($base, r; [i64, cval_i(&$vals[0]), i64, cval_i(&$vals[1]), f64, cval_f(&$vals[2])]),
            (Shape::I64, Shape::F64, Shape::I64, r) => emit_arm!($base, r; [i64, cval_i(&$vals[0]), f64, cval_f(&$vals[1]), i64, cval_i(&$vals[2])]),
            (Shape::I64, Shape::F64, Shape::F64, r) => emit_arm!($base, r; [i64, cval_i(&$vals[0]), f64, cval_f(&$vals[1]), f64, cval_f(&$vals[2])]),
            (Shape::F64, Shape::I64, Shape::I64, r) => emit_arm!($base, r; [f64, cval_f(&$vals[0]), i64, cval_i(&$vals[1]), i64, cval_i(&$vals[2])]),
            (Shape::F64, Shape::I64, Shape::F64, r) => emit_arm!($base, r; [f64, cval_f(&$vals[0]), i64, cval_i(&$vals[1]), f64, cval_f(&$vals[2])]),
            (Shape::F64, Shape::F64, Shape::I64, r) => emit_arm!($base, r; [f64, cval_f(&$vals[0]), f64, cval_f(&$vals[1]), i64, cval_i(&$vals[2])]),
            (Shape::F64, Shape::F64, Shape::F64, r) => emit_arm!($base, r; [f64, cval_f(&$vals[0]), f64, cval_f(&$vals[1]), f64, cval_f(&$vals[2])]),
        }
    };
}

macro_rules! arity4 {
    ($base:expr, $vals:expr, $ret:expr, $pts:expr) => {
        match ($pts[0], $pts[1], $pts[2], $pts[3], $ret) {
            (Shape::I64, Shape::I64, Shape::I64, Shape::I64, r) => emit_arm!($base, r; [i64, cval_i(&$vals[0]), i64, cval_i(&$vals[1]), i64, cval_i(&$vals[2]), i64, cval_i(&$vals[3])]),
            (Shape::I64, Shape::I64, Shape::I64, Shape::F64, r) => emit_arm!($base, r; [i64, cval_i(&$vals[0]), i64, cval_i(&$vals[1]), i64, cval_i(&$vals[2]), f64, cval_f(&$vals[3])]),
            (Shape::I64, Shape::I64, Shape::F64, Shape::I64, r) => emit_arm!($base, r; [i64, cval_i(&$vals[0]), i64, cval_i(&$vals[1]), f64, cval_f(&$vals[2]), i64, cval_i(&$vals[3])]),
            (Shape::I64, Shape::I64, Shape::F64, Shape::F64, r) => emit_arm!($base, r; [i64, cval_i(&$vals[0]), i64, cval_i(&$vals[1]), f64, cval_f(&$vals[2]), f64, cval_f(&$vals[3])]),
            (Shape::I64, Shape::F64, Shape::I64, Shape::I64, r) => emit_arm!($base, r; [i64, cval_i(&$vals[0]), f64, cval_f(&$vals[1]), i64, cval_i(&$vals[2]), i64, cval_i(&$vals[3])]),
            (Shape::I64, Shape::F64, Shape::I64, Shape::F64, r) => emit_arm!($base, r; [i64, cval_i(&$vals[0]), f64, cval_f(&$vals[1]), i64, cval_i(&$vals[2]), f64, cval_f(&$vals[3])]),
            (Shape::I64, Shape::F64, Shape::F64, Shape::I64, r) => emit_arm!($base, r; [i64, cval_i(&$vals[0]), f64, cval_f(&$vals[1]), f64, cval_f(&$vals[2]), i64, cval_i(&$vals[3])]),
            (Shape::I64, Shape::F64, Shape::F64, Shape::F64, r) => emit_arm!($base, r; [i64, cval_i(&$vals[0]), f64, cval_f(&$vals[1]), f64, cval_f(&$vals[2]), f64, cval_f(&$vals[3])]),
            (Shape::F64, Shape::I64, Shape::I64, Shape::I64, r) => emit_arm!($base, r; [f64, cval_f(&$vals[0]), i64, cval_i(&$vals[1]), i64, cval_i(&$vals[2]), i64, cval_i(&$vals[3])]),
            (Shape::F64, Shape::I64, Shape::I64, Shape::F64, r) => emit_arm!($base, r; [f64, cval_f(&$vals[0]), i64, cval_i(&$vals[1]), i64, cval_i(&$vals[2]), f64, cval_f(&$vals[3])]),
            (Shape::F64, Shape::I64, Shape::F64, Shape::I64, r) => emit_arm!($base, r; [f64, cval_f(&$vals[0]), i64, cval_i(&$vals[1]), f64, cval_f(&$vals[2]), i64, cval_i(&$vals[3])]),
            (Shape::F64, Shape::I64, Shape::F64, Shape::F64, r) => emit_arm!($base, r; [f64, cval_f(&$vals[0]), i64, cval_i(&$vals[1]), f64, cval_f(&$vals[2]), f64, cval_f(&$vals[3])]),
            (Shape::F64, Shape::F64, Shape::I64, Shape::I64, r) => emit_arm!($base, r; [f64, cval_f(&$vals[0]), f64, cval_f(&$vals[1]), i64, cval_i(&$vals[2]), i64, cval_i(&$vals[3])]),
            (Shape::F64, Shape::F64, Shape::I64, Shape::F64, r) => emit_arm!($base, r; [f64, cval_f(&$vals[0]), f64, cval_f(&$vals[1]), i64, cval_i(&$vals[2]), f64, cval_f(&$vals[3])]),
            (Shape::F64, Shape::F64, Shape::F64, Shape::I64, r) => emit_arm!($base, r; [f64, cval_f(&$vals[0]), f64, cval_f(&$vals[1]), f64, cval_f(&$vals[2]), i64, cval_i(&$vals[3])]),
            (Shape::F64, Shape::F64, Shape::F64, Shape::F64, r) => emit_arm!($base, r; [f64, cval_f(&$vals[0]), f64, cval_f(&$vals[1]), f64, cval_f(&$vals[2]), f64, cval_f(&$vals[3])]),
        }
    };
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
        if args.len() > 4 {
            return Err(ClsError::RuntimeError(format!(
                "La función nativa '{}' tiene {} argumentos: el dispatcher de extension soporta hasta 4 (extender los macros arityN!)",
                symbol, args.len()
            )));
        }
        let resolved = resolve_library(library);
        // Librería cacheadas por path (get-or-insert): no re-abrir dlopen en
        // cada llamada. El `Arc` clonado mantiene la librería viva durante la
        // llamada, así el puntero del símbolo extraído es válido.
        let lib = {
            let mut cache = NATIVE_LIBS.lock().unwrap();
            match cache.get(&resolved) {
                Some(l) => l.clone(),
                None => {
                    let l = Arc::new(unsafe { Library::new(&resolved) }.map_err(|e| {
                        ClsError::RuntimeError(format!(
                            "No se pudo cargar la librería nativa '{}' (resuelta a '{}'): {}",
                            library, resolved, e
                        ))
                    })?);
                    cache.insert(resolved.clone(), l.clone());
                    l
                }
            }
        };

        // Símbolo como puntero crudo (la firma se elige en el dispatcher).
        let sym: libloading::Symbol<'_, unsafe extern "C" fn()> =
            unsafe { lib.get(symbol.as_bytes()) }.map_err(|e| {
                ClsError::RuntimeError(format!(
                    "Símbolo nativo '{}' no encontrado en '{}': {}",
                    symbol, resolved, e
                ))
            })?;
        let base = *sym as usize;

        // Convertir args -> registros; los CString viven en buffers durante la llamada.
        let mut buffers: Vec<CString> = Vec::new();
        let mut cvals: Vec<CVal> = Vec::with_capacity(args.len());
        let mut shapes: Vec<Shape> = Vec::with_capacity(args.len());
        for (i, arg) in args.iter().enumerate() {
            let nt = param_types.get(i).cloned().unwrap_or(NativeType::Any);
            cvals.push(conv_arg(&nt, arg, &mut buffers)?);
            shapes.push(arg_shape(&nt)?);
        }
        let rshape = ret_shape(&ret)?;

        let raw: RawRet = match args.len() {
            0 => arity0!(base, rshape),
            1 => arity1!(base, cvals, rshape, shapes),
            2 => arity2!(base, cvals, rshape, shapes),
            3 => arity3!(base, cvals, rshape, shapes),
            4 => arity4!(base, cvals, rshape, shapes),
            _ => unreachable!(),
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

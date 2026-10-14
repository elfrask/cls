//! FFI: backend nativo para la feature `extension` (librerías del sistema).
//!
//! El trait `NativeBackend` opera sobre el `Value` de CLS (definido en
//! `cls-runtime::value`). El `Value` se movió a su propio módulo
//! (Fase 7) desacoplado del tree-walker: el FFI no depende del walker.
//!
//! El runtime es agnóstico al entorno: el **nodo** (clx/clxr) implementa
//! este trait (o usa `DynamicBackend` por default) para resolver
//! símbolos en librerías del sistema.

use crate::error::ClsResult;
use crate::value::Value;

/// Tipo nativo (ABI C) de un parámetro/retorno/variable de `extension`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeType {
    /// Sin anotación: el backend decide (valor dinámico).
    Any,
    /// `int` -> `i64`
    Int,
    /// `float` -> `f64`
    Float,
    /// `bool` -> `i32` (0/1)
    Bool,
    Void,
    /// `CString` -> `char*` (CLS String -> buffer null-terminated, copia)
    CString,
    /// `CPtr<T>` -> `void*` / puntero a `T`
    CPtr,
    CInt,
    CUInt,
    CShort,
    CUShort,
    CLong,
    CULong,
    CChar,
    CUChar,
    CFloat,
    CDouble,
    /// `structure` nativa (layout C); el nombre identifica el layout.
    Struct(String),
    /// `CRecord` — record CLS -> ptr al layout `[cap][len][(key,val,tag)*24]`.
    /// El valor viaja como puntero a la memoria lineal del módulo.
    CRecord,
    /// `CArray` — array CLS -> ptr al layout `[cap][len][elems*es]`.
    /// El valor viaja como puntero a la memoria lineal del módulo.
    CArray,
    /// `CStruct` — struct CLS -> ptr al layout contiguo (offsets).
    /// El valor viaja como puntero a la memoria lineal del módulo.
    CStruct,
}

/// Resultado de una llamada nativa. `Value` es el caso general; `Bytes`
/// transporta un retorno `CString` **byte a byte**: CLS usa semántica de bytes y
/// un `char*` de C no tiene por qué ser UTF-8 válido, así que no debe
/// normalizarse a U+FFFD antes de escribirlo en la memoria del módulo.
#[derive(Debug, Clone)]
pub enum NativeRet {
    Value(Value),
    Bytes(Vec<u8>),
}

impl NativeRet {
    /// Extrae el `Value`. `Bytes` sólo aparece en el retorno `CString` (que el
    /// JIT consume como bytes); fuera de ese camino se degrada a `String`.
    pub fn into_value(self) -> Value {
        match self {
            NativeRet::Value(v) => v,
            NativeRet::Bytes(b) => Value::String(String::from_utf8_lossy(&b).into_owned()),
        }
    }
}

/// Backend que resuelve símbolos de librerías del sistema (linkadas por el SO).
pub trait NativeBackend: Send + Sync {
    /// Llama una función nativa declarada en `extension`.
    fn call_function(
        &self,
        library: &str,
        symbol: &str,
        args: &[Value],
        param_types: &[NativeType],
        ret: NativeType,
    ) -> ClsResult<Value>;

    /// Variante del camino FFI del JIT: igual que `call_function` pero entrega
    /// los retornos `CString` como bytes crudos (`NativeRet::Bytes`), para que
    /// sean byte-transparentes. Por defecto delega en `call_function`.
    fn call_function_raw(
        &self,
        library: &str,
        symbol: &str,
        args: &[Value],
        param_types: &[NativeType],
        ret: NativeType,
    ) -> ClsResult<NativeRet> {
        self.call_function(library, symbol, args, param_types, ret)
            .map(NativeRet::Value)
    }

    /// Lee una variable nativa (símbolo `extern`).
    fn get_variable(&self, library: &str, name: &str, ty: NativeType) -> ClsResult<Value>;

    /// Escribe una variable nativa (símbolo `extern`).
    fn set_variable(&self, library: &str, name: &str, ty: NativeType, value: &Value) -> ClsResult<()>;
}

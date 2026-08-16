//! FFI: backend nativo para la feature `extension` (librerías del sistema).
//!
//! El runtime es agnóstico al entorno: el **nodo** (clx/clxr) implementa
//! `NativeBackend` y lo inyecta con `Interpreter::set_native_backend`.
//! Los símbolos los linkea el sistema operativo (el binario host los expone);
//! CLS no hace dlopen/LoadLibrary.

use crate::error::ClsResult;
use crate::walker::value::Value;

/// Tipo nativo (ABI C) de un parámetro/retorno/variable de `extension`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeType {
    /// Sin anotación: el backend decide (valor dinámico).
    Any,
    /// `int` → `i64`
    Int,
    /// `float` → `f64`
    Float,
    /// `bool` → `i32` (0/1)
    Bool,
    Void,
    /// `CString` → `char*` (CLS String → buffer null-terminated, copia)
    CString,
    /// `CPtr<T>` → `void*` / puntero a `T`
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

    /// Lee una variable nativa (símbolo `extern`).
    fn get_variable(&self, library: &str, name: &str, ty: NativeType) -> ClsResult<Value>;

    /// Escribe una variable nativa (símbolo `extern`).
    fn set_variable(&self, library: &str, name: &str, ty: NativeType, value: &Value) -> ClsResult<()>;
}

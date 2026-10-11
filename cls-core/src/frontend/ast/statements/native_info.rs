//! AST - NativeInfo (marca de declaración nativa/FFI).

use super::ExtensionKind;
use serde::{Deserialize, Serialize};


/// Marca de una declaración nativa (FFI): una `function`/`structure`/`var`
/// declarada dentro de `extension "lib" as C { ... }`. La declaración es su
/// **equivalente nativo del lenguaje** (una `FunctionDecl`/`StructureDecl`/
/// `VarDecl` normal), lo que la hace compatible con todo el sistema de módulos
/// (exports, imports, flatten, rename). `symbol` es el nombre real en la
/// librería (distinto de `name` si se reexporta con `as`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NativeInfo {
    /// Librería nativa (`ws2_32.dll`, `libc.so.6`, ...).
    pub library: String,
    /// Tipo de extensión (`C`, `Wasi`, ...).
    pub kind: ExtensionKind,
    /// Símbolo real en la librería. Normalmente igual al nombre de la
    /// declaración; se conserva separado para que el rename de la exportación
    /// (`export { strlen as len }`) no cambie el símbolo C.
    pub symbol: String,
}

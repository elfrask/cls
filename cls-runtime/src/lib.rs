pub mod error;
pub mod error_report;
pub mod ffi;
pub mod native_backend;
pub mod value;
pub mod vfs;

// Compat transitoria: errores y paths de módulo que los nodos usan directamente.
pub use error::{ClsError, ClsResult};
pub use error_report::{ErrorReport, ErrorFormat, ErrorFormatter, format_error, format_runtime_error, format_syntax_error, show_runtime_error, show_syntax_error, show_config_error};
pub use value::{Value, FunValue, FunKind, NativeFn, ClosureEnv, Pollable, PollState, Promise, StructDef, StructField, StructInstance, ClassDef, ClassInstance, EnumDef, EnumValue, CmxValue};
pub use vfs::{VfsResolver, VfsProtocol, LocalFs, ZipFs, resolve_safe};

/// Backend nativo FFI via libloading (dlopen/LoadLibrary). Reusado por
/// `nodos/clx` y `nodos/clxr` (dev-2, migrado de `nodos/clx/src/native.rs`).
pub use native_backend::{DynamicBackend, MAX_NATIVE_ARGS};

/// Version del runtime CLS
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

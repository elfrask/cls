pub mod error;
pub mod error_report;
pub mod ffi;
pub mod native_backend;
pub mod walker;

// Compat transitoria: paths de módulo que los nodos usan directamente
// (se eliminan junto con el walker tras 2.0-dev1).
pub use walker::stdlib;
pub use walker::value;

pub use error::{ClsError, ClsResult};
pub use error_report::{ErrorReport, ErrorFormat, ErrorFormatter, format_error, format_runtime_error, format_syntax_error, show_runtime_error, show_syntax_error, show_config_error};
pub use walker::{Value, Promise, Pollable, PollState, ClassDef, ClassInstance};
pub use walker::Environment;
pub use walker::{Interpreter, ImportFrame};
pub use walker::Intrinsics;
pub use walker::{ModuleResolver, user_modules_dir, global_modules_dir};
pub use walker::{VfsResolver, VfsProtocol, LocalFs, ZipFs, resolve_safe};
pub use walker::{ClsLibIndex, ClsLibEntry, ClsLibResolver, compute_hash_bytes};
pub use walker::GarbageCollector;
pub use walker::Sandbox;

/// Backend nativo FFI via libloading (dlopen/LoadLibrary). Reusado por
/// `nodos/clx` y `nodos/clxr` (dev-2, migrado de `nodos/clx/src/native.rs`).
pub use native_backend::{DynamicBackend, MAX_NATIVE_ARGS};

/// Version del runtime CLS
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

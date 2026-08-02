pub mod value;
pub mod environment;
pub mod interpreter;
pub mod intrinsics;
pub mod resolver;
pub mod vfs;
pub mod clslib;
pub mod error_report;
pub mod gc;
pub mod sandbox;
pub mod modules;
pub mod stdlib;
pub mod host_api;
pub mod ffi;
pub mod error;

pub use value::{Value, Promise, Pollable, PollState};
pub use environment::Environment;
pub use interpreter::{Interpreter, ImportFrame};
pub use intrinsics::Intrinsics;
pub use resolver::ModuleResolver;
pub use vfs::{VfsResolver, VfsProtocol, LocalFs, ZipFs, resolve_safe};
pub use clslib::{ClsLibIndex, ClsLibEntry, ClsLibResolver, compute_hash_bytes};
pub use error_report::{ErrorReport, show_runtime_error, show_syntax_error, show_config_error};
pub use gc::GarbageCollector;
pub use sandbox::Sandbox;
pub use error::{ClsError, ClsResult};

/// Version del runtime CLS
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

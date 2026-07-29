pub mod value;
pub mod environment;
pub mod interpreter;
pub mod intrinsics;
pub mod resolver;
pub mod vfs;
pub mod clslib;
pub mod gc;
pub mod sandbox;
pub mod modules;
pub mod stdlib;
pub mod host_api;
pub mod ffi;
pub mod error;

pub use value::Value;
pub use environment::Environment;
pub use interpreter::{Interpreter, ImportFrame};
pub use intrinsics::Intrinsics;
pub use resolver::ModuleResolver;
pub use vfs::{VfsResolver, VfsProtocol, LocalFs, resolve_safe};
pub use gc::GarbageCollector;
pub use sandbox::Sandbox;
pub use error::{ClsError, ClsResult};

/// Version del runtime CLS
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

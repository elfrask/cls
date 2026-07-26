pub mod value;
pub mod environment;
pub mod interpreter;
pub mod intrinsics;
pub mod resolver;
pub mod gc;
pub mod sandbox;
pub mod modules;
pub mod stdlib;
pub mod host_api;
pub mod ffi;
pub mod error;

pub use value::Value;
pub use environment::Environment;
pub use interpreter::Interpreter;
pub use intrinsics::Intrinsics;
pub use resolver::ModuleResolver;
pub use gc::GarbageCollector;
pub use sandbox::Sandbox;
pub use error::{ClsError, ClsResult};

/// Versión del runtime CLS
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

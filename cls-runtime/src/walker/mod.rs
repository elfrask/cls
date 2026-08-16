//! ⚠️ AST-walker (intérprete DEPRECADO).
//!
//! TODO ESTE SUBÁRBOL SE DEPRECA CON EL WALKER tras la salida de CLS 2.0-dev1.
//! El intérprete objetivo es el JIT (`clx run`, CLS → WASM → wasmtime).
//! `mod.rs` re-exporta los items para compat transitoria con los nodos.

pub mod clslib;
pub mod environment;
pub mod gc;
pub mod host_api;
pub mod interpreter;
pub mod intrinsics;
pub mod modules;
pub mod resolver;
pub mod sandbox;
pub mod stdlib;
pub mod value;
pub mod vfs;

pub use clslib::{ClsLibIndex, ClsLibEntry, ClsLibResolver, compute_hash_bytes};
pub use environment::Environment;
pub use gc::GarbageCollector;
pub use interpreter::{Interpreter, ImportFrame};
pub use intrinsics::Intrinsics;
pub use resolver::{ModuleResolver, user_modules_dir, global_modules_dir};
pub use sandbox::Sandbox;
pub use value::{Value, Promise, Pollable, PollState, ClassDef, ClassInstance};
pub use vfs::{VfsResolver, VfsProtocol, LocalFs, ZipFs, resolve_safe};

//! Virtual File System (VFS) de CLS.
//!
//! Resolver de protocolos (res, app, user, tmp, rutas personalizadas) y
//! backends (LocalFs, ZipFs). Usado por el LSP y por `clx run`/`clxr`
//! para resolver paths de imports (e.g. `import "app://config.clsx"`).
//!
//! Migracion dev-2 (Fase 7): este modulo vivia en `cls-runtime/src/walker/vfs/`.
//! Se movio a su propio modulo para desacoplarlo del tree-walker
//! (que se elimina en esta misma fase).

pub mod protocol;
pub mod resolver;
pub mod security;

pub use protocol::{LocalFs, VfsProtocol, ZipFs};
pub use resolver::VfsResolver;
pub use security::resolve_safe;

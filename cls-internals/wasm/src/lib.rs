//! Módulo WASM de internals de CLS (compilado a `wasm32-unknown-unknown`).
//!
//! Cada función se exporta como `__intr_<area>_<op>` con el ABI de
//! `cls-internals/src/abi.rs` (mismo layout que `cls-core/src/backend/wasm/`).
//! La memoria lineal es la del módulo; en la Fase 3 se compartirá con el
//! módulo CLS (fusión de secciones).

mod arrays;
mod convert;
mod fmt;
mod math;
mod mem;
mod records;
mod strings;

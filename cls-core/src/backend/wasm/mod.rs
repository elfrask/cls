//! Backend WASM: compila AST tipado -> módulo WebAssembly.
//!
//! Estrategia: el emisor camina el AST directamente (WASM es stack-based, por lo
//! que las expresiones se emiten en post-order y dejan su valor en el stack).
//! El type map (Span -> Type) del TypeChecker determina las representaciones:
//!
//! | Type CLS  | WASM             | Notas                                  |
//! |-----------|------------------|----------------------------------------|
//! | Int       | i64              |                                        |
//! | Float     | f64              |                                        |
//! | Bool      | i32 (0/1)        |                                        |
//! | Char      | i32 (u32 codep)  |                                        |
//! | String    | i64 (ptr<<32|len)| ptr = offset en memoria lineal         |
//! | Array<T>  | i64 (ptr)        | header [len:i64][elem...]              |
//!
//! El allocator es bump (sin free) con la memoria embebida en el módulo; el host
//! solo inyecta funciones `env.*` (print, conversiones, trap) y `alloc` para los
//! args de `main`.

#![cfg(feature = "wasm-backend")]

pub mod host_fn;
mod emitter;
mod arrows;
mod engine;
mod helpers;
mod layout;
mod types;
use arrows::*;
use emitter::{FuncEmitter, HostCaller};
use engine::*;
use helpers::*;
use host_fn::HostFn;
use layout::*;
use types::*;

use crate::error::ClsResult;
use crate::error::Span;
use crate::frontend::ast::*;
use crate::frontend::token::Operator;
use crate::middleware::typeck::expr_span;
use crate::middleware::types::{HostIntrinsic, LitVal, Type};
use std::borrow::Cow;
use std::collections::HashMap;
use std::collections::HashSet;
use wasm_encoder::{
    BlockType, Catch, CodeSection, ConstExpr, CustomSection, DataSection, DataSegment,
    DataSegmentMode, ElementSection, Elements, EntityType, ExportKind, ExportSection, Function,
    FunctionSection, GlobalSection, GlobalType, Ieee64, ImportSection, Instruction, MemArg,
    MemorySection, MemoryType, Module as WasmModule, RefType, TableSection, TableType, TagKind,
    TagSection, TagType, TypeSection, ValType,
};



/// Código de kind CLS para la sección custom `clx:exports` (firma tipada que el



#[derive(Clone, Debug)]
pub struct WasmBackendOptions {
    /// `true` = emite el tag de excepción CLS + try_table/throw (wasmtime).
    /// `false` = modo sin excepciones (wasmi): sin tag, errores de runtime como
    /// `unreachable` y `try/catch`/`throw` fallan con error claro.
    pub exceptions: bool,
    /// `true` = el módulo DEBE tener `main(args: String[])` (modo app).
    /// `false` = modo librería: si no hay main se sintetiza un main no-op
    /// (para `.clx`-librería que solo expone `export function`).
    pub require_main: bool,
    /// Funciones host del NODO (intrinsics): las llamadas a esos nombres se
    /// compilan vía el canal `env.host_call(id, ptr, n)`.
    pub intrinsics: Vec<HostIntrinsic>,
}

impl Default for WasmBackendOptions {
    fn default() -> Self {
        Self {
            exceptions: true,
            require_main: true,
            intrinsics: Vec::new(),
        }
    }
}

/// Compila un Module tipado a un binario WASM.
/// Backend WASM. Toma el type map `Span -> Type` por referencia (el caller -
/// `jit.rs` - mantiene el `TypeChecker` vivo durante la emisión) para no clonar
/// el mapa en cada compilación.
pub struct WasmBackend<'a> {
    types: &'a HashMap<Span, Type>,
    target: Target,
    exceptions: bool,
    require_main: bool,
    intrinsics: Vec<HostIntrinsic>,
}

impl<'a> WasmBackend<'a> {
    pub fn new(types: &'a HashMap<Span, Type>) -> Self {
        Self::with_target(types, Target::host())
    }

    /// Backend con un target explícito (para `when` compile-time).
    pub fn with_target(types: &'a HashMap<Span, Type>, target: Target) -> Self {
        Self::with_options(types, target, WasmBackendOptions::default())
    }

    /// Backend con opciones explícitas.
    pub fn with_options(
        types: &'a HashMap<Span, Type>,
        target: Target,
        opts: WasmBackendOptions,
    ) -> Self {
        Self {
            types,
            target,
            exceptions: opts.exceptions,
            require_main: opts.require_main,
            intrinsics: opts.intrinsics,
        }
    }

    /// Backend sin excepciones WASM (para runtimes que no implementan la
    /// propuesta de exception-handling, p.ej. wasmi en el navegador).
    pub fn without_exceptions(types: &'a HashMap<Span, Type>, target: Target) -> Self {
        Self::with_options(
            types,
            target,
            WasmBackendOptions {
                exceptions: false,
                ..Default::default()
            },
        )
    }

    /// Backend en modo librería (sin `main` obligatorio): útil para `.clx`
    /// que solo exponen `export function` (futuro nodo de bindings).
    pub fn library_mode(types: &'a HashMap<Span, Type>, target: Target) -> Self {
        Self::with_options(
            types,
            target,
            WasmBackendOptions {
                require_main: false,
                ..Default::default()
            },
        )
    }

    /// Backend en modo librería Y sin excepciones (bindings browser).
    pub fn library_without_exceptions(types: &'a HashMap<Span, Type>, target: Target) -> Self {
        Self::with_options(
            types,
            target,
            WasmBackendOptions {
                exceptions: false,
                require_main: false,
                intrinsics: Vec::new(),
            },
        )
    }

    pub fn emit(&self, module: &Module) -> ClsResult<Vec<u8>> {
        self.emit_with_pool(module).map(|(bytes, _)| bytes)
    }

    /// Igual que [`Self::emit`] pero además devuelve el string pool final del
    /// módulo (orden de interning, append-only). El REPL JIT lo usa para
    /// re-sembrar los mismos offsets en la sesión siguiente (los punteros de
    /// strings transferidos entre instancias apuntan a esta región).
    pub fn emit_with_pool(&self, module: &Module) -> ClsResult<(Vec<u8>, Vec<String>)> {
        let mut engine = Engine::new(self.types, self.target.clone());
        engine.exceptions = self.exceptions;
        engine.require_main = self.require_main;
        engine.intrinsics = self
            .intrinsics
            .iter()
            .map(|i| (i.name.clone(), i.clone()))
            .collect();
        let bytes = engine.emit(module)?;
        Ok((bytes, engine.string_pool.clone()))
    }
}

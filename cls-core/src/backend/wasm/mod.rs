//! Backend WASM: compila AST tipado â†’ mÃ³dulo WebAssembly.
//!
//! Estrategia: el emisor camina el AST directamente (WASM es stack-based, por lo
//! que las expresiones se emiten en post-order y dejan su valor en el stack).
//! El type map (Span â†’ Type) del TypeChecker determina las representaciones:
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
//! El allocator es bump (sin free) con la memoria embebida en el mÃ³dulo; el host
//! solo inyecta funciones `env.*` (print, conversiones, trap) y `alloc` para los
//! args de `main`.

#![cfg(feature = "wasm-backend")]

pub mod host_fn;
mod emitter;
mod engine;
mod helpers;
mod layout;
mod types;
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



/// CÃ³digo de kind CLS para la secciÃ³n custom `clx:exports` (firma tipada que el



#[derive(Clone, Debug)]
pub struct WasmBackendOptions {
    /// `true` = emite el tag de excepciÃ³n CLS + try_table/throw (wasmtime).
    /// `false` = modo sin excepciones (wasmi): sin tag, errores de runtime como
    /// `unreachable` y `try/catch`/`throw` fallan con error claro.
    pub exceptions: bool,
    /// `true` = el mÃ³dulo DEBE tener `main(args: String[])` (modo app).
    /// `false` = modo librerÃ­a: si no hay main se sintetiza un main no-op
    /// (para `.clx`-librerÃ­a que solo expone `export function`).
    pub require_main: bool,
    /// Funciones host del NODO (intrinsics): las llamadas a esos nombres se
    /// compilan vÃ­a el canal `env.host_call(id, ptr, n)`.
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
/// Backend WASM. Toma el type map `Span â†’ Type` por referencia (el caller â€”
/// `jit.rs` â€” mantiene el `TypeChecker` vivo durante la emisiÃ³n) para no clonar
/// el mapa en cada compilaciÃ³n.
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

    /// Backend con un target explÃ­cito (para `when` compile-time).
    pub fn with_target(types: &'a HashMap<Span, Type>, target: Target) -> Self {
        Self::with_options(types, target, WasmBackendOptions::default())
    }

    /// Backend con opciones explÃ­citas.
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

    /// Backend en modo librerÃ­a (sin `main` obligatorio): Ãºtil para `.clx`
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

    /// Backend en modo librerÃ­a Y sin excepciones (bindings browser).
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

    /// Igual que [`Self::emit`] pero ademÃ¡s devuelve el string pool final del
    /// mÃ³dulo (orden de interning, append-only). El REPL JIT lo usa para
    /// re-sembrar los mismos offsets en la sesiÃ³n siguiente (los punteros de
    /// strings transferidos entre instancias apuntan a esta regiÃ³n).
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

/// Motor de emisiÃ³n a nivel de mÃ³dulo.
fn collect_arrows_in_block(block: &Block, out: &mut Vec<ArrowFunctionExpr>) {
    for stmt in &block.statements {
        collect_arrows_in_stmt(stmt, out);
    }
}

fn collect_arrows_in_stmt(stmt: &Statement, out: &mut Vec<ArrowFunctionExpr>) {
    match stmt {
        Statement::VarDecl(v) | Statement::ConstDecl(v) => {
            if let Some(val) = &v.value {
                collect_arrows_in_expr(val, out);
            }
        }
        Statement::Expression(e) => collect_arrows_in_expr(e, out),
        Statement::Return(Some(e)) => collect_arrows_in_expr(e, out),
        Statement::If(i) => {
            collect_arrows_in_expr(&i.condition, out);
            collect_arrows_in_block(&i.then_block, out);
            for e in &i.elif_branches {
                collect_arrows_in_expr(&e.condition, out);
                collect_arrows_in_block(&e.block, out);
            }
            if let Some(eb) = &i.else_block {
                collect_arrows_in_block(eb, out);
            }
        }
        Statement::While(w) => {
            collect_arrows_in_expr(&w.condition, out);
            collect_arrows_in_block(&w.block, out);
        }
        Statement::For(f) => {
            if let Some(init) = &f.init {
                collect_arrows_in_stmt(init, out);
            }
            if let Some(cond) = &f.condition {
                collect_arrows_in_expr(cond, out);
            }
            if let Some(upd) = &f.update {
                collect_arrows_in_expr(upd, out);
            }
            collect_arrows_in_block(&f.block, out);
        }
        Statement::ForEach(fe) => {
            collect_arrows_in_expr(&fe.iterable, out);
            collect_arrows_in_block(&fe.block, out);
        }
        Statement::Switch(s) => {
            collect_arrows_in_expr(&s.value, out);
            for c in &s.cases {
                collect_arrows_in_block(&c.block, out);
            }
            if let Some(d) = &s.default {
                collect_arrows_in_block(d, out);
            }
        }
        Statement::With(w) => {
            collect_arrows_in_expr(&w.value, out);
            collect_arrows_in_block(&w.block, out);
        }
        Statement::Loop(b) => collect_arrows_in_block(b, out),
        _ => {}
    }
}

fn collect_arrows_in_expr(expr: &Expression, out: &mut Vec<ArrowFunctionExpr>) {
    match expr {
        Expression::ArrowFunction(a) => {
            out.push((*a).clone());
            collect_arrows_in_block(&a.body, out);
        }
        Expression::Call(c) => {
            collect_arrows_in_expr(&c.callee, out);
            for arg in &c.args {
                collect_arrows_in_expr(arg, out);
            }
        }
        Expression::MemberAccess(m) => collect_arrows_in_expr(&m.object, out),
        Expression::Index(i) => {
            collect_arrows_in_expr(&i.object, out);
            collect_arrows_in_expr(&i.index, out);
        }
        Expression::Array(a) => {
            for el in &a.elements {
                collect_arrows_in_expr(el, out);
            }
        }
        Expression::Tuple(t) => {
            for el in &t.elements {
                collect_arrows_in_expr(el, out);
            }
        }
        Expression::Record(r) => {
            for (_, v) in &r.entries {
                collect_arrows_in_expr(v, out);
            }
        }
        Expression::Binary(b) => {
            collect_arrows_in_expr(&b.left, out);
            collect_arrows_in_expr(&b.right, out);
        }
        Expression::Unary(u) => collect_arrows_in_expr(&u.operand, out),
        Expression::Conditional(c) => {
            collect_arrows_in_expr(&c.condition, out);
            collect_arrows_in_expr(&c.then_expr, out);
            collect_arrows_in_expr(&c.else_expr, out);
        }
        Expression::Assignment(a) => {
            collect_arrows_in_expr(&a.target, out);
            collect_arrows_in_expr(&a.value, out);
        }
        Expression::Parenthesized(e, _) => collect_arrows_in_expr(e, out),
        Expression::StringInterpolation(s) => {
            for part in &s.parts {
                if let InterpolationPart::Expr(e) = part {
                    collect_arrows_in_expr(e, out);
                }
            }
        }
        Expression::Cmx(c) => {
            for attr in &c.attributes {
                if let Some(CmxAttributeValue::Expression(e)) = &attr.value {
                    collect_arrows_in_expr(e, out);
                }
            }
            for child in &c.children {
                match child {
                    CmxChild::Expression(e) => collect_arrows_in_expr(e, out),
                    CmxChild::Element(el) => {
                        collect_arrows_in_expr(&Expression::Cmx((**el).clone()), out)
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
}

/// Recolecta los identifiers libres del body de una arrow (closures).
/// `locals` acumula params + variables declaradas dentro; `free` acumula los
/// identifiers que se usan pero no son params ni declarados localmente.
fn collect_free_vars_in_block(block: &Block, locals: &mut Vec<String>, free: &mut Vec<String>) {
    for stmt in &block.statements {
        collect_free_vars_in_stmt(stmt, locals, free);
    }
}

fn collect_free_vars_in_stmt(stmt: &Statement, locals: &mut Vec<String>, free: &mut Vec<String>) {
    match stmt {
        Statement::VarDecl(v) | Statement::ConstDecl(v) => {
            if let Some(val) = &v.value {
                collect_free_vars_in_expr(val, locals, free);
            }
            locals.push(v.name.clone());
        }
        Statement::Expression(e) => collect_free_vars_in_expr(e, locals, free),
        Statement::Return(Some(e)) => collect_free_vars_in_expr(e, locals, free),
        Statement::If(i) => {
            collect_free_vars_in_expr(&i.condition, locals, free);
            collect_free_vars_in_block(&i.then_block, locals, free);
            for e in &i.elif_branches {
                collect_free_vars_in_expr(&e.condition, locals, free);
                collect_free_vars_in_block(&e.block, locals, free);
            }
            if let Some(eb) = &i.else_block {
                collect_free_vars_in_block(eb, locals, free);
            }
        }
        Statement::While(w) => {
            collect_free_vars_in_expr(&w.condition, locals, free);
            collect_free_vars_in_block(&w.block, locals, free);
        }
        Statement::For(f) => {
            if let Some(init) = &f.init {
                collect_free_vars_in_stmt(init, locals, free);
            }
            if let Some(cond) = &f.condition {
                collect_free_vars_in_expr(cond, locals, free);
            }
            if let Some(upd) = &f.update {
                collect_free_vars_in_expr(upd, locals, free);
            }
            collect_free_vars_in_block(&f.block, locals, free);
        }
        Statement::ForEach(fe) => {
            collect_free_vars_in_expr(&fe.iterable, locals, free);
            locals.push(fe.item_name.clone());
            if let Some(iname) = &fe.index_name {
                locals.push(iname.clone());
            }
            collect_free_vars_in_block(&fe.block, locals, free);
        }
        Statement::Switch(s) => {
            collect_free_vars_in_expr(&s.value, locals, free);
            for c in &s.cases {
                collect_free_vars_in_block(&c.block, locals, free);
            }
            if let Some(d) = &s.default {
                collect_free_vars_in_block(d, locals, free);
            }
        }
        Statement::With(w) => {
            collect_free_vars_in_expr(&w.value, locals, free);
            locals.push(w.name.clone());
            collect_free_vars_in_block(&w.block, locals, free);
        }
        Statement::Loop(b) => collect_free_vars_in_block(b, locals, free),
        _ => {}
    }
}

fn collect_free_vars_in_expr(expr: &Expression, locals: &mut Vec<String>, free: &mut Vec<String>) {
    match expr {
        Expression::Identifier(name, _) => {
            if !locals.contains(name) && !free.contains(name) {
                free.push(name.clone());
            }
        }
        Expression::Call(c) => {
            collect_free_vars_in_expr(&c.callee, locals, free);
            for arg in &c.args {
                collect_free_vars_in_expr(arg, locals, free);
            }
        }
        Expression::MemberAccess(m) => collect_free_vars_in_expr(&m.object, locals, free),
        Expression::Index(i) => {
            collect_free_vars_in_expr(&i.object, locals, free);
            collect_free_vars_in_expr(&i.index, locals, free);
        }
        Expression::Array(a) => {
            for el in &a.elements {
                collect_free_vars_in_expr(el, locals, free);
            }
        }
        Expression::Tuple(t) => {
            for el in &t.elements {
                collect_free_vars_in_expr(el, locals, free);
            }
        }
        Expression::Record(r) => {
            for (_, v) in &r.entries {
                collect_free_vars_in_expr(v, locals, free);
            }
        }
        Expression::Binary(b) => {
            collect_free_vars_in_expr(&b.left, locals, free);
            collect_free_vars_in_expr(&b.right, locals, free);
        }
        Expression::Unary(u) => collect_free_vars_in_expr(&u.operand, locals, free),
        Expression::Conditional(c) => {
            collect_free_vars_in_expr(&c.condition, locals, free);
            collect_free_vars_in_expr(&c.then_expr, locals, free);
            collect_free_vars_in_expr(&c.else_expr, locals, free);
        }
        Expression::Assignment(a) => {
            collect_free_vars_in_expr(&a.target, locals, free);
            collect_free_vars_in_expr(&a.value, locals, free);
        }
        Expression::Parenthesized(e, _) => collect_free_vars_in_expr(e, locals, free),
        Expression::StringInterpolation(s) => {
            for part in &s.parts {
                if let InterpolationPart::Expr(e) = part {
                    collect_free_vars_in_expr(e, locals, free);
                }
            }
        }
        Expression::Cmx(c) => {
            for attr in &c.attributes {
                if let Some(CmxAttributeValue::Expression(e)) = &attr.value {
                    collect_free_vars_in_expr(e, locals, free);
                }
            }
            for child in &c.children {
                match child {
                    CmxChild::Expression(e) => collect_free_vars_in_expr(e, locals, free),
                    CmxChild::Element(el) => {
                        collect_free_vars_in_expr(&Expression::Cmx((**el).clone()), locals, free)
                    }
                    _ => {}
                }
            }
        }
        Expression::ArrowFunction(a) => {
            // Arrow anidada: sus variables libres tambiÃ©n son libres para la arrow
            // externa (el padre debe capturarlas para construir el handle interno).
            // Los params de la arrow interna se excluyen.
            let mut inner_locals: Vec<String> = a.params.iter().map(|p| p.name.clone()).collect();
            inner_locals.extend(locals.iter().cloned());
            collect_free_vars_in_block(&a.body, &mut inner_locals, free);
        }
        _ => {}
    }
}

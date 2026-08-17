#![no_std]
//! Módulo WASM de internals de CLS (compilado a `wasm32-unknown-unknown`).
//! `no_std`: sin runtime Rust (sin shadow stack ni data segment de std) — el
//! módulo queda reducido a las funciones `__intr_*` + bump allocator, lo que
//! hace trivial la fusión con el módulo CLS en la Fase 3.

extern crate alloc;

mod allocator;
mod arrays;
mod convert;
mod fmt;
mod math;
mod mem;
mod records;
mod strings;

#[no_mangle]
pub extern "C" fn __cls_panic_abort() -> ! {
    loop {
        core::arch::wasm32::unreachable()
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        core::arch::wasm32::unreachable()
    }
}

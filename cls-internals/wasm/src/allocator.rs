//! Allocator global del módulo de internals: delega en la función `__cls_alloc`
//! IMPORTADA del módulo CLS (el bump allocator del backend, `engine/globals.rs`).
//!
//! En el módulo standalone (tests wasmi) `__cls_alloc` la provee el linker del
//! test. En la fusión (Fase 3), el linker de módulos RESUELVE este import a la
//! función `__alloc` del módulo CLS → ambos comparten el MISMO bump allocator
//! (heap_ptr global 0 del CLS, inicia en 1MB): punteros compatibles, sin
//! colisión con strings/shadow stack, y CERO imports host en el módulo final.
//!
//! `__cls_alloc` tiene firma `(i32) -> i32` (wasm32: usize ↔ puntero). El linker
//! de fusión inserta un adaptador que convierte a la firma i64->i64 de `__alloc`.

extern "C" {
    /// Bump allocator del CLS: `(size: i32) -> ptr: i32` (0 si size <= 0).
    fn __cls_alloc(size: i32) -> i32;
}

/// Aloca `size` bytes vía el bump allocator del CLS (0 si size <= 0).
pub(crate) unsafe fn bump_alloc(size: usize) -> usize {
    if size == 0 {
        return 0;
    }
    __cls_alloc(size as i32) as usize
}

pub struct ClsAlloc;

unsafe impl core::alloc::GlobalAlloc for ClsAlloc {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        bump_alloc(layout.size()) as *mut u8
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: core::alloc::Layout) {
        // bump allocator: no hay free (paridad con el CLS).
    }

    unsafe fn realloc(
        &self,
        ptr: *mut u8,
        layout: core::alloc::Layout,
        new_size: usize,
    ) -> *mut u8 {
        let new_ptr = bump_alloc(new_size) as *mut u8;
        if !new_ptr.is_null() {
            core::ptr::copy_nonoverlapping(ptr, new_ptr, layout.size().min(new_size));
        }
        new_ptr
    }
}

#[global_allocator]
static ALLOC: ClsAlloc = ClsAlloc;

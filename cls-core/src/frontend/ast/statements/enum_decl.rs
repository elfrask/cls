//! AST - EnumDecl (Fase 1: extraido de frontend/ast.rs).
//!
//! F2: las variantes pueden llevar **payload** (`ok(String)`, `err(int, String)`).
//! Un enum cuyas variantes son **todas unit** se representa empacado en i64
//! `(def_id<<32)|idx`; si alguna variante tiene payload, **todos** los valores
//! del enum pasan a ser objetos de heap (ver `MATCH_PLAN.md` §3.1).

use crate::frontend::ast::*;
use crate::error::diagnostic::Span;
use serde::{Deserialize, Serialize};


/// Una variante de enum: `Rojo` (unit) o `ok(String)` / `err(int, String)`
/// (con payload). Los `params` son la **tupla de tipos** del payload; vacía = unit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumVariant {
    pub name: String,
    pub params: Vec<TypeAnnotation>,
    pub span: Span,
}

/// Declaración de enum: `enum Color { Rojo, Verde, Azul };`
/// Las variantes son constantes con identidad única (índice dentro del enum);
/// las variantes con payload (`ok(String)`) construyen objetos de heap.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumDecl {
    pub name: String,
    pub variants: Vec<EnumVariant>,
    pub span: Span,
    /// Visibilidad (export -> disponible en módulos importados)
    pub visibility: Visibility,
}

impl EnumDecl {
    /// ¿Alguna variante tiene payload? (define la representación del enum).
    pub fn has_payload(&self) -> bool {
        self.variants.iter().any(|v| !v.params.is_empty())
    }

    /// Nombres de las variantes en orden (para `(def_id, Vec<String>)`).
    pub fn variant_names(&self) -> Vec<String> {
        self.variants.iter().map(|v| v.name.clone()).collect()
    }
}

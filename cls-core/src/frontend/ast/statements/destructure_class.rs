//! AST - DestructureClass (desempaquetado de clases con `var`).
//!
//! `var Clase(a, b) = expr;` desempaqueta el payload que la clase expone con el
//! magic `__destructurate() -> T | (T1, T2, …)`. El retorno anota la forma:
//! 1 bind = valor único; N binds = tupla de aridad N. `_` ignora un slot.
//! El valor debe ser estáticamente de tipo `Clase` (o subclase).

use crate::error::diagnostic::Span;
use crate::frontend::ast::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DestructureClass {
    /// Clase que define el magic (`Clase`).
    pub class_name: String,
    /// Nombres ligados por posición al payload (`_` ignora el slot).
    pub binds: Vec<String>,
    /// Valor (de tipo `Clase`) del que se obtiene el payload.
    pub value: Expression,
    pub span: Span,
}

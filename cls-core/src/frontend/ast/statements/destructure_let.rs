//! AST - DestructureLet (feature `match`, Fase F2).
//!
//! `var resp::ok(result) = c;` — desempaqueta el payload de una variante de enum.
//!
//! **Decisión de provabilidad (F2)**: los enums son estáticos y los verifica el
//! compilador. La forma es válida cuando la variante es **demostrable** en
//! compile-time: el inicializador es una construcción directa de la MISMA
//! variante (`var c = resp::ok(x); var resp::ok(result) = c;`) o la variable
//! guarda una construcción registrada por el typeck. Si no se puede probar, es
//! **error de compilación** (no hay check de variante en runtime). `_` ignora un
//! slot del payload.

use crate::frontend::ast::*;
use crate::error::diagnostic::Span;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DestructureLet {
    /// Enum del que se desempaqueta (`resp`).
    pub enum_name: String,
    /// Variante esperada (`ok`).
    pub variant: String,
    /// Nombres ligados por posición al payload (`_` ignora el slot).
    pub binds: Vec<String>,
    /// Valor del que se extrae el payload.
    pub value: Expression,
    pub span: Span,
}

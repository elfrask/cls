//! AST - ArrowReturn (feature F4: `<-`).
//!
//! `<-` devuelve un valor desde un bloque al **target** léxico indicado por la
//! cantidad de estrellas:
//!
//! - `* <- expr`, `** <- expr`, ... -> **ternary-targets**: la rama de un
//!   `if`-expresión o el cuerpo de un arma de `match` (`-> { ... }`). `*` = el
//!   más cercano; `**` = salta 1; `***` = salta 2; ...
//! - `match <- expr`, `match* <- expr`, `match** <- expr`, ... -> **match-targets**:
//!   cada `match` envolvente. `match` = el más cercano; `match*` = salta 1; ...
//!
//! - `stars`: cantidad de `*`/`**` que siguen (0 para `match` sin estrellas).
//! - `is_match`: `true` si el target es un `match` (`match`/`match*`/...).
//! - El target del emisor se resuelve LÉXICAMENTE (misma función); no hay
//!   cross-function (descartado, ver `agent-context/MATCH_PLAN.md` §2.4).

use crate::frontend::ast::*;
use crate::error::diagnostic::Span;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArrowReturn {
    pub stars: usize,
    pub is_match: bool,
    pub value: Box<Expression>,
    pub span: Span,
}

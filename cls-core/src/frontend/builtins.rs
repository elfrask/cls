//! Declaraciones core inyectadas en todo módulo principal.
//!
//! Contiene la **clase `CMX`** (respaldada por el builtin `Cmx`): sus métodos
//! son CLS puro que opera sobre el valor nativo de 48B vía los campos del
//! builtin (`me.tag`, `me.isComponent`, `me.children`, ...). El typeck asocia
//! `CMX` con `Type::Cmx` (ver `middleware/typeck`), y el emisor los compila
//! como funciones `CMX::<metodo>` con `me` = ptr del Cmx.
//!
//! Centralizar la lógica acá evita tablas hardcodeadas en el emisor: agregar un
//! método a CMX = escribir CLS en `CMX_SRC`.

use crate::frontend::ast::Module;
use crate::frontend::{Lexer, Parser};

/// Offset de línea para los spans de la clase core inyectada (evita colisión con
/// el módulo principal, que ocupa líneas 1..N).
const CORE_SPAN_OFFSET: u32 = 5_000_000;

const CMX_SRC: &str = r#"
# Clase CMX (respaldada por el builtin `Cmx`). No se declaran campos: el acceso
# a `.tag/.props/.children/.kind/.tagCallable/.isComponent` viene del builtin.
class CMX {
    function isPrimitive() -> Bool {
        return !me.isComponent;
    }
    function isEmpty() -> Bool {
        return me.tag.length == 0;
    }
    function isElementOf(targetTag: String) -> Bool {
        return me.tag == targetTag;
    }
    function flattenChildren() -> Cmx[] {
        return me.children;
    }
    function tryCallTag() -> Value {
        if (me.isComponent) {
            return me.tagCallable(me.props, me.children);
        }
        return null;
    }
}
"#;

/// Inyecta las declaraciones core (clase `CMX`) al frente del módulo principal.
/// Idempotente respecto de parseo; no verifica duplicados (lo hace el typeck).
pub fn inject_builtins(module: &mut Module) {
    let toks = match Lexer::new(CMX_SRC).tokenize() {
        Ok(t) => t,
        Err(_) => return,
    };
    let mut m = match Parser::new(toks).parse() {
        Ok(m) => m,
        Err(_) => return,
    };
    // Desplazar los spans de la clase core para que NO colisionen con los del
    // módulo principal (el mapa span->tipo se pisaría entre `me.tag` del core y
    // el código del usuario en la misma línea/columna).
    crate::frontend::span_shift::shift_module(&mut m, CORE_SPAN_OFFSET);
    let mut stmts = m.statements;
    stmts.append(&mut module.statements);
    module.statements = stmts;
}

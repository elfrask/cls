//! Parte del motor de emision (Fase 1: extraido de engine/mod.rs).

use super::*;

pub(crate) struct FieldVis {
    pub(crate)is_private: bool,
    pub(crate)is_protected: bool,
    pub(crate)is_readonly: bool,
}

impl Clone for FieldVis {
    fn clone(&self) -> Self {
        *self
    }
}
impl Copy for FieldVis {}

impl FieldVis {
    pub(crate) fn is_private(&self) -> bool { self.is_private }
    pub(crate) fn is_protected(&self) -> bool { self.is_protected }
    pub(crate) fn is_readonly(&self) -> bool { self.is_readonly }
}

#[derive(Clone)]
pub(crate) struct ClassInfo {
    pub(crate)parent: Option<String>,
    /// id de clase (ÃƒÂ­ndice en orden de declaraciÃƒÂ³n) para `is` por herencia.
    pub(crate)class_id: u32,
    /// cadena de ancestors: [padre, abuelo, ...].
    pub(crate)ancestors: Vec<String>,
    /// campos (nombre, tipo CLS, tipo WASM, offset en bytes desde 16, visibilidad).
    pub(crate)fields: Vec<(String, Type, WasTy, i64, FieldVis)>,
    /// nombres de mÃƒÂ©todos en orden canÃƒÂ³nico (posiciÃƒÂ³n = slot de la vtable).
    pub(crate)methods: Vec<String>,
    /// visibilidad de cada mÃƒÂ©todo (private/protected/public) para enforzarla en
    /// llamadas desde fuera de la clase.
    pub(crate)method_vis: std::collections::HashMap<String, FieldVis>,
    /// ÃƒÂ­ndice de la tabla donde empieza la vtable de esta clase.
    pub(crate)vtable_start: u32,
    /// tamaÃƒÂ±o total del objeto (16 + campos).
    pub(crate)total: i64,
}


/// DefiniciÃƒÂ³n de un structure compilada: campos con tipos, offsets y tamaÃƒÂ±o.
#[derive(Clone)]
pub(crate) struct StructInfo {
    pub(crate)def_id: u32,
    /// campos (nombre, tipo CLS, tipo WASM).
    pub(crate)fields: Vec<(String, Type, WasTy)>,
    pub(crate)offsets: Vec<i64>,
    pub(crate)total: i64,
}

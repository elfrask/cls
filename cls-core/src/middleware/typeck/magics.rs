//! TypeChecker â€” dispatch de magic methods (Fase 1: extraido de middleware/typeck.rs).

use super::*;

impl TypeChecker {


    /// Tipo de retorno de un magic method (`__add`, `__len`, ...) si `ty` es una
    /// clase que lo define (incluye heredados ví­a `class_members`). `None` si no.
    pub(crate) fn named_magic_ret(&self, ty: &Type, magic: &str) -> Option<Type> {
        if let Type::Named(cn, _) = ty {
            if let Some(members) = self.class_members.get(cn.as_str()) {
                return members.get(magic).cloned();
            }
        }
        None
    }


    /// Parí¡metros de un método de clase (`ty` puede ser una subclase â€” se
    /// resuelven ví­a `magic_params`, que copia los del padre).
    pub(crate) fn magic_params_for(&self, ty: &Type, magic: &str) -> Option<Vec<Type>> {
        if let Type::Named(cn, _) = ty {
            if let Some(params) = self.magic_params.get(cn.as_str()) {
                return params.get(magic).cloned();
            }
        }
        None
    }


    /// Tipo del parí¡metro `idx` de un magic method, o `None`.
    pub(crate) fn magic_param(&self, ty: &Type, magic: &str, idx: usize) -> Option<Type> {
        self.magic_params_for(ty, magic)
            .and_then(|ps| ps.get(idx).cloned())
    }


    /// Â¿`ty` es asignable a `expected`, considerando la herencia de clases?
    /// (`Hijo` es asignable a `Base` â€” M2: un magic de la base recibe subclases).
    pub(crate) fn is_assignable_with_inheritance(&self, ty: &Type, expected: &Type) -> bool {
        if ty.is_assignable_to(expected) {
            return true;
        }
        if let (Type::Named(cn, _), Type::Named(en, _)) = (ty, expected) {
            let mut cur = self.class_parents.get(cn).cloned();
            while let Some(p) = cur {
                if p == *en {
                    return true;
                }
                cur = self.class_parents.get(&p).cloned();
            }
        }
        false
    }


    /// Valida el operando de un dispatch binario mí¡gico: (a) el tipo debe ser
    /// asignable al parí¡metro del magic (si no â†’ error claro, en vez de basura
    /// de memoria al interpretar el valor como ptr de objeto â€” M1/M4), y (b) el
    /// magic debe declarar exactamente 1 parí¡metro para un operador binario.
    pub(crate) fn validate_magic_binary_operand(&mut self, obj: &Type, operand: &Type, magic: &str, span: Span) {
        if let Some(param) = self.magic_param(obj, magic, 0) {
            if !self.is_assignable_with_inheritance(operand, &param) {
                self.error(
                    &format!(
                        "el operando {} no es asignable al parí¡metro de '{}' (esperaba {}, recibió {})",
                        operand,
                        magic,
                        param,
                        operand
                    ),
                    span,
                );
            }
        }
        if let Some(params) = self.magic_params_for(obj, magic) {
            if params.len() != 1 {
                self.error(
                    &format!(
                        "el magic '{}' debe declarar exactamente 1 parí¡metro para el operador binario (declaró {})",
                        magic,
                        params.len()
                    ),
                    span,
                );
            }
        }
    }

}
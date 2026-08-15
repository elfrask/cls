//! Classes: emit_class_method, magic methods (Fase 1: extraido de emitter/mod.rs).

use super::*;

impl<'a> FuncEmitter<'a> {


    /// Llama un mÃƒÂ©todo de clase por nombre (p.ej. `__type`/`__toJson`) sobre el
    /// objeto expresado. Devuelve `false` si la clase no define ese mÃƒÂ©todo.
    pub(crate) fn emit_class_method(&mut self, name: &str, object: &Expression) -> ClsResult<bool> {
        self.emit_class_method_args(name, object, &[])
    }


    /// Como [`Self::emit_class_method`] pero con argumentos: emite el objeto,
    /// lo guarda en un local, pushea `me`, emite los args y hace el
    /// call_indirect `(me, args...)` vÃƒÂ­a vtable. El orden de evaluaciÃƒÂ³n es
    /// objeto Ã¢â€ â€™ args (paridad walker). El stack del call_indirect es
    /// `[me, args..., fnptr]` (me al fondo).
    pub(crate) fn emit_class_method_args(
        &mut self,
        name: &str,
        object: &Expression,
        args: &[Expression],
    ) -> ClsResult<bool> {
        let obj_ty = self.types.get(&expr_span(object)).cloned();
        // M2: resolver la clase que DEFINE el mÃƒÂ©todo (sube por ancestors) Ã¢â‚¬â€
        // un magic heredado vive como `Base::__add`, no `Hijo::__add`.
        if let Some(dn) = self.class_magic_method(&obj_ty, name) {
            self.emit_expression(object)?;
            let obj_tmp = self.fresh_local();
            self.body.push(Instruction::LocalSet(obj_tmp));
            return self.emit_class_method_call_on(name, dn.as_str(), obj_tmp, args);
        }
        Ok(false)
    }


    /// Emite el call_indirect de un mÃƒÂ©todo de clase sobre el objeto en el local
    /// `obj_ptr`: pushea `me` (al fondo del stack), emite los args y despacha
    /// por vtable. El call_indirect espera `[me, args..., fnptr]`.
    /// Sube por `ancestors` para resolver la clase que define el mÃƒÂ©todo (M2).
    pub(crate) fn emit_class_method_call_on(
        &mut self,
        name: &str,
        class_name: &str,
        obj_ptr: u32,
        args: &[Expression],
    ) -> ClsResult<bool> {
        let mut cur = Some(class_name.to_string());
        while let Some(c) = cur {
            if let Some(info) = self.class_defs.get(&c) {
                if let Some(slot) = info.methods.iter().position(|m| m == name) {
                    let method_key = format!("{}::{}", c, name);
                    if let Some(&ty) = self.method_type_indexes.get(&method_key) {
                        // receiver (me) Ã¢â‚¬â€ al fondo; los args van DESPUÃƒâ€°S (el
                        // call_indirect los espera en orden: me, args...).
                        self.body.push(Instruction::LocalGet(obj_ptr));
                        for a in args {
                            self.emit_expression(a)?;
                        }
                        // vtable(obj[0]) + slot
                        self.body.push(Instruction::LocalGet(obj_ptr));
                        self.body.push(Instruction::I32WrapI64);
                        self.body.push(Instruction::I64Load(MemArg {
                            offset: 0,
                            align: 3,
                            memory_index: 0,
                        }));
                        self.body.push(Instruction::I64Const(slot as i64));
                        self.body.push(Instruction::I64Add);
                        self.body.push(Instruction::I32WrapI64);
                        self.body.push(Instruction::CallIndirect {
                            type_index: ty,
                            table_index: 0,
                        });
                        return Ok(true);
                    }
                }
                cur = info.ancestors.first().cloned();
            } else {
                break;
            }
        }
        Ok(false)
    }


    /// Ã‚Â¿El tipo (estÃƒÂ¡tico) es una clase que define el magic `name`? Devuelve el
    /// nombre de la clase que LO DEFINE (sube por `ancestors` Ã¢â‚¬â€ M2: un magic
    /// heredado se registra como `Base::__add`, no `Hijo::__add`). `None` si no.
    pub(crate) fn class_magic_method(&self, ty: &Option<Type>, name: &str) -> Option<String> {
        if let Some(Type::Named(cn, _)) = ty {
            let mut cur = Some(cn.clone());
            while let Some(c) = cur {
                if let Some(info) = self.class_defs.get(&c) {
                    if info.methods.iter().any(|m| m == name) {
                        return Some(c);
                    }
                    cur = info.ancestors.first().cloned();
                } else {
                    break;
                }
            }
        }
        None
    }


    /// Tipo CLS del retorno anotado de un mÃƒÂ©todo de clase (o `None` si no tiene).
    /// Sube por `ancestors` para los mÃƒÂ©todos heredados (M2).
    pub(crate) fn magic_ret_type(&self, class_name: &str, name: &str) -> Option<Type> {
        let mut cur = Some(class_name.to_string());
        while let Some(c) = cur {
            if let Some(t) = self
                .func_types
                .get(&format!("{}::{}", c, name))
                .and_then(|(_, r)| r.clone())
            {
                return Some(t);
            }
            cur = self.class_defs.get(&c).and_then(|i| i.ancestors.first().cloned());
        }
        None
    }


    /// WasTy del retorno de un magic: el JIT necesita el tipo anotado (distinto
    /// de void) para el dispatch (el call_indirect devuelve segÃƒÂºn la firma).
    pub(crate) fn magic_ret_was(&self, class_name: &str, name: &str) -> ClsResult<WasTy> {
        match self.magic_ret_type(class_name, name) {
            Some(t) if t != Type::Void => was_type(&t),
            _ => Err(crate::error::ClsError::CompileError(format!(
                "'{}::{}' debe anotar su tipo de retorno (distinto de void) para \
                 el dispatch del magic en el JIT",
                class_name, name
            ))),
        }
    }


    /// Dispatch de un magic binario: `left.__op(right)`, luego `right.__op(left)`
    /// (paridad walker `binary_magic`). Devuelve `Ok(Some(WasTy))` del retorno
    /// del mÃƒÂ©todo si se emitiÃƒÂ³, `Ok(None)` si ningÃƒÂºn lado define el magic.
    pub(crate) fn try_binary_magic(
        &mut self,
        left: &Expression,
        right: &Expression,
        magic: &str,
    ) -> ClsResult<Option<WasTy>> {
        let lty = self.types.get(&expr_span(left)).cloned();
        let rty = self.types.get(&expr_span(right)).cloned();
        if let Some(cn) = self.class_magic_method(&lty, magic) {
            let ret = self.magic_ret_was(&cn, magic)?;
            self.emit_class_method_args(magic, left, &[right.clone()])?;
            return Ok(Some(ret));
        }
        if let Some(cn) = self.class_magic_method(&rty, magic) {
            let ret = self.magic_ret_was(&cn, magic)?;
            self.emit_class_method_args(magic, right, &[left.clone()])?;
            return Ok(Some(ret));
        }
        Ok(None)
    }


    /// Variantes de un enum por nombre. Resuelve exacto (`Color`) o por sufijo
    /// (`lib::Color` cuando el typeck tipa la variante como `Named("Color")` pero
    /// el flatten registrÃƒÂ³ el enum prefijado).
    pub(crate) fn enum_variants(&self, name: &str) -> Option<&Vec<String>> {
        if let Some((_, v)) = self.enum_defs.get(name) {
            return Some(v);
        }
        let suffix = format!("::{}", name);
        self.enum_defs
            .iter()
            .find(|(k, _)| k.ends_with(&suffix))
            .map(|(_, (_, v))| v)
    }

}
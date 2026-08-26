//! Strings: to_string, print args, shape to json (Fase 1: extraido de emitter/mod.rs).

use super::*;

impl<'a> FuncEmitter<'a> {


    /// `__intr_str_*` si las internals están fusionadas (call directo por nombre);
    /// si no, fallback al host (misma firma/ABI, orden de stack idéntico).
    pub(crate) fn emit_str_host(&mut self, name: &str, host: HostFn) {
        if let Some(&idx) = self.func_indexes.get(name) {
            self.body.push(Instruction::Call(idx));
        } else {
            self.host.call(host, &mut self.body);
        }
    }


    /// Construye la representación `Punto { x: 3, y: 4 }` de un struct y la deja
    /// en el stack (el ptr del struct está en `ptr`).
    pub(crate) fn emit_struct_to_string(&mut self, name: &str, ptr: u32) -> ClsResult<()> {
        let info = self.struct_defs[name].clone();
        let open = format!("{} {{ ", name);
        let s = self.intern_string(&open);
        self.emit_load_str(s);
        let res = self.fresh_local();
        self.body.push(Instruction::LocalSet(res));
        for (i, (fname, t_cls, w)) in info.fields.iter().enumerate() {
            let label = format!("{}: ", fname);
            let ls = self.intern_string(&label);
            self.emit_load_str(ls);
            let lt = self.fresh_local();
            self.body.push(Instruction::LocalSet(lt));
            self.body.push(Instruction::LocalGet(res));
            self.body.push(Instruction::LocalGet(lt));
            self.emit_str_host("__intr_str_concat", HostFn::StrConcat);
            self.body.push(Instruction::LocalSet(res));
            self.body.push(Instruction::LocalGet(ptr));
            self.body.push(Instruction::I64Const(info.offsets[i]));
            self.body.push(Instruction::I64Add);
            self.body.push(Instruction::I32WrapI64);
            match w {
                WasTy::F64 => self.body.push(Instruction::F64Load(MemArg {
                    offset: 0,
                    align: 3,
                    memory_index: 0,
                })),
                WasTy::I32 => self.body.push(Instruction::I32Load(MemArg {
                    offset: 0,
                    align: 2,
                    memory_index: 0,
                })),
                WasTy::I64 => self.body.push(Instruction::I64Load(MemArg {
                    offset: 0,
                    align: 3,
                    memory_index: 0,
                })),
            }
            if matches!(t_cls, Type::String) {
                let q = self.intern_string("\"");
                self.emit_load_str(q);
                let qt = self.fresh_local();
                self.body.push(Instruction::LocalSet(qt));
                self.body.push(Instruction::LocalGet(res));
                self.body.push(Instruction::LocalGet(qt));
                self.emit_str_host("__intr_str_concat", HostFn::StrConcat);
                self.body.push(Instruction::LocalSet(res));
                let sv = self.fresh_local();
                self.body.push(Instruction::LocalSet(sv));
                self.body.push(Instruction::LocalGet(res));
                self.body.push(Instruction::LocalGet(sv));
                self.emit_str_host("__intr_str_concat", HostFn::StrConcat);
                self.body.push(Instruction::LocalSet(res));
                let q2 = self.intern_string("\"");
                self.emit_load_str(q2);
                let qt2 = self.fresh_local();
                self.body.push(Instruction::LocalSet(qt2));
                self.body.push(Instruction::LocalGet(res));
                self.body.push(Instruction::LocalGet(qt2));
                self.emit_str_host("__intr_str_concat", HostFn::StrConcat);
                self.body.push(Instruction::LocalSet(res));
            } else {
                match w {
                    WasTy::F64 => self.emit_str_host("__intr_str_float", HostFn::StrFloat),
                    _ => self.emit_str_host("__intr_str_int", HostFn::StrInt),
                }
                let sv = self.fresh_local();
                self.body.push(Instruction::LocalSet(sv));
                self.body.push(Instruction::LocalGet(res));
                self.body.push(Instruction::LocalGet(sv));
                self.emit_str_host("__intr_str_concat", HostFn::StrConcat);
                self.body.push(Instruction::LocalSet(res));
            }
            if i < info.fields.len() - 1 {
                let sep = self.intern_string(", ");
                self.emit_load_str(sep);
                let st = self.fresh_local();
                self.body.push(Instruction::LocalSet(st));
                self.body.push(Instruction::LocalGet(res));
                self.body.push(Instruction::LocalGet(st));
                self.emit_str_host("__intr_str_concat", HostFn::StrConcat);
                self.body.push(Instruction::LocalSet(res));
            }
        }
        let close = self.intern_string(" }");
        self.emit_load_str(close);
        let ct = self.fresh_local();
        self.body.push(Instruction::LocalSet(ct));
        self.body.push(Instruction::LocalGet(res));
        self.body.push(Instruction::LocalGet(ct));
        self.emit_str_host("__intr_str_concat", HostFn::StrConcat);
        self.body.push(Instruction::LocalSet(res));
        self.body.push(Instruction::LocalGet(res));
        Ok(())
    }



    pub(crate) fn emit_to_string(&mut self, arg: &Expression) -> ClsResult<()> {
        let span = expr_span(arg);
        let t = self.types.get(&span).cloned().unwrap_or(Type::Any);
        match t {
            Type::String => {}
            Type::Bool => self.emit_str_host("__intr_str_bool", HostFn::StrBool),
            Type::Char => self.emit_str_host("__intr_str_char", HostFn::StrChar),
            Type::Float => self.emit_str_host("__intr_str_float", HostFn::StrFloat),
            Type::Null => {
                // null -> string "null"
                self.body.push(Instruction::Drop);
                let n = self.intern_string("null");
                self.emit_load_str(n);
            }
            Type::Named(name, _) if self.struct_defs.contains_key(&name) => {
                let ptr = self.fresh_local();
                self.body.push(Instruction::LocalSet(ptr));
                self.emit_struct_to_string(&name, ptr)?;
            }
            Type::Named(name, _) if self.class_defs.contains_key(&name) => {
                // toString(obj) -> __toString si existe; si no, __repr; el ptr está en stack.
                if let Some(idx) = self.func_indexes.get(&format!("{}::__toString", name)) {
                    self.body.push(Instruction::Call(*idx));
                } else if let Some(idx) = self.func_indexes.get(&format!("{}::__repr", name)) {
                    self.body.push(Instruction::Call(*idx));
                } else {
                    self.emit_str_host("__intr_str_int", HostFn::StrInt);
                }
            }
            Type::Array(elem) => {
                // `[e1, e2, ...]` como el walker (paridad en interpolación).
                let w = was_type(&*elem)?;
                let kind = arr_kind_code(&*elem);
                let es = if matches!(*elem, Type::Cmx) {
                    16
                } else {
                    elem_size_bytes(w)
                };
                self.body.push(Instruction::I64Const(es));
                self.body.push(Instruction::I64Const(kind));
                self.host.call(HostFn::ArrToString, &mut self.body);
            }
            Type::Fun(..) => {
                // Handle de función -> `<function X>` (el nombre está en el handle).
                self.host.call(HostFn::FnToString, &mut self.body);
            }
            Type::Any | Type::Unknown | Type::Json | Type::Value => {
                // Valor dinámico (leído de record/JSON): el tag del runtime
                // decide la conversión. El caller dejó el val en el stack sin
                // tag; re-emitimos con emit_any_chain (val + tag) y despachamos
                // por tag. (str("x") con punteros ya no: se imprime el
                // contenido por tag string.)
                self.body.push(Instruction::Drop);
                self.emit_any_chain(arg)?;
                self.emit_str_host("__intr_any_to_string", HostFn::AnyToString);
            }
            _ => self.emit_str_host("__intr_str_int", HostFn::StrInt),
        }
        Ok(())
    }



    /// Convierte un valor WASM (ya en el stack) a string según su tipo CLS.
    /// No consume el ptr; lo usa directo para hosts de string.
    pub(crate) fn emit_was_to_string(&mut self, w: WasTy, cls_t: &Type) -> ClsResult<()> {
        match cls_t {
            Type::String => Ok(()),
            Type::Bool => {
                self.emit_str_host("__intr_str_bool", HostFn::StrBool);
                Ok(())
            }
            Type::Char => {
                self.emit_str_host("__intr_str_char", HostFn::StrChar);
                Ok(())
            }
            Type::Float => {
                self.emit_str_host("__intr_str_float", HostFn::StrFloat);
                Ok(())
            }
            Type::Array(_) | Type::Tuple(_) | Type::Record(_, _) | Type::Cmx => {
                // Contenedor anidado: imprimir como string de su tipo.
                let _ = w;
                self.emit_str_host("__intr_str_int", HostFn::StrInt);
                Ok(())
            }
            Type::Shape(_) => {
                // DEFAULT INVERTIDO: los shapes viven como hashmap en runtime ->
                // misma representación que un record: record_to_string.
                self.host.call(HostFn::RecordToString, &mut self.body);
                Ok(())
            }
            _ => {
                self.emit_str_host("__intr_str_int", HostFn::StrInt);
                Ok(())
            }
        }
    }



    /// `u.values()` sobre un shape -> string `[v1, v2, ...]` (keys ordenadas alf.).
    pub(crate) fn emit_shape_values_to_string(
        &mut self,
        m: &MemberAccessExpr,
        fields: &[(String, Type)],
    ) -> ClsResult<()> {
        self.emit_expression(&m.object)?;
        let ptr = self.fresh_local();
        self.body.push(Instruction::LocalSet(ptr));
        let open = self.intern_string("[");
        self.emit_load_str(open);
        let res = self.fresh_local();
        self.body.push(Instruction::LocalSet(res));
        let layout = self.shape_layout(fields)?;
        let mut ordered: Vec<&(String, WasTy, i64)> = layout.iter().collect();
        ordered.sort_by(|a, b| a.0.cmp(&b.0));
        for (i, (fname, w, off)) in ordered.iter().enumerate() {
            if i > 0 {
                let sep = self.intern_string(", ");
                self.emit_load_str(sep);
                let st = self.fresh_local();
                self.body.push(Instruction::LocalSet(st));
                self.body.push(Instruction::LocalGet(res));
                self.body.push(Instruction::LocalGet(st));
                self.emit_str_host("__intr_str_concat", HostFn::StrConcat);
                self.body.push(Instruction::LocalSet(res));
            }
            self.body.push(Instruction::LocalGet(ptr));
            self.body.push(Instruction::I64Const(*off));
            self.body.push(Instruction::I64Add);
            self.body.push(Instruction::I32WrapI64);
            match *w {
                WasTy::F64 => self.body.push(Instruction::F64Load(MemArg {
                    offset: 0,
                    align: 3,
                    memory_index: 0,
                })),
                WasTy::I32 => self.body.push(Instruction::I32Load(MemArg {
                    offset: 0,
                    align: 2,
                    memory_index: 0,
                })),
                WasTy::I64 => self.body.push(Instruction::I64Load(MemArg {
                    offset: 0,
                    align: 3,
                    memory_index: 0,
                })),
            }
            let cls_t = fields
                .iter()
                .find(|(n, _)| *n == *fname)
                .map(|(_, t)| t.clone())
                .unwrap_or(Type::Any);
            if matches!(cls_t, Type::String) {
                let q = self.intern_string("\"");
                self.emit_load_str(q);
                let qt = self.fresh_local();
                self.body.push(Instruction::LocalSet(qt));
                self.body.push(Instruction::LocalGet(res));
                self.body.push(Instruction::LocalGet(qt));
                self.emit_str_host("__intr_str_concat", HostFn::StrConcat);
                self.body.push(Instruction::LocalSet(res));
            }
            self.emit_was_to_string(*w, &cls_t)?;
            let vt = self.fresh_local();
            self.body.push(Instruction::LocalSet(vt));
            self.body.push(Instruction::LocalGet(res));
            self.body.push(Instruction::LocalGet(vt));
            self.emit_str_host("__intr_str_concat", HostFn::StrConcat);
            self.body.push(Instruction::LocalSet(res));
            if matches!(cls_t, Type::String) {
                let q = self.intern_string("\"");
                self.emit_load_str(q);
                let qt = self.fresh_local();
                self.body.push(Instruction::LocalSet(qt));
                self.body.push(Instruction::LocalGet(res));
                self.body.push(Instruction::LocalGet(qt));
                self.emit_str_host("__intr_str_concat", HostFn::StrConcat);
                self.body.push(Instruction::LocalSet(res));
            }
        }
        let close = self.intern_string("]");
        self.emit_load_str(close);
        let ct = self.fresh_local();
        self.body.push(Instruction::LocalSet(ct));
        self.body.push(Instruction::LocalGet(res));
        self.body.push(Instruction::LocalGet(ct));
        self.emit_str_host("__intr_str_concat", HostFn::StrConcat);
        self.host.call(HostFn::PrintStr, &mut self.body);
        Ok(())
    }



    /// `json.stringify(shape)` -> string JSON `{"k": v, ...}` (deja el string en stack).
    pub(crate) fn emit_shape_to_json_string(
        &mut self,
        expr: &Expression,
        fields: &[(String, Type)],
    ) -> ClsResult<()> {
        self.emit_expression(expr)?;
        let ptr = self.fresh_local();
        self.body.push(Instruction::LocalSet(ptr));
        let open = self.intern_string("{");
        self.emit_load_str(open);
        let res = self.fresh_local();
        self.body.push(Instruction::LocalSet(res));
        let layout = self.shape_layout(fields)?;
        let mut ordered: Vec<&(String, WasTy, i64)> = layout.iter().collect();
        ordered.sort_by(|a, b| a.0.cmp(&b.0));
        for (i, (fname, w, off)) in ordered.iter().enumerate() {
            if i > 0 {
                let sep = self.intern_string(",");
                self.emit_load_str(sep);
                let st = self.fresh_local();
                self.body.push(Instruction::LocalSet(st));
                self.body.push(Instruction::LocalGet(res));
                self.body.push(Instruction::LocalGet(st));
                self.emit_str_host("__intr_str_concat", HostFn::StrConcat);
                self.body.push(Instruction::LocalSet(res));
            }
            let key_json = format!("\"{}\":", fname);
            let ks = self.intern_string(&key_json);
            self.emit_load_str(ks);
            let kt = self.fresh_local();
            self.body.push(Instruction::LocalSet(kt));
            self.body.push(Instruction::LocalGet(res));
            self.body.push(Instruction::LocalGet(kt));
            self.emit_str_host("__intr_str_concat", HostFn::StrConcat);
            self.body.push(Instruction::LocalSet(res));
            self.body.push(Instruction::LocalGet(ptr));
            self.body.push(Instruction::I64Const(*off));
            self.body.push(Instruction::I64Add);
            self.body.push(Instruction::I32WrapI64);
            match *w {
                WasTy::F64 => self.body.push(Instruction::F64Load(MemArg {
                    offset: 0,
                    align: 3,
                    memory_index: 0,
                })),
                WasTy::I32 => self.body.push(Instruction::I32Load(MemArg {
                    offset: 0,
                    align: 2,
                    memory_index: 0,
                })),
                WasTy::I64 => self.body.push(Instruction::I64Load(MemArg {
                    offset: 0,
                    align: 3,
                    memory_index: 0,
                })),
            }
            let cls_t = fields
                .iter()
                .find(|(n, _)| *n == *fname)
                .map(|(_, t)| t.clone())
                .unwrap_or(Type::Any);
            // JSON: strings con comillas, ints/floats planos, bool true/false.
            if matches!(cls_t, Type::String) {
                let q = self.intern_string("\"");
                self.emit_load_str(q);
                let qt = self.fresh_local();
                self.body.push(Instruction::LocalSet(qt));
                self.body.push(Instruction::LocalGet(res));
                self.body.push(Instruction::LocalGet(qt));
                self.emit_str_host("__intr_str_concat", HostFn::StrConcat);
                self.body.push(Instruction::LocalSet(res));
                self.emit_was_to_string(*w, &cls_t)?;
                let vt = self.fresh_local();
                self.body.push(Instruction::LocalSet(vt));
                self.body.push(Instruction::LocalGet(res));
                self.body.push(Instruction::LocalGet(vt));
                self.emit_str_host("__intr_str_concat", HostFn::StrConcat);
                self.body.push(Instruction::LocalSet(res));
                let q2 = self.intern_string("\"");
                self.emit_load_str(q2);
                let q2t = self.fresh_local();
                self.body.push(Instruction::LocalSet(q2t));
                self.body.push(Instruction::LocalGet(res));
                self.body.push(Instruction::LocalGet(q2t));
                self.emit_str_host("__intr_str_concat", HostFn::StrConcat);
                self.body.push(Instruction::LocalSet(res));
            } else {
                match cls_t {
                    Type::Float => self.emit_str_host("__intr_str_float", HostFn::StrFloat),
                    Type::Bool => self.emit_str_host("__intr_str_bool", HostFn::StrBool),
                    _ => self.emit_str_host("__intr_str_int", HostFn::StrInt),
                }
                let vt = self.fresh_local();
                self.body.push(Instruction::LocalSet(vt));
                self.body.push(Instruction::LocalGet(res));
                self.body.push(Instruction::LocalGet(vt));
                self.emit_str_host("__intr_str_concat", HostFn::StrConcat);
                self.body.push(Instruction::LocalSet(res));
            }
        }
        let close = self.intern_string("}");
        self.emit_load_str(close);
        let ct = self.fresh_local();
        self.body.push(Instruction::LocalSet(ct));
        self.body.push(Instruction::LocalGet(res));
        self.body.push(Instruction::LocalGet(ct));
        self.emit_str_host("__intr_str_concat", HostFn::StrConcat);
        Ok(())
    }



    /// `[ptr]` en stack -> string del shape (recursivo para shapes anidados).
    pub(crate) fn emit_shape_field_to_string(&mut self, ptr: u32, fields: &[(String, Type)]) -> ClsResult<()> {
        let layout = self.shape_layout(fields)?;
        let open = self.intern_string("{");
        self.emit_load_str(open);
        let res = self.fresh_local();
        self.body.push(Instruction::LocalSet(res));
        let mut ordered: Vec<&(String, WasTy, i64)> = layout.iter().collect();
        ordered.sort_by(|a, b| a.0.cmp(&b.0));
        for (i, (fname, w, off)) in ordered.iter().enumerate() {
            if i > 0 {
                let sep = self.intern_string(", ");
                self.emit_load_str(sep);
                let st = self.fresh_local();
                self.body.push(Instruction::LocalSet(st));
                self.body.push(Instruction::LocalGet(res));
                self.body.push(Instruction::LocalGet(st));
                self.emit_str_host("__intr_str_concat", HostFn::StrConcat);
                self.body.push(Instruction::LocalSet(res));
            }
            let label = format!("{}: ", fname);
            let ls = self.intern_string(&label);
            self.emit_load_str(ls);
            let lt = self.fresh_local();
            self.body.push(Instruction::LocalSet(lt));
            self.body.push(Instruction::LocalGet(res));
            self.body.push(Instruction::LocalGet(lt));
            self.emit_str_host("__intr_str_concat", HostFn::StrConcat);
            self.body.push(Instruction::LocalSet(res));
            self.body.push(Instruction::LocalGet(ptr));
            self.body.push(Instruction::I64Const(*off));
            self.body.push(Instruction::I64Add);
            self.body.push(Instruction::I32WrapI64);
            match *w {
                WasTy::F64 => self.body.push(Instruction::F64Load(MemArg {
                    offset: 0,
                    align: 3,
                    memory_index: 0,
                })),
                WasTy::I32 => self.body.push(Instruction::I32Load(MemArg {
                    offset: 0,
                    align: 2,
                    memory_index: 0,
                })),
                WasTy::I64 => self.body.push(Instruction::I64Load(MemArg {
                    offset: 0,
                    align: 3,
                    memory_index: 0,
                })),
            }
            let cls_t = fields
                .iter()
                .find(|(n, _)| *n == *fname)
                .map(|(_, t)| t.clone())
                .unwrap_or(Type::Any);
            // Los strings de un shape se imprimen con comillas (paridad walker).
            if matches!(cls_t, Type::String) {
                let q = self.intern_string("\"");
                self.emit_load_str(q);
                let qt = self.fresh_local();
                self.body.push(Instruction::LocalSet(qt));
                self.body.push(Instruction::LocalGet(res));
                self.body.push(Instruction::LocalGet(qt));
                self.emit_str_host("__intr_str_concat", HostFn::StrConcat);
                self.body.push(Instruction::LocalSet(res));
            }
            self.emit_was_to_string(*w, &cls_t)?;
            let vt = self.fresh_local();
            self.body.push(Instruction::LocalSet(vt));
            self.body.push(Instruction::LocalGet(res));
            self.body.push(Instruction::LocalGet(vt));
            self.emit_str_host("__intr_str_concat", HostFn::StrConcat);
            self.body.push(Instruction::LocalSet(res));
            if matches!(cls_t, Type::String) {
                let q = self.intern_string("\"");
                self.emit_load_str(q);
                let qt = self.fresh_local();
                self.body.push(Instruction::LocalSet(qt));
                self.body.push(Instruction::LocalGet(res));
                self.body.push(Instruction::LocalGet(qt));
                self.emit_str_host("__intr_str_concat", HostFn::StrConcat);
                self.body.push(Instruction::LocalSet(res));
            }
        }
        let close = self.intern_string("}");
        self.emit_load_str(close);
        let ct = self.fresh_local();
        self.body.push(Instruction::LocalSet(ct));
        self.body.push(Instruction::LocalGet(res));
        self.body.push(Instruction::LocalGet(ct));
        self.emit_str_host("__intr_str_concat", HostFn::StrConcat);
        Ok(())
    }



    pub(crate) fn emit_to_int(&mut self, arg: &Expression) -> ClsResult<()> {
        let span = expr_span(arg);
        let t = self.types.get(&span).cloned().unwrap_or(Type::Any);
        match t {
            Type::Int => {}
            Type::Float => self.body.push(Instruction::I64TruncSatF64S),
            Type::Bool => self.body.push(Instruction::I64ExtendI32U),
            Type::String => {
                self.emit_call_site(&span);
                if let Some(&idx) = self.func_indexes.get("__intr_parse_int") {
                    self.body.push(Instruction::Call(idx));
                } else {
                    self.host.call(HostFn::ParseInt, &mut self.body)
                }
            }
            _ => {}
        }
        Ok(())
    }



    pub(crate) fn emit_to_float(&mut self, arg: &Expression) -> ClsResult<()> {
        let span = expr_span(arg);
        let t = self.types.get(&span).cloned().unwrap_or(Type::Any);
        match t {
            Type::Float => {}
            Type::Int => self.body.push(Instruction::F64ConvertI64S),
            Type::Bool => {
                self.body.push(Instruction::I64ExtendI32U);
                self.body.push(Instruction::F64ConvertI64S);
            }
            Type::String => {
                self.emit_call_site(&span);
                if let Some(&idx) = self.func_indexes.get("__intr_parse_float") {
                    self.body.push(Instruction::Call(idx));
                } else {
                    self.host.call(HostFn::ParseFloat, &mut self.body)
                }
            }
            _ => {}
        }
        Ok(())
    }



    pub(crate) fn emit_to_bool(&mut self, arg: &Expression) -> ClsResult<()> {
        // Reutiliza coerce_to_bool: la misma semántica de truthiness del walker
        // (int/float != 0, string len != 0, array/record len != 0, cmx/objetos
        // true). Antes los compuestos (cmx/array/record/any) caían en `_` y
        // dejaban el ptr i64 en el stack → `if (bool(x))` emitía WASM inválido.
        self.coerce_to_bool(arg)
    }

}

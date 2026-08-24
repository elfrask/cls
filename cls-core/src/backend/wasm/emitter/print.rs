//! print.rs (Fase 1: extraido de cls-core/src/backend/wasm/emitter/strings.rs).

use super::*;

impl<'a> FuncEmitter<'a> {



    /// Despacha el print de un campo de record heterogéneo según su tag.
    pub(crate) fn emit_print_record_field(&mut self, ptr_tmp: u32, key_tmp: u32) {
        self.body.push(Instruction::LocalGet(ptr_tmp));
        self.body.push(Instruction::LocalGet(key_tmp));
        if let Some(&idx) = self.func_indexes.get("__intr_record_get") {
            self.body.push(Instruction::Call(idx));
        } else {
            self.host.call(HostFn::RecordGet, &mut self.body);
        }
        let val_tmp = self.fresh_local();
        self.body.push(Instruction::LocalSet(val_tmp));
        self.body.push(Instruction::LocalGet(ptr_tmp));
        self.body.push(Instruction::LocalGet(key_tmp));
        if let Some(&idx) = self.func_indexes.get("__intr_record_tag") {
            self.body.push(Instruction::Call(idx));
        } else {
            self.host.call(HostFn::RecordTag, &mut self.body);
        }
        let tag_tmp = self.fresh_local();
        self.body.push(Instruction::LocalSet(tag_tmp));
        self.body.push(Instruction::LocalGet(val_tmp));
        self.body.push(Instruction::LocalGet(tag_tmp));
        self.host.call(HostFn::PrintAny, &mut self.body);
    }



    /// Formatea una tupla `(e0, e1, ...)` con repr (strings entre comillas), como
    /// el walker. El ptr de la tupla ya está en el stack.
    pub(crate) fn emit_tuple_to_string(&mut self, slots: &[Type], _arg: &Expression) -> ClsResult<()> {
        let ptr = self.fresh_local();
        self.body.push(Instruction::LocalSet(ptr));
        let open = self.intern_string("(");
        self.emit_load_str(open);
        let res = self.fresh_local();
        self.body.push(Instruction::LocalSet(res));
        for (i, slot) in slots.iter().enumerate() {
            if i > 0 {
                self.body.push(Instruction::LocalGet(res));
                let sep = self.intern_string(", ");
                self.emit_load_str(sep);
                self.emit_str_host("__intr_str_concat", HostFn::StrConcat);
                self.body.push(Instruction::LocalSet(res));
            }
            self.body.push(Instruction::LocalGet(ptr));
            self.body.push(Instruction::I64Const(16 + (i as i64) * 8));
            self.body.push(Instruction::I64Add);
            self.body.push(Instruction::I32WrapI64);
            let w = was_type(slot).unwrap_or(WasTy::I64);
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
            let val_tmp = self.fresh_local_ty(w);
            self.body.push(match w {
                WasTy::F64 => Instruction::LocalSet(val_tmp),
                WasTy::I32 => Instruction::LocalSet(val_tmp),
                WasTy::I64 => Instruction::LocalSet(val_tmp),
            });
            let sv = self.fresh_local();
            match slot {
                Type::String => {
                    self.body.push(Instruction::LocalGet(val_tmp));
                    self.emit_str_host("__intr_str_repr", HostFn::StrRepr);
                }
                Type::Float => {
                    self.body.push(Instruction::LocalGet(val_tmp));
                    self.emit_str_host("__intr_str_float", HostFn::StrFloat);
                }
                Type::Bool => {
                    self.body.push(Instruction::LocalGet(val_tmp));
                    self.emit_str_host("__intr_str_bool", HostFn::StrBool);
                }
                Type::Char => {
                    self.body.push(Instruction::LocalGet(val_tmp));
                    self.emit_str_host("__intr_str_char", HostFn::StrChar);
                }
                _ => {
                    self.body.push(Instruction::LocalGet(val_tmp));
                    self.emit_str_host("__intr_str_int", HostFn::StrInt);
                }
            }
            self.body.push(Instruction::LocalSet(sv));
            self.body.push(Instruction::LocalGet(res));
            self.body.push(Instruction::LocalGet(sv));
            self.emit_str_host("__intr_str_concat", HostFn::StrConcat);
            self.body.push(Instruction::LocalSet(res));
        }
        let close = self.intern_string(")");
        self.body.push(Instruction::LocalGet(res));
        self.emit_load_str(close);
        self.emit_str_host("__intr_str_concat", HostFn::StrConcat);
        self.body.push(Instruction::LocalSet(res));
        self.body.push(Instruction::LocalGet(res));
        Ok(())
    }



    pub(crate) fn emit_print_arg(&mut self, arg: &Expression) -> ClsResult<()> {        // `u.values()` sobre un record con shape -> imprimir `[v1, v2, ...]` inline
        // (el typeck da Array<Any>, no imprimible por el backend genérico).
        if let Expression::Call(c) = arg {
            if let Expression::MemberAccess(m) = &*c.callee {
                if m.member == "values" {
                    let obj_ty = self.types.get(&expr_span(&m.object)).cloned();
                    if let Some(Type::Shape(fields)) = &obj_ty {
                        return self.emit_shape_values_to_string(m, fields);
                    }
                }
            }
        }
        // Index de array de Cmx (`app.children[i]`): despachar por el tag del child
        // (el elemento puede ser cmx, string, array, int, ...).
        if let Expression::Index(ix) = arg {
            let obj_ty = self.types.get(&expr_span(&ix.object)).cloned();
            if matches!(obj_ty, Some(Type::Array(e)) if matches!(*e, Type::Cmx)) {
                self.emit_expression(&ix.object)?;
                self.emit_expression(&ix.index)?;
                let ptr = self.fresh_local();
                let idx = self.fresh_local();
                self.body.push(Instruction::LocalSet(idx));
                self.body.push(Instruction::LocalSet(ptr));
                self.bounds_check(ptr, idx, &ix.span);
                // addr = 16 + idx*16 -> val y tag
                self.body.push(Instruction::LocalGet(ptr));
                self.body.push(Instruction::LocalGet(idx));
                self.body.push(Instruction::I64Const(16));
                self.body.push(Instruction::I64Mul);
                self.body.push(Instruction::I64Const(16));
                self.body.push(Instruction::I64Add);
                self.body.push(Instruction::I64Add);
                self.body.push(Instruction::I32WrapI64);
                self.body.push(Instruction::I64Load(MemArg {
                    offset: 0,
                    align: 3,
                    memory_index: 0,
                }));
                let val_tmp = self.fresh_local();
                self.body.push(Instruction::LocalSet(val_tmp));
                self.body.push(Instruction::LocalGet(ptr));
                self.body.push(Instruction::LocalGet(idx));
                self.body.push(Instruction::I64Const(16));
                self.body.push(Instruction::I64Mul);
                self.body.push(Instruction::I64Const(24));
                self.body.push(Instruction::I64Add);
                self.body.push(Instruction::I64Add);
                self.body.push(Instruction::I32WrapI64);
                self.body.push(Instruction::I64Load(MemArg {
                    offset: 0,
                    align: 3,
                    memory_index: 0,
                }));
                let tag_tmp = self.fresh_local();
                self.body.push(Instruction::LocalSet(tag_tmp));
                self.body.push(Instruction::LocalGet(val_tmp));
                self.body.push(Instruction::LocalGet(tag_tmp));
                self.host.call(HostFn::PrintAny, &mut self.body);
                return Ok(());
            }
        }
        // Index sobre un record heterogéneo (value Any): imprimir según el tag del valor.
        if let Expression::Index(i) = arg {
            let obj_ty = self.types.get(&expr_span(&i.object)).cloned();
            if matches!(obj_ty, Some(Type::Record(_, _))) {
                self.emit_expression(&i.object)?;
                self.emit_expression(&i.index)?;
                let key_tmp = self.fresh_local();
                let ptr_tmp = self.fresh_local();
                self.body.push(Instruction::LocalSet(key_tmp));
                self.body.push(Instruction::LocalSet(ptr_tmp));
                self.emit_print_record_field(ptr_tmp, key_tmp);
                return Ok(());
            }
        }
        // Member access `record.campo` con value heterogéneo -> igual, por tag.
        if let Expression::MemberAccess(m) = arg {
            let obj_ty = self.types.get(&expr_span(&m.object)).cloned();
            if matches!(obj_ty, Some(Type::Record(_, _)))
                && !matches!(m.member.as_str(), "length" | "size")
            {
                self.emit_expression(&m.object)?;
                let ptr_tmp = self.fresh_local();
                self.body.push(Instruction::LocalSet(ptr_tmp));
                let k = self.intern_string(&m.member);
                self.emit_load_str(k);
                let key_tmp = self.fresh_local();
                self.body.push(Instruction::LocalSet(key_tmp));
                self.emit_print_record_field(ptr_tmp, key_tmp);
                return Ok(());
            }
            // `app.tag`: puede ser un string (tag minúscula) o un handle de función
            // (tag mayúscula). Despachar por tag-bit: handle (par O impar) =
            // bits altos cero; string CLS = (off<<32)|len (bits altos != 0).
            if matches!(obj_ty, Some(Type::Cmx)) && m.member == "tag" {
                self.emit_expression(&m.object)?;
                self.emit_cmx_field(0)?;
                let v = self.fresh_local();
                self.body.push(Instruction::LocalSet(v));
                // if (v>>32 == 0) && (v != 0) -> handle -> FnToString
                self.body.push(Instruction::LocalGet(v));
                self.body.push(Instruction::I64Const(32));
                self.body.push(Instruction::I64ShrU);
                self.body.push(Instruction::I64Eqz);
                self.body.push(Instruction::LocalGet(v));
                self.body.push(Instruction::I64Eqz);
                self.body.push(Instruction::I32Eqz);
                self.body.push(Instruction::I32And);
                self.block_depth += 1;
                self.body.push(Instruction::If(BlockType::Empty));
                self.body.push(Instruction::LocalGet(v));
                self.host.call(HostFn::FnToString, &mut self.body);
                self.host.call(HostFn::PrintStr, &mut self.body);
                self.body.push(Instruction::Else);
                self.body.push(Instruction::LocalGet(v));
                self.host.call(HostFn::PrintStr, &mut self.body);
                self.body.push(Instruction::End);
                self.block_depth -= 1;
                return Ok(());
            }
        }
        // Cadenas de acceso sobre `Any`/Record (json.parse anidado): `o.x[0]`,
        // `o.a.c`, `o.a.b[0]`. El objeto de la cadena tiene tipo `Any` o Record;
        // despachar por tag en runtime y formatear el valor (val, tag) real.
        if let Expression::Index(ix) = arg {
            let obj_ty = self.types.get(&expr_span(&ix.object)).cloned();
            if matches!(obj_ty, Some(Type::Any) | Some(Type::Json) | Some(Type::Value)) {
                self.emit_any_chain(arg)?;
                self.host.call(HostFn::PrintAny, &mut self.body);
                return Ok(());
            }
        }
        if let Expression::MemberAccess(m) = arg {
            let obj_ty = self.types.get(&expr_span(&m.object)).cloned();
            if matches!(obj_ty, Some(Type::Any) | Some(Type::Json) | Some(Type::Value)) {
                self.emit_any_chain(arg)?;
                self.host.call(HostFn::PrintAny, &mut self.body);
                return Ok(());
            }
        }
        self.emit_expression(arg)?;
        // json.stringify devuelve String (no un int): print_str.
        if let Expression::Call(c) = arg {
            if let Expression::MemberAccess(m) = &*c.callee {
                if let Expression::Identifier(o, _) = &*m.object {
                    if o == "json" && m.member == "stringify" {
                        self.host.call(HostFn::PrintStr, &mut self.body);
                        return Ok(());
                    }
                }
            }
        }
        // Llamadas a funciones nativas (extensión) -> tipo de retorno codificado.
        if let Expression::Call(c) = arg {
            if let Expression::Identifier(name, _) = &*c.callee {
                if let Some(rc) = self.native_ret.get(name) {
                    match rc {
                        'f' => self.host.call(HostFn::PrintFloat, &mut self.body),
                        's' => self.host.call(HostFn::PrintStr, &mut self.body),
                        'b' | 'c' => self.host.call(HostFn::PrintBool, &mut self.body),
                        _ => self.host.call(HostFn::PrintInt, &mut self.body),
                    }
                    return Ok(());
                }
            }
        }
        // Llamadas a módulos stdlib -> tipo de retorno conocido (print float/int).
        // math.range devuelve un array (el typeck no lo tipa): formatear `[..]`.
        if is_math_range_call(arg) {
            self.emit_expression(arg)?;
            self.body.push(Instruction::I64Const(8));
            self.body.push(Instruction::I64Const(0));
            if let Some(&idx) = self.func_indexes.get("__intr_arr_to_string") {
                self.body.push(Instruction::Call(idx));
            } else {
                self.host.call(HostFn::ArrToString, &mut self.body);
            }
            self.host.call(HostFn::PrintStr, &mut self.body);
            return Ok(());
        }
        // Los contenedores (array/record/cmx/tuple) los formatea el match de tipos.
        if let Some(w) = self.module_call_ret(arg) {
            let t = self
                .types
                .get(&expr_span(arg))
                .cloned()
                .unwrap_or(Type::Any);
            let is_container = matches!(
                t,
                Type::Array(_) | Type::Record(_, _) | Type::Cmx | Type::Tuple(_)
            );
            if !is_container {
                // El tipo real del span decide (String -> PrintStr; Float -> PrintFloat;
                // Bool -> PrintBool); para tipos sin información, usar el WasTy.
                match &t {
                    Type::String => {
                        self.host.call(HostFn::PrintStr, &mut self.body);
                        return Ok(());
                    }
                    Type::Bool => {
                        self.host.call(HostFn::PrintBool, &mut self.body);
                        return Ok(());
                    }
                    Type::Char => {
                        self.host.call(HostFn::PrintChar, &mut self.body);
                        return Ok(());
                    }
                    Type::Float => {
                        self.host.call(HostFn::PrintFloat, &mut self.body);
                        return Ok(());
                    }
                    _ => {}
                }
                match w {
                    WasTy::F64 => {
                        self.host.call(HostFn::PrintFloat, &mut self.body);
                        return Ok(());
                    }
                    WasTy::I32 => {
                        self.host.call(HostFn::PrintBool, &mut self.body);
                        return Ok(());
                    }
                    _ => {
                        self.host.call(HostFn::PrintInt, &mut self.body);
                        return Ok(());
                    }
                }
            }
        }
        let span = expr_span(arg);
        let t = self.types.get(&span).cloned().unwrap_or(Type::Any);
        match t {
            Type::String => self.host.call(HostFn::PrintStr, &mut self.body),
            Type::Bool => self.host.call(HostFn::PrintBool, &mut self.body),
            Type::Char => self.host.call(HostFn::PrintChar, &mut self.body),
            Type::Float => self.host.call(HostFn::PrintFloat, &mut self.body),
            Type::Null => {
                // `null` -> imprimir "null" (paridad walker).
                self.body.push(Instruction::Drop);
                let n = self.intern_string("null");
                self.emit_load_str(n);
                self.host.call(HostFn::PrintStr, &mut self.body);
            }
            Type::Array(elem) => {
                // Formatear `[e1, e2, ...]` como el walker (evita imprimir el ptr).
                let w = was_type(&*elem)?;
                let kind = arr_kind_code(&*elem);
                let es = if matches!(*elem, Type::Cmx) {
                    16
                } else {
                    elem_size_bytes(w)
                };
                self.body.push(Instruction::I64Const(es));
                self.body.push(Instruction::I64Const(kind));
                if let Some(&idx) = self.func_indexes.get("__intr_arr_to_string") {
                    self.body.push(Instruction::Call(idx));
                } else {
                    self.host.call(HostFn::ArrToString, &mut self.body);
                }
                self.host.call(HostFn::PrintStr, &mut self.body);
            }
            Type::Record(_, _) => {
                // Formatear `{k: v, ...}` como el walker (evita imprimir el ptr).
                if let Some(&idx) = self.func_indexes.get("__intr_record_to_string") {
                    self.body.push(Instruction::Call(idx));
                } else {
                    self.host.call(HostFn::RecordToString, &mut self.body);
                }
                self.host.call(HostFn::PrintStr, &mut self.body);
            }
            Type::Shape(fields) => {
                // Formatear `{k: v, ...}` (keys ordenadas alfabéticamente, paridad walker).
                let layout = self.shape_layout(&fields)?;
                let ptr = self.fresh_local();
                self.body.push(Instruction::LocalSet(ptr));
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
                    // valor del campo: load por offset + a string según el tipo del campo
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
                self.host.call(HostFn::PrintStr, &mut self.body);
            }
            Type::Tuple(slots) => {
                self.emit_tuple_to_string(&slots, arg)?;
                self.host.call(HostFn::PrintStr, &mut self.body);
            }
            Type::Named(name, _) if self.class_defs.contains_key(&name) => {
                // Si la clase define __repr -> usarlo (el ptr ya está en el stack).
                if let Some(idx) = self.func_indexes.get(&format!("{}::__repr", name)) {
                    self.body.push(Instruction::Call(*idx));
                    self.host.call(HostFn::PrintStr, &mut self.body);
                } else {
                    // Formatear `<Clase {campo: valor, ...}>` como el walker.
                    let info = self.class_defs[&name].clone();
                    let ptr = self.fresh_local();
                    self.body.push(Instruction::LocalSet(ptr));
                    let open = format!("<{} {{", name);
                    let s = self.intern_string(&open);
                    self.emit_load_str(s);
                    let res = self.fresh_local();
                    self.body.push(Instruction::LocalSet(res));
                    for (i, (fname, t_cls, w, off, _vis)) in info.fields.iter().enumerate() {
                        let label = format!("{}: ", fname);
                        let ls = self.intern_string(&label);
                        self.emit_load_str(ls);
                        let lt = self.fresh_local();
                        self.body.push(Instruction::LocalSet(lt));
                        self.body.push(Instruction::LocalGet(res));
                        self.body.push(Instruction::LocalGet(lt));
                        self.emit_str_host("__intr_str_concat", HostFn::StrConcat);
                        self.body.push(Instruction::LocalSet(res));
                        // valor
                        self.body.push(Instruction::LocalGet(ptr));
                        self.body.push(Instruction::I64Const(*off));
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
                            // el valor ya es un string (ptr<<32|len): concatenar directo
                        } else {
                            match w {
                                WasTy::F64 => self.emit_str_host("__intr_str_float", HostFn::StrFloat),
                                _ => self.emit_str_host("__intr_str_int", HostFn::StrInt),
                            }
                        }
                        let sv = self.fresh_local();
                        self.body.push(Instruction::LocalSet(sv));
                        self.body.push(Instruction::LocalGet(res));
                        self.body.push(Instruction::LocalGet(sv));
                        self.emit_str_host("__intr_str_concat", HostFn::StrConcat);
                        self.body.push(Instruction::LocalSet(res));
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
                    let close = self.intern_string("}>");
                    self.emit_load_str(close);
                    let ct = self.fresh_local();
                    self.body.push(Instruction::LocalSet(ct));
                    self.body.push(Instruction::LocalGet(res));
                    self.body.push(Instruction::LocalGet(ct));
                    self.emit_str_host("__intr_str_concat", HostFn::StrConcat);
                    self.body.push(Instruction::LocalSet(res));
                    self.body.push(Instruction::LocalGet(res));
                    self.host.call(HostFn::PrintStr, &mut self.body);
                }
            }
            Type::Cmx => {
                self.host.call(HostFn::CmxToString, &mut self.body);
                self.host.call(HostFn::PrintStr, &mut self.body);
            }
            Type::Fun(..) => {
                // Handle de función -> `<function X>` (el nombre está en el handle).
                self.host.call(HostFn::FnToString, &mut self.body);
                self.host.call(HostFn::PrintStr, &mut self.body);
            }
            Type::Named(name, _) if self.struct_defs.contains_key(&name) => {
                let ptr = self.fresh_local();
                self.body.push(Instruction::LocalSet(ptr));
                // Struct def como valor (ptr 0) -> `<function X>` (paridad walker).
                self.body.push(Instruction::LocalGet(ptr));
                self.body.push(Instruction::I64Eqz);
                self.block_depth += 1;
                self.body.push(Instruction::If(BlockType::Empty));
                let fs = self.intern_string(&format!("<function {}>", name));
                self.emit_load_str(fs);
                self.host.call(HostFn::PrintStr, &mut self.body);
                self.body.push(Instruction::Else);
                self.emit_struct_to_string(&name, ptr)?;
                self.host.call(HostFn::PrintStr, &mut self.body);
                self.body.push(Instruction::End);
                self.block_depth -= 1;
                return Ok(());
            }
            Type::Named(name, _) if self.enum_variants(&name).is_some() => {
                let variants = self.enum_variants(&name).unwrap().clone();
                // index = v & 0xffffffff -> seleccionar el string de la variante
                self.body.push(Instruction::I64Const(0xffff_ffff));
                self.body.push(Instruction::I64And);
                let idx = self.fresh_local();
                self.body.push(Instruction::LocalSet(idx));
                // Enum def como valor (index 0xffffffff) -> `<enum X>` (paridad walker).
                self.body.push(Instruction::LocalGet(idx));
                self.body.push(Instruction::I64Const(0xffff_ffff));
                self.body.push(Instruction::I64Eq);
                self.block_depth += 1;
                self.body.push(Instruction::If(BlockType::Empty));
                let eds = self.intern_string(&format!("<enum {}>", name));
                self.emit_load_str(eds);
                self.host.call(HostFn::PrintStr, &mut self.body);
                self.body.push(Instruction::Else);
                let n = variants.len();
                if n == 0 {
                    let s = self.intern_string("");
                    self.emit_load_str(s);
                    self.host.call(HostFn::PrintStr, &mut self.body);
                    self.body.push(Instruction::End);
                    self.block_depth -= 1;
                    return Ok(());
                }
                self.body.push(Instruction::LocalGet(idx));
                self.body.push(Instruction::I64Const(0));
                self.body.push(Instruction::I64Eq);
                self.block_depth += 1;
                self.body
                    .push(Instruction::If(BlockType::Result(ValType::I64)));
                let s0 = self.intern_string(&variants[0]);
                self.emit_load_str(s0);
                if n > 1 {
                    for (i, variant) in variants.iter().enumerate().skip(1) {
                        self.body.push(Instruction::Else);
                        if i == n - 1 {
                            let s = self.intern_string(variant);
                            self.emit_load_str(s);
                        } else {
                            self.body.push(Instruction::LocalGet(idx));
                            self.body.push(Instruction::I64Const(i as i64));
                            self.body.push(Instruction::I64Eq);
                            self.block_depth += 1;
                            self.body
                                .push(Instruction::If(BlockType::Result(ValType::I64)));
                            let s = self.intern_string(variant);
                            self.emit_load_str(s);
                        }
                    }
                    for _ in 0..(n - 1) {
                        self.body.push(Instruction::End);
                        self.block_depth -= 1;
                    }
                } else {
                    self.body.push(Instruction::End);
                    self.block_depth -= 1;
                }
                self.host.call(HostFn::PrintStr, &mut self.body);
                self.body.push(Instruction::End);
                self.block_depth -= 1;
                return Ok(());
            }
            Type::Union(_) => match union_base(&t) {
                Type::String => self.host.call(HostFn::PrintStr, &mut self.body),
                Type::Float => self.host.call(HostFn::PrintFloat, &mut self.body),
                Type::Bool => self.host.call(HostFn::PrintBool, &mut self.body),
                _ => self.host.call(HostFn::PrintInt, &mut self.body),
            },
            Type::Literal(l) => match l {
                LitVal::Str(_) => self.host.call(HostFn::PrintStr, &mut self.body),
                LitVal::Float(_) => self.host.call(HostFn::PrintFloat, &mut self.body),
                LitVal::Bool(_) => self.host.call(HostFn::PrintBool, &mut self.body),
                _ => self.host.call(HostFn::PrintInt, &mut self.body),
            },
            Type::Void | Type::Empty => {
                // `print("x", time.sleep(5))` -> imprime "void" (paridad walker).
                // La llamada void no deja valor en el stack: solo imprimir la etiqueta.
                let n = self.intern_string("void");
                self.emit_load_str(n);
                self.host.call(HostFn::PrintStr, &mut self.body);
            }
            _ => self.host.call(HostFn::PrintInt, &mut self.body),
        }
        Ok(())
    }

}
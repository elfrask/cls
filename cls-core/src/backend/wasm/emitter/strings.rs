//! Strings: to_string, print args, shape to json (Fase 1: extraido de emitter/mod.rs).

use super::*;

impl<'a> FuncEmitter<'a> {


    /// `__intr_str_*` (call directo por nombre). La fusión de internals es
    /// incondicional (emit.rs llama fuse_internals siempre), así que la
    /// internals existe; no hay fallback host (los hosts de str no se registran).
    pub(crate) fn emit_str_host(&mut self, name: &str) {
        if let Some(&idx) = self.func_indexes.get(name) {
            self.body.push(Instruction::Call(idx));
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
            self.emit_str_host("__intr_str_concat");
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
                self.emit_str_host("__intr_str_concat");
                self.body.push(Instruction::LocalSet(res));
                let sv = self.fresh_local();
                self.body.push(Instruction::LocalSet(sv));
                self.body.push(Instruction::LocalGet(res));
                self.body.push(Instruction::LocalGet(sv));
                self.emit_str_host("__intr_str_concat");
                self.body.push(Instruction::LocalSet(res));
                let q2 = self.intern_string("\"");
                self.emit_load_str(q2);
                let qt2 = self.fresh_local();
                self.body.push(Instruction::LocalSet(qt2));
                self.body.push(Instruction::LocalGet(res));
                self.body.push(Instruction::LocalGet(qt2));
                self.emit_str_host("__intr_str_concat");
                self.body.push(Instruction::LocalSet(res));
            } else {
                match w {
                    WasTy::F64 => self.emit_str_host("__intr_str_float"),
                    _ => self.emit_str_host("__intr_str_int"),
                }
                let sv = self.fresh_local();
                self.body.push(Instruction::LocalSet(sv));
                self.body.push(Instruction::LocalGet(res));
                self.body.push(Instruction::LocalGet(sv));
                self.emit_str_host("__intr_str_concat");
                self.body.push(Instruction::LocalSet(res));
            }
            if i < info.fields.len() - 1 {
                let sep = self.intern_string(", ");
                self.emit_load_str(sep);
                let st = self.fresh_local();
                self.body.push(Instruction::LocalSet(st));
                self.body.push(Instruction::LocalGet(res));
                self.body.push(Instruction::LocalGet(st));
                self.emit_str_host("__intr_str_concat");
                self.body.push(Instruction::LocalSet(res));
            }
        }
        let close = self.intern_string(" }");
        self.emit_load_str(close);
        let ct = self.fresh_local();
        self.body.push(Instruction::LocalSet(ct));
        self.body.push(Instruction::LocalGet(res));
        self.body.push(Instruction::LocalGet(ct));
        self.emit_str_host("__intr_str_concat");
        self.body.push(Instruction::LocalSet(res));
        self.body.push(Instruction::LocalGet(res));
        Ok(())
    }



    pub(crate) fn emit_to_string(&mut self, arg: &Expression) -> ClsResult<()> {
        let span = expr_span(arg);
        let t = self.types.get(&span).cloned().unwrap_or(Type::Any);
        match t {
            Type::String => {}
            Type::Bool => self.emit_str_host("__intr_str_bool"),
            Type::Char => self.emit_str_host("__intr_str_char"),
            Type::Float => self.emit_str_host("__intr_str_float"),
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
                    self.emit_str_host("__intr_str_int");
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
                // Bug fix dev-2 (Fase 7): antes llamaba `self.host.call(HostFn::ArrToString, ...)`
                // pero ese host ya no se importa (las internals están fusionadas en el
                // módulo). La internal correcta es `__intr_arr_to_string(ptr, es, kind)`.
                // Migracion Fase 3 (paso 3) dejo este path muerto.
                self.emit_str_host("__intr_arr_to_string");
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
                self.emit_str_host("__intr_any_to_string");
            }
            _ => self.emit_str_host("__intr_str_int"),
        }
        Ok(())
    }



    /// Frontera de append: emite la pieza como `(val:i64, tag:i64)` — el ABI de
    /// `__intr_str_append`. Escalares concretos emiten valor+tag estático;
    /// dinámicos usan el camino por tag runtime (emit_any_chain).
    pub(crate) fn emit_append_piece(&mut self, expr: &Expression) -> ClsResult<()> {
        let t = self.types.get(&expr_span(expr)).cloned().unwrap_or(Type::Any);
        if Self::is_dynamic_dest(&t) || matches!(t, Type::Fun(..) | Type::Named(..)) {
            return self.emit_any_chain(expr);
        }
        self.emit_expression(expr)?;
        match was_type(&t)? {
            WasTy::F64 => self.body.push(Instruction::I64ReinterpretF64),
            WasTy::I32 => self.body.push(Instruction::I64ExtendI32U),
            _ => {}
        }
        self.body.push(Instruction::I64Const(runtime_tag_code_compound(&t)));
        Ok(())
    }


    /// Secuencia de append: `old` en el stack + pieza (val, tag) ->
    /// `__intr_str_append` (internal fusionada o host como fallback).
    pub(crate) fn emit_str_append(
        &mut self,
        old: &Expression,
        piece: &Expression,
    ) -> ClsResult<()> {
        self.emit_expression(old)?;
        self.emit_append_piece(piece)?;
        self.emit_str_host("__intr_str_append");
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

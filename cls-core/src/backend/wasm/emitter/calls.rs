//! Calls: emit_call, host/module calls, math/fs/http/os/path/process/time/random (Fase 1: extraido de emitter/mod.rs).

use super::*;

impl<'a> FuncEmitter<'a> {


    pub(crate) fn emit_call(&mut self, c: &CallExpr) -> ClsResult<()> {
        // Constructor de structure: `Punto(3, 4)` -> alloc + stores.
        if let Expression::Identifier(name, _) = &*c.callee {
            if let Some(info) = self.struct_defs.get(name).cloned() {
                self.body.push(Instruction::I64Const(info.total));
                let alloc = self.func_indexes["__alloc"];
                self.body.push(Instruction::Call(alloc));
                let ptr = self.fresh_local();
                self.body.push(Instruction::LocalSet(ptr));
                self.body.push(Instruction::LocalGet(ptr));
                self.body.push(Instruction::I64Const(info.def_id as i64));
                self.emit_i64_store(0);
                self.body.push(Instruction::LocalGet(ptr));
                self.body
                    .push(Instruction::I64Const(info.fields.len() as i64));
                self.emit_i64_store(8);
                for (i, (_, _, w)) in info.fields.iter().enumerate() {
                    if i < c.args.len() {
                        self.emit_expression(&c.args[i])?;
                    } else {
                        self.body.push(Instruction::I64Const(0));
                    }
                    let val_tmp = self.fresh_local_ty(*w);
                    let addr_tmp = self.fresh_local();
                    self.body.push(match w {
                        WasTy::F64 => Instruction::LocalSet(val_tmp),
                        WasTy::I32 => Instruction::LocalSet(val_tmp),
                        WasTy::I64 => Instruction::LocalSet(val_tmp),
                    });
                    self.body.push(Instruction::LocalGet(ptr));
                    self.body.push(Instruction::I64Const(info.offsets[i]));
                    self.body.push(Instruction::I64Add);
                    self.body.push(Instruction::LocalSet(addr_tmp));
                    self.body.push(Instruction::LocalGet(addr_tmp));
                    self.body.push(Instruction::I32WrapI64);
                    self.body.push(match w {
                        WasTy::F64 => Instruction::LocalGet(val_tmp),
                        WasTy::I32 => Instruction::LocalGet(val_tmp),
                        WasTy::I64 => Instruction::LocalGet(val_tmp),
                    });
                    match w {
                        WasTy::F64 => self.body.push(Instruction::F64Store(MemArg {
                            offset: 0,
                            align: 3,
                            memory_index: 0,
                        })),
                        WasTy::I32 => self.body.push(Instruction::I32Store(MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        })),
                        WasTy::I64 => self.body.push(Instruction::I64Store(MemArg {
                            offset: 0,
                            align: 3,
                            memory_index: 0,
                        })),
                    }
                }
                self.body.push(Instruction::LocalGet(ptr));
                return Ok(());
            }
        }
        // Constructor de clase: `Clase(args)` -> alloc + vtable + init fields + ctor.
        if let Expression::Identifier(name, _) = &*c.callee {
            if self.class_defs.contains_key(name.as_str()) {
                self.emit_class_constructor(name, c)?;
                return Ok(());
            }
        }
        // Llamada a función nativa (extensión): import `env.<sym>__<sig>@<lib>`.
        if let Expression::Identifier(name, _) = &*c.callee {
            if let Some(idx) = self.native_indexes.get(name) {
                for a in &c.args {
                    self.emit_expression(a)?;
                }
                self.body.push(Instruction::Call(*idx));
                return Ok(());
            }
        }
        // Métodos de primitivos (callee MemberAccess) e intrinsics por nombre.
        if self.emit_primitive_method(c)? {
            return Ok(());
        }
        // `x::f(...)` - módulo/namespace importado: call directo a `x::f`.
        if let Expression::NamespaceAccess(ns, member, _) = &*c.callee {
            let key = format!("{}::{}", ns, member);
            // Constructor de clase namespaced: `lib::App(args)` -> la clase fue
            // flatteneada como `lib::App`.
            if self.class_defs.contains_key(&key) {
                self.emit_class_constructor(&key, c)?;
                return Ok(());
            }
            if let Some(fidx) = self.func_indexes.get(&key).copied() {
                let expected = self.func_types.get(&key).map(|(p, _)| p.clone());
                self.body.push(Instruction::I64Const(0)); // __capturas
                for (i, arg) in c.args.iter().enumerate() {
                    match &expected {
                        Some(ps) => self.emit_call_arg(arg, Some(ps), i)?,
                        None => self.emit_call_arg(arg, None, i)?,
                    }
                }
                self.emit_call_site(&c.span);
                self.body.push(Instruction::Call(fidx));
                return Ok(());
            }
            return Err(crate::error::ClsError::compile_at(
                &format!(
                    "El miembro '{}' no existe o no se exporta en el módulo '{}' (fase de emisión).",
                    member, ns
                ),
                &expr_span(&c.callee),
            ));
        }
        if let Expression::Identifier(name, _) = &*c.callee {
            if let Some(fidx) = self.func_indexes.get(name).copied() {
                let expected = self.func_types.get(name).map(|(p, _)| p.clone());
                // Firma uniforme (B5): las funciones CLS top-level reciben
                // __capturas (0) como primer arg. Internas y main no.
                if !name.starts_with("__") && name != "main" {
                    self.body.push(Instruction::I64Const(0));
                }
                for (i, arg) in c.args.iter().enumerate() {
                    match &expected {
                        Some(ps) => self.emit_call_arg(arg, Some(ps), i)?,
                        None => self.emit_call_arg(arg, None, i)?,
                    }
                }
                // Args faltantes -> valores por defecto (en el call site)
                if let Some(defaults) = self.func_defaults.get(name) {
                    let provided = c.args.len();
                    for d in defaults.iter().skip(provided) {
                        match d {
                            Some(expr) => self.emit_expression(expr)?,
                            None => self.body.push(Instruction::I64Const(0)),
                        }
                    }
                }
                self.emit_call_site(&c.span);
                self.body.push(Instruction::Call(fidx));
                return Ok(());
            }
            // Función host del nodo (intrinsic): canal `env.host_call(id, ptr, n)`.
            if let Some(intr) = self.intrinsics.get(name) {
                self.emit_host_call(intr, c)?;
                return Ok(());
            }
        }
        // Función como valor (variable con handle) -> call_indirect por tipo.
        let callee_ty = self.types.get(&expr_span(&c.callee)).cloned();
        // `handler(req, res)` con `handler: Any/Value` (callback almacenado en un
        // record/Any): call dinámico por handle. La firma es universal
        // `[capturas, i64...N] -> i64` (los args de objetos/records viajan como
        // ptr i64; el retorno es el valor como i64). El tag-bit del handle
        // decide si es closure (capturas en memoria) o función simple.
        if matches!(callee_ty, Some(Type::Any | Type::Unknown | Type::Value | Type::Json)) {
            let n = c.args.len();
            let mut pv_closure = vec![ValType::I64];
            pv_closure.extend(std::iter::repeat(ValType::I64).take(n));
            let tidx_closure = self.register_func_type(pv_closure, vec![ValType::I64]);
            self.emit_expression(&c.callee)?;
            let v = self.fresh_local();
            self.body.push(Instruction::LocalSet(v));
            // Rama closure (impar): capturas = handle[8].
            self.body.push(Instruction::LocalGet(v));
            self.body.push(Instruction::I64Const(1));
            self.body.push(Instruction::I64And);
            self.body.push(Instruction::I32WrapI64);
            self.block_depth += 1;
            self.body.push(Instruction::If(BlockType::Result(ValType::I64)));
            self.body.push(Instruction::LocalGet(v));
            self.body.push(Instruction::I64Const(1));
            self.body.push(Instruction::I64ShrU);
            self.body.push(Instruction::I64Const(8));
            self.body.push(Instruction::I64Add);
            self.body.push(Instruction::I32WrapI64);
            self.body.push(Instruction::I64Load(MemArg {
                offset: 0,
                align: 3,
                memory_index: 0,
            }));
            let caps_tmp = self.fresh_local();
            self.body.push(Instruction::LocalSet(caps_tmp));
            // Args (empujar capturas al fondo, luego los args).
            self.body.push(Instruction::LocalGet(caps_tmp));
            for a in &c.args {
                self.emit_call_arg(a, None, 0)?;
            }
            // El fnptr se obtiene del handle en memoria (offset 0).
            self.body.push(Instruction::LocalGet(v));
            self.body.push(Instruction::I64Const(1));
            self.body.push(Instruction::I64ShrU);
            self.body.push(Instruction::I32WrapI64);
            self.body.push(Instruction::I64Load(MemArg {
                offset: 0,
                align: 3,
                memory_index: 0,
            }));
            self.body.push(Instruction::I32WrapI64);
            self.emit_call_site(&c.span);
            self.body.push(Instruction::CallIndirect {
                type_index: tidx_closure,
                table_index: 0,
            });
            // Rama par (función simple): capturas = 0, tabla_idx = v>>1.
            self.body.push(Instruction::Else);
            self.body.push(Instruction::I64Const(0));
            for (i, a) in c.args.iter().enumerate() {
                self.emit_call_arg(a, None, i)?;
            }
            self.body.push(Instruction::LocalGet(v));
            self.body.push(Instruction::I64Const(1));
            self.body.push(Instruction::I64ShrU);
            self.body.push(Instruction::I32WrapI64);
            self.emit_call_site(&c.span);
            self.body.push(Instruction::CallIndirect {
                type_index: tidx_closure,
                table_index: 0,
            });
            self.body.push(Instruction::End);
            self.block_depth -= 1;
            return Ok(());
        }
        if let Some(Type::Fun(params, ret)) = callee_ty {
            let mut pv: Vec<ValType> = Vec::new();
            for t in &params {
                pv.push(was_type(t)?.val_type());
            }
            let rv: Vec<ValType> = match *ret {
                Type::Void => vec![],
                r => vec![was_type(&r)?.val_type()],
            };
            // Firma uniforme (B5): closure = [capturas(i64), params...].
            // Toda función CLS (top-level y arrows) se compila con el capturas
            // como primer param. El dispatch usa tag-bit: impar = closure (lee
            // el ptr de capturas del handle en memoria); par = función simple
            // (capturas = 0 literal, sin handle).
            let mut pv_closure = vec![ValType::I64];
            pv_closure.extend(pv.iter().copied());
            let tidx_closure = self.register_func_type(pv_closure, rv.clone());
            // v = eval(callee); valor con tag (par = simple, impar = closure).
            self.emit_expression(&c.callee)?;
            let v = self.fresh_local();
            self.body.push(Instruction::LocalSet(v));
            // block $done (resultado del call) -> cada rama hace call_indirect + br.
            let ret_block = if rv.is_empty() {
                BlockType::Empty
            } else {
                BlockType::Result(rv[0])
            };
            // tag = v & 1 -> condición del if (impar = closure). Convertir a i32.
            self.body.push(Instruction::LocalGet(v));
            self.body.push(Instruction::I64Const(1));
            self.body.push(Instruction::I64And);
            self.body.push(Instruction::I32WrapI64);
            self.block_depth += 1;
            self.body.push(Instruction::If(ret_block));
            // Rama closure (impar): ptr = v>>1; capturas = handle[8] (aplanado).
            self.body.push(Instruction::LocalGet(v));
            self.body.push(Instruction::I64Const(1));
            self.body.push(Instruction::I64ShrU);
            self.body.push(Instruction::I64Const(8));
            self.body.push(Instruction::I64Add);
            self.body.push(Instruction::I32WrapI64);
            self.body.push(Instruction::I64Load(MemArg {
                offset: 0,
                align: 3,
                memory_index: 0,
            }));
            let caps_tmp = self.fresh_local();
            self.body.push(Instruction::LocalSet(caps_tmp));
            // push [capturas, args..., tabla_idx]
            self.body.push(Instruction::LocalGet(caps_tmp));
            for (i, arg) in c.args.iter().enumerate() {
                self.emit_call_arg(arg, Some(&params), i)?;
            }
            // Params faltantes -> Null (0), como el walker (default o Null).
            for _ in c.args.len()..params.len() {
                self.body.push(Instruction::I64Const(0));
            }
            self.body.push(Instruction::LocalGet(v));
            self.body.push(Instruction::I64Const(1));
            self.body.push(Instruction::I64ShrU);
            self.body.push(Instruction::I32WrapI64);
            self.body.push(Instruction::I64Load(MemArg {
                offset: 0,
                align: 3,
                memory_index: 0,
            }));
            self.body.push(Instruction::I32WrapI64);
            self.emit_call_site(&c.span);
            self.body.push(Instruction::CallIndirect {
                type_index: tidx_closure,
                table_index: 0,
            });
            self.body.push(Instruction::Else);
            // Rama simple (par): tabla_idx = v>>1; push [capturas=0, args..., tabla_idx].
            self.body.push(Instruction::I64Const(0));
            for (i, arg) in c.args.iter().enumerate() {
                self.emit_call_arg(arg, Some(&params), i)?;
            }
            for _ in c.args.len()..params.len() {
                self.body.push(Instruction::I64Const(0));
            }
            self.body.push(Instruction::LocalGet(v));
            self.body.push(Instruction::I64Const(1));
            self.body.push(Instruction::I64ShrU);
            self.body.push(Instruction::I32WrapI64);
            self.emit_call_site(&c.span);
            self.body.push(Instruction::CallIndirect {
                type_index: tidx_closure,
                table_index: 0,
            });
            self.body.push(Instruction::End);
            self.block_depth -= 1;
            return Ok(());
        }
        // Magic __call: el callee es un objeto de clase con __call ->
        // obj(args...) = __call(obj, args...) (paridad walker interpreter.rs:1644).
        let callee_ty = self.types.get(&expr_span(&c.callee)).cloned();
        if let Some(cn) = self.class_magic_method(&callee_ty, "__call") {
            let _ = self.magic_ret_was(&cn, "__call")?;
            self.emit_expression(&c.callee)?;
            let obj_tmp = self.fresh_local();
            self.body.push(Instruction::LocalSet(obj_tmp));
            self.emit_class_method_call_on("__call", &cn, obj_tmp, &c.args)?;
            return Ok(());
        }
        // Objeto sin __call invocado como función -> error claro (paridad walker).
        if let Some(Type::Named(cn2, _)) = callee_ty {
            if self.class_defs.contains_key(cn2.as_str()) {
                return Err(crate::error::ClsError::compile_at(
                    &format!(
                        "El objeto de tipo '{}' no es callable (falta __call)",
                        cn2
                    ),
                    &c.span,
                ));
            }
        }
        Err(self.unsupported_expr(&Expression::Call(c.clone())))
    }

    /// Emite un argumento de llamada. Si el argumento es un record literal
    /// (Shape contiguo) y el parámetro esperado es dinámico (Record/JSON/Value/
    /// Any — o desconocido, p.ej. métodos de clase), lo convierte a HASHMAP:
    /// un shape contiguo no es legible como record por el callee (json.stringify,
    /// acceso por clave, stringify dinámico).
    pub(crate) fn emit_call_arg(
        &mut self,
        expr: &Expression,
        expected_params: Option<&[Type]>,
        idx: usize,
    ) -> ClsResult<()> {
        // Frontera única: delega en emit_coerce con el tipo esperado del param.
        let dest = expected_params.and_then(|ps| ps.get(idx));
        self.emit_coerce(expr, dest)
    }

    /// Constructor de clase: `Clase(args)` -> alloc + vtable + init fields + ctor.
    /// `name` puede ser el nombre simple (`App`) o prefijado (`lib::App`).
    fn emit_class_constructor(&mut self, name: &str, c: &CallExpr) -> ClsResult<()> {
        let info = self.class_defs[name].clone();
        self.body.push(Instruction::I64Const(info.total));
        let alloc = self.func_indexes["__alloc"];
        self.body.push(Instruction::Call(alloc));
        let obj = self.fresh_local();
        self.body.push(Instruction::LocalSet(obj));
        // vtable_ptr[0] = vtable_start, class_id[8] = id
        self.body.push(Instruction::LocalGet(obj));
        self.body
            .push(Instruction::I64Const(info.vtable_start as i64));
        self.emit_i64_store(0);
        self.body.push(Instruction::LocalGet(obj));
        self.body.push(Instruction::I64Const(info.class_id as i64));
        self.emit_i64_store(8);
        // init fields a 0
        for (_fn, _t, w, off, _vis) in &info.fields {
            self.body.push(Instruction::LocalGet(obj));
            self.body.push(Instruction::I64Const(*off));
            self.body.push(Instruction::I64Add);
            self.body.push(Instruction::I32WrapI64);
            match w {
                WasTy::F64 => self
                    .body
                    .push(Instruction::F64Const(Ieee64::new(0.0f64.to_bits()))),
                WasTy::I32 => self.body.push(Instruction::I32Const(0)),
                WasTy::I64 => self.body.push(Instruction::I64Const(0)),
            }
            match w {
                WasTy::F64 => self.body.push(Instruction::F64Store(MemArg {
                    offset: 0,
                    align: 3,
                    memory_index: 0,
                })),
                WasTy::I32 => self.body.push(Instruction::I32Store(MemArg {
                    offset: 0,
                    align: 2,
                    memory_index: 0,
                })),
                WasTy::I64 => self.body.push(Instruction::I64Store(MemArg {
                    offset: 0,
                    align: 3,
                    memory_index: 0,
                })),
            }
        }
        // call Clase::__ctor (o el del padre si no se define) con me.
        let callsite = c.span.clone();
        let mut cur = Some(name.to_string());
        while let Some(cls) = cur {
            if let Some(idx) = self.func_indexes.get(&format!("{}::__ctor", cls)) {
                self.body.push(Instruction::LocalGet(obj));
                for a in &c.args {
                    self.emit_expression(a)?;
                }
                self.emit_call_site(&callsite);
                self.body.push(Instruction::Call(*idx));
                break;
            }
            cur = self.class_defs.get(&cls).and_then(|i| i.parent.clone());
        }
        self.body.push(Instruction::LocalGet(obj));
        Ok(())
    }

}
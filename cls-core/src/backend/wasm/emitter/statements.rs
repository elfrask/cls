//! Statements: emit_statement, foreach, switch, try, if, while, loop, for (Fase 1: extraido de emitter/mod.rs).

use super::*;

impl<'a> FuncEmitter<'a> {


    // Ã¢â€â‚¬Ã¢â€â‚¬ EmisiÃƒÂ³n de statements Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬

    pub(crate) fn emit_statement(&mut self, stmt: &Statement) -> ClsResult<()> {
        match stmt {
            Statement::VarDecl(v) | Statement::ConstDecl(v) => {
                let ty = match (&v.type_ann, &v.value) {
                    (Some(ann), Some(val)) => match was_type(&annotation_to_type(ann)) {
                        Ok(w) => w,
                        // AnotaciÃƒÂ³n no resuelta (alias/unioÃƒÂ³n) Ã¢â€ â€™ tipo del valor.
                        Err(_) => self.value_type(val)?,
                    },
                    (Some(ann), None) => was_type(&annotation_to_type(ann))?,
                    (None, Some(val)) => self.value_type(val)?,
                    (None, None) => WasTy::I64,
                };
                let idx = self.declare_var_ty(&v.name, ty);
                if let Some(value) = &v.value {
                    self.emit_expression(value)?;
                    if self.promoted.contains(&v.name) {
                        // Variable promovida: alloc slot `[valor]`, guardar ptr en
                        // el local, store el valor en el slot.
                        let val_tmp = self.fresh_local_ty(ty);
                        self.body.push(match ty {
                            WasTy::F64 => Instruction::LocalSet(val_tmp),
                            WasTy::I32 => Instruction::LocalSet(val_tmp),
                            WasTy::I64 => Instruction::LocalSet(val_tmp),
                        });
                        self.body.push(Instruction::I64Const(8));
                        let alloc = self.func_indexes["__alloc"];
                        self.body.push(Instruction::Call(alloc));
                        self.body.push(Instruction::LocalSet(idx));
                        self.body.push(Instruction::LocalGet(idx));
                        self.body.push(Instruction::I32WrapI64);
                        self.body.push(match ty {
                            WasTy::F64 => Instruction::LocalGet(val_tmp),
                            WasTy::I32 => Instruction::LocalGet(val_tmp),
                            WasTy::I64 => Instruction::LocalGet(val_tmp),
                        });
                        match ty {
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
                    } else {
                        self.body.push(Instruction::LocalSet(idx));
                    }
                }
                Ok(())
            }
            Statement::FunctionDecl(_) => Ok(()),
            Statement::Expression(e) => {
                self.emit_expression(e)?;
                self.emit_drop(e)
            }
            Statement::Return(e) => {
                if e.is_some() {
                    self.emit_expression(e.as_ref().unwrap())?;
                }
                // Des-registrar el frame antes de cortar: `Instruction::Return`
                // salta al final sin pasar por el `fn_exit` del cuerpo.
                self.emit_fn_exit();
                self.body.push(Instruction::Return);
                Ok(())
            }
            Statement::Break(bspan) => {
                let ctx = self.loop_stack.last().ok_or_else(|| {
                    crate::error::ClsError::compile_at("break fuera de loop", bspan)
                })?;
                let depth = self.block_depth.saturating_sub(ctx.break_at);
                self.body.push(Instruction::Br(depth));
                Ok(())
            }
            Statement::Continue(cspan) => {
                let ctx = self.loop_stack.last().ok_or_else(|| {
                    crate::error::ClsError::compile_at("continue fuera de loop", cspan)
                })?;
                let depth = self.block_depth.saturating_sub(ctx.continue_at);
                self.body.push(Instruction::Br(depth));
                Ok(())
            }
            Statement::If(i) => self.emit_if(i),
            Statement::Try(t) => self.emit_try(t),
            Statement::While(w) => self.emit_while(w),
            Statement::Loop(b) => self.emit_loop(b),
            Statement::For(f) => self.emit_for(f),
            Statement::ForEach(fe) => self.emit_foreach(fe),
            Statement::Switch(s) => self.emit_switch(s),
            Statement::With(w) => self.emit_with(w),
            // `when` Ã¢â€ â€™ compile-time: emitir solo la rama que matchea el target actual.
            Statement::When(w) => {
                if let Some(branch) = w.branches.iter().find(|b| self.target.matches(&b.cond)) {
                    for st in &branch.block.statements {
                        self.emit_statement(st)?;
                    }
                }
                Ok(())
            }
            // Compile-time / no-runtime: alias, imports, interfaces, namespaces, config.
            Statement::TypeAlias(_)
            | Statement::Import(_)
            | Statement::FromImport(_)
            | Statement::Include(_)
            | Statement::InterfaceDecl(_)
            | Statement::NamespaceDecl(_)
            | Statement::ModuleDecl(_)
            | Statement::Config(_) => Ok(()),
            Statement::Cmx(c) => {
                self.emit_cmx(c)?;
                self.emit_drop(&Expression::Cmx(c.clone()))
            }
            other => Err(self.unsupported_stmt(other)),
        }
    }


    pub(crate) fn unsupported_stmt(&self, stmt: &Statement) -> crate::error::ClsError {
        crate::error::ClsError::CompileError(format!(
            "El JIT (subconjunto WASM) aÃƒÂºn no soporta este statement: {}",
            statement_display(stmt)
        ))
    }


    /// `arr.map(f)` Ã¢â‚¬â€ aplica la funciÃƒÂ³n (handle) a cada elemento y devuelve un
    /// array nuevo con los resultados. El array original YA estÃƒÂ¡ en el stack
    /// (lo emitiÃƒÂ³ el dispatch del mÃƒÂ©todo).
    pub(crate) fn emit_array_map(
        &mut self,
        _member: &MemberAccessExpr,
        c: &CallExpr,
        elem_ty: WasTy,
        elem_size: i64,
    ) -> ClsResult<()> {
        let arr_ptr = self.fresh_local();
        self.body.push(Instruction::LocalSet(arr_ptr));
        self.emit_expression(&c.args[0])?;
        let f_handle = self.fresh_local();
        self.body.push(Instruction::LocalSet(f_handle));
        // tipo de f Ã¢â€ â€™ Fun(params, ret)
        let ft = self
            .types
            .get(&expr_span(&c.args[0]))
            .cloned()
            .unwrap_or(Type::Any);
        let (f_params, f_ret) = match ft {
            Type::Fun(p, r) => (p, *r),
            _ => {
                return Err(crate::error::ClsError::CompileError(
                    "map: el argumento debe ser una funciÃƒÂ³n".to_string(),
                ))
            }
        };
        let ret_was = was_type(&f_ret).unwrap_or(WasTy::I64);
        let es_ret = elem_size_bytes(ret_was);
        let mut pv: Vec<ValType> = Vec::new();
        for t in &f_params {
            pv.push(was_type(t)?.val_type());
        }
        let rv: Vec<ValType> = match f_ret {
            Type::Void => vec![],
            r => vec![was_type(&r)?.val_type()],
        };
        // nuevo array [cap][len][ret...] del mismo tamaÃƒÂ±o que el original.
        let i = self.fresh_local();
        let new_ptr = self.fresh_local();
        self.body.push(Instruction::LocalGet(arr_ptr));
        self.body.push(Instruction::I32WrapI64);
        self.body.push(Instruction::I64Load(MemArg {
            offset: 8,
            align: 3,
            memory_index: 0,
        }));
        self.body.push(Instruction::LocalSet(i)); // n
        self.body.push(Instruction::I64Const(16));
        self.body.push(Instruction::LocalGet(i));
        self.body.push(Instruction::I64Const(es_ret));
        self.body.push(Instruction::I64Mul);
        self.body.push(Instruction::I64Add);
        let alloc = self.func_indexes["__alloc"];
        self.body.push(Instruction::Call(alloc));
        self.body.push(Instruction::LocalSet(new_ptr));
        // cap y len del nuevo array
        self.body.push(Instruction::LocalGet(new_ptr));
        self.body.push(Instruction::LocalGet(i));
        self.emit_i64_store(0);
        self.body.push(Instruction::LocalGet(new_ptr));
        self.body.push(Instruction::LocalGet(i));
        self.emit_i64_store(8);
        // loop i desde 0
        self.body.push(Instruction::I64Const(0));
        self.body.push(Instruction::LocalSet(i));
        self.block_depth += 1;
        self.body.push(Instruction::Block(BlockType::Empty));
        let break_at = self.block_depth;
        self.block_depth += 1;
        self.body.push(Instruction::Loop(BlockType::Empty));
        let loop_at = self.block_depth;
        // cond: i >= n
        self.body.push(Instruction::LocalGet(i));
        self.body.push(Instruction::LocalGet(new_ptr));
        self.body.push(Instruction::I32WrapI64);
        self.body.push(Instruction::I64Load(MemArg {
            offset: 8,
            align: 3,
            memory_index: 0,
        }));
        self.body.push(Instruction::I64GeS);
        let depth = self.block_depth.saturating_sub(break_at);
        self.body.push(Instruction::BrIf(depth));
        // addr del destino en el nuevo array Ã¢â€ â€™ guardar en local.
        self.body.push(Instruction::LocalGet(new_ptr));
        self.body.push(Instruction::LocalGet(i));
        self.body.push(Instruction::I64Const(es_ret));
        self.body.push(Instruction::I64Mul);
        self.body.push(Instruction::I64Const(16));
        self.body.push(Instruction::I64Add);
        self.body.push(Instruction::I64Add);
        let addr_tmp = self.fresh_local();
        self.body.push(Instruction::LocalSet(addr_tmp));
        // elem = arr[16 + i*elem_size] Ã¢â€ â€™ guardar en local.
        self.body.push(Instruction::LocalGet(arr_ptr));
        self.body.push(Instruction::LocalGet(i));
        self.body.push(Instruction::I64Const(elem_size));
        self.body.push(Instruction::I64Mul);
        self.body.push(Instruction::I64Const(16));
        self.body.push(Instruction::I64Add);
        self.body.push(Instruction::I64Add);
        self.body.push(Instruction::I32WrapI64);
        match elem_ty {
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
        let elem_tmp = self.fresh_local_ty(elem_ty);
        self.body.push(Instruction::LocalSet(elem_tmp));
        // llamar f(handle) con dispatch tag-bit (B5).
        let mut pv_caps = vec![ValType::I64];
        pv_caps.extend(pv.iter().copied());
        let tidx_caps = self.register_func_type(pv_caps, rv.clone());
        self.body.push(Instruction::LocalGet(f_handle));
        self.body.push(Instruction::I64Const(1));
        self.body.push(Instruction::I64And);
        self.body.push(Instruction::I32WrapI64);
        self.block_depth += 1;
        self.body.push(Instruction::If(if rv.is_empty() {
            BlockType::Empty
        } else {
            BlockType::Result(rv[0])
        }));
        // closure: push [capturas, elem, tabla]
        self.body.push(Instruction::LocalGet(f_handle));
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
        self.body.push(Instruction::LocalGet(caps_tmp));
        self.body.push(Instruction::LocalGet(elem_tmp));
        self.body.push(Instruction::LocalGet(f_handle));
        self.body.push(Instruction::I64Const(1));
        self.body.push(Instruction::I64ShrU);
        self.body.push(Instruction::I32WrapI64);
        self.body.push(Instruction::I64Load(MemArg {
            offset: 0,
            align: 3,
            memory_index: 0,
        }));
        self.body.push(Instruction::I32WrapI64);
        self.body.push(Instruction::CallIndirect {
            type_index: tidx_caps,
            table_index: 0,
        });
        self.body.push(Instruction::Else);
        // simple: push [capturas=0, elem, tabla]
        self.body.push(Instruction::I64Const(0));
        self.body.push(Instruction::LocalGet(elem_tmp));
        self.body.push(Instruction::LocalGet(f_handle));
        self.body.push(Instruction::I64Const(1));
        self.body.push(Instruction::I64ShrU);
        self.body.push(Instruction::I32WrapI64);
        self.body.push(Instruction::CallIndirect {
            type_index: tidx_caps,
            table_index: 0,
        });
        self.body.push(Instruction::End);
        self.block_depth -= 1;
        // store el resultado en [addr_tmp, result]: guardar resultado en local,
        // luego pushear addr y resultado en orden limpio.
        let res_tmp = self.fresh_local_ty(ret_was);
        self.body.push(Instruction::LocalSet(res_tmp));
        self.body.push(Instruction::LocalGet(addr_tmp));
        self.body.push(Instruction::I32WrapI64);
        self.body.push(Instruction::LocalGet(res_tmp));
        match ret_was {
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
        // i++
        self.body.push(Instruction::LocalGet(i));
        self.body.push(Instruction::I64Const(1));
        self.body.push(Instruction::I64Add);
        self.body.push(Instruction::LocalSet(i));
        let depth = self.block_depth.saturating_sub(loop_at);
        self.body.push(Instruction::Br(depth));
        self.body.push(Instruction::End);
        self.block_depth -= 1;
        self.body.push(Instruction::End);
        self.block_depth -= 1;
        self.body.push(Instruction::LocalGet(new_ptr));
        Ok(())
    }


    /// `for each x [and i] in (col)` sobre array/tuple.
    pub(crate) fn emit_foreach(&mut self, fe: &ForEachStatement) -> ClsResult<()> {
        // Enum: `for each v in (Nivel)` o `for each v in (lib::Color)` (namespaced)
        // Ã¢â€ â€™ loop 0..variants.len()
        let enum_key = match &fe.iterable {
            Expression::Identifier(name, _) => Some(name.clone()),
            Expression::NamespaceAccess(ns, name, _) => Some(format!("{}::{}", ns, name)),
            _ => None,
        };
        if let Some(key) = enum_key {
            if let Some((def_id, variants)) = self.enum_defs.get(&key).cloned() {
                let n = variants.len() as i64;
                let i = self.fresh_local();
                self.body.push(Instruction::I64Const(0));
                self.body.push(Instruction::LocalSet(i));
                let item_local = self.declare_var_ty(&fe.item_name, WasTy::I64);
                if let Some(iname) = &fe.index_name {
                    self.declare_var_ty(iname, WasTy::I64);
                }
                self.block_depth += 1;
                self.body.push(Instruction::Block(BlockType::Empty));
                let break_at = self.block_depth;
                self.block_depth += 1;
                self.body.push(Instruction::Loop(BlockType::Empty));
                // continue block: el `continue` salta aquÃƒÂ­ y ejecuta el incremento.
                self.block_depth += 1;
                self.body.push(Instruction::Block(BlockType::Empty));
                let continue_at = self.block_depth;
                self.loop_stack.push(LoopGuard {
                    break_at,
                    continue_at,
                });
                self.body.push(Instruction::LocalGet(i));
                self.body.push(Instruction::I64Const(n));
                self.body.push(Instruction::I64GeS);
                let depth = self.block_depth.saturating_sub(break_at);
                self.body.push(Instruction::BrIf(depth));
                self.body.push(Instruction::I64Const((def_id as i64) << 32));
                self.body.push(Instruction::LocalGet(i));
                self.body.push(Instruction::I64Or);
                self.body.push(Instruction::LocalSet(item_local));
                if let Some(iname) = &fe.index_name {
                    let idx_local = self.local_for(iname);
                    self.body.push(Instruction::LocalGet(i));
                    self.body.push(Instruction::LocalSet(idx_local));
                }
                for st in &fe.block.statements {
                    self.emit_statement(st)?;
                }
                // cerrar el continue block Ã¢â€ â€™ incremento
                self.body.push(Instruction::End);
                self.block_depth -= 1;
                self.body.push(Instruction::LocalGet(i));
                self.body.push(Instruction::I64Const(1));
                self.body.push(Instruction::I64Add);
                self.body.push(Instruction::LocalSet(i));
                let depth = self.block_depth.saturating_sub(continue_at - 1);
                self.body.push(Instruction::Br(depth));
                self.body.push(Instruction::End);
                self.block_depth -= 1;
                self.body.push(Instruction::End);
                self.block_depth -= 1;
                self.loop_stack.pop();
                return Ok(());
            }
        }
        let iterable_ty = self
            .types
            .get(&expr_span(&fe.iterable))
            .cloned()
            .unwrap_or(Type::Any);
        // Magic methods __iter/__next (paridad walker interpreter.rs:723-767):
        // __iter() Ã¢â€ â€™ Array (caso 1) u objeto iterador con __next() hasta null
        // (caso 2). El tipo del iterable debe ser una clase con __iter.
        if let Some(cn) = self.class_magic_method(&Some(iterable_ty.clone()), "__iter") {
            return self.emit_foreach_magic(fe, &cn, &iterable_ty);
        }
        let (elem_ty, elem_size) = match &iterable_ty {
            Type::Array(elem) => {
                let w = was_type(elem)?;
                // Array de Cmx Ã¢â€ â€™ entradas `[val, tag]` stride 16.
                let es = if matches!(**elem, Type::Cmx) {
                    16
                } else {
                    elem_size_bytes(w)
                };
                (w, es)
            }
            Type::Tuple(slots) => {
                let w = slots.first().map(was_type).unwrap_or(Ok(WasTy::I64))?;
                (w, 8)
            }
            _ => {
                return Err(crate::error::ClsError::CompileError(
                    "for each solo soporta arrays y tuplas en el JIT (por ahora)".to_string(),
                ))
            }
        };
        self.emit_expression(&fe.iterable)?;
        let iter = self.fresh_local();
        self.body.push(Instruction::LocalSet(iter));
        self.emit_foreach_array_loop(iter, elem_ty, elem_size, fe)
    }


    /// Magic __iter/__next: `it = obj.__iter()`; si devuelve Array Ã¢â€ â€™ loop nativo;
    /// si devuelve una clase iteradora Ã¢â€ â€™ `it.__next()` hasta `null` (0 en el JIT).
    pub(crate) fn emit_foreach_magic(
        &mut self,
        fe: &ForEachStatement,
        cn: &str,
        _iterable_ty: &Type,
    ) -> ClsResult<()> {
        self.emit_class_method_args("__iter", &fe.iterable, &[])?;
        let iter = self.fresh_local();
        self.body.push(Instruction::LocalSet(iter));
        match self.magic_ret_type(cn, "__iter") {
            // Caso 1: __iter devolviÃƒÂ³ un Array Ã¢â€ â€™ iterar con el loop nativo.
            Some(Type::Array(elem)) => {
                let w = was_type(&*elem)?;
                let es = if matches!(*elem, Type::Cmx) {
                    16
                } else {
                    elem_size_bytes(w)
                };
                self.emit_foreach_array_loop(iter, w, es, fe)
            }
            // Caso 2: objeto iterador Ã¢â€ â€™ __next() hasta null.
            Some(Type::Named(it_cn, _)) => self.emit_foreach_next_loop(iter, &it_cn, fe),
            _ => Err(crate::error::ClsError::CompileError(format!(
                "'{}::__iter' debe anotar su retorno (Array<X> o una clase iteradora \
                 con __next) para el for each en el JIT",
                cn
            ))),
        }
    }


    /// Loop nativo de `for each`: `iter` (ptr de array ya en local) + contador.
    pub(crate) fn emit_foreach_array_loop(
        &mut self,
        iter: u32,
        elem_ty: WasTy,
        elem_size: i64,
        fe: &ForEachStatement,
    ) -> ClsResult<()> {
        let i = self.fresh_local();
        self.body.push(Instruction::I64Const(0));
        self.body.push(Instruction::LocalSet(i));
        let item_local = self.declare_var_ty(&fe.item_name, elem_ty);
        if let Some(iname) = &fe.index_name {
            self.declare_var_ty(iname, WasTy::I64);
        }
        let d = self.block_depth;
        self.block_depth += 1;
        self.body.push(Instruction::Block(BlockType::Empty));
        let break_at = self.block_depth;
        self.block_depth += 1;
        self.body.push(Instruction::Loop(BlockType::Empty));
        // continue block: el `continue` salta aquÃƒÂ­ y ejecuta el incremento.
        self.block_depth += 1;
        self.body.push(Instruction::Block(BlockType::Empty));
        let continue_at = self.block_depth;
        self.loop_stack.push(LoopGuard {
            break_at,
            continue_at,
        });
        // cond: i >= len(iter)
        self.body.push(Instruction::LocalGet(i));
        self.body.push(Instruction::LocalGet(iter));
        self.body.push(Instruction::I32WrapI64);
        self.body.push(Instruction::I64Load(MemArg {
            offset: 8,
            align: 3,
            memory_index: 0,
        }));
        self.body.push(Instruction::I64GeS);
        let depth = self.block_depth.saturating_sub(break_at);
        self.body.push(Instruction::BrIf(depth));
        // item = iter[i]
        self.body.push(Instruction::LocalGet(iter));
        self.body.push(Instruction::LocalGet(i));
        self.body.push(Instruction::I64Const(elem_size));
        self.body.push(Instruction::I64Mul);
        self.body.push(Instruction::I64Const(16));
        self.body.push(Instruction::I64Add);
        self.body.push(Instruction::I64Add);
        self.body.push(Instruction::I32WrapI64);
        match elem_ty {
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
        self.body.push(match elem_ty {
            WasTy::F64 => Instruction::LocalSet(item_local),
            WasTy::I32 => Instruction::LocalSet(item_local),
            WasTy::I64 => Instruction::LocalSet(item_local),
        });
        if let Some(iname) = &fe.index_name {
            let idx_local = self.local_for(iname);
            self.body.push(Instruction::LocalGet(i));
            self.body.push(Instruction::LocalSet(idx_local));
        }
        for st in &fe.block.statements {
            self.emit_statement(st)?;
        }
        // cerrar el continue block Ã¢â€ â€™ i++
        self.body.push(Instruction::End);
        self.block_depth -= 1;
        self.body.push(Instruction::LocalGet(i));
        self.body.push(Instruction::I64Const(1));
        self.body.push(Instruction::I64Add);
        self.body.push(Instruction::LocalSet(i));
        let depth = self.block_depth.saturating_sub(continue_at - 1);
        self.body.push(Instruction::Br(depth));
        self.body.push(Instruction::End); // loop
        self.block_depth -= 1;
        self.body.push(Instruction::End); // block
        self.block_depth -= 1;
        self.loop_stack.pop();
        let _ = d;
        Ok(())
    }


    /// Loop del iterador: `v = it.__next()`; si `v == 0` (null) Ã¢â€ â€™ break; si no,
    /// item = v, index = i, cuerpo, i++.
    pub(crate) fn emit_foreach_next_loop(&mut self, iter: u32, it_cn: &str, fe: &ForEachStatement) -> ClsResult<()> {
        let item_was = match self.magic_ret_type(it_cn, "__next") {
            Some(t) if t != Type::Void => was_type(&t)?,
            _ => {
                return Err(crate::error::ClsError::CompileError(format!(
                    "'{}::__next' debe anotar su tipo de retorno (distinto de void) \
                     para el for each en el JIT",
                    it_cn
                )))
            }
        };
        let item_local = self.declare_var_ty(&fe.item_name, item_was);
        if let Some(iname) = &fe.index_name {
            self.declare_var_ty(iname, WasTy::I64);
        }
        let i = self.fresh_local();
        self.body.push(Instruction::I64Const(0));
        self.body.push(Instruction::LocalSet(i));
        self.block_depth += 1;
        self.body.push(Instruction::Block(BlockType::Empty));
        let break_at = self.block_depth;
        self.block_depth += 1;
        self.body.push(Instruction::Loop(BlockType::Empty));
        // continue block: el `continue` salta aquÃƒÂ­ y ejecuta el incremento.
        self.block_depth += 1;
        self.body.push(Instruction::Block(BlockType::Empty));
        let continue_at = self.block_depth;
        self.loop_stack.push(LoopGuard {
            break_at,
            continue_at,
        });
        // v = it.__next()
        self.emit_class_method_call_on("__next", it_cn, iter, &[])?;
        let v = self.fresh_local_ty(item_was);
        self.body.push(match item_was {
            WasTy::F64 => Instruction::LocalSet(v),
            WasTy::I32 => Instruction::LocalSet(v),
            WasTy::I64 => Instruction::LocalSet(v),
        });
        // if v == null (sentinel del protocolo __next) Ã¢â€ â€™ break
        self.body.push(Instruction::LocalGet(v));
        match item_was {
            WasTy::I32 => self.body.push(Instruction::I32Eqz),
            _ => {
                self.body.push(Instruction::I64Const(NULL_ITER_SENTINEL));
                self.body.push(Instruction::I64Eq);
            }
        }
        let depth = self.block_depth.saturating_sub(break_at);
        self.body.push(Instruction::BrIf(depth));
        // item = v; index = i
        self.body.push(Instruction::LocalGet(v));
        self.body.push(match item_was {
            WasTy::F64 => Instruction::LocalSet(item_local),
            WasTy::I32 => Instruction::LocalSet(item_local),
            WasTy::I64 => Instruction::LocalSet(item_local),
        });
        if let Some(iname) = &fe.index_name {
            let idx_local = self.local_for(iname);
            self.body.push(Instruction::LocalGet(i));
            self.body.push(Instruction::LocalSet(idx_local));
        }
        for st in &fe.block.statements {
            self.emit_statement(st)?;
        }
        // cerrar el continue block Ã¢â€ â€™ i++
        self.body.push(Instruction::End);
        self.block_depth -= 1;
        self.body.push(Instruction::LocalGet(i));
        self.body.push(Instruction::I64Const(1));
        self.body.push(Instruction::I64Add);
        self.body.push(Instruction::LocalSet(i));
        let depth = self.block_depth.saturating_sub(continue_at - 1);
        self.body.push(Instruction::Br(depth));
        self.body.push(Instruction::End); // loop
        self.block_depth -= 1;
        self.body.push(Instruction::End); // block
        self.block_depth -= 1;
        self.loop_stack.pop();
        Ok(())
    }


    /// `switch (v) { case (p) { ... } case default { ... } }` (sin fallthrough).
    pub(crate) fn emit_switch(&mut self, s: &SwitchStatement) -> ClsResult<()> {
        self.emit_expression(&s.value)?;
        let v = self.fresh_local();
        self.body.push(Instruction::LocalSet(v));
        let d = self.block_depth;
        self.block_depth += 1;
        self.body.push(Instruction::Block(BlockType::Empty));
        let done_at = self.block_depth;
        for case in &s.cases {
            if matches!(case.pattern, CasePattern::Default) {
                continue;
            }
            self.body.push(Instruction::LocalGet(v));
            match &case.pattern {
                CasePattern::Literal(l) => self.emit_literal(l)?,
                CasePattern::Identifier(name) => {
                    let idx = self.local_for(name);
                    self.body.push(Instruction::LocalGet(idx));
                }
                CasePattern::Default => {}
            }
            self.push_eq(WasTy::I64)?;
            self.block_depth += 1;
            self.body.push(Instruction::If(BlockType::Empty));
            for st in &case.block.statements {
                self.emit_statement(st)?;
            }
            let depth = self.block_depth.saturating_sub(done_at);
            self.body.push(Instruction::Br(depth));
            self.body.push(Instruction::End);
            self.block_depth -= 1;
        }
        if let Some(def) = &s.default {
            for st in &def.statements {
                self.emit_statement(st)?;
            }
        }
        self.body.push(Instruction::End); // block done
        self.block_depth -= 1;
        let _ = d;
        Ok(())
    }


    /// `with x in (expr) { ... }` Ã¢â€ â€™ local temporal + bloque.
    pub(crate) fn emit_with(&mut self, w: &WithStatement) -> ClsResult<()> {
        self.emit_expression(&w.value)?;
        let ty = self.value_type(&w.value)?;
        let idx = self.declare_var_ty(&w.name, ty);
        self.body.push(Instruction::LocalSet(idx));
        for st in &w.block.statements {
            self.emit_statement(st)?;
        }
        Ok(())
    }


    /// `try { ... } catch (e) { ... } finally { ... }` Ã¢â‚¬â€ excepciones WASM (try_table).
    /// Paridad con el walker: el finally solo se ejecuta si NO hubo catch; el catch
    /// recibe `e = "Error de runtime: " + msg` (e.to_string() del walker).
    pub(crate) fn emit_try(&mut self, stmt: &TryStatement) -> ClsResult<()> {
        if !self.exceptions {
            return Err(crate::error::ClsError::compile_at(
                "try/catch no soportado en este runtime: el backend se compilÃƒÂ³ sin \
                 excepciones WASM (wasmi). Usa el runtime wasmtime o el WASM nativo del navegador.",
                &stmt.span,
            ));
        }
        // block $outer (Empty)
        self.block_depth += 1;
        self.body.push(Instruction::Block(BlockType::Empty));
        let outer = self.block_depth;
        // block $handler (result [i64, i64]) Ã¢â‚¬â€ su label (continuation, tras su End)
        // es donde aterriza el catch con el payload [msg, span].
        self.block_depth += 1;
        self.body.push(Instruction::Block(BlockType::FunctionType(
            self.eh_handler_ty,
        )));
        let handler = self.block_depth;
        // try_table: captura nuestro tag Ã¢â€ â€™ br al label del $handler con [msg, span]
        // El label del catch NO cuenta el try_table como scope (br 0 = $handler).
        self.block_depth += 1;
        let catch_label = self.block_depth - handler - 1;
        self.body.push(Instruction::TryTable(
            BlockType::Empty,
            Cow::Owned(vec![Catch::One {
                tag: self.tag_idx,
                label: catch_label,
            }]),
        ));
        for s in &stmt.try_block.statements {
            self.emit_statement(s)?;
        }
        self.body.push(Instruction::End); // cierra try_table
        self.block_depth -= 1;
        // flujo normal (sin excepciÃƒÂ³n) Ã¢â€ â€™ br al $outer (salta el handler)
        let br_outer = self.block_depth - outer;
        self.body.push(Instruction::Br(br_outer));
        self.body.push(Instruction::End); // cierra $handler Ã¢â€ â€™ el catch aterriza AQUÃƒÂ con [msg, span]
        self.block_depth -= 1;
        // handler: payload [msg, span] en el stack (span arriba, msg debajo)
        if stmt.catch_clauses.is_empty() {
            let span_tmp = self.fresh_local();
            self.body.push(Instruction::LocalSet(span_tmp));
            let msg_tmp = self.fresh_local();
            self.body.push(Instruction::LocalSet(msg_tmp));
            if let Some(f) = &stmt.finally_block {
                for s in &f.statements {
                    self.emit_statement(s)?;
                }
            }
            // re-lanzar con el mismo payload (equivalente a Rethrow)
            self.body.push(Instruction::LocalGet(msg_tmp));
            self.body.push(Instruction::LocalGet(span_tmp));
            self.body.push(Instruction::Throw(self.tag_idx));
            self.body.push(Instruction::Unreachable);
        } else {
            let catch = &stmt.catch_clauses[0];
            let span_tmp = self.fresh_local();
            self.body.push(Instruction::LocalSet(span_tmp));
            let msg_tmp = self.fresh_local();
            self.body.push(Instruction::LocalSet(msg_tmp));
            // e = "Error de runtime: " + msg
            let pref = self.intern_string("Error de runtime: ");
            self.emit_load_str(pref);
            self.body.push(Instruction::LocalGet(msg_tmp));
            self.host.call(HostFn::StrConcat, &mut self.body);
            let e_local = self.declare_var_ty(&catch.param_name, WasTy::I64);
            self.body.push(Instruction::LocalSet(e_local));
            for s in &catch.block.statements {
                self.emit_statement(s)?;
            }
        }
        self.body.push(Instruction::End); // cierra $outer
        self.block_depth -= 1;
        Ok(())
    }


    pub(crate) fn emit_if(&mut self, i: &IfStatement) -> ClsResult<()> {
        self.emit_expression(&i.condition)?;
        self.coerce_to_bool(&i.condition)?;
        self.block_depth += 1;
        self.body.push(Instruction::If(BlockType::Empty));
        for s in &i.then_block.statements {
            self.emit_statement(s)?;
        }
        let has_elif = !i.elif_branches.is_empty();
        let has_else = i.else_block.is_some();
        if has_elif || has_else {
            self.body.push(Instruction::Else);
        }
        // Cadena de elifs anidados dentro del else; el ÃƒÂºltimo cede al else final.
        for (k, branch) in i.elif_branches.iter().enumerate() {
            self.emit_expression(&branch.condition)?;
            self.coerce_to_bool(&branch.condition)?;
            self.block_depth += 1;
            self.body.push(Instruction::If(BlockType::Empty));
            for s in &branch.block.statements {
                self.emit_statement(s)?;
            }
            let last = k == i.elif_branches.len() - 1;
            if last {
                if let Some(else_b) = &i.else_block {
                    self.body.push(Instruction::Else);
                    for s in &else_b.statements {
                        self.emit_statement(s)?;
                    }
                }
            } else {
                self.body.push(Instruction::Else);
            }
            self.body.push(Instruction::End);
            self.block_depth -= 1;
        }
        if !has_elif && has_else {
            let else_b = i.else_block.as_ref().unwrap();
            for s in &else_b.statements {
                self.emit_statement(s)?;
            }
        }
        self.body.push(Instruction::End);
        self.block_depth -= 1;
        Ok(())
    }


    pub(crate) fn emit_while(&mut self, w: &WhileStatement) -> ClsResult<()> {
        let d = self.block_depth;
        self.block_depth += 1;
        self.body.push(Instruction::Block(BlockType::Empty));
        let break_at = self.block_depth;
        self.block_depth += 1;
        self.body.push(Instruction::Loop(BlockType::Empty));
        let continue_at = self.block_depth;
        self.loop_stack.push(LoopGuard {
            break_at,
            continue_at,
        });
        let _ = d;
        self.emit_expression(&w.condition)?;
        self.coerce_to_bool(&w.condition)?;
        self.body.push(Instruction::I32Eqz);
        let depth = self.block_depth.saturating_sub(break_at);
        self.body.push(Instruction::BrIf(depth));
        for s in &w.block.statements {
            self.emit_statement(s)?;
        }
        let depth = self.block_depth.saturating_sub(continue_at);
        self.body.push(Instruction::Br(depth));
        self.body.push(Instruction::End); // loop
        self.block_depth -= 1;
        self.body.push(Instruction::End); // block
        self.block_depth -= 1;
        self.loop_stack.pop();
        Ok(())
    }


    pub(crate) fn emit_loop(&mut self, b: &Block) -> ClsResult<()> {
        let d = self.block_depth;
        self.block_depth += 1;
        self.body.push(Instruction::Block(BlockType::Empty));
        let break_at = self.block_depth;
        self.block_depth += 1;
        self.body.push(Instruction::Loop(BlockType::Empty));
        let continue_at = self.block_depth;
        self.loop_stack.push(LoopGuard {
            break_at,
            continue_at,
        });
        let _ = d;
        for s in &b.statements {
            self.emit_statement(s)?;
        }
        let depth = self.block_depth.saturating_sub(continue_at);
        self.body.push(Instruction::Br(depth));
        self.body.push(Instruction::End); // loop
        self.block_depth -= 1;
        self.body.push(Instruction::End); // block
        self.block_depth -= 1;
        self.loop_stack.pop();
        Ok(())
    }


    pub(crate) fn emit_for(&mut self, f: &ForStatement) -> ClsResult<()> {
        if let Some(init) = &f.init {
            self.emit_statement(init)?;
        }
        // break block
        self.block_depth += 1;
        self.body.push(Instruction::Block(BlockType::Empty));
        let break_at = self.block_depth;
        // loop
        self.block_depth += 1;
        self.body.push(Instruction::Loop(BlockType::Empty));
        // continue block: el `continue` salta aquÃƒÂ­ y ejecuta el update (evita
        // que se salte el incremento y produzca un loop infinito).
        self.block_depth += 1;
        self.body.push(Instruction::Block(BlockType::Empty));
        let continue_at = self.block_depth;
        self.loop_stack.push(LoopGuard {
            break_at,
            continue_at,
        });
        if let Some(cond) = &f.condition {
            self.emit_expression(cond)?;
            self.coerce_to_bool(cond)?;
            self.body.push(Instruction::I32Eqz);
            let depth = self.block_depth.saturating_sub(break_at);
            self.body.push(Instruction::BrIf(depth));
        }
        for s in &f.block.statements {
            self.emit_statement(s)?;
        }
        // cerrar el continue block Ã¢â€ â€™ se ejecuta el update
        self.body.push(Instruction::End);
        self.block_depth -= 1;
        if let Some(update) = &f.update {
            self.emit_expression(update)?;
            self.emit_drop(update)?;
        }
        // volver al loop (que estÃƒÂ¡ en continue_at - 1)
        let depth = self.block_depth.saturating_sub(continue_at - 1);
        self.body.push(Instruction::Br(depth));
        self.body.push(Instruction::End); // loop
        self.block_depth -= 1;
        self.body.push(Instruction::End); // block
        self.block_depth -= 1;
        self.loop_stack.pop();
        Ok(())
    }

}
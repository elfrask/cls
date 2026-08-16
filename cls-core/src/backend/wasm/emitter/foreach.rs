//! foreach.rs (Fase 1: extraido de cls-core/src/backend/wasm/emitter/statements.rs).

use super::*;

impl<'a> FuncEmitter<'a> {



    /// `arr.map(f)` - aplica la función (handle) a cada elemento y devuelve un
    /// array nuevo con los resultados. El array original YA está en el stack
    /// (lo emitió el dispatch del método).
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
        // tipo de f -> Fun(params, ret)
        let ft = self
            .types
            .get(&expr_span(&c.args[0]))
            .cloned()
            .unwrap_or(Type::Any);
        let (f_params, f_ret) = match ft {
            Type::Fun(p, r) => (p, *r),
            _ => {
                return Err(crate::error::ClsError::CompileError(
                    "map: el argumento debe ser una función".to_string(),
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
        // nuevo array [cap][len][ret...] del mismo tamaño que el original.
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
        // addr del destino en el nuevo array -> guardar en local.
        self.body.push(Instruction::LocalGet(new_ptr));
        self.body.push(Instruction::LocalGet(i));
        self.body.push(Instruction::I64Const(es_ret));
        self.body.push(Instruction::I64Mul);
        self.body.push(Instruction::I64Const(16));
        self.body.push(Instruction::I64Add);
        self.body.push(Instruction::I64Add);
        let addr_tmp = self.fresh_local();
        self.body.push(Instruction::LocalSet(addr_tmp));
        // elem = arr[16 + i*elem_size] -> guardar en local.
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
        // -> loop 0..variants.len()
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
                // continue block: el `continue` salta aquí y ejecuta el incremento.
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
                // cerrar el continue block -> incremento
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
        // __iter() -> Array (caso 1) u objeto iterador con __next() hasta null
        // (caso 2). El tipo del iterable debe ser una clase con __iter.
        if let Some(cn) = self.class_magic_method(&Some(iterable_ty.clone()), "__iter") {
            return self.emit_foreach_magic(fe, &cn, &iterable_ty);
        }
        let (elem_ty, elem_size) = match &iterable_ty {
            Type::Array(elem) => {
                let w = was_type(elem)?;
                // Array de Cmx -> entradas `[val, tag]` stride 16.
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



    /// Magic __iter/__next: `it = obj.__iter()`; si devuelve Array -> loop nativo;
    /// si devuelve una clase iteradora -> `it.__next()` hasta `null` (0 en el JIT).
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
            // Caso 1: __iter devolvió un Array -> iterar con el loop nativo.
            Some(Type::Array(elem)) => {
                let w = was_type(&*elem)?;
                let es = if matches!(*elem, Type::Cmx) {
                    16
                } else {
                    elem_size_bytes(w)
                };
                self.emit_foreach_array_loop(iter, w, es, fe)
            }
            // Caso 2: objeto iterador -> __next() hasta null.
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
        // continue block: el `continue` salta aquí y ejecuta el incremento.
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
        // cerrar el continue block -> i++
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



    /// Loop del iterador: `v = it.__next()`; si `v == 0` (null) -> break; si no,
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
        // continue block: el `continue` salta aquí y ejecuta el incremento.
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
        // if v == null (sentinel del protocolo __next) -> break
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
        // cerrar el continue block -> i++
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

}
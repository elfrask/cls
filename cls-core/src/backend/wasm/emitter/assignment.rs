//! assignment.rs (Fase 1: extraido de cls-core/src/backend/wasm/emitter/binary.rs).

use super::*;

impl<'a> FuncEmitter<'a> {



    pub(crate) fn emit_assignment(&mut self, a: &AssignmentExpr) -> ClsResult<()> {
        let op = a.op;
        match &*a.target {
            Expression::Identifier(name, _) => {
                if is_compound(op) {
                    // Magic: `a += b` -> a = a.__add(b) (paridad walker apply_compound).
                    let compound_magic = match op {
                        Operator::PlusEqual => "__add",
                        Operator::MinusEqual => "__sub",
                        Operator::StarEqual => "__mul",
                        Operator::SlashEqual => "__div",
                        Operator::PercentEqual => "__mod",
                        _ => "",
                    };
                    if !compound_magic.is_empty() {
                        let ty = self.types.get(&expr_span(&a.target)).cloned();
                        if let Some(cn) = self.class_magic_method(&ty, compound_magic) {
                            let _ = self.magic_ret_was(&cn, compound_magic)?;
                            self.emit_ident_load(name);
                            let obj_tmp = self.fresh_local();
                            self.body.push(Instruction::LocalSet(obj_tmp));
                            self.emit_class_method_call_on(
                                compound_magic,
                                &cn,
                                obj_tmp,
                                &[(*a.value).clone()],
                            )?;
                            self.emit_ident_store(name);
                            self.emit_ident_load(name);
                            return Ok(());
                        }
                    }
                    // Elegir operación según el tipo del identificador (int vs float).
                    let ty = self.value_type(&a.target)?;
                    self.emit_ident_load(name);
                    self.emit_expression(&a.value)?;
                    // `s += x` con String: concatenar (StrConcat), NO sumar
                    // los punteros empaquetados (producía bytes NUL).
                    let cls_t = self
                        .types
                        .get(&expr_span(&a.target))
                        .cloned()
                        .unwrap_or(Type::Any);
                    if op == Operator::PlusEqual && matches!(cls_t, Type::String) {
                        self.emit_str_host("__intr_str_concat", HostFn::StrConcat);
                    } else if ty == WasTy::F64 {
                        self.f64_promote(&a.value)?;
                        match op {
                            Operator::PlusEqual => self.body.push(Instruction::F64Add),
                            Operator::MinusEqual => self.body.push(Instruction::F64Sub),
                            Operator::StarEqual => self.body.push(Instruction::F64Mul),
                            Operator::SlashEqual => self.body.push(Instruction::F64Div),
                            // `%=` float: WASM no tiene resto float -> host fmod.
                            _ => self.host.call(HostFn::Fmod, &mut self.body),
                        }
                    } else {
                        match op {
                            Operator::PlusEqual => self.body.push(Instruction::I64Add),
                            Operator::MinusEqual => self.body.push(Instruction::I64Sub),
                            Operator::StarEqual => self.body.push(Instruction::I64Mul),
                            Operator::SlashEqual => self.body.push(Instruction::I64DivS),
                            _ => self.body.push(Instruction::I64RemS),
                        }
                    }
                } else {
                    // Frontera única: shape contiguo hacia destino dinámico.
                    let dest = self
                        .local_cls_types
                        .get(name)
                        .cloned()
                        .or_else(|| self.types.get(&expr_span(&a.target)).cloned());
                    self.emit_coerce(&a.value, dest.as_ref())?;
                    // Assignment simple `f = k`: si el target es float y el RHS
                    // es int, promover a f64 (el store del local espera f64).
                    if self.value_type(&a.target)? == WasTy::F64 {
                        self.f64_promote(&a.value)?;
                    }
                }
                self.emit_ident_store(name);
                self.emit_ident_load(name);
                Ok(())
            }
            Expression::Index(i)
                if matches!(
                    self.types.get(&expr_span(&i.object)),
                    Some(Type::Record(_, _))
                ) =>
            {
                if is_compound(op) {
                    return Err(crate::error::ClsError::CompileError(
                        "Operadores compuestos (+=) sobre registros no soportados en el JIT"
                            .to_string(),
                    ));
                }
                // r["key"] = val -> record_set(ptr, key, val_bits)
                let elem_ty = self.index_elem_type(i)?;
                let val_tmp = self.fresh_local_ty(elem_ty);
                self.emit_expression(&i.object)?;
                self.emit_expression(&i.index)?;
                // Frontera única: shape contiguo (o literal) hacia valor de
                // record dinámico -> hashmap.
                let dest_v = match self.types.get(&expr_span(&i.object)).cloned() {
                    Some(Type::Record(_, v)) => Some((*v).clone()),
                    _ => Some(Type::Any),
                };
                self.emit_coerce(&a.value, dest_v.as_ref())?;
                self.body.push(match elem_ty {
                    WasTy::F64 => Instruction::LocalSet(val_tmp),
                    WasTy::I32 => Instruction::LocalSet(val_tmp),
                    WasTy::I64 => Instruction::LocalSet(val_tmp),
                });
                self.body.push(match elem_ty {
                    WasTy::F64 => Instruction::LocalGet(val_tmp),
                    WasTy::I32 => Instruction::LocalGet(val_tmp),
                    WasTy::I64 => Instruction::LocalGet(val_tmp),
                });
                match elem_ty {
                    WasTy::F64 => self.body.push(Instruction::I64ReinterpretF64),
                    WasTy::I32 => self.body.push(Instruction::I64ExtendI32U),
                    WasTy::I64 => {}
                }
                let cls_t = self
                    .types
                    .get(&expr_span(&a.value))
                    .cloned()
                    .unwrap_or(Type::Any);
                // Tag del RUNTIME (runtime_tag_code: 1=string 6=array 7=record),
                // NO arr_kind_code (binding: 4=string 5=array 6=record) — el
                // record_set y la lectura/stringify usan el esquema runtime.
                self.body.push(Instruction::I64Const(runtime_tag_code(&cls_t)));
                if let Some(&idx) = self.func_indexes.get("__intr_record_set") {
                    self.body.push(Instruction::Call(idx));
                } else {
                    self.host.call(HostFn::RecordSet, &mut self.body);
                }
                // write-back del ptr (el record pudo crecer y reallocarse).
                // writeback_array maneja Identifier Y MemberAccess (`me.record`)
                // y deja el ptr del record en el stack; aquí no se usa como
                // receiver, así que se descarta (el statement devuelve el valor).
                self.writeback_array(&i.object)?;
                self.body.push(Instruction::Drop);
                self.body.push(match elem_ty {
                    WasTy::F64 => Instruction::LocalGet(val_tmp),
                    WasTy::I32 => Instruction::LocalGet(val_tmp),
                    WasTy::I64 => Instruction::LocalGet(val_tmp),
                });
                Ok(())
            }
            Expression::Index(i)
                if matches!(self.types.get(&expr_span(&i.object)), Some(Type::Shape(_))) =>
            {
                if is_compound(op) {
                    return Err(crate::error::ClsError::CompileError(
                        "Operadores compuestos (+=) sobre records con shape no soportados en el JIT".to_string(),
                    ));
                }
                // r["campo"] = val -> store por offset (solo campos existentes).
                let shape = self.types.get(&expr_span(&i.object)).cloned();
                let fields = match &shape {
                    Some(Type::Shape(f)) => f.clone(),
                    _ => return Ok(()),
                };
                let key = match &*i.index {
                    Expression::Literal(l) if matches!(l.kind, LiteralKind::String(_)) => {
                        match &l.kind { LiteralKind::String(k) => k.clone(), _ => String::new() }
                    }
                    _ => {
                        return Err(crate::error::ClsError::compile_at(
                            "Índice dinámico no soportado en un record con shape (usa Record<K,V> o any)",
                            &i.span,
                        ))
                    }
                };
                let (_, w, off) = self.shape_layout(&fields)?
                    .into_iter()
                    .find(|(n, _, _)| *n == key)
                    .ok_or_else(|| crate::error::ClsError::compile_at(
                        &format!("El record no tiene el campo '{}' (no se pueden agregar campos a un shape)", key),
                        &i.span,
                    ))?;
                self.emit_expression(&i.object)?;
                let ptr_tmp = self.fresh_local();
                self.body.push(Instruction::LocalSet(ptr_tmp));
                self.emit_expression(&a.value)?;
                let val_tmp = self.fresh_local_ty(w);
                self.body.push(Instruction::LocalSet(val_tmp));
                self.body.push(Instruction::LocalGet(ptr_tmp));
                self.body.push(Instruction::I64Const(off));
                self.body.push(Instruction::I64Add);
                self.body.push(Instruction::I32WrapI64);
                self.body.push(Instruction::LocalGet(val_tmp));
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
                self.body.push(Instruction::LocalGet(ptr_tmp));
                Ok(())
            }
            Expression::Index(i) => {
                // Magic __set: obj[i] = v -> obj.__set(index, value) con write-back
                // del objeto mutado (paridad walker interpreter.rs:2120-2128).
                let obj_ty = self.types.get(&expr_span(&i.object)).cloned();
                if let Some(cn) = self.class_magic_method(&obj_ty, "__set") {
                    if is_compound(op) {
                        return Err(crate::error::ClsError::CompileError(
                            "Operadores compuestos (+=) sobre objetos con __set no soportados en el JIT"
                                .to_string(),
                        ));
                    }
                    self.emit_expression(&i.object)?;
                    let obj_tmp = self.fresh_local();
                    self.body.push(Instruction::LocalSet(obj_tmp));
                    self.emit_class_method_call_on(
                        "__set",
                        &cn,
                        obj_tmp,
                        &[(*i.index).clone(), (*a.value).clone()],
                    )?;
                    // El retorno del __set (si lo hay) se descarta.
                    if let Some(t) = self.magic_ret_type(&cn, "__set") {
                        if t != Type::Void {
                            self.body.push(Instruction::Drop);
                        }
                    }
                    // write-back del objeto (el ptr no cambia en mutación in-place,
                    // pero la reasignación del slot es paridad walker).
                    if let Expression::Identifier(name, _) = &*i.object {
                        self.body.push(Instruction::LocalGet(obj_tmp));
                        self.emit_ident_store(name);
                    }
                    // Valor del assignment = el objeto (para el Drop del statement).
                    self.body.push(Instruction::LocalGet(obj_tmp));
                    return Ok(());
                }
                if is_compound(op) {
                    let elem_ty = self.index_elem_type(i)?;
                    let ptr = self.fresh_local();
                    let idx = self.fresh_local();
                    let cur = self.fresh_local_ty(elem_ty);
                    let v = self.fresh_local_ty(elem_ty);
                    let res = self.fresh_local_ty(elem_ty);
                    self.emit_expression(&i.object)?;
                    self.body.push(Instruction::LocalSet(ptr));
                    self.emit_expression(&i.index)?;
                    self.body.push(Instruction::LocalSet(idx));
                    // cur = arr[i]
                    self.body.push(Instruction::LocalGet(ptr));
                    self.body.push(Instruction::LocalGet(idx));
                    let elem_size = self.container_elem_size(i, elem_ty);
                    self.emit_index_access(elem_ty, elem_size, i)?;
                    self.body.push(match elem_ty {
                        WasTy::F64 => Instruction::LocalSet(cur),
                        WasTy::I32 => Instruction::LocalSet(cur),
                        WasTy::I64 => Instruction::LocalSet(cur),
                    });
                    self.emit_expression(&a.value)?;
                    // `farr[i] += 2` con array F64: el RHS int debe promover a f64
                    // (paridad con el write simple `farr[i] = 7` del fix R4).
                    if elem_ty == WasTy::F64 {
                        self.f64_promote(&a.value)?;
                    }
                    self.body.push(match elem_ty {
                        WasTy::F64 => Instruction::LocalSet(v),
                        WasTy::I32 => Instruction::LocalSet(v),
                        WasTy::I64 => Instruction::LocalSet(v),
                    });
                    self.body.push(match elem_ty {
                        WasTy::F64 => Instruction::LocalGet(cur),
                        WasTy::I32 => Instruction::LocalGet(cur),
                        WasTy::I64 => Instruction::LocalGet(cur),
                    });
                    self.body.push(match elem_ty {
                        WasTy::F64 => Instruction::LocalGet(v),
                        WasTy::I32 => Instruction::LocalGet(v),
                        WasTy::I64 => Instruction::LocalGet(v),
                    });
                    if elem_ty == WasTy::F64 && op == Operator::PercentEqual {
                        // `farr[i] %= v` float: WASM no tiene resto float -> host fmod.
                        self.host.call(HostFn::Fmod, &mut self.body);
                    } else {
                        apply_compound_ty(&mut self.body, op, elem_ty)?;
                    }
                    self.body.push(match elem_ty {
                        WasTy::F64 => Instruction::LocalSet(res),
                        WasTy::I32 => Instruction::LocalSet(res),
                        WasTy::I64 => Instruction::LocalSet(res),
                    });
                    self.body.push(Instruction::LocalGet(ptr));
                    self.body.push(Instruction::LocalGet(idx));
                    self.body.push(match elem_ty {
                        WasTy::F64 => Instruction::LocalGet(res),
                        WasTy::I32 => Instruction::LocalGet(res),
                        WasTy::I64 => Instruction::LocalGet(res),
                    });
                    self.emit_index_set(i, elem_size)?;
                    self.body.push(match elem_ty {
                        WasTy::F64 => Instruction::LocalGet(res),
                        WasTy::I32 => Instruction::LocalGet(res),
                        WasTy::I64 => Instruction::LocalGet(res),
                    });
                } else {
                    // Las tuplas son inmutables: escritura -> error.
                    let obj_ty = self.types.get(&expr_span(&i.object)).cloned();
                    if matches!(obj_ty, Some(Type::Tuple(_))) {
                        return Err(crate::error::ClsError::compile_at(
                            "Las tuplas son inmutables (no se puede escribir t[i] = v)",
                            &i.span,
                        ));
                    }
                    let elem_ty = self.index_elem_type(i)?;
                    let elem_size = self.container_elem_size(i, elem_ty);
                    self.emit_expression(&i.object)?;
                    self.emit_expression(&i.index)?;
                    self.emit_expression(&a.value)?;
                    // Array de float con valor int: promover el RHS a f64 antes
                    // del store (el layout del array es homogéneo).
                    if elem_ty == WasTy::F64 {
                        self.f64_promote(&a.value)?;
                    }
                    self.emit_index_set(i, elem_size)?;
                    // Dejar un valor en el stack (el array mutado) para que el
                    // Drop del statement (o el uso del valor) lo consuma.
                    self.emit_expression(&i.object)?;
                }
                Ok(())
            }
            Expression::MemberAccess(m) => {
                // `Clase.campo = v` (campo estático) -> global.set.
                if let Expression::Identifier(cn, _) = &*m.object {
                    if let Some(&g) = self.static_fields.get(&format!("{}::{}", cn, m.member)) {
                        if is_compound(op) {
                            return Err(crate::error::ClsError::CompileError(
                                "Operadores compuestos sobre campos estáticos no soportados en el JIT"
                                    .to_string(),
                            ));
                        }
                        self.emit_expression(&a.value)?;
                        self.body.push(Instruction::GlobalSet(g));
                        let w = self.value_type(&a.value)?;
                        self.body.push(match w {
                            WasTy::F64 => Instruction::GlobalGet(g),
                            _ => Instruction::GlobalGet(g),
                        });
                        return Ok(());
                    }
                }
                let obj_ty = self.types.get(&expr_span(&m.object)).cloned();
                if let Some(Type::Named(name, _)) = obj_ty {
                    if let Some(info) = self.class_defs.get(name.as_str()) {
                        if is_compound(op) {
                            return Err(crate::error::ClsError::CompileError(
                                "Operadores compuestos sobre campos de clase no soportados en el JIT (B3)".to_string(),
                            ));
                        }
                        let fidx = info
                            .fields
                            .iter()
                   .position(|(n, _, _, _, _)| *n == m.member)
                            .ok_or_else(|| {
                                crate::error::ClsError::compile_at(
                                    &format!(
                                        "El campo '{}' no existe en la clase '{}'",
                                        m.member, name
                                    ),
                                    &m.span,
                                )
                            })?;
                        let (_, _t, w, off, vis) = &info.fields[fidx];
                        // Escritura: private/protected desde fuera, o readonly.
                        self.check_field_access(name.as_str(), m.member.as_str(), *vis, &m.span)?;
                        if vis.is_readonly() {
                            // readonly: solo escritura interna (me.campo).
                            let inside = self
                                .current_class
                                .as_deref()
                                .map(|c| c == name.as_str())
                                .unwrap_or(false);
                            if !inside {
                                return Err(crate::error::ClsError::compile_at(
                                    &format!(
                                        "El campo '{}' es readonly (solo se puede escribir desde la clase)",
                                        m.member
                                    ),
                                    &m.span,
                                ));
                            }
                        }
                        let w = *w;
                        let off = *off;
                        let obj_tmp = self.fresh_local();
                        let val_tmp = self.fresh_local_ty(w);
                        self.emit_expression(&m.object)?;
                        self.body.push(Instruction::LocalSet(obj_tmp));
                        // Frontera única: shape contiguo hacia campo dinámico.
                        self.emit_coerce(&a.value, Some(_t))?;
                        self.body.push(match w {
                            WasTy::F64 => Instruction::LocalSet(val_tmp),
                            WasTy::I32 => Instruction::LocalSet(val_tmp),
                            WasTy::I64 => Instruction::LocalSet(val_tmp),
                        });
                        self.body.push(Instruction::LocalGet(obj_tmp));
                        self.body.push(Instruction::I64Const(off));
                        self.body.push(Instruction::I64Add);
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
                        self.body.push(match w {
                            WasTy::F64 => Instruction::LocalGet(val_tmp),
                            WasTy::I32 => Instruction::LocalGet(val_tmp),
                            WasTy::I64 => Instruction::LocalGet(val_tmp),
                        });
                        return Ok(());
                    }
                }
                // Struct: `p.campo = val` -> store por offset del campo.
                if let Some(Type::Named(sn, _)) = self.types.get(&expr_span(&m.object)).cloned() {
                    if let Some(info) = self.struct_defs.get(sn.as_str()) {
                        if is_compound(op) {
                            return Err(crate::error::ClsError::compile_at(
                                "Operadores compuestos sobre campos de struct no soportados en el JIT",
                                &m.span,
                            ));
                        }
                        let fidx = info
                            .fields
                            .iter()
                            .position(|(n, _, _)| *n == m.member)
                            .ok_or_else(|| {
                                crate::error::ClsError::compile_at(
                                    &format!("El campo '{}' no existe en el struct '{}'", m.member, sn),
                                    &m.span,
                                )
                            })?;
                        let w = info.fields[fidx].2;
                        let off = info.offsets[fidx];
                        let obj_tmp = self.fresh_local();
                        let val_tmp = self.fresh_local_ty(w);
                        self.emit_expression(&m.object)?;
                        self.body.push(Instruction::LocalSet(obj_tmp));
                        self.emit_expression(&a.value)?;
                        self.body.push(match w {
                            WasTy::F64 => Instruction::LocalSet(val_tmp),
                            WasTy::I32 => Instruction::LocalSet(val_tmp),
                            WasTy::I64 => Instruction::LocalSet(val_tmp),
                        });
                        self.body.push(Instruction::LocalGet(obj_tmp));
                        self.body.push(Instruction::I64Const(off));
                        self.body.push(Instruction::I64Add);
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
                        self.body.push(match w {
                            WasTy::F64 => Instruction::LocalGet(val_tmp),
                            WasTy::I32 => Instruction::LocalGet(val_tmp),
                            WasTy::I64 => Instruction::LocalGet(val_tmp),
                        });
                        return Ok(());
                    }
                }
                // Record dinámico: `r.campo = val` -> record_set(ptr, "campo",
                // val_bits, tag) + write-back. El access por `.` sobre un
                // Record<String,Any> (diferente del shape contiguo).
                if matches!(
                    self.types.get(&expr_span(&m.object)),
                    Some(Type::Record(_, _))
                ) {
                    if is_compound(op) {
                        return Err(crate::error::ClsError::CompileError(
                            "Operadores compuestos (+=) sobre registros no soportados en el JIT"
                                .to_string(),
                        ));
                    }
                    let obj_tmp = self.fresh_local();
                    let val_tmp = self.fresh_local();
                    self.emit_expression(&m.object)?;
                    self.body.push(Instruction::LocalSet(obj_tmp));
                    // Frontera única: shape contiguo/literal hacia valor de
                    // record dinámico -> hashmap.
                    let dest_v = match self.types.get(&expr_span(&m.object)).cloned() {
                        Some(Type::Record(_, v)) => Some((*v).clone()),
                        _ => Some(Type::Any),
                    };
                    self.emit_coerce(&a.value, dest_v.as_ref())?;
                    // El valor se guarda como i64 (el local es i64): bool/char
                    // (i32) -> extender ANTES del set; float -> bits.
                    match self.value_type(&a.value)? {
                        WasTy::F64 => self.body.push(Instruction::I64ReinterpretF64),
                        WasTy::I32 => self.body.push(Instruction::I64ExtendI32U),
                        _ => {}
                    }
                    self.body.push(Instruction::LocalSet(val_tmp));
                    self.body.push(Instruction::LocalGet(obj_tmp));
                    let k = self.intern_string(&m.member);
                    self.emit_load_str(k);
                    self.body.push(Instruction::LocalGet(val_tmp));
                    let cls_t = self
                        .types
                        .get(&expr_span(&a.value))
                        .cloned()
                        .unwrap_or(Type::Any);
                    self.body.push(Instruction::I64Const(runtime_tag_code(&cls_t)));
                    if let Some(&idx) = self.func_indexes.get("__intr_record_set") {
                        self.body.push(Instruction::Call(idx));
                    } else {
                        self.host.call(HostFn::RecordSet, &mut self.body);
                    }
                    self.writeback_array(&m.object)?;
                    self.body.push(Instruction::Drop);
                    self.body.push(Instruction::LocalGet(val_tmp));
                    return Ok(());
                }
                // Record con shape: r.campo = val -> store por offset (campo existente).
                if let Some(Type::Shape(fields)) = self.types.get(&expr_span(&m.object)).cloned() {                    if is_compound(op) {
                        return Err(crate::error::ClsError::CompileError(
                            "Operadores compuestos sobre campos de record con shape no soportados en el JIT".to_string(),
                        ));
                    }
                    let (_, w, off) = self.shape_layout(&fields)?
                        .into_iter()
                        .find(|(n, _, _)| *n == m.member)
                        .ok_or_else(|| crate::error::ClsError::compile_at(
                            &format!("El record no tiene el campo '{}' (no se pueden agregar campos a un shape)", m.member),
                            &m.span,
                        ))?;
                    let obj_tmp = self.fresh_local();
                    let val_tmp = self.fresh_local_ty(w);
                    self.emit_expression(&m.object)?;
                    self.body.push(Instruction::LocalSet(obj_tmp));
                    self.emit_expression(&a.value)?;
                    self.body.push(match w {
                        WasTy::F64 => Instruction::LocalSet(val_tmp),
                        WasTy::I32 => Instruction::LocalSet(val_tmp),
                        WasTy::I64 => Instruction::LocalSet(val_tmp),
                    });
                    self.body.push(Instruction::LocalGet(obj_tmp));
                    self.body.push(Instruction::I64Const(off));
                    self.body.push(Instruction::I64Add);
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
                    self.body.push(match w {
                        WasTy::F64 => Instruction::LocalGet(val_tmp),
                        WasTy::I32 => Instruction::LocalGet(val_tmp),
                        WasTy::I64 => Instruction::LocalGet(val_tmp),
                    });
                    return Ok(());
                }
                Err(self.unsupported_expr(&Expression::MemberAccess(m.clone())))
            }
            other => Err(self.unsupported_expr(other)),
        }
    }

}
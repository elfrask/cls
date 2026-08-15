//! Expressions: emit_expression, literals, callsite, conditional, tuple, interpolation (Fase 1: extraido de emitter/mod.rs).

use super::*;

impl<'a> FuncEmitter<'a> {


    // Ã¢â€â‚¬Ã¢â€â‚¬ EmisiÃƒÂ³n de expresiones Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬

    pub(crate) fn emit_expression(&mut self, expr: &Expression) -> ClsResult<()> {
        match expr {
            Expression::Literal(l) => self.emit_literal(l),
            Expression::Identifier(name, _) => {
                self.emit_ident_load(name);
                Ok(())
            }
            Expression::Binary(b) => self.emit_binary(b),
            Expression::Unary(u) => self.emit_unary(u),
            Expression::Call(c) => self.emit_call(c),
            Expression::Index(i) => self.emit_index_get(i),
            Expression::Array(a) => self.emit_array(a),
            Expression::Tuple(t) => self.emit_tuple(t),
            Expression::Record(r) => self.emit_record(r),
            Expression::Cmx(c) => self.emit_cmx(c),
            Expression::ArrowFunction(a) => {
                // Arrow Ã¢â€ â€™ handle de su funciÃƒÂ³n sintÃƒÂ©tica `__arrow_<n>`.
                // Si captura variables (closure): evaluarlas en un bloque
                // `[n, v1, v2, ...]` y pasar el ptr como tercer arg del handle.
                let name = self.arrow_names.get(&a.span).ok_or_else(|| {
                    crate::error::ClsError::CompileError(
                        "Arrow function sin funciÃƒÂ³n sintÃƒÂ©tica (recolecciÃƒÂ³n)".to_string(),
                    )
                })?;
                let ti = self.fn_table_idx[name];
                let captures = self
                    .arrow_captures
                    .get(&a.span)
                    .cloned()
                    .unwrap_or_default();
                // Bloque de capturas `[n, v1, v2, ...]` (se evalÃƒÂºa primero).
                let cap_ptr = self.fresh_local();
                if captures.is_empty() {
                    self.body.push(Instruction::I64Const(0));
                    self.body.push(Instruction::LocalSet(cap_ptr));
                } else {
                    let ncap = captures.len() as i64;
                    let es = 8i64;
                    self.body.push(Instruction::I64Const(ncap));
                    self.body.push(Instruction::I64Const(es));
                    self.body.push(Instruction::I64Mul);
                    self.body.push(Instruction::I64Const(16));
                    self.body.push(Instruction::I64Add);
                    let alloc = self.func_indexes["__alloc"];
                    self.body.push(Instruction::Call(alloc));
                    self.body.push(Instruction::LocalSet(cap_ptr));
                    self.body.push(Instruction::LocalGet(cap_ptr));
                    self.body.push(Instruction::I64Const(ncap));
                    self.emit_i64_store(0);
                    for (i, cap) in captures.iter().enumerate() {
                        self.body.push(Instruction::LocalGet(cap_ptr));
                        self.body.push(Instruction::I64Const(16 + (i as i64) * 8));
                        self.body.push(Instruction::I64Add);
                        self.body.push(Instruction::I32WrapI64);
                        self.emit_ident_ptr(cap);
                        self.body.push(Instruction::I64Store(MemArg {
                            offset: 0,
                            align: 3,
                            memory_index: 0,
                        }));
                    }
                }
                self.body.push(Instruction::I64Const(ti as i64));
                let n = self.intern_string("<anonymous>");
                self.emit_load_str(n);
                self.body.push(Instruction::LocalGet(cap_ptr));
                self.host.call(HostFn::FnHandle, &mut self.body);
                Ok(())
            }
            Expression::MemberAccess(m) => self.emit_member_access(m),
            Expression::Conditional(c) => self.emit_conditional(c),
            Expression::Assignment(a) => self.emit_assignment(a),
            Expression::Parenthesized(inner, _) => self.emit_expression(inner),
            Expression::StringInterpolation(s) => self.emit_interpolation(s),
            // `x::miembro` (mÃƒÂ³dulo/namespace importado): global `x::miembro`.
            Expression::NamespaceAccess(ns, member, span) => {
                let key = format!("{}::{}", ns, member);
                if let Some(g) = self.globals.get(&key).copied() {
                    self.body.push(Instruction::GlobalGet(g));
                    Ok(())
                } else {
                    Err(crate::error::ClsError::compile_at(
                        &format!(
                            "El miembro '{}' no existe o no se exporta en el mÃƒÂ³dulo '{}' (fase de emisiÃƒÂ³n).",
                            member, ns
                        ),
                        span,
                    ))
                }
            }
            other => Err(self.unsupported_expr(other)),
        }
    }


    pub(crate) fn unsupported_expr(&self, expr: &Expression) -> crate::error::ClsError {
        let span = expr_span(expr);
        crate::error::ClsError::compile_at(
            &format!(
                "El JIT (subconjunto WASM) aÃƒÂºn no soporta esta expresiÃƒÂ³n: `{}`",
                expr_display(expr)
            ),
            &span,
        )
    }


    pub(crate) fn emit_literal(&mut self, l: &Literal) -> ClsResult<()> {
        match &l.kind {
            LiteralKind::Int(v) => self.body.push(Instruction::I64Const(*v)),
            LiteralKind::Float(v) => self
                .body
                .push(Instruction::F64Const(Ieee64::new(v.to_bits()))),
            LiteralKind::Bool(v) => self
                .body
                .push(Instruction::I32Const(if *v { 1 } else { 0 })),
            LiteralKind::Char(c) => self.body.push(Instruction::I32Const(*c as u32 as i32)),
            LiteralKind::String(s) => {
                let idx = self.intern_string(s);
                self.emit_load_str(idx);
            }
            LiteralKind::Null => {
                // Dentro de `__next`, el `null` es el sentinel de fin de iteraciÃƒÂ³n
                // (distinto de 0 Ã¢â‚¬â€ un iterador puede devolver 0 como valor
                // legÃƒÂ­timo). Fuera del protocolo, null = 0 (paridad histÃƒÂ³rica).
                if self.current_method.as_deref() == Some("__next") {
                    self.body.push(Instruction::I64Const(NULL_ITER_SENTINEL));
                } else {
                    self.body.push(Instruction::I64Const(0));
                }
            }
            LiteralKind::Unknown => {
                return Err(self.unsupported_expr(&Expression::Literal(l.clone())))
            }
        }
        Ok(())
    }


    /// Emite `env.fn_enter(nombre, line, col)` al inicio de una funciÃƒÂ³n CLS.
    /// Registra la funciÃƒÂ³n en el shadow call stack del host (para el trace de
    /// errores de runtime). `main` (la entrada) se registra sin ubicaciÃƒÂ³n
    /// (lÃƒÂ­nea 0): el formateador lo muestra como `Ã¢â€ â€™ main` sin lÃƒÂ­nea.
    pub(crate) fn emit_fn_enter(&mut self, f: &FunctionDecl) -> ClsResult<()> {
        let display = f
            .name
            .rsplit("::")
            .next()
            .unwrap_or(&f.name)
            .trim_start_matches("__s__")
            .to_string();
        let name_idx = self.intern_string(&display);
        self.emit_load_str(name_idx);
        let (line, col) = if f.name == "main" {
            (0, 0)
        } else {
            (f.span.start_line, f.span.start_col)
        };
        self.body.push(Instruction::I64Const(line as i64));
        self.body.push(Instruction::I64Const(col as i64));
        self.host.call(HostFn::FnEnter, &mut self.body);
        Ok(())
    }


    /// Emite `env.fn_call_site(line, col)` con el span del call site (la llamada
    /// dentro del llamador). El host lo guarda como pendiente; el `fn_enter` del
    /// callee lo consume como span del frame.
    pub(crate) fn emit_call_site(&mut self, span: &Span) {
        self.body.push(Instruction::I64Const(span.start_line as i64));
        self.body.push(Instruction::I64Const(span.start_col as i64));
        self.host.call(HostFn::CallSite, &mut self.body);
    }


    /// Emite una llamada a una funciÃƒÂ³n host del nodo (intrinsic) vÃƒÂ­a el canal
    /// genÃƒÂ©rico `env.host_call(id, ptr, n)`. Los args viajan empaquetados en
    /// memoria: `[n:i64][(val:i64, tag:i64)*n]` (tag = `cls_kind_code`).
    pub(crate) fn emit_host_call(&mut self, intr: &HostIntrinsic, c: &CallExpr) -> ClsResult<()> {
        let n = c.args.len() as i64;
        // 1. Evaluar cada arg y guardarlo en un temporal (bits uniformes i64:
        //    float Ã¢â€ â€™ reinterpret bits; bool/char Ã¢â€ â€™ extender a i64).
        let mut tmps: Vec<u32> = Vec::with_capacity(c.args.len());
        for (i, arg) in c.args.iter().enumerate() {
            self.emit_expression(arg)?;
            match intr.params.get(i) {
                Some(Type::Float) | Some(Type::F32) | Some(Type::F64)
                | Some(Type::Literal(LitVal::Float(_))) => {
                    self.body.push(Instruction::I64ReinterpretF64);
                }
                Some(Type::Bool) | Some(Type::Char)
                | Some(Type::Literal(LitVal::Bool(_))) => {
                    self.body.push(Instruction::I64ExtendI32S);
                }
                _ => {}
            }
            let tmp = self.fresh_local();
            self.body.push(Instruction::LocalSet(tmp));
            tmps.push(tmp);
        }
        // 2. Alocar el bloque [n][(val,tag)*n].
        let size = 8 + n * 16;
        self.body.push(Instruction::I64Const(size));
        self.body.push(Instruction::Call(self.func_indexes["__alloc"]));
        let ptr = self.fresh_local();
        self.body.push(Instruction::LocalSet(ptr));
        // 3. Escribir n.
        self.body.push(Instruction::LocalGet(ptr));
        self.body.push(Instruction::I32WrapI64);
        self.body.push(Instruction::I64Const(n));
        self.body.push(Instruction::I64Store(MemArg {
            offset: 0,
            align: 3,
            memory_index: 0,
        }));
        // 4. Por arg: val + tag. (El addr de los memory ops es i32 Ã¢â€ â€™ wrap.)
        for (i, tmp) in tmps.iter().enumerate() {
            let base = 8 + (i as i64) * 16;
            self.body.push(Instruction::LocalGet(ptr));
            self.body.push(Instruction::I64Const(base));
            self.body.push(Instruction::I64Add);
            self.body.push(Instruction::I32WrapI64);
            self.body.push(Instruction::LocalGet(*tmp));
            self.body.push(Instruction::I64Store(MemArg {
                offset: 0,
                align: 3,
                memory_index: 0,
            }));
            self.body.push(Instruction::LocalGet(ptr));
            self.body.push(Instruction::I64Const(base + 8));
            self.body.push(Instruction::I64Add);
            self.body.push(Instruction::I32WrapI64);
            self.body.push(Instruction::I64Const(cls_kind_code(
                intr.params.get(i).unwrap_or(&Type::Any),
            )));
            self.body.push(Instruction::I64Store(MemArg {
                offset: 0,
                align: 3,
                memory_index: 0,
            }));
        }
        // 5. `env.host_call(id, ptr, n)`.
        self.body.push(Instruction::I64Const(intr.id as i64));
        self.body.push(Instruction::LocalGet(ptr));
        self.body.push(Instruction::I64Const(n));
        self.host.call(HostFn::HostCall, &mut self.body);
        // 6. Convertir el retorno (bits i64) al tipo nativo del CLS.
        match &intr.ret {
            Type::Void | Type::Empty => {
                self.body.push(Instruction::Drop);
            }
            Type::Float | Type::F32 | Type::F64 | Type::Literal(LitVal::Float(_)) => {
                self.body.push(Instruction::F64ReinterpretI64);
            }
            Type::Bool | Type::Char | Type::Literal(LitVal::Bool(_)) => {
                self.body.push(Instruction::I32WrapI64);
            }
            _ => {}
        }
        Ok(())
    }


    /// Emite `env.fn_exit()` antes de salir de una funciÃƒÂ³n CLS.
    pub(crate) fn emit_fn_exit(&mut self) {
        self.host.call(HostFn::FnExit, &mut self.body);
    }


    pub(crate) fn intern_string(&mut self, s: &str) -> u32 {
        if let Some(idx) = self.string_index.get(s) {
            return *idx;
        }
        let idx = self.string_pool.len() as u32;
        self.string_pool.push(s.to_string());
        self.string_index.insert(s.to_string(), idx);
        idx
    }


    pub(crate) fn emit_load_str(&mut self, idx: u32) {
        self.body.push(Instruction::I64Const(idx as i64));
        let fidx = self.func_indexes["__load_str"];
        self.body.push(Instruction::Call(fidx));
    }


    pub(crate) fn emit_conditional(&mut self, c: &ConditionalExpr) -> ClsResult<()> {
        let w = self.value_type(&c.then_expr)?;
        self.emit_expression(&c.condition)?;
        self.block_depth += 1;
        self.body
            .push(Instruction::If(BlockType::Result(w.val_type())));
        self.emit_expression(&c.then_expr)?;
        self.body.push(Instruction::Else);
        self.emit_expression(&c.else_expr)?;
        self.body.push(Instruction::End);
        self.block_depth -= 1;
        Ok(())
    }


    pub(crate) fn emit_tuple(&mut self, t: &TupleExpr) -> ClsResult<()> {
        // Layout igual al array: [cap:i64][len:i64][slots...] con slots de 8 bytes.
        let n = t.elements.len() as i64;
        self.body.push(Instruction::I64Const(n));
        self.body.push(Instruction::I64Const(8));
        self.body.push(Instruction::I64Mul);
        self.body.push(Instruction::I64Const(16));
        self.body.push(Instruction::I64Add);
        let alloc = self.func_indexes["__alloc"];
        self.body.push(Instruction::Call(alloc));
        let ptr = self.fresh_local();
        self.body.push(Instruction::LocalSet(ptr));
        self.body.push(Instruction::LocalGet(ptr));
        self.body.push(Instruction::I64Const(n));
        self.emit_i64_store(0);
        self.body.push(Instruction::LocalGet(ptr));
        self.body.push(Instruction::I64Const(n));
        self.emit_i64_store(8);
        for (i, el) in t.elements.iter().enumerate() {
            self.emit_expression(el)?;
            let elem_ty = self.value_type(el)?;
            let val_tmp = self.fresh_local_ty(elem_ty);
            let addr_tmp = self.fresh_local();
            self.body.push(match elem_ty {
                WasTy::F64 => Instruction::LocalSet(val_tmp),
                WasTy::I32 => Instruction::LocalSet(val_tmp),
                WasTy::I64 => Instruction::LocalSet(val_tmp),
            });
            self.body.push(Instruction::LocalGet(ptr));
            self.body.push(Instruction::I64Const(16 + (i as i64) * 8));
            self.body.push(Instruction::I64Add);
            self.body.push(Instruction::LocalSet(addr_tmp));
            self.body.push(Instruction::LocalGet(addr_tmp));
            self.body.push(Instruction::I32WrapI64);
            self.body.push(match elem_ty {
                WasTy::F64 => Instruction::LocalGet(val_tmp),
                WasTy::I32 => Instruction::LocalGet(val_tmp),
                WasTy::I64 => Instruction::LocalGet(val_tmp),
            });
            match elem_ty {
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
        Ok(())
    }


    /// `"Hola $nombre ${expr}"` Ã¢â€ â€™ concatenaciÃƒÂ³n de las partes (toString de cada expr).
    pub(crate) fn emit_interpolation(&mut self, s: &StringInterpolation) -> ClsResult<()> {
        let empty = self.intern_string("");
        self.emit_load_str(empty);
        let acc = self.fresh_local();
        self.body.push(Instruction::LocalSet(acc));
        for part in &s.parts {
            match part {
                InterpolationPart::Text(t) => {
                    let idx = self.intern_string(t);
                    self.emit_load_str(idx);
                }
                InterpolationPart::Expr(e) => {
                    self.emit_expression(e)?;
                    self.emit_to_string(e)?;
                }
            }
            let tmp = self.fresh_local();
            self.body.push(Instruction::LocalSet(tmp));
            self.body.push(Instruction::LocalGet(acc));
            self.body.push(Instruction::LocalGet(tmp));
            self.host.call(HostFn::StrConcat, &mut self.body);
            self.body.push(Instruction::LocalSet(acc));
        }
        self.body.push(Instruction::LocalGet(acc));
        Ok(())
    }

}
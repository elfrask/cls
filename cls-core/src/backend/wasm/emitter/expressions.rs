//! Expressions: emit_expression, literals, callsite, conditional, tuple, interpolation (Fase 1: extraido de emitter/mod.rs).

use super::*;

impl<'a> FuncEmitter<'a> {


    // == Emisión de expresiones ===========================================

    pub(crate) fn emit_expression(&mut self, expr: &Expression) -> ClsResult<()> {
        self.emit_expression_inner(expr)?;
        // Los arrays tienen IDENTIDAD estable: cuando `push`/`unshift` crecen,
        // el buffer viejo se marca como forward hacia el nuevo (`__intr_arr_*`)
        // para que cualquier alias (p.ej. la variable del llamador cuando el
        // mutador se llamó sobre un parámetro) vea la versión crecida
        // (semántica de referencia). Resolver la cadena acá, en el único punto
        // por el que pasa todo valor leído, hace transparente el forward.
        if !matches!(expr, Expression::Array(_) | Expression::Tuple(_)) {
            if matches!(self.types.get(&expr_span(expr)), Some(Type::Array(_))) {
                if let Some(&idx) = self.func_indexes.get("__intr_arr_deref") {
                    self.body.push(Instruction::Call(idx));
                }
            }
        }
        Ok(())
    }

    fn emit_expression_inner(&mut self, expr: &Expression) -> ClsResult<()> {
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
                // Arrow -> handle de su función sintética `__arrow_<n>`.
                // Si captura variables (closure): evaluarlas en un bloque
                // `[n, v1, v2, ...]` y pasar el ptr como tercer arg del handle.
                let name = self.arrow_names.get(&a.span).ok_or_else(|| {
                    crate::error::ClsError::CompileError(
                        "Arrow function sin función sintética (recolección)".to_string(),
                    )
                })?;
                let ti = self.fn_table_idx[name];
                let captures = self
                    .arrow_captures
                    .get(&a.span)
                    .cloned()
                    .unwrap_or_default();
                // Bloque de capturas `[n, v1, v2, ...]` (se evalúa primero).
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
            Expression::Ternary(c) => self.emit_ternary(c),
            Expression::IfExpr(c) => self.emit_if_expr(c),
            Expression::Match(m) => self.emit_match(m),
            Expression::Assignment(a) => self.emit_assignment(a),
            Expression::Parenthesized(inner, _) => self.emit_expression(inner),
            Expression::StringInterpolation(s) => self.emit_interpolation(s),
            // `x::miembro` (módulo/namespace importado): global `x::miembro`.
            Expression::NamespaceAccess(ns, member, span) => {
                // Variante de enum como valor (`resp::s1`): construye el valor
                // (unit -> empacado; payload-enum unit -> objeto de aridad 0).
                if let Some(ekey) = self.resolve_enum_key(ns) {
                    self.emit_enum_construct(&ekey, member, &[])?;
                    return Ok(());
                }
                let key = format!("{}::{}", ns, member);
                if let Some(g) = self.globals.get(&key).copied() {
                    self.body.push(Instruction::GlobalGet(g));
                    Ok(())
                } else {
                    Err(crate::error::ClsError::compile_at(
                        &format!(
                            "El miembro '{}' no existe o no se exporta en el módulo '{}' (fase de emisión).",
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
                "El JIT (subconjunto WASM) aún no soporta esta expresión: `{}`",
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
                // Dentro de `__next`, el `null` es el sentinel de fin de iteración
                // (distinto de 0 - un iterador puede devolver 0 como valor
                // legítimo). Fuera del protocolo, null = 0 (paridad histórica).
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


    /// Emite `env.fn_enter(nombre, line, col)` al inicio de una función CLS.
    /// Registra la función en el shadow call stack del host (para el trace de
    /// errores de runtime). `main` (la entrada) se registra sin ubicación
    /// (línea 0): el formateador lo muestra como `-> main` sin línea.
    pub(crate) fn emit_fn_enter(&mut self, f: &FunctionDecl) -> ClsResult<()> {
        use crate::backend::wasm::layout::{FRAME_SIZE, PENDING_CALL_SLOT_ADDR, SHADOW_LIMIT};
        let display = f
            .name
            .rsplit("::")
            .next()
            .unwrap_or(&f.name)
            .trim_start_matches("__s__")
            .to_string();
        let name_idx = self.intern_string(&display);
        let (line, col) = if f.name == "main" {
            (0u32, 0u32)
        } else {
            (f.span.start_line, f.span.start_col)
        };
        if self.shadow_ptr_global == 0 {
            // No instrumentado (sin global del shadow stack): no-op.
            return Ok(());
        }
        // Shadow call stack en memoria lineal (plan-performance/shadow-stack-wasm.md):
        // if shadow_ptr < SHADOW_LIMIT { shadow_ptr += 8; frame = {name_idx, line, col} }
        self.body.push(Instruction::GlobalGet(self.shadow_ptr_global));
        self.body.push(Instruction::I32Const(SHADOW_LIMIT as i32));
        self.body.push(Instruction::I32LtU);
        self.body.push(Instruction::If(BlockType::Empty));
        self.body.push(Instruction::GlobalGet(self.shadow_ptr_global));
        self.body.push(Instruction::I32Const(FRAME_SIZE as i32));
        self.body.push(Instruction::I32Add);
        self.body.push(Instruction::GlobalSet(self.shadow_ptr_global));
        let addr = self.fresh_local_ty(WasTy::I32);
        self.body.push(Instruction::GlobalGet(self.shadow_ptr_global));
        self.body.push(Instruction::I32Const(FRAME_SIZE as i32));
        self.body.push(Instruction::I32Sub);
        self.body.push(Instruction::LocalSet(addr));
        // name_idx (u32 -> i32 store).
        self.body.push(Instruction::LocalGet(addr));
        self.body.push(Instruction::I32Const(name_idx as i32));
        self.body.push(Instruction::I32Store(MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        }));
        if f.name == "main" {
            self.body.push(Instruction::LocalGet(addr));
            self.body.push(Instruction::I32Const(0));
            self.body.push(Instruction::I32Store(MemArg {
                offset: 4,
                align: 2,
                memory_index: 0,
            }));
            self.body.push(Instruction::LocalGet(addr));
            self.body.push(Instruction::I32Const(0));
            self.body.push(Instruction::I32Store(MemArg {
                offset: 8,
                align: 2,
                memory_index: 0,
            }));
        } else {
            // line = pending_call_site.line si != 0 (call site del llamador),
            // si no f.span.line (paridad con host_fn_enter).
            // En WASM: `if pending == 0 { line = f.span.line }` (If entra al
            // then solo si la condicion es != 0, por eso I32Eqz).
            let tl = self.fresh_local_ty(WasTy::I32);
            self.body.push(Instruction::I32Const(PENDING_CALL_SLOT_ADDR as i32));
            self.body.push(Instruction::I32Load(MemArg {
                offset: 0,
                align: 2,
                memory_index: 0,
            }));
            self.body.push(Instruction::LocalSet(tl));
            self.body.push(Instruction::LocalGet(tl));
            self.body.push(Instruction::I32Eqz);
            self.body.push(Instruction::If(BlockType::Empty));
            self.body.push(Instruction::I32Const(line as i32));
            self.body.push(Instruction::LocalSet(tl));
            self.body.push(Instruction::End);
            let tc = self.fresh_local_ty(WasTy::I32);
            self.body.push(Instruction::I32Const((PENDING_CALL_SLOT_ADDR + 4) as i32));
            self.body.push(Instruction::I32Load(MemArg {
                offset: 0,
                align: 2,
                memory_index: 0,
            }));
            self.body.push(Instruction::LocalSet(tc));
            self.body.push(Instruction::LocalGet(tc));
            self.body.push(Instruction::I32Eqz);
            self.body.push(Instruction::If(BlockType::Empty));
            self.body.push(Instruction::I32Const(col as i32));
            self.body.push(Instruction::LocalSet(tc));
            self.body.push(Instruction::End);
            self.body.push(Instruction::LocalGet(addr));
            self.body.push(Instruction::LocalGet(tl));
            self.body.push(Instruction::I32Store(MemArg {
                offset: 4,
                align: 2,
                memory_index: 0,
            }));
            self.body.push(Instruction::LocalGet(addr));
            self.body.push(Instruction::LocalGet(tc));
            self.body.push(Instruction::I32Store(MemArg {
                offset: 8,
                align: 2,
                memory_index: 0,
            }));
        }
        self.body.push(Instruction::End);
        Ok(())
    }


    /// Emite el call site pendiente (line, col) del llamador en el slot fijo.
    /// El `fn_enter` del callee lo lee como span del frame.
    pub(crate) fn emit_call_site(&mut self, span: &Span) {
        use crate::backend::wasm::layout::PENDING_CALL_SLOT_ADDR;
        if self.shadow_ptr_global == 0 {
            return;
        }
        self.body.push(Instruction::I32Const(PENDING_CALL_SLOT_ADDR as i32));
        self.body.push(Instruction::I32Const(span.start_line as i32));
        self.body.push(Instruction::I32Store(MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        }));
        self.body.push(Instruction::I32Const((PENDING_CALL_SLOT_ADDR + 4) as i32));
        self.body.push(Instruction::I32Const(span.start_col as i32));
        self.body.push(Instruction::I32Store(MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        }));
    }


    /// Emite una llamada a una función host del nodo (intrinsic) vía el canal
    /// genérico `env.host_call(id, ptr, n)`. Los args viajan empaquetados en
    /// memoria: `[n:i64][(val:i64, tag:i64)*n]` (tag = `cls_kind_code`).
    pub(crate) fn emit_host_call(&mut self, intr: &HostIntrinsic, c: &CallExpr) -> ClsResult<()> {
        let n = c.args.len() as i64;
        // 1. Evaluar cada arg y guardarlo en un temporal (bits uniformes i64:
        //    float -> reinterpret bits; bool/char -> extender a i64).
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
        // 4. Por arg: val + tag. (El addr de los memory ops es i32 -> wrap.)
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


    /// Decrementa el shadow call stack al salir de una función CLS.
    pub(crate) fn emit_fn_exit(&mut self) {
        use crate::backend::wasm::layout::FRAME_SIZE;
        if self.shadow_ptr_global == 0 {
            return;
        }
        self.body.push(Instruction::GlobalGet(self.shadow_ptr_global));
        self.body.push(Instruction::I32Const(FRAME_SIZE as i32));
        self.body.push(Instruction::I32Sub);
        self.body.push(Instruction::GlobalSet(self.shadow_ptr_global));
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

    /// `?:` estilo JS (F3). Expresión pura: no es target de `<-` (emite un
    /// `If(Result)` como el ternario legacy). El tipo del resultado lo fija la
    /// rama `then` (paridad con `emit_conditional`).
    pub(crate) fn emit_ternary(&mut self, c: &TernaryExpr) -> ClsResult<()> {
        let w = self.value_type(&c.then_expr)?;
        self.emit_expression(&c.condition)?;
        self.coerce_to_bool(&c.condition)?;
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

    /// `if (c) { ... } else { ... }` como expresión (F3). Abre `block (result)`
    /// como **ternary-target**; las ramas entregan su valor con `* <-`
    /// (arrowReturn, F4), que emite `br` a ese bloque. Requiere `else`.
    pub(crate) fn emit_if_expr(&mut self, e: &IfExpr) -> ClsResult<()> {
        let res_cls = self.types.get(&e.span).cloned().unwrap_or(Type::Void);
        let is_void = matches!(res_cls, Type::Void | Type::Empty);
        if is_void {
            return Err(crate::error::ClsError::compile_at(
                "la expresión `if` debe devolver un valor: cada rama debe terminarlo con `<-`",
                &e.span,
            ));
        }
        let res_w = was_type(&res_cls).unwrap_or(WasTy::I64);

        self.emit_expression(&e.condition)?;
        self.coerce_to_bool(&e.condition)?;

        self.block_depth += 1;
        self.body
            .push(Instruction::If(BlockType::Result(res_w.val_type())));
        let done_at = self.block_depth;
        self.ternary_targets.push((done_at, res_w));

        self.dead_flow = false;
        self.emit_scoped_block(&e.then_block)?;
        if !self.dead_flow {
            return Err(crate::error::ClsError::compile_at(
                "la rama `then` de la expresión `if` debe entregar su valor con `<-`",
                &e.then_block.span,
            ));
        }
        // La rama divergió (`br` al target de este `if`-expr, a un target más
        // externo, `return` o `unreachable`). El validador WASM NO propaga la
        // divergencia a través del `end` de un `if`/`match`/`switch` interno
        // cuyas ramas se fueron por `br`: tras ese `end` la caída se considera
        // alcanzable y el bloque `(result)` quedaría sin valor en la pila
        // ("expected i64 but nothing on stack"). Emitir `unreachable` marca el
        // tramo final como divergente (pila polimórfica) y valida el bloque.
        self.body.push(Instruction::Unreachable);
        self.body.push(Instruction::Else);
        self.dead_flow = false;
        self.emit_scoped_block(&e.else_block)?;
        if !self.dead_flow {
            return Err(crate::error::ClsError::compile_at(
                "la rama `else` de la expresión `if` debe entregar su valor con `<-`",
                &e.else_block.span,
            ));
        }
        self.body.push(Instruction::Unreachable);
        self.body.push(Instruction::End); // if (result) == $done
        self.block_depth -= 1;

        self.ternary_targets.pop();
        self.dead_flow = false;
        Ok(())
    }

    /// `* <- expr` / `** <- expr` / `match <- expr` / `match* <- expr` / ... (F4).
    /// Resuelve el target léxico en las pilas y emite el valor + `br` al bloque
    /// `(result)` del target (desenrollando `if`/`while`/`for` intermedios: un
    /// `br` a un bloque ancestro los sale naturalmente). Sin target -> error.
    pub(crate) fn emit_arrow_return(&mut self, a: &ArrowReturn) -> ClsResult<()> {
        let (depth, target_w): (u32, WasTy) = if a.is_match {
            let entry = self.match_targets.iter().rev().nth(a.stars).ok_or_else(|| {
                crate::error::ClsError::compile_at(
                    &format!(
                        "`{}<-` sin `match` envolvente en la función",
                        "match".to_string() + &"*".repeat(a.stars)
                    ),
                    &a.span,
                )
            })?;
            match entry.1 {
                Some(w) => (entry.0, w),
                None => {
                    return Err(crate::error::ClsError::compile_at(
                        "no se puede devolver un valor (`<-`) a un `match` que no devuelve valor",
                        &a.span,
                    ))
                }
            }
        } else {
            if a.stars == 0 {
                return Err(crate::error::ClsError::compile_at(
                    "`<-` requiere al menos un `*` (ternary-target) o `match` (match-target)",
                    &a.span,
                ));
            }
            *self.ternary_targets.iter().rev().nth(a.stars - 1).ok_or_else(|| {
                crate::error::ClsError::compile_at(
                    &format!(
                        "`{}<-` sin target: no hay `if`-expresión ni arma de `match` (bloque) envolvente",
                        "*".repeat(a.stars)
                    ),
                    &a.span,
                )
            })?
        };
        let val_w = self.value_type(&a.value)?;
        self.emit_expression(&a.value)?;
        if target_w == WasTy::F64 && val_w != WasTy::F64 {
            self.f64_promote(&a.value)?;
        }
        self.body.push(Instruction::Br(self.block_depth - depth));
        self.dead_flow = true;
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


    /// `"Hola $nombre ${expr}"` -> concatenación de las partes (toString de cada expr).
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
            self.emit_str_host("__intr_str_concat");
            self.body.push(Instruction::LocalSet(acc));
        }
        self.body.push(Instruction::LocalGet(acc));
        Ok(())
    }

}
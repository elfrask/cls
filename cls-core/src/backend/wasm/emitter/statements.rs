//! Statements: emit_statement, foreach, switch, try, if, while, loop, for (Fase 1: extraido de emitter/mod.rs).

use super::*;

impl<'a> FuncEmitter<'a> {


    // == Emisión de statements ============================================

    pub(crate) fn emit_statement(&mut self, stmt: &Statement) -> ClsResult<()> {
        // Código muerto (después de un return/break/continue o de un if con
        // todas las ramas terminadas): se omite. Emitirlo genera código muerto
        // inválido para cranelift tras el `end` de frames (if/switch/try).
        if self.dead_flow {
            return Ok(());
        }
        match stmt {
            Statement::VarDecl(v) | Statement::ConstDecl(v) => {
                let ty = match (&v.type_ann, &v.value) {
                    (Some(ann), Some(val)) => match was_type(&annotation_to_type(ann)) {
                        Ok(w) => w,
                        // Anotación no resuelta (alias/unioón) -> tipo del valor.
                        Err(_) => self.value_type(val)?,
                    },
                    (Some(ann), None) => was_type(&annotation_to_type(ann))?,
                    (None, Some(val)) => self.value_type(val)?,
                    (None, None) => WasTy::I64,
                };
                let idx = self.declare_var_ty(&v.name, ty);
                // Tipo CLS declarado de la local (frontera de asignación).
                let declared_cls: Option<Type> = v
                    .type_ann
                    .as_ref()
                    .map(|a| annotation_to_type(a))
                    .or_else(|| {
                        v.value
                            .as_ref()
                            .and_then(|val| self.types.get(&expr_span(val)).cloned())
                    });
                match &declared_cls {
                    Some(t) => {
                        self.local_cls_types.insert(v.name.clone(), t.clone());
                    }
                    None => {}
                }
                if let Some(value) = &v.value {
                    // Frontera única: shape contiguo hacia destino dinámico.
                    self.emit_coerce(value, declared_cls.as_ref())?;
                    // VarDecl `f: Float = 42;`: si el local es float y el RHS
                    // es int (o literal int), promover a f64. El store del
                    // local espera f64; sin esto, el modulo WASM es invalido
                    // (type mismatch: expected f64, found i64). Mismo patron
                    // que el assignment simple en assignment.rs:110-114.
                    if ty == WasTy::F64
                        && self.value_type(value)? != WasTy::F64
                    {
                        self.f64_promote(value)?;
                    }
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
                self.dead_flow = false;
                Ok(())
            }
            Statement::FunctionDecl(_) => Ok(()),
            Statement::Expression(e) => {
                self.emit_expression(e)?;
                self.emit_drop(e)?;
                self.dead_flow = false;
                Ok(())
            }
            Statement::Return(e) => {
                // Salida abrupta que cruza un `finally` abierto: diferirla. Se
                // guarda el valor (si lo hay) y se salta al `finally`; el
                // despacho tras el `finally` completa el `return`.
                if let Some(&frame_label) = self.finally_stack.last() {
                    let kind = self.ensure_exit_kind_local();
                    if self.returns_value() {
                        if let Some(expr) = e {
                            let dest = self.fn_ret.clone();
                            self.emit_coerce(expr, dest.as_ref())?;
                            let val = self.ensure_exit_val_local();
                            self.body.push(Instruction::LocalSet(val));
                        }
                    }
                    self.body.push(Instruction::I64Const(1));
                    self.body.push(Instruction::LocalSet(kind));
                    let depth = self.block_depth - frame_label;
                    self.body.push(Instruction::Br(depth));
                    self.dead_flow = true;
                    return Ok(());
                }
                if e.is_some() {
                    let expr = e.as_ref().unwrap();
                    // Frontera única: shape contiguo hacia retorno dinámico.
                    let dest = self.fn_ret.clone();
                    self.emit_coerce(expr, dest.as_ref())?;
                }
                // Des-registrar el frame antes de cortar: `Instruction::Return`
                // salta al final sin pasar por el `fn_exit` del cuerpo.
                self.emit_fn_exit();
                self.body.push(Instruction::Return);
                self.dead_flow = true;
                Ok(())
            }
            Statement::Break(bspan) => {
                let break_at = self.loop_stack.last().ok_or_else(|| {
                    crate::error::ClsError::compile_at("break fuera de loop", bspan)
                })?.break_at;
                // Si el `finally` más interno está DENTRO del loop (su label es
                // más profundo que el target del break), el break sale del
                // `try`: diferirlo para que corra el `finally` antes de saltar.
                if let Some(&frame_label) = self.finally_stack.last() {
                    if frame_label > break_at {
                        let kind = self.ensure_exit_kind_local();
                        self.body.push(Instruction::I64Const(2));
                        self.body.push(Instruction::LocalSet(kind));
                        let depth = self.block_depth - frame_label;
                        self.body.push(Instruction::Br(depth));
                        self.dead_flow = true;
                        return Ok(());
                    }
                }
                let depth = self.block_depth.saturating_sub(break_at);
                self.body.push(Instruction::Br(depth));
                self.dead_flow = true;
                Ok(())
            }
            Statement::Continue(cspan) => {
                let continue_at = self.loop_stack.last().ok_or_else(|| {
                    crate::error::ClsError::compile_at("continue fuera de loop", cspan)
                })?.continue_at;
                // Igual que `break` pero contra el target del `continue`.
                if let Some(&frame_label) = self.finally_stack.last() {
                    if frame_label > continue_at {
                        let kind = self.ensure_exit_kind_local();
                        self.body.push(Instruction::I64Const(3));
                        self.body.push(Instruction::LocalSet(kind));
                        let depth = self.block_depth - frame_label;
                        self.body.push(Instruction::Br(depth));
                        self.dead_flow = true;
                        return Ok(());
                    }
                }
                let depth = self.block_depth.saturating_sub(continue_at);
                self.body.push(Instruction::Br(depth));
                self.dead_flow = true;
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
            // `when` -> compile-time: emitir solo la rama que matchea el target actual.
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
            | Statement::ExportList(_)
            | Statement::InterfaceDecl(_)
            | Statement::NamespaceDecl(_)
            | Statement::ModuleDecl(_)
            | Statement::Config(_) => Ok(()),
            Statement::Cmx(c) => {
                self.emit_cmx(c)?;
                self.emit_drop(&Expression::Cmx(c.clone()))?;
                self.dead_flow = false;
                Ok(())
            }
            other => Err(self.unsupported_stmt(other)),
        }
    }



    pub(crate) fn unsupported_stmt(&self, stmt: &Statement) -> crate::error::ClsError {
        crate::error::ClsError::CompileError(format!(
            "El JIT (subconjunto WASM) aún no soporta este statement: {}",
            statement_display(stmt)
        ))
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
            // Cada case es una rama independiente (alcanzable por el else del
            // case anterior): el flujo muerto de un case no se propaga al otro.
            self.dead_flow = false;
            for st in &case.block.statements {
                self.emit_statement(st)?;
            }
            let depth = self.block_depth.saturating_sub(done_at);
            self.body.push(Instruction::Br(depth));
            self.body.push(Instruction::End);
            self.block_depth -= 1;
        }
        if let Some(def) = &s.default {
            self.dead_flow = false;
            for st in &def.statements {
                self.emit_statement(st)?;
            }
        }
        self.body.push(Instruction::End); // block done
        self.block_depth -= 1;
        let _ = d;
        // Sin default, o default con caída -> flujo vivo (los casos br al done).
        if s.default.is_none() {
            self.dead_flow = false;
        }
        Ok(())
    }



    /// `with x in (expr) { ... }` -> local temporal + bloque.
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



    /// `try { ... } catch (e) { ... } finally { ... }` - excepciones WASM (try_table).
    /// El `finally` se ejecuta SIEMPRE: en la salida normal del `try`, tras un
    /// `catch` local que captura la excepción, y antes de re-lanzar cuando la
    /// excepción propaga (sin `catch`). El `catch` recibe
    /// `e = "Error de runtime: " + msg` (paridad con `e.to_string()`).
    pub(crate) fn emit_try(&mut self, stmt: &TryStatement) -> ClsResult<()> {
        if !self.exceptions {
            return Err(crate::error::ClsError::compile_at(
                "try/catch no soportado en este runtime: el backend se compiló sin \
                 excepciones WASM (wasmi). Usa el runtime wasmtime o el WASM nativo del navegador.",
                &stmt.span,
            ));
        }
        let was_dead = self.dead_flow;
        let has_catch = !stmt.catch_clauses.is_empty();
        let has_finally = stmt.finally_block.is_some();

        // Excepción pendiente de re-lanzar TRAS el `finally`: (a) sin `catch`,
        // la del cuerpo del `try`; (b) con `catch` Y `finally`, la que lance el
        // propio cuerpo del `catch` (si no, saltaría el `finally`). Se guarda el
        // payload en locales y se re-lanza con el mismo tag/span.
        let (pend_msg, pend_span, pend_flag) = if !has_catch || has_finally {
            let m = self.fresh_local();
            let s = self.fresh_local();
            let f = self.fresh_local();
            self.body.push(Instruction::I64Const(0));
            self.body.push(Instruction::LocalSet(f));
            (m, s, f)
        } else {
            (0, 0, 0)
        };

        // block $finally_label (Empty): ambas salidas (normal y capturada)
        // convergen aquí; tras su End corre el `finally` una sola vez.
        self.block_depth += 1;
        self.body.push(Instruction::Block(BlockType::Empty));
        let finally_label = self.block_depth;
        // Registrar el `finally` abierto: las salidas abruptas desde el cuerpo
        // del `try`/`catch` que lo cruzan se diferirán a este label.
        if has_finally {
            self.finally_stack.push(finally_label);
        }
        // block $handler (result [i64, i64]) - el catch aterriza tras su End con
        // el payload [msg, span].
        self.block_depth += 1;
        self.body.push(Instruction::Block(BlockType::FunctionType(
            self.eh_handler_ty,
        )));
        let handler = self.block_depth;
        // try_table: captura nuestro tag -> br al label del $handler con [msg, span]
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
        self.dead_flow = false;
        for s in &stmt.try_block.statements {
            self.emit_statement(s)?;
        }
        let try_dead = self.dead_flow;
        self.body.push(Instruction::End); // cierra try_table
        self.block_depth -= 1;
        // flujo normal (sin excepción) -> br a $finally_label (salta el handler).
        let br_finally = self.block_depth - finally_label;
        self.body.push(Instruction::Br(br_finally));
        self.body.push(Instruction::End); // cierra $handler -> el catch aterriza AQUÍ con [msg, span]
        self.block_depth -= 1;
        // handler: payload [msg, span] en el stack (span arriba, msg debajo)
        let mut catch_dead = true;
        if has_catch {
            let catch = &stmt.catch_clauses[0];
            let span_tmp = self.fresh_local();
            self.body.push(Instruction::LocalSet(span_tmp));
            let msg_tmp = self.fresh_local();
            self.body.push(Instruction::LocalSet(msg_tmp));
            // e = "Error de runtime: " + msg
            let pref = self.intern_string("Error de runtime: ");
            self.emit_load_str(pref);
            self.body.push(Instruction::LocalGet(msg_tmp));
            self.emit_str_host("__intr_str_concat");
            let e_local = self.declare_var_ty(&catch.param_name, WasTy::I64);
            self.body.push(Instruction::LocalSet(e_local));
            if has_finally {
                // El cuerpo del `catch` también puede salir abruptamente con una
                // excepción: se envuelve en un try_table que captura el tag y lo
                // deja PENDIENTE para re-lanzarlo tras el `finally` (sin esto, la
                // excepción del `catch` saltaría el `finally`).
                self.block_depth += 1;
                self.body.push(Instruction::Block(BlockType::FunctionType(
                    self.eh_handler_ty,
                )));
                let guard = self.block_depth;
                self.block_depth += 1;
                let guard_label = self.block_depth - guard - 1;
                self.body.push(Instruction::TryTable(
                    BlockType::Empty,
                    Cow::Owned(vec![Catch::One {
                        tag: self.tag_idx,
                        label: guard_label,
                    }]),
                ));
                self.dead_flow = false;
                for s in &catch.block.statements {
                    self.emit_statement(s)?;
                }
                self.body.push(Instruction::End); // cierra try_table
                self.block_depth -= 1;
                // salida normal del catch -> al `finally` (salta el handler).
                let d = self.block_depth - finally_label;
                self.body.push(Instruction::Br(d));
                self.body.push(Instruction::End); // cierra $guard (handler)
                self.block_depth -= 1;
                // handler: payload [msg, span] -> excepción pendiente.
                self.body.push(Instruction::LocalSet(pend_span));
                self.body.push(Instruction::LocalSet(pend_msg));
                self.body.push(Instruction::I64Const(1));
                self.body.push(Instruction::LocalSet(pend_flag));
                catch_dead = false;
            } else {
                self.dead_flow = false;
                for s in &catch.block.statements {
                    self.emit_statement(s)?;
                }
                catch_dead = self.dead_flow;
            }
        } else {
            // sin catch: guardar el payload y marcar re-lanzamiento pendiente.
            self.body.push(Instruction::LocalSet(pend_span));
            self.body.push(Instruction::LocalSet(pend_msg));
            self.body.push(Instruction::I64Const(1));
            self.body.push(Instruction::LocalSet(pend_flag));
        }
        self.body.push(Instruction::End); // cierra $finally_label
        self.block_depth -= 1;
        // El `finally` ya no está "abierto": una salida abrupta dentro del
        // propio `finally` no debe re-diferirse a él (evita bucles), sino a los
        // `finally` externos.
        if has_finally {
            self.finally_stack.pop();
        }
        // finally: se ejecuta en TODOS los caminos vivos.
        let mut finally_dead = false;
        if let Some(f) = &stmt.finally_block {
            self.dead_flow = false;
            for s in &f.statements {
                self.emit_statement(s)?;
            }
            finally_dead = self.dead_flow;
        }
        // Re-lanzar si quedó una excepción pendiente (sin `catch`, o lanzada
        // por el cuerpo del `catch` cuando hay `finally`).
        if !has_catch || has_finally {
            self.dead_flow = false;
            self.body.push(Instruction::LocalGet(pend_flag));
            self.body.push(Instruction::I64Eqz);
            self.body.push(Instruction::I32Eqz);
            self.block_depth += 1;
            self.body.push(Instruction::If(BlockType::Empty));
            self.body.push(Instruction::LocalGet(pend_msg));
            self.body.push(Instruction::LocalGet(pend_span));
            self.body.push(Instruction::Throw(self.tag_idx));
            self.body.push(Instruction::Unreachable);
            self.body.push(Instruction::End);
            self.block_depth -= 1;
        }
        // Despachar las salidas abruptas diferidas: si `exit_kind` quedó fijado
        // (return/break/continue desde el `try`/`catch`), saltar al `finally`
        // externo (si la salida todavía debe cruzarlo) o completar la salida.
        if has_finally {
            self.emit_deferred_exit_dispatch();
        }
        // Vivo si algún camino sobrevive (normal y/o capturado) y el finally no
        // terminó el flujo.
        let normal_alive = !try_dead;
        let caught_alive = has_catch && !catch_dead;
        let mut alive = normal_alive || caught_alive;
        if has_finally && finally_dead {
            alive = false;
        }
        self.dead_flow = was_dead || !alive;
        Ok(())
    }



    /// Despacha una salida abrupta diferida a un `finally` (ver `emit_try`).
    ///
    /// Se emite tras ejecutar el `finally` y lee el local compartido
    /// `exit_kind` (0 = normal, 1 = return, 2 = break, 3 = continue):
    /// - si la salida todavía debe cruzar otro `finally` externo, salta a él
    ///   dejando `exit_kind` intacto (su despacho completará la salida);
    /// - si no, completa la salida (`return` con su valor, o `br` al loop).
    ///
    /// Los `break`/`continue` targetean el loop más interno vivo en el punto del
    /// `try` (el único al que una salida diferida puede apuntar: si el loop
    /// estuviera dentro del `try`, el `finally` no se cruzaría y no se difiere).
    pub(crate) fn emit_deferred_exit_dispatch(&mut self) {
        let kind = match self.exit_kind_local {
            Some(k) => k,
            None => return,
        };
        let has_val = self.returns_value();
        let val = self.exit_val_local.unwrap_or(0);
        let outer_finally = self.finally_stack.last().copied();
        let loop_targets = self.loop_stack.last().map(|c| (c.break_at, c.continue_at));

        // --- return (exit_kind == 1) ---
        self.body.push(Instruction::LocalGet(kind));
        self.body.push(Instruction::I64Const(1));
        self.body.push(Instruction::I64Eq);
        self.block_depth += 1;
        self.body.push(Instruction::If(BlockType::Empty));
        if let Some(lbl) = outer_finally {
            let d = self.block_depth - lbl;
            self.body.push(Instruction::Br(d));
        } else {
            if has_val {
                self.body.push(Instruction::LocalGet(val));
            }
            self.emit_fn_exit();
            self.body.push(Instruction::Return);
        }
        self.body.push(Instruction::End);
        self.block_depth -= 1;

        // --- break / continue (exit_kind == 2 / 3) ---
        if let Some((break_at, continue_at)) = loop_targets {
            self.emit_deferred_loop_exit(kind, 2, break_at, outer_finally);
            self.emit_deferred_loop_exit(kind, 3, continue_at, outer_finally);
        }
    }

    /// Una rama del despacho diferido para `break` (code 2) o `continue`
    /// (code 3): salta al `finally` externo si aún está dentro del loop, o al
    /// target del loop (limpiando `exit_kind` para no re-disparar el despacho).
    fn emit_deferred_loop_exit(
        &mut self,
        kind: u32,
        code: i64,
        target: u32,
        outer_finally: Option<u32>,
    ) {
        self.body.push(Instruction::LocalGet(kind));
        self.body.push(Instruction::I64Const(code));
        self.body.push(Instruction::I64Eq);
        self.block_depth += 1;
        self.body.push(Instruction::If(BlockType::Empty));
        if let Some(lbl) = outer_finally.filter(|l| *l > target) {
            let d = self.block_depth - lbl;
            self.body.push(Instruction::Br(d));
        } else {
            self.body.push(Instruction::I64Const(0));
            self.body.push(Instruction::LocalSet(kind));
            let d = self.block_depth - target;
            self.body.push(Instruction::Br(d));
        }
        self.body.push(Instruction::End);
        self.block_depth -= 1;
    }



    pub(crate) fn emit_if(&mut self, i: &IfStatement) -> ClsResult<()> {
        let was_dead = self.dead_flow;
        self.emit_expression(&i.condition)?;
        self.coerce_to_bool(&i.condition)?;
        self.block_depth += 1;
        self.body.push(Instruction::If(BlockType::Empty));
        self.dead_flow = false;
        for s in &i.then_block.statements {
            self.emit_statement(s)?;
        }
        let then_dead = self.dead_flow;
        let has_elif = !i.elif_branches.is_empty();
        let has_else = i.else_block.is_some();
        if has_elif || has_else {
            self.body.push(Instruction::Else);
        }
        // Cadena de elifs anidados dentro del else; el último cede al else final.
        let mut branch_deads: Vec<bool> = Vec::new();
        let mut else_dead = false;
        for (k, branch) in i.elif_branches.iter().enumerate() {
            self.dead_flow = false;
            self.emit_expression(&branch.condition)?;
            self.coerce_to_bool(&branch.condition)?;
            self.block_depth += 1;
            self.body.push(Instruction::If(BlockType::Empty));
            self.dead_flow = false;
            for s in &branch.block.statements {
                self.emit_statement(s)?;
            }
            branch_deads.push(self.dead_flow);
            let last = k == i.elif_branches.len() - 1;
            if last {
                if let Some(else_b) = &i.else_block {
                    self.body.push(Instruction::Else);
                    self.dead_flow = false;
                    for s in &else_b.statements {
                        self.emit_statement(s)?;
                    }
                    else_dead = self.dead_flow;
                }
            } else {
                self.body.push(Instruction::Else);
            }
            self.body.push(Instruction::End);
            self.block_depth -= 1;
        }
        if !has_elif && has_else {
            self.dead_flow = false;
            let else_b = i.else_block.as_ref().unwrap();
            for s in &else_b.statements {
                self.emit_statement(s)?;
            }
            else_dead = self.dead_flow;
        }
        self.body.push(Instruction::End);
        self.block_depth -= 1;
        // Flujo muerto tras el if: todas las ramas terminaron (then + toda la
        // cadena elif + else final cuando existe).
        let chain_dead = if has_elif {
            branch_deads.iter().all(|d| *d) && (if has_else { else_dead } else { false })
        } else if has_else {
            else_dead
        } else {
            false
        };
        self.dead_flow = was_dead || (then_dead && chain_dead);
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
        self.dead_flow = false;
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
        // El loop siempre puede salir (condición/break) -> flujo vivo tras él.
        self.dead_flow = false;
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
        self.dead_flow = false;
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
        self.dead_flow = false;
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
        // continue block: el `continue` salta aquí y ejecuta el update (evita
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
        self.dead_flow = false;
        for s in &f.block.statements {
            self.emit_statement(s)?;
        }
        // cerrar el continue block -> se ejecuta el update
        self.body.push(Instruction::End);
        self.block_depth -= 1;
        // El update se alcanza vía el `continue` (back-edge) aunque el body
        // haya terminado el flujo: es código vivo sintácticamente.
        self.dead_flow = false;
        if let Some(update) = &f.update {
            self.emit_expression(update)?;
            self.emit_drop(update)?;
        }
        // volver al loop (que está en continue_at - 1)
        let depth = self.block_depth.saturating_sub(continue_at - 1);
        self.body.push(Instruction::Br(depth));
        self.body.push(Instruction::End); // loop
        self.block_depth -= 1;
        self.body.push(Instruction::End); // block
        self.block_depth -= 1;
        self.loop_stack.pop();
        self.dead_flow = false;
        Ok(())
    }

}

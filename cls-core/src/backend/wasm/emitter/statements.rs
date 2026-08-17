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
                if e.is_some() {
                    self.emit_expression(e.as_ref().unwrap())?;
                }
                // Des-registrar el frame antes de cortar: `Instruction::Return`
                // salta al final sin pasar por el `fn_exit` del cuerpo.
                self.emit_fn_exit();
                self.body.push(Instruction::Return);
                self.dead_flow = true;
                Ok(())
            }
            Statement::Break(bspan) => {
                let ctx = self.loop_stack.last().ok_or_else(|| {
                    crate::error::ClsError::compile_at("break fuera de loop", bspan)
                })?;
                let depth = self.block_depth.saturating_sub(ctx.break_at);
                self.body.push(Instruction::Br(depth));
                self.dead_flow = true;
                Ok(())
            }
            Statement::Continue(cspan) => {
                let ctx = self.loop_stack.last().ok_or_else(|| {
                    crate::error::ClsError::compile_at("continue fuera de loop", cspan)
                })?;
                let depth = self.block_depth.saturating_sub(ctx.continue_at);
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
            | Statement::Include(_)
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
    /// Paridad con el walker: el finally solo se ejecuta si NO hubo catch; el catch
    /// recibe `e = "Error de runtime: " + msg` (e.to_string() del walker).
    pub(crate) fn emit_try(&mut self, stmt: &TryStatement) -> ClsResult<()> {
        if !self.exceptions {
            return Err(crate::error::ClsError::compile_at(
                "try/catch no soportado en este runtime: el backend se compiló sin \
                 excepciones WASM (wasmi). Usa el runtime wasmtime o el WASM nativo del navegador.",
                &stmt.span,
            ));
        }
        let was_dead = self.dead_flow;
        // block $outer (Empty)
        self.block_depth += 1;
        self.body.push(Instruction::Block(BlockType::Empty));
        let outer = self.block_depth;
        // block $handler (result [i64, i64]) - su label (continuation, tras su End)
        // es donde aterriza el catch con el payload [msg, span].
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
        // flujo normal (sin excepción) -> br al $outer (salta el handler)
        let br_outer = self.block_depth - outer;
        self.body.push(Instruction::Br(br_outer));
        self.body.push(Instruction::End); // cierra $handler -> el catch aterriza AQUÍ con [msg, span]
        self.block_depth -= 1;
        // handler: payload [msg, span] en el stack (span arriba, msg debajo)
        let mut catch_dead = true;
        if stmt.catch_clauses.is_empty() {
            let span_tmp = self.fresh_local();
            self.body.push(Instruction::LocalSet(span_tmp));
            let msg_tmp = self.fresh_local();
            self.body.push(Instruction::LocalSet(msg_tmp));
            if let Some(f) = &stmt.finally_block {
                self.dead_flow = false;
                for s in &f.statements {
                    self.emit_statement(s)?;
                }
            }
            // re-lanzar con el mismo payload (equivalente a Rethrow)
            self.body.push(Instruction::LocalGet(msg_tmp));
            self.body.push(Instruction::LocalGet(span_tmp));
            self.body.push(Instruction::Throw(self.tag_idx));
            self.body.push(Instruction::Unreachable);
            catch_dead = true;
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
            self.emit_str_host("__intr_str_concat", HostFn::StrConcat);
            let e_local = self.declare_var_ty(&catch.param_name, WasTy::I64);
            self.body.push(Instruction::LocalSet(e_local));
            self.dead_flow = false;
            for s in &catch.block.statements {
                self.emit_statement(s)?;
            }
            catch_dead = self.dead_flow;
        }
        self.body.push(Instruction::End); // cierra $outer
        self.block_depth -= 1;
        self.dead_flow = was_dead || (try_dead && catch_dead);
        Ok(())
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

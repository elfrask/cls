//! `match` (feature F1): emisión como EXPRESIÓN con resultado.
//!
//! Patrón de `emit_conditional`: se abre `block $done` con el tipo de resultado
//! (o `Empty` si el `match` no devuelve valor) y cada arma emite su condición
//! (i32) seguida de un `If(Empty)` cuyo cuerpo deja el resultado y hace
//! `br $done`. Sin `default` y sin coincidencia -> error de runtime
//! (`throw`/`unreachable`).
//!
//! Los 4 modos comparten el armazón; sólo cambia cómo se computa la condición
//! de cada arma:
//! - VALOR: igualdad `scrut == literal` (String por `__intr_str_eq`, Int/enum
//!   por `i64.eq`, Bool/Char por `i32.eq`, Float por `f64.eq`).
//! - PATRÓN: igualdad contra el índice empaquetado de la variante de enum.
//! - TIPO: chequeo de tipo del valor (`is`), reusando la lógica de `binary.rs`.
//! - CONDICIÓN: la expresión de la arma (Bool) directamente.

use super::*;

impl<'a> FuncEmitter<'a> {
    pub(crate) fn emit_match(&mut self, m: &MatchExpr) -> ClsResult<()> {
        let res_ty = self.types.get(&m.span).cloned().unwrap_or(Type::Void);
        let is_void = matches!(res_ty, Type::Void | Type::Empty);

        let res_w = if is_void {
            WasTy::I64
        } else {
            let first = m.arms.first().ok_or_else(|| {
                crate::error::ClsError::compile_at("`match` vacío", &m.span)
            })?;
            match &first.body {
                MatchBody::Expr(e) => self.value_type(e)?,
                MatchBody::Block(_) => return Err(self.match_block_unsupported(&m.span)),
            }
        };

        // Scrutinee (evaluado UNA vez) salvo en modo condición.
        let mut scrut_local: Option<u32> = None;
        let mut scrut_was = WasTy::I64;
        let mut scrut_cls = Type::Any;
        let mut scrut_is_str = false;
        if m.mode != MatchMode::Cond {
            let s = m.scrutinee.as_ref().ok_or_else(|| {
                crate::error::ClsError::compile_at("`match` sin scrutinee", &m.span)
            })?;
            scrut_cls = self.types.get(&expr_span(s)).cloned().unwrap_or(Type::Any);
            scrut_is_str = self.is_string_expr(s);
            scrut_was = if scrut_is_str {
                WasTy::I64
            } else {
                self.value_type(s)?
            };
            self.emit_expression(s)?;
            let l = self.fresh_local_ty(scrut_was);
            self.body.push(Instruction::LocalSet(l));
            scrut_local = Some(l);
        }

        self.block_depth += 1;
        self.body.push(Instruction::Block(if is_void {
            BlockType::Empty
        } else {
            BlockType::Result(res_w.val_type())
        }));
        let done_at = self.block_depth;

        for arm in &m.arms {
            self.emit_match_condition(
                m.mode,
                &arm.patterns,
                scrut_local,
                scrut_was,
                scrut_is_str,
                &scrut_cls,
            )?;
            self.block_depth += 1;
            self.body.push(Instruction::If(BlockType::Empty));
            self.dead_flow = false;
            // F2: ligar el payload de la variante (modo PATRÓN con `v(binds)`).
            let saved = self.enter_var_scope();
            if let (Some(sl), Some((key, vidx, binds))) =
                (scrut_local, self.arm_payload_binding(&scrut_cls, &arm.patterns))
            {
                self.emit_match_payload_binds(sl, &key, vidx, &binds)?;
            }
            let body = match &arm.body {
                MatchBody::Expr(e) => e,
                MatchBody::Block(_) => return Err(self.match_block_unsupported(&arm.span)),
            };
            self.emit_expression(body)?;
            if is_void {
                self.emit_drop(body)?;
            } else {
                // El `block $done` tiene el tipo de la PRIMERA arma; si una arma
                // devuelve Int y el resultado es Float (widening permitido),
                // promover para no emitir WASM inválido.
                if res_w == WasTy::F64 && self.value_type(body)? != WasTy::F64 {
                    self.f64_promote(body)?;
                }
            }
            self.exit_var_scope(saved);
            let depth = self.block_depth - done_at;
            self.body.push(Instruction::Br(depth));
            self.body.push(Instruction::End);
            self.block_depth -= 1;
            self.dead_flow = false;
        }

        if let Some(def) = &m.default {
            let body = match def {
                MatchBody::Expr(e) => e,
                MatchBody::Block(_) => return Err(self.match_block_unsupported(&m.span)),
            };
            self.emit_expression(body)?;
            if is_void {
                self.emit_drop(body)?;
            } else if res_w == WasTy::F64 && self.value_type(body)? != WasTy::F64 {
                self.f64_promote(body)?;
            }
            let depth = self.block_depth - done_at;
            self.body.push(Instruction::Br(depth));
        } else {
            // Sin `default` y sin coincidencia: error de runtime.
            self.emit_throw("ningún caso de `match` coincidió", &m.span);
            self.body.push(Instruction::Unreachable);
        }

        self.body.push(Instruction::End); // $done
        self.block_depth -= 1;
        self.dead_flow = false;
        Ok(())
    }

    fn match_block_unsupported(&self, span: &Span) -> crate::error::ClsError {
        crate::error::ClsError::compile_at(
            "el cuerpo de arma `-> { ... }` (arrowReturn) es F4 y aún no está soportado",
            span,
        )
    }

    /// OR de las condiciones de los patrones de un arma. Deja un i32.
    fn emit_match_condition(
        &mut self,
        mode: MatchMode,
        patterns: &[MatchPattern],
        local: Option<u32>,
        was: WasTy,
        is_str: bool,
        cls: &Type,
    ) -> ClsResult<()> {
        let mut emitted = false;
        for p in patterns {
            self.emit_single_match_condition(mode, p, local, was, is_str, cls)?;
            if emitted {
                self.body.push(Instruction::I32Or);
            }
            emitted = true;
        }
        if !emitted {
            self.body.push(Instruction::I32Const(0));
        }
        Ok(())
    }

    fn emit_single_match_condition(
        &mut self,
        _mode: MatchMode,
        p: &MatchPattern,
        local: Option<u32>,
        was: WasTy,
        is_str: bool,
        cls: &Type,
    ) -> ClsResult<()> {
        match p {
            MatchPattern::Literal(lit) => {
                let l = local.unwrap_or(0);
                self.body.push(Instruction::LocalGet(l));
                self.emit_literal(lit)?;
                // Promover literal int a float si el scrutinee es float.
                if was == WasTy::F64 && matches!(lit.kind, LiteralKind::Int(_)) {
                    self.body.push(Instruction::F64ConvertI64S);
                }
                if is_str {
                    self.emit_str_host("__intr_str_eq");
                } else {
                    self.push_eq(was)?;
                }
                Ok(())
            }
            MatchPattern::Ident(name) | MatchPattern::Variant { name, .. } => {
                self.emit_enum_variant_eq(local.unwrap_or(0), cls, name)
            }
            MatchPattern::Cond(e) => {
                self.emit_expression(e)?;
                self.coerce_to_bool(e)
            }
            MatchPattern::Type(ta) => {
                let target = annotation_to_type(ta);
                self.emit_match_type_check(local.unwrap_or(0), cls, &target)
            }
        }
    }

    /// `scrut == variante` sobre el enum del scrutinee. Uniforme para ambas
    /// representaciones (empacado i64 o heap): compara el **índice de variante**.
    fn emit_enum_variant_eq(&mut self, local: u32, cls: &Type, name: &str) -> ClsResult<()> {
        let enum_key = match cls {
            Type::Named(n, _) => self.resolve_enum_key(n),
            _ => None,
        }
        .ok_or_else(|| {
            crate::error::ClsError::CompileError(format!(
                "patrón `{}` de `match`: el scrutinee no es un enum conocido (tipo {})",
                name, cls
            ))
        })?;
        let (_, variants) = self.enum_defs[&enum_key].clone();
        let idx = variants.iter().position(|v| v == name).ok_or_else(|| {
            crate::error::ClsError::CompileError(format!(
                "la variante `{}` no existe en el enum `{}`",
                name, enum_key
            ))
        })?;
        let payload = self.enum_key_has_payload(&enum_key);
        self.emit_enum_variant_index(local, payload);
        self.body.push(Instruction::I64Const(idx as i64));
        self.body.push(Instruction::I64Eq);
        Ok(())
    }

    /// ¿El enum `key` es un enum con payload (objeto de heap)?
    pub(crate) fn enum_key_has_payload(&self, key: &str) -> bool {
        self.enum_params
            .get(key)
            .map(|vs| vs.iter().any(|p| !p.is_empty()))
            .unwrap_or(false)
    }

    /// Deja el **índice de variante** del valor enum (local `i64`) como i64.
    /// `payload` = el valor es un objeto de heap `[def_id][idx][arity][...]`.
    pub(crate) fn emit_enum_variant_index(&mut self, local: u32, payload: bool) {
        self.body.push(Instruction::LocalGet(local));
        if payload {
            self.body.push(Instruction::I32WrapI64);
            self.body.push(Instruction::I64Load(MemArg {
                offset: 8,
                align: 3,
                memory_index: 0,
            }));
        } else {
            self.body.push(Instruction::I64Const(0xffff_ffff));
            self.body.push(Instruction::I64And);
        }
    }

    /// Deja el **def_id** del valor enum (local `i64`) como i64.
    pub(crate) fn emit_enum_def_id(&mut self, local: u32, payload: bool) {
        self.body.push(Instruction::LocalGet(local));
        if payload {
            self.body.push(Instruction::I32WrapI64);
            self.body.push(Instruction::I64Load(MemArg {
                offset: 0,
                align: 3,
                memory_index: 0,
            }));
        } else {
            self.body.push(Instruction::I64Const(32));
            self.body.push(Instruction::I64ShrU);
        }
    }

    /// Deja el `payload[i]` (i64 raw) del enum con payload (ptr en `local`).
    pub(crate) fn emit_enum_payload_slot(&mut self, local: u32, i: usize) {
        self.body.push(Instruction::LocalGet(local));
        self.body.push(Instruction::I32WrapI64);
        self.body.push(Instruction::I64Load(MemArg {
            offset: (24 + 8 * i) as u64,
            align: 3,
            memory_index: 0,
        }));
    }

    /// Si el arma tiene UNA variante con binds (`ok(a, b)`), devuelve
    /// `(enum_key, índice_variante, binds)`. `None` si no hay binds.
    fn arm_payload_binding(
        &self,
        cls: &Type,
        patterns: &[MatchPattern],
    ) -> Option<(String, usize, Vec<String>)> {
        let mut found: Option<(String, usize, Vec<String>)> = None;
        for p in patterns {
            if let MatchPattern::Variant { name, binds } = p {
                if binds.is_empty() {
                    continue;
                }
                let key = match cls {
                    Type::Named(n, _) => self.resolve_enum_key(n)?,
                    _ => return None,
                };
                let idx = self.enum_defs.get(&key)?.1.iter().position(|v| v == name)?;
                found = Some((key, idx, binds.clone()));
            }
        }
        found
    }

    /// Liga los binds del arma a `payload[i]` (por posición), declarando un
    /// local por bind con el ancho de su tipo de param. `_` ignora el slot.
    fn emit_match_payload_binds(
        &mut self,
        local: u32,
        key: &str,
        vidx: usize,
        binds: &[String],
    ) -> ClsResult<()> {
        let params = self
            .enum_params
            .get(key)
            .and_then(|vs| vs.get(vidx))
            .cloned()
            .unwrap_or_default();
        for (i, b) in binds.iter().enumerate() {
            if b == "_" {
                continue;
            }
            let pty = params.get(i).cloned().unwrap_or(Type::Any);
            let w = was_type(&pty).unwrap_or(WasTy::I64);
            self.emit_enum_payload_slot(local, i);
            match w {
                WasTy::F64 => self.body.push(Instruction::F64ReinterpretI64),
                WasTy::I32 => self.body.push(Instruction::I32WrapI64),
                WasTy::I64 => {}
            }
            let idx = self.declare_var_ty(b, w);
            self.body.push(Instruction::LocalSet(idx));
            self.local_cls_types.insert(b.clone(), pty);
        }
        Ok(())
    }

    /// Construcción de una variante de enum (F2): unit-only -> i64 empacado;
    /// con payload -> objeto de heap `[def_id][idx][arity][payload...]`.
    pub(crate) fn emit_enum_construct(
        &mut self,
        key: &str,
        variant: &str,
        args: &[Expression],
    ) -> ClsResult<()> {
        let (def_id, variants) = self.enum_defs[key].clone();
        let vidx = variants.iter().position(|v| v == variant).ok_or_else(|| {
            crate::error::ClsError::CompileError(format!(
                "La variante '{}' no existe en el enum '{}'",
                variant, key
            ))
        })?;
        if !self.enum_key_has_payload(key) {
            // Unit-only: i64 empacado `(def_id<<32)|idx`.
            self.body.push(Instruction::I64Const(
                ((def_id as i64) << 32) | vidx as i64,
            ));
            return Ok(());
        }
        // Objeto de heap: [def_id][idx][arity][slot0..].
        let arity = args.len() as i64;
        self.body.push(Instruction::I64Const(24 + 8 * arity));
        let alloc = self.func_indexes["__alloc"];
        self.body.push(Instruction::Call(alloc));
        let ptr = self.fresh_local();
        self.body.push(Instruction::LocalSet(ptr));
        self.body.push(Instruction::LocalGet(ptr));
        self.body.push(Instruction::I64Const(def_id as i64));
        self.emit_i64_store(0);
        self.body.push(Instruction::LocalGet(ptr));
        self.body.push(Instruction::I64Const(vidx as i64));
        self.emit_i64_store(8);
        self.body.push(Instruction::LocalGet(ptr));
        self.body.push(Instruction::I64Const(arity));
        self.emit_i64_store(16);
        for (i, a) in args.iter().enumerate() {
            let w = self.value_type(a)?;
            self.emit_expression(a)?;
            // Slot i64 raw: f64 -> bits, i32 -> extender, i64 -> as-is.
            match w {
                WasTy::F64 => self.body.push(Instruction::I64ReinterpretF64),
                WasTy::I32 => self.body.push(Instruction::I64ExtendI32U),
                WasTy::I64 => {}
            }
            let val = self.fresh_local();
            self.body.push(Instruction::LocalSet(val));
            self.body.push(Instruction::LocalGet(ptr));
            self.body.push(Instruction::I64Const(24 + 8 * i as i64));
            self.body.push(Instruction::I64Add);
            self.body.push(Instruction::LocalGet(val));
            self.emit_i64_store(0);
        }
        self.body.push(Instruction::LocalGet(ptr));
        Ok(())
    }

    /// `var Enum::variant(a, b) = valor;` (F2, estático): extrae los slots del
    /// payload del valor a locals. El typeck ya validó forma/aridad/tipos.
    pub(crate) fn emit_destructure_let(&mut self, d: &DestructureLet) -> ClsResult<()> {
        let key = self
            .resolve_enum_key(&d.enum_name)
            .ok_or_else(|| {
                crate::error::ClsError::compile_at(
                    &format!("'{}' no es un enum conocido", d.enum_name),
                    &d.span,
                )
            })?;
        let (_, variants) = self.enum_defs[&key].clone();
        let vidx = variants.iter().position(|v| v == &d.variant).ok_or_else(|| {
            crate::error::ClsError::compile_at(
                &format!("La variante '{}' no existe en el enum '{}'", d.variant, d.enum_name),
                &d.span,
            )
        })?;
        // Valor -> local (ptr si el enum tiene payload).
        self.emit_expression(&d.value)?;
        let val = self.fresh_local();
        self.body.push(Instruction::LocalSet(val));
        let params = self
            .enum_params
            .get(&key)
            .and_then(|vs| vs.get(vidx))
            .cloned()
            .unwrap_or_default();
        for (i, b) in d.binds.iter().enumerate() {
            if b == "_" {
                continue;
            }
            let pty = params.get(i).cloned().unwrap_or(Type::Any);
            let w = was_type(&pty).unwrap_or(WasTy::I64);
            self.emit_enum_payload_slot(val, i);
            match w {
                WasTy::F64 => self.body.push(Instruction::F64ReinterpretI64),
                WasTy::I32 => self.body.push(Instruction::I32WrapI64),
                WasTy::I64 => {}
            }
            let idx = self.declare_var_ty(b, w);
            self.body.push(Instruction::LocalSet(idx));
            self.local_cls_types.insert(b.clone(), pty);
        }
        self.dead_flow = false;
        Ok(())
    }

    pub(crate) fn resolve_enum_key(&self, name: &str) -> Option<String> {
        if self.enum_defs.contains_key(name) {
            return Some(name.to_string());
        }
        let suffix = format!("::{}", name);
        self.enum_defs
            .keys()
            .find(|k| k.ends_with(&suffix))
            .cloned()
    }

    /// Chequeo "scrut is TIPO" (modo Tipo). Deja un i32. Para clases/enums/structs
    /// usa identidad runtime (reusa la lógica de `is` en `binary.rs`); para
    /// builtins/arrays, el tipo estático (paridad con `is` de builtins).
    fn emit_match_type_check(&mut self, local: u32, actual: &Type, target: &Type) -> ClsResult<()> {
        match target {
            // `(A, B, ...)`: unión de tipos -> OR de los chequeos.
            Type::Union(members) | Type::Tuple(members) => {
                let mut emitted = false;
                for t in members {
                    self.emit_match_type_check(local, actual, t)?;
                    if emitted {
                        self.body.push(Instruction::I32Or);
                    }
                    emitted = true;
                }
                if !emitted {
                    self.body.push(Instruction::I32Const(0));
                }
                Ok(())
            }
            Type::Named(name, _) => {
                if let Some(key) = self.resolve_enum_key(name) {
                    let (def_id, _) = self.enum_defs[&key].clone();
                    let payload = self.enum_key_has_payload(&key);
                    self.emit_enum_def_id(local, payload);
                    self.body.push(Instruction::I64Const(def_id as i64));
                    self.body.push(Instruction::I64Eq);
                } else if let Some(key) = self.resolve_class_key(name) {
                    self.emit_class_is(local, &key)?;
                } else if let Some(info) = self.struct_defs.get(name) {
                    let def_id = info.def_id;
                    self.body.push(Instruction::LocalGet(local));
                    self.body.push(Instruction::I32WrapI64);
                    self.body.push(Instruction::I64Load(MemArg {
                        offset: 0,
                        align: 3,
                        memory_index: 0,
                    }));
                    self.body.push(Instruction::I64Const(def_id as i64));
                    self.body.push(Instruction::I64Eq);
                } else {
                    // Tipo desconocido: no matchea (compile-time).
                    self.body.push(Instruction::I32Const(0));
                }
                Ok(())
            }
            // Builtins y arrays: chequeo estático del tipo del scrutinee.
            _ => {
                let matched = match_type_static(actual, target);
                self.body
                    .push(Instruction::I32Const(if matched { 1 } else { 0 }));
                Ok(())
            }
        }
    }

    /// `o is Clase` por identidad de `class_id` (+ subclases). Réplica de
    /// `binary.rs` (`Operator::Is`) usando el local del scrutinee.
    fn emit_class_is(&mut self, local: u32, key: &str) -> ClsResult<()> {
        let target_id = self.class_defs[key].class_id;
        let mut ids: Vec<u32> = Vec::new();
        for info in self.class_defs.values() {
            let is_descendant = info
                .ancestors
                .iter()
                .any(|a| self.ancestor_class_id(a) == Some(target_id));
            if info.class_id == target_id || is_descendant {
                if !ids.contains(&info.class_id) {
                    ids.push(info.class_id);
                }
            }
        }
        let obj_tmp = self.fresh_local();
        let cid_tmp = self.fresh_local();
        self.body.push(Instruction::LocalGet(local));
        self.body.push(Instruction::LocalSet(obj_tmp));
        self.body.push(Instruction::LocalGet(obj_tmp));
        self.body.push(Instruction::I32WrapI64);
        self.body.push(Instruction::I64Load(MemArg {
            offset: 8,
            align: 3,
            memory_index: 0,
        }));
        self.body.push(Instruction::LocalSet(cid_tmp));
        let mut first = true;
        for id in &ids {
            self.body.push(Instruction::LocalGet(cid_tmp));
            self.body.push(Instruction::I64Const(*id as i64));
            self.body.push(Instruction::I64Eq);
            if !first {
                self.body.push(Instruction::I32Or);
            }
            first = false;
        }
        if first {
            self.body.push(Instruction::I32Const(0));
        }
        Ok(())
    }
}

/// Chequeo estático "el tipo `actual` matchea el tipo `target`" (modo Tipo,
/// builtins/arrays). Unión de actual -> algún miembro matchea.
fn match_type_static(actual: &Type, target: &Type) -> bool {
    match actual {
        Type::Union(ms) => ms.iter().any(|m| match_type_static(m, target)),
        Type::Literal(v) => match_type_static(&v.base_type(), target),
        _ => match (target, actual) {
            (Type::Int, Type::Int | Type::I8 | Type::I16 | Type::I32 | Type::I64) => true,
            (Type::Float, Type::Float | Type::F32 | Type::F64) => true,
            (Type::String, Type::String) => true,
            (Type::Bool, Type::Bool) => true,
            (Type::Char, Type::Char) => true,
            (Type::Array(t), Type::Array(a)) => match_type_static(a, t),
            _ => actual == target,
        },
    }
}

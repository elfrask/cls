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

/// Discriminante resuelto del modo PATRÓN (F6): enum nativo o clase que define
/// el protocolo `__match() -> E`.
struct PatternDisc {
    /// Clave del enum cuyas variantes se comparan.
    enum_key: String,
    /// Local i64 con el valor del discriminante (el enum, o el resultado de
    /// `obj.__match()` para una clase).
    local: u32,
    /// Clave de la clase si el protocolo es de clase (para `__unpack`).
    class_key: Option<String>,
    /// Retorno anotado de `__unpack` (clase), para el layout del payload.
    unpack_ret: Option<Type>,
}

/// Plan de ligado del payload de un arma (modo PATRÓN).
enum PayloadPlan {
    /// Enum nativo: slots leídos del objeto `[def_id][idx][arity][...]`.
    Enum { key: String, vidx: usize, binds: Vec<String> },
    /// Clase: payload computado por `obj.__unpack(tag)`.
    Class {
        class_key: String,
        tag: String,
        binds: Vec<String>,
        slot_types: Vec<Type>,
    },
}

impl<'a> FuncEmitter<'a> {
    pub(crate) fn emit_match(&mut self, m: &MatchExpr) -> ClsResult<()> {
        if m.arms.is_empty() {
            return Err(crate::error::ClsError::compile_at("`match` vacío", &m.span));
        }
        // Tipo de resultado del `match`: lo fija el typeck en el span del match
        // (a partir de la primera arma, incluido el yield de un arma-bloque).
        let res_cls = self.types.get(&m.span).cloned().unwrap_or(Type::Void);
        let is_void = matches!(res_cls, Type::Void | Type::Empty);
        let res_w = if is_void {
            WasTy::I64
        } else {
            was_type(&res_cls).unwrap_or(WasTy::I64)
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

        // F6: discriminante del modo PATRÓN (enum nativo o clase con protocolo).
        // Para una clase, emite `obj.__match()` una vez y guarda el enum.
        let pattern_disc: Option<PatternDisc> = if m.mode == MatchMode::Pattern {
            self.resolve_pattern_disc(&scrut_cls, scrut_local)?
        } else {
            None
        };

        self.block_depth += 1;
        self.body.push(Instruction::Block(if is_void {
            BlockType::Empty
        } else {
            BlockType::Result(res_w.val_type())
        }));
        let done_at = self.block_depth;
        // F4: este `match` es match-target para `match <-`/`match* <-`/...
        // (`None` = match-sentencia: no admite `<-`).
        self.match_targets
            .push((done_at, if is_void { None } else { Some(res_w) }));

        for arm in &m.arms {
            self.emit_match_condition(
                m.mode,
                &arm.patterns,
                scrut_local,
                scrut_was,
                scrut_is_str,
                &scrut_cls,
                pattern_disc.as_ref(),
            )?;
            self.block_depth += 1;
            self.body.push(Instruction::If(BlockType::Empty));
            self.dead_flow = false;
            // F2/F6: ligar el payload de la variante (modo PATRÓN con `v(binds)`).
            let saved = self.enter_var_scope();
            if let (Some(sl), Some(plan)) = (
                scrut_local,
                pattern_disc
                    .as_ref()
                    .and_then(|d| self.arm_payload_plan(d, &arm.patterns)),
            ) {
                self.emit_match_payload_binds(sl, &plan, &arm.span)?;
            }
            match &arm.body {
                MatchBody::Expr(e) => {
                    self.emit_expression(e)?;
                    if is_void {
                        self.emit_drop(e)?;
                    } else {
                        // El `block $done` tiene el tipo del match; si un arma
                        // devuelve Int y el resultado es Float (widening
                        // permitido), promover para no emitir WASM inválido.
                        if res_w == WasTy::F64 && self.value_type(e)? != WasTy::F64 {
                            self.f64_promote(e)?;
                        }
                    }
                }
                MatchBody::Block(b) => {
                    self.emit_match_arm_block(b, res_w, is_void, &arm.span)?;
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
            match def {
                MatchBody::Expr(e) => {
                    self.emit_expression(e)?;
                    if is_void {
                        self.emit_drop(e)?;
                    } else if res_w == WasTy::F64 && self.value_type(e)? != WasTy::F64 {
                        self.f64_promote(e)?;
                    }
                }
                MatchBody::Block(b) => {
                    self.emit_match_arm_block(b, res_w, is_void, &m.span)?;
                }
            }
            let depth = self.block_depth - done_at;
            self.body.push(Instruction::Br(depth));
        } else {
            // Sin `default` y sin coincidencia: error de runtime.
            self.emit_throw("ningún caso de `match` coincidió", &m.span);
            self.body.push(Instruction::Unreachable);
        }

        self.match_targets.pop();
        self.body.push(Instruction::End); // $done
        self.block_depth -= 1;
        self.dead_flow = false;
        Ok(())
    }

    /// Cuerpo de arma `-> { ... }` (F4). Es un **ternary-target**: `* <- valor`
    /// entrega el resultado del arma (y `match <-` salta al match). El bloque
    /// debe terminar con `<-` (si no, el valor del arma no existe).
    fn emit_match_arm_block(
        &mut self,
        b: &Block,
        res_w: WasTy,
        is_void: bool,
        span: &Span,
    ) -> ClsResult<()> {
        if is_void {
            // Match-sentencia: el arma-bloque es un bloque normal (no target).
            self.emit_scoped_block(b)?;
            return Ok(());
        }
        self.block_depth += 1;
        self.body
            .push(Instruction::Block(BlockType::Result(res_w.val_type())));
        let arm_at = self.block_depth;
        self.ternary_targets.push((arm_at, res_w));
        self.dead_flow = false;
        self.emit_scoped_block(b)?;
        if !self.dead_flow {
            self.ternary_targets.pop();
            return Err(crate::error::ClsError::compile_at(
                "el cuerpo de arma `-> { }` debe devolver su valor con `<-`",
                span,
            ));
        }
        self.ternary_targets.pop();
        self.body.push(Instruction::End); // $arm
        self.block_depth -= 1;
        self.dead_flow = false;
        Ok(())
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
        disc: Option<&PatternDisc>,
    ) -> ClsResult<()> {
        let mut emitted = false;
        for p in patterns {
            self.emit_single_match_condition(mode, p, local, was, is_str, cls, disc)?;
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
        mode: MatchMode,
        p: &MatchPattern,
        local: Option<u32>,
        was: WasTy,
        is_str: bool,
        cls: &Type,
        disc: Option<&PatternDisc>,
    ) -> ClsResult<()> {
        match p {
            MatchPattern::Literal(lit) => {
                // F6: modo VALOR sobre una clase con `__equals` -> obj.__equals(lit).
                if mode == MatchMode::Value {
                    if let Some(cn) = self.class_magic_method(&Some(cls.clone()), "__equals") {
                        let ret = self.magic_ret_was(&cn, "__equals")?;
                        let obj = local.unwrap_or(0);
                        let lit_expr = Expression::Literal(lit.clone());
                        if !self.emit_class_method_call_on("__equals", &cn, obj, &[lit_expr])? {
                            return Err(crate::error::ClsError::CompileError(format!(
                                "`{}::__equals` no está disponible al emitir el modo VALOR de `match`",
                                cn
                            )));
                        }
                        match ret {
                            WasTy::I64 => {
                                self.body.push(Instruction::I64Const(0));
                                self.body.push(Instruction::I64Ne);
                            }
                            WasTy::F64 => {
                                self.body
                                    .push(Instruction::F64Const(Ieee64::new(0.0f64.to_bits())));
                                self.body.push(Instruction::F64Ne);
                            }
                            WasTy::I32 => {}
                        }
                        return Ok(());
                    }
                }
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
                match disc {
                    Some(d) => self.emit_variant_eq_key(d.local, &d.enum_key, name),
                    None => {
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
                        self.emit_variant_eq_key(local.unwrap_or(0), &enum_key, name)
                    }
                }
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

    /// `discriminante == variante` sobre el enum `enum_key`. Uniforme para ambas
    /// representaciones (empacado i64 o heap): compara el **índice de variante**.
    fn emit_variant_eq_key(&mut self, local: u32, enum_key: &str, name: &str) -> ClsResult<()> {
        let (_, variants) = self.enum_defs.get(enum_key).cloned().ok_or_else(|| {
            crate::error::ClsError::CompileError(format!(
                "patrón `{}` de `match`: el enum `{}` no está registrado",
                name, enum_key
            ))
        })?;
        let idx = variants.iter().position(|v| v == name).ok_or_else(|| {
            crate::error::ClsError::CompileError(format!(
                "la variante `{}` no existe en el enum `{}`",
                name, enum_key
            ))
        })?;
        let payload = self.enum_key_has_payload(enum_key);
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

    /// Resuelve el discriminante del modo PATRÓN (F6).
    /// - Enum nativo: el local del scrutinee ya contiene el enum.
    /// - Clase que define `__match() -> E`: emite `obj.__match()` y guarda `E`.
    /// `None` si el scrutinee no es un enum ni una clase con protocolo.
    fn resolve_pattern_disc(
        &mut self,
        cls: &Type,
        local: Option<u32>,
    ) -> ClsResult<Option<PatternDisc>> {
        let Type::Named(n, _) = cls else {
            return Ok(None);
        };
        if let Some(key) = self.resolve_enum_key(n) {
            return Ok(Some(PatternDisc {
                enum_key: key,
                local: local.unwrap_or(0),
                class_key: None,
                unpack_ret: None,
            }));
        }
        if let Some(ck) = self.resolve_class_key(n) {
            if let Some(Type::Named(en, _)) = self.magic_ret_type(&ck, "__match") {
                if let Some(ek) = self.resolve_enum_key(&en) {
                    let obj = local.unwrap_or(0);
                    let emitted = self.emit_class_method_call_on("__match", &ck, obj, &[])?;
                    if !emitted {
                        return Err(crate::error::ClsError::CompileError(format!(
                            "la clase `{}` define `__match` en el typeck pero no en el emisor",
                            ck
                        )));
                    }
                    let dl = self.fresh_local();
                    self.body.push(Instruction::LocalSet(dl));
                    let unpack_ret = self.magic_ret_type(&ck, "__unpack");
                    return Ok(Some(PatternDisc {
                        enum_key: ek,
                        local: dl,
                        class_key: Some(ck),
                        unpack_ret,
                    }));
                }
            }
        }
        Ok(None)
    }

    /// Plan de ligado del payload de un arma: la ÚNICA variante con binds
    /// (enum nativo) o el payload computado de una clase (`__unpack(tag)`).
    fn arm_payload_plan(
        &self,
        disc: &PatternDisc,
        patterns: &[MatchPattern],
    ) -> Option<PayloadPlan> {
        let (name, binds) = patterns.iter().find_map(|p| match p {
            MatchPattern::Variant { name, binds } if binds.iter().any(|b| b != "_") => {
                Some((name.clone(), binds.clone()))
            }
            _ => None,
        })?;
        if let Some(class_key) = &disc.class_key {
            let arity = binds.len();
            // Aridad 1 -> valor único; >1 -> tupla `[cap][len][slots*8]`.
            let slot_types = match class_bind_types(disc.unpack_ret.as_ref(), arity) {
                Some(ts) => ts,
                None => vec![Type::Any; arity],
            };
            return Some(PayloadPlan::Class {
                class_key: class_key.clone(),
                tag: name,
                binds,
                slot_types,
            });
        }
        let vidx = self
            .enum_defs
            .get(&disc.enum_key)?
            .1
            .iter()
            .position(|v| v == &name)?;
        Some(PayloadPlan::Enum {
            key: disc.enum_key.clone(),
            vidx,
            binds,
        })
    }

    /// Liga los binds del arma (por posición). `_` ignora el slot.
    /// - Enum nativo: `payload[i]` del objeto `[def_id][idx][arity][...]`.
    /// - Clase: `obj.__unpack(tag)` -> valor único (1 binding) o tupla (N).
    fn emit_match_payload_binds(
        &mut self,
        obj_local: u32,
        plan: &PayloadPlan,
        span: &Span,
    ) -> ClsResult<()> {
        match plan {
            PayloadPlan::Enum { key, vidx, binds } => {
                let params = self
                    .enum_params
                    .get(key)
                    .and_then(|vs| vs.get(*vidx))
                    .cloned()
                    .unwrap_or_default();
                for (i, b) in binds.iter().enumerate() {
                    if b == "_" {
                        continue;
                    }
                    let pty = params.get(i).cloned().unwrap_or(Type::Any);
                    let w = was_type(&pty).unwrap_or(WasTy::I64);
                    self.emit_enum_payload_slot(obj_local, i);
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
            PayloadPlan::Class {
                class_key,
                tag,
                binds,
                slot_types,
            } => {
                // `obj.__unpack(tag)`: el tag viaja como String interned.
                let tag_expr = Expression::Literal(Literal {
                    kind: LiteralKind::String(tag.clone()),
                    span: span.clone(),
                });
                let emitted =
                    self.emit_class_method_call_on("__unpack", class_key, obj_local, &[tag_expr])?;
                if !emitted {
                    return Err(crate::error::ClsError::compile_at(
                        &format!("la clase `{}` define `__unpack` en el typeck pero no en el emisor", class_key),
                        span,
                    ));
                }
                let up = self.fresh_local();
                self.body.push(Instruction::LocalSet(up));
                let single = binds.len() == 1;
                if !single {
                    // Guard de runtime: el payload debe ser una tupla de aridad
                    // `binds.len()` (header `[cap][len]`).
                    self.body.push(Instruction::LocalGet(up));
                    self.body.push(Instruction::I32WrapI64);
                    self.body.push(Instruction::I64Load(MemArg {
                        offset: 8,
                        align: 3,
                        memory_index: 0,
                    }));
                    self.body.push(Instruction::I64Const(binds.len() as i64));
                    self.body.push(Instruction::I64Eq);
                    self.body.push(Instruction::I32Eqz);
                    self.block_depth += 1;
                    self.body.push(Instruction::If(BlockType::Empty));
                    self.emit_throw(
                        "el payload de `__unpack` no es una tupla de la aridad esperada por el patrón",
                        span,
                    );
                    self.body.push(Instruction::Unreachable);
                    self.body.push(Instruction::End);
                    self.block_depth -= 1;
                }
                for (i, b) in binds.iter().enumerate() {
                    if b == "_" {
                        continue;
                    }
                    let pty = slot_types.get(i).cloned().unwrap_or(Type::Any);
                    let w = was_type(&pty).unwrap_or(WasTy::I64);
                    self.body.push(Instruction::LocalGet(up));
                    if !single {
                        // slot i de la tupla: base 16, stride 8.
                        self.body.push(Instruction::I32WrapI64);
                        self.body.push(Instruction::I64Load(MemArg {
                            offset: (16 + 8 * i as i64) as u64,
                            align: 3,
                            memory_index: 0,
                        }));
                    }
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
        }
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

/// Tipos de los slots del payload de una clase (F6), best-effort contra el
/// retorno anotado de `__unpack` (`T | (T1, T2, …)`). Espejo del helper del
/// typeck: la aridad 1 es valor único; >1, tupla. `None` = no determinable.
fn class_bind_types(unpack_ret: Option<&Type>, arity: usize) -> Option<Vec<Type>> {
    fn from_type(t: &Type, arity: usize) -> Option<Vec<Type>> {
        match t {
            Type::Union(ms) => ms.iter().find_map(|m| from_type(m, arity)),
            Type::Tuple(ts) if ts.len() == arity => Some(ts.clone()),
            Type::Tuple(_) => None,
            Type::Any | Type::Unknown | Type::Value | Type::Json => Some(vec![Type::Any; arity]),
            _ if arity == 1 => Some(vec![t.clone()]),
            _ => None,
        }
    }
    unpack_ret.and_then(|t| from_type(t, arity))
}

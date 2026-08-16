//! Binary/unary: emit_binary, coerce, cmp, throw, assignment (Fase 1: extraido de emitter/mod.rs).

use super::*;

impl<'a> FuncEmitter<'a> {


    pub(crate) fn emit_binary(&mut self, b: &BinaryExpr) -> ClsResult<()> {
        use Operator::*;
        let lt = self.value_type(&b.left)?;
        let rt = self.value_type(&b.right)?;
        // Magic methods de clase (paridad walker `binary_magic`): aritmética,
        // igualdad y comparación se despachan a la clase ANTES de los paths
        // nativos (el typeck ya validó el tipo del resultado).
        let rty = self.types.get(&expr_span(&b.right)).cloned();
        let arith_magic = match b.op {
            Plus => "__add",
            Minus => "__sub",
            Star => "__mul",
            Slash => "__div",
            Percent => "__mod",
            StarStar => "__pow",
            _ => "",
        };
        if !arith_magic.is_empty() {
            if self.try_binary_magic(&b.left, &b.right, arith_magic)?.is_some() {
                return Ok(());
            }
        }
        match b.op {
            StrictEqual | NotEqual => {
                // __equals: left.__equals(right) -> truthiness; `!=` niega.
                if let Some(ret_was) = self.try_binary_magic(&b.left, &b.right, "__equals")? {
                    match ret_was {
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
                    if b.op == NotEqual {
                        self.body.push(Instruction::I32Eqz);
                    }
                    return Ok(());
                }
            }
            LessThan | LessEqual | GreaterThan | GreaterEqual => {
                // __compare: resultado int -> c <0/<=0/>0/>=0 según el operador.
                if let Some(ret_was) = self.try_binary_magic(&b.left, &b.right, "__compare")? {
                    match ret_was {
                        WasTy::I32 => self.body.push(Instruction::I64ExtendI32S),
                        WasTy::F64 => self.body.push(Instruction::I64TruncF64S),
                        WasTy::I64 => {}
                    }
                    let c = self.fresh_local_ty(WasTy::I64);
                    self.body.push(Instruction::LocalSet(c));
                    self.body.push(Instruction::LocalGet(c));
                    self.body.push(Instruction::I64Const(0));
                    let cmp = match b.op {
                        LessThan => Instruction::I64LtS,
                        LessEqual => Instruction::I64LeS,
                        GreaterThan => Instruction::I64GtS,
                        _ => Instruction::I64GeS,
                    };
                    self.body.push(cmp);
                    return Ok(());
                }
            }
            _ => {}
        }
        match b.op {
            Plus if lt == WasTy::I64 && rt == WasTy::I64 => {
                let is_str = |e: &Expression| {
                    self.types
                        .get(&expr_span(e))
                        .map(|t| *t == Type::String)
                        .unwrap_or(false)
                };
                self.emit_expression(&b.left)?;
                self.emit_expression(&b.right)?;
                if is_str(&b.left) || is_str(&b.right) {
                    self.host.call(HostFn::StrConcat, &mut self.body);
                } else {
                    self.body.push(Instruction::I64Add);
                }
            }
            Plus if lt == WasTy::F64 && rt == WasTy::F64 => {
                self.emit_expression(&b.left)?;
                self.emit_expression(&b.right)?;
                self.body.push(Instruction::F64Add);
            }
            Plus if lt == WasTy::I64 && rt == WasTy::F64 => {
                self.emit_expression(&b.left)?;
                self.body.push(Instruction::F64ConvertI64S);
                self.emit_expression(&b.right)?;
                self.body.push(Instruction::F64Add);
            }
            Plus if lt == WasTy::F64 && rt == WasTy::I64 => {
                self.emit_expression(&b.left)?;
                self.emit_expression(&b.right)?;
                self.body.push(Instruction::F64ConvertI64S);
                self.body.push(Instruction::F64Add);
            }
            Plus => {
                self.emit_expression(&b.left)?;
                self.emit_expression(&b.right)?;
                self.host.call(HostFn::StrConcat, &mut self.body);
            }
            Minus if lt == WasTy::F64 || rt == WasTy::F64 => {
                self.emit_expression(&b.left)?;
                self.f64_promote(&b.left)?;
                self.emit_expression(&b.right)?;
                self.f64_promote(&b.right)?;
                self.body.push(Instruction::F64Sub);
            }
            Minus => {
                self.emit_expression(&b.left)?;
                self.emit_expression(&b.right)?;
                self.body.push(Instruction::I64Sub);
            }
            Star if lt == WasTy::F64 || rt == WasTy::F64 => {
                self.emit_expression(&b.left)?;
                self.f64_promote(&b.left)?;
                self.emit_expression(&b.right)?;
                self.f64_promote(&b.right)?;
                self.body.push(Instruction::F64Mul);
            }
            Star => {
                self.emit_expression(&b.left)?;
                self.emit_expression(&b.right)?;
                self.body.push(Instruction::I64Mul);
            }
            Slash if lt == WasTy::F64 || rt == WasTy::F64 => {
                self.emit_expression(&b.left)?;
                self.f64_promote(&b.left)?;
                self.emit_expression(&b.right)?;
                self.f64_promote(&b.right)?;
                self.body.push(Instruction::F64Div);
            }
            Slash => {
                self.emit_expression(&b.left)?;
                self.emit_expression(&b.right)?;
                self.div_zero_trap(&b.span)?;
                self.body.push(Instruction::I64DivS);
            }
            Percent if lt == WasTy::F64 || rt == WasTy::F64 => {
                self.emit_expression(&b.left)?;
                self.f64_promote(&b.left)?;
                self.emit_expression(&b.right)?;
                self.f64_promote(&b.right)?;
                self.host.call(HostFn::Fmod, &mut self.body);
            }
            Percent => {
                self.emit_expression(&b.left)?;
                self.emit_expression(&b.right)?;
                self.div_zero_trap(&b.span)?;
                self.body.push(Instruction::I64RemS);
            }
            StarStar if lt == WasTy::F64 || rt == WasTy::F64 => {
                // Potencia con float: promover ambos a f64 y usar math_pow.
                self.emit_expression(&b.left)?;
                self.f64_promote(&b.left)?;
                self.emit_expression(&b.right)?;
                self.f64_promote(&b.right)?;
                self.host.call(HostFn::MathPow, &mut self.body);
            }
            StarStar => {
                self.emit_expression(&b.left)?;
                self.emit_expression(&b.right)?;
                self.host.call(HostFn::PowNum, &mut self.body);
            }
            // Operadores bit a bit (enteros): ^ << >>
            Caret => {
                self.emit_expression(&b.left)?;
                self.emit_expression(&b.right)?;
                self.body.push(Instruction::I64Xor);
            }
            ShiftLeft => {
                self.emit_expression(&b.left)?;
                self.emit_expression(&b.right)?;
                self.body.push(Instruction::I64Shl);
            }
            ShiftRight => {
                self.emit_expression(&b.left)?;
                self.emit_expression(&b.right)?;
                self.body.push(Instruction::I64ShrS);
            }
            StrictEqual => {
                self.emit_expression(&b.left)?;
                if lt == WasTy::F64 || rt == WasTy::F64 {
                    self.f64_promote(&b.left)?;
                }
                self.emit_expression(&b.right)?;
                if lt == WasTy::F64 || rt == WasTy::F64 {
                    self.f64_promote(&b.right)?;
                }
                self.push_eq(if lt == WasTy::F64 || rt == WasTy::F64 {
                    WasTy::F64
                } else {
                    lt
                })?;
            }
            NotEqual => {
                self.emit_expression(&b.left)?;
                if lt == WasTy::F64 || rt == WasTy::F64 {
                    self.f64_promote(&b.left)?;
                }
                self.emit_expression(&b.right)?;
                if lt == WasTy::F64 || rt == WasTy::F64 {
                    self.f64_promote(&b.right)?;
                }
                self.push_eq(if lt == WasTy::F64 || rt == WasTy::F64 {
                    WasTy::F64
                } else {
                    lt
                })?;
                self.body.push(Instruction::I32Eqz);
            }
            LessThan => {
                self.emit_expression(&b.left)?;
                if lt == WasTy::F64 || rt == WasTy::F64 {
                    self.f64_promote(&b.left)?;
                }
                self.emit_expression(&b.right)?;
                if lt == WasTy::F64 || rt == WasTy::F64 {
                    self.f64_promote(&b.right)?;
                }
                self.push_cmp(
                    if lt == WasTy::F64 || rt == WasTy::F64 {
                        WasTy::F64
                    } else {
                        lt
                    },
                    true,
                    false,
                )?;
            }
            LessEqual => {
                self.emit_expression(&b.left)?;
                if lt == WasTy::F64 || rt == WasTy::F64 {
                    self.f64_promote(&b.left)?;
                }
                self.emit_expression(&b.right)?;
                if lt == WasTy::F64 || rt == WasTy::F64 {
                    self.f64_promote(&b.right)?;
                }
                self.push_cmp(
                    if lt == WasTy::F64 || rt == WasTy::F64 {
                        WasTy::F64
                    } else {
                        lt
                    },
                    true,
                    true,
                )?;
            }
            GreaterThan => {
                self.emit_expression(&b.left)?;
                if lt == WasTy::F64 || rt == WasTy::F64 {
                    self.f64_promote(&b.left)?;
                }
                self.emit_expression(&b.right)?;
                if lt == WasTy::F64 || rt == WasTy::F64 {
                    self.f64_promote(&b.right)?;
                }
                self.push_cmp(
                    if lt == WasTy::F64 || rt == WasTy::F64 {
                        WasTy::F64
                    } else {
                        lt
                    },
                    false,
                    false,
                )?;
            }
            GreaterEqual => {
                self.emit_expression(&b.left)?;
                if lt == WasTy::F64 || rt == WasTy::F64 {
                    self.f64_promote(&b.left)?;
                }
                self.emit_expression(&b.right)?;
                if lt == WasTy::F64 || rt == WasTy::F64 {
                    self.f64_promote(&b.right)?;
                }
                self.push_cmp(
                    if lt == WasTy::F64 || rt == WasTy::F64 {
                        WasTy::F64
                    } else {
                        lt
                    },
                    false,
                    true,
                )?;
            }
            And => {
                self.emit_expression(&b.left)?;
                self.body.push(Instruction::I32Eqz);
                self.block_depth += 1;
                self.body
                    .push(Instruction::If(BlockType::Result(ValType::I32)));
                self.body.push(Instruction::I32Const(0));
                self.body.push(Instruction::Else);
                self.emit_expression(&b.right)?;
                self.body.push(Instruction::End);
                self.block_depth -= 1;
            }
            Or => {
                self.emit_expression(&b.left)?;
                self.block_depth += 1;
                self.body
                    .push(Instruction::If(BlockType::Result(ValType::I32)));
                self.body.push(Instruction::I32Const(1));
                self.body.push(Instruction::Else);
                self.emit_expression(&b.right)?;
                self.body.push(Instruction::End);
                self.block_depth -= 1;
            }
            In => {
                // __contains: container.__contains(needle) si la clase lo define.
                if let Some(cn) = self.class_magic_method(&rty, "__contains") {
                    let _ = self.magic_ret_was(&cn, "__contains")?;
                    self.emit_class_method_args("__contains", &b.right, &[(*b.left).clone()])?;
                    return Ok(());
                }
                // `x in "texto"` -> substring (arrays en A4). StrContains(container, needle)
                self.emit_expression(&b.right)?;
                self.emit_expression(&b.left)?;
                self.host.call(HostFn::StrContains, &mut self.body);
            }
            Is => {
                // `v is Nivel` (enum), `p is Punto` (struct) o `o is Clase` (herencia)
                // `v is String`/`Int`/... (tipo builtin) -> se evalúa estáticamente
                // con el tipo del lado izquierdo.
                if let Expression::Identifier(right_name, _) = &*b.right {
                    if let Some(t) = builtin_was_type(right_name) {
                        // El tipo del left determina el resultado en compile-time.
                        // Comparar por Type (no WasTy: String e Int son ambos i64).
                        let left_span = expr_span(&b.left);
                        let lt = self.types.get(&left_span).cloned().unwrap_or(Type::Any);
                        let matches = builtin_type_matches(&lt, &t);
                        self.emit_expression(&b.left)?;
                        self.body.push(Instruction::Drop);
                        self.body
                            .push(Instruction::I32Const(if matches { 1 } else { 0 }));
                        return Ok(());
                    }
                }
                self.emit_expression(&b.left)?;
                if let Expression::Identifier(right_name, _) = &*b.right {
                    if let Some(info) = self.class_defs.get(right_name.as_str()) {
                        // cid = obj[8]; true si el objeto ES la clase o una SUBCLASE.
                        let obj_tmp = self.fresh_local();
                        let cid_tmp = self.fresh_local();
                        self.body.push(Instruction::LocalSet(obj_tmp));
                        self.body.push(Instruction::LocalGet(obj_tmp));
                        self.body.push(Instruction::I32WrapI64);
                        self.body.push(Instruction::I64Load(MemArg {
                            offset: 8,
                            align: 3,
                            memory_index: 0,
                        }));
                        self.body.push(Instruction::LocalSet(cid_tmp));
                        let mut ids = vec![info.class_id];
                        for (_, other) in self.class_defs.iter() {
                            if other.ancestors.contains(&right_name) {
                                ids.push(other.class_id);
                            }
                        }
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
                        return Ok(());
                    }
                }
                let (def_id, is_enum) = match &*b.right {
                    Expression::Identifier(name, _) => {
                        if let Some((d, _)) = self.enum_defs.get(name) {
                            (*d, true)
                        } else if let Some(info) = self.struct_defs.get(name) {
                            (info.def_id, false)
                        } else {
                            return Err(crate::error::ClsError::CompileError(format!(
                                "'is' con '{}': se esperaba un enum o structure en el JIT",
                                name
                            )));
                        }
                    }
                    // `c is lib::Color` (enum namespaced importado).
                    Expression::NamespaceAccess(ns, name, _) => {
                        let key = format!("{}::{}", ns, name);
                        if let Some((d, _)) = self.enum_defs.get(&key) {
                            (*d, true)
                        } else if let Some(info) = self.struct_defs.get(&key) {
                            (info.def_id, false)
                        } else {
                            return Err(crate::error::ClsError::CompileError(format!(
                                "'is' con '{}::{}': se esperaba un enum o structure en el JIT",
                                ns, name
                            )));
                        }
                    }
                    _ => {
                        return Err(crate::error::ClsError::CompileError(
                            "'is' requiere un nombre a la derecha en el JIT".to_string(),
                        ))
                    }
                };
                if is_enum {
                    self.body.push(Instruction::I64Const(32));
                    self.body.push(Instruction::I64ShrU);
                } else {
                    self.body.push(Instruction::I32WrapI64);
                    self.body.push(Instruction::I64Load(MemArg {
                        offset: 0,
                        align: 3,
                        memory_index: 0,
                    }));
                }
                self.body.push(Instruction::I64Const(def_id as i64));
                self.body.push(Instruction::I64Eq);
            }
            PlusEqual | MinusEqual | StarEqual | SlashEqual | PercentEqual => {
                self.emit_expression(&b.left)?;
                self.emit_expression(&b.right)?;
                match b.op {
                    PlusEqual => self.body.push(Instruction::I64Add),
                    MinusEqual => self.body.push(Instruction::I64Sub),
                    StarEqual => self.body.push(Instruction::I64Mul),
                    SlashEqual => self.body.push(Instruction::I64DivS),
                    _ => self.body.push(Instruction::I64RemS),
                }
            }
            op => {
                return Err(crate::error::ClsError::CompileError(format!(
                    "Operador {} no soportado por el JIT",
                    op
                )))
            }
        }
        Ok(())
    }


    pub(crate) fn f64_promote(&mut self, expr: &Expression) -> ClsResult<()> {        let is_int_literal = matches!(
            expr,
            Expression::Literal(l) if matches!(l.kind, LiteralKind::Int(_))
        );
        let vt = self.value_type(expr)?;
        if is_int_literal || matches!(vt, WasTy::I64) {
            self.body.push(Instruction::F64ConvertI64S);
        }
        Ok(())
    }


    /// Coacciona el valor en el stack (emitido por `emit_expression`) a un
    /// bool i32, con paridad a `Value::is_truthy` del walker. `expr` se usa
    /// solo para consultar el tipo estático (el valor ya está en el stack).
    /// Numéricos: != 0. String: len != 0 (los bits bajos del packed). Array/
    /// Tuple/Record/Shape: len del header (ptr+8) != 0. Char/Bool: ya son i32.
    /// Cmx/Named/objetos: true (paridad walker). Any/Unknown/Null: error claro
    /// (antes emitía WASM inv�lido "expected i32, found i64").
    pub(crate) fn coerce_to_bool(&mut self, expr: &Expression) -> ClsResult<()> {
        let ty = self
            .types
            .get(&expr_span(expr))
            .cloned()
            .unwrap_or(Type::Any);
        match &ty {
            Type::Bool | Type::Char => Ok(()),
            Type::Int | Type::I8 | Type::I16 | Type::I32 | Type::I64 => {
                self.body.push(Instruction::I64Const(0));
                self.body.push(Instruction::I64Ne);
                Ok(())
            }
            Type::Float | Type::F32 | Type::F64 => {
                self.body.push(Instruction::F64Const(Ieee64::new(0.0f64.to_bits())));
                self.body.push(Instruction::F64Ne);
                Ok(())
            }
            Type::String => {
                // packed = (ptr << 32) | len -> truthy si len != 0.
                self.body.push(Instruction::I64Const(0xffff_ffff));
                self.body.push(Instruction::I64And);
                self.body.push(Instruction::I64Const(0));
                self.body.push(Instruction::I64Ne);
                Ok(())
            }
            Type::Array(_) | Type::Tuple(_) | Type::Record(_, _) => {
                // Header CLS: [cap:i64][len:i64] -> truthy si len (ptr+8) != 0.
                self.body.push(Instruction::I64Const(8));
                self.body.push(Instruction::I64Add);
                self.body.push(Instruction::I32WrapI64);
                self.body.push(Instruction::I64Load(MemArg {
                    offset: 0,
                    align: 3,
                    memory_index: 0,
                }));
                self.body.push(Instruction::I64Const(0));
                self.body.push(Instruction::I64Ne);
                Ok(())
            }
            // Shape: se emite como struct contiguo SIN header [cap][len] (los
            // campos van directos) -> no se puede leer el len; un shape con
            // campos declarados siempre es truthy (paridad walker).
            Type::Shape(_) => {
                self.body.push(Instruction::I32Const(1));
                Ok(())
            }
            Type::Cmx | Type::Named(_, _) | Type::Null => {
                // Objetos/valores con identidad: siempre truthy (paridad walker).
                self.body.push(Instruction::I32Const(1));
                Ok(())
            }
            other => Err(crate::error::ClsError::compile_at(
                &format!(
                    "la condición debe ser Bool, encontró {} (usa bool(...) para convertir)",
                    other
                ),
                &expr_span(expr),
            )),
        }
    }


    pub(crate) fn push_eq(&mut self, ty: WasTy) -> ClsResult<()> {
        match ty {
            WasTy::F64 => self.body.push(Instruction::F64Eq),
            WasTy::I32 => self.body.push(Instruction::I32Eq),
            WasTy::I64 => self.body.push(Instruction::I64Eq),
        }
        Ok(())
    }


    pub(crate) fn push_cmp(&mut self, ty: WasTy, less: bool, equal: bool) -> ClsResult<()> {
        match ty {
            WasTy::F64 => {
                let op = match (less, equal) {
                    (true, false) => Instruction::F64Lt,
                    (true, true) => Instruction::F64Le,
                    (false, false) => Instruction::F64Gt,
                    (false, true) => Instruction::F64Ge,
                };
                self.body.push(op);
            }
            WasTy::I64 => {
                let op = match (less, equal) {
                    (true, false) => Instruction::I64LtS,
                    (true, true) => Instruction::I64LeS,
                    (false, false) => Instruction::I64GtS,
                    (false, true) => Instruction::I64GeS,
                };
                self.body.push(op);
            }
            WasTy::I32 => {
                let op = match (less, equal) {
                    (true, false) => Instruction::I32LtS,
                    (true, true) => Instruction::I32LeS,
                    (false, false) => Instruction::I32GtS,
                    (false, true) => Instruction::I32GeS,
                };
                self.body.push(op);
            }
        }
        Ok(())
    }


    pub(crate) fn div_zero_trap(&mut self, span: &Span) -> ClsResult<()> {
        let tmp = self.fresh_local();
        self.body.push(Instruction::LocalSet(tmp));
        self.body.push(Instruction::LocalGet(tmp));
        self.body.push(Instruction::I64Eqz);
        self.block_depth += 1;
        self.body.push(Instruction::If(BlockType::Empty));
        self.emit_throw("División por cero", span);
        self.body.push(Instruction::Unreachable);
        self.body.push(Instruction::End);
        self.block_depth -= 1;
        self.body.push(Instruction::LocalGet(tmp));
        Ok(())
    }


    /// Lanza la excepción CLS: `throw(tag)` con payload (msg, span_empaquetado).
    /// En modo sin excepciones (wasmi): `unreachable` (trap) - el host muestra el
    /// error como trap con el shadow call stack (sin caret del span CLS).
    pub(crate) fn emit_throw(&mut self, msg: &str, span: &Span) {
        if !self.exceptions {
            self.body.push(Instruction::Unreachable);
            return;
        }
        let m = self.intern_string(msg);
        self.emit_load_str(m);
        let packed = ((span.start_line as i64) << 32) | (span.start_col as i64);
        self.body.push(Instruction::I64Const(packed));
        self.body.push(Instruction::Throw(self.tag_idx));
    }


    pub(crate) fn emit_unary(&mut self, u: &UnaryExpr) -> ClsResult<()> {
        match u.op {
            UnaryOp::Negate => {
                // Magic __neg: clase con __neg -> call sin args (paridad walker).
                let oty = self.types.get(&expr_span(&u.operand)).cloned();
                if let Some(cn) = self.class_magic_method(&oty, "__neg") {
                    let _ = self.magic_ret_was(&cn, "__neg")?;
                    self.emit_class_method_args("__neg", &u.operand, &[])?;
                    return Ok(());
                }
                let w = self.value_type(&u.operand)?;
                match w {
                    WasTy::F64 => {
                        self.emit_expression(&u.operand)?;
                        self.body.push(Instruction::F64Neg);
                    }
                    WasTy::I64 => {
                        // 0 - x: push 0 primero, luego el operando, luego sub.
                        self.body.push(Instruction::I64Const(0));
                        self.emit_expression(&u.operand)?;
                        self.body.push(Instruction::I64Sub);
                    }
                    WasTy::I32 => {
                        self.body.push(Instruction::I32Const(0));
                        self.emit_expression(&u.operand)?;
                        self.body.push(Instruction::I32Sub);
                    }
                }
            }
            UnaryOp::Not => {
                // Magic __not: clase con __not -> call sin args; si no, truthiness
                // (paridad walker: `!obj` -> __not() o !is_truthy()).
                let oty = self.types.get(&expr_span(&u.operand)).cloned();
                if let Some(cn) = self.class_magic_method(&oty, "__not") {
                    let _ = self.magic_ret_was(&cn, "__not")?;
                    self.emit_class_method_args("__not", &u.operand, &[])?;
                    return Ok(());
                }
                self.emit_expression(&u.operand)?;
                self.coerce_to_bool(&u.operand)?;
                self.body.push(Instruction::I32Eqz);
            }
            UnaryOp::TypeOf => {
                let span = expr_span(&u.operand);
                let t = self.types.get(&span).cloned().unwrap_or(Type::Any);
                let idx = self.intern_string(type_name_str(&t));
                self.emit_load_str(idx);
            }
            UnaryOp::PostInc | UnaryOp::PreInc | UnaryOp::PostDec | UnaryOp::PreDec => {
                self.emit_incdec(&u.operand, u.op.clone())?
            }
            UnaryOp::BitwiseNot => {
                // ~x -> x ^ -1 (en i64)
                self.emit_expression(&u.operand)?;
                self.body.push(Instruction::I64Const(-1));
                self.body.push(Instruction::I64Xor);
            }
        }
        Ok(())
    }


    /// `x++` / `++x` / `x--` / `--x` sobre un identificador.
    pub(crate) fn emit_incdec(&mut self, operand: &Expression, op: UnaryOp) -> ClsResult<()> {
        if let Expression::Identifier(name, _) = operand {
            let post = matches!(op, UnaryOp::PostInc | UnaryOp::PostDec);
            let inc = matches!(op, UnaryOp::PreInc | UnaryOp::PostInc);
            if post {
                let tmp = self.fresh_local();
                self.emit_ident_load(name);
                self.body.push(Instruction::LocalSet(tmp));
                self.emit_ident_load(name);
                self.body.push(Instruction::I64Const(1));
                if inc {
                    self.body.push(Instruction::I64Add);
                } else {
                    self.body.push(Instruction::I64Sub);
                }
                self.emit_ident_store(name);
                self.body.push(Instruction::LocalGet(tmp));
            } else {
                self.emit_ident_load(name);
                self.body.push(Instruction::I64Const(1));
                if inc {
                    self.body.push(Instruction::I64Add);
                } else {
                    self.body.push(Instruction::I64Sub);
                }
                self.emit_ident_store(name);
                self.emit_ident_load(name);
            }
            Ok(())
        } else {
            Err(crate::error::ClsError::CompileError(
                "++/-- solo se soporta sobre variables (identifier) en el JIT".to_string(),
            ))
        }
    }


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
                        self.host.call(HostFn::StrConcat, &mut self.body);
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
                    self.emit_expression(&a.value)?;
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
                self.emit_expression(&a.value)?;
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
                self.body.push(Instruction::I64Const(arr_kind_code(&cls_t)));
                self.host.call(HostFn::RecordSet, &mut self.body);
                // write-back del ptr (el record pudo crecer y reallocarse)
                if let Expression::Identifier(name, _) = &*i.object {
                    self.emit_ident_store(name);
                } else {
                    self.body.push(Instruction::Drop);
                }
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
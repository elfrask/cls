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
                    self.emit_str_host("__intr_str_concat");
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
                self.emit_str_host("__intr_str_concat");
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
                if let Some(&idx) = self.func_indexes.get("__intr_math_fmod") {
                    self.body.push(Instruction::Call(idx));
                }
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
                if let Some(&idx) = self.func_indexes.get("__intr_math_pow") {
                    self.body.push(Instruction::Call(idx));
                }
            }
            StarStar => {
                self.emit_expression(&b.left)?;
                self.emit_expression(&b.right)?;
                if let Some(&idx) = self.func_indexes.get("__intr_pow_num") {
                    self.body.push(Instruction::Call(idx));
                }
            }
            // Operadores bit a bit (enteros): ^ << >>
            Caret => {
                self.emit_expression(&b.left)?;
                self.emit_expression(&b.right)?;
                self.body.push(Instruction::I64Xor);
            }
            // `&`: bit a bit sobre enteros (i64.and) o conjunción lógica sobre Bool.
            BitAnd if lt == WasTy::I64 || rt == WasTy::I64 => {
                self.emit_expression(&b.left)?;
                self.emit_expression(&b.right)?;
                self.body.push(Instruction::I64And);
            }
            BitAnd => {
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
            // `|`: bit a bit sobre enteros (i64.or) o disyunción lógica sobre Bool.
            BitOr if lt == WasTy::I64 || rt == WasTy::I64 => {
                self.emit_expression(&b.left)?;
                self.emit_expression(&b.right)?;
                self.body.push(Instruction::I64Or);
            }
            BitOr => {
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
                // Cmx (child de texto) comparado con String: formatear el Cmx a
                // string -> un child de texto se comporta como su contenido.
                if self.emit_cmx_string_eq(&b.left, &b.right, false)? {
                    return Ok(());
                }
                // Enum con payload (F2): `==` por **identidad def_id+idx**
                // (documentado en MATCH_PLAN §4-F2). Un enum unit-only conserva
                // la comparación empaquetada (equivalente a la identidad).
                if self.emit_payload_enum_eq(&b.left, &b.right, false)? {
                    return Ok(());
                }
                // Strings: comparar por CONTENIDO (`==` de punteros empaquetados
                // daba false para buffers distintos con el mismo texto).
                if self.is_string_expr(&b.left) || self.is_string_expr(&b.right) {
                    self.emit_expression(&b.left)?;
                    self.emit_expression(&b.right)?;
                    self.emit_str_host("__intr_str_eq");
                    return Ok(());
                }
                // Tipo común de comparación: si un lado es i64 (Int/Any dinámico)
                // y el otro i32 (Bool/Char), extender el i32 a i64 (antes emitía
                // `i64.eq` con un i32 en el stack -> WASM inválido).
                let cmp_ty = if lt == WasTy::F64 || rt == WasTy::F64 {
                    WasTy::F64
                } else if lt == WasTy::I64 || rt == WasTy::I64 {
                    WasTy::I64
                } else {
                    WasTy::I32
                };
                self.emit_expression(&b.left)?;
                if cmp_ty == WasTy::F64 {
                    self.f64_promote(&b.left)?;
                } else if cmp_ty == WasTy::I64 && lt == WasTy::I32 {
                    self.body.push(Instruction::I64ExtendI32S);
                }
                self.emit_expression(&b.right)?;
                if cmp_ty == WasTy::F64 {
                    self.f64_promote(&b.right)?;
                } else if cmp_ty == WasTy::I64 && rt == WasTy::I32 {
                    self.body.push(Instruction::I64ExtendI32S);
                }
                self.push_eq(cmp_ty)?;
            }
            NotEqual => {
                if self.emit_cmx_string_eq(&b.left, &b.right, true)? {
                    return Ok(());
                }
                if self.emit_payload_enum_eq(&b.left, &b.right, true)? {
                    return Ok(());
                }
                if self.is_string_expr(&b.left) || self.is_string_expr(&b.right) {
                    self.emit_expression(&b.left)?;
                    self.emit_expression(&b.right)?;
                    self.emit_str_host("__intr_str_eq");
                    self.body.push(Instruction::I32Eqz);
                    return Ok(());
                }
                let cmp_ty = if lt == WasTy::F64 || rt == WasTy::F64 {
                    WasTy::F64
                } else if lt == WasTy::I64 || rt == WasTy::I64 {
                    WasTy::I64
                } else {
                    WasTy::I32
                };
                self.emit_expression(&b.left)?;
                if cmp_ty == WasTy::F64 {
                    self.f64_promote(&b.left)?;
                } else if cmp_ty == WasTy::I64 && lt == WasTy::I32 {
                    self.body.push(Instruction::I64ExtendI32S);
                }
                self.emit_expression(&b.right)?;
                if cmp_ty == WasTy::F64 {
                    self.f64_promote(&b.right)?;
                } else if cmp_ty == WasTy::I64 && rt == WasTy::I32 {
                    self.body.push(Instruction::I64ExtendI32S);
                }
                self.push_eq(cmp_ty)?;
                self.body.push(Instruction::I32Eqz);
            }
            LessThan => {
                if self.is_string_expr(&b.left) || self.is_string_expr(&b.right) {
                    self.emit_expression(&b.left)?;
                    self.emit_expression(&b.right)?;
                    self.emit_string_cmp();
                    self.body.push(Instruction::I64Const(0));
                    self.body.push(Instruction::I64LtS);
                    return Ok(());
                }
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
                if self.is_string_expr(&b.left) || self.is_string_expr(&b.right) {
                    self.emit_expression(&b.left)?;
                    self.emit_expression(&b.right)?;
                    self.emit_string_cmp();
                    self.body.push(Instruction::I64Const(0));
                    self.body.push(Instruction::I64LeS);
                    return Ok(());
                }
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
                if self.is_string_expr(&b.left) || self.is_string_expr(&b.right) {
                    self.emit_expression(&b.left)?;
                    self.emit_expression(&b.right)?;
                    self.emit_string_cmp();
                    self.body.push(Instruction::I64Const(0));
                    self.body.push(Instruction::I64GtS);
                    return Ok(());
                }
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
                if self.is_string_expr(&b.left) || self.is_string_expr(&b.right) {
                    self.emit_expression(&b.left)?;
                    self.emit_expression(&b.right)?;
                    self.emit_string_cmp();
                    self.body.push(Instruction::I64Const(0));
                    self.body.push(Instruction::I64GeS);
                    return Ok(());
                }
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
                // Array: `x in arr` -> includes(arr, x) (misma semántica que .includes).
                if let Some(Type::Array(elem)) = &rty {
                    let elem_ty = was_type(elem)?;
                    let elem_size = elem_size_bytes(elem_ty);
                    self.emit_expression(&b.right)?;
                    self.emit_expression(&b.left)?;
                    self.elem_to_bits(&b.left, elem_ty)?;
                    self.body.push(Instruction::I64Const(elem_size));
                    if let Some(&idx) = self.func_indexes.get("__intr_arr_includes") {
                        self.body.push(Instruction::Call(idx));
                    }
                    return Ok(());
                }
                // Record/Shape: `key in rec` -> record_has(rec, key).
                if matches!(rty, Some(Type::Record(_, _)) | Some(Type::Shape(_))) {
                    self.emit_expression(&b.right)?;
                    self.emit_expression(&b.left)?;
                    if matches!(
                        self.types.get(&expr_span(&b.left)),
                        Some(
                            Type::Int
                                | Type::I8
                                | Type::I16
                                | Type::I32
                                | Type::I64
                                | Type::Literal(LitVal::Int(_))
                        )
                    ) {
                        self.emit_str_host("__intr_str_int");
                    }
                    if let Some(&idx) = self.func_indexes.get("__intr_record_has") {
                        self.body.push(Instruction::Call(idx));
                    }
                    return Ok(());
                }
                // `x in "texto"` -> substring (arrays en A4). StrContains(container, needle)
                self.emit_expression(&b.right)?;
                self.emit_expression(&b.left)?;
                self.emit_str_host("__intr_str_contains");
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
                    // `v is Callable`: el valor es un handle de función si
                    // v != 0 && v>>32 == 0 (el mismo predicado que print usa
                    // para formatear `<function X>`). Par = función simple,
                    // impar = closure — ambos son handles invocables.
                    if right_name == "Callable" {
                        self.emit_expression(&b.left)?;
                        let v = self.fresh_local();
                        self.body.push(Instruction::LocalSet(v));
                        // v != 0
                        self.body.push(Instruction::LocalGet(v));
                        self.body.push(Instruction::I64Eqz);
                        self.body.push(Instruction::I32Eqz);
                        // && (v>>32 == 0)
                        self.body.push(Instruction::LocalGet(v));
                        self.body.push(Instruction::I64Const(32));
                        self.body.push(Instruction::I64ShrU);
                        self.body.push(Instruction::I64Eqz);
                        self.body.push(Instruction::I32And);
                        return Ok(());
                    }
                }
                self.emit_expression(&b.left)?;
                // `o is Clase` / `o is mod::Clase` (clase importada cualificada).
                // La clase se resuelve por DEFINICIÓN: `mod::Clase` y `Clase`
                // comparten `class_id` (mismo span de la declaración al aplanar),
                // así que el resultado no depende del alias con que se importó.
                if let Some(target_key) = self.resolve_is_class_key(&b.right) {
                    let target_id = self.class_defs[&target_key].class_id;
                    // Ids de la clase objetivo + todas sus subclases (por
                    // ancestros, resueltos a su id de definición).
                    let mut ids: Vec<u32> = Vec::new();
                    for info in self.class_defs.values() {
                        let is_descendant = info.ancestors.iter().any(|a| {
                            self.ancestor_class_id(a) == Some(target_id)
                        });
                        if info.class_id == target_id || is_descendant {
                            if !ids.contains(&info.class_id) {
                                ids.push(info.class_id);
                            }
                        }
                    }
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
                    // F2: un enum con payload guarda el def_id en `[0]` del
                    // objeto de heap; un enum unit-only lo lleva en `>>32`.
                    let payload = match self.types.get(&expr_span(&b.left)) {
                        Some(Type::Named(n, _)) => self
                            .resolve_enum_key(n)
                            .map(|k| self.enum_key_has_payload(&k))
                            .unwrap_or(false),
                        _ => false,
                    };
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
    /// (antes emitía WASM inválido "expected i32, found i64").
    /// ¿La expresión está tipada como String en el type map? (para `==`/`!=`
    /// de strings: comparar contenido, no el puntero empaquetado).
    pub(crate) fn is_string_expr(&self, expr: &Expression) -> bool {
        matches!(
            self.types.get(&expr_span(expr)),
            Some(Type::String)
        )
    }

    /// `strcmp` lexicográfico por bytes (semántica de C `strcmp`/`memcmp`) para
    /// los operadores relacionales de `String` (`>`,`<`,`>=`,`<=`). Consume dos
    /// strings empaquetados `(ptr<<32)|len` del stack y deja un `i64`:
    /// -1 si `a < b`, 0 si `a == b`, 1 si `a > b`. Los bytes se comparan SIN
    /// signo (`i32.load8_u`); si un string es prefijo del otro, el más corto es
    /// menor. Implementado inline en WASM (sin host_calls ni exponer punteros).
    pub(crate) fn emit_string_cmp(&mut self) {
        use Instruction::*;
        let b = self.fresh_local();
        self.body.push(LocalSet(b));
        let a = self.fresh_local();
        self.body.push(LocalSet(a));
        let (a_ptr, a_len) = (self.fresh_local(), self.fresh_local());
        let (b_ptr, b_len) = (self.fresh_local(), self.fresh_local());
        for (src, ptr, len) in [(a, a_ptr, a_len), (b, b_ptr, b_len)] {
            self.body.push(LocalGet(src));
            self.body.push(I64Const(32));
            self.body.push(I64ShrU);
            self.body.push(LocalSet(ptr));
            self.body.push(LocalGet(src));
            self.body.push(I64Const(0xffff_ffff));
            self.body.push(I64And);
            self.body.push(LocalSet(len));
        }
        // n = min(a_len, b_len)
        let n = self.fresh_local();
        self.body.push(LocalGet(a_len));
        self.body.push(LocalGet(b_len));
        self.body.push(I64LtS);
        self.block_depth += 1;
        self.body.push(If(BlockType::Result(ValType::I64)));
        self.body.push(LocalGet(a_len));
        self.body.push(Else);
        self.body.push(LocalGet(b_len));
        self.body.push(End);
        self.block_depth -= 1;
        self.body.push(LocalSet(n));
        // i = 0, res = 0
        let i = self.fresh_local();
        let res = self.fresh_local();
        self.body.push(I64Const(0));
        self.body.push(LocalSet(i));
        self.body.push(I64Const(0));
        self.body.push(LocalSet(res));
        // block $done { loop $loop { ... } }
        self.block_depth += 1;
        self.body.push(Block(BlockType::Empty));
        let done_at = self.block_depth;
        self.block_depth += 1;
        self.body.push(Loop(BlockType::Empty));
        let loop_at = self.block_depth;
        // if i >= n -> break
        self.body.push(LocalGet(i));
        self.body.push(LocalGet(n));
        self.body.push(I64GeS);
        self.body.push(BrIf(self.block_depth - done_at));
        // ba = byte(a + i), bb = byte(b + i)
        let ba = self.fresh_local();
        self.body.push(LocalGet(a_ptr));
        self.body.push(LocalGet(i));
        self.body.push(I64Add);
        self.body.push(I32WrapI64);
        self.body.push(I32Load8U(MemArg { offset: 0, align: 0, memory_index: 0 }));
        self.body.push(I64ExtendI32U);
        self.body.push(LocalSet(ba));
        let bb = self.fresh_local();
        self.body.push(LocalGet(b_ptr));
        self.body.push(LocalGet(i));
        self.body.push(I64Add);
        self.body.push(I32WrapI64);
        self.body.push(I32Load8U(MemArg { offset: 0, align: 0, memory_index: 0 }));
        self.body.push(I64ExtendI32U);
        self.body.push(LocalSet(bb));
        // if ba != bb { res = ba < bb ? -1 : 1; break }
        self.body.push(LocalGet(ba));
        self.body.push(LocalGet(bb));
        self.body.push(I64Ne);
        self.block_depth += 1;
        self.body.push(If(BlockType::Empty));
        self.body.push(LocalGet(ba));
        self.body.push(LocalGet(bb));
        self.body.push(I64LtU);
        self.block_depth += 1;
        self.body.push(If(BlockType::Result(ValType::I64)));
        self.body.push(I64Const(-1));
        self.body.push(Else);
        self.body.push(I64Const(1));
        self.body.push(End);
        self.block_depth -= 1;
        self.body.push(LocalSet(res));
        self.body.push(Br(self.block_depth - done_at));
        self.body.push(End);
        self.block_depth -= 1;
        // i++
        self.body.push(LocalGet(i));
        self.body.push(I64Const(1));
        self.body.push(I64Add);
        self.body.push(LocalSet(i));
        self.body.push(Br(self.block_depth - loop_at));
        self.body.push(End); // loop
        self.block_depth -= 1;
        self.body.push(End); // block $done (solo envuelve el loop)
        self.block_depth -= 1;
        // prefijo común -> decide la longitud. Va FUERA del block: el `br` de
        // salida del loop aterriza acá (si estuviera dentro, el `br` saltaría
        // esta decisión y devolvería 0 para todos los prefijos).
        self.body.push(LocalGet(res));
        self.body.push(I64Eqz);
        self.block_depth += 1;
        self.body.push(If(BlockType::Empty));
        self.body.push(LocalGet(a_len));
        self.body.push(LocalGet(b_len));
        self.body.push(I64LtS);
        self.block_depth += 1;
        self.body.push(If(BlockType::Result(ValType::I64)));
        self.body.push(I64Const(-1));
        self.body.push(Else);
        self.body.push(LocalGet(a_len));
        self.body.push(LocalGet(b_len));
        self.body.push(I64GtS);
        self.block_depth += 1;
        self.body.push(If(BlockType::Result(ValType::I64)));
        self.body.push(I64Const(1));
        self.body.push(Else);
        self.body.push(I64Const(0));
        self.body.push(End);
        self.block_depth -= 1;
        self.body.push(End);
        self.block_depth -= 1;
        self.body.push(LocalSet(res));
        self.body.push(End);
        self.block_depth -= 1;
        self.body.push(LocalGet(res));
    }

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
            Type::Array(_) | Type::Tuple(_) | Type::Record(_, _) | Type::Shape(_) => {
                // Header CLS: [cap:i64][len:i64] -> truthy si len (ptr+8) != 0.
                // (Tras invertir el default, los shapes viven como hashmap y
                // tienen el mismo header que los records.)
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
            Type::Cmx | Type::Named(_, _) | Type::Null => {
                // Objetos/valores con identidad: siempre truthy (paridad walker).
                self.body.push(Instruction::I32Const(1));
                Ok(())
            }
            // Valor dinámico (Any/Value/JSON, leído de record/JSON): despachar
            // por tag en runtime (host_any_to_bool). `if (m.found)` donde `m`
            // es Record<String,Any> o `bool(x)` sobre un valor JSON.
            Type::Any | Type::Unknown | Type::Value | Type::Json => {
                self.body.push(Instruction::Drop);
                self.emit_any_chain(expr)?;
                self.host.call(HostFn::AnyToBool, &mut self.body);
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



    /// `Cmx == String` / `Cmx != String`: formatea el lado Cmx a string (un
    /// child de texto se comporta como su contenido) y compara por contenido.
    /// Devuelve `true` si la comparación se manejó. `negate` = true para `!=`.
    fn emit_cmx_string_eq(
        &mut self,
        left: &Expression,
        right: &Expression,
        negate: bool,
    ) -> ClsResult<bool> {
        let l_cmx = matches!(self.types.get(&expr_span(left)), Some(Type::Cmx));
        let r_cmx = matches!(self.types.get(&expr_span(right)), Some(Type::Cmx));
        let l_str = self.is_string_expr(left);
        let r_str = self.is_string_expr(right);
        if !((l_cmx && r_str) || (r_cmx && l_str)) {
            return Ok(false);
        }
        self.emit_expression(left)?;
        if l_cmx {
            self.host.call(HostFn::CmxToString, &mut self.body);
        }
        self.emit_expression(right)?;
        if r_cmx {
            self.host.call(HostFn::CmxToString, &mut self.body);
        }
        self.emit_str_host("__intr_str_eq");
        if negate {
            self.body.push(Instruction::I32Eqz);
        }
        Ok(true)
    }

    /// `==`/`!=` para enums con payload (F2): compara la identidad de variante
    /// (`def_id` + `idx`), como recomienda `MATCH_PLAN.md` §4-F2. Los enums
    /// unit-only no pasan por aquí (el i64 empacado ya es identidad). Devuelve
    /// `true` si la comparación se manejó.
    fn emit_payload_enum_eq(
        &mut self,
        left: &Expression,
        right: &Expression,
        negate: bool,
    ) -> ClsResult<bool> {
        let lk = match self.types.get(&expr_span(left)) {
            Some(Type::Named(n, _)) => self.resolve_enum_key(n),
            _ => None,
        };
        let rk = match self.types.get(&expr_span(right)) {
            Some(Type::Named(n, _)) => self.resolve_enum_key(n),
            _ => None,
        };
        let (Some(lk), Some(rk)) = (lk, rk) else {
            return Ok(false);
        };
        if lk != rk || !self.enum_key_has_payload(&lk) {
            return Ok(false);
        }
        // def_id (mismo enum) + idx de cada lado.
        self.emit_expression(left)?;
        let l = self.fresh_local();
        self.body.push(Instruction::LocalSet(l));
        self.emit_expression(right)?;
        let r = self.fresh_local();
        self.body.push(Instruction::LocalSet(r));
        self.emit_enum_def_id(l, true);
        self.emit_enum_def_id(r, true);
        self.body.push(Instruction::I64Eq);
        self.emit_enum_variant_index(l, true);
        self.emit_enum_variant_index(r, true);
        self.body.push(Instruction::I64Eq);
        self.body.push(Instruction::I32And);
        if negate {
            self.body.push(Instruction::I32Eqz);
        }
        Ok(true)
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

}

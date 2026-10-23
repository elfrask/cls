//! Uniones de primitivos con tag runtime.
//!
//! `alias M = String | int | Boolean;` no puede representarse como un i64 "a
//! secas": al recibir `1`, `true` o `"x"` no hay forma de saber cuál es. Para
//! permitir `match is`, estas uniones se guardan como un puntero a:
//!
//! ```text
//! [tag: i64][valor: i64]   // 16 bytes
//! ```
//!
//! El tag usa la tabla runtime (`0=int, 1=string, 2=float, 3=bool, 4=char`).
//! Sólo se boxean uniones **de primitivos no literales**; las uniones de
//! clases/enums siguen usando su identidad de runtime y las de literales se
//! discriminan por valor.

use super::*;

pub(crate) const TAGGED_UNION_SIZE: i64 = 16;
pub(crate) const TAGGED_UNION_VALUE_OFFSET: u64 = 8;

impl<'a> FuncEmitter<'a> {
    /// ¿El tipo se representa como caja `[tag][valor]`?
    pub(crate) fn is_tagged_union(ty: &Type) -> bool {
        ty.is_tagged_builtin_union()
    }

    /// ¿`ty` es un type param genérico sin reificar (`T` en `f<T>`)? El motor
    /// compila un único cuerpo con `T` borrado a i64: en las fronteras hay que
    /// ensanchar f64/i32 a i64 al pasar y revertir al volver.
    pub(crate) fn is_generic_type_var(&self, ty: &Type) -> bool {
        match ty {
            Type::Named(n, args) => {
                args.is_empty()
                    && !n.contains("::")
                    && !self.class_defs.contains_key(n)
                    && !self.struct_defs.contains_key(n)
                    && !self.enum_defs.contains_key(n)
            }
            _ => false,
        }
    }

    /// Convierte el resultado de un call cuyo retorno declarado es un type param
    /// (`fn id<T>(x:T)->T`) al tipo concreto que el typeck infirió en el call
    /// site. Sin esto, `id(3.5)` dejaba bits f64 donde el caller esperaba f64
    /// (o i64 donde esperaba f64) y el WASM quedaba inválido.
    pub(crate) fn emit_generic_result_coerce(&mut self, ret: &Type, span: &Span) {
        if !self.is_generic_type_var(ret) {
            return;
        }
        let actual = self
            .types
            .get(span)
            .cloned()
            .unwrap_or(Type::Value)
            .literal_base();
        match actual {
            Type::Float | Type::F32 | Type::F64 => {
                self.body.push(Instruction::F64ReinterpretI64)
            }
            Type::Bool | Type::Char => self.body.push(Instruction::I32WrapI64),
            _ => {}
        }
    }

    /// Emite `expr` como puntero a una caja `[tag, valor]`. Si ya es una unión
    /// boxeada (mismo layout), no re-boxea.
    pub(crate) fn emit_box_tagged_union(&mut self, expr: &Expression) -> ClsResult<()> {
        let actual = self
            .types
            .get(&expr_span(expr))
            .cloned()
            .unwrap_or(Type::Value);
        if Self::is_tagged_union(&actual) {
            return self.emit_expression(expr);
        }

        let was = self.value_type(expr)?;
        self.emit_expression(expr)?;
        match was {
            WasTy::I32 => self.body.push(Instruction::I64ExtendI32U),
            WasTy::F64 => self.body.push(Instruction::I64ReinterpretF64),
            WasTy::I64 => {}
        }
        let val = self.fresh_local_ty(WasTy::I64);
        self.body.push(Instruction::LocalSet(val));

        self.body.push(Instruction::I64Const(TAGGED_UNION_SIZE));
        let alloc = self.func_indexes["__alloc"];
        self.body.push(Instruction::Call(alloc));
        let ptr = self.fresh_local_ty(WasTy::I64);
        self.body.push(Instruction::LocalSet(ptr));

        // tag
        self.body.push(Instruction::LocalGet(ptr));
        self.body.push(Instruction::I32WrapI64);
        let tag = runtime_tag_code(&actual.literal_base());
        self.body.push(Instruction::I64Const(tag));
        self.body.push(Instruction::I64Store(MemArg {
            offset: 0,
            align: 3,
            memory_index: 0,
        }));

        // valor
        self.body.push(Instruction::LocalGet(ptr));
        self.body.push(Instruction::I32WrapI64);
        self.body.push(Instruction::LocalGet(val));
        self.body.push(Instruction::I64Store(MemArg {
            offset: TAGGED_UNION_VALUE_OFFSET,
            align: 3,
            memory_index: 0,
        }));

        self.body.push(Instruction::LocalGet(ptr));
        Ok(())
    }

    /// Carga el tag i64 de una caja.
    pub(crate) fn emit_tagged_union_tag(&mut self, ptr: u32) {
        self.body.push(Instruction::LocalGet(ptr));
        self.body.push(Instruction::I32WrapI64);
        self.body.push(Instruction::I64Load(MemArg {
            offset: 0,
            align: 3,
            memory_index: 0,
        }));
    }

    /// Carga el valor de una caja como i64 crudo.
    pub(crate) fn emit_tagged_union_raw_value(&mut self, ptr: u32) {
        self.body.push(Instruction::LocalGet(ptr));
        self.body.push(Instruction::I32WrapI64);
        self.body.push(Instruction::I64Load(MemArg {
            offset: TAGGED_UNION_VALUE_OFFSET,
            align: 3,
            memory_index: 0,
        }));
    }

    /// Carga el valor de una caja convertido al tipo WASM de `target`
    /// (String/int -> i64, Bool/Char -> i32, Float -> f64).
    pub(crate) fn emit_tagged_union_value(
        &mut self,
        ptr: u32,
        target: &Type,
    ) -> ClsResult<()> {
        self.emit_tagged_union_raw_value(ptr);
        match was_type(&target.literal_base())? {
            WasTy::I32 => self.body.push(Instruction::I32WrapI64),
            WasTy::F64 => self.body.push(Instruction::F64ReinterpretI64),
            WasTy::I64 => {}
        }
        Ok(())
    }

    /// Compara un tag runtime ya cargado en `tag_local` contra el tipo target.
    /// Se usa para `match is` sobre un campo de record cuyo tipo estático es un
    /// type param genérico (`ObjType<T>.value`): el tag real viaja en el record.
    pub(crate) fn emit_tag_local_type_check(
        &mut self,
        tag_local: u32,
        target: &Type,
    ) -> ClsResult<()> {
        match target {
            Type::Union(ms) | Type::Tuple(ms) => {
                let mut first = true;
                for member in ms {
                    self.emit_tag_local_type_check(tag_local, member)?;
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
            _ => {
                let base = target.literal_base();
                if !matches!(
                    base,
                    Type::Int
                        | Type::I8
                        | Type::I16
                        | Type::I32
                        | Type::I64
                        | Type::Float
                        | Type::F32
                        | Type::F64
                        | Type::String
                        | Type::Bool
                        | Type::Char
                ) {
                    self.body.push(Instruction::I32Const(0));
                    return Ok(());
                }
                self.body.push(Instruction::LocalGet(tag_local));
                self.body
                    .push(Instruction::I64Const(runtime_tag_code(&base)));
                self.body.push(Instruction::I64Eq);
                Ok(())
            }
        }
    }

    /// `match is` sobre una unión boxeada: compara el tag runtime.
    pub(crate) fn emit_tagged_union_type_check(
        &mut self,
        ptr: u32,
        target: &Type,
    ) -> ClsResult<()> {
        match target {
            Type::Union(ms) | Type::Tuple(ms) => {
                let mut first = true;
                for member in ms {
                    self.emit_tagged_union_type_check(ptr, member)?;
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
            _ => {
                let base = target.literal_base();
                if !matches!(
                    base,
                    Type::Int
                        | Type::I8
                        | Type::I16
                        | Type::I32
                        | Type::I64
                        | Type::Float
                        | Type::F32
                        | Type::F64
                        | Type::String
                        | Type::Bool
                        | Type::Char
                ) {
                    self.body.push(Instruction::I32Const(0));
                    return Ok(());
                }
                let tag = runtime_tag_code(&base);
                self.emit_tagged_union_tag(ptr);
                self.body.push(Instruction::I64Const(tag));
                self.body.push(Instruction::I64Eq);
                Ok(())
            }
        }
    }

    /// `match (v) { 5 -> ... }` sobre una unión boxeada: primero compara el
    /// tag del literal, y sólo si coincide compara el valor desempaquetado
    /// (comparar un puntero con un literal String sería un falso positivo y
    /// podría desreferenciar basura).
    pub(crate) fn emit_tagged_union_literal_check(
        &mut self,
        ptr: u32,
        literal: &Literal,
    ) -> ClsResult<()> {
        let (base, tag) = match &literal.kind {
            LiteralKind::Int(_) => (Type::Int, runtime_tag_code(&Type::Int)),
            LiteralKind::Float(_) => (Type::Float, runtime_tag_code(&Type::Float)),
            LiteralKind::String(_) => (Type::String, runtime_tag_code(&Type::String)),
            LiteralKind::Bool(_) => (Type::Bool, runtime_tag_code(&Type::Bool)),
            LiteralKind::Char(_) => (Type::Char, runtime_tag_code(&Type::Char)),
            _ => {
                self.body.push(Instruction::I32Const(0));
                return Ok(());
            }
        };

        self.emit_tagged_union_tag(ptr);
        self.body.push(Instruction::I64Const(tag));
        self.body.push(Instruction::I64Eq);
        self.block_depth += 1;
        self.body
            .push(Instruction::If(BlockType::Result(ValType::I32)));
        self.emit_tagged_union_value(ptr, &base)?;
        self.emit_literal(literal)?;
        match base {
            Type::String => self.emit_str_host("__intr_str_eq"),
            Type::Float => self.body.push(Instruction::F64Eq),
            Type::Bool | Type::Char => self.body.push(Instruction::I32Eq),
            _ => self.body.push(Instruction::I64Eq),
        }
        self.body.push(Instruction::Else);
        self.body.push(Instruction::I32Const(0));
        self.body.push(Instruction::End);
        self.block_depth -= 1;
        Ok(())
    }

    /// Si `name` es una variable declarada como unión boxeada y el type map la
    /// estrechó a un primitivo concreto en esta posición, desempaqueta el valor.
    /// Fuera de un brazo de `match` el tipo sigue siendo la unión y se deja el
    /// puntero.
    pub(crate) fn maybe_unbox_tagged_identifier(
        &mut self,
        name: &str,
        span: &Span,
    ) -> ClsResult<()> {
        let declared = self
            .local_cls_types
            .get(name)
            .cloned()
            .or_else(|| {
                self.match_union_bindings
                    .iter()
                    .rev()
                    .find(|(n, _)| n == name)
                    .map(|(_, t)| t.clone())
            });
        let Some(declared) = declared else {
            return Ok(());
        };
        if !Self::is_tagged_union(&declared) {
            return Ok(());
        }
        let current = self.types.get(span).cloned().unwrap_or_else(|| declared.clone());
        if Self::is_tagged_union(&current) {
            return Ok(());
        }
        let base = current.literal_base();
        if was_type(&base).is_err() {
            return Ok(());
        }
        let ptr = self.fresh_local_ty(WasTy::I64);
        self.body.push(Instruction::LocalSet(ptr));
        self.emit_tagged_union_value(ptr, &base)
    }
}

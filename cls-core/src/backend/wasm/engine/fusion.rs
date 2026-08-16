//! Fusión del módulo de internals (`INTERNALS_WASM`) dentro del módulo CLS.
//!
//! Estrategia (plan-performance/adaptacion-core.md §3.1 + HANDOFF-FASE3 Paso 2):
//! las funciones `__intr_*` viven en el MISMO módulo que el código CLS (cero
//! imports de internals), compartiendo la memoria lineal.
//!
//! Layout de la memoria fusionada (layout.rs):
//!   [0 .. INTERNALS_WINDOW_END)  = ventana de internals: el data segment del
//!     sub-crate (42KB en offset 70341) + bss + shadow stack de Rust (crece
//!     hacia abajo desde 1048576) — TODAS sus direcciones internas quedan
//!     INTACTAS (no se re-mapean constantes).
//!   [STRING_DATA_BASE ..]        = string pool del CLS (data segment).
//!   [STRING_TABLE_BASE ..]       = tabla de strings del CLS.
//!   [HEAP_START ..]              = heap bump del CLS (allocator compartido).
//!   [SHADOW_STACK_BASE ..]       = shadow call stack del CLS (trace de errores).
//!
//! Re-mapeo (solo índices WASM, NO direcciones):
//!   - types:   offset = type_count actual del CLS.
//!   - funcs:   offset = func_count actual; el import `__cls_alloc` (idx 0 del
//!     módulo internals) se RESUELVE a la función interna `__alloc` del CLS
//!     (firma (i64)->i64 idéntica) — se elimina del import section.
//!   - globals: offset = global_count actual (el shadow stack de Rust apunta a
//!     1048576, dentro de la ventana de internals: no colisiona con el heap).
//!   - table:   los call_indirect de internals usan la tabla del CLS (0), con
//!     el elem segment de internals re-mapeado al final de esa tabla.
//!   - data:    se copia el segmento de internals (offset 70341) a la misma
//!     dirección (ventana), como segmento adicional de la memoria 0.
//!   - exports: los `__intr_*` se registran en `func_indexes` del engine para
//!     que el emisor los llame por nombre (Paso 3).

use super::*;
use crate::backend::wasm::layout::INTERNALS_WINDOW_END;
use wasmparser::{ExternalKind, Parser, Payload};

/// Resultado de la fusión: índices de función de internals por nombre exportado.
pub(crate) struct FusionResult {
    /// `__intr_<area>_<op>` → índice WASM en el módulo fusionado.
    pub internals: HashMap<String, u32>,
}

impl<'a> Engine<'a> {
    /// Inyecta las secciones de `INTERNALS_WASM` en el módulo CLS. Se llama al
    /// final de `emit`, tras declarar todas las funciones/globals del CLS y
    /// agregar sus bodies al code_sec (los de internals van después).
    pub(crate) fn fuse_internals(&mut self) -> ClsResult<FusionResult> {
        let wasm = cls_internals::INTERNALS_WASM;
        let mut result = FusionResult { internals: HashMap::new() };

        // ── 1. Recorrer las secciones del módulo internals ────────────────────
        // type_count/func_count/global_count actuales = offsets de re-mapeo.
        let type_delta = self.type_count;
        let func_delta = self.func_count;
        let global_delta = self.globals_sec.len();
        let alloc_idx = self
            .func_indexes
            .get("__alloc")
            .copied()
            .ok_or_else(|| crate::error::ClsError::CompileError(
                "fuse_internals: __alloc no declarado".to_string(),
            ))?;

        // Tipos y funciones del módulo internals.
        let mut intr_types: Vec<(Vec<ValType>, Vec<ValType>)> = Vec::new();
        // funcs: type_idx (dentro de internals) por función definida.
        let mut intr_funcs: Vec<u32> = Vec::new();
        // Cuerpos en orden (code section = funcs definidas, tras el import).
        let mut bodies: Vec<Function> = Vec::new();
        // Data segments de internals: (offset_expr, bytes).
        let mut data_segments: Vec<(i32, Vec<u8>)> = Vec::new();
        // Elem segment de internals (tabla funcref para fmt dispatch).
        let mut internals_elem: Vec<u32> = Vec::new();
        // La tabla del módulo internals (para saber si hay call_indirect).
        let mut internals_table_min: Option<u64> = None;
        // Globals de internals: (GlobalType, bytes de init).
        let mut intr_globals: Vec<(GlobalType, ConstExpr)> = Vec::new();
        // Exports de función __intr_* → índice de función dentro de internals.
        let mut intr_exports: Vec<(String, u32)> = Vec::new();

        for payload in Parser::new(0).parse_all(wasm) {
            match payload.map_err(|e| crate::error::ClsError::CompileError(e.to_string()))? {
                Payload::TypeSection(reader) => {
                    for t in reader {
                        let t = t.map_err(|e| crate::error::ClsError::CompileError(e.to_string()))?;
                        for sub in t.types() {
                            if let wasmparser::CompositeInnerType::Func(ft) = &sub.composite_type.inner
                            {
                                let params = ft
                                    .params()
                                    .iter()
                                    .map(|v| parse_val_type(*v))
                                    .collect::<ClsResult<Vec<ValType>>>()?;
                                let results = ft
                                    .results()
                                    .iter()
                                    .map(|v| parse_val_type(*v))
                                    .collect::<ClsResult<Vec<ValType>>>()?;
                                intr_types.push((params, results));
                            }
                        }
                    }
                }
                Payload::ImportSection(reader) => {
                    // Único import esperado: env.__cls_alloc (se resuelve a __alloc).
                    for _imp in reader {
                        // No se agrega al import section: se re-mapea en los calls.
                    }
                }
                Payload::FunctionSection(reader) => {
                    for f in reader {
                        let f = f.map_err(|e| crate::error::ClsError::CompileError(e.to_string()))?;
                        intr_funcs.push(f);
                    }
                }
                Payload::GlobalSection(reader) => {
                    for g in reader {
                        let g = g.map_err(|e| crate::error::ClsError::CompileError(e.to_string()))?;
                        let gt = GlobalType {
                            val_type: val_type_of(&g.ty),
                            mutable: g.ty.mutable,
                            shared: false,
                        };
                        let init = const_expr_from_ops(&g.init_expr)?;
                        intr_globals.push((gt, init));
                    }
                }
                Payload::TableSection(reader) => {
                    for t in reader {
                        let t = t.map_err(|e| crate::error::ClsError::CompileError(e.to_string()))?;
                        internals_table_min = Some(t.ty.initial);
                    }
                }
                Payload::ElementSection(reader) => {
                    for e in reader {
                        let e = e.map_err(|err| crate::error::ClsError::CompileError(err.to_string()))?;
                        if let wasmparser::ElementKind::Active { .. } = e.kind {
                            if let wasmparser::ElementItems::Functions(fs) = e.items {
                                for f in fs {
                                    let f = f.map_err(|x| crate::error::ClsError::CompileError(x.to_string()))?;
                                    internals_elem.push(f);
                                }
                            }
                        }
                    }
                }
                Payload::CodeSectionEntry(body) => {
                    let body = rewrite_body(body, func_delta, type_delta, alloc_idx, global_delta)?;
                    bodies.push(body);
                }
                Payload::DataSection(reader) => {
                    for d in reader {
                        let d = d.map_err(|e| crate::error::ClsError::CompileError(e.to_string()))?;
                        if let wasmparser::DataKind::Active { offset_expr, .. } = &d.kind {
                            let off = i32_const_value(offset_expr).unwrap_or(0);
                            data_segments.push((off, d.data.to_vec()));
                        }
                    }
                }
                Payload::ExportSection(reader) => {
                    for e in reader {
                        let e = e.map_err(|x| crate::error::ClsError::CompileError(x.to_string()))?;
                        if e.kind == ExternalKind::Func {
                            intr_exports.push((e.name.to_string(), e.index));
                        }
                    }
                }
                _ => {}
            }
        }

        // ── 2. Registro de tipos de internals (offset = type_delta) ──────────
        let mut type_map: Vec<u32> = Vec::with_capacity(intr_types.len());
        for (params, results) in &intr_types {
            let tidx = self.register_func_type(params.clone(), results.clone());
            type_map.push(tidx);
        }

        // ── 3. Registro de funciones de internals (offset = func_delta) ───────
        // El índice 0 del módulo internals es el import __cls_alloc (NO se
        // declara): los funcs definidos empiezan en 1 y se mapean a
        // `func_delta + i` (i = posición entre los definidos). Los bodies se
        // agregaron en orden en `bodies` (misma posición que intr_funcs).
        let mut func_map: Vec<u32> = Vec::with_capacity(intr_funcs.len());
        for (i, &type_idx) in intr_funcs.iter().enumerate() {
            let tidx = type_map[type_idx as usize];
            let fidx = self.func_count;
            self.func_count += 1;
            self.funcs_sec.function(tidx);
            func_map.push(fidx);
            // Los bodies de internals van al code_sec tras los del CLS.
            if i < bodies.len() {
                self.code_sec.function(&bodies[i]);
            }
        }

        // ── 4. Globals de internals (offset = global_delta) ──────────────────
        for (gt, init) in &intr_globals {
            self.globals_sec.global(gt.clone(), init);
        }

        // ── 5. Tabla de internals (índice 1): el CLS ya declaró su tabla 0 ────
        // El CLS usa la tabla 0 (vtables + handles); las internals necesitan la
        // suya para el call_indirect de fmt dispatch. Se agrega como tabla 1 y
        // los call_indirect de internals se re-mapean a `table_index: 1`.
        if !internals_elem.is_empty() {
            // Mapear los índices de función del elem (dentro de internals) al
            // módulo fusionado: 0 = __cls_alloc (NO se exporta a la tabla),
            // 1.. = funcs definidos.
            let mapped: Vec<u32> = internals_elem
                .iter()
                .filter_map(|f| {
                    if *f == 0 {
                        None // __cls_alloc no va a la tabla (es un import resuelto)
                    } else {
                        func_map.get((*f - 1) as usize).copied()
                    }
                })
                .collect();
            if !mapped.is_empty() {
                // Tabla 1 (la 0 es del CLS). El elem va en offset 0 de la 1.
                self.tables_sec.table(TableType {
                    element_type: RefType::FUNCREF,
                    table64: false,
                    minimum: mapped.len() as u64,
                    maximum: None,
                    shared: false,
                });
                self.elements_sec.active(
                    Some(1),
                    &ConstExpr::i32_const(0),
                    Elements::Functions(std::borrow::Cow::Owned(mapped)),
                );
            }
        }

        // ── 6. Data segments de internals a la ventana [0..INTERNALS_WINDOW_END)
        for (off, bytes) in data_segments {
            self.data_sec.segment(DataSegment {
                mode: DataSegmentMode::Active {
                    memory_index: 0,
                    offset: &ConstExpr::i32_const(off),
                },
                data: bytes,
            });
        }

        // ── 7. Exports __intr_* → func_indexes (el emisor los llama por nombre)
        for (name, idx) in intr_exports {
            if let Some(fidx) = func_map.get((idx - 1) as usize) {
                result.internals.insert(name.clone(), *fidx);
                self.func_indexes.insert(name, *fidx);
            }
        }

        // ── 8. Asegurar que la memoria cubre la ventana de internals ──────────
        // La memoria del CLS arranca en 32 páginas (2MB) — cubre la ventana.

        let _ = (internals_table_min, INTERNALS_WINDOW_END);
        Ok(result)
    }
}

// ── Helpers de re-mapeo ─────────────────────────────────────────────────────

fn parse_val_type(v: wasmparser::ValType) -> ClsResult<ValType> {
    match v {
        wasmparser::ValType::I32 => Ok(ValType::I32),
        wasmparser::ValType::I64 => Ok(ValType::I64),
        wasmparser::ValType::F32 => Ok(ValType::F32),
        wasmparser::ValType::F64 => Ok(ValType::F64),
        wasmparser::ValType::V128 => Ok(ValType::V128),
        wasmparser::ValType::Ref(_) => Ok(ValType::Ref(RefType::FUNCREF)),
    }
}

fn val_type_of(t: &wasmparser::GlobalType) -> ValType {
    match t.content_type {
        wasmparser::ValType::I32 => ValType::I32,
        wasmparser::ValType::I64 => ValType::I64,
        wasmparser::ValType::F32 => ValType::F32,
        wasmparser::ValType::F64 => ValType::F64,
        _ => ValType::I64,
    }
}

/// Lee una ConstExpr que es `i32.const <v>`.
fn i32_const_value(expr: &wasmparser::ConstExpr) -> Option<i32> {
    let mut reader = expr.get_operators_reader();
    if let Ok(wasmparser::Operator::I32Const { value }) = reader.read() {
        return Some(value);
    }
    None
}

/// Re-emite la ConstExpr de un global (i32/i64/f32/f64 const).
fn const_expr_from_ops(expr: &wasmparser::ConstExpr) -> ClsResult<ConstExpr> {
    let mut reader = expr.get_operators_reader();
    let op = reader
        .read()
        .map_err(|e| crate::error::ClsError::CompileError(e.to_string()))?;
    Ok(match op {
        wasmparser::Operator::I32Const { value } => ConstExpr::i32_const(value),
        wasmparser::Operator::I64Const { value } => ConstExpr::i64_const(value),
        wasmparser::Operator::F32Const { value } => {
            ConstExpr::f32_const(wasm_encoder::Ieee32::new(value.bits()))
        }
        wasmparser::Operator::F64Const { value } => {
            ConstExpr::f64_const(wasm_encoder::Ieee64::new(value.bits()))
        }
        wasmparser::Operator::GlobalGet { .. } => {
            return Err(crate::error::ClsError::CompileError(
                "global de internals con init GlobalGet no soportado".to_string(),
            ));
        }
        _ => {
            return Err(crate::error::ClsError::CompileError(
                "const expr de internals no soportado".to_string(),
            ))
        }
    })
}

/// Re-escribe un body de internals: re-mapea `call`/`call_indirect`/`global`
/// indices (type/table/global) al espacio del módulo fusionado. Las funciones
/// definidas se desplazan `+func_delta`; el import `__cls_alloc` (idx 0) →
/// `alloc_idx` del CLS. Los `i32.const` con direcciones NO se tocan (ventana).
fn rewrite_body(
    body: wasmparser::FunctionBody,
    func_delta: u32,
    type_delta: u32,
    alloc_idx: u32,
    global_delta: u32,
) -> ClsResult<Function> {
    use wasm_encoder::Instruction as I;

    let mut locals: Vec<ValType> = Vec::new();
    for group in body.get_locals_reader().map_err(|e| crate::error::ClsError::CompileError(e.to_string()))? {
        let (count, ty) = group.map_err(|e| crate::error::ClsError::CompileError(e.to_string()))?;
        let vt = parse_val_type(ty)?;
        for _ in 0..count {
            locals.push(vt);
        }
    }
    // Agrupar locals por tipo (optimización menor; puede haber runs).
    let grouped: Vec<(u32, ValType)> = {
        let mut g: Vec<(u32, ValType)> = Vec::new();
        for t in &locals {
            match g.last_mut() {
                Some((n, lt)) if lt == t => *n += 1,
                _ => g.push((1, *t)),
            }
        }
        g
    };

    let mut func = Function::new(grouped);
    let ops = body
        .get_operators_reader()
        .map_err(|e| crate::error::ClsError::CompileError(e.to_string()))?;
    for op in ops {
        let op = op.map_err(|e| crate::error::ClsError::CompileError(e.to_string()))?;
        let inst = map_operator(op, func_delta, type_delta, alloc_idx, global_delta)?;
        func.instruction(&inst);
    }
    Ok(func)
}

/// Traduce un operador de internals a una instrucción del módulo fusionado.
fn map_operator(
    op: wasmparser::Operator,
    func_delta: u32,
    type_delta: u32,
    alloc_idx: u32,
    global_delta: u32,
) -> ClsResult<wasm_encoder::Instruction> {
    use wasm_encoder::Instruction as I;
    use wasm_encoder::MemArg;
    Ok(match op {
        // ── calls ────────────────────────────────────────────────────────
        wasmparser::Operator::Call { function_index } => {
            let target = if function_index == 0 {
                alloc_idx // import __cls_alloc → __alloc del CLS
            } else {
                func_delta + (function_index - 1)
            };
            I::Call(target)
        }
        wasmparser::Operator::CallIndirect { type_index, table_index: _ } => {
            // La tabla de internals es la 1 del módulo fusionado (la 0 es del CLS).
            I::CallIndirect { type_index: type_delta + type_index, table_index: 1 }
        }
        // ── blocks (pueden llevar un type index del módulo internals) ────
        wasmparser::Operator::Block { blockty } => I::Block(map_blockty(blockty, type_delta)?),
        wasmparser::Operator::Loop { blockty } => I::Loop(map_blockty(blockty, type_delta)?),
        wasmparser::Operator::If { blockty } => I::If(map_blockty(blockty, type_delta)?),
        // ── globals (offset = global_delta: los de internals van después) ─
        wasmparser::Operator::GlobalGet { global_index } => I::GlobalGet(global_delta + global_index),
        wasmparser::Operator::GlobalSet { global_index } => I::GlobalSet(global_delta + global_index),
        // ── memory (la 0 del CLS) ────────────────────────────────────────
        wasmparser::Operator::I32Load { memarg } => I::I32Load(ma(memarg)),
        wasmparser::Operator::I64Load { memarg } => I::I64Load(ma(memarg)),
        wasmparser::Operator::F32Load { memarg } => I::F32Load(ma(memarg)),
        wasmparser::Operator::F64Load { memarg } => I::F64Load(ma(memarg)),
        wasmparser::Operator::I32Load8S { memarg } => I::I32Load8S(ma(memarg)),
        wasmparser::Operator::I32Load8U { memarg } => I::I32Load8U(ma(memarg)),
        wasmparser::Operator::I32Load16S { memarg } => I::I32Load16S(ma(memarg)),
        wasmparser::Operator::I32Load16U { memarg } => I::I32Load16U(ma(memarg)),
        wasmparser::Operator::I64Load8S { memarg } => I::I64Load8S(ma(memarg)),
        wasmparser::Operator::I64Load8U { memarg } => I::I64Load8U(ma(memarg)),
        wasmparser::Operator::I64Load16S { memarg } => I::I64Load16S(ma(memarg)),
        wasmparser::Operator::I64Load16U { memarg } => I::I64Load16U(ma(memarg)),
        wasmparser::Operator::I64Load32S { memarg } => I::I64Load32S(ma(memarg)),
        wasmparser::Operator::I64Load32U { memarg } => I::I64Load32U(ma(memarg)),
        wasmparser::Operator::I32Store { memarg } => I::I32Store(ma(memarg)),
        wasmparser::Operator::I64Store { memarg } => I::I64Store(ma(memarg)),
        wasmparser::Operator::F32Store { memarg } => I::F32Store(ma(memarg)),
        wasmparser::Operator::F64Store { memarg } => I::F64Store(ma(memarg)),
        wasmparser::Operator::I32Store8 { memarg } => I::I32Store8(ma(memarg)),
        wasmparser::Operator::I32Store16 { memarg } => I::I32Store16(ma(memarg)),
        wasmparser::Operator::I64Store8 { memarg } => I::I64Store8(ma(memarg)),
        wasmparser::Operator::I64Store16 { memarg } => I::I64Store16(ma(memarg)),
        wasmparser::Operator::I64Store32 { memarg } => I::I64Store32(ma(memarg)),
        wasmparser::Operator::MemorySize { .. } => I::MemorySize(0),
        wasmparser::Operator::MemoryGrow { .. } => I::MemoryGrow(0),
        wasmparser::Operator::MemoryCopy { .. } => I::MemoryCopy { dst_mem: 0, src_mem: 0 },
        wasmparser::Operator::MemoryFill { .. } => I::MemoryFill(0),
        // ── locales ──────────────────────────────────────────────────────
        wasmparser::Operator::LocalGet { local_index } => I::LocalGet(local_index),
        wasmparser::Operator::LocalSet { local_index } => I::LocalSet(local_index),
        wasmparser::Operator::LocalTee { local_index } => I::LocalTee(local_index),
        // ── const ────────────────────────────────────────────────────────
        wasmparser::Operator::I32Const { value } => I::I32Const(value),
        wasmparser::Operator::I64Const { value } => I::I64Const(value),
        wasmparser::Operator::F32Const { value } => {
            I::F32Const(wasm_encoder::Ieee32::new(value.bits()))
        }
        wasmparser::Operator::F64Const { value } => {
            I::F64Const(wasm_encoder::Ieee64::new(value.bits()))
        }
        // ── control ──────────────────────────────────────────────────────
        wasmparser::Operator::Unreachable => I::Unreachable,
        wasmparser::Operator::Nop => I::Nop,
        wasmparser::Operator::Return => I::Return,
        wasmparser::Operator::End => I::End,
        wasmparser::Operator::Br { relative_depth } => I::Br(relative_depth),
        wasmparser::Operator::BrIf { relative_depth } => I::BrIf(relative_depth),
        wasmparser::Operator::BrTable { targets } => {
            let ts: Vec<u32> = targets
                .targets()
                .collect::<std::result::Result<Vec<u32>, _>>()
                .map_err(|e| crate::error::ClsError::CompileError(e.to_string()))?;
            let default = targets.default();
            I::BrTable(std::borrow::Cow::Owned(ts), default)
        }
        wasmparser::Operator::Drop => I::Drop,
        wasmparser::Operator::Select => I::Select,
        wasmparser::Operator::TypedSelect { ty } => I::TypedSelect(parse_val_type(ty)?),
        // ── i32 ──────────────────────────────────────────────────────────
        wasmparser::Operator::I32Eqz => I::I32Eqz,
        wasmparser::Operator::I32Eq => I::I32Eq,
        wasmparser::Operator::I32Ne => I::I32Ne,
        wasmparser::Operator::I32LtS => I::I32LtS,
        wasmparser::Operator::I32LtU => I::I32LtU,
        wasmparser::Operator::I32GtS => I::I32GtS,
        wasmparser::Operator::I32GtU => I::I32GtU,
        wasmparser::Operator::I32LeS => I::I32LeS,
        wasmparser::Operator::I32LeU => I::I32LeU,
        wasmparser::Operator::I32GeS => I::I32GeS,
        wasmparser::Operator::I32GeU => I::I32GeU,
        wasmparser::Operator::I32Add => I::I32Add,
        wasmparser::Operator::I32Sub => I::I32Sub,
        wasmparser::Operator::I32Mul => I::I32Mul,
        wasmparser::Operator::I32DivS => I::I32DivS,
        wasmparser::Operator::I32DivU => I::I32DivU,
        wasmparser::Operator::I32RemS => I::I32RemS,
        wasmparser::Operator::I32RemU => I::I32RemU,
        wasmparser::Operator::I32And => I::I32And,
        wasmparser::Operator::I32Or => I::I32Or,
        wasmparser::Operator::I32Xor => I::I32Xor,
        wasmparser::Operator::I32Shl => I::I32Shl,
        wasmparser::Operator::I32ShrS => I::I32ShrS,
        wasmparser::Operator::I32ShrU => I::I32ShrU,
        wasmparser::Operator::I32Rotl => I::I32Rotl,
        wasmparser::Operator::I32Rotr => I::I32Rotr,
        wasmparser::Operator::I32Clz => I::I32Clz,
        wasmparser::Operator::I32Ctz => I::I32Ctz,
        wasmparser::Operator::I32Popcnt => I::I32Popcnt,
        wasmparser::Operator::I32WrapI64 => I::I32WrapI64,
        wasmparser::Operator::I32Extend8S => I::I32Extend8S,
        wasmparser::Operator::I32Extend16S => I::I32Extend16S,
        // ── i64 ──────────────────────────────────────────────────────────
        wasmparser::Operator::I64Eqz => I::I64Eqz,
        wasmparser::Operator::I64Eq => I::I64Eq,
        wasmparser::Operator::I64Ne => I::I64Ne,
        wasmparser::Operator::I64LtS => I::I64LtS,
        wasmparser::Operator::I64LtU => I::I64LtU,
        wasmparser::Operator::I64GtS => I::I64GtS,
        wasmparser::Operator::I64GtU => I::I64GtU,
        wasmparser::Operator::I64LeS => I::I64LeS,
        wasmparser::Operator::I64LeU => I::I64LeU,
        wasmparser::Operator::I64GeS => I::I64GeS,
        wasmparser::Operator::I64GeU => I::I64GeU,
        wasmparser::Operator::I64Add => I::I64Add,
        wasmparser::Operator::I64Sub => I::I64Sub,
        wasmparser::Operator::I64Mul => I::I64Mul,
        wasmparser::Operator::I64DivS => I::I64DivS,
        wasmparser::Operator::I64DivU => I::I64DivU,
        wasmparser::Operator::I64RemS => I::I64RemS,
        wasmparser::Operator::I64RemU => I::I64RemU,
        wasmparser::Operator::I64And => I::I64And,
        wasmparser::Operator::I64Or => I::I64Or,
        wasmparser::Operator::I64Xor => I::I64Xor,
        wasmparser::Operator::I64Shl => I::I64Shl,
        wasmparser::Operator::I64ShrS => I::I64ShrS,
        wasmparser::Operator::I64ShrU => I::I64ShrU,
        wasmparser::Operator::I64Rotl => I::I64Rotl,
        wasmparser::Operator::I64Rotr => I::I64Rotr,
        wasmparser::Operator::I64Clz => I::I64Clz,
        wasmparser::Operator::I64Ctz => I::I64Ctz,
        wasmparser::Operator::I64Popcnt => I::I64Popcnt,
        wasmparser::Operator::I64ExtendI32S => I::I64ExtendI32S,
        wasmparser::Operator::I64ExtendI32U => I::I64ExtendI32U,
        wasmparser::Operator::I64Extend8S => I::I64Extend8S,
        wasmparser::Operator::I64Extend16S => I::I64Extend16S,
        wasmparser::Operator::I64Extend32S => I::I64Extend32S,
        wasmparser::Operator::I64TruncF32S => I::I64TruncF32S,
        wasmparser::Operator::I64TruncF32U => I::I64TruncF32U,
        wasmparser::Operator::I64TruncF64S => I::I64TruncF64S,
        wasmparser::Operator::I64TruncF64U => I::I64TruncF64U,
        wasmparser::Operator::I32TruncSatF64S => I::I32TruncSatF64S,
        wasmparser::Operator::I32TruncSatF64U => I::I32TruncSatF64U,
        wasmparser::Operator::I32TruncF64S => I::I32TruncF64S,
        wasmparser::Operator::I32TruncF64U => I::I32TruncF64U,
        wasmparser::Operator::I32TruncF32S => I::I32TruncF32S,
        wasmparser::Operator::I32TruncF32U => I::I32TruncF32U,
        // ── f32/f64 ──────────────────────────────────────────────────────
        wasmparser::Operator::F32Eq => I::F32Eq,
        wasmparser::Operator::F32Ne => I::F32Ne,
        wasmparser::Operator::F32Lt => I::F32Lt,
        wasmparser::Operator::F32Gt => I::F32Gt,
        wasmparser::Operator::F32Le => I::F32Le,
        wasmparser::Operator::F32Ge => I::F32Ge,
        wasmparser::Operator::F32Add => I::F32Add,
        wasmparser::Operator::F32Sub => I::F32Sub,
        wasmparser::Operator::F32Mul => I::F32Mul,
        wasmparser::Operator::F32Div => I::F32Div,
        wasmparser::Operator::F32Min => I::F32Min,
        wasmparser::Operator::F32Max => I::F32Max,
        wasmparser::Operator::F32Abs => I::F32Abs,
        wasmparser::Operator::F32Neg => I::F32Neg,
        wasmparser::Operator::F32Sqrt => I::F32Sqrt,
        wasmparser::Operator::F32Ceil => I::F32Ceil,
        wasmparser::Operator::F32Floor => I::F32Floor,
        wasmparser::Operator::F32Trunc => I::F32Trunc,
        wasmparser::Operator::F32Nearest => I::F32Nearest,
        wasmparser::Operator::F32ConvertI32S => I::F32ConvertI32S,
        wasmparser::Operator::F32ConvertI32U => I::F32ConvertI32U,
        wasmparser::Operator::F32ConvertI64S => I::F32ConvertI64S,
        wasmparser::Operator::F32ConvertI64U => I::F32ConvertI64U,
        wasmparser::Operator::F32DemoteF64 => I::F32DemoteF64,
        wasmparser::Operator::F32ReinterpretI32 => I::F32ReinterpretI32,
        wasmparser::Operator::F64Eq => I::F64Eq,
        wasmparser::Operator::F64Ne => I::F64Ne,
        wasmparser::Operator::F64Lt => I::F64Lt,
        wasmparser::Operator::F64Gt => I::F64Gt,
        wasmparser::Operator::F64Le => I::F64Le,
        wasmparser::Operator::F64Ge => I::F64Ge,
        wasmparser::Operator::F64Add => I::F64Add,
        wasmparser::Operator::F64Sub => I::F64Sub,
        wasmparser::Operator::F64Mul => I::F64Mul,
        wasmparser::Operator::F64Div => I::F64Div,
        wasmparser::Operator::F64Min => I::F64Min,
        wasmparser::Operator::F64Max => I::F64Max,
        wasmparser::Operator::F64Abs => I::F64Abs,
        wasmparser::Operator::F64Neg => I::F64Neg,
        wasmparser::Operator::F64Sqrt => I::F64Sqrt,
        wasmparser::Operator::F64Ceil => I::F64Ceil,
        wasmparser::Operator::F64Floor => I::F64Floor,
        wasmparser::Operator::F64Trunc => I::F64Trunc,
        wasmparser::Operator::F64Nearest => I::F64Nearest,
        wasmparser::Operator::F64ConvertI32S => I::F64ConvertI32S,
        wasmparser::Operator::F64ConvertI32U => I::F64ConvertI32U,
        wasmparser::Operator::F64ConvertI64S => I::F64ConvertI64S,
        wasmparser::Operator::F64ConvertI64U => I::F64ConvertI64U,
        wasmparser::Operator::F64PromoteF32 => I::F64PromoteF32,
        wasmparser::Operator::F64ReinterpretI64 => I::F64ReinterpretI64,
        wasmparser::Operator::F64Copysign => I::F64Copysign,
        wasmparser::Operator::I64ReinterpretF64 => I::I64ReinterpretF64,
        wasmparser::Operator::I32ReinterpretF32 => I::I32ReinterpretF32,
        wasmparser::Operator::I64TruncSatF32S => I::I64TruncSatF32S,
        wasmparser::Operator::I64TruncSatF32U => I::I64TruncSatF32U,
        wasmparser::Operator::I64TruncSatF64S => I::I64TruncSatF64S,
        wasmparser::Operator::I64TruncSatF64U => I::I64TruncSatF64U,
        wasmparser::Operator::I32TruncSatF32S => I::I32TruncSatF32S,
        wasmparser::Operator::I32TruncSatF32U => I::I32TruncSatF32U,
        _ => {
            return Err(crate::error::ClsError::CompileError(format!(
                "operador de internals no soportado en la fusión: {:?}",
                op
            )))
        }
    })
}

fn map_blockty(b: wasmparser::BlockType, type_delta: u32) -> ClsResult<wasm_encoder::BlockType> {
    use wasm_encoder::BlockType;
    match b {
        wasmparser::BlockType::Empty => Ok(BlockType::Empty),
        wasmparser::BlockType::Type(t) => {
            let vt = parse_val_type(t)?;
            Ok(BlockType::Result(vt))
        }
        wasmparser::BlockType::FuncType(idx) => Ok(BlockType::FunctionType(type_delta + idx)),
    }
}

fn ma(m: wasmparser::MemArg) -> MemArg {
    MemArg { offset: m.offset, align: m.align as u32, memory_index: m.memory }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// El módulo internals debe exportar las funciones del manifiesto ABI.
    #[test]
    fn internals_wasm_has_abi_exports() {
        use wasmparser::{ExternalKind, Parser, Payload};
        let wasm = cls_internals::INTERNALS_WASM;
        let mut found: std::collections::HashSet<String> = std::collections::HashSet::new();
        for payload in Parser::new(0).parse_all(wasm) {
            if let Payload::ExportSection(r) = payload.unwrap() {
                for e in r {
                    let e = e.unwrap();
                    if e.kind == ExternalKind::Func {
                        found.insert(e.name.to_string());
                    }
                }
            }
        }
        for f in cls_internals::INTERNALS_FUNCTIONS {
            assert!(
                found.contains(f.name),
                "ABI export faltante en internals.wasm: {}",
                f.name
            );
        }
    }
}

//! Parte del motor de emision (Fase 1: extraido de engine/mod.rs).

use super::*;

impl<'a> Engine<'a> {
    pub(crate) fn build_global_init(&mut self) -> ClsResult<Option<Function>> {
        if self.global_inits.is_empty() {
            return Ok(None);
        }
        let mut fe = FuncEmitter::new(
            self.types,
            HostCaller {
                indexes: self.host_indexes.clone(),
            },
            &mut self.string_pool,
            &mut self.string_index,
            &self.func_indexes,
            &self.func_defaults,
            &self.fn_table_idx,
            &self.arrow_names,
            &self.arrow_captures,
            &mut self.type_count,
            &mut self.types_sec,
            &self.enum_defs,
            &self.struct_defs,
            &self.native_indexes,
            &self.native_ret,
            &self.globals,
            &self.static_fields,
            &self.class_defs,
            &self.method_type_indexes,
                &self.func_types,
                None,
            &self.target,
            self.tag_idx,
            self.eh_handler_ty,
            self.exceptions,
            &self.intrinsics,
        );
        for (idx, val) in &self.global_inits {
            fe.emit_expression(val)?;
            fe.body.push(Instruction::GlobalSet(*idx));
        }
        fe.body.push(Instruction::End);
        // Declarar los temporales que la emisiÃƒÆ’Ã‚Â³n pudo crear (emit_array, etc.).
        let local_types: Vec<ValType> = (0..fe.next_local)
            .map(|i| {
                fe.local_tys
                    .get(&i)
                    .copied()
                    .unwrap_or(WasTy::I64)
                    .val_type()
            })
            .collect();
        let grouped: Vec<(u32, ValType)> = local_types.iter().map(|t| (1, *t)).collect();
        let mut func = Function::new(grouped);
        for inst in fe.body {
            func.instruction(&inst);
        }
        Ok(Some(func))
    }

    /// Declara una funciÃƒÆ’Ã‚Â³n de clase (`Clase::m` o ctor) con `me` como primer param.
    /// Los mÃƒÆ’Ã‚Â©todos `static` NO reciben `me` (se registran como `Clase::__s__m`).
    pub(crate) fn build_allocator(&self) -> Function {
        // (func (param $n i64) (result i64)
        //   local 0 = n (param), local 1 = ptr, local 2 = end
        //   ptr = global 0
        //   end = (ptr + n + 8) & -8
        //   if end > memsize*65536 ÃƒÂ¢Ã¢â‚¬Â Ã¢â‚¬â„¢ grow las pÃƒÆ’Ã‚Â¡ginas exactas para cubrir `end`
        //   global 0 = end
        //   ptr)
        let mut b = vec![
            Instruction::GlobalGet(0),
            Instruction::LocalSet(1),
            Instruction::LocalGet(1),
            Instruction::LocalGet(0),
            Instruction::I64Add,
            Instruction::I64Const(8),
            Instruction::I64Add,
            Instruction::I64Const(-8),
            Instruction::I64And,
            Instruction::LocalSet(2),
            Instruction::Block(BlockType::Empty),
            // if end <= memsize*65536 ÃƒÂ¢Ã¢â‚¬Â Ã¢â‚¬â„¢ skip grow
            Instruction::LocalGet(2),
            Instruction::MemorySize(0),
            Instruction::I64ExtendI32U,
            Instruction::I64Const(65536),
            Instruction::I64Mul,
            Instruction::I64LeU,
            Instruction::BrIf(0),
            // pages_needed = ceil((end - memsize*65536) / 65536)
            Instruction::LocalGet(2),
            Instruction::MemorySize(0),
            Instruction::I64ExtendI32U,
            Instruction::I64Const(65536),
            Instruction::I64Mul,
            Instruction::I64Sub,
            Instruction::I64Const(65535),
            Instruction::I64Add,
            Instruction::I64Const(65536),
            Instruction::I64DivU,
            Instruction::I32WrapI64,
            Instruction::MemoryGrow(0),
            Instruction::Drop,
            Instruction::End,
            Instruction::LocalGet(2),
            Instruction::GlobalSet(0),
            Instruction::LocalGet(1),
            Instruction::End,
        ];
        let mut func = Function::new(vec![(2, ValType::I64)]);
        for inst in b.drain(..) {
            func.instruction(&inst);
        }
        func
    }

    pub(crate) fn build_load_str(&self) -> Function {
        // (func (param $i i64) (result i64)
        //   local 0 = i (param), 1 = entry, 2 = off, 3 = len
        //   entry = STRING_TABLE_BASE + i*8
        //   off = i32.load(entry)
        //   len = i32.load(entry+4)
        //   result = (off << 32) | len)
        let mut b = vec![
            Instruction::LocalGet(0),
            Instruction::I64Const(8),
            Instruction::I64Mul,
            Instruction::I64Const(STRING_TABLE_BASE as i64),
            Instruction::I64Add,
            Instruction::LocalSet(1),
            Instruction::LocalGet(1),
            Instruction::I32WrapI64,
            Instruction::I32Load(MemArg {
                offset: 0,
                align: 2,
                memory_index: 0,
            }),
            Instruction::I64ExtendI32U,
            Instruction::LocalSet(2),
            Instruction::LocalGet(1),
            Instruction::I64Const(4),
            Instruction::I64Add,
            Instruction::I32WrapI64,
            Instruction::I32Load(MemArg {
                offset: 0,
                align: 2,
                memory_index: 0,
            }),
            Instruction::I64ExtendI32U,
            Instruction::LocalSet(3),
            Instruction::LocalGet(2),
            Instruction::I64Const(32),
            Instruction::I64Shl,
            Instruction::LocalGet(3),
            Instruction::I64Or,
            Instruction::End,
        ];
        let mut func = Function::new(vec![(3, ValType::I64)]);
        for inst in b.drain(..) {
            func.instruction(&inst);
        }
        func
    }

    pub(crate) fn build_string_data(&self) -> Vec<u8> {
        let data_bytes: usize = self.string_pool.iter().map(|s| s.len()).sum();
        // El layout es: [0 .. data_len) = bytes de los strings (en orden de
        // interning, append-only) y [STRING_TABLE_BASE .. + 8N) = tabla de
        // ÃƒÆ’Ã‚Â­ndices (offset, len). Con base FIJA, los offsets de los datos NO
        // dependen del tamaÃƒÆ’Ã‚Â±o total del pool: el REPL (estado persistente)
        // transfiere punteros entre instancias y estos siguen siendo vÃƒÆ’Ã‚Â¡lidos
        // mientras las entradas compartidas conserven su posiciÃƒÆ’Ã‚Â³n (prefix).
        assert!(
            data_bytes <= STRING_TABLE_BASE as usize,
            "el string pool excede la regiÃƒÆ’Ã‚Â³n de datos ({} > {} bytes)",
            data_bytes,
            STRING_TABLE_BASE
        );
        let mut bytes: Vec<u8> =
            vec![0u8; STRING_TABLE_BASE as usize + self.string_pool.len() * 8];
        let mut data_off = 0u32;
        for s in self.string_pool.iter() {
            bytes[data_off as usize..data_off as usize + s.len()].copy_from_slice(s.as_bytes());
            data_off += s.len() as u32;
        }
        let mut entry = STRING_TABLE_BASE as usize;
        let mut off = 0u32;
        for s in self.string_pool.iter() {
            let len = s.len() as u32;
            bytes[entry..entry + 4].copy_from_slice(&off.to_le_bytes());
            bytes[entry + 4..entry + 8].copy_from_slice(&len.to_le_bytes());
            off += len;
            entry += 8;
        }
        bytes
    }
}

//! Strings: to_string, print args, shape to json (Fase 1: extraido de emitter/mod.rs).

use super::*;

impl<'a> FuncEmitter<'a> {


    /// `__intr_str_*` (call directo por nombre). La fusión de internals es
    /// incondicional (emit.rs llama fuse_internals siempre), así que la
    /// internals existe; no hay fallback host (los hosts de str no se registran).
    pub(crate) fn emit_str_host(&mut self, name: &str) {
        if let Some(&idx) = self.func_indexes.get(name) {
            self.body.push(Instruction::Call(idx));
        }
    }

    /// Convierte un valor (ya en el stack, cargado según `w`) a su string según
    /// el TIPO CLS `t`. RECURSIVO para contenedores y tipos Named: un campo que
    /// es struct/clase/enum/array/record/tuple se serializa con su formato, no
    /// como int/ptr (bug dev-2: campos anidados mostraban punteros).
    /// Deja el string packed en el stack.
    pub(crate) fn emit_typed_value_to_string(&mut self, w: WasTy, t: &Type) -> ClsResult<()> {
        match t {
            Type::String => {} // ya es string packed
            Type::Bool => self.emit_str_host("__intr_str_bool"),
            Type::Char => self.emit_str_host("__intr_str_char"),
            Type::Float | Type::F32 | Type::F64 => self.emit_str_host("__intr_str_float"),
            Type::Named(name, _) if self.struct_defs.contains_key(name) => {
                let ptr = self.fresh_local();
                self.body.push(Instruction::LocalSet(ptr));
                self.emit_struct_to_string(name, ptr)?;
            }
            Type::Named(name, _) if self.enum_variants(name).is_some() => {
                self.emit_enum_to_string(name)?;
            }
            Type::Named(name, _) if self.class_defs.contains_key(name) => {
                if let Some(idx) = self.func_indexes.get(&format!("{}::__toString", name)) {
                    self.body.push(Instruction::Call(*idx));
                } else if let Some(idx) = self.func_indexes.get(&format!("{}::__repr", name)) {
                    self.body.push(Instruction::Call(*idx));
                } else {
                    let ptr = self.fresh_local();
                    self.body.push(Instruction::LocalSet(ptr));
                    self.emit_class_to_string(name, ptr)?;
                }
            }
            Type::Array(elem) => {
                let ew = was_type(elem)?;
                let kind = arr_kind_code(elem);
                let es = if matches!(**elem, Type::Cmx) {
                    16
                } else {
                    elem_size_bytes(ew)
                };
                self.body.push(Instruction::I64Const(es));
                self.body.push(Instruction::I64Const(kind));
                self.emit_str_host("__intr_arr_to_string");
            }
            Type::Tuple(slots) => {
                // El ptr de la tupla está en el stack; emit_tuple_to_string
                // hace su propio LocalSet.
                self.emit_tuple_to_string(slots)?;
            }
            Type::Record(_, _) | Type::Shape(_) => {
                self.emit_str_host("__intr_record_to_string");
            }
            Type::Fun(..) => {
                self.host.call(HostFn::FnToString, &mut self.body);
            }
            _ => match w {
                WasTy::F64 => self.emit_str_host("__intr_str_float"),
                WasTy::I32 => {
                    self.body.push(Instruction::I64ExtendI32U);
                    self.emit_str_host("__intr_str_int");
                }
                _ => self.emit_str_host("__intr_str_int"),
            },
        }
        Ok(())
    }

    /// Formatea `<Clase {campo: valor, ...}>` (el ptr de la instancia está en
    /// `ptr`) y deja el string en el stack. Recursivo: los campos que son
    /// contenedores/Named se serializan con su tipo (bug dev-2: campos anidados
    /// mostraban punteros). Extraído del inline de print.rs.
    pub(crate) fn emit_class_to_string(&mut self, name: &str, ptr: u32) -> ClsResult<()> {
        let info = self.class_defs[name].clone();
        let open = format!("<{} {{", name);
        let s = self.intern_string(&open);
        self.emit_load_str(s);
        let res = self.fresh_local();
        self.body.push(Instruction::LocalSet(res));
        for (i, (fname, t_cls, w, off, _vis)) in info.fields.iter().enumerate() {
            let label = format!("{}: ", fname);
            let ls = self.intern_string(&label);
            self.emit_load_str(ls);
            let lt = self.fresh_local();
            self.body.push(Instruction::LocalSet(lt));
            self.body.push(Instruction::LocalGet(res));
            self.body.push(Instruction::LocalGet(lt));
            self.emit_str_host("__intr_str_concat");
            self.body.push(Instruction::LocalSet(res));
            // valor
            self.body.push(Instruction::LocalGet(ptr));
            self.body.push(Instruction::I64Const(*off));
            self.body.push(Instruction::I64Add);
            self.body.push(Instruction::I32WrapI64);
            match w {
                WasTy::F64 => self.body.push(Instruction::F64Load(MemArg {
                    offset: 0,
                    align: 3,
                    memory_index: 0,
                })),
                WasTy::I32 => self.body.push(Instruction::I32Load(MemArg {
                    offset: 0,
                    align: 2,
                    memory_index: 0,
                })),
                WasTy::I64 => self.body.push(Instruction::I64Load(MemArg {
                    offset: 0,
                    align: 3,
                    memory_index: 0,
                })),
            }
            if matches!(t_cls, Type::String) {
                // el valor ya es un string (ptr<<32|len): concatenar directo
            } else {
                self.emit_typed_value_to_string(*w, t_cls)?;
            }
            let sv = self.fresh_local();
            self.body.push(Instruction::LocalSet(sv));
            self.body.push(Instruction::LocalGet(res));
            self.body.push(Instruction::LocalGet(sv));
            self.emit_str_host("__intr_str_concat");
            self.body.push(Instruction::LocalSet(res));
            if i < info.fields.len() - 1 {
                let sep = self.intern_string(", ");
                self.emit_load_str(sep);
                let st = self.fresh_local();
                self.body.push(Instruction::LocalSet(st));
                self.body.push(Instruction::LocalGet(res));
                self.body.push(Instruction::LocalGet(st));
                self.emit_str_host("__intr_str_concat");
                self.body.push(Instruction::LocalSet(res));
            }
        }
        let close = self.intern_string("}>");
        self.emit_load_str(close);
        let ct = self.fresh_local();
        self.body.push(Instruction::LocalSet(ct));
        self.body.push(Instruction::LocalGet(res));
        self.body.push(Instruction::LocalGet(ct));
        self.emit_str_host("__intr_str_concat");
        self.body.push(Instruction::LocalSet(res));
        self.body.push(Instruction::LocalGet(res));
        Ok(())
    }

    /// Formatea un valor enum (en el stack, `(def_id<<32)|idx`) a su nombre de
    /// variante (`Activo`) y deja el string en el stack. Extraído del inline
    /// de print.rs (sin el PrintStr final).
    pub(crate) fn emit_enum_to_string(&mut self, name: &str) -> ClsResult<()> {
        let variants = self.enum_variants(name).unwrap().clone();
        // index = v & 0xffffffff
        self.body.push(Instruction::I64Const(0xffff_ffff));
        self.body.push(Instruction::I64And);
        let idx = self.fresh_local();
        self.body.push(Instruction::LocalSet(idx));
        // Enum def como valor (index 0xffffffff) -> `<enum X>`.
        self.body.push(Instruction::LocalGet(idx));
        self.body.push(Instruction::I64Const(0xffff_ffff));
        self.body.push(Instruction::I64Eq);
        self.block_depth += 1;
        self.body.push(Instruction::If(BlockType::Result(ValType::I64)));
        let eds = self.intern_string(&format!("<enum {}>", name));
        self.emit_load_str(eds);
        self.body.push(Instruction::Else);
        let n = variants.len();
        if n == 0 {
            let s = self.intern_string("");
            self.emit_load_str(s);
        } else {
            self.body.push(Instruction::LocalGet(idx));
            self.body.push(Instruction::I64Const(0));
            self.body.push(Instruction::I64Eq);
            self.block_depth += 1;
            self.body.push(Instruction::If(BlockType::Result(ValType::I64)));
            let s0 = self.intern_string(&variants[0]);
            self.emit_load_str(s0);
            if n > 1 {
                for (i, variant) in variants.iter().enumerate().skip(1) {
                    self.body.push(Instruction::Else);
                    if i == n - 1 {
                        let s = self.intern_string(variant);
                        self.emit_load_str(s);
                    } else {
                        self.body.push(Instruction::LocalGet(idx));
                        self.body.push(Instruction::I64Const(i as i64));
                        self.body.push(Instruction::I64Eq);
                        self.block_depth += 1;
                        self.body.push(Instruction::If(BlockType::Result(ValType::I64)));
                        let s = self.intern_string(variant);
                        self.emit_load_str(s);
                    }
                }
                // Cierra los Ifs anidados del loop: para n>1 hay n-1 Ifs
                // (idx==0 + los intermedios). El último Else no abre If.
                for _ in 0..(n - 1) {
                    self.body.push(Instruction::End);
                    self.block_depth -= 1;
                }
            } else {
                // n == 1: un solo If (idx==0) -> un End.
                self.body.push(Instruction::End);
                self.block_depth -= 1;
            }
        }
        self.body.push(Instruction::End);
        self.block_depth -= 1;
        Ok(())
    }


    /// Serializa un array de valores Named (ptr del array en el stack) a
    /// `[e1, e2, ...]` donde cada elemento se serializa con su tipo (struct/
    /// clase/enum). El formateo genérico de arrays no conoce tipos del programa
    /// -> mostraba ptrs (bug dev-2: `celulas: [1900568]`).
    pub(crate) fn emit_named_array_to_string(&mut self, elem: &Type) -> ClsResult<()> {
        // ptr del array en stack -> local
        let arr_ptr = self.fresh_local();
        self.body.push(Instruction::LocalSet(arr_ptr));
        let open = self.intern_string("[");
        self.emit_load_str(open);
        let res = self.fresh_local();
        self.body.push(Instruction::LocalSet(res));
        let len = self.fresh_local();
        self.body.push(Instruction::LocalGet(arr_ptr));
        self.body.push(Instruction::I64Const(8));
        self.body.push(Instruction::I64Add);
        self.body.push(Instruction::I32WrapI64);
        self.body.push(Instruction::I64Load(MemArg {
            offset: 0,
            align: 3,
            memory_index: 0,
        }));
        self.body.push(Instruction::LocalSet(len));
        let i = self.fresh_local();
        self.body.push(Instruction::I64Const(0));
        self.body.push(Instruction::LocalSet(i));
        let d = self.block_depth;
        self.block_depth += 1;
        self.body.push(Instruction::Block(BlockType::Empty));
        let break_at = self.block_depth;
        self.block_depth += 1;
        self.body.push(Instruction::Loop(BlockType::Empty));
        // cond: i >= len -> break
        self.body.push(Instruction::LocalGet(i));
        self.body.push(Instruction::LocalGet(len));
        self.body.push(Instruction::I64GeS);
        self.body.push(Instruction::BrIf(break_at));
        // separador ", " si i > 0
        self.body.push(Instruction::LocalGet(i));
        self.body.push(Instruction::I64Const(0));
        self.body.push(Instruction::I64GtS);
        self.body.push(Instruction::If(BlockType::Empty));
        self.body.push(Instruction::LocalGet(res));
        let sep = self.intern_string(", ");
        self.emit_load_str(sep);
        self.emit_str_host("__intr_str_concat");
        self.body.push(Instruction::LocalSet(res));
        self.body.push(Instruction::End);
        // elemento = arr[i] (es=8, ptrs/i64)
        self.body.push(Instruction::LocalGet(arr_ptr));
        self.body.push(Instruction::LocalGet(i));
        self.body.push(Instruction::I64Const(8));
        self.body.push(Instruction::I64Mul);
        self.body.push(Instruction::I64Const(16));
        self.body.push(Instruction::I64Add);
        self.body.push(Instruction::I64Add);
        self.body.push(Instruction::I32WrapI64);
        self.body.push(Instruction::I64Load(MemArg {
            offset: 0,
            align: 3,
            memory_index: 0,
        }));
        // serializar el elemento por su tipo
        self.emit_typed_value_to_string(WasTy::I64, elem)?;
        let sv = self.fresh_local();
        self.body.push(Instruction::LocalSet(sv));
        self.body.push(Instruction::LocalGet(res));
        self.body.push(Instruction::LocalGet(sv));
        self.emit_str_host("__intr_str_concat");
        self.body.push(Instruction::LocalSet(res));
        // i++
        self.body.push(Instruction::LocalGet(i));
        self.body.push(Instruction::I64Const(1));
        self.body.push(Instruction::I64Add);
        self.body.push(Instruction::LocalSet(i));
        self.body.push(Instruction::Br(0));
        self.body.push(Instruction::End); // loop
        self.block_depth -= 1;
        self.body.push(Instruction::End); // block
        self.block_depth = d;
        let close = self.intern_string("]");
        self.body.push(Instruction::LocalGet(res));
        self.emit_load_str(close);
        self.emit_str_host("__intr_str_concat");
        self.body.push(Instruction::LocalSet(res));
        self.body.push(Instruction::LocalGet(res));
        Ok(())
    }

    /// Construye la representación `Punto { x: 3, y: 4 }` de un struct y la deja
    /// en el stack (el ptr del struct está en `ptr`).
    pub(crate) fn emit_struct_to_string(&mut self, name: &str, ptr: u32) -> ClsResult<()> {
        let info = self.struct_defs[name].clone();
        let open = format!("{} {{ ", name);
        let s = self.intern_string(&open);
        self.emit_load_str(s);
        let res = self.fresh_local();
        self.body.push(Instruction::LocalSet(res));
        for (i, (fname, t_cls, w)) in info.fields.iter().enumerate() {
            // Los arrays inline (`T[n]`) solo ocupan espacio: no se imprimen.
            let Some(fw) = info.kinds[i].scalar() else {
                continue;
            };
            let label = format!("{}: ", fname);
            let ls = self.intern_string(&label);
            self.emit_load_str(ls);
            let lt = self.fresh_local();
            self.body.push(Instruction::LocalSet(lt));
            self.body.push(Instruction::LocalGet(res));
            self.body.push(Instruction::LocalGet(lt));
            self.emit_str_host("__intr_str_concat");
            self.body.push(Instruction::LocalSet(res));
            self.body.push(Instruction::LocalGet(ptr));
            self.body.push(Instruction::I64Const(info.offsets[i]));
            self.body.push(Instruction::I64Add);
            self.body.push(Instruction::I32WrapI64);
            // Carga con el ancho C del campo (i16 -> 2 bytes + extend).
            fw.push_load(&mut self.body, *w);
            if matches!(t_cls, Type::String) {
                let q = self.intern_string("\"");
                self.emit_load_str(q);
                let qt = self.fresh_local();
                self.body.push(Instruction::LocalSet(qt));
                self.body.push(Instruction::LocalGet(res));
                self.body.push(Instruction::LocalGet(qt));
                self.emit_str_host("__intr_str_concat");
                self.body.push(Instruction::LocalSet(res));
                let sv = self.fresh_local();
                self.body.push(Instruction::LocalSet(sv));
                self.body.push(Instruction::LocalGet(res));
                self.body.push(Instruction::LocalGet(sv));
                self.emit_str_host("__intr_str_concat");
                self.body.push(Instruction::LocalSet(res));
                let q2 = self.intern_string("\"");
                self.emit_load_str(q2);
                let qt2 = self.fresh_local();
                self.body.push(Instruction::LocalSet(qt2));
                self.body.push(Instruction::LocalGet(res));
                self.body.push(Instruction::LocalGet(qt2));
                self.emit_str_host("__intr_str_concat");
                self.body.push(Instruction::LocalSet(res));
            } else {
                // No-String: despacho recursivo por tipo CLS (bool/char/float/
                // int, y contenedores/Named: struct/clase/enum/array/record/
                // tuple anidados se serializan, no como ptr — bug dev-2).
                self.emit_typed_value_to_string(*w, t_cls)?;
                let sv = self.fresh_local();
                self.body.push(Instruction::LocalSet(sv));
                self.body.push(Instruction::LocalGet(res));
                self.body.push(Instruction::LocalGet(sv));
                self.emit_str_host("__intr_str_concat");
                self.body.push(Instruction::LocalSet(res));
            }
            if i < info.fields.len() - 1 {
                let sep = self.intern_string(", ");
                self.emit_load_str(sep);
                let st = self.fresh_local();
                self.body.push(Instruction::LocalSet(st));
                self.body.push(Instruction::LocalGet(res));
                self.body.push(Instruction::LocalGet(st));
                self.emit_str_host("__intr_str_concat");
                self.body.push(Instruction::LocalSet(res));
            }
        }
        let close = self.intern_string(" }");
        self.emit_load_str(close);
        let ct = self.fresh_local();
        self.body.push(Instruction::LocalSet(ct));
        self.body.push(Instruction::LocalGet(res));
        self.body.push(Instruction::LocalGet(ct));
        self.emit_str_host("__intr_str_concat");
        self.body.push(Instruction::LocalSet(res));
        self.body.push(Instruction::LocalGet(res));
        Ok(())
    }



    pub(crate) fn emit_to_string(&mut self, arg: &Expression) -> ClsResult<()> {
        let span = expr_span(arg);
        let t = self.types.get(&span).cloned().unwrap_or(Type::Any);
        match t {
            Type::String => {}
            Type::Bool => self.emit_str_host("__intr_str_bool"),
            Type::Char => self.emit_str_host("__intr_str_char"),
            Type::Float => self.emit_str_host("__intr_str_float"),
            Type::Null => {
                // null -> string "null"
                self.body.push(Instruction::Drop);
                let n = self.intern_string("null");
                self.emit_load_str(n);
            }
            Type::Named(name, _) if self.struct_defs.contains_key(&name) => {
                let ptr = self.fresh_local();
                self.body.push(Instruction::LocalSet(ptr));
                self.emit_struct_to_string(&name, ptr)?;
            }
            Type::Named(name, _) if self.class_defs.contains_key(&name) => {
                // toString(obj) -> __toString si existe; si no, __repr; el ptr
                // está en stack. Fallback: formato <Clase {...}> recursivo
                // (antes __intr_str_int -> ptr, bug dev-2).
                if let Some(idx) = self.func_indexes.get(&format!("{}::__toString", name)) {
                    self.body.push(Instruction::Call(*idx));
                } else if let Some(idx) = self.func_indexes.get(&format!("{}::__repr", name)) {
                    self.body.push(Instruction::Call(*idx));
                } else {
                    let ptr = self.fresh_local();
                    self.body.push(Instruction::LocalSet(ptr));
                    self.emit_class_to_string(&name, ptr)?;
                }
            }
            Type::Named(name, _) if self.enum_variants(&name).is_some() => {
                // str(enum) -> nombre de la variante (antes caía a str_int con
                // el valor crudo (def_id<<32)|idx, bug dev-2).
                self.emit_enum_to_string(&name)?;
            }
            Type::Array(elem) => {
                // `[e1, e2, ...]` como el walker (paridad en interpolación).
                let w = was_type(&*elem)?;
                let kind = arr_kind_code(&*elem);
                let es = if matches!(*elem, Type::Cmx) {
                    16
                } else {
                    elem_size_bytes(w)
                };
                self.body.push(Instruction::I64Const(es));
                self.body.push(Instruction::I64Const(kind));
                // Bug fix dev-2 (Fase 7): antes llamaba `self.host.call(HostFn::ArrToString, ...)`
                // pero ese host ya no se importa (las internals están fusionadas en el
                // módulo). La internal correcta es `__intr_arr_to_string(ptr, es, kind)`.
                // Migracion Fase 3 (paso 3) dejo este path muerto.
                self.emit_str_host("__intr_arr_to_string");
            }
            Type::Fun(..) => {
                // Handle de función -> `<function X>` (el nombre está en el handle).
                self.host.call(HostFn::FnToString, &mut self.body);
            }
            Type::Any | Type::Unknown | Type::Json | Type::Value => {
                // Valor dinámico (leído de record/JSON): el tag del runtime
                // decide la conversión. El caller dejó el val en el stack sin
                // tag; re-emitimos con emit_any_chain (val + tag) y despachamos
                // por tag. (str("x") con punteros ya no: se imprime el
                // contenido por tag string.)
                self.body.push(Instruction::Drop);
                self.emit_any_chain(arg)?;
                self.emit_str_host("__intr_any_to_string");
            }
            _ => self.emit_str_host("__intr_str_int"),
        }
        Ok(())
    }



    /// Frontera de append: emite la pieza como `(val:i64, tag:i64)` — el ABI de
    /// `__intr_str_append`. Escalares concretos emiten valor+tag estático;
    /// dinámicos usan el camino por tag runtime (emit_any_chain).
    pub(crate) fn emit_append_piece(&mut self, expr: &Expression) -> ClsResult<()> {
        let t = self.types.get(&expr_span(expr)).cloned().unwrap_or(Type::Any);
        if Self::is_dynamic_dest(&t) || matches!(t, Type::Fun(..) | Type::Named(..)) {
            return self.emit_any_chain(expr);
        }
        self.emit_expression(expr)?;
        match was_type(&t)? {
            WasTy::F64 => self.body.push(Instruction::I64ReinterpretF64),
            WasTy::I32 => self.body.push(Instruction::I64ExtendI32U),
            _ => {}
        }
        self.body.push(Instruction::I64Const(runtime_tag_code_compound(&t)));
        Ok(())
    }


    /// Secuencia de append: `old` en el stack + pieza (val, tag) ->
    /// `__intr_str_append` (internal fusionada o host como fallback).
    pub(crate) fn emit_str_append(
        &mut self,
        old: &Expression,
        piece: &Expression,
    ) -> ClsResult<()> {
        self.emit_expression(old)?;
        self.emit_append_piece(piece)?;
        self.emit_str_host("__intr_str_append");
        Ok(())
    }




    pub(crate) fn emit_to_int(&mut self, arg: &Expression) -> ClsResult<()> {
        let span = expr_span(arg);
        let t = self.types.get(&span).cloned().unwrap_or(Type::Any);
        match t {
            Type::Int => {}
            Type::Float => self.body.push(Instruction::I64TruncSatF64S),
            Type::Bool => self.body.push(Instruction::I64ExtendI32U),
            Type::String => {
                self.emit_call_site(&span);
                if !self.exceptions {
                    // wasmi (sin excepciones WASM): no se puede trapear con
                    // mensaje; comportamiento previo (0 en fallo).
                    if let Some(&idx) = self.func_indexes.get("__intr_parse_int") {
                        self.body.push(Instruction::Call(idx));
                    }
                    return Ok(());
                }
                // Parsear y, si falla (flag de error de las internals), lanzar
                // una excepción CLS clara en vez de devolver 0 en silencio
                // (paridad con `host_parse_int`, que devuelve Err).
                let s_local = self.fresh_local();
                self.body.push(Instruction::LocalSet(s_local));
                self.body.push(Instruction::LocalGet(s_local));
                if let Some(&idx) = self.func_indexes.get("__intr_parse_int") {
                    self.body.push(Instruction::Call(idx));
                }
                let r_local = self.fresh_local();
                self.body.push(Instruction::LocalSet(r_local));
                if let Some(&idx) = self.func_indexes.get("__intr_parse_error_get") {
                    self.body.push(Instruction::Call(idx));
                    self.block_depth += 1;
                    self.body.push(Instruction::If(BlockType::Empty));
                    let msg = self.intern_string("int: no se puede convertir la cadena a entero");
                    self.emit_load_str(msg);
                    let packed = ((span.start_line as i64) << 32) | (span.start_col as i64);
                    self.body.push(Instruction::I64Const(packed));
                    self.body.push(Instruction::Throw(self.tag_idx));
                    self.body.push(Instruction::Unreachable);
                    self.body.push(Instruction::End);
                    self.block_depth -= 1;
                }
                self.body.push(Instruction::LocalGet(r_local));
            }
            _ => {}
        }
        Ok(())
    }



    pub(crate) fn emit_to_float(&mut self, arg: &Expression) -> ClsResult<()> {
        let span = expr_span(arg);
        let t = self.types.get(&span).cloned().unwrap_or(Type::Any);
        match t {
            Type::Float => {}
            Type::Int => self.body.push(Instruction::F64ConvertI64S),
            Type::Bool => {
                self.body.push(Instruction::I64ExtendI32U);
                self.body.push(Instruction::F64ConvertI64S);
            }
            Type::String => {
                self.emit_call_site(&span);
                if let Some(&idx) = self.func_indexes.get("__intr_parse_float") {
                    self.body.push(Instruction::Call(idx));
                }
            }
            _ => {}
        }
        Ok(())
    }



    pub(crate) fn emit_to_bool(&mut self, arg: &Expression) -> ClsResult<()> {
        // Reutiliza coerce_to_bool: la misma semántica de truthiness del walker
        // (int/float != 0, string len != 0, array/record len != 0, cmx/objetos
        // true). Antes los compuestos (cmx/array/record/any) caían en `_` y
        // dejaban el ptr i64 en el stack → `if (bool(x))` emitía WASM inválido.
        self.coerce_to_bool(arg)
    }

}

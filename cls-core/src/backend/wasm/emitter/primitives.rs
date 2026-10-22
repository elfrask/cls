//! primitives.rs (Fase 1: extraido de emitter/calls.rs).

use super::*;

impl<'a> FuncEmitter<'a> {
    pub(crate) fn emit_primitive_method(&mut self, c: &CallExpr) -> ClsResult<bool> {
    // Métodos de primitivos (callee MemberAccess)
    if let Expression::MemberAccess(member) = &*c.callee {
        // Métodos de CMX (rework): predicados, inspeccion y toString (inline, 0 host
        // salvo toString que usa cmx_to_string).
        if matches!(self.types.get(&expr_span(&member.object)), Some(Type::Cmx)) {
            // Métodos de la clase core `CMX` (frontend/builtins): dispatch DIRECTO
            // `CMX::metodo(me, args)` (CMX es final; sin vtable).
            if let Some(info) = self.class_defs.get("CMX") {
                if info.methods.iter().any(|m| m == member.member.as_str()) {
                    if let Some(&idx) = self.func_indexes.get(&format!("CMX::{}", member.member)) {
                        self.emit_expression(&member.object)?; // me
                        for a in &c.args {
                            self.emit_expression(a)?;
                        }
                        self.emit_call_site(&c.span);
                        self.body.push(Instruction::Call(idx));
                        return Ok(true);
                    }
                }
            }
            match member.member.as_str() {
                "toString" | "__str" | "__repr" => {
                    self.emit_expression(&member.object)?;
                    self.emit_cmx_fmt();
                    return Ok(true);
                }
                "cloneWith" => {
                    // cloneWith(props?, children?) -> nuevo Cmx (copia inmutable;
                    // conserva tag/kind/tagCallable/isComponent). Construccion inline.
                    self.emit_expression(&member.object)?;
                    let p = self.fresh_local();
                    self.body.push(Instruction::LocalSet(p));
                    let alloc = self.func_indexes["__alloc"];
                    self.body.push(Instruction::I64Const(48));
                    self.body.push(Instruction::Call(alloc));
                    let n = self.fresh_local();
                    self.body.push(Instruction::LocalSet(n));
                    self.emit_cmx_copy_store(n, p, 0, 0)?; // tag
                    self.emit_cmx_store_arg(n, 8, c.args.first(), p, 8)?; // props
                    self.emit_cmx_store_arg(n, 16, c.args.get(1), p, 16)?; // children
                    self.emit_cmx_copy_store(n, p, 24, 24)?; // kind
                    self.emit_cmx_copy_store(n, p, 32, 32)?; // tagCallable
                    self.emit_cmx_copy_store(n, p, 40, 40)?; // isComponent
                    self.body.push(Instruction::LocalGet(n));
                    return Ok(true);
                }
                "tryCallTag" => {
                    // tryCallTag() -> Value: si es componente, invoca
                    // `tagCallable(props, children)`; si no, Null (0).
                    self.emit_expression(&member.object)?;
                    let p = self.fresh_local();
                    self.body.push(Instruction::LocalSet(p));
                    self.body.push(Instruction::LocalGet(p));
                    self.emit_cmx_field(40)?;
                    self.body.push(Instruction::I64Eqz);
                    self.block_depth += 1;
                    self.body.push(Instruction::If(BlockType::Result(ValType::I64)));
                    self.body.push(Instruction::I64Const(0)); // Null
                    self.body.push(Instruction::Else);
                    self.body.push(Instruction::LocalGet(p));
                    self.emit_cmx_field(32)?; // handle
                    let h = self.fresh_local();
                    self.body.push(Instruction::LocalSet(h));
                    let tidx = self.register_func_type(
                        vec![ValType::I64, ValType::I64, ValType::I64],
                        vec![ValType::I64],
                    );
                    self.body.push(Instruction::LocalGet(h));
                    self.body.push(Instruction::I64Const(1));
                    self.body.push(Instruction::I64And);
                    self.body.push(Instruction::I32WrapI64);
                    self.block_depth += 1;
                    self.body.push(Instruction::If(BlockType::Result(ValType::I64)));
                    // closure: caps + props + children + fnptr
                    self.body.push(Instruction::LocalGet(h));
                    self.body.push(Instruction::I64Const(1));
                    self.body.push(Instruction::I64ShrU);
                    self.body.push(Instruction::I64Const(8));
                    self.body.push(Instruction::I64Add);
                    self.body.push(Instruction::I32WrapI64);
                    self.body.push(Instruction::I64Load(MemArg { offset: 0, align: 3, memory_index: 0 }));
                    self.body.push(Instruction::LocalGet(p));
                    self.emit_cmx_field(8)?;
                    self.body.push(Instruction::LocalGet(p));
                    self.emit_cmx_field(16)?;
                    self.body.push(Instruction::LocalGet(h));
                    self.body.push(Instruction::I64Const(1));
                    self.body.push(Instruction::I64ShrU);
                    self.body.push(Instruction::I32WrapI64);
                    self.body.push(Instruction::I64Load(MemArg { offset: 0, align: 3, memory_index: 0 }));
                    self.body.push(Instruction::I32WrapI64);
                    self.emit_call_site(&c.span);
                    self.body.push(Instruction::CallIndirect { type_index: tidx, table_index: 0 });
                    self.body.push(Instruction::Else);
                    // simple: caps=0 + props + children + idx
                    self.body.push(Instruction::I64Const(0));
                    self.body.push(Instruction::LocalGet(p));
                    self.emit_cmx_field(8)?;
                    self.body.push(Instruction::LocalGet(p));
                    self.emit_cmx_field(16)?;
                    self.body.push(Instruction::LocalGet(h));
                    self.body.push(Instruction::I64Const(1));
                    self.body.push(Instruction::I64ShrU);
                    self.body.push(Instruction::I32WrapI64);
                    self.emit_call_site(&c.span);
                    self.body.push(Instruction::CallIndirect { type_index: tidx, table_index: 0 });
                    self.body.push(Instruction::End);
                    self.block_depth -= 1;
                    self.body.push(Instruction::End);
                    self.block_depth -= 1;
                    return Ok(true);
                }
                _ => {}
            }
        }
        // `super.m(args)` -> call directo al método del padre (sin vtable).
        if let Expression::Identifier(sn, _) = &*member.object {
            if sn == "super" {
                if let Some(cur) = &self.current_class {
                    if let Some(parent) =
                        self.class_defs.get(cur).and_then(|i| i.parent.clone())
                    {
                        // `super.main(...)` -> ctor del padre (ClassDef.ctor se
                        // emite como `__ctor`). `super.metodo(...)` -> método.
                        let key = if member.member == "main" {
                            format!("{}::__ctor", parent)
                        } else {
                            format!("{}::{}", parent, member.member)
                        };
                        if let Some(idx) = self.func_indexes.get(&key).copied() {
                            let expected =
                                self.func_types.get(&key).map(|(p, _)| p.clone());
                            self.body.push(Instruction::LocalGet(0)); // me
                            for (i, a) in c.args.iter().enumerate() {
                                self.emit_call_arg(a, expected.as_deref(), i + 1)?;
                            }
                            self.emit_call_site(&c.span);
                            self.body.push(Instruction::Call(idx));
                            return Ok(true);
                        }
                    }
                }
                return Err(crate::error::ClsError::CompileError(
                    "super solo se puede usar dentro de métodos de clase (JIT)".to_string(),
                ));
            }
        }
        // Módulos stdlib: math / json / fs
        if let Expression::Identifier(obj_name, _) = &*member.object {
            // Resolver alias de import: `import "math" as m` -> m es math.
            let canonical = self
                .import_aliases
                .get(obj_name)
                .cloned()
                .unwrap_or_else(|| obj_name.clone());
            let obj_name = &canonical;
            if obj_name == "math" {
                self.emit_math_call(member, c)?; return Ok(true);
            }
            if obj_name == "json" {
                if member.member == "parse" {
                    self.emit_expression(&c.args[0])?;
                    self.host.call(HostFn::JsonParse, &mut self.body);
                    return Ok(true);
                }
                if member.member == "stringify" {
                    let t = self
                        .types
                        .get(&expr_span(&c.args[0]))
                        .cloned()
                        .unwrap_or(Type::Value);
                    // Objeto de clase: __toJson si lo define; si no -> "null" (paridad walker).
                    if let Type::Named(cn, _) = &t {
                        if self.class_defs.contains_key(cn.as_str()) {
                            if self.emit_class_method("__toJson", &c.args[0])? {
                                return Ok(true);
                            }
                            self.emit_expression(&c.args[0])?;
                            self.body.push(Instruction::Drop);
                            let n = self.intern_string("null");
                            self.emit_load_str(n);
                            return Ok(true);
                        }
                        // struct/enum sin serialización -> "null" (paridad walker).
                        if self.struct_defs.contains_key(cn.as_str())
                            || self.enum_defs.contains_key(cn.as_str())
                        {
                            self.emit_expression(&c.args[0])?;
                            self.body.push(Instruction::Drop);
                            let n = self.intern_string("null");
                            self.emit_load_str(n);
                            return Ok(true);
                        }
                    }
                    // Shape -> stringify inline (json.stringify({x:1}) -> '{"x":1}').
                    // Shape: los literals viven como hashmap (default invertido)
                    // -> serializar por el camino dinámico con tag 7 (record).
                    if let Type::Shape(_) = &t {
                        self.emit_coerce(&c.args[0], Some(&Type::Json))?;
                        self.body.push(Instruction::I64Const(7));
                        self.host.call(HostFn::JsonStringify, &mut self.body);
                        return Ok(true);
                    }
                    // Para `Value`/`JSON` (valor leído de un record/JSON o
                    // dinámico): emitir con emit_any_chain (val + tag) y serializar
                    // por TAG real en runtime (host_json_stringify serializa
                    // escalares/record/array por tag). Sin esto,
                    // `json.stringify(d["int"])` devolvía el raw (puntero).
                    if matches!(t, Type::Value | Type::Unknown | Type::Json) {
                        self.emit_any_chain(&c.args[0])?;   // (val, tag)
                        self.host.call(HostFn::JsonStringify, &mut self.body);
                        return Ok(true);
                    }
                    self.emit_expression(&c.args[0])?;
                    // kind = tag del RUNTIME del valor (compuesto para arrays:
                    // es/kind correcto en el formateo, bug dev-2). Para tipos
                    // concretos (bool/int/float/string/char/record/array) el
                    // tag se conoce en compile-time y el host serializa por tag.
                    let kind = runtime_tag_code_compound(&t);
                    // Escalares: el valor debe viajar como i64 en el stack
                    // (host_json_stringify espera v: i64). Bool/char (i32) ->
                    // extender; float (f64) -> reinterpretar a bits i64.
                    match was_type(&t) {
                        Ok(WasTy::I32) => self.body.push(Instruction::I64ExtendI32U),
                        Ok(WasTy::F64) => self.body.push(Instruction::I64ReinterpretF64),
                        _ => {}
                    }
                    self.body.push(Instruction::I64Const(kind));
                    self.host.call(HostFn::JsonStringify, &mut self.body);
                    return Ok(true);
                }
            }
            if obj_name == "fs" {
                self.emit_fs_call(member, c)?; return Ok(true);
            }
            if obj_name == "http" {
                self.emit_http_call(member, c)?; return Ok(true);
            }
            if obj_name == "os" {
                self.emit_os_call(member, c)?; return Ok(true);
            }
            if obj_name == "path" {
                self.emit_path_call(member, c)?; return Ok(true);
            }
            if obj_name == "process" {
                self.emit_process_call(member, c)?; return Ok(true);
            }
            if obj_name == "time" {
                self.emit_time_call(member, c)?; return Ok(true);
            }
            if obj_name == "random" {
                self.emit_random_call(member, c)?; return Ok(true);
            }
            if obj_name == "net" {
                // net eliminado (dev-2): cae al error genérico de miembro no soportado.
                return Err(self.unsupported_expr(&Expression::Call(c.clone())));
            }
            if obj_name == "strings" {
                self.emit_strings_call(member, c)?; return Ok(true);
            }
            // `Clase.metodo()` con método static -> call directo (sin me).
            if self.class_defs.contains_key(obj_name.as_str()) {
                let skey = format!("{}::__s__{}", obj_name, member.member);
                if let Some(&idx) = self.func_indexes.get(&skey) {
                    for a in &c.args {
                        self.emit_expression(a)?;
                    }
                    self.emit_call_site(&c.span);
                    self.body.push(Instruction::Call(idx));
                    return Ok(true);
                }
            }
        }
        let obj_ty = self
            .types
            .get(&expr_span(&member.object))
            .cloned()
            .unwrap_or(Type::Value);
        let obj_ty = match self.object_class_key(&member.object) {
            Some(key) => Type::Named(key, vec![]),
            None => obj_ty,
        };
        if obj_ty.is_tagged_builtin_union() && member.member == "toString" {
            self.emit_expression(&member.object)?;
            let ptr = self.fresh_local_ty(WasTy::I64);
            self.body.push(Instruction::LocalSet(ptr));
            self.emit_tagged_union_raw_value(ptr);
            self.emit_tagged_union_tag(ptr);
            self.emit_str_host("__intr_any_to_string");
            return Ok(true);
        }
        match obj_ty {
            Type::Tuple(_) => match member.member.as_str() {
                "join" => { self.emit_tuple_join(member, c)?; return Ok(true); },
                _ => return Err(self.unsupported_expr(&Expression::Call(c.clone()))),
            },
            Type::String => {
                self.emit_expression(&member.object)?;
                match member.member.as_str() {
                    "upper" | "lower" | "trim" => {
                        let name = match member.member.as_str() {
                            "upper" => "__intr_str_upper",
                            "lower" => "__intr_str_lower",
                            _ => "__intr_str_trim",
                        };
                        self.emit_str_host(name);
                        return Ok(true);
                    }
                    "contains" | "startsWith" | "endsWith" => {
                        self.emit_expression(&c.args[0])?;
                        let name = match member.member.as_str() {
                            "contains" => "__intr_str_contains",
                            "startsWith" => "__intr_str_starts_with",
                            _ => "__intr_str_ends_with",
                        };
                        self.emit_str_host(name);
                        return Ok(true);
                    }
                    "isEmpty" => {
                        self.emit_str_host("__intr_str_is_empty");
                        return Ok(true);
                    }
                    "indexOf" => {
                        self.emit_expression(&c.args[0])?;
                        self.emit_str_host("__intr_str_index_of");
                        return Ok(true);
                    }
                    "slice" => {
                        self.emit_expression(&c.args[0])?;
                        self.emit_expression(&c.args[1])?;
                        self.emit_str_host("__intr_str_slice");
                        return Ok(true);
                    }
                    "split" => {
                        self.emit_expression(&c.args[0])?;
                        self.emit_str_host("__intr_str_split");
                        return Ok(true);
                    }
                    "toString" => { return Ok(true); },
                    "toChar" => {
                        // Bytes del string como Char[] (unidad de 1 byte).
                        self.emit_str_host("__intr_str_to_chars");
                        return Ok(true);
                    }
                    "toBytes" => {
                        // Bytes del string como int[] (stride 8; para buffers/FFI).
                        self.emit_str_host("__intr_str_to_bytes");
                        return Ok(true);
                    }
                    _ => return Err(self.unsupported_expr(&Expression::Call(c.clone()))),
                }
            }
            Type::Array(_) => {
                let elem_ty = self.array_elem_was_type(&member.object)?;
                let elem_size = elem_size_bytes(elem_ty);
                self.emit_expression(&member.object)?;
                match member.member.as_str() {
                    "push" => {
                        // Frontera única: shape contiguo a array dinámico.
                        let elem_cls = self.array_elem_cls_type(&member.object)?;
                        self.emit_coerce(&c.args[0], Some(&elem_cls))?;
                        self.elem_to_bits(&c.args[0], elem_ty)?;
                        self.body.push(Instruction::I64Const(elem_size));
                        if let Some(&idx) = self.func_indexes.get("__intr_arr_push") {
                            self.body.push(Instruction::Call(idx));
                        }
                        self.writeback_array(&member.object)?;
                        return Ok(true);
                    }
                    "pop" => {
                        self.body.push(Instruction::I64Const(elem_size));
                        if let Some(&idx) = self.func_indexes.get("__intr_arr_pop") {
                            self.body.push(Instruction::Call(idx));
                        }
                        self.writeback_array(&member.object)?;
                        return Ok(true);
                    }
                    "shift" => {
                        self.body.push(Instruction::I64Const(elem_size));
                        if let Some(&idx) = self.func_indexes.get("__intr_arr_shift") {
                            self.body.push(Instruction::Call(idx));
                        }
                        self.writeback_array(&member.object)?;
                        return Ok(true);
                    }
                    "unshift" => {
                        // Frontera única: shape contiguo a array dinámico.
                        let elem_cls = self.array_elem_cls_type(&member.object)?;
                        self.emit_coerce(&c.args[0], Some(&elem_cls))?;
                        self.elem_to_bits(&c.args[0], elem_ty)?;
                        self.body.push(Instruction::I64Const(elem_size));
                        if let Some(&idx) = self.func_indexes.get("__intr_arr_unshift") {
                            self.body.push(Instruction::Call(idx));
                        }
                        self.writeback_array(&member.object)?;
                        return Ok(true);
                    }
                    "reverse" => {
                        self.body.push(Instruction::I64Const(elem_size));
                        if let Some(&idx) = self.func_indexes.get("__intr_arr_reverse") {
                            self.body.push(Instruction::Call(idx));
                        }
                        self.writeback_array(&member.object)?;
                        return Ok(true);
                    }
                    "indexOf" => {
                        self.emit_expression(&c.args[0])?;
                        self.elem_to_bits(&c.args[0], elem_ty)?;
                        self.body.push(Instruction::I64Const(elem_size));
                        if let Some(&idx) = self.func_indexes.get("__intr_arr_index_of") {
                            self.body.push(Instruction::Call(idx));
                        }
                        return Ok(true);
                    }
                    "includes" => {
                        self.emit_expression(&c.args[0])?;
                        self.elem_to_bits(&c.args[0], elem_ty)?;
                        self.body.push(Instruction::I64Const(elem_size));
                        if let Some(&idx) = self.func_indexes.get("__intr_arr_includes") {
                            self.body.push(Instruction::Call(idx));
                        }
                        return Ok(true);
                    }
                    "join" => {
                        self.emit_expression(&c.args[0])?;
                        self.body.push(Instruction::I64Const(elem_size));
                        let cls_t = self.array_elem_cls_type(&member.object)?;
                        self.body.push(Instruction::I64Const(arr_kind_code(&cls_t)));
                        if let Some(&idx) = self.func_indexes.get("__intr_arr_join") {
                            self.body.push(Instruction::Call(idx));
                        }
                        return Ok(true);
                    }
                    "map" => { self.emit_array_map(member, c, elem_ty, elem_size)?; return Ok(true); },
                    _ => return Err(self.unsupported_expr(&Expression::Call(c.clone()))),
                }
            }
            Type::Record(_, _) => {
                self.emit_expression(&member.object)?;
                match member.member.as_str() {
                    "has" => {
                        self.emit_expression(&c.args[0])?;
                        if let Some(&idx) = self.func_indexes.get("__intr_record_has") {
                            self.body.push(Instruction::Call(idx));
                        }
                        return Ok(true);
                    }
                    "keys" => {
                        if let Some(&idx) = self.func_indexes.get("__intr_record_keys") {
                            self.body.push(Instruction::Call(idx));
                        }
                        return Ok(true);
                    }
                    "values" => {
                        if let Some(&idx) = self.func_indexes.get("__intr_record_values") {
                            self.body.push(Instruction::Call(idx));
                        }
                        return Ok(true);
                    }
                    _ => return Err(self.unsupported_expr(&Expression::Call(c.clone()))),
                }
            }
            Type::Shape(_) => {
                // DEFAULT INVERTIDO: los shapes viven como hashmap en runtime ->
                // mismas operaciones que un record (record_has/keys/values).
                // Bug fix dev-2 (Fase 7): faltaba `self.emit_expression(&member.object)?;`
                // al inicio de la rama Shape. En la rama Record (linea 299) si se
                // emite el receiver; en Shape no, lo que dejaba el stack sin el
                // ptr del hashmap -> el call a __intr_record_has(ptr, key) tomaba
                // el primer i64 (la key) como ptr y el segundo (lo que fuera) como
                // key, o peor, sin argumentos. Sintoma: "type mismatch: expected
                // i64 but nothing on stack" en el offset del call.
                self.emit_expression(&member.object)?;
                match member.member.as_str() {
                    "has" => {
                        self.emit_expression(&c.args[0])?;
                        if let Some(&idx) = self.func_indexes.get("__intr_record_has") {
                            self.body.push(Instruction::Call(idx));
                        }
                        return Ok(true);
                    }
                    "keys" => {
                        self.body.push(Instruction::I64Const(0)); // kind dummy
                        if let Some(&idx) = self.func_indexes.get("__intr_record_keys") {
                            self.body.push(Instruction::Call(idx));
                        }
                        return Ok(true);
                    }
                    "values" => {
                        self.body.push(Instruction::I64Const(0)); // kind dummy
                        if let Some(&idx) = self.func_indexes.get("__intr_record_values") {
                            self.body.push(Instruction::Call(idx));
                        }
                        return Ok(true);
                    }
                    _ => return Err(self.unsupported_expr(&Expression::Call(c.clone()))),
                }
            }
            Type::Named(name, _) => {
                let name = match self.resolve_class_key(&name) {
                    Some(k) => k,
                    None => return Err(self.unsupported_expr(&Expression::Call(c.clone()))),
                };
                if let Some(info) = self.class_defs.get(name.as_str()) {
                    let method_slot = info
                        .methods
                        .iter()
                        .position(|m| *m == member.member)
                        .ok_or_else(|| {
                            crate::error::ClsError::compile_at(
                                &format!(
                                    "El método '{}' no existe en la clase '{}'",
                                    member.member, name
                                ),
                                &member.span,
                            )
                        })? as u32;
                    // Visibilidad del método: private/protected desde fuera -> error.
                    // Se resuelve subiendo por ancestors (un método puede venir
                    // del padre sin override).
                    let mut vis_cls = name.to_string();
                    let vis = loop {
                        if let Some(v) = self
                            .class_defs
                            .get(&vis_cls)
                            .and_then(|i| i.method_vis.get(&member.member))
                        {
                            break Some(*v);
                        }
                        match self
                            .class_defs
                            .get(&vis_cls)
                            .and_then(|i| i.parent.clone())
                        {
                            Some(p) => vis_cls = p,
                            None => break None,
                        }
                    };
                    if let Some(v) = vis {
                        self.check_method_access(&name, &member.member, v, &member.span)?;
                    }
                    // Método heredado sin override: buscar el índice en la clase
                    // que lo declara (no fallar con "Método sin tipo WASM").
                    let mut fn_cls = name.to_string();
                    let (ty, method_key) = loop {
                        let key = format!("{}::{}", fn_cls, member.member);
                        if let Some(t) = self.method_type_indexes.get(&key) {
                            break (*t, key);
                        }
                        match self
                            .class_defs
                            .get(&fn_cls)
                            .and_then(|i| i.parent.clone())
                        {
                            Some(p) => fn_cls = p,
                            None => {
                                return Err(crate::error::ClsError::compile_at(
                                    &format!(
                                        "El método '{}' no existe en la clase '{}'",
                                        member.member, name
                                    ),
                                    &member.span,
                                ))
                            }
                        }
                    };
                    // Params reales del método (incluye `me` como param 0) para
                    // ensanchar escalares en la frontera (`Value` es i64, bool
                    // i32 / float f64).
                    let expected = self.func_types.get(&method_key).map(|(p, _)| p.clone());
                    // `me` es el param 0 de los métodos de instancia.
                    let me_offset = if expected.is_some() { 1 } else { 0 };
                    let obj_tmp = self.fresh_local();
                    self.emit_expression(&member.object)?;
                    self.body.push(Instruction::LocalSet(obj_tmp));
                    self.body.push(Instruction::LocalGet(obj_tmp));
                    for (i, a) in c.args.iter().enumerate() {
                        self.emit_call_arg(a, expected.as_deref(), i + me_offset)?;
                    }
                    // slot = vtable(obj[0]) + method_slot
                    self.body.push(Instruction::LocalGet(obj_tmp));
                    self.body.push(Instruction::I32WrapI64);
                    self.body.push(Instruction::I64Load(MemArg {
                        offset: 0,
                        align: 3,
                        memory_index: 0,
                    }));
                    self.body.push(Instruction::I64Const(method_slot as i64));
                    self.body.push(Instruction::I64Add);
                    self.body.push(Instruction::I32WrapI64);
                    self.body.push(Instruction::CallIndirect {
                        type_index: ty,
                        table_index: 0,
                    });
                    return Ok(true);
                }
                return Err(self.unsupported_expr(&Expression::Call(c.clone())));
            }
            Type::Int => {
                self.emit_expression(&member.object)?;
                match member.member.as_str() {
                    "toString" => {
                        self.emit_str_host("__intr_str_int");
                        return Ok(true);
                    }
                    "abs" => {
                        if let Some(&idx) = self.func_indexes.get("__intr_int_abs") {
                            self.body.push(Instruction::Call(idx));
                        }
                        return Ok(true);
                    }
                    _ => return Err(self.unsupported_expr(&Expression::Call(c.clone()))),
                }
            }
            Type::Float => {
                self.emit_expression(&member.object)?;
                match member.member.as_str() {
                    "toString" => {
                        self.emit_str_host("__intr_str_float");
                        return Ok(true);
                    }
                    "abs" => {
                        self.body.push(Instruction::F64Abs);
                        return Ok(true);
                    }
                    _ => return Err(self.unsupported_expr(&Expression::Call(c.clone()))),
                }
            }
            Type::Bool => {
                self.emit_expression(&member.object)?;
                match member.member.as_str() {
                    "toString" => {
                        self.emit_str_host("__intr_str_bool");
                        return Ok(true);
                    }
                    _ => return Err(self.unsupported_expr(&Expression::Call(c.clone()))),
                }
            }
            Type::Char => {
                self.emit_expression(&member.object)?;
                match member.member.as_str() {
                    "toString" => {
                        self.emit_str_host("__intr_str_char");
                        return Ok(true);
                    }
                    _ => return Err(self.unsupported_expr(&Expression::Call(c.clone()))),
                }
            }
            // Unknown/Value/JSON (valor dinámico leído de un Record/JSON o
            // anotado `Value`/`JSON`): `x.toString()` -> string por TAG en
            // runtime (__intr_any_to_string), un solo eval del receiver. Otros
            // métodos sobre estos tipos no se despachan acá (caen al camino
            // dinámico de emit_call).
            Type::Value | Type::Unknown | Type::Json => {
                if member.member.as_str() == "toString" {
                    self.emit_any_chain(&member.object)?; // (val, tag)
                    self.emit_str_host("__intr_any_to_string");
                    return Ok(true);
                }
            }
            _ => {}
        }
    }
    if let Expression::Identifier(name, _) = &*c.callee {
        match name.as_str() {
            "throw" => {
                // throw(msg) -> excepción CLS (tag con payload msg + span).
                if !self.exceptions {
                    return Err(crate::error::ClsError::compile_at(
                        "'throw' no soportado en este runtime: el backend se compiló sin \
                         excepciones WASM (wasmi).",
                        &c.span,
                    ));
                }
                if let Some(arg0) = c.args.first() {
                    self.emit_expression(arg0)?;
                    self.emit_to_string(arg0)?;
                } else {
                    let s = self.intern_string("error");
                    self.emit_load_str(s);
                }
                let packed = ((c.span.start_line as i64) << 32) | (c.span.start_col as i64);
                self.body.push(Instruction::I64Const(packed));
                self.body.push(Instruction::Throw(self.tag_idx));
                return Ok(true);
            }
            "print" => {
                for arg in &c.args {
                    self.emit_print_arg(arg)?;
                }
                self.host.call(HostFn::PrintEnd, &mut self.body);
                return Ok(true);
            }
            "len" => {
                let arg = &c.args[0];
                // Magic __len: clase con __len -> call sin args (paridad walker).
                if self.emit_class_method("__len", arg)? {
                    return Ok(true);
                }
                self.emit_expression(arg)?;
                // String -> decodifica el pack (ptr<<32|len); array/tuple/record
                // -> lee el header. Despachar por el tipo del argumento.
                let t = self.types.get(&expr_span(arg)).cloned().unwrap_or(Type::Value);
                match t {
                    Type::String => {
                        self.emit_str_host("__intr_str_length");
                    }
                    Type::Record(_, _) | Type::Shape(_) => {
                        if let Some(&idx) = self.func_indexes.get("__intr_record_len") {
                            self.body.push(Instruction::Call(idx));
                        }
                    }
                    _ => self.emit_array_len(),
                }
                return Ok(true);
            }
            "toString" | "str" => {
                let arg = &c.args[0];
                let t = self
                    .types
                    .get(&expr_span(arg))
                    .cloned()
                    .unwrap_or(Type::Value);
                if t.is_tagged_builtin_union() {
                    self.emit_expression(arg)?;
                    let ptr = self.fresh_local_ty(WasTy::I64);
                    self.body.push(Instruction::LocalSet(ptr));
                    self.emit_tagged_union_raw_value(ptr);
                    self.emit_tagged_union_tag(ptr);
                    self.emit_str_host("__intr_any_to_string");
                    return Ok(true);
                }
                self.emit_expression(arg)?;
                self.emit_to_string(arg)?;
                return Ok(true);
            }
            "input" => {
                self.host.call(HostFn::Input, &mut self.body);
                return Ok(true);
            }
            "int" => {
                let arg = &c.args[0];
                // Magic __int: clase con __int -> call sin args (paridad walker).
                if self.emit_class_method("__int", arg)? {
                    return Ok(true);
                }
                self.emit_expression(arg)?;
                self.emit_to_int(arg)?;
                return Ok(true);
            }
            "float" => {
                let arg = &c.args[0];
                // Magic __float: clase con __float -> call sin args.
                if self.emit_class_method("__float", arg)? {
                    return Ok(true);
                }
                self.emit_expression(arg)?;
                self.emit_to_float(arg)?;
                return Ok(true);
            }
            "bool" => {
                let arg = &c.args[0];
                // Magic __bool: clase con __bool -> call sin args.
                if self.emit_class_method("__bool", arg)? {
                    return Ok(true);
                }
                self.emit_expression(arg)?;
                self.emit_to_bool(arg)?;
                return Ok(true);
            }
            "char" => {
                // `char(n) -> Char`: unidad de 1 byte (`n & 0xFF`). Trivial i32.
                self.emit_expression(&c.args[0])?;
                self.body.push(Instruction::I32WrapI64);
                self.body.push(Instruction::I32Const(0xff));
                self.body.push(Instruction::I32And);
                return Ok(true);
            }
            "ord" => {
                // `ord(c) -> int`: codigo del byte 0..255 de un Char.
                self.emit_expression(&c.args[0])?;
                self.body.push(Instruction::I32Const(0xff));
                self.body.push(Instruction::I32And);
                self.body.push(Instruction::I64ExtendI32U);
                return Ok(true);
            }
            "type" => {
                let arg = &c.args[0];
                // Si la clase define __type -> llamarla (paridad con el walker).
                if self.emit_class_method("__type", arg)? {
                    return Ok(true);
                }
                let span = expr_span(arg);
                let t = self.types.get(&span).cloned().unwrap_or(Type::Value);
                // type_name del walker: clase->"Object", struct->"Struct", enum->"Enum".
                let name = match &t {
                    Type::Named(cn, _) if self.class_defs.contains_key(cn.as_str()) => "Object",
                    Type::Named(cn, _) if self.struct_defs.contains_key(cn.as_str()) => {
                        "Struct"
                    }
                    Type::Named(cn, _) if self.enum_defs.contains_key(cn.as_str()) => "Enum",
                    Type::Named(_, _) => "Object",
                    _ => type_name_str(&t),
                };
                let idx = self.intern_string(name);
                self.emit_load_str(idx);
                return Ok(true);
            }
            "now" => {
                self.host.call(HostFn::Now, &mut self.body);
                return Ok(true);
            }
            // `config()` -> intrinsic de introspección (Record<String, String>
            // construido y cacheado por el host). Sin argumentos.
            "config" => {
                self.host.call(HostFn::ConfigRecord, &mut self.body);
                return Ok(true);
            }
            "exit" => {
                self.emit_expression(&c.args[0])?;
                self.host.call(HostFn::Exit, &mut self.body);
                return Ok(true);
            }
            "sleep" => {
                self.emit_expression(&c.args[0])?;
                self.host.call(HostFn::Sleep, &mut self.body);
                return Ok(true);
            }
            _ => {}
        }
    }
        Ok(false)
    }
}

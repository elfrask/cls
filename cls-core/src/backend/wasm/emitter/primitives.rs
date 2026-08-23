//! primitives.rs (Fase 1: extraido de emitter/calls.rs).

use super::*;

impl<'a> FuncEmitter<'a> {
    pub(crate) fn emit_primitive_method(&mut self, c: &CallExpr) -> ClsResult<bool> {
    // Métodos de primitivos (callee MemberAccess)
    if let Expression::MemberAccess(member) = &*c.callee {
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
                        if let Some(idx) = self.func_indexes.get(&key) {
                            self.body.push(Instruction::LocalGet(0)); // me
                            for a in &c.args {
                                self.emit_expression(a)?;
                            }
                            self.emit_call_site(&c.span);
                            self.body.push(Instruction::Call(*idx));
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
                        .unwrap_or(Type::Any);
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
                    if let Type::Shape(fields) = &t {
                        self.emit_shape_to_json_string(&c.args[0], fields)?; return Ok(true);
                    }
                    // Para `Any` (valor leído de un record/JSON): emitir con
                    // emit_any_chain (val + tag) y serializar por TAG real en
                    // runtime (host_json_stringify serializa escalares/record/
                    // array por tag). Sin esto, `json.stringify(d["int"])`
                    // devolvía el raw (puntero) -> str() lo leía como string.
                    if matches!(t, Type::Any | Type::Unknown) {
                        self.emit_any_chain(&c.args[0])?;   // (val, tag)
                        self.host.call(HostFn::JsonStringify, &mut self.body);
                        return Ok(true);
                    }
                    self.emit_expression(&c.args[0])?;
                    // kind = tag del RUNTIME del valor. Para tipos concretos
                    // (bool/int/float/string/char/record/array) el tag se conoce
                    // en compile-time y el host serializa por tag.
                    let kind = runtime_tag_code(&t);
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
            .unwrap_or(Type::Any);
        match obj_ty {
            Type::Tuple(_) => match member.member.as_str() {
                "join" => { self.emit_tuple_join(member, c)?; return Ok(true); },
                _ => return Err(self.unsupported_expr(&Expression::Call(c.clone()))),
            },
            Type::String => {
                self.emit_expression(&member.object)?;
                match member.member.as_str() {
                    "upper" | "lower" | "trim" => {
                        let (name, h) = match member.member.as_str() {
                            "upper" => ("__intr_str_upper", HostFn::StrUpper),
                            "lower" => ("__intr_str_lower", HostFn::StrLower),
                            _ => ("__intr_str_trim", HostFn::StrTrim),
                        };
                        self.emit_str_host(name, h);
                        return Ok(true);
                    }
                    "contains" | "startsWith" | "endsWith" => {
                        self.emit_expression(&c.args[0])?;
                        let (name, h) = match member.member.as_str() {
                            "contains" => ("__intr_str_contains", HostFn::StrContains),
                            "startsWith" => ("__intr_str_starts_with", HostFn::StrStartsWith),
                            _ => ("__intr_str_ends_with", HostFn::StrEndsWith),
                        };
                        self.emit_str_host(name, h);
                        return Ok(true);
                    }
                    "isEmpty" => {
                        self.emit_str_host("__intr_str_is_empty", HostFn::StrIsEmpty);
                        return Ok(true);
                    }
                    "toString" => { return Ok(true); },
                    _ => return Err(self.unsupported_expr(&Expression::Call(c.clone()))),
                }
            }
            Type::Array(_) => {
                let elem_ty = self.array_elem_was_type(&member.object)?;
                let elem_size = elem_size_bytes(elem_ty);
                self.emit_expression(&member.object)?;
                match member.member.as_str() {
                    "push" => {
                        self.emit_expression(&c.args[0])?;
                        self.elem_to_bits(&c.args[0], elem_ty)?;
                        self.body.push(Instruction::I64Const(elem_size));
                        if let Some(&idx) = self.func_indexes.get("__intr_arr_push") {
                            self.body.push(Instruction::Call(idx));
                        } else {
                            self.host.call(HostFn::ArrPush, &mut self.body);
                        }
                        self.writeback_array(&member.object)?;
                        return Ok(true);
                    }
                    "pop" => {
                        self.body.push(Instruction::I64Const(elem_size));
                        if let Some(&idx) = self.func_indexes.get("__intr_arr_pop") {
                            self.body.push(Instruction::Call(idx));
                        } else {
                            self.host.call(HostFn::ArrPop, &mut self.body);
                        }
                        self.writeback_array(&member.object)?;
                        return Ok(true);
                    }
                    "shift" => {
                        self.body.push(Instruction::I64Const(elem_size));
                        if let Some(&idx) = self.func_indexes.get("__intr_arr_shift") {
                            self.body.push(Instruction::Call(idx));
                        } else {
                            self.host.call(HostFn::ArrShift, &mut self.body);
                        }
                        self.writeback_array(&member.object)?;
                        return Ok(true);
                    }
                    "unshift" => {
                        self.emit_expression(&c.args[0])?;
                        self.elem_to_bits(&c.args[0], elem_ty)?;
                        self.body.push(Instruction::I64Const(elem_size));
                        if let Some(&idx) = self.func_indexes.get("__intr_arr_unshift") {
                            self.body.push(Instruction::Call(idx));
                        } else {
                            self.host.call(HostFn::ArrUnshift, &mut self.body);
                        }
                        self.writeback_array(&member.object)?;
                        return Ok(true);
                    }
                    "reverse" => {
                        self.body.push(Instruction::I64Const(elem_size));
                        if let Some(&idx) = self.func_indexes.get("__intr_arr_reverse") {
                            self.body.push(Instruction::Call(idx));
                        } else {
                            self.host.call(HostFn::ArrReverse, &mut self.body);
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
                        } else {
                            self.host.call(HostFn::ArrIndexOf, &mut self.body);
                        }
                        return Ok(true);
                    }
                    "includes" => {
                        self.emit_expression(&c.args[0])?;
                        self.elem_to_bits(&c.args[0], elem_ty)?;
                        self.body.push(Instruction::I64Const(elem_size));
                        if let Some(&idx) = self.func_indexes.get("__intr_arr_includes") {
                            self.body.push(Instruction::Call(idx));
                        } else {
                            self.host.call(HostFn::ArrIncludes, &mut self.body);
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
                        } else {
                            self.host.call(HostFn::ArrJoin, &mut self.body);
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
                        } else {
                            self.host.call(HostFn::RecordHas, &mut self.body);
                        }
                        return Ok(true);
                    }
                    "keys" => {
                        if let Some(&idx) = self.func_indexes.get("__intr_record_keys") {
                            self.body.push(Instruction::Call(idx));
                        } else {
                            self.host.call(HostFn::RecordKeys, &mut self.body);
                        }
                        return Ok(true);
                    }
                    "values" => {
                        if let Some(&idx) = self.func_indexes.get("__intr_record_values") {
                            self.body.push(Instruction::Call(idx));
                        } else {
                            self.host.call(HostFn::RecordValues, &mut self.body);
                        }
                        return Ok(true);
                    }
                    _ => return Err(self.unsupported_expr(&Expression::Call(c.clone()))),
                }
            }
            Type::Shape(fields) => {
                match member.member.as_str() {
                    "has" => {
                        // Compile-time: si la clave (literal) está en el shape.
                        let has = match &c.args[0] {
                            Expression::Literal(l)
                                if matches!(l.kind, LiteralKind::String(_)) =>
                            {
                                match &l.kind {
                                    LiteralKind::String(k) => {
                                        fields.iter().any(|(n, _)| *n == *k)
                                    }
                                    _ => false,
                                }
                            }
                            _ => true, // clave dinámica -> se asume que puede existir
                        };
                        self.body
                            .push(Instruction::I32Const(if has { 1 } else { 0 }));
                        return Ok(true);
                    }
                    "keys" => {
                        // Construir array<String> con las keys del shape.
                        let mut sorted: Vec<&String> = fields.iter().map(|(n, _)| n).collect();
                        sorted.sort();
                        let n = sorted.len() as i64;
                        let es = 8i64;
                        self.body.push(Instruction::I64Const(n));
                        self.body.push(Instruction::I64Const(es));
                        self.body.push(Instruction::I64Mul);
                        self.body.push(Instruction::I64Const(16));
                        self.body.push(Instruction::I64Add);
                        let alloc = self.func_indexes["__alloc"];
                        self.body.push(Instruction::Call(alloc));
                        let ptr = self.fresh_local();
                        self.body.push(Instruction::LocalSet(ptr));
                        self.body.push(Instruction::LocalGet(ptr));
                        self.body.push(Instruction::I64Const(n));
                        self.emit_i64_store(0);
                        self.body.push(Instruction::LocalGet(ptr));
                        self.body.push(Instruction::I64Const(n));
                        self.emit_i64_store(8);
                        for (i, k) in sorted.iter().enumerate() {
                            self.body.push(Instruction::LocalGet(ptr));
                            self.body.push(Instruction::I64Const(16 + (i as i64) * 8));
                            self.body.push(Instruction::I64Add);
                            self.body.push(Instruction::I32WrapI64);
                            let s = self.intern_string(k);
                            self.emit_load_str(s);
                            self.body.push(Instruction::I64Store(MemArg {
                                offset: 0,
                                align: 3,
                                memory_index: 0,
                            }));
                        }
                        self.body.push(Instruction::LocalGet(ptr));
                        return Ok(true);
                    }
                    "values" => {
                        // Construir array con los valores (según el tipo de cada campo).
                        self.emit_expression(&member.object)?;
                        let ptr = self.fresh_local();
                        self.body.push(Instruction::LocalSet(ptr));
                        let layout = self.shape_layout(&fields)?;
                        let mut ordered: Vec<&(String, WasTy, i64)> = layout.iter().collect();
                        ordered.sort_by(|a, b| a.0.cmp(&b.0));
                        let n = fields.len() as i64;
                        let es = 8i64;
                        self.body.push(Instruction::I64Const(n));
                        self.body.push(Instruction::I64Const(es));
                        self.body.push(Instruction::I64Mul);
                        self.body.push(Instruction::I64Const(16));
                        self.body.push(Instruction::I64Add);
                        let alloc = self.func_indexes["__alloc"];
                        self.body.push(Instruction::Call(alloc));
                        let arr = self.fresh_local();
                        self.body.push(Instruction::LocalSet(arr));
                        self.body.push(Instruction::LocalGet(arr));
                        self.body.push(Instruction::I64Const(n));
                        self.emit_i64_store(0);
                        self.body.push(Instruction::LocalGet(arr));
                        self.body.push(Instruction::I64Const(n));
                        self.emit_i64_store(8);
                        for (i, (_, w, off)) in ordered.iter().enumerate() {
                            self.body.push(Instruction::LocalGet(arr));
                            self.body.push(Instruction::I64Const(16 + (i as i64) * 8));
                            self.body.push(Instruction::I64Add);
                            self.body.push(Instruction::I32WrapI64);
                            self.body.push(Instruction::LocalGet(ptr));
                            self.body.push(Instruction::I64Const(*off));
                            self.body.push(Instruction::I64Add);
                            self.body.push(Instruction::I32WrapI64);
                            match *w {
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
                            // bits a i64 (f64 -> reinterpret; i32 -> extend)
                            match *w {
                                WasTy::F64 => self.body.push(Instruction::I64ReinterpretF64),
                                WasTy::I32 => self.body.push(Instruction::I64ExtendI32U),
                                WasTy::I64 => {}
                            }
                            self.body.push(Instruction::I64Store(MemArg {
                                offset: 0,
                                align: 3,
                                memory_index: 0,
                            }));
                        }
                        self.body.push(Instruction::LocalGet(arr));
                        return Ok(true);
                    }
                    _ => return Err(self.unsupported_expr(&Expression::Call(c.clone()))),
                }
            }
            Type::Named(name, _) => {
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
                    let ty = loop {
                        let key = format!("{}::{}", fn_cls, member.member);
                        if let Some(t) = self.method_type_indexes.get(&key) {
                            break *t;
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
                    let obj_tmp = self.fresh_local();
                    self.emit_expression(&member.object)?;
                    self.body.push(Instruction::LocalSet(obj_tmp));
                    self.body.push(Instruction::LocalGet(obj_tmp));
                    for a in &c.args {
                        self.emit_expression(a)?;
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
                        self.emit_str_host("__intr_str_int", HostFn::StrInt);
                        return Ok(true);
                    }
                    "abs" => {
                        if let Some(&idx) = self.func_indexes.get("__intr_int_abs") {
                            self.body.push(Instruction::Call(idx));
                        } else {
                            self.host.call(HostFn::IntAbs, &mut self.body);
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
                        self.emit_str_host("__intr_str_float", HostFn::StrFloat);
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
                        self.emit_str_host("__intr_str_bool", HostFn::StrBool);
                        return Ok(true);
                    }
                    _ => return Err(self.unsupported_expr(&Expression::Call(c.clone()))),
                }
            }
            Type::Char => {
                self.emit_expression(&member.object)?;
                match member.member.as_str() {
                    "toString" => {
                        self.emit_str_host("__intr_str_char", HostFn::StrChar);
                        return Ok(true);
                    }
                    _ => return Err(self.unsupported_expr(&Expression::Call(c.clone()))),
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
                let t = self.types.get(&expr_span(arg)).cloned().unwrap_or(Type::Any);
                match t {
                    Type::String => {
                        self.emit_str_host("__intr_str_length", HostFn::StrLength);
                    }
                    Type::Record(_, _) | Type::Shape(_) => {
                        if let Some(&idx) = self.func_indexes.get("__intr_record_len") {
                            self.body.push(Instruction::Call(idx));
                        } else {
                            self.host.call(HostFn::RecordLen, &mut self.body);
                        }
                    }
                    _ => self.emit_array_len(),
                }
                return Ok(true);
            }
            "toString" => {
                let arg = &c.args[0];
                self.emit_expression(arg)?;
                self.emit_to_string(arg)?;
                return Ok(true);
            }
            "str" => {
                let arg = &c.args[0];
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
            "type" => {
                let arg = &c.args[0];
                // Si la clase define __type -> llamarla (paridad con el walker).
                if self.emit_class_method("__type", arg)? {
                    return Ok(true);
                }
                let span = expr_span(arg);
                let t = self.types.get(&span).cloned().unwrap_or(Type::Any);
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

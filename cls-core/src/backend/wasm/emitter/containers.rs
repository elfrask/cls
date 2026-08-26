//! Containers: array/record/cmx/index access, writeback (Fase 1: extraido de emitter/mod.rs).

use super::*;

impl<'a> FuncEmitter<'a> {


    /// Carga un campo del CmxValue (tag/props/children) - el ptr está en el stack.
    pub(crate) fn emit_cmx_field(&mut self, offset: i64) -> ClsResult<()> {
        self.body.push(Instruction::I64Const(offset));
        self.body.push(Instruction::I64Add);
        self.body.push(Instruction::I32WrapI64);
        self.body.push(Instruction::I64Load(MemArg {
            offset: 0,
            align: 3,
            memory_index: 0,
        }));
        Ok(())
    }


    pub(crate) fn emit_array(&mut self, a: &ArrayExpr) -> ClsResult<()> {
        let elem_ty = self.array_elem_type(a)?;
        // Array de Cmx -> entradas `[val, tag]` stride 16 (children del Cmx, etc.).
        let is_cmx = a
            .elements
            .first()
            .and_then(|el| cmx_literal_type(el).or_else(|| self.types.get(&expr_span(el)).cloned()))
            .map(|t| matches!(t, Type::Cmx))
            .unwrap_or(false);
        let elem_size = if is_cmx { 16 } else { elem_size_bytes(elem_ty) };
        let n = a.elements.len() as i64;
        // layout: [cap:i64][len:i64][elem...] (base 16)
        self.body.push(Instruction::I64Const(n));
        self.body.push(Instruction::I64Const(elem_size));
        self.body.push(Instruction::I64Mul);
        self.body.push(Instruction::I64Const(16));
        self.body.push(Instruction::I64Add);
        let alloc = self.func_indexes["__alloc"];
        self.body.push(Instruction::Call(alloc));
        let ptr = self.fresh_local();
        self.body.push(Instruction::LocalSet(ptr));
        // cap (offset 0) y len (offset 8)
        self.body.push(Instruction::LocalGet(ptr));
        self.body.push(Instruction::I64Const(n));
        self.emit_i64_store(0);
        self.body.push(Instruction::LocalGet(ptr));
        self.body.push(Instruction::I64Const(n));
        self.emit_i64_store(8);
        // elementos
        for (i, el) in a.elements.iter().enumerate() {
            self.emit_expression(el)?;
            if is_cmx {
                // guardar [val, tag]
                let val_tmp = self.fresh_local_ty(elem_ty);
                self.body.push(Instruction::LocalSet(val_tmp));
                self.body.push(Instruction::LocalGet(ptr));
                self.body.push(Instruction::I64Const(16 + (i as i64) * 16));
                self.body.push(Instruction::I64Add);
                self.body.push(Instruction::I32WrapI64);
                self.body.push(Instruction::LocalGet(val_tmp));
                self.body.push(Instruction::I64Store(MemArg {
                    offset: 0,
                    align: 3,
                    memory_index: 0,
                }));
                self.body.push(Instruction::LocalGet(ptr));
                self.body
                    .push(Instruction::I64Const(16 + (i as i64) * 16 + 8));
                self.body.push(Instruction::I64Add);
                self.body.push(Instruction::I32WrapI64);
                let el_cls = cmx_literal_type(el)
                    .or_else(|| self.types.get(&expr_span(el)).cloned())
                    .unwrap_or(Type::Any);
                self.body
                    .push(Instruction::I64Const(cmx_tag_for_type(&el_cls)));
                self.body.push(Instruction::I64Store(MemArg {
                    offset: 0,
                    align: 3,
                    memory_index: 0,
                }));
            } else {
                let val_tmp = self.fresh_local_ty(elem_ty);
                let addr_tmp = self.fresh_local();
                // Si el array es f64 y el elemento es un literal/expresión int,
                // promoverlo a f64 para el store (layout homogéneo).
                if elem_ty == WasTy::F64 {
                    self.f64_promote(el)?;
                }
                self.body.push(Instruction::LocalSet(val_tmp));
                self.body.push(Instruction::LocalGet(ptr));
                self.body
                    .push(Instruction::I64Const(16 + (i as i64) * elem_size));
                self.body.push(Instruction::I64Add);
                self.body.push(Instruction::LocalSet(addr_tmp));
                self.body.push(Instruction::LocalGet(addr_tmp));
                self.body.push(Instruction::I32WrapI64);
                self.body.push(Instruction::LocalGet(val_tmp));
                match elem_ty {
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
            }
        }
        self.body.push(Instruction::LocalGet(ptr));
        Ok(())
    }


    pub(crate) fn array_elem_type(&self, a: &ArrayExpr) -> ClsResult<WasTy> {
        if let Some(first) = a.elements.first() {
            // Promoción: si CUALQUIER elemento es float, el array es de f64
            // (p.ej. `[1, 2.0]` -> f64). El store promueve los ints a f64.
            let has_float = a
                .elements
                .iter()
                .any(|el| matches!(self.value_type(el), Ok(WasTy::F64)));
            if has_float {
                return Ok(WasTy::F64);
            }
            return self.value_type(first);
        }
        // Array vacío: usar el tipo anotado registrado por el typeck (span del literal),
        // p.ej. `const out: int[] = []`.
        if let Some(Type::Array(elem)) = self.types.get(&a.span) {
            if let Ok(w) = was_type(elem) {
                return Ok(w);
            }
        }
        Err(crate::error::ClsError::compile_at(
            "Array literal vacío sin tipo: agrega la anotación del elemento (p.ej. `int[] = []`)",
            &a.span,
        ))
    }


    /// Literal de record `{ a: 1, b: "x" }` -> record_new + record_set.
    pub(crate) fn emit_record(&mut self, r: &RecordExpr) -> ClsResult<()> {
        // Si el type map dice Shape -> emitir como struct contiguo (offsets fijos).
        // Es el caso de `var x = {a: 1, b: "1"}` (inferido) o anotado con
        // interface/alias de shape. Sin hashmap, sin keys en memoria, sin tags.
        if let Some(shape) = self.types.get(&r.span).cloned() {
            if let Type::Shape(fields) = &shape {
                return self.emit_shape_record(r, fields);
            }
        }
        self.emit_record_hashmap(r)
    }

    /// Emite un literal de record como HASHMAP (`[cap][len][(key,val,tag)*24]`),
    /// sin importar si el type map dice Shape. Se usa cuando el literal es un
    /// VALOR que se guarda en un contenedor dinámico (`Record<String,any>`):
    /// un shape contiguo NO es legible como hashmap por la lectura/stringify.
    pub(crate) fn emit_record_hashmap(&mut self, r: &RecordExpr) -> ClsResult<()> {
        let n = r.entries.len() as i64;
        self.body.push(Instruction::I64Const(n));
        if let Some(&idx) = self.func_indexes.get("__intr_record_new") {
            self.body.push(Instruction::Call(idx));
        } else {
            self.host.call(HostFn::RecordNew, &mut self.body);
        }
        let ptr = self.fresh_local();
        self.body.push(Instruction::LocalSet(ptr));
        for (key, val) in &r.entries {
            let cls_t = self
                .types
                .get(&expr_span(val))
                .cloned()
                .unwrap_or(Type::Any);
            self.body.push(Instruction::LocalGet(ptr));
            let k = self.intern_string(key);
            self.emit_load_str(k);
            // Valor Shape anidado: convertir a hashmap (un ptr contiguo con tag 7
            // no es legible como record por la lectura/stringify).
            if let Type::Shape(fields) = &cls_t {
                self.emit_shape_to_hashmap(val, fields)?;
            } else {
                self.emit_expression(val)?;
            }
            match self.value_type(val)? {
                WasTy::F64 => self.body.push(Instruction::I64ReinterpretF64),
                WasTy::I32 => self.body.push(Instruction::I64ExtendI32U),
                WasTy::I64 => {}
            }
            let cls_t = self
                .types
                .get(&expr_span(val))
                .cloned()
                .unwrap_or(Type::Any);
            // Tag del valor en el record: tag del RUNTIME interno (Record -> 7,
            // Array -> 6, String -> 1...). Antes usaba arr_kind_code, que devolvía
            // 0 para records -> el binding los leía como int (ptr crudo).
            self.body.push(Instruction::I64Const(runtime_tag_code(&cls_t)));
            if let Some(&idx) = self.func_indexes.get("__intr_record_set") {
                self.body.push(Instruction::Call(idx));
            } else {
                self.host.call(HostFn::RecordSet, &mut self.body);
            }
            self.body.push(Instruction::Drop);
        }
        self.body.push(Instruction::LocalGet(ptr));
        Ok(())
    }


    /// Emite un record con shape como struct contiguo: `[campo0][campo1]...`.
    /// Los offsets se calculan del shape (cada campo con su WasTy).
    pub(crate) fn emit_shape_record(&mut self, r: &RecordExpr, fields: &[(String, Type)]) -> ClsResult<()> {
        let layout = self.shape_layout(fields)?;
        let mut total = 0i64;
        for (_, w, off) in &layout {
            total = *off + elem_size_bytes(*w);
        }
        self.body.push(Instruction::I64Const(total));
        let alloc = self.func_indexes["__alloc"];
        self.body.push(Instruction::Call(alloc));
        let ptr = self.fresh_local();
        self.body.push(Instruction::LocalSet(ptr));
        for (name, val) in &r.entries {
            let (_, w, off) = layout
                .iter()
                .find(|(n, _, _)| n == name)
                .cloned()
                .ok_or_else(|| {
                    crate::error::ClsError::compile_at(
                        &format!("El record no tiene el campo '{}' en su shape", name),
                        &r.span,
                    )
                })?;
            self.emit_expression(val)?;
            let val_tmp = self.fresh_local_ty(w);
            self.body.push(Instruction::LocalSet(val_tmp));
            self.body.push(Instruction::LocalGet(ptr));
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
        }
        self.body.push(Instruction::LocalGet(ptr));
        Ok(())
    }


    /// Convierte un SHAPE (estructura contigua con offsets) a un HASHMAP
    /// (`[cap][len][(key,val,tag)*24]`). Se usa cuando un valor tipado Shape se
    /// guarda en un contenedor dinámico (`Record<String,any>`): la lectura/
    /// stringify lo trata como hashmap, y un shape contiguo no es legible como
    /// tal. Evalúa `expr` (deja el ptr del shape) y lo copia campo a campo.
    /// ¿Es un destino que lee los valores como hashmap dinámico?
    pub(crate) fn is_dynamic_dest(t: &Type) -> bool {
        matches!(
            t,
            Type::Record(_, _) | Type::Json | Type::Value | Type::Any | Type::Unknown
        )
    }

    /// FRONTERA ÚNICA de valores: emite `expr` para ser consumido como `dest`.
    /// Si el valor es un shape contiguo y el destino es dinámico
    /// (Record/JSON/Value/Any/Unknown — o desconocido: calls dinámicos/métodos),
    /// lo convierte a HASHMAP recursivo. En cualquier otro caso emite directo.
    /// TODA transferencia de valor (args, return, assign, campos, elems,
    /// inicializadores anidados) debe pasar por aquí.
    pub(crate) fn emit_coerce(&mut self, expr: &Expression, dest: Option<&Type>) -> ClsResult<()> {
        let convert = match dest {
            Some(t) => Self::is_dynamic_dest(t),
            None => true,
        };
        if convert {
            if let Some(Type::Shape(fields)) = self.types.get(&expr_span(expr)).cloned() {
                return self.emit_shape_to_hashmap(expr, &fields);
            }
        }
        self.emit_expression(expr)
    }

    pub(crate) fn emit_shape_to_hashmap(&mut self, expr: &Expression, fields: &[(String, Type)]) -> ClsResult<()> {
        self.emit_expression(expr)?;
        let shape_ptr = self.fresh_local();
        self.body.push(Instruction::LocalSet(shape_ptr));
        self.shape_to_hashmap_from_local(shape_ptr, fields)?;
        Ok(())
    }

    /// Convierte el shape contiguo apuntado por `shape_ptr` a hashmap (record
    /// `[cap][len][(key,val,tag)*24]`) y deja el ptr del record en el stack.
    /// RECURSIVO: los campos cuyo tipo es un Shape anidado se convierten también
    /// (un ptr contiguo guardado con tag 7 no es legible como record).
    fn shape_to_hashmap_from_local(
        &mut self,
        shape_ptr: u32,
        fields: &[(String, Type)],
    ) -> ClsResult<()> {
        let layout = self.shape_layout(fields)?;
        // record_new(cap = n)
        let n = fields.len() as i64;
        self.body.push(Instruction::I64Const(n));
        if let Some(&idx) = self.func_indexes.get("__intr_record_new") {
            self.body.push(Instruction::Call(idx));
        } else {
            self.host.call(HostFn::RecordNew, &mut self.body);
        }
        let rec_ptr = self.fresh_local();
        self.body.push(Instruction::LocalSet(rec_ptr));
        for (name, t) in fields {
            let (_, w, off) = layout
                .iter()
                .find(|(nm, _, _)| nm == name)
                .cloned()
                .unwrap();
            // key = nombre del campo (string del pool)
            let k = self.intern_string(name);
            self.body.push(Instruction::LocalGet(rec_ptr));
            self.emit_load_str(k);
            // val = shape[off]
            self.body.push(Instruction::LocalGet(shape_ptr));
            self.body.push(Instruction::I64Const(off));
            self.body.push(Instruction::I64Add);
            self.body.push(Instruction::I32WrapI64);
            match w {
                WasTy::F64 => self.body.push(Instruction::F64Load(MemArg { offset: 0, align: 3, memory_index: 0 })),
                WasTy::I32 => self.body.push(Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 })),
                WasTy::I64 => self.body.push(Instruction::I64Load(MemArg { offset: 0, align: 3, memory_index: 0 })),
            }
            // Normalizar a i64 (float -> bits; bool/char -> extender)
            match w {
                WasTy::F64 => self.body.push(Instruction::I64ReinterpretF64),
                WasTy::I32 => self.body.push(Instruction::I64ExtendI32U),
                WasTy::I64 => {}
            }
            // Campo Shape anidado: convertir recursivamente antes de guardar.
            if let Type::Shape(nested) = t {
                let nested_ptr = self.fresh_local();
                self.body.push(Instruction::LocalSet(nested_ptr));
                self.shape_to_hashmap_from_local(nested_ptr, nested)?;
                let conv_tmp = self.fresh_local();
                self.body.push(Instruction::LocalSet(conv_tmp));
                self.body.push(Instruction::LocalGet(rec_ptr));
                self.emit_load_str(k);
                self.body.push(Instruction::LocalGet(conv_tmp));
            }
            self.body.push(Instruction::I64Const(runtime_tag_code(t)));
            if let Some(&idx) = self.func_indexes.get("__intr_record_set") {
                self.body.push(Instruction::Call(idx));
            } else {
                self.host.call(HostFn::RecordSet, &mut self.body);
            }
            self.body.push(Instruction::Drop);
        }
        self.body.push(Instruction::LocalGet(rec_ptr));
        Ok(())
    }

    /// Calcula `(nombre, WasTy, offset)` para cada campo de un shape (contiguo).
    pub(crate) fn shape_layout(&self, fields: &[(String, Type)]) -> ClsResult<Vec<(String, WasTy, i64)>> {
        let mut out = Vec::with_capacity(fields.len());
        let mut off = 0i64;
        for (name, t) in fields {
            let w = was_type(t)?;
            out.push((name.clone(), w, off));
            off += elem_size_bytes(w);
        }
        Ok(out)
    }


    /// Construye un `CmxValue` en memoria: [tag][props_ptr][children_ptr].
    pub(crate) fn emit_cmx(&mut self, c: &CmxElement) -> ClsResult<()> {
        // tag mayúscula -> resolver la variable/valor SIEMPRE (debe existir; si no, error).
        // tag minúscula -> String.
        if c.tag.starts_with(|ch: char| ch.is_uppercase()) {
            let name = c.tag.clone();
            if self.globals.contains_key(&name) || self.locals.contains_key(&name) {
                self.emit_ident_load(&name);
            } else if self.fn_table_idx.contains_key(&name) {
                // Función como tag -> handle de función (tag-bit) para que
                // `app.tag` sea invocable y se imprima `<function X>` (paridad walker).
                let ti = self.fn_table_idx[&name];
                let n = self.intern_string(&format!("<function {}>", name));
                self.body.push(Instruction::I64Const(ti as i64));
                self.emit_load_str(n);
                self.body.push(Instruction::I64Const(0));
                self.host.call(HostFn::FnHandle, &mut self.body);
            } else {
                return Err(crate::error::ClsError::CompileError(format!(
                    "El tag '<{}>' usa mayúscula pero '{}' no está definido: \
                     los tags con inicial mayúscula deben ser una función/valor existente",
                    c.tag, name
                )));
            }
        } else {
            let t = self.intern_string(&c.tag);
            self.emit_load_str(t);
        }
        self.body.push(Instruction::I64Const(0)); // kind=0 -> elemento
        self.host.call(HostFn::CmxNew, &mut self.body);
        let ptr = self.fresh_local();
        self.body.push(Instruction::LocalSet(ptr));
        for attr in &c.attributes {
            self.body.push(Instruction::LocalGet(ptr));
            let k = self.intern_string(&attr.name);
            self.emit_load_str(k);
            let val_type: Option<Type> = match &attr.value {
                Some(CmxAttributeValue::String(s)) => {
                    let s = self.intern_string(s);
                    self.emit_load_str(s);
                    Some(Type::String)
                }
                Some(CmxAttributeValue::Expression(expr)) => {
                    self.emit_expression(expr)?;
                    // Literales -> su tipo real (el type map puede dar Any).
                    cmx_literal_type(expr).or_else(|| self.types.get(&expr_span(expr)).cloned())
                }
                Some(CmxAttributeValue::Shorthand(name)) => {
                    self.emit_ident_load(name);
                    None
                }
                None => {
                    self.body.push(Instruction::I64Const(1));
                    Some(Type::Bool)
                }
            };
            let was = match &val_type {
                Some(t) => was_type(t).unwrap_or(WasTy::I64),
                None => WasTy::I64,
            };
            match was {
                WasTy::F64 => self.body.push(Instruction::I64ReinterpretF64),
                WasTy::I32 => self.body.push(Instruction::I64ExtendI32U),
                WasTy::I64 => {}
            }
            let cls = val_type.unwrap_or(Type::Any);
            self.body
                .push(Instruction::I64Const(cmx_tag_for_type(&cls)));
            self.host.call(HostFn::CmxSetProp, &mut self.body);
            self.body.push(Instruction::Drop);
        }
        for child in &c.children {
            self.body.push(Instruction::LocalGet(ptr));
            match child {
                CmxChild::Text(s) => {
                    // Texto -> CmxValue de texto (kind=1): el print lo muestra plano.
                    let s = self.intern_string(s);
                    self.emit_load_str(s);
                    self.body.push(Instruction::I64Const(1));
                    self.host.call(HostFn::CmxNew, &mut self.body);
                    self.body.push(Instruction::I64Const(5 << 8));
                }
                CmxChild::Expression(expr) => {
                    self.emit_expression(expr)?;
                    let t = cmx_literal_type(expr)
                        .or_else(|| self.types.get(&expr_span(expr)).cloned())
                        .unwrap_or(Type::Any);
                    self.body.push(Instruction::I64Const(cmx_tag_for_type(&t)));
                }
                CmxChild::Element(el) => {
                    self.emit_cmx(el)?;
                    self.body.push(Instruction::I64Const(5 << 8));
                }
            }
            self.host.call(HostFn::CmxAddChild, &mut self.body);
            self.body.push(Instruction::Drop);
        }
        self.body.push(Instruction::LocalGet(ptr));
        Ok(())
    }


    pub(crate) fn emit_index_get(&mut self, i: &IndexExpr) -> ClsResult<()> {
        // Record: r["key"] -> record_get(ptr, key)
        let obj_ty = self.types.get(&expr_span(&i.object)).cloned();
        // `o.x[0]` con `o.x` Any/Value/JSON (json.parse anidado): indexar despachando por tag.
        if matches!(obj_ty, Some(Type::Any) | Some(Type::Json) | Some(Type::Value)) {
            let expr = Expression::Index(i.clone());
            self.emit_any_chain(&expr)?;
            // Resultado (val, tag) -> dejar solo el val.
            self.body.push(Instruction::Drop);
            return Ok(());
        }
        // Magic __get: clase con __get -> obj.__get(index) (paridad walker:
        // "Indexado no soportado en objeto (falta __get)" si no lo define).
        if let Some(cn) = self.class_magic_method(&obj_ty, "__get") {
            let _ = self.magic_ret_was(&cn, "__get")?;
            self.emit_class_method_args("__get", &i.object, &[(*i.index).clone()])?;
            return Ok(());
        }
        if matches!(obj_ty, Some(Type::Record(_, _))) {
            self.emit_expression(&i.object)?;
            self.emit_expression(&i.index)?;
            // Índice numérico sobre un Record (p.ej. `json.parse("[..]")[j]`
            // que se representa como record con claves "0","1",...): convertir
            // el int a string antes de record_get (la key es string).
            if matches!(
                self.types.get(&expr_span(&i.index)),
                Some(
                    Type::Int
                        | Type::I8
                        | Type::I16
                        | Type::I32
                        | Type::I64
                        | Type::Literal(LitVal::Int(_))
                )
            ) {
                self.emit_str_host("__intr_str_int", HostFn::StrInt);
            }
            if let Some(&idx) = self.func_indexes.get("__intr_record_get") {
                self.body.push(Instruction::Call(idx));
            } else {
                self.host.call(HostFn::RecordGet, &mut self.body);
            }
            let elem_ty = self.index_elem_type(i)?;
            self.bits_to_elem(elem_ty)?;
            return Ok(());
        }
        // Shape: r["campo"] con clave literal -> load por offset (como member access).
        if let Some(Type::Shape(fields)) = &obj_ty {
            if let Expression::Literal(l) = &*i.index {
                if let LiteralKind::String(key) = &l.kind {
                    let (_, w, off) = self
                        .shape_layout(fields)?
                        .into_iter()
                        .find(|(n, _, _)| n == key)
                        .ok_or_else(|| {
                            crate::error::ClsError::compile_at(
                                &format!("El record no tiene el campo '{}'", key),
                                &i.span,
                            )
                        })?;
                    self.emit_expression(&i.object)?;
                    self.body.push(Instruction::I64Const(off));
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
                    return Ok(());
                }
            }
            return Err(crate::error::ClsError::compile_at(
                "Índice dinámico no soportado en un record con shape (usa Record<K,V> o any)",
                &i.span,
            ));
        }
        let elem_ty = self.index_elem_type(i)?;
        self.emit_expression(&i.object)?;
        self.emit_expression(&i.index)?;
        // Array de Cmx -> entradas `[val, tag]` stride 16 (children del Cmx, etc.).
        let is_cmx = matches!(&obj_ty, Some(Type::Array(e)) if matches!(**e, Type::Cmx));
        let elem_size = if is_cmx {
            16
        } else {
            self.container_elem_size(i, elem_ty)
        };
        self.emit_index_access(elem_ty, elem_size, i)
    }


    /// Asume [ptr, idx] en stack; deja el valor del elemento (con bounds check).
    pub(crate) fn emit_index_access(
        &mut self,
        elem_ty: WasTy,
        elem_size: i64,
        i: &IndexExpr,
    ) -> ClsResult<()> {
        let ptr = self.fresh_local();
        let idx = self.fresh_local();
        self.body.push(Instruction::LocalSet(idx));
        self.body.push(Instruction::LocalSet(ptr));
        // bounds check
        self.bounds_check(ptr, idx, &i.span);
        // addr = ptr + 16 + idx*elem_size
        self.body.push(Instruction::LocalGet(ptr));
        self.body.push(Instruction::LocalGet(idx));
        self.body.push(Instruction::I64Const(elem_size));
        self.body.push(Instruction::I64Mul);
        self.body.push(Instruction::I64Const(16));
        self.body.push(Instruction::I64Add);
        self.body.push(Instruction::I64Add);
        self.body.push(Instruction::I32WrapI64);
        match elem_ty {
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
        Ok(())
    }


    /// Emite el check `0 <= idx < len[ptr]`, trap si falla. Usa locals.
    pub(crate) fn bounds_check(&mut self, ptr: u32, idx: u32, span: &Span) {
        self.body.push(Instruction::LocalGet(idx));
        self.body.push(Instruction::I64Const(0));
        self.body.push(Instruction::I64LtS);
        self.body.push(Instruction::LocalGet(idx));
        self.body.push(Instruction::LocalGet(ptr));
        self.body.push(Instruction::I32WrapI64);
        self.body.push(Instruction::I64Load(MemArg {
            offset: 8,
            align: 3,
            memory_index: 0,
        }));
        self.body.push(Instruction::I64GeS);
        self.body.push(Instruction::I32Or);
        self.block_depth += 1;
        self.body.push(Instruction::If(BlockType::Empty));
        self.emit_throw("Índice fuera de rango", span);
        self.body.push(Instruction::Unreachable);
        self.body.push(Instruction::End);
        self.block_depth -= 1;
    }


    pub(crate) fn index_elem_type(&self, i: &IndexExpr) -> ClsResult<WasTy> {
        let span = expr_span(&i.object);
        let t = self.types.get(&span).ok_or_else(|| {
            crate::error::ClsError::CompileError("Index object sin tipo".to_string())
        })?;
        match t {
            Type::Array(elem) => was_type(elem),
            Type::Record(_, v) => was_type(v),
            Type::Tuple(slots) => {
                // índice literal → slot exacto; dinámico → primer slot (o i64)
                match &*i.index {
                    Expression::Literal(l) => match &l.kind {
                        LiteralKind::Int(v) if *v >= 0 && (*v as usize) < slots.len() => {
                            was_type(&slots[*v as usize])
                        }
                        _ => Ok(WasTy::I64),
                    },
                    _ => match slots.first() {
                        Some(s) => was_type(s),
                        None => Ok(WasTy::I64),
                    },
                }
            }
            other => Err(crate::error::ClsError::CompileError(format!(
                "Indexado sobre '{}' no soportado",
                other
            ))),
        }
    }


    /// Tamaño de slot de un contenedor: tuplas usan slots de 8 bytes; arrays el
    /// tamaño del tipo del elemento.
    pub(crate) fn container_elem_size(&self, i: &IndexExpr, elem_ty: WasTy) -> i64 {
        let span = expr_span(&i.object);
        match self.types.get(&span) {
            Some(Type::Tuple(_)) => 8,
            _ => elem_size_bytes(elem_ty),
        }
    }


    /// Asume [arr_ptr, idx, value] en stack. Escribe el valor.
    pub(crate) fn emit_index_set(&mut self, i: &IndexExpr, elem_size: i64) -> ClsResult<()> {
        let elem_ty = self.index_elem_type(i)?;
        let value = self.fresh_local_ty(elem_ty);
        let idx = self.fresh_local();
        let ptr = self.fresh_local();
        self.body.push(Instruction::LocalSet(value));
        self.body.push(Instruction::LocalSet(idx));
        self.body.push(Instruction::LocalSet(ptr));
        self.bounds_check(ptr, idx, &i.span);
        let addr_tmp = self.fresh_local();
        self.body.push(Instruction::LocalGet(ptr));
        self.body.push(Instruction::LocalGet(idx));
        self.body.push(Instruction::I64Const(elem_size));
        self.body.push(Instruction::I64Mul);
        self.body.push(Instruction::I64Const(16));
        self.body.push(Instruction::I64Add);
        self.body.push(Instruction::I64Add);
        self.body.push(Instruction::LocalSet(addr_tmp));
        self.body.push(Instruction::LocalGet(addr_tmp));
        self.body.push(Instruction::I32WrapI64);
        self.body.push(Instruction::LocalGet(value));
        match elem_ty {
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
        Ok(())
    }


    pub(crate) fn emit_array_len(&mut self) {
        // ptr está en stack -> len = i64.load(ptr+8)
        self.body.push(Instruction::I32WrapI64);
        self.body.push(Instruction::I64Load(MemArg {
            offset: 8,
            align: 3,
            memory_index: 0,
        }));
    }


    /// Tipo WASM del elemento de un array (del type map del object).
    pub(crate) fn array_elem_was_type(&self, obj: &Expression) -> ClsResult<WasTy> {
        let span = expr_span(obj);
        match self.types.get(&span) {
            Some(Type::Array(elem)) => was_type(elem),
            _ => Err(crate::error::ClsError::CompileError(
                "El objeto de la llamada no es un array".to_string(),
            )),
        }
    }


    /// Tipo CLS del elemento de un array.
    pub(crate) fn array_elem_cls_type(&self, obj: &Expression) -> ClsResult<Type> {
        let span = expr_span(obj);
        match self.types.get(&span) {
            Some(Type::Array(elem)) => Ok((**elem).clone()),
            _ => Err(crate::error::ClsError::CompileError(
                "El objeto de la llamada no es un array".to_string(),
            )),
        }
    }


    /// Convierte el valor en stack (del elem type) a i64 bits (para los hosts).
    pub(crate) fn elem_to_bits(&mut self, _arg: &Expression, elem_ty: WasTy) -> ClsResult<()> {
        match elem_ty {
            WasTy::F64 => self.body.push(Instruction::I64ReinterpretF64),
            WasTy::I32 => self.body.push(Instruction::I64ExtendI32U),
            WasTy::I64 => {}
        }
        Ok(())
    }


    /// Convierte i64 bits (del host) al valor del elem type.
    pub(crate) fn bits_to_elem(&mut self, elem_ty: WasTy) -> ClsResult<()> {
        match elem_ty {
            WasTy::F64 => self.body.push(Instruction::F64ReinterpretI64),
            WasTy::I32 => {}
            WasTy::I64 => {}
        }
        Ok(())
    }


    /// Escribe de vuelta el ptr mutado (resultado de push/unshift/reverse) a la
    /// variable y deja el valor como resultado (para `drop` del statement).
    pub(crate) fn writeback_array(&mut self, obj: &Expression) -> ClsResult<()> {
        if let Expression::Identifier(name, _) = obj {
            self.emit_ident_store(name);
            self.emit_ident_load(name);
            return Ok(());
        }
        // `me.items.push(...)` / `obj.items.push(...)`: el array pudo
        // reallocarse -> re-escribir el ptr en el campo (y dejar el ptr como
        // valor del receiver, paridad con el path de identifiers).
        if let Expression::MemberAccess(m) = obj {
            let obj_ty = self.types.get(&expr_span(&m.object)).cloned();
            if let Some(Type::Named(cls, _)) = obj_ty {
                if let Some(info) = self.class_defs.get(cls.as_str()) {
                    if let Some((_, _t, w, off, _vis)) =
                        info.fields.iter().find(|(n, _, _, _, _)| n == &m.member)
                    {
                        if *w == WasTy::I64 {
                            // El store espera `[addr, value]` (value al tope):
                            // guardar el ptr del array (ya en el stack) y
                            // pushear addr abajo, luego el value.
                            let arr_tmp = self.fresh_local();
                            self.body.push(Instruction::LocalSet(arr_tmp));
                            self.emit_expression(&m.object)?;
                            self.body.push(Instruction::I64Const(*off));
                            self.body.push(Instruction::I64Add);
                            self.body.push(Instruction::I32WrapI64);
                            self.body.push(Instruction::LocalGet(arr_tmp));
                            self.body.push(Instruction::I64Store(MemArg {
                                offset: 0,
                                align: 3,
                                memory_index: 0,
                            }));
                            // Valor del receiver = el ptr del array (reallocado).
                            self.emit_expression(&m.object)?;
                            self.body.push(Instruction::I64Const(*off));
                            self.body.push(Instruction::I64Add);
                            self.body.push(Instruction::I32WrapI64);
                            self.body.push(Instruction::I64Load(MemArg {
                                offset: 0,
                                align: 3,
                                memory_index: 0,
                            }));
                            return Ok(());
                        }
                    }
                }
            }
        }
        Ok(())
    }


    pub(crate) fn emit_i64_store(&mut self, offset: u32) {
        // stack: [addr(i64), value] -> reordenar con wrap
        let v = self.fresh_local();
        self.body.push(Instruction::LocalSet(v));
        self.body.push(Instruction::I32WrapI64);
        self.body.push(Instruction::LocalGet(v));
        self.body.push(Instruction::I64Store(MemArg {
            offset: offset as u64,
            align: 3,
            memory_index: 0,
        }));
    }

}
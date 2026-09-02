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
        let has_spread = a.elements.iter().any(|e| matches!(e, Expression::Spread(_, _)));
        let elem_ty = self.array_elem_type(a)?;
        // Array de Cmx -> entradas `[val, tag]` stride 16 (children del Cmx, etc.).
        let is_cmx = a
            .elements
            .first()
            .and_then(|el| cmx_literal_type(el).or_else(|| self.types.get(&expr_span(el)).cloned()))
            .map(|t| matches!(t, Type::Cmx))
            .unwrap_or(false);
        let elem_size = if is_cmx { 16 } else { elem_size_bytes(elem_ty) };
        // Spread `[...arr, x]`: el tamaño final es dinámico -> construir con
        // `__intr_arr_push` (maneja realloc) en vez de pre-alocar (REST_SPREAD_PLAN).
        if has_spread {
            if is_cmx {
                return Err(self.unsupported_expr(&Expression::Array(a.clone())));
            }
            // Empezar con un array de capacidad 0 (solo el header 16 bytes).
            self.body.push(Instruction::I64Const(0));
            self.body.push(Instruction::I64Const(elem_size));
            self.body.push(Instruction::I64Mul);
            self.body.push(Instruction::I64Const(16));
            self.body.push(Instruction::I64Add);
            let alloc = self.func_indexes["__alloc"];
            self.body.push(Instruction::Call(alloc));
            let ptr = self.fresh_local();
            self.body.push(Instruction::LocalSet(ptr));
            self.body.push(Instruction::LocalGet(ptr));
            self.body.push(Instruction::I64Const(0));
            self.emit_i64_store(0); // cap
            self.body.push(Instruction::LocalGet(ptr));
            self.body.push(Instruction::I64Const(0));
            self.emit_i64_store(8); // len
            for el in &a.elements {
                if let Expression::Spread(inner, _) = el {
                    // Copiar cada elemento del array inner al nuevo (push).
                    self.emit_expression(inner)?;
                    let src_ptr = self.fresh_local();
                    self.body.push(Instruction::LocalSet(src_ptr));
                    let src_len = self.fresh_local();
                    self.body.push(Instruction::LocalGet(src_ptr));
                    self.body.push(Instruction::I64Const(8));
                    self.body.push(Instruction::I64Add);
                    self.body.push(Instruction::I32WrapI64);
                    self.body.push(Instruction::I64Load(MemArg {
                        offset: 0,
                        align: 3,
                        memory_index: 0,
                    }));
                    self.body.push(Instruction::LocalSet(src_len));
                    let i = self.fresh_local();
                    self.body.push(Instruction::I64Const(0));
                    self.body.push(Instruction::LocalSet(i));
                    let d = self.block_depth;
                    self.block_depth += 1;
                    self.body.push(Instruction::Block(BlockType::Empty));
                    let break_at = self.block_depth;
                    self.block_depth += 1;
                    self.body.push(Instruction::Loop(BlockType::Empty));
                    // cond: i >= src_len -> break
                    self.body.push(Instruction::LocalGet(i));
                    self.body.push(Instruction::LocalGet(src_len));
                    self.body.push(Instruction::I64GeS);
                    self.body.push(Instruction::BrIf(break_at));
                    // val = src[i]
                    self.body.push(Instruction::LocalGet(src_ptr));
                    self.body.push(Instruction::LocalGet(i));
                    self.body.push(Instruction::I64Const(elem_size));
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
                    // push(dst, val, es): guardar val en local, luego [ptr, val, es].
                    let val_tmp = self.fresh_local();
                    self.body.push(Instruction::LocalSet(val_tmp));
                    self.body.push(Instruction::LocalGet(ptr));
                    self.body.push(Instruction::LocalGet(val_tmp));
                    self.body.push(Instruction::I64Const(elem_size));
                    if let Some(&idx) = self.func_indexes.get("__intr_arr_push") {
                        self.body.push(Instruction::Call(idx));
                    }
                    self.body.push(Instruction::LocalSet(ptr)); // write-back (realloc)
                    // i++
                    self.body.push(Instruction::LocalGet(i));
                    self.body.push(Instruction::I64Const(1));
                    self.body.push(Instruction::I64Add);
                    self.body.push(Instruction::LocalSet(i));
                    self.body.push(Instruction::Br(0)); // continuar loop
                    self.body.push(Instruction::End); // loop
                    self.block_depth -= 1;
                    self.body.push(Instruction::End); // block
                    self.block_depth = d;
                } else {
                    // Elemento normal: push(dst, val, es) — primero el ptr,
                    // luego el valor (el stack del push es [ptr, val, es]).
                    self.body.push(Instruction::LocalGet(ptr));
                    self.emit_expression(el)?;
                    if elem_ty == WasTy::F64 {
                        self.f64_promote(el)?;
                    }
                    self.body.push(Instruction::I64Const(elem_size));
                    if let Some(&idx) = self.func_indexes.get("__intr_arr_push") {
                        self.body.push(Instruction::Call(idx));
                    }
                    self.body.push(Instruction::LocalSet(ptr));
                }
            }
            self.body.push(Instruction::LocalGet(ptr));
            return Ok(());
        }
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
        // Los spreads `[...arr]` no aportan tipo directo (el inner es Array<T>);
        // inferir de los elementos normales.
        let normals: Vec<&Expression> = a
            .elements
            .iter()
            .filter(|e| !matches!(e, Expression::Spread(_, _)))
            .collect();
        if let Some(first) = normals.first() {
            // Promoción: si CUALQUIER elemento es float, el array es de f64
            // (p.ej. `[1, 2.0]` -> f64). El store promueve los ints a f64.
            let has_float = normals
                .iter()
                .any(|el| matches!(self.value_type(el), Ok(WasTy::F64)));
            if has_float {
                return Ok(WasTy::F64);
            }
            return self.value_type(first);
        }
        // Solo spreads (`[...a]`) o array vacío: usar el tipo anotado registrado
        // por el typeck (span del literal), p.ej. `const out: int[] = []`.
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
        // DEFAULT INVERTIDO (dev-3 paso 3): los record literals se emiten SIEMPRE
        // como hashmap `[cap][len][(key,val,tag)*24]`. El layout contiguo queda
        // reservado a las estructuras nombradas (`structure`) e instancias de
        // clase, donde los offsets son parte del contrato (FFI CStruct, campos).
        // Asi ningun valor `{...}` puede cruzar una frontera dinamica con un
        // layout ilegible.
        self.emit_record_hashmap(r)
    }

    /// Emite un literal de record como HASHMAP (`[cap][len][(key,val,tag)*24]`),
    /// sin importar si el type map dice Shape. Se usa cuando el literal es un
    /// VALOR que se guarda en un contenedor dinámico (`Record<String,any>`):
    /// un shape contiguo NO es legible como hashmap por la lectura/stringify.
    pub(crate) fn emit_record_hashmap(&mut self, r: &RecordExpr) -> ClsResult<()> {
        let n = (r.entries.len() + r.spreads.len()) as i64;
        self.body.push(Instruction::I64Const(n));
        if let Some(&idx) = self.func_indexes.get("__intr_record_new") {
            self.body.push(Instruction::Call(idx));
        }
        let ptr = self.fresh_local();
        self.body.push(Instruction::LocalSet(ptr));
        // Spreads `{...expr, ...}`: evaluar cada uno (deja ptr de record) y
        // mergear sus campos al nuevo record (REST_SPREAD_PLAN Fase 2).
        // merge(dst, src): dst = ptr nuevo (LocalGet), src = expr del spread.
        for spread in &r.spreads {
            self.body.push(Instruction::LocalGet(ptr));
            self.emit_expression(spread)?;
            if let Some(&idx) = self.func_indexes.get("__intr_record_merge") {
                self.body.push(Instruction::Call(idx));
            }
            // Write-back: merge puede reallocar (copiar N campos al nuevo).
            self.body.push(Instruction::LocalSet(ptr));
        }
        for (key, val) in &r.entries {
            self.body.push(Instruction::LocalGet(ptr));
            let k = self.intern_string(key);
            self.emit_load_str(k);
            // Valor anidado: los literals viven como hashmap (default invertido),
            // así que la emisión recursiva ya produce un record legible.
            self.emit_expression(val)?;
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
            // Tag del valor en el record: para ARRAYS debe ser COMPUESTO
            // (6<<8 | kind del elem): el formateador (`fmt_val_to_string`)
            // interpreta kind=6 como "array de Cmx" (es=16) y lee ints a saltos
            // de 16 -> basura (bug dev-2: `celulas: [1,2,3]` como floats/ptr).
            self.body.push(Instruction::I64Const(runtime_tag_code_compound(&cls_t)));
            if let Some(&idx) = self.func_indexes.get("__intr_record_set") {
                self.body.push(Instruction::Call(idx));
            }
            // Write-back: record_set puede reallocar si el capacity se excede
            // (p. ej. con spreads que ya llenaron el record). El ptr retornado
            // reemplaza al local para que el siguiente set/lectura use el nuevo.
            self.body.push(Instruction::LocalSet(ptr));
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
    /// Tras INVERTIR EL DEFAULT (los record literals viven como hashmap), ningún
    /// valor de tipo `Shape` es contiguo en runtime, así que esta frontera es
    /// hoy pass-through: existe como PUNTO único de coerción para futuros
    /// layouts (si algo vuelve a ser contiguo, la conversión se implementa aquí
    /// y todas las transferencias lo heredan).
    pub(crate) fn emit_coerce(&mut self, expr: &Expression, _dest: Option<&Type>) -> ClsResult<()> {
        self.emit_expression(expr)
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
        // Spreads de props `{...expr}` (REST_SPREAD_PLAN): las props del CmxValue
        // viven como hashmap (offset 8). Si hay spreads, alocar props (record
        // vacío), setear en el CmxValue y mergear cada spread.
        if !c.spreads.is_empty() {
            self.body.push(Instruction::I64Const(0));
            if let Some(&idx) = self.func_indexes.get("__intr_record_new") {
                self.body.push(Instruction::Call(idx));
            }
            let props_ptr = self.fresh_local();
            self.body.push(Instruction::LocalSet(props_ptr));
            // CmxValue.offset8 = props_ptr
            self.body.push(Instruction::LocalGet(ptr));
            self.body.push(Instruction::I64Const(8));
            self.body.push(Instruction::I64Add);
            self.body.push(Instruction::I32WrapI64);
            self.body.push(Instruction::LocalGet(props_ptr));
            self.body.push(Instruction::I64Store(MemArg {
                offset: 0,
                align: 3,
                memory_index: 0,
            }));
            for spread in &c.spreads {
                // merge(dst=props_ptr, src=spread_ptr) — primero dst, luego src.
                self.body.push(Instruction::LocalGet(props_ptr));
                self.emit_expression(spread)?;
                if let Some(&idx) = self.func_indexes.get("__intr_record_merge") {
                    self.body.push(Instruction::Call(idx));
                }
                self.body.push(Instruction::LocalSet(props_ptr)); // write-back
            }
            // Actualizar CmxValue.offset8 con el props_ptr final (el merge pudo
            // reallocar). Sin esto, CmxSetProp/lectura usan el ptr viejo.
            self.body.push(Instruction::LocalGet(ptr));
            self.body.push(Instruction::I64Const(8));
            self.body.push(Instruction::I64Add);
            self.body.push(Instruction::I32WrapI64);
            self.body.push(Instruction::LocalGet(props_ptr));
            self.body.push(Instruction::I64Store(MemArg {
                offset: 0,
                align: 3,
                memory_index: 0,
            }));
        }
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
                self.emit_str_host("__intr_str_int");
            }
            if let Some(&idx) = self.func_indexes.get("__intr_record_get") {
                self.body.push(Instruction::Call(idx));
            }
            let elem_ty = self.index_elem_type(i)?;
            self.bits_to_elem(elem_ty)?;
            return Ok(());
        }
        // Shape: r["campo"] con clave literal -> los literals viven como hashmap
        // (default invertido): record_get por la clave.
        if let Some(Type::Shape(fields)) = &obj_ty {
            if let Expression::Literal(l) = &*i.index {
                if let LiteralKind::String(key) = &l.kind {
                    let (_, w, _off) = self
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
                    let k = self.intern_string(key);
                    self.emit_load_str(k);
                    if let Some(&idx) = self.func_indexes.get("__intr_record_get") {
                        self.body.push(Instruction::Call(idx));
                    }
                    self.bits_to_elem(w)?;
                    return Ok(());
                }
            }
            // Clave no literal sobre shape: tratar el shape como hashmap y usar
            // AnyIndex (el typeck ya tipó el resultado como Value/Any).
            let expr = Expression::Index(i.clone());
            self.emit_any_chain(&expr)?;
            self.body.push(Instruction::Drop);
            return Ok(());
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
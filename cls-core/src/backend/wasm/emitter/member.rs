//! Member access: check_field/method, any_chain, emit_member_access (Fase 1: extraido de emitter/mod.rs).

use super::*;

impl<'a> FuncEmitter<'a> {


    /// Valida visibilidad de un campo de clase (private/protected) para lectura
    /// o escritura desde el contexto actual. `private` y `protected` requieren
    /// estar dentro de la clase (o subclase para protected).
    pub(crate) fn check_field_access(
        &self,
        class_name: &str,
        field: &str,
        vis: FieldVis,
        span: &Span,
    ) -> ClsResult<()> {
        if vis.is_private() {
            let inside = self
                .current_class
                .as_deref()
                .map(|c| c == class_name)
                .unwrap_or(false);
            if !inside {
                return Err(crate::error::ClsError::compile_at(
                    &format!("El campo '{}' es private (solo accesible desde la clase)", field),
                    span,
                ));
            }
        }
        if vis.is_protected() {
            // Accesible desde la clase y sus subclases.
            let allowed = self
                .current_class
                .as_deref()
                .map(|cur| {
                    if cur == class_name {
                        true
                    } else {
                        self.class_defs
                            .get(cur)
                            .map(|info| info.ancestors.iter().any(|a| a == class_name))
                            .unwrap_or(false)
                    }
                })
                .unwrap_or(false);
            if !allowed {
                return Err(crate::error::ClsError::compile_at(
                    &format!(
                        "El campo '{}' es protected (solo accesible desde la clase o sus subclases)",
                        field
                    ),
                    span,
                ));
            }
        }
        Ok(())
    }


    /// Enforca la visibilidad de un método: private -> solo desde la clase;
    /// protected -> desde la clase o subclases. Paridad con el walker.
    pub(crate) fn check_method_access(
        &self,
        class_name: &str,
        method: &str,
        vis: FieldVis,
        span: &Span,
    ) -> ClsResult<()> {
        if vis.is_private() {
            let inside = self
                .current_class
                .as_deref()
                .map(|c| c == class_name)
                .unwrap_or(false);
            if !inside {
                return Err(crate::error::ClsError::compile_at(
                    &format!("El método '{}' es private (solo accesible desde la clase)", method),
                    span,
                ));
            }
        }
        if vis.is_protected() {
            let allowed = self
                .current_class
                .as_deref()
                .map(|cur| {
                    if cur == class_name {
                        true
                    } else {
                        self.class_defs
                            .get(cur)
                            .map(|info| info.ancestors.iter().any(|a| a == class_name))
                            .unwrap_or(false)
                    }
                })
                .unwrap_or(false);
            if !allowed {
                return Err(crate::error::ClsError::compile_at(
                    &format!(
                        "El método '{}' es protected (solo accesible desde la clase o sus subclases)",
                        method
                    ),
                    span,
                ));
            }
        }
        Ok(())
    }


    /// Tag runtime estático de un tipo (paridad con `fmt_val_to_string` del host):
    /// 0=int,1=string,2=float,3=bool,4=char,5=cmx,6=array,7=record.
    pub(crate) fn any_static_tag(&self, t: &Type) -> i64 {
        match t {
            Type::Record(_, _) | Type::Json | Type::Shape(_) => 7,
            Type::Array(_) => 6,
            Type::String => 1,
            Type::Bool => 3,
            Type::Float | Type::F32 | Type::F64 => 2,
            Type::Char => 4,
            Type::Cmx => 5,
            _ => 0,
        }
    }


    /// Evalúa una cadena de acceso `o.a.c`, `o.x[0]`, `o.a.b[0]` sobre valores
    /// `Value`/Record de json.parse, despachando por tag en runtime. Deja `(val, tag)`
    /// en el stack. La base (raíz de la cadena) se emite con su tag estático.
    pub(crate) fn emit_any_chain(&mut self, expr: &Expression) -> ClsResult<()> {
        match expr {
            Expression::MemberAccess(m) => {
                // Si el objeto es una CLASE (o struct) y el miembro es un campo,
                // `me.campo` es acceso a un campo real (p.ej. un Record), no una
                // cadena JSON anidada. Evaluarlo como miembro normal + tag del
                // tipo del campo; AnyMember solo aplica a cadenas sobre JSON.
                let is_class_field = if let Some(Type::Named(cn, _)) =
                    self.types.get(&expr_span(&m.object))
                {
                    self.class_defs.contains_key(cn.as_str())
                        || self.struct_defs.contains_key(cn.as_str())
                } else {
                    false
                };
                if is_class_field {
                    self.emit_expression(&Expression::MemberAccess(m.clone()))?;
                    let t = self
                        .types
                        .get(&expr_span(&Expression::MemberAccess(m.clone())))
                        .cloned()
                        .unwrap_or(Type::Value);
                    let tag = self.any_static_tag(&t);
                    self.body.push(Instruction::I64Const(tag));
                    return Ok(());
                }
                self.emit_any_chain(&m.object)?;
                let k = self.intern_string(&m.member);
                self.emit_load_str(k);
                self.host.call(HostFn::AnyMember, &mut self.body);
                Ok(())
            }
            Expression::Index(i) => {
                self.emit_any_chain(&i.object)?;
                self.emit_expression(&i.index)?;
                // Índice numérico (p.ej. `json.parse("[..]")[j]` o `arr[0]` de un
                // array JSON): AnyIndex acepta idx numérico (tag 6 = array) y
                // string (tag 7 = record con claves "0","1",...); el host lo
                // normaliza.
                self.host.call(HostFn::AnyIndex, &mut self.body);
                Ok(())
            }
            other => {
                self.emit_expression(other)?;
                let t = self
                    .types
                    .get(&expr_span(other))
                    .cloned()
                    .unwrap_or(Type::Value);
                let tag = self.any_static_tag(&t);
                self.body.push(Instruction::I64Const(tag));
                Ok(())
            }
        }
    }


    /// Member access de primitivos: `.length` sobre tuplas/arrays, variantes de enum.
    pub(crate) fn emit_member_access(&mut self, m: &MemberAccessExpr) -> ClsResult<()> {        if let Expression::Identifier(obj_name, _) = &*m.object {
            if let Some((def_id, variants)) = self.enum_defs.get(obj_name).cloned() {
                let idx = variants
                    .iter()
                    .position(|v| *v == m.member)
                    .ok_or_else(|| {
                        crate::error::ClsError::CompileError(format!(
                            "La variante '{}' no existe en el enum '{}'",
                            m.member, obj_name
                        ))
                    })?;
                // Enum con payload: `obj.miembro` (dot) solo es válido para
                // variantes unit; las de payload se construyen con `obj::v(...)`.
                if self.enum_key_has_payload(obj_name) {
                    let has_params = self
                        .enum_params
                        .get(obj_name)
                        .and_then(|vs| vs.get(idx))
                        .map(|p| !p.is_empty())
                        .unwrap_or(false);
                    if has_params {
                        return Err(crate::error::ClsError::compile_at(
                            &format!(
                                "La variante '{}' tiene payload; construila con `{}::{}(...)`",
                                m.member, obj_name, m.member
                            ),
                            &m.span,
                        ));
                    }
                    return self.emit_enum_construct(obj_name, &m.member, &[]);
                }
                let val = ((def_id as i64) << 32) | idx as i64;
                self.body.push(Instruction::I64Const(val));
                return Ok(());
            }
            // Constantes de módulos stdlib: math.PI / math.E. Soporta alias
            // (`import "math" as m` -> `m.PI`).
            let canon = self
                .import_aliases
                .get(obj_name)
                .cloned()
                .unwrap_or_else(|| obj_name.clone());
            if canon == "math" {
                match m.member.as_str() {
                    "PI" => {
                        self.body.push(Instruction::F64Const(Ieee64::new(
                            std::f64::consts::PI.to_bits(),
                        )));
                        return Ok(());
                    }
                    "E" => {
                        self.body.push(Instruction::F64Const(Ieee64::new(
                            std::f64::consts::E.to_bits(),
                        )));
                        return Ok(());
                    }
                    _ => return Err(self.unsupported_expr(&Expression::MemberAccess(m.clone()))),
                }
            }
        }
        // `lib::Color.Rojo`: el objeto es un access namespaced cuyo prefijo apunta
        // a un enum del módulo importado (flattened como `lib::Color`).
        if let Expression::NamespaceAccess(ns, name, _) = &*m.object {
            let key = format!("{}::{}", ns, name);
            if let Some((def_id, variants)) = self.enum_defs.get(&key).cloned() {
                let idx = variants
                    .iter()
                    .position(|v| *v == m.member)
                    .ok_or_else(|| {
                        crate::error::ClsError::CompileError(format!(
                            "La variante '{}' no existe en el enum '{}'",
                            m.member, key
                        ))
                    })?;
                if self.enum_key_has_payload(&key) {
                    let has_params = self
                        .enum_params
                        .get(&key)
                        .and_then(|vs| vs.get(idx))
                        .map(|p| !p.is_empty())
                        .unwrap_or(false);
                    if has_params {
                        return Err(crate::error::ClsError::compile_at(
                            &format!(
                                "La variante '{}' tiene payload; construila con `{}::{}(...)`",
                                m.member, key, m.member
                            ),
                            &m.span,
                        ));
                    }
                    return self.emit_enum_construct(&key, &m.member, &[]);
                }
                let val = ((def_id as i64) << 32) | idx as i64;
                self.body.push(Instruction::I64Const(val));
                return Ok(());
            }
            // Estático de clase namespaced: `lib::Clase.campo` -> el flatten trae
            // la clase como `lib::Clase` y sus estáticos como `lib::Clase::campo`.
            if let Some(&g) = self.static_fields.get(&format!("{}::{}", key, m.member)) {
                self.body.push(Instruction::GlobalGet(g));
                return Ok(());
            }
        }
        // `Clase.campo` (campo estático): el objeto es el nombre de la clase.
        if let Expression::Identifier(cn, _) = &*m.object {
            if let Some(&g) = self.static_fields.get(&format!("{}::{}", cn, m.member)) {
                self.body.push(Instruction::GlobalGet(g));
                return Ok(());
            }
        }
        let obj_ty = self
            .types
            .get(&expr_span(&m.object))
            .cloned()
            .unwrap_or(Type::Value);
        let obj_ty = match self.object_class_key(&m.object) {
            Some(key) => Type::Named(key, vec![]),
            None => obj_ty,
        };
        self.emit_expression(&m.object)?;
        match obj_ty {
            Type::String => match m.member.as_str() {
                "length" => {
                    self.emit_str_host("__intr_str_length");
                    Ok(())
                }
                _ => Err(self.unsupported_expr(&Expression::MemberAccess(m.clone()))),
            },
            Type::Tuple(_) | Type::Array(_) => match m.member.as_str() {
                "length" => {
                    self.emit_array_len();
                    Ok(())
                }
                _ => Err(self.unsupported_expr(&Expression::MemberAccess(m.clone()))),
            },
            Type::Record(_, _) => match m.member.as_str() {
                "length" | "size" => {
                    if let Some(&idx) = self.func_indexes.get("__intr_record_len") {
                        self.body.push(Instruction::Call(idx));
                    }
                    Ok(())
                }
                _ => {
                    // acceso por nombre de campo: r.campo -> record_get(ptr, "campo")
                    let k = self.intern_string(&m.member);
                    self.emit_load_str(k);
                    if let Some(&idx) = self.func_indexes.get("__intr_record_get") {
                        self.body.push(Instruction::Call(idx));
                    }
                    Ok(())
                }
            },
            Type::Shape(fields) => {
                // DEFAULT INVERTIDO: los record literals viven como hashmap, asi
                // que un valor tipado Shape se lee con las ops de record
                // (record_len/record_get). El chequeo estatico de campos sigue
                // siendo trabajo del typeck.
                match m.member.as_str() {
                    "length" | "size" => {
                        if let Some(&idx) = self.func_indexes.get("__intr_record_len") {
                            self.body.push(Instruction::Call(idx));
                        }
                        Ok(())
                    }
                    "has" => Err(crate::error::ClsError::compile_at(
                        "`has` sobre record usa la forma r.has(key)",
                        &m.span,
                    )),
                    _ => {
                        let k = self.intern_string(&m.member);
                        self.emit_load_str(k);
                        if let Some(&idx) = self.func_indexes.get("__intr_record_get") {
                            self.body.push(Instruction::Call(idx));
                        }
                        // Bug fix dev-2 (Fase 7): __intr_record_get devuelve los
                        // bits crudos del valor (i64). Para Float hay que
                        // reinterpretar los bits como f64. El typeck conoce el
                        // tipo del campo por el Shape; lo buscamos y aplicamos
                        // F64ReinterpretI64 si corresponde. Antes el emisor
                        // dejaba el i64 crudo en el stack, lo que producia
                        // "type mismatch: expected f64, found i64" en
                        // contextos que esperan Float (e.g. `print("x:", pt.x)`).
                        if let Some((_, field_ty)) = fields.iter().find(|(n, _)| n == &m.member) {
                            if matches!(field_ty, Type::Float) {
                                self.body.push(Instruction::F64ReinterpretI64);
                            }
                        }
                        Ok(())
                    }
                }
            },
            Type::Cmx => match m.member.as_str() {
                "tag" => self.emit_cmx_field(0),
                "props" => self.emit_cmx_field(8),
                "children" => self.emit_cmx_field(16),
                // kind en offset 24: 0=elemento, 1=texto (layout host_cmx_new).
                "kind" => self.emit_cmx_field(24),
                "tagCallable" => self.emit_cmx_field(32),
                "isComponent" => {
                    self.emit_cmx_field(40)?;
                    self.body.push(Instruction::I32WrapI64);
                    Ok(())
                }
                _ => Err(self.unsupported_expr(&Expression::MemberAccess(m.clone()))),
            },
            Type::Named(name, _) => {
                if let Some(info) = self.struct_defs.get(name.as_str()) {
                    let fidx = info
                        .fields
                        .iter()
                        .position(|(n, _, _)| *n == m.member)
                        .ok_or_else(|| {
                            crate::error::ClsError::CompileError(format!(
                                "El campo '{}' no existe en '{}'",
                                m.member, name
                            ))
                        })?;
                    let w = info.fields[fidx].2;
                    let fw = info.kinds[fidx].scalar().ok_or_else(|| {
                        crate::error::ClsError::compile_at(
                            &format!(
                                "El campo '{}' es un array inline (T[n]) del struct '{}': \
                                 no se accede como escalar (solo layout C)",
                                m.member, name
                            ),
                            &m.span,
                        )
                    })?;
                    self.body.push(Instruction::I64Const(info.offsets[fidx]));
                    self.body.push(Instruction::I64Add);
                    self.body.push(Instruction::I32WrapI64);
                    // Carga con el ancho C del campo (i16 -> 2 bytes + extend).
                    fw.push_load(&mut self.body, w);
                    Ok(())
                } else if let Some(key) = self.resolve_class_key(&name) {
                    let info = self.class_defs.get(&key).unwrap();
                    let fidx = info
                        .fields
                        .iter()
                        .position(|(n, _, _, _, _)| *n == m.member)
                        .ok_or_else(|| {
                            crate::error::ClsError::compile_at(
                                &format!(
                                    "El campo '{}' no existe en la clase '{}'",
                                    m.member, key
                                ),
                                &m.span,
                            )
                        })?;
                    let (_, _t, w, off, vis) = &info.fields[fidx];
                    // Validar visibilidad: private/protected desde fuera.
                    self.check_field_access(key.as_str(), m.member.as_str(), *vis, &m.span)?;
                    let w = *w;
                    let off = *off;
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
                    Ok(())
                } else {
                    Err(self.unsupported_expr(&Expression::MemberAccess(m.clone())))
                }
            }
            // Unión temporal (F5): el valor es un i64 con identidad de tipo en
            // runtime (class_id en `[obj+8]`, def_id de struct en `[obj+0]`).
            // Se discrimina por ese id y se carga el campo en el offset de CADA
            // miembro (los offsets pueden diferir). El objeto ya está en el stack.
            Type::Union(members) => self.emit_union_member_access(m, &members),
            Type::Value | Type::Json => {
                // `o.a.c` donde `o.a` es Value/JSON (json.parse anidado): despachar por tag.
                let expr = Expression::MemberAccess(m.clone());
                self.emit_any_chain(&expr)?;
                // Resultado (val, tag) en el stack -> dejar solo el val (el tag se
                // pierde en un valor Value; los prints usan emit_print_arg con PrintAny).
                self.body.push(Instruction::Drop);
                Ok(())
            }
            _ => Err(self.unsupported_expr(&Expression::MemberAccess(m.clone()))),
        }
    }

    /// Acceso a miembro `valor.campo` sobre una **unión de clases/structs** (F5).
    /// El objeto (i64) ya está en el stack. Se lee su identificador de tipo en
    /// runtime (clase: `class_id` en `[obj+8]`, como `is` de clase; struct:
    /// `def_id` en `[obj+0]`) y se emite una cadena de `if/else` que carga el
    /// campo en el offset propio de cada miembro; todas las ramas convergen al
    /// mismo tipo WASM (sin boxing). Clases y structs NO se mezclan (el id vive
    /// en offsets distintos) y los anchos del campo deben coincidir.
    fn emit_union_member_access(
        &mut self,
        m: &MemberAccessExpr,
        members: &[Type],
    ) -> ClsResult<()> {
        // Tipo del resultado (común o unión) fijado por el typeck.
        let res_ty = self
            .types
            .get(&expr_span(&Expression::MemberAccess(m.clone())))
            .cloned()
            .unwrap_or(Type::Value);
        let res_w = was_type(&res_ty).unwrap_or(WasTy::I64);

        struct Arm {
            ids: Vec<u32>,
            offset: i64,
            field_w: WasTy,
            kind: Option<FieldWidth>,
        }
        let mut arms: Vec<Arm> = Vec::new();
        let mut all_class = true;
        let mut all_struct = true;
        for mt in members {
            let name = match mt {
                Type::Named(n, _) => n,
                _ => {
                    return Err(crate::error::ClsError::compile_at(
                        &format!(
                            "acceso al miembro `{}` sobre una unión: el tipo `{}` no es una \
                             clase ni un struct (no hay dispatch por id sin boxing)",
                            m.member, mt
                        ),
                        &m.span,
                    ));
                }
            };
            if let Some(key) = self.resolve_class_key(name) {
                let info = self.class_defs.get(&key).unwrap();
                let fidx = info
                    .fields
                    .iter()
                    .position(|(n, _, _, _, _)| *n == m.member)
                    .ok_or_else(|| {
                        crate::error::ClsError::compile_at(
                            &format!("El campo `{}` no existe en la clase `{}`", m.member, key),
                            &m.span,
                        )
                    })?;
                let (_, _t, w, off, vis) = &info.fields[fidx];
                self.check_field_access(key.as_str(), m.member.as_str(), *vis, &m.span)?;
                let (w, off) = (*w, *off);
                // ids que matchean: la clase + todas sus subclases (paridad `is`).
                let target_id = info.class_id;
                let mut ids: Vec<u32> = Vec::new();
                for cinfo in self.class_defs.values() {
                    let desc = cinfo
                        .ancestors
                        .iter()
                        .any(|a| self.ancestor_class_id(a) == Some(target_id));
                    if cinfo.class_id == target_id || desc {
                        if !ids.contains(&cinfo.class_id) {
                            ids.push(cinfo.class_id);
                        }
                    }
                }
                all_struct = false;
                arms.push(Arm {
                    ids,
                    offset: off,
                    field_w: w,
                    kind: None,
                });
            } else if let Some(info) = self.struct_defs.get(name.as_str()) {
                let fidx = info
                    .fields
                    .iter()
                    .position(|(n, _, _)| *n == m.member)
                    .ok_or_else(|| {
                        crate::error::ClsError::compile_at(
                            &format!("El campo `{}` no existe en el struct `{}`", m.member, name),
                            &m.span,
                        )
                    })?;
                let w = info.fields[fidx].2;
                let fw = info.kinds[fidx].scalar().ok_or_else(|| {
                    crate::error::ClsError::compile_at(
                        &format!(
                            "El campo `{}` del struct `{}` es un array inline: no se accede \
                             como escalar",
                            m.member, name
                        ),
                        &m.span,
                    )
                })?;
                all_class = false;
                arms.push(Arm {
                    ids: vec![info.def_id],
                    offset: info.offsets[fidx],
                    field_w: w,
                    kind: Some(fw),
                });
            } else {
                return Err(crate::error::ClsError::compile_at(
                    &format!(
                        "acceso al miembro `{}` sobre una unión: el tipo `{}` no es una clase \
                         ni un struct",
                        m.member, mt
                    ),
                    &m.span,
                ));
            }
        }
        if arms.is_empty() {
            return Err(crate::error::ClsError::compile_at(
                "acceso a miembro sobre una unión vacía",
                &m.span,
            ));
        }
        if !all_class && !all_struct {
            return Err(crate::error::ClsError::compile_at(
                "acceso a miembro sobre una unión mixta de clases y structs: no soportado \
                 (el id de tipo vive en offsets distintos)",
                &m.span,
            ));
        }
        // Discriminante y cargas deben compartir el ancho WASM (sin boxing).
        if arms.iter().any(|a| a.field_w != res_w) {
            return Err(crate::error::ClsError::compile_at(
                &format!(
                    "acceso al miembro `{}` sobre una unión con tipos heterogéneos: no se \
                     puede sin boxing",
                    m.member
                ),
                &m.span,
            ));
        }

        // Objeto -> local; id de tipo -> local.
        let obj = self.fresh_local();
        self.body.push(Instruction::LocalSet(obj));
        let cid = self.fresh_local();
        self.body.push(Instruction::LocalGet(obj));
        self.body.push(Instruction::I32WrapI64);
        self.body.push(Instruction::I64Load(MemArg {
            offset: if all_class { 8 } else { 0 },
            align: 3,
            memory_index: 0,
        }));
        self.body.push(Instruction::LocalSet(cid));

        // Cadena `if (cid == id_k) { load_k } else { ... }` (n-1 elses + default).
        let n = arms.len();
        for arm in &arms {
            let mut first = true;
            for id in &arm.ids {
                self.body.push(Instruction::LocalGet(cid));
                self.body.push(Instruction::I64Const(*id as i64));
                self.body.push(Instruction::I64Eq);
                if !first {
                    self.body.push(Instruction::I32Or);
                }
                first = false;
            }
            self.block_depth += 1;
            self.body
                .push(Instruction::If(BlockType::Result(res_w.val_type())));
            self.body.push(Instruction::LocalGet(obj));
            self.body.push(Instruction::I64Const(arm.offset));
            self.body.push(Instruction::I64Add);
            self.body.push(Instruction::I32WrapI64);
            if let Some(kind) = arm.kind {
                kind.push_load(&mut self.body, arm.field_w);
            } else {
                match arm.field_w {
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
            }
            self.body.push(Instruction::Else);
        }
        // Sin coincidencia: no debería ocurrir (la unión es cerrada) -> trap claro.
        self.emit_throw(
            "acceso a miembro sobre unión: subtipo de runtime no reconocido",
            &m.span,
        );
        self.body.push(Instruction::Unreachable);
        for _ in 0..n {
            self.body.push(Instruction::End);
            self.block_depth -= 1;
        }
        Ok(())
    }
}
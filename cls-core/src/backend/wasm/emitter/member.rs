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
            Type::Record(_, _) => 7,
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
    /// `Any`/Record de json.parse, despachando por tag en runtime. Deja `(val, tag)`
    /// en el stack. La base (raíz de la cadena) se emite con su tag est�tico.
    pub(crate) fn emit_any_chain(&mut self, expr: &Expression) -> ClsResult<()> {
        match expr {
            Expression::MemberAccess(m) => {
                self.emit_any_chain(&m.object)?;
                let k = self.intern_string(&m.member);
                self.emit_load_str(k);
                self.host.call(HostFn::AnyMember, &mut self.body);
                Ok(())
            }
            Expression::Index(i) => {
                self.emit_any_chain(&i.object)?;
                self.emit_expression(&i.index)?;
                self.host.call(HostFn::AnyIndex, &mut self.body);
                Ok(())
            }
            other => {
                self.emit_expression(other)?;
                let t = self
                    .types
                    .get(&expr_span(other))
                    .cloned()
                    .unwrap_or(Type::Any);
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
                let val = ((def_id as i64) << 32) | idx as i64;
                self.body.push(Instruction::I64Const(val));
                return Ok(());
            }
            // Constantes de módulos stdlib: math.PI / math.E
            if obj_name == "math" {
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
                let val = ((def_id as i64) << 32) | idx as i64;
                self.body.push(Instruction::I64Const(val));
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
            .unwrap_or(Type::Any);
        self.emit_expression(&m.object)?;
        match obj_ty {
            Type::String => match m.member.as_str() {
                "length" => {
                    self.host.call(HostFn::StrLength, &mut self.body);
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
                    self.host.call(HostFn::RecordLen, &mut self.body);
                    Ok(())
                }
                _ => {
                    // acceso por nombre de campo: r.campo -> record_get(ptr, "campo")
                    let k = self.intern_string(&m.member);
                    self.emit_load_str(k);
                    self.host.call(HostFn::RecordGet, &mut self.body);
                    Ok(())
                }
            },
            Type::Shape(fields) => match m.member.as_str() {
                "length" | "size" => {
                    // Compile-time: el shape tiene un n�� de campos fijo.
                    self.body.push(Instruction::I64Const(fields.len() as i64));
                    Ok(())
                }
                "has" => {
                    let has = fields.iter().any(|(n, _)| *n == m.member);
                    self.body
                        .push(Instruction::I32Const(if has { 1 } else { 0 }));
                    Ok(())
                }
                _ => {
                    let (_, w, off) = self
                        .shape_layout(&fields)?
                        .into_iter()
                        .find(|(n, _, _)| *n == m.member)
                        .ok_or_else(|| {
                            crate::error::ClsError::compile_at(
                                &format!("El record no tiene el campo '{}'", m.member),
                                &m.span,
                            )
                        })?;
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
                }
            },
            Type::Cmx => match m.member.as_str() {
                "tag" => self.emit_cmx_field(0),
                "props" => self.emit_cmx_field(8),
                "children" => self.emit_cmx_field(16),
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
                    self.body.push(Instruction::I64Const(info.offsets[fidx]));
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
                } else if let Some(info) = self.class_defs.get(name.as_str()) {
                    let fidx = info
                        .fields
                        .iter()
                        .position(|(n, _, _, _, _)| *n == m.member)
                        .ok_or_else(|| {
                            crate::error::ClsError::compile_at(
                                &format!(
                                    "El campo '{}' no existe en la clase '{}'",
                                    m.member, name
                                ),
                                &m.span,
                            )
                        })?;
                    let (_, _t, w, off, vis) = &info.fields[fidx];
                    // Validar visibilidad: private/protected desde fuera.
                    self.check_field_access(name.as_str(), m.member.as_str(), *vis, &m.span)?;
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
            Type::Any => {
                // `o.a.c` donde `o.a` es Any (json.parse anidado): despachar por tag.
                let expr = Expression::MemberAccess(m.clone());
                self.emit_any_chain(&expr)?;
                // Resultado (val, tag) en el stack -> dejar solo el val (el tag se
                // pierde en un valor Any; los prints usan emit_print_arg con PrintAny).
                self.body.push(Instruction::Drop);
                Ok(())
            }
            _ => Err(self.unsupported_expr(&Expression::MemberAccess(m.clone()))),
        }
    }

}
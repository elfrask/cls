//! Parte del motor de emision (Fase 1: extraido de engine/mod.rs).

use super::*;

impl<'a> Engine<'a> {
    pub(crate) fn collect_functions(&mut self, module: &Module) -> ClsResult<()> {
        for stmt in &module.statements {
            if let Statement::FunctionDecl(f) = stmt {
                self.collect_function(f)?;
            }
        }
        if !self.func_types.contains_key("main") {
            if self.require_main {
                return Err(crate::error::ClsError::CompileError(
                    "No se encontrí³ function main(args: String[]) para el JIT".to_string(),
                ));
            }
            // Modo librería: main no-op sintetizado (el host lo llama con args=0).
            self.func_types.insert(
                "main".to_string(),
                (vec![Type::Array(Box::new(Type::String))], Some(Type::Int)),
            );
            self.func_defaults.insert("main".to_string(), vec![None]);
        }
        Ok(())
    }


    pub(crate) fn collect_function(&mut self, f: &FunctionDecl) -> ClsResult<()> {
        let mut params: Vec<Type> = Vec::new();
        let mut defaults: Vec<Option<Expression>> = Vec::new();
        for p in &f.params {
            let t = p.type_ann.as_ref().ok_or_else(|| {
                crate::error::ClsError::CompileError(format!(
                    "Parámetro '{}' de '{}' sin anotació de tipo (requerido por el JIT)",
                    p.name, f.name
                ))
            })?;
            params.push(self.resolve_annotation_type(t)?);
            defaults.push(p.default_value.clone());
        }
        let ret = match &f.return_type {
            Some(t) => Some(self.resolve_annotation_type(t)?),
            None => None,
        };
        self.func_types.insert(f.name.clone(), (params, ret));
        self.func_defaults.insert(f.name.clone(), defaults);
        Ok(())
    }


    fn resolve_annotation_type(&self, ann: &TypeAnnotation) -> ClsResult<Type> {
        let t = annotation_to_type(ann);
        match t {
            Type::Any | Type::Unknown => Err(crate::error::ClsError::CompileError(
                "Anotació de tipo no soportada por el JIT (se requiere tipo concreto)".to_string(),
            )),
            other => Ok(other),
        }
    }


    /// Tipo concreto de un campo de struct/clase. Si la anotació no resuelve a
    /// un tipo concreto (`Any`/`Unknown`), se intenta el type map (el campo tiene
    /// un span); si el kind es un tipo nombrado (struct/clase/enum) se trata como
    /// puntero (i64); si nada resuelve, error claro en vez de asumir i64.
    pub(crate) fn resolve_field_type(
        &self,
        ann: &TypeAnnotation,
    ) -> ClsResult<Type> {
        let t = annotation_to_type(ann);
        if !matches!(t, Type::Any | Type::Unknown) {
            return Ok(t);
        }
        if let Some(rt) = self.types.get(&ann.span).cloned() {
            if !matches!(rt, Type::Any | Type::Unknown) {
                return Ok(rt);
            }
        }
        if let TypeKind::Named(name, args) = &ann.kind {
            return Ok(Type::Named(
                name.clone(),
                args.iter().map(annotation_to_type).collect(),
            ));
        }
        // `Any`/`Unknown` sin tipo concreto: campo dinámico (viaja como i64).
        // El acceso a un campo `Any` usa dispatch por tag en runtime (records,
        // callbacks, valores JSON). Es el comportamiento del "dinamismo 0":
        // `Any` sigue siendo un tipo de campo válido (el JIT lo rechaza solo en
        // params/retornos de funciones, donde se requiere la firma WASM).
        Ok(t)
    }


    pub(crate) fn compile_function(&mut self, f: &FunctionDecl) -> ClsResult<Function> {
        let (param_types, ret) = self.func_types[&f.name].clone();
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
        // Métodos de clase: `me` (la instancia) es el primer param implícito.
        // `Clase::metodo` es método si el prefijo es una clase conocida; si no,
        // es un símbolo de mí³dulo importado (`mod::fn`, sin `me`).

        let is_method = f
            .name
            .split("::")
            .next()
            .map(|c| self.class_defs.contains_key(c))
            .unwrap_or(false);
        // Un método static NO recibe `me` ni establece la clase actual (así que
        // `me.` dentro de él da error de variable no definida, paridad walker).
        let is_static = f
            .modifiers
            .contains(&crate::frontend::ast::FunctionModifier::Static);
        let current_class = if is_method && !is_static {
            f.name.split("::").next().map(|s| s.to_string())
        } else {
            None
        };
        fe.current_class = current_class;
        fe.current_method = if is_method {
            f.name.split("::").nth(1).map(|s| s.to_string())
        } else {
            None
        };
        fe.current_fn_span = f.span.clone();
        // Retorno declarado (frontera de `return`): convertir Shape→hashmap si
        // el retorno es dinámico y el valor es un shape contiguo.
        fe.fn_ret = self.func_types.get(&f.name).and_then(|(_, r)| r.clone());
        fe.shadow_ptr_global = self.shadow_ptr_global;
        let is_main = f.name == "main";
        // Promover al heap las variables locales capturadas por arrows del body:
        // para que la mutació del closure sea visible en el scope externo (paridad
        // con el walker, que captura por referencia). Aplica también a main (que
        // puede declarar arrows locales).
        if !is_method {
            let mut arrows: Vec<ArrowFunctionExpr> = Vec::new();
            collect_arrows_in_block(&f.body, &mut arrows);
            for a in &arrows {
                if let Some(caps) = self.arrow_captures.get(&a.span) {
                    for c in caps {
                        fe.promoted.insert(c.clone());
                    }
                }
            }
            // Si esta funció ES una arrow con capturas, sus capturas son
            // referencias a slots promovidos (para doble deref en el acceso).
            if let Some(caps) = self.arrow_captures.get(&f.span) {
                for c in caps {
                    fe.promoted.insert(c.clone());
                }
            }
        }
        // Firma uniforme (B5): las funciones top-level (excepto main, entry del
        // host) reciben __capturas (i64) como param/local 0. Las arrows ademas
        // registran sus variables libres (closures) para cargarlas del bloque.
        let has_caps = !is_method && !is_main;
        let param_offset = if has_caps { 1 } else { 0 };
        if has_caps {
            fe.declare_var_ty("__capturas", WasTy::I64);
            let arrow_caps = self
                .arrow_captures
                .get(&f.span)
                .cloned()
                .unwrap_or_default();
            for (i, cap) in arrow_caps.iter().enumerate() {
                fe.captures.insert(cap.clone(), (i + 1) as u32);
            }
        }
        if is_method && !is_static {
            fe.declare_var_ty("me", was_type(&param_types[0])?);
            for (i, p) in f.params.iter().enumerate() {
                fe.declare_var_ty(&p.name, was_type(&param_types[i + 1])?);
            }
        } else {
            for (i, p) in f.params.iter().enumerate() {
                // Firma normalizada (dev-2): TODOS los params de arrows son I64
                // en la firma WASM. Los params float se convierten de vuelta a
                // f64 vía prologue (shadow local) después de la declaración.
                let is_arrow = f.name.starts_with("__arrow_");
                if is_arrow {
                    fe.declare_var_ty(&p.name, WasTy::I64);
                } else {
                    fe.declare_var_ty(&p.name, was_type(&param_types[i])?);
                }
            }
            // Prologue de conversión: para cada param float de una arrow,
            // aloca un shadow f64 local y emite
            // `local.get(i64_param); f64.reinterpret_i64; local.set(f64_shadow)`.
            // Luego redirige el nombre al shadow para que el body lea f64.
            let is_arrow = f.name.starts_with("__arrow_");
            if is_arrow {
                for p in f.params.iter() {
                    let ann_t = p.type_ann.as_ref()
                        .map(|ta| crate::backend::wasm::types::annotation_to_type(ta))
                        .unwrap_or(Type::Any);
                    if matches!(ann_t, Type::Float | Type::F32 | Type::F64)
                        && !matches!(ann_t, Type::Any | Type::Unknown)
                    {
                        let i64_idx = fe.local_for(&p.name);
                        let f64_idx = fe.fresh_local_ty(WasTy::F64);
                        fe.body.push(Instruction::LocalGet(i64_idx));
                        fe.body.push(Instruction::F64ReinterpretI64);
                        fe.body.push(Instruction::LocalSet(f64_idx));
                        fe.override_local(&p.name, f64_idx, WasTy::F64);
                    }
                }
            }
        }
        // Reservar los índices de los params implícitos (main sintetizado en
        // modo librería: `f.params` vacío pero `func_types["main"]` tiene el
        // array de args). Sin esto, `emit_fn_enter` (stores del shadow stack)
        // usa `fresh_local_ty` que devuelve local 0 — pisando el `(param i64)`
        // de la firma y Cranelift rechaza el módulo ("failed to compile
        // function[N]").
        let expected_params = (param_types.len() as u32) + param_offset;
        while fe.next_local < expected_params {
            fe.local_tys.insert(fe.next_local, WasTy::I64);
            fe.next_local += 1;
        }        // main inicializa las globals top-level al arrancar.
        if f.name == "main" {
            if let Some(idx) = self.func_indexes.get("__init_globals") {
                fe.body.push(Instruction::Call(*idx));
            }
        }
        // Shadow call stack: registrar la entrada de la funció (nombre + span)
        // y des-registrarla al salir (antes de cada End).
        fe.emit_fn_enter(f)?;
        for s in &f.body.statements {
            fe.emit_statement(s)?;
        }
        if fe.dead_flow {
            // Flujo terminado (return/break/if con todas las ramas terminadas):
            // el fn_exit ya se emitió en el return; el `unreachable` marca el
            // código muerto de forma VÁLIDA (cranelift rechaza el `end` con
            // stack vacío tras un frame de if/switch/try con todas las ramas
            // terminadas: "expected i64 but nothing on stack").
            fe.body.push(Instruction::Unreachable);
        } else {
            // Flujo vivo hasta el final (p.ej. `while (true) { return }`, o una
            // función que cae sin return): el `End` exige el resultado en el
            // stack. Empujar el default (null/0) — nunca se ejecuta en los
            // casos de loop infinito; en los demás, el walker también devuelve
            // null al caer sin return.
            if let Some(rt) = ret.as_ref() {
                if *rt != Type::Void {
                    match was_type(rt)? {
                        WasTy::F64 => {
                            fe.body.push(Instruction::F64Const(Ieee64::new(0.0f64.to_bits())))
                        }
                        WasTy::I32 => fe.body.push(Instruction::I32Const(0)),
                        WasTy::I64 => fe.body.push(Instruction::I64Const(0)),
                    }
                }
            }
            fe.emit_fn_exit();
        }
        // End final del cuerpo de la funció (wasm-encoder no lo añade).
        fe.body.push(Instruction::End);
        // locals: cada índice con su tipo (fallback I64).
        // Importante: los params ocupan los índices 0..param_types.len(); los
        // locals declarados empiezan después. Cada local = un grupo de 1 para
        // preservar los índices exactos (agrupar reordenaría y rompería tipos
        // mixtos).
        let nparams = (param_types.len() as u32) + param_offset;
        let local_types: Vec<ValType> = (nparams..fe.next_local)
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
        Ok(func)
    }


    /// Tipo WASM de una expresió desde el type map (fallback I64).
    pub(crate) fn expr_was_type(&self, e: &Expression) -> ClsResult<WasTy> {
        let span = expr_span(e);
        if let Some(t) = self.types.get(&span) {
            was_type(t)
        } else {
            Ok(WasTy::I64)
        }
    }


    /// `__init_globals`: setea cada global de usuario con su valor inicial.
    pub(crate) fn declare_class_function(&mut self, class: &str, f: &FunctionDecl) {
        let is_static = f
            .modifiers
            .contains(&crate::frontend::ast::FunctionModifier::Static);
        let mut param_cls = Vec::new();
        let mut pv = Vec::new();
        if !is_static {
            param_cls.push(Type::Int); // me (ptr del objeto)
            pv.push(ValType::I64);
        }
        for p in &f.params {
            let t = p
                .type_ann
                .as_ref()
                .map(annotation_to_type)
                .unwrap_or(Type::Int);
            param_cls.push(t.clone());
            pv.push(was_type(&t).unwrap_or(WasTy::I64).val_type());
        }
        let rv: Vec<ValType> = match &f.return_type {
            Some(ann) => {
                let t = annotation_to_type(ann);
                if t != Type::Void {
                    vec![was_type(&t).unwrap_or(WasTy::I64).val_type()]
                } else {
                    vec![]
                }
            }
            None => vec![],
        };
        let ret_cls = f.return_type.as_ref().map(annotation_to_type);
        let tidx = self.register_func_type(pv, rv);
        let fidx = self.func_count;
        self.func_count += 1;
        self.funcs_sec.function(tidx);
        let key = if is_static {
            format!("{}::__s__{}", class, f.name)
        } else {
            format!("{}::{}", class, f.name)
        };
        self.func_indexes.insert(key.clone(), fidx);
        self.func_types.insert(key.clone(), (param_cls, ret_cls));
        self.method_type_indexes.insert(key.clone(), tidx);
        self.cls_funcs_extra.push((key, f.clone()));
    }


    /// Índice de funció de un método: en la clase o subiendo por ancestors.
    pub(crate) fn resolve_method_index(&self, class: &str, m: &str) -> Option<u32> {
        let mut cur = Some(class.to_string());
        while let Some(c) = cur {
            if let Some(idx) = self.func_indexes.get(&format!("{}::{}", c, m)) {
                return Some(*idx);
            }
            cur = self.class_defs.get(&c).and_then(|i| i.parent.clone());
        }
        None
    }

}

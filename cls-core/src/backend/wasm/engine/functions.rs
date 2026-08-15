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
                    "No se encontrÃƒÆ’Ã‚Â³ function main(args: String[]) para el JIT".to_string(),
                ));
            }
            // Modo librerÃƒÆ’Ã‚Â­a: main no-op sintetizado (el host lo llama con args=0).
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
                    "ParÃƒÆ’Ã‚Â¡metro '{}' de '{}' sin anotaciÃƒÆ’Ã‚Â³n de tipo (requerido por el JIT)",
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
                "AnotaciÃƒÆ’Ã‚Â³n de tipo no soportada por el JIT (se requiere tipo concreto)".to_string(),
            )),
            other => Ok(other),
        }
    }

    /// Tipo concreto de un campo de struct/clase. Si la anotaciÃƒÆ’Ã‚Â³n no resuelve a
    /// un tipo concreto (`Any`/`Unknown`), se intenta el type map (el campo tiene
    /// un span); si el kind es un tipo nombrado (struct/clase/enum) se trata como
    /// puntero (i64); si nada resuelve, error claro en vez de asumir i64.
    fn resolve_field_type(
        &self,
        owner: &str,
        field: &str,
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
        Err(crate::error::ClsError::CompileError(format!(
            "Campo '{}' de '{}' con tipo desconocido (el JIT requiere un tipo concreto)",
            field, owner
        )))
    }

    pub(crate) fn emit(&mut self, module: &Module) -> ClsResult<Vec<u8>> {
        self.collect_functions(module)?;

        // Recolectar enums ÃƒÂ¢Ã¢â‚¬Â Ã¢â‚¬â„¢ (def_id, variantes) para constantes `Nivel.Alto`.
        let mut def_id = 0u32;
        for stmt in &module.statements {
            if let Statement::EnumDecl(e) = stmt {
                self.enum_defs
                    .insert(e.name.clone(), (def_id, e.variants.clone()));
                def_id += 1;
            }
        }
        // Recolectar structures ÃƒÂ¢Ã¢â‚¬Â Ã¢â‚¬â„¢ offsets de campos (layout [def_id][len][campos]).
        let mut sdef_id = 0u32;
        for stmt in &module.statements {
            if let Statement::StructureDecl(s) = stmt {
                let mut fields = Vec::new();
                let mut offsets = Vec::new();
                let mut off = 16i64;
                for f in &s.fields {
                    let t = self.resolve_field_type(&s.name, &f.name, &f.type_ann)?;
                    let w = was_type(&t)?;
                    offsets.push(off);
                    fields.push((f.name.clone(), t, w));
                    off += elem_size_bytes(w);
                }
                self.struct_defs.insert(
                    s.name.clone(),
                    StructInfo {
                        def_id: sdef_id,
                        fields,
                        offsets,
                        total: off,
                    },
                );
                sdef_id += 1;
            }
        }
        // Recolectar clases ÃƒÂ¢Ã¢â‚¬Â Ã¢â‚¬â„¢ class_defs (layout de objeto) + declarar mÃƒÆ’Ã‚Â©todos/ctor.
        let mut next_class_id = 0u32;
        for stmt in &module.statements {
            if let Statement::ClassDecl(c) = stmt {
                let mut fields = Vec::new();
                let mut methods = Vec::new();
                let mut method_vis = std::collections::HashMap::new();
                let mut off = 16i64; // 0..7 = vtable, 8..15 = class_id
                let mut total = off;
                let mut ancestors = Vec::new();
                if let Some(parent) = &c.extends {
                    if let Some(pinfo) = self.class_defs.get(parent) {
                        fields.extend(pinfo.fields.clone());
                        methods = pinfo.methods.clone();
                        method_vis = pinfo.method_vis.clone();
                        off = pinfo.total;
                        total = pinfo.total;
                        ancestors.push(parent.clone());
                        ancestors.extend(pinfo.ancestors.clone());
                    }
                }
                for member in &c.body {
                    match member {
                        ClassMember::Property(p) if !p.is_static => {
                            let (w, t_cls) = match (&p.type_ann, &p.value) {
                                (Some(ann), _) => {
                                    let t = self.resolve_field_type(&c.name, &p.name, ann)?;
                                    let w = was_type(&t).unwrap_or(WasTy::I64);
                                    (w, t)
                                }
                                (None, Some(v)) => {
                                    let w = self.expr_was_type(v).unwrap_or(WasTy::I64);
                                    let t_cls = if matches!(w, WasTy::F64) {
                                        Type::Float
                                    } else {
                                        Type::Int
                                    };
                                    (w, t_cls)
                                }
                                (None, None) => (WasTy::I64, Type::Int),
                            };
                            let vis = FieldVis {
                                is_private: p.visibility == Visibility::Private,
                                is_protected: p.visibility == Visibility::Protected,
                                is_readonly: p.is_readonly,
                            };
                            fields.push((p.name.clone(), t_cls, w, off, vis));
                            off += elem_size_bytes(w);
                            total = off;
                        }
                        ClassMember::Method(m) => {
                            // Los mÃƒÆ’Ã‚Â©todos static NO van en la vtable (no reciben me).
                            if !m
                                .modifiers
                                .contains(&crate::frontend::ast::FunctionModifier::Static)
                            {
                                if !methods.contains(&m.name) {
                                    methods.push(m.name.clone());
                                }
                            }
                            let vis = FieldVis {
                                is_private: m.visibility == Visibility::Private,
                                is_protected: m.visibility == Visibility::Protected,
                                is_readonly: false,
                            };
                            method_vis.insert(m.name.clone(), vis);
                            let m2 = m.clone();
                            let cn = c.name.clone();
                            self.pending_class_methods.push((cn, m2));
                        }
                        ClassMember::Constructor(cf) => {
                            let mut c2 = cf.clone();
                            c2.name = "__ctor".to_string();
                            let cn = c.name.clone();
                            self.pending_class_methods.push((cn, c2));
                        }
                        _ => {}
                    }
                }
                let cid = next_class_id;
                next_class_id += 1;
                // El vtable_start se asigna AQUÃƒÆ’Ã‚Â (antes de compilar cuerpos): el
                // ctor del objeto lo lee al emitir, y no debe depender del orden
                // (no determinista) del HashMap.
                let vs = self.next_table_slot;
                self.next_table_slot += methods.len() as u32;
                self.class_defs.insert(
                    c.name.clone(),
                    ClassInfo {
                        parent: c.extends.clone(),
                        class_id: cid,
                        ancestors,
                        fields,
                        methods,
                        method_vis,
                        vtable_start: vs,
                        total,
                    },
                );
            }
        }
        // Recolectar extensiones ÃƒÂ¢Ã¢â‚¬Â Ã¢â‚¬â„¢ imports `env.<sym>__<sig>@<lib>`.
        for stmt in &module.statements {
            if let Statement::Extension(e) = stmt {
                for d in &e.declarations {
                    if let NativeDecl::Function(f) = d {
                        let mut params_was = Vec::new();
                        let mut params_code = String::new();
                        for p in &f.params {
                            let t = p
                                .type_ann
                                .as_ref()
                                .map(annotation_to_type)
                                .unwrap_or(Type::Int);
                            let (c, w) = ty_code(&t);
                            params_was.push(was_to_val(w));
                            params_code.push(c);
                        }
                        let ret_t = f
                            .return_type
                            .as_ref()
                            .map(annotation_to_type)
                            .unwrap_or(Type::Void);
                        let (rc, rw) = ty_code(&ret_t);
                        let results = if rc == 'v' {
                            vec![]
                        } else {
                            vec![was_to_val(rw)]
                        };
                        let import_name =
                            format!("{}__{}{}@{}", f.name, rc, params_code, e.library);
                        let tidx = self.register_func_type(params_was, results);
                        let idx = self.func_count;
                        self.func_count += 1;
                        self.imports_sec
                            .import("env", &import_name, EntityType::Function(tidx));
                        self.native_indexes.insert(f.name.clone(), idx);
                        self.native_ret.insert(f.name.clone(), rc);
                    }
                }
            }
        }

        use HostFn::*;
        for h in [
            PrintInt,
            PrintFloat,
            PrintBool,
            PrintChar,
            PrintStr,
            PrintEnd,
            Now,
            Exit,
            Sleep,
            Trap,
            ParseInt,
            ParseFloat,
            ParseBool,
            StrConcat,
            StrInt,
            StrFloat,
            StrBool,
            StrChar,
            PowNum,
            Fmod,
            Input,
            StrUpper,
            StrLower,
            StrTrim,
            StrContains,
            StrStartsWith,
            StrEndsWith,
            StrIsEmpty,
            StrLength,
            StrRepr,
            IntAbs,
            FloatAbs,
            ArrPush,
            ArrPop,
            ArrShift,
            ArrUnshift,
            ArrIndexOf,
            ArrIncludes,
            ArrJoin,
            ArrReverse,
            MathSqrt,
            MathPow,
            MathMin,
            MathMax,
            MathFloor,
            MathCeil,
            MathRound,
            MathRandom,
            MathSin,
            MathCos,
            MathTan,
            MathLog,
            MathRange,
            JsonStringify,
            JsonParse,
            FsExists,
            FsCwd,
            FsReadFile,
            FsWriteFile,
            FsListDir,
            FsMkdir,
            FsRm,
            RecordNew,
            RecordSet,
            RecordGet,
            RecordHas,
            RecordTag,
            RecordLen,
            RecordKeys,
            RecordValues,
            RecordToString,
            HttpGet,
            HttpPost,
            ArrToString,
            CmxNew,
            CmxSetProp,
            CmxAddChild,
            CmxToString,
            PrintAny,
            AnyMember,
            AnyIndex,
            FnHandle,
            FnToString,
            FnEnter,
            FnExit,
            CallSite,
            HostCall,
            OsPlatform,
            OsArch,
            OsVersion,
            OsHostname,
            OsHome,
            OsTempdir,
            OsCpus,
            OsPid,
            OsUptime,
            OsEnv,
            OsSep,
            OsIsWindows,
            OsIsUnix,
            PathJoin,
            PathBasename,
            PathDirname,
            PathExtname,
            PathResolve,
            PathNormalize,
            PathIsAbsolute,
            PathSep,
            ProcessArgs,
            ProcessCwd,
            ProcessEnv,
            ProcessExit,
            ProcessPid,
            ProcessPlatform,
            ProcessTitle,
            TimeNow,
            TimeSeconds,
            TimeIso,
            TimeDate,
            TimeClock,
            TimeYear,
            TimeMonth,
            TimeDay,
            TimeHour,
            TimeMinute,
            TimeSecond,
            TimeSleep,
            RandomRandom,
            RandomInt,
            RandomFloat,
            RandomUuid,
        ] {
            self.register_host(h);
        }

        // Tag de excepciÃƒÆ’Ã‚Â³n CLS: payload (msg: i64, span: i64). Solo en modo con
        // excepciones (wasmtime); en modo sin excepciones (wasmi) no hay tag, no
        // hay try_table y los `Throw` se emiten como `unreachable` (trap).
        if self.exceptions {
            let tag_ty = self.register_func_type(vec![ValType::I64, ValType::I64], vec![]);
            self.eh_handler_ty = self.register_func_type(vec![], vec![ValType::I64, ValType::I64]);
            self.tag_idx = self.tags_sec.len();
            self.tags_sec.tag(TagType {
                kind: TagKind::Exception,
                func_type_idx: tag_ty,
            });
        }

        // Memoria (1 pÃƒÆ’Ã‚Â¡gina = 64KB). MÃƒÆ’Ã‚Â­nimo 16 pÃƒÆ’Ã‚Â¡ginas (1MB): el string pool
        // (datos + tabla de ÃƒÆ’Ã‚Â­ndices) vive bajo el heap, que arranca en 1MB; el
        // allocator hace grow para el heap a partir de ahÃƒÆ’Ã‚Â­.
        self.memories_sec.memory(MemoryType {
            minimum: 16,
            maximum: None,
            memory64: false,
            shared: false,
            page_size_log2: None,
        });

        // Global: heap_ptr, mut, inicial 1MB (tras el string pool).
        self.globals_sec.global(
            GlobalType {
                val_type: ValType::I64,
                mutable: true,
                shared: false,
            },
            &ConstExpr::i64_const(1048576),
        );

        // Globals de usuario: `var x` / `const x` top-level ÃƒÂ¢Ã¢â‚¬Â Ã¢â‚¬â„¢ secciÃƒÆ’Ã‚Â³n globals.
        // ÃƒÆ’Ã‚Â­ndice 0 = heap_ptr; los de usuario empiezan en 1. Los `pool_seed`
        // (seed del string pool del REPL) NO crean global: sus strings se
        // internan en el pool, pero no deben ocupar ÃƒÆ’Ã‚Â­ndices de globals (los
        // ÃƒÆ’Ã‚Â­ndices de los vars de usuario se transfieren por posiciÃƒÆ’Ã‚Â³n entre
        // instancias y deben mantenerse estables).
        let mut next_global = 1u32;
        for stmt in &module.statements {
            if let Statement::VarDecl(v) | Statement::ConstDecl(v) = stmt {
                if v.pool_seed {
                    continue;
                }
                let w = match (&v.type_ann, &v.value) {
                    (Some(ann), _) => was_type(&annotation_to_type(ann)).unwrap_or(WasTy::I64),
                    (None, Some(val)) => self.expr_was_type(val).unwrap_or(WasTy::I64),
                    // Sin anotaciÃƒÆ’Ã‚Â³n ni init (REPL con estado persistente): el
                    // tipo viene del type map (registrado por el typeck en el
                    // span de la declaraciÃƒÆ’Ã‚Â³n original).
                    (None, None) => self
                        .types
                        .get(&v.span)
                        .and_then(|t| was_type(t).ok())
                        .unwrap_or(WasTy::I64),
                };
                let is_const = matches!(stmt, Statement::ConstDecl(_));
                let _ = is_const;
                let idx = next_global;
                next_global += 1;
                self.user_global_idxs.push(idx);
                self.globals.insert(v.name.clone(), idx);
                // mutable=true siempre: __init_globals las setea (incluso const, que
                // no se vuelve a escribir en runtime).
                self.globals_sec.global(
                    GlobalType {
                        val_type: w.val_type(),
                        mutable: true,
                        shared: false,
                    },
                    &match w {
                        WasTy::F64 => ConstExpr::f64_const(Ieee64::new(0.0f64.to_bits())),
                        WasTy::I32 => ConstExpr::i32_const(0),
                        WasTy::I64 => ConstExpr::i64_const(0),
                    },
                );
                if let Some(val) = &v.value {
                    // REPL: inits pool-only NO se ejecutan (el valor llega por
                    // transferencia de estado entre instancias); sus strings se
                    // siembran igualmente en el pool (bloque de seed abajo).
                    if !v.pool_only {
                        self.global_inits.push((idx, val.clone()));
                    }
                }
            }
        }

        // Campos estÃƒÆ’Ã‚Â¡ticos de clase: cada `static var` ÃƒÂ¢Ã¢â‚¬Â Ã¢â‚¬â„¢ un global WASM mutable
        // (accesible como `Clase.campo`). Se declaran tras los globals de usuario.
        for stmt in &module.statements {
            if let Statement::ClassDecl(c) = stmt {
                for member in &c.body {
                    if let ClassMember::Property(p) = member {
                        if p.is_static {
                            let w = match (&p.type_ann, &p.value) {
                                (Some(ann), _) => {
                                    was_type(&annotation_to_type(ann)).unwrap_or(WasTy::I64)
                                }
                                (None, Some(val)) => {
                                    self.expr_was_type(val).unwrap_or(WasTy::I64)
                                }
                                (None, None) => self
                                    .types
                                    .get(&p.span)
                                    .and_then(|t| was_type(t).ok())
                                    .unwrap_or(WasTy::I64),
                            };
                            let idx = next_global;
                            next_global += 1;
                            self.user_global_idxs.push(idx);
                            let key = format!("{}::{}", c.name, p.name);
                            self.static_fields.insert(key, idx);
                            self.globals_sec.global(
                                GlobalType {
                                    val_type: w.val_type(),
                                    mutable: true,
                                    shared: false,
                                },
                                &match w {
                                    WasTy::F64 => {
                                        ConstExpr::f64_const(Ieee64::new(0.0f64.to_bits()))
                                    }
                                    WasTy::I32 => ConstExpr::i32_const(0),
                                    WasTy::I64 => ConstExpr::i64_const(0),
                                },
                            );
                            if let Some(val) = &p.value {
                                self.global_inits.push((idx, val.clone()));
                            }
                        }
                    }
                }
            }
        }

        // Internas __alloc y __load_str.
        let alloc_idx = self.declare_wasm_function(vec![ValType::I64], vec![ValType::I64]);
        self.func_indexes.insert("__alloc".to_string(), alloc_idx);
        let ls_idx = self.declare_wasm_function(vec![ValType::I64], vec![ValType::I64]);
        self.func_indexes.insert("__load_str".to_string(), ls_idx);

        // __init_globals: se declara DESPUÃƒÆ’Ã¢â‚¬Â°S de alloc/load_str para que el code_sec
        // quede alineado (alloc, load_str, init, cls...).
        if !self.global_inits.is_empty() {
            let ig_idx = self.declare_wasm_function(vec![], vec![]);
            self.func_indexes
                .insert("__init_globals".to_string(), ig_idx);
        }

        // Seed del string pool (REPL con estado persistente): los inits de TODAS
        // las declaraciones top-level (reales y pool-only) se emiten a un buffer
        // descartado ANTES de compilar los cuerpos. AsÃƒÆ’Ã‚Â­ el pool queda con el
        // prefijo [inits de decls en orden de statements] idÃƒÆ’Ã‚Â©ntico entre sesiones
        // y los punteros de strings transferidos entre instancias siguen
        // siendo vÃƒÆ’Ã‚Â¡lidos (los cuerpos/init reales re-internan como no-op).
        let seed: Vec<Expression> = module
            .statements
            .iter()
            .filter_map(|s| match s {
                Statement::VarDecl(v) | Statement::ConstDecl(v) => v.value.clone(),
                _ => None,
            })
            .collect();
        if !seed.is_empty() {
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
            for init in &seed {
                let _ = fe.emit_expression(init);
            }
        }
        // MÃƒÆ’Ã‚Â©todos/ctor de clase: se declaran aquÃƒÆ’Ã‚Â­ (tras alloc/load_str/init) para
        // que el code_sec (que los compila despuÃƒÆ’Ã‚Â©s) quede alineado.
        let pending: Vec<(String, FunctionDecl)> = std::mem::take(&mut self.pending_class_methods);
        for (class, f) in pending {
            self.declare_class_function(&class, &f);
        }

        // Funciones CLS.
        let mut cls_funcs: Vec<FunctionDecl> = Vec::new();
        for stmt in &module.statements {
            if let Statement::FunctionDecl(f) = stmt {
                let (params, ret) = self.func_types[&f.name].clone();
                let mut pv: Vec<ValType> = Vec::new();
                // Firma uniforme (B5): toda funcion CLS top-level (excepto main,
                // entry del host) recibe __capturas (i64) como primer param.
                if f.name != "main" {
                    pv.push(ValType::I64);
                }
                for t in &params {
                    pv.push(was_type(t)?.val_type());
                }
                let rv: Vec<ValType> = match &ret {
                    Some(r) if *r != Type::Void => vec![was_type(r)?.val_type()],
                    _ => vec![],
                };
                // type index (para call_indirect) + ÃƒÆ’Ã‚Â­ndice de funciÃƒÆ’Ã‚Â³n.
                let tidx = self.register_func_type(pv.clone(), rv.clone());
                let fidx = self.func_count;
                self.func_count += 1;
                self.funcs_sec.function(tidx);
                self.func_indexes.insert(f.name.clone(), fidx);
                self.fn_type_indexes.insert(f.name.clone(), tidx);
                // MÃƒÆ’Ã‚Â³dulo importado (`mod::fn`): registrar el nombre base como
                // alias para que las llamadas internas del mÃƒÆ’Ã‚Â³dulo (`nivel1()`)
                // resuelvan sin prefijo (el body se fusiona tal cual).
                if let Some((_, base)) = f.name.split_once("::") {
                    if !self.func_indexes.contains_key(base) {
                        self.func_indexes.insert(base.to_string(), fidx);
                        self.fn_type_indexes.insert(base.to_string(), tidx);
                    }
                }
                cls_funcs.push(f.clone());
            }
        }
        // Modo librerÃƒÆ’Ã‚Â­a: declarar el main no-op sintetizado (firma (i64) -> i64).
        if !self.func_indexes.contains_key("main") {
            let tidx = self.register_func_type(vec![ValType::I64], vec![ValType::I64]);
            let fidx = self.func_count;
            self.func_count += 1;
            self.funcs_sec.function(tidx);
            self.func_indexes.insert("main".to_string(), fidx);
            self.fn_type_indexes.insert("main".to_string(), tidx);
            cls_funcs.push(noop_main_decl());
        }
        // ÃƒÆ’Ã‚Ândices de tabla de las funciones CLS (para handles de funciÃƒÆ’Ã‚Â³n) ÃƒÂ¢Ã¢â€šÂ¬Ã¢â‚¬Â se
        // calculan ANTES de compilar cuerpos (el emisor los usa en emit_ident_load).
        let mut cls_names: Vec<String> = self.func_types.keys().cloned().collect();
        cls_names.sort();
        let mut ti = self.next_table_slot;
        for name in &cls_names {
            if self.func_indexes.contains_key(name) {
                self.fn_table_idx.insert(name.clone(), ti);
                ti += 1;
            }
        }

        // Arrow functions (B5): recolectar de los cuerpos ÃƒÂ¢Ã¢â‚¬Â Ã¢â‚¬â„¢ funciones sintÃƒÆ’Ã‚Â©ticas
        // `__arrow_<n>`, declarar y asignarles ÃƒÆ’Ã‚Â­ndice de tabla.
        let mut arrow_funcs: Vec<FunctionDecl> = Vec::new();
        {
            let mut arrows: Vec<ArrowFunctionExpr> = Vec::new();
            for f in &cls_funcs {
                collect_arrows_in_block(&f.body, &mut arrows);
            }
            for (n, a) in arrows.iter().enumerate() {
                let name = format!("__arrow_{}", n);
                self.arrow_names.insert(a.span, name.clone());
                // Variables libres del body (closures): params y vars declaradas
                // dentro se excluyen; el resto son capturas.
                let mut locals: Vec<String> = a.params.iter().map(|p| p.name.clone()).collect();
                let mut free: Vec<String> = Vec::new();
                collect_free_vars_in_block(&a.body, &mut locals, &mut free);
                self.arrow_captures.insert(a.span, free.clone());
                arrow_funcs.push(FunctionDecl {
                    name,
                    params: a.params.clone(),
                    return_type: a.return_type.clone(),
                    body: (*a.body).clone(),
                    visibility: Visibility::Public,
                    modifiers: vec![],
                    span: a.span,
                    type_params: vec![],
                    is_native: false,
                });
            }
        }
        for f in &arrow_funcs {
            // Tipo del typeck (retorno inferido si la arrow no anota `-> T`).
            let arrow_ty = self.types.get(&f.span).cloned().unwrap_or(Type::Any);
            let (params, ret) = match arrow_ty {
                Type::Fun(p, r) => (p, *r),
                _ => {
                    let params: Vec<Type> = f
                        .params
                        .iter()
                        .map(|p| {
                            p.type_ann
                                .as_ref()
                                .map(annotation_to_type)
                                .unwrap_or(Type::Any)
                        })
                        .collect();
                    let ret = f
                        .return_type
                        .as_ref()
                        .map(annotation_to_type)
                        .unwrap_or(Type::Void);
                    (params, ret)
                }
            };
            self.func_types
                .insert(f.name.clone(), (params.clone(), Some(ret.clone())));
            let mut pv: Vec<ValType> = Vec::new();
            // Firma uniforme (B5): toda arrow recibe __capturas (i64) como primer param.
            pv.push(ValType::I64);
            for t in &params {
                pv.push(was_type(t)?.val_type());
            }
            let rv: Vec<ValType> = if ret != Type::Void {
                vec![was_type(&ret)?.val_type()]
            } else {
                vec![]
            };
            let tidx = self.register_func_type(pv.clone(), rv.clone());
            let fidx = self.func_count;
            self.func_count += 1;
            self.funcs_sec.function(tidx);
            self.func_indexes.insert(f.name.clone(), fidx);
            self.fn_type_indexes.insert(f.name.clone(), tidx);
            self.fn_table_idx.insert(f.name.clone(), ti);
            ti += 1;
        }

        // Compilar cuerpos (internan strings). El orden del code_sec DEBE coincidir
        // con el orden de declaraciÃƒÆ’Ã‚Â³n: alloc, load_str, [init], mÃƒÆ’Ã‚Â©todos, cls.
        let mut bodies: Vec<(String, Function)> = Vec::new();
        let extras: Vec<(String, FunctionDecl)> = self.cls_funcs_extra.clone();
        for (key, f) in &extras {
            let mut f2 = f.clone();
            f2.name = key.clone();
            let body = self.compile_function(&f2)?;
            bodies.push((key.clone(), body));
        }
        for f in &cls_funcs {
            let body = self.compile_function(f)?;
            bodies.push((f.name.clone(), body));
        }
        for f in &arrow_funcs {
            let body = self.compile_function(f)?;
            bodies.push((f.name.clone(), body));
        }

        // __alloc y __load_str (el pool de strings ya estÃƒÆ’Ã‚Â¡ completo).
        let alloc_body = self.build_allocator();
        let load_str_body = self.build_load_str();
        // __init_globals se construye ANTES del data segment: sus strings (valores
        // iniciales de las globals) deben internarse en el pool antes del data.
        let init_body = self.build_global_init()?;

        // Tabla de vtables: segmento con los funcref de los mÃƒÆ’Ã‚Â©todos de cada clase
        // (los vtable_start ya se asignaron en la recolecciÃƒÆ’Ã‚Â³n, en orden).
        // La ranura 0 se RESERVA (dummy) para que ningÃƒÆ’Ã‚Âºn handle par valga 0
        // (colisiÃƒÆ’Ã‚Â³n con Null); `next_table_slot` empieza en 1.
        let mut table_funcs: Vec<u32> = Vec::new();
        table_funcs.push(self.func_indexes["__alloc"]);
        let mut ordered: Vec<(u32, String)> = self
            .class_defs
            .iter()
            .map(|(n, i)| (i.vtable_start, n.clone()))
            .collect();
        ordered.sort_by_key(|(s, _)| *s);
        for (_, cn) in ordered {
            let methods: Vec<String> = self.class_defs[&cn].methods.clone();
            for m in &methods {
                if let Some(idx) = self.resolve_method_index(&cn, m) {
                    table_funcs.push(idx);
                }
            }
        }
        // Funciones CLS ÃƒÂ¢Ã¢â‚¬Â Ã¢â‚¬â„¢ handles (B5): los ÃƒÆ’Ã‚Â­ndices de tabla ya se calcularon.
        let mut cls_names: Vec<String> = self.fn_table_idx.keys().cloned().collect();
        cls_names.sort_by_key(|n| self.fn_table_idx[n]);
        for name in cls_names {
            if let Some(&fidx) = self.func_indexes.get(&name) {
                table_funcs.push(fidx);
            }
        }
        if !table_funcs.is_empty() {
            self.tables_sec.table(TableType {
                element_type: RefType::FUNCREF,
                table64: false,
                minimum: table_funcs.len() as u64,
                maximum: None,
                shared: false,
            });
            self.elements_sec.active(
                Some(0),
                &ConstExpr::i32_const(0),
                Elements::Functions(std::borrow::Cow::Borrowed(&table_funcs)),
            );
        }

        // Data segment con la tabla de strings.
        let data_bytes = self.build_string_data();
        self.data_sec.segment(DataSegment {
            mode: DataSegmentMode::Active {
                memory_index: 0,
                offset: &ConstExpr::i32_const(0),
            },
            data: data_bytes,
        });

        // Code section en el MISMO orden que las funciones: alloc, load_str, init, cls...
        self.code_sec.function(&alloc_body);
        self.code_sec.function(&load_str_body);
        if let Some(init) = init_body {
            self.code_sec.function(&init);
        }
        for (_name, body) in bodies {
            self.code_sec.function(&body);
        }

        // Exports.
        self.exports_sec
            .export("main", ExportKind::Func, self.func_indexes["main"]);
        self.exports_sec
            .export("alloc", ExportKind::Func, self.func_indexes["__alloc"]);
        self.exports_sec.export("memory", ExportKind::Memory, 0);

        // Globals de usuario exportadas como `__g_{idx}` (0 = heap_ptr, 1.. =
        // vars/consts/static fields). El host las usa para transferir estado
        // persistente entre instancias (REPL): leer del mÃƒÆ’Ã‚Â³dulo anterior y
        // escribir en el nuevo antes de llamar a `main`.
        self.exports_sec
            .export("__g_0", ExportKind::Global, 0);
        for &idx in &self.user_global_idxs {
            let name = format!("__g_{}", idx);
            self.exports_sec.export(&name, ExportKind::Global, idx);
        }

        // `export function f(...)` top-level ÃƒÂ¢Ã¢â‚¬Â Ã¢â‚¬â„¢ export WASM con su firma concreta
        // (el host la llama pasando `__capturas=0` como primer param). La firma
        // tipada viaja en la secciÃƒÆ’Ã‚Â³n custom `clx:exports` (JSON) para que el
        // host sepa el marshalling exacto de cada parÃƒÆ’Ã‚Â¡metro/retorno.
        let mut exports_meta: Vec<serde_json::Value> = Vec::new();
        for stmt in &module.statements {
            if let Statement::FunctionDecl(f) = stmt {
                if f.visibility != Visibility::Export || f.name == "main" {
                    continue;
                }
                if let Some(&fidx) = self.func_indexes.get(&f.name) {
                    self.exports_sec.export(&f.name, ExportKind::Func, fidx);
                }
                let (params, ret) = self.func_types[&f.name].clone();
                // elem kind de los arrays (para el marshalling del host): el
                // kind del array anidado; -1 si no es array.
                let elem_kind = |t: &Type| match t {
                    Type::Array(inner) => cls_kind_code(inner),
                    _ => -1,
                };
                // Descriptor recursivo de tipo para el marshalling del host:
                // `{"k": <kind>}` escalar; `{"k":5,"e":<desc>}` array; `{"k":6,
                // "v":<desc>}` record homogÃƒÆ’Ã‚Â©neo; `{"k":6,"s":{key:<desc>}}`
                // shape por clave. Permite decodificar arrays anidados en
                // records (la memoria del runtime no guarda el tipo del
                // elemento).
                fn type_desc(t: &Type) -> serde_json::Value {
                    let k = cls_kind_code(t);
                    match t {
                        Type::Array(inner) => {
                            serde_json::json!({"k": k, "e": type_desc(inner)})
                        }
                        Type::Record(_, v) => {
                            serde_json::json!({"k": k, "v": type_desc(v)})
                        }
                        Type::Shape(fields) => {
                            let map: serde_json::Map<String, serde_json::Value> =
                                fields.iter().map(|(n, t)| (n.clone(), type_desc(t))).collect();
                            serde_json::json!({"k": k, "s": serde_json::Value::Object(map)})
                        }
                        _ => serde_json::json!({"k": k}),
                    }
                }
                exports_meta.push(serde_json::json!({
                    "name": f.name,
                    "params": params.iter().map(cls_kind_code).collect::<Vec<i64>>(),
                    "pe": params.iter().map(elem_kind).collect::<Vec<i64>>(),
                    "ret": ret.as_ref().map(cls_kind_code).unwrap_or(9),
                    "re": ret.as_ref().map(elem_kind).unwrap_or(-1),
                    "pt": params.iter().map(type_desc).collect::<Vec<_>>(),
                    "rt": ret.as_ref().map(type_desc),
                }));
            }
        }
        if !exports_meta.is_empty() {
            self.exports_meta = serde_json::to_string(&exports_meta).unwrap_or_default().into_bytes();
        }

        Ok(self.build_module().finish())
    }

    pub(crate) fn compile_function(&mut self, f: &FunctionDecl) -> ClsResult<Function> {        let (param_types, _ret) = self.func_types[&f.name].clone();
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
        // MÃƒÆ’Ã‚Â©todos de clase: `me` (la instancia) es el primer param implÃƒÆ’Ã‚Â­cito.
        // `Clase::metodo` es mÃƒÆ’Ã‚Â©todo si el prefijo es una clase conocida; si no,
        // es un sÃƒÆ’Ã‚Â­mbolo de mÃƒÆ’Ã‚Â³dulo importado (`mod::fn`, sin `me`).
        let is_method = f
            .name
            .split("::")
            .next()
            .map(|c| self.class_defs.contains_key(c))
            .unwrap_or(false);
        // Un mÃƒÆ’Ã‚Â©todo static NO recibe `me` ni establece la clase actual (asÃƒÆ’Ã‚Â­ que
        // `me.` dentro de ÃƒÆ’Ã‚Â©l da error de variable no definida, paridad walker).
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
        let is_main = f.name == "main";
        // Promover al heap las variables locales capturadas por arrows del body:
        // para que la mutaciÃƒÆ’Ã‚Â³n del closure sea visible en el scope externo (paridad
        // con el walker, que captura por referencia). Aplica tambiÃƒÆ’Ã‚Â©n a main (que
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
            // Si esta funciÃƒÆ’Ã‚Â³n ES una arrow con capturas, sus capturas son
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
                fe.declare_var_ty(&p.name, was_type(&param_types[i])?);
            }
        }
        // main inicializa las globals top-level al arrancar.
        if f.name == "main" {
            if let Some(idx) = self.func_indexes.get("__init_globals") {
                fe.body.push(Instruction::Call(*idx));
            }
        }
        // Shadow call stack: registrar la entrada de la funciÃƒÆ’Ã‚Â³n (nombre + span)
        // y des-registrarla al salir (antes de cada End).
        fe.emit_fn_enter(f)?;
        for s in &f.body.statements {
            fe.emit_statement(s)?;
        }
        fe.emit_fn_exit();
        // End final del cuerpo de la funciÃƒÆ’Ã‚Â³n (wasm-encoder no lo aÃƒÆ’Ã‚Â±ade).
        fe.body.push(Instruction::End);
        // locals: cada ÃƒÆ’Ã‚Â­ndice con su tipo (fallback I64).
        // Importante: los params ocupan los ÃƒÆ’Ã‚Â­ndices 0..param_types.len(); los
        // locals declarados empiezan despuÃƒÆ’Ã‚Â©s. Cada local = un grupo de 1 para
        // preservar los ÃƒÆ’Ã‚Â­ndices exactos (agrupar reordenarÃƒÆ’Ã‚Â­a y romperÃƒÆ’Ã‚Â­a tipos
        // mixtos).
        let nparams = (param_types.len() + param_offset) as u32;
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

    /// Tipo WASM de una expresiÃƒÆ’Ã‚Â³n desde el type map (fallback I64).
    fn expr_was_type(&self, e: &Expression) -> ClsResult<WasTy> {
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

    /// ÃƒÆ’Ã‚Ândice de funciÃƒÆ’Ã‚Â³n de un mÃƒÆ’Ã‚Â©todo: en la clase o subiendo por ancestors.
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
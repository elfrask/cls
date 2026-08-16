//! emit.rs (Fase 1: extraido de cls-core/src/backend/wasm/engine/functions.rs).

use super::*;

impl<'a> Engine<'a> {


    pub(crate) fn emit(&mut self, module: &Module) -> ClsResult<Vec<u8>> {
        self.collect_functions(module)?;

        // Recolectar enums -> (def_id, variantes) para constantes `Nivel.Alto`.
        let mut def_id = 0u32;
        for stmt in &module.statements {
            if let Statement::EnumDecl(e) = stmt {
                self.enum_defs
                    .insert(e.name.clone(), (def_id, e.variants.clone()));
                def_id += 1;
            }
        }
        // Recolectar structures -> offsets de campos (layout [def_id][len][campos]).
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
        // Recolectar clases -> class_defs (layout de objeto) + declarar métodos/ctor.
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
                            // Los métodos static NO van en la vtable (no reciben me).
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
                // El vtable_start se asigna AQUÍ (antes de compilar cuerpos): el
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
        // Recolectar extensiones -> imports `env.<sym>__<sig>@<lib>`.
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
            // Nota: FnEnter/FnExit/CallSite ya NO se importan — el shadow call
            // stack vive en la memoria lineal del módulo (plan-performance/
            // shadow-stack-wasm.md).
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

        // Tag de excepció CLS: payload (msg: i64, span: i64). Solo en modo con
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

        // Memoria (1 página = 64KB). Mínimo 32 páginas (2MB): la ventana de
        // internals ocupa [0..1.11MB) + strings [1.11..1.56MB] + tabla
        // [1.56..1.81MB] + heap [1.81MB..] con grow a partir de ahí.
        self.memories_sec.memory(MemoryType {
            minimum: 32,
            maximum: None,
            memory64: false,
            shared: false,
            page_size_log2: None,
        });

        // Global: heap_ptr, mut, inicial HEAP_START (tras strings+tabla).
        self.globals_sec.global(
            GlobalType {
                val_type: ValType::I64,
                mutable: true,
                shared: false,
            },
            &ConstExpr::i64_const(HEAP_START as i64),
        );

        // Globals de usuario: `var x` / `const x` top-level -> secció globals.
        // índice 0 = heap_ptr; los de usuario empiezan en 1. Los `pool_seed`
        // (seed del string pool del REPL) NO crean global: sus strings se
        // internan en el pool, pero no deben ocupar índices de globals (los
        // índices de los vars de usuario se transfieren por posició entre
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
                    // Sin anotació ni init (REPL con estado persistente): el
                    // tipo viene del type map (registrado por el typeck en el
                    // span de la declaració original).
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

        // Campos estáticos de clase: cada `static var` -> un global WASM mutable
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

        // Globals del shadow call stack (plan-performance/shadow-stack-wasm.md):
        // `shadow_ptr` (tope del stack en memoria) + consts que el host lee para
        // resolver `idx → nombre` (tabla de strings) y la región de frames.
        // Todos por debajo del heap (1 MB): NO se transfieren en el REPL.
        let shadow_ptr_idx = next_global;
        next_global += 1;
        self.shadow_ptr_global = shadow_ptr_idx;
        self.globals_sec.global(
            GlobalType {
                val_type: ValType::I32,
                mutable: true,
                shared: false,
            },
            &ConstExpr::i32_const(SHADOW_STACK_BASE as i32),
        );
        for (gtype, init) in [
            (GlobalType { val_type: ValType::I32, mutable: false, shared: false }, SHADOW_STACK_BASE as i32),
            (GlobalType { val_type: ValType::I32, mutable: false, shared: false }, STRING_TABLE_BASE as i32),
            (GlobalType { val_type: ValType::I32, mutable: false, shared: false }, self.string_pool.len() as i32),
        ] {
            self.globals_sec.global(gtype, &ConstExpr::i32_const(init));
        }
        self.shadow_base_global = next_global;
        self.string_table_base_global = next_global + 1;
        self.string_pool_len_global = next_global + 2;

        // Internas __alloc y __load_str.
        let alloc_idx = self.declare_wasm_function(vec![ValType::I64], vec![ValType::I64]);
        self.func_indexes.insert("__alloc".to_string(), alloc_idx);
        let ls_idx = self.declare_wasm_function(vec![ValType::I64], vec![ValType::I64]);
        self.func_indexes.insert("__load_str".to_string(), ls_idx);

        // __init_globals: se declara DESPUÉS de alloc/load_str para que el code_sec
        // quede alineado (alloc, load_str, init, cls...).
        if !self.global_inits.is_empty() {
            let ig_idx = self.declare_wasm_function(vec![], vec![]);
            self.func_indexes
                .insert("__init_globals".to_string(), ig_idx);
        }

        // Seed del string pool (REPL con estado persistente): los inits de TODAS
        // las declaraciones top-level (reales y pool-only) se emiten a un buffer
        // descartado ANTES de compilar los cuerpos. Así el pool queda con el
        // prefijo [inits de decls en orden de statements] idéntico entre sesiones
        // y los punteros de strings transferidos entre instancias siguen
        // siendo válidos (los cuerpos/init reales re-internan como no-op).
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
        // Métodos/ctor de clase: se declaran aquí (tras alloc/load_str/init) para
        // que el code_sec (que los compila después) quede alineado.
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
                // type index (para call_indirect) + índice de funció.
                let tidx = self.register_func_type(pv.clone(), rv.clone());
                let fidx = self.func_count;
                self.func_count += 1;
                self.funcs_sec.function(tidx);
                self.func_indexes.insert(f.name.clone(), fidx);
                self.fn_type_indexes.insert(f.name.clone(), tidx);
                // Mí³dulo importado (`mod::fn`): registrar el nombre base como
                // alias para que las llamadas internas del mí³dulo (`nivel1()`)
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
        // Modo librería: declarar el main no-op sintetizado (firma (i64) -> i64).
        if !self.func_indexes.contains_key("main") {
            let tidx = self.register_func_type(vec![ValType::I64], vec![ValType::I64]);
            let fidx = self.func_count;
            self.func_count += 1;
            self.funcs_sec.function(tidx);
            self.func_indexes.insert("main".to_string(), fidx);
            self.fn_type_indexes.insert("main".to_string(), tidx);
            cls_funcs.push(noop_main_decl());
        }
        // Índices de tabla de las funciones CLS (para handles de funció) - se
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

        // Arrow functions (B5): recolectar de los cuerpos -> funciones sintéticas
        // `__arrow_<n>`, declarar y asignarles índice de tabla.
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
        // con el orden de declaració: alloc, load_str, [init], métodos, cls.
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

        // __alloc y __load_str (el pool de strings ya está completo).
        let alloc_body = self.build_allocator();
        let load_str_body = self.build_load_str();
        // __init_globals se construye ANTES del data segment: sus strings (valores
        // iniciales de las globals) deben internarse en el pool antes del data.
        let init_body = self.build_global_init()?;

        // Tabla de vtables: segmento con los funcref de los métodos de cada clase
        // (los vtable_start ya se asignaron en la recolecció, en orden).
        // La ranura 0 se RESERVA (dummy) para que ningíºn handle par valga 0
        // (colisió con Null); `next_table_slot` empieza en 1.
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
        // Funciones CLS -> handles (B5): los índices de tabla ya se calcularon.
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

        // Data segment con la tabla de strings: se coloca en STRING_DATA_BASE
        // (tras la ventana de internals [0..INTERNALS_WINDOW_END) — la fusión
        // de internals escribe su propio data segment en esa ventana).
        let data_bytes = self.build_string_data();
        self.data_sec.segment(DataSegment {
            mode: DataSegmentMode::Active {
                memory_index: 0,
                offset: &ConstExpr::i32_const(STRING_DATA_BASE as i32),
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

        // Fusión de internals: inyecta las funciones `__intr_*` en este módulo
        // (cero imports), compartiendo la memoria lineal. Registra sus índices
        // en func_indexes para que el emisor las llame por nombre (Paso 3).
        self.fuse_internals()?;

        // Exports.
        self.exports_sec
            .export("main", ExportKind::Func, self.func_indexes["main"]);
        self.exports_sec
            .export("alloc", ExportKind::Func, self.func_indexes["__alloc"]);
        self.exports_sec.export("memory", ExportKind::Memory, 0);
        // Shadow call stack: `__shadow_ptr` (tope de la región, mutable — el
        // host lo lee en el error) y consts para resolver idx → nombre.
        self.exports_sec
            .export("__shadow_ptr", ExportKind::Global, self.shadow_ptr_global);
        self.exports_sec
            .export("__shadow_base", ExportKind::Global, self.shadow_base_global);
        self.exports_sec
            .export("__string_table_base", ExportKind::Global, self.string_table_base_global);
        self.exports_sec
            .export("__string_pool_len", ExportKind::Global, self.string_pool_len_global);

        // Globals de usuario exportadas como `__g_{idx}` (0 = heap_ptr, 1.. =
        // vars/consts/static fields). El host las usa para transferir estado
        // persistente entre instancias (REPL): leer del mí³dulo anterior y
        // escribir en el nuevo antes de llamar a `main`.
        self.exports_sec
            .export("__g_0", ExportKind::Global, 0);
        for &idx in &self.user_global_idxs {
            let name = format!("__g_{}", idx);
            self.exports_sec.export(&name, ExportKind::Global, idx);
        }

        // `export function f(...)` top-level -> export WASM con su firma concreta
        // (el host la llama pasando `__capturas=0` como primer param). La firma
        // tipada viaja en la secció custom `clx:exports` (JSON) para que el
        // host sepa el marshalling exacto de cada parámetro/retorno.
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
                // "v":<desc>}` record homogéneo; `{"k":6,"s":{key:<desc>}}`
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

}
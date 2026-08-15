//! Motor de emisiï¿½n a nivel de mï¿½dulo (Fase 1: extraï¿½do de wasm/mod.rs).

use super::*;
pub(super) struct Engine<'a> {
    types: &'a HashMap<Span, Type>,
    // Builders de secciÃƒÂ³n persistentes: se agregan al mÃƒÂ³dulo en el orden WASM.
    types_sec: TypeSection,
    imports_sec: ImportSection,
    funcs_sec: FunctionSection,
    tables_sec: TableSection,
    memories_sec: MemorySection,
    tags_sec: TagSection,
    globals_sec: GlobalSection,
    exports_sec: ExportSection,
    data_sec: DataSection,
    code_sec: CodeSection,
    type_count: u32,
    func_count: u32,
    /// `true` = el mÃƒÂ³dulo lleva el tag de excepciÃƒÂ³n CLS (payload: msg + span) y
    /// los try/catch/throw funcionan (wasmtime). `false` = modo sin excepciones
    /// (wasmi): sin tag; errores de runtime como traps.
    pub(super) exceptions: bool,
    /// `true` = main obligatorio (modo app); `false` = se sintetiza main no-op
    /// (modo librerÃƒÂ­a, solo exports).
    pub(super) require_main: bool,
    /// Funciones host del nodo por nombre (canal `env.host_call`).
    pub(super) intrinsics: HashMap<String, HostIntrinsic>,
    /// Metadatos de los exports tipados (JSON, secciÃƒÂ³n custom `clx:exports`).
    exports_meta: Vec<u8>,
    /// ÃƒÂndice del tag de excepciÃƒÂ³n CLS (payload: msg + span).
    tag_idx: u32,
    /// Type `[] -> [i64, i64]` del block handler del try_table.
    eh_handler_ty: u32,
    func_indexes: HashMap<String, u32>,
    func_types: HashMap<String, (Vec<Type>, Option<Type>)>,
    func_defaults: HashMap<String, Vec<Option<Expression>>>,
    // B5: funciones como valor. Handle = [tabla_idx][capturas] (16 bytes).
    fn_table_idx: HashMap<String, u32>,
    fn_type_indexes: HashMap<String, u32>,
    /// arrow functions Ã¢â€ â€™ nombre sintÃƒÂ©tico `__arrow_<n>` (por span).
    arrow_names: HashMap<Span, String>,
    /// Variables libres capturadas por cada arrow (por span del ArrowFunctionExpr).
    arrow_captures: HashMap<Span, Vec<String>>,
    host_indexes: HashMap<HostFn, u32>,
    pub(super) string_pool: Vec<String>,
    string_index: HashMap<String, u32>,
    enum_defs: HashMap<String, (u32, Vec<String>)>,
    struct_defs: HashMap<String, StructInfo>,
    native_indexes: HashMap<String, u32>,
    native_ret: HashMap<String, char>,
    globals: HashMap<String, u32>,
    global_inits: Vec<(u32, Expression)>,
    /// ÃƒÂndices WASM de las globals de usuario (var/const top-level y static
    /// fields), en orden de declaraciÃƒÂ³n; el host las exporta como `__g_{idx}`
    /// (0 = heap_ptr) para transferir estado entre instancias (REPL).
    user_global_idxs: Vec<u32>,
    /// Campos estÃƒÂ¡ticos de clase: `Clase::campo` Ã¢â€ â€™ global WASM (mutable).
    static_fields: HashMap<String, u32>,
    elements_sec: ElementSection,
    class_defs: HashMap<String, ClassInfo>,
    next_table_slot: u32,
    /// Funciones de clase a compilar: (clave `Clase::m`, FunctionDecl).
    cls_funcs_extra: Vec<(String, FunctionDecl)>,
    /// type index WASM de cada mÃƒÂ©todo de clase (para `call_indirect`).
    method_type_indexes: HashMap<String, u32>,
    /// MÃƒÂ©todos de clase pendientes de declarar (tras alloc/load_str).
    pending_class_methods: Vec<(String, FunctionDecl)>,
    target: Target,
}

/// DefiniciÃƒÂ³n de una clase compilada: layout de objeto + vtable.
pub(super) struct FieldVis {
    pub(super)is_private: bool,
    pub(super)is_protected: bool,
    pub(super)is_readonly: bool,
}

impl Clone for FieldVis {
    fn clone(&self) -> Self {
        *self
    }
}
impl Copy for FieldVis {}

impl FieldVis {
    pub(super) fn is_private(&self) -> bool { self.is_private }
    pub(super) fn is_protected(&self) -> bool { self.is_protected }
    pub(super) fn is_readonly(&self) -> bool { self.is_readonly }
}

#[derive(Clone)]
pub(super) struct ClassInfo {
    pub(super)parent: Option<String>,
    /// id de clase (ÃƒÂ­ndice en orden de declaraciÃƒÂ³n) para `is` por herencia.
    pub(super)class_id: u32,
    /// cadena de ancestors: [padre, abuelo, ...].
    pub(super)ancestors: Vec<String>,
    /// campos (nombre, tipo CLS, tipo WASM, offset en bytes desde 16, visibilidad).
    pub(super)fields: Vec<(String, Type, WasTy, i64, FieldVis)>,
    /// nombres de mÃƒÂ©todos en orden canÃƒÂ³nico (posiciÃƒÂ³n = slot de la vtable).
    pub(super)methods: Vec<String>,
    /// visibilidad de cada mÃƒÂ©todo (private/protected/public) para enforzarla en
    /// llamadas desde fuera de la clase.
    pub(super)method_vis: std::collections::HashMap<String, FieldVis>,
    /// ÃƒÂ­ndice de la tabla donde empieza la vtable de esta clase.
    pub(super)vtable_start: u32,
    /// tamaÃƒÂ±o total del objeto (16 + campos).
    pub(super)total: i64,
}


/// DefiniciÃƒÂ³n de un structure compilada: campos con tipos, offsets y tamaÃƒÂ±o.
#[derive(Clone)]
pub(super) struct StructInfo {
    pub(super)def_id: u32,
    /// campos (nombre, tipo CLS, tipo WASM).
    pub(super)fields: Vec<(String, Type, WasTy)>,
    pub(super)offsets: Vec<i64>,
    pub(super)total: i64,
}

impl<'a> Engine<'a> {
    pub(super) fn new(types: &'a HashMap<Span, Type>, target: Target) -> Self {
        Self {
            types,
            types_sec: TypeSection::new(),
            imports_sec: ImportSection::new(),
            funcs_sec: FunctionSection::new(),
            memories_sec: MemorySection::new(),
            globals_sec: GlobalSection::new(),
            exports_sec: ExportSection::new(),
            data_sec: DataSection::new(),
            code_sec: CodeSection::new(),
            type_count: 0,
            func_count: 0,
            exceptions: true,
            require_main: true,
            intrinsics: HashMap::new(),
            exports_meta: Vec::new(),
            func_indexes: HashMap::new(),
            func_types: HashMap::new(),
            func_defaults: HashMap::new(),
            fn_table_idx: HashMap::new(),
            fn_type_indexes: HashMap::new(),
            arrow_names: HashMap::new(),
            arrow_captures: HashMap::new(),
            host_indexes: HashMap::new(),
            string_pool: Vec::new(),
            string_index: HashMap::new(),
            enum_defs: HashMap::new(),
            struct_defs: HashMap::new(),
            native_indexes: HashMap::new(),
            native_ret: HashMap::new(),
            globals: HashMap::new(),
        global_inits: Vec::new(),
        user_global_idxs: Vec::new(),
        static_fields: HashMap::new(),
            tables_sec: TableSection::new(),
            tags_sec: TagSection::new(),
            tag_idx: 0,
            eh_handler_ty: 0,
            elements_sec: ElementSection::new(),
            class_defs: HashMap::new(),
            next_table_slot: 1,
            cls_funcs_extra: Vec::new(),
            method_type_indexes: HashMap::new(),
            pending_class_methods: Vec::new(),
            target,
        }
    }

    fn register_func_type(&mut self, params: Vec<ValType>, results: Vec<ValType>) -> u32 {
        let idx = self.type_count;
        self.type_count += 1;
        self.types_sec.ty().function(params, results);
        idx
    }

    fn register_host(&mut self, h: HostFn) -> u32 {
        if let Some(idx) = self.host_indexes.get(&h) {
            return *idx;
        }
        let (params, results) = h.signature();
        let tidx = self.register_func_type(params.clone(), results.clone());
        let idx = self.func_count;
        self.func_count += 1;
        self.imports_sec
            .import("env", h.import_name(), EntityType::Function(tidx));
        self.host_indexes.insert(h, idx);
        idx
    }

    fn declare_wasm_function(&mut self, params: Vec<ValType>, results: Vec<ValType>) -> u32 {
        let tidx = self.register_func_type(params, results);
        let idx = self.func_count;
        self.func_count += 1;
        self.funcs_sec.function(tidx);
        idx
    }

    /// Agrega las secciones al mÃƒÂ³dulo en el orden WASM correcto.
    fn build_module(&mut self) -> WasmModule {
        let mut m = WasmModule::new();
        m.section(&self.types_sec);
        m.section(&self.imports_sec);
        m.section(&self.funcs_sec);
        m.section(&self.tables_sec);
        m.section(&self.memories_sec);
        // Solo en modo con excepciones: sin tag (wasmi) la secciÃƒÂ³n debe omitirse
        // (una secciÃƒÂ³n de tags vacÃƒÂ­a sigue siendo sintaxis de exception-handling).
        if self.exceptions {
            m.section(&self.tags_sec);
        }
        m.section(&self.globals_sec);
        m.section(&self.exports_sec);
        m.section(&self.elements_sec);
        m.section(&self.code_sec);
        m.section(&self.data_sec);
        // SecciÃƒÂ³n custom con las firmas tipadas de los exports (para el host
        // de bindings). Solo si hay `export function`.
        if !self.exports_meta.is_empty() {
            m.section(&CustomSection {
                name: "clx:exports".into(),
                data: std::borrow::Cow::Owned(self.exports_meta.clone()),
            });
        }
        m
    }

    fn collect_functions(&mut self, module: &Module) -> ClsResult<()> {
        for stmt in &module.statements {
            if let Statement::FunctionDecl(f) = stmt {
                self.collect_function(f)?;
            }
        }
        if !self.func_types.contains_key("main") {
            if self.require_main {
                return Err(crate::error::ClsError::CompileError(
                    "No se encontrÃƒÂ³ function main(args: String[]) para el JIT".to_string(),
                ));
            }
            // Modo librerÃƒÂ­a: main no-op sintetizado (el host lo llama con args=0).
            self.func_types.insert(
                "main".to_string(),
                (vec![Type::Array(Box::new(Type::String))], Some(Type::Int)),
            );
            self.func_defaults.insert("main".to_string(), vec![None]);
        }
        Ok(())
    }

    fn collect_function(&mut self, f: &FunctionDecl) -> ClsResult<()> {
        let mut params: Vec<Type> = Vec::new();
        let mut defaults: Vec<Option<Expression>> = Vec::new();
        for p in &f.params {
            let t = p.type_ann.as_ref().ok_or_else(|| {
                crate::error::ClsError::CompileError(format!(
                    "ParÃƒÂ¡metro '{}' de '{}' sin anotaciÃƒÂ³n de tipo (requerido por el JIT)",
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
                "AnotaciÃƒÂ³n de tipo no soportada por el JIT (se requiere tipo concreto)".to_string(),
            )),
            other => Ok(other),
        }
    }

    /// Tipo concreto de un campo de struct/clase. Si la anotaciÃƒÂ³n no resuelve a
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

    pub(super) fn emit(&mut self, module: &Module) -> ClsResult<Vec<u8>> {
        self.collect_functions(module)?;

        // Recolectar enums Ã¢â€ â€™ (def_id, variantes) para constantes `Nivel.Alto`.
        let mut def_id = 0u32;
        for stmt in &module.statements {
            if let Statement::EnumDecl(e) = stmt {
                self.enum_defs
                    .insert(e.name.clone(), (def_id, e.variants.clone()));
                def_id += 1;
            }
        }
        // Recolectar structures Ã¢â€ â€™ offsets de campos (layout [def_id][len][campos]).
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
        // Recolectar clases Ã¢â€ â€™ class_defs (layout de objeto) + declarar mÃƒÂ©todos/ctor.
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
                            // Los mÃƒÂ©todos static NO van en la vtable (no reciben me).
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
                // El vtable_start se asigna AQUÃƒÂ (antes de compilar cuerpos): el
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
        // Recolectar extensiones Ã¢â€ â€™ imports `env.<sym>__<sig>@<lib>`.
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

        // Tag de excepciÃƒÂ³n CLS: payload (msg: i64, span: i64). Solo en modo con
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

        // Memoria (1 pÃƒÂ¡gina = 64KB). MÃƒÂ­nimo 16 pÃƒÂ¡ginas (1MB): el string pool
        // (datos + tabla de ÃƒÂ­ndices) vive bajo el heap, que arranca en 1MB; el
        // allocator hace grow para el heap a partir de ahÃƒÂ­.
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

        // Globals de usuario: `var x` / `const x` top-level Ã¢â€ â€™ secciÃƒÂ³n globals.
        // ÃƒÂ­ndice 0 = heap_ptr; los de usuario empiezan en 1. Los `pool_seed`
        // (seed del string pool del REPL) NO crean global: sus strings se
        // internan en el pool, pero no deben ocupar ÃƒÂ­ndices de globals (los
        // ÃƒÂ­ndices de los vars de usuario se transfieren por posiciÃƒÂ³n entre
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
                    // Sin anotaciÃƒÂ³n ni init (REPL con estado persistente): el
                    // tipo viene del type map (registrado por el typeck en el
                    // span de la declaraciÃƒÂ³n original).
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

        // Campos estÃƒÂ¡ticos de clase: cada `static var` Ã¢â€ â€™ un global WASM mutable
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

        // __init_globals: se declara DESPUÃƒâ€°S de alloc/load_str para que el code_sec
        // quede alineado (alloc, load_str, init, cls...).
        if !self.global_inits.is_empty() {
            let ig_idx = self.declare_wasm_function(vec![], vec![]);
            self.func_indexes
                .insert("__init_globals".to_string(), ig_idx);
        }

        // Seed del string pool (REPL con estado persistente): los inits de TODAS
        // las declaraciones top-level (reales y pool-only) se emiten a un buffer
        // descartado ANTES de compilar los cuerpos. AsÃƒÂ­ el pool queda con el
        // prefijo [inits de decls en orden de statements] idÃƒÂ©ntico entre sesiones
        // y los punteros de strings transferidos entre instancias siguen
        // siendo vÃƒÂ¡lidos (los cuerpos/init reales re-internan como no-op).
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
        // MÃƒÂ©todos/ctor de clase: se declaran aquÃƒÂ­ (tras alloc/load_str/init) para
        // que el code_sec (que los compila despuÃƒÂ©s) quede alineado.
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
                // type index (para call_indirect) + ÃƒÂ­ndice de funciÃƒÂ³n.
                let tidx = self.register_func_type(pv.clone(), rv.clone());
                let fidx = self.func_count;
                self.func_count += 1;
                self.funcs_sec.function(tidx);
                self.func_indexes.insert(f.name.clone(), fidx);
                self.fn_type_indexes.insert(f.name.clone(), tidx);
                // MÃƒÂ³dulo importado (`mod::fn`): registrar el nombre base como
                // alias para que las llamadas internas del mÃƒÂ³dulo (`nivel1()`)
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
        // Modo librerÃƒÂ­a: declarar el main no-op sintetizado (firma (i64) -> i64).
        if !self.func_indexes.contains_key("main") {
            let tidx = self.register_func_type(vec![ValType::I64], vec![ValType::I64]);
            let fidx = self.func_count;
            self.func_count += 1;
            self.funcs_sec.function(tidx);
            self.func_indexes.insert("main".to_string(), fidx);
            self.fn_type_indexes.insert("main".to_string(), tidx);
            cls_funcs.push(noop_main_decl());
        }
        // ÃƒÂndices de tabla de las funciones CLS (para handles de funciÃƒÂ³n) Ã¢â‚¬â€ se
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

        // Arrow functions (B5): recolectar de los cuerpos Ã¢â€ â€™ funciones sintÃƒÂ©ticas
        // `__arrow_<n>`, declarar y asignarles ÃƒÂ­ndice de tabla.
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
        // con el orden de declaraciÃƒÂ³n: alloc, load_str, [init], mÃƒÂ©todos, cls.
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

        // __alloc y __load_str (el pool de strings ya estÃƒÂ¡ completo).
        let alloc_body = self.build_allocator();
        let load_str_body = self.build_load_str();
        // __init_globals se construye ANTES del data segment: sus strings (valores
        // iniciales de las globals) deben internarse en el pool antes del data.
        let init_body = self.build_global_init()?;

        // Tabla de vtables: segmento con los funcref de los mÃƒÂ©todos de cada clase
        // (los vtable_start ya se asignaron en la recolecciÃƒÂ³n, en orden).
        // La ranura 0 se RESERVA (dummy) para que ningÃƒÂºn handle par valga 0
        // (colisiÃƒÂ³n con Null); `next_table_slot` empieza en 1.
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
        // Funciones CLS Ã¢â€ â€™ handles (B5): los ÃƒÂ­ndices de tabla ya se calcularon.
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
        // persistente entre instancias (REPL): leer del mÃƒÂ³dulo anterior y
        // escribir en el nuevo antes de llamar a `main`.
        self.exports_sec
            .export("__g_0", ExportKind::Global, 0);
        for &idx in &self.user_global_idxs {
            let name = format!("__g_{}", idx);
            self.exports_sec.export(&name, ExportKind::Global, idx);
        }

        // `export function f(...)` top-level Ã¢â€ â€™ export WASM con su firma concreta
        // (el host la llama pasando `__capturas=0` como primer param). La firma
        // tipada viaja en la secciÃƒÂ³n custom `clx:exports` (JSON) para que el
        // host sepa el marshalling exacto de cada parÃƒÂ¡metro/retorno.
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
                // "v":<desc>}` record homogÃƒÂ©neo; `{"k":6,"s":{key:<desc>}}`
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

    fn compile_function(&mut self, f: &FunctionDecl) -> ClsResult<Function> {        let (param_types, _ret) = self.func_types[&f.name].clone();
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
        // MÃƒÂ©todos de clase: `me` (la instancia) es el primer param implÃƒÂ­cito.
        // `Clase::metodo` es mÃƒÂ©todo si el prefijo es una clase conocida; si no,
        // es un sÃƒÂ­mbolo de mÃƒÂ³dulo importado (`mod::fn`, sin `me`).
        let is_method = f
            .name
            .split("::")
            .next()
            .map(|c| self.class_defs.contains_key(c))
            .unwrap_or(false);
        // Un mÃƒÂ©todo static NO recibe `me` ni establece la clase actual (asÃƒÂ­ que
        // `me.` dentro de ÃƒÂ©l da error de variable no definida, paridad walker).
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
        // para que la mutaciÃƒÂ³n del closure sea visible en el scope externo (paridad
        // con el walker, que captura por referencia). Aplica tambiÃƒÂ©n a main (que
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
            // Si esta funciÃƒÂ³n ES una arrow con capturas, sus capturas son
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
        // Shadow call stack: registrar la entrada de la funciÃƒÂ³n (nombre + span)
        // y des-registrarla al salir (antes de cada End).
        fe.emit_fn_enter(f)?;
        for s in &f.body.statements {
            fe.emit_statement(s)?;
        }
        fe.emit_fn_exit();
        // End final del cuerpo de la funciÃƒÂ³n (wasm-encoder no lo aÃƒÂ±ade).
        fe.body.push(Instruction::End);
        // locals: cada ÃƒÂ­ndice con su tipo (fallback I64).
        // Importante: los params ocupan los ÃƒÂ­ndices 0..param_types.len(); los
        // locals declarados empiezan despuÃƒÂ©s. Cada local = un grupo de 1 para
        // preservar los ÃƒÂ­ndices exactos (agrupar reordenarÃƒÂ­a y romperÃƒÂ­a tipos
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

    /// Tipo WASM de una expresiÃƒÂ³n desde el type map (fallback I64).
    fn expr_was_type(&self, e: &Expression) -> ClsResult<WasTy> {
        let span = expr_span(e);
        if let Some(t) = self.types.get(&span) {
            was_type(t)
        } else {
            Ok(WasTy::I64)
        }
    }

    /// `__init_globals`: setea cada global de usuario con su valor inicial.
    fn build_global_init(&mut self) -> ClsResult<Option<Function>> {
        if self.global_inits.is_empty() {
            return Ok(None);
        }
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
        for (idx, val) in &self.global_inits {
            fe.emit_expression(val)?;
            fe.body.push(Instruction::GlobalSet(*idx));
        }
        fe.body.push(Instruction::End);
        // Declarar los temporales que la emisiÃƒÂ³n pudo crear (emit_array, etc.).
        let local_types: Vec<ValType> = (0..fe.next_local)
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
        Ok(Some(func))
    }

    /// Declara una funciÃƒÂ³n de clase (`Clase::m` o ctor) con `me` como primer param.
    /// Los mÃƒÂ©todos `static` NO reciben `me` (se registran como `Clase::__s__m`).
    fn declare_class_function(&mut self, class: &str, f: &FunctionDecl) {
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

    /// ÃƒÂndice de funciÃƒÂ³n de un mÃƒÂ©todo: en la clase o subiendo por ancestors.
    fn resolve_method_index(&self, class: &str, m: &str) -> Option<u32> {
        let mut cur = Some(class.to_string());
        while let Some(c) = cur {
            if let Some(idx) = self.func_indexes.get(&format!("{}::{}", c, m)) {
                return Some(*idx);
            }
            cur = self.class_defs.get(&c).and_then(|i| i.parent.clone());
        }
        None
    }

    fn build_allocator(&self) -> Function {
        // (func (param $n i64) (result i64)
        //   local 0 = n (param), local 1 = ptr, local 2 = end
        //   ptr = global 0
        //   end = (ptr + n + 8) & -8
        //   if end > memsize*65536 Ã¢â€ â€™ grow las pÃƒÂ¡ginas exactas para cubrir `end`
        //   global 0 = end
        //   ptr)
        let mut b = vec![
            Instruction::GlobalGet(0),
            Instruction::LocalSet(1),
            Instruction::LocalGet(1),
            Instruction::LocalGet(0),
            Instruction::I64Add,
            Instruction::I64Const(8),
            Instruction::I64Add,
            Instruction::I64Const(-8),
            Instruction::I64And,
            Instruction::LocalSet(2),
            Instruction::Block(BlockType::Empty),
            // if end <= memsize*65536 Ã¢â€ â€™ skip grow
            Instruction::LocalGet(2),
            Instruction::MemorySize(0),
            Instruction::I64ExtendI32U,
            Instruction::I64Const(65536),
            Instruction::I64Mul,
            Instruction::I64LeU,
            Instruction::BrIf(0),
            // pages_needed = ceil((end - memsize*65536) / 65536)
            Instruction::LocalGet(2),
            Instruction::MemorySize(0),
            Instruction::I64ExtendI32U,
            Instruction::I64Const(65536),
            Instruction::I64Mul,
            Instruction::I64Sub,
            Instruction::I64Const(65535),
            Instruction::I64Add,
            Instruction::I64Const(65536),
            Instruction::I64DivU,
            Instruction::I32WrapI64,
            Instruction::MemoryGrow(0),
            Instruction::Drop,
            Instruction::End,
            Instruction::LocalGet(2),
            Instruction::GlobalSet(0),
            Instruction::LocalGet(1),
            Instruction::End,
        ];
        let mut func = Function::new(vec![(2, ValType::I64)]);
        for inst in b.drain(..) {
            func.instruction(&inst);
        }
        func
    }

    fn build_load_str(&self) -> Function {
        // (func (param $i i64) (result i64)
        //   local 0 = i (param), 1 = entry, 2 = off, 3 = len
        //   entry = STRING_TABLE_BASE + i*8
        //   off = i32.load(entry)
        //   len = i32.load(entry+4)
        //   result = (off << 32) | len)
        let mut b = vec![
            Instruction::LocalGet(0),
            Instruction::I64Const(8),
            Instruction::I64Mul,
            Instruction::I64Const(STRING_TABLE_BASE as i64),
            Instruction::I64Add,
            Instruction::LocalSet(1),
            Instruction::LocalGet(1),
            Instruction::I32WrapI64,
            Instruction::I32Load(MemArg {
                offset: 0,
                align: 2,
                memory_index: 0,
            }),
            Instruction::I64ExtendI32U,
            Instruction::LocalSet(2),
            Instruction::LocalGet(1),
            Instruction::I64Const(4),
            Instruction::I64Add,
            Instruction::I32WrapI64,
            Instruction::I32Load(MemArg {
                offset: 0,
                align: 2,
                memory_index: 0,
            }),
            Instruction::I64ExtendI32U,
            Instruction::LocalSet(3),
            Instruction::LocalGet(2),
            Instruction::I64Const(32),
            Instruction::I64Shl,
            Instruction::LocalGet(3),
            Instruction::I64Or,
            Instruction::End,
        ];
        let mut func = Function::new(vec![(3, ValType::I64)]);
        for inst in b.drain(..) {
            func.instruction(&inst);
        }
        func
    }

    fn build_string_data(&self) -> Vec<u8> {
        let data_bytes: usize = self.string_pool.iter().map(|s| s.len()).sum();
        // El layout es: [0 .. data_len) = bytes de los strings (en orden de
        // interning, append-only) y [STRING_TABLE_BASE .. + 8N) = tabla de
        // ÃƒÂ­ndices (offset, len). Con base FIJA, los offsets de los datos NO
        // dependen del tamaÃƒÂ±o total del pool: el REPL (estado persistente)
        // transfiere punteros entre instancias y estos siguen siendo vÃƒÂ¡lidos
        // mientras las entradas compartidas conserven su posiciÃƒÂ³n (prefix).
        assert!(
            data_bytes <= STRING_TABLE_BASE as usize,
            "el string pool excede la regiÃƒÂ³n de datos ({} > {} bytes)",
            data_bytes,
            STRING_TABLE_BASE
        );
        let mut bytes: Vec<u8> =
            vec![0u8; STRING_TABLE_BASE as usize + self.string_pool.len() * 8];
        let mut data_off = 0u32;
        for s in self.string_pool.iter() {
            bytes[data_off as usize..data_off as usize + s.len()].copy_from_slice(s.as_bytes());
            data_off += s.len() as u32;
        }
        let mut entry = STRING_TABLE_BASE as usize;
        let mut off = 0u32;
        for s in self.string_pool.iter() {
            let len = s.len() as u32;
            bytes[entry..entry + 4].copy_from_slice(&off.to_le_bytes());
            bytes[entry + 4..entry + 8].copy_from_slice(&len.to_le_bytes());
            off += len;
            entry += 8;
        }
        bytes
    }
}

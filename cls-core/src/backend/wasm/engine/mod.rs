//! Motor de emision a nivel de modulo (Fase 1: extraido de wasm/mod.rs).

mod functions;
mod metadata;
pub(crate) use metadata::{ClassInfo, FieldVis, StructInfo};
mod globals;

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
    pub(crate) exceptions: bool,
    /// `true` = main obligatorio (modo app); `false` = se sintetiza main no-op
    /// (modo librerÃƒÂ­a, solo exports).
    pub(crate) require_main: bool,
    /// Funciones host del nodo por nombre (canal `env.host_call`).
    pub(crate) intrinsics: HashMap<String, HostIntrinsic>,
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
    pub(crate) string_pool: Vec<String>,
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
impl<'a> Engine<'a> {
    pub(crate) fn new(types: &'a HashMap<Span, Type>, target: Target) -> Self {
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



}

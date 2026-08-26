//! Emisor de funciones CLS a WASM (Fase 1: extraido de wasm/mod.rs).

use super::*;

mod assignment;
mod binary;
mod calls;
mod classes;
mod containers;
mod expressions;
mod foreach;
mod member;
mod module_calls;
mod print;
mod primitives;
mod statements;
mod strings;

/// Contexto de un loop para resolver `break`/`continue`.
pub(crate) struct LoopGuard {
    break_at: u32,
    continue_at: u32,
}
pub(crate) struct HostCaller {
    pub(crate) indexes: HashMap<HostFn, u32>,
}

impl HostCaller {
    fn call(&self, h: HostFn, body: &mut Vec<Instruction<'static>>) {
        let idx = self.indexes[&h];
        body.push(Instruction::Call(idx));
    }
}

/// Emisor con el estado de compilación de una función.
pub(crate) struct FuncEmitter<'a> {
    types: &'a HashMap<Span, Type>,
    pub(crate) body: Vec<Instruction<'static>>,
    locals: HashMap<String, u32>,
    pub(crate) local_tys: HashMap<u32, WasTy>,
    pub(crate) next_local: u32,
    block_depth: u32,
    loop_stack: Vec<LoopGuard>,
    host: HostCaller,
    string_pool: &'a mut Vec<String>,
    string_index: &'a mut HashMap<String, u32>,
    func_indexes: &'a HashMap<String, u32>,
    func_defaults: &'a HashMap<String, Vec<Option<Expression>>>,
    fn_table_idx: &'a HashMap<String, u32>,
    arrow_names: &'a HashMap<Span, String>,
    arrow_captures: &'a HashMap<Span, Vec<String>>,
    /// Registro de types dinámicos (call_indirect de funciones como valor).
    type_count: &'a mut u32,
    types_sec: &'a mut TypeSection,
    enum_defs: &'a HashMap<String, (u32, Vec<String>)>,
    struct_defs: &'a HashMap<String, StructInfo>,
    native_indexes: &'a HashMap<String, u32>,
    native_ret: &'a HashMap<String, char>,
    globals: &'a HashMap<String, u32>,
    static_fields: &'a HashMap<String, u32>,
    class_defs: &'a HashMap<String, ClassInfo>,
    method_type_indexes: &'a HashMap<String, u32>,
    /// Firmas tipadas de las funciones (`Clase::m` -> (params, ret)) - el retorno
    /// de los magic methods (el call_indirect lo produce según la firma).
    func_types: &'a HashMap<String, (Vec<Type>, Option<Type>)>,
    /// clase actual (al compilar un método) - para `super` y `me`.
    pub(crate) current_class: Option<String>,
    /// nombre del método que se está compilando (sin el prefijo de clase), para
    /// el protocolo `__next` (el `null` termina la iteración con un sentinel
    /// distinto de 0 - un iterador puede devolver 0 como valor legítimo).
    pub(crate) current_method: Option<String>,
    /// Span de la función que se está compilando (para errores de statements sin
    /// span propio, p.ej. `break`/`continue` fuera de loop).
    pub(crate) current_fn_span: Span,
    /// Tipo de retorno declarado de la función/método en compilación (frontera
    /// de `return`: convertir Shape→hashmap si el retorno es dinámico).
    pub(crate) fn_ret: Option<Type>,
    /// Tipos CLS declarados de las variables locales del cuerpo (nombre -> tipo,
    /// por anotación o inferencia registrada al compilar su VarDecl) — frontera
    /// de asignación `destino = valor`.
    pub(crate) local_cls_types: HashMap<String, Type>,
    target: &'a Target,
    /// Índice del tag de excepción CLS (para `Instruction::Throw`).
    tag_idx: u32,
    /// Type `[] -> [i64, i64]` del block handler del try_table.
    eh_handler_ty: u32,
    /// `true` = excepciones CLS habilitadas (wasmtime); `false` = modo sin
    /// excepciones (wasmi): try/catch/throw fallan y los errores de runtime se
    /// emiten como `unreachable` (trap).
    exceptions: bool,
    /// Funciones host del nodo por nombre (canal `env.host_call`).
    intrinsics: &'a HashMap<String, HostIntrinsic>,
    /// Closures (B5): nombre de variable capturada -> índice 1-based en el bloque
    /// de capturas `[n, v1, v2, ...]`. El param 0 del frame es `__capturas` (ptr).
    pub(crate) captures: HashMap<String, u32>,
    /// Variables promovidas al heap (capturadas por una arrow del scope): el
    /// local guarda un PTR a un slot `[valor]`; los accesos pasan por ahí.
    pub(crate) promoted: HashSet<String>,
    /// Índice del global WASM `shadow_ptr` (tope del shadow call stack). Lo
    /// setea el engine después de `new` (el índice se conoce al emitir los
    /// globals). 0 mientras no se instrumente.
    pub(crate) shadow_ptr_global: u32,
    /// `true` = el flujo ya no puede continuar (return/break/continue o if con
    /// todas las ramas terminadas). Los statements siguientes se omiten y el
    /// cierre de la función emite `unreachable` (evita código muerto inválido
    /// tras un `end` de if/switch/try con todas las ramas terminadas, que
    /// cranelift rechaza con "expected i64 but nothing on stack").
    pub(crate) dead_flow: bool,
}

impl<'a> FuncEmitter<'a> {
    pub(crate) fn new(
        types: &'a HashMap<Span, Type>,
        host: HostCaller,
        string_pool: &'a mut Vec<String>,
        string_index: &'a mut HashMap<String, u32>,
        func_indexes: &'a HashMap<String, u32>,
        func_defaults: &'a HashMap<String, Vec<Option<Expression>>>,
        fn_table_idx: &'a HashMap<String, u32>,
        arrow_names: &'a HashMap<Span, String>,
        arrow_captures: &'a HashMap<Span, Vec<String>>,
        type_count: &'a mut u32,
        types_sec: &'a mut TypeSection,
        enum_defs: &'a HashMap<String, (u32, Vec<String>)>,
        struct_defs: &'a HashMap<String, StructInfo>,
        native_indexes: &'a HashMap<String, u32>,
        native_ret: &'a HashMap<String, char>,
        globals: &'a HashMap<String, u32>,
        static_fields: &'a HashMap<String, u32>,
        class_defs: &'a HashMap<String, ClassInfo>,
        method_type_indexes: &'a HashMap<String, u32>,
        func_types: &'a HashMap<String, (Vec<Type>, Option<Type>)>,
    current_class: Option<String>,
        target: &'a Target,
        tag_idx: u32,
        eh_handler_ty: u32,
        exceptions: bool,
        intrinsics: &'a HashMap<String, HostIntrinsic>,
    ) -> Self {
        Self {
            types,
            body: Vec::new(),
            locals: HashMap::new(),
            local_tys: HashMap::new(),
            next_local: 0,
            block_depth: 0,
            loop_stack: Vec::new(),
            host,
            string_pool,
            string_index,
            func_indexes,
            func_defaults,
            fn_table_idx,
            arrow_names,
            arrow_captures,
            type_count,
            types_sec,
            enum_defs,
            struct_defs,
            native_indexes,
            native_ret,
            globals,
            static_fields,
            class_defs,
            method_type_indexes,
            func_types,
            current_class,
            current_method: None,
            current_fn_span: Span::new(1, 1, 1, 1),
            fn_ret: None,
            local_cls_types: HashMap::new(),
            target,
            tag_idx,
            eh_handler_ty,
            exceptions,
            intrinsics,
            captures: HashMap::new(),
            promoted: HashSet::new(),
            shadow_ptr_global: 0,
            dead_flow: false,
        }
    }


    pub(crate) fn fresh_local_ty(&mut self, ty: WasTy) -> u32 {
        let l = self.next_local;
        self.next_local += 1;
        self.local_tys.insert(l, ty);
        l
    }


    /// Registra un type de función (para `call_indirect` de funciones como valor).
    pub(crate) fn register_func_type(&mut self, params: Vec<ValType>, results: Vec<ValType>) -> u32 {
        let idx = *self.type_count;
        *self.type_count += 1;
        self.types_sec.ty().function(params, results);
        idx
    }


    pub(crate) fn fresh_local(&mut self) -> u32 {
        self.fresh_local_ty(WasTy::I64)
    }


    pub(crate) fn local_for(&mut self, name: &str) -> u32 {
        *self.locals.entry(name.to_string()).or_insert_with(|| {
            let l = self.next_local;
            self.next_local += 1;
            l
        })
    }


    /// Carga un identificador: global (si es un `export var` top-level) o local.
    pub(crate) fn emit_ident_load(&mut self, name: &str) {
        if name == "super" && self.current_class.is_some() {
            self.body.push(Instruction::LocalGet(0));
        } else if let Some(&ti) = self.fn_table_idx.get(name) {
            // Función CLS como valor -> handle [tabla_idx][capturas=0][nombre]
            // con tag-bit (ptr<<1)|1. El nombre se guarda para fn_to_string.
            let n = self.intern_string(&format!("<function {}>", name));
            self.body.push(Instruction::I64Const(ti as i64));
            self.emit_load_str(n);
            self.body.push(Instruction::I64Const(0));
            self.host.call(HostFn::FnHandle, &mut self.body);
        } else if let Some((def_id, _)) = self.enum_defs.get(name) {
            // Enum def como valor -> marker `def_id<<32 | 0xffff_ffff` (imprime `<enum X>`).
            let v = ((*def_id as i64) << 32) | 0xffff_ffff;
            self.body.push(Instruction::I64Const(v));
        } else if self.struct_defs.contains_key(name) {
            // Struct def como valor -> ptr 0 (marker: imprime `<function X>`).
            self.body.push(Instruction::I64Const(0));
        } else if let Some(g) = self.globals.get(name) {
            self.body.push(Instruction::GlobalGet(*g));
        } else if let Some(fidx) = self.intrinsic_handle_idx(name) {
            // Intrinsic (print/input/now/...) como valor -> handle de función con
            // índice de tabla sintético (negativo, estable por nombre): se
            // imprime `<function print>` (paridad walker). Un var/global de
            // usuario con el mismo nombre gana (shadowing).
            let n = self.intern_string(&format!("<function {}>", name));
            self.body.push(Instruction::I64Const(fidx));
            self.emit_load_str(n);
            self.body.push(Instruction::I64Const(0));
            self.host.call(HostFn::FnHandle, &mut self.body);
        } else if let Some(&ci) = self.captures.get(name) {
            // Variable capturada (closure): el bloque guarda el valor (si no es
            // promovida) o el PTR al slot (si es promovida). Acceder al slot.
            self.body.push(Instruction::LocalGet(0));
            self.body
                .push(Instruction::I64Const(16 + (ci as i64 - 1) * 8));
            self.body.push(Instruction::I64Add);
            self.body.push(Instruction::I32WrapI64);
            self.body.push(Instruction::I64Load(MemArg {
                offset: 0,
                align: 3,
                memory_index: 0,
            }));
            if self.promoted.contains(name) {
                // La captura es un ptr al slot: doble deref.
                self.body.push(Instruction::I32WrapI64);
                self.body.push(Instruction::I64Load(MemArg {
                    offset: 0,
                    align: 3,
                    memory_index: 0,
                }));
            }
        } else if self.promoted.contains(name) {
            // Variable del scope promovida al heap: el local guarda el ptr al slot.
            let idx = self.local_for(name);
            self.body.push(Instruction::LocalGet(idx));
            self.body.push(Instruction::I32WrapI64);
            self.body.push(Instruction::I64Load(MemArg {
                offset: 0,
                align: 3,
                memory_index: 0,
            }));
        } else {
            let idx = self.local_for(name);
            self.body.push(Instruction::LocalGet(idx));
        }
    }


    /// Índice de tabla "sintético" (negativo, estable por nombre) para el handle
    /// de función de un intrinsic, o `None` si el nombre no es intrinsic.
    /// Los builtins se resuelven por nombre (el emisor los despacha en los
    /// calls); los host intrinsics del nodo (canal `env.host_call`) se indexan
    /// con claves ordenadas para que el índice no dependa del orden de
    /// iteración del HashMap (REPL: sesión a sesión).
    fn intrinsic_handle_idx(&self, name: &str) -> Option<i64> {
        const BUILTINS: &[&str] = &[
            "print", "len", "toString", "str", "input", "int", "float", "bool",
            "type", "now", "exit", "sleep", "throw",
        ];
        if let Some(i) = BUILTINS.iter().position(|b| *b == name) {
            return Some(-(i as i64 + 1));
        }
        if self.intrinsics.contains_key(name) {
            let mut keys: Vec<&String> = self.intrinsics.keys().collect();
            keys.sort();
            if let Some(i) = keys.iter().position(|k| k.as_str() == name) {
                return Some(-(1000 + i as i64));
            }
        }
        None
    }


    /// Carga el PTR al slot de una variable promovida (para capturarla por ref).
    /// Si no es promovida, carga el valor directo (comportamiento por valor).
    fn emit_ident_ptr(&mut self, name: &str) {
        if let Some(g) = self.globals.get(name) {
            // Globals ya son compartidas: cargar el valor directamente.
            self.body.push(Instruction::GlobalGet(*g));
        } else if let Some(&ci) = self.captures.get(name) {
            // Captura: el bloque guarda el PTR al slot (promovida) o el valor.
            // Las capturas van ANTES que promoted (una captura no es un local).
            self.body.push(Instruction::LocalGet(0));
            self.body
                .push(Instruction::I64Const(16 + (ci as i64 - 1) * 8));
            self.body.push(Instruction::I64Add);
            self.body.push(Instruction::I32WrapI64);
            self.body.push(Instruction::I64Load(MemArg {
                offset: 0,
                align: 3,
                memory_index: 0,
            }));
        } else if self.promoted.contains(name) {
            let idx = self.local_for(name);
            self.body.push(Instruction::LocalGet(idx));
        } else {
            let idx = self.local_for(name);
            self.body.push(Instruction::LocalGet(idx));
        }
    }


    /// Escribe un identificador: global (si es un `export var` top-level) o local.
    pub(crate) fn emit_ident_store(&mut self, name: &str) {
        if let Some(g) = self.globals.get(name) {
            self.body.push(Instruction::GlobalSet(*g));
        } else if let Some(&ci) = self.captures.get(name) {
            // Variable capturada (closure): el bloque guarda el valor o el PTR.
            let v = self.fresh_local();
            self.body.push(Instruction::LocalSet(v));
            // addr = ptr del bloque + offset de la captura.
            self.body.push(Instruction::LocalGet(0));
            self.body
                .push(Instruction::I64Const(16 + (ci as i64 - 1) * 8));
            self.body.push(Instruction::I64Add);
            self.body.push(Instruction::I32WrapI64);
            if self.promoted.contains(name) {
                // El bloque guarda un ptr al slot: store en `[ptr_al_slot]` -> valor.
                self.body.push(Instruction::I64Load(MemArg {
                    offset: 0,
                    align: 3,
                    memory_index: 0,
                }));
                self.body.push(Instruction::I32WrapI64);
            }
            self.body.push(Instruction::LocalGet(v));
            self.body.push(Instruction::I64Store(MemArg {
                offset: 0,
                align: 3,
                memory_index: 0,
            }));
        } else if self.promoted.contains(name) {
            // Variable del scope promovida: el local guarda el ptr al slot.
            let v = self.fresh_local();
            self.body.push(Instruction::LocalSet(v));
            let idx = self.local_for(name);
            self.body.push(Instruction::LocalGet(idx));
            self.body.push(Instruction::I32WrapI64);
            self.body.push(Instruction::LocalGet(v));
            self.body.push(Instruction::I64Store(MemArg {
                offset: 0,
                align: 3,
                memory_index: 0,
            }));
        } else {
            let idx = self.local_for(name);
            self.body.push(Instruction::LocalSet(idx));
        }
    }


    pub(crate) fn declare_var_ty(&mut self, name: &str, ty: WasTy) -> u32 {
        let idx = self.local_for(name);
        self.local_tys.entry(idx).or_insert(ty);
        idx
    }


    pub(crate) fn value_type(&self, expr: &Expression) -> ClsResult<WasTy> {
        // Literales: el kind ES el tipo (los spans del parser colisionan entre
        // un literal y la expresión que lo contiene, así que el type map puede
        // estar contaminado). Esto mantiene el tipo real del valor emitido.
        if let Expression::Literal(l) = expr {
            return Ok(match &l.kind {
                LiteralKind::Int(_) => WasTy::I64,
                LiteralKind::Float(_) => WasTy::F64,
                LiteralKind::Bool(_) => WasTy::I32,
                LiteralKind::Char(_) => WasTy::I32,
                _ => WasTy::I64,
            });
        }
        // Llamadas a funciones nativas (extensión) -> tipo de retorno codificado.
        if let Expression::Call(c) = expr {
            if let Expression::Identifier(name, _) = &*c.callee {
                if let Some(rc) = self.native_ret.get(name) {
                    return Ok(code_to_was(*rc));
                }
            }
        }
        // Llamadas a módulos stdlib -> tipo de retorno conocido.
        if let Some(w) = self.module_call_ret(expr) {
            return Ok(w);
        }
        let span = expr_span(expr);
        let t = self.types.get(&span).ok_or_else(|| {
            crate::error::ClsError::CompileError(format!(
                "Expresión sin tipo ({}:{}:{}): el JIT requiere el type checker",
                span.start_line,
                span.start_col,
                expr_display(expr)
            ))
        })?;
        match t {
            Type::Unknown => Err(crate::error::ClsError::CompileError(format!(
                "Expresión sin tipo concreto ({}:{}): {}",
                span.start_line,
                span.start_col,
                expr_display(expr)
            ))),
            _ => was_type(t),
        }
    }


    pub(crate) fn emit_drop(&mut self, expr: &Expression) -> ClsResult<()> {
        let span = expr_span(expr);
        if let Some(t) = self.types.get(&span) {
            if *t == Type::Void {
                return Ok(());
            }
        }
        self.body.push(Instruction::Drop);
        Ok(())
    }

}

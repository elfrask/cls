//! Emisor de funciones CLS a WASM (Fase 1: extraido de wasm/mod.rs).

use super::*;

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

/// Emisor con el estado de compilaciÃ³n de una funciÃ³n.
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
    /// Registro de types dinÃ¡micos (call_indirect de funciones como valor).
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
    /// Firmas tipadas de las funciones (`Clase::m` â†’ (params, ret)) â€” el retorno
    /// de los magic methods (el call_indirect lo produce segÃºn la firma).
    func_types: &'a HashMap<String, (Vec<Type>, Option<Type>)>,
    /// clase actual (al compilar un mÃ©todo) â€” para `super` y `me`.
    pub(crate) current_class: Option<String>,
    /// nombre del mÃ©todo que se estÃ¡ compilando (sin el prefijo de clase), para
    /// el protocolo `__next` (el `null` termina la iteraciÃ³n con un sentinel
    /// distinto de 0 â€” un iterador puede devolver 0 como valor legÃ­timo).
    pub(crate) current_method: Option<String>,
    /// Span de la funciÃ³n que se estÃ¡ compilando (para errores de statements sin
    /// span propio, p.ej. `break`/`continue` fuera de loop).
    pub(crate) current_fn_span: Span,
    target: &'a Target,
    /// Ãndice del tag de excepciÃ³n CLS (para `Instruction::Throw`).
    tag_idx: u32,
    /// Type `[] -> [i64, i64]` del block handler del try_table.
    eh_handler_ty: u32,
    /// `true` = excepciones CLS habilitadas (wasmtime); `false` = modo sin
    /// excepciones (wasmi): try/catch/throw fallan y los errores de runtime se
    /// emiten como `unreachable` (trap).
    exceptions: bool,
    /// Funciones host del nodo por nombre (canal `env.host_call`).
    intrinsics: &'a HashMap<String, HostIntrinsic>,
    /// Closures (B5): nombre de variable capturada â†’ Ã­ndice 1-based en el bloque
    /// de capturas `[n, v1, v2, ...]`. El param 0 del frame es `__capturas` (ptr).
    pub(crate) captures: HashMap<String, u32>,
    /// Variables promovidas al heap (capturadas por una arrow del scope): el
    /// local guarda un PTR a un slot `[valor]`; los accesos pasan por ahÃ­.
    pub(crate) promoted: HashSet<String>,
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
            target,
            tag_idx,
            eh_handler_ty,
            exceptions,
            intrinsics,
            captures: HashMap::new(),
            promoted: HashSet::new(),
        }
    }

    pub(crate) fn fresh_local_ty(&mut self, ty: WasTy) -> u32 {
        let l = self.next_local;
        self.next_local += 1;
        self.local_tys.insert(l, ty);
        l
    }

    /// Registra un type de funciÃ³n (para `call_indirect` de funciones como valor).
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
            // FunciÃ³n CLS como valor â†’ handle [tabla_idx][capturas=0][nombre]
            // con tag-bit (ptr<<1)|1. El nombre se guarda para fn_to_string.
            let n = self.intern_string(&format!("<function {}>", name));
            self.body.push(Instruction::I64Const(ti as i64));
            self.emit_load_str(n);
            self.body.push(Instruction::I64Const(0));
            self.host.call(HostFn::FnHandle, &mut self.body);
        } else if let Some((def_id, _)) = self.enum_defs.get(name) {
            // Enum def como valor â†’ marker `def_id<<32 | 0xffff_ffff` (imprime `<enum X>`).
            let v = ((*def_id as i64) << 32) | 0xffff_ffff;
            self.body.push(Instruction::I64Const(v));
        } else if self.struct_defs.contains_key(name) {
            // Struct def como valor â†’ ptr 0 (marker: imprime `<function X>`).
            self.body.push(Instruction::I64Const(0));
        } else if let Some(g) = self.globals.get(name) {
            self.body.push(Instruction::GlobalGet(*g));
        } else if let Some(fidx) = self.intrinsic_handle_idx(name) {
            // Intrinsic (print/input/now/...) como valor â†’ handle de funciÃ³n con
            // Ã­ndice de tabla sintÃ©tico (negativo, estable por nombre): se
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

    /// Ãndice de tabla "sintÃ©tico" (negativo, estable por nombre) para el handle
    /// de funciÃ³n de un intrinsic, o `None` si el nombre no es intrinsic.
    /// Los builtins se resuelven por nombre (el emisor los despacha en los
    /// calls); los host intrinsics del nodo (canal `env.host_call`) se indexan
    /// con claves ordenadas para que el Ã­ndice no dependa del orden de
    /// iteraciÃ³n del HashMap (REPL: sesiÃ³n a sesiÃ³n).
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
                // El bloque guarda un ptr al slot: store en `[ptr_al_slot]` â†’ valor.
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
        // un literal y la expresiÃ³n que lo contiene, asÃ­ que el type map puede
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
        // Llamadas a funciones nativas (extensiÃ³n) â†’ tipo de retorno codificado.
        if let Expression::Call(c) = expr {
            if let Expression::Identifier(name, _) = &*c.callee {
                if let Some(rc) = self.native_ret.get(name) {
                    return Ok(code_to_was(*rc));
                }
            }
        }
        // Llamadas a mÃ³dulos stdlib â†’ tipo de retorno conocido.
        if let Some(w) = self.module_call_ret(expr) {
            return Ok(w);
        }
        let span = expr_span(expr);
        let t = self.types.get(&span).ok_or_else(|| {
            crate::error::ClsError::CompileError(format!(
                "ExpresiÃ³n sin tipo ({}:{}:{}): el JIT requiere el type checker",
                span.start_line,
                span.start_col,
                expr_display(expr)
            ))
        })?;
        match t {
            Type::Any | Type::Unknown => Err(crate::error::ClsError::CompileError(format!(
                "ExpresiÃ³n sin tipo concreto ({}:{}): {}",
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

    // â”€â”€ EmisiÃ³n de statements â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    pub(crate) fn emit_statement(&mut self, stmt: &Statement) -> ClsResult<()> {
        match stmt {
            Statement::VarDecl(v) | Statement::ConstDecl(v) => {
                let ty = match (&v.type_ann, &v.value) {
                    (Some(ann), Some(val)) => match was_type(&annotation_to_type(ann)) {
                        Ok(w) => w,
                        // AnotaciÃ³n no resuelta (alias/unioÃ³n) â†’ tipo del valor.
                        Err(_) => self.value_type(val)?,
                    },
                    (Some(ann), None) => was_type(&annotation_to_type(ann))?,
                    (None, Some(val)) => self.value_type(val)?,
                    (None, None) => WasTy::I64,
                };
                let idx = self.declare_var_ty(&v.name, ty);
                if let Some(value) = &v.value {
                    self.emit_expression(value)?;
                    if self.promoted.contains(&v.name) {
                        // Variable promovida: alloc slot `[valor]`, guardar ptr en
                        // el local, store el valor en el slot.
                        let val_tmp = self.fresh_local_ty(ty);
                        self.body.push(match ty {
                            WasTy::F64 => Instruction::LocalSet(val_tmp),
                            WasTy::I32 => Instruction::LocalSet(val_tmp),
                            WasTy::I64 => Instruction::LocalSet(val_tmp),
                        });
                        self.body.push(Instruction::I64Const(8));
                        let alloc = self.func_indexes["__alloc"];
                        self.body.push(Instruction::Call(alloc));
                        self.body.push(Instruction::LocalSet(idx));
                        self.body.push(Instruction::LocalGet(idx));
                        self.body.push(Instruction::I32WrapI64);
                        self.body.push(match ty {
                            WasTy::F64 => Instruction::LocalGet(val_tmp),
                            WasTy::I32 => Instruction::LocalGet(val_tmp),
                            WasTy::I64 => Instruction::LocalGet(val_tmp),
                        });
                        match ty {
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
                    } else {
                        self.body.push(Instruction::LocalSet(idx));
                    }
                }
                Ok(())
            }
            Statement::FunctionDecl(_) => Ok(()),
            Statement::Expression(e) => {
                self.emit_expression(e)?;
                self.emit_drop(e)
            }
            Statement::Return(e) => {
                if e.is_some() {
                    self.emit_expression(e.as_ref().unwrap())?;
                }
                // Des-registrar el frame antes de cortar: `Instruction::Return`
                // salta al final sin pasar por el `fn_exit` del cuerpo.
                self.emit_fn_exit();
                self.body.push(Instruction::Return);
                Ok(())
            }
            Statement::Break(bspan) => {
                let ctx = self.loop_stack.last().ok_or_else(|| {
                    crate::error::ClsError::compile_at("break fuera de loop", bspan)
                })?;
                let depth = self.block_depth.saturating_sub(ctx.break_at);
                self.body.push(Instruction::Br(depth));
                Ok(())
            }
            Statement::Continue(cspan) => {
                let ctx = self.loop_stack.last().ok_or_else(|| {
                    crate::error::ClsError::compile_at("continue fuera de loop", cspan)
                })?;
                let depth = self.block_depth.saturating_sub(ctx.continue_at);
                self.body.push(Instruction::Br(depth));
                Ok(())
            }
            Statement::If(i) => self.emit_if(i),
            Statement::Try(t) => self.emit_try(t),
            Statement::While(w) => self.emit_while(w),
            Statement::Loop(b) => self.emit_loop(b),
            Statement::For(f) => self.emit_for(f),
            Statement::ForEach(fe) => self.emit_foreach(fe),
            Statement::Switch(s) => self.emit_switch(s),
            Statement::With(w) => self.emit_with(w),
            // `when` â†’ compile-time: emitir solo la rama que matchea el target actual.
            Statement::When(w) => {
                if let Some(branch) = w.branches.iter().find(|b| self.target.matches(&b.cond)) {
                    for st in &branch.block.statements {
                        self.emit_statement(st)?;
                    }
                }
                Ok(())
            }
            // Compile-time / no-runtime: alias, imports, interfaces, namespaces, config.
            Statement::TypeAlias(_)
            | Statement::Import(_)
            | Statement::FromImport(_)
            | Statement::Include(_)
            | Statement::InterfaceDecl(_)
            | Statement::NamespaceDecl(_)
            | Statement::ModuleDecl(_)
            | Statement::Config(_) => Ok(()),
            Statement::Cmx(c) => {
                self.emit_cmx(c)?;
                self.emit_drop(&Expression::Cmx(c.clone()))
            }
            other => Err(self.unsupported_stmt(other)),
        }
    }

    fn unsupported_stmt(&self, stmt: &Statement) -> crate::error::ClsError {
        crate::error::ClsError::CompileError(format!(
            "El JIT (subconjunto WASM) aÃºn no soporta este statement: {}",
            statement_display(stmt)
        ))
    }

    /// `arr.map(f)` â€” aplica la funciÃ³n (handle) a cada elemento y devuelve un
    /// array nuevo con los resultados. El array original YA estÃ¡ en el stack
    /// (lo emitiÃ³ el dispatch del mÃ©todo).
    fn emit_array_map(
        &mut self,
        _member: &MemberAccessExpr,
        c: &CallExpr,
        elem_ty: WasTy,
        elem_size: i64,
    ) -> ClsResult<()> {
        let arr_ptr = self.fresh_local();
        self.body.push(Instruction::LocalSet(arr_ptr));
        self.emit_expression(&c.args[0])?;
        let f_handle = self.fresh_local();
        self.body.push(Instruction::LocalSet(f_handle));
        // tipo de f â†’ Fun(params, ret)
        let ft = self
            .types
            .get(&expr_span(&c.args[0]))
            .cloned()
            .unwrap_or(Type::Any);
        let (f_params, f_ret) = match ft {
            Type::Fun(p, r) => (p, *r),
            _ => {
                return Err(crate::error::ClsError::CompileError(
                    "map: el argumento debe ser una funciÃ³n".to_string(),
                ))
            }
        };
        let ret_was = was_type(&f_ret).unwrap_or(WasTy::I64);
        let es_ret = elem_size_bytes(ret_was);
        let mut pv: Vec<ValType> = Vec::new();
        for t in &f_params {
            pv.push(was_type(t)?.val_type());
        }
        let rv: Vec<ValType> = match f_ret {
            Type::Void => vec![],
            r => vec![was_type(&r)?.val_type()],
        };
        // nuevo array [cap][len][ret...] del mismo tamaÃ±o que el original.
        let i = self.fresh_local();
        let new_ptr = self.fresh_local();
        self.body.push(Instruction::LocalGet(arr_ptr));
        self.body.push(Instruction::I32WrapI64);
        self.body.push(Instruction::I64Load(MemArg {
            offset: 8,
            align: 3,
            memory_index: 0,
        }));
        self.body.push(Instruction::LocalSet(i)); // n
        self.body.push(Instruction::I64Const(16));
        self.body.push(Instruction::LocalGet(i));
        self.body.push(Instruction::I64Const(es_ret));
        self.body.push(Instruction::I64Mul);
        self.body.push(Instruction::I64Add);
        let alloc = self.func_indexes["__alloc"];
        self.body.push(Instruction::Call(alloc));
        self.body.push(Instruction::LocalSet(new_ptr));
        // cap y len del nuevo array
        self.body.push(Instruction::LocalGet(new_ptr));
        self.body.push(Instruction::LocalGet(i));
        self.emit_i64_store(0);
        self.body.push(Instruction::LocalGet(new_ptr));
        self.body.push(Instruction::LocalGet(i));
        self.emit_i64_store(8);
        // loop i desde 0
        self.body.push(Instruction::I64Const(0));
        self.body.push(Instruction::LocalSet(i));
        self.block_depth += 1;
        self.body.push(Instruction::Block(BlockType::Empty));
        let break_at = self.block_depth;
        self.block_depth += 1;
        self.body.push(Instruction::Loop(BlockType::Empty));
        let loop_at = self.block_depth;
        // cond: i >= n
        self.body.push(Instruction::LocalGet(i));
        self.body.push(Instruction::LocalGet(new_ptr));
        self.body.push(Instruction::I32WrapI64);
        self.body.push(Instruction::I64Load(MemArg {
            offset: 8,
            align: 3,
            memory_index: 0,
        }));
        self.body.push(Instruction::I64GeS);
        let depth = self.block_depth.saturating_sub(break_at);
        self.body.push(Instruction::BrIf(depth));
        // addr del destino en el nuevo array â†’ guardar en local.
        self.body.push(Instruction::LocalGet(new_ptr));
        self.body.push(Instruction::LocalGet(i));
        self.body.push(Instruction::I64Const(es_ret));
        self.body.push(Instruction::I64Mul);
        self.body.push(Instruction::I64Const(16));
        self.body.push(Instruction::I64Add);
        self.body.push(Instruction::I64Add);
        let addr_tmp = self.fresh_local();
        self.body.push(Instruction::LocalSet(addr_tmp));
        // elem = arr[16 + i*elem_size] â†’ guardar en local.
        self.body.push(Instruction::LocalGet(arr_ptr));
        self.body.push(Instruction::LocalGet(i));
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
        let elem_tmp = self.fresh_local_ty(elem_ty);
        self.body.push(Instruction::LocalSet(elem_tmp));
        // llamar f(handle) con dispatch tag-bit (B5).
        let mut pv_caps = vec![ValType::I64];
        pv_caps.extend(pv.iter().copied());
        let tidx_caps = self.register_func_type(pv_caps, rv.clone());
        self.body.push(Instruction::LocalGet(f_handle));
        self.body.push(Instruction::I64Const(1));
        self.body.push(Instruction::I64And);
        self.body.push(Instruction::I32WrapI64);
        self.block_depth += 1;
        self.body.push(Instruction::If(if rv.is_empty() {
            BlockType::Empty
        } else {
            BlockType::Result(rv[0])
        }));
        // closure: push [capturas, elem, tabla]
        self.body.push(Instruction::LocalGet(f_handle));
        self.body.push(Instruction::I64Const(1));
        self.body.push(Instruction::I64ShrU);
        self.body.push(Instruction::I64Const(8));
        self.body.push(Instruction::I64Add);
        self.body.push(Instruction::I32WrapI64);
        self.body.push(Instruction::I64Load(MemArg {
            offset: 0,
            align: 3,
            memory_index: 0,
        }));
        let caps_tmp = self.fresh_local();
        self.body.push(Instruction::LocalSet(caps_tmp));
        self.body.push(Instruction::LocalGet(caps_tmp));
        self.body.push(Instruction::LocalGet(elem_tmp));
        self.body.push(Instruction::LocalGet(f_handle));
        self.body.push(Instruction::I64Const(1));
        self.body.push(Instruction::I64ShrU);
        self.body.push(Instruction::I32WrapI64);
        self.body.push(Instruction::I64Load(MemArg {
            offset: 0,
            align: 3,
            memory_index: 0,
        }));
        self.body.push(Instruction::I32WrapI64);
        self.body.push(Instruction::CallIndirect {
            type_index: tidx_caps,
            table_index: 0,
        });
        self.body.push(Instruction::Else);
        // simple: push [capturas=0, elem, tabla]
        self.body.push(Instruction::I64Const(0));
        self.body.push(Instruction::LocalGet(elem_tmp));
        self.body.push(Instruction::LocalGet(f_handle));
        self.body.push(Instruction::I64Const(1));
        self.body.push(Instruction::I64ShrU);
        self.body.push(Instruction::I32WrapI64);
        self.body.push(Instruction::CallIndirect {
            type_index: tidx_caps,
            table_index: 0,
        });
        self.body.push(Instruction::End);
        self.block_depth -= 1;
        // store el resultado en [addr_tmp, result]: guardar resultado en local,
        // luego pushear addr y resultado en orden limpio.
        let res_tmp = self.fresh_local_ty(ret_was);
        self.body.push(Instruction::LocalSet(res_tmp));
        self.body.push(Instruction::LocalGet(addr_tmp));
        self.body.push(Instruction::I32WrapI64);
        self.body.push(Instruction::LocalGet(res_tmp));
        match ret_was {
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
        // i++
        self.body.push(Instruction::LocalGet(i));
        self.body.push(Instruction::I64Const(1));
        self.body.push(Instruction::I64Add);
        self.body.push(Instruction::LocalSet(i));
        let depth = self.block_depth.saturating_sub(loop_at);
        self.body.push(Instruction::Br(depth));
        self.body.push(Instruction::End);
        self.block_depth -= 1;
        self.body.push(Instruction::End);
        self.block_depth -= 1;
        self.body.push(Instruction::LocalGet(new_ptr));
        Ok(())
    }

    /// `for each x [and i] in (col)` sobre array/tuple.
    fn emit_foreach(&mut self, fe: &ForEachStatement) -> ClsResult<()> {
        // Enum: `for each v in (Nivel)` o `for each v in (lib::Color)` (namespaced)
        // â†’ loop 0..variants.len()
        let enum_key = match &fe.iterable {
            Expression::Identifier(name, _) => Some(name.clone()),
            Expression::NamespaceAccess(ns, name, _) => Some(format!("{}::{}", ns, name)),
            _ => None,
        };
        if let Some(key) = enum_key {
            if let Some((def_id, variants)) = self.enum_defs.get(&key).cloned() {
                let n = variants.len() as i64;
                let i = self.fresh_local();
                self.body.push(Instruction::I64Const(0));
                self.body.push(Instruction::LocalSet(i));
                let item_local = self.declare_var_ty(&fe.item_name, WasTy::I64);
                if let Some(iname) = &fe.index_name {
                    self.declare_var_ty(iname, WasTy::I64);
                }
                self.block_depth += 1;
                self.body.push(Instruction::Block(BlockType::Empty));
                let break_at = self.block_depth;
                self.block_depth += 1;
                self.body.push(Instruction::Loop(BlockType::Empty));
                // continue block: el `continue` salta aquÃ­ y ejecuta el incremento.
                self.block_depth += 1;
                self.body.push(Instruction::Block(BlockType::Empty));
                let continue_at = self.block_depth;
                self.loop_stack.push(LoopGuard {
                    break_at,
                    continue_at,
                });
                self.body.push(Instruction::LocalGet(i));
                self.body.push(Instruction::I64Const(n));
                self.body.push(Instruction::I64GeS);
                let depth = self.block_depth.saturating_sub(break_at);
                self.body.push(Instruction::BrIf(depth));
                self.body.push(Instruction::I64Const((def_id as i64) << 32));
                self.body.push(Instruction::LocalGet(i));
                self.body.push(Instruction::I64Or);
                self.body.push(Instruction::LocalSet(item_local));
                if let Some(iname) = &fe.index_name {
                    let idx_local = self.local_for(iname);
                    self.body.push(Instruction::LocalGet(i));
                    self.body.push(Instruction::LocalSet(idx_local));
                }
                for st in &fe.block.statements {
                    self.emit_statement(st)?;
                }
                // cerrar el continue block â†’ incremento
                self.body.push(Instruction::End);
                self.block_depth -= 1;
                self.body.push(Instruction::LocalGet(i));
                self.body.push(Instruction::I64Const(1));
                self.body.push(Instruction::I64Add);
                self.body.push(Instruction::LocalSet(i));
                let depth = self.block_depth.saturating_sub(continue_at - 1);
                self.body.push(Instruction::Br(depth));
                self.body.push(Instruction::End);
                self.block_depth -= 1;
                self.body.push(Instruction::End);
                self.block_depth -= 1;
                self.loop_stack.pop();
                return Ok(());
            }
        }
        let iterable_ty = self
            .types
            .get(&expr_span(&fe.iterable))
            .cloned()
            .unwrap_or(Type::Any);
        // Magic methods __iter/__next (paridad walker interpreter.rs:723-767):
        // __iter() â†’ Array (caso 1) u objeto iterador con __next() hasta null
        // (caso 2). El tipo del iterable debe ser una clase con __iter.
        if let Some(cn) = self.class_magic_method(&Some(iterable_ty.clone()), "__iter") {
            return self.emit_foreach_magic(fe, &cn, &iterable_ty);
        }
        let (elem_ty, elem_size) = match &iterable_ty {
            Type::Array(elem) => {
                let w = was_type(elem)?;
                // Array de Cmx â†’ entradas `[val, tag]` stride 16.
                let es = if matches!(**elem, Type::Cmx) {
                    16
                } else {
                    elem_size_bytes(w)
                };
                (w, es)
            }
            Type::Tuple(slots) => {
                let w = slots.first().map(was_type).unwrap_or(Ok(WasTy::I64))?;
                (w, 8)
            }
            _ => {
                return Err(crate::error::ClsError::CompileError(
                    "for each solo soporta arrays y tuplas en el JIT (por ahora)".to_string(),
                ))
            }
        };
        self.emit_expression(&fe.iterable)?;
        let iter = self.fresh_local();
        self.body.push(Instruction::LocalSet(iter));
        self.emit_foreach_array_loop(iter, elem_ty, elem_size, fe)
    }

    /// Magic __iter/__next: `it = obj.__iter()`; si devuelve Array â†’ loop nativo;
    /// si devuelve una clase iteradora â†’ `it.__next()` hasta `null` (0 en el JIT).
    fn emit_foreach_magic(
        &mut self,
        fe: &ForEachStatement,
        cn: &str,
        _iterable_ty: &Type,
    ) -> ClsResult<()> {
        self.emit_class_method_args("__iter", &fe.iterable, &[])?;
        let iter = self.fresh_local();
        self.body.push(Instruction::LocalSet(iter));
        match self.magic_ret_type(cn, "__iter") {
            // Caso 1: __iter devolviÃ³ un Array â†’ iterar con el loop nativo.
            Some(Type::Array(elem)) => {
                let w = was_type(&*elem)?;
                let es = if matches!(*elem, Type::Cmx) {
                    16
                } else {
                    elem_size_bytes(w)
                };
                self.emit_foreach_array_loop(iter, w, es, fe)
            }
            // Caso 2: objeto iterador â†’ __next() hasta null.
            Some(Type::Named(it_cn, _)) => self.emit_foreach_next_loop(iter, &it_cn, fe),
            _ => Err(crate::error::ClsError::CompileError(format!(
                "'{}::__iter' debe anotar su retorno (Array<X> o una clase iteradora \
                 con __next) para el for each en el JIT",
                cn
            ))),
        }
    }

    /// Loop nativo de `for each`: `iter` (ptr de array ya en local) + contador.
    fn emit_foreach_array_loop(
        &mut self,
        iter: u32,
        elem_ty: WasTy,
        elem_size: i64,
        fe: &ForEachStatement,
    ) -> ClsResult<()> {
        let i = self.fresh_local();
        self.body.push(Instruction::I64Const(0));
        self.body.push(Instruction::LocalSet(i));
        let item_local = self.declare_var_ty(&fe.item_name, elem_ty);
        if let Some(iname) = &fe.index_name {
            self.declare_var_ty(iname, WasTy::I64);
        }
        let d = self.block_depth;
        self.block_depth += 1;
        self.body.push(Instruction::Block(BlockType::Empty));
        let break_at = self.block_depth;
        self.block_depth += 1;
        self.body.push(Instruction::Loop(BlockType::Empty));
        // continue block: el `continue` salta aquÃ­ y ejecuta el incremento.
        self.block_depth += 1;
        self.body.push(Instruction::Block(BlockType::Empty));
        let continue_at = self.block_depth;
        self.loop_stack.push(LoopGuard {
            break_at,
            continue_at,
        });
        // cond: i >= len(iter)
        self.body.push(Instruction::LocalGet(i));
        self.body.push(Instruction::LocalGet(iter));
        self.body.push(Instruction::I32WrapI64);
        self.body.push(Instruction::I64Load(MemArg {
            offset: 8,
            align: 3,
            memory_index: 0,
        }));
        self.body.push(Instruction::I64GeS);
        let depth = self.block_depth.saturating_sub(break_at);
        self.body.push(Instruction::BrIf(depth));
        // item = iter[i]
        self.body.push(Instruction::LocalGet(iter));
        self.body.push(Instruction::LocalGet(i));
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
        self.body.push(match elem_ty {
            WasTy::F64 => Instruction::LocalSet(item_local),
            WasTy::I32 => Instruction::LocalSet(item_local),
            WasTy::I64 => Instruction::LocalSet(item_local),
        });
        if let Some(iname) = &fe.index_name {
            let idx_local = self.local_for(iname);
            self.body.push(Instruction::LocalGet(i));
            self.body.push(Instruction::LocalSet(idx_local));
        }
        for st in &fe.block.statements {
            self.emit_statement(st)?;
        }
        // cerrar el continue block â†’ i++
        self.body.push(Instruction::End);
        self.block_depth -= 1;
        self.body.push(Instruction::LocalGet(i));
        self.body.push(Instruction::I64Const(1));
        self.body.push(Instruction::I64Add);
        self.body.push(Instruction::LocalSet(i));
        let depth = self.block_depth.saturating_sub(continue_at - 1);
        self.body.push(Instruction::Br(depth));
        self.body.push(Instruction::End); // loop
        self.block_depth -= 1;
        self.body.push(Instruction::End); // block
        self.block_depth -= 1;
        self.loop_stack.pop();
        let _ = d;
        Ok(())
    }

    /// Loop del iterador: `v = it.__next()`; si `v == 0` (null) â†’ break; si no,
    /// item = v, index = i, cuerpo, i++.
    fn emit_foreach_next_loop(&mut self, iter: u32, it_cn: &str, fe: &ForEachStatement) -> ClsResult<()> {
        let item_was = match self.magic_ret_type(it_cn, "__next") {
            Some(t) if t != Type::Void => was_type(&t)?,
            _ => {
                return Err(crate::error::ClsError::CompileError(format!(
                    "'{}::__next' debe anotar su tipo de retorno (distinto de void) \
                     para el for each en el JIT",
                    it_cn
                )))
            }
        };
        let item_local = self.declare_var_ty(&fe.item_name, item_was);
        if let Some(iname) = &fe.index_name {
            self.declare_var_ty(iname, WasTy::I64);
        }
        let i = self.fresh_local();
        self.body.push(Instruction::I64Const(0));
        self.body.push(Instruction::LocalSet(i));
        self.block_depth += 1;
        self.body.push(Instruction::Block(BlockType::Empty));
        let break_at = self.block_depth;
        self.block_depth += 1;
        self.body.push(Instruction::Loop(BlockType::Empty));
        // continue block: el `continue` salta aquÃ­ y ejecuta el incremento.
        self.block_depth += 1;
        self.body.push(Instruction::Block(BlockType::Empty));
        let continue_at = self.block_depth;
        self.loop_stack.push(LoopGuard {
            break_at,
            continue_at,
        });
        // v = it.__next()
        self.emit_class_method_call_on("__next", it_cn, iter, &[])?;
        let v = self.fresh_local_ty(item_was);
        self.body.push(match item_was {
            WasTy::F64 => Instruction::LocalSet(v),
            WasTy::I32 => Instruction::LocalSet(v),
            WasTy::I64 => Instruction::LocalSet(v),
        });
        // if v == null (sentinel del protocolo __next) â†’ break
        self.body.push(Instruction::LocalGet(v));
        match item_was {
            WasTy::I32 => self.body.push(Instruction::I32Eqz),
            _ => {
                self.body.push(Instruction::I64Const(NULL_ITER_SENTINEL));
                self.body.push(Instruction::I64Eq);
            }
        }
        let depth = self.block_depth.saturating_sub(break_at);
        self.body.push(Instruction::BrIf(depth));
        // item = v; index = i
        self.body.push(Instruction::LocalGet(v));
        self.body.push(match item_was {
            WasTy::F64 => Instruction::LocalSet(item_local),
            WasTy::I32 => Instruction::LocalSet(item_local),
            WasTy::I64 => Instruction::LocalSet(item_local),
        });
        if let Some(iname) = &fe.index_name {
            let idx_local = self.local_for(iname);
            self.body.push(Instruction::LocalGet(i));
            self.body.push(Instruction::LocalSet(idx_local));
        }
        for st in &fe.block.statements {
            self.emit_statement(st)?;
        }
        // cerrar el continue block â†’ i++
        self.body.push(Instruction::End);
        self.block_depth -= 1;
        self.body.push(Instruction::LocalGet(i));
        self.body.push(Instruction::I64Const(1));
        self.body.push(Instruction::I64Add);
        self.body.push(Instruction::LocalSet(i));
        let depth = self.block_depth.saturating_sub(continue_at - 1);
        self.body.push(Instruction::Br(depth));
        self.body.push(Instruction::End); // loop
        self.block_depth -= 1;
        self.body.push(Instruction::End); // block
        self.block_depth -= 1;
        self.loop_stack.pop();
        Ok(())
    }

    /// `switch (v) { case (p) { ... } case default { ... } }` (sin fallthrough).
    fn emit_switch(&mut self, s: &SwitchStatement) -> ClsResult<()> {
        self.emit_expression(&s.value)?;
        let v = self.fresh_local();
        self.body.push(Instruction::LocalSet(v));
        let d = self.block_depth;
        self.block_depth += 1;
        self.body.push(Instruction::Block(BlockType::Empty));
        let done_at = self.block_depth;
        for case in &s.cases {
            if matches!(case.pattern, CasePattern::Default) {
                continue;
            }
            self.body.push(Instruction::LocalGet(v));
            match &case.pattern {
                CasePattern::Literal(l) => self.emit_literal(l)?,
                CasePattern::Identifier(name) => {
                    let idx = self.local_for(name);
                    self.body.push(Instruction::LocalGet(idx));
                }
                CasePattern::Default => {}
            }
            self.push_eq(WasTy::I64)?;
            self.block_depth += 1;
            self.body.push(Instruction::If(BlockType::Empty));
            for st in &case.block.statements {
                self.emit_statement(st)?;
            }
            let depth = self.block_depth.saturating_sub(done_at);
            self.body.push(Instruction::Br(depth));
            self.body.push(Instruction::End);
            self.block_depth -= 1;
        }
        if let Some(def) = &s.default {
            for st in &def.statements {
                self.emit_statement(st)?;
            }
        }
        self.body.push(Instruction::End); // block done
        self.block_depth -= 1;
        let _ = d;
        Ok(())
    }

    /// `with x in (expr) { ... }` â†’ local temporal + bloque.
    fn emit_with(&mut self, w: &WithStatement) -> ClsResult<()> {
        self.emit_expression(&w.value)?;
        let ty = self.value_type(&w.value)?;
        let idx = self.declare_var_ty(&w.name, ty);
        self.body.push(Instruction::LocalSet(idx));
        for st in &w.block.statements {
            self.emit_statement(st)?;
        }
        Ok(())
    }

    /// `try { ... } catch (e) { ... } finally { ... }` â€” excepciones WASM (try_table).
    /// Paridad con el walker: el finally solo se ejecuta si NO hubo catch; el catch
    /// recibe `e = "Error de runtime: " + msg` (e.to_string() del walker).
    fn emit_try(&mut self, stmt: &TryStatement) -> ClsResult<()> {
        if !self.exceptions {
            return Err(crate::error::ClsError::compile_at(
                "try/catch no soportado en este runtime: el backend se compilÃ³ sin \
                 excepciones WASM (wasmi). Usa el runtime wasmtime o el WASM nativo del navegador.",
                &stmt.span,
            ));
        }
        // block $outer (Empty)
        self.block_depth += 1;
        self.body.push(Instruction::Block(BlockType::Empty));
        let outer = self.block_depth;
        // block $handler (result [i64, i64]) â€” su label (continuation, tras su End)
        // es donde aterriza el catch con el payload [msg, span].
        self.block_depth += 1;
        self.body.push(Instruction::Block(BlockType::FunctionType(
            self.eh_handler_ty,
        )));
        let handler = self.block_depth;
        // try_table: captura nuestro tag â†’ br al label del $handler con [msg, span]
        // El label del catch NO cuenta el try_table como scope (br 0 = $handler).
        self.block_depth += 1;
        let catch_label = self.block_depth - handler - 1;
        self.body.push(Instruction::TryTable(
            BlockType::Empty,
            Cow::Owned(vec![Catch::One {
                tag: self.tag_idx,
                label: catch_label,
            }]),
        ));
        for s in &stmt.try_block.statements {
            self.emit_statement(s)?;
        }
        self.body.push(Instruction::End); // cierra try_table
        self.block_depth -= 1;
        // flujo normal (sin excepciÃ³n) â†’ br al $outer (salta el handler)
        let br_outer = self.block_depth - outer;
        self.body.push(Instruction::Br(br_outer));
        self.body.push(Instruction::End); // cierra $handler â†’ el catch aterriza AQUÃ con [msg, span]
        self.block_depth -= 1;
        // handler: payload [msg, span] en el stack (span arriba, msg debajo)
        if stmt.catch_clauses.is_empty() {
            let span_tmp = self.fresh_local();
            self.body.push(Instruction::LocalSet(span_tmp));
            let msg_tmp = self.fresh_local();
            self.body.push(Instruction::LocalSet(msg_tmp));
            if let Some(f) = &stmt.finally_block {
                for s in &f.statements {
                    self.emit_statement(s)?;
                }
            }
            // re-lanzar con el mismo payload (equivalente a Rethrow)
            self.body.push(Instruction::LocalGet(msg_tmp));
            self.body.push(Instruction::LocalGet(span_tmp));
            self.body.push(Instruction::Throw(self.tag_idx));
            self.body.push(Instruction::Unreachable);
        } else {
            let catch = &stmt.catch_clauses[0];
            let span_tmp = self.fresh_local();
            self.body.push(Instruction::LocalSet(span_tmp));
            let msg_tmp = self.fresh_local();
            self.body.push(Instruction::LocalSet(msg_tmp));
            // e = "Error de runtime: " + msg
            let pref = self.intern_string("Error de runtime: ");
            self.emit_load_str(pref);
            self.body.push(Instruction::LocalGet(msg_tmp));
            self.host.call(HostFn::StrConcat, &mut self.body);
            let e_local = self.declare_var_ty(&catch.param_name, WasTy::I64);
            self.body.push(Instruction::LocalSet(e_local));
            for s in &catch.block.statements {
                self.emit_statement(s)?;
            }
        }
        self.body.push(Instruction::End); // cierra $outer
        self.block_depth -= 1;
        Ok(())
    }

    fn emit_if(&mut self, i: &IfStatement) -> ClsResult<()> {
        self.emit_expression(&i.condition)?;
        self.coerce_to_bool(&i.condition)?;
        self.block_depth += 1;
        self.body.push(Instruction::If(BlockType::Empty));
        for s in &i.then_block.statements {
            self.emit_statement(s)?;
        }
        let has_elif = !i.elif_branches.is_empty();
        let has_else = i.else_block.is_some();
        if has_elif || has_else {
            self.body.push(Instruction::Else);
        }
        // Cadena de elifs anidados dentro del else; el Ãºltimo cede al else final.
        for (k, branch) in i.elif_branches.iter().enumerate() {
            self.emit_expression(&branch.condition)?;
            self.coerce_to_bool(&branch.condition)?;
            self.block_depth += 1;
            self.body.push(Instruction::If(BlockType::Empty));
            for s in &branch.block.statements {
                self.emit_statement(s)?;
            }
            let last = k == i.elif_branches.len() - 1;
            if last {
                if let Some(else_b) = &i.else_block {
                    self.body.push(Instruction::Else);
                    for s in &else_b.statements {
                        self.emit_statement(s)?;
                    }
                }
            } else {
                self.body.push(Instruction::Else);
            }
            self.body.push(Instruction::End);
            self.block_depth -= 1;
        }
        if !has_elif && has_else {
            let else_b = i.else_block.as_ref().unwrap();
            for s in &else_b.statements {
                self.emit_statement(s)?;
            }
        }
        self.body.push(Instruction::End);
        self.block_depth -= 1;
        Ok(())
    }

    fn emit_while(&mut self, w: &WhileStatement) -> ClsResult<()> {
        let d = self.block_depth;
        self.block_depth += 1;
        self.body.push(Instruction::Block(BlockType::Empty));
        let break_at = self.block_depth;
        self.block_depth += 1;
        self.body.push(Instruction::Loop(BlockType::Empty));
        let continue_at = self.block_depth;
        self.loop_stack.push(LoopGuard {
            break_at,
            continue_at,
        });
        let _ = d;
        self.emit_expression(&w.condition)?;
        self.coerce_to_bool(&w.condition)?;
        self.body.push(Instruction::I32Eqz);
        let depth = self.block_depth.saturating_sub(break_at);
        self.body.push(Instruction::BrIf(depth));
        for s in &w.block.statements {
            self.emit_statement(s)?;
        }
        let depth = self.block_depth.saturating_sub(continue_at);
        self.body.push(Instruction::Br(depth));
        self.body.push(Instruction::End); // loop
        self.block_depth -= 1;
        self.body.push(Instruction::End); // block
        self.block_depth -= 1;
        self.loop_stack.pop();
        Ok(())
    }

    fn emit_loop(&mut self, b: &Block) -> ClsResult<()> {
        let d = self.block_depth;
        self.block_depth += 1;
        self.body.push(Instruction::Block(BlockType::Empty));
        let break_at = self.block_depth;
        self.block_depth += 1;
        self.body.push(Instruction::Loop(BlockType::Empty));
        let continue_at = self.block_depth;
        self.loop_stack.push(LoopGuard {
            break_at,
            continue_at,
        });
        let _ = d;
        for s in &b.statements {
            self.emit_statement(s)?;
        }
        let depth = self.block_depth.saturating_sub(continue_at);
        self.body.push(Instruction::Br(depth));
        self.body.push(Instruction::End); // loop
        self.block_depth -= 1;
        self.body.push(Instruction::End); // block
        self.block_depth -= 1;
        self.loop_stack.pop();
        Ok(())
    }

    fn emit_for(&mut self, f: &ForStatement) -> ClsResult<()> {
        if let Some(init) = &f.init {
            self.emit_statement(init)?;
        }
        // break block
        self.block_depth += 1;
        self.body.push(Instruction::Block(BlockType::Empty));
        let break_at = self.block_depth;
        // loop
        self.block_depth += 1;
        self.body.push(Instruction::Loop(BlockType::Empty));
        // continue block: el `continue` salta aquÃ­ y ejecuta el update (evita
        // que se salte el incremento y produzca un loop infinito).
        self.block_depth += 1;
        self.body.push(Instruction::Block(BlockType::Empty));
        let continue_at = self.block_depth;
        self.loop_stack.push(LoopGuard {
            break_at,
            continue_at,
        });
        if let Some(cond) = &f.condition {
            self.emit_expression(cond)?;
            self.coerce_to_bool(cond)?;
            self.body.push(Instruction::I32Eqz);
            let depth = self.block_depth.saturating_sub(break_at);
            self.body.push(Instruction::BrIf(depth));
        }
        for s in &f.block.statements {
            self.emit_statement(s)?;
        }
        // cerrar el continue block â†’ se ejecuta el update
        self.body.push(Instruction::End);
        self.block_depth -= 1;
        if let Some(update) = &f.update {
            self.emit_expression(update)?;
            self.emit_drop(update)?;
        }
        // volver al loop (que estÃ¡ en continue_at - 1)
        let depth = self.block_depth.saturating_sub(continue_at - 1);
        self.body.push(Instruction::Br(depth));
        self.body.push(Instruction::End); // loop
        self.block_depth -= 1;
        self.body.push(Instruction::End); // block
        self.block_depth -= 1;
        self.loop_stack.pop();
        Ok(())
    }

    // â”€â”€ EmisiÃ³n de expresiones â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    pub(crate) fn emit_expression(&mut self, expr: &Expression) -> ClsResult<()> {
        match expr {
            Expression::Literal(l) => self.emit_literal(l),
            Expression::Identifier(name, _) => {
                self.emit_ident_load(name);
                Ok(())
            }
            Expression::Binary(b) => self.emit_binary(b),
            Expression::Unary(u) => self.emit_unary(u),
            Expression::Call(c) => self.emit_call(c),
            Expression::Index(i) => self.emit_index_get(i),
            Expression::Array(a) => self.emit_array(a),
            Expression::Tuple(t) => self.emit_tuple(t),
            Expression::Record(r) => self.emit_record(r),
            Expression::Cmx(c) => self.emit_cmx(c),
            Expression::ArrowFunction(a) => {
                // Arrow â†’ handle de su funciÃ³n sintÃ©tica `__arrow_<n>`.
                // Si captura variables (closure): evaluarlas en un bloque
                // `[n, v1, v2, ...]` y pasar el ptr como tercer arg del handle.
                let name = self.arrow_names.get(&a.span).ok_or_else(|| {
                    crate::error::ClsError::CompileError(
                        "Arrow function sin funciÃ³n sintÃ©tica (recolecciÃ³n)".to_string(),
                    )
                })?;
                let ti = self.fn_table_idx[name];
                let captures = self
                    .arrow_captures
                    .get(&a.span)
                    .cloned()
                    .unwrap_or_default();
                // Bloque de capturas `[n, v1, v2, ...]` (se evalÃºa primero).
                let cap_ptr = self.fresh_local();
                if captures.is_empty() {
                    self.body.push(Instruction::I64Const(0));
                    self.body.push(Instruction::LocalSet(cap_ptr));
                } else {
                    let ncap = captures.len() as i64;
                    let es = 8i64;
                    self.body.push(Instruction::I64Const(ncap));
                    self.body.push(Instruction::I64Const(es));
                    self.body.push(Instruction::I64Mul);
                    self.body.push(Instruction::I64Const(16));
                    self.body.push(Instruction::I64Add);
                    let alloc = self.func_indexes["__alloc"];
                    self.body.push(Instruction::Call(alloc));
                    self.body.push(Instruction::LocalSet(cap_ptr));
                    self.body.push(Instruction::LocalGet(cap_ptr));
                    self.body.push(Instruction::I64Const(ncap));
                    self.emit_i64_store(0);
                    for (i, cap) in captures.iter().enumerate() {
                        self.body.push(Instruction::LocalGet(cap_ptr));
                        self.body.push(Instruction::I64Const(16 + (i as i64) * 8));
                        self.body.push(Instruction::I64Add);
                        self.body.push(Instruction::I32WrapI64);
                        self.emit_ident_ptr(cap);
                        self.body.push(Instruction::I64Store(MemArg {
                            offset: 0,
                            align: 3,
                            memory_index: 0,
                        }));
                    }
                }
                self.body.push(Instruction::I64Const(ti as i64));
                let n = self.intern_string("<anonymous>");
                self.emit_load_str(n);
                self.body.push(Instruction::LocalGet(cap_ptr));
                self.host.call(HostFn::FnHandle, &mut self.body);
                Ok(())
            }
            Expression::MemberAccess(m) => self.emit_member_access(m),
            Expression::Conditional(c) => self.emit_conditional(c),
            Expression::Assignment(a) => self.emit_assignment(a),
            Expression::Parenthesized(inner, _) => self.emit_expression(inner),
            Expression::StringInterpolation(s) => self.emit_interpolation(s),
            // `x::miembro` (mÃ³dulo/namespace importado): global `x::miembro`.
            Expression::NamespaceAccess(ns, member, span) => {
                let key = format!("{}::{}", ns, member);
                if let Some(g) = self.globals.get(&key).copied() {
                    self.body.push(Instruction::GlobalGet(g));
                    Ok(())
                } else {
                    Err(crate::error::ClsError::compile_at(
                        &format!(
                            "El miembro '{}' no existe o no se exporta en el mÃ³dulo '{}' (fase de emisiÃ³n).",
                            member, ns
                        ),
                        span,
                    ))
                }
            }
            other => Err(self.unsupported_expr(other)),
        }
    }

    fn unsupported_expr(&self, expr: &Expression) -> crate::error::ClsError {
        let span = expr_span(expr);
        crate::error::ClsError::compile_at(
            &format!(
                "El JIT (subconjunto WASM) aÃºn no soporta esta expresiÃ³n: `{}`",
                expr_display(expr)
            ),
            &span,
        )
    }

    fn emit_literal(&mut self, l: &Literal) -> ClsResult<()> {
        match &l.kind {
            LiteralKind::Int(v) => self.body.push(Instruction::I64Const(*v)),
            LiteralKind::Float(v) => self
                .body
                .push(Instruction::F64Const(Ieee64::new(v.to_bits()))),
            LiteralKind::Bool(v) => self
                .body
                .push(Instruction::I32Const(if *v { 1 } else { 0 })),
            LiteralKind::Char(c) => self.body.push(Instruction::I32Const(*c as u32 as i32)),
            LiteralKind::String(s) => {
                let idx = self.intern_string(s);
                self.emit_load_str(idx);
            }
            LiteralKind::Null => {
                // Dentro de `__next`, el `null` es el sentinel de fin de iteraciÃ³n
                // (distinto de 0 â€” un iterador puede devolver 0 como valor
                // legÃ­timo). Fuera del protocolo, null = 0 (paridad histÃ³rica).
                if self.current_method.as_deref() == Some("__next") {
                    self.body.push(Instruction::I64Const(NULL_ITER_SENTINEL));
                } else {
                    self.body.push(Instruction::I64Const(0));
                }
            }
            LiteralKind::Unknown => {
                return Err(self.unsupported_expr(&Expression::Literal(l.clone())))
            }
        }
        Ok(())
    }

    /// Emite `env.fn_enter(nombre, line, col)` al inicio de una funciÃ³n CLS.
    /// Registra la funciÃ³n en el shadow call stack del host (para el trace de
    /// errores de runtime). `main` (la entrada) se registra sin ubicaciÃ³n
    /// (lÃ­nea 0): el formateador lo muestra como `â†’ main` sin lÃ­nea.
    pub(crate) fn emit_fn_enter(&mut self, f: &FunctionDecl) -> ClsResult<()> {
        let display = f
            .name
            .rsplit("::")
            .next()
            .unwrap_or(&f.name)
            .trim_start_matches("__s__")
            .to_string();
        let name_idx = self.intern_string(&display);
        self.emit_load_str(name_idx);
        let (line, col) = if f.name == "main" {
            (0, 0)
        } else {
            (f.span.start_line, f.span.start_col)
        };
        self.body.push(Instruction::I64Const(line as i64));
        self.body.push(Instruction::I64Const(col as i64));
        self.host.call(HostFn::FnEnter, &mut self.body);
        Ok(())
    }

    /// Emite `env.fn_call_site(line, col)` con el span del call site (la llamada
    /// dentro del llamador). El host lo guarda como pendiente; el `fn_enter` del
    /// callee lo consume como span del frame.
    pub(crate) fn emit_call_site(&mut self, span: &Span) {
        self.body.push(Instruction::I64Const(span.start_line as i64));
        self.body.push(Instruction::I64Const(span.start_col as i64));
        self.host.call(HostFn::CallSite, &mut self.body);
    }

    /// Emite una llamada a una funciÃ³n host del nodo (intrinsic) vÃ­a el canal
    /// genÃ©rico `env.host_call(id, ptr, n)`. Los args viajan empaquetados en
    /// memoria: `[n:i64][(val:i64, tag:i64)*n]` (tag = `cls_kind_code`).
    fn emit_host_call(&mut self, intr: &HostIntrinsic, c: &CallExpr) -> ClsResult<()> {
        let n = c.args.len() as i64;
        // 1. Evaluar cada arg y guardarlo en un temporal (bits uniformes i64:
        //    float â†’ reinterpret bits; bool/char â†’ extender a i64).
        let mut tmps: Vec<u32> = Vec::with_capacity(c.args.len());
        for (i, arg) in c.args.iter().enumerate() {
            self.emit_expression(arg)?;
            match intr.params.get(i) {
                Some(Type::Float) | Some(Type::F32) | Some(Type::F64)
                | Some(Type::Literal(LitVal::Float(_))) => {
                    self.body.push(Instruction::I64ReinterpretF64);
                }
                Some(Type::Bool) | Some(Type::Char)
                | Some(Type::Literal(LitVal::Bool(_))) => {
                    self.body.push(Instruction::I64ExtendI32S);
                }
                _ => {}
            }
            let tmp = self.fresh_local();
            self.body.push(Instruction::LocalSet(tmp));
            tmps.push(tmp);
        }
        // 2. Alocar el bloque [n][(val,tag)*n].
        let size = 8 + n * 16;
        self.body.push(Instruction::I64Const(size));
        self.body.push(Instruction::Call(self.func_indexes["__alloc"]));
        let ptr = self.fresh_local();
        self.body.push(Instruction::LocalSet(ptr));
        // 3. Escribir n.
        self.body.push(Instruction::LocalGet(ptr));
        self.body.push(Instruction::I32WrapI64);
        self.body.push(Instruction::I64Const(n));
        self.body.push(Instruction::I64Store(MemArg {
            offset: 0,
            align: 3,
            memory_index: 0,
        }));
        // 4. Por arg: val + tag. (El addr de los memory ops es i32 â†’ wrap.)
        for (i, tmp) in tmps.iter().enumerate() {
            let base = 8 + (i as i64) * 16;
            self.body.push(Instruction::LocalGet(ptr));
            self.body.push(Instruction::I64Const(base));
            self.body.push(Instruction::I64Add);
            self.body.push(Instruction::I32WrapI64);
            self.body.push(Instruction::LocalGet(*tmp));
            self.body.push(Instruction::I64Store(MemArg {
                offset: 0,
                align: 3,
                memory_index: 0,
            }));
            self.body.push(Instruction::LocalGet(ptr));
            self.body.push(Instruction::I64Const(base + 8));
            self.body.push(Instruction::I64Add);
            self.body.push(Instruction::I32WrapI64);
            self.body.push(Instruction::I64Const(cls_kind_code(
                intr.params.get(i).unwrap_or(&Type::Any),
            )));
            self.body.push(Instruction::I64Store(MemArg {
                offset: 0,
                align: 3,
                memory_index: 0,
            }));
        }
        // 5. `env.host_call(id, ptr, n)`.
        self.body.push(Instruction::I64Const(intr.id as i64));
        self.body.push(Instruction::LocalGet(ptr));
        self.body.push(Instruction::I64Const(n));
        self.host.call(HostFn::HostCall, &mut self.body);
        // 6. Convertir el retorno (bits i64) al tipo nativo del CLS.
        match &intr.ret {
            Type::Void | Type::Empty => {
                self.body.push(Instruction::Drop);
            }
            Type::Float | Type::F32 | Type::F64 | Type::Literal(LitVal::Float(_)) => {
                self.body.push(Instruction::F64ReinterpretI64);
            }
            Type::Bool | Type::Char | Type::Literal(LitVal::Bool(_)) => {
                self.body.push(Instruction::I32WrapI64);
            }
            _ => {}
        }
        Ok(())
    }

    /// Emite `env.fn_exit()` antes de salir de una funciÃ³n CLS.
    pub(crate) fn emit_fn_exit(&mut self) {
        self.host.call(HostFn::FnExit, &mut self.body);
    }

    pub(crate) fn intern_string(&mut self, s: &str) -> u32 {
        if let Some(idx) = self.string_index.get(s) {
            return *idx;
        }
        let idx = self.string_pool.len() as u32;
        self.string_pool.push(s.to_string());
        self.string_index.insert(s.to_string(), idx);
        idx
    }

    pub(crate) fn emit_load_str(&mut self, idx: u32) {
        self.body.push(Instruction::I64Const(idx as i64));
        let fidx = self.func_indexes["__load_str"];
        self.body.push(Instruction::Call(fidx));
    }

    fn emit_binary(&mut self, b: &BinaryExpr) -> ClsResult<()> {
        use Operator::*;
        let lt = self.value_type(&b.left)?;
        let rt = self.value_type(&b.right)?;
        // Magic methods de clase (paridad walker `binary_magic`): aritmÃ©tica,
        // igualdad y comparaciÃ³n se despachan a la clase ANTES de los paths
        // nativos (el typeck ya validÃ³ el tipo del resultado).
        let rty = self.types.get(&expr_span(&b.right)).cloned();
        let arith_magic = match b.op {
            Plus => "__add",
            Minus => "__sub",
            Star => "__mul",
            Slash => "__div",
            Percent => "__mod",
            StarStar => "__pow",
            _ => "",
        };
        if !arith_magic.is_empty() {
            if self.try_binary_magic(&b.left, &b.right, arith_magic)?.is_some() {
                return Ok(());
            }
        }
        match b.op {
            StrictEqual | NotEqual => {
                // __equals: left.__equals(right) â†’ truthiness; `!=` niega.
                if let Some(ret_was) = self.try_binary_magic(&b.left, &b.right, "__equals")? {
                    match ret_was {
                        WasTy::I64 => {
                            self.body.push(Instruction::I64Const(0));
                            self.body.push(Instruction::I64Ne);
                        }
                        WasTy::F64 => {
                            self.body
                                .push(Instruction::F64Const(Ieee64::new(0.0f64.to_bits())));
                            self.body.push(Instruction::F64Ne);
                        }
                        WasTy::I32 => {}
                    }
                    if b.op == NotEqual {
                        self.body.push(Instruction::I32Eqz);
                    }
                    return Ok(());
                }
            }
            LessThan | LessEqual | GreaterThan | GreaterEqual => {
                // __compare: resultado int â†’ c <0/<=0/>0/>=0 segÃºn el operador.
                if let Some(ret_was) = self.try_binary_magic(&b.left, &b.right, "__compare")? {
                    match ret_was {
                        WasTy::I32 => self.body.push(Instruction::I64ExtendI32S),
                        WasTy::F64 => self.body.push(Instruction::I64TruncF64S),
                        WasTy::I64 => {}
                    }
                    let c = self.fresh_local_ty(WasTy::I64);
                    self.body.push(Instruction::LocalSet(c));
                    self.body.push(Instruction::LocalGet(c));
                    self.body.push(Instruction::I64Const(0));
                    let cmp = match b.op {
                        LessThan => Instruction::I64LtS,
                        LessEqual => Instruction::I64LeS,
                        GreaterThan => Instruction::I64GtS,
                        _ => Instruction::I64GeS,
                    };
                    self.body.push(cmp);
                    return Ok(());
                }
            }
            _ => {}
        }
        match b.op {
            Plus if lt == WasTy::I64 && rt == WasTy::I64 => {
                let is_str = |e: &Expression| {
                    self.types
                        .get(&expr_span(e))
                        .map(|t| *t == Type::String)
                        .unwrap_or(false)
                };
                self.emit_expression(&b.left)?;
                self.emit_expression(&b.right)?;
                if is_str(&b.left) || is_str(&b.right) {
                    self.host.call(HostFn::StrConcat, &mut self.body);
                } else {
                    self.body.push(Instruction::I64Add);
                }
            }
            Plus if lt == WasTy::F64 && rt == WasTy::F64 => {
                self.emit_expression(&b.left)?;
                self.emit_expression(&b.right)?;
                self.body.push(Instruction::F64Add);
            }
            Plus if lt == WasTy::I64 && rt == WasTy::F64 => {
                self.emit_expression(&b.left)?;
                self.body.push(Instruction::F64ConvertI64S);
                self.emit_expression(&b.right)?;
                self.body.push(Instruction::F64Add);
            }
            Plus if lt == WasTy::F64 && rt == WasTy::I64 => {
                self.emit_expression(&b.left)?;
                self.emit_expression(&b.right)?;
                self.body.push(Instruction::F64ConvertI64S);
                self.body.push(Instruction::F64Add);
            }
            Plus => {
                self.emit_expression(&b.left)?;
                self.emit_expression(&b.right)?;
                self.host.call(HostFn::StrConcat, &mut self.body);
            }
            Minus if lt == WasTy::F64 || rt == WasTy::F64 => {
                self.emit_expression(&b.left)?;
                self.f64_promote(&b.left)?;
                self.emit_expression(&b.right)?;
                self.f64_promote(&b.right)?;
                self.body.push(Instruction::F64Sub);
            }
            Minus => {
                self.emit_expression(&b.left)?;
                self.emit_expression(&b.right)?;
                self.body.push(Instruction::I64Sub);
            }
            Star if lt == WasTy::F64 || rt == WasTy::F64 => {
                self.emit_expression(&b.left)?;
                self.f64_promote(&b.left)?;
                self.emit_expression(&b.right)?;
                self.f64_promote(&b.right)?;
                self.body.push(Instruction::F64Mul);
            }
            Star => {
                self.emit_expression(&b.left)?;
                self.emit_expression(&b.right)?;
                self.body.push(Instruction::I64Mul);
            }
            Slash if lt == WasTy::F64 || rt == WasTy::F64 => {
                self.emit_expression(&b.left)?;
                self.f64_promote(&b.left)?;
                self.emit_expression(&b.right)?;
                self.f64_promote(&b.right)?;
                self.body.push(Instruction::F64Div);
            }
            Slash => {
                self.emit_expression(&b.left)?;
                self.emit_expression(&b.right)?;
                self.div_zero_trap(&b.span)?;
                self.body.push(Instruction::I64DivS);
            }
            Percent if lt == WasTy::F64 || rt == WasTy::F64 => {
                self.emit_expression(&b.left)?;
                self.f64_promote(&b.left)?;
                self.emit_expression(&b.right)?;
                self.f64_promote(&b.right)?;
                self.host.call(HostFn::Fmod, &mut self.body);
            }
            Percent => {
                self.emit_expression(&b.left)?;
                self.emit_expression(&b.right)?;
                self.div_zero_trap(&b.span)?;
                self.body.push(Instruction::I64RemS);
            }
            StarStar if lt == WasTy::F64 || rt == WasTy::F64 => {
                // Potencia con float: promover ambos a f64 y usar math_pow.
                self.emit_expression(&b.left)?;
                self.f64_promote(&b.left)?;
                self.emit_expression(&b.right)?;
                self.f64_promote(&b.right)?;
                self.host.call(HostFn::MathPow, &mut self.body);
            }
            StarStar => {
                self.emit_expression(&b.left)?;
                self.emit_expression(&b.right)?;
                self.host.call(HostFn::PowNum, &mut self.body);
            }
            // Operadores bit a bit (enteros): ^ << >>
            Caret => {
                self.emit_expression(&b.left)?;
                self.emit_expression(&b.right)?;
                self.body.push(Instruction::I64Xor);
            }
            ShiftLeft => {
                self.emit_expression(&b.left)?;
                self.emit_expression(&b.right)?;
                self.body.push(Instruction::I64Shl);
            }
            ShiftRight => {
                self.emit_expression(&b.left)?;
                self.emit_expression(&b.right)?;
                self.body.push(Instruction::I64ShrS);
            }
            StrictEqual => {
                self.emit_expression(&b.left)?;
                if lt == WasTy::F64 || rt == WasTy::F64 {
                    self.f64_promote(&b.left)?;
                }
                self.emit_expression(&b.right)?;
                if lt == WasTy::F64 || rt == WasTy::F64 {
                    self.f64_promote(&b.right)?;
                }
                self.push_eq(if lt == WasTy::F64 || rt == WasTy::F64 {
                    WasTy::F64
                } else {
                    lt
                })?;
            }
            NotEqual => {
                self.emit_expression(&b.left)?;
                if lt == WasTy::F64 || rt == WasTy::F64 {
                    self.f64_promote(&b.left)?;
                }
                self.emit_expression(&b.right)?;
                if lt == WasTy::F64 || rt == WasTy::F64 {
                    self.f64_promote(&b.right)?;
                }
                self.push_eq(if lt == WasTy::F64 || rt == WasTy::F64 {
                    WasTy::F64
                } else {
                    lt
                })?;
                self.body.push(Instruction::I32Eqz);
            }
            LessThan => {
                self.emit_expression(&b.left)?;
                if lt == WasTy::F64 || rt == WasTy::F64 {
                    self.f64_promote(&b.left)?;
                }
                self.emit_expression(&b.right)?;
                if lt == WasTy::F64 || rt == WasTy::F64 {
                    self.f64_promote(&b.right)?;
                }
                self.push_cmp(
                    if lt == WasTy::F64 || rt == WasTy::F64 {
                        WasTy::F64
                    } else {
                        lt
                    },
                    true,
                    false,
                )?;
            }
            LessEqual => {
                self.emit_expression(&b.left)?;
                if lt == WasTy::F64 || rt == WasTy::F64 {
                    self.f64_promote(&b.left)?;
                }
                self.emit_expression(&b.right)?;
                if lt == WasTy::F64 || rt == WasTy::F64 {
                    self.f64_promote(&b.right)?;
                }
                self.push_cmp(
                    if lt == WasTy::F64 || rt == WasTy::F64 {
                        WasTy::F64
                    } else {
                        lt
                    },
                    true,
                    true,
                )?;
            }
            GreaterThan => {
                self.emit_expression(&b.left)?;
                if lt == WasTy::F64 || rt == WasTy::F64 {
                    self.f64_promote(&b.left)?;
                }
                self.emit_expression(&b.right)?;
                if lt == WasTy::F64 || rt == WasTy::F64 {
                    self.f64_promote(&b.right)?;
                }
                self.push_cmp(
                    if lt == WasTy::F64 || rt == WasTy::F64 {
                        WasTy::F64
                    } else {
                        lt
                    },
                    false,
                    false,
                )?;
            }
            GreaterEqual => {
                self.emit_expression(&b.left)?;
                if lt == WasTy::F64 || rt == WasTy::F64 {
                    self.f64_promote(&b.left)?;
                }
                self.emit_expression(&b.right)?;
                if lt == WasTy::F64 || rt == WasTy::F64 {
                    self.f64_promote(&b.right)?;
                }
                self.push_cmp(
                    if lt == WasTy::F64 || rt == WasTy::F64 {
                        WasTy::F64
                    } else {
                        lt
                    },
                    false,
                    true,
                )?;
            }
            And => {
                self.emit_expression(&b.left)?;
                self.body.push(Instruction::I32Eqz);
                self.block_depth += 1;
                self.body
                    .push(Instruction::If(BlockType::Result(ValType::I32)));
                self.body.push(Instruction::I32Const(0));
                self.body.push(Instruction::Else);
                self.emit_expression(&b.right)?;
                self.body.push(Instruction::End);
                self.block_depth -= 1;
            }
            Or => {
                self.emit_expression(&b.left)?;
                self.block_depth += 1;
                self.body
                    .push(Instruction::If(BlockType::Result(ValType::I32)));
                self.body.push(Instruction::I32Const(1));
                self.body.push(Instruction::Else);
                self.emit_expression(&b.right)?;
                self.body.push(Instruction::End);
                self.block_depth -= 1;
            }
            In => {
                // __contains: container.__contains(needle) si la clase lo define.
                if let Some(cn) = self.class_magic_method(&rty, "__contains") {
                    let _ = self.magic_ret_was(&cn, "__contains")?;
                    self.emit_class_method_args("__contains", &b.right, &[(*b.left).clone()])?;
                    return Ok(());
                }
                // `x in "texto"` â†’ substring (arrays en A4). StrContains(container, needle)
                self.emit_expression(&b.right)?;
                self.emit_expression(&b.left)?;
                self.host.call(HostFn::StrContains, &mut self.body);
            }
            Is => {
                // `v is Nivel` (enum), `p is Punto` (struct) o `o is Clase` (herencia)
                // `v is String`/`Int`/... (tipo builtin) â†’ se evalÃºa estÃ¡ticamente
                // con el tipo del lado izquierdo.
                if let Expression::Identifier(right_name, _) = &*b.right {
                    if let Some(t) = builtin_was_type(right_name) {
                        // El tipo del left determina el resultado en compile-time.
                        // Comparar por Type (no WasTy: String e Int son ambos i64).
                        let left_span = expr_span(&b.left);
                        let lt = self.types.get(&left_span).cloned().unwrap_or(Type::Any);
                        let matches = builtin_type_matches(&lt, &t);
                        self.emit_expression(&b.left)?;
                        self.body.push(Instruction::Drop);
                        self.body
                            .push(Instruction::I32Const(if matches { 1 } else { 0 }));
                        return Ok(());
                    }
                }
                self.emit_expression(&b.left)?;
                if let Expression::Identifier(right_name, _) = &*b.right {
                    if let Some(info) = self.class_defs.get(right_name.as_str()) {
                        // cid = obj[8]; true si el objeto ES la clase o una SUBCLASE.
                        let obj_tmp = self.fresh_local();
                        let cid_tmp = self.fresh_local();
                        self.body.push(Instruction::LocalSet(obj_tmp));
                        self.body.push(Instruction::LocalGet(obj_tmp));
                        self.body.push(Instruction::I32WrapI64);
                        self.body.push(Instruction::I64Load(MemArg {
                            offset: 8,
                            align: 3,
                            memory_index: 0,
                        }));
                        self.body.push(Instruction::LocalSet(cid_tmp));
                        let mut ids = vec![info.class_id];
                        for (_, other) in self.class_defs.iter() {
                            if other.ancestors.contains(&right_name) {
                                ids.push(other.class_id);
                            }
                        }
                        let mut first = true;
                        for id in &ids {
                            self.body.push(Instruction::LocalGet(cid_tmp));
                            self.body.push(Instruction::I64Const(*id as i64));
                            self.body.push(Instruction::I64Eq);
                            if !first {
                                self.body.push(Instruction::I32Or);
                            }
                            first = false;
                        }
                        return Ok(());
                    }
                }
                let (def_id, is_enum) = match &*b.right {
                    Expression::Identifier(name, _) => {
                        if let Some((d, _)) = self.enum_defs.get(name) {
                            (*d, true)
                        } else if let Some(info) = self.struct_defs.get(name) {
                            (info.def_id, false)
                        } else {
                            return Err(crate::error::ClsError::CompileError(format!(
                                "'is' con '{}': se esperaba un enum o structure en el JIT",
                                name
                            )));
                        }
                    }
                    // `c is lib::Color` (enum namespaced importado).
                    Expression::NamespaceAccess(ns, name, _) => {
                        let key = format!("{}::{}", ns, name);
                        if let Some((d, _)) = self.enum_defs.get(&key) {
                            (*d, true)
                        } else if let Some(info) = self.struct_defs.get(&key) {
                            (info.def_id, false)
                        } else {
                            return Err(crate::error::ClsError::CompileError(format!(
                                "'is' con '{}::{}': se esperaba un enum o structure en el JIT",
                                ns, name
                            )));
                        }
                    }
                    _ => {
                        return Err(crate::error::ClsError::CompileError(
                            "'is' requiere un nombre a la derecha en el JIT".to_string(),
                        ))
                    }
                };
                if is_enum {
                    self.body.push(Instruction::I64Const(32));
                    self.body.push(Instruction::I64ShrU);
                } else {
                    self.body.push(Instruction::I32WrapI64);
                    self.body.push(Instruction::I64Load(MemArg {
                        offset: 0,
                        align: 3,
                        memory_index: 0,
                    }));
                }
                self.body.push(Instruction::I64Const(def_id as i64));
                self.body.push(Instruction::I64Eq);
            }
            PlusEqual | MinusEqual | StarEqual | SlashEqual | PercentEqual => {
                self.emit_expression(&b.left)?;
                self.emit_expression(&b.right)?;
                match b.op {
                    PlusEqual => self.body.push(Instruction::I64Add),
                    MinusEqual => self.body.push(Instruction::I64Sub),
                    StarEqual => self.body.push(Instruction::I64Mul),
                    SlashEqual => self.body.push(Instruction::I64DivS),
                    _ => self.body.push(Instruction::I64RemS),
                }
            }
            op => {
                return Err(crate::error::ClsError::CompileError(format!(
                    "Operador {} no soportado por el JIT",
                    op
                )))
            }
        }
        Ok(())
    }

    fn f64_promote(&mut self, expr: &Expression) -> ClsResult<()> {        let is_int_literal = matches!(
            expr,
            Expression::Literal(l) if matches!(l.kind, LiteralKind::Int(_))
        );
        let vt = self.value_type(expr)?;
        if is_int_literal || matches!(vt, WasTy::I64) {
            self.body.push(Instruction::F64ConvertI64S);
        }
        Ok(())
    }

    /// Coacciona el valor en el stack (emitido por `emit_expression`) a un
    /// bool i32, con paridad a `Value::is_truthy` del walker. `expr` se usa
    /// solo para consultar el tipo estÃ¡tico (el valor ya estÃ¡ en el stack).
    /// NumÃ©ricos: != 0. String: len != 0 (los bits bajos del packed). Array/
    /// Tuple/Record/Shape: len del header (ptr+8) != 0. Char/Bool: ya son i32.
    /// Cmx/Named/objetos: true (paridad walker). Any/Unknown/Null: error claro
    /// (antes emitÃ­a WASM invÃ¡lido "expected i32, found i64").
    fn coerce_to_bool(&mut self, expr: &Expression) -> ClsResult<()> {
        let ty = self
            .types
            .get(&expr_span(expr))
            .cloned()
            .unwrap_or(Type::Any);
        match &ty {
            Type::Bool | Type::Char => Ok(()),
            Type::Int | Type::I8 | Type::I16 | Type::I32 | Type::I64 => {
                self.body.push(Instruction::I64Const(0));
                self.body.push(Instruction::I64Ne);
                Ok(())
            }
            Type::Float | Type::F32 | Type::F64 => {
                self.body.push(Instruction::F64Const(Ieee64::new(0.0f64.to_bits())));
                self.body.push(Instruction::F64Ne);
                Ok(())
            }
            Type::String => {
                // packed = (ptr << 32) | len â†’ truthy si len != 0.
                self.body.push(Instruction::I64Const(0xffff_ffff));
                self.body.push(Instruction::I64And);
                self.body.push(Instruction::I64Const(0));
                self.body.push(Instruction::I64Ne);
                Ok(())
            }
            Type::Array(_) | Type::Tuple(_) | Type::Record(_, _) => {
                // Header CLS: [cap:i64][len:i64] â†’ truthy si len (ptr+8) != 0.
                self.body.push(Instruction::I64Const(8));
                self.body.push(Instruction::I64Add);
                self.body.push(Instruction::I32WrapI64);
                self.body.push(Instruction::I64Load(MemArg {
                    offset: 0,
                    align: 3,
                    memory_index: 0,
                }));
                self.body.push(Instruction::I64Const(0));
                self.body.push(Instruction::I64Ne);
                Ok(())
            }
            // Shape: se emite como struct contiguo SIN header [cap][len] (los
            // campos van directos) â†’ no se puede leer el len; un shape con
            // campos declarados siempre es truthy (paridad walker).
            Type::Shape(_) => {
                self.body.push(Instruction::I32Const(1));
                Ok(())
            }
            Type::Cmx | Type::Named(_, _) | Type::Null => {
                // Objetos/valores con identidad: siempre truthy (paridad walker).
                self.body.push(Instruction::I32Const(1));
                Ok(())
            }
            other => Err(crate::error::ClsError::compile_at(
                &format!(
                    "la condiciÃ³n debe ser Bool, encontrÃ³ {} (usa bool(...) para convertir)",
                    other
                ),
                &expr_span(expr),
            )),
        }
    }

    fn push_eq(&mut self, ty: WasTy) -> ClsResult<()> {
        match ty {
            WasTy::F64 => self.body.push(Instruction::F64Eq),
            WasTy::I32 => self.body.push(Instruction::I32Eq),
            WasTy::I64 => self.body.push(Instruction::I64Eq),
        }
        Ok(())
    }

    fn push_cmp(&mut self, ty: WasTy, less: bool, equal: bool) -> ClsResult<()> {
        match ty {
            WasTy::F64 => {
                let op = match (less, equal) {
                    (true, false) => Instruction::F64Lt,
                    (true, true) => Instruction::F64Le,
                    (false, false) => Instruction::F64Gt,
                    (false, true) => Instruction::F64Ge,
                };
                self.body.push(op);
            }
            WasTy::I64 => {
                let op = match (less, equal) {
                    (true, false) => Instruction::I64LtS,
                    (true, true) => Instruction::I64LeS,
                    (false, false) => Instruction::I64GtS,
                    (false, true) => Instruction::I64GeS,
                };
                self.body.push(op);
            }
            WasTy::I32 => {
                let op = match (less, equal) {
                    (true, false) => Instruction::I32LtS,
                    (true, true) => Instruction::I32LeS,
                    (false, false) => Instruction::I32GtS,
                    (false, true) => Instruction::I32GeS,
                };
                self.body.push(op);
            }
        }
        Ok(())
    }

    fn div_zero_trap(&mut self, span: &Span) -> ClsResult<()> {
        let tmp = self.fresh_local();
        self.body.push(Instruction::LocalSet(tmp));
        self.body.push(Instruction::LocalGet(tmp));
        self.body.push(Instruction::I64Eqz);
        self.block_depth += 1;
        self.body.push(Instruction::If(BlockType::Empty));
        self.emit_throw("DivisiÃ³n por cero", span);
        self.body.push(Instruction::Unreachable);
        self.body.push(Instruction::End);
        self.block_depth -= 1;
        self.body.push(Instruction::LocalGet(tmp));
        Ok(())
    }

    /// Lanza la excepciÃ³n CLS: `throw(tag)` con payload (msg, span_empaquetado).
    /// En modo sin excepciones (wasmi): `unreachable` (trap) â€” el host muestra el
    /// error como trap con el shadow call stack (sin caret del span CLS).
    fn emit_throw(&mut self, msg: &str, span: &Span) {
        if !self.exceptions {
            self.body.push(Instruction::Unreachable);
            return;
        }
        let m = self.intern_string(msg);
        self.emit_load_str(m);
        let packed = ((span.start_line as i64) << 32) | (span.start_col as i64);
        self.body.push(Instruction::I64Const(packed));
        self.body.push(Instruction::Throw(self.tag_idx));
    }

    fn emit_unary(&mut self, u: &UnaryExpr) -> ClsResult<()> {
        match u.op {
            UnaryOp::Negate => {
                // Magic __neg: clase con __neg â†’ call sin args (paridad walker).
                let oty = self.types.get(&expr_span(&u.operand)).cloned();
                if let Some(cn) = self.class_magic_method(&oty, "__neg") {
                    let _ = self.magic_ret_was(&cn, "__neg")?;
                    self.emit_class_method_args("__neg", &u.operand, &[])?;
                    return Ok(());
                }
                let w = self.value_type(&u.operand)?;
                match w {
                    WasTy::F64 => {
                        self.emit_expression(&u.operand)?;
                        self.body.push(Instruction::F64Neg);
                    }
                    WasTy::I64 => {
                        // 0 - x: push 0 primero, luego el operando, luego sub.
                        self.body.push(Instruction::I64Const(0));
                        self.emit_expression(&u.operand)?;
                        self.body.push(Instruction::I64Sub);
                    }
                    WasTy::I32 => {
                        self.body.push(Instruction::I32Const(0));
                        self.emit_expression(&u.operand)?;
                        self.body.push(Instruction::I32Sub);
                    }
                }
            }
            UnaryOp::Not => {
                // Magic __not: clase con __not â†’ call sin args; si no, truthiness
                // (paridad walker: `!obj` â†’ __not() o !is_truthy()).
                let oty = self.types.get(&expr_span(&u.operand)).cloned();
                if let Some(cn) = self.class_magic_method(&oty, "__not") {
                    let _ = self.magic_ret_was(&cn, "__not")?;
                    self.emit_class_method_args("__not", &u.operand, &[])?;
                    return Ok(());
                }
                self.emit_expression(&u.operand)?;
                self.coerce_to_bool(&u.operand)?;
                self.body.push(Instruction::I32Eqz);
            }
            UnaryOp::TypeOf => {
                let span = expr_span(&u.operand);
                let t = self.types.get(&span).cloned().unwrap_or(Type::Any);
                let idx = self.intern_string(type_name_str(&t));
                self.emit_load_str(idx);
            }
            UnaryOp::PostInc | UnaryOp::PreInc | UnaryOp::PostDec | UnaryOp::PreDec => {
                self.emit_incdec(&u.operand, u.op.clone())?
            }
            UnaryOp::BitwiseNot => {
                // ~x â†’ x ^ -1 (en i64)
                self.emit_expression(&u.operand)?;
                self.body.push(Instruction::I64Const(-1));
                self.body.push(Instruction::I64Xor);
            }
        }
        Ok(())
    }

    /// `x++` / `++x` / `x--` / `--x` sobre un identificador.
    pub(crate) fn emit_incdec(&mut self, operand: &Expression, op: UnaryOp) -> ClsResult<()> {
        if let Expression::Identifier(name, _) = operand {
            let post = matches!(op, UnaryOp::PostInc | UnaryOp::PostDec);
            let inc = matches!(op, UnaryOp::PreInc | UnaryOp::PostInc);
            if post {
                let tmp = self.fresh_local();
                self.emit_ident_load(name);
                self.body.push(Instruction::LocalSet(tmp));
                self.emit_ident_load(name);
                self.body.push(Instruction::I64Const(1));
                if inc {
                    self.body.push(Instruction::I64Add);
                } else {
                    self.body.push(Instruction::I64Sub);
                }
                self.emit_ident_store(name);
                self.body.push(Instruction::LocalGet(tmp));
            } else {
                self.emit_ident_load(name);
                self.body.push(Instruction::I64Const(1));
                if inc {
                    self.body.push(Instruction::I64Add);
                } else {
                    self.body.push(Instruction::I64Sub);
                }
                self.emit_ident_store(name);
                self.emit_ident_load(name);
            }
            Ok(())
        } else {
            Err(crate::error::ClsError::CompileError(
                "++/-- solo se soporta sobre variables (identifier) en el JIT".to_string(),
            ))
        }
    }

    fn emit_conditional(&mut self, c: &ConditionalExpr) -> ClsResult<()> {
        let w = self.value_type(&c.then_expr)?;
        self.emit_expression(&c.condition)?;
        self.block_depth += 1;
        self.body
            .push(Instruction::If(BlockType::Result(w.val_type())));
        self.emit_expression(&c.then_expr)?;
        self.body.push(Instruction::Else);
        self.emit_expression(&c.else_expr)?;
        self.body.push(Instruction::End);
        self.block_depth -= 1;
        Ok(())
    }

    fn emit_assignment(&mut self, a: &AssignmentExpr) -> ClsResult<()> {
        let op = a.op;
        match &*a.target {
            Expression::Identifier(name, _) => {
                if is_compound(op) {
                    // Magic: `a += b` â†’ a = a.__add(b) (paridad walker apply_compound).
                    let compound_magic = match op {
                        Operator::PlusEqual => "__add",
                        Operator::MinusEqual => "__sub",
                        Operator::StarEqual => "__mul",
                        Operator::SlashEqual => "__div",
                        Operator::PercentEqual => "__mod",
                        _ => "",
                    };
                    if !compound_magic.is_empty() {
                        let ty = self.types.get(&expr_span(&a.target)).cloned();
                        if let Some(cn) = self.class_magic_method(&ty, compound_magic) {
                            let _ = self.magic_ret_was(&cn, compound_magic)?;
                            self.emit_ident_load(name);
                            let obj_tmp = self.fresh_local();
                            self.body.push(Instruction::LocalSet(obj_tmp));
                            self.emit_class_method_call_on(
                                compound_magic,
                                &cn,
                                obj_tmp,
                                &[(*a.value).clone()],
                            )?;
                            self.emit_ident_store(name);
                            self.emit_ident_load(name);
                            return Ok(());
                        }
                    }
                    // Elegir operaciÃ³n segÃºn el tipo del identificador (int vs float).
                    let ty = self.value_type(&a.target)?;
                    self.emit_ident_load(name);
                    self.emit_expression(&a.value)?;
                    // `s += x` con String: concatenar (StrConcat), NO sumar
                    // los punteros empaquetados (producÃ­a bytes NUL).
                    let cls_t = self
                        .types
                        .get(&expr_span(&a.target))
                        .cloned()
                        .unwrap_or(Type::Any);
                    if op == Operator::PlusEqual && matches!(cls_t, Type::String) {
                        self.host.call(HostFn::StrConcat, &mut self.body);
                    } else if ty == WasTy::F64 {
                        self.f64_promote(&a.value)?;
                        match op {
                            Operator::PlusEqual => self.body.push(Instruction::F64Add),
                            Operator::MinusEqual => self.body.push(Instruction::F64Sub),
                            Operator::StarEqual => self.body.push(Instruction::F64Mul),
                            Operator::SlashEqual => self.body.push(Instruction::F64Div),
                            // `%=` float: WASM no tiene resto float â†’ host fmod.
                            _ => self.host.call(HostFn::Fmod, &mut self.body),
                        }
                    } else {
                        match op {
                            Operator::PlusEqual => self.body.push(Instruction::I64Add),
                            Operator::MinusEqual => self.body.push(Instruction::I64Sub),
                            Operator::StarEqual => self.body.push(Instruction::I64Mul),
                            Operator::SlashEqual => self.body.push(Instruction::I64DivS),
                            _ => self.body.push(Instruction::I64RemS),
                        }
                    }
                } else {
                    self.emit_expression(&a.value)?;
                    // Assignment simple `f = k`: si el target es float y el RHS
                    // es int, promover a f64 (el store del local espera f64).
                    if self.value_type(&a.target)? == WasTy::F64 {
                        self.f64_promote(&a.value)?;
                    }
                }
                self.emit_ident_store(name);
                self.emit_ident_load(name);
                Ok(())
            }
            Expression::Index(i)
                if matches!(
                    self.types.get(&expr_span(&i.object)),
                    Some(Type::Record(_, _))
                ) =>
            {
                if is_compound(op) {
                    return Err(crate::error::ClsError::CompileError(
                        "Operadores compuestos (+=) sobre registros no soportados en el JIT"
                            .to_string(),
                    ));
                }
                // r["key"] = val â†’ record_set(ptr, key, val_bits)
                let elem_ty = self.index_elem_type(i)?;
                let val_tmp = self.fresh_local_ty(elem_ty);
                self.emit_expression(&i.object)?;
                self.emit_expression(&i.index)?;
                self.emit_expression(&a.value)?;
                self.body.push(match elem_ty {
                    WasTy::F64 => Instruction::LocalSet(val_tmp),
                    WasTy::I32 => Instruction::LocalSet(val_tmp),
                    WasTy::I64 => Instruction::LocalSet(val_tmp),
                });
                self.body.push(match elem_ty {
                    WasTy::F64 => Instruction::LocalGet(val_tmp),
                    WasTy::I32 => Instruction::LocalGet(val_tmp),
                    WasTy::I64 => Instruction::LocalGet(val_tmp),
                });
                match elem_ty {
                    WasTy::F64 => self.body.push(Instruction::I64ReinterpretF64),
                    WasTy::I32 => self.body.push(Instruction::I64ExtendI32U),
                    WasTy::I64 => {}
                }
                let cls_t = self
                    .types
                    .get(&expr_span(&a.value))
                    .cloned()
                    .unwrap_or(Type::Any);
                self.body.push(Instruction::I64Const(arr_kind_code(&cls_t)));
                self.host.call(HostFn::RecordSet, &mut self.body);
                // write-back del ptr (el record pudo crecer y reallocarse)
                if let Expression::Identifier(name, _) = &*i.object {
                    self.emit_ident_store(name);
                } else {
                    self.body.push(Instruction::Drop);
                }
                self.body.push(match elem_ty {
                    WasTy::F64 => Instruction::LocalGet(val_tmp),
                    WasTy::I32 => Instruction::LocalGet(val_tmp),
                    WasTy::I64 => Instruction::LocalGet(val_tmp),
                });
                Ok(())
            }
            Expression::Index(i)
                if matches!(self.types.get(&expr_span(&i.object)), Some(Type::Shape(_))) =>
            {
                if is_compound(op) {
                    return Err(crate::error::ClsError::CompileError(
                        "Operadores compuestos (+=) sobre records con shape no soportados en el JIT".to_string(),
                    ));
                }
                // r["campo"] = val â†’ store por offset (solo campos existentes).
                let shape = self.types.get(&expr_span(&i.object)).cloned();
                let fields = match &shape {
                    Some(Type::Shape(f)) => f.clone(),
                    _ => return Ok(()),
                };
                let key = match &*i.index {
                    Expression::Literal(l) if matches!(l.kind, LiteralKind::String(_)) => {
                        match &l.kind { LiteralKind::String(k) => k.clone(), _ => String::new() }
                    }
                    _ => {
                        return Err(crate::error::ClsError::compile_at(
                            "Ãndice dinÃ¡mico no soportado en un record con shape (usa Record<K,V> o any)",
                            &i.span,
                        ))
                    }
                };
                let (_, w, off) = self.shape_layout(&fields)?
                    .into_iter()
                    .find(|(n, _, _)| *n == key)
                    .ok_or_else(|| crate::error::ClsError::compile_at(
                        &format!("El record no tiene el campo '{}' (no se pueden agregar campos a un shape)", key),
                        &i.span,
                    ))?;
                self.emit_expression(&i.object)?;
                let ptr_tmp = self.fresh_local();
                self.body.push(Instruction::LocalSet(ptr_tmp));
                self.emit_expression(&a.value)?;
                let val_tmp = self.fresh_local_ty(w);
                self.body.push(Instruction::LocalSet(val_tmp));
                self.body.push(Instruction::LocalGet(ptr_tmp));
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
                self.body.push(Instruction::LocalGet(ptr_tmp));
                Ok(())
            }
            Expression::Index(i) => {
                // Magic __set: obj[i] = v â†’ obj.__set(index, value) con write-back
                // del objeto mutado (paridad walker interpreter.rs:2120-2128).
                let obj_ty = self.types.get(&expr_span(&i.object)).cloned();
                if let Some(cn) = self.class_magic_method(&obj_ty, "__set") {
                    if is_compound(op) {
                        return Err(crate::error::ClsError::CompileError(
                            "Operadores compuestos (+=) sobre objetos con __set no soportados en el JIT"
                                .to_string(),
                        ));
                    }
                    self.emit_expression(&i.object)?;
                    let obj_tmp = self.fresh_local();
                    self.body.push(Instruction::LocalSet(obj_tmp));
                    self.emit_class_method_call_on(
                        "__set",
                        &cn,
                        obj_tmp,
                        &[(*i.index).clone(), (*a.value).clone()],
                    )?;
                    // El retorno del __set (si lo hay) se descarta.
                    if let Some(t) = self.magic_ret_type(&cn, "__set") {
                        if t != Type::Void {
                            self.body.push(Instruction::Drop);
                        }
                    }
                    // write-back del objeto (el ptr no cambia en mutaciÃ³n in-place,
                    // pero la reasignaciÃ³n del slot es paridad walker).
                    if let Expression::Identifier(name, _) = &*i.object {
                        self.body.push(Instruction::LocalGet(obj_tmp));
                        self.emit_ident_store(name);
                    }
                    // Valor del assignment = el objeto (para el Drop del statement).
                    self.body.push(Instruction::LocalGet(obj_tmp));
                    return Ok(());
                }
                if is_compound(op) {
                    let elem_ty = self.index_elem_type(i)?;
                    let ptr = self.fresh_local();
                    let idx = self.fresh_local();
                    let cur = self.fresh_local_ty(elem_ty);
                    let v = self.fresh_local_ty(elem_ty);
                    let res = self.fresh_local_ty(elem_ty);
                    self.emit_expression(&i.object)?;
                    self.body.push(Instruction::LocalSet(ptr));
                    self.emit_expression(&i.index)?;
                    self.body.push(Instruction::LocalSet(idx));
                    // cur = arr[i]
                    self.body.push(Instruction::LocalGet(ptr));
                    self.body.push(Instruction::LocalGet(idx));
                    let elem_size = self.container_elem_size(i, elem_ty);
                    self.emit_index_access(elem_ty, elem_size, i)?;
                    self.body.push(match elem_ty {
                        WasTy::F64 => Instruction::LocalSet(cur),
                        WasTy::I32 => Instruction::LocalSet(cur),
                        WasTy::I64 => Instruction::LocalSet(cur),
                    });
                    self.emit_expression(&a.value)?;
                    // `farr[i] += 2` con array F64: el RHS int debe promover a f64
                    // (paridad con el write simple `farr[i] = 7` del fix R4).
                    if elem_ty == WasTy::F64 {
                        self.f64_promote(&a.value)?;
                    }
                    self.body.push(match elem_ty {
                        WasTy::F64 => Instruction::LocalSet(v),
                        WasTy::I32 => Instruction::LocalSet(v),
                        WasTy::I64 => Instruction::LocalSet(v),
                    });
                    self.body.push(match elem_ty {
                        WasTy::F64 => Instruction::LocalGet(cur),
                        WasTy::I32 => Instruction::LocalGet(cur),
                        WasTy::I64 => Instruction::LocalGet(cur),
                    });
                    self.body.push(match elem_ty {
                        WasTy::F64 => Instruction::LocalGet(v),
                        WasTy::I32 => Instruction::LocalGet(v),
                        WasTy::I64 => Instruction::LocalGet(v),
                    });
                    if elem_ty == WasTy::F64 && op == Operator::PercentEqual {
                        // `farr[i] %= v` float: WASM no tiene resto float â†’ host fmod.
                        self.host.call(HostFn::Fmod, &mut self.body);
                    } else {
                        apply_compound_ty(&mut self.body, op, elem_ty)?;
                    }
                    self.body.push(match elem_ty {
                        WasTy::F64 => Instruction::LocalSet(res),
                        WasTy::I32 => Instruction::LocalSet(res),
                        WasTy::I64 => Instruction::LocalSet(res),
                    });
                    self.body.push(Instruction::LocalGet(ptr));
                    self.body.push(Instruction::LocalGet(idx));
                    self.body.push(match elem_ty {
                        WasTy::F64 => Instruction::LocalGet(res),
                        WasTy::I32 => Instruction::LocalGet(res),
                        WasTy::I64 => Instruction::LocalGet(res),
                    });
                    self.emit_index_set(i, elem_size)?;
                    self.body.push(match elem_ty {
                        WasTy::F64 => Instruction::LocalGet(res),
                        WasTy::I32 => Instruction::LocalGet(res),
                        WasTy::I64 => Instruction::LocalGet(res),
                    });
                } else {
                    // Las tuplas son inmutables: escritura â†’ error.
                    let obj_ty = self.types.get(&expr_span(&i.object)).cloned();
                    if matches!(obj_ty, Some(Type::Tuple(_))) {
                        return Err(crate::error::ClsError::compile_at(
                            "Las tuplas son inmutables (no se puede escribir t[i] = v)",
                            &i.span,
                        ));
                    }
                    let elem_ty = self.index_elem_type(i)?;
                    let elem_size = self.container_elem_size(i, elem_ty);
                    self.emit_expression(&i.object)?;
                    self.emit_expression(&i.index)?;
                    self.emit_expression(&a.value)?;
                    // Array de float con valor int: promover el RHS a f64 antes
                    // del store (el layout del array es homogÃ©neo).
                    if elem_ty == WasTy::F64 {
                        self.f64_promote(&a.value)?;
                    }
                    self.emit_index_set(i, elem_size)?;
                    // Dejar un valor en el stack (el array mutado) para que el
                    // Drop del statement (o el uso del valor) lo consuma.
                    self.emit_expression(&i.object)?;
                }
                Ok(())
            }
            Expression::MemberAccess(m) => {
                // `Clase.campo = v` (campo estÃ¡tico) â†’ global.set.
                if let Expression::Identifier(cn, _) = &*m.object {
                    if let Some(&g) = self.static_fields.get(&format!("{}::{}", cn, m.member)) {
                        if is_compound(op) {
                            return Err(crate::error::ClsError::CompileError(
                                "Operadores compuestos sobre campos estÃ¡ticos no soportados en el JIT"
                                    .to_string(),
                            ));
                        }
                        self.emit_expression(&a.value)?;
                        self.body.push(Instruction::GlobalSet(g));
                        let w = self.value_type(&a.value)?;
                        self.body.push(match w {
                            WasTy::F64 => Instruction::GlobalGet(g),
                            _ => Instruction::GlobalGet(g),
                        });
                        return Ok(());
                    }
                }
                let obj_ty = self.types.get(&expr_span(&m.object)).cloned();
                if let Some(Type::Named(name, _)) = obj_ty {
                    if let Some(info) = self.class_defs.get(name.as_str()) {
                        if is_compound(op) {
                            return Err(crate::error::ClsError::CompileError(
                                "Operadores compuestos sobre campos de clase no soportados en el JIT (B3)".to_string(),
                            ));
                        }
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
                        // Escritura: private/protected desde fuera, o readonly.
                        self.check_field_access(name.as_str(), m.member.as_str(), *vis, &m.span)?;
                        if vis.is_readonly() {
                            // readonly: solo escritura interna (me.campo).
                            let inside = self
                                .current_class
                                .as_deref()
                                .map(|c| c == name.as_str())
                                .unwrap_or(false);
                            if !inside {
                                return Err(crate::error::ClsError::compile_at(
                                    &format!(
                                        "El campo '{}' es readonly (solo se puede escribir desde la clase)",
                                        m.member
                                    ),
                                    &m.span,
                                ));
                            }
                        }
                        let w = *w;
                        let off = *off;
                        let obj_tmp = self.fresh_local();
                        let val_tmp = self.fresh_local_ty(w);
                        self.emit_expression(&m.object)?;
                        self.body.push(Instruction::LocalSet(obj_tmp));
                        self.emit_expression(&a.value)?;
                        self.body.push(match w {
                            WasTy::F64 => Instruction::LocalSet(val_tmp),
                            WasTy::I32 => Instruction::LocalSet(val_tmp),
                            WasTy::I64 => Instruction::LocalSet(val_tmp),
                        });
                        self.body.push(Instruction::LocalGet(obj_tmp));
                        self.body.push(Instruction::I64Const(off));
                        self.body.push(Instruction::I64Add);
                        self.body.push(Instruction::I32WrapI64);
                        self.body.push(match w {
                            WasTy::F64 => Instruction::LocalGet(val_tmp),
                            WasTy::I32 => Instruction::LocalGet(val_tmp),
                            WasTy::I64 => Instruction::LocalGet(val_tmp),
                        });
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
                        self.body.push(match w {
                            WasTy::F64 => Instruction::LocalGet(val_tmp),
                            WasTy::I32 => Instruction::LocalGet(val_tmp),
                            WasTy::I64 => Instruction::LocalGet(val_tmp),
                        });
                        return Ok(());
                    }
                }
                // Struct: `p.campo = val` â†’ store por offset del campo.
                if let Some(Type::Named(sn, _)) = self.types.get(&expr_span(&m.object)).cloned() {
                    if let Some(info) = self.struct_defs.get(sn.as_str()) {
                        if is_compound(op) {
                            return Err(crate::error::ClsError::compile_at(
                                "Operadores compuestos sobre campos de struct no soportados en el JIT",
                                &m.span,
                            ));
                        }
                        let fidx = info
                            .fields
                            .iter()
                            .position(|(n, _, _)| *n == m.member)
                            .ok_or_else(|| {
                                crate::error::ClsError::compile_at(
                                    &format!("El campo '{}' no existe en el struct '{}'", m.member, sn),
                                    &m.span,
                                )
                            })?;
                        let w = info.fields[fidx].2;
                        let off = info.offsets[fidx];
                        let obj_tmp = self.fresh_local();
                        let val_tmp = self.fresh_local_ty(w);
                        self.emit_expression(&m.object)?;
                        self.body.push(Instruction::LocalSet(obj_tmp));
                        self.emit_expression(&a.value)?;
                        self.body.push(match w {
                            WasTy::F64 => Instruction::LocalSet(val_tmp),
                            WasTy::I32 => Instruction::LocalSet(val_tmp),
                            WasTy::I64 => Instruction::LocalSet(val_tmp),
                        });
                        self.body.push(Instruction::LocalGet(obj_tmp));
                        self.body.push(Instruction::I64Const(off));
                        self.body.push(Instruction::I64Add);
                        self.body.push(Instruction::I32WrapI64);
                        self.body.push(match w {
                            WasTy::F64 => Instruction::LocalGet(val_tmp),
                            WasTy::I32 => Instruction::LocalGet(val_tmp),
                            WasTy::I64 => Instruction::LocalGet(val_tmp),
                        });
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
                        self.body.push(match w {
                            WasTy::F64 => Instruction::LocalGet(val_tmp),
                            WasTy::I32 => Instruction::LocalGet(val_tmp),
                            WasTy::I64 => Instruction::LocalGet(val_tmp),
                        });
                        return Ok(());
                    }
                }
                // Record con shape: r.campo = val â†’ store por offset (campo existente).
                if let Some(Type::Shape(fields)) = self.types.get(&expr_span(&m.object)).cloned() {                    if is_compound(op) {
                        return Err(crate::error::ClsError::CompileError(
                            "Operadores compuestos sobre campos de record con shape no soportados en el JIT".to_string(),
                        ));
                    }
                    let (_, w, off) = self.shape_layout(&fields)?
                        .into_iter()
                        .find(|(n, _, _)| *n == m.member)
                        .ok_or_else(|| crate::error::ClsError::compile_at(
                            &format!("El record no tiene el campo '{}' (no se pueden agregar campos a un shape)", m.member),
                            &m.span,
                        ))?;
                    let obj_tmp = self.fresh_local();
                    let val_tmp = self.fresh_local_ty(w);
                    self.emit_expression(&m.object)?;
                    self.body.push(Instruction::LocalSet(obj_tmp));
                    self.emit_expression(&a.value)?;
                    self.body.push(match w {
                        WasTy::F64 => Instruction::LocalSet(val_tmp),
                        WasTy::I32 => Instruction::LocalSet(val_tmp),
                        WasTy::I64 => Instruction::LocalSet(val_tmp),
                    });
                    self.body.push(Instruction::LocalGet(obj_tmp));
                    self.body.push(Instruction::I64Const(off));
                    self.body.push(Instruction::I64Add);
                    self.body.push(Instruction::I32WrapI64);
                    self.body.push(match w {
                        WasTy::F64 => Instruction::LocalGet(val_tmp),
                        WasTy::I32 => Instruction::LocalGet(val_tmp),
                        WasTy::I64 => Instruction::LocalGet(val_tmp),
                    });
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
                    self.body.push(match w {
                        WasTy::F64 => Instruction::LocalGet(val_tmp),
                        WasTy::I32 => Instruction::LocalGet(val_tmp),
                        WasTy::I64 => Instruction::LocalGet(val_tmp),
                    });
                    return Ok(());
                }
                Err(self.unsupported_expr(&Expression::MemberAccess(m.clone())))
            }
            other => Err(self.unsupported_expr(other)),
        }
    }

    /// `.join(sep)` sobre una tupla: unroll estÃ¡tico (slots conocidos en compile-time).
    fn emit_tuple_join(&mut self, member: &MemberAccessExpr, c: &CallExpr) -> ClsResult<()> {
        let obj_ty = self
            .types
            .get(&expr_span(&member.object))
            .cloned()
            .unwrap_or(Type::Any);
        let slots = match &obj_ty {
            Type::Tuple(s) => s.clone(),
            _ => vec![],
        };
        self.emit_expression(&member.object)?;
        let ptr = self.fresh_local();
        self.body.push(Instruction::LocalSet(ptr));
        self.emit_expression(&c.args[0])?;
        let sep = self.fresh_local();
        self.body.push(Instruction::LocalSet(sep));
        let empty = self.intern_string("");
        self.emit_load_str(empty);
        let res = self.fresh_local();
        self.body.push(Instruction::LocalSet(res));
        for (i, slot) in slots.iter().enumerate() {
            if i > 0 {
                self.body.push(Instruction::LocalGet(res));
                self.body.push(Instruction::LocalGet(sep));
                self.host.call(HostFn::StrConcat, &mut self.body);
                self.body.push(Instruction::LocalSet(res));
            }
            let slot_ty = was_type(slot)?;
            let s_tmp = self.fresh_local();
            self.body.push(Instruction::LocalGet(ptr));
            self.body.push(Instruction::I64Const(16 + (i as i64) * 8));
            self.body.push(Instruction::I64Add);
            self.body.push(Instruction::I32WrapI64);
            match slot_ty {
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
            match (slot_ty, slot) {
                (WasTy::F64, _) => self.host.call(HostFn::StrFloat, &mut self.body),
                (WasTy::I32, Type::Bool) => self.host.call(HostFn::StrBool, &mut self.body),
                (WasTy::I32, _) => self.host.call(HostFn::StrChar, &mut self.body),
                (WasTy::I64, Type::String) => {}
                (WasTy::I64, _) => self.host.call(HostFn::StrInt, &mut self.body),
            }
            self.body.push(Instruction::LocalSet(s_tmp));
            self.body.push(Instruction::LocalGet(res));
            self.body.push(Instruction::LocalGet(s_tmp));
            self.host.call(HostFn::StrConcat, &mut self.body);
            self.body.push(Instruction::LocalSet(res));
        }
        self.body.push(Instruction::LocalGet(res));
        Ok(())
    }

    /// `math.X(...)` â†’ host del mÃ³dulo math.
    fn emit_math_call(&mut self, member: &MemberAccessExpr, c: &CallExpr) -> ClsResult<()> {
        use HostFn::*;
        match member.member.as_str() {
            "abs" => {
                self.emit_expression(&c.args[0])?;
                match self.value_type(&c.args[0])? {
                    WasTy::F64 => self.host.call(FloatAbs, &mut self.body),
                    _ => self.host.call(IntAbs, &mut self.body),
                }
                Ok(())
            }
            "sqrt" => {
                self.emit_expression(&c.args[0])?;
                self.f64_promote(&c.args[0])?;
                self.host.call(MathSqrt, &mut self.body);
                Ok(())
            }
            "floor" => {
                self.emit_expression(&c.args[0])?;
                self.f64_promote(&c.args[0])?;
                self.host.call(MathFloor, &mut self.body);
                Ok(())
            }
            "ceil" => {
                self.emit_expression(&c.args[0])?;
                self.f64_promote(&c.args[0])?;
                self.host.call(MathCeil, &mut self.body);
                Ok(())
            }
            "round" => {
                self.emit_expression(&c.args[0])?;
                self.f64_promote(&c.args[0])?;
                self.host.call(MathRound, &mut self.body);
                Ok(())
            }
            "sin" => {
                self.emit_expression(&c.args[0])?;
                self.f64_promote(&c.args[0])?;
                self.host.call(MathSin, &mut self.body);
                Ok(())
            }
            "cos" => {
                self.emit_expression(&c.args[0])?;
                self.f64_promote(&c.args[0])?;
                self.host.call(MathCos, &mut self.body);
                Ok(())
            }
            "tan" => {
                self.emit_expression(&c.args[0])?;
                self.f64_promote(&c.args[0])?;
                self.host.call(MathTan, &mut self.body);
                Ok(())
            }
            "log" => {
                self.emit_expression(&c.args[0])?;
                self.f64_promote(&c.args[0])?;
                self.host.call(MathLog, &mut self.body);
                Ok(())
            }
            "pow" => {
                self.emit_expression(&c.args[0])?;
                self.f64_promote(&c.args[0])?;
                self.emit_expression(&c.args[1])?;
                self.f64_promote(&c.args[1])?;
                self.host.call(MathPow, &mut self.body);
                Ok(())
            }
            "min" => {
                self.emit_expression(&c.args[0])?;
                self.f64_promote(&c.args[0])?;
                self.emit_expression(&c.args[1])?;
                self.f64_promote(&c.args[1])?;
                self.host.call(MathMin, &mut self.body);
                Ok(())
            }
            "max" => {
                self.emit_expression(&c.args[0])?;
                self.f64_promote(&c.args[0])?;
                self.emit_expression(&c.args[1])?;
                self.f64_promote(&c.args[1])?;
                self.host.call(MathMax, &mut self.body);
                Ok(())
            }
            "random" => {
                self.host.call(MathRandom, &mut self.body);
                Ok(())
            }
            "range" => {
                self.emit_expression(&c.args[0])?;
                self.emit_expression(&c.args[1])?;
                self.host.call(MathRange, &mut self.body);
                Ok(())
            }
            _ => Err(self.unsupported_expr(&Expression::Call(c.clone()))),
        }
    }

    /// `fs.X(...)` â†’ host del mÃ³dulo fs (bÃ¡sico: exists/cwd/readFile/writeFile/listDir/mkdir/rm).
    fn emit_fs_call(&mut self, member: &MemberAccessExpr, c: &CallExpr) -> ClsResult<()> {
        use HostFn::*;
        match member.member.as_str() {
            "exists" => {
                self.emit_expression(&c.args[0])?;
                self.host.call(FsExists, &mut self.body);
                Ok(())
            }
            "cwd" => {
                self.host.call(FsCwd, &mut self.body);
                Ok(())
            }
            "readFile" => {
                self.emit_expression(&c.args[0])?;
                self.host.call(FsReadFile, &mut self.body);
                Ok(())
            }
            "writeFile" => {
                self.emit_expression(&c.args[0])?;
                self.emit_expression(&c.args[1])?;
                self.host.call(FsWriteFile, &mut self.body);
                Ok(())
            }
            "listDir" => {
                self.emit_expression(&c.args[0])?;
                self.host.call(FsListDir, &mut self.body);
                Ok(())
            }
            "mkdir" => {
                self.emit_expression(&c.args[0])?;
                self.host.call(FsMkdir, &mut self.body);
                Ok(())
            }
            "rm" => {
                self.emit_expression(&c.args[0])?;
                self.host.call(FsRm, &mut self.body);
                Ok(())
            }
            _ => Err(self.unsupported_expr(&Expression::Call(c.clone()))),
        }
    }

    /// `http.X(...)` â†’ host del mÃ³dulo http.
    fn emit_http_call(&mut self, member: &MemberAccessExpr, c: &CallExpr) -> ClsResult<()> {
        use HostFn::*;
        match member.member.as_str() {
            "get" => {
                self.emit_expression(&c.args[0])?;
                self.host.call(HttpGet, &mut self.body);
                Ok(())
            }
            "post" => {
                self.emit_expression(&c.args[0])?;
                self.emit_expression(&c.args[1])?;
                self.host.call(HttpPost, &mut self.body);
                Ok(())
            }
            _ => Err(self.unsupported_expr(&Expression::Call(c.clone()))),
        }
    }

    /// `os.X(...)` â†’ host del mÃ³dulo os.
    fn emit_os_call(&mut self, member: &MemberAccessExpr, c: &CallExpr) -> ClsResult<()> {
        use HostFn::*;
        match member.member.as_str() {
            "platform" => self.host.call(OsPlatform, &mut self.body),
            "arch" => self.host.call(OsArch, &mut self.body),
            "version" => self.host.call(OsVersion, &mut self.body),
            "hostname" => self.host.call(OsHostname, &mut self.body),
            "home" => self.host.call(OsHome, &mut self.body),
            "tempdir" => self.host.call(OsTempdir, &mut self.body),
            "cpus" => self.host.call(OsCpus, &mut self.body),
            "pid" => self.host.call(OsPid, &mut self.body),
            "uptime" => self.host.call(OsUptime, &mut self.body),
            "env" => {
                self.emit_expression(self.call_arg(c, 0, "os.env")?)?;
                self.host.call(OsEnv, &mut self.body);
            }
            "sep" => self.host.call(OsSep, &mut self.body),
            "isWindows" => self.host.call(OsIsWindows, &mut self.body),
            "isUnix" => self.host.call(OsIsUnix, &mut self.body),
            _ => return Err(self.unsupported_expr(&Expression::Call(c.clone()))),
        }
        Ok(())
    }

    /// `path.X(...)` â†’ host del mÃ³dulo path.
    fn emit_path_call(&mut self, member: &MemberAccessExpr, c: &CallExpr) -> ClsResult<()> {
        use HostFn::*;
        match member.member.as_str() {
            "join" => {
                self.emit_expression(self.call_arg(c, 0, "path.join")?)?;
                self.emit_expression(self.call_arg(c, 1, "path.join")?)?;
                self.host.call(PathJoin, &mut self.body);
            }
            "basename" => {
                self.emit_expression(self.call_arg(c, 0, "path.basename")?)?;
                self.host.call(PathBasename, &mut self.body);
            }
            "dirname" => {
                self.emit_expression(self.call_arg(c, 0, "path.dirname")?)?;
                self.host.call(PathDirname, &mut self.body);
            }
            "extname" => {
                self.emit_expression(self.call_arg(c, 0, "path.extname")?)?;
                self.host.call(PathExtname, &mut self.body);
            }
            "resolve" => {
                self.emit_expression(self.call_arg(c, 0, "path.resolve")?)?;
                self.host.call(PathResolve, &mut self.body);
            }
            "normalize" => {
                self.emit_expression(self.call_arg(c, 0, "path.normalize")?)?;
                self.host.call(PathNormalize, &mut self.body);
            }
            "isAbsolute" => {
                self.emit_expression(self.call_arg(c, 0, "path.isAbsolute")?)?;
                self.host.call(PathIsAbsolute, &mut self.body);
            }
            "sep" => self.host.call(PathSep, &mut self.body),
            _ => return Err(self.unsupported_expr(&Expression::Call(c.clone()))),
        }
        Ok(())
    }

    /// `process.X(...)` â†’ host del mÃ³dulo process.
    fn emit_process_call(&mut self, member: &MemberAccessExpr, c: &CallExpr) -> ClsResult<()> {
        use HostFn::*;
        match member.member.as_str() {
            "args" => self.host.call(ProcessArgs, &mut self.body),
            "cwd" => self.host.call(ProcessCwd, &mut self.body),
            "env" => {
                self.emit_expression(self.call_arg(c, 0, "process.env")?)?;
                self.host.call(ProcessEnv, &mut self.body);
            }
            "exit" => {
                self.emit_expression(self.call_arg(c, 0, "process.exit")?)?;
                self.host.call(ProcessExit, &mut self.body);
            }
            "pid" => self.host.call(ProcessPid, &mut self.body),
            "platform" => self.host.call(ProcessPlatform, &mut self.body),
            "title" => self.host.call(ProcessTitle, &mut self.body),
            _ => return Err(self.unsupported_expr(&Expression::Call(c.clone()))),
        }
        Ok(())
    }

    /// `time.X(...)` â†’ host del mÃ³dulo time.
    fn emit_time_call(&mut self, member: &MemberAccessExpr, c: &CallExpr) -> ClsResult<()> {
        use HostFn::*;
        match member.member.as_str() {
            "now" => self.host.call(TimeNow, &mut self.body),
            "seconds" => self.host.call(TimeSeconds, &mut self.body),
            "iso" => self.host.call(TimeIso, &mut self.body),
            "date" => self.host.call(TimeDate, &mut self.body),
            "clock" => self.host.call(TimeClock, &mut self.body),
            "year" => self.host.call(TimeYear, &mut self.body),
            "month" => self.host.call(TimeMonth, &mut self.body),
            "day" => self.host.call(TimeDay, &mut self.body),
            "hour" => self.host.call(TimeHour, &mut self.body),
            "minute" => self.host.call(TimeMinute, &mut self.body),
            "second" => self.host.call(TimeSecond, &mut self.body),
            "sleep" => {
                self.emit_expression(self.call_arg(c, 0, "time.sleep")?)?;
                self.host.call(TimeSleep, &mut self.body);
            }
            _ => return Err(self.unsupported_expr(&Expression::Call(c.clone()))),
        }
        Ok(())
    }

    /// `random.X(...)` â†’ host del mÃ³dulo random.
    fn emit_random_call(&mut self, member: &MemberAccessExpr, c: &CallExpr) -> ClsResult<()> {
        use HostFn::*;
        match member.member.as_str() {
            "random" => self.host.call(RandomRandom, &mut self.body),
            "int" => {
                self.emit_expression(self.call_arg(c, 0, "random.int")?)?;
                self.emit_expression(self.call_arg(c, 1, "random.int")?)?;
                self.host.call(RandomInt, &mut self.body);
            }
            "float" => {
                let a0 = self.call_arg(c, 0, "random.float")?;
                let a1 = self.call_arg(c, 1, "random.float")?;
                self.emit_expression(a0)?;
                self.f64_promote(a0)?;
                self.emit_expression(a1)?;
                self.f64_promote(a1)?;
                self.host.call(RandomFloat, &mut self.body);
            }
            "uuid" => self.host.call(RandomUuid, &mut self.body),
            _ => return Err(self.unsupported_expr(&Expression::Call(c.clone()))),
        }
        Ok(())
    }

    /// Valida la aridad de una llamada a host de mÃ³dulo y devuelve el arg `i`.
    /// Evita `c.args[i]` con Ã­ndice fuera de rango (panic â†’ error de compilaciÃ³n).
    fn call_arg<'e>(&self, c: &'e CallExpr, i: usize, fn_name: &str) -> ClsResult<&'e Expression> {
        c.args.get(i).ok_or_else(|| {
            crate::error::ClsError::compile_at(
                &format!("{} esperaba {} argumento(s), recibiÃ³ {}", fn_name, i + 1, c.args.len()),
                &c.span,
            )
        })
    }

    /// Tipo de retorno de una llamada o miembro de un mÃ³dulo stdlib.
    fn module_call_ret(&self, expr: &Expression) -> Option<WasTy> {
        if let Expression::Call(c) = expr {
            if let Expression::MemberAccess(member) = &*c.callee {
                if let Expression::Identifier(obj, _) = &*member.object {
                    if obj == "math" {
                        return match member.member.as_str() {
                            "sqrt" | "pow" | "min" | "max" | "floor" | "ceil" | "round"
                            | "random" | "sin" | "cos" | "tan" | "log" => Some(WasTy::F64),
                            "range" => Some(WasTy::I64),
                            // `abs` devuelve el tipo del primer argumento.
                            "abs" => {
                                let arg_ty = c.args.first()
                                    .and_then(|a| self.types.get(&expr_span(a)))
                                    .cloned()
                                    .unwrap_or(Type::Any);
                                if matches!(arg_ty, Type::Float | Type::F32 | Type::F64) {
                                    Some(WasTy::F64)
                                } else {
                                    Some(WasTy::I64)
                                }
                            }
                            _ => None,
                        };
                    }
                    if obj == "json" && member.member == "stringify" {
                        return Some(WasTy::I64);
                    }
                    if obj == "json" && member.member == "parse" {
                        return Some(WasTy::I64);
                    }
                    if obj == "fs" {
                        return match member.member.as_str() {
                            "exists" => Some(WasTy::I32),
                            _ => Some(WasTy::I64),
                        };
                    }
                    if obj == "http" {
                        return Some(WasTy::I64);
                    }
                    if obj == "os" {
                        return match member.member.as_str() {
                            "isWindows" | "isUnix" => Some(WasTy::I32),
                            _ => Some(WasTy::I64),
                        };
                    }
                    if obj == "path" {
                        return match member.member.as_str() {
                            "isAbsolute" => Some(WasTy::I32),
                            _ => Some(WasTy::I64),
                        };
                    }
                    if obj == "process" {
                        // exit es void: no reportar valor (romperÃ­a `print(exit(0))`).
                        return match member.member.as_str() {
                            "exit" => None,
                            _ => Some(WasTy::I64),
                        };
                    }
                    if obj == "time" {
                        // sleep es void: no reportar valor.
                        return match member.member.as_str() {
                            "sleep" => None,
                            _ => Some(WasTy::I64),
                        };
                    }
                    if obj == "random" {
                        return match member.member.as_str() {
                            "random" | "float" => Some(WasTy::F64),
                            "int" => Some(WasTy::I64),
                            _ => Some(WasTy::I64),
                        };
                    }
                }
            }
        }
        // Miembros de mÃ³dulos sin llamada: math.PI / math.E
        if let Expression::MemberAccess(member) = expr {
            if let Expression::Identifier(obj, _) = &*member.object {
                if obj == "math" && (member.member == "PI" || member.member == "E") {
                    return Some(WasTy::F64);
                }
            }
        }
        None
    }

    fn emit_call(&mut self, c: &CallExpr) -> ClsResult<()> {
        // Constructor de structure: `Punto(3, 4)` â†’ alloc + stores.
        if let Expression::Identifier(name, _) = &*c.callee {
            if let Some(info) = self.struct_defs.get(name).cloned() {
                self.body.push(Instruction::I64Const(info.total));
                let alloc = self.func_indexes["__alloc"];
                self.body.push(Instruction::Call(alloc));
                let ptr = self.fresh_local();
                self.body.push(Instruction::LocalSet(ptr));
                self.body.push(Instruction::LocalGet(ptr));
                self.body.push(Instruction::I64Const(info.def_id as i64));
                self.emit_i64_store(0);
                self.body.push(Instruction::LocalGet(ptr));
                self.body
                    .push(Instruction::I64Const(info.fields.len() as i64));
                self.emit_i64_store(8);
                for (i, (_, _, w)) in info.fields.iter().enumerate() {
                    if i < c.args.len() {
                        self.emit_expression(&c.args[i])?;
                    } else {
                        self.body.push(Instruction::I64Const(0));
                    }
                    let val_tmp = self.fresh_local_ty(*w);
                    let addr_tmp = self.fresh_local();
                    self.body.push(match w {
                        WasTy::F64 => Instruction::LocalSet(val_tmp),
                        WasTy::I32 => Instruction::LocalSet(val_tmp),
                        WasTy::I64 => Instruction::LocalSet(val_tmp),
                    });
                    self.body.push(Instruction::LocalGet(ptr));
                    self.body.push(Instruction::I64Const(info.offsets[i]));
                    self.body.push(Instruction::I64Add);
                    self.body.push(Instruction::LocalSet(addr_tmp));
                    self.body.push(Instruction::LocalGet(addr_tmp));
                    self.body.push(Instruction::I32WrapI64);
                    self.body.push(match w {
                        WasTy::F64 => Instruction::LocalGet(val_tmp),
                        WasTy::I32 => Instruction::LocalGet(val_tmp),
                        WasTy::I64 => Instruction::LocalGet(val_tmp),
                    });
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
                return Ok(());
            }
        }
        // Constructor de clase: `Clase(args)` â†’ alloc + vtable + init fields + ctor.
        if let Expression::Identifier(name, _) = &*c.callee {
            if let Some(info) = self.class_defs.get(name).cloned() {
                self.body.push(Instruction::I64Const(info.total));
                let alloc = self.func_indexes["__alloc"];
                self.body.push(Instruction::Call(alloc));
                let obj = self.fresh_local();
                self.body.push(Instruction::LocalSet(obj));
                // vtable_ptr[0] = vtable_start, class_id[8] = id
                self.body.push(Instruction::LocalGet(obj));
                self.body
                    .push(Instruction::I64Const(info.vtable_start as i64));
                self.emit_i64_store(0);
                self.body.push(Instruction::LocalGet(obj));
                self.body.push(Instruction::I64Const(info.class_id as i64));
                self.emit_i64_store(8);
                // init fields a 0
                for (_fn, _t, w, off, _vis) in &info.fields {
                    self.body.push(Instruction::LocalGet(obj));
                    self.body.push(Instruction::I64Const(*off));
                    self.body.push(Instruction::I64Add);
                    self.body.push(Instruction::I32WrapI64);
                    match w {
                        WasTy::F64 => self
                            .body
                            .push(Instruction::F64Const(Ieee64::new(0.0f64.to_bits()))),
                        WasTy::I32 => self.body.push(Instruction::I32Const(0)),
                        WasTy::I64 => self.body.push(Instruction::I64Const(0)),
                    }
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
                // call Clase::__ctor (o el del padre si no se define) con me.
                // Solo se pushea `me`+args si EXISTE el ctor: si la clase no lo
                // define, el stack debe quedar limpio (el leftover rompÃ­a la
                // validaciÃ³n WASM en `__init_globals`, que no tiene resultado
                // que lo consuma â€” el modo archivo lo enmascaraba con `return`).
                let callsite = c.span.clone();
                let mut cur = Some(name.to_string());
                while let Some(cls) = cur {
                    if let Some(idx) = self.func_indexes.get(&format!("{}::__ctor", cls)) {
                        self.body.push(Instruction::LocalGet(obj));
                        for a in &c.args {
                            self.emit_expression(a)?;
                        }
                        self.emit_call_site(&callsite);
                        self.body.push(Instruction::Call(*idx));
                        break;
                    }
                    cur = self.class_defs.get(&cls).and_then(|i| i.parent.clone());
                }
                self.body.push(Instruction::LocalGet(obj));
                return Ok(());
            }
        }
        // Llamada a funciÃ³n nativa (extensiÃ³n): import `env.<sym>__<sig>@<lib>`.
        if let Expression::Identifier(name, _) = &*c.callee {
            if let Some(idx) = self.native_indexes.get(name) {
                for a in &c.args {
                    self.emit_expression(a)?;
                }
                self.body.push(Instruction::Call(*idx));
                return Ok(());
            }
        }
        // MÃ©todos de primitivos (callee MemberAccess)
        if let Expression::MemberAccess(member) = &*c.callee {
            // `super.m(args)` â†’ call directo al mÃ©todo del padre (sin vtable).
            if let Expression::Identifier(sn, _) = &*member.object {
                if sn == "super" {
                    if let Some(cur) = &self.current_class {
                        if let Some(parent) =
                            self.class_defs.get(cur).and_then(|i| i.parent.clone())
                        {
                            // `super.main(...)` â†’ ctor del padre (ClassDef.ctor se
                            // emite como `__ctor`). `super.metodo(...)` â†’ mÃ©todo.
                            let key = if member.member == "main" {
                                format!("{}::__ctor", parent)
                            } else {
                                format!("{}::{}", parent, member.member)
                            };
                            if let Some(idx) = self.func_indexes.get(&key) {
                                self.body.push(Instruction::LocalGet(0)); // me
                                for a in &c.args {
                                    self.emit_expression(a)?;
                                }
                                self.emit_call_site(&c.span);
                                self.body.push(Instruction::Call(*idx));
                                return Ok(());
                            }
                        }
                    }
                    return Err(crate::error::ClsError::CompileError(
                        "super solo se puede usar dentro de mÃ©todos de clase (JIT)".to_string(),
                    ));
                }
            }
            // MÃ³dulos stdlib: math / json / fs
            if let Expression::Identifier(obj_name, _) = &*member.object {
                if obj_name == "math" {
                    return self.emit_math_call(member, c);
                }
                if obj_name == "json" {
                    if member.member == "parse" {
                        self.emit_expression(&c.args[0])?;
                        self.host.call(HostFn::JsonParse, &mut self.body);
                        return Ok(());
                    }
                    if member.member == "stringify" {
                        let t = self
                            .types
                            .get(&expr_span(&c.args[0]))
                            .cloned()
                            .unwrap_or(Type::Any);
                        // Objeto de clase: __toJson si lo define; si no â†’ "null" (paridad walker).
                        if let Type::Named(cn, _) = &t {
                            if self.class_defs.contains_key(cn.as_str()) {
                                if self.emit_class_method("__toJson", &c.args[0])? {
                                    return Ok(());
                                }
                                self.emit_expression(&c.args[0])?;
                                self.body.push(Instruction::Drop);
                                let n = self.intern_string("null");
                                self.emit_load_str(n);
                                return Ok(());
                            }
                            // struct/enum sin serializaciÃ³n â†’ "null" (paridad walker).
                            if self.struct_defs.contains_key(cn.as_str())
                                || self.enum_defs.contains_key(cn.as_str())
                            {
                                self.emit_expression(&c.args[0])?;
                                self.body.push(Instruction::Drop);
                                let n = self.intern_string("null");
                                self.emit_load_str(n);
                                return Ok(());
                            }
                        }
                        // Shape â†’ stringify inline (json.stringify({x:1}) â†’ '{"x":1}').
                        if let Type::Shape(fields) = &t {
                            return self.emit_shape_to_json_string(&c.args[0], fields);
                        }
                        self.emit_expression(&c.args[0])?;
                        let kind = match t {
                            Type::Record(_, _) => 1,
                            Type::Array(_) => 2,
                            _ => 0,
                        };
                        self.body.push(Instruction::I64Const(kind));
                        self.host.call(HostFn::JsonStringify, &mut self.body);
                        return Ok(());
                    }
                }
                if obj_name == "fs" {
                    return self.emit_fs_call(member, c);
                }
                if obj_name == "http" {
                    return self.emit_http_call(member, c);
                }
                if obj_name == "os" {
                    return self.emit_os_call(member, c);
                }
                if obj_name == "path" {
                    return self.emit_path_call(member, c);
                }
                if obj_name == "process" {
                    return self.emit_process_call(member, c);
                }
                if obj_name == "time" {
                    return self.emit_time_call(member, c);
                }
                if obj_name == "random" {
                    return self.emit_random_call(member, c);
                }
                // `Clase.metodo()` con mÃ©todo static â†’ call directo (sin me).
                if self.class_defs.contains_key(obj_name.as_str()) {
                    let skey = format!("{}::__s__{}", obj_name, member.member);
                    if let Some(&idx) = self.func_indexes.get(&skey) {
                        for a in &c.args {
                            self.emit_expression(a)?;
                        }
                        self.emit_call_site(&c.span);
                        self.body.push(Instruction::Call(idx));
                        return Ok(());
                    }
                }
            }
            let obj_ty = self
                .types
                .get(&expr_span(&member.object))
                .cloned()
                .unwrap_or(Type::Any);
            match obj_ty {
                Type::Tuple(_) => match member.member.as_str() {
                    "join" => return self.emit_tuple_join(member, c),
                    _ => return Err(self.unsupported_expr(&Expression::Call(c.clone()))),
                },
                Type::String => {
                    self.emit_expression(&member.object)?;
                    match member.member.as_str() {
                        "upper" | "lower" | "trim" => {
                            let h = match member.member.as_str() {
                                "upper" => HostFn::StrUpper,
                                "lower" => HostFn::StrLower,
                                _ => HostFn::StrTrim,
                            };
                            self.host.call(h, &mut self.body);
                            return Ok(());
                        }
                        "contains" | "startsWith" | "endsWith" => {
                            self.emit_expression(&c.args[0])?;
                            let h = match member.member.as_str() {
                                "contains" => HostFn::StrContains,
                                "startsWith" => HostFn::StrStartsWith,
                                _ => HostFn::StrEndsWith,
                            };
                            self.host.call(h, &mut self.body);
                            return Ok(());
                        }
                        "isEmpty" => {
                            self.host.call(HostFn::StrIsEmpty, &mut self.body);
                            return Ok(());
                        }
                        "toString" => return Ok(()),
                        _ => return Err(self.unsupported_expr(&Expression::Call(c.clone()))),
                    }
                }
                Type::Array(_) => {
                    let elem_ty = self.array_elem_was_type(&member.object)?;
                    let elem_size = elem_size_bytes(elem_ty);
                    self.emit_expression(&member.object)?;
                    match member.member.as_str() {
                        "push" => {
                            self.emit_expression(&c.args[0])?;
                            self.elem_to_bits(&c.args[0], elem_ty)?;
                            self.body.push(Instruction::I64Const(elem_size));
                            self.host.call(HostFn::ArrPush, &mut self.body);
                            self.writeback_array(&member.object)?;
                            return Ok(());
                        }
                        "pop" => {
                            self.body.push(Instruction::I64Const(elem_size));
                            self.host.call(HostFn::ArrPop, &mut self.body);
                            self.writeback_array(&member.object)?;
                            return Ok(());
                        }
                        "shift" => {
                            self.body.push(Instruction::I64Const(elem_size));
                            self.host.call(HostFn::ArrShift, &mut self.body);
                            self.writeback_array(&member.object)?;
                            return Ok(());
                        }
                        "unshift" => {
                            self.emit_expression(&c.args[0])?;
                            self.elem_to_bits(&c.args[0], elem_ty)?;
                            self.body.push(Instruction::I64Const(elem_size));
                            self.host.call(HostFn::ArrUnshift, &mut self.body);
                            self.writeback_array(&member.object)?;
                            return Ok(());
                        }
                        "reverse" => {
                            self.body.push(Instruction::I64Const(elem_size));
                            self.host.call(HostFn::ArrReverse, &mut self.body);
                            self.writeback_array(&member.object)?;
                            return Ok(());
                        }
                        "indexOf" => {
                            self.emit_expression(&c.args[0])?;
                            self.elem_to_bits(&c.args[0], elem_ty)?;
                            self.body.push(Instruction::I64Const(elem_size));
                            self.host.call(HostFn::ArrIndexOf, &mut self.body);
                            return Ok(());
                        }
                        "includes" => {
                            self.emit_expression(&c.args[0])?;
                            self.elem_to_bits(&c.args[0], elem_ty)?;
                            self.body.push(Instruction::I64Const(elem_size));
                            self.host.call(HostFn::ArrIncludes, &mut self.body);
                            return Ok(());
                        }
                        "join" => {
                            self.emit_expression(&c.args[0])?;
                            self.body.push(Instruction::I64Const(elem_size));
                            let cls_t = self.array_elem_cls_type(&member.object)?;
                            self.body.push(Instruction::I64Const(arr_kind_code(&cls_t)));
                            self.host.call(HostFn::ArrJoin, &mut self.body);
                            return Ok(());
                        }
                        "map" => return self.emit_array_map(member, c, elem_ty, elem_size),
                        _ => return Err(self.unsupported_expr(&Expression::Call(c.clone()))),
                    }
                }
                Type::Record(_, _) => {
                    self.emit_expression(&member.object)?;
                    match member.member.as_str() {
                        "has" => {
                            self.emit_expression(&c.args[0])?;
                            self.host.call(HostFn::RecordHas, &mut self.body);
                            return Ok(());
                        }
                        "keys" => {
                            self.host.call(HostFn::RecordKeys, &mut self.body);
                            return Ok(());
                        }
                        "values" => {
                            self.host.call(HostFn::RecordValues, &mut self.body);
                            return Ok(());
                        }
                        _ => return Err(self.unsupported_expr(&Expression::Call(c.clone()))),
                    }
                }
                Type::Shape(fields) => {
                    match member.member.as_str() {
                        "has" => {
                            // Compile-time: si la clave (literal) estÃ¡ en el shape.
                            let has = match &c.args[0] {
                                Expression::Literal(l)
                                    if matches!(l.kind, LiteralKind::String(_)) =>
                                {
                                    match &l.kind {
                                        LiteralKind::String(k) => {
                                            fields.iter().any(|(n, _)| *n == *k)
                                        }
                                        _ => false,
                                    }
                                }
                                _ => true, // clave dinÃ¡mica â†’ se asume que puede existir
                            };
                            self.body
                                .push(Instruction::I32Const(if has { 1 } else { 0 }));
                            return Ok(());
                        }
                        "keys" => {
                            // Construir array<String> con las keys del shape.
                            let mut sorted: Vec<&String> = fields.iter().map(|(n, _)| n).collect();
                            sorted.sort();
                            let n = sorted.len() as i64;
                            let es = 8i64;
                            self.body.push(Instruction::I64Const(n));
                            self.body.push(Instruction::I64Const(es));
                            self.body.push(Instruction::I64Mul);
                            self.body.push(Instruction::I64Const(16));
                            self.body.push(Instruction::I64Add);
                            let alloc = self.func_indexes["__alloc"];
                            self.body.push(Instruction::Call(alloc));
                            let ptr = self.fresh_local();
                            self.body.push(Instruction::LocalSet(ptr));
                            self.body.push(Instruction::LocalGet(ptr));
                            self.body.push(Instruction::I64Const(n));
                            self.emit_i64_store(0);
                            self.body.push(Instruction::LocalGet(ptr));
                            self.body.push(Instruction::I64Const(n));
                            self.emit_i64_store(8);
                            for (i, k) in sorted.iter().enumerate() {
                                self.body.push(Instruction::LocalGet(ptr));
                                self.body.push(Instruction::I64Const(16 + (i as i64) * 8));
                                self.body.push(Instruction::I64Add);
                                self.body.push(Instruction::I32WrapI64);
                                let s = self.intern_string(k);
                                self.emit_load_str(s);
                                self.body.push(Instruction::I64Store(MemArg {
                                    offset: 0,
                                    align: 3,
                                    memory_index: 0,
                                }));
                            }
                            self.body.push(Instruction::LocalGet(ptr));
                            return Ok(());
                        }
                        "values" => {
                            // Construir array con los valores (segÃºn el tipo de cada campo).
                            self.emit_expression(&member.object)?;
                            let ptr = self.fresh_local();
                            self.body.push(Instruction::LocalSet(ptr));
                            let layout = self.shape_layout(&fields)?;
                            let mut ordered: Vec<&(String, WasTy, i64)> = layout.iter().collect();
                            ordered.sort_by(|a, b| a.0.cmp(&b.0));
                            let n = fields.len() as i64;
                            let es = 8i64;
                            self.body.push(Instruction::I64Const(n));
                            self.body.push(Instruction::I64Const(es));
                            self.body.push(Instruction::I64Mul);
                            self.body.push(Instruction::I64Const(16));
                            self.body.push(Instruction::I64Add);
                            let alloc = self.func_indexes["__alloc"];
                            self.body.push(Instruction::Call(alloc));
                            let arr = self.fresh_local();
                            self.body.push(Instruction::LocalSet(arr));
                            self.body.push(Instruction::LocalGet(arr));
                            self.body.push(Instruction::I64Const(n));
                            self.emit_i64_store(0);
                            self.body.push(Instruction::LocalGet(arr));
                            self.body.push(Instruction::I64Const(n));
                            self.emit_i64_store(8);
                            for (i, (_, w, off)) in ordered.iter().enumerate() {
                                self.body.push(Instruction::LocalGet(arr));
                                self.body.push(Instruction::I64Const(16 + (i as i64) * 8));
                                self.body.push(Instruction::I64Add);
                                self.body.push(Instruction::I32WrapI64);
                                self.body.push(Instruction::LocalGet(ptr));
                                self.body.push(Instruction::I64Const(*off));
                                self.body.push(Instruction::I64Add);
                                self.body.push(Instruction::I32WrapI64);
                                match *w {
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
                                // bits a i64 (f64 â†’ reinterpret; i32 â†’ extend)
                                match *w {
                                    WasTy::F64 => self.body.push(Instruction::I64ReinterpretF64),
                                    WasTy::I32 => self.body.push(Instruction::I64ExtendI32U),
                                    WasTy::I64 => {}
                                }
                                self.body.push(Instruction::I64Store(MemArg {
                                    offset: 0,
                                    align: 3,
                                    memory_index: 0,
                                }));
                            }
                            self.body.push(Instruction::LocalGet(arr));
                            return Ok(());
                        }
                        _ => return Err(self.unsupported_expr(&Expression::Call(c.clone()))),
                    }
                }
                Type::Named(name, _) => {
                    if let Some(info) = self.class_defs.get(name.as_str()) {
                        let method_slot = info
                            .methods
                            .iter()
                            .position(|m| *m == member.member)
                            .ok_or_else(|| {
                                crate::error::ClsError::compile_at(
                                    &format!(
                                        "El mÃ©todo '{}' no existe en la clase '{}'",
                                        member.member, name
                                    ),
                                    &member.span,
                                )
                            })? as u32;
                        // Visibilidad del mÃ©todo: private/protected desde fuera â†’ error.
                        // Se resuelve subiendo por ancestors (un mÃ©todo puede venir
                        // del padre sin override).
                        let mut vis_cls = name.to_string();
                        let vis = loop {
                            if let Some(v) = self
                                .class_defs
                                .get(&vis_cls)
                                .and_then(|i| i.method_vis.get(&member.member))
                            {
                                break Some(*v);
                            }
                            match self
                                .class_defs
                                .get(&vis_cls)
                                .and_then(|i| i.parent.clone())
                            {
                                Some(p) => vis_cls = p,
                                None => break None,
                            }
                        };
                        if let Some(v) = vis {
                            self.check_method_access(&name, &member.member, v, &member.span)?;
                        }
                        // MÃ©todo heredado sin override: buscar el Ã­ndice en la clase
                        // que lo declara (no fallar con "MÃ©todo sin tipo WASM").
                        let mut fn_cls = name.to_string();
                        let ty = loop {
                            let key = format!("{}::{}", fn_cls, member.member);
                            if let Some(t) = self.method_type_indexes.get(&key) {
                                break *t;
                            }
                            match self
                                .class_defs
                                .get(&fn_cls)
                                .and_then(|i| i.parent.clone())
                            {
                                Some(p) => fn_cls = p,
                                None => {
                                    return Err(crate::error::ClsError::compile_at(
                                        &format!(
                                            "El mÃ©todo '{}' no existe en la clase '{}'",
                                            member.member, name
                                        ),
                                        &member.span,
                                    ))
                                }
                            }
                        };
                        let obj_tmp = self.fresh_local();
                        self.emit_expression(&member.object)?;
                        self.body.push(Instruction::LocalSet(obj_tmp));
                        self.body.push(Instruction::LocalGet(obj_tmp));
                        for a in &c.args {
                            self.emit_expression(a)?;
                        }
                        // slot = vtable(obj[0]) + method_slot
                        self.body.push(Instruction::LocalGet(obj_tmp));
                        self.body.push(Instruction::I32WrapI64);
                        self.body.push(Instruction::I64Load(MemArg {
                            offset: 0,
                            align: 3,
                            memory_index: 0,
                        }));
                        self.body.push(Instruction::I64Const(method_slot as i64));
                        self.body.push(Instruction::I64Add);
                        self.body.push(Instruction::I32WrapI64);
                        self.body.push(Instruction::CallIndirect {
                            type_index: ty,
                            table_index: 0,
                        });
                        return Ok(());
                    }
                    return Err(self.unsupported_expr(&Expression::Call(c.clone())));
                }
                Type::Int => {
                    self.emit_expression(&member.object)?;
                    match member.member.as_str() {
                        "toString" => {
                            self.host.call(HostFn::StrInt, &mut self.body);
                            return Ok(());
                        }
                        "abs" => {
                            self.host.call(HostFn::IntAbs, &mut self.body);
                            return Ok(());
                        }
                        _ => return Err(self.unsupported_expr(&Expression::Call(c.clone()))),
                    }
                }
                Type::Float => {
                    self.emit_expression(&member.object)?;
                    match member.member.as_str() {
                        "toString" => {
                            self.host.call(HostFn::StrFloat, &mut self.body);
                            return Ok(());
                        }
                        "abs" => {
                            self.host.call(HostFn::FloatAbs, &mut self.body);
                            return Ok(());
                        }
                        _ => return Err(self.unsupported_expr(&Expression::Call(c.clone()))),
                    }
                }
                Type::Bool => {
                    self.emit_expression(&member.object)?;
                    match member.member.as_str() {
                        "toString" => {
                            self.host.call(HostFn::StrBool, &mut self.body);
                            return Ok(());
                        }
                        _ => return Err(self.unsupported_expr(&Expression::Call(c.clone()))),
                    }
                }
                Type::Char => {
                    self.emit_expression(&member.object)?;
                    match member.member.as_str() {
                        "toString" => {
                            self.host.call(HostFn::StrChar, &mut self.body);
                            return Ok(());
                        }
                        _ => return Err(self.unsupported_expr(&Expression::Call(c.clone()))),
                    }
                }
                _ => {}
            }
        }
        if let Expression::Identifier(name, _) = &*c.callee {
            match name.as_str() {
                "throw" => {
                    // throw(msg) â†’ excepciÃ³n CLS (tag con payload msg + span).
                    if !self.exceptions {
                        return Err(crate::error::ClsError::compile_at(
                            "'throw' no soportado en este runtime: el backend se compilÃ³ sin \
                             excepciones WASM (wasmi).",
                            &c.span,
                        ));
                    }
                    if let Some(arg0) = c.args.first() {
                        self.emit_expression(arg0)?;
                        self.emit_to_string(arg0)?;
                    } else {
                        let s = self.intern_string("error");
                        self.emit_load_str(s);
                    }
                    let packed = ((c.span.start_line as i64) << 32) | (c.span.start_col as i64);
                    self.body.push(Instruction::I64Const(packed));
                    self.body.push(Instruction::Throw(self.tag_idx));
                    return Ok(());
                }
                "print" => {
                    for arg in &c.args {
                        self.emit_print_arg(arg)?;
                    }
                    self.host.call(HostFn::PrintEnd, &mut self.body);
                    return Ok(());
                }
                "len" => {
                    let arg = &c.args[0];
                    // Magic __len: clase con __len â†’ call sin args (paridad walker).
                    if self.emit_class_method("__len", arg)? {
                        return Ok(());
                    }
                    self.emit_expression(arg)?;
                    // String â†’ decodifica el pack (ptr<<32|len); array/tuple/record
                    // â†’ lee el header. Despachar por el tipo del argumento.
                    let t = self.types.get(&expr_span(arg)).cloned().unwrap_or(Type::Any);
                    match t {
                        Type::String => {
                            self.host.call(HostFn::StrLength, &mut self.body);
                        }
                        Type::Record(_, _) | Type::Shape(_) => {
                            self.host.call(HostFn::RecordLen, &mut self.body);
                        }
                        _ => self.emit_array_len(),
                    }
                    return Ok(());
                }
                "toString" => {
                    let arg = &c.args[0];
                    self.emit_expression(arg)?;
                    self.emit_to_string(arg)?;
                    return Ok(());
                }
                "str" => {
                    let arg = &c.args[0];
                    self.emit_expression(arg)?;
                    self.emit_to_string(arg)?;
                    return Ok(());
                }
                "input" => {
                    self.host.call(HostFn::Input, &mut self.body);
                    return Ok(());
                }
                "int" => {
                    let arg = &c.args[0];
                    // Magic __int: clase con __int â†’ call sin args (paridad walker).
                    if self.emit_class_method("__int", arg)? {
                        return Ok(());
                    }
                    self.emit_expression(arg)?;
                    self.emit_to_int(arg)?;
                    return Ok(());
                }
                "float" => {
                    let arg = &c.args[0];
                    // Magic __float: clase con __float â†’ call sin args.
                    if self.emit_class_method("__float", arg)? {
                        return Ok(());
                    }
                    self.emit_expression(arg)?;
                    self.emit_to_float(arg)?;
                    return Ok(());
                }
                "bool" => {
                    let arg = &c.args[0];
                    // Magic __bool: clase con __bool â†’ call sin args.
                    if self.emit_class_method("__bool", arg)? {
                        return Ok(());
                    }
                    self.emit_expression(arg)?;
                    self.emit_to_bool(arg)?;
                    return Ok(());
                }
                "type" => {
                    let arg = &c.args[0];
                    // Si la clase define __type â†’ llamarla (paridad con el walker).
                    if self.emit_class_method("__type", arg)? {
                        return Ok(());
                    }
                    let span = expr_span(arg);
                    let t = self.types.get(&span).cloned().unwrap_or(Type::Any);
                    // type_name del walker: claseâ†’"Object", structâ†’"Struct", enumâ†’"Enum".
                    let name = match &t {
                        Type::Named(cn, _) if self.class_defs.contains_key(cn.as_str()) => "Object",
                        Type::Named(cn, _) if self.struct_defs.contains_key(cn.as_str()) => {
                            "Struct"
                        }
                        Type::Named(cn, _) if self.enum_defs.contains_key(cn.as_str()) => "Enum",
                        Type::Named(_, _) => "Object",
                        _ => type_name_str(&t),
                    };
                    let idx = self.intern_string(name);
                    self.emit_load_str(idx);
                    return Ok(());
                }
                "now" => {
                    self.host.call(HostFn::Now, &mut self.body);
                    return Ok(());
                }
                "exit" => {
                    self.emit_expression(&c.args[0])?;
                    self.host.call(HostFn::Exit, &mut self.body);
                    return Ok(());
                }
                "sleep" => {
                    self.emit_expression(&c.args[0])?;
                    self.host.call(HostFn::Sleep, &mut self.body);
                    return Ok(());
                }
                _ => {}
            }
        }
        // `x::f(...)` â€” mÃ³dulo/namespace importado: call directo a `x::f`.
        if let Expression::NamespaceAccess(ns, member, _) = &*c.callee {
            let key = format!("{}::{}", ns, member);
            if let Some(fidx) = self.func_indexes.get(&key).copied() {
                self.body.push(Instruction::I64Const(0)); // __capturas
                for arg in &c.args {
                    self.emit_expression(arg)?;
                }
                self.emit_call_site(&c.span);
                self.body.push(Instruction::Call(fidx));
                return Ok(());
            }
            return Err(crate::error::ClsError::compile_at(
                &format!(
                    "El miembro '{}' no existe o no se exporta en el mÃ³dulo '{}' (fase de emisiÃ³n).",
                    member, ns
                ),
                &expr_span(&c.callee),
            ));
        }
        if let Expression::Identifier(name, _) = &*c.callee {
            if let Some(fidx) = self.func_indexes.get(name).copied() {
                // Firma uniforme (B5): las funciones CLS top-level reciben
                // __capturas (0) como primer arg. Internas y main no.
                if !name.starts_with("__") && name != "main" {
                    self.body.push(Instruction::I64Const(0));
                }
                for arg in &c.args {
                    self.emit_expression(arg)?;
                }
                // Args faltantes â†’ valores por defecto (en el call site)
                if let Some(defaults) = self.func_defaults.get(name) {
                    let provided = c.args.len();
                    for d in defaults.iter().skip(provided) {
                        match d {
                            Some(expr) => self.emit_expression(expr)?,
                            None => self.body.push(Instruction::I64Const(0)),
                        }
                    }
                }
                self.emit_call_site(&c.span);
                self.body.push(Instruction::Call(fidx));
                return Ok(());
            }
            // FunciÃ³n host del nodo (intrinsic): canal `env.host_call(id, ptr, n)`.
            if let Some(intr) = self.intrinsics.get(name) {
                self.emit_host_call(intr, c)?;
                return Ok(());
            }
        }
        // FunciÃ³n como valor (variable con handle) â†’ call_indirect por tipo.
        let callee_ty = self.types.get(&expr_span(&c.callee)).cloned();
        if let Some(Type::Fun(params, ret)) = callee_ty {
            let mut pv: Vec<ValType> = Vec::new();
            for t in &params {
                pv.push(was_type(t)?.val_type());
            }
            let rv: Vec<ValType> = match &*ret {
                Type::Void => vec![],
                r => vec![was_type(r)?.val_type()],
            };
            // Firma uniforme (B5): closure = [capturas(i64), params...].
            // Toda funciÃ³n CLS (top-level y arrows) se compila con el capturas
            // como primer param. El dispatch usa tag-bit: impar = closure (lee
            // el ptr de capturas del handle en memoria); par = funciÃ³n simple
            // (capturas = 0 literal, sin handle).
            let mut pv_closure = vec![ValType::I64];
            pv_closure.extend(pv.iter().copied());
            let tidx_closure = self.register_func_type(pv_closure, rv.clone());
            // v = eval(callee); valor con tag (par = simple, impar = closure).
            self.emit_expression(&c.callee)?;
            let v = self.fresh_local();
            self.body.push(Instruction::LocalSet(v));
            // block $done (resultado del call) â†’ cada rama hace call_indirect + br.
            let ret_block = if rv.is_empty() {
                BlockType::Empty
            } else {
                BlockType::Result(rv[0])
            };
            // tag = v & 1 â†’ condiciÃ³n del if (impar = closure). Convertir a i32.
            self.body.push(Instruction::LocalGet(v));
            self.body.push(Instruction::I64Const(1));
            self.body.push(Instruction::I64And);
            self.body.push(Instruction::I32WrapI64);
            self.block_depth += 1;
            self.body.push(Instruction::If(ret_block));
            // Rama closure (impar): ptr = v>>1; capturas = handle[8] (aplanado).
            self.body.push(Instruction::LocalGet(v));
            self.body.push(Instruction::I64Const(1));
            self.body.push(Instruction::I64ShrU);
            self.body.push(Instruction::I64Const(8));
            self.body.push(Instruction::I64Add);
            self.body.push(Instruction::I32WrapI64);
            self.body.push(Instruction::I64Load(MemArg {
                offset: 0,
                align: 3,
                memory_index: 0,
            }));
            let caps_tmp = self.fresh_local();
            self.body.push(Instruction::LocalSet(caps_tmp));
            // push [capturas, args..., tabla_idx]
            self.body.push(Instruction::LocalGet(caps_tmp));
            for arg in &c.args {
                self.emit_expression(arg)?;
            }
            // Params faltantes â†’ Null (0), como el walker (default o Null).
            for _ in c.args.len()..params.len() {
                self.body.push(Instruction::I64Const(0));
            }
            self.body.push(Instruction::LocalGet(v));
            self.body.push(Instruction::I64Const(1));
            self.body.push(Instruction::I64ShrU);
            self.body.push(Instruction::I32WrapI64);
            self.body.push(Instruction::I64Load(MemArg {
                offset: 0,
                align: 3,
                memory_index: 0,
            }));
            self.body.push(Instruction::I32WrapI64);
            self.emit_call_site(&c.span);
            self.body.push(Instruction::CallIndirect {
                type_index: tidx_closure,
                table_index: 0,
            });
            self.body.push(Instruction::Else);
            // Rama simple (par): tabla_idx = v>>1; push [capturas=0, args..., tabla_idx].
            self.body.push(Instruction::I64Const(0));
            for arg in &c.args {
                self.emit_expression(arg)?;
            }
            for _ in c.args.len()..params.len() {
                self.body.push(Instruction::I64Const(0));
            }
            self.body.push(Instruction::LocalGet(v));
            self.body.push(Instruction::I64Const(1));
            self.body.push(Instruction::I64ShrU);
            self.body.push(Instruction::I32WrapI64);
            self.emit_call_site(&c.span);
            self.body.push(Instruction::CallIndirect {
                type_index: tidx_closure,
                table_index: 0,
            });
            self.body.push(Instruction::End);
            self.block_depth -= 1;
            return Ok(());
        }
        // Magic __call: el callee es un objeto de clase con __call â†’
        // obj(args...) = __call(obj, args...) (paridad walker interpreter.rs:1644).
        let callee_ty = self.types.get(&expr_span(&c.callee)).cloned();
        if let Some(cn) = self.class_magic_method(&callee_ty, "__call") {
            let _ = self.magic_ret_was(&cn, "__call")?;
            self.emit_expression(&c.callee)?;
            let obj_tmp = self.fresh_local();
            self.body.push(Instruction::LocalSet(obj_tmp));
            self.emit_class_method_call_on("__call", &cn, obj_tmp, &c.args)?;
            return Ok(());
        }
        // Objeto sin __call invocado como funciÃ³n â†’ error claro (paridad walker).
        if let Some(Type::Named(cn2, _)) = callee_ty {
            if self.class_defs.contains_key(cn2.as_str()) {
                return Err(crate::error::ClsError::compile_at(
                    &format!(
                        "El objeto de tipo '{}' no es callable (falta __call)",
                        cn2
                    ),
                    &c.span,
                ));
            }
        }
        Err(self.unsupported_expr(&Expression::Call(c.clone())))
    }

    /// Llama un mÃ©todo de clase por nombre (p.ej. `__type`/`__toJson`) sobre el
    /// objeto expresado. Devuelve `false` si la clase no define ese mÃ©todo.
    fn emit_class_method(&mut self, name: &str, object: &Expression) -> ClsResult<bool> {
        self.emit_class_method_args(name, object, &[])
    }

    /// Como [`Self::emit_class_method`] pero con argumentos: emite el objeto,
    /// lo guarda en un local, pushea `me`, emite los args y hace el
    /// call_indirect `(me, args...)` vÃ­a vtable. El orden de evaluaciÃ³n es
    /// objeto â†’ args (paridad walker). El stack del call_indirect es
    /// `[me, args..., fnptr]` (me al fondo).
    fn emit_class_method_args(
        &mut self,
        name: &str,
        object: &Expression,
        args: &[Expression],
    ) -> ClsResult<bool> {
        let obj_ty = self.types.get(&expr_span(object)).cloned();
        // M2: resolver la clase que DEFINE el mÃ©todo (sube por ancestors) â€”
        // un magic heredado vive como `Base::__add`, no `Hijo::__add`.
        if let Some(dn) = self.class_magic_method(&obj_ty, name) {
            self.emit_expression(object)?;
            let obj_tmp = self.fresh_local();
            self.body.push(Instruction::LocalSet(obj_tmp));
            return self.emit_class_method_call_on(name, dn.as_str(), obj_tmp, args);
        }
        Ok(false)
    }

    /// Emite el call_indirect de un mÃ©todo de clase sobre el objeto en el local
    /// `obj_ptr`: pushea `me` (al fondo del stack), emite los args y despacha
    /// por vtable. El call_indirect espera `[me, args..., fnptr]`.
    /// Sube por `ancestors` para resolver la clase que define el mÃ©todo (M2).
    fn emit_class_method_call_on(
        &mut self,
        name: &str,
        class_name: &str,
        obj_ptr: u32,
        args: &[Expression],
    ) -> ClsResult<bool> {
        let mut cur = Some(class_name.to_string());
        while let Some(c) = cur {
            if let Some(info) = self.class_defs.get(&c) {
                if let Some(slot) = info.methods.iter().position(|m| m == name) {
                    let method_key = format!("{}::{}", c, name);
                    if let Some(&ty) = self.method_type_indexes.get(&method_key) {
                        // receiver (me) â€” al fondo; los args van DESPUÃ‰S (el
                        // call_indirect los espera en orden: me, args...).
                        self.body.push(Instruction::LocalGet(obj_ptr));
                        for a in args {
                            self.emit_expression(a)?;
                        }
                        // vtable(obj[0]) + slot
                        self.body.push(Instruction::LocalGet(obj_ptr));
                        self.body.push(Instruction::I32WrapI64);
                        self.body.push(Instruction::I64Load(MemArg {
                            offset: 0,
                            align: 3,
                            memory_index: 0,
                        }));
                        self.body.push(Instruction::I64Const(slot as i64));
                        self.body.push(Instruction::I64Add);
                        self.body.push(Instruction::I32WrapI64);
                        self.body.push(Instruction::CallIndirect {
                            type_index: ty,
                            table_index: 0,
                        });
                        return Ok(true);
                    }
                }
                cur = info.ancestors.first().cloned();
            } else {
                break;
            }
        }
        Ok(false)
    }

    /// Â¿El tipo (estÃ¡tico) es una clase que define el magic `name`? Devuelve el
    /// nombre de la clase que LO DEFINE (sube por `ancestors` â€” M2: un magic
    /// heredado se registra como `Base::__add`, no `Hijo::__add`). `None` si no.
    fn class_magic_method(&self, ty: &Option<Type>, name: &str) -> Option<String> {
        if let Some(Type::Named(cn, _)) = ty {
            let mut cur = Some(cn.clone());
            while let Some(c) = cur {
                if let Some(info) = self.class_defs.get(&c) {
                    if info.methods.iter().any(|m| m == name) {
                        return Some(c);
                    }
                    cur = info.ancestors.first().cloned();
                } else {
                    break;
                }
            }
        }
        None
    }

    /// Tipo CLS del retorno anotado de un mÃ©todo de clase (o `None` si no tiene).
    /// Sube por `ancestors` para los mÃ©todos heredados (M2).
    fn magic_ret_type(&self, class_name: &str, name: &str) -> Option<Type> {
        let mut cur = Some(class_name.to_string());
        while let Some(c) = cur {
            if let Some(t) = self
                .func_types
                .get(&format!("{}::{}", c, name))
                .and_then(|(_, r)| r.clone())
            {
                return Some(t);
            }
            cur = self.class_defs.get(&c).and_then(|i| i.ancestors.first().cloned());
        }
        None
    }

    /// WasTy del retorno de un magic: el JIT necesita el tipo anotado (distinto
    /// de void) para el dispatch (el call_indirect devuelve segÃºn la firma).
    fn magic_ret_was(&self, class_name: &str, name: &str) -> ClsResult<WasTy> {
        match self.magic_ret_type(class_name, name) {
            Some(t) if t != Type::Void => was_type(&t),
            _ => Err(crate::error::ClsError::CompileError(format!(
                "'{}::{}' debe anotar su tipo de retorno (distinto de void) para \
                 el dispatch del magic en el JIT",
                class_name, name
            ))),
        }
    }

    /// Dispatch de un magic binario: `left.__op(right)`, luego `right.__op(left)`
    /// (paridad walker `binary_magic`). Devuelve `Ok(Some(WasTy))` del retorno
    /// del mÃ©todo si se emitiÃ³, `Ok(None)` si ningÃºn lado define el magic.
    fn try_binary_magic(
        &mut self,
        left: &Expression,
        right: &Expression,
        magic: &str,
    ) -> ClsResult<Option<WasTy>> {
        let lty = self.types.get(&expr_span(left)).cloned();
        let rty = self.types.get(&expr_span(right)).cloned();
        if let Some(cn) = self.class_magic_method(&lty, magic) {
            let ret = self.magic_ret_was(&cn, magic)?;
            self.emit_class_method_args(magic, left, &[right.clone()])?;
            return Ok(Some(ret));
        }
        if let Some(cn) = self.class_magic_method(&rty, magic) {
            let ret = self.magic_ret_was(&cn, magic)?;
            self.emit_class_method_args(magic, right, &[left.clone()])?;
            return Ok(Some(ret));
        }
        Ok(None)
    }

    /// Carga un campo del CmxValue (tag/props/children) â€” el ptr estÃ¡ en el stack.
    fn emit_cmx_field(&mut self, offset: i64) -> ClsResult<()> {
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

    /// Despacha el print de un campo de record heterogÃ©neo segÃºn su tag.
    fn emit_print_record_field(&mut self, ptr_tmp: u32, key_tmp: u32) {
        self.body.push(Instruction::LocalGet(ptr_tmp));
        self.body.push(Instruction::LocalGet(key_tmp));
        self.host.call(HostFn::RecordGet, &mut self.body);
        let val_tmp = self.fresh_local();
        self.body.push(Instruction::LocalSet(val_tmp));
        self.body.push(Instruction::LocalGet(ptr_tmp));
        self.body.push(Instruction::LocalGet(key_tmp));
        self.host.call(HostFn::RecordTag, &mut self.body);
        let tag_tmp = self.fresh_local();
        self.body.push(Instruction::LocalSet(tag_tmp));
        self.body.push(Instruction::LocalGet(val_tmp));
        self.body.push(Instruction::LocalGet(tag_tmp));
        self.host.call(HostFn::PrintAny, &mut self.body);
    }

    /// Formatea una tupla `(e0, e1, ...)` con repr (strings entre comillas), como
    /// el walker. El ptr de la tupla ya estÃ¡ en el stack.
    fn emit_tuple_to_string(&mut self, slots: &[Type], _arg: &Expression) -> ClsResult<()> {
        let ptr = self.fresh_local();
        self.body.push(Instruction::LocalSet(ptr));
        let open = self.intern_string("(");
        self.emit_load_str(open);
        let res = self.fresh_local();
        self.body.push(Instruction::LocalSet(res));
        for (i, slot) in slots.iter().enumerate() {
            if i > 0 {
                self.body.push(Instruction::LocalGet(res));
                let sep = self.intern_string(", ");
                self.emit_load_str(sep);
                self.host.call(HostFn::StrConcat, &mut self.body);
                self.body.push(Instruction::LocalSet(res));
            }
            self.body.push(Instruction::LocalGet(ptr));
            self.body.push(Instruction::I64Const(16 + (i as i64) * 8));
            self.body.push(Instruction::I64Add);
            self.body.push(Instruction::I32WrapI64);
            let w = was_type(slot).unwrap_or(WasTy::I64);
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
            let val_tmp = self.fresh_local_ty(w);
            self.body.push(match w {
                WasTy::F64 => Instruction::LocalSet(val_tmp),
                WasTy::I32 => Instruction::LocalSet(val_tmp),
                WasTy::I64 => Instruction::LocalSet(val_tmp),
            });
            let sv = self.fresh_local();
            match slot {
                Type::String => {
                    self.body.push(Instruction::LocalGet(val_tmp));
                    self.host.call(HostFn::StrRepr, &mut self.body);
                }
                Type::Float => {
                    self.body.push(Instruction::LocalGet(val_tmp));
                    self.host.call(HostFn::StrFloat, &mut self.body);
                }
                Type::Bool => {
                    self.body.push(Instruction::LocalGet(val_tmp));
                    self.host.call(HostFn::StrBool, &mut self.body);
                }
                Type::Char => {
                    self.body.push(Instruction::LocalGet(val_tmp));
                    self.host.call(HostFn::StrChar, &mut self.body);
                }
                _ => {
                    self.body.push(Instruction::LocalGet(val_tmp));
                    self.host.call(HostFn::StrInt, &mut self.body);
                }
            }
            self.body.push(Instruction::LocalSet(sv));
            self.body.push(Instruction::LocalGet(res));
            self.body.push(Instruction::LocalGet(sv));
            self.host.call(HostFn::StrConcat, &mut self.body);
            self.body.push(Instruction::LocalSet(res));
        }
        let close = self.intern_string(")");
        self.body.push(Instruction::LocalGet(res));
        self.emit_load_str(close);
        self.host.call(HostFn::StrConcat, &mut self.body);
        self.body.push(Instruction::LocalSet(res));
        self.body.push(Instruction::LocalGet(res));
        Ok(())
    }

    /// Variantes de un enum por nombre. Resuelve exacto (`Color`) o por sufijo
    /// (`lib::Color` cuando el typeck tipa la variante como `Named("Color")` pero
    /// el flatten registrÃ³ el enum prefijado).
    fn enum_variants(&self, name: &str) -> Option<&Vec<String>> {
        if let Some((_, v)) = self.enum_defs.get(name) {
            return Some(v);
        }
        let suffix = format!("::{}", name);
        self.enum_defs
            .iter()
            .find(|(k, _)| k.ends_with(&suffix))
            .map(|(_, (_, v))| v)
    }

    fn emit_print_arg(&mut self, arg: &Expression) -> ClsResult<()> {        // `u.values()` sobre un record con shape â†’ imprimir `[v1, v2, ...]` inline
        // (el typeck da Array<Any>, no imprimible por el backend genÃ©rico).
        if let Expression::Call(c) = arg {
            if let Expression::MemberAccess(m) = &*c.callee {
                if m.member == "values" {
                    let obj_ty = self.types.get(&expr_span(&m.object)).cloned();
                    if let Some(Type::Shape(fields)) = &obj_ty {
                        return self.emit_shape_values_to_string(m, fields);
                    }
                }
            }
        }
        // Index de array de Cmx (`app.children[i]`): despachar por el tag del child
        // (el elemento puede ser cmx, string, array, int, ...).
        if let Expression::Index(ix) = arg {
            let obj_ty = self.types.get(&expr_span(&ix.object)).cloned();
            if matches!(obj_ty, Some(Type::Array(e)) if matches!(*e, Type::Cmx)) {
                self.emit_expression(&ix.object)?;
                self.emit_expression(&ix.index)?;
                let ptr = self.fresh_local();
                let idx = self.fresh_local();
                self.body.push(Instruction::LocalSet(idx));
                self.body.push(Instruction::LocalSet(ptr));
                self.bounds_check(ptr, idx, &ix.span);
                // addr = 16 + idx*16 â†’ val y tag
                self.body.push(Instruction::LocalGet(ptr));
                self.body.push(Instruction::LocalGet(idx));
                self.body.push(Instruction::I64Const(16));
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
                let val_tmp = self.fresh_local();
                self.body.push(Instruction::LocalSet(val_tmp));
                self.body.push(Instruction::LocalGet(ptr));
                self.body.push(Instruction::LocalGet(idx));
                self.body.push(Instruction::I64Const(16));
                self.body.push(Instruction::I64Mul);
                self.body.push(Instruction::I64Const(24));
                self.body.push(Instruction::I64Add);
                self.body.push(Instruction::I64Add);
                self.body.push(Instruction::I32WrapI64);
                self.body.push(Instruction::I64Load(MemArg {
                    offset: 0,
                    align: 3,
                    memory_index: 0,
                }));
                let tag_tmp = self.fresh_local();
                self.body.push(Instruction::LocalSet(tag_tmp));
                self.body.push(Instruction::LocalGet(val_tmp));
                self.body.push(Instruction::LocalGet(tag_tmp));
                self.host.call(HostFn::PrintAny, &mut self.body);
                return Ok(());
            }
        }
        // Index sobre un record heterogÃ©neo (value Any): imprimir segÃºn el tag del valor.
        if let Expression::Index(i) = arg {
            let obj_ty = self.types.get(&expr_span(&i.object)).cloned();
            if matches!(obj_ty, Some(Type::Record(_, _))) {
                self.emit_expression(&i.object)?;
                self.emit_expression(&i.index)?;
                let key_tmp = self.fresh_local();
                let ptr_tmp = self.fresh_local();
                self.body.push(Instruction::LocalSet(key_tmp));
                self.body.push(Instruction::LocalSet(ptr_tmp));
                self.emit_print_record_field(ptr_tmp, key_tmp);
                return Ok(());
            }
        }
        // Member access `record.campo` con value heterogÃ©neo â†’ igual, por tag.
        if let Expression::MemberAccess(m) = arg {
            let obj_ty = self.types.get(&expr_span(&m.object)).cloned();
            if matches!(obj_ty, Some(Type::Record(_, _)))
                && !matches!(m.member.as_str(), "length" | "size")
            {
                self.emit_expression(&m.object)?;
                let ptr_tmp = self.fresh_local();
                self.body.push(Instruction::LocalSet(ptr_tmp));
                let k = self.intern_string(&m.member);
                self.emit_load_str(k);
                let key_tmp = self.fresh_local();
                self.body.push(Instruction::LocalSet(key_tmp));
                self.emit_print_record_field(ptr_tmp, key_tmp);
                return Ok(());
            }
            // `app.tag`: puede ser un string (tag minÃºscula) o un handle de funciÃ³n
            // (tag mayÃºscula). Despachar por tag-bit: handle (par O impar) =
            // bits altos cero; string CLS = (off<<32)|len (bits altos != 0).
            if matches!(obj_ty, Some(Type::Cmx)) && m.member == "tag" {
                self.emit_expression(&m.object)?;
                self.emit_cmx_field(0)?;
                let v = self.fresh_local();
                self.body.push(Instruction::LocalSet(v));
                // if (v>>32 == 0) && (v != 0) â†’ handle â†’ FnToString
                self.body.push(Instruction::LocalGet(v));
                self.body.push(Instruction::I64Const(32));
                self.body.push(Instruction::I64ShrU);
                self.body.push(Instruction::I64Eqz);
                self.body.push(Instruction::LocalGet(v));
                self.body.push(Instruction::I64Eqz);
                self.body.push(Instruction::I32Eqz);
                self.body.push(Instruction::I32And);
                self.block_depth += 1;
                self.body.push(Instruction::If(BlockType::Empty));
                self.body.push(Instruction::LocalGet(v));
                self.host.call(HostFn::FnToString, &mut self.body);
                self.host.call(HostFn::PrintStr, &mut self.body);
                self.body.push(Instruction::Else);
                self.body.push(Instruction::LocalGet(v));
                self.host.call(HostFn::PrintStr, &mut self.body);
                self.body.push(Instruction::End);
                self.block_depth -= 1;
                return Ok(());
            }
        }
        // Cadenas de acceso sobre `Any`/Record (json.parse anidado): `o.x[0]`,
        // `o.a.c`, `o.a.b[0]`. El objeto de la cadena tiene tipo `Any` o Record;
        // despachar por tag en runtime y formatear el valor (val, tag) real.
        if let Expression::Index(ix) = arg {
            let obj_ty = self.types.get(&expr_span(&ix.object)).cloned();
            if matches!(obj_ty, Some(Type::Any)) {
                self.emit_any_chain(arg)?;
                self.host.call(HostFn::PrintAny, &mut self.body);
                return Ok(());
            }
        }
        if let Expression::MemberAccess(m) = arg {
            let obj_ty = self.types.get(&expr_span(&m.object)).cloned();
            if matches!(obj_ty, Some(Type::Any)) {
                self.emit_any_chain(arg)?;
                self.host.call(HostFn::PrintAny, &mut self.body);
                return Ok(());
            }
        }
        self.emit_expression(arg)?;
        // json.stringify devuelve String (no un int): print_str.
        if let Expression::Call(c) = arg {
            if let Expression::MemberAccess(m) = &*c.callee {
                if let Expression::Identifier(o, _) = &*m.object {
                    if o == "json" && m.member == "stringify" {
                        self.host.call(HostFn::PrintStr, &mut self.body);
                        return Ok(());
                    }
                }
            }
        }
        // Llamadas a funciones nativas (extensiÃ³n) â†’ tipo de retorno codificado.
        if let Expression::Call(c) = arg {
            if let Expression::Identifier(name, _) = &*c.callee {
                if let Some(rc) = self.native_ret.get(name) {
                    match rc {
                        'f' => self.host.call(HostFn::PrintFloat, &mut self.body),
                        's' => self.host.call(HostFn::PrintStr, &mut self.body),
                        'b' | 'c' => self.host.call(HostFn::PrintBool, &mut self.body),
                        _ => self.host.call(HostFn::PrintInt, &mut self.body),
                    }
                    return Ok(());
                }
            }
        }
        // Llamadas a mÃ³dulos stdlib â†’ tipo de retorno conocido (print float/int).
        // math.range devuelve un array (el typeck no lo tipa): formatear `[..]`.
        if is_math_range_call(arg) {
            self.emit_expression(arg)?;
            self.body.push(Instruction::I64Const(8));
            self.body.push(Instruction::I64Const(0));
            self.host.call(HostFn::ArrToString, &mut self.body);
            self.host.call(HostFn::PrintStr, &mut self.body);
            return Ok(());
        }
        // Los contenedores (array/record/cmx/tuple) los formatea el match de tipos.
        if let Some(w) = self.module_call_ret(arg) {
            let t = self
                .types
                .get(&expr_span(arg))
                .cloned()
                .unwrap_or(Type::Any);
            let is_container = matches!(
                t,
                Type::Array(_) | Type::Record(_, _) | Type::Cmx | Type::Tuple(_)
            );
            if !is_container {
                // El tipo real del span decide (String â†’ PrintStr; Float â†’ PrintFloat;
                // Bool â†’ PrintBool); para tipos sin informaciÃ³n, usar el WasTy.
                match &t {
                    Type::String => {
                        self.host.call(HostFn::PrintStr, &mut self.body);
                        return Ok(());
                    }
                    Type::Bool => {
                        self.host.call(HostFn::PrintBool, &mut self.body);
                        return Ok(());
                    }
                    Type::Char => {
                        self.host.call(HostFn::PrintChar, &mut self.body);
                        return Ok(());
                    }
                    Type::Float => {
                        self.host.call(HostFn::PrintFloat, &mut self.body);
                        return Ok(());
                    }
                    _ => {}
                }
                match w {
                    WasTy::F64 => {
                        self.host.call(HostFn::PrintFloat, &mut self.body);
                        return Ok(());
                    }
                    WasTy::I32 => {
                        self.host.call(HostFn::PrintBool, &mut self.body);
                        return Ok(());
                    }
                    _ => {
                        self.host.call(HostFn::PrintInt, &mut self.body);
                        return Ok(());
                    }
                }
            }
        }
        let span = expr_span(arg);
        let t = self.types.get(&span).cloned().unwrap_or(Type::Any);
        match t {
            Type::String => self.host.call(HostFn::PrintStr, &mut self.body),
            Type::Bool => self.host.call(HostFn::PrintBool, &mut self.body),
            Type::Char => self.host.call(HostFn::PrintChar, &mut self.body),
            Type::Float => self.host.call(HostFn::PrintFloat, &mut self.body),
            Type::Null => {
                // `null` â†’ imprimir "null" (paridad walker).
                self.body.push(Instruction::Drop);
                let n = self.intern_string("null");
                self.emit_load_str(n);
                self.host.call(HostFn::PrintStr, &mut self.body);
            }
            Type::Array(elem) => {
                // Formatear `[e1, e2, ...]` como el walker (evita imprimir el ptr).
                let w = was_type(&*elem)?;
                let kind = arr_kind_code(&*elem);
                let es = if matches!(*elem, Type::Cmx) {
                    16
                } else {
                    elem_size_bytes(w)
                };
                self.body.push(Instruction::I64Const(es));
                self.body.push(Instruction::I64Const(kind));
                self.host.call(HostFn::ArrToString, &mut self.body);
                self.host.call(HostFn::PrintStr, &mut self.body);
            }
            Type::Record(_, _) => {
                // Formatear `{k: v, ...}` como el walker (evita imprimir el ptr).
                self.host.call(HostFn::RecordToString, &mut self.body);
                self.host.call(HostFn::PrintStr, &mut self.body);
            }
            Type::Shape(fields) => {
                // Formatear `{k: v, ...}` (keys ordenadas alfabÃ©ticamente, paridad walker).
                let layout = self.shape_layout(&fields)?;
                let ptr = self.fresh_local();
                self.body.push(Instruction::LocalSet(ptr));
                let open = self.intern_string("{");
                self.emit_load_str(open);
                let res = self.fresh_local();
                self.body.push(Instruction::LocalSet(res));
                let mut ordered: Vec<&(String, WasTy, i64)> = layout.iter().collect();
                ordered.sort_by(|a, b| a.0.cmp(&b.0));
                for (i, (fname, w, off)) in ordered.iter().enumerate() {
                    if i > 0 {
                        let sep = self.intern_string(", ");
                        self.emit_load_str(sep);
                        let st = self.fresh_local();
                        self.body.push(Instruction::LocalSet(st));
                        self.body.push(Instruction::LocalGet(res));
                        self.body.push(Instruction::LocalGet(st));
                        self.host.call(HostFn::StrConcat, &mut self.body);
                        self.body.push(Instruction::LocalSet(res));
                    }
                    let label = format!("{}: ", fname);
                    let ls = self.intern_string(&label);
                    self.emit_load_str(ls);
                    let lt = self.fresh_local();
                    self.body.push(Instruction::LocalSet(lt));
                    self.body.push(Instruction::LocalGet(res));
                    self.body.push(Instruction::LocalGet(lt));
                    self.host.call(HostFn::StrConcat, &mut self.body);
                    self.body.push(Instruction::LocalSet(res));
                    // valor del campo: load por offset + a string segÃºn el tipo del campo
                    self.body.push(Instruction::LocalGet(ptr));
                    self.body.push(Instruction::I64Const(*off));
                    self.body.push(Instruction::I64Add);
                    self.body.push(Instruction::I32WrapI64);
                    match *w {
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
                    let cls_t = fields
                        .iter()
                        .find(|(n, _)| *n == *fname)
                        .map(|(_, t)| t.clone())
                        .unwrap_or(Type::Any);
                    // Los strings de un shape se imprimen con comillas (paridad walker).
                    if matches!(cls_t, Type::String) {
                        let q = self.intern_string("\"");
                        self.emit_load_str(q);
                        let qt = self.fresh_local();
                        self.body.push(Instruction::LocalSet(qt));
                        self.body.push(Instruction::LocalGet(res));
                        self.body.push(Instruction::LocalGet(qt));
                        self.host.call(HostFn::StrConcat, &mut self.body);
                        self.body.push(Instruction::LocalSet(res));
                    }
                    self.emit_was_to_string(*w, &cls_t)?;
                    let vt = self.fresh_local();
                    self.body.push(Instruction::LocalSet(vt));
                    self.body.push(Instruction::LocalGet(res));
                    self.body.push(Instruction::LocalGet(vt));
                    self.host.call(HostFn::StrConcat, &mut self.body);
                    self.body.push(Instruction::LocalSet(res));
                    if matches!(cls_t, Type::String) {
                        let q = self.intern_string("\"");
                        self.emit_load_str(q);
                        let qt = self.fresh_local();
                        self.body.push(Instruction::LocalSet(qt));
                        self.body.push(Instruction::LocalGet(res));
                        self.body.push(Instruction::LocalGet(qt));
                        self.host.call(HostFn::StrConcat, &mut self.body);
                        self.body.push(Instruction::LocalSet(res));
                    }
                }
                let close = self.intern_string("}");
                self.emit_load_str(close);
                let ct = self.fresh_local();
                self.body.push(Instruction::LocalSet(ct));
                self.body.push(Instruction::LocalGet(res));
                self.body.push(Instruction::LocalGet(ct));
                self.host.call(HostFn::StrConcat, &mut self.body);
                self.host.call(HostFn::PrintStr, &mut self.body);
            }
            Type::Tuple(slots) => {
                self.emit_tuple_to_string(&slots, arg)?;
                self.host.call(HostFn::PrintStr, &mut self.body);
            }
            Type::Named(name, _) if self.class_defs.contains_key(&name) => {
                // Si la clase define __repr â†’ usarlo (el ptr ya estÃ¡ en el stack).
                if let Some(idx) = self.func_indexes.get(&format!("{}::__repr", name)) {
                    self.body.push(Instruction::Call(*idx));
                    self.host.call(HostFn::PrintStr, &mut self.body);
                } else {
                    // Formatear `<Clase {campo: valor, ...}>` como el walker.
                    let info = self.class_defs[&name].clone();
                    let ptr = self.fresh_local();
                    self.body.push(Instruction::LocalSet(ptr));
                    let open = format!("<{} {{", name);
                    let s = self.intern_string(&open);
                    self.emit_load_str(s);
                    let res = self.fresh_local();
                    self.body.push(Instruction::LocalSet(res));
                    for (i, (fname, t_cls, w, off, _vis)) in info.fields.iter().enumerate() {
                        let label = format!("{}: ", fname);
                        let ls = self.intern_string(&label);
                        self.emit_load_str(ls);
                        let lt = self.fresh_local();
                        self.body.push(Instruction::LocalSet(lt));
                        self.body.push(Instruction::LocalGet(res));
                        self.body.push(Instruction::LocalGet(lt));
                        self.host.call(HostFn::StrConcat, &mut self.body);
                        self.body.push(Instruction::LocalSet(res));
                        // valor
                        self.body.push(Instruction::LocalGet(ptr));
                        self.body.push(Instruction::I64Const(*off));
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
                        if matches!(t_cls, Type::String) {
                            // el valor ya es un string (ptr<<32|len): concatenar directo
                        } else {
                            match w {
                                WasTy::F64 => self.host.call(HostFn::StrFloat, &mut self.body),
                                _ => self.host.call(HostFn::StrInt, &mut self.body),
                            }
                        }
                        let sv = self.fresh_local();
                        self.body.push(Instruction::LocalSet(sv));
                        self.body.push(Instruction::LocalGet(res));
                        self.body.push(Instruction::LocalGet(sv));
                        self.host.call(HostFn::StrConcat, &mut self.body);
                        self.body.push(Instruction::LocalSet(res));
                        if i < info.fields.len() - 1 {
                            let sep = self.intern_string(", ");
                            self.emit_load_str(sep);
                            let st = self.fresh_local();
                            self.body.push(Instruction::LocalSet(st));
                            self.body.push(Instruction::LocalGet(res));
                            self.body.push(Instruction::LocalGet(st));
                            self.host.call(HostFn::StrConcat, &mut self.body);
                            self.body.push(Instruction::LocalSet(res));
                        }
                    }
                    let close = self.intern_string("}>");
                    self.emit_load_str(close);
                    let ct = self.fresh_local();
                    self.body.push(Instruction::LocalSet(ct));
                    self.body.push(Instruction::LocalGet(res));
                    self.body.push(Instruction::LocalGet(ct));
                    self.host.call(HostFn::StrConcat, &mut self.body);
                    self.body.push(Instruction::LocalSet(res));
                    self.body.push(Instruction::LocalGet(res));
                    self.host.call(HostFn::PrintStr, &mut self.body);
                }
            }
            Type::Cmx => {
                self.host.call(HostFn::CmxToString, &mut self.body);
                self.host.call(HostFn::PrintStr, &mut self.body);
            }
            Type::Fun(..) => {
                // Handle de funciÃ³n â†’ `<function X>` (el nombre estÃ¡ en el handle).
                self.host.call(HostFn::FnToString, &mut self.body);
                self.host.call(HostFn::PrintStr, &mut self.body);
            }
            Type::Named(name, _) if self.struct_defs.contains_key(&name) => {
                let ptr = self.fresh_local();
                self.body.push(Instruction::LocalSet(ptr));
                // Struct def como valor (ptr 0) â†’ `<function X>` (paridad walker).
                self.body.push(Instruction::LocalGet(ptr));
                self.body.push(Instruction::I64Eqz);
                self.block_depth += 1;
                self.body.push(Instruction::If(BlockType::Empty));
                let fs = self.intern_string(&format!("<function {}>", name));
                self.emit_load_str(fs);
                self.host.call(HostFn::PrintStr, &mut self.body);
                self.body.push(Instruction::Else);
                self.emit_struct_to_string(&name, ptr)?;
                self.host.call(HostFn::PrintStr, &mut self.body);
                self.body.push(Instruction::End);
                self.block_depth -= 1;
                return Ok(());
            }
            Type::Named(name, _) if self.enum_variants(&name).is_some() => {
                let variants = self.enum_variants(&name).unwrap().clone();
                // index = v & 0xffffffff â†’ seleccionar el string de la variante
                self.body.push(Instruction::I64Const(0xffff_ffff));
                self.body.push(Instruction::I64And);
                let idx = self.fresh_local();
                self.body.push(Instruction::LocalSet(idx));
                // Enum def como valor (index 0xffffffff) â†’ `<enum X>` (paridad walker).
                self.body.push(Instruction::LocalGet(idx));
                self.body.push(Instruction::I64Const(0xffff_ffff));
                self.body.push(Instruction::I64Eq);
                self.block_depth += 1;
                self.body.push(Instruction::If(BlockType::Empty));
                let eds = self.intern_string(&format!("<enum {}>", name));
                self.emit_load_str(eds);
                self.host.call(HostFn::PrintStr, &mut self.body);
                self.body.push(Instruction::Else);
                let n = variants.len();
                if n == 0 {
                    let s = self.intern_string("");
                    self.emit_load_str(s);
                    self.host.call(HostFn::PrintStr, &mut self.body);
                    self.body.push(Instruction::End);
                    self.block_depth -= 1;
                    return Ok(());
                }
                self.body.push(Instruction::LocalGet(idx));
                self.body.push(Instruction::I64Const(0));
                self.body.push(Instruction::I64Eq);
                self.block_depth += 1;
                self.body
                    .push(Instruction::If(BlockType::Result(ValType::I64)));
                let s0 = self.intern_string(&variants[0]);
                self.emit_load_str(s0);
                if n > 1 {
                    for (i, variant) in variants.iter().enumerate().skip(1) {
                        self.body.push(Instruction::Else);
                        if i == n - 1 {
                            let s = self.intern_string(variant);
                            self.emit_load_str(s);
                        } else {
                            self.body.push(Instruction::LocalGet(idx));
                            self.body.push(Instruction::I64Const(i as i64));
                            self.body.push(Instruction::I64Eq);
                            self.block_depth += 1;
                            self.body
                                .push(Instruction::If(BlockType::Result(ValType::I64)));
                            let s = self.intern_string(variant);
                            self.emit_load_str(s);
                        }
                    }
                    for _ in 0..(n - 1) {
                        self.body.push(Instruction::End);
                        self.block_depth -= 1;
                    }
                } else {
                    self.body.push(Instruction::End);
                    self.block_depth -= 1;
                }
                self.host.call(HostFn::PrintStr, &mut self.body);
                self.body.push(Instruction::End);
                self.block_depth -= 1;
                return Ok(());
            }
            Type::Union(_) => match union_base(&t) {
                Type::String => self.host.call(HostFn::PrintStr, &mut self.body),
                Type::Float => self.host.call(HostFn::PrintFloat, &mut self.body),
                Type::Bool => self.host.call(HostFn::PrintBool, &mut self.body),
                _ => self.host.call(HostFn::PrintInt, &mut self.body),
            },
            Type::Literal(l) => match l {
                LitVal::Str(_) => self.host.call(HostFn::PrintStr, &mut self.body),
                LitVal::Float(_) => self.host.call(HostFn::PrintFloat, &mut self.body),
                LitVal::Bool(_) => self.host.call(HostFn::PrintBool, &mut self.body),
                _ => self.host.call(HostFn::PrintInt, &mut self.body),
            },
            Type::Void | Type::Empty => {
                // `print("x", time.sleep(5))` â†’ imprime "void" (paridad walker).
                // La llamada void no deja valor en el stack: solo imprimir la etiqueta.
                let n = self.intern_string("void");
                self.emit_load_str(n);
                self.host.call(HostFn::PrintStr, &mut self.body);
            }
            _ => self.host.call(HostFn::PrintInt, &mut self.body),
        }
        Ok(())
    }

    /// Construye la representaciÃ³n `Punto { x: 3, y: 4 }` de un struct y la deja
    /// en el stack (el ptr del struct estÃ¡ en `ptr`).
    fn emit_struct_to_string(&mut self, name: &str, ptr: u32) -> ClsResult<()> {
        let info = self.struct_defs[name].clone();
        let open = format!("{} {{ ", name);
        let s = self.intern_string(&open);
        self.emit_load_str(s);
        let res = self.fresh_local();
        self.body.push(Instruction::LocalSet(res));
        for (i, (fname, t_cls, w)) in info.fields.iter().enumerate() {
            let label = format!("{}: ", fname);
            let ls = self.intern_string(&label);
            self.emit_load_str(ls);
            let lt = self.fresh_local();
            self.body.push(Instruction::LocalSet(lt));
            self.body.push(Instruction::LocalGet(res));
            self.body.push(Instruction::LocalGet(lt));
            self.host.call(HostFn::StrConcat, &mut self.body);
            self.body.push(Instruction::LocalSet(res));
            self.body.push(Instruction::LocalGet(ptr));
            self.body.push(Instruction::I64Const(info.offsets[i]));
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
            if matches!(t_cls, Type::String) {
                let q = self.intern_string("\"");
                self.emit_load_str(q);
                let qt = self.fresh_local();
                self.body.push(Instruction::LocalSet(qt));
                self.body.push(Instruction::LocalGet(res));
                self.body.push(Instruction::LocalGet(qt));
                self.host.call(HostFn::StrConcat, &mut self.body);
                self.body.push(Instruction::LocalSet(res));
                let sv = self.fresh_local();
                self.body.push(Instruction::LocalSet(sv));
                self.body.push(Instruction::LocalGet(res));
                self.body.push(Instruction::LocalGet(sv));
                self.host.call(HostFn::StrConcat, &mut self.body);
                self.body.push(Instruction::LocalSet(res));
                let q2 = self.intern_string("\"");
                self.emit_load_str(q2);
                let qt2 = self.fresh_local();
                self.body.push(Instruction::LocalSet(qt2));
                self.body.push(Instruction::LocalGet(res));
                self.body.push(Instruction::LocalGet(qt2));
                self.host.call(HostFn::StrConcat, &mut self.body);
                self.body.push(Instruction::LocalSet(res));
            } else {
                match w {
                    WasTy::F64 => self.host.call(HostFn::StrFloat, &mut self.body),
                    _ => self.host.call(HostFn::StrInt, &mut self.body),
                }
                let sv = self.fresh_local();
                self.body.push(Instruction::LocalSet(sv));
                self.body.push(Instruction::LocalGet(res));
                self.body.push(Instruction::LocalGet(sv));
                self.host.call(HostFn::StrConcat, &mut self.body);
                self.body.push(Instruction::LocalSet(res));
            }
            if i < info.fields.len() - 1 {
                let sep = self.intern_string(", ");
                self.emit_load_str(sep);
                let st = self.fresh_local();
                self.body.push(Instruction::LocalSet(st));
                self.body.push(Instruction::LocalGet(res));
                self.body.push(Instruction::LocalGet(st));
                self.host.call(HostFn::StrConcat, &mut self.body);
                self.body.push(Instruction::LocalSet(res));
            }
        }
        let close = self.intern_string(" }");
        self.emit_load_str(close);
        let ct = self.fresh_local();
        self.body.push(Instruction::LocalSet(ct));
        self.body.push(Instruction::LocalGet(res));
        self.body.push(Instruction::LocalGet(ct));
        self.host.call(HostFn::StrConcat, &mut self.body);
        self.body.push(Instruction::LocalSet(res));
        self.body.push(Instruction::LocalGet(res));
        Ok(())
    }

    fn emit_to_string(&mut self, arg: &Expression) -> ClsResult<()> {
        let span = expr_span(arg);
        let t = self.types.get(&span).cloned().unwrap_or(Type::Any);
        match t {
            Type::String => {}
            Type::Bool => self.host.call(HostFn::StrBool, &mut self.body),
            Type::Char => self.host.call(HostFn::StrChar, &mut self.body),
            Type::Float => self.host.call(HostFn::StrFloat, &mut self.body),
            Type::Null => {
                // null â†’ string "null"
                self.body.push(Instruction::Drop);
                let n = self.intern_string("null");
                self.emit_load_str(n);
            }
            Type::Named(name, _) if self.struct_defs.contains_key(&name) => {
                let ptr = self.fresh_local();
                self.body.push(Instruction::LocalSet(ptr));
                self.emit_struct_to_string(&name, ptr)?;
            }
            Type::Named(name, _) if self.class_defs.contains_key(&name) => {
                // toString(obj) â†’ __toString si existe; si no, __repr; el ptr estÃ¡ en stack.
                if let Some(idx) = self.func_indexes.get(&format!("{}::__toString", name)) {
                    self.body.push(Instruction::Call(*idx));
                } else if let Some(idx) = self.func_indexes.get(&format!("{}::__repr", name)) {
                    self.body.push(Instruction::Call(*idx));
                } else {
                    self.host.call(HostFn::StrInt, &mut self.body);
                }
            }
            Type::Array(elem) => {
                // `[e1, e2, ...]` como el walker (paridad en interpolaciÃ³n).
                let w = was_type(&*elem)?;
                let kind = arr_kind_code(&*elem);
                let es = if matches!(*elem, Type::Cmx) {
                    16
                } else {
                    elem_size_bytes(w)
                };
                self.body.push(Instruction::I64Const(es));
                self.body.push(Instruction::I64Const(kind));
                self.host.call(HostFn::ArrToString, &mut self.body);
            }
            Type::Fun(..) => {
                // Handle de funciÃ³n â†’ `<function X>` (el nombre estÃ¡ en el handle).
                self.host.call(HostFn::FnToString, &mut self.body);
            }
            _ => self.host.call(HostFn::StrInt, &mut self.body),
        }
        Ok(())
    }

    /// Convierte un valor WASM (ya en el stack) a string segÃºn su tipo CLS.
    /// No consume el ptr; lo usa directo para hosts de string.
    fn emit_was_to_string(&mut self, w: WasTy, cls_t: &Type) -> ClsResult<()> {
        match cls_t {
            Type::String => Ok(()),
            Type::Bool => {
                self.host.call(HostFn::StrBool, &mut self.body);
                Ok(())
            }
            Type::Char => {
                self.host.call(HostFn::StrChar, &mut self.body);
                Ok(())
            }
            Type::Float => {
                self.host.call(HostFn::StrFloat, &mut self.body);
                Ok(())
            }
            Type::Array(_) | Type::Tuple(_) | Type::Record(_, _) | Type::Cmx => {
                // Contenedor anidado: imprimir como string de su tipo.
                let _ = w;
                self.host.call(HostFn::StrInt, &mut self.body);
                Ok(())
            }
            Type::Shape(fields) => {
                // Shape anidado: recorrer y formatear recursivamente.
                let ptr = self.fresh_local();
                self.body.push(Instruction::LocalSet(ptr));
                self.emit_shape_field_to_string(ptr, &fields)?;
                Ok(())
            }
            _ => {
                self.host.call(HostFn::StrInt, &mut self.body);
                Ok(())
            }
        }
    }

    /// `u.values()` sobre un shape â†’ string `[v1, v2, ...]` (keys ordenadas alf.).
    fn emit_shape_values_to_string(
        &mut self,
        m: &MemberAccessExpr,
        fields: &[(String, Type)],
    ) -> ClsResult<()> {
        self.emit_expression(&m.object)?;
        let ptr = self.fresh_local();
        self.body.push(Instruction::LocalSet(ptr));
        let open = self.intern_string("[");
        self.emit_load_str(open);
        let res = self.fresh_local();
        self.body.push(Instruction::LocalSet(res));
        let layout = self.shape_layout(fields)?;
        let mut ordered: Vec<&(String, WasTy, i64)> = layout.iter().collect();
        ordered.sort_by(|a, b| a.0.cmp(&b.0));
        for (i, (fname, w, off)) in ordered.iter().enumerate() {
            if i > 0 {
                let sep = self.intern_string(", ");
                self.emit_load_str(sep);
                let st = self.fresh_local();
                self.body.push(Instruction::LocalSet(st));
                self.body.push(Instruction::LocalGet(res));
                self.body.push(Instruction::LocalGet(st));
                self.host.call(HostFn::StrConcat, &mut self.body);
                self.body.push(Instruction::LocalSet(res));
            }
            self.body.push(Instruction::LocalGet(ptr));
            self.body.push(Instruction::I64Const(*off));
            self.body.push(Instruction::I64Add);
            self.body.push(Instruction::I32WrapI64);
            match *w {
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
            let cls_t = fields
                .iter()
                .find(|(n, _)| *n == *fname)
                .map(|(_, t)| t.clone())
                .unwrap_or(Type::Any);
            if matches!(cls_t, Type::String) {
                let q = self.intern_string("\"");
                self.emit_load_str(q);
                let qt = self.fresh_local();
                self.body.push(Instruction::LocalSet(qt));
                self.body.push(Instruction::LocalGet(res));
                self.body.push(Instruction::LocalGet(qt));
                self.host.call(HostFn::StrConcat, &mut self.body);
                self.body.push(Instruction::LocalSet(res));
            }
            self.emit_was_to_string(*w, &cls_t)?;
            let vt = self.fresh_local();
            self.body.push(Instruction::LocalSet(vt));
            self.body.push(Instruction::LocalGet(res));
            self.body.push(Instruction::LocalGet(vt));
            self.host.call(HostFn::StrConcat, &mut self.body);
            self.body.push(Instruction::LocalSet(res));
            if matches!(cls_t, Type::String) {
                let q = self.intern_string("\"");
                self.emit_load_str(q);
                let qt = self.fresh_local();
                self.body.push(Instruction::LocalSet(qt));
                self.body.push(Instruction::LocalGet(res));
                self.body.push(Instruction::LocalGet(qt));
                self.host.call(HostFn::StrConcat, &mut self.body);
                self.body.push(Instruction::LocalSet(res));
            }
        }
        let close = self.intern_string("]");
        self.emit_load_str(close);
        let ct = self.fresh_local();
        self.body.push(Instruction::LocalSet(ct));
        self.body.push(Instruction::LocalGet(res));
        self.body.push(Instruction::LocalGet(ct));
        self.host.call(HostFn::StrConcat, &mut self.body);
        self.host.call(HostFn::PrintStr, &mut self.body);
        Ok(())
    }

    /// `json.stringify(shape)` â†’ string JSON `{"k": v, ...}` (deja el string en stack).
    fn emit_shape_to_json_string(
        &mut self,
        expr: &Expression,
        fields: &[(String, Type)],
    ) -> ClsResult<()> {
        self.emit_expression(expr)?;
        let ptr = self.fresh_local();
        self.body.push(Instruction::LocalSet(ptr));
        let open = self.intern_string("{");
        self.emit_load_str(open);
        let res = self.fresh_local();
        self.body.push(Instruction::LocalSet(res));
        let layout = self.shape_layout(fields)?;
        let mut ordered: Vec<&(String, WasTy, i64)> = layout.iter().collect();
        ordered.sort_by(|a, b| a.0.cmp(&b.0));
        for (i, (fname, w, off)) in ordered.iter().enumerate() {
            if i > 0 {
                let sep = self.intern_string(",");
                self.emit_load_str(sep);
                let st = self.fresh_local();
                self.body.push(Instruction::LocalSet(st));
                self.body.push(Instruction::LocalGet(res));
                self.body.push(Instruction::LocalGet(st));
                self.host.call(HostFn::StrConcat, &mut self.body);
                self.body.push(Instruction::LocalSet(res));
            }
            let key_json = format!("\"{}\":", fname);
            let ks = self.intern_string(&key_json);
            self.emit_load_str(ks);
            let kt = self.fresh_local();
            self.body.push(Instruction::LocalSet(kt));
            self.body.push(Instruction::LocalGet(res));
            self.body.push(Instruction::LocalGet(kt));
            self.host.call(HostFn::StrConcat, &mut self.body);
            self.body.push(Instruction::LocalSet(res));
            self.body.push(Instruction::LocalGet(ptr));
            self.body.push(Instruction::I64Const(*off));
            self.body.push(Instruction::I64Add);
            self.body.push(Instruction::I32WrapI64);
            match *w {
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
            let cls_t = fields
                .iter()
                .find(|(n, _)| *n == *fname)
                .map(|(_, t)| t.clone())
                .unwrap_or(Type::Any);
            // JSON: strings con comillas, ints/floats planos, bool true/false.
            if matches!(cls_t, Type::String) {
                let q = self.intern_string("\"");
                self.emit_load_str(q);
                let qt = self.fresh_local();
                self.body.push(Instruction::LocalSet(qt));
                self.body.push(Instruction::LocalGet(res));
                self.body.push(Instruction::LocalGet(qt));
                self.host.call(HostFn::StrConcat, &mut self.body);
                self.body.push(Instruction::LocalSet(res));
                self.emit_was_to_string(*w, &cls_t)?;
                let vt = self.fresh_local();
                self.body.push(Instruction::LocalSet(vt));
                self.body.push(Instruction::LocalGet(res));
                self.body.push(Instruction::LocalGet(vt));
                self.host.call(HostFn::StrConcat, &mut self.body);
                self.body.push(Instruction::LocalSet(res));
                let q2 = self.intern_string("\"");
                self.emit_load_str(q2);
                let q2t = self.fresh_local();
                self.body.push(Instruction::LocalSet(q2t));
                self.body.push(Instruction::LocalGet(res));
                self.body.push(Instruction::LocalGet(q2t));
                self.host.call(HostFn::StrConcat, &mut self.body);
                self.body.push(Instruction::LocalSet(res));
            } else {
                match cls_t {
                    Type::Float => self.host.call(HostFn::StrFloat, &mut self.body),
                    Type::Bool => self.host.call(HostFn::StrBool, &mut self.body),
                    _ => self.host.call(HostFn::StrInt, &mut self.body),
                }
                let vt = self.fresh_local();
                self.body.push(Instruction::LocalSet(vt));
                self.body.push(Instruction::LocalGet(res));
                self.body.push(Instruction::LocalGet(vt));
                self.host.call(HostFn::StrConcat, &mut self.body);
                self.body.push(Instruction::LocalSet(res));
            }
        }
        let close = self.intern_string("}");
        self.emit_load_str(close);
        let ct = self.fresh_local();
        self.body.push(Instruction::LocalSet(ct));
        self.body.push(Instruction::LocalGet(res));
        self.body.push(Instruction::LocalGet(ct));
        self.host.call(HostFn::StrConcat, &mut self.body);
        Ok(())
    }

    /// `[ptr]` en stack â†’ string del shape (recursivo para shapes anidados).
    fn emit_shape_field_to_string(&mut self, ptr: u32, fields: &[(String, Type)]) -> ClsResult<()> {
        let layout = self.shape_layout(fields)?;
        let open = self.intern_string("{");
        self.emit_load_str(open);
        let res = self.fresh_local();
        self.body.push(Instruction::LocalSet(res));
        let mut ordered: Vec<&(String, WasTy, i64)> = layout.iter().collect();
        ordered.sort_by(|a, b| a.0.cmp(&b.0));
        for (i, (fname, w, off)) in ordered.iter().enumerate() {
            if i > 0 {
                let sep = self.intern_string(", ");
                self.emit_load_str(sep);
                let st = self.fresh_local();
                self.body.push(Instruction::LocalSet(st));
                self.body.push(Instruction::LocalGet(res));
                self.body.push(Instruction::LocalGet(st));
                self.host.call(HostFn::StrConcat, &mut self.body);
                self.body.push(Instruction::LocalSet(res));
            }
            let label = format!("{}: ", fname);
            let ls = self.intern_string(&label);
            self.emit_load_str(ls);
            let lt = self.fresh_local();
            self.body.push(Instruction::LocalSet(lt));
            self.body.push(Instruction::LocalGet(res));
            self.body.push(Instruction::LocalGet(lt));
            self.host.call(HostFn::StrConcat, &mut self.body);
            self.body.push(Instruction::LocalSet(res));
            self.body.push(Instruction::LocalGet(ptr));
            self.body.push(Instruction::I64Const(*off));
            self.body.push(Instruction::I64Add);
            self.body.push(Instruction::I32WrapI64);
            match *w {
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
            let cls_t = fields
                .iter()
                .find(|(n, _)| *n == *fname)
                .map(|(_, t)| t.clone())
                .unwrap_or(Type::Any);
            // Los strings de un shape se imprimen con comillas (paridad walker).
            if matches!(cls_t, Type::String) {
                let q = self.intern_string("\"");
                self.emit_load_str(q);
                let qt = self.fresh_local();
                self.body.push(Instruction::LocalSet(qt));
                self.body.push(Instruction::LocalGet(res));
                self.body.push(Instruction::LocalGet(qt));
                self.host.call(HostFn::StrConcat, &mut self.body);
                self.body.push(Instruction::LocalSet(res));
            }
            self.emit_was_to_string(*w, &cls_t)?;
            let vt = self.fresh_local();
            self.body.push(Instruction::LocalSet(vt));
            self.body.push(Instruction::LocalGet(res));
            self.body.push(Instruction::LocalGet(vt));
            self.host.call(HostFn::StrConcat, &mut self.body);
            self.body.push(Instruction::LocalSet(res));
            if matches!(cls_t, Type::String) {
                let q = self.intern_string("\"");
                self.emit_load_str(q);
                let qt = self.fresh_local();
                self.body.push(Instruction::LocalSet(qt));
                self.body.push(Instruction::LocalGet(res));
                self.body.push(Instruction::LocalGet(qt));
                self.host.call(HostFn::StrConcat, &mut self.body);
                self.body.push(Instruction::LocalSet(res));
            }
        }
        let close = self.intern_string("}");
        self.emit_load_str(close);
        let ct = self.fresh_local();
        self.body.push(Instruction::LocalSet(ct));
        self.body.push(Instruction::LocalGet(res));
        self.body.push(Instruction::LocalGet(ct));
        self.host.call(HostFn::StrConcat, &mut self.body);
        Ok(())
    }

    fn emit_to_int(&mut self, arg: &Expression) -> ClsResult<()> {
        let span = expr_span(arg);
        let t = self.types.get(&span).cloned().unwrap_or(Type::Any);
        match t {
            Type::Int => {}
            Type::Float => self.body.push(Instruction::I64TruncSatF64S),
            Type::Bool => self.body.push(Instruction::I64ExtendI32U),
            Type::String => {
                self.emit_call_site(&span);
                self.host.call(HostFn::ParseInt, &mut self.body)
            }
            _ => {}
        }
        Ok(())
    }

    fn emit_to_float(&mut self, arg: &Expression) -> ClsResult<()> {
        let span = expr_span(arg);
        let t = self.types.get(&span).cloned().unwrap_or(Type::Any);
        match t {
            Type::Float => {}
            Type::Int => self.body.push(Instruction::F64ConvertI64S),
            Type::Bool => {
                self.body.push(Instruction::I64ExtendI32U);
                self.body.push(Instruction::F64ConvertI64S);
            }
            Type::String => {
                self.emit_call_site(&span);
                self.host.call(HostFn::ParseFloat, &mut self.body)
            }
            _ => {}
        }
        Ok(())
    }

    fn emit_to_bool(&mut self, arg: &Expression) -> ClsResult<()> {
        // Reutiliza coerce_to_bool: la misma semÃ¡ntica de truthiness del walker
        // (int/float â‰  0, string len â‰  0, array/record len â‰  0, cmx/objetos
        // true). Antes los compuestos (cmx/array/record/any) caÃ­an en `_` y
        // dejaban el ptr i64 en el stack â†’ `if (bool(x))` emitÃ­a WASM invÃ¡lido.
        self.coerce_to_bool(arg)
    }

    fn emit_tuple(&mut self, t: &TupleExpr) -> ClsResult<()> {
        // Layout igual al array: [cap:i64][len:i64][slots...] con slots de 8 bytes.
        let n = t.elements.len() as i64;
        self.body.push(Instruction::I64Const(n));
        self.body.push(Instruction::I64Const(8));
        self.body.push(Instruction::I64Mul);
        self.body.push(Instruction::I64Const(16));
        self.body.push(Instruction::I64Add);
        let alloc = self.func_indexes["__alloc"];
        self.body.push(Instruction::Call(alloc));
        let ptr = self.fresh_local();
        self.body.push(Instruction::LocalSet(ptr));
        self.body.push(Instruction::LocalGet(ptr));
        self.body.push(Instruction::I64Const(n));
        self.emit_i64_store(0);
        self.body.push(Instruction::LocalGet(ptr));
        self.body.push(Instruction::I64Const(n));
        self.emit_i64_store(8);
        for (i, el) in t.elements.iter().enumerate() {
            self.emit_expression(el)?;
            let elem_ty = self.value_type(el)?;
            let val_tmp = self.fresh_local_ty(elem_ty);
            let addr_tmp = self.fresh_local();
            self.body.push(match elem_ty {
                WasTy::F64 => Instruction::LocalSet(val_tmp),
                WasTy::I32 => Instruction::LocalSet(val_tmp),
                WasTy::I64 => Instruction::LocalSet(val_tmp),
            });
            self.body.push(Instruction::LocalGet(ptr));
            self.body.push(Instruction::I64Const(16 + (i as i64) * 8));
            self.body.push(Instruction::I64Add);
            self.body.push(Instruction::LocalSet(addr_tmp));
            self.body.push(Instruction::LocalGet(addr_tmp));
            self.body.push(Instruction::I32WrapI64);
            self.body.push(match elem_ty {
                WasTy::F64 => Instruction::LocalGet(val_tmp),
                WasTy::I32 => Instruction::LocalGet(val_tmp),
                WasTy::I64 => Instruction::LocalGet(val_tmp),
            });
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
        self.body.push(Instruction::LocalGet(ptr));
        Ok(())
    }

    /// Valida visibilidad de un campo de clase (private/protected) para lectura
    /// o escritura desde el contexto actual. `private` y `protected` requieren
    /// estar dentro de la clase (o subclase para protected).
    fn check_field_access(
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

    /// Enforca la visibilidad de un mÃ©todo: private â†’ solo desde la clase;
    /// protected â†’ desde la clase o subclases. Paridad con el walker.
    fn check_method_access(
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
                    &format!("El mÃ©todo '{}' es private (solo accesible desde la clase)", method),
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
                        "El mÃ©todo '{}' es protected (solo accesible desde la clase o sus subclases)",
                        method
                    ),
                    span,
                ));
            }
        }
        Ok(())
    }

    /// Tag runtime estÃ¡tico de un tipo (paridad con `fmt_val_to_string` del host):
    /// 0=int,1=string,2=float,3=bool,4=char,5=cmx,6=array,7=record.
    fn any_static_tag(&self, t: &Type) -> i64 {
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

    /// EvalÃºa una cadena de acceso `o.a.c`, `o.x[0]`, `o.a.b[0]` sobre valores
    /// `Any`/Record de json.parse, despachando por tag en runtime. Deja `(val, tag)`
    /// en el stack. La base (raÃ­z de la cadena) se emite con su tag estÃ¡tico.
    fn emit_any_chain(&mut self, expr: &Expression) -> ClsResult<()> {
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
    fn emit_member_access(&mut self, m: &MemberAccessExpr) -> ClsResult<()> {        if let Expression::Identifier(obj_name, _) = &*m.object {
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
            // Constantes de mÃ³dulos stdlib: math.PI / math.E
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
        // a un enum del mÃ³dulo importado (flattened como `lib::Color`).
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
        // `Clase.campo` (campo estÃ¡tico): el objeto es el nombre de la clase.
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
                    // acceso por nombre de campo: r.campo â†’ record_get(ptr, "campo")
                    let k = self.intern_string(&m.member);
                    self.emit_load_str(k);
                    self.host.call(HostFn::RecordGet, &mut self.body);
                    Ok(())
                }
            },
            Type::Shape(fields) => match m.member.as_str() {
                "length" | "size" => {
                    // Compile-time: el shape tiene un nÂº de campos fijo.
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
                // Resultado (val, tag) en el stack â†’ dejar solo el val (el tag se
                // pierde en un valor Any; los prints usan emit_print_arg con PrintAny).
                self.body.push(Instruction::Drop);
                Ok(())
            }
            _ => Err(self.unsupported_expr(&Expression::MemberAccess(m.clone()))),
        }
    }

    /// `"Hola $nombre ${expr}"` â†’ concatenaciÃ³n de las partes (toString de cada expr).
    fn emit_interpolation(&mut self, s: &StringInterpolation) -> ClsResult<()> {
        let empty = self.intern_string("");
        self.emit_load_str(empty);
        let acc = self.fresh_local();
        self.body.push(Instruction::LocalSet(acc));
        for part in &s.parts {
            match part {
                InterpolationPart::Text(t) => {
                    let idx = self.intern_string(t);
                    self.emit_load_str(idx);
                }
                InterpolationPart::Expr(e) => {
                    self.emit_expression(e)?;
                    self.emit_to_string(e)?;
                }
            }
            let tmp = self.fresh_local();
            self.body.push(Instruction::LocalSet(tmp));
            self.body.push(Instruction::LocalGet(acc));
            self.body.push(Instruction::LocalGet(tmp));
            self.host.call(HostFn::StrConcat, &mut self.body);
            self.body.push(Instruction::LocalSet(acc));
        }
        self.body.push(Instruction::LocalGet(acc));
        Ok(())
    }

    fn emit_array(&mut self, a: &ArrayExpr) -> ClsResult<()> {
        let elem_ty = self.array_elem_type(a)?;
        // Array de Cmx â†’ entradas `[val, tag]` stride 16 (children del Cmx, etc.).
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
                // Si el array es f64 y el elemento es un literal/expresiÃ³n int,
                // promoverlo a f64 para el store (layout homogÃ©neo).
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

    fn array_elem_type(&self, a: &ArrayExpr) -> ClsResult<WasTy> {
        if let Some(first) = a.elements.first() {
            // PromociÃ³n: si CUALQUIER elemento es float, el array es de f64
            // (p.ej. `[1, 2.0]` â†’ f64). El store promueve los ints a f64.
            let has_float = a
                .elements
                .iter()
                .any(|el| matches!(self.value_type(el), Ok(WasTy::F64)));
            if has_float {
                return Ok(WasTy::F64);
            }
            return self.value_type(first);
        }
        // Array vacÃ­o: usar el tipo anotado registrado por el typeck (span del literal),
        // p.ej. `const out: int[] = []`.
        if let Some(Type::Array(elem)) = self.types.get(&a.span) {
            if let Ok(w) = was_type(elem) {
                return Ok(w);
            }
        }
        Err(crate::error::ClsError::compile_at(
            "Array literal vacÃ­o sin tipo: agrega la anotaciÃ³n del elemento (p.ej. `int[] = []`)",
            &a.span,
        ))
    }

    /// Literal de record `{ a: 1, b: "x" }` â†’ record_new + record_set.
    fn emit_record(&mut self, r: &RecordExpr) -> ClsResult<()> {
        // Si el type map dice Shape â†’ emitir como struct contiguo (offsets fijos).
        // Es el caso de `var x = {a: 1, b: "1"}` (inferido) o anotado con
        // interface/alias de shape. Sin hashmap, sin keys en memoria, sin tags.
        if let Some(shape) = self.types.get(&r.span).cloned() {
            if let Type::Shape(fields) = &shape {
                return self.emit_shape_record(r, fields);
            }
        }
        let n = r.entries.len() as i64;
        self.body.push(Instruction::I64Const(n));
        self.host.call(HostFn::RecordNew, &mut self.body);
        let ptr = self.fresh_local();
        self.body.push(Instruction::LocalSet(ptr));
        for (key, val) in &r.entries {
            self.body.push(Instruction::LocalGet(ptr));
            let k = self.intern_string(key);
            self.emit_load_str(k);
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
            // Tag del valor en el record: tag del RUNTIME interno (Record â†’ 7,
            // Array â†’ 6, String â†’ 1...). Antes usaba arr_kind_code, que devolvÃ­a
            // 0 para records â†’ el binding los leÃ­a como int (ptr crudo).
            self.body.push(Instruction::I64Const(runtime_tag_code(&cls_t)));
            self.host.call(HostFn::RecordSet, &mut self.body);
            self.body.push(Instruction::Drop);
        }
        self.body.push(Instruction::LocalGet(ptr));
        Ok(())
    }

    /// Emite un record con shape como struct contiguo: `[campo0][campo1]...`.
    /// Los offsets se calculan del shape (cada campo con su WasTy).
    fn emit_shape_record(&mut self, r: &RecordExpr, fields: &[(String, Type)]) -> ClsResult<()> {
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

    /// Calcula `(nombre, WasTy, offset)` para cada campo de un shape (contiguo).
    fn shape_layout(&self, fields: &[(String, Type)]) -> ClsResult<Vec<(String, WasTy, i64)>> {
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
    fn emit_cmx(&mut self, c: &CmxElement) -> ClsResult<()> {
        // tag mayÃºscula â†’ resolver la variable/valor SIEMPRE (debe existir; si no, error).
        // tag minÃºscula â†’ String.
        if c.tag.starts_with(|ch: char| ch.is_uppercase()) {
            let name = c.tag.clone();
            if self.globals.contains_key(&name) || self.locals.contains_key(&name) {
                self.emit_ident_load(&name);
            } else if self.fn_table_idx.contains_key(&name) {
                // FunciÃ³n como tag â†’ handle de funciÃ³n (tag-bit) para que
                // `app.tag` sea invocable y se imprima `<function X>` (paridad walker).
                let ti = self.fn_table_idx[&name];
                let n = self.intern_string(&format!("<function {}>", name));
                self.body.push(Instruction::I64Const(ti as i64));
                self.emit_load_str(n);
                self.body.push(Instruction::I64Const(0));
                self.host.call(HostFn::FnHandle, &mut self.body);
            } else {
                return Err(crate::error::ClsError::CompileError(format!(
                    "El tag '<{}>' usa mayÃºscula pero '{}' no estÃ¡ definido: \
                     los tags con inicial mayÃºscula deben ser una funciÃ³n/valor existente",
                    c.tag, name
                )));
            }
        } else {
            let t = self.intern_string(&c.tag);
            self.emit_load_str(t);
        }
        self.body.push(Instruction::I64Const(0)); // kind=0 â†’ elemento
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
                    // Literales â†’ su tipo real (el type map puede dar Any).
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
                    // Texto â†’ CmxValue de texto (kind=1): el print lo muestra plano.
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

    fn emit_index_get(&mut self, i: &IndexExpr) -> ClsResult<()> {
        // Record: r["key"] â†’ record_get(ptr, key)
        let obj_ty = self.types.get(&expr_span(&i.object)).cloned();
        // `o.x[0]` con `o.x` Any (json.parse anidado): indexar despachando por tag.
        if matches!(obj_ty, Some(Type::Any)) {
            let expr = Expression::Index(i.clone());
            self.emit_any_chain(&expr)?;
            // Resultado (val, tag) â†’ dejar solo el val.
            self.body.push(Instruction::Drop);
            return Ok(());
        }
        // Magic __get: clase con __get â†’ obj.__get(index) (paridad walker:
        // "Indexado no soportado en objeto (falta __get)" si no lo define).
        if let Some(cn) = self.class_magic_method(&obj_ty, "__get") {
            let _ = self.magic_ret_was(&cn, "__get")?;
            self.emit_class_method_args("__get", &i.object, &[(*i.index).clone()])?;
            return Ok(());
        }
        if matches!(obj_ty, Some(Type::Record(_, _))) {
            self.emit_expression(&i.object)?;
            self.emit_expression(&i.index)?;
            self.host.call(HostFn::RecordGet, &mut self.body);
            let elem_ty = self.index_elem_type(i)?;
            self.bits_to_elem(elem_ty)?;
            return Ok(());
        }
        // Shape: r["campo"] con clave literal â†’ load por offset (como member access).
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
                "Ãndice dinÃ¡mico no soportado en un record con shape (usa Record<K,V> o any)",
                &i.span,
            ));
        }
        let elem_ty = self.index_elem_type(i)?;
        self.emit_expression(&i.object)?;
        self.emit_expression(&i.index)?;
        // Array de Cmx â†’ entradas `[val, tag]` stride 16 (children del Cmx, etc.).
        let is_cmx = matches!(&obj_ty, Some(Type::Array(e)) if matches!(**e, Type::Cmx));
        let elem_size = if is_cmx {
            16
        } else {
            self.container_elem_size(i, elem_ty)
        };
        self.emit_index_access(elem_ty, elem_size, i)
    }

    /// Asume [ptr, idx] en stack; deja el valor del elemento (con bounds check).
    fn emit_index_access(
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
    fn bounds_check(&mut self, ptr: u32, idx: u32, span: &Span) {
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
        self.emit_throw("Ãndice fuera de rango", span);
        self.body.push(Instruction::Unreachable);
        self.body.push(Instruction::End);
        self.block_depth -= 1;
    }

    fn index_elem_type(&self, i: &IndexExpr) -> ClsResult<WasTy> {
        let span = expr_span(&i.object);
        let t = self.types.get(&span).ok_or_else(|| {
            crate::error::ClsError::CompileError("Index object sin tipo".to_string())
        })?;
        match t {
            Type::Array(elem) => was_type(elem),
            Type::Record(_, v) => was_type(v),
            Type::Tuple(slots) => {
                // Ã­ndice literal â†’ slot exacto; dinÃ¡mico â†’ primer slot (o i64)
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

    /// TamaÃ±o de slot de un contenedor: tuplas usan slots de 8 bytes; arrays el
    /// tamaÃ±o del tipo del elemento.
    fn container_elem_size(&self, i: &IndexExpr, elem_ty: WasTy) -> i64 {
        let span = expr_span(&i.object);
        match self.types.get(&span) {
            Some(Type::Tuple(_)) => 8,
            _ => elem_size_bytes(elem_ty),
        }
    }

    /// Asume [arr_ptr, idx, value] en stack. Escribe el valor.
    fn emit_index_set(&mut self, i: &IndexExpr, elem_size: i64) -> ClsResult<()> {
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

    fn emit_array_len(&mut self) {
        // ptr estÃ¡ en stack â†’ len = i64.load(ptr+8)
        self.body.push(Instruction::I32WrapI64);
        self.body.push(Instruction::I64Load(MemArg {
            offset: 8,
            align: 3,
            memory_index: 0,
        }));
    }

    /// Tipo WASM del elemento de un array (del type map del object).
    fn array_elem_was_type(&self, obj: &Expression) -> ClsResult<WasTy> {
        let span = expr_span(obj);
        match self.types.get(&span) {
            Some(Type::Array(elem)) => was_type(elem),
            _ => Err(crate::error::ClsError::CompileError(
                "El objeto de la llamada no es un array".to_string(),
            )),
        }
    }

    /// Tipo CLS del elemento de un array.
    fn array_elem_cls_type(&self, obj: &Expression) -> ClsResult<Type> {
        let span = expr_span(obj);
        match self.types.get(&span) {
            Some(Type::Array(elem)) => Ok((**elem).clone()),
            _ => Err(crate::error::ClsError::CompileError(
                "El objeto de la llamada no es un array".to_string(),
            )),
        }
    }

    /// Convierte el valor en stack (del elem type) a i64 bits (para los hosts).
    fn elem_to_bits(&mut self, _arg: &Expression, elem_ty: WasTy) -> ClsResult<()> {
        match elem_ty {
            WasTy::F64 => self.body.push(Instruction::I64ReinterpretF64),
            WasTy::I32 => self.body.push(Instruction::I64ExtendI32U),
            WasTy::I64 => {}
        }
        Ok(())
    }

    /// Convierte i64 bits (del host) al valor del elem type.
    fn bits_to_elem(&mut self, elem_ty: WasTy) -> ClsResult<()> {
        match elem_ty {
            WasTy::F64 => self.body.push(Instruction::F64ReinterpretI64),
            WasTy::I32 => {}
            WasTy::I64 => {}
        }
        Ok(())
    }

    /// Escribe de vuelta el ptr mutado (resultado de push/unshift/reverse) a la
    /// variable y deja el valor como resultado (para `drop` del statement).
    fn writeback_array(&mut self, obj: &Expression) -> ClsResult<()> {
        if let Expression::Identifier(name, _) = obj {
            self.emit_ident_store(name);
            self.emit_ident_load(name);
            return Ok(());
        }
        // `me.items.push(...)` / `obj.items.push(...)`: el array pudo
        // reallocarse â†’ re-escribir el ptr en el campo (y dejar el ptr como
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

    fn emit_i64_store(&mut self, offset: u32) {
        // stack: [addr(i64), value] â†’ reordenar con wrap
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






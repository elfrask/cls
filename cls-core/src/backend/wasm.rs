//! Backend WASM: compila AST tipado → módulo WebAssembly.
//!
//! Estrategia: el emisor camina el AST directamente (WASM es stack-based, por lo
//! que las expresiones se emiten en post-order y dejan su valor en el stack).
//! El type map (Span → Type) del TypeChecker determina las representaciones:
//!
//! | Type CLS  | WASM             | Notas                                  |
//! |-----------|------------------|----------------------------------------|
//! | Int       | i64              |                                        |
//! | Float     | f64              |                                        |
//! | Bool      | i32 (0/1)        |                                        |
//! | Char      | i32 (u32 codep)  |                                        |
//! | String    | i64 (ptr<<32|len)| ptr = offset en memoria lineal         |
//! | Array<T>  | i64 (ptr)        | header [len:i64][elem...]              |
//!
//! El allocator es bump (sin free) con la memoria embebida en el módulo; el host
//! solo inyecta funciones `env.*` (print, conversiones, trap) y `alloc` para los
//! args de `main`.

#![cfg(feature = "wasm-backend")]

use crate::error::ClsResult;
use crate::error::Span;
use crate::frontend::ast::*;
use crate::frontend::token::Operator;
use crate::middleware::typeck::expr_span;
use crate::middleware::types::{LitVal, Type};
use std::collections::HashMap;
use wasm_encoder::{
    BlockType, CodeSection, DataSection, DataSegment, DataSegmentMode, ElementMode, ElementSection,
    Elements, EntityType, Export, ExportSection, Function, FunctionSection, GlobalSection,
    GlobalType, ImportSection, Instruction, Limits, MemArg, MemorySection, MemoryType,
    Module as WasmModule, TableSection, TableType, TypeSection, ValType,
};

/// Funciones host (`env.*`) que el nodo JIT debe implementar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HostFn {
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
    ArrToString,
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
}

impl HostFn {
    pub fn import_name(&self) -> &'static str {
        use HostFn::*;
        match self {
            PrintInt => "print_int",
            PrintFloat => "print_float",
            PrintBool => "print_bool",
            PrintChar => "print_char",
            PrintStr => "print_str",
            PrintEnd => "print_end",
            Now => "now",
            Exit => "exit",
            Sleep => "sleep",
            Trap => "trap",
            ParseInt => "parse_int",
            ParseFloat => "parse_float",
            ParseBool => "parse_bool",
            StrConcat => "str_concat",
            StrInt => "str_int",
            StrFloat => "str_float",
            StrBool => "str_bool",
            StrChar => "str_char",
            PowNum => "pow_num",
            Fmod => "fmod",
            Input => "input",
            StrUpper => "str_upper",
            StrLower => "str_lower",
            StrTrim => "str_trim",
            StrContains => "str_contains",
            StrStartsWith => "str_starts_with",
            StrEndsWith => "str_ends_with",
            StrIsEmpty => "str_is_empty",
            StrLength => "str_length",
            IntAbs => "int_abs",
            FloatAbs => "float_abs",
            ArrPush => "arr_push",
            ArrPop => "arr_pop",
            ArrShift => "arr_shift",
            ArrUnshift => "arr_unshift",
            ArrIndexOf => "arr_index_of",
            ArrIncludes => "arr_includes",
            ArrJoin => "arr_join",
            ArrReverse => "arr_reverse",
            ArrToString => "arr_to_string",
            MathSqrt => "math_sqrt",
            MathPow => "math_pow",
            MathMin => "math_min",
            MathMax => "math_max",
            MathFloor => "math_floor",
            MathCeil => "math_ceil",
            MathRound => "math_round",
            MathRandom => "math_random",
            MathSin => "math_sin",
            MathCos => "math_cos",
            MathTan => "math_tan",
            MathLog => "math_log",
            MathRange => "math_range",
            JsonStringify => "json_stringify",
            JsonParse => "json_parse",
            FsExists => "fs_exists",
            FsCwd => "fs_cwd",
            FsReadFile => "fs_read_file",
            FsWriteFile => "fs_write_file",
            FsListDir => "fs_list_dir",
            FsMkdir => "fs_mkdir",
            FsRm => "fs_rm",
            RecordNew => "record_new",
            RecordSet => "record_set",
            RecordGet => "record_get",
            RecordHas => "record_has",
            RecordTag => "record_tag",
            RecordLen => "record_len",
            RecordKeys => "record_keys",
            RecordValues => "record_values",
            RecordToString => "record_to_string",
            HttpGet => "http_get",
            HttpPost => "http_post",
        }
    }

    fn signature(&self) -> (Vec<ValType>, Vec<ValType>) {
        use HostFn::*;
        let i64p = vec![ValType::I64];
        match self {
            PrintInt | PrintStr => (i64p.clone(), vec![]),
            PrintFloat => (vec![ValType::F64], vec![]),
            PrintBool | PrintChar => (vec![ValType::I32], vec![]),
            PrintEnd => (vec![], vec![]),
            Now => (vec![], vec![ValType::I64]),
            Exit | Sleep => (i64p.clone(), vec![]),
            Trap => (i64p.clone(), vec![]),
            ParseInt | StrInt => (i64p.clone(), vec![ValType::I64]),
            ParseBool => (i64p.clone(), vec![ValType::I32]),
            ParseFloat => (i64p.clone(), vec![ValType::F64]),
            StrFloat => (vec![ValType::F64], vec![ValType::I64]),
            StrBool | StrChar => (vec![ValType::I32], vec![ValType::I64]),
            StrConcat => (vec![ValType::I64, ValType::I64], vec![ValType::I64]),
            PowNum => (vec![ValType::I64, ValType::I64], vec![ValType::I64]),
            Fmod => (vec![ValType::F64, ValType::F64], vec![ValType::F64]),
            Input => (vec![], vec![ValType::I64]),
            StrUpper | StrLower | StrTrim | StrLength => (i64p.clone(), vec![ValType::I64]),
            StrContains | StrStartsWith | StrEndsWith => (vec![ValType::I64, ValType::I64], vec![ValType::I32]),
            StrIsEmpty => (i64p.clone(), vec![ValType::I32]),
            IntAbs => (i64p.clone(), vec![ValType::I64]),
            FloatAbs => (vec![ValType::F64], vec![ValType::F64]),
            ArrPush | ArrUnshift => (vec![ValType::I64, ValType::I64, ValType::I64], vec![ValType::I64]),
            ArrPop | ArrShift | ArrReverse => (vec![ValType::I64, ValType::I64], vec![ValType::I64]),
            ArrIndexOf => (vec![ValType::I64, ValType::I64, ValType::I64], vec![ValType::I64]),
            ArrIncludes => (vec![ValType::I64, ValType::I64, ValType::I64], vec![ValType::I32]),
            ArrJoin => (vec![ValType::I64, ValType::I64, ValType::I64, ValType::I64], vec![ValType::I64]),
            ArrToString => (vec![ValType::I64, ValType::I64, ValType::I64], vec![ValType::I64]),
            MathSqrt | MathFloor | MathCeil | MathRound | MathSin | MathCos | MathTan | MathLog => {
                (vec![ValType::F64], vec![ValType::F64])
            }
            MathPow | MathMin | MathMax => (vec![ValType::F64, ValType::F64], vec![ValType::F64]),
            MathRandom => (vec![], vec![ValType::F64]),
            MathRange => (vec![ValType::I64, ValType::I64], vec![ValType::I64]),
            JsonStringify => (vec![ValType::I64, ValType::I64], vec![ValType::I64]),
            JsonParse => (i64p.clone(), vec![ValType::I64]),
            FsExists => (i64p.clone(), vec![ValType::I32]),
            FsCwd => (vec![], vec![ValType::I64]),
            FsReadFile => (i64p.clone(), vec![ValType::I64]),
            FsWriteFile => (vec![ValType::I64, ValType::I64], vec![ValType::I64]),
            FsListDir | FsMkdir | FsRm => (i64p.clone(), vec![ValType::I64]),
            RecordNew => (i64p.clone(), vec![ValType::I64]),
            RecordSet => (vec![ValType::I64, ValType::I64, ValType::I64, ValType::I64], vec![ValType::I64]),
            RecordGet => (vec![ValType::I64, ValType::I64], vec![ValType::I64]),
            RecordHas => (vec![ValType::I64, ValType::I64], vec![ValType::I32]),
            RecordTag => (vec![ValType::I64, ValType::I64], vec![ValType::I64]),
            RecordLen => (i64p.clone(), vec![ValType::I64]),
            RecordKeys | RecordValues => (i64p.clone(), vec![ValType::I64]),
            RecordToString => (i64p.clone(), vec![ValType::I64]),
            HttpGet => (i64p.clone(), vec![ValType::I64]),
            HttpPost => (vec![ValType::I64, ValType::I64], vec![ValType::I64]),
        }
    }
}

/// Tipo WASM de un valor (los que dejan un único valor en el stack).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WasTy {
    I64,
    F64,
    I32,
}

impl WasTy {
    fn val_type(self) -> ValType {
        match self {
            WasTy::I64 => ValType::I64,
            WasTy::F64 => ValType::F64,
            WasTy::I32 => ValType::I32,
        }
    }
}

/// Convierte un Type CLS a su representación WASM.
fn was_type(t: &Type) -> ClsResult<WasTy> {
    match t {
        Type::Int | Type::I8 | Type::I16 | Type::I32 | Type::I64 => Ok(WasTy::I64),
        Type::Float | Type::F32 | Type::F64 => Ok(WasTy::F64),
        Type::Bool => Ok(WasTy::I32),
        Type::Char => Ok(WasTy::I32),
        Type::String => Ok(WasTy::I64),
        Type::Array(_) => Ok(WasTy::I64),
        Type::Tuple(_) => Ok(WasTy::I64),
        Type::Record(_, _) => Ok(WasTy::I64),
        Type::Literal(LitVal::Float(_)) => Ok(WasTy::F64),
        Type::Literal(LitVal::Bool(_)) => Ok(WasTy::I32),
        Type::Named(..) | Type::Literal(_) => Ok(WasTy::I64),
        Type::Union(members) => {
            let mut it = members.iter();
            let first = it.next().and_then(|m| was_type(m).ok());
            if let Some(f) = first {
                if it.all(|m| was_type(m).ok() == Some(f)) {
                    return Ok(f);
                }
            }
            Ok(WasTy::I64)
        }
        Type::Void | Type::Empty => Ok(WasTy::I64),
        other => Err(crate::error::ClsError::CompileError(format!(
            "Tipo '{}' no soportado por el backend WASM (subconjunto JIT)",
            other
        ))),
    }
}

/// Contexto de un loop para resolver `break`/`continue`.
struct LoopGuard {
    break_at: u32,
    continue_at: u32,
}

struct HostCaller {
    indexes: HashMap<HostFn, u32>,
}

impl HostCaller {
    fn call(&self, h: HostFn, body: &mut Vec<Instruction<'static>>) {
        let idx = self.indexes[&h];
        body.push(Instruction::Call(idx));
    }
}

/// Emisor con el estado de compilación de una función.
struct FuncEmitter<'a> {
    types: &'a HashMap<Span, Type>,
    body: Vec<Instruction<'static>>,
    locals: HashMap<String, u32>,
    local_tys: HashMap<u32, WasTy>,
    next_local: u32,
    block_depth: u32,
    loop_stack: Vec<LoopGuard>,
    host: HostCaller,
    string_pool: &'a mut Vec<String>,
    string_index: &'a mut HashMap<String, u32>,
    func_indexes: &'a HashMap<String, u32>,
    func_defaults: &'a HashMap<String, Vec<Option<Expression>>>,
    enum_defs: &'a HashMap<String, (u32, Vec<String>)>,
    struct_defs: &'a HashMap<String, StructInfo>,
    native_indexes: &'a HashMap<String, u32>,
    native_ret: &'a HashMap<String, char>,
    globals: &'a HashMap<String, u32>,
    class_defs: &'a HashMap<String, ClassInfo>,
    method_type_indexes: &'a HashMap<String, u32>,
    /// clase actual (al compilar un método) — para `super` y `me`.
    current_class: Option<String>,
    target: &'a Target,
}

impl<'a> FuncEmitter<'a> {
    fn new(
        types: &'a HashMap<Span, Type>,
        host: HostCaller,
        string_pool: &'a mut Vec<String>,
        string_index: &'a mut HashMap<String, u32>,
        func_indexes: &'a HashMap<String, u32>,
        func_defaults: &'a HashMap<String, Vec<Option<Expression>>>,
        enum_defs: &'a HashMap<String, (u32, Vec<String>)>,
        struct_defs: &'a HashMap<String, StructInfo>,
        native_indexes: &'a HashMap<String, u32>,
        native_ret: &'a HashMap<String, char>,
        globals: &'a HashMap<String, u32>,
        class_defs: &'a HashMap<String, ClassInfo>,
        method_type_indexes: &'a HashMap<String, u32>,
        current_class: Option<String>,
        target: &'a Target,
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
            enum_defs,
            struct_defs,
            native_indexes,
            native_ret,
            globals,
            class_defs,
            method_type_indexes,
            current_class,
            target,
        }
    }

    fn fresh_local_ty(&mut self, ty: WasTy) -> u32 {
        let l = self.next_local;
        self.next_local += 1;
        self.local_tys.insert(l, ty);
        l
    }

    fn fresh_local(&mut self) -> u32 {
        self.fresh_local_ty(WasTy::I64)
    }

    fn local_for(&mut self, name: &str) -> u32 {
        *self.locals.entry(name.to_string()).or_insert_with(|| {
            let l = self.next_local;
            self.next_local += 1;
            l
        })
    }

    /// Carga un identificador: global (si es un `export var` top-level) o local.
    fn emit_ident_load(&mut self, name: &str) {
        if name == "super" && self.current_class.is_some() {
            self.body.push(Instruction::LocalGet(0));
        } else if let Some(g) = self.globals.get(name) {
            self.body.push(Instruction::GlobalGet(*g));
        } else {
            let idx = self.local_for(name);
            self.body.push(Instruction::LocalGet(idx));
        }
    }

    /// Escribe un identificador: global (si es un `export var` top-level) o local.
    fn emit_ident_store(&mut self, name: &str) {
        if let Some(g) = self.globals.get(name) {
            self.body.push(Instruction::GlobalSet(*g));
        } else {
            let idx = self.local_for(name);
            self.body.push(Instruction::LocalSet(idx));
        }
    }

    fn declare_var_ty(&mut self, name: &str, ty: WasTy) -> u32 {
        let idx = self.local_for(name);
        self.local_tys.entry(idx).or_insert(ty);
        idx
    }

    fn value_type(&self, expr: &Expression) -> ClsResult<WasTy> {
        // Llamadas a funciones nativas (extensión) → tipo de retorno codificado.
        if let Expression::Call(c) = expr {
            if let Expression::Identifier(name, _) = &*c.callee {
                if let Some(rc) = self.native_ret.get(name) {
                    return Ok(code_to_was(*rc));
                }
            }
        }
        // Llamadas a módulos stdlib → tipo de retorno conocido.
        if let Some(w) = self.module_call_ret(expr) {
            return Ok(w);
        }
        let span = expr_span(expr);
        let t = self.types.get(&span).ok_or_else(|| {
            crate::error::ClsError::CompileError(format!(
                "Expresión sin tipo ({}:{}:{}): el JIT requiere el type checker",
                span.start_line, span.start_col,
                expr_display(expr)
            ))
        })?;
        match t {
            Type::Any | Type::Unknown => Err(crate::error::ClsError::CompileError(format!(
                "Expresión sin tipo concreto ({}:{}): {}",
                span.start_line,
                span.start_col,
                expr_display(expr)
            ))),
            _ => was_type(t),
        }
    }

    fn emit_drop(&mut self, expr: &Expression) -> ClsResult<()> {
        let span = expr_span(expr);
        if let Some(t) = self.types.get(&span) {
            if *t == Type::Void {
                return Ok(());
            }
        }
        self.body.push(Instruction::Drop);
        Ok(())
    }

    // ── Emisión de statements ────────────────────────────────────────────

    fn emit_statement(&mut self, stmt: &Statement) -> ClsResult<()> {
        match stmt {
            Statement::VarDecl(v) | Statement::ConstDecl(v) => {
                let ty = match (&v.type_ann, &v.value) {
                    (Some(ann), Some(val)) => match was_type(&annotation_to_type(ann)) {
                        Ok(w) => w,
                        // Anotación no resuelta (alias/unioón) → tipo del valor.
                        Err(_) => self.value_type(val)?,
                    },
                    (Some(ann), None) => was_type(&annotation_to_type(ann))?,
                    (None, Some(val)) => self.value_type(val)?,
                    (None, None) => WasTy::I64,
                };
                let idx = self.declare_var_ty(&v.name, ty);
                if let Some(value) = &v.value {
                    self.emit_expression(value)?;
                    self.body.push(Instruction::LocalSet(idx));
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
                self.body.push(Instruction::Return);
                Ok(())
            }
            Statement::Break => {
                let ctx = self.loop_stack.last().ok_or_else(|| {
                    crate::error::ClsError::CompileError("break fuera de loop".to_string())
                })?;
                let depth = self.block_depth.saturating_sub(ctx.break_at);
                self.body.push(Instruction::Br(depth));
                Ok(())
            }
            Statement::Continue => {
                let ctx = self.loop_stack.last().ok_or_else(|| {
                    crate::error::ClsError::CompileError("continue fuera de loop".to_string())
                })?;
                let depth = self.block_depth.saturating_sub(ctx.continue_at);
                self.body.push(Instruction::Br(depth));
                Ok(())
            }
            Statement::If(i) => self.emit_if(i),
            Statement::While(w) => self.emit_while(w),
            Statement::Loop(b) => self.emit_loop(b),
            Statement::For(f) => self.emit_for(f),
            Statement::ForEach(fe) => self.emit_foreach(fe),
            Statement::Switch(s) => self.emit_switch(s),
            Statement::With(w) => self.emit_with(w),
            // `when` → compile-time: emitir solo la rama que matchea el target actual.
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
            other => Err(self.unsupported_stmt(other)),
        }
    }

    fn unsupported_stmt(&self, stmt: &Statement) -> crate::error::ClsError {
        crate::error::ClsError::CompileError(format!(
            "Statement no soportado por el JIT (subconjunto): {}",
            statement_display(stmt)
        ))
    }

    /// `for each x [and i] in (col)` sobre array/tuple.
    fn emit_foreach(&mut self, fe: &ForEachStatement) -> ClsResult<()> {
        // Enum: `for each v in (Nivel)` → loop 0..variants.len()
        if let Expression::Identifier(name, _) = &fe.iterable {
            if let Some((def_id, variants)) = self.enum_defs.get(name).cloned() {
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
                let continue_at = self.block_depth;
                self.loop_stack.push(LoopGuard { break_at, continue_at });
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
                self.body.push(Instruction::LocalGet(i));
                self.body.push(Instruction::I64Const(1));
                self.body.push(Instruction::I64Add);
                self.body.push(Instruction::LocalSet(i));
                let depth = self.block_depth.saturating_sub(continue_at);
                self.body.push(Instruction::Br(depth));
                self.body.push(Instruction::End);
                self.block_depth -= 1;
                self.body.push(Instruction::End);
                self.block_depth -= 1;
                self.loop_stack.pop();
                return Ok(());
            }
        }
        let iterable_ty = self.types.get(&expr_span(&fe.iterable)).cloned().unwrap_or(Type::Any);
        let (elem_ty, elem_size) = match &iterable_ty {
            Type::Array(elem) => {
                let w = was_type(elem)?;
                (w, elem_size_bytes(w))
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
        let continue_at = self.block_depth;
        self.loop_stack.push(LoopGuard { break_at, continue_at });
        // cond: i >= len(iter)
        self.body.push(Instruction::LocalGet(i));
        self.body.push(Instruction::LocalGet(iter));
        self.body.push(Instruction::I32WrapI64);
        self.body.push(Instruction::I64Load(MemArg { offset: 8, align: 3, memory_index: 0 }));
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
            WasTy::F64 => self.body.push(Instruction::F64Load(MemArg { offset: 0, align: 3, memory_index: 0 })),
            WasTy::I32 => self.body.push(Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 })),
            WasTy::I64 => self.body.push(Instruction::I64Load(MemArg { offset: 0, align: 3, memory_index: 0 })),
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
        // i++
        self.body.push(Instruction::LocalGet(i));
        self.body.push(Instruction::I64Const(1));
        self.body.push(Instruction::I64Add);
        self.body.push(Instruction::LocalSet(i));
        let depth = self.block_depth.saturating_sub(continue_at);
        self.body.push(Instruction::Br(depth));
        self.body.push(Instruction::End); // loop
        self.block_depth -= 1;
        self.body.push(Instruction::End); // block
        self.block_depth -= 1;
        self.loop_stack.pop();
        let _ = d;
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

    /// `with x in (expr) { ... }` → local temporal + bloque.
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

    fn emit_if(&mut self, i: &IfStatement) -> ClsResult<()> {        self.emit_expression(&i.condition)?;
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
        // Cadena de elifs anidados dentro del else; el último cede al else final.
        for (k, branch) in i.elif_branches.iter().enumerate() {
            self.emit_expression(&branch.condition)?;
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
        if let Some(cond) = &f.condition {
            self.emit_expression(cond)?;
            self.body.push(Instruction::I32Eqz);
            let depth = self.block_depth.saturating_sub(break_at);
            self.body.push(Instruction::BrIf(depth));
        }
        for s in &f.block.statements {
            self.emit_statement(s)?;
        }
        if let Some(update) = &f.update {
            self.emit_expression(update)?;
            self.emit_drop(update)?;
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

    // ── Emisión de expresiones ───────────────────────────────────────────

    fn emit_expression(&mut self, expr: &Expression) -> ClsResult<()> {
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
            Expression::MemberAccess(m) => self.emit_member_access(m),
            Expression::Conditional(c) => self.emit_conditional(c),
            Expression::Assignment(a) => self.emit_assignment(a),
            Expression::Parenthesized(inner, _) => self.emit_expression(inner),
            Expression::StringInterpolation(s) => self.emit_interpolation(s),
            other => Err(self.unsupported_expr(other)),
        }
    }

    fn unsupported_expr(&self, expr: &Expression) -> crate::error::ClsError {
        crate::error::ClsError::CompileError(format!(
            "Expresión no soportada por el JIT (subconjunto): {}",
            expr_display(expr)
        ))
    }

    fn emit_literal(&mut self, l: &Literal) -> ClsResult<()> {
        match &l.kind {
            LiteralKind::Int(v) => self.body.push(Instruction::I64Const(*v)),
            LiteralKind::Float(v) => self.body.push(Instruction::F64Const(*v)),
            LiteralKind::Bool(v) => {
                self.body.push(Instruction::I32Const(if *v { 1 } else { 0 }))
            }
            LiteralKind::Char(c) => self.body.push(Instruction::I32Const(*c as u32 as i32)),
            LiteralKind::String(s) => {
                let idx = self.intern_string(s);
                self.emit_load_str(idx);
            }
            LiteralKind::Null | LiteralKind::Unknown => {
                return Err(self.unsupported_expr(&Expression::Literal(l.clone())))
            }
        }
        Ok(())
    }

    fn intern_string(&mut self, s: &str) -> u32 {
        if let Some(idx) = self.string_index.get(s) {
            return *idx;
        }
        let idx = self.string_pool.len() as u32;
        self.string_pool.push(s.to_string());
        self.string_index.insert(s.to_string(), idx);
        idx
    }

    fn emit_load_str(&mut self, idx: u32) {
        self.body.push(Instruction::I64Const(idx as i64));
        let fidx = self.func_indexes["__load_str"];
        self.body.push(Instruction::Call(fidx));
    }

    fn emit_binary(&mut self, b: &BinaryExpr) -> ClsResult<()> {
        use Operator::*;
        let lt = self.value_type(&b.left)?;
        let rt = self.value_type(&b.right)?;
        match b.op {
            Plus if lt == WasTy::I64 && rt == WasTy::I64 => {
                let is_str = |e: &Expression| {
                    self.types.get(&expr_span(e)).map(|t| *t == Type::String).unwrap_or(false)
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
                self.div_zero_trap()?;
                self.body.push(Instruction::I64DivS);
            }
            Percent if lt == WasTy::F64 || rt == WasTy::F64 => {
                self.emit_expression(&b.left)?;
                self.emit_expression(&b.right)?;
                self.host.call(HostFn::Fmod, &mut self.body);
            }            Percent => {
                self.emit_expression(&b.left)?;
                self.emit_expression(&b.right)?;
                self.div_zero_trap()?;
                self.body.push(Instruction::I64RemS);
            }
            StarStar => {
                self.emit_expression(&b.left)?;
                self.emit_expression(&b.right)?;
                self.host.call(HostFn::PowNum, &mut self.body);
            }
            StrictEqual => {
                self.emit_expression(&b.left)?;
                self.emit_expression(&b.right)?;
                self.push_eq(lt)?;
            }
            NotEqual => {
                self.emit_expression(&b.left)?;
                self.emit_expression(&b.right)?;
                self.push_eq(lt)?;
                self.body.push(Instruction::I32Eqz);
            }
            LessThan => {
                self.emit_expression(&b.left)?;
                self.emit_expression(&b.right)?;
                self.push_cmp(lt, true, false)?;
            }
            LessEqual => {
                self.emit_expression(&b.left)?;
                self.emit_expression(&b.right)?;
                self.push_cmp(lt, true, true)?;
            }
            GreaterThan => {
                self.emit_expression(&b.left)?;
                self.emit_expression(&b.right)?;
                self.push_cmp(lt, false, false)?;
            }
            GreaterEqual => {
                self.emit_expression(&b.left)?;
                self.emit_expression(&b.right)?;
                self.push_cmp(lt, false, true)?;
            }
            And => {
                self.emit_expression(&b.left)?;
                self.body.push(Instruction::I32Eqz);
                self.block_depth += 1;
                self.body.push(Instruction::If(BlockType::Result(ValType::I32)));
                self.body.push(Instruction::I32Const(0));
                self.body.push(Instruction::Else);
                self.emit_expression(&b.right)?;
                self.body.push(Instruction::End);
                self.block_depth -= 1;
            }
            Or => {
                self.emit_expression(&b.left)?;
                self.block_depth += 1;
                self.body.push(Instruction::If(BlockType::Result(ValType::I32)));
                self.body.push(Instruction::I32Const(1));
                self.body.push(Instruction::Else);
                self.emit_expression(&b.right)?;
                self.body.push(Instruction::End);
                self.block_depth -= 1;
            }
            In => {
                // `x in "texto"` → substring (arrays en A4). StrContains(container, needle)
                self.emit_expression(&b.right)?;
                self.emit_expression(&b.left)?;
                self.host.call(HostFn::StrContains, &mut self.body);
            }
            Is => {
                // `v is Nivel` (enum), `p is Punto` (struct) o `o is Clase` (herencia)
                self.emit_expression(&b.left)?;
                if let Expression::Identifier(right_name, _) = &*b.right {
                    if let Some(info) = self.class_defs.get(right_name.as_str()) {
                        // cid = obj[8]; true si el objeto ES la clase o una SUBCLASE.
                        let obj_tmp = self.fresh_local();
                        let cid_tmp = self.fresh_local();
                        self.body.push(Instruction::LocalSet(obj_tmp));
                        self.body.push(Instruction::LocalGet(obj_tmp));
                        self.body.push(Instruction::I32WrapI64);
                        self.body.push(Instruction::I64Load(MemArg { offset: 8, align: 3, memory_index: 0 }));
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
                    self.body.push(Instruction::I64Load(MemArg { offset: 0, align: 3, memory_index: 0 }));
                }
                self.body.push(Instruction::I64Const(def_id as i64));
                self.body.push(Instruction::I64Eq);
            }
            PlusEqual | MinusEqual | StarEqual | SlashEqual => {
                self.emit_expression(&b.left)?;
                self.emit_expression(&b.right)?;
                match b.op {
                    PlusEqual => self.body.push(Instruction::I64Add),
                    MinusEqual => self.body.push(Instruction::I64Sub),
                    StarEqual => self.body.push(Instruction::I64Mul),
                    _ => self.body.push(Instruction::I64DivS),
                }
            }
            op => return Err(crate::error::ClsError::CompileError(format!(
                "Operador {} no soportado por el JIT",
                op
            ))),
        }
        Ok(())
    }

    fn f64_promote(&mut self, expr: &Expression) -> ClsResult<()> {
        if let WasTy::I64 = self.value_type(expr)? {
            self.body.push(Instruction::F64ConvertI64S);
        }
        Ok(())
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

    fn div_zero_trap(&mut self) -> ClsResult<()> {
        let tmp = self.fresh_local();
        self.body.push(Instruction::LocalSet(tmp));
        self.body.push(Instruction::LocalGet(tmp));
        self.body.push(Instruction::I64Eqz);
        self.block_depth += 1;
        self.body.push(Instruction::If(BlockType::Empty));
        let msg = self.intern_string("División por cero");
        self.emit_load_str(msg);
        self.host.call(HostFn::Trap, &mut self.body);
        self.body.push(Instruction::Unreachable);
        self.body.push(Instruction::End);
        self.block_depth -= 1;
        self.body.push(Instruction::LocalGet(tmp));
        Ok(())
    }

    fn emit_unary(&mut self, u: &UnaryExpr) -> ClsResult<()> {
        match u.op {
            UnaryOp::Negate => {
                let w = self.value_type(&u.operand)?;
                self.emit_expression(&u.operand)?;
                match w {
                    WasTy::F64 => self.body.push(Instruction::F64Neg),
                    WasTy::I64 => {
                        self.body.push(Instruction::I64Const(0));
                        self.body.push(Instruction::I64Sub);
                    }
                    WasTy::I32 => {
                        self.body.push(Instruction::I32Const(0));
                        self.body.push(Instruction::I32Sub);
                    }
                }
            }
            UnaryOp::Not => {
                self.emit_expression(&u.operand)?;
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
            UnaryOp::BitwiseNot => return Err(self.unsupported_expr(&Expression::Unary(u.clone()))),
        }
        Ok(())
    }

    /// `x++` / `++x` / `x--` / `--x` sobre un identificador.
    fn emit_incdec(&mut self, operand: &Expression, op: UnaryOp) -> ClsResult<()> {
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
                    self.emit_ident_load(name);
                    self.emit_expression(&a.value)?;
                    match op {
                        Operator::PlusEqual => self.body.push(Instruction::I64Add),
                        Operator::MinusEqual => self.body.push(Instruction::I64Sub),
                        Operator::StarEqual => self.body.push(Instruction::I64Mul),
                        _ => self.body.push(Instruction::I64DivS),
                    }
                } else {
                    self.emit_expression(&a.value)?;
                }
                self.emit_ident_store(name);
                self.emit_ident_load(name);
                Ok(())
            }
            Expression::Index(i) if matches!(self.types.get(&expr_span(&i.object)), Some(Type::Record(_, _))) => {
                if is_compound(op) {
                    return Err(crate::error::ClsError::CompileError(
                        "Operadores compuestos (+=) sobre registros no soportados en el JIT".to_string(),
                    ));
                }
                // r["key"] = val → record_set(ptr, key, val_bits)
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
                let cls_t = self.types.get(&expr_span(&a.value)).cloned().unwrap_or(Type::Any);
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
            Expression::Index(i) => {
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
                    apply_compound_ty(&mut self.body, op, elem_ty)?;
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
                    let elem_ty = self.index_elem_type(i)?;
                    let elem_size = self.container_elem_size(i, elem_ty);
                    self.emit_expression(&i.object)?;
                    self.emit_expression(&i.index)?;
                    self.emit_expression(&a.value)?;
                    self.emit_index_set(i, elem_size)?;
                }
                Ok(())
            }
            Expression::MemberAccess(m) => {
                let obj_ty = self.types.get(&expr_span(&m.object)).cloned();
                if let Some(Type::Named(name, _)) = obj_ty {
                    if let Some(info) = self.class_defs.get(name.as_str()) {
                        if is_compound(op) {
                            return Err(crate::error::ClsError::CompileError(
                                "Operadores compuestos sobre campos de clase no soportados en el JIT (B3)".to_string(),
                            ));
                        }
                        let fidx = info.fields.iter().position(|(n, _, _, _)| *n == m.member).ok_or_else(|| {
                            crate::error::ClsError::CompileError(format!(
                                "El campo '{}' no existe en la clase '{}'",
                                m.member, name
                            ))
                        })?;
                    let (_, _t, w, off) = &info.fields[fidx];
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
                            WasTy::F64 => self.body.push(Instruction::F64Store(MemArg { offset: 0, align: 3, memory_index: 0 })),
                            WasTy::I32 => self.body.push(Instruction::I32Store(MemArg { offset: 0, align: 2, memory_index: 0 })),
                            WasTy::I64 => self.body.push(Instruction::I64Store(MemArg { offset: 0, align: 3, memory_index: 0 })),
                        }
                        self.body.push(match w {
                            WasTy::F64 => Instruction::LocalGet(val_tmp),
                            WasTy::I32 => Instruction::LocalGet(val_tmp),
                            WasTy::I64 => Instruction::LocalGet(val_tmp),
                        });
                        return Ok(());
                    }
                }
                Err(self.unsupported_expr(&Expression::MemberAccess(m.clone())))
            }
            other => Err(self.unsupported_expr(other)),
        }
    }

    /// `.join(sep)` sobre una tupla: unroll estático (slots conocidos en compile-time).
    fn emit_tuple_join(&mut self, member: &MemberAccessExpr, c: &CallExpr) -> ClsResult<()> {
        let obj_ty = self.types.get(&expr_span(&member.object)).cloned().unwrap_or(Type::Any);
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
                WasTy::F64 => self.body.push(Instruction::F64Load(MemArg { offset: 0, align: 3, memory_index: 0 })),
                WasTy::I32 => self.body.push(Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 })),
                WasTy::I64 => self.body.push(Instruction::I64Load(MemArg { offset: 0, align: 3, memory_index: 0 })),
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

    /// `math.X(...)` → host del módulo math.
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

    /// `fs.X(...)` → host del módulo fs (básico: exists/cwd/readFile/writeFile/listDir/mkdir/rm).
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

    /// `http.X(...)` → host del módulo http.
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

    /// Tipo de retorno de una llamada o miembro de un módulo stdlib.
    fn module_call_ret(&self, expr: &Expression) -> Option<WasTy> {
        if let Expression::Call(c) = expr {
            if let Expression::MemberAccess(member) = &*c.callee {
                if let Expression::Identifier(obj, _) = &*member.object {
                    if obj == "math" {
                        return match member.member.as_str() {
                            "sqrt" | "pow" | "min" | "max" | "floor" | "ceil" | "round"
                            | "random" | "sin" | "cos" | "tan" | "log" => Some(WasTy::F64),
                            "abs" | "range" => Some(WasTy::I64),
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
                }
            }
        }
        // Miembros de módulos sin llamada: math.PI / math.E
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
        // Constructor de structure: `Punto(3, 4)` → alloc + stores.
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
                self.body.push(Instruction::I64Const(info.fields.len() as i64));
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
                        WasTy::F64 => self.body.push(Instruction::F64Store(MemArg { offset: 0, align: 3, memory_index: 0 })),
                        WasTy::I32 => self.body.push(Instruction::I32Store(MemArg { offset: 0, align: 2, memory_index: 0 })),
                        WasTy::I64 => self.body.push(Instruction::I64Store(MemArg { offset: 0, align: 3, memory_index: 0 })),
                    }
                }
                self.body.push(Instruction::LocalGet(ptr));
                return Ok(());
            }
        }
        // Constructor de clase: `Clase(args)` → alloc + vtable + init fields + ctor.
        if let Expression::Identifier(name, _) = &*c.callee {
            if let Some(info) = self.class_defs.get(name).cloned() {
                self.body.push(Instruction::I64Const(info.total));
                let alloc = self.func_indexes["__alloc"];
                self.body.push(Instruction::Call(alloc));
                let obj = self.fresh_local();
                self.body.push(Instruction::LocalSet(obj));
                // vtable_ptr[0] = vtable_start, class_id[8] = id
                self.body.push(Instruction::LocalGet(obj));
                self.body.push(Instruction::I64Const(info.vtable_start as i64));
                self.emit_i64_store(0);
                self.body.push(Instruction::LocalGet(obj));
                self.body.push(Instruction::I64Const(info.class_id as i64));
                self.emit_i64_store(8);
                // init fields a 0
                for (_fn, _t, w, off) in &info.fields {
                    self.body.push(Instruction::LocalGet(obj));
                    self.body.push(Instruction::I64Const(*off));
                    self.body.push(Instruction::I64Add);
                    self.body.push(Instruction::I32WrapI64);
                    match w {
                        WasTy::F64 => self.body.push(Instruction::F64Const(0.0)),
                        WasTy::I32 => self.body.push(Instruction::I32Const(0)),
                        WasTy::I64 => self.body.push(Instruction::I64Const(0)),
                    }
                    match w {
                        WasTy::F64 => self.body.push(Instruction::F64Store(MemArg { offset: 0, align: 3, memory_index: 0 })),
                        WasTy::I32 => self.body.push(Instruction::I32Store(MemArg { offset: 0, align: 2, memory_index: 0 })),
                        WasTy::I64 => self.body.push(Instruction::I64Store(MemArg { offset: 0, align: 3, memory_index: 0 })),
                    }
                }
                // call Clase::__ctor (o el del padre si no se define) con me
                self.body.push(Instruction::LocalGet(obj));
                for a in &c.args {
                    self.emit_expression(a)?;
                }
                let mut cur = Some(name.to_string());
                while let Some(c) = cur {
                    if let Some(idx) = self.func_indexes.get(&format!("{}::__ctor", c)) {
                        self.body.push(Instruction::Call(*idx));
                        break;
                    }
                    cur = self.class_defs.get(&c).and_then(|i| i.parent.clone());
                }
                self.body.push(Instruction::LocalGet(obj));
                return Ok(());
            }
        }
        // Llamada a función nativa (extensión): import `env.<sym>__<sig>@<lib>`.
        if let Expression::Identifier(name, _) = &*c.callee {
            if let Some(idx) = self.native_indexes.get(name) {
                for a in &c.args {
                    self.emit_expression(a)?;
                }
                self.body.push(Instruction::Call(*idx));
                return Ok(());
            }
        }
        // Métodos de primitivos (callee MemberAccess)
        if let Expression::MemberAccess(member) = &*c.callee {
            // `super.m(args)` → call directo al método del padre (sin vtable).
            if let Expression::Identifier(sn, _) = &*member.object {
                if sn == "super" {
                    if let Some(cur) = &self.current_class {
                        if let Some(parent) = self.class_defs.get(cur).and_then(|i| i.parent.clone()) {
                            let key = format!("{}::{}", parent, member.member);
                            if let Some(idx) = self.func_indexes.get(&key) {
                                self.body.push(Instruction::LocalGet(0)); // me
                                for a in &c.args {
                                    self.emit_expression(a)?;
                                }
                                self.body.push(Instruction::Call(*idx));
                                return Ok(());
                            }
                        }
                    }
                    return Err(crate::error::ClsError::CompileError(
                        "super solo se puede usar dentro de métodos de clase (JIT)".to_string(),
                    ));
                }
            }
            // Módulos stdlib: math / json / fs
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
                        self.emit_expression(&c.args[0])?;
                        let t = self.types.get(&expr_span(&c.args[0])).cloned().unwrap_or(Type::Any);
                        let kind = match t {
                            Type::Record(_, _) => 1,
                            Type::Array(_) => 2,
                            _ => 0,
                        };
                        self.body.push(Instruction::I64Const(kind));
                        self.host.call(HostFn::JsonStringify, &mut self.body);
                        return Ok(());
                    }
                    return Err(self.unsupported_expr(&Expression::Call(c.clone())));
                }
                if obj_name == "fs" {
                    return self.emit_fs_call(member, c);
                }
                if obj_name == "http" {
                    return self.emit_http_call(member, c);
                }
            }
            let obj_ty = self.types.get(&expr_span(&member.object)).cloned().unwrap_or(Type::Any);
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
                Type::Named(name, _) => {
                    if let Some(info) = self.class_defs.get(name.as_str()) {
                        let method_slot = info
                            .methods
                            .iter()
                            .position(|m| *m == member.member)
                            .ok_or_else(|| {
                                crate::error::ClsError::CompileError(format!(
                                    "El método '{}' no existe en la clase '{}'",
                                    member.member, name
                                ))
                            })? as u32;
                        let method_key = format!("{}::{}", name, member.member);
                        let ty = self.method_type_indexes.get(&method_key).copied().ok_or_else(|| {
                            crate::error::ClsError::CompileError("Método sin tipo WASM".to_string())
                        })?;
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
                        self.body.push(Instruction::I64Load(MemArg { offset: 0, align: 3, memory_index: 0 }));
                        self.body.push(Instruction::I64Const(method_slot as i64));
                        self.body.push(Instruction::I64Add);
                        self.body.push(Instruction::I32WrapI64);
                        self.body.push(Instruction::CallIndirect { ty, table: 0 });
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
                "print" => {
                    for arg in &c.args {
                        self.emit_print_arg(arg)?;
                    }
                    self.host.call(HostFn::PrintEnd, &mut self.body);
                    return Ok(());
                }
                "len" => {
                    let arg = &c.args[0];
                    self.emit_expression(arg)?;
                    self.emit_array_len();
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
                    self.emit_expression(arg)?;
                    self.emit_to_int(arg)?;
                    return Ok(());
                }
                "float" => {
                    let arg = &c.args[0];
                    self.emit_expression(arg)?;
                    self.emit_to_float(arg)?;
                    return Ok(());
                }
                "bool" => {
                    let arg = &c.args[0];
                    self.emit_expression(arg)?;
                    self.emit_to_bool(arg)?;
                    return Ok(());
                }
                "type" => {
                    let arg = &c.args[0];
                    let span = expr_span(arg);
                    let t = self.types.get(&span).cloned().unwrap_or(Type::Any);
                    let idx = self.intern_string(type_name_str(&t));
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
        if let Expression::Identifier(name, _) = &*c.callee {
            if let Some(fidx) = self.func_indexes.get(name).copied() {
                for arg in &c.args {
                    self.emit_expression(arg)?;
                }
                // Args faltantes → valores por defecto (en el call site)
                if let Some(defaults) = self.func_defaults.get(name) {
                    let provided = c.args.len();
                    for d in defaults.iter().skip(provided) {
                        match d {
                            Some(expr) => self.emit_expression(expr)?,
                            None => self.body.push(Instruction::I64Const(0)),
                        }
                    }
                }
                self.body.push(Instruction::Call(fidx));
                return Ok(());
            }
        }
        Err(self.unsupported_expr(&Expression::Call(c.clone())))
    }

    fn emit_print_arg(&mut self, arg: &Expression) -> ClsResult<()> {
        // Index sobre un record heterogéneo (value Any): imprimir según el tag del valor.
        if let Expression::Index(i) = arg {
            let obj_ty = self.types.get(&expr_span(&i.object)).cloned();
            if matches!(obj_ty, Some(Type::Record(_, _))) {
                self.emit_expression(&i.object)?;
                self.emit_expression(&i.index)?;
                let key_tmp = self.fresh_local();
                let ptr_tmp = self.fresh_local();
                self.body.push(Instruction::LocalSet(key_tmp));
                self.body.push(Instruction::LocalSet(ptr_tmp));
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
                // if tag == 1 → string
                self.body.push(Instruction::LocalGet(tag_tmp));
                self.body.push(Instruction::I64Const(1));
                self.body.push(Instruction::I64Eq);
                self.block_depth += 1;
                self.body.push(Instruction::If(BlockType::Empty));
                self.body.push(Instruction::LocalGet(val_tmp));
                self.host.call(HostFn::PrintStr, &mut self.body);
                self.body.push(Instruction::Else);
                // elif tag == 2 → float
                self.body.push(Instruction::LocalGet(tag_tmp));
                self.body.push(Instruction::I64Const(2));
                self.body.push(Instruction::I64Eq);
                self.block_depth += 1;
                self.body.push(Instruction::If(BlockType::Empty));
                self.body.push(Instruction::LocalGet(val_tmp));
                self.body.push(Instruction::F64ReinterpretI64);
                self.host.call(HostFn::PrintFloat, &mut self.body);
                self.body.push(Instruction::Else);
                // elif tag == 3 → bool
                self.body.push(Instruction::LocalGet(tag_tmp));
                self.body.push(Instruction::I64Const(3));
                self.body.push(Instruction::I64Eq);
                self.block_depth += 1;
                self.body.push(Instruction::If(BlockType::Empty));
                self.body.push(Instruction::LocalGet(val_tmp));
                self.body.push(Instruction::I32WrapI64);
                self.host.call(HostFn::PrintBool, &mut self.body);
                self.body.push(Instruction::Else);
                // else → int
                self.body.push(Instruction::LocalGet(val_tmp));
                self.host.call(HostFn::PrintInt, &mut self.body);
                self.body.push(Instruction::End);
                self.block_depth -= 1;
                self.body.push(Instruction::End);
                self.block_depth -= 1;
                self.body.push(Instruction::End);
                self.block_depth -= 1;
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
        // Llamadas a funciones nativas (extensión) → tipo de retorno codificado.
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
        // Llamadas a módulos stdlib → tipo de retorno conocido (print float/int).
        if let Some(w) = self.module_call_ret(arg) {
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
        let span = expr_span(arg);
        let t = self.types.get(&span).cloned().unwrap_or(Type::Any);
        match t {
            Type::String => self.host.call(HostFn::PrintStr, &mut self.body),
            Type::Bool => self.host.call(HostFn::PrintBool, &mut self.body),
            Type::Char => self.host.call(HostFn::PrintChar, &mut self.body),
            Type::Float => self.host.call(HostFn::PrintFloat, &mut self.body),
            Type::Array(elem) => {
                // Formatear `[e1, e2, ...]` como el walker (evita imprimir el ptr).
                let w = was_type(&*elem)?;
                let kind = arr_kind_code(&*elem);
                self.body.push(Instruction::I64Const(elem_size_bytes(w)));
                self.body.push(Instruction::I64Const(kind));
                self.host.call(HostFn::ArrToString, &mut self.body);
                self.host.call(HostFn::PrintStr, &mut self.body);
            }
            Type::Record(_, _) => {
                // Formatear `{k: v, ...}` como el walker (evita imprimir el ptr).
                self.host.call(HostFn::RecordToString, &mut self.body);
                self.host.call(HostFn::PrintStr, &mut self.body);
            }
            Type::Named(name, _) if self.class_defs.contains_key(&name) => {
                // Si la clase define __repr → usarlo (el ptr ya está en el stack).
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
                for (i, (fname, t_cls, w, off)) in info.fields.iter().enumerate() {
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
                        WasTy::F64 => self.body.push(Instruction::F64Load(MemArg { offset: 0, align: 3, memory_index: 0 })),
                        WasTy::I32 => self.body.push(Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 })),
                        WasTy::I64 => self.body.push(Instruction::I64Load(MemArg { offset: 0, align: 3, memory_index: 0 })),
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
            Type::Named(name, _) if self.struct_defs.contains_key(&name) => {
                let ptr = self.fresh_local();
                self.body.push(Instruction::LocalSet(ptr));
                self.emit_struct_to_string(&name, ptr)?;
                self.host.call(HostFn::PrintStr, &mut self.body);
                return Ok(());
            }
            Type::Named(name, _) if self.enum_defs.contains_key(&name) => {
                let variants = self.enum_defs[&name].1.clone();
                // index = v & 0xffffffff → seleccionar el string de la variante
                self.body.push(Instruction::I64Const(0xffff_ffff));
                self.body.push(Instruction::I64And);
                let idx = self.fresh_local();
                self.body.push(Instruction::LocalSet(idx));
                let n = variants.len();
                if n == 0 {
                    let s = self.intern_string("");
                    self.emit_load_str(s);
                    self.host.call(HostFn::PrintStr, &mut self.body);
                    return Ok(());
                }
                self.body.push(Instruction::LocalGet(idx));
                self.body.push(Instruction::I64Const(0));
                self.body.push(Instruction::I64Eq);
                self.block_depth += 1;
                self.body.push(Instruction::If(BlockType::Result(ValType::I64)));
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
                            self.body.push(Instruction::If(BlockType::Result(ValType::I64)));
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
                return Ok(());
            }
            Type::Union(_) => {
                match union_base(&t) {
                    Type::String => self.host.call(HostFn::PrintStr, &mut self.body),
                    Type::Float => self.host.call(HostFn::PrintFloat, &mut self.body),
                    Type::Bool => self.host.call(HostFn::PrintBool, &mut self.body),
                    _ => self.host.call(HostFn::PrintInt, &mut self.body),
                }
            }
            Type::Literal(l) => match l {
                LitVal::Str(_) => self.host.call(HostFn::PrintStr, &mut self.body),
                LitVal::Float(_) => self.host.call(HostFn::PrintFloat, &mut self.body),
                LitVal::Bool(_) => self.host.call(HostFn::PrintBool, &mut self.body),
                _ => self.host.call(HostFn::PrintInt, &mut self.body),
            },
            _ => self.host.call(HostFn::PrintInt, &mut self.body),
        }
        Ok(())
    }

    /// Construye la representación `Punto { x: 3, y: 4 }` de un struct y la deja
    /// en el stack (el ptr del struct está en `ptr`).
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
                WasTy::F64 => self.body.push(Instruction::F64Load(MemArg { offset: 0, align: 3, memory_index: 0 })),
                WasTy::I32 => self.body.push(Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 })),
                WasTy::I64 => self.body.push(Instruction::I64Load(MemArg { offset: 0, align: 3, memory_index: 0 })),
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
            Type::Named(name, _) if self.struct_defs.contains_key(&name) => {
                let ptr = self.fresh_local();
                self.body.push(Instruction::LocalSet(ptr));
                self.emit_struct_to_string(&name, ptr)?;
            }
            Type::Named(name, _) if self.class_defs.contains_key(&name) => {
                // toString(obj) → __toString si existe; si no, __repr; el ptr está en stack.
                if let Some(idx) = self.func_indexes.get(&format!("{}::__toString", name)) {
                    self.body.push(Instruction::Call(*idx));
                } else if let Some(idx) = self.func_indexes.get(&format!("{}::__repr", name)) {
                    self.body.push(Instruction::Call(*idx));
                } else {
                    self.host.call(HostFn::StrInt, &mut self.body);
                }
            }
            _ => self.host.call(HostFn::StrInt, &mut self.body),
        }
        Ok(())
    }

    fn emit_to_int(&mut self, arg: &Expression) -> ClsResult<()> {
        let span = expr_span(arg);
        let t = self.types.get(&span).cloned().unwrap_or(Type::Any);
        match t {
            Type::Int => {}
            Type::Float => self.body.push(Instruction::I64TruncSatF64S),
            Type::Bool => self.body.push(Instruction::I64ExtendI32U),
            Type::String => self.host.call(HostFn::ParseInt, &mut self.body),
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
            Type::String => self.host.call(HostFn::ParseFloat, &mut self.body),
            _ => {}
        }
        Ok(())
    }

    fn emit_to_bool(&mut self, arg: &Expression) -> ClsResult<()> {
        let span = expr_span(arg);
        let t = self.types.get(&span).cloned().unwrap_or(Type::Any);
        match t {
            Type::Bool => {}
            Type::Int => {
                self.body.push(Instruction::I64Eqz);
                self.body.push(Instruction::I32Eqz);
            }
            Type::Float => {
                self.body.push(Instruction::F64Const(0.0f64));
                self.body.push(Instruction::F64Neq);
            }
            Type::String => self.host.call(HostFn::ParseBool, &mut self.body),
            _ => {}
        }
        Ok(())
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
                WasTy::F64 => self.body.push(Instruction::F64Store(MemArg { offset: 0, align: 3, memory_index: 0 })),
                WasTy::I32 => self.body.push(Instruction::I32Store(MemArg { offset: 0, align: 2, memory_index: 0 })),
                WasTy::I64 => self.body.push(Instruction::I64Store(MemArg { offset: 0, align: 3, memory_index: 0 })),
            }
        }
        self.body.push(Instruction::LocalGet(ptr));
        Ok(())
    }

    /// Member access de primitivos: `.length` sobre tuplas/arrays, variantes de enum.
    fn emit_member_access(&mut self, m: &MemberAccessExpr) -> ClsResult<()> {
        // Enum: `Nivel.Alto` → constante (def_id<<32 | index)
        if let Expression::Identifier(obj_name, _) = &*m.object {
            if let Some((def_id, variants)) = self.enum_defs.get(obj_name).cloned() {
                let idx = variants.iter().position(|v| *v == m.member).ok_or_else(|| {
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
                        self.body.push(Instruction::F64Const(std::f64::consts::PI));
                        return Ok(());
                    }
                    "E" => {
                        self.body.push(Instruction::F64Const(std::f64::consts::E));
                        return Ok(());
                    }
                    _ => return Err(self.unsupported_expr(&Expression::MemberAccess(m.clone()))),
                }
            }
        }
        let obj_ty = self.types.get(&expr_span(&m.object)).cloned().unwrap_or(Type::Any);
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
                _ => Err(self.unsupported_expr(&Expression::MemberAccess(m.clone()))),
            },
            Type::Named(name, _) => {
                if let Some(info) = self.struct_defs.get(name.as_str()) {
                    let fidx = info.fields.iter().position(|(n, _, _)| *n == m.member).ok_or_else(|| {
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
                        WasTy::F64 => self.body.push(Instruction::F64Load(MemArg { offset: 0, align: 3, memory_index: 0 })),
                        WasTy::I32 => self.body.push(Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 })),
                        WasTy::I64 => self.body.push(Instruction::I64Load(MemArg { offset: 0, align: 3, memory_index: 0 })),
                    }
                    Ok(())
                } else if let Some(info) = self.class_defs.get(name.as_str()) {
                    let fidx = info.fields.iter().position(|(n, _, _, _)| *n == m.member).ok_or_else(|| {
                        crate::error::ClsError::CompileError(format!(
                            "El campo '{}' no existe en la clase '{}'",
                            m.member, name
                        ))
                    })?;
                        let (_, _t, w, off) = &info.fields[fidx];
                        let w = *w;
                        let off = *off;
                    self.body.push(Instruction::I64Const(off));
                    self.body.push(Instruction::I64Add);
                    self.body.push(Instruction::I32WrapI64);
                    match w {
                        WasTy::F64 => self.body.push(Instruction::F64Load(MemArg { offset: 0, align: 3, memory_index: 0 })),
                        WasTy::I32 => self.body.push(Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 })),
                        WasTy::I64 => self.body.push(Instruction::I64Load(MemArg { offset: 0, align: 3, memory_index: 0 })),
                    }
                    Ok(())
                } else {
                    Err(self.unsupported_expr(&Expression::MemberAccess(m.clone())))
                }
            }
            _ => Err(self.unsupported_expr(&Expression::MemberAccess(m.clone()))),
        }
    }

    /// `"Hola $nombre ${expr}"` → concatenación de las partes (toString de cada expr).
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
        let elem_size = elem_size_bytes(elem_ty);
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
            let val_tmp = self.fresh_local_ty(elem_ty);
            let addr_tmp = self.fresh_local();
            self.body.push(Instruction::LocalSet(val_tmp));
            self.body.push(Instruction::LocalGet(ptr));
            self.body.push(Instruction::I64Const(16 + (i as i64) * elem_size));
            self.body.push(Instruction::I64Add);
            self.body.push(Instruction::LocalSet(addr_tmp));
            self.body.push(Instruction::LocalGet(addr_tmp));
            self.body.push(Instruction::I32WrapI64);
            self.body.push(Instruction::LocalGet(val_tmp));
            match elem_ty {
                WasTy::F64 => {
                    self.body.push(Instruction::F64Store(MemArg { offset: 0, align: 3, memory_index: 0 }))
                }
                WasTy::I32 => {
                    self.body.push(Instruction::I32Store(MemArg { offset: 0, align: 2, memory_index: 0 }))
                }
                WasTy::I64 => {
                    self.body.push(Instruction::I64Store(MemArg { offset: 0, align: 3, memory_index: 0 }))
                }
            }
        }
        self.body.push(Instruction::LocalGet(ptr));
        Ok(())
    }

    fn array_elem_type(&self, a: &ArrayExpr) -> ClsResult<WasTy> {
        if let Some(first) = a.elements.first() {
            return self.value_type(first);
        }
        Err(crate::error::ClsError::CompileError(
            "Array literal vacío sin tipo no soportado por el JIT".to_string(),
        ))
    }

    /// Literal de record `{ a: 1, b: "x" }` → record_new + record_set.
    fn emit_record(&mut self, r: &RecordExpr) -> ClsResult<()> {
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
            let cls_t = self.types.get(&expr_span(val)).cloned().unwrap_or(Type::Any);
            self.body.push(Instruction::I64Const(arr_kind_code(&cls_t)));
            self.host.call(HostFn::RecordSet, &mut self.body);
            self.body.push(Instruction::Drop);
        }
        self.body.push(Instruction::LocalGet(ptr));
        Ok(())
    }

    fn emit_index_get(&mut self, i: &IndexExpr) -> ClsResult<()> {
        // Record: r["key"] → record_get(ptr, key)
        let obj_ty = self.types.get(&expr_span(&i.object)).cloned();
        if matches!(obj_ty, Some(Type::Record(_, _))) {
            self.emit_expression(&i.object)?;
            self.emit_expression(&i.index)?;
            self.host.call(HostFn::RecordGet, &mut self.body);
            let elem_ty = self.index_elem_type(i)?;
            self.bits_to_elem(elem_ty)?;
            return Ok(());
        }
        let elem_ty = self.index_elem_type(i)?;
        self.emit_expression(&i.object)?;
        self.emit_expression(&i.index)?;
        let elem_size = self.container_elem_size(i, elem_ty);
        self.emit_index_access(elem_ty, elem_size, i)
    }

    /// Asume [ptr, idx] en stack; deja el valor del elemento (con bounds check).
    fn emit_index_access(&mut self, elem_ty: WasTy, elem_size: i64, _i: &IndexExpr) -> ClsResult<()> {
        let ptr = self.fresh_local();
        let idx = self.fresh_local();
        self.body.push(Instruction::LocalSet(idx));
        self.body.push(Instruction::LocalSet(ptr));
        // bounds check
        self.bounds_check(ptr, idx);
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
            WasTy::F64 => self.body.push(Instruction::F64Load(MemArg { offset: 0, align: 3, memory_index: 0 })),
            WasTy::I32 => self.body.push(Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 })),
            WasTy::I64 => self.body.push(Instruction::I64Load(MemArg { offset: 0, align: 3, memory_index: 0 })),
        }
        Ok(())
    }

    /// Emite el check `0 <= idx < len[ptr]`, trap si falla. Usa locals.
    fn bounds_check(&mut self, ptr: u32, idx: u32) {
        self.body.push(Instruction::LocalGet(idx));
        self.body.push(Instruction::I64Const(0));
        self.body.push(Instruction::I64LtS);
        self.body.push(Instruction::LocalGet(idx));
        self.body.push(Instruction::LocalGet(ptr));
        self.body.push(Instruction::I32WrapI64);
        self.body.push(Instruction::I64Load(MemArg { offset: 8, align: 3, memory_index: 0 }));
        self.body.push(Instruction::I64GeS);
        self.body.push(Instruction::I32Or);
        self.block_depth += 1;
        self.body.push(Instruction::If(BlockType::Empty));
        let msg = self.intern_string("Índice fuera de rango");
        self.emit_load_str(msg);
        self.host.call(HostFn::Trap, &mut self.body);
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
        self.bounds_check(ptr, idx);
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
            WasTy::F64 => self.body.push(Instruction::F64Store(MemArg { offset: 0, align: 3, memory_index: 0 })),
            WasTy::I32 => self.body.push(Instruction::I32Store(MemArg { offset: 0, align: 2, memory_index: 0 })),
            WasTy::I64 => self.body.push(Instruction::I64Store(MemArg { offset: 0, align: 3, memory_index: 0 })),
        }
        Ok(())
    }

    fn emit_array_len(&mut self) {
        // ptr está en stack → len = i64.load(ptr+8)
        self.body.push(Instruction::I32WrapI64);
        self.body.push(Instruction::I64Load(MemArg { offset: 8, align: 3, memory_index: 0 }));
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
        }
        Ok(())
    }

    fn emit_i64_store(&mut self, offset: u32) {
        // stack: [addr(i64), value] → reordenar con wrap
        let v = self.fresh_local();
        self.body.push(Instruction::LocalSet(v));
        self.body.push(Instruction::I32WrapI64);
        self.body.push(Instruction::LocalGet(v));
        self.body.push(Instruction::I64Store(MemArg { offset, align: 3, memory_index: 0 }));
    }
}

fn is_compound(op: Operator) -> bool {
    matches!(
        op,
        Operator::PlusEqual | Operator::MinusEqual | Operator::StarEqual | Operator::SlashEqual
    )
}

/// Código del tipo de elemento para `arr_join` (0=int, 1=string, 2=float, 3=bool, 4=char).
fn arr_kind_code(t: &Type) -> i64 {
    match t {
        Type::String => 1,
        Type::Float | Type::F32 | Type::F64 => 2,
        Type::Bool => 3,
        Type::Char => 4,
        _ => 0,
    }
}

/// Tipo runtime de una unión (monomórfica) → el tipo base de sus miembros.
fn union_base(t: &Type) -> Type {
    if let Type::Union(members) = t {
        if members
            .iter()
            .all(|m| matches!(m, Type::String | Type::Literal(LitVal::Str(_))))
        {
            return Type::String;
        }
        if members
            .iter()
            .all(|m| matches!(m, Type::Int | Type::Literal(LitVal::Int(_))))
        {
            return Type::Int;
        }
        if members.iter().all(|m| {
            matches!(
                m,
                Type::Float | Type::F32 | Type::F64 | Type::Literal(LitVal::Float(_))
            )
        }) {
            return Type::Float;
        }
        if members
            .iter()
            .all(|m| matches!(m, Type::Bool | Type::Literal(LitVal::Bool(_))))
        {
            return Type::Bool;
        }
    }
    t.clone()
}

/// Aplica el operador compuesto a los dos valores del stack (según el tipo).
fn apply_compound_ty(
    body: &mut Vec<Instruction>,
    op: Operator,
    ty: WasTy,
) -> Result<(), crate::error::ClsError> {
    let inst = match (op, ty) {
        (Operator::PlusEqual, WasTy::F64) => Instruction::F64Add,
        (Operator::MinusEqual, WasTy::F64) => Instruction::F64Sub,
        (Operator::StarEqual, WasTy::F64) => Instruction::F64Mul,
        (Operator::SlashEqual, WasTy::F64) => Instruction::F64Div,
        (Operator::PlusEqual, _) => Instruction::I64Add,
        (Operator::MinusEqual, _) => Instruction::I64Sub,
        (Operator::StarEqual, _) => Instruction::I64Mul,
        (Operator::SlashEqual, _) => Instruction::I64DivS,
        _ => {
            return Err(crate::error::ClsError::CompileError(
                "Operador compuesto no soportado por el JIT".to_string(),
            ))
        }
    };
    body.push(inst);
    Ok(())
}

fn elem_size_bytes(w: WasTy) -> i64 {
    match w {
        WasTy::I64 | WasTy::F64 => 8,
        WasTy::I32 => 4,
    }
}

fn type_name_str(t: &Type) -> &'static str {
    match t {
        Type::Int | Type::I8 | Type::I16 | Type::I32 | Type::I64 => "Int",
        Type::Float | Type::F32 | Type::F64 => "Float",
        Type::String => "String",
        Type::Bool => "Bool",
        Type::Char => "Char",
        Type::Array(_) => "Array",
        _ => "Any",
    }
}

fn expr_display(expr: &Expression) -> String {
    format!("{:?}", expr)
}

fn statement_display(stmt: &Statement) -> String {
    format!("{}", stmt)
}

fn annotation_to_type(ann: &TypeAnnotation) -> Type {
    use crate::frontend::ast::TypeKind;
    match &ann.kind {
        TypeKind::Int | TypeKind::I32 | TypeKind::I64 | TypeKind::I16 | TypeKind::I8 => Type::Int,
        TypeKind::Float | TypeKind::F32 | TypeKind::F64 => Type::Float,
        TypeKind::String => Type::String,
        TypeKind::Bool => Type::Bool,
        TypeKind::Char => Type::Char,
        TypeKind::Void | TypeKind::Empty => Type::Void,
        TypeKind::Array(inner) => Type::Array(Box::new(annotation_to_type(inner))),
        _ => Type::Any,
    }
}

/// Compila un Module tipado a un binario WASM.
pub struct WasmBackend {
    types: HashMap<Span, Type>,
    target: Target,
}

impl WasmBackend {
    pub fn new(types: HashMap<Span, Type>) -> Self {
        Self {
            types,
            target: Target::host(),
        }
    }

    /// Backend con un target explícito (para `when` compile-time).
    pub fn with_target(types: HashMap<Span, Type>, target: Target) -> Self {
        Self { types, target }
    }

    pub fn emit(&self, module: &Module) -> ClsResult<Vec<u8>> {
        let mut engine = Engine::new(&self.types, self.target.clone());
        engine.emit(module)
    }
}

/// Motor de emisión a nivel de módulo.
struct Engine<'a> {
    types: &'a HashMap<Span, Type>,
    // Builders de sección persistentes: se agregan al módulo en el orden WASM.
    types_sec: TypeSection,
    imports_sec: ImportSection,
    funcs_sec: FunctionSection,
    memories_sec: MemorySection,
    globals_sec: GlobalSection,
    exports_sec: ExportSection,
    data_sec: DataSection,
    code_sec: CodeSection,
    type_count: u32,
    func_count: u32,
    func_indexes: HashMap<String, u32>,
    func_types: HashMap<String, (Vec<Type>, Option<Type>)>,
    func_defaults: HashMap<String, Vec<Option<Expression>>>,
    host_indexes: HashMap<HostFn, u32>,
    string_pool: Vec<String>,
    string_index: HashMap<String, u32>,
    enum_defs: HashMap<String, (u32, Vec<String>)>,
    struct_defs: HashMap<String, StructInfo>,
    native_indexes: HashMap<String, u32>,
    native_ret: HashMap<String, char>,
    globals: HashMap<String, u32>,
    global_inits: Vec<(u32, Expression)>,
    tables_sec: TableSection,
    elements_sec: ElementSection,
    class_defs: HashMap<String, ClassInfo>,
    next_table_slot: u32,
    /// Funciones de clase a compilar: (clave `Clase::m`, FunctionDecl).
    cls_funcs_extra: Vec<(String, FunctionDecl)>,
    /// type index WASM de cada método de clase (para `call_indirect`).
    method_type_indexes: HashMap<String, u32>,
    /// Métodos de clase pendientes de declarar (tras alloc/load_str).
    pending_class_methods: Vec<(String, FunctionDecl)>,
    target: Target,
}

/// Definición de una clase compilada: layout de objeto + vtable.
#[derive(Clone)]
struct ClassInfo {
    parent: Option<String>,
    /// id de clase (índice en orden de declaración) para `is` por herencia.
    class_id: u32,
    /// cadena de ancestors: [padre, abuelo, ...].
    ancestors: Vec<String>,
    /// campos (nombre, tipo CLS, tipo WASM, offset en bytes desde 16).
    fields: Vec<(String, Type, WasTy, i64)>,
    /// nombres de métodos en orden canónico (posición = slot de la vtable).
    methods: Vec<String>,
    /// índice de la tabla donde empieza la vtable de esta clase.
    vtable_start: u32,
    /// tamaño total del objeto (16 + campos).
    total: i64,
}

/// Definición de una extensión compilada (import `env.<sym>__<sig>@<lib>`).
#[derive(Clone)]
struct NativeSig {
    lib: String,
    params: Vec<char>,
    ret: char,
}

/// Código de tipo nativo para la firma de extensiones: i=int, f=float, b=bool,
/// c=char, s=string, v=void. El nombre del import codifica ret+params.
fn ty_code(t: &Type) -> (char, WasTy) {
    match t {
        Type::String => ('s', WasTy::I64),
        Type::Float => ('f', WasTy::F64),
        Type::Bool => ('b', WasTy::I32),
        Type::Char => ('c', WasTy::I32),
        Type::Void => ('v', WasTy::I64),
        _ => ('i', WasTy::I64),
    }
}

fn code_to_was(c: char) -> WasTy {
    match c {
        'f' => WasTy::F64,
        'b' | 'c' => WasTy::I32,
        _ => WasTy::I64,
    }
}

fn was_to_val(w: WasTy) -> ValType {
    match w {
        WasTy::F64 => ValType::F64,
        WasTy::I32 => ValType::I32,
        WasTy::I64 => ValType::I64,
    }
}

/// Definición de un structure compilada: campos con tipos, offsets y tamaño.
#[derive(Clone)]
struct StructInfo {
    def_id: u32,
    /// campos (nombre, tipo CLS, tipo WASM).
    fields: Vec<(String, Type, WasTy)>,
    offsets: Vec<i64>,
    total: i64,
}

impl<'a> Engine<'a> {
    fn new(types: &'a HashMap<Span, Type>, target: Target) -> Self {
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
            func_indexes: HashMap::new(),
            func_types: HashMap::new(),
            func_defaults: HashMap::new(),
            host_indexes: HashMap::new(),
            string_pool: Vec::new(),
            string_index: HashMap::new(),
            enum_defs: HashMap::new(),
            struct_defs: HashMap::new(),
            native_indexes: HashMap::new(),
            native_ret: HashMap::new(),
            globals: HashMap::new(),
            global_inits: Vec::new(),
            tables_sec: TableSection::new(),
            elements_sec: ElementSection::new(),
            class_defs: HashMap::new(),
            next_table_slot: 0,
            cls_funcs_extra: Vec::new(),
            method_type_indexes: HashMap::new(),
            pending_class_methods: Vec::new(),
            target,
        }
    }

    fn register_func_type(&mut self, params: Vec<ValType>, results: Vec<ValType>) -> u32 {
        let idx = self.type_count;
        self.type_count += 1;
        self.types_sec.function(params, results);
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
            .import("env", Some(h.import_name()), EntityType::Function(tidx));
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

    /// Agrega las secciones al módulo en el orden WASM correcto.
    fn build_module(&mut self) -> WasmModule {
        let mut m = WasmModule::new();
        m.section(&self.types_sec);
        m.section(&self.imports_sec);
        m.section(&self.funcs_sec);
        m.section(&self.tables_sec);
        m.section(&self.memories_sec);
        m.section(&self.globals_sec);
        m.section(&self.exports_sec);
        m.section(&self.elements_sec);
        m.section(&self.code_sec);
        m.section(&self.data_sec);
        m
    }

    fn collect_functions(&mut self, module: &Module) -> ClsResult<()> {
        for stmt in &module.statements {
            if let Statement::FunctionDecl(f) = stmt {
                self.collect_function(f)?;
            }
        }
        if !self.func_types.contains_key("main") {
            return Err(crate::error::ClsError::CompileError(
                "No se encontró function main(args: String[]) para el JIT".to_string(),
            ));
        }
        Ok(())
    }

    fn collect_function(&mut self, f: &FunctionDecl) -> ClsResult<()> {
        let mut params: Vec<Type> = Vec::new();
        let mut defaults: Vec<Option<Expression>> = Vec::new();
        for p in &f.params {
            let t = p.type_ann.as_ref().ok_or_else(|| {
                crate::error::ClsError::CompileError(format!(
                    "Parámetro '{}' de '{}' sin anotación de tipo (requerido por el JIT)",
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
                "Anotación de tipo no soportada por el JIT (se requiere tipo concreto)".to_string(),
            )),
            other => Ok(other),
        }
    }

    fn emit(&mut self, module: &Module) -> ClsResult<Vec<u8>> {
        self.collect_functions(module)?;

        // Recolectar enums → (def_id, variantes) para constantes `Nivel.Alto`.
        let mut def_id = 0u32;
        for stmt in &module.statements {
            if let Statement::EnumDecl(e) = stmt {
                self.enum_defs.insert(e.name.clone(), (def_id, e.variants.clone()));
                def_id += 1;
            }
        }
        // Recolectar structures → offsets de campos (layout [def_id][len][campos]).
        let mut sdef_id = 0u32;
        for stmt in &module.statements {
            if let Statement::StructureDecl(s) = stmt {
                let mut fields = Vec::new();
                let mut offsets = Vec::new();
                let mut off = 16i64;
                for f in &s.fields {
                    let t = annotation_to_type(&f.type_ann);
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
        // Recolectar clases → class_defs (layout de objeto) + declarar métodos/ctor.
        let mut next_class_id = 0u32;
        for stmt in &module.statements {
            if let Statement::ClassDecl(c) = stmt {
                let mut fields = Vec::new();
                let mut methods = Vec::new();
                let mut off = 16i64; // 0..7 = vtable, 8..15 = class_id
                let mut total = off;
                let mut ancestors = Vec::new();
                if let Some(parent) = &c.extends {
                    if let Some(pinfo) = self.class_defs.get(parent) {
                        fields.extend(pinfo.fields.clone());
                        methods = pinfo.methods.clone();
                        off = pinfo.total;
                        total = pinfo.total;
                        ancestors.push(parent.clone());
                        ancestors.extend(pinfo.ancestors.clone());
                    }
                }
                for member in &c.body {
                    match member {
                        ClassMember::Property(p) if !p.is_static => {
                            let w = match (&p.type_ann, &p.value) {
                                (Some(ann), _) => {
                                    was_type(&annotation_to_type(ann)).unwrap_or(WasTy::I64)
                                }
                                (None, Some(v)) => self.expr_was_type(v).unwrap_or(WasTy::I64),
                                (None, None) => WasTy::I64,
                            };
                            let t_cls = p
                                .type_ann
                                .as_ref()
                                .map(annotation_to_type)
                                .unwrap_or_else(|| {
                                    if matches!(w, WasTy::F64) {
                                        Type::Float
                                    } else {
                                        Type::Int
                                    }
                                });
                            fields.push((p.name.clone(), t_cls, w, off));
                            off += elem_size_bytes(w);
                            total = off;
                        }
                        ClassMember::Method(m) => {
                            if !methods.contains(&m.name) {
                                methods.push(m.name.clone());
                            }
                            let mut m2 = m.clone();
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
                        vtable_start: vs,
                        total,
                    },
                );
            }
        }
        // Recolectar extensiones → imports `env.<sym>__<sig>@<lib>`.
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
                        let import_name = format!("{}__{}{}@{}", f.name, rc, params_code, e.library);
                        let tidx = self.register_func_type(params_was, results);
                        let idx = self.func_count;
                        self.func_count += 1;
                        self.imports_sec
                            .import("env", Some(&import_name), EntityType::Function(tidx));
                        self.native_indexes.insert(f.name.clone(), idx);
                        self.native_ret.insert(f.name.clone(), rc);
                    }
                }
            }
        }

        use HostFn::*;
        for h in [
            PrintInt, PrintFloat, PrintBool, PrintChar, PrintStr, PrintEnd, Now, Exit, Sleep,
            Trap, ParseInt, ParseFloat, ParseBool, StrConcat, StrInt, StrFloat, StrBool, StrChar,
            PowNum, Fmod, Input, StrUpper, StrLower, StrTrim, StrContains, StrStartsWith,
            StrEndsWith, StrIsEmpty, StrLength, IntAbs, FloatAbs, ArrPush, ArrPop, ArrShift,
            ArrUnshift, ArrIndexOf, ArrIncludes, ArrJoin, ArrReverse, MathSqrt, MathPow, MathMin,
            MathMax, MathFloor, MathCeil, MathRound, MathRandom, MathSin, MathCos, MathTan, MathLog,
            MathRange, JsonStringify, JsonParse, FsExists, FsCwd, FsReadFile, FsWriteFile, FsListDir, FsMkdir,
            FsRm, RecordNew, RecordSet, RecordGet, RecordHas, RecordTag, RecordLen, RecordKeys, RecordValues,
            RecordToString, HttpGet, HttpPost, ArrToString,
        ] {
            self.register_host(h);
        }

        // Memoria (1 página = 64KB; el allocator hace grow).
        self.memories_sec.memory(MemoryType {
            limits: Limits { min: 1, max: None },
        });

        // Global: heap_ptr, mut, inicial 1MB (tras el string pool).
        self.globals_sec.global(
            GlobalType {
                val_type: ValType::I64,
                mutable: true,
            },
            Instruction::I64Const(1048576),
        );

        // Globals de usuario: `var x` / `const x` top-level → sección globals.
        // índice 0 = heap_ptr; los de usuario empiezan en 1.
        let mut next_global = 1u32;
        for stmt in &module.statements {
            if let Statement::VarDecl(v) | Statement::ConstDecl(v) = stmt {
                let w = match (&v.type_ann, &v.value) {
                    (Some(ann), _) => was_type(&annotation_to_type(ann)).unwrap_or(WasTy::I64),
                    (None, Some(val)) => self.expr_was_type(val).unwrap_or(WasTy::I64),
                    (None, None) => WasTy::I64,
                };
                let is_const = matches!(stmt, Statement::ConstDecl(_));
                let _ = is_const;
                let idx = next_global;
                next_global += 1;
                self.globals.insert(v.name.clone(), idx);
                // mutable=true siempre: __init_globals las setea (incluso const, que
                // no se vuelve a escribir en runtime).
                self.globals_sec.global(
                    GlobalType {
                        val_type: w.val_type(),
                        mutable: true,
                    },
                    match w {
                        WasTy::F64 => Instruction::F64Const(0.0),
                        WasTy::I32 => Instruction::I32Const(0),
                        WasTy::I64 => Instruction::I64Const(0),
                    },
                );
                if let Some(val) = &v.value {
                    self.global_inits.push((idx, val.clone()));
                }
            }
        }

        // Internas __alloc y __load_str.
        let alloc_idx = self.declare_wasm_function(vec![ValType::I64], vec![ValType::I64]);
        self.func_indexes.insert("__alloc".to_string(), alloc_idx);
        let ls_idx = self.declare_wasm_function(vec![ValType::I64], vec![ValType::I64]);
        self.func_indexes.insert("__load_str".to_string(), ls_idx);

        // __init_globals: se declara DESPUÉS de alloc/load_str para que el code_sec
        // quede alineado (alloc, load_str, init, cls...).
        if !self.global_inits.is_empty() {
            let ig_idx = self.declare_wasm_function(vec![], vec![]);
            self.func_indexes.insert("__init_globals".to_string(), ig_idx);
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
                for t in &params {
                    pv.push(was_type(t)?.val_type());
                }
                let rv: Vec<ValType> = match &ret {
                    Some(r) if *r != Type::Void => vec![was_type(r)?.val_type()],
                    _ => vec![],
                };
                let fidx = self.declare_wasm_function(pv, rv);
                self.func_indexes.insert(f.name.clone(), fidx);
                cls_funcs.push(f.clone());
            }
        }

        // Compilar cuerpos (internan strings). El orden del code_sec DEBE coincidir
        // con el orden de declaración: alloc, load_str, [init], métodos, cls.
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

        // __alloc y __load_str (el pool de strings ya está completo).
        let alloc_body = self.build_allocator();
        let load_str_body = self.build_load_str();
        // __init_globals se construye ANTES del data segment: sus strings (valores
        // iniciales de las globals) deben internarse en el pool antes del data.
        let init_body = self.build_global_init()?;

        // Tabla de vtables: segmento con los funcref de los métodos de cada clase
        // (los vtable_start ya se asignaron en la recolección, en orden).
        let mut table_funcs: Vec<u32> = Vec::new();
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
        if !table_funcs.is_empty() {
            self.tables_sec.table(TableType {
                element_type: ValType::FuncRef,
                limits: Limits {
                    min: table_funcs.len() as u32,
                    max: None,
                },
            });
            self.elements_sec.active(
                Some(0),
                Instruction::I32Const(0),
                ValType::FuncRef,
                Elements::Functions(&table_funcs),
            );
        }

        // Data segment con la tabla de strings.
        let data_bytes = self.build_string_data();
        self.data_sec.segment(DataSegment {
            mode: DataSegmentMode::Active {
                memory_index: 0,
                offset: Instruction::I32Const(0),
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
            .export("main", Export::Function(self.func_indexes["main"]));
        self.exports_sec
            .export("alloc", Export::Function(self.func_indexes["__alloc"]));
        self.exports_sec.export("memory", Export::Memory(0));

        Ok(self.build_module().finish())
    }

    fn compile_function(&mut self, f: &FunctionDecl) -> ClsResult<Function> {
        let (param_types, _ret) = self.func_types[&f.name].clone();
        let mut fe = FuncEmitter::new(
            self.types,
            HostCaller {
                indexes: self.host_indexes.clone(),
            },
            &mut self.string_pool,
            &mut self.string_index,
            &self.func_indexes,
            &self.func_defaults,
            &self.enum_defs,
            &self.struct_defs,
            &self.native_indexes,
            &self.native_ret,
            &self.globals,
            &self.class_defs,
            &self.method_type_indexes,
            None,
            &self.target,
        );
        // Métodos de clase: `me` (la instancia) es el primer param implícito.
        let is_method = f.name.contains("::");
        let current_class = if is_method {
            f.name.split("::").next().map(|s| s.to_string())
        } else {
            None
        };
        fe.current_class = current_class;
        if is_method {
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
        for s in &f.body.statements {
            fe.emit_statement(s)?;
        }
        // End final del cuerpo de la función (wasm-encoder no lo añade).
        fe.body.push(Instruction::End);
        // locals: cada índice con su tipo (fallback I64).
        // Importante: los params ocupan los índices 0..param_types.len(); los
        // locals declarados empiezan después. Cada local = un grupo de 1 para
        // preservar los índices exactos (agrupar reordenaría y rompería tipos
        // mixtos).
        let nparams = param_types.len() as u32;
        let local_types: Vec<ValType> = (nparams..fe.next_local)
            .map(|i| fe.local_tys.get(&i).copied().unwrap_or(WasTy::I64).val_type())
            .collect();
        let grouped: Vec<(u32, ValType)> = local_types.iter().map(|t| (1, *t)).collect();
        let mut func = Function::new(grouped);
        for inst in fe.body {
            func.instruction(inst);
        }
        Ok(func)
    }

    /// Tipo WASM de una expresión desde el type map (fallback I64).
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
            &self.enum_defs,
            &self.struct_defs,
            &self.native_indexes,
            &self.native_ret,
            &self.globals,
            &self.class_defs,
            &self.method_type_indexes,
            None,
            &self.target,
        );
        for (idx, val) in &self.global_inits {
            fe.emit_expression(val)?;
            fe.body.push(Instruction::GlobalSet(*idx));
        }
        fe.body.push(Instruction::End);
        // Declarar los temporales que la emisión pudo crear (emit_array, etc.).
        let local_types: Vec<ValType> = (0..fe.next_local)
            .map(|i| fe.local_tys.get(&i).copied().unwrap_or(WasTy::I64).val_type())
            .collect();
        let grouped: Vec<(u32, ValType)> = local_types.iter().map(|t| (1, *t)).collect();
        let mut func = Function::new(grouped);
        for inst in fe.body {
            func.instruction(inst);
        }
        Ok(Some(func))
    }

    /// Declara una función de clase (`Clase::m` o ctor) con `me` como primer param.
    fn declare_class_function(&mut self, class: &str, f: &FunctionDecl) {
        let mut param_cls = vec![Type::Int]; // me (ptr del objeto)
        let mut pv = vec![ValType::I64];
        for p in &f.params {
            let t = p.type_ann.as_ref().map(annotation_to_type).unwrap_or(Type::Int);
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
        let key = format!("{}::{}", class, f.name);
        self.func_indexes.insert(key.clone(), fidx);
        self.func_types.insert(key.clone(), (param_cls, ret_cls));
        self.method_type_indexes.insert(key.clone(), tidx);
        self.cls_funcs_extra.push((key, f.clone()));
    }

    /// Índice de función de un método: en la clase o subiendo por ancestors.
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

    fn build_allocator(&self) -> Function {        // (func (param $n i64) (result i64)
        //   local 0 = n (param), local 1 = ptr, local 2 = end
        //   ptr = global 0
        //   end = (ptr + n + 8) & -8
        //   if end > memsize*65536 → grow 16 páginas
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
            Instruction::LocalGet(2),
            Instruction::MemorySize(0),
            Instruction::I64ExtendI32U,
            Instruction::I64Const(65536),
            Instruction::I64Mul,
            Instruction::I64LeU,
            Instruction::BrIf(0),
            Instruction::I32Const(16),
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
            func.instruction(inst);
        }
        func
    }

    fn build_load_str(&self) -> Function {
        // (func (param $i i64) (result i64)
        //   local 0 = i (param), 1 = entry, 2 = off, 3 = len
        //   entry = i*8
        //   off = i32.load(entry)
        //   len = i32.load(entry+4)
        //   result = (off << 32) | len)
        let mut b = vec![
            Instruction::LocalGet(0),
            Instruction::I64Const(8),
            Instruction::I64Mul,
            Instruction::LocalSet(1),
            Instruction::LocalGet(1),
            Instruction::I32WrapI64,
            Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 }),
            Instruction::I64ExtendI32U,
            Instruction::LocalSet(2),
            Instruction::LocalGet(1),
            Instruction::I64Const(4),
            Instruction::I64Add,
            Instruction::I32WrapI64,
            Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 }),
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
            func.instruction(inst);
        }
        func
    }

    fn build_string_data(&self) -> Vec<u8> {
        let table_bytes = self.string_pool.len() * 8;
        let mut bytes: Vec<u8> = vec![0u8; table_bytes];
        let mut offset = table_bytes as u32;
        for (i, s) in self.string_pool.iter().enumerate() {
            let len = s.len() as u32;
            bytes[i * 8..i * 8 + 4].copy_from_slice(&offset.to_le_bytes());
            bytes[i * 8 + 4..i * 8 + 8].copy_from_slice(&len.to_le_bytes());
            bytes.extend_from_slice(s.as_bytes());
            offset += len;
        }
        bytes
    }
}

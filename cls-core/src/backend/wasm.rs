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
use crate::middleware::types::Type;
use std::collections::HashMap;
use wasm_encoder::{
    BlockType, CodeSection, DataSection, DataSegment, DataSegmentMode, EntityType, Export,
    ExportSection, Function, FunctionSection, GlobalSection, GlobalType, ImportSection, Instruction,
    Limits, MemArg, MemorySection, MemoryType, Module as WasmModule, TypeSection, ValType,
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
}

impl<'a> FuncEmitter<'a> {
    fn new(
        types: &'a HashMap<Span, Type>,
        host: HostCaller,
        string_pool: &'a mut Vec<String>,
        string_index: &'a mut HashMap<String, u32>,
        func_indexes: &'a HashMap<String, u32>,
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

    fn declare_var_ty(&mut self, name: &str, ty: WasTy) -> u32 {
        let idx = self.local_for(name);
        self.local_tys.entry(idx).or_insert(ty);
        idx
    }

    fn value_type(&self, expr: &Expression) -> ClsResult<WasTy> {
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
                    (Some(ann), _) => was_type(&annotation_to_type(ann))?,
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
            other => Err(self.unsupported_stmt(other)),
        }
    }

    fn unsupported_stmt(&self, stmt: &Statement) -> crate::error::ClsError {
        crate::error::ClsError::CompileError(format!(
            "Statement no soportado por el JIT (subconjunto): {}",
            statement_display(stmt)
        ))
    }

    fn emit_if(&mut self, i: &IfStatement) -> ClsResult<()> {
        self.emit_expression(&i.condition)?;
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
                let idx = self.local_for(name);
                self.body.push(Instruction::LocalGet(idx));
                Ok(())
            }
            Expression::Binary(b) => self.emit_binary(b),
            Expression::Unary(u) => self.emit_unary(u),
            Expression::Call(c) => self.emit_call(c),
            Expression::Index(i) => self.emit_index_get(i),
            Expression::Array(a) => self.emit_array(a),
            Expression::Conditional(c) => self.emit_conditional(c),
            Expression::Assignment(a) => self.emit_assignment(a),
            Expression::Parenthesized(inner, _) => self.emit_expression(inner),
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
                return Err(crate::error::ClsError::CompileError(
                    "Operador % con floats no soportado por el JIT".to_string(),
                ))
            }            Percent => {
                self.emit_expression(&b.left)?;
                self.emit_expression(&b.right)?;
                self.div_zero_trap()?;
                self.body.push(Instruction::I64RemS);
            }
            StarStar => return Err(self.unsupported_expr(&Expression::Binary(b.clone()))),
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
                self.body.push(Instruction::If(BlockType::Empty));
                self.body.push(Instruction::I32Const(0));
                self.body.push(Instruction::Else);
                self.emit_expression(&b.right)?;
                self.body.push(Instruction::End);
                self.block_depth -= 1;
            }
            Or => {
                self.emit_expression(&b.left)?;
                self.block_depth += 1;
                self.body.push(Instruction::If(BlockType::Empty));
                self.body.push(Instruction::I32Const(1));
                self.body.push(Instruction::Else);
                self.emit_expression(&b.right)?;
                self.body.push(Instruction::End);
                self.block_depth -= 1;
            }
            In | Is => return Err(self.unsupported_expr(&Expression::Binary(b.clone()))),
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
            _ => return Err(self.unsupported_expr(&Expression::Unary(u.clone()))),
        }
        Ok(())
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
                let idx = self.local_for(name);
                if is_compound(op) {
                    self.body.push(Instruction::LocalGet(idx));
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
                self.body.push(Instruction::LocalSet(idx));
                self.body.push(Instruction::LocalGet(idx));
                Ok(())
            }
            Expression::Index(i) => {
                self.emit_expression(&i.object)?;
                self.emit_expression(&i.index)?;
                self.emit_expression(&a.value)?;
                self.emit_index_set(i)?;
                Ok(())
            }
            other => Err(self.unsupported_expr(other)),
        }
    }

    fn emit_call(&mut self, c: &CallExpr) -> ClsResult<()> {
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
                self.body.push(Instruction::Call(fidx));
                return Ok(());
            }
        }
        Err(self.unsupported_expr(&Expression::Call(c.clone())))
    }

    fn emit_print_arg(&mut self, arg: &Expression) -> ClsResult<()> {
        self.emit_expression(arg)?;
        let span = expr_span(arg);
        let t = self.types.get(&span).cloned().unwrap_or(Type::Any);
        match t {
            Type::String => self.host.call(HostFn::PrintStr, &mut self.body),
            Type::Bool => self.host.call(HostFn::PrintBool, &mut self.body),
            Type::Char => self.host.call(HostFn::PrintChar, &mut self.body),
            Type::Float => self.host.call(HostFn::PrintFloat, &mut self.body),
            Type::Array(_) => self.host.call(HostFn::PrintInt, &mut self.body),
            _ => self.host.call(HostFn::PrintInt, &mut self.body),
        }
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

    fn emit_array(&mut self, a: &ArrayExpr) -> ClsResult<()> {
        let elem_ty = self.array_elem_type(a)?;
        let elem_size = elem_size_bytes(elem_ty);
        let n = a.elements.len() as i64;
        self.body.push(Instruction::I64Const(n));
        self.body.push(Instruction::I64Const(elem_size));
        self.body.push(Instruction::I64Mul);
        self.body.push(Instruction::I64Const(8));
        self.body.push(Instruction::I64Add);
        let alloc = self.func_indexes["__alloc"];
        self.body.push(Instruction::Call(alloc));
        let ptr = self.fresh_local();
        self.body.push(Instruction::LocalSet(ptr));
        // header: len
        self.body.push(Instruction::LocalGet(ptr));
        self.body.push(Instruction::I64Const(n));
        self.emit_i64_store(0);
        // elementos
        for (i, el) in a.elements.iter().enumerate() {
            self.emit_expression(el)?;
            let val_tmp = self.fresh_local_ty(elem_ty);
            let addr_tmp = self.fresh_local();
            self.body.push(Instruction::LocalSet(val_tmp));
            self.body.push(Instruction::LocalGet(ptr));
            self.body.push(Instruction::I64Const(8 + (i as i64) * elem_size));
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

    fn emit_index_get(&mut self, i: &IndexExpr) -> ClsResult<()> {
        let elem_ty = self.index_elem_type(i)?;
        let elem_size = elem_size_bytes(elem_ty);
        self.emit_expression(&i.object)?;
        self.emit_expression(&i.index)?;
        let ptr = self.fresh_local();
        let idx = self.fresh_local();
        self.body.push(Instruction::LocalSet(idx));
        self.body.push(Instruction::LocalSet(ptr));
        // bounds check
        self.bounds_check(ptr, idx);
        // addr = ptr + 8 + idx*elem_size
        self.body.push(Instruction::LocalGet(ptr));
        self.body.push(Instruction::LocalGet(idx));
        self.body.push(Instruction::I64Const(elem_size));
        self.body.push(Instruction::I64Mul);
        self.body.push(Instruction::I64Const(8));
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
        self.body.push(Instruction::I64Load(MemArg { offset: 0, align: 3, memory_index: 0 }));
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
            other => Err(crate::error::ClsError::CompileError(format!(
                "Indexado sobre '{}' no soportado",
                other
            ))),
        }
    }

    /// Asume [arr_ptr, idx, value] en stack. Escribe el valor.
    fn emit_index_set(&mut self, i: &IndexExpr) -> ClsResult<()> {
        let elem_ty = self.index_elem_type(i)?;
        let elem_size = elem_size_bytes(elem_ty);
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
        self.body.push(Instruction::I64Const(8));
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
        // ptr está en stack → len = i64.load(ptr)
        self.body.push(Instruction::I32WrapI64);
        self.body.push(Instruction::I64Load(MemArg { offset: 0, align: 3, memory_index: 0 }));
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
}

impl WasmBackend {
    pub fn new(types: HashMap<Span, Type>) -> Self {
        Self { types }
    }

    pub fn emit(&self, module: &Module) -> ClsResult<Vec<u8>> {
        let mut engine = Engine::new(&self.types);
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
    host_indexes: HashMap<HostFn, u32>,
    string_pool: Vec<String>,
    string_index: HashMap<String, u32>,
}

impl<'a> Engine<'a> {
    fn new(types: &'a HashMap<Span, Type>) -> Self {
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
            host_indexes: HashMap::new(),
            string_pool: Vec::new(),
            string_index: HashMap::new(),
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
        m.section(&self.memories_sec);
        m.section(&self.globals_sec);
        m.section(&self.exports_sec);
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
        for p in &f.params {
            let t = p.type_ann.as_ref().ok_or_else(|| {
                crate::error::ClsError::CompileError(format!(
                    "Parámetro '{}' de '{}' sin anotación de tipo (requerido por el JIT)",
                    p.name, f.name
                ))
            })?;
            params.push(self.resolve_annotation_type(t)?);
        }
        let ret = match &f.return_type {
            Some(t) => Some(self.resolve_annotation_type(t)?),
            None => None,
        };
        self.func_types.insert(f.name.clone(), (params, ret));
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

        use HostFn::*;
        for h in [
            PrintInt, PrintFloat, PrintBool, PrintChar, PrintStr, PrintEnd, Now, Exit, Sleep,
            Trap, ParseInt, ParseFloat, ParseBool, StrConcat, StrInt, StrFloat, StrBool, StrChar,
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

        // Internas __alloc y __load_str.
        let alloc_idx = self.declare_wasm_function(vec![ValType::I64], vec![ValType::I64]);
        self.func_indexes.insert("__alloc".to_string(), alloc_idx);
        let ls_idx = self.declare_wasm_function(vec![ValType::I64], vec![ValType::I64]);
        self.func_indexes.insert("__load_str".to_string(), ls_idx);

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

        // Compilar cuerpos (internan strings).
        let mut bodies: Vec<(String, Function)> = Vec::new();
        for f in &cls_funcs {
            let body = self.compile_function(f)?;
            bodies.push((f.name.clone(), body));
        }

        // __alloc y __load_str (el pool de strings ya está completo).
        let alloc_body = self.build_allocator();
        let load_str_body = self.build_load_str();

        // Data segment con la tabla de strings.
        let data_bytes = self.build_string_data();
        self.data_sec.segment(DataSegment {
            mode: DataSegmentMode::Active {
                memory_index: 0,
                offset: Instruction::I32Const(0),
            },
            data: data_bytes,
        });

        // Code section en el MISMO orden que las funciones: alloc, load_str, cls...
        self.code_sec.function(&alloc_body);
        self.code_sec.function(&load_str_body);
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
        );
        for (i, p) in f.params.iter().enumerate() {
            fe.declare_var_ty(&p.name, was_type(&param_types[i])?);
        }
        for s in &f.body.statements {
            fe.emit_statement(s)?;
        }
        // End final del cuerpo de la función (wasm-encoder no lo añade).
        fe.body.push(Instruction::End);
        // locals: cada índice con su tipo (fallback I64).
        let mut local_types: Vec<ValType> = Vec::new();
        for i in 0..fe.next_local {
            let ty = fe.local_tys.get(&i).copied().unwrap_or(WasTy::I64);
            local_types.push(ty.val_type());
        }
        // Function::new espera (count, type); cada local es count 1.
        let grouped = group_locals(&local_types);
        let mut func = Function::new(grouped);
        for inst in fe.body {
            func.instruction(inst);
        }
        Ok(func)
    }

    fn build_allocator(&self) -> Function {
        // (func (param $n i64) (result i64)
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

fn group_locals(locals: &[ValType]) -> Vec<(u32, ValType)> {
    let mut out: Vec<(u32, ValType)> = Vec::new();
    for t in locals {
        if let Some(last) = out.last_mut() {
            if last.1 == *t {
                last.0 += 1;
                continue;
            }
        }
        out.push((1, *t));
    }
    out
}

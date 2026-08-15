//! REPL JIT: sesión interactiva de CLS con estado persistente entre líneas.
//!
//! Cada línea se compila como un módulo CLS completo (CLS → WASM → wasmtime) y
//! se ejecuta como el `main` de ese módulo. El estado sobrevive entre líneas
//! porque el host transfiere las globals WASM (`__g_{idx}`) y la región de heap
//! del módulo anterior al nuevo antes de llamar a `main`.
//!
//! Modelo de sesión:
//! - Los `var`/`const` top-level de una línea se "hoistean": la línea original
//!   conserva su inicializador; a partir de la siguiente sesión la declaración
//!   se emite SIN valor (solo con anotación de tipo, extraída del typeck de la
//!   línea original) y su valor lo provee la transferencia de estado.
//! - Redectarar un nombre ya hoisted se convierte en una asignación en `main`
//!   (el tipo lo valida el typeck: si el tipo difiere, error de tipos).
//! - Las demás declaraciones top-level (funciones, clases, enums, imports,
//!   `when`, ...) se re-emiten verbatim en cada sesión.
//! - `function main` definida por el usuario: rechazada (el REPL la sintetiza).
//! - Los spans de cada línea se desplazan con un offset único (100000 * n) para
//!   no colisionar en el type map; el render de errores de-shiftea.
//!
//! Limitaciones conocidas:
//! - Solo runtime wasmtime (las excepciones CLS requieren exception-handling).
//! - Los `static var` de clase se re-inicializan en cada línea (no se
//!   transfieren; solo las globals de módulo).
//! - Si el string pool de la sesión supera 1MB, la transferencia del heap
//!   (que parte de 1MB) pisa el pool nuevo.

use cls_core::config::types::TypesConfig;
use cls_core::error::diagnostic::Severity;
use cls_core::error::{ClsError, Span};
use cls_core::frontend::ast::{
    AssignmentExpr, Block, Expression, FunctionDecl, Literal, LiteralKind, Module, Parameter,
    Statement, TypeAnnotation, TypeKind, VarDecl, Visibility,
};
use cls_core::frontend::span_shift::shift_module;
use cls_core::frontend::token::Operator;
use cls_core::middleware::typeck::expr_span;
use cls_core::middleware::types::{LitVal, Type};
use cls_core::middleware::TypeChecker;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use wasmtime::{Caller, Engine, Instance, Linker, Memory, Module as WasmModule, Store, Val};

use crate::engine::{build_error_string, unpack_span};
use crate::flatten::flatten_imports;
use crate::host::host_trap_message;
use crate::state::HostState;
use crate::wasmtime_rt::{register_host_functions_opt, register_native_hosts, HOST};
use crate::JitContext;

/// Offset de línea base para los spans de cada línea del REPL (y de los
/// imports). Las líneas ocupan 100000*(n+1); los imports de la sesión
/// 100000*(n+2+i).
const LINE_BASE: u32 = 100000;

/// Inicio del heap del módulo (heap_ptr global inicial = 1MB, tras el string
/// pool). Solo se transfiere la región [1MB, len): los datos bajo 1MB son el
/// string pool del módulo NUEVO (data segments re-emitidos).
const HEAP_START: usize = 1048576;

/// Resultado de procesar una línea del REPL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplResult {
    /// Línea ejecutada y commiteada (el estado quedó actualizado).
    Ok,
    /// Error de sintaxis (lexer/parser): la línea NO se commiteó.
    SyntaxError,
    /// Error de compilación (typeck/backend): la línea NO se commiteó.
    CompileError,
    /// Error de runtime: la línea NO se commiteó (el estado anterior sigue vivo).
    RuntimeError,
}

/// Una línea commiteada: declaraciones hoisted (vars/consts sin init, con
/// anotación) y el resto de declaraciones top-level, con spans ya desplazados
/// por su offset de línea.
struct ReplLine {
    hoisted: Vec<Statement>,
    other: Vec<Statement>,
}

/// Instancia WASM viva (estado persistente entre líneas).
struct LiveState {
    store: Store<HostState>,
    instance: Instance,
    memory: Memory,
}

/// Sesión REPL con estado persistente.
pub struct ReplSession {
    engine: Engine,
    lines: Vec<ReplLine>,
    /// Nombres de vars/consts hoisted (redeclarar = asignar).
    var_names: HashSet<String>,
    /// Tipos hoisted de los vars/consts: validan las reasignaciones entre
    /// líneas (B2 — el typeck permisivo no rechaza el cambio de tipo y la
    /// transferencia de bytes corrompería el slot).
    var_types: HashMap<String, Type>,
    /// String pool de la sesión (orden de interning, append-only): se re-siembra
    /// como decls pool-only en cada módulo nuevo ANTES de los decls de la línea
    /// actual. Así los punteros de strings transferidos entre instancias
    /// conservan su offset (B1).
    pool_strings: Vec<String>,
    /// (offset de línea, source de la línea) por línea commiteada.
    line_sources: Vec<(u32, String)>,
    live: Option<LiveState>,
}

impl ReplSession {
    pub fn new() -> Result<Self, String> {
        let mut config = wasmtime::Config::new();
        config.wasm_exceptions(true);
        let engine = Engine::new(&config).map_err(|e| e.to_string())?;
        Ok(ReplSession {
            engine,
            lines: Vec::new(),
            var_names: HashSet::new(),
            var_types: HashMap::new(),
            pool_strings: Vec::new(),
            line_sources: Vec::new(),
            live: None,
        })
    }

    /// Ejecuta una línea de código CLS. `source` debe ser la línea YA preparada
    /// (el CLI envuelve expresiones sueltas en `print(...)` y completa el `;`).
    /// Si la línea falla (sintaxis/compilación/runtime), el estado de la sesión
    /// queda intacto (rollback): las variables vuelven al valor anterior.
    pub fn run_line(&mut self, source: &str, ctx: &JitContext) -> ReplResult {
        if source.trim().is_empty() {
            return ReplResult::Ok;
        }
        let line_index = self.lines.len();
        let offset = LINE_BASE * (line_index as u32 + 1);

        // 1) Parseo de la línea (spans naturales; los errores se muestran aquí).
        let mut lexer = cls_core::frontend::Lexer::new(source);
        let tokens = match lexer.tokenize() {
            Ok(t) => t,
            Err(e) => {
                cls_runtime::show_syntax_error(e, source, "<repl>");
                return ReplResult::SyntaxError;
            }
        };
        let mut parser = cls_core::frontend::Parser::new(tokens);
        let mut line_module = match parser.parse() {
            Ok(m) => m,
            Err(e) => {
                cls_runtime::show_syntax_error(e, source, "<repl>");
                return ReplResult::SyntaxError;
            }
        };
        if line_module.statements.is_empty() {
            return ReplResult::Ok;
        }
        // Desplazar los spans de la línea: únicos dentro del módulo de la sesión.
        shift_module(&mut line_module, offset);

        // 2) Clasificar: decls de vars (hoist/reassign), otras declaraciones
        //    top-level (verbatim), statements ejecutables (van al main).
        let mut current_decls: Vec<(bool, VarDecl)> = Vec::new();
        let mut other: Vec<Statement> = Vec::new();
        let mut runnable: Vec<Statement> = Vec::new();
        for stmt in &line_module.statements {
            match stmt {
                Statement::VarDecl(v) => current_decls.push((false, v.clone())),
                Statement::ConstDecl(v) => current_decls.push((true, v.clone())),
                Statement::FunctionDecl(f) if f.name == "main" => {
                    self.show_shifted(
                        "ERROR",
                        "main() está reservado para el intérprete del REPL",
                        Some(&f.span),
                        source,
                    );
                    return ReplResult::CompileError;
                }
                Statement::FunctionDecl(_)
                | Statement::ClassDecl(_)
                | Statement::StructureDecl(_)
                | Statement::InterfaceDecl(_)
                | Statement::ModuleDecl(_)
                | Statement::NamespaceDecl(_)
                | Statement::TypeAlias(_)
                | Statement::EnumDecl(_)
                | Statement::Import(_)
                | Statement::FromImport(_)
                | Statement::Include(_)
                | Statement::Extension(_)
                | Statement::When(_)
                | Statement::Config(_)
                | Statement::Meta(_) => other.push(stmt.clone()),
                _ => runnable.push(stmt.clone()),
            }
        }

        // 3) Módulo de la sesión: [hoisted y other de líneas previas] +
        //    [seed del string pool] + [decls nuevas de esta línea] + [other de
        //    esta línea] + [main].
        let mut module_statements: Vec<Statement> = Vec::new();
        for line in &self.lines {
            module_statements.extend(line.hoisted.iter().cloned());
            module_statements.extend(line.other.iter().cloned());
        }
        // Seed del string pool de la sesión (B1): los literales de líneas
        // previas (asignaciones, prints, bodies de funciones, ...) se re-internan
        // como decls pool-only ANTES de los decls de esta línea — posición
        // idéntica al orden de interning de la sesión anterior — para que los
        // punteros de strings transferidos entre instancias conserven su offset.
        let pool_base = LINE_BASE * 3000;
        for (i, s) in self.pool_strings.iter().enumerate() {
            let sp = Span::new(pool_base + i as u32, 1, 1, 1);
            module_statements.push(Statement::VarDecl(VarDecl {
                name: format!("__clspool_{}", i),
                type_ann: None,
                value: Some(Expression::Literal(Literal {
                    kind: LiteralKind::String(s.clone()),
                    span: sp.clone(),
                })),
                visibility: Visibility::Private,
                span: sp.clone(),
                is_static: false,
                is_readonly: false,
                pool_only: true,
                pool_seed: true,
            }));
        }
        // Reasignaciones a vars hoisted (nombre, span del valor): B2 las
        // valida tras el typeck (el typeck permisivo no rechaza el cambio de
        // tipo y la transferencia de bytes corrompería el slot).
        let mut reassign_checks: Vec<(String, Span)> = Vec::new();
        let mut main_body: Vec<Statement> = Vec::new();
        for (is_const, v) in &current_decls {
            if self.var_names.contains(&v.name) {
                // Redeclaración → asignación en main (el tipo lo valida el typeck).
                if let Some(value) = &v.value {
                    let span = v.span.clone();
                    reassign_checks.push((v.name.clone(), expr_span(value)));
                    main_body.push(Statement::Expression(Expression::Assignment(
                        AssignmentExpr {
                            target: Box::new(Expression::Identifier(v.name.clone(), v.span.clone())),
                            op: Operator::Equal,
                            value: Box::new(value.clone()),
                            span,
                        },
                    )));
                }
                let _ = is_const;
            } else {
                let stmt = if *is_const {
                    Statement::ConstDecl(v.clone())
                } else {
                    Statement::VarDecl(v.clone())
                };
                module_statements.push(stmt);
            }
        }
        for stmt in &runnable {
            if let Statement::Expression(Expression::Assignment(a)) = stmt {
                if let Expression::Identifier(name, _) = a.target.as_ref() {
                    reassign_checks.push((name.clone(), expr_span(&a.value)));
                }
            }
        }
        module_statements.extend(other.iter().cloned());
        main_body.extend(runnable);
        module_statements.push(Statement::FunctionDecl(synthesize_main(main_body)));
        let module = Module {
            statements: module_statements,
            span: Span::new(1, 1, 1, 1),
        };

        // 4) Typeck de la sesión (con imports como prelude). Config casi
        //    PERMISIVA: el REPL es un playground (como el walker), sin
        //    anotaciones los params/funciones caen a Any y el JIT despacha en
        //    runtime. PERO `no_implicit_any: true`: un identificador NO
        //    definido es un error (antes caía a Any y `print(clear)` emitía 0
        //    silenciosamente).
        let types_config = TypesConfig {
            check: true,
            strict: false,
            no_implicit_any: true,
            null_safety: false,
        };
        let mut checker = TypeChecker::new(types_config);
        checker.register_host_intrinsics(ctx.host_intrinsics);
        let base_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let mut seen = HashSet::new();
        let mut imports: Vec<(String, String, Module)> = Vec::new();
        let manifest = cls_core::config::ModuleManifest::find_in_dir(&base_dir);
        if let Err(e) = crate::resolve::load_import_modules_hooked(
            &module,
            &base_dir,
            &mut seen,
            &mut imports,
            manifest.as_ref(),
            ctx.module_source_resolver,
        ) {
            self.show_shifted_error(&e, source);
            return ReplResult::CompileError;
        }
        // Desplazar los imports (detrás de todas las líneas de la sesión).
        for (i, (_path, _src, m)) in imports.iter_mut().enumerate() {
            shift_module(m, LINE_BASE * (line_index as u32 + 2 + i as u32));
        }
        let prelude: Vec<(String, Module)> = imports
            .iter()
            .map(|(p, _, m)| (p.clone(), m.clone()))
            .collect();
        if let Err(e) = checker.check_with_prelude(&module, &prelude) {
            self.show_shifted_error(&e, source);
            return ReplResult::CompileError;
        }
        let has_type_errors = checker
            .diagnostics()
            .iter()
            .any(|d| matches!(d.severity, Severity::Error));
        if has_type_errors {
            for diag in checker.diagnostics() {
                if matches!(diag.severity, Severity::Error) {
                    self.show_shifted(
                        "ERROR",
                        &diag.message,
                        Some(&diag.span),
                        source,
                    );
                    return ReplResult::CompileError;
                }
            }
        }
        // B2: rechazar cambios de tipo en reasignaciones (a vars hoisted con
        // tipo conocido). El typeck PERMISIVO los deja pasar, pero la
        // transferencia de estado copia bytes crudos sin migrar el tipo → el
        // slot se lee con el tipo declarado viejo (basura).
        for (name, value_span) in &reassign_checks {
            if let (Some(new_t), Some(old_t)) =
                (checker.type_map().get(value_span), self.var_types.get(name))
            {
                if !new_t.is_assignable_to(old_t) {
                    self.show_shifted(
                        "ERROR",
                        &format!(
                            "no se puede reasignar '{}': {} no es asignable a {} (el REPL no permite cambiar el tipo de una variable entre líneas)",
                            name,
                            new_t,
                            old_t
                        ),
                        Some(value_span),
                        source,
                    );
                    return ReplResult::CompileError;
                }
            }
        }
        let type_map = checker.type_map();

        // 5) Emisión WASM.
        let opts = cls_core::backend::wasm::WasmBackendOptions {
            exceptions: true,
            require_main: true,
            intrinsics: ctx.host_intrinsics.to_vec(),
        };
        let backend = cls_core::backend::wasm::WasmBackend::with_options(
            type_map,
            cls_core::frontend::ast::Target::host(),
            opts,
        );
        let merged = flatten_imports(&module, &prelude);
        let (wasm_bytes, emitted_pool) = match backend.emit_with_pool(&merged) {
            Ok(b) => b,
            Err(e) => {
                self.show_shifted_error(&e, source);
                return ReplResult::CompileError;
            }
        };

        // 6) wasmtime: instancia nueva con las host functions del motor.
        let wasm_module = match WasmModule::new(&self.engine, &wasm_bytes) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("[JIT] Módulo WASM inválido en el REPL: {:#}", e);
                return ReplResult::CompileError;
            }
        };
        let mut modules_map: Vec<(u32, String, String)> = Vec::new();
        for (off, src) in &self.line_sources {
            modules_map.push((*off, "<repl>".to_string(), src.clone()));
        }
        modules_map.push((offset, "<repl>".to_string(), source.to_string()));
        for (i, (path, src, _m)) in imports.iter().enumerate() {
            modules_map.push((
                LINE_BASE * (line_index as u32 + 2 + i as u32),
                path.clone(),
                src.clone(),
            ));
        }
        let mut store = Store::new(
            &self.engine,
            HostState {
                first_in_line: true,
                source_file: "<repl>".to_string(),
                modules: modules_map,
                string_caps: HashMap::new(),
                call_stack: Vec::new(),
                pending_call_site: None,
                simple_fn_names: HashMap::new(),
                host_call: ctx.host_call_handler.clone(),
                output: ctx.output.clone(),
                app_args: Vec::new(),
            },
        );
        let mut linker = Linker::new(&self.engine);
        if let Err(e) = register_host_functions_opt(&mut linker, true, false) {
            eprintln!("[JIT] Error registrando funciones host: {}", e);
            return ReplResult::CompileError;
        }
        if let Err(e) = register_native_hosts(&mut linker, &wasm_module, ctx.native_backend.clone()) {
            eprintln!("[JIT] Error registrando hosts de extensiones: {}", e);
            return ReplResult::CompileError;
        }
        // exit()/trap() → traps codificados (no matan el proceso del REPL).
        if let Err(e) = linker.func_wrap(
            HOST,
            "exit",
            |_: Caller<'_, HostState>, code: i64| -> Result<(), wasmtime::Error> {
                Err(wasmtime::Error::msg(format!("__clsb_exit__:{}", code)))
            },
        ) {
            eprintln!("[JIT] Error registrando env.exit: {}", e);
            return ReplResult::CompileError;
        }
        if let Err(e) = linker.func_wrap(
            HOST,
            "trap",
            |mut c: Caller<'_, HostState>, m: i64, s: i64| -> Result<(), wasmtime::Error> {
                let msg = host_trap_message(&mut c, m, s);
                Err(wasmtime::Error::msg(format!("__clsb_trap__:{}", msg)))
            },
        ) {
            eprintln!("[JIT] Error registrando env.trap: {}", e);
            return ReplResult::CompileError;
        }
        let instance = match linker.instantiate(&mut store, &wasm_module) {
            Ok(i) => i,
            Err(e) => {
                eprintln!("[JIT] Error de instanciación en el REPL: {}", e);
                return ReplResult::CompileError;
            }
        };
        let memory = match instance.get_memory(&mut store, "memory") {
            Some(m) => m,
            None => {
                eprintln!("[JIT] Export 'memory' no disponible");
                return ReplResult::CompileError;
            }
        };

        // 7) Transferir el estado de la instancia anterior (rollback-friendly:
        //    la anterior sigue viva hasta que `main` termine sin error).
        if let Some(prev) = &mut self.live {
            transfer_state(prev, &instance, &mut store, &memory);
        }

        // 8) Ejecutar main (args = 0: el main sintetizado no usa args).
        let main = match instance.get_typed_func::<i64, i64>(&mut store, "main") {
            Ok(f) => f,
            Err(e) => {
                eprintln!("[JIT] Export 'main' no disponible: {}", e);
                return ReplResult::CompileError;
            }
        };
        if let Err(e) = main.call(&mut store, 0) {
            let root = e.root_cause().to_string();
            if root.contains("__clsb_exit__:") {
                // exit() → silencio; la línea se commitea igual.
            } else {
                let payload: Vec<Val> = {
                    let exn = store.take_pending_exception();
                    let mut fields = match exn.as_ref() {
                        Some(x) => x.fields(&mut store).ok(),
                        None => None,
                    };
                    match fields.as_mut() {
                        Some(iter) => iter.collect(),
                        None => Vec::new(),
                    }
                };
                let msg = payload
                    .first()
                    .and_then(|v| v.i64())
                    .map(|packed| read_packed_str(&mut store, &memory, packed))
                    .unwrap_or_default();
                let span = payload.get(1).and_then(|v| v.i64()).map(unpack_span);
                let call_stack = store.data().call_stack.clone();
                let pending = store.data().pending_call_site;
                let trap_text = if root.contains("__clsb_trap__:") {
                    root.trim_start_matches("__clsb_trap__:").to_string()
                } else if root.is_empty() {
                    e.to_string()
                } else {
                    root
                };
                let text = build_error_string(
                    msg,
                    span,
                    call_stack,
                    pending,
                    trap_text,
                    "<repl>",
                    &store.data().modules,
                );
                eprintln!("{}", text);
                return ReplResult::RuntimeError;
            }
        }

        // 9) Commit: la línea pasó. Hoistear los decls nuevos (sin init, con
        //    la anotación del typeck) para las próximas sesiones. El string
        //    pool de la sesión se captura del backend (re-seed en la línea
        //    siguiente); los inits hoisted NO se re-sembran (sus strings ya
        //    viajan en el pool capturado).
        let mut hoisted: Vec<Statement> = Vec::new();
        for (is_const, v) in &current_decls {
            if self.var_names.contains(&v.name) {
                continue;
            }
            let ann = v
                .type_ann
                .clone()
                .or_else(|| checker.type_map().get(&v.span).and_then(annotation_from_type));
            let decl = VarDecl {
                name: v.name.clone(),
                type_ann: ann,
                value: None,
                visibility: v.visibility.clone(),
                span: v.span.clone(),
                is_static: false,
                is_readonly: v.is_readonly,
                pool_only: true,
                pool_seed: false,
            };
            hoisted.push(if *is_const {
                Statement::ConstDecl(decl)
            } else {
                Statement::VarDecl(decl)
            });
            self.var_names.insert(v.name.clone());
            if let Some(t) = checker.type_map().get(&v.span) {
                self.var_types.insert(v.name.clone(), t.clone());
            }
        }
        self.lines.push(ReplLine { hoisted, other });
        self.line_sources.push((offset, source.to_string()));
        self.pool_strings = emitted_pool;
        self.live = Some(LiveState { store, instance, memory });
        ReplResult::Ok
    }

    /// Muestra un error de compilación cuyo span puede estar desplazado
    /// (pertenece al módulo fusionado de la sesión).
    fn show_shifted_error(&self, error: &ClsError, current_source: &str) {
        let span = match error {
            ClsError::SyntaxErrorAt(_, s) | ClsError::CompileErrorAt(_, s) => Some(s.clone()),
            ClsError::CompileError(m) | ClsError::RuntimeError(m) => {
                self.show_shifted("ERROR", m, None, current_source);
                return;
            }
            other => {
                self.show_shifted("ERROR", &other.to_string(), None, current_source);
                return;
            }
        };
        let msg = match error {
            ClsError::SyntaxErrorAt(m, _) | ClsError::CompileErrorAt(m, _) => m.clone(),
            other => other.to_string(),
        };
        self.show_shifted("ERROR", &msg, span.as_ref(), current_source);
    }

    /// Render de un error con span desplazado: de-shiftea contra las fuentes de
    /// la sesión (líneas commiteadas + línea actual) y muestra línea + caret.
    fn show_shifted(&self, severity: &str, msg: &str, span: Option<&Span>, current_source: &str) {
        use cls_core::ansi;
        let mut sources: Vec<(u32, String)> = self.line_sources.clone();
        let cur_offset = LINE_BASE * (self.lines.len() as u32 + 1);
        sources.push((cur_offset, current_source.to_string()));
        let sev = ansi::bold(true, ansi::fg(true, ansi::codes::BRIGHT_RED, severity));
        let colored = ansi::fg(true, ansi::codes::BRIGHT_RED, msg);
        match span {
            Some(s) => {
                let raw_line = s.start_line;
                if raw_line >= LINE_BASE {
                    let idx = raw_line / LINE_BASE;
                    let off = idx * LINE_BASE;
                    let real_line = raw_line - off;
                    if let Some((_, src)) = sources.iter().find(|(o, _)| *o == off) {
                        eprintln!("[{}] {} (<repl>:{}:{})", sev, colored, real_line, s.start_col);
                        if let Some(src_line) = src.lines().nth(real_line.saturating_sub(1) as usize) {
                            let pad = " ".repeat(real_line.to_string().len());
                            eprintln!("{} | {}", pad, src_line);
                            eprintln!(
                                "{} | {}^",
                                pad,
                                " ".repeat(s.start_col.saturating_sub(1) as usize)
                            );
                        }
                        return;
                    }
                }
                eprintln!("[{}] {} (<repl>:{}:{})", sev, colored, s.start_line, s.start_col);
            }
            None => eprintln!("[{}] {} (<repl>)", sev, colored),
        }
    }
}

/// Sintetiza el `main` de la sesión: `(args: String[]) -> Int` con el cuerpo
/// de la línea actual (siempre termina con `return 0;`).
fn synthesize_main(body: Vec<Statement>) -> FunctionDecl {
    let span = Span::new(1, 1, 1, 1);
    let mut statements = body;
    statements.push(Statement::Return(Some(Expression::Literal(Literal {
        kind: LiteralKind::Int(0),
        span: span.clone(),
    }))));
    FunctionDecl {
        name: "main".to_string(),
        params: vec![Parameter {
            name: "args".to_string(),
            type_ann: Some(TypeAnnotation {
                kind: TypeKind::Array(Box::new(TypeAnnotation {
                    kind: TypeKind::String,
                    span: span.clone(),
                })),
                span: span.clone(),
            }),
            default_value: None,
            span: span.clone(),
        }],
        return_type: Some(TypeAnnotation {
            kind: TypeKind::Int,
            span: span.clone(),
        }),
        body: Block { statements, span },
        visibility: Visibility::Private,
        modifiers: vec![],
        span,
        type_params: vec![],
        is_native: false,
    }
}

/// Convierte un `Type` del typeck en una anotación reutilizable para el hoist.
/// Devuelve `None` para tipos no anotables (Any/Null/Unknown/Void/Empty), en
/// cuyo caso el hoist queda sin anotación (el backend cae al type map).
fn annotation_from_type(t: &Type) -> Option<TypeAnnotation> {
    let span = Span::new(1, 1, 1, 1);
    let kind = match t {
        Type::Int | Type::I32 | Type::I64 | Type::I16 | Type::I8 => TypeKind::Int,
        Type::Float | Type::F32 | Type::F64 => TypeKind::Float,
        Type::String => TypeKind::String,
        Type::Bool => TypeKind::Bool,
        Type::Char => TypeKind::Char,
        Type::Any | Type::Unknown | Type::Null | Type::Void | Type::Empty => return None,
        Type::Cmx => TypeKind::Cmx,
        Type::Array(inner) => TypeKind::Array(Box::new(annotation_from_type(inner)?)),
        Type::Tuple(items) => TypeKind::Tuple(
            items
                .iter()
                .map(annotation_from_type)
                .collect::<Option<Vec<_>>>()?,
        ),
        Type::Record(k, v) => TypeKind::Record(
            Box::new(annotation_from_type(k)?),
            Box::new(annotation_from_type(v)?),
        ),
        Type::Shape(fields) => {
            let mut out = Vec::new();
            for (n, t) in fields {
                out.push((n.clone(), annotation_from_type(t)?));
            }
            TypeKind::Shape(out)
        }
        Type::Union(members) => TypeKind::Union(
            members
                .iter()
                .map(annotation_from_type)
                .collect::<Option<Vec<_>>>()?,
        ),
        Type::Fun(params, ret) => TypeKind::Fun(
            params
                .iter()
                .map(annotation_from_type)
                .collect::<Option<Vec<_>>>()?,
            Box::new(annotation_from_type(ret)?),
        ),
        // Los literales se hoistean con su tipo BASE (String/Int/Float/Bool):
        // el valor en runtime puede ser cualquiera del tipo (reassign no aplica
        // a const, pero la concatenación/uso no debe fallar por el literal type).
        Type::Literal(lit) => match lit {
            LitVal::Str(_) => TypeKind::String,
            LitVal::Int(_) => TypeKind::Int,
            LitVal::Float(_) => TypeKind::Float,
            LitVal::Bool(_) => TypeKind::Bool,
        },
        Type::Named(name, args) => TypeKind::Named(
            name.clone(),
            args.iter()
                .map(annotation_from_type)
                .collect::<Option<Vec<_>>>()?,
        ),
    };
    Some(TypeAnnotation { kind, span })
}

/// Transfiere el estado de la instancia anterior a la nueva: globals
/// exportadas `__g_0` (heap_ptr) y `__g_1..`, y la región de heap [1MB, len).
/// Solo se copia el heap a partir de 1MB: bajo ese límite vive el string pool
/// del módulo NUEVO (los data segments re-emiten los strings de la sesión).
fn transfer_state(
    prev: &mut LiveState,
    new_instance: &Instance,
    new_store: &mut Store<HostState>,
    new_memory: &Memory,
) {
    let mut gi = 0u32;
    loop {
        let name = format!("__g_{}", gi);
        let old = prev.instance.get_global(&mut prev.store, &name);
        let new = new_instance.get_global(&mut *new_store, &name);
        match (old, new) {
            (Some(o), Some(n)) => {
                let v = o.get(&mut prev.store);
                if n.set(&mut *new_store, v).is_err() {
                    break;
                }
                gi += 1;
            }
            _ => break,
        }
    }
    let old_data = prev.memory.data(&prev.store);
    if old_data.len() > HEAP_START {
        let new_len = new_memory.data_size(&*new_store);
        if old_data.len() > new_len {
            let pages = (old_data.len() - new_len + 65535) / 65536;
            let _ = new_memory.grow(&mut *new_store, pages as u64);
        }
        let _ = new_memory.write(&mut *new_store, HEAP_START, &old_data[HEAP_START..]);
    }
}

/// Lee un string empaquetado `(ptr<<32)|len` desde la memoria del módulo.
fn read_packed_str(store: &mut Store<HostState>, memory: &Memory, packed: i64) -> String {
    let ptr = (packed >> 32) as usize;
    let len = (packed & 0xffff_ffff) as usize;
    memory
        .data(store)
        .get(ptr..ptr.saturating_add(len))
        .map(|b| String::from_utf8_lossy(b).into_owned())
        .unwrap_or_default()
}

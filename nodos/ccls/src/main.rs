//! ccls — CLI principal de CLS
//! Subcomandos: run, check, build, ast
mod fs;
mod http;

use cls_runtime::{Intrinsics, ModuleResolver, Interpreter, Environment, Value};
use std::env;
use std::fs as std_fs;
use std::process;

/// Crea el resolver para el nodo desktop:
/// core stdlib (math, json) + módulos propios (fs, http) + hook externo
fn make_desktop_resolver() -> ModuleResolver {
    let mut resolver = ModuleResolver::new()
        .with_core_stdlib(); // math + json

    // Módulos del nodo desktop
    resolver.add_internal("fs", fs::module());
    resolver.add_internal("http", http::module());

    // Hook externo: busca archivos .ccls en el filesystem
    resolver.set_external(|path: String, _env: &mut Environment| -> Option<Value> {
        let candidate = format!("{}.ccls", path);
        if let Ok(source) = std_fs::read_to_string(&candidate) {
            // Compilar el módulo externo
            let mut lexer = cls_core::frontend::Lexer::new(&source);
            let tokens = lexer.tokenize().ok()?;
            let mut parser = cls_core::frontend::Parser::new(tokens);
            let module = parser.parse().ok()?;
            // Ejecutarlo en un scope aislado y recolectar exports
            let mut sub_env = Environment::new();
            let mut sub_interpreter = SubInterpreter::new(sub_env);
            sub_interpreter.execute_module(&module).ok()?;
            let exports = sub_interpreter.collect_exports();
            Some(Value::Record(exports))
        } else {
            None
        }
    });

    resolver
}

/// Mini-intérprete para cargar módulos externos
struct SubInterpreter {
    env: Environment,
}

impl SubInterpreter {
    fn new(env: Environment) -> Self { Self { env } }

    fn execute_module(&mut self, module: &cls_core::frontend::ast::Module) -> cls_core::error::ClsResult<()> {
        for stmt in &module.statements {
            self.execute_stmt(stmt)?;
        }
        Ok(())
    }

    fn execute_stmt(&mut self, stmt: &cls_core::frontend::ast::Statement) -> cls_core::error::ClsResult<()> {
        match stmt {
            cls_core::frontend::ast::Statement::VarDecl(v) => {
                let val = self.eval_or_null(&v.value);
                self.env.define(&v.name, val);
            }
            cls_core::frontend::ast::Statement::ConstDecl(v) => {
                let val = self.eval_or_null(&v.value);
                self.env.define(&v.name, val);
            }
            cls_core::frontend::ast::Statement::FunctionDecl(f) => {
                use cls_runtime::value::FunValue;
                self.env.define(&f.name, Value::Fun(FunValue::new_user(
                    &f.name, f.params.clone(), f.body.clone(),
                )));
            }
            _ => {}
        }
        Ok(())
    }

    fn eval_or_null(&mut self, expr: &Option<cls_core::frontend::ast::Expression>) -> Value {
        match expr {
            Some(e) => self.eval_literal_or_string(e),
            None => Value::Null,
        }
    }

    fn eval_literal_or_string(&mut self, expr: &cls_core::frontend::ast::Expression) -> Value {
        use cls_core::frontend::ast::*;
        match expr {
            Expression::Literal(l) => match &l.kind {
                LiteralKind::Int(v) => Value::Int(*v),
                LiteralKind::Float(v) => Value::Float(*v),
                LiteralKind::String(s) => Value::String(s.clone()),
                LiteralKind::Bool(b) => Value::Bool(*b),
                _ => Value::Null,
            },
            _ => Value::Null,
        }
    }

    fn collect_exports(&self) -> std::collections::HashMap<String, Value> {
        // Por ahora, devolvemos todo lo del scope global
        // En el futuro, solo lo marcado con 'export'
        self.env.all()
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("ccls 2.0 — CLS Language Compiler & Runner");
        println!("Uso: ccls <comando> [opciones...]");
        println!("");
        println!("Comandos:");
        println!("  run   <archivo> [args...]    Ejecutar script .ccls o .clsapp");
        println!("  check <archivo>              Verificar tipos");
        println!("  build <archivo> -o <salida>  Compilar a .clsapp");
        println!("  ast   <archivo> --json       Dump AST como JSON");
        return;
    }
    let cmd = &args[1];
    let result = match cmd.as_str() {
        "run" => cmd_run(&args[2..]),
        "check" => cmd_check(&args[2..]),
        "build" => cmd_build(&args[2..]),
        "ast" => cmd_ast(&args[2..]),
        _ => {
            eprintln!("Comando desconocido: {}. Usa 'ccls' sin argumentos para ayuda.", cmd);
            process::exit(1);
        }
    };
    process::exit(result);
}

fn cmd_run(args: &[String]) -> i32 {
    if args.is_empty() {
        eprintln!("Uso: ccls run <archivo> [args...]");
        return 1;
    }
    let path = &args[0];
    let app_args: Vec<String> = args[1..].to_vec();

    // 1. Leer el archivo
    let source = match std_fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error al leer '{}': {}", path, e);
            return 1;
        }
    };

    // 2. Tokenizar
    let mut lexer = cls_core::frontend::Lexer::new(&source);
    let tokens = match lexer.tokenize() {
        Ok(t) => t,
        Err(e) => {
            show_error(&source, &e.to_string(), path);
            return 1;
        }
    };

    // 3. Parsear
    let mut parser = cls_core::frontend::Parser::new(tokens);
    let module = match parser.parse() {
        Ok(m) => m,
        Err(e) => {
            show_error(&source, &e.to_string(), path);
            return 1;
        }
    };

    // 4. Ejecutar (tree-walker)
    let intrinsics = Intrinsics::desktop_defaults(app_args);
    let resolver = make_desktop_resolver();
    let mut interpreter = Interpreter::new(intrinsics, resolver);
    if let Err(e) = interpreter.execute(&module) {
        eprintln!("Error de ejecución: {}", e);
        return 1;
    }

    // 5. Llamar main() y retornar código de salida
    match interpreter.call_main() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("Error en main(): {}", e);
            1
        }
    }
}

fn cmd_check(args: &[String]) -> i32 {
    if args.is_empty() {
        eprintln!("Uso: ccls check <archivo>");
        return 1;
    }

    let source = match std_fs::read_to_string(&args[0]) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error al leer '{}': {}", args[0], e);
            return 1;
        }
    };

    let mut lexer = cls_core::frontend::Lexer::new(&source);
    let tokens = match lexer.tokenize() {
        Ok(t) => t,
        Err(e) => {
            show_error(&source, &e.to_string(), &args[0]);
            return 1;
        }
    };

    let mut parser = cls_core::frontend::Parser::new(tokens);
    let module = match parser.parse() {
        Ok(m) => m,
        Err(e) => {
            show_error(&source, &e.to_string(), &args[0]);
            return 1;
        }
    };

    let config = cls_core::config::types::TypesConfig::default();
    let mut checker = cls_core::middleware::TypeChecker::new(config);
    if let Err(e) = checker.check(&module) {
        eprintln!("Error interno: {}", e);
        return 1;
    };
    let diagnostics = checker.diagnostics();

    if diagnostics.is_empty() {
        println!("No se encontraron errores de tipo.");
        return 0;
    }

    for diag in diagnostics {
        let severity = match diag.severity {
            cls_core::error::diagnostic::Severity::Error => "ERROR",
            cls_core::error::diagnostic::Severity::Warning => "WARN",
            _ => "INFO",
        };
        eprintln!("[{}] {} ({}:{})",
            severity,
            diag.message,
            diag.span.start_line,
            diag.span.start_col
        );
    }
    let errors = diagnostics.iter().filter(|d| matches!(d.severity, cls_core::error::diagnostic::Severity::Error)).count();
    if errors > 0 { 1 } else { 0 }
}

/// Muestra un error con contexto del código fuente
fn show_error(source: &str, error_msg: &str, path: &str) {
    eprintln!("Error en '{}': {}", path, error_msg);

    // Intentar extraer línea y columna del mensaje de error
    // Busca patrones como "línea N, columna M" o "(línea N, columna M)"
    let line_col: Option<(usize, usize)> = error_msg
        .split("línea")
        .nth(1)
        .and_then(|s| {
            let parts: Vec<&str> = s.splitn(2, ',').collect();
            let line = parts.first()?.trim().parse::<usize>().ok()?;
            let col = if parts.len() > 1 {
                parts[1]
                    .split("columna")
                    .nth(1)
                    .and_then(|c| c.trim().trim_matches(|p| p == ')' || p == '(').parse::<usize>().ok())?
            } else {
                1
            };
            Some((line, col))
        });

    if let Some((line, col)) = line_col {
        let source_line = source.lines().nth(line.saturating_sub(1));
        if let Some(src_line) = source_line {
            eprintln!("");
            eprintln!("  {} | {}", line, src_line);
            if col > 1 {
                eprintln!("  {} | {}{}", " ".repeat(line.to_string().len()), " ".repeat(col - 1), "^");
            } else {
                eprintln!("  {} | ^", " ".repeat(line.to_string().len()));
            }
        }
    }
}

fn cmd_build(args: &[String]) -> i32 {
    if args.is_empty() {
        eprintln!("Uso: ccls build <archivo> [-o <salida>]");
        return 1;
    }
    // TODO: implementar compilación a .clsapp
    println!("[ccls] Build no implementado aún");
    0
}

fn cmd_ast(args: &[String]) -> i32 {
    if args.is_empty() {
        eprintln!("Uso: ccls ast <archivo> [--json]");
        return 1;
    }

    let source = match std_fs::read_to_string(&args[0]) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error al leer '{}': {}", args[0], e);
            return 1;
        }
    };

    let mut lexer = cls_core::frontend::Lexer::new(&source);
    let tokens = match lexer.tokenize() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Error de tokenización: {}", e);
            return 1;
        }
    };

    let mut parser = cls_core::frontend::Parser::new(tokens);
    let module = match parser.parse() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Error de parseo: {}", e);
            return 1;
        }
    };

    let use_json = args.iter().any(|a| a == "--json");
    if use_json {
        let backend = cls_core::backend::JsonBackend::new();
        match backend.emit(&module) {
            Ok(json) => println!("{}", json),
            Err(e) => {
                eprintln!("Error al serializar AST: {}", e);
                return 1;
            }
        }
    } else {
        println!("{:#?}", module);
    }
    0
}

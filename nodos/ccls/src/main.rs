//! ccls — CLI principal de CLS
//! Subcomandos: run, check, build, ast
mod fs;
mod http;

use cls_runtime::{Intrinsics, ModuleResolver, Interpreter, Environment, Value, ImportFrame};
use cls_runtime::value::FunValue;
use cls_core::error::ClsResult;
use cls_core::frontend::ast::Visibility;
use std::collections::HashSet;
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
    resolver.set_external(|path: String, _env: &mut Environment| -> ClsResult<Option<Value>> {
        let candidate = format!("{}.ccls", path);
        match std_fs::read_to_string(&candidate) {
            Ok(source) => {
                let module = SubInterpreter::load_module(&source)?;
                Ok(Some(module))
            }
            Err(_) => Ok(None),
        }
    });

    resolver
}

/// Mini-intérprete para cargar módulos externos (archivos .ccls)
struct SubInterpreter {
    env: Environment,
    exports: HashSet<String>,
}

impl SubInterpreter {
    fn load_module(source: &str) -> cls_core::error::ClsResult<Value> {
        let mut lexer = cls_core::frontend::Lexer::new(source);
        let tokens = lexer.tokenize()?;
        let mut parser = cls_core::frontend::Parser::new(tokens);
        let module = parser.parse()?;

        let mut sub = SubInterpreter {
            env: Environment::new(),
            exports: HashSet::new(),
        };

        // Registrar math y json para que el módulo los use
        sub.env.define("math", cls_runtime::stdlib::math::module());
        sub.env.define("json", cls_runtime::stdlib::json::module());

        for stmt in &module.statements {
            sub.execute_stmt(stmt)?;
        }

        // Solo devolver exportados
        let mut entries = std::collections::HashMap::new();
        for name in &sub.exports {
            if let Some(val) = sub.env.get(name) {
                entries.insert(name.clone(), val.clone());
            }
        }

        Ok(Value::Record(entries))
    }

    fn execute_stmt(&mut self, stmt: &cls_core::frontend::ast::Statement) -> cls_core::error::ClsResult<()> {
        match stmt {
            cls_core::frontend::ast::Statement::FunctionDecl(f) => {
                let fun = Value::Fun(FunValue::new_user(
                    &f.name,
                    f.params.clone(),
                    f.body.clone(),
                ));
                self.env.define(&f.name, fun);
                if let Visibility::Export = f.visibility {
                    self.exports.insert(f.name.clone());
                }
            }
            cls_core::frontend::ast::Statement::VarDecl(v) => {
                let val = if let Some(expr) = &v.value {
                    sub_eval_literal(expr)
                } else {
                    Value::Null
                };
                self.env.define(&v.name, val);
                if let Visibility::Export = v.visibility {
                    self.exports.insert(v.name.clone());
                }
            }
            cls_core::frontend::ast::Statement::ConstDecl(v) => {
                let val = if let Some(expr) = &v.value {
                    sub_eval_literal(expr)
                } else {
                    Value::Null
                };
                self.env.define(&v.name, val);
                if let Visibility::Export = v.visibility {
                    self.exports.insert(v.name.clone());
                }
            }
            _ => {}
        }
        Ok(())
    }
}

/// Evalúa literales y expresiones simples para módulos
fn sub_eval_literal(expr: &cls_core::frontend::ast::Expression) -> Value {
    use cls_core::frontend::ast::*;
    match expr {
        Expression::Literal(l) => match &l.kind {
            LiteralKind::Int(v) => Value::Int(*v),
            LiteralKind::Float(v) => Value::Float(*v),
            LiteralKind::String(s) => Value::String(s.clone()),
            LiteralKind::Bool(b) => Value::Bool(*b),
            _ => Value::Null,
        },
        Expression::Array(arr) => {
            Value::Array(arr.elements.iter().map(|e| sub_eval_literal(e)).collect())
        }
        Expression::Record(rec) => {
            Value::Record(rec.entries.iter().map(|(k, e)| (k.clone(), sub_eval_literal(e))).collect())
        }
        Expression::Identifier(name, _) => Value::String(name.clone()),
        Expression::Binary(b) => {
            let l = sub_eval_literal(&b.left);
            let r = sub_eval_literal(&b.right);
            match (&l, &r) {
                (Value::Int(a), Value::Int(b)) => Value::Int(a + b),
                _ => Value::Null,
            }
        }
        Expression::Parenthesized(inner, _) => sub_eval_literal(inner),
        _ => Value::Null,
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
    interpreter.set_source_file(path.to_string());

    if let Err(e) = interpreter.execute(&module) {
        show_runtime_error(&e.to_string(), interpreter.get_import_trace(), path);
        return 1;
    }

    // 5. Llamar main() y retornar código de salida
    match interpreter.call_main() {
        Ok(code) => code,
        Err(e) => {
            show_runtime_error(&e.to_string(), interpreter.get_import_trace(), path);
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
    eprintln!("Error en '{}':", path);
    eprintln!("  {}", error_msg);

    // Si hay source disponible, intentar extraer línea/col y mostrar contexto
    if source.is_empty() { return; }

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

/// Muestra un error con trace numerado usando frames de importación
fn show_runtime_error(error_msg: &str, trace: &[ImportFrame], source_file: &str) {
    // Extraer módulo y error de la cadena
    let file_hint = error_msg.split('\'').nth(1).map(|s| s.trim());
    
    // Error desc: todo desde el último "Error de "
    let error_desc = {
        if let Some(pos) = error_msg.rfind(": Error de ") {
            &error_msg[pos + 2..]
        } else {
            error_msg
        }
    };

    // Extraer línea/col del error (hasta el primer cierre de paréntesis)
    let error_line_col: Option<(usize, usize)> = error_msg
        .split("línea").nth(1)
        .and_then(|s| {
            let end = s.find(')').unwrap_or(s.len());
            let inner = &s[..end];
            let parts: Vec<&str> = inner.splitn(2, ',').collect();
            let line = parts.first()?.trim().parse::<usize>().ok()?;
            let col = if parts.len() > 1 {
                parts[1].split("columna").nth(1)
                    .and_then(|c| c.trim().parse::<usize>().ok())?
            } else { 1 };
            Some((line, col))
        });

    // Determinar archivo fuente: usar el último módulo del trace si hay
    let src_file: String = if let Some(module) = file_hint {
        format!("{}.ccls", module)
    } else if let Some(frame) = trace.last() {
        format!("{}.ccls", frame.module_name)
    } else {
        source_file.to_string()
    };

    // Mostrar cabecera
    if file_hint.is_some() {
        eprintln!("Error al importar módulo '{}':\n", file_hint.unwrap());
    } else {
        eprintln!("Error de ejecución:\n");
    }

    // Paso 1: mostrar cada frame del trace
    for (i, frame) in trace.iter().enumerate() {
        let num = i + 1;
        if let Ok(source) = std_fs::read_to_string(&frame.source_file) {
            let src_line = source.lines()
                .nth(frame.line.saturating_sub(1) as usize)
                .unwrap_or("");
            eprintln!("{}. En {}:{}:{}",
                num, frame.source_file, frame.line, frame.col);
            eprintln!("  {} | {}", frame.line, src_line);
            let pad = " ".repeat(frame.line.to_string().len());
            if frame.col > 1 {
                eprintln!("  {} | {}^^^^^^", pad, " ".repeat(frame.col.saturating_sub(1) as usize));
            } else {
                eprintln!("  {} | ^^^^^^", pad);
            }
        } else {
            eprintln!("{}. import '{}' desde {}:{}:{}",
                num, frame.module_name,
                frame.source_file, frame.line, frame.col);
        }
    }

    // Paso final: el error en el archivo fuente
    let step = trace.len() + 1;
    if let Ok(source) = std_fs::read_to_string(&src_file) {
        if let Some((line, col)) = error_line_col {
            let src_line = source.lines().nth(line.saturating_sub(1)).unwrap_or("");
            let label = if file_hint.is_some() { "[Sintaxis Inválida]" } else { "[Runtime Error]" };
            eprintln!("{}. En {}:{}:{} {}",
                step, src_file, line, col, label);
            eprintln!("  {} | {}", line, src_line);
            let pad = " ".repeat(line.to_string().len());
            if col > 1 {
                eprintln!("  {} | {}{}", pad, " ".repeat(col.saturating_sub(1) as usize), "^");
            } else {
                eprintln!("  {} | ^", pad);
            }
        }
    }
    
    eprintln!("  Error: {}", error_desc.trim());
    eprintln!("");
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

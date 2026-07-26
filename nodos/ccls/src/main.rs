//! ccls — CLI principal de CLS
//! Subcomandos: run, check, build, ast
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("ccls 2.0 — CLS Language Compiler & Runner");
        println!("Uso: ccls <comando> [opciones...]");
        println!("  run   <archivo> [args...]    Ejecutar script .ccls o .clsapp");
        println!("  check <archivo>              Verificar tipos");
        println!("  build <archivo> -o <salida>  Compilar a .clsapp");
        println!("  ast   <archivo> --json       Dump AST");
        return;
    }
    let cmd = &args[1];
    match cmd.as_str() {
        "run" => cmd_run(&args[2..]),
        "check" => cmd_check(&args[2..]),
        "build" => cmd_build(&args[2..]),
        "ast" => cmd_ast(&args[2..]),
        _ => println!("Comando desconocido: {}", cmd),
    }
}

fn cmd_run(args: &[String]) {
    if args.is_empty() {
        println!("Uso: ccls run <archivo> [args...]");
        return;
    }
    let path = &args[0];
    let app_args: Vec<String> = args[1..].to_vec();
    println!("[ccls] Ejecutando: {} con args: {:?}", path, app_args);
    // TODO: cls-core::compile() → cls-runtime::Interpreter::execute()
}

fn cmd_check(args: &[String]) {
    if args.is_empty() {
        println!("Uso: ccls check <archivo>");
        return;
    }
    println!("[ccls] Verificando: {}", args[0]);
}

fn cmd_build(args: &[String]) {
    if args.is_empty() {
        println!("Uso: ccls build <archivo> [-o <salida>]");
        return;
    }
    println!("[ccls] Compilando: {}", args[0]);
}

fn cmd_ast(args: &[String]) {
    if args.is_empty() {
        println!("Uso: ccls ast <archivo> [--json]");
        return;
    }
    println!("[ccls] AST de: {}", args[0]);
}

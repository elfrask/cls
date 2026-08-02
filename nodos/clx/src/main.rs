//! clx — CLS Toolchain
//! CLI de desarrollo para el lenguaje CLS.
mod modules;
mod module_loader;
mod lsp;
mod type_defs;
mod subcommands;

use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_help();
        return;
    }

    let cmd = &args[1];
    let result = match cmd.as_str() {
        // Gestión de proyectos
        "new" => subcommands::new::execute(&args[2..]),
        "init" => { eprintln!("[clx init] No implementado aún"); 0 }

        // Gestión de paquetes
        "add" | "remove" | "rm" | "install" | "i" => {
            subcommands::pkg::execute(cmd, &args[2..])
        }

        // Desarrollo
        "run" => subcommands::run::execute(&args[2..]),
        "check" => subcommands::check::execute(&args[2..]),
        "repl" => subcommands::repl::execute(&args[2..]),

        // Compilación
        "build" => subcommands::build::execute(&args[2..]),

        // Servidor LSP
        "lsp" => subcommands::lsp::execute(&args[2..]),

        // Inspección y tipos
        "ast" => subcommands::ast::execute(&args[2..]),
        "maptype" => subcommands::maptype::execute(&args[2..]),
        "tree" => { eprintln!("[clx tree] No implementado aún"); 0 }

        // Formateo
        "fmt" => { eprintln!("[clx fmt] No implementado aún"); 0 }

        // Globales
        "-v" | "--version" => {
            println!("clx {}", env!("CARGO_PKG_VERSION"));
            println!("CLS Language Compiler & Runtime");
            0
        }
        "-h" | "--help" => { print_help(); 0 }
        "--quiet" => 0,

        _ => {
            eprintln!("Comando desconocido: '{}'. Usa 'clx -h' para ayuda.", cmd);
            1
        }
    };
    process::exit(result);
}

fn print_help() {
    println!("clx {} — CLS Toolchain", env!("CARGO_PKG_VERSION"));
    println!();
    println!("Uso: clx <subcomando> [opciones] [argumentos]");
    println!();
    println!("Gestión de proyectos:");
    println!("  new <nombre> [--lib]        Crear proyecto CLS");
    println!("  init                        Inicializar proyecto (placeholder)");
    println!();
    println!("Gestión de paquetes:");
    println!("  add <paquete> [--dev]       Agregar dependencia a cls.json");
    println!("  remove|rm <paquete>         Quitar dependencia de cls.json");
    println!("  install|i                   Instalar dependencias desde registry");
    println!();
    println!("Desarrollo:");
    println!("  run [archivo] [-- args]     Ejecutar (usa 'entry' de cls.json si no se da archivo)");
    println!("  check [archivo|dir]         Type checking (escanea directorio si no se da archivo)");
    println!("  repl                        REPL interactivo (placeholder)");
    println!();
    println!("Compilación:");
    println!("  build [archivo] -o <out>    Empaquetar a .clsapp (usa 'entry' de cls.json)");
    println!();
    println!("Servidor LSP:");
    println!("  lsp [--tcp addr] [--silent] Language Server (stdin/stdout o TCP)");
    println!();
    println!("Inspección y tipos:");
    println!("  ast <archivo> --json        Dump AST como JSON");
    println!("  maptype [path] -o <dir>     Generar type maps (.type.json)");
    println!("    --watch, -w               Regenerar automáticamente al detectar cambios");
    println!("    (default output: ./.cls-types, preserva estructura de directorios)");
    println!("  tree                        Árbol de dependencias/AST (placeholder)");
    println!();
    println!("Formateo:");
    println!("  fmt                         Formateador de código (placeholder)");
    println!();
    println!("Globales:");
    println!("  -h, --help                  Ayuda");
    println!("  -v, --version               Versión");
    println!("  --quiet                     Silenciar logs");
}

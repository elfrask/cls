//! clx - CLS Toolchain
//! CLI de desarrollo para el lenguaje CLS.
mod modules;
mod lsp;
mod type_defs;
mod subcommands;
mod jit;
mod native;
mod module_index;

use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_help();
        return;
    }

    let cmd = &args[1];
    let result = dispatch(cmd, &args[2..]);
    process::exit(result);
}

/// Despacha un subcomando con sus argumentos. Separado para que `--quiet`
/// pueda preceder al subcomando (`clx --quiet run ...`).
fn dispatch(cmd: &str, args: &[String]) -> i32 {
    match cmd {
        // Gestión de proyectos
        "new" => subcommands::new::execute(args),
        "init" => { eprintln!("'init' no implementado aún (usa 'clx new')"); 1 }

        // Gestión de paquetes
        "add" | "remove" | "rm" | "install" | "i" => {
            subcommands::pkg::execute(cmd, args)
        }

        // Desarrollo
        "run" => subcommands::run::execute(args),
        "check" => subcommands::check::execute(args),
        "repl" => subcommands::repl::execute(args),

        // Compilación
        "build" => subcommands::build::execute(args),

        // Servidor LSP
        "lsp" => subcommands::lsp::execute(args),

        // Inspección y tipos
        "ast" => subcommands::ast::execute(args),
        "maptype" => subcommands::maptype::execute(args),
        "tree" => { eprintln!("'tree' no implementado aún"); 1 }

        // Caché
        "clean" => subcommands::clean::execute(args),

        // Formateo
        "fmt" => { eprintln!("'fmt' no implementado aún"); 1 }

        // Globales (--quiet se acepta ANTES del subcomando: `clx --quiet run ...`)
        "--quiet" if args.is_empty() => { print_help(); 0 }
        "--quiet" => {
            let rest = args;
            dispatch(&rest[0], &rest[1..])
        }
        "-v" | "--version" => {
            println!("clx {}", env!("CARGO_PKG_VERSION"));
            println!("CLS Language Compiler & Runtime");
            0
        }
        "-h" | "--help" => { print_help(); 0 }

        _ => {
            eprintln!("Comando desconocido: '{}'. Usa 'clx -h' para ayuda.", cmd);
            1
        }
    }
}

fn print_help() {
    println!("clx {} - CLS Toolchain", env!("CARGO_PKG_VERSION"));
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
    println!("  run [archivo] [-- args]     Ejecutar con el JIT (default; usa 'entry' de cls.json si no se da archivo)");
    println!("    --ast-walker              Ejecutar con el tree-walker DEPRECADO (solo referencia)");
    println!("  check [archivo|dir]         Type checking (escanea directorio si no se da archivo)");
    println!("  repl                        REPL interactivo JIT (estado persistente entre líneas)");
    println!();
    println!("Compilación:");
    println!("  build [archivo] -o <out>    Empaquetar a .clsapp (usa 'entry' de cls.json)");
    println!();
    println!("Servidor LSP:");
    println!("  lsp [--silent]              Language Server (stdin/stdout)");
    println!();
    println!("Inspección y tipos:");
    println!("  ast <archivo> --json        Dump AST como JSON");
    println!("  maptype [path] -o <dir>     Generar type maps (.type.json)");
    println!("    --watch, -w               Regenerar automáticamente al detectar cambios");
    println!("    (default output: ./.cls-types, preserva estructura de directorios)");
    println!("  tree                        Árbol de dependencias/AST (placeholder)");
    println!();
    println!("Caché:");
    println!("  clean [--all]               Limpiar caché de compilación (~/.cache/cls)");
    println!();
    println!("Formateo:");
    println!("  fmt                         Formateador de código (placeholder)");
    println!();
    println!("Globales:");
    println!("  -h, --help                  Ayuda");
    println!("  -v, --version               Versión");
    println!("  --quiet                     Silenciar logs (antes del subcomando: 'clx --quiet run')");
}

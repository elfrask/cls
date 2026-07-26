//! cpkg — CLS Package Manager
//! Subcomandos: new, install, build, publish, run
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("cpkg 2.0 — CLS Package Manager");
        println!("Uso: cpkg <comando> [opciones...]");
        println!("  new      <nombre>          Crear proyecto");
        println!("  install  [paquete]         Instalar dependencias");
        println!("  build                     Compilar proyecto");
        println!("  publish                   Publicar paquete");
        println!("  run       [args...]        Ejecutar proyecto");
        return;
    }
    let cmd = &args[1];
    match cmd.as_str() {
        "new" => {
            let name = args.get(2).map(|s| s.as_str()).unwrap_or("mi-app");
            println!("[cpkg] Creando proyecto: {}", name);
        }
        "install" => println!("[cpkg] Instalando dependencias..."),
        "build" => println!("[cpkg] Compilando proyecto..."),
        "publish" => println!("[cpkg] Publicando paquete..."),
        "run" => println!("[cpkg] Ejecutando proyecto..."),
        _ => println!("Comando desconocido: {}", cmd),
    }
}

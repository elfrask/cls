//! ecls — Ejecutor directo de .clsapp
//! Carga cls-runtime y ejecuta una app empaquetada
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("ecls 2.0 — CLS Application Executor");
        println!("Uso: ecls <archivo.clsapp> [args...]");
        return;
    }
    let path = &args[1];
    let app_args: Vec<String> = args[2..].to_vec();
    println!("[ecls] Cargando: {}", path);
    println!("[ecls] Args: {:?}", app_args);
    // TODO: cargar .clsapp, extraer main.wasm, ejecutar en runtime
}

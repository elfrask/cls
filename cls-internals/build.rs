//! Build: compila `cls-internals-wasm` (sub-crate) a `wasm32-unknown-unknown`
//! y embebe el `internals.wasm` resultante con `include_bytes!` en el crate host.

use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let wasm_manifest = manifest_dir.join("wasm").join("Cargo.toml");
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let target_dir = out_dir.join("wasm-target");

    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());

    // El sub-crate `wasm/` se compila como cdylib standalone con `__cls_alloc`
    // indefinido a proposito (el linker de fusion lo resuelve al `__alloc` del
    // modulo CLS). rust-lld de toolchains nuevos rechaza simbolos indefinidos
    // por defecto (undefined symbol: __cls_alloc), asi que se permite el import
    // explicitamente. Se pasa por --config (no RUSTFLAGS env: el cargo padre le
    // pasa CARGO_ENCODED_RUSTFLAGS al build script, que anula RUSTFLAGS; no
    // config file: la discovery depende del CWD del build script).
    let config = "target.wasm32-unknown-unknown.rustflags=[\"-C\",\"link-arg=--allow-undefined\"]";
    let status = Command::new(&cargo)
        .args(["build", "--release", "--manifest-path"])
        .arg(&wasm_manifest)
        .args(["--target", "wasm32-unknown-unknown"])
        .args(["--config", config])
        .env("CARGO_TARGET_DIR", &target_dir)
        .status()
        .expect("no se pudo ejecutar cargo para cls-internals-wasm");
    if !status.success() {
        panic!(
            "cargo build de cls-internals-wasm fallo (target wasm32-unknown-unknown instalado? \
             usar: rustup target add wasm32-unknown-unknown)"
        );
    }

    let release = target_dir.join("wasm32-unknown-unknown").join("release");
    let wasm = if release.join("cls_internals_wasm.wasm").exists() {
        release.join("cls_internals_wasm.wasm")
    } else {
        // fallback: cualquier *.wasm del dir (naming de cargo puede variar)
        let found = std::fs::read_dir(&release)
            .expect("dir release del sub-crate")
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .find(|p| p.extension().map_or(false, |x| x == "wasm"));
        match found {
            Some(p) => p,
            None => panic!("no se encontro el .wasm compilado de cls-internals-wasm"),
        }
    };

    let size = std::fs::metadata(&wasm).map(|m| m.len()).unwrap_or(0);
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=wasm/src");
    println!("cargo:rerun-if-changed=wasm/Cargo.toml");
    std::fs::copy(&wasm, out_dir.join("internals.wasm")).expect("copiar internals.wasm");
    println!("cargo:info=internals.wasm: {} bytes", size);
}

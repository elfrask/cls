//! Tests de la Fase 1: exports tipados de `export function` + modo librería.

use cls_core::backend::wasm::{WasmBackend, WasmBackendOptions};
use cls_core::config::types::TypesConfig;
use cls_core::error::ClsResult;
use cls_core::frontend::ast::Target;
use cls_core::frontend::{Lexer, Parser};
use cls_core::middleware::TypeChecker;

fn emit(source: &str, opts: WasmBackendOptions) -> ClsResult<Vec<u8>> {
    let tokens = Lexer::new(source).tokenize()?;
    let module = Parser::new(tokens).parse()?;
    let mut checker = TypeChecker::new(TypesConfig {
        check: true,
        strict: true,
        no_implicit_any: true,
        null_safety: true,
    });
    checker.check_with_prelude(&module, &[])?;
    let types = checker.type_map();
    let backend = WasmBackend::with_options(types, Target::host(), opts);
    backend.emit(&module)
}

const DEFAULT: WasmBackendOptions = WasmBackendOptions {
    exceptions: true,
    require_main: true,
    intrinsics: Vec::new(),
};

const LIBRARY: WasmBackendOptions = WasmBackendOptions {
    exceptions: true,
    require_main: false,
    intrinsics: Vec::new(),
};

#[test]
fn exports_tipados_y_seccion_custom() {
    let src = r#"
        export function suma(a: int, b: int) -> int { return a + b; }
        export function saludo(nombre: String) -> String { return "hola " + nombre; }
        export function ratio(a: float, b: float) -> float { return a / b; }
    "#;
    let bytes = emit(src, LIBRARY).expect("emit modo librería");
    let wat = wasmprinter::print_bytes(&bytes).expect("wat");
    // Exports WASM de las funciones exportadas (firma B5: __capturas primero).
    assert!(wat.contains("(export \"suma\""), "falta export suma:\n{wat}");
    assert!(wat.contains("(export \"saludo\""), "falta export saludo");
    assert!(wat.contains("(export \"ratio\""), "falta export ratio");
    // main sintetizado (modo librería).
    assert!(wat.contains("(export \"main\""), "falta main sintetizado");
    // Sección custom con las firmas tipadas.
    let text = String::from_utf8_lossy(&bytes);
    assert!(text.contains("clx:exports"), "falta custom section");
    assert!(text.contains("\"params\":[0,0]"), "suma: params int,int");
    assert!(text.contains("\"ret\":4"), "saludo: ret String (4)");
    assert!(text.contains("\"name\":\"ratio\""), "ratio presente");
}

#[test]
fn modo_app_requiere_main() {
    let src = "export function f(a: int) -> int { return a; };";
    assert!(emit(src, DEFAULT).is_err(), "sin main en modo app debe fallar");
    let bytes = emit(src, LIBRARY).expect("modo librería sin main debe emitir");
    assert!(!bytes.is_empty());
    let wat = wasmprinter::print_bytes(&bytes).unwrap();
    assert!(wat.contains("(export \"f\""));
    assert!(wat.contains("(export \"main\""));
}

#[test]
fn default_sin_exports_no_custom_section() {
    // Un módulo normal (sin export function) NO debe llevar la custom section
    // (los bytes del camino default quedan idénticos al baseline).
    let src = r#"
        function main(args: String[]) -> int {
            return 0;
        };
    "#;
    let bytes = emit(src, DEFAULT).expect("emit default");
    let text = String::from_utf8_lossy(&bytes);
    assert!(!text.contains("clx:exports"), "no debe haber custom section");
}

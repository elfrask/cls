//! Diagnóstico temporal: kinds de los exports con Record (F3).

use cls_core::backend::wasm::{WasmBackend, WasmBackendOptions};
use cls_core::config::types::TypesConfig;
use cls_core::frontend::ast::Target;
use cls_core::frontend::{Lexer, Parser};
use cls_core::middleware::TypeChecker;

#[test]
fn record_kind_debug() {
    let src = r#"
        export function datos() -> Record<String, String> { var d: Record<String, String> = {a: "1"}; return d; }
    "#;
    let tokens = Lexer::new(src).tokenize().unwrap();
    let module = Parser::new(tokens).parse().unwrap();
    let mut checker = TypeChecker::new(TypesConfig {
        check: true,
        strict: true,
        no_implicit_any: true,
        null_safety: true,
    });
    checker.check_with_prelude(&module, &[]).unwrap();
    let backend = WasmBackend::with_options(
        checker.type_map(),
        Target::host(),
        WasmBackendOptions { require_main: false, ..WasmBackendOptions::default() },
    );
    let bytes = backend.emit(&module).unwrap();
    let exports = cls_jit::parse_clx_exports(&bytes);
    println!("exports: {:?}", exports);
    assert!(!exports.is_empty());
    let e = &exports[0];
    assert_eq!(e.ret, 6, "Record ret debe ser kind 6, es {}", e.ret);
}

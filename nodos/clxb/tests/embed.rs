//! Tests de integración del nodo clxb (F2a): compile, call, run_main, eval,
//! captura de print y SDK de nodo (intrinsics + resolver).

use clsb::{ClsEngine, ClsValue};
use cls_core::middleware::types::Type;
use std::sync::{Arc, Mutex};

/// Sink de output que captura el print del script.
struct Capture {
    buf: Mutex<String>,
}

impl cls_jit::OutputSink for Capture {
    fn write(&self, s: &str) {
        self.buf.lock().unwrap().push_str(s);
    }
    fn end_line(&self) {
        self.buf.lock().unwrap().push('\n');
    }
}

#[test]
fn call_exports_scalares() {
    let engine = ClsEngine::new();
    let src = r#"
        export function suma(a: int, b: int) -> int { return a + b; }
        export function ratio(a: float, b: float) -> float { return a / b; }
        export function mayor(a: int, b: int) -> bool { return a > b; }
        export function primera(s: String) -> String { if (s.startsWith("x")) { return "empieza"; } return "no"; }
    "#;
    let mut module = engine.compile_source(src, "test", std::path::Path::new(".")).unwrap();

    let r = module.call("suma", &[ClsValue::Int(20), ClsValue::Int(22)]).unwrap();
    assert_eq!(r, ClsValue::Int(42));

    let r = module.call("ratio", &[ClsValue::Float(10.0), ClsValue::Float(4.0)]).unwrap();
    assert_eq!(r, ClsValue::Float(2.5));

    let r = module.call("mayor", &[ClsValue::Int(5), ClsValue::Int(3)]).unwrap();
    assert_eq!(r, ClsValue::Bool(true));

    let r = module.call("primera", &[ClsValue::Str("xyz".into())]).unwrap();
    assert_eq!(r, ClsValue::Str("empieza".into()));
}

#[test]
fn call_exports_arrays_records() {
    let engine = ClsEngine::new();
    let src = r#"
        export function total(nums: int[]) -> int {
            var t: int = 0;
            for each n in (nums) { t += n; }
            return t;
        }
        export function nombres(xs: String[]) -> int {
            return xs.length;
        }
        export function ciudad(d: Record<String, String>) -> String {
            return d["ciudad"];
        }
        export function par(x: int) -> int[] {
            return [x, x * 2];
        }
    "#;
    let mut module = engine.compile_source(src, "test", std::path::Path::new(".")).unwrap();

    let r = module
        .call(
            "total",
            &[ClsValue::Array(vec![ClsValue::Int(1), ClsValue::Int(2), ClsValue::Int(3)])],
        )
        .unwrap();
    assert_eq!(r, ClsValue::Int(6));

    let r = module
        .call(
            "nombres",
            &[ClsValue::Array(vec![ClsValue::Str("a".into()), ClsValue::Str("b".into())])],
        )
        .unwrap();
    assert_eq!(r, ClsValue::Int(2));

    let r = module
        .call(
            "ciudad",
            &[ClsValue::Record(vec![
                ("ciudad".to_string(), ClsValue::Str("Lima".into())),
                ("pais".to_string(), ClsValue::Str("Peru".into())),
            ])],
        )
        .unwrap();
    assert_eq!(r, ClsValue::Str("Lima".into()));

    // Array como retorno (elems int).
    let r = module.call("par", &[ClsValue::Int(7)]).unwrap();
    assert_eq!(r, ClsValue::Array(vec![ClsValue::Int(7), ClsValue::Int(14)]));
}

#[test]
fn run_main_y_eval() {
    let engine = ClsEngine::new();
    let src = r#"
        function main(args: String[]) -> int {
            print("hola " + args[0]);
            return 3;
        };
    "#;
    let mut module = engine.compile_source(src, "main", std::path::Path::new(".")).unwrap();
    let code = module.run_main(&["mundo".to_string()]).unwrap();
    assert_eq!(code, 3);

    // eval: compila + llama al primer export (sin args).
    let r = engine
        .eval("export function siete() -> int { return 7; };")
        .unwrap();
    assert_eq!(r, ClsValue::Int(7));
}

#[test]
fn output_capturado() {
    let cap = Arc::new(Capture { buf: Mutex::new(String::new()) });
    let mut engine = ClsEngine::new();
    engine.set_output(cap.clone());
    let src = r#"
        function main(args: String[]) -> int {
            print("a", 1, 2.5);
            print("segunda");
            return 0;
        };
    "#;
    let mut module = engine.compile_source(src, "main", std::path::Path::new(".")).unwrap();
    module.run_main(&[]).unwrap();
    let out = cap.buf.lock().unwrap().clone();
    assert!(out.contains("a 1 2.5"), "salida: {:?}", out);
    assert!(out.contains("segunda"), "salida: {:?}", out);
}

#[test]
fn sdk_intrinsics() {
    let mut engine = ClsEngine::new();
    // Función host del nodo: `duplicar(int) -> int`.
    struct Handler;
    impl cls_jit::HostCallHandler for Handler {
        fn call(
            &self,
            _id: u32,
            args: &[cls_jit::HostCallArg],
        ) -> Result<cls_jit::HostCallResult, String> {
            Ok(cls_jit::HostCallResult {
                tag: 0,
                bits: args[0].bits * 2,
                text: None,
            })
        }
    }
    engine.register_host_function("duplicar", vec![Type::Int], Type::Int, Arc::new(Handler));

    let src = r#"
        export function usa() -> int {
            return duplicar(21);
        }
    "#;
    let mut module = engine.compile_source(src, "test", std::path::Path::new(".")).unwrap();
    let r = module.call("usa", &[]).unwrap();
    assert_eq!(r, ClsValue::Int(42));
}

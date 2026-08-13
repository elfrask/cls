//! Tests del canal `env.host_call` (Fase 1): funciones host del nodo llamadas
//! desde CLS como `nombre(args)`, con marshalling de int/string/float.

use cls_core::middleware::types::{HostIntrinsic, Type};
use cls_jit::{HostCallArg, HostCallHandler, HostCallResult, JitContext, RuntimeKind};
use std::sync::Arc;

struct TestHandler;

impl HostCallHandler for TestHandler {
    fn call(&self, id: u32, args: &[HostCallArg]) -> Result<HostCallResult, String> {
        match id {
            // duplicar(int) -> int
            1 => {
                let v = args[0].bits;
                Ok(HostCallResult {
                    tag: 0,
                    bits: v * 2,
                    text: None,
                })
            }
            // saludar(String) -> String
            2 => Ok(HostCallResult {
                tag: 4,
                bits: 0,
                text: Some(format!(
                    "hola {}",
                    args[0].text.as_deref().unwrap_or("?")
                )),
            }),
            // doble_f(float) -> float
            3 => {
                let v = f64::from_bits(args[0].bits as u64) * 2.0;
                Ok(HostCallResult {
                    tag: 1,
                    bits: v.to_bits() as i64,
                    text: None,
                })
            }
            _ => Err(format!("id desconocido: {}", id)),
        }
    }
}

fn intrinsics() -> Vec<HostIntrinsic> {
    vec![
        HostIntrinsic {
            id: 1,
            name: "duplicar".into(),
            params: vec![Type::Int],
            ret: Type::Int,
        },
        HostIntrinsic {
            id: 2,
            name: "saludar".into(),
            params: vec![Type::String],
            ret: Type::String,
        },
        HostIntrinsic {
            id: 3,
            name: "doble_f".into(),
            params: vec![Type::Float],
            ret: Type::Float,
        },
    ]
}

fn run_with(runtime: RuntimeKind) -> i32 {
    let src = r#"
        function main(args: String[]) -> int {
            var x: int = duplicar(21);
            var s: String = saludar("mundo");
            var f: float = doble_f(2.5);
            if (x != 42) { return 10; }
            if (s.length != 10) { return 11; }
            if (!s.startsWith("hola")) { return 12; }
            if (f != 5.0) { return 13; }
            return 0;
        };
    "#;
    let dir = std::env::temp_dir().join("cls-hostcall-test");
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("main.clsx");
    std::fs::write(&file, src).unwrap();
    let file = file.to_string_lossy().to_string();

    struct NoNative;
    impl cls_runtime::ffi::NativeBackend for NoNative {
        fn call_function(
            &self,
            _lib: &str,
            _sym: &str,
            _args: &[cls_runtime::Value],
            _arg_types: &[cls_runtime::ffi::NativeType],
            _ret: cls_runtime::ffi::NativeType,
        ) -> cls_core::error::ClsResult<cls_runtime::Value> {
            Err(cls_core::error::ClsError::RuntimeError("no native".into()))
        }
        fn get_variable(
            &self,
            _lib: &str,
            _sym: &str,
            _ty: cls_runtime::ffi::NativeType,
        ) -> cls_core::error::ClsResult<cls_runtime::Value> {
            Err(cls_core::error::ClsError::RuntimeError("no native".into()))
        }
        fn set_variable(
            &self,
            _lib: &str,
            _sym: &str,
            _val: cls_runtime::ffi::NativeType,
            _ty: &cls_runtime::Value,
        ) -> cls_core::error::ClsResult<()> {
            Err(cls_core::error::ClsError::RuntimeError("no native".into()))
        }
    }

    let intrinsics = intrinsics();
    let ctx = JitContext {
        native_backend: Arc::new(NoNative),
        module_index: None,
        host_intrinsics: &intrinsics,
        host_call_handler: Some(Arc::new(TestHandler)),
    };
    cls_jit::run_jit_with(&file, &[], None, &ctx, runtime)
}

#[test]
fn host_call_wasmtime() {
    let code = run_with(RuntimeKind::Wasmtime);
    assert_eq!(code, 0, "exit code wasmtime (10/11/12 = intrinsic roto)");
}

#[test]
fn host_call_wasmi() {
    let code = run_with(RuntimeKind::Wasmi);
    assert_eq!(code, 0, "exit code wasmi (10/11/12 = intrinsic roto)");
}

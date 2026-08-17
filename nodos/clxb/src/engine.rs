//! Motor de embedding de clxb: compila CLS, instancia el WASM y expone
//! `run_main` / `call` / `eval` con marshalling de valores.

use std::path::Path;
use std::sync::Arc;

use cls_core::middleware::types::{HostIntrinsic, Type};
use cls_jit::host::HostCtx;
use cls_jit::state::HostState;
use wasmtime::{Caller, Engine, Linker, Memory, Module, Store, TypedFunc};

use crate::value::{read_value, write_value, ClsValue, StoreCtx};
use crate::ClsError;

/// Sink de salida compartido: `set_output` afecta también a los módulos ya
/// instanciados (el HostState guarda el wrapper, que delega al sink actual).
#[derive(Default)]
pub struct SharedSink {
    current: std::sync::RwLock<Option<Arc<dyn cls_jit::OutputSink>>>,
}

impl cls_jit::OutputSink for SharedSink {
    fn write(&self, s: &str) {
        if let Some(sink) = self.current.read().unwrap().as_ref() {
            sink.write(s);
        } else {
            print!("{}", s);
        }
    }
    fn end_line(&self) {
        if let Some(sink) = self.current.read().unwrap().as_ref() {
            sink.end_line();
        } else {
            println!();
        }
    }
}

/// Despacha `host_call(id, ...)` al handler registrado para ese id. Permite
/// registrar varias host functions en el mismo engine (B3: antes solo la
/// última funcionaba porque el engine guardaba un handler global).
struct DispatchHostCall {
    handlers: std::sync::Mutex<std::collections::HashMap<u32, Arc<dyn cls_jit::HostCallHandler>>>,
}

impl DispatchHostCall {
    fn new() -> Self {
        Self {
            handlers: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }

    fn insert(&self, id: u32, h: Arc<dyn cls_jit::HostCallHandler>) {
        self.handlers.lock().unwrap().insert(id, h);
    }
}

impl cls_jit::HostCallHandler for DispatchHostCall {
    fn call(&self, id: u32, args: &[cls_jit::HostCallArg]) -> Result<cls_jit::HostCallResult, String> {
        let handlers = self.handlers.lock().unwrap();
        match handlers.get(&id) {
            Some(h) => h.call(id, args),
            None => Err(format!("host function id {} no registrada", id)),
        }
    }
}

/// Engine de embedding: configuración (intrinsics, output, resolver) + compila.
pub struct ClsEngine {
    native: Arc<dyn cls_runtime::ffi::NativeBackend>,
    intrinsics: Vec<HostIntrinsic>,
    host_call: Arc<DispatchHostCall>,
    resolver: Option<Arc<dyn cls_jit::ModuleSourceResolver>>,
    output: Arc<SharedSink>,
    next_id: u32,
    /// Sandbox: oculta los módulos del nodo desktop (fs/http/os/path/process/
    /// time/random) - solo core. Por defecto `true` (seguro para embeds).
    sandbox: bool,
}
impl Default for ClsEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ClsEngine {
    pub fn new() -> Self {
        Self {
            native: Arc::new(NoNative),
            intrinsics: Vec::new(),
            host_call: Arc::new(DispatchHostCall::new()),
            resolver: None,
            output: Arc::new(SharedSink::default()),
            next_id: 1,
            sandbox: true,
        }
    }

    /// Controla el sandbox: `true` oculta fs/http/os/path/process/time/random
    /// (por defecto). `false` los expone (embedding con nodo desktop completo).
    pub fn set_sandbox(&mut self, sandbox: bool) {
        self.sandbox = sandbox;
    }

    pub fn set_output(&mut self, sink: Arc<dyn cls_jit::OutputSink>) {
        *self.output.current.write().unwrap() = Some(sink);
    }

    pub fn set_module_resolver(&mut self, r: Arc<dyn cls_jit::ModuleSourceResolver>) {
        self.resolver = Some(r);
    }

    /// Registra una función host del nodo: devuelve el id (para el handler).
    pub fn register_host_function(
        &mut self,
        name: &str,
        params: Vec<Type>,
        ret: Type,
        handler: Arc<dyn cls_jit::HostCallHandler>,
    ) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.intrinsics.push(HostIntrinsic {
            id,
            name: name.to_string(),
            params,
            ret,
        });
        self.host_call.insert(id, handler);
        id
    }

    fn jit_ctx(&self) -> cls_jit::JitContext<'_> {
        cls_jit::JitContext {
            native_backend: self.native.clone(),
            module_index: None,
            host_intrinsics: &self.intrinsics,
            host_call_handler: Some(self.host_call.clone()),
            module_source_resolver: self.resolver.as_deref(),
            output: Some(self.output.clone()),
        }
    }

    /// Compila un programa CLS (source en memoria) y lo instancia.
    pub fn compile_source(&self, source: &str, name: &str, base_dir: &Path) -> Result<ClsModule, ClsError> {
        let opts = cls_jit::CompileOptions {
            runtime: cls_jit::RuntimeKind::Wasmtime,
            require_main: false,
            target: None,
            trace_calls: true,
        };
        let compiled = cls_jit::compile_source(source, name, base_dir, &self.jit_ctx(), &opts)?;
        let module = ClsModule::instantiate(compiled, &self.jit_ctx(), self.sandbox)?;
        Ok(module)
    }

    /// Compila un programa CLS desde un archivo y lo instancia.
    pub fn compile_file(&self, path: &str) -> Result<ClsModule, ClsError> {
        let opts = cls_jit::CompileOptions {
            runtime: cls_jit::RuntimeKind::Wasmtime,
            require_main: false,
            target: None,
            trace_calls: true,
        };
        let compiled = cls_jit::compile_file(path, &self.jit_ctx(), &opts)?;
        let module = ClsModule::instantiate(compiled, &self.jit_ctx(), self.sandbox)?;
        Ok(module)
    }

    /// Ejecución inmediata: compila el source y llama al primer export (o a
    /// `main` si no hay exports) con 0 args.
    pub fn eval(&self, source: &str) -> Result<ClsValue, ClsError> {
        let mut module = self.compile_source(source, "eval", Path::new("."))?;
        if let Some(sig) = module.exports.first() {
            let name = sig.name.clone();
            module.call(&name, &[])
        } else {
            module.run_main(&[])?;
            Ok(ClsValue::Null)
        }
    }
}

/// Backend nativo dummy: clxb no soporta `extension` por ahora.
struct NoNative;

impl cls_runtime::ffi::NativeBackend for NoNative {
    fn call_function(
        &self,
        lib: &str,
        sym: &str,
        _args: &[cls_runtime::Value],
        _arg_types: &[cls_runtime::ffi::NativeType],
        _ret: cls_runtime::ffi::NativeType,
    ) -> cls_core::error::ClsResult<cls_runtime::Value> {
        Err(cls_core::error::ClsError::RuntimeError(format!(
            "extension nativa '{}.{}' no soportada por el binding (clxb)",
            lib, sym
        )))
    }
    fn get_variable(
        &self,
        _lib: &str,
        _sym: &str,
        _ty: cls_runtime::ffi::NativeType,
    ) -> cls_core::error::ClsResult<cls_runtime::Value> {
        Err(cls_core::error::ClsError::RuntimeError(
            "variables nativas no soportadas por el binding (clxb)".into(),
        ))
    }
    fn set_variable(
        &self,
        _lib: &str,
        _sym: &str,
        _ty: cls_runtime::ffi::NativeType,
        _val: &cls_runtime::Value,
    ) -> cls_core::error::ClsResult<()> {
        Err(cls_core::error::ClsError::RuntimeError(
            "variables nativas no soportadas por el binding (clxb)".into(),
        ))
    }
}

/// Módulo CLS compilado e instanciado: lista para `run_main`/`call`.
pub struct ClsModule {
    exports: Vec<cls_jit::ExportSig>,
    store: Store<HostState>,
    instance: wasmtime::Instance,
    memory: Memory,
    alloc: TypedFunc<i64, i64>,
}

impl ClsModule {
    fn instantiate(
        compiled: cls_jit::CompiledModule,
        ctx: &cls_jit::JitContext,
        sandbox: bool,
    ) -> Result<Self, ClsError> {
        let mut config = wasmtime::Config::new();
        config.wasm_exceptions(true);
        let engine = Engine::new(&config)
            .map_err(|e| ClsError::new(format!("error creando engine: {}", e)))?;
        let module = Module::new(&engine, &compiled.wasm)
            .map_err(|e| ClsError::new(format!("módulo WASM inválido: {}", e)))?;

        let mut store = Store::new(
            &engine,
            HostState {
                first_in_line: true,
                source_file: "clsb".to_string(),
                modules: Vec::new(),
                string_caps: std::collections::HashMap::new(),
                call_stack: Vec::new(),
                pending_call_site: None,
                simple_fn_names: std::collections::HashMap::new(),
                host_call: ctx.host_call_handler.clone(),
                output: ctx.output.clone(),
                app_args: Vec::new(),
            },
        );
        let mut linker = Linker::new(&engine);
        cls_jit::wasmtime_rt::register_host_functions_opt(&mut linker, true, sandbox)
            .map_err(ClsError::new)?;
        if sandbox {
            // Sandbox: los imports no registrados (fs/http/os/path/process/
            // time/random y exit/trap) se definen como traps -> el script recibe
            // error de runtime si intenta acceder (instanciación OK).
            linker
                .define_unknown_imports_as_traps(&module)
                .map_err(|e| ClsError::new(format!("sandbox: {}", e)))?;
        } else {
            // `exit`/`trap` del binding: NO matan el proceso del embedder. Lanzan
            // un trap WASM con mensaje codificado que `run_main`/`call` traducen.
            linker
                .func_wrap("env", "exit", |_: Caller<'_, HostState>, code: i64| -> Result<(), wasmtime::Error> {
                    Err(wasmtime::Error::new(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("__clsb_exit__:{}", code),
                    )))
                })
                .map_err(|e| e.to_string())
                .map_err(ClsError::new)?;
            linker
                .func_wrap(
                    "env",
                    "trap",
                    |mut c: Caller<'_, HostState>, m: i64, s: i64| -> Result<(), wasmtime::Error> {
                        let msg = cls_jit::host::host_trap_message(&mut c, m, s);
                        Err(wasmtime::Error::new(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!("__clsb_trap__:{}", msg),
                        )))
                    },
                )
                .map_err(|e| e.to_string())
                .map_err(ClsError::new)?;
        }
        // Extensiones nativas: el backend del engine (NoNative -> error claro).
        cls_jit::wasmtime_rt::register_native_hosts(&mut linker, &module, ctx.native_backend.clone())
            .map_err(ClsError::new)?;
        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| ClsError::new(format!("instanciación falló: {}", e)))?;
        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| ClsError::new("export 'memory' no disponible".to_string()))?;
        let alloc = instance
            .get_typed_func::<i64, i64>(&mut store, "alloc")
            .map_err(|e| ClsError::new(format!("export 'alloc' no disponible: {}", e)))?;

        Ok(Self {
            exports: compiled.exports,
            store,
            instance,
            memory,
            alloc,
        })
    }

    /// Firmas de los exports (`export function`).
    pub fn exports(&self) -> &[cls_jit::ExportSig] {
        &self.exports
    }

    /// Ejecuta `main(args)` y devuelve el exit code.
    pub fn run_main(&mut self, args: &[String]) -> Result<i64, ClsError> {
        let main = self
            .instance
            .get_typed_func::<i64, i64>(&mut self.store, "main")
            .map_err(|e| ClsError::new(format!("export 'main' no disponible: {}", e)))?;
        let args_ptr = {
            let mut ctx = StoreCtx {
                store: &mut self.store,
                memory: self.memory,
                alloc: &mut self.alloc,
            };
            write_args(&mut ctx, args)
        };
        match main.call(&mut self.store, args_ptr) {
            Ok(code) => Ok(code),
            Err(e) => match exit_code_from_trap(&e) {
                Some(code) => Ok(code),
                None => Err(self.runtime_error(String::new(), None, e.to_string())),
            },
        }
    }

    /// Llama a una función exportada con valores CLS.
    pub fn call(&mut self, name: &str, args: &[ClsValue]) -> Result<ClsValue, ClsError> {
        let sig = self
            .exports
            .iter()
            .find(|e| e.name == name)
            .cloned()
            .ok_or_else(|| ClsError::new(format!("no existe el export '{}'", name)))?;
        if args.len() != sig.params.len() {
            return Err(ClsError::new(format!(
                "'{}' espera {} argumentos, se pasaron {}",
                name,
                sig.params.len(),
                args.len()
            )));
        }
        let func = self
            .instance
            .get_func(&mut self.store, name)
            .ok_or_else(|| ClsError::new(format!("export '{}' no disponible", name)))?;

        // Escribir los args (escalares directos; string/array/record a memoria).
        let mut params: Vec<wasmtime::Val> = Vec::with_capacity(args.len());
        {
            let mut ctx = StoreCtx {
                store: &mut self.store,
                memory: self.memory,
                alloc: &mut self.alloc,
            };
            // El frame CLS espera `__capturas` (0) como primer param (B5).
            params.push(wasmtime::Val::I64(0));
            for (i, arg) in args.iter().enumerate() {
                let desc = sig.param_types.get(i);
                let bits = write_value(&mut ctx, arg, desc)?;
                let v = match sig.params.get(i).copied().unwrap_or(8) {
                    1 => wasmtime::Val::F64(bits as u64),
                    2 | 3 => wasmtime::Val::I32(bits as i32),
                    _ => wasmtime::Val::I64(bits),
                };
                params.push(v);
            }
        }

        // Resultados según el tipo del retorno (void -> ninguno).
        let mut results = if sig.ret == 9 {
            Vec::new()
        } else {
            vec![wasmtime::Val::I64(0)]
        };
        match func.call(&mut self.store, &params, &mut results) {
            Ok(()) => {
                if sig.ret == 9 {
                    return Ok(ClsValue::Null);
                }
                // Decodificar el resultado según el tipo del retorno (float y
                // bool/char viajan como Val::F64/F32 / Val::I32).
                let raw = match sig.ret {
                    1 => results[0]
                        .f64()
                        .map(|f| f.to_bits() as i64)
                        .unwrap_or(0),
                    2 | 3 => results[0].i32().map(|v| v as i64).unwrap_or(0),
                    _ => results[0].i64().unwrap_or(0),
                };
                let mut ctx = StoreCtx {
                    store: &mut self.store,
                    memory: self.memory,
                    alloc: &mut self.alloc,
                };
                read_value(&mut ctx, raw, sig.ret, sig.ret_type.as_ref()).map_err(ClsError::new)
            }
            Err(e) => match exit_code_from_trap(&e) {
                Some(_) => Err(ClsError::new("exit() llamado dentro de una función exportada (no aplicable)".to_string())),
                None => Err(self.runtime_error(String::new(), None, e.to_string())),
            },
        }
    }

    fn runtime_error(&self, msg: String, span: Option<cls_core::error::Span>, trap: String) -> ClsError {
        let text = cls_jit::engine::build_error_string(
            msg,
            span,
            self.store.data().call_stack.clone(),
            self.store.data().pending_call_site,
            trap,
            "clsb",
            &[],
        );
        ClsError::new(text)
    }
}

/// Escribe los args como Array<String> en memoria y devuelve el ptr.
fn write_args(ctx: &mut StoreCtx, args: &[String]) -> i64 {
    let n = args.len() as i64;
    let array_ptr = ctx.alloc(n * 8 + 16);
    ctx.write_i64(array_ptr as usize, n);
    ctx.write_i64(array_ptr as usize + 8, n);
    for (i, arg) in args.iter().enumerate() {
        let sptr = ctx.write_str(arg);
        ctx.write_i64(array_ptr as usize + 16 + i * 8, sptr);
    }
    array_ptr
}

/// Extrae el código de `exit(n)` del trap interceptado por el binding
/// (`__clsb_exit__:<code>`). `None` si el trap no es un exit. Busca en el
/// error y sus causas raíz (wasmtime envuelve el io::Error del host).
fn exit_code_from_trap(e: &wasmtime::Error) -> Option<i64> {
    let mut msgs: Vec<String> = Vec::new();
    msgs.push(e.to_string());
    msgs.push(e.root_cause().to_string());
    for msg in msgs {
        if let Some(rest) = msg.split("__clsb_exit__:").nth(1) {
            if let Ok(code) = rest.trim().parse::<i64>() {
                return Some(code);
            }
        }
    }
    None
}

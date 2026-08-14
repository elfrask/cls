//! API de compilación sin ejecución (para bindings/embedding).
//!
//! [`compile_source`]/[`compile_file`] producen un [`CompiledModule`] (WASM +
//! firmas tipadas de los exports) sin correr nada. El nodo de bindings
//! (`clxb`) instancia el módulo en su propio runtime y llama `main`/exports
//! con marshalling de valores.

use cls_core::config::types::TypesConfig;
use cls_core::error::{ClsError, ClsResult};
use cls_core::frontend::ast::Module as ClsModule;
use cls_core::middleware::TypeChecker;
use std::path::Path;

use crate::flatten::flatten_imports;
use crate::resolve::load_import_modules_hooked;
use crate::{JitContext, RuntimeKind};

/// Descriptor recursivo de tipo de la sección custom `clx:exports` (`pt`/`rt`):
/// permite al host decodificar arrays/records anidados (la memoria del runtime
/// no guarda el tipo del elemento de un array).
#[derive(Debug, Clone, PartialEq)]
pub struct TypeDesc {
    pub kind: i64,
    /// Tipo del elemento (`Array`).
    pub elem: Option<Box<TypeDesc>>,
    /// Tipo del valor (`Record<K, V>` homogéneo).
    pub value: Option<Box<TypeDesc>>,
    /// Tipos por clave (`Shape` heterogéneo).
    pub shape: Vec<(String, TypeDesc)>,
}

impl TypeDesc {
    /// Descriptor sin información de tipos anidados.
    pub fn none() -> Self {
        Self { kind: -1, elem: None, value: None, shape: Vec::new() }
    }

    pub fn simple(kind: i64) -> Self {
        Self { kind, elem: None, value: None, shape: Vec::new() }
    }
}

/// Firma tipada de un export (`export function`), del canal del host.
/// `kind` = `cls_kind_code`: 0=int 1=float 2=bool 3=char 4=string 5=array
/// 6=record/shape 7=tuple 8=otro-i64 9=void 10=cmx 11=función 12=null.
/// `*_elem` = kind del elemento para los arrays (-1 si no es array).
/// `param_types`/`ret_type` = descs recursivos (`pt`/`rt`); vacíos si el
/// binario no los lleva (firma plana antigua).
#[derive(Debug, Clone, PartialEq)]
pub struct ExportSig {
    pub name: String,
    pub params: Vec<i64>,
    pub param_elems: Vec<i64>,
    pub ret: i64,
    pub ret_elem: i64,
    pub param_types: Vec<TypeDesc>,
    pub ret_type: Option<TypeDesc>,
}

/// Módulo CLS compilado a WASM (sin ejecutar).
#[derive(Debug, Clone)]
pub struct CompiledModule {
    pub wasm: Vec<u8>,
    pub exports: Vec<ExportSig>,
}

/// Opciones de compilación.
#[derive(Debug, Clone)]
pub struct CompileOptions {
    pub runtime: RuntimeKind,
    /// `false` = modo librería (se sintetiza main no-op si no hay).
    pub require_main: bool,
    /// Target para la directiva `when` (None = host).
    pub target: Option<String>,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            runtime: RuntimeKind::Wasmtime,
            require_main: true,
            target: None,
        }
    }
}

/// Compila un programa CLS (source en memoria) a WASM.
///
/// - `name`: nombre del módulo (para los errores).
/// - `base_dir`: directorio base para resolver `import "mod"`.
pub fn compile_source(
    source: &str,
    name: &str,
    base_dir: &Path,
    ctx: &JitContext,
    opts: &CompileOptions,
) -> ClsResult<CompiledModule> {
    let tokens = cls_core::frontend::Lexer::new(source).tokenize()?;
    let module = cls_core::frontend::Parser::new(tokens).parse()?;
    compile_module(&module, source, name, base_dir, ctx, opts)
}

/// Compila un programa CLS desde un archivo.
pub fn compile_file(
    path: &str,
    ctx: &JitContext,
    opts: &CompileOptions,
) -> ClsResult<CompiledModule> {
    let source = std::fs::read_to_string(path)
        .map_err(|e| ClsError::CompileError(format!("No se pudo leer '{}': {}", path, e)))?;
    let base_dir = Path::new(path)
        .parent()
        .unwrap_or(Path::new("."))
        .to_path_buf();
    compile_source(&source, path, &base_dir, ctx, opts)
}

/// Pipeline común: typeck (strict) + imports + span_shift + flatten + emit.
fn compile_module(
    module: &ClsModule,
    source: &str,
    name: &str,
    base_dir: &Path,
    ctx: &JitContext,
    opts: &CompileOptions,
) -> ClsResult<CompiledModule> {
    let types_config = TypesConfig {
        check: true,
        strict: true,
        no_implicit_any: true,
        null_safety: true,
    };
    let mut checker = TypeChecker::new(types_config);
    checker.register_host_intrinsics(ctx.host_intrinsics);

    // Resolver imports (errores con los candidatos probados + hook del nodo).
    let mut seen = std::collections::HashSet::new();
    let mut imports: Vec<(String, String, ClsModule)> = Vec::new();
    let manifest = cls_core::config::ModuleManifest::find_in_dir(base_dir);
    load_import_modules_hooked(
        module,
        base_dir,
        &mut seen,
        &mut imports,
        manifest.as_ref(),
        ctx.module_source_resolver,
    )?;

    // Desplazar spans de los módulos importados (offset único por módulo).
    for (i, (_path, _src, m)) in imports.iter_mut().enumerate() {
        let offset = 100000u32 * (i as u32 + 1);
        cls_core::frontend::span_shift::shift_module(m, offset);
    }
    let prelude: Vec<(String, ClsModule)> = imports
        .iter()
        .map(|(p, _, m)| (p.clone(), m.clone()))
        .collect();
    checker.check_with_prelude(module, &prelude)?;

    // Diagnostics de tipo → error estructurado (el primero).
    for diag in checker.diagnostics() {
        if matches!(diag.severity, cls_core::error::diagnostic::Severity::Error) {
            return Err(ClsError::compile_at(&diag.message, &diag.span));
        }
    }

    // Target + flatten + emit.
    let target = match &opts.target {
        Some(tt) => cls_core::frontend::ast::Target::parse(tt),
        None => cls_core::frontend::ast::Target::host(),
    };
    let merged = flatten_imports(module, &prelude);
    let backend_opts = cls_core::backend::wasm::WasmBackendOptions {
        exceptions: matches!(opts.runtime, RuntimeKind::Wasmtime),
        require_main: opts.require_main,
        intrinsics: ctx.host_intrinsics.to_vec(),
    };
    let backend =
        cls_core::backend::wasm::WasmBackend::with_options(checker.type_map(), target, backend_opts);
    let wasm = backend.emit(&merged).map_err(|e| match e {
        ClsError::SyntaxErrorAt(m, s) => ClsError::SyntaxErrorAt(m, s),
        other => {
            // Los errores del emisor no tienen span del archivo real: anclar al
            // inicio para que el formateador muestre el archivo.
            ClsError::compile_at(&other.to_string(), &cls_core::error::Span::new(1, 1, 1, 1))
        }
    })?;

    let exports = parse_clx_exports(&wasm);
    let _ = name;
    let _ = source;
    Ok(CompiledModule { wasm, exports })
}

// ── Sección custom `clx:exports` ────────────────────────────────────────────

fn leb128(b: &[u8], pos: usize) -> (u64, usize) {
    let mut result: u64 = 0;
    let mut shift = 0u32;
    let mut p = pos;
    while p < b.len() {
        let byte = b[p];
        p += 1;
        result |= ((byte & 0x7f) as u64) << shift;
        if byte & 0x80 == 0 {
            break;
        }
        shift += 7;
    }
    (result, p - pos)
}

/// Busca la sección custom `clx:exports` en el binario WASM y la devuelve.
fn find_custom_section<'a>(wasm: &'a [u8], name: &str) -> Option<&'a [u8]> {
    if wasm.len() < 8 {
        return None;
    }
    let mut pos = 8usize;
    while pos < wasm.len() {
        let id = wasm[pos];
        pos += 1;
        let (size, n) = leb128(wasm, pos);
        pos += n;
        let end = pos + size as usize;
        if end > wasm.len() {
            return None;
        }
        if id == 0 {
            let (nlen, n2) = leb128(wasm, pos);
            let name_start = pos + n2;
            let name_end = name_start + nlen as usize;
            if name_end <= end && &wasm[name_start..name_end] == name.as_bytes() {
                return Some(&wasm[name_end..end]);
            }
        }
        pos = end;
    }
    None
}

/// Parsea un descriptor recursivo `{"k": kind, "e"?, "v"?, "s"?}`.
fn parse_type_desc(v: &serde_json::Value) -> TypeDesc {
    let kind = v.get("k").and_then(|x| x.as_i64()).unwrap_or(-1);
    let elem = v.get("e").map(|e| Box::new(parse_type_desc(e)));
    let value = v.get("v").map(|x| Box::new(parse_type_desc(x)));
    let shape = v
        .get("s")
        .and_then(|s| s.as_object())
        .map(|m| m.iter().map(|(k, d)| (k.clone(), parse_type_desc(d))).collect())
        .unwrap_or_default();
    TypeDesc { kind, elem, value, shape }
}

/// Parsea las firmas tipadas de los exports desde la sección custom.
pub fn parse_clx_exports(wasm: &[u8]) -> Vec<ExportSig> {
    let Some(data) = find_custom_section(wasm, "clx:exports") else {
        return Vec::new();
    };
    let Ok(entries): Result<serde_json::Value, _> = serde_json::from_slice(data) else {
        return Vec::new();
    };
    let Some(arr) = entries.as_array() else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|e| {
            let name = e.get("name")?.as_str()?.to_string();
            let params = e
                .get("params")?
                .as_array()?
                .iter()
                .map(|p| p.as_i64().unwrap_or(8))
                .collect::<Vec<i64>>();
            let param_elems = e
                .get("pe")
                .map(|v| {
                    v.as_array()
                        .map(|a| a.iter().map(|p| p.as_i64().unwrap_or(-1)).collect())
                        .unwrap_or_else(|| vec![-1; params.len()])
                })
                .unwrap_or_else(|| vec![-1; params.len()]);
            let ret = e.get("ret")?.as_i64().unwrap_or(8);
            let ret_elem = e.get("re").and_then(|v| v.as_i64()).unwrap_or(-1);
            // Descs recursivos (`pt`/`rt`); fallback: reconstruir los planos
            // (`pe`/`re`) para binarios viejos sin `pt`/`rt`.
            let param_types = e
                .get("pt")
                .and_then(|v| v.as_array())
                .map(|a| a.iter().map(parse_type_desc).collect())
                .unwrap_or_else(|| {
                    params
                        .iter()
                        .zip(param_elems.iter())
                        .map(|(k, ek)| {
                            if *ek != -1 {
                                TypeDesc { kind: *k, elem: Some(Box::new(TypeDesc::simple(*ek))), value: None, shape: Vec::new() }
                            } else {
                                TypeDesc::simple(*k)
                            }
                        })
                        .collect()
                });
            let ret_type = e
                .get("rt")
                .map(parse_type_desc)
                .or_else(|| {
                    if ret_elem != -1 {
                        Some(TypeDesc { kind: ret, elem: Some(Box::new(TypeDesc::simple(ret_elem))), value: None, shape: Vec::new() })
                    } else {
                        Some(TypeDesc::simple(ret))
                    }
                });
            Some(ExportSig {
                name,
                params,
                param_elems,
                ret,
                ret_elem,
                param_types,
                ret_type,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_section_roundtrip() {
        // Emitir un módulo con exports y leer la sección custom de vuelta.
        let src = r#"
            export function suma(a: int, b: int) -> int { return a + b; }
            export function saludo(n: String) -> String { return n; }
        "#;
        struct NoNative;
        impl cls_runtime::ffi::NativeBackend for NoNative {
            fn call_function(
                &self,
                _l: &str,
                _s: &str,
                _a: &[cls_runtime::Value],
                _t: &[cls_runtime::ffi::NativeType],
                _r: cls_runtime::ffi::NativeType,
            ) -> ClsResult<cls_runtime::Value> {
                Err(ClsError::RuntimeError("no native".into()))
            }
            fn get_variable(
                &self,
                _l: &str,
                _s: &str,
                _t: cls_runtime::ffi::NativeType,
            ) -> ClsResult<cls_runtime::Value> {
                Err(ClsError::RuntimeError("no native".into()))
            }
            fn set_variable(
                &self,
                _l: &str,
                _s: &str,
                _t: cls_runtime::ffi::NativeType,
                _v: &cls_runtime::Value,
            ) -> ClsResult<()> {
                Err(ClsError::RuntimeError("no native".into()))
            }
        }
        let ctx = JitContext {
            native_backend: std::sync::Arc::new(NoNative),
            module_index: None,
            host_intrinsics: &[],
            host_call_handler: None,
            module_source_resolver: None,
            output: None,
        };
        let opts = CompileOptions {
            runtime: RuntimeKind::Wasmtime,
            require_main: false,
            target: None,
        };
        let cm = compile_source(
            src,
            "test",
            Path::new("."),
            &ctx,
            &opts,
        )
        .expect("compila");
        assert_eq!(cm.exports.len(), 2, "dos exports");
        let suma = cm.exports.iter().find(|e| e.name == "suma").unwrap();
        assert_eq!(suma.params, vec![0, 0], "int,int");
        assert_eq!(suma.ret, 0, "ret int");
        let saludo = cm.exports.iter().find(|e| e.name == "saludo").unwrap();
        assert_eq!(saludo.params, vec![4], "String");
        assert_eq!(saludo.ret, 4, "ret String");
    }
}

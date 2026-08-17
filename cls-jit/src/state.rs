//! Estado del host compartido por todos los runtimes (wasmtime, wasmi).

/// Estado del host: separador de argumentos en `print`, archivo fuente y el
/// mapa `offset -> (archivo, source)` de los módulos importados (para de-shiftear
/// spans desplazados en errores de runtime).
#[derive(Default)]
pub struct HostState {
    pub first_in_line: bool,
    pub source_file: String,
    /// Módulos importados: (offset de línea, archivo, source) - offset = 100000*(i+1).
    pub modules: Vec<(u32, String, String)>,
    /// Capacidad alocada por cada string dinámica (ptr del buffer -> bytes).
    /// Permite a `str_concat` reutilizar el buffer (crecimiento amortizado) en
    /// vez de alocar exacto cada vez (O(n²) en loops de concatenación).
    pub string_caps: std::collections::HashMap<i64, i64>,
    /// Shadow call stack: nombres y spans de las funciones CLS en ejecución.
    /// Lo alimentan los hosts `fn_enter`/`fn_exit` emitidos por el backend.
    pub call_stack: Vec<(String, cls_core::error::diagnostic::Span)>,
    /// Call site pendiente: lo setea `fn_call_site` (emitido por el backend en el
    /// punto de llamada, justo antes del `Call`). El `fn_enter` del callee lo
    /// consume como span del frame (el frame apunta al CALL SITE del llamador).
    pub pending_call_site: Option<cls_core::error::diagnostic::Span>,
    /// Nombres de las funciones simples (capturas=0) usadas como valor:
    /// tabla_idx -> nombre. Lo llena `fn_handle` (handle par, sin handle en
    /// memoria) y lo consulta `fn_to_string` para imprimir `<function X>`.
    pub simple_fn_names: std::collections::HashMap<i64, String>,
    /// Handler del canal `env.host_call` (intrinsics del nodo), si el nodo
    /// registró uno.
    pub host_call: Option<std::sync::Arc<dyn crate::host::HostCallHandler>>,
    /// Destino de `print` (si el nodo registró uno; si no, stdout).
    pub output: Option<std::sync::Arc<dyn crate::host::OutputSink>>,
    /// Args de la aplicación (los que el nodo inyectó tras `--`). Los usa
    /// `process.args()`.
    pub app_args: Vec<String>,
}

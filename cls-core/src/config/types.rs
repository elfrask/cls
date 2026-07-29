use serde::{Deserialize, Serialize};

/// Configuración de tipos del compilador
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TypesConfig {
    /// Habilitar chequeo de tipos (true = híbrido, false = dinámico)
    #[serde(default = "default_true")]
    pub check: bool,

    /// Tipado estricto (solo si check = true)
    #[serde(default)]
    pub strict: bool,

    /// Prohibir tipos no inferidos
    #[serde(default)]
    pub no_implicit_any: bool,

    /// Evitar null pointer exceptions
    #[serde(default = "default_true")]
    pub null_safety: bool,
}

fn default_true() -> bool {
    true
}

impl Default for TypesConfig {
    fn default() -> Self {
        Self {
            check: true,
            strict: false,
            no_implicit_any: false,
            null_safety: true,
        }
    }
}

/// Configuración del compilador
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompilerConfig {
    /// Arquitectura objetivo: "x86_64", "arm64", "wasm", "bytecode"
    #[serde(default = "default_arch")]
    pub target_architecture: String,

    /// Nivel de optimización: "O0", "O1", "O2", "O3", "Os"
    #[serde(default = "default_opt")]
    pub optimization_level: String,

    /// Configuración de tipos
    #[serde(default)]
    pub types: TypesConfig,

    /// Features habilitados
    #[serde(default)]
    pub features: FeaturesConfig,

    /// Configuración de warnings
    #[serde(default)]
    pub warnings: WarningsConfig,

    /// Generar source maps para debugging
    #[serde(default = "default_true")]
    pub source_maps: bool,
}

fn default_arch() -> String {
    "wasm".to_string()
}

fn default_opt() -> String {
    "O2".to_string()
}

impl Default for CompilerConfig {
    fn default() -> Self {
        Self {
            target_architecture: default_arch(),
            optimization_level: default_opt(),
            types: TypesConfig::default(),
            features: FeaturesConfig::default(),
            warnings: WarningsConfig::default(),
            source_maps: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FeaturesConfig {
    #[serde(default)]
    pub async_: bool,

    #[serde(default)]
    pub macros: bool,

    #[serde(default)]
    pub experimental: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WarningsConfig {
    #[serde(default = "default_warn")]
    pub unused_variables: String,

    #[serde(default = "default_warn")]
    pub dead_code: String,

    #[serde(default)]
    pub treat_warnings_as_errors: bool,
}

fn default_warn() -> String {
    "warn".to_string()
}

impl Default for WarningsConfig {
    fn default() -> Self {
        Self {
            unused_variables: default_warn(),
            dead_code: default_warn(),
            treat_warnings_as_errors: false,
        }
    }
}

/// Configuración del intérprete/runtime
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InterpreterConfig {
    /// Habilitar optimización en interpretación
    #[serde(default)]
    pub optimization: bool,

    /// Modo de interpretación: "pure-ast" | "jit"
    #[serde(default = "default_mode")]
    pub mode: String,

    /// Configuración de memoria del runtime
    #[serde(default)]
    pub runtime: RuntimeMemoryConfig,

    /// Configuración de sandbox
    #[serde(default)]
    pub sandbox: SandboxConfig,
}

fn default_mode() -> String {
    "pure-ast".to_string()
}

impl Default for InterpreterConfig {
    fn default() -> Self {
        Self {
            optimization: false,
            mode: default_mode(),
            runtime: RuntimeMemoryConfig::default(),
            sandbox: SandboxConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeMemoryConfig {
    #[serde(default = "default_mem")]
    pub memory_limit: String,

    #[serde(default = "default_stack")]
    pub stack_size: String,

    #[serde(default)]
    pub gc: GcConfig,
}

fn default_mem() -> String {
    "512MB".to_string()
}

fn default_stack() -> String {
    "8MB".to_string()
}

impl Default for RuntimeMemoryConfig {
    fn default() -> Self {
        Self {
            memory_limit: default_mem(),
            stack_size: default_stack(),
            gc: GcConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GcConfig {
    #[serde(default)]
    pub enabled: bool,

    /// "active" (GC en runtime) | "compiled" (GC quemado en wasm)
    #[serde(default = "default_gc")]
    pub strategy: String,

    #[serde(default = "default_threshold")]
    pub threshold: String,
}

fn default_gc() -> String {
    "compiled".to_string()
}

fn default_threshold() -> String {
    "64MB".to_string()
}

impl Default for GcConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            strategy: default_gc(),
            threshold: default_threshold(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SandboxConfig {
    #[serde(default)]
    pub allow_fs: bool,

    #[serde(default)]
    pub allow_net: bool,

    #[serde(default = "default_timeout")]
    pub max_execution_time: u64,
}

fn default_timeout() -> u64 {
    5000
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            allow_fs: false,
            allow_net: false,
            max_execution_time: default_timeout(),
        }
    }
}

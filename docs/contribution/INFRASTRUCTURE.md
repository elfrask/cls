# Infraestructura de Errores

## ClsError

**Archivo:** `cls-core/src/error/mod.rs`

```rust
pub enum ClsError {
    CompileError(String),
    RuntimeError(String),
    TypeError(String),
    SyntaxError(String),
    IoError(std::io::Error),
    ConfigError(String),
}
```

### Métodos:
- `syntax_at(msg, span)` — error sintáctico con posición
- `with_span(msg, span)` — error con posición

## Diagnostic

**Archivo:** `cls-core/src/error/diagnostic.rs`

```rust
pub struct Diagnostic {
    pub message: String,
    pub span: Span,
    pub severity: Severity,
    pub source_file: Option<String>,
}
```

### Severity:
```rust
pub enum Severity { Error, Warning, Info, Hint }
```

## Span

```rust
pub struct Span {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}
```

## Formato de error en terminal

Los errores se muestran con:
- Archivo y mensaje
- Línea y columna
- Código fuente de la línea
- Cursor `^` apuntando al error

Ejemplo:
```
Error en 'app.ccls': Error de sintaxis: Esperaba ';' (línea 3, columna 12)

  3 |     print("hello"   ← falta ')'
    |            ^
```

---

# Configuración

## ModuleManifest

**Archivo:** `cls-core/src/config/manifest.rs`

```rust
pub struct ModuleManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub authors: Vec<String>,
    pub license: String,
    pub project: ProjectConfig,
    pub compiler: CompilerConfig,
    pub interpreter: InterpreterConfig,
    pub dependencies: HashMap<String, String>,
    pub dev_dependencies: HashMap<String, String>,
}
```

## ProjectConfig

```rust
pub struct ProjectConfig {
    pub entry: String,       // "src/main.ccls"
    pub source_dir: String,  // "src"
    pub out_dir: String,     // "dist"
    pub target: String,      // "executable" | "library" | "dynamic-lib"
}
```

## CompilerConfig

```rust
pub struct CompilerConfig {
    pub target_architecture: String,  // "wasm" | "x86_64" | "arm64"
    pub optimization_level: String,   // "O0".."O3"
    pub types: TypesConfig,
    pub features: FeaturesConfig,
    pub warnings: WarningsConfig,
    pub source_maps: bool,
}
```

## InterpreterConfig

```rust
pub struct InterpreterConfig {
    pub optimization: bool,
    pub mode: String,  // "pure-ast" | "jit"
    pub runtime: RuntimeMemoryConfig,
    pub sandbox: SandboxConfig,
}
```

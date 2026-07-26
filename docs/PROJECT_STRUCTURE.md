# CLS 2.0 — Estructura del proyecto

```
cls/                                    # Workspace Rust
├── Cargo.toml                          # Workspace raíz (resolver = "2")
│
├── cls-core/                           # LIB: Compilador
│   ├── Cargo.toml                      # + serde, serde_json, thiserror
│   └── src/
│       ├── lib.rs                      # API pública
│       ├── config/
│       │   ├── mod.rs
│       │   ├── manifest.rs             # ModuleManifest (module.clsconfig)
│       │   └── types.rs                # TypesConfig, CompilerConfig, InterpreterConfig
│       ├── frontend/
│       │   ├── mod.rs
│       │   ├── lexer.rs                # Lexer (carácter por carácter)
│       │   ├── token.rs                # Token, Keyword, Operator, Symbol, CmxToken
│       │   ├── parser.rs               # Parser recursive descent
│       │   └── ast.rs                  # AST nodes (70+ types)
│       ├── middleware/
│       │   ├── mod.rs
│       │   ├── types.rs                # Type enum (sistema de tipos)
│       │   ├── typeck.rs               # TypeChecker configurable
│       │   ├── resolver.rs             # NameResolver (scopes anidados)
│       │   └── optimizer.rs            # Optimizer (AST transforms)
│       ├── backend/
│       │   ├── mod.rs
│       │   ├── json.rs                 # AST → JSON
│       │   ├── wasm.rs                 # AST → WASM (placeholder)
│       │   └── visitor.rs              # AstVisitor trait
│       └── error/
│           ├── mod.rs                  # ClsError enum, ClsResult<T>
│           └── diagnostic.rs           # Span, Diagnostic, Severity
│
├── cls-runtime/                        # LIB: Motor de ejecución
│   ├── Cargo.toml                      # + cls-core
│   └── src/
│       ├── lib.rs                      # API pública
│       ├── error.rs                    # Re-export de ClsError/ClsResult
│       ├── value.rs                    # Value enum (tipos runtime)
│       ├── environment.rs              # Environment (scopes anidados)
│       ├── interpreter.rs              # Tree-walker interpreter
│       ├── gc.rs                       # GarbageCollector (placeholder)
│       ├── sandbox.rs                  # Sandbox (FS, net, time limits)
│       ├── modules.rs                  # ModuleManager (.clsapp)
│       ├── host_api.rs                 # HostApi (funciones del host)
│       ├── ffi.rs                      # exports extern "C"
│       └── stdlib/
│           ├── mod.rs
│           ├── io.rs                   # print, input
│           ├── math.rs                 # abs, sqrt, etc.
│           ├── fs.rs                   # readFile, writeFile
│           ├── json.rs                 # parseJson, stringifyJson
│           └── http.rs                 # httpGet, httpPost
│
├── nodos/                              # EJECUTABLES (todos Rust nativo)
│   ├── ccls/                           # CLI principal
│   │   ├── Cargo.toml                  # + cls-core, cls-runtime
│   │   └── src/main.rs                 # Subcomandos: run, check, build, ast
│   ├── ccls-repl/                      # REPL interactivo
│   │   ├── Cargo.toml
│   │   └── src/main.rs
│   ├── cpkg/                           # Gestor de paquetes
│   │   ├── Cargo.toml
│   │   └── src/main.rs                 # Subcomandos: new, install, build, publish, run
│   └── ecls/                           # Ejecutor de .clsapp
│       ├── Cargo.toml
│       └── src/main.rs
│
├── host-libs/                          # WRAPPERS (NO ejecutables)
│   ├── python/                         # pip install cls-runtime (~100 líneas)
│   │   └── cls/
│   │       ├── __init__.py             # Carga cls-runtime.wasm
│   │       └── runtime.wasm            # Binario empaquetado
│   ├── js/                             # npm install cls-runtime
│   └── go/                             # go get cls-runtime
│
├── docs/                               # Documentación
├── agent-context/                      # Planes y contexto para agentes
├── main.ccls                           # Ejemplo de sintaxis
├── test.ccls                           # Archivo de prueba
└── .gitignore
```

---

## Nodos (ejecutables)

| Nodo | Propósito | Dependencias |
|------|-----------|-------------|
| `ccls` | CLI principal: `run`, `check`, `build`, `ast` | cls-core, cls-runtime |
| `ccls-repl` | REPL interactivo | cls-core, cls-runtime |
| `cpkg` | Gestor de paquetes: `new`, `install`, `build`, `publish` | cls-core, cls-runtime |
| `ecls` | Ejecutor directo de `.clsapp` | cls-runtime |

### Comandos CLI

```bash
ccls run <archivo> [args...]       # Ejecutar .ccls o .clsapp (args pasan al programa)
ccls check <archivo>               # Type checking
ccls build <archivo> -o <salida>   # Compilar a .clsapp/.clslib
ccls ast <archivo> --json           # Dump AST

ccls-repl                          # REPL interactivo

cpkg new <nombre>                  # Crear proyecto
cpkg install [paquete]             # Instalar dependencias
cpkg build                         # Compilar proyecto
cpkg publish                       # Publicar paquete
cpkg run [args...]                 # Ejecutar proyecto

ecls <archivo.clsapp> [args...]    # Ejecutar app empaquetada
```

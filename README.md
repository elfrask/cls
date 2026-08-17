# CLS 2.0 - Modular WASM-first Programming Language

**Version**: 2.0.0 · **License**: MIT  
[GitHub](https://github.com/frask/cls) · WASM-first · Multiplatform

CLS is a programming language designed for **modular, safe, cross-platform development**. It compiles to WASM (future), has an integrated type checker, a JSX-like native syntax (CMX), a VFS abstraction with sandbox support, and a full LSP server for editor integration.

```
.clsx -> Lexer -> Parser -> AST -> Tree-walker (Interpreter)
                              -> JSON backend (AST dump)
                              -> WASM codegen (future -> .clbin)
```

---

## Quick Start

```bash
# Create a project
clx new my-app
cd my-app

# Run it
clx run

# Check types
clx check

# Build to .clsapp
clx build

# Generate type maps (for editor autocomplete)
clx maptype . -o .cls-types --watch
```

### Hello World

```clx
function main(args: String[]) -> int {
    print("Hello, World!");
    return 0;
};
```

---

## Installation

### Requirements
- Rust 2021 edition
- Cargo

### From source
```bash
git clone https://github.com/frask/cls
cd cls
cargo build -p clx    # Development CLI
cargo build -p clxr   # Runtime executor
```

### Verify
```bash
clx --version
clx -h
```

---

## Language Guide

### Comments

```
# This is a comment
```

### Variables

```clx
var x = 42;                      # Mutable
const PI = 3.14159;              # Immutable
let name: String = "CLS";        # With type annotation
```

### Functions

```clx
function add(a: int, b: int) -> int {
    return a + b;
};

# Void function (no return type)
function greet(name: String) {
    print("Hello,", name);
};

# Async function
async function fetch(url: String) -> String {
    var result = await http.get(url);
    return result;
};
```

### Control Flow

```clx
# If / elif / else
if (x > 0) {
    print("positive");
} elif (x < 0) {
    print("negative");
} else {
    print("zero");
};

# While
while (x > 0) {
    x = x - 1;
};

# Loop with break/continue
loop {
    if (done) { break; };
    if (skip) { continue; };
};

# C-style for
for (i = 0; i < 10; i++) {
    print(i);
};

# For each
for each item in (items) {
    print(item);
};
for each item and idx in (items) {
    print(idx, ":", item);
};
```

### Data Structures

```clx
# Array
var arr = [1, 2, 3];
print(arr[0]);          # -> 1
print(len(arr));        # -> 3

# Record / Object
var obj = { "name": "CLS", "version": 2 };
print(obj["name"]);     # -> "CLS"
```

### String Interpolation

```clx
var name = "CLS";
var msg = "Hello, $name!";
var calc = "${a + b} items";
```

### Arrow Functions

```clx
var double = (x) -> x * 2;
var add    = (a, b) -> int { a + b };
var greet  = () -> { print("hi") };
```

### Import System

```clx
import "math" as m;
from "json" import parse, stringify;
from "fs" import readFile as read;
include "lib";
```

### Types

```clx
var a: int = 42;
var b: float = 3.14;
var c: String = "hello";
var d: bool = true;
var e: Array<int> = [1, 2, 3];
var f: Record<String, int> = { "a": 1, "b": 2 };
```

### Structures

```clx
structure Person {
    name: String,
    age: int
};

var p = Person("Alice", 30);
print(p.name);          # -> "Alice"
p.name = "Bob";         # mutation
```

### Classes

```clx
class Counter {
    export var count: int = 0;

    function increment() {
        me.count = me.count + 1;
    };
};
```

### Interfaces

```clx
interface Printable {
    print(): void
};
```

### CMX (Native JSX)

```clx
# Lowercase tag -> CmxValue
var el = <button label="Click" onClick={handleClick} />;
print(el.tag);          # -> "button"
print(el.props.label);  # -> "Click"

# Uppercase tag -> reference lookup
<App title="Hello" />;  # calls function App({title: "Hello"})

# With children
var comp = (
    <Container>
        <Header />
        <Body>{ content }</Body>
    </Container>
);
```

### Error Handling

```clx
# Throw
if (x < 0) { throw("x must be positive"); };

# Try / Catch
try {
    var result = riskyOperation();
} catch (e) {
    print("Error:", e);
} finally {
    cleanup();
};
```

---

## Intrinsic Functions

These are available globally without any import:

| Function | Description |
|----------|-------------|
| `print(...values)` | Print to stdout |
| `input() -> String` | Read from stdin |
| `toString(val) -> String` | String representation |
| `int(val) -> int` | Convert to integer |
| `float(val) -> float` | Convert to float |
| `str(val) -> String` | Convert to string |
| `bool(val) -> bool` | Convert to boolean |
| `len(val) -> int` | Length of array/string/record |
| `type(val) -> String` | Get type name |
| `now() -> int` | Current timestamp (ms) |
| `exit(code)` | Terminate program |
| `sleep(ms)` | Sleep for ms |
| `throw(msg)` | Runtime error |

---

## Standard Library

### Math (`import "math" as math`)

```clx
math.PI                # 3.14159...
math.E                 # 2.71828...
math.abs(-5)           # 5
math.sqrt(9)           # 3.0
math.pow(2, 3)         # 8.0
math.min(3, 7)         # 3.0
math.max(3, 7)         # 7.0
math.floor(3.7)        # 3
math.ceil(3.7)         # 4
math.round(3.7)        # 4
math.random()          # Random in [0, 1)
math.sin(x), cos(x), tan(x)
math.log(x)            # Natural log
math.range(1, 5)       # [1, 2, 3, 4]
```

### JSON (`import "json" as json`)

```clx
json.parse('{"a": 1}')        # -> { a: 1 }
json.stringify({ "a": 1 })    # -> '{"a":1}'
```

### FS (`import "fs" as fs`) - desktop only

```clx
fs.readFile("path")
fs.writeFile("path", "content")
fs.exists("path")
fs.rm("path")
fs.mkdir("path")
fs.listDir("path")
fs.cwd()
```

### HTTP (`import "http" as http`) - desktop only

```clx
http.get("https://api.example.com")
http.post("https://api.example.com", "body")
```

---

## VFS (Virtual File System)

Filesystem access via protocol URIs, supported by `fs` module functions:

| Protocol | Description | Access |
|----------|-------------|--------|
| `app://file.txt` | Application directory (CWD) | Read/Write |
| `user://file.txt` | User home directory | Read/Write |
| `tmp://file.txt` | Temp directory | Read/Write |
| `res://file.txt` | Resources inside `.clsapp` | Read-only |

```clx
fs.readFile("app://config.json");
fs.writeFile("tmp://cache.dat", data);
```

---

## CLI Reference

### `clx` - Development Toolchain

| Command | Description |
|---------|-------------|
| `new <name>` | Create a new CLS project |
| `run [file] [-- args]` | Execute a script |
| `check [file\|dir]` | Run type checker |
| `build [file] -o` | Package into `.clsapp` |
| `maptype [path] -o` | Generate type maps (.type.json) |
| `ast <file> --json` | Dump AST as JSON |
| `repl` | Interactive REPL |
| `add <pkg>` | Add dependency |
| `remove <pkg>` | Remove dependency |
| `install` | Install dependencies |
| `lsp` | Start LSP server |

### `clxr` - Runtime Executor

```bash
clxr app.clsx          # Run source
clxr app.clsapp        # Run packaged app
```

---

## Project Configuration

Every CLS project has a `cls.json` file:

```json
{
  "name": "my-app",
  "version": "0.1.0",
  "entry": "src/main.clsx",
  "compiler": { "targetArchitecture": "wasm", "optimizationLevel": "O2" },
  "interpreter": {
    "sandbox": { "allowFs": false, "allowNet": false }
  },
  "dependencies": {}
}
```

See `docs/use/CLS_CONFIG.md` for full schema reference.

---

## Editor Support (VS Code)

The extension at `.vscode/extensions/ccls-lang/` provides:

- **Syntax highlighting** for `.clsx`, `.clsi`
- **Autocompletion** via type maps (`.type.json`)
- **Snippets**: `if` + Tab, `function` + Tab, `for` + Tab, etc.
- **LSP server** (optional): `clx lsp` for diagnostics, hover, go-to-definition
- **JSON schema** for `cls.json` with validation
- **Type maps**: auto-generated via `clx maptype --watch`

```bash
# Generate type maps for the workspace
clx maptype . -o .cls-types --watch
```

Configuration in `.vscode/settings.json`:

```json
"cls.options.unnestableFeatures": {
    "lspServer": false,
    "useStaticTypes": true,
    "useMapClsi": true
}
```

---

## Type Maps

Type maps are JSON files that describe all declarations (functions, variables, structures, imports) in `.clsx` and `.clsi` files. They're used by the editor for autocompletion.

```bash
# Generate from file
clx maptype src/main.clsx -o .cls-types/src/main.type.json

# Generate from directory (preserves structure)
clx maptype . -o .cls-types

# Watch mode (auto-regenerate on changes)
clx maptype . -o .cls-types --watch
```

Type maps include: function names, signatures, parameters (with types and docs), return types, variable types, documentation (`@description`, `@params`, `@return`, `@version`, `@deprecated`).

---

## Language Server Protocol

`clx lsp` starts an LSP server over stdin/stdout (or TCP with `--tcp`):

```bash
clx lsp              # stdin/stdout
clx lsp --tcp 127.0.0.1:9876   # TCP socket
```

**Capabilities:**
- Diagnostics (syntax + type errors in real-time)
- Completions (keywords, intrinsics, modules, scope symbols)
- Hover with documentation
- Go-to-definition (functions, variables)
- Document symbols (function list via Ctrl+Shift+O)

---

## Architecture

```
┌─────────────┐    ┌──────────────┐    ┌───────────────┐
│   cls-core   │    │  cls-runtime  │    │  clx / clxr   │
│  (compiler)  │───▶│  (executor)   │───▶│   (nodes)     │
│ lexer, parser│    │ interpreter   │    │ CLI, modules, │
│ type checker │    │ stdlib, VFS   │    │ LSP server    │
│ optimizer    │    │ intrinsics    │    │ package mgmt  │
└─────────────┘    └──────────────┘    └───────────────┘
```

- **cls-core**: No filesystem access. Pure language logic.
- **cls-runtime**: Runtime execution. No direct I/O (uses node-provided resolvers).
- **clx/clxr**: Nodes provide filesystem, network, CLI, and configuration.

---

## Project Structure

```
cls/
├── cls-core/              # Language core (lexer, parser, AST, types)
│   └── src/
│       ├── frontend/      # Lexer, Parser, AST, Token
│       ├── middleware/     # TypeChecker, NameResolver, Optimizer
│       ├── backend/        # JSON backend, WASM backend (future)
│       ├── config/         # ModuleManifest, TypesConfig
│       └── error/          # ClsError, Span, Diagnostic
├── cls-runtime/           # Runtime engine
│   ├── clsi/              # Type definition files (.clsi)
│   └── src/
│       ├── interpreter.rs # Tree-walker
│       ├── value.rs       # Runtime values
│       ├── environment.rs # Scoped environment
│       ├── resolver.rs    # Module resolver
│       ├── vfs/           # Virtual filesystem
│       ├── stdlib/        # math, json modules
│       └── sandbox.rs     # Security sandbox
├── nodos/
│   ├── clx/               # Desktop CLI
│   │   └── src/
│   │       ├── subcommands/  # run, check, build, maptype, lsp...
│   │       ├── modules/      # fs, http, lib (node modules)
│   │       └── lsp.rs       # LSP server
│   └── clxr/              # Runtime executor
│       └── src/main.rs
├── docs/                  # Documentation
├── examples/tests/        # Example scripts
├── agent-context/         # Development plans
└── .vscode/extensions/ccls-lang/  # VS Code extension
```

---

## Milestones

| Phase | Description | Status |
|-------|-------------|--------|
| F1 | Workspace + crates + base nodes | ✅ |
| F2 | Pipeline: lexer -> parser -> tree-walker | ✅ |
| F3 | Type checker + name resolver + optimizer | ✅ |
| F4 | Stdlib: math, json, fs, http, intrinsics | ✅ |
| F5a | ModuleResolver + imports | ✅ |
| F5b | Exports + user modules | ✅ |
| F6 | Migration ccls->clx, .ccls->.clsx | ✅ |
| F7 | VFS + ClsLib indexing | ✅ |
| - | LSP server + VS Code extension | ✅ |
| - | Type maps + autocompletion | ✅ |
| - | Async/await syntax | ✅ |
| - | Structure + Interface | ✅ |
| - | CMX with reference lookup | ✅ |
| - | Error system with traceback | ✅ |
| Future | WASM backend (.clbin) | 🚧 |
| Future | WASM runtime in clxr | 🚧 |
| Future | Registry + package publishing | 🚧 |

---

## License

MIT - see LICENSE file.

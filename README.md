# CLS 2.0 - Modular WASM-first Programming Language

**Version**: 2.0.0-dev-2 · **License**: MIT

[GitHub](https://github.com/frask/cls) · WASM-first · Multiplatform

CLS is a programming language designed for **modular, safe, cross-platform development**. Its **JIT** compiles to WASM and runs on **wasmtime/wasmi**, with an integrated type checker, a JSX-like native syntax (CMX), pattern matching, a VFS abstraction with sandbox support, and a full LSP server for editor integration.

```
.clsx -> Lexer -> Parser -> AST -> TypeChecker -> WASM backend -> WASM (wasmtime/wasmi)
                                                -> JSON backend (AST dump)
```

The **JIT is the only execution engine** (the old tree-walker was removed).

---

## Quick Start

```bash
# Create a project
clx new my-app
cd my-app

# Run it (JIT, default)
clx run

# Check types
clx check

# Run the project's tests
clx test

# Interactive REPL
clx repl

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
- Target `wasm32-unknown-unknown` (used to precompile `cls-internals`)

### From source
```bash
git clone https://github.com/frask/cls
cd cls
rustup target add wasm32-unknown-unknown
cargo build --release --workspace   # clx, clxr, clxb
# or
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

# Async function (syntax; execution pending)
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
    private var count: int;
    static var instances: int = 0;

    function main(start: int) {
        me.count = start;
        Counter.instances = Counter.instances + 1;
    };

    public function increment() {
        me.count = me.count + 1;
    };

    public function value() -> int { return me.count; };

    # Magic: payload used by `var Counter(n) = obj;`
    function __destructurate() -> int { return me.count; };
};

var c = Counter(41);
c.increment();
var Counter(n) = c;     # n = 42
print(n);               # -> 42
```

Classes support inheritance (`class Child: Base`), `super`, `is`,
`private`/`protected`/`public`/`static`/`readonly`, and magic methods
(`__destructurate`, `__match`/`__unpack`, `__equals`, `__bool`, ...).

### Interfaces

```clx
interface Printable {
    print(): void
};
```

### Enums and `match`

```clx
enum Shape {
    circle(float),
    rect(float, float),
    empty,
};

function area(s: Shape) -> float {
    return match (v <- s) {
        circle(r)  -> 3.14159 * r * r,
        rect(w, h) -> w * h,
        default    -> 0.0,
    };
};

var s = Shape::rect(2.0, 3.0);
print(area(s));         # -> 6
```

`match` supports value (`in`), type (`is`), condition (`if`) and pattern
modes, with binding (`name <- expr`). Enums can carry payloads and classes can
opt into pattern matching with `__match()` + `__unpack(tag)`.

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

> Self-containment note: `fs`/`http`/`json`/`strings` are **host-side debt**
> today. The self-contained stdlib is being migrated to `modules/` (CLS + FFI);
> see `modules/README.md` and `examples/patrones.md` §2.7.

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
| `run [file] [-- args]` | Execute a script with the JIT (default) |
| `check [file\|dir]` | Run type checker |
| `test [files] [--filter <s>]` | Run project tests (JIT) |
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

## Testing

```bash
# Rust workspace (unit + integration)
cargo test --workspace

# JIT feature suites (oracle: .expect)
powershell -File examples/test/jit-test/run-tests.ps1
powershell -File examples/test/jit-test/run-availible.ps1

# Differential projects (external oracles)
powershell -File examples/conformance/run.ps1
powershell -File examples/cmx-templates/run.ps1
```

Current status: `units` **84 PASS / 0 FAIL**, `availible` **24 PASS / 1 SKIP**,
`cargo test --workspace` **0 failed**. The JIT is the target: `--ast-walker` was
removed (it only remains as a deprecated, ignored flag).

---

## Project Configuration

Every CLS project has a `cls.json` file:

```json
{
  "name": "my-app",
  "version": "0.1.0",
  "entry": "src/main.clsx",
  "testEntry": ["tests/**/*.clsx"],
  "compiler": { "targetArchitecture": "wasm", "optimizationLevel": "O2" },
  "interpreter": {
    "mode": "jit",
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
┌──────────────┐    ┌───────────────┐    ┌────────────────┐
│   cls-core   │───▶│    cls-jit    │───▶│  clx / clxr    │
│ frontend     │    │ compile/emit  │    │  (nodes)       │
│ type checker │    │ wasmtime/wasmi│    │ CLI, LSP, I/O  │
└──────────────┘    └───────────────┘    └────────────────┘
        │
        ▼
┌────────────────────────────────────────────────────────┐
│ cls-internals: helpers precompiled to wasm32 and fused │
│ into the CLS module (strings, math, records, allocator) │
└────────────────────────────────────────────────────────┘
```

- **cls-core**: frontend (lexer, parser, AST), type checker, module resolver and
  the WASM emitter. No filesystem access.
- **cls-jit**: reusable JIT engine (`compile`, `engine`, `flatten`, `host`,
  `repl`, `resolve`, `state`) with `wasmtime`/`wasmi` backends.
- **cls-internals**: `no_std` runtime helpers compiled to WASM in `build.rs` and
  embedded; they are fused into the CLS module (`__cls_alloc` resolves to the
  module allocator, zero extra imports).
- **cls-runtime**: runtime values, error reporting/formatters, VFS and shared
  support. The tree-walker was removed.
- **clx/clxr**: nodes provide filesystem, network, CLI, LSP and package
  management; `clxb` exposes the C/embedding API.

---

## Project Structure

```
cls/
├── cls-core/              # Language core: lexer, parser, AST, typeck, WASM emitter
├── cls-jit/               # JIT engine: flatten, host, wasmtime/wasmi, REPL
├── cls-runtime/           # Value, error reporting, VFS, intrinsics support
├── cls-internals/         # Runtime helpers precompiled to wasm32-unknown-unknown
├── nodos/
│   ├── clx/               # Development CLI (run/check/build/test/repl/lsp/...)
│   ├── clxr/              # Lightweight runtime executor
│   └── clxb/              # C bindings / embedding (ClsEngine)
├── modules/               # Shared self-contained CLS modules (CLS + FFI)
├── cls-repositories/      # Real projects (git submodules / repos)
├── examples/              # Feature tests + differential projects
├── docs/                  # Language, runtime and development docs
├── devlogs/               # Release notes per version
├── agent-context/         # Development plans and handoffs
└── .vscode/extensions/ccls-lang/  # VS Code extension
```

---

## Real Projects (`/cls-repositories`)

Real, standalone projects live in `cls-repositories/`, separate from feature
tests (`examples/`) and shared modules (`modules/`). Each subfolder is its own
git repository; rules and conventions are in `cls-repositories/README.md`.

| Project | Description |
|---------|-------------|
| `tombstone` | Web framework for CLS (git submodule) — https://github.com/elfrask/tombstone |

```bash
# Clone with submodules
git clone --recurse-submodules https://github.com/frask/cls

# Or initialize them in an existing clone
git submodule update --init --recursive
```

---

## Milestones

| Phase | Description | Status |
|-------|-------------|--------|
| F1–F7 | Workspace, pipeline, type checker, stdlib, modules, VFS, migration | Done |
| P-F1–P-F3 | WASM backend refactor, `cls-internals` fusion, shadow call stack | Done |
| dev-1 | WASM JIT as sole engine, structured FFI, module flattening | Done |
| dev-2 | JSON/`Value`, `match` + enums, CMX rework, `__destructurate`, static typing (no `Any`), differential tests #1–#10 | Done |
| dev-2 | `tombstone` framework (CLS + FFI) | In progress |
| Future | AOT `.clbin` packaging, `.clslib`, registry publishing | Planned |

---

## License

MIT - see LICENSE file.

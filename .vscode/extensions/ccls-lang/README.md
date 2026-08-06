# CLS Language Support for VS Code

Syntax highlighting, snippets, themes and language support for CLS (.clsx).

## Features

- Syntax highlighting for keywords, types, operators, strings, comments
- CMX (JSX-like) syntax support with expression interpolation
- String interpolation (`$name` and `${expr}`)
- Bracket matching and auto-closing pairs
- Function declaration highlighting
- Comment toggling with `#`
- **Snippets** para todas las estructuras del lenguaje (funciones, clases,
  interfaces, alias, genéricos, tuplas, records, control de flujo, CMX, ...)
- **Autocompletado contextual** de la directiva `when`: al posicionarte en
  `when ` se sugieren los selectores `os:`/`arch:`/`abi:`/`platform:`/`target:`
  con sus valores
- **Theme "CLS Tipos Diferenciados"** con colores propios para Tipos, Clases y
  Métodos

## Nuevas sintaxis resaltadas

| Sintaxis | Ejemplo |
|----------|---------|
| Tuplas heterogéneas | `var p: (Int, String) = (1, "x");` |
| Uniones de literales | `alias Color = "red" \| "green" \| "blue";` |
| Alias de tipos | `alias Vec3 = (Int, Int, Int);` |
| Interfaces genéricas | `interface Hello<T=Int> { num: T }` |
| Extracción de tipos | `var n: Hello["num"] = 1;` |
| Genéricos | `function id<T>(x: T) -> T` / `class Caja<T>` |
| Phantom | `interface M<T> { f: !T }` |
| Herencia `:` | `class Dog: Animal` |
| `is` / `super` | `d is Dog`, `super.speak()` |
| Visibilidad | `private` `protected` `readonly` `static` |
| **FFI `extension`** | `extension "libm" as C { function sqrt(x: CDouble) -> CDouble; }` |
| **Tipos nativos C** | `CString` `CPtr` `CInt` `CUInt` `CShort` `CUShort` `CLong` `CULong` `CChar` `CUChar` `CFloat` `CDouble` |
| **Directiva `when`** | `when os: linux and arch: arm64 { ... }` / `default { ... }` |

## Snippets

Escribe el `prefix` y pulsa Tab (o Ctrl+Space para verlos). Los selectores de
`when` (`os`, `arch`, `abi`, `platform`, `target`) solo se sugieren **dentro de
una condición `when`** (scope `meta.when.ccls`).

### FFI / extension

| Prefix | Inserta |
|--------|---------|
| `ext` | `extension "lib" as C { function f(...) -> CInt; }` |
| `extfn` | Función nativa sin cuerpo (dentro de extension) |
| `extst` | Estructura nativa (layout C) |
| `exte` | Extensión con `export` (símbolos para módulos) |
| `extcty` | Función nativa con tipos del ABI C |
| `cty` | Tipos nativos C (`CString|CPtr|CInt|...`) |

### Directiva `when` (multi-entorno)

| Prefix | Inserta |
|--------|---------|
| `when` | `when os: linux { ... }` |
| `whenx` | `when os: linux and arch: arm64 { ... }` (condiciones combinadas) |
| `whent` | `when target: "riscv32-none-elf" { ... }` (tripla, embebido) |
| `dflt` | `default { ... }` (rama que siempre matchea) |
| `extw` | `when os: windows { extension "lib" as C { ... } }` (FFI por plataforma) |

### Selectores de `when` (autocompletado contextual)

| Prefix | Inserta |
|--------|---------|
| `os` | `os: \|windows,linux,macos,none,bare-metal,freebsd\|` |
| `arch` | `arch: \|x86_64,arm64,aarch64,arm,riscv32,riscv64,avr\|` |
| `abi` | `abi: \|gnu,msvc,eabi,elf\|` |
| `platform` | `platform: \|pc,esp32,stm32f4,rp2040,none\|` |
| `target` | `target: "riscv32-none-elf"` |

## Colores diferenciados (theme)

Para activar: `Ctrl+K Ctrl+T` → **CLS Tipos Diferenciados**.

| Scope | Color |
|-------|-------|
| Tipos primitivos (`storage.type.*.ccls`) | Teal |
| Tipos nativos C (`storage.type.native.ccls`) | Teal |
| Clases/interfaces/alias (`entity.name.type.ccls`) | Dorado, negrita |
| Funciones (`entity.name.function.ccls`) | Amarillo suave |
| Métodos de clase (`entity.name.function.method.ccls`) | Azul |
| Genéricos / acceso / tuplas | Cian |
| Phantom `!T` | Púrpura, negrita |
| Keywords y plataformas (`keyword.control.*`, `keyword.operator.selector`) | Morado |

## Installation

Copy the `ccls-lang` folder to your VS Code extensions directory:

```bash
cp -r ccls-lang ~/.vscode/extensions/
```

Or from this workspace, VS Code will pick up `.vscode/extensions/ccls-lang/` automatically.

## Syntax Overview

| Element | Tokens highlighted |
|---------|-------------------|
| Comments | `# comment` |
| Numbers | `42` `3.14` |
| Strings | `"hello"` `'world'` `` `template` `` |
| Interpolation | `$name` `${expr}` |
| Keywords | `if` `else` `while` `for` `function` `var` `const` `is` |
| Types | `Int` `String` `Float` `Bool` `i32` `str` `Tuple` `Record` |
| Operators | `+` `-` `==` `!=` `->` `::` `++` `\|` |
| CMX/JSX | `<Component attr={expr}>children</Component>` |
| Constants | `true` `false` `null` `unknown` |
| Self | `me` `super` |

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

## Colores diferenciados (theme)

Para activar: `Ctrl+K Ctrl+T` → **CLS Tipos Diferenciados**.

| Scope | Color |
|-------|-------|
| Tipos primitivos (`storage.type.*.ccls`) | Teal |
| Clases/interfaces/alias (`entity.name.type.ccls`) | Dorado, negrita |
| Funciones (`entity.name.function.ccls`) | Amarillo suave |
| Métodos de clase (`entity.name.function.method.ccls`) | Azul |
| Genéricos / acceso / tuplas | Cian |
| Phantom `!T` | Púrpura, negrita |

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

# CLS Language Support for VS Code

Syntax highlighting for CLS (.ccls) files.

## Features

- Syntax highlighting for keywords, types, operators, strings, comments
- CMX (JSX-like) syntax support with expression interpolation
- String interpolation (`$name` and `${expr}`)
- Bracket matching and auto-closing pairs
- Function declaration highlighting
- Comment toggling with `#`

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
| Keywords | `if` `else` `while` `for` `function` `var` `const` |
| Types | `Int` `String` `Float` `Bool` `i32` `str` |
| Operators | `+` `-` `==` `!=` `->` `::` `++` |
| CMX/JSX | `<Component attr={expr}>children</Component>` |
| Constants | `true` `false` `null` `unknown` |
| Self | `me` |

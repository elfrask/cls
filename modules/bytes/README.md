# `bytes` — utilidades de bytes (`ord`/`chr`/hex/diff)

Módulo **CLS puro** (sin FFI) con utilidades de bytes reutilizables. `ord`/`chr`
se apoyan en los **intrínsecos primitivos** del lenguaje sobre `Char` = unidad de
1 byte (`char(n)`, `ord(c)`); el hex y el diff son CLS puro.

No usa `host_calls` y no expone punteros.

## API

| Símbolo | Descripción |
|---|---|
| `ord(c: Char) -> int` | código del byte (0..255) de un `Char` |
| `chr(n: int) -> String` | string de 1 byte con el código `n` (0..255), byte-transparente |
| `hexEncode(bytes: Char[]) -> String` | buffer de bytes → hex minúsculas (2 chars/byte) |
| `hexDecode(hex: String) -> int[]` | hex → `int[]` (nibble inválido → -1) |
| `firstDiff(a: int[], b: int[]) -> int` | -2 si iguales; si no, índice de la 1ª diferencia |

Uso:

```clx
import "bytes" as bytes;

var n = bytes::ord("A".toChar()[0]);      # 65
var s = bytes::chr(66);                    # "B"
var h = bytes::hexEncode("á".toChar());    # "c3a1" (los 2 bytes UTF-8)
var b = bytes::hexDecode("cafe");          # [202, 254]
var d = bytes::firstDiff([1,2,3], [1,9,3]);  # 1
```

> Los bytes de un `String` se obtienen con `String.toChar() -> Char[]`; el código
> de cada byte con el intrínseco `ord`. `Char` es la unidad de 1 byte (modelo
> `bytes`/`str`), no un codepoint Unicode.

## Compilar / consumir

No requiere build (CLS puro). Solo:

```clx
import "bytes" as bytes;
```

## Limitaciones conocidas

- `hexEncode` recibe un buffer de bytes como `Char[]` (usar `s.toChar()`).

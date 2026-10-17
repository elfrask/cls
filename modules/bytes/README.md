# `bytes` — primitivas de byte y buffers (CLS puro)

Ladrillo de la stdlib autocontenida (`examples/patrones.md` §11): **CLS puro, sin
FFI**. Es el **hogar de las primitivas de byte/buffer**: conversión de buffers,
hex y enteros big-endian. Los módulos que manipulan bytes (`net`, `io`, `dns`,
`httpc`) consumen `bytes` en vez de reimplementar.

`Char` es la unidad de 1 byte (0..255); los buffers viajan como:
- `int[]` (stride 8) para el FFI, y
- `Char[]` (stride 4) para el lenguaje.

> Las conversiones **String → buffers** ya son intrínsecas del lenguaje:
> `String.toChar() -> Char[]` y `String.toBytes() -> int[]`.

## Estructura

```
bytes/
├── cls.json
├── byte.clsx   # ord/chr/normalize + toChars/toInts/toStr
├── hex.clsx    # hexEncode/hexDecode/hexBytes/hexByte/hexDigit/hexValue/hexToInt/intToHex
├── endian.clsx # getU16/getU32/pushU16/pushU32/pushAll
├── diff.clsx   # firstDiff
└── mod.clsx    # barrel
```

## API

| Símbolo | Descripción |
|---|---|
| `ord(c: Char) -> int` | código del byte (0..255) de un `Char` |
| `chr(n: int) -> String` | string de 1 byte con el código `n` |
| `normalize(b: int) -> int` | byte sin signo (0..255) |
| `toChars(b: int[]) -> Char[]` | `int[]` → `Char[]` |
| `toInts(c: Char[]) -> int[]` | `Char[]` → `int[]` |
| `toStr(b: int[]) -> String` | buffer de bytes → String (byte-transparente) |
| `hexEncode(c: Char[]) -> String` | `Char[]` → hex minúsculas |
| `hexBytes(b: int[]) -> String` | `int[]` → hex minúsculas |
| `hexDecode(hex: String) -> int[]` | hex → `int[]` (nibble impar: ignorado; inválido: -1) |
| `hexByte(b: int) -> String` | byte → 2 dígitos hex |
| `hexDigit(nibble: int) -> String` | nibble (0..15) → 1 dígito hex |
| `hexValue(c: int) -> int` | dígito hex → valor 0..15 (`-1` si no) |
| `hexToInt(hex: String) -> int` | hex → int (parsea; ignora no-dígitos) |
| `intToHex(v: int) -> String` | int → hex sin ceros a la izquierda |
| `getU16/getU32(b, off) -> int` | lee entero big-endian |
| `pushU16/pushU32(out, v)` | escribe entero big-endian (in-place) |
| `pushAll(dst, src)` | concatena buffers |
| `firstDiff(a, b) -> int` | -2 si iguales; si no, índice de la 1ª diferencia |

## Uso

```clx
import "bytes" as bytes;

var b: int[] = "á".toBytes();              # [195, 161] (2 bytes UTF-8)
var h = bytes::hexBytes(b);                 # "c3a1"
var c: Char[] = bytes::toChars(b);          # Char[]
var s = bytes::toStr(b);                    # "á"
var n = bytes::hexToInt("1f");              # 31
```

> No requiere build (CLS puro).

## Limitaciones conocidas

- El tipo de buffer (`int[]` vs `Char[]`) lo fija el llamador; `toChars`/`toInts`
  **convierten** (copiando) porque los strides difieren (4 vs 8).

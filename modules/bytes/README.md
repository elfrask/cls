# `bytes` — utilidades de bytes (`ord`/`chr`/hex/diff)

Módulo **CLS + FFI** con utilidades de bytes reutilizables. Los primitivos
`ord`/`chr` (código de byte y string de 1 byte) son **FFI** al shim
`bytes_shim.c` (`bytes_shim.dll`); el hex y el diff son **CLS puro**.

No usa `host_calls` y no expone punteros (`CString`/`CLong`).

> **Supersede** a los helpers hex de `modules/diag` (`hexByte`, `bytesToHex`).
> `diag` se mantiene por compatibilidad pero queda marcado como superseded
> (ver `modules/README.md`).

## API

| Símbolo | Descripción |
|---|---|
| `ord(s, i) -> int` | código del byte `i` de `s` (0..255); -1 fuera de rango |
| `chr(n) -> String` | string de 1 byte con el código `n` (0..255) |
| `hexEncode(bytes) -> String` | `int[]` → hex minúsculas (2 chars/byte) |
| `hexDecode(hex) -> int[]` | hex → `int[]` (nibble inválido → -1) |
| `firstDiff(a, b) -> int` | -2 si iguales; si no, índice de la 1ª diferencia |

Uso:

```clx
import "bytes" as bytes;

var n = bytes::ord("A", 0);            # 65
var s = bytes::chr(66);                # "B"
var h = bytes::hexEncode([0, 255]);    # "00ff"
var b = bytes::hexDecode("cafe");      # [202, 254]
var d = bytes::firstDiff([1,2,3], [1,9,3]);  # 1
```

## Compilar / consumir

```powershell
# Compila bytes_shim.dll (desde cualquier CWD)
powershell -File modules\bytes\build.ps1
```

```clx
import "bytes" as bytes;
```

## Limitaciones conocidas

- **La ruta del `.dll` FFI es relativa al CWD**, no al archivo `.clsx`. Correr
  desde la **raíz del repo** (o ajustar la ruta).
- `chr(0)` devuelve `""`: el `CString` del FFI no representa un byte NUL.

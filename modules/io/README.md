# `io` — I/O de archivos y descriptores (FFI a C)

Módulo **CLS + FFI** que da acceso al sistema de archivos y a descriptores de
archivo. Es la **semilla del futuro `fs`** de la stdlib autocontenida
(`examples/patrones.md` §11) y **paga la deuda** de §2.7: reemplaza al
`host_call` `fs` (`readFile`, `writeFile`, `exists`, `rm`, `mkdir`, `listDir`,
`cwd`) por FFI puro + código CLS.

No usa `host_calls`. El shim `io_shim.c` (`io_shim.dll`, `io_shim.so` o
`io_shim.dylib`) envuelve `fopen`/`fwrite`/`fseek` y las llamadas de
descriptores de la libc (`_stat`/`_open`/`_read`/`_write`/`_close`/`_setmode`
en Windows; `stat`/`open`/`read`/`write`/`close` en POSIX), y **nunca expone
punteros** a CLS: todo via `CString`/`CArray`/`CLong`/`CInt`.

## API

| Símbolo | Descripción |
|---|---|
| `readText(path) -> String` | archivo completo como texto (null-terminated) |
| `readBytes(path) -> int[]` | archivo completo como bytes crudos (seguro con NUL) |
| `readChars(path) -> Char[]` | **[experimental]** igual que `readBytes` pero como buffer `Char[]` (0..255); ver *Buffers con `Char`* |
| `writeText(path, data) -> int` | escribe (trunca) byte-exacto; bytes escritos o -1 |
| `appendText(path, data) -> int` | agrega al final byte-exacto; bytes escritos o -1 |
| `exists(path) -> int` | 1/0 |
| `isDir(path) -> int` | 1/0 |
| `isFile(path) -> int` | 1/0 |
| `remove(path) -> int` | borra un archivo (0 ok) |
| `cwd() -> String` | directorio de trabajo actual |
| `chdir(path) -> int` | cambia de directorio (0 ok) |
| `open(path, flags, mode) -> int` | abre un descriptor |
| `close(fd) -> int` | cierra el descriptor |
| `writeFd(fd, data) -> int` | escribe `data` al descriptor (byte-exacto en modo binario) |
| `readLine(fd) -> String` | una línea sin `\n`; `""` en EOF |
| `setmode(fd, mode) -> int` | `_setmode` del descriptor (binario/texto) |

Constantes: `O_RDONLY`, `O_WRONLY`, `O_CREAT`, `O_TRUNC`, `O_APPEND`, `O_BINARY`.
Los valores son los de msvcrt; en POSIX el shim los traduce a los flags de
`open(2)`. `setmode` es no-op en POSIX (no hay modo texto/binario).

Uso:

```clx
import "io" as io;

var src = io::readText("src/main.clsx");
io::writeText("out.txt", "hola\n");
var raw = io::readBytes("data.bin");   # int[] seguro con bytes NUL
```

## Buffers con `Char` (`readChars`, experimental)

`Char` es la **unidad de 1 byte** para trabajo con buffers (modelo `bytes`/`str`
de Python), junto a `String.toChar()`/`char()`/`ord()`. `readChars` expone el
archivo como `Char[]`:

```clx
var buf = io::readChars("data.bin");   # Char[] (0..255)
var hex = bytes::hexEncode(buf);       # buffer -> hex
var s   = buf.join("");                # Char[] -> String (byte-exacto)
```

> **Por qué convive con `readBytes`:** el marshalling de `CArray` del FFI hoy usa
> **stride de 8 bytes** (solo `int[]`), y un `Char[]` es i32 (stride 4). Por eso
> `readChars` se implementa **convirtiendo en CLS** el `int[]` del shim
> (transitorio), sin tocar el FFI ni eliminar `readBytes`. Cuando el FFI soporte
> `Char[]` (stride 4) nativamente, `readChars` podrá cruzar directo (sin copia).

## Compilar / consumir

```powershell
# Windows: compila io_shim.dll (desde cualquier CWD)
powershell -File modules\io\build.ps1
```

```bash
# Linux/macOS (o `bash scripts/build-standard-modules.sh`)
gcc -shared -fPIC -O2 -o modules/io/io_shim.so modules/io/io_shim.c
```

```clx
import "io" as io;
```

## Limitaciones conocidas

- **La ruta del shim FFI se resuelve contra la raíz del paquete**, no contra el
  CWD. `extension "./io_shim.dll"` (o `.so`/`.dylib`) se busca en el `cls.json`
  del módulo.
- `writeText`/`appendText`/`writeFd` escriben hasta el primer byte NUL
  (los datos viajan como `CString`). Para bytes con NUL usar `readBytes` y un
  future `writeBytes` con `CArray`.
- No hay `mkdir`, `listDir` ni `rm` de directorios todavía (solo `remove` de
  archivos).

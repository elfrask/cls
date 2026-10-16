# `io` — I/O de archivos y descriptores (FFI a C)

Módulo **CLS + FFI** que da acceso al sistema de archivos y a descriptores de
archivo. Es la **semilla del futuro `fs`** de la stdlib autocontenida
(`examples/patrones.md` §11) y **paga la deuda** de §2.7: reemplaza al
`host_call` `fs` (`readFile`, `writeFile`, `exists`, `rm`, `mkdir`, `listDir`,
`cwd`) por FFI puro + código CLS.

No usa `host_calls`. El shim `io_shim.c` (`io_shim.dll`) envuelve `fopen`/
`fwrite`/`fseek`, `_stat`, `_open`/`_read`/`_write`/`_close`/`_setmode` de la
libc, y **nunca expone punteros** a CLS: todo via `CString`/`CArray`/`CLong`/
`CInt`.

## API

| Símbolo | Descripción |
|---|---|
| `readText(path) -> String` | archivo completo como texto (null-terminated) |
| `readBytes(path) -> int[]` | archivo completo como bytes crudos (seguro con NUL) |
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

Uso:

```clx
import "io" as io;

var src = io::readText("src/main.clsx");
io::writeText("out.txt", "hola\n");
var raw = io::readBytes("data.bin");   # int[] seguro con bytes NUL
```

## Compilar / consumir

```powershell
# Compila io_shim.dll (desde cualquier CWD)
powershell -File modules\io\build.ps1
```

```clx
import "io" as io;
```

## Limitaciones conocidas

- **La ruta del `.dll` FFI es relativa al CWD**, no al archivo `.clsx`. La
  declaración `extension "modules/io/io_shim.dll"` se resuelve desde el
  directorio de trabajo, así que hay que ejecutar desde la **raíz del repo**.
- `writeText`/`appendText`/`writeFd` escriben hasta el primer byte NUL
  (los datos viajan como `CString`). Para bytes con NUL usar `readBytes` y un
  future `writeBytes` con `CArray`.
- No hay `mkdir`, `listDir` ni `rm` de directorios todavía (solo `remove` de
  archivos).

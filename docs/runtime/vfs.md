# VFS (sistema de archivos virtual)

El VFS abstrae el acceso a archivos del runtime: protocolos con prefijo
(`app://`, `user://`, `tmp://`, `res://`) o rutas relativas a la app.
Vive en `cls-runtime/src/vfs/` (`mod.rs`, `protocol.rs`, `resolver.rs`,
`security.rs`).

## Protocolos

### `VfsProtocol` (`protocol.rs`)

```rust
pub trait VfsProtocol: Send + Sync {
    fn read(&self, path: &str) -> ClsResult<Vec<u8>>;
    fn read_to_string(&self, path: &str) -> ClsResult<String>;
    fn write(&self, path: &str, data: &[u8]) -> ClsResult<()>;
    fn exists(&self, path: &str) -> bool;
    fn list_dir(&self, path: &str) -> ClsResult<Vec<String>>;
    fn name(&self) -> &str;
    fn is_read_only(&self) -> bool;
}
```

### `LocalFs`

Filesystem local para `app://`, `user://` y `tmp://`.

- `LocalFs::new(name, base, read_only)` - base es el directorio raíz.
- `write` crea los directorios padre con `create_dir_all` y falla si es
  read-only.
- `list_dir` devuelve nombres de entrada (no rutas completas).
- Todas las rutas pasan por `resolve_safe` (jail).

### `ZipFs`

`res://` dentro de un `.clsapp` (zip).

- `ZipFs::open(name, path)` abre el archivo zip; `ZipFs::empty()` es un
  placeholder sin archive.
- **Read-only** (`write` falla con "res:// es read-only").
- `list_dir` no está implementado (error "res:// listDir no implementado").
- Caché de archivos leídos en memoria (tabla con tope de 256 entradas).
- `exists` consulta el zip por nombre de entry.

## `VfsResolver` (`resolver.rs`)

| Método | Comportamiento |
|---|---|
| `register(name, protocol)` | Registra un protocolo |
| `add_route(name, target)` | Ruta personalizada; **no sobrescribe** los reservados `res`, `app`, `user`, `tmp` |
| `resolve(path)` | `proto://ruta` -> protocolo + ruta; sin protocolo -> relativo a `app://` |
| `read_file` / `read_to_string` / `write_file` / `exists` / `list_dir` | Delegan al protocolo resuelto |
| `remove(path)` | Borra archivo o directorio (falla si el protocolo es read-only) |
| `create_dir(path)` | `create_dir_all` (falla si read-only) |

Las rutas personalizadas pueden apuntar a otro protocolo
(`target` con `://`) o a un path relativo bajo `app://`.

## Seguridad (`security.rs`)

`resolve_safe(path, base)` implementa un chroot jail:

- Bloquea **rutas absolutas** (raíz o prefijo).
- Bloquea `..` que salga del base (`Path traversal detectado: ...`).
- Verifica al final que el resultado siga dentro del base.

## Mapeo de protocolos por ejecutor

### `clx run` (walker, `nodos/clx/src/subcommands/run.rs::make_vfs`)

| Protocolo | Base |
|---|---|
| `app` | CWD |
| `user` | `HOME`/`USERPROFILE` |
| `tmp` | temp del sistema |
| `project` | ruta personalizada al **parent del entry** de `cls.json` |

### `clxr`

| Protocolo | Base |
|---|---|
| `app` | CWD |
| `user` | `HOME` (o `USERPROFILE`) |
| `tmp` | temp del sistema |
| `res` | `ZipFs` sobre el `.clsapp` (solo al ejecutar `.clsapp`) |

`clxr` lee el source del zip (entry según `manifest.json` del paquete o
`source.clsx`) y registra `res://` para que el script acceda a los archivos
del paquete.

## Uso desde CLS

`fs.readFile`/`fs.writeFile` (nodo desktop) aceptan paths con protocolo:

```clx
import "fs" as fs;

function main(args: String[]) -> int {
    var src = fs.readFile("res://source.clsx");  # dentro del .clsapp
    var app = fs.readFile("app://config.json");  # relativo al CWD
    return 0;
};
```

Sin `://`, los módulos del nodo desktop usan `std::fs` directo (path del
sistema); con `://` pasan por el `VfsResolver`.
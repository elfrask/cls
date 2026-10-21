# `argon2`

Argon2id para CLS. Usa **libargon2** (PHC winner) compilada estáticamente dentro
del shim, sin DLL externa adicional.

## API

```clx
import "argon2" as argon2;

var encoded = argon2::hash("password", "00112233445566778899aabbccddeeff");
argon2::verify(encoded, "password");       // true
argon2::needs_rehash(encoded);             // false
```

- `hash(password, salt_hex)` usa `Argon2id`, `t=2`, `m=19456 KiB`, `p=1`,
  hash de 32 bytes.
- `verify(encoded, password)` valida el string PHC.
- `needs_rehash(encoded)` indica si los parámetros actuales cambiaron.

## Recursos y compilación

La fuente de libargon2 no se vendoriza. El descargador verifica SHA-256 y cachea
en `.resources/libargon2/`:

```powershell
powershell -File scripts/fetch-resources.ps1 -Only libargon2
powershell -File modules/argon2/build.ps1
```

```bash
bash scripts/fetch-resources.sh libargon2
bash modules/argon2/build.sh
```

Windows genera `argon2_shim.dll`; Linux `argon2_shim.so`; macOS
`argon2_shim.dylib`. La ruta se resuelve contra el `cls.json` del paquete, no
contra el CWD.

## Notas de seguridad

- El salt debe venir de un CSPRNG (`modules/csprng`).
- El formato encoded incluye versión y parámetros; `password_needs_rehash` en
  Tombstone rehashea al hacer login.
- No se aceptan contraseñas con NUL (limitación de `CString`).

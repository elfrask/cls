# `csprng`

CSPRNG del sistema operativo para tokens, salts y sesiones. Reemplaza al PRNG
determinista (`modules/prng`), que **no** es apto para seguridad.

- Windows: `BCryptGenRandom` (`bcrypt.dll`).
- Linux/macOS: lectura completa de `/dev/urandom`.
- API: `random_bytes(n) -> int[]`, `random_hex(n)`, `random_token(n)`.
- El shim vive en `modules/csprng/random_shim.dll` (Windows) y se resuelve
  relativo al CWD, igual que `sqlite`/`socket`/`io`: ejecutar desde la raíz del
  repo CLS.

## Compilar el shim (Windows)

```powershell
powershell -File modules/csprng/build.ps1
# equivale a: gcc -shared -O2 -o modules/csprng/random_shim.dll \
#   modules/csprng/random_shim.c -lbcrypt
```

En Unix, compilar `random_shim.c` como `.so` con `-shared -fPIC`; el código usa
`/dev/urandom`. La ruta FFI esperada es `modules/csprng/random_shim.so`.

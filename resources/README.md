# Recursos nativos externos

`manifest.json` declara recursos que no se vendorizan en el repo. El descargador
los cachea en `.resources/` (ignorado por git) y verifica SHA-256. En Linux/macOS\nrequiere `curl`, `unzip`, `sha256sum`/`shasum` y `python3`; en Windows usa\nPowerShell.

```powershell
powershell -File scripts/fetch-resources.ps1
powershell -File scripts/fetch-resources.ps1 -Only libargon2 -Force
```

```bash
bash scripts/fetch-resources.sh
bash scripts/fetch-resources.sh libargon2 --force
```

Actualmente:

| Recurso | Uso | Versión |
|---|---|---|
| `libargon2` | `modules/argon2` | 20190702 |

Para agregar otro recurso, añadir una entrada con `url`, `sha256`, `archive` y
`extract_dir`; no hace falta cambiar los scripts.

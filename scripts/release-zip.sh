#!/usr/bin/env bash
# Empaca el contenido de un directorio en un .zip de forma portable:
#   - Linux: usa `zip`
#   - Windows (git-bash): usa powershell Compress-Archive
# Uso: release-zip.sh <directorio> <zip-destino>
set -euo pipefail

SRC="${1:?uso: release-zip.sh <dir> <zip>}"
DEST="${2:?uso: release-zip.sh <dir> <zip>}"

if [[ "$(uname -s)" == "Linux" ]]; then
    (cd "$SRC" && zip -qr "$OLDPWD/$DEST" .)
else
    powershell -NoProfile -Command "Compress-Archive -Path '$SRC/*' -DestinationPath '$DEST' -Force"
fi
#!/usr/bin/env bash
# Empaqueta la extension VS Code CLS en un .vsix (via vsce) e instala con `code`.
# Uso: release-vsix.sh [--install] [--out <archivo>]
#   --install   instala el vsix despues de empaquetar (code --install-extension --force)
#   --out       nombre del vsix de salida (default: cls-lang.vsix en la raiz del repo)
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
EXT_DIR="$ROOT/.vscode/extensions/ccls-lang"
OUT="$ROOT/cls-lang.vsix"
INSTALL=0

while [[ $# -gt 0 ]]; do
    case "$1" in
        --install) INSTALL=1 ;;
        --out) OUT="${2:?uso: --out <archivo>}"; shift ;;
        *) echo "uso: release-vsix.sh [--install] [--out <archivo>]"; exit 1 ;;
    esac
    shift
done

[[ -f "$EXT_DIR/package.json" ]] || { echo "ERROR: no se encontro la extension en $EXT_DIR"; exit 1; }

(cd "$EXT_DIR" && npx --yes @vscode/vsce package --out "$OUT")

echo
echo "=== Empaquetado: $OUT ==="
if [[ "$INSTALL" == "1" ]]; then
    code --install-extension "$OUT" --force
    echo "Instalada."
fi
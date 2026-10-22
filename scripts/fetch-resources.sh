#!/usr/bin/env bash
# fetch-resources.sh - descarga recursos nativos externos declarados en
# resources/manifest.json. Cache local: <repo>/.resources/<recurso>/.
#
# Uso:
#   bash scripts/fetch-resources.sh
#   bash scripts/fetch-resources.sh libargon2 --force
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MANIFEST="$ROOT/resources/manifest.json"
CACHE="$ROOT/.resources"
FORCE=0
ONLY=()

for arg in "$@"; do
    case "$arg" in
        --force|-f) FORCE=1 ;;
        -*) echo "ERROR: opcion desconocida: $arg" >&2; exit 2 ;;
        *) ONLY+=("$arg") ;;
    esac
done

[ -f "$MANIFEST" ] || { echo "ERROR: falta $MANIFEST" >&2; exit 2; }
mkdir -p "$CACHE"

command -v python3 >/dev/null 2>&1 || { echo "ERROR: falta python3 para leer manifest.json" >&2; exit 2; }

# name<TAB>version<TAB>url<TAB>sha256<TAB>archive<TAB>extract_dir
# Sin `mapfile`: macOS trae bash 3.2 y no lo soporta.
while IFS= read -r row; do
    IFS=$'\t' read -r name version url sha archive extract_name <<< "$row"
    if [ "${#ONLY[@]}" -gt 0 ]; then
        found=0
        for want in "${ONLY[@]}"; do [ "$want" = "$name" ] && found=1; done
        [ "$found" = 1 ] || continue
    fi

    dir="$CACHE/$name"
    zip="$dir/$archive"
    extract="$dir/$extract_name"
    mkdir -p "$dir"

    if [ "$FORCE" = 1 ] || [ ! -f "$zip" ]; then
        echo "descargando $name $version: $url"
        curl -fL --retry 3 -o "$zip" "$url"
    fi

    if command -v sha256sum >/dev/null 2>&1; then
        got="$(sha256sum "$zip" | awk '{print $1}')"
    else
        got="$(shasum -a 256 "$zip" | awk '{print $1}')"
    fi
    if [ "$got" != "$sha" ]; then
        rm -f "$zip"
        echo "ERROR: sha256 de $name no coincide" >&2
        echo "  esperado: $sha" >&2
        echo "  obtenido: $got" >&2
        exit 1
    fi

    if [ "$FORCE" = 1 ] || [ ! -d "$extract" ]; then
        rm -rf "$extract"
        echo "extrayendo $name..."
        unzip -q -o "$zip" -d "$dir"
    fi
    echo "OK $name -> $extract"
done < <(python3 - "$MANIFEST" <<'PY'
import json, sys
m = json.load(open(sys.argv[1], encoding='utf-8'))
for name, r in m.items():
    print('\t'.join([name, r['version'], r['url'], r['sha256'], r['archive'], r['extract_dir']]))
PY
)

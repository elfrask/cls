#!/usr/bin/env bash
# build-standard-modules.sh - compila los shims nativos de /modules para Linux/macOS.
# Windows usa los build.ps1 de cada modulo.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CC="${CC:-gcc}"

build_shim() {
    local mod="$1" src="$2" out="$3"
    shift 3
    echo "== module $mod: $out =="
    "$CC" $FLAGS -O2 -o "$ROOT/modules/$mod/$out" "$ROOT/modules/$mod/$src" "$@"
}

case "$(uname -s)" in
    Darwin) SUFFIX="dylib"; FLAGS="-dynamiclib"; SQLITE_LIBS="";;
    *)      SUFFIX="so";    FLAGS="-shared -fPIC"; SQLITE_LIBS="-ldl";;
esac

build_shim csprng random_shim.c "random_shim.$SUFFIX"
build_shim io io_shim.c "io_shim.$SUFFIX"
build_shim net net_shim.c "net_shim.$SUFFIX"
build_shim socket socket_shim.c "socket_shim.$SUFFIX"
build_shim sqlite sqlite_shim.c "sqlite_shim.$SUFFIX" $SQLITE_LIBS

if [ ! -f "$ROOT/modules/argon2/argon2_shim.$SUFFIX" ]; then
    bash "$ROOT/modules/argon2/build.sh"
fi

echo "OK: standard modules"

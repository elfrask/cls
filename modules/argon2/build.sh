#!/usr/bin/env bash
# build.sh - compila argon2_shim (libargon2 estatico) para Linux/macOS.
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
SRC="$ROOT/.resources/libargon2/phc-winner-argon2-20190702"

if [ ! -f "$SRC/include/argon2.h" ]; then
    bash "$ROOT/scripts/fetch-resources.sh" libargon2
fi

THREAD_FLAG="-pthread"
case "$(uname -s)" in
    Darwin) OUT="$HERE/argon2_shim.dylib"; FLAGS="-dynamiclib" ;;
    MINGW*|MSYS*|CYGWIN*) OUT="$HERE/argon2_shim.dll"; FLAGS="-shared"; THREAD_FLAG="" ;;
    *)      OUT="$HERE/argon2_shim.so";    FLAGS="-shared -fPIC" ;;
esac

CC="${CC:-gcc}"
"$CC" $FLAGS -O2 -o "$OUT" \
    "$HERE/argon2_shim.c" \
    "$SRC/src/argon2.c" \
    "$SRC/src/core.c" \
    "$SRC/src/ref.c" \
    "$SRC/src/blake2/blake2b.c" \
    "$SRC/src/thread.c" \
    "$SRC/src/encoding.c" \
    -I "$SRC/include" -I "$SRC/src" $THREAD_FLAG
echo "OK $OUT"

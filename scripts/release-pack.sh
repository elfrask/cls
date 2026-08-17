#!/usr/bin/env bash
# Empaqueta los artefactos de CLS para un OS dado (portable + binarios sueltos
# + bindings con su librería nativa). Se ejecuta desde la raíz del repo.
#
# Uso: release-pack.sh <os> <exe_ext> <native_lib> <version> <sha7>
#   <os>          "windows" | "linux"
#   <exe_ext>     ".exe" (windows) | "" (linux)
#   <native_lib>  "clsb.dll" (windows) | "libclsb.so" (linux)
#   <version>     versión CLS (ej: 2.0.0)
#   <sha7>        hash corto del commit
set -euo pipefail

OS="${1:?uso: release-pack.sh <os> <exe_ext> <native_lib> <version> <sha7>}"
EXE_EXT="${2:-}"
NATIVE_LIB="${3:?}"
VERSION="${4:?}"
SHA7="${5:?}"

ROOT="$(pwd)"
REL="$ROOT/target/release"
OUT="$ROOT/dist/$OS"
ZIP="$ROOT/scripts/release-zip.sh"
mkdir -p "$OUT"

# ── 2.1 Portable ─────────────────────────────────────────────────────────────
PORTABLE_DIR="$OUT/portable"
mkdir -p "$PORTABLE_DIR"
cp "$REL/clx$EXE_EXT" "$PORTABLE_DIR/"
cp "$REL/clxr$EXE_EXT" "$PORTABLE_DIR/"
(cd "$PORTABLE_DIR" && "$ZIP" . "../cls-$OS-v$VERSION-$SHA7.zip")

# ── 2.2 Binarios sueltos ─────────────────────────────────────────────────────
cp "$REL/clx$EXE_EXT" "$OUT/clx-$OS-x64$EXE_EXT"
cp "$REL/clxr$EXE_EXT" "$OUT/clxr-$OS-x64$EXE_EXT"

# ── 2.4 Bindings ─────────────────────────────────────────────────────────────
# clsb-c: header + lib nativa + harness
C_DIR="$OUT/clsb-c"
mkdir -p "$C_DIR"
cp "$ROOT/nodos/clxb/include/clsb.h" "$C_DIR/"
cp "$REL/$NATIVE_LIB" "$C_DIR/"
cp "$ROOT/nodos/clxb/examples/harness.c" "$C_DIR/"
(cd "$C_DIR" && "$ZIP" . "../clsb-c-$OS.zip")

# clsb-python: bindings/python completo + lib en clsb/bin/
PY_DIR="$OUT/clsb-python"
mkdir -p "$PY_DIR/clsb/bin"
cp -r "$ROOT/bindings/python/." "$PY_DIR/"
cp "$REL/$NATIVE_LIB" "$PY_DIR/clsb/bin/"
(cd "$PY_DIR" && "$ZIP" . "../clsb-python-$OS.zip")

# clsb-js: src/ + lib/ + package.json + types/ + lib nativa en lib/
JS_DIR="$OUT/clsb-js"
mkdir -p "$JS_DIR/lib"
cp -r "$ROOT/bindings/js/src" "$JS_DIR/src"
cp -r "$ROOT/bindings/js/types" "$JS_DIR/types"
cp "$ROOT/bindings/js/package.json" "$JS_DIR/"
cp "$ROOT/bindings/js/lib/loader.js" "$JS_DIR/lib/"
cp "$REL/$NATIVE_LIB" "$JS_DIR/lib/"
(cd "$JS_DIR" && "$ZIP" . "../clsb-js-$OS.zip")

# Limpieza de dirs de trabajo (solo quedan los zips y binarios)
rm -rf "$PORTABLE_DIR" "$C_DIR" "$PY_DIR" "$JS_DIR"

echo "=== Artefactos $OS ==="
ls -lh "$OUT"
#!/bin/bash
# Build: driver de git (gix) para clx -> libclxgix.so / .dylib
# Crate standalone (NO en el workspace): mantiene gix fuera del binario clx.
cd "$(dirname "$0")/.."
cargo build --manifest-path drivers/gix-driver/Cargo.toml --release "$@"

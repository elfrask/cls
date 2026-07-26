#!/bin/bash
# Build: ecls — compila el ejecutor
cd "$(dirname "$0")/.."
cargo build --bin ecls "$@"

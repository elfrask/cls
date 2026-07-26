#!/bin/bash
# Build: ccls — compila el CLI principal
cd "$(dirname "$0")/.."
cargo build --bin ccls "$@"

#!/bin/bash
# Build: ccls-repl — compila el REPL
cd "$(dirname "$0")/.."
cargo build --bin ccls-repl "$@"

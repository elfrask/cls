#!/bin/bash
# Build: cpkg — compila el gestor de paquetes
cd "$(dirname "$0")/.."
cargo build --bin cpkg "$@"

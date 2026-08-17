#!/bin/bash
# Build: clxr
cd "$(dirname "$0")/.."
cargo build --bin clxr "$@"

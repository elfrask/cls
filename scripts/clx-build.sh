#!/bin/bash
# Build: clx
cd "$(dirname "$0")/.."
cargo build --bin clx "$@"

#!/bin/bash
# clx — CLS Toolchain
cd "$(dirname "$0")/.."
./target/debug/clx "$@"

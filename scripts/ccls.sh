#!/bin/bash
# ccls — CLS Language Compiler & Runner
cd "$(dirname "$0")/.."
./target/debug/ccls "$@"

#!/bin/bash
# ccls-repl — CLS REPL
cd "$(dirname "$0")/.."
./target/debug/ccls-repl "$@"

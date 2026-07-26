#!/bin/bash
# cpkg — CLS Package Manager
cd "$(dirname "$0")/.."
./target/debug/cpkg "$@"

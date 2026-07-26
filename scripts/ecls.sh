#!/bin/bash
# ecls — CLS Application Executor
cd "$(dirname "$0")/.."
./target/debug/ecls "$@"

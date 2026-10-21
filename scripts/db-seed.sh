#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
exec ./target/debug/clx run cls-repositories/tombstone/src/seed.clsx "$@"

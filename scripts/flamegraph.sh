#!/usr/bin/env bash

set -euo pipefail

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo is required" >&2
  exit 1
fi

if ! cargo flamegraph --help >/dev/null 2>&1; then
  echo "cargo-flamegraph is not installed. Run: cargo install flamegraph" >&2
  exit 1
fi

exec cargo flamegraph "$@"

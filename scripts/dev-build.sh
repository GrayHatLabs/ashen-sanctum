#!/usr/bin/env bash
# Desktop (x86_64 Linux / WSL) build. Usage: scripts/dev-build.sh [run] [game args]
set -e
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_TARGET_DIR="$HOME/.cache/ashensanctum-target"
cargo build --release
if [ "$1" = "run" ]; then
  exec "$CARGO_TARGET_DIR/release/ashensanctum" "${@:2}"
fi

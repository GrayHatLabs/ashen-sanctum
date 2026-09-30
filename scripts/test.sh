#!/usr/bin/env bash
# Headless checks: self-test (bot plays ~6 minutes) and snapshot frames in snapshots/.
set -e
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_TARGET_DIR="$HOME/.cache/ashensanctum-target"
cargo test --release 2>&1 | tail -5
cargo build --release
BIN="$CARGO_TARGET_DIR/release/ashensanctum"
"$BIN" --selftest
"$BIN" --snapshot snapshots
"$BIN" --snapshot snapshots/tall --tall

#!/usr/bin/env bash
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_TARGET_DIR="$HOME/.cache/ashensanctum-target"
cargo test --release 2>&1 | grep -E "test |result|error|panicked" 

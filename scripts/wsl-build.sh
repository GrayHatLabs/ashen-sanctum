#!/usr/bin/env bash
# Build helper used from Windows: wsl -d Ubuntu -e bash scripts/wsl-build.sh [cargo args]
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_TARGET_DIR="$HOME/.cache/ashensanctum-target"
cargo build --release "$@" 2>&1 | grep -v '^\s*Compiling' 
exit ${PIPESTATUS[0]}

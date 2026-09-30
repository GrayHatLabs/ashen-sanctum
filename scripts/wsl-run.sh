#!/usr/bin/env bash
# Run the release binary headless from Windows: wsl -d Ubuntu -e bash scripts/wsl-run.sh --snapshot snapshots
cd "$(dirname "$0")/.."
exec "$HOME/.cache/ashensanctum-target/release/ashensanctum" "$@"

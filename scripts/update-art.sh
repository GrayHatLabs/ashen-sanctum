#!/usr/bin/env bash
# Pull the latest generated art into the game: pack sheets in the art repo, embed them, rebuild.
# Usage (WSL): scripts/update-art.sh
set -e
cd "$(dirname "$0")/.."
ART="${ART_DIR:-/mnt/d/projects/AshenSanctum-art}"
(cd "$ART/tools" && python3 pack.py)
python3 scripts/import_art.py "$ART"
scripts/dev-build.sh

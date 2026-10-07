#!/usr/bin/env bash
# Cross-compile for aarch64 Linux handhelds (Anbernic RG35XX H etc.) and assemble
# a PortMaster-style port folder in dist/.
set -e
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_TARGET_DIR="$HOME/.cache/ashensanctum-target"
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc
export RUSTFLAGS="-L /usr/lib/aarch64-linux-gnu"
cargo build --release --target aarch64-unknown-linux-gnu

OUT=dist/ports
rm -rf "$OUT"
mkdir -p "$OUT/ashensanctum"
cp "$CARGO_TARGET_DIR/aarch64-unknown-linux-gnu/release/ashensanctum" "$OUT/ashensanctum/"
cp port/AshenSanctum.sh "$OUT/"
cp port/ashensanctum.gptk "$OUT/ashensanctum/"
# Menu art and metadata for the Ports list (PortMaster: port.json + cover/screenshot; EmulationStation: gameinfo.xml).
cp port/cover.png port/screenshot.png port/port.json port/gameinfo.xml "$OUT/ashensanctum/"
cp README.md "$OUT/ashensanctum/"
chmod +x "$OUT/AshenSanctum.sh" "$OUT/ashensanctum/ashensanctum"
(cd dist && rm -f AshenSanctum-aarch64.zip && python3 -m zipfile -c AshenSanctum-aarch64.zip ports/)
file "$OUT/ashensanctum/ashensanctum" || true
echo "Built dist/AshenSanctum-aarch64.zip"

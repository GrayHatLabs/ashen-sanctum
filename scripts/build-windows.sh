#!/usr/bin/env bash
# Cross-compile a standalone Windows build (SDL2 built in, no DLLs to ship) from WSL, and zip it
# into dist/AshenSanctum-windows.zip. Needs: cmake, mingw-w64, `rustup target add x86_64-pc-windows-gnu`.
set -e
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_TARGET_DIR="$HOME/.cache/ashensanctum-target"
export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc
export CC_x86_64_pc_windows_gnu=x86_64-w64-mingw32-gcc
export CXX_x86_64_pc_windows_gnu=x86_64-w64-mingw32-g++
export AR_x86_64_pc_windows_gnu=x86_64-w64-mingw32-ar
# SDL2's registry calls need advapi32, which the sdl2 crate doesn't link on its own.
export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUSTFLAGS="-C link-arg=-ladvapi32"
cargo build --release --target x86_64-pc-windows-gnu --features bundled

OUT=dist/AshenSanctum-windows
rm -rf "$OUT"
mkdir -p "$OUT"
cp "$CARGO_TARGET_DIR/x86_64-pc-windows-gnu/release/ashensanctum.exe" "$OUT/AshenSanctum.exe"
cp README.md THIRD_PARTY_LICENSES.txt "$OUT/"
(cd dist && rm -f AshenSanctum-windows.zip && python3 -m zipfile -c AshenSanctum-windows.zip AshenSanctum-windows/)
echo "Built dist/AshenSanctum-windows/AshenSanctum.exe and dist/AshenSanctum-windows.zip"

# CLAUDE.md — Ashen Sanctum

Project rules and reference. Read this before every major change.

## Platform and technology

- Isometric Diablo 2-style action RPG. Borrow ideas from Flare (flarerpg.org), but this is our own engine.
- Rust + SDL2, software framebuffer (no GPU code). Same toolchain as Elemental Legends.
- Framebuffer 640x360 on desktop (16:9), 640x480 on the ANBERNIC RG35XX H (`--tall`, the default on aarch64).
- Distribution: PortMaster-compatible aarch64 package (`scripts/build-handheld.sh`).
- Builds run in WSL **Ubuntu** (`wsl -d Ubuntu`); the default WSL distro (LocalDEV) is broken.
  From Windows: `wsl -d Ubuntu -e bash scripts/wsl-build.sh`, `scripts/wsl-run.sh <args>`, `scripts/wsl-test.sh`.
- Keep the handheld's performance budget in mind (the draw is ~1–2 ms on desktop x86).

## Controls (keep both schemes working)

- Mouse (D2 style): left click moves / attacks the hovered monster, right click casts at the cursor,
  Shift + left click casts in place, Q / E potions.
- Pad (twin-stick): left stick moves, right stick aims and casts, A / X / R1 / RT casts (auto-targets the
  nearest visible foe when the right stick is centred), L1 / LT health potion, Y mana potion, START
  confirms, SELECT + START quits.

## Art pipeline

- Art lives in `D:\projects\AshenSanctum-art` (PixelLab API, key in the `PIXELLAB_API_KEY` user env var;
  never print it). `tools/gen.py` generates, `tools/pack.py` packs sheets + `sheets/manifest.json`,
  `scripts/import_art.py` (in this repo) embeds them into `src/art_gen.rs` + `assets/art/*.bin`.
- Characters: 8-direction isometric, `create-character-with-8-directions` standard mode (1 generation),
  animations with `animate-character` v3 (1 generation per direction, ~3–6 min each, one job at a time).
  Request 5 directions (S, SE, E, NE, N); `pack.py` mirrors the other 3.
- Tiles: `create-isometric-tile`, 32 px (32x16 diamond). Walls are one block stacked 3x by `pack.py`.
- Anything missing falls back to the code-drawn sprites in `src/sprites.rs`.
- MVP art budget: ~400 generations (user-approved 2026-09-30). Log: `generated/ashen_usage.log`,
  `python tools/gen.py spent`.

## Development rules

- Before every major change: read this file and IMPLEMENTATION_STATUS.md, inspect the implementation.
- Verify with `scripts/test.sh` (unit tests, 6-minute bot self-test, snapshots) and look at the
  snapshot frames. Headless checks do not prove feel; say so rather than marking things played.
- Verify the handheld cross-build still succeeds.
- Update IMPLEMENTATION_STATUS.md and commit verified milestones.

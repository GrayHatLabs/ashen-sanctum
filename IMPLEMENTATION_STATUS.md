# Implementation Status — Ashen Sanctum

Last updated: 2026-09-30

## Verification levels

| Level | Meaning |
|---|---|
| **H** | Headless-verified: `scripts/test.sh` (unit tests + a bot playing 6 minutes through the real game loop, input, collisions and renderer) passes |
| **S** | Screenshot-reviewed: rendered frames from `--snapshot` inspected by eye |
| **D** | Desktop-played: run interactively in the SDL window on desktop (WSL) |
| **HW** | Verified on the ANBERNIC RG35XX H hardware |

Headless checks prove logic and render output. They do **not** prove feel, timing, audio, or
handheld performance. The desktop window has been launched (SDL video, renderer and audio open
without errors) but not played by a human yet. Nothing is HW yet.

## Builds

| Item | Status |
|---|---|
| Desktop build (`scripts/dev-build.sh`) | Builds clean, no warnings |
| Handheld cross-build (`scripts/build-handheld.sh`) | Succeeds: aarch64 ELF + `dist/AshenSanctum-aarch64.zip` |
| Running on RG35XX H | **Not yet verified (needs hardware)**. Desktop draw is ~1–2 ms per frame; the handheld's A53 should stay well under 16 ms but that's untested |

## MVP (2026-09-30)

| Feature | Status | Notes |
|---|---|---|
| Isometric renderer: 32x16 tiles, depth-sorted walls/actors, 640x360 wide / 640x480 tall | H S | Front walls (floor behind them) are cut down to a ledge, D2 style; walls in front of the mage dither see-through |
| Lighting: player light radius, fireball and explosion lights, additive fire | H S | Static base light map (camera follows the player) + per-light adds |
| Procedural dungeon: 14 rooms, 2-wide corridors, loops, pillars | H S | Unit test: every floor tile reachable, floors never touch the void (39 seeds) |
| Fireball: 5 mana, splash, burning DoT, knockback, hit recovery, scorch decals | H S | |
| Monsters: zombie (slow/tough), skeleton (fast/fragile); idle wander, LOS aggro, pack aggro, A* chase, wind-up swings | H S | |
| Loot: gold, health/mana potions; HUD globes, potions, skill slot, D2 monster name bar | H S | |
| Level cleared → descend (harder, bigger packs); death → restart | H | |
| Automap (Tab / SELECT) | H S | Explored walls + seen foes; every foe shows once 5 or fewer remain |
| Controls: mouse click-to-move / right-click cast; twin-stick pad with auto-aim | H (bot uses the stick path) | Mouse path needs a D check |
| Synth SFX (cast, boom, hit, hurt, death, swing, pickup, drink, descend) | built | Not listened to yet |
| Art: PixelLab 8-direction mage, zombie, skeleton, isometric floors + wall | S | Walk/cast/attack animations in progress (see art status) |

## Art status

- Generated so far: mage/zombie/skeleton 8-direction rotations, 2 floor tiles, 1 wall block, mage walk.
- Queued (5 directions each, mirrored to 8): mage cast, zombie walk + attack, skeleton walk + attack.
- Spend: `python D:\projects\AshenSanctum-art\tools\gen.py spent` (budget ~400).

## Next ideas

- Stairs/waypoints instead of "clear the level to descend", XP and level-ups, item drops with affixes,
  more skills (Frost Nova, Charged Bolt), a town, music.

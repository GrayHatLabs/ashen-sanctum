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
| Ember Bolt: free fallback when out of mana (3–5 dmg, single target, 0.4 s cast, no burn/stun) | H S | Unit test: fires with no mana, costs nothing, fireball returns with mana |
| Run / walk toggle with stamina (D2 style): walk 4.3, run 6.5 tiles/s; winded at 0 until 20 | H S | Unit test: run faster, drains, winded, recovers, toggle. Replaced the dash (user preference) |
| Food / hunger: drains 0.35/s (0.8 running); starving drains life, stops life regen, halves stamina regen. Apple/bread/roast (PixelLab) placed per level + monster drops; left on the floor when full | H S | Unit test: starving hurts, eating feeds, full leaves food, levels have food. Bot survives 6 min eating 7 times |
| Automap (Tab / SELECT) | H S | Explored walls + seen foes; every foe shows once 5 or fewer remain |
| Controls: mouse click-to-move / right-click cast; twin-stick pad with auto-aim | H (bot uses the stick path) | Mouse path needs a D check |
| Synth SFX (cast, boom, hit, hurt, death, swing, pickup, drink, descend) | built | Not listened to yet |
| Art: PixelLab 8-direction mage (idle/walk/cast), zombie + skeleton (idle/walk/attack), isometric floors + wall | S | 5 directions generated per animation, 3 mirrored. `pack.py` trims frames where PixelLab drifted and keys out a stray background |

## World expansion (2026-09-30)

| Feature | Status | Notes |
|---|---|---|
| Overworld 112x112: forests (noise), rocks, bushes, roads to 4 entrances, 30 roaming packs, wild food | H S | Unit test: every entrance reachable from town, NPCs not in walls, no monsters in town |
| Hollowmere: palisade with gates, 4 houses, stall, well, campfire; safe zone (no casting, no hunger, monsters give up) | H S | Front palisade cut low like dungeon front walls |
| NPCs: Elder (story), Gerta (shop), Aldric (heal), Rolf + 4 villagers (hints, villagers stroll) | H S | Talk by clicking or cast/confirm nearby; dialogue with pages and options (keys, pad, mouse) |
| Story: elder -> 3 Wardens -> seals -> Sanctum unsealed -> Ash King -> epilogue | H S | Unit test plays the whole quest line |
| 4 dungeons (2/2/3/3 floors), themes, stairs, levels persist, town portal after bosses | H S | Unit tests: stairs/boss per floor, entrance <-> floors <-> overworld round trip |
| Monsters: wolf, imp (flees when kin die), skeleton archer (ranged, keeps distance) | H S | Generated rotations; walk/attack animations queued |
| Bosses: Bone Warden (summons), Plague Warden (poison pools), Hex Warden (3-bolt volleys, blink, archer adds), Ash King (bolt spreads, ash nova, enrage + imps) | H S | Stand-in art (scaled/tinted) until their PixelLab sheets land |
| XP / character levels, seal power-ups, quest log, area names, boss bar, death -> wake in town (-10% gold), victory screen | H S | |

## Fire skills, step 1 (2026-10-01)

| Feature | Status | Notes |
|---|---|---|
| Skill points (1 per level, 1 per seal), ranks 1-10, tier gates, prerequisites | H | Unit tests: learning rules, save/load |
| Two slots (L primary / R secondary), 1-4 and R1 to pick, HUD slots + points button | H S | |
| Skill tree screen (K / hold SELECT / click the HUD); world pauses while open | H S | Keys, pad and mouse |
| Inferno (channelled cone), Fire Nova (ring + knockback), Warmth (passive regen) | H S | Unit tests for each |
| Fireball scales with its rank; Aldric respec for 50 x char level gold | H | Unit test |
| Old saves get the points they would have earned | H | |
| Step 2: Fire Wall (flame line, 0.2 s burn ticks), Blaze (fire trail while moving), Combust (detonates burning foes, scales with burn time, Fire Wall synergy); 3-tier tree | H S | Unit tests for each |

## Art status

- Complete for the MVP: 3 characters x 8 rotations, 7 animations, 2 floor tiles, 1 wall block.
- Weaker pieces: zombie north attack and skeleton south-east / north-east attack are trimmed
  (PixelLab turned the figure or morphed the sword late in the swing); the mirrored west-facing mage
  holds the staff in the other hand.
- Spend: about 45 generations for the whole MVP (well under the ~400 budget).
- World expansion art: 12 characters, 17 props, 5 overworld tiles generated; 15 animations queued
  (`tools/world_art.py chars`), imported as they finish.

## Next ideas

- Stairs/waypoints instead of "clear the level to descend", XP and level-ups, item drops with affixes,
  more skills (Frost Nova, Charged Bolt), a town, music.

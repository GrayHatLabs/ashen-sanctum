# Level files

Every map in Ashen Sanctum can be replaced by a hand-made level: each act's overland (with its town)
and every dungeon floor, all six acts. (The endgame's Ash Rifts are random by design.) Make them with the level editor (`tools/level-editor/index.html`).

## How the game finds levels

For each map the game uses the first of:

1. `levels/<name>.json` on disk, relative to where the game runs (`$ASHEN_LEVELS` overrides the
   folder). Edit and save, then re-enter the level: no rebuild needed.
2. The same file embedded in the binary. `build.rs` embeds every `levels/*.json` when you build,
   so the handheld zip ships with your levels.
3. The procedural generator.

A file that fails to load is reported on the console and the generator is used instead.
`levels/generated/` holds exported copies of the generated maps as starting points; the game
never loads from it.

| Level | File name | Id |
|---|---|---|
| Act 1: the Ashlands (overland + town) | `overworld` | `overworld` |
| The Bone Crypt, floors 1-2 | `bone_crypt_floor1`, `_floor2` | `dungeon:0:0`, `dungeon:0:1` |
| The Rotting Warrens, floors 1-2 | `rotting_warrens_floor1`, `_floor2` | `dungeon:1:0`, `dungeon:1:1` |
| The Hexed Catacombs, floors 1-3 | `hexed_catacombs_floor1` ... `_floor3` | `dungeon:2:0` ... `dungeon:2:2` |
| The Ashen Sanctum, floors 1-3 | `ashen_sanctum_floor1` ... `_floor3` | `dungeon:3:0` ... `dungeon:3:2` |
| Act 2: the Frostmarch (overland + town) | `frostmarch` | `frostmarch` |
| The Frozen Mines, floors 1-2 | `frozen_mines_floor1`, `_floor2` | `dungeon:4:0`, `dungeon:4:1` |
| The Howling Caves, floors 1-2 | `howling_caves_floor1`, `_floor2` | `dungeon:5:0`, `dungeon:5:1` |
| The Rime Temple, floors 1-3 | `rime_temple_floor1` ... `_floor3` | `dungeon:6:0` ... `dungeon:6:2` |
| The Glacier's Heart, floors 1-3 | `glaciers_heart_floor1` ... `_floor3` | `dungeon:7:0` ... `dungeon:7:2` |
| Act 3: the Mistwood (overland + town) | `mistwood` | `mistwood` |
| The Sunken Chapel, floors 1-2 | `sunken_chapel_floor1`, `_floor2` | `dungeon:8:0`, `dungeon:8:1` |
| The Gallows Catacombs, floors 1-2 | `gallows_catacombs_floor1`, `_floor2` | `dungeon:9:0`, `dungeon:9:1` |
| The Barrow Of Knights, floors 1-3 | `barrow_of_knights_floor1` ... `_floor3` | `dungeon:10:0` ... `dungeon:10:2` |
| Castle Vardak, floors 1-3 | `castle_vardak_floor1` ... `_floor3` | `dungeon:11:0` ... `dungeon:11:2` |
| Act 4: the Grinding Fields (overland + town) | `dominion` | `dominion` |
| The Foundry Of Souls, floors 1-2 | `foundry_of_souls_floor1`, `_floor2` | `dungeon:12:0`, `dungeon:12:1` |
| The Choir Engine, floors 1-2 | `choir_engine_floor1`, `_floor2` | `dungeon:13:0`, `dungeon:13:1` |
| The Archive Of Gears, floors 1-3 | `archive_of_gears_floor1` ... `_floor3` | `dungeon:14:0` ... `dungeon:14:2` |
| The Heart Of The Clock, floors 1-3 | `heart_of_the_clock_floor1` ... `_floor3` | `dungeon:15:0` ... `dungeon:15:2` |
| Act 5: the Sunken Reach (overland + town) | `deep` | `deep` |
| The Wreck Of The Sovereign, floors 1-2 | `wreck_of_the_sovereign_floor1`, `_floor2` | `dungeon:16:0`, `dungeon:16:1` |
| The Coral Cathedral, floors 1-2 | `coral_cathedral_floor1`, `_floor2` | `dungeon:17:0`, `dungeon:17:1` |
| The Midnight Trench, floors 1-3 | `midnight_trench_floor1` ... `_floor3` | `dungeon:18:0` ... `dungeon:18:2` |
| The Drowned Sanctum, floors 1-3 | `drowned_sanctum_floor1` ... `_floor3` | `dungeon:19:0` ... `dungeon:19:2` |
| Act 6: the Skyreach (overland + town) | `heavens` | `heavens` |
| The Broken Choir, floors 1-2 | `broken_choir_floor1`, `_floor2` | `dungeon:20:0`, `dungeon:20:1` |
| The Storm Spire, floors 1-2 | `storm_spire_floor1`, `_floor2` | `dungeon:21:0`, `dungeon:21:1` |
| The Wheel Of Eyes, floors 1-3 | `wheel_of_eyes_floor1` ... `_floor3` | `dungeon:22:0` ... `dungeon:22:2` |
| The True Sanctum, floors 1-3 | `true_sanctum_floor1` ... `_floor3` | `dungeon:23:0` ... `dungeon:23:2` |

## Commands

```bash
# Export the current maps (from your save's world seed, or --seed N) as starting points
scripts/dev-build.sh run --export-levels levels/generated

# Jump straight into a level to test it
scripts/dev-build.sh run --level bone_crypt_floor2

# Refresh the editor's catalog (themes, props, monsters, people, dungeons) after changing the game
scripts/dev-build.sh run --export-catalog tools/level-editor/catalog.js
python tools/level-editor/make_thumbs.py
```

The editor's palettes and checks come from `tools/level-editor/catalog.js`, which the game writes
(`levels::catalog_js`), so a new monster, prop or dungeon shows up in the editor after re-exporting it.
The full lists of kinds are in that file and in the editor's palettes.

## Format (version 1)

```json
{
  "version": 1,
  "id": "dungeon:0:1",
  "name": "THE BONE CRYPT - LEVEL 2",
  "theme": "crypt",
  "tier": 1.25,
  "width": 72, "height": 72,
  "tiles": ["   ###...", "..."],
  "ground": [],
  "start": [56.5, 60.5],
  "safe": null,
  "props": [{ "kind": "house1", "x": 46, "y": 52, "w": 4, "h": 4 }],
  "portals": [{ "kind": "up", "x": 56.5, "y": 60.5 }],
  "monsters": [{ "kind": "skeleton", "x": 40.2, "y": 12.8, "tier": 1.25 }],
  "npcs": [{ "kind": "elder", "x": 57.5, "y": 64.5 }],
  "items": [{ "kind": "apple", "x": 30.5, "y": 22.5 }]
}
```

- **id**: which map this replaces (must match the file name, see the table).
- **theme**: `overworld`, `crypt`, `warrens`, `catacombs`, `sanctum`; Act 2: `tundra` (the Frostmarch),
  `mines`, `icecaves`, `rime`, `glacier`; Act 3: `mistwood`, `chapel`, `gallows`, `barrow`, `castle`; Act 4:
  `dominion`, `foundry`, `choir`, `archive`, `clock`; Act 5: `deep` (the Sunken Reach), `wreck`, `reef`,
  `trench`, `drowned`; Act 6: `heavens` (the Skyreach), `seraph`, `spire`, `wheel`, `zenith` (art, lighting,
  weather and each act's rules: the tide, the wind and the open sky).
- **tier**: difficulty. Monsters' life and damage scale with it (a monster's own `tier` wins).
  Bosses without a `tier` use half the level's tier plus 0.5, like the generator.
- **tiles**: one string per row. `.` floor (walkable), `#` wall (dungeon wall; on an overland,
  the town wall), `o` blocked by a prop, space = void. On the Act 6 themes the void is open sky: walk
  (or get knocked) off the edge and you fall.
- **ground**: overland maps only, one string per row, `g` / `d` / `r`:

  | Theme | `g` | `d` | `r` |
  |---|---|---|---|
  | `overworld` | grass | dirt | road |
  | `tundra` | snow | frozen lake ice | snowy road |
  | `mistwood` | earth | glowing moss | mud road |
  | `dominion` | brass plate | verdigris | conveyor |
  | `deep` | sand | tide flats (flood at high tide: you wade, sea monsters swim) | boardwalk |
  | `heavens` | cloud marble | sky grass | chain bridge |
- **start**: where you arrive when there's no matching portal (and, on the overworld, where you
  wake after dying). Must be on a floor tile.
- **safe**: the town's safe zone `[x0, y0, x1, y1]`: no casting, no hunger, monsters give up.
- **props**: `x`, `y` = top-left tile of the footprint, `w` x `h` tiles, all marked `o`. Every kind
  is in the editor's Props palette with the footprint the generator gives it (entrance buildings 3x3
  or 4x3, houses 4x4, trees 1x1 ...). Stairs are drawn automatically for `up` / `down` portals.
- **portals**: tile centres (`x.5`). Kinds: `up`, `down`, `townportal`; `entrance0` ... `entrance23`
  (the dungeon doors, four per act on that act's overland, numbered as in the table); `pass0` ...
  `pass5`, the ways between acts: `passN` leads to Act N+1 (the Ashlands' pass north is `pass1`, the
  Grinding Fields' diving bell down to the Deep is `pass4`, the Deep's stair of light up is `pass5`);
  `dock0`, the Skyreach's airship docks (put two: the airship ferries you between them).
- **monsters**: every monster's art name, e.g. `zombie`, `frost_wolf`, `cultist`, `gearwraith`, `merrow`,
  `siren`, `ophanim`, `thunderbird`, and each dungeon's boss (`boss_bone` ... `boss_solanthos`). Old
  files may say `imp` for `goblin`. `crate`, `barrel` and `urn` are breakables.
- **npcs**: the town people of each act, e.g. `elder`, `merchant`, `captain`, `hunter`, `tally`, `ysolde`,
  `seraphine`, the jewelers `jeweler0` ... `jeweler5`, and villagers (`villager0`, `fisher1`, `diver2`,
  `deckhand0` ...).
- **items**: `apple`, `bread`, `roast`, `health_potion`, `mana_potion`, `gold<N>` (e.g. `gold25`).

## What a level needs (the editor checks these)

- Each overland: its four doors, its ways to the neighbouring acts, its story-giver and ideally its
  shops, its jeweler and a safe zone:

  | Overland | Doors | Ways | Story-giver | Shops |
  |---|---|---|---|---|
  | `overworld` | `entrance0..3` | `pass1` | Elder Maren (`elder`) | `merchant`, `healer`, `jeweler0` |
  | `frostmarch` | `entrance4..7` | `pass0`, `pass2` | Captain Brenna (`captain`) | `trader`, `seer`, `jeweler1` |
  | `mistwood` | `entrance8..11` | `pass1`, `pass3` | Abelard (`hunter`) | `widow`, `priest`, `jeweler2` |
  | `dominion` | `entrance12..15` | `pass2`, `pass4` | Tally (`tally`) | `vesper`, `oiler`, `jeweler3` |
  | `deep` | `entrance16..19` | `pass3`, `pass5` | Captain Ysolde Marrow (`ysolde`) | `nessa`, `coral`, `jeweler4` |
  | `heavens` | `entrance20..23` | `pass4`, two `dock0` | Seraphine (`seraphine`) | `bram`, `aurel`, `jeweler5` |

- Dungeon floors: stairs `up`; stairs `down` on every floor but the last; the dungeon's boss on
  the last floor. The act's tokens (seals, runes, sigils, keys, pearls, shards) and the ending
  depend on the bosses.
- Everything important reachable from the start over floor tiles (the airship docks count as linked).

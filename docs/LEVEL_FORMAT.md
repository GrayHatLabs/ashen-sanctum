# Level files

Every map in Ashen Sanctum can be replaced by a hand-made level: the overworld (with Hollowmere)
and each dungeon floor. Make them with the level editor (`tools/level-editor/index.html`).

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
| Overworld + Hollowmere | `overworld` | `overworld` |
| The Bone Crypt, floors 1-2 | `bone_crypt_floor1`, `bone_crypt_floor2` | `dungeon:0:0`, `dungeon:0:1` |
| The Rotting Warrens, floors 1-2 | `rotting_warrens_floor1`, `_floor2` | `dungeon:1:0`, `dungeon:1:1` |
| The Hexed Catacombs, floors 1-3 | `hexed_catacombs_floor1` ... `_floor3` | `dungeon:2:0` ... `dungeon:2:2` |
| The Ashen Sanctum, floors 1-3 | `ashen_sanctum_floor1` ... `_floor3` | `dungeon:3:0` ... `dungeon:3:2` |

## Commands

```bash
# Export the current maps (from your save's world seed, or --seed N) as starting points
scripts/dev-build.sh run --export-levels levels/generated

# Jump straight into a level to test it
scripts/dev-build.sh run --level bone_crypt_floor2
```

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
- **theme**: `overworld`, `crypt`, `warrens`, `catacombs` or `sanctum` (art and lighting).
- **tier**: difficulty. Monsters' life and damage scale with it (a monster's own `tier` wins).
  Bosses without a `tier` use half the level's tier plus 0.5, like the generator.
- **tiles**: one string per row. `.` floor (walkable), `#` wall (dungeon wall; on the overworld,
  the town palisade), `o` blocked by a prop, space = void.
- **ground**: overworld only, one string per row: `g` grass, `d` dirt, `r` road.
- **start**: where you arrive when there's no matching portal (and, on the overworld, where you
  wake after dying). Must be on a floor tile.
- **safe**: the town's safe zone `[x0, y0, x1, y1]`: no casting, no hunger, monsters give up.
- **props**: `x`, `y` = top-left tile of the footprint, `w` x `h` tiles, all marked `o`.
  Kinds: `tree_oak`, `tree_pine`, `tree_dead`, `rock1`, `bush1`, `house1`, `house2`, `tent1`
  (market stall), `campfire`, `well`, `ent_crypt`, `ent_warrens`, `ent_catacombs`, `ent_sanctum`
  (dungeon entrance buildings). Stairs are drawn automatically for `up` / `down` portals.
- **portals**: tile centres (`x.5`). Kinds: `up`, `down`, `entrance0` ... `entrance3`
  (overworld doors to the Crypt, Warrens, Catacombs, Sanctum), `townportal`.
- **monsters**: `zombie`, `skeleton`, `wolf`, `goblin` (old files may say `imp`), `archer`, `boss_bone`, `boss_plague`,
  `boss_hex`, `boss_ashking`.
- **npcs**: `elder`, `merchant`, `healer`, `guard`, `villager0` ... `villager3`.
- **items**: `apple`, `bread`, `roast`, `health_potion`, `mana_potion`, `gold<N>` (e.g. `gold25`).

## What a level needs (the editor checks these)

- Overworld: the four doors `entrance0..3`, Elder Maren (the story starts with her), ideally
  Gerta, Brother Aldric and a safe zone.
- Dungeon floors: stairs `up`; stairs `down` on every floor but the last; the dungeon's boss on
  the last floor (Bone Warden / Plague Warden / Hex Warden / Ash King). The seals and the
  ending depend on the bosses.
- Everything important reachable from the start over floor tiles.

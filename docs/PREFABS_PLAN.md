# Prefab pieces: hand-made parts in random maps (plan, 2026-10-09)

The user, 2026-10-09: random maps and the level editor shouldn't fight each other. Picked: **prefab pieces**, the
Diablo 2 way. You design rooms, camps, boss arenas and set pieces in the level editor; the game stitches them
into each map at random. Every playthrough stays different, but what you meet is hand-designed.

Status: **planned.** Built in phases, with a review after each.

## How it works today

- Every level is generated from a seed (the hero's world seed, the difficulty, and a new seed when an act is
  rekindled), so maps differ between heroes, difficulties and rekindlings.
- A level saved from the editor into `levels/` replaces the random one completely: that level is then the same
  every time. `levels/generated/` holds seed-7 exports, for reference and as starting points.
- `src/mapcheck.rs` builds every level over many seeds and all difficulties and checks that everything can be
  reached (stairs, doors, roads, bosses, people, pickups) and nothing stands in a wall. It runs with the tests
  (4 seeds, about 1,500 levels); `MAPCHECK_SEEDS=200` sweeps about 75,000.

## The prefab

A small JSON map made in the editor (the level format, cut down: `docs/LEVEL_FORMAT.md`):

- **Tiles and props:** floor, walls, scenery, as in a level.
- **Connectors:** marked openings on its edges where corridors or roads join it. The generator only places a
  prefab where its connectors line up with the map, so it can never seal anything off.
- **Markers** instead of fixed things: *monster pack* (the level's own kinds), *champion pack*, *elite*, *boss*,
  *chest*, *shrine*, *breakables*, *loot pile*, *light*. Each is rolled when the map is built, so a camp is
  laid out the same but holds different foes and treasure.
- **Tags:** which acts and themes it suits; what it is (*room*, *camp*, *crossroads*, *boss arena*,
  *set piece*); how often it may appear (weight, at most N per level); whether it may be turned and mirrored
  (four turns times a mirror gives eight variants from one design).

Files live in `prefabs/<act>/` and are built into the game like levels are (`build.rs`).

## The generator

- **Dungeons:** the room-and-corridor layout stays random. Each room is either filled from a prefab that fits
  its size and theme, or left plain. Boss floors take a *boss arena* prefab for the last room when there is one.
- **Overland areas:** prefabs are dropped as points of interest (camps, ruins, shrines, set pieces) on open
  ground away from the roads and exits, and the roads are routed past them.
- **Fallback:** with no prefab that fits, the generator builds the room or spot as it does today, so the game
  works with any number of prefabs, from none up.
- **Safety:** after placing prefabs the map goes through the same tidy pass and reachability check as now
  (`world::tidy`, `mapcheck`), and the map check also runs every prefab in many maps.

## The editor

- A **Prefab** mode: a small canvas (8x8 to 32x32), the same tile/prop/monster palettes, plus connector and
  marker tools, and the tags.
- **Preview:** drop the prefab into a random map of its act and roll seeds to see it in place.
- **Check:** warns if a connector can't reach the others, or a marker sits in a wall.
- Saving a whole level stays possible, for maps that should be the same every time (towns, special lairs).

## Phases

1. **Format and stamping:** the prefab file, loading and embedding, placing prefabs into dungeon rooms
   (connectors, turning and mirroring, markers), and the map check over prefabs. A few test prefabs written by
   hand.
2. **The editor's Prefab mode:** drawing, connectors, markers, tags, preview with a seed slider, the checks.
3. **Overland points of interest and boss arenas.**
4. **A starter set per act:** about 6-10 prefabs for each act's dungeons and wilds (camps, shrines, crossroads,
   arenas), made by Claude in the editor's format, for the user to change or add to.

## Open questions for later

- Should some prefabs be guaranteed per act (a set piece you always meet once), or all purely random?
- Should Act 7's surges be allowed to reshape prefab rooms, or should prefabs count as stilled ground?

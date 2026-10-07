# Breakables: crates, barrels and urns to smash (plan, 2026-10-06)

**Decided with the user:**
- **Where:** in dungeons, and a few outdoors (never in towns).
- **Loot:** a D2-style mix: often nothing, sometimes supplies, rarely something good.
- **Kinds:** a themed set for each act.
- **Breaking:** any hit breaks them: melee, spells and area skills, and clicking one attacks it like a monster.

## What the player sees

Smashable props stand in dungeon rooms and corridors, and near outdoor camps and ruins.
- **Breaking one:** one hit shatters it, with splinters (or shards, bone or brass, by act), a sound, and broken debris left on the floor.
- **Loot:** what it held spills out.
- **Clicking one:** walks you to it and attacks it, the same as clicking a monster.

| Act | Breakables |
|---|---|
| 1 Ashlands | wooden crate, barrel, clay urn |
| 2 Frostmarch | ice-crusted crate, frozen barrel, frost-bound urn |
| 3 Mistwood | upright coffin, bone urn, rotten barrel |
| 4 Mechanus | brass crate, clockwork box, oil drum |

## Loot (per break)

| Chance | Drop |
|---|---|
| 55% | nothing |
| 20% | gold (scaled by area level) |
| 10% | a healing or mana potion |
| 8% | food |
| 4% | a gem (graded by area, like monster gems) |
| 3% | an item (magic or better, magic find applies) |

These numbers are easy to tune; they live in one table.

## How it's built

**Breakables are a kind of monster with no mind.** That way every one of the 88 skills (projectiles, cones, novas, whirls, chains, holy fire, poison, thralls) already "hits" them with no per-skill work.
- **A `prop` flag in `mobs::Def`:** they don't move, attack, aggro, give XP, count as kills or quest progress, or become champions.
- **Things that skip them:** allies, thralls, charm, brands and the test bot's target picking.
- **Health:** 1 hit, with a little HP so damage-over-time finishes them too.
- **When one breaks:** debris is left as a floor decal, there's a particle burst themed by act, and the loot table above rolls.

**Placement** (in `world.rs` / dungeon generation):
- **Dungeons:** clusters of 2-5 in about a third of rooms, hugging the walls; now and then 1-2 in corridors.
- **Outdoors:** 1-3 beside camps, ruins and dungeon entrances.
- **Never** in towns, on portals or waypoints, or blocking the only path.
- **Deterministic per level seed,** so a level's breakables don't change on re-entry. Once broken they stay broken.

**Art:**
- 12 isometric props from PixelLab (about 24 generations): each intact plus a broken-debris version.
- Code-drawn stand-ins until the art is in.

**Tests:**
- they break from a projectile, a melee swing and an area skill;
- they never act or give XP;
- the drop rates over many breaks;
- none in towns or on portals;
- the bot doesn't get stuck on them.
- Snapshots: a room full of them in each act, and one shattering.

## Steps

1. The `prop` monster flag and the systems that ignore props (AI, XP, kills, allies, brands, bot).
2. Breaking: debris, particles, sound, loot table.
3. Placement in dungeons and outdoors, with the act sets.
4. Code-drawn stand-ins, tests, snapshots.
5. PixelLab art for the 12 props (intact + broken), packed in.
6. Docs, commit, push.

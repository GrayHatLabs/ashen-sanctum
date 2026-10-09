# Outdoor areas (plan, 2026-10-08)

**Decided with the user:**
- Diablo II scale: each act's outdoors becomes a town map plus 6-8 wild areas.
- The areas follow a main road with side branches.
- One waypoint per area.
- Act 1 first, then a review; Acts 2-6 after that.
- Side content phase 2 waits until the areas exist, because it will live in them.

**Today:** each act is one overland map, about 112x112 tiles, with the town in the middle and every dungeon
door on it.

**The change:** an act's outdoors becomes a chain of areas you walk between, like D2's Blood Moor, Cold
Plains and Stony Field:
- **Crossing:** a road runs off the edge of the map into the next area.
- **Waypoints:** each area has its own.
- **Dungeons:** each area holds one dungeon door, or none.
- **Side branches:** they hold the side content (super uniques, optional dungeons).
- **The way on:** the pass to the next act sits in the last area.

## Act 1: the Ashlands

```
                      [6] THE ASHEN STEPPE      Ashen Sanctum door, the pass north (Act 2)
                               |
                      [5] THE CINDER WASTE      no door: open ash plain, heavy packs
                               |
                      [4] THE CINDER HILLS      Hexed Catacombs
                               |
                      [3] THE ROTWOOD           Rotting Warrens (dense forest)
                               |
 [8] SKRAT'S GULCH  -- [2] THE BARROW FIELDS    Bone Crypt
                               |
 [7] SHEPHERD'S VALE -- [1] THE ASHLANDS        the Charnel Well (optional)
                               |
                         HOLLOWMERE (town)
```

| Area | Tier | Size | Door | Monsters |
|---|---|---|---|---|
| Hollowmere (town map) | - | 80x80 | - | none |
| 1 The Ashlands | 1.0 | 96x96 | the Charnel Well | wolves, goblins |
| 2 The Barrow Fields | 1.3 | 96x96 | the Bone Crypt | zombies, skeletons, wolves |
| 3 The Rotwood | 1.7 | 96x96 | the Rotting Warrens | zombies, goblins, wolves |
| 4 The Cinder Hills | 2.1 | 96x96 | the Hexed Catacombs | skeletons, archers, goblins |
| 5 The Cinder Waste | 2.6 | 112x80 | - | mixed, more packs |
| 6 The Ashen Steppe | 3.0 | 96x96 | the Ashen Sanctum, the pass north | mixed |
| 7 Shepherd's Vale (branch) | 1.2 | 64x64 | - (the Hollow Shepherd) | zombies, wolves |
| 8 Skrat's Gulch (branch) | 1.5 | 64x64 | - (Skrat One-Ear, Gerta's cart) | goblins, wolves |

About 5x today's outdoor space in Act 1.

## Acts 2-6 (built 2026-10-08)

Each act's old overland is now its **town map**:
- **Kept:** the town, its lake or mist or clocks or tides or sky islands, the way in from the previous act,
  and the monsters around it.
- **Moved out:** the four dungeon doors and the way on to the next act. The road out to area 1 takes the
  place of the old way on (`areas::strip_town`).

Each act then has 5 areas on the main road and 1 side branch:

| Act | Areas (main road) | Branch |
|---|---|---|
| 2 Frostmarch | Frozen Shore (Mines), Howling Tundra (Caves), Rime Woods (Temple), White Waste, Glacier's Edge (Glacier's Heart, the mist road east) | Raiders' Fjord |
| 3 Mistwood | Blighted Fields (Chapel), Gallows Moor (Gallows), Hollow Wood, Barrow Hills (Barrow), Castle Approach (Castle Vardak, the gear gate) | Witch's Bog |
| 4 the Clockwork Dominion | Gearfields (Foundry), Piston Flats (Choir Engine), Cogworks, Archive Stacks (Archive), Clockface Plain (Heart of the Clock, the diving bell) | Scrapyard |
| 5 the Deep | Kelp Shallows (Wreck), Coral Gardens (Cathedral), Bone Reef, Trench Rim (Trench), Abyssal Plain (Drowned Sanctum, the stair of light) | Sunken Galleon |
| 6 the Heavens | Broken Steps (Broken Choir), Stormfields (Storm Spire), Fallen Gardens, Halo Isles (Wheel of Eyes), the Zenith (True Sanctum) | Drifting Isles |

**Each act's look:**

| Act | Trees | Rocks and bushes |
|---|---|---|
| 2 | snow pines | snow rocks, ice crystals |
| 3 | mist pines, twisted trees | gravestones, glow-shrooms |
| 4 | gear towers, gas lamps | cog piles, steam vents |
| 5 | kelp | coral |
| 6 | cloud trees, marble ruins | angel statues, sky lamps |

- **The Heavens' areas** are islands in open sky, joined by chain bridges.
- **The tide** floods the flats on every Act 5 outdoor map, and **the wind** blows on every Act 6 one.

## How it works in code (`src/areas.rs`)

- **Ids:**
  - `LevelId::Area(act, n)` is a wild area.
  - The act's town map keeps its old id (`LevelId::Overworld` for Hollowmere).
  - `PortalKind::Exit(n)` leads to area `n` of the same act (0 = the town).
- **The area table:** `AREAS` gives, for each area:
  - its name and tier;
  - its size;
  - its exits (an edge, a spot along it, and where it leads);
  - its dungeon doors and the pass;
  - its monsters and a look (forest, dead trees, ash and dirt, rocks).
- **Arrivals:**
  - Arriving from another area puts you at the exit back to it.
  - Leaving a dungeon puts you at its door, in the area that holds it (`dungeon_home`).
  - Coming back from the next act lands you at the pass.
- **The waypoint menu:** it lists this act's waypoints and the other acts' towns.
- **The test bot** follows the area graph to the area that holds the dungeon it wants (`route`).
- **The level editor and level files:** each area is a level of its own (`area:0:3`, `barrow_fields.json`).

## Order

1. **Act 1** (this step): the areas, Hollowmere's town map, routing, waypoints, the bot, saves, the
   editor catalog, tests and snapshots. Then a review.
2. **Acts 2-6:** each act's overland split the same way, with its own area plan.
3. **Side content phase 2+** then lives in the new areas.

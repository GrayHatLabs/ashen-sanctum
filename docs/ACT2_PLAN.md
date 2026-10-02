# Act 2: The Frostmarch

Decided with the user on 2026-10-02: it unlocks after the Ash King, is the size of Act 1, and the
boss is a white dragon. The feel is Icewind Dale: a frozen frontier, a lakeside town, barbarians,
yetis, and a dragon under the glacier.

## Story

With the Ash King dead, his fire no longer holds back the cold. A mountain pass opens north of the
Ashlands. In the frozen frontier town of **Kaldholm** on the lake, **Captain Brenna** explains:
**Vorthrax the Rime Wyrm**, an ancient white dragon, has woken beneath the glacier. Three
**Frost Heralds** guard the **Frost Runes** that seal the way to her lair. Slay them, take the
runes, open the **Glacier's Heart** and end the wyrm.

## World

- **The Frostmarch** (overland): snow, snowy pines, a frozen lake beside Kaldholm, blizzard
  snowfall, roaming frost wolves, barbarian raiders, yetis.
- **Kaldholm**: a log palisade and longhouses.
  - **Captain Brenna**: the story.
  - **Old Sigurd**: a fur trader with the same goods as Gerta, plus gear.
  - **Mother Ylva**: a seer who heals and resets skills.
  - Ice fishermen.
- **Dungeons**, entered from the Frostmarch:
  1. **The Frozen Mines** (2 floors, raiders and ice trolls): the **Frost Giant Overseer** slams
     the ground and hurls ice boulders.
  2. **The Howling Caves** (2 floors, frost wolves and yetis): the **Yeti Matriarch** roars,
     calls her brood, and shakes icicles loose from the ceiling.
  3. **The Rime Temple** (3 floors, ice wraiths and raiders): the **Rime Witch** fires ice bolt
     volleys, freezes the floor, blinks and raises wraiths.
  4. **The Glacier's Heart** (3 floors, sealed until you hold the three runes): **Vorthrax the
     Rime Wyrm**.
     - Frost breath cone.
     - Icicles falling from the ceiling.
     - A wing gust that knocks you back.
     - Enrages at half life.
- Waypoints in Kaldholm and on every floor. A pass connects the two acts.

## Mechanics

- **Chill**: frost attacks slow your movement and casting for a moment. You're tinted blue and
  the HUD says CHILLED.
- **Cold creatures** (everything in Act 2) take 25% more fire damage. Ice trolls regenerate
  unless they are burning.
- Nightmare and Hell are now offered after the dragon, not after the Ash King.
- Act 2 monsters are stronger than the Ash King's Sanctum (tier 3.4 to 4.8).

## Art (`tools/act2_art.py` in the art repo)

- 13 characters:
  - 5 monsters.
  - 4 townsfolk.
  - 4 bosses: frost giant, yeti matriarch, Rime Witch, white dragon.
- 19 animations.
- 8 snow / ice tiles.
- 11 props: snowy pines, longhouses, fur stall, ice crystals, 4 entrances.

Until a sheet lands, the game uses tinted stand-ins of the Act 1 art.

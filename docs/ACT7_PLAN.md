# Act 7: The Churning Chaos (draft plan, 2026-10-08)

The user, 2026-10-08: "plan out an Act 7 in the realm of chaos, like the D&D planar realm."

The model is the planar idea of a **realm of raw chaos** (D&D's Limbo): a soup of the elements that never
holds a shape unless a mind holds it still. Everything here is our own invention, with no D&D names: no
slaadi, no githzerai.

Status: **being built in stages, with a review after each.**
- **Stage 1 (done 2026-10-08, `src/chaos.rs`):** Stillhold (Abbot Quiet, Sister Ferro, Brother Hush, the
  Cutter, monks), six areas (the Breach, the Drift of Worlds, the Elemental Tangle, the Spawning Mire, the
  Shattered Monastery, the Eye of the Churn), the way down through a crack in the Zenith once Solanthos is
  ended, chaos surges, anchor stones, the stillness meter, and lava, ice and water ground. Monsters are
  borrowed from earlier acts for now; the art is stand-ins (chaos-stone is graded stone).
- **Stage 2 (done 2026-10-08, `src/churnfolk.rs`):** chaos toads (red leap, blue frost, green takes your
  shape, grey champions), the Unmade, chaos knights (charge and slam), riftmaws (swallow you and spit you out
  elsewhere), chaos matter (splits twice), and surges that recolour toads and reroll champions' powers. The
  herald dungeons and their Anchor Keys: the Unfinished Cathedral (the Breach; its floors surge too) with the
  Architect of Nothing (walls you in, throws bricks), the Toad King's Warren (the Spawning Mire) with
  Grumbleguts the Many-Hued (red, then blue, then green by thirds of its life), and the Hall of Mirrors (the
  Shattered Monastery) with the Mirror Abbot (your own class's attacks, and copies of you). Art: OpenAI for the
  toad, the chaos matter, the Architect, the Mirror Abbot and the three entrances; PixelLab for the chaos
  knight. Bot survey: all 8 heroes take the three keys in 7-10 minutes with no deaths.
- Stage 3: Ylgrath and the Stillpoint, the story, and Nightmare moved to after Act 7.
- Stage 4: side content, the extras, art and music.

## The user's picks (2026-10-08)

- **Story:** Act 7 comes after Act 6. Solanthos's light was the lid on the chaos; Nightmare opens after Act 7.
- **Shifting ground:** whole rooms reshape (walls melt and rise, floors turn to lava, ice or water). Lit
  anchor stones stop it for good around them.
- **Stillness meter:** yes.
- **Extras:** all four (Chaos Roulette, the Wandering Room, Gravity Wells, Echo Hunt).
- **The user's own idea: Ylgrath can only be destroyed with an object from Mechanus (Act 4).**
  - **How it plays (agreed with the user, 2026-10-08):**
    1. **First fight, without the Stillpoint:** you beat him and he comes apart. He drops a fair reward (a
       rare, some gold, his first lore page) but no uniques and no ancients.
    2. While he pulls himself back together he gloats, and tells you what can unmake him: "only the first gear
       of the Clockmaker could hold me still... and it's lost in Mechanus." Then he flees deeper into the Eye.
    3. **The Eye seals behind him.** His chamber stays shut until you carry the Stillpoint, so leaving and
       coming back does not bring him back: nothing to farm. The quest log points you to Mechanus.
    4. Back in Act 4 by waypoint, a sealed spot (deep in the Heart of the Clock, or behind the Timeless
       Vault) has opened since his hint. It holds **the Stillpoint**, the Clockmaker's first and perfect gear.
    5. **The real fight:** with the Stillpoint, the Eye opens and he fights at full strength. At the end the
       Stillpoint locks him into one shape and he truly dies: the full boss loot (his uniques), then the
       ending, then Nightmare.
    6. Afterwards he is like every act boss: the Rekindling Brazier can bring him back for normal boss loot.
  - In Nightmare and Hell the Stillpoint has to be found again (it resets with the difficulty, like quests).

## Where it fits

- **Unlock:** after Solanthos (Act 6).
  - Solanthos was the burnt-out god; his light was the lid on the chaos.
  - With him dead the light goes out, and the Ashen Sanctum's floor cracks open on the Churn below.
  - Act 7 is what the gods were holding back. This makes Ashen Sanctum the story of the lid, and Act 7 the
    thing under it.
- **Nightmare and Hell** would then open after Act 7's final boss instead of after Act 6.
- Level range: tier about 14 to 16, right after Act 6's 12-13.6.

## Town: Stillhold

- A monastery that floats in the chaos, held together only by its monks' unbroken meditation.
- Its walls flicker if you stray to the edge of town.
- **NPCs:**
  - **Abbot Quiet**, the quest-giver, who has not spoken aloud in forty years. His words appear as text.
  - **Sister Ferro**, a smith who forges with "stilled" chaos-steel. Sells like Gerta and Bram did.
  - **Mad Orrin**, a gambler-merchant. He sells chaos-rolled items: the mods are random, and so is the price.
  - **The Stranger**, a version of the hero from another timeline. Rerolls one item affix for gold (an
    enchanter).
  - **Waypoint and stash:** the stash is a box that is always slightly different on the outside.

## Overland: the Churn (6 areas, each with a waypoint, like Acts 1-6)

1. **The Breach:** the crack below the Sanctum. Ash and broken marble fall into the chaos; it is the gentlest
   area.
2. **The Drift of Worlds:** chunks of every earlier act float here and stitch together: Ashlands ground next to
   Frostmarch ice next to reef coral.
   - This reuses our existing tile art.
   - Monsters come from all six acts, with chaos mods.
3. **The Elemental Tangle:** fire, ice, storm and water zones that **swap places** every minute or so.
4. **The Spawning Mire:** a bubbling swamp where the chaos toads breed (a branch area).
5. **The Shattered Monastery:** the ruins of Stillhold's twin, which lost its focus and fell into madness.
6. **The Eye of the Churn:** the calm, terrible centre; the way to the final dungeon.

## The act's rule: chaos and stillness

Each act has one signature rule (Act 3 the mist, Act 4 the clock-laws, Act 5 the tides, Act 6 the wind).
Act 7's is **instability**:

- **The ground reshapes.**
  - Every 40-60 seconds a **chaos surge** warns, then rerolls chunks of the map around you: walls melt, new
    ones rise, floors become lava, ice or water.
  - Paths that were there are gone.
- **Anchor stones.**
  - Scattered stone obelisks. Touch one and it lights, and the land within ~10 tiles stops shifting for good.
  - They give you a safe camp and are a reason to fight your way to them.
- **Chaos-touched monsters.**
  - Each surge can reroll a monster's modifiers (fast becomes fire-enchanted, and so on).
  - A monster's colour shows its current mods.
- **A Stillness meter** (a player resource in this act, optional):
  - It fills by standing still or near anchors.
  - Spend it to **force the next surge to skip you**.
  - Or "will" the ground to stay as it is for a while.

## Monsters (new art)

- **Chaos toads**, the signature monster: big, intelligent, toad-like brutes in four colours with four
  behaviours.
  - Red: they leap and slam.
  - Blue: they cast frost.
  - Green: they shapeshift into a copy of you.
  - Grey: champions with swords.
  - (Our own design, inspired by the classic chaos-frog idea, with a new name.)
- **The Unmade:** half-formed creatures, a mix of two earlier monsters. Their art could be generated as
  mashups.
- **Chaos knights:** armoured order-zealots who went mad trying to impose law; the opposite of Act 4.
- **Riftmaws:** mouths that open in the floor, pull you in, and spit you out somewhere else on the map.
- **Echoes:** shadowy replays of earlier bosses, weaker but with their signature moves.
- **Mindless matter:** blobs of raw chaos that split into smaller blobs when hit.

## Dungeons and heralds (the relics are three **Anchor Keys**)

1. **The Unfinished Cathedral** (the Breach): a church that builds and unbuilds itself as you walk. Boss: **the
   Architect of Nothing**, who raises walls to trap you.
2. **The Toad King's Warren** (the Spawning Mire): boss **Grumbleguts the Many-Hued**, a giant chaos toad that
   changes colour, and so its behaviour, at each third of its life.
3. **The Hall of Mirrors** (the Shattered Monastery): boss **the Mirror Abbot**, the twin monastery's fallen
   abbot. He fights with copies of your own skills.
4. **The Eye of the Churn**, the final dungeon (3 floors): boss **Ylgrath, the Unshaped**. He can only be
   destroyed while you carry **the Stillpoint** from Mechanus (see the user's picks above).
   - It is chaos given a will.
   - It takes the shape of a different earlier act boss each phase (the Butcher's cleaver, the Frost Jarl's ice,
     Vardak's bats, the Clockmaker's gears, the Leviathan's tide, Solanthos's light).
   - Its last form is its own: a storm of every element.
   - Afterwards: the true ending (the hero chooses to become the new lid, or to seal it), then Nightmare.

## Side content (same pattern as Acts 1-6)

- **Optional dungeon: the Probability Vault.** A treasury where every chest is a gamble: it might be gold, a
  mimic, or a unique. Boss: **the Dice-Saint**.
- **Side quests:**
  - **Abbot Quiet's Lost Voice:** find the three words he gave up. Reward: a skill point.
  - **The Other You:** the Stranger asks you to kill the version of them that went wrong. Reward: a respec.
  - **Ferro's Stilled Steel:** bring chaos ore from three anchors. Reward: a socket.
- **Extras (pick any):**
  - **Chaos Roulette:** a shrine that rolls a random blessing or curse for 3 minutes.
  - **The Wandering Room:** a whole room that teleports around the act; catch it for loot.
  - **Gravity Wells:** spots where "down" changes direction and projectiles curve.
  - **Echo Hunt:** one random earlier boss's echo per playthrough, with its unique.
- **Super uniques:** three named toads (one per colour), a chaos knight commander and a riftmaw queen.
- **Lore:** five pages about the gods, the lid and Stillhold's founding.

## Systems and art

- **Tech:**
  - The surge needs a "reroll this chunk" pass on the area map, kept away from the hero, anchors, waypoints
    and doors.
  - The map system already regenerates whole levels, so this is the main new piece.
- **Art:**
  - A new tile set: swirling purple-gold chaos ground, solid colour-bleed.
  - About 6 monsters plus 4 bosses, Stillhold's buildings and NPCs, anchors and props.
  - PixelLab for monsters and props, OpenAI turnarounds for the big single-image bosses (like the Junk Golem
    and Old Barnacle).
  - Size: about Act 5-6's art budget.
- **Music:** a shifting track that changes key at each surge.

## Questions for the user

1. Story: does Act 7 come **after Act 6's ending** (Solanthos was the lid), or is it a separate,
   post-game-only realm?
2. How strong should the shifting ground be? Pick from: whole rooms change; only the floor types change; or
   it only happens near "surge storms".
3. Should the Stillness meter be in, or should the anchor stones alone be enough?
4. Which of the extras do you want?

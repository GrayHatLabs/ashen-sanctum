# Side content (plan, 2026-10-08)

**Decided with the user:** side content for every act; plan it all, build it in phases, stop for a review
after each phase.

**What exists today:**
- each act's main quest: three herald bosses, their tokens, then the act boss;
- champion and elite packs (D2 style, random modifiers and names);
- breakables (crates, barrels, urns, the rare gilded chest), the stash, waypoints;
- the endgame: rekindling, rifts, ancients, embers.

**What's missing:** anything optional. Every dungeon is on the critical path, and nothing in the world is
there just to be found.

**Changed while building phase 1:**
- The game has no resistances, so the "+10% resists" rewards became **wards**: -5% damage taken for good,
  each (up to six).
- Gerta pays a rare item and gold instead of a shop discount.
- The optional dungeons are always open (like D2's Den of Evil); killing the boss before taking the quest
  still counts, and the reward waits for you.
- Ambushes: two packs, each led by one champion (six champions at once were too much early in Act 1).

**Goal:** each act gets a few things worth leaving the road for: an optional dungeon with a quest and a real
reward, two side quests, named monsters with their own loot, and things that happen while you explore.
Rewards follow Diablo II: skill points, permanent life/resist bonuses, sockets, a free rare item, a respec.
Short and pad-friendly, so it works on the handhelds.

## 1. The systems (shared by every act)

### Shrines
Glowing altars, 1-2 per dungeon floor and 2-3 per overland. Click one for a 90-second blessing:

| Shrine | Effect |
|---|---|
| Armor | -25% damage taken |
| Combat | +30% damage |
| Mana | mana regenerates 3x |
| Experience | +40% XP |
| Refill | life and mana restored at once |
| Skill | +1 to all skills |
| Gem | upgrades a gem in your bag (or drops one) |
| Haste | +25% move and cast speed |

- **Art:** one shrine prop per act style (6 props).
- **Look:** used shrines go dark.
- **HUD:** an icon under the orbs shows the blessing and its timer.

### Super uniques
Named monsters with a fixed home, fixed modifiers and their own dialogue line, like Bishibosh or Rakanishu.
- Each one is a base monster:
  - enlarged ×1.25;
  - tinted and lit with an aura;
  - with its own name and two or three champion modifiers.
- **Art:** no new art needed.
- **Loot:**
  - always drops a rare, plus gold;
  - each has a small chance at a unique tied to it.
- **Tracking:** they're marked on the map once seen, and the journal lists the ones you've slain.

### Side quests
- **Engine:** a small quest engine next to the main story (`sidequest.rs`):
  - quest state saved per difficulty;
  - lines in the quest log;
  - a town NPC who gives the quest and pays the reward.
- **Rewards:** a skill point, +20 life, +10% to all resists, a socket punched into an item, a free rare
  item for your class (the "imbue"), a free respec, gold.
- **Feedback:** a completion banner and sound.

### Optional dungeons
One per act, 1-2 floors:
- a door on the overland, locked or hidden until its quest is given;
- a theme variant of that act's dungeons, with tint and lighting changes, so it needs little art;
- a boss of its own at the bottom;
- fully editable in the level editor (they join the catalog).

### Events
Random encounters on overlands and in dungeons. At most one is active at a time.

| Event | What happens |
|---|---|
| Ambush | the ground shakes; three champion packs close in from the dark |
| Cursed chest | open it and survive 30 s of waves; the chest pays out by how many you killed |
| Hoarder | a fleeing gold-thief (each act has its own: goblin, ice imp, grave rat, clockwork crow, crab, gilded cherub); catch it before it escapes and it bursts into gold and gems |
| Fallen adventurer | a corpse with a pack to loot and a note (lore) |
| Wandering peddler | a rare merchant on the overland selling two rare items for gold |

### Lore and the journal
- **Pages:** each act hides five lore pages:
  - in optional dungeons;
  - on fallen adventurers;
  - on super uniques.
- **Journal:** a new tab in the pause menu, listing:
  - the pages you've found;
  - side quests, done and open;
  - super uniques slain;
  - shrines used.
- **Rewards:** every page gives a little XP, and all five pages of an act give a permanent +5% XP for that
  act.

## 2. The acts

### Act 1: the Ashlands (Hollowmere)
- **Optional dungeon: the Charnel Well** (2 floors), under the dry well outside Hollowmere.
  - **Quest:** *The Well Runs Red*, from Brother Aldric. Something below poisons the water.
  - **Boss:** **the Well-Witch**, a hag who calls up plague zombies.
  - **Reward:** a skill point (D2's Den of Evil).
- **Side quest: Gerta's Caravan.**
  - Gerta's supply cart was raided on the Ashlands road.
  - Find the wreck and follow the trail to **Skrat One-Ear**, a goblin chief with his gang.
  - **Reward:** Gerta's shop is 15% cheaper, and she offers one free rare item.
- **Side quest: The Hollow Shepherd.**
  - A villager's flock was turned to zombies.
  - Kill the **Hollow Shepherd** (a super-unique zombie) in the old pens.
  - **Reward:** +20 life.
- **Super uniques:**
  - Skrat One-Ear (goblin, fast + extra strong);
  - Old Bonejaw (skeleton archer captain, multishot + cold);
  - the Hollow Shepherd (zombie, cursed + fire enchanted).

### Act 2: the Frostmarch (Kaldholm)
- **Optional dungeon: the Icebound Longship** (2 floors), a raider ship frozen into the lake ice.
  - **Quest:** *Sigurd's Axe*, from Old Sigurd. His father's axe went down with the ship.
  - **Boss:** **Hrolf Ice-Beard**, a raider king.
  - **Reward:** Sigurd punches a socket into any item.
- **Side quest: The Lost Patrol.**
  - Captain Brenna's three scouts are frozen in ice in the Frostmarch, guarded by ice wraiths.
  - Kill the wraiths to thaw each scout.
  - **Reward:** +10% cold resistance for good, and Brenna's horn (a rare).
- **Side quest: The Frozen Bride.**
  - Mother Ylva asks you to lay a weeping ice wraith to rest at the lake shrine.
  - **Reward:** a free respec.
- **Super uniques:**
  - Grimfang the White (winter wolf, fast + cold aura);
  - Hrolf Ice-Beard (as boss);
  - the Frozen Bride (ice wraith, teleports + mana burn).

### Act 3: the Mistwood (Mournhold)
- **Optional dungeon: the Gravedigger's Cellar** (2 floors), under Widow Kasia's cottage.
  - **Quest:** *A Husband's Grave*, from Widow Kasia. Her husband's grave is empty.
  - **Boss:** **the Gravedigger**, a giant ghoul with a shovel ground-slam.
  - **Reward:** +20 life and the husband's ring (a rare).
- **Side quest: The Bells of Mournhold.**
  - Father Lucian needs the three bell-shrines in the Mistwood rung to silence the banshee chorus.
  - Each bell is guarded.
  - **Reward:** a skill point.
- **Side quest: The Pale Huntsman.**
  - Abelard's rival hunter became a werewolf.
  - Track him by his kills.
  - **Reward:** Abelard's longbow charm (+% damage).
- **Super uniques:**
  - the Pale Huntsman (werewolf, berserk + fast);
  - Sister Mournwail (banshee, extra strong + lightning);
  - Blackmoor the Gibbet-Hanged (cultist, spectral hit).

### Act 4: the Grinding Fields (the Last Escapement)
- **Optional dungeon: the Scrapheap Labyrinth** (2 floors), a maze of conveyor belts and crushers.
  - **Quest:** *Oil for the Saint*, from Brother Piston. The holy oil was stolen by scrap-scavengers.
  - **Boss:** **the Junk Golem**, which rebuilds itself from scrap twice.
  - **Reward:** Madame Vesper's imbue (a free rare for your class, ilvl +5).
- **Side quest: Tally's Count.**
  - Three ledger-gears were stolen by clockwork crows.
  - Chase the crows across the overland; they flee.
  - **Reward:** gold, and Tally's shop is 15% cheaper.
- **Side quest: The Grand Inquisitor.**
  - Halvane, a heretic inquisitor, preaches the clock-law in the gear fields.
  - Defeat him and his zealots.
  - **Reward:** +10% to all resists.
- **Super uniques:**
  - Mainspring (boiler brute, stone skin + fire);
  - Grand Inquisitor Halvane (inquisitor, holy fire + conviction);
  - Tick-Tock Jack (spring-heeled Jack, teleports + fast).

### Act 5: the Sunken Reach
- **Optional dungeon: the Pearl Grotto** (2 floors), a sea cave whose entrance is only open at low tide.
  - **Quest:** *The Black Pearl*, from Nessa the Pearl-Diver.
  - **Boss:** **Old Barnacle**, a giant shellguard.
  - **Reward:** a skill point, and the black pearl (a gem that fits any socket).
- **Side quest: The Ghosts of the Sovereign.**
  - Captain Ysolde's drowned crew walk the reef.
  - Lay three named drowned sailors to rest; each gives a last line.
  - **Reward:** +20 life and Ysolde's cutlass (a rare).
- **Side quest: The Siren's Price.**
  - Brother Coral's apprentice was lured away by **Lirael**, a siren.
  - Rescue him before the tide comes in.
  - **Reward:** +10% lightning resistance for good.
- **Super uniques:**
  - Bosun Krake (drowned sailor, cursed + cold);
  - Lirael (siren, mana burn + teleport);
  - Old Barnacle (as boss).

### Act 6: the Skyreach
- **Optional dungeon: the Fallen Observatory** (2 floors), a broken star-tower on its own island, reached by
  the airship.
  - **Quest:** *The Star Chart*, from Sister Aurel.
  - **Boss:** **the Astronomer**, a fallen seraph who throws constellations.
  - **Reward:** a skill point and a free respec.
- **Side quest: Bram's Lost Cargo.**
  - A storm scattered Quartermaster Bram's cargo across the islands.
  - Recover three crates; the wind is the hazard.
  - **Reward:** gold and a rare.
- **Side quest: The Weeping Seraph.**
  - A seraph statue weeps light.
  - Bring it the three tears dropped by the super uniques of the sky.
  - **Reward:** +10% to all resists.
- **Super uniques:**
  - Choirmaster Ezekar (sun-zealot, holy fire + extra strong);
  - Stormwing (thunderbird, lightning enchanted + fast);
  - the Astronomer (as boss).

## 2b. Added with the user (2026-10-08)

- **The Ember Wyrm's hoard** (Act 1, fixed every playthrough; `src/dragon.rs`). Vaurath, a fire dragon,
  sleeps on its gold in a cave in Emberpeak Pass (area 9, north off the Cinder Hills). It's a heist:
  - Gold goes into a **hoard sack**. The sack slows you (up to 25%) and is banked only when you leave the
    cave; if you die in there, it goes back on the heap.
  - **Noise** wakes the dragon: grabbing gold (+22), casting near it (+5 each), standing close
    (+14/s within 5 tiles, +4/s within 10), and running nearby (+6/s). Being quiet lets it fade (-5/s).
    Wounding it wakes it at once.
  - Awake, it hunts you through the cave with fire breath (burning pools) and a tail sweep.
  - Come back later and it's asleep again, at full health.
  - Killing it is possible: 9000 base life, far beyond Act 1. Its unique is EMBERSCALE.
  - Captain Rolf's dare: get out with 500 gold or more, for a ward.
- **Random errands** (every playthrough; `src/errands.rs`).
  - Each outdoor area rolls one task, and an act that is still one overland rolls two. Rolls depend on the
    hero's world seed and the difficulty, and no kind repeats within an act until all eight have come up.
  - The eight kinds: a bounty on a named champion, a cull of N of one monster, a captive to free from
    guards, 5 herbs to gather, a lost heirloom on a corpse, a bone totem raising the dead, a ward stone
    held through 3 waves, and a treasure map then the X to dig at.
  - Twists: about 1 in 6 is timed (4-6 minutes), about 1 in 6 pays double gold.
  - Each one pays gold, XP and an item on the spot (bounties, sieges and treasure always give a rare).
  - They show in the quest log and in the journal under AREA TASKS.

## 2c. The user's picks (2026-10-08): extras per act, built act by act with a review after each

The user took every idea offered and added two of their own (marked *).

| Act | Extras |
|---|---|
| 1 | **Burning Barn**: a burning farm in the Barrow Fields; pull the villagers out before the roof falls. **Goblin Market**: a neutral goblin camp in Skrat's Gulch that trades stolen goods, until you attack. **Ash Storm**: random weather; you can barely see, and ash elementals roam until it passes. |
| 2 | **Thin Ice**: a frozen lake that cracks under fighting and heavy feet; fall in and you're hurt and chilled, monsters drown. **Raider Longhall**: the mead-chief's hall in the Raiders' Fjord; steal his war horn, or challenge him to a duel. **Frozen Merchant**: thaw him (fire is best) and he sells rare goods once. **Yeti Cubs**: lead a lost cub home and that area's yetis stop hunting you. |
| 3 | **Witch of the Bog**: deals with an upside and a downside. **Wolf Moon**: werewolf packs, double pelt bounty. **Haunted Manor**: three keepsakes lay a ghost to rest. **Vardak's Brides**: three brides, a unique ring if all fall before the Count. *A ghost that haunts you: chase it down and lay it to rest in a random tomb. |
| 4 | **Broken Automaton**: five gears for a clockwork companion. **Unlawful Court**: fight, bribe or argue. **Scrapyard Lottery**: scrap piles, and a junk golem. **Timeless Vault**: opens only while time is stopped. |
| 5 | **Low Tide Treasure**: wrecks and chests at low tide. **Siren's Song**: a choir in the Sunken Galleon. **Kraken's Arm**: a tentacle in the Bone Reef. **Message in a Bottle**: a chain of bottles to a pirate's cache. |
| 6 | **Island Hopping**: islands that crumble behind you. **Storm Chase**: charge a lightning relic. **Fallen Star**: star-spawn and star-metal. **The Last Choir**: three singers open a hidden sanctum. |
| All | **Bestiary**: kills per monster, small bonuses per tier. *Rival adventurer: an opt-in quest; beat them and they leave, and a new rival can come later. **Merchant caravans** between towns. **Arena challenges** with a best-time board. |

**Order:**
1. Act 1's extras and all of Act 2: the extras above, plus the planned optional dungeon, side quests, super uniques and pages. *(done 2026-10-08, `src/features.rs`)*
2. Then Acts 3, 4, 5 and 6, in turn.
3. Then the all-act systems.

## 3. Phases (stop for a review after each)

1. **Systems + Act 1 as the proof.** *(done 2026-10-08)*
   - Shrines (art for Act 1's shrine), super uniques, the side-quest engine, events (ambush, hoarder, fallen
     adventurer), lore pages and the journal tab.
   - Act 1's content: the Charnel Well, both side quests, three super uniques, five pages.
   - Saves, quest log, level-editor catalog, tests, bot coverage.
2. **Acts 2 and 3:**
   - optional dungeons, side quests, super uniques and pages;
   - the cursed chest and the wandering peddler.
3. **Acts 4, 5 and 6:** the same; the tide-gated grotto and the airship-only observatory.
4. **Polish:**
   - art passes (shrines, optional-dungeon entrances and bosses);
   - balance survey with side content on;
   - Nightmare/Hell scaling;
   - the README;
   - rebuilt packages.

## 4. Art (PixelLab budget)

**New art:**

| Art | Count |
|---|---|
| Shrine props (one per act style) | 6 |
| Optional-dungeon entrance props | 6 |
| Optional-dungeon bosses: Well-Witch, Gravedigger, Junk Golem, Astronomer | 4 new characters |
| Item icons: lore page, black pearl | 2 |

- Hrolf and Old Barnacle reuse the raider and shellguard sprites, enlarged and tinted.
- Each hoarder can reuse an existing small creature with a tint where it fits.

**Reused, no new art:**
- **Super uniques** are tinted, enlarged base monsters.
- **Optional-dungeon tiles** are tinted variants of each act's dungeon set.
- **NPCs** are the existing town people.

## 5. Not in this round
- Act 7.
- Runewords and the crafting cube.
- Uber bosses.

These are separate plans if wanted later.

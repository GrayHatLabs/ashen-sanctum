# ASHEN SANCTUM

An isometric, Diablo 2-style action RPG. You are a fire sorceress descending into a
procedurally generated sanctum full of the dead. Fling fireballs, collect gold and
potions, cleanse each level and descend deeper.

Written in Rust with SDL2. Renders a 640x360 software framebuffer (640x480 on the
ANBERNIC RG35XX H), with pixel art generated with PixelLab and embedded in the binary.
Sound effects and music are synthesised in code: a plucked-guitar theme in town, wind and
drones in the wilds, bells and a heartbeat in the dungeons, drums for boss fights.

## Title screen and heroes

The game opens on the **title screen**: Play, Options (music, controls), Quit.
- **Play** opens the **character select**: up to 8 saved heroes, each with their portrait, level, class, act and difficulty.
  - Pick one to continue.
  - X / Space deletes one (it asks first).
  - **New Hero** picks a class (the Inventor is coming soon) and a name. Type one, or take a random gothic name.
- **In game, Esc** saves and returns to the character select.
- Each hero has their own save file in `~/.local/share/ashensanctum/heroes/`. An old single save is copied in as the first hero.

## Seven heroes

A new character starts on the **class select** screen:

- **The Sorceress** burns: Fireball, Inferno, Fire Wall, Meteor, Hydra, Ash Phoenix.
- **The Vampire** drains. Her blood magic steals life with almost every hit, but the bloodless
  (skeletons, wraiths, the Wardens) can't be drained and resist it a little.
  - **Blood Lance:** a crimson bolt that heals her.
  - **Rake:** a clawed slash that steals three times the life.
  - **Thirst** (passive): life steal.
  - **Bat Swarm:** homing bats.
  - **Mesmerize:** a foe fights for her.
  - **Mist Step:** slip through enemies, untouchable for a moment.
  - **Crimson Nova.**
  - **Thrall:** raise a corpse to fight for her.
  - **Night Mastery** (passive).
  - **Blood Moon:** a field that bleeds foes and feeds her.
  - **Embrace:** bat form. Faster, screeching, and skills cost no mana.

  - **Blood hunger:** instead of food she has a red **BLOOD** bar that slowly drains. She can't eat;
    she feeds only by drawing blood. Her blood magic on living foes refills it, and out of mana she
    **bites** (a free attack that drinks deeply). Skeletons and the undead have no blood to give.
    Run it dry and she is **BLOODTHIRSTY**: she loses life and doesn't regenerate.
`--vampire` or `--sorceress` skips the class screen for a new character.

- **The Inventor** builds: a steampunk gunslinger who runs on **heat** instead of mana.
  - **Heat:** firing heats her up (the orange gauge) and it cools on its own. Fill it and she
    **overheats**: her weapons lock for 2 s and only a weak shot works.
  - **Venting:** E / Y vents steam, dumping all heat and blasting foes back. Mana potions are coolant.
  - **Running hot:** above 70% heat she hits harder.
  - **Ray Pistol:** fast aether bolts. **Clockwork Bomb:** thrown, bursts after a fuse.
    **Tinkerer** (passive).
  - **Arc Coil:** chain lightning. **Sentry Turret.** **Grapple Hook:** zip to a spot, or yank a foe to you.
  - **Tesla Field:** a shocking aura. **Clockwork Spider:** a companion. **Overclock** (passive).
  - **Airship Strike:** carpet bombing. **Steam Suit:** half damage taken, faster, skills run cold.
  - `--inventor` skips the class screen.

- **The Valkyrie** fights up close: a frost spear-warrior and the first melee hero.
  - **Valor** instead of mana: it starts empty, fills as her spear lands and as she takes blows,
    and drains away out of combat. Her skills spend it. At **full Valor** her runes blaze (+15% damage).
    Mana potions are **mead**.
  - **Melee:** click a foe (or press the pad's attack) and she walks into reach, then strikes.
  - **Frost:** her hits pile frost on foes (they slow down). With **Frost Brand**, a full load
    **freezes** a foe solid, and frozen foes she kills **shatter** into shards that hit their friends.
  - **Rune Spear:** a reaching thrust, free, builds Valor. **Rime Sweep:** a frost arc. **Northborn** (passive): toughness.
  - **Raven Strike:** her raven marks a foe to take more damage. **Glacier Leap:** her ice wings spread for a
    leap and a freezing landing. **Frost Brand** (passive): freeze and shatter.
  - **Rune Javelin:** pierces a line and flies back. **Winter's Wrath:** a full spinning sweep.
    **Einherjar:** spectral warriors fight beside her.
  - **Ride of the Valkyrie:** her warhorse charges through a line of foes. **Fimbulwinter:** a killing blizzard.
  - `--valkyrie` skips the class screen. Her own art isn't made yet (a steel-blue stand-in for now).

- **The Berserker** is a barbarian warlord with a giant two-handed axe and a **dire wolf** that never
  leaves her side. No magic.
  - **Rage** instead of mana: it starts empty and fills from **pain** (blows she takes) and **kills**, not
    from her own hits. Skills spend it; **Cleave** is free. It fades out of combat.
  - **The lower her life, the harder she hits** (up to +40%).
  - **Gore:** her axe makes foes bleed, and the **Executioner** can take a wounded foe's head, scaring its friends.
  - **Cleave:** a wide arc. **Rend:** bleeding and sundered armor. **Iron Hide** (passive): tougher below half life.
  - **Leap Slam:** a leap that knocks foes down. **Dire Wolf:** a stronger wolf, which howls foes away.
    **Bloodlust** (passive): kills heal her and quicken her axe.
  - **War Cry:** foes flee and she hits harder. **Whirlwind:** spin through them. **Executioner** (passive).
  - **Hurl Axe:** it spins through a line and comes back. **Berserk:** for a while she can't die, swings
    faster and heals with every hit, then she's spent.
  - `--berserker` skips the class screen. Her own art isn't made yet (a raider stand-in for now).

- **The Reaper** is an immortal guardian of a forbidden library, with a rune scythe, a spirit lantern
  and the Ledger of the Forgotten. Half melee, half spirit magic.
  - **Souls** instead of mana: every foe that dies near her releases a soul that drifts into her
    lantern (3 for bosses and for foes written in her Ledger). Skills spend souls; the scythe is free.
  - **The rune blade:** each hit on the same prey lights a rune; at seven the blade blazes and her
    next sweep cleaves for triple damage.
  - **Reaping Scythe**, **Spirit Lantern** (a homing spirit flame), **Patient Archivist** (passive).
  - **Ledger Mark** (a marked foe takes more and gives 3 souls), **Scholar Spirits** (ghost allies),
    **Shadow Step**.
  - **Archive Chains** (bind an area), **Hourglass** (foes crawl), **Rune Blade** (passive).
  - **Soul Harvest:** a great circle that reaps the badly wounded outright. **Open the Ledger:**
    double souls, and the dead rise as scholar spirits.
  - `--reaper` skips the class screen.

- **The Druid** is a plague summoner who keeps nature's balance. Her creatures fight while she spreads
  poison and mends her own.
  - **Decay and Bloom:** besides mana she has a balance bar. Plague skills push it toward Decay, growth skills
    (summons, healing) toward Bloom, and each side empowers the other's skills by up to +30%, so she
    plays best alternating. It drifts back to the middle.
  - **Spore Cloud**, **Rat Swarm**, **Green Doctor** (passive: stronger poison, tougher creatures).
  - **Thorn Lash** (vines bind a line of foes), **Moss Wolf** (stays until it falls, follows her between
    levels), **Rejuvenate** (heals her and her creatures).
  - **Fungal Bloom** (mushrooms burst into poison), **Corpse Bloom** (a corpse bursts and rats crawl
    out), **Cycle of Rot** (passive).
  - **Pestilence** (poisons everything around; the plague spreads when they die) and the **Thorn Warden**,
    a guardian of dead trees, roots and bone.
  - `--druid` skips the class screen.

## The story

Ash falls on the village of Hollowmere. Beneath the Ashen Sanctum sleeps the Ash King, a
fallen archmage who tried to burn away death itself. Three Wardens gave their lives to seal him,
but their tombs have been defiled and the Wardens walk again. Elder Maren asks you to slay them,
take back their Seals, unseal the Sanctum and end the Ash King.

## The world

- **Hollowmere** (the starting town) is safe: no monsters, no hunger. Talk to everyone:
  - **Elder Maren** (by the campfire): the story and your quests. A **!** means she has news.
  - **Gerta** (market stall): healing and mana potions, bread and roasts for gold, and gear
    ("SHOW ME YOUR GEAR"): her shelf restocks each time you come back from a dungeon.
  - **Brother Aldric**: heals you fully whenever you talk to him.
  - **Captain Rolf** and the villagers: hints.
- **The Ashlands** (overworld): forests, roads, roaming wolves, goblins and undead, wild food.
- **Four dungeons**, each with its own look, floors joined by stairs, and a boss at the bottom:
  1. **The Bone Crypt** (southwest, 2 floors): the **Bone Warden** raises skeletons.
  2. **The Rotting Warrens** (southeast, 2 floors): the **Plague Warden** spits poison pools.
  3. **The Hexed Catacombs** (northeast, 3 floors): the **Hex Warden** fires bolt volleys and blinks away.
  4. **The Ashen Sanctum** (northwest, 3 floors, sealed until you have all three Seals): the **Ash King**.
- **Waypoints** (D2 style): a rune circle in Hollowmere and near the start of every dungeon floor.
  Step on one to activate it; stepping onto any waypoint lets you travel to every one you've activated.
- Each Warden drops a **Seal** (more life, mana and fireball power) and opens a portal home.
  Levels remember what you killed. If you die, you wake in Hollowmere and lose 10% of your gold.

## How to play

- **Fireball** costs 5 mana. It explodes on impact, splashes nearby foes and sets them burning.
- **Ember Bolt**: when you're out of mana the same button fires a free, weaker bolt (single target, no burn).
- **Run / walk** (R or pad B), like D2: running is fast but drains the yellow **stamina** bar, which refills while you walk or stand. Run it dry and you're tired (walk only) until it recovers.
- **Food**: the food bar slowly empties, faster while running. When it's empty you're **starving**: you lose life, don't regenerate it, and stamina refills slowly. Apples, bread and roasts lie around and drop from monsters (walk over them to eat), and Gerta sells them.
- **Experience**: kills level you up (more life, mana and damage) and give a **skill point**; each
  Warden's Seal gives another. Spend them in the **skill tree** (K / hold SELECT):
  - **Fireball**: exploding bolt that sets foes burning.
  - **Inferno**: hold to breathe a cone of flame (drains mana while held).
  - **Warmth** (passive): faster mana regeneration.
  - **Fire Nova** (char level 6, needs Inferno): a ring of fire that burns and hurls back everything nearby.
  - **Fire Wall** (char level 6, needs Fireball): a line of flames across the target spot that burns anything in it.
  - **Blaze** (char level 6, needs Warmth): for a while you leave burning ground behind you as you move.
  - **Combust** (char level 12, needs Fire Wall): every burning foe in sight explodes; the longer it has
    burned, the bigger the blast (Fire Wall ranks make it stronger).
  - **Meteor** (char level 12, needs Fire Nova): a shadow marks the spot, then a meteor crashes down and
    leaves the ground burning (Meteor ranks also boost Fireball).
  - **Fire Mastery** (char level 12, passive, needs Blaze): all fire hits harder and burns longer; lengthens Blaze.
  - **Hydra** (char level 18, needs Combust, 12 s cooldown): a fire hydra spits fireballs at nearby foes for 10 s.
  - **Ash Phoenix** (char level 18, needs Meteor, 30 s cooldown): fiery wings; you move faster and skills
    cost no mana, then you explode in flame.
  Out of mana, any skill fires the free Ember Bolt. Brother Aldric resets your skills for gold.
- **Equipment** (D2 style): about one monster in ten drops gear. Walk over it to pick it up.
  - Nine slots: staff, helm, armor, gloves, boots, belt, two rings and an amulet. You start with a gnarled staff.
  - **White** items are plain, **blue** (magic) have 1-2 random stats, **yellow** (rare) 3-5, and
    **gold** (unique) are fixed, named items. Each Warden and the Ash King drops its own unique.
  - Stats: life, mana, fire damage, armor (less damage taken), life / mana regeneration, faster cast
    rate, faster run/walk, slower stamina drain, slower hunger, + to fire skills, extra gold, better
    chance of magic items, life / mana after each kill. Deeper areas drop better gear; some items
    need a character level.
  - Open the **inventory** with I, START or the BAG button. Wear / take off with Enter / A (or click
    the item twice). X or right click drops an item, or sells it in Hollowmere.
  - In Hollowmere the inventory also shows your **stash** (30 slots, saved): Y / E moves an item
    between your bag and the stash.
- **Monsters**: zombies are slow and hit hard, skeletons are fast and fragile, wolves hunt in fast
  packs, goblins panic and flee when one of their own dies, and skeleton archers keep their distance.
  **Champion packs** (blue names, tinted blue) are tougher, with one modifier; **elites** (gold
  names) lead minions and have two: fast, strong, stone skin, vampiric (heals by hitting you), mana
  burn, or fire enchanted (bursts into flame when it dies: step away). They drop more and better gear.
  Bosses telegraph their big attacks with markers on the floor: step out of them.

## Act 2: the Frostmarch

Killing the Ash King opens the **mountain pass** north of Hollowmere. Beyond it lies the
**Frostmarch**: snowfields, snowy pine forests, a frozen lake and the frontier town of **Kaldholm**.

- **Captain Brenna** (story), **Old Sigurd** (fur trader: potions, food and gear) and **Mother Ylva**
  (seer: heals and resets skills).
- **Vorthrax the Rime Wyrm**, an ancient white dragon, has woken under the glacier. Her three Frost
  Heralds hold the **Frost Runes** that unseal her lair:
  - **The Frozen Mines** (west): the **Frost Giant Overseer** slams the ground and hurls ice boulders.
  - **The Howling Caves** (northeast): the **Yeti Matriarch** shakes icicles from the roof and calls her brood.
  - **The Rime Temple** (northwest): the **Rime Witch** fires ice-bolt volleys, freezes the floor and blinks away.
  - **The Glacier's Heart** (far north, needs all three runes): the dragon herself. Frost breath, icicles,
    and she enrages at half life.
- **Monsters**: winter wolves, northern raiders, yetis, ice trolls (they regenerate unless burning) and
  ice wraiths (ice bolts).
- **The cold**: frost attacks **chill** you (slower moving and casting). Creatures of the cold take 25%
  more fire damage.

## Act 3: the Mistwood

Slaying the Rime Wyrm lifts the mist on the **road east** out of Kaldholm. Beyond it lies the
**Mistwood**, a Ravenloft-style haunted forest of blue-grey earth, glowing green moss, twisted trees
and graveyards, with the walled village of **Mournhold** and the dark spires of Castle Vardak above it.

- **Abelard the Hunter** (story), **Widow Kasia** (shop) and **Father Lucian** (heals and resets skills).
- **Count Vardak** rules from his castle. His three heralds, the skeleton lords, hold the **Blood Sigils**
  that open the castle gate:
  - **The Sunken Chapel**: **Lord Ossric, the Bone Baron**.
  - **The Gallows Catacombs**: **Duchess Grimhilde**, a necromancer who hurls green soulfire.
  - **The Barrow of Knights**: **Sir Malgrave, the Death Knight**.
  - **Castle Vardak** (needs all three sigils): the Count himself. He duels with blood-bolt fans and
    bats, turns to **mist** (untouchable, reappearing beside you while wolves answer his call), and at
    the end becomes a **giant bat**.
- **Monsters**: ghouls, werewolves (they regenerate), banshees, will-o'-wisps, cultists and vampire bats.
- Fog banks and drifting wisp-lights over the forest; a haunted waltz in the woods, an organ in the crypts.

## Act 4: Mechanus

When Count Vardak falls, a ring of brass **gears** behind his castle starts to turn. Through it lies
**Mechanus, the Clockwork Dominion**: the Grinding Fields, an island of sooty brass plates over the void,
with gear towers, boilers and steam vents, and the refuge town of **the Last Escapement**, where clockwork
servants who grew souls hide from their maker.

- **Tally** (story), **Madame Vesper** (tinkerer shop) and **Brother Piston** (heals and resets skills).
- **The Clockmaker** wound the Ash King, the Wyrm and the Count. His great clock is locked with three
  **Winding Keys**, held by his heralds:
  - **The Foundry of Souls**: **the Forgemother** pours slag pools and rebuilds her fallen automatons.
  - **The Choir Engine**: **the Cantor**, an organ on spider legs, fires rings of sound with a turning gap to dodge through.
  - **The Archive of Gears**: **the Archivist** "files" you elsewhere and summons records of old bosses.
  - **The Heart of the Clock** (needs all three keys): **the Clockmaker**. He throws spirals of clock-hand
    blades and **rewinds** you to where you stood three seconds ago. Below half life he climbs into his great
    engine and swings pendulum slams across the arena.
- **Monsters**: brass scarabs (clockwork beetles that swarm), inquisitor automatons (steam censers), gearwraiths (they phase out), spring-heeled
  jacks (leap in), boiler brutes (they explode when they fall) and the **Ordinals**: squads of floating clockwork
  shapes (cubits, prisms and a marshal) that march in step. Kill the marshal and the squad falls into disorder.
- Drifting steam and brass sparks; a ticking harpsichord outside, an engine's clangour in the works.

## After the Clockmaker

Like Diablo 2, beating the game opens **Nightmare**, then **Hell**: talk to Tally. The
world is rebuilt with much tougher monsters and richer drops, and the quests start over, while
your character, skills and gear carry on.

## Controls

| Action | Mouse + keyboard | Gamepad / handheld |
|---|---|---|
| Move | Hold left click on the floor, or WASD / arrows | Left stick / D-pad |
| Primary skill (L slot) | Left click a monster, Shift + left click, F | A, RT or the right stick (auto-aims at the nearest foe) |
| Secondary skill (R slot) | Right click, Space | X |
| Pick the secondary skill | 1-4 | R1 (cycles) |
| Skill tree | K, or click the skill slots / the + button | Hold SELECT |
| Inventory | I, or click BAG | START |
| Run / walk toggle | R | B |
| Health potion | Q or 1 | L1 / L2 |
| Talk to someone | Left click them, or F / Space next to them | A next to them |
| Choose in a conversation | Up / Down + Enter, or click | D-pad + A |
| Map | Tab or M | SELECT |
| Mana potion | E or 2 | Y |
| Music on / off | N | |
| Continue after death / the ending | Enter | START |
| Close menu / quit | Esc (closes a conversation or the map first) | SELECT + START |

## Level editor

Every map (the overworld with Hollowmere, and each dungeon floor) can be hand-made in the
browser level editor:

```powershell
D:\projects\AshenSanctum\scripts\level-editor.ps1
```

Click **Open levels folder**, pick `D:\projects\AshenSanctum\levels`, choose a map (exported
starting points are in `levels/generated/`), edit, **Save**. The game uses `levels/<name>.json`
instead of the generated map. Test with `--level <name>`. Details: `docs/LEVEL_FORMAT.md`.

## Saving

Your character saves automatically (whenever you reach the overworld, after boss kills and
when you quit) to `~/.local/share/ashensanctum/save.txt`. Like Diablo 2, your hero, gold,
potions and quest progress carry over, while the monsters and loot are fresh each time you
start the game. Start a brand-new character with `--new`. Your gear and bag are saved too.

To listen to the music outside the game: `--export-music music` writes the loops as WAV files.

To try a later act straight away, `--act3` (level 26 in Mournhold) or `--act4` (level 34 in the Last
Escapement) work like `--act2`, each with its own save file. `--act2` starts a ready-made level 18 sorceress in Kaldholm (Act 1
done, skill points to spend, a full set of gear). It has its own save file (`save_act2.txt`), so your
real character is untouched; `--act2 --new` starts it over.

For testing, `--cheats` turns on F9 (gain a character level) and F10 (drop loot at your feet).

## Building

Builds run in WSL Ubuntu:

```bash
scripts/dev-build.sh run          # desktop build + play (WSLg window)
scripts/dev-build.sh run --tall   # the handheld's 640x480 view on desktop
scripts/test.sh                   # unit tests, 6-minute bot self-test, snapshot frames
scripts/build-handheld.sh         # dist/AshenSanctum-aarch64.zip for PortMaster
```

From Windows PowerShell: `wsl -d Ubuntu -e bash scripts/dev-build.sh run`.

## Art

Generated art lives in `D:\projects\AshenSanctum-art`. After generating, run
`python tools/pack.py` there, then `python scripts/import_art.py` here and rebuild.

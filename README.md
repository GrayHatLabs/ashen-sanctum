# ASHEN SANCTUM

An isometric, Diablo 2-style action RPG. You are a fire sorceress descending into a
procedurally generated sanctum full of the dead. Fling fireballs, collect gold and
potions, cleanse each level and descend deeper.

Written in Rust with SDL2. Renders a 640x360 software framebuffer (640x480 on the
ANBERNIC RG35XX H), with pixel art generated with PixelLab and embedded in the binary.
Sound effects and music are synthesised in code: a plucked-guitar theme in town, wind and
drones in the wilds, bells and a heartbeat in the dungeons, drums for boss fights.

## The story

Ash falls on the village of Hollowmere. Beneath the Ashen Sanctum sleeps the Ash King, a
fallen archmage who tried to burn away death itself. Three Wardens gave their lives to seal him,
but their tombs have been defiled and the Wardens walk again. Elder Maren asks you to slay them,
take back their Seals, unseal the Sanctum and end the Ash King.

## The world

- **Hollowmere** (the starting town) is safe: no monsters, no hunger. Talk to everyone:
  - **Elder Maren** (by the campfire): the story and your quests. A **!** means she has news.
  - **Gerta** (market stall): healing and mana potions, bread and roasts for gold.
  - **Brother Aldric**: heals you fully whenever you talk to him.
  - **Captain Rolf** and the villagers: hints.
- **The Ashlands** (overworld): forests, roads, roaming wolves, goblins and undead, wild food.
- **Four dungeons**, each with its own look, floors joined by stairs, and a boss at the bottom:
  1. **The Bone Crypt** (southwest, 2 floors): the **Bone Warden** raises skeletons.
  2. **The Rotting Warrens** (southeast, 2 floors): the **Plague Warden** spits poison pools.
  3. **The Hexed Catacombs** (northeast, 3 floors): the **Hex Warden** fires bolt volleys and blinks away.
  4. **The Ashen Sanctum** (northwest, 3 floors, sealed until you have all three Seals): the **Ash King**.
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
- **Monsters**: zombies are slow and hit hard, skeletons are fast and fragile, wolves hunt in fast
  packs, goblins panic and flee when one of their own dies, and skeleton archers keep their distance.
  **Champion packs** (blue names, tinted blue) are tougher, with one modifier; **elites** (gold
  names) lead minions and have two: fast, strong, stone skin, vampiric (heals by hitting you), mana
  burn, or fire enchanted (bursts into flame when it dies: step away). They drop more and better gear.
  Bosses telegraph their big attacks with markers on the floor: step out of them.

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

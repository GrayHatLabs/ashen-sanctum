# ASHEN SANCTUM

An isometric, Diablo 2-style action RPG. You are a fire sorceress descending into a
procedurally generated sanctum full of the dead. Fling fireballs, collect gold and
potions, cleanse each level and descend deeper.

Written in Rust with SDL2. Renders a 640x360 software framebuffer (640x480 on the
ANBERNIC RG35XX H), with pixel art generated with PixelLab and embedded in the binary.
Sound effects are synthesised in code.

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
- **Experience**: kills level you up (more life, mana and fireball damage).
- **Monsters**: zombies are slow and hit hard, skeletons are fast and fragile, wolves hunt in fast
  packs, goblins panic and flee when one of their own dies, and skeleton archers keep their distance.
  Bosses telegraph their big attacks with markers on the floor: step out of them.

## Controls

| Action | Mouse + keyboard | Gamepad / handheld |
|---|---|---|
| Move | Hold left click on the floor, or WASD / arrows | Left stick / D-pad |
| Fireball / Ember Bolt | Right click (at the cursor), left click a monster, Shift + left click, F or Space | Right stick (aim + cast), or A / X / R1 / RT (auto-aims at the nearest foe) |
| Run / walk toggle | R | B |
| Health potion | Q or 1 | L1 / L2 |
| Talk to someone | Left click them, or F / Space next to them | A next to them |
| Choose in a conversation | Up / Down + Enter, or click | D-pad + A |
| Map | Tab or M | SELECT |
| Mana potion | E or 2 | Y |
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
start the game. Start a brand-new character with `--new`.

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

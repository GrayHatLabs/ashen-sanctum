# ASHEN SANCTUM

An isometric, Diablo 2-style action RPG. You are a fire sorceress descending into a
procedurally generated sanctum full of the dead. Fling fireballs, collect gold and
potions, cleanse each level and descend deeper.

Written in Rust with SDL2. Renders a 640x360 software framebuffer (640x480 on the
ANBERNIC RG35XX H), with pixel art generated with PixelLab and embedded in the binary.
Sound effects are synthesised in code.

## How to play

- **Fireball** costs 5 mana. It explodes on impact, splashes nearby foes and sets them burning.
- **Ember Bolt**: when you're out of mana the same button fires a free, weaker bolt (single target, no burn).
- **Dash** a few tiles in the direction you're moving (or toward the cursor). You can't be hit mid-dash; 1.2 s cooldown.
- **Zombies** are slow and hit hard. **Skeletons** are fast and fragile. Monsters in a pack alert each other.
- Getting hit by a fireball interrupts a monster's swing.
- Life and mana regenerate slowly. Drink potions when you need them (monsters drop more).
- Kill every monster on a level to cleanse it, then press Enter / START to descend.
  Each level is bigger and meaner.

## Controls

| Action | Mouse + keyboard | Gamepad / handheld |
|---|---|---|
| Move | Hold left click on the floor, or WASD / arrows | Left stick / D-pad |
| Fireball / Ember Bolt | Right click (at the cursor), left click a monster, Shift + left click, F | Right stick (aim + cast), or A / X / R1 / RT (auto-aims at the nearest foe) |
| Dash | Space | B |
| Health potion | Q or 1 | L1 / L2 |
| Map | Tab or M | SELECT |
| Mana potion | E or 2 | Y |
| Descend / restart | Enter | START |
| Quit | Esc | SELECT + START |

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

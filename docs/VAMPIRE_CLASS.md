# The Vampire: second playable class (design draft)

The user asked for this on 2026-10-05.
- **Look:** a glamorous 1980s goth vampire woman.
  - Huge teased black hair with a purple streak, pale skin, glowing red eyes.
  - A plum velvet gown, studded belt, choker with a red gem.
  - A black cape with a tall blood-red collar.
  - Hand-on-hip, sly smirk.
- **Concept art (art repo, `tools/vampire_art.py`):**
  - A class-select portrait (140x200, with a moonlit castle). PixelLab only makes pixel art, so this is the cartoon look in pixel form.
  - The 8-direction in-game sprite (48 px, small head like the sorceress).

## Identity

The sorceress burns; the vampire **drains**. Blood magic hurts her a little to cast, but almost everything heals her back. She's at her best in the thick of a pack, stealing life faster than it is taken. Her damage is **blood** (not fire): undead (skeletons, zombies, the Wardens, skeleton lords in Act 3) have no blood and resist the drain.

## Draft skill tree (4 tiers, like the sorceress: char levels 1 / 6 / 12 / 18)

| Tier | Skill | What it does |
|---|---|---|
| 1 | **Blood Lance** | Crimson bolt; heals her for part of the damage. Her Fireball. |
| 1 | **Rake** | Clawed melee strike, cheap, big life steal. |
| 1 | **Thirst** (passive) | Life steal on all her damage; regenerates faster at low life. |
| 2 | **Bat Swarm** | Bats fly out and chase nearby foes, nibbling them. |
| 2 | **Mesmerize** | A foe fights for her for a few seconds (D2 Confuse). |
| 2 | **Mist Step** | Turns to mist and slips a few tiles through enemies (short cooldown). *Optional: the user disliked the sorceress's dash.* |
| 3 | **Crimson Nova** | Burst of blood that drains everything around her. |
| 3 | **Thrall** | Raises a slain monster as a minion for a while. |
| 3 | **Night Mastery** (passive) | More blood damage and life steal. |
| 4 | **Blood Moon** | A field where foes bleed and she heals. |
| 4 | **Countess's Embrace** | Becomes a giant bat for a while: fast, screeching AoE, flies over hazards. |

## What it needs

- **Class choice:** a class select screen on a new character, using the portrait. Saves remember the class.
- **Skill trees and HUD:**
  - A second skill tree, plus HUD icons and descriptions.
  - Per-class casting code (`skills.rs` grows a vampire half).
- **Mechanics:**
  - Blood damage type.
  - Undead resist the drain.
  - Life steal.
  - Minion AI (thralls, mesmerized foes fighting for her).
- **Art:**
  - Walk, cast and claw-attack animations (about 15 generations).
  - A tinted plum gown (the sprite's gown came out black).
  - 11 skill icons.
- **Gear:** class-specific items, such as "+ to blood skills" and claws or daggers as her weapon type (vs staffs).

# The Inventor: third playable class (design, 2026-10-05)

**Decided with the user:**
- **Name:** Inventor.
- **Playstyle:** gunslinger + gadgets.
- **Resource:** Heat instead of mana.
- **Movement:** Grapple Hook is in.

- **Look (the user's description):** a glamorous steampunk goth woman inventor.
  - Long dark auburn curls, pale skin, amber eyes.
  - A small tilted black top hat with brass goggles (teal lenses), a lace choker with a brass gear.
  - A brown leather corset over a black ruffled blouse, a black and oxblood bustle skirt.
  - Lace-up boots and fingerless gloves.
  - **One clockwork brass arm** holding an ornate brass **ray pistol**, and a pocket watch chain.
  - A foggy Victorian city of airships and steam behind her; brass and teal palette.
- **Concept art (art repo, `tools/inventor_art.py`):** portrait and 8-direction sprite.
- **Home turf:** Act 4, the Clockwork Dominion of Mechanus (docs/ACT4_IDEAS.md). She's the mortal genius who can out-invent the Clockmaker.

## Identity

The sorceress burns and the vampire drains; the inventor **builds**. She fights with **aether-lightning**:
- A fast ray pistol for steady damage.
- Gadgets she sets down and lets work (bombs, turrets, a clockwork spider).
- Big inventions for the late game.

Her damage is **lightning (aether)**:
- Wet and armored foes conduct it.
- Act 4's automatons are grounded and resist it a little.

## Draft skill tree (char levels 1 / 6 / 12 / 18)

| Tier | Skill | What it does |
|---|---|---|
| 1 | **Ray Pistol** | Rapid aether bolts. Her Fireball: cheap, fast, accurate. |
| 1 | **Clockwork Bomb** | Throws a ticking brass bomb; it bursts after a short fuse (knockback). |
| 1 | **Tinkerer** (passive) | Faster firing, and her gadgets last longer. |
| 2 | **Arc Coil** | A bolt that chains from foe to foe. |
| 2 | **Sentry Turret** | Sets down a brass turret that shoots nearby foes. |
| 2 | **Grapple Hook** | Fires a hook: on a wall or the floor she zips there; on a foe she yanks it to her. |
| 3 | **Tesla Field** | Crackling field around her: shocks and slows everything nearby. |
| 3 | **Clockwork Spider** | A mechanical spider companion that follows her and bites. |
| 3 | **Overclock** (passive) | More aether damage; ray bolts can pierce. |
| 4 | **Airship Strike** | A brass airship passes over and carpet-bombs a line. |
| 4 | **Steam Suit** | Climbs into a walking steam armour for a while: tough, fast, with a steam cannon. |

## Signature mechanic: Heat (decided)

- **The bar:** an orange **heat** bar, 0-100, replaces her blue mana globe. Every skill adds heat instead of costing mana.
- **Cooling:** heat drains on its own, faster when she isn't firing.
- **Overheating:** at 100 her weapons lock for ~2 s while steam pours off her. The Ray Pistol still fires, but weakly.
- **Venting:** the potion key Y / E vents instead: a steam blast that dumps all heat and pushes nearby foes back (short cooldown).
- **The risk:** running hot (above 70) adds damage. You play close to the edge.
- Mana potions become **coolant** for her.
- Gear "+mana" counts as **max heat**, and "mana regeneration" counts as **cooling rate**.

## What it needs

Built on the class system the Vampire added:
- A third tree, `inventor.rs`.
- A portrait on the class select screen (three side by side).
- Art: walk, shoot and throw animations. Turret, spider, airship and steam-suit sprites (PixelLab props / characters). Skill icons.

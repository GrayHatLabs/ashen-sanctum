# The Berserker: fifth playable class, the second melee hero (design, 2026-10-05)

**Built 2026-10-05** (code complete with a raider stand-in sprite; her PixelLab art waits on generations).

**Decided with the user:**
- **Name:** Berserker.
- **Resource:** Rage (builds from pain and kills; lower life = more damage).
- **Wolf:** always with her.
- **Gore:** D2-style (bleeding, blood bursts, beheading kills).

## Look (the user's description, condensed)

A **Gothic Berserker Queen**: a barbarian warlord who fights at the front of every battle.

- **Build and face:**
  - tall, athletic and visibly muscular, battle-scarred, with a dangerous half-smile;
  - very long, wild ash-brown hair in thick Nordic braids with bronze rings, black feathers, teeth and iron charms;
  - pale but weathered skin, scars, amber-gold eyes, smoky black makeup, black war paint down one eye, brown-black lips;
  - black Nordic tattoos (knotwork, wolves, ravens).
- **Crown:** brutal hammered iron made of broken spearheads.
- **Armor:**
  - a scratched, dented black-leather battle corset reinforced with blackened iron plates and big buckles;
  - a massive wolf-fur mantle with an iron clasp;
  - bare muscular arms with leather bracers and scavenged iron;
  - a skirt of black leather strips, charcoal cloth, hides and fur;
  - belts of chains, throwing knives, pouches and trophies, and a **skull** on the hip;
  - huge knee-high boots of brown leather, iron plates and wolf fur.
- **Weapons:**
  - a **gigantic two-handed executioner's battle axe**: chipped dark iron, rough runes, fur-wrapped haft;
  - a broad Nordic knife across the back of her belt.
- **Companion:** a huge scarred black **dire wolf** with an iron-ringed collar.
- **No magic:** "her power comes from physical strength, reputation and fearlessness."
- **Palette:** blackened iron, dark leather, charcoal, wolf-fur grey, earthy brown, muted bronze, **subtle dried-crimson accents**.
- **Background (portrait):** a muddy ruined battlefield with burning wooden fortifications, smoke, storm clouds, distant snowy mountains, and warriors raising axes under black banners.

**At sprite size:** the axe, fur mantle and hair read well. The wolf is a real companion that runs beside her, so it gets its own sprite.

## How she differs from the Valkyrie

| | Valkyrie | Berserker |
|---|---|---|
| Weapon | Spear: reach, thrusts, lines | Two-handed axe: short, huge cleaving arcs |
| Damage | Physical + frost (chill, freeze, shatter) | Pure physical: bleeding, armor-breaking, knockdowns |
| Magic | Rune magic, ravens, spectral warriors | None. Shouts, rage and the wolf |
| Defence | Tankiest: life and armor | Lower armor, but she **heals by killing** and refuses to die |
| Feel | Disciplined warrior queen holding the line | Reckless warlord who gets stronger the more hurt she is |

## Identity

She wades in and **cleaves**. Every axe swing hits everything in a wide arc.
- **Bleeding:** her blows make foes bleed (damage over time).
- **Armor breaking:** heavy hits shred armor, so foes take more damage.
- **Knockdowns:** big swings knock foes down.
- **The wolf:** her **dire wolf** fights beside her the whole time, a permanent companion who levels up with her skills.

## Draft skill tree (char levels 1 / 6 / 12 / 18)

| Tier | Skill | What it does |
|---|---|---|
| 1 | **Cleave** | Her basic attack. A heavy axe swing hitting everything in a wide arc in front of her. |
| 1 | **Rend** | A savage chop that makes the target and those beside it **bleed**. |
| 1 | **Iron Hide** (passive) | More life, and she takes less damage while her life is below half. |
| 2 | **Leap Slam** | Jumps to a spot and slams the axe down: damage and a knockdown around the landing. Her movement skill. |
| 2 | **Dire Wolf** | Her wolf grows stronger: more damage and life, and it howls to make foes flee for a moment. (The wolf is with her from level 1.) |
| 2 | **Bloodlust** (passive) | Each kill heals her and makes her attack faster for a few seconds (stacks). |
| 3 | **War Cry** | A battle shout: foes nearby are shaken (deal less damage, some flee), and she and the wolf hit harder. |
| 3 | **Whirlwind** | Spins with the axe extended, moving while she spins, shredding everything she passes. (The D2 classic.) |
| 3 | **Executioner** (passive) | Massive bonus damage against foes below 30% life; finishing blows can **behead** (a gory kill that scares nearby foes). |
| 4 | **Hurl Axe** | Throws the giant axe: it spins through a line of foes and returns to her hand. |
| 4 | **Berserk** | Her ultimate. For several seconds she can't die (life can't drop below 1), attacks much faster, and every hit heals her. Afterward she's briefly exhausted. |

## Signature mechanic: Rage (decided)

She runs on **Rage**, a dark-crimson bar. To differ from the Valkyrie's Valor, it builds on **pain**, not hits:
- **Building:** Rage builds when she **takes damage** and when she **kills**, not from her own hits.
- **Spending:** skills spend Rage, and Cleave is free.
- **Low life:** the lower her life, the more damage she deals, up to **+40%** near death. Playing on the edge is the point.
- **Decay:** Rage drains when she's out of combat.

## Wolf companion

- Always with her: it spawns with her, follows her, attacks what she attacks, and respawns in town or after 20 s if killed.
- It's a permanent ally that reuses the vampire's thrall code (charmed allies hunt hostile monsters), with no timer.
- It scales with the **Dire Wolf** skill rank.

## What it needs (beyond the Valkyrie's melee groundwork)

- **Monster status:** a bleed timer and armor-break (a damage-taken multiplier).
- **Knockdown:** a longer stun with a "falls over" tilt.
- **Behead:** a kill effect with a head-and-blood burst, fear on nearby foes, and a spooked flee using the goblin-panic code.
- **The permanent wolf companion** (above).
- **The Whirlwind state:** move while spinning, damage ticks along the path.
- **The Berserk state:** an unkillable timer, then exhaustion.
- **A fifth class card:** the class select row becomes a carousel or two rows; this is needed for the Valkyrie already.

## Art plan (PixelLab, `tools/berserker_art.py`)

- **Portrait:** from the user's description, composited onto a burning-battlefield background. Muscular but small realistic head, never chibi.
- **Sprite (about 56 px), 8 directions:** the axe over her shoulder and the fur mantle.
- **Animations:**
  - walk;
  - cleave (wide swing);
  - overhead chop;
  - leap slam;
  - whirlwind spin;
  - throw;
  - war-cry shout.
- **Dire wolf:** its own character (the `dog` template, scaled up), with run, bite and howl animations.

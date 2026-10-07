# The Inquisitor's whip: longer, real lashing strikes (plan, 2026-10-06)

**The user:** "give more length to the inquisitor's attack, give it more of long whipping actions, it's so short when she attacks".

## What it does today

- **Censer Strike** (her free basic attack) reaches **3.6 tiles** (Zealotry adds a little more) in a narrow cone.
- The hit lands **instantly**, the moment you click.
- The chain is drawn for 0.34 s, unrolling with one curl and a crack at the end.
- **Chain Lash** reaches 5.5 tiles in a line and stops at walls.

So it reads as a short jab: the damage is done before the chain has even finished unrolling.

## The plan

### 1. Longer reach

| | Now | Planned |
|---|---|---|
| Censer Strike | 3.6 | **5.5** |
| Chain Lash | 5.5 | **7.5** |
| Zealotry bonus | small | the same small bonus, on top |

- Both stop at the first wall, as Chain Lash already does.
- Censer Strike becomes a **long narrow line** (like Lash, but thinner) instead of a cone, so it reads as a whip rather than a swing.

### 2. A real whip crack: the damage lands when the tip arrives

- **Wind-up (about 0.12 s):** the chain swings back over her shoulder. A small arc is drawn behind her.
- **Lash (about 0.18 s):** the chain unrolls outward along the line, with a travelling S-shaped wave.
- **The crack:** a white-gold flash, a ring of sparks, a tiny screen shake and a "crack" sound.
- **Damage timing:** foes along the chain are hit as the wave passes them. The **tip, the last 25% of the reach, hits for +50% ("the sweet spot")** and sets holy fire.
  - Up close she still hits, for normal damage. Standing back and catching foes with the tip is the skilful play.

### 3. Alternating lashes, with a combo finisher

- Strikes alternate **forehand and backhand**: the chain curls to the left, then to the right.
- **Every third strike** in a row (within 1.5 s) is an **overhead crack**:
  - the chain arcs up high and slams down;
  - it reaches a little further (+1 tile);
  - it hits a wider area at the tip.

### 4. Looks

- The chain is longer and drawn with **motion trails** (its last two positions fading behind it).
- The censer at the tip swings with **glowing coals that leave embers**.
- A **new attack animation** (PixelLab, about 6 generations): a big overhead whip swing with her arm fully extended, to match the longer chain. The current pose is a short swing. I'll show you it before using it.

### 5. Feel and balance

- The attack takes slightly longer: 0.45 s now → **0.5 s**, with the wind-up included.
  - Melee-range enemies can punish her a bit more.
  - The long reach keeps her safe.
- Damage numbers stay the same, plus the tip bonus.
- Brands, holy fire, Judgment gain and Zealotry all work as before.
- The test bot learns to stand back about 4 tiles when she's the hero.

### 6. Tests

- A foe at 5 tiles is hit; at 6.5 tiles it is not.
- Walls stop the whip.
- A foe at the tip takes 1.5× damage.
- The third strike in a combo is the overhead crack.
- The damage is delayed until the wave passes, so nothing is hit on the first frame.
- Bot runs with the Inquisitor in all 4 acts.
- Snapshots: the wind-up, mid-lash, the crack, and the overhead finisher.

## Order of work

1. Reach, line hit, wall stop and delayed damage, with tests.
2. Wind-up, travelling wave, crack, trails and embers (code visuals), with snapshots.
3. Forehand, backhand and the overhead finisher.
4. The PixelLab attack animation (shown to the user first).
5. Bot tuning, docs, commit and push.

# The Inquisitor: eighth playable class, a chained-censer hunter of the cursed (design, 2026-10-06)

**Decided with the user:**
- **Name:** the Inquisitor. The Act 4 monster once called the "Inquisitor Automaton" is now the **Censer Automaton**.
- **Resource:** **Judgment**.
- **Chain abilities:** all four kinds:
  - hook and pull;
  - binding chains;
  - chain lash and whirl;
  - chain links between branded foes.
- **Cursed foes:** the unholy and every boss.

**Look:** the user's Gothic Inquisitor, verbatim in [docs/concepts/GOTHIC_INQUISITOR.md](concepts/GOTHIC_INQUISITOR.md). In short:
- Ash-blonde hair, braided with chains and charms, under a black hood.
- The Iron Halo of Judgment behind her head.
- A black corset and a split inquisitorial coat, stitched with burned prayer strips.
- Blackened gauntlets, with a chain wrapped around one forearm.
- The **Censer of Judgment**: a spiked blackened-brass censer full of glowing coals, swung on several feet of chain.

## Identity

She is **reach melee**.
- **The censer:** she swings it on its chain, so she hits from 2.6 tiles away, further than any other melee hero.
- **Chains:** every skill uses the chain, to:
  - lash a line of foes;
  - hook a foe to her, or pull her to a big one;
  - bind a crowd to the floor;
  - spin the chain in circles;
  - link branded foes so that pain spreads between them.

She **brands** what she hunts.
- **What a brand does:** a branded foe takes more damage from her and glows through the dark.
- **Cursed foes:** the unholy (undead, ghosts, witches, vampires, werewolves, cultists, wraiths) and every boss. Against them she is stronger still: the stronger the curse (champions, elites, bosses), the stronger she gets.

## Holy fire (added at the user's request, 2026-10-06)

Her censer burns with **holy fire**, white-gold flames unlike any ordinary fire.
- **Burning:** every foe her censer touches (strikes and the sweep) keeps burning for 3 seconds, taking an eighth of the blow's damage per second. Cursed foes burn twice as hard.
- **Resistance:** holy fire ignores fire resistance.
- **Spreading:** when a cursed foe dies burning, the flame leaps to the nearest unburnt foe within 4 tiles.

## Signature mechanic: Judgment

**Basics:**
- Judgment runs from 0 to 100 and starts empty.
- **Building it:**
  - every censer hit adds Judgment;
  - a hit on a cursed foe adds more;
  - a hit on a branded cursed foe adds the most;
  - branding a foe adds some too.
- **Spending it:** her skills spend it. The Censer Strike is free.
- **Fading:** out of combat it slowly fades.

**Zeal:** each branded cursed foe within 10 tiles makes her faster (+5% each, up to +25%). Her eyes and the brands glow amber.

## Skill tree (char levels 1 / 6 / 12 / 18)

| Tier | Skill | What it does |
|---|---|---|
| 1 | **Censer Strike** | Free. Swings the censer on its chain at reach, hitting everything in a narrow arc. Cursed foes catch holy fire. |
| 1 | **Brand of Judgment** | Burns a seal into a foe (and those right beside it). Branded foes take more damage from her; cursed ones far more. |
| 1 | **Zealotry** (passive) | More damage against the cursed, and a longer chain (reach). |
| 2 | **Chain Lash** | Cracks the chain out in a long straight line, cutting and staggering everything along it. |
| 2 | **Hook** | Throws the chain: drags the first foe hit to her feet, stunned. A boss is too heavy, so she is pulled to it instead. |
| 2 | **Iron Halo** (passive) | Cursed foes hurt her less, and every blow she takes adds Judgment. |
| 3 | **Censer Sweep** | Spins the censer around her in great circles for a few seconds, staggering everything near; cursed foes ignite. |
| 3 | **Binding Chains** | Chains burst from the floor at a spot and pin every foe there in place. |
| 3 | **Chain Links** (passive) | Branded foes are chained together: part of the damage she deals to one spills to every other branded foe nearby. |
| 4 | **Purification** | Slams the censer into the floor: a wave of burning incense. Cursed foes take double. Champions lose their powers. Some possessed foes break free and briefly fight for her. Everything hit is branded. |
| 4 | **Final Judgment** | Her ultimate state, for a few seconds: the halo ignites and every brand blazes. She moves and strikes faster, and hits harder for every branded foe nearby. Every censer hit sends holy fire to all branded foes. |

**Out of Judgment:** any skill falls back to the free Censer Strike.

## Art as built (2026-10-06, iterated with the user)

- **Sprite:** v2, chosen for its spiked halo, black leather and oxblood.
- **Halo:** `tools/inquisitor_halo.py` (run by pack.py on every frame):
  - strips PixelLab's gold sun halo;
  - paints the user's design: spikes behind her head, with a band arching through them;
  - colours the halo and her armor in **darkened gold** (the user: "kind of darkened gold").
- **The censer** is drawn by the game on her chain when she strikes.
- **Portrait:** built from her sprite, standing straight (the user didn't want bent legs), and redrawn at strength 450. The halo was then painted back in, the censer's gem recoloured to holy fire, and her eyes made amber (`tools/inquisitor_portrait.py`).

## Art plan (PixelLab, `tools/inquisitor_art.py`)

- **Portrait:** full body on a ruined-cathedral background (candles, the stained-glass judge). Short, feature-first prompts; "full body"; never "portrait".
- **Sprite (56 px):** the hood, the halo, the long coat and the censer on its chain.
- **Animations:**
  - walk;
  - attack (a flail swing);
  - cast (pointing to brand);
  - spin (Censer Sweep).
- **Effects:** the chain, the brands and the halo are drawn in code.

# The Valkyrie: fourth playable class, the first melee hero (design, 2026-10-05)

**Built 2026-10-05** (code complete with a stand-in sprite; her PixelLab art waits on generations).

**Decided with the user:**
- **Name:** Valkyrie.
- **Resource:** Valor (builds by fighting, spent by skills, fades out of combat).
- **Warhorse:** a charge skill only (Ride of the Valkyrie), not a mount.
- **Wings:** only in skills (Glacier Leap, Fimbulwinter) and the portrait; not on the walking sprite.

## Look (the user's description, condensed)

A glamorous northern **Frost-Rime Valkyrie**, a dark Nordic gothic warrior queen.

- **Hair:** very long, ash-black fading to icy silver-blue, Nordic braids with silver rings, bones, raven feathers and blue crystals, frost in the strands.
- **Face:** pale skin, glowing glacier-blue eyes, charcoal war paint, smoky winged eyeliner, blue-black lips. A jagged silver crown of frozen branches and runes with sapphires and ice shards.
- **Armor:**
  - A blackened-steel breastplate and corset with glowing blue runes and silver knotwork.
  - Black fur over one shoulder, woven with frosted raven feathers.
  - A layered battle skirt of black leather, charcoal cloth, grey fur and midnight blue, with the edges freezing into ice.
  - Engraved gauntlets with glowing cracks.
  - Over-the-knee steel boots with ice spikes.
- **Weapon:** an enormous dark iron and silver **rune spear** with a glacier-ice blade that glows blue.
- **Companions:** a **frost raven** on her shoulder and an armored black **warhorse** with a frost-white mane.
- **Wings:** huge raven-feather wings turning into ice shards.
- **Palette:** blackened steel, charcoal, silver, glacier blue and midnight blue. **Absolutely no red.**
- **Background (portrait):** a frozen battlefield in a blizzard, with aurora, standing stones, frozen longships, ravens and spectral warriors.

**At sprite size (about 56 px)** the wings, horse and raven would turn into noise if drawn all the time. So:
- **The portrait** shows everything.
- **The walking sprite** shows the spear, the armor and the fur mantle.
- The raven, the wings and the horse appear as **skills**: the raven flies off to strike, the wings spread for her leap and her ultimate, and the horse comes for a charge.

**Home turf:** Act 2, the Frostmarch. Kaldholm's skalds sing of her kind.

## Identity

The sorceress burns, the vampire drains, the inventor builds; the valkyrie **closes in and holds the line**.

- **Range:** spear reach (about 2 tiles, longer than the vampire's claws).
- **Hits:** wide arcs that strike several foes at once.
- **Frost:** her attacks **chill** and eventually **freeze** enemies; frozen enemies **shatter** when killed.
- **Toughness:** she has the most life and armor of any class and the least mana.

**Damage type:** a split of physical and frost.
- **Frost** slows and freezes.
- Act 2's creatures of the cold shrug off the frost part, but the **physical** half of every hit still lands, so she is never helpless in her own homeland.
- Fire-themed foes (the Ash King's servants) take extra frost damage.

## Draft skill tree (char levels 1 / 6 / 12 / 18)

| Tier | Skill | What it does |
|---|---|---|
| 1 | **Rune Spear** | Her basic attack. A fast reaching thrust that hits everything in a short line and builds the class resource. (Her Fireball.) |
| 1 | **Rime Sweep** | A wide frost arc in front of her: hits every foe in the cone and chills them. |
| 1 | **Northborn** (passive) | More life, armor and cold resistance; chill on her wears off faster. |
| 2 | **Raven Strike** | Her raven dives at a target, pecks it and **marks** it. Marked foes take more damage from her for a few seconds. |
| 2 | **Glacier Leap** | Her ice wings spread: she leaps to a spot and lands with a freezing shockwave. Her movement skill. |
| 2 | **Frost Brand** (passive) | Hits build chill stacks; at full stacks the foe **freezes** solid. Frozen foes killed by her **shatter** into ice shards that hit nearby foes. |
| 3 | **Rune Javelin** | Hurls a spectral copy of her spear: it pierces through a line of foes, then flies back to her. |
| 3 | **Winter's Wrath** | Whirls the spear in a full circle: big damage all around her. |
| 3 | **Einherjar** | Calls 2-3 spectral warriors from the snow to fight beside her for a while. |
| 4 | **Ride of the Valkyrie** | Her frost warhorse charges in with her: a long trampling charge in a line that knocks foes aside. |
| 4 | **Fimbulwinter** | Her ultimate. Wings spread, she rises, and a killing blizzard rages around her for several seconds, freezing everything inside. |

## Signature mechanic: Valor (decided)

A melee hero should *want* to be in the fight. So she uses **Valor** instead of mana, like Diablo 3's Fury:
- **The bar:** an ice-blue **Valor** bar, starting empty.
- **Building:** every Rune Spear hit and every hit she *takes* adds Valor.
- **Spending:** her other skills spend Valor.
- **Decay:** Valor slowly drains when she's out of combat.
- **At full Valor** her runes blaze: +15% damage until she spends.
- **Potions:** mana potions become **mead**, which grants instant Valor.
- **Gear:** "+mana" counts as max Valor, and "mana regeneration" slows the decay.

## Melee in the engine (what has to be built)

The game is built around casters, so melee needs some groundwork:

1. **Attack-move:** clicking or holding on a monster walks her into reach, then attacks. The vampire's Rake and Bite already do a short version of this.
2. **Arc and line hit tests:**
   - cone hits for Rime Sweep;
   - line hits for Rune Spear, Javelin and the horse charge;
   - circle hits for Winter's Wrath and Fimbulwinter.
3. **Monster status:** a `marked` timer, chill *stacks* and a `frozen` state. Frozen = can't move or attack, tinted ice-blue; the existing stun and chill code covers half of this.
4. **The shatter-on-death burst:** reuses the burst-on-death hook added for boiler brutes.
5. **Summons:**
   - the Einherjar reuse the vampire's thralls;
   - the raven is a short-lived flying projectile;
   - the horse charge is a timed form, like the Steam Suit.
6. **Tuning so melee isn't suicide:**
   - she gets a short **hit-recovery** shrug: brief damage reduction after taking a big hit;
   - she has the best base life and armor.
7. **Controller / handheld:** the primary button attacks the nearest foe in front of her (auto-target), so she plays well on the RG35XX.

## Art plan (PixelLab, `tools/valkyrie_art.py`)

- **Portrait:** from the user's description, keyed onto a frozen battlefield background like the other three. Small realistic head, never chibi.
- **Sprite (about 56 px), 8 directions:** a `heroic` preset, the spear held upright, the fur mantle, no wings.
- **Animations:**
  - walk;
  - spear thrust (attack);
  - wide sweep;
  - whirl;
  - javelin throw;
  - a cast pose (hand raised, runes).
- **Extra sprites:**
  - the frost raven (small flier);
  - the armored warhorse, with her riding it for the charge (a quadruped template, the way the dragon used `bear`);
  - spectral warrior (Einherjar, tinted ghostly blue);
  - wing and blizzard effects drawn in code.
- **Skill icons:** code-drawn, like the others.
- **Cost estimate:** roughly the Inventor's budget plus the horse: about 1 character set, 6 animations, 3 extra characters with 2-3 animations each, and 1 portrait.

## Build order (when approved)

1. **Engine groundwork:** attack-move, cone/line/circle hit tests, chill stacks, frozen, marked, shatter.
2. **Valor resource:** the HUD globe and gear mapping.
3. **The 11 skills** in `valkyrie.rs`, with unit tests for each, like the vampire and inventor.
4. **Class select:** a fourth card (the menu is laid out for three, so it needs a wider row or a carousel).
5. **Art:** portrait first for approval, then the sprite and animations, then the raven, horse and Einherjar.
6. **Snapshots, docs, a `--valkyrie` flag, commit and push.**

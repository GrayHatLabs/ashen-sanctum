# The Reaper: sixth playable class (design, 2026-10-05)

**Decided with the user:**
- **Name:** Reaper.
- **Playstyle:** scythe + spirits, a hybrid of melee and caster.
- **Resource:** Souls.
- **Rune blade:** the hunt stacks are in.

**Look:** the user's Rune Library Reaper, verbatim in [docs/concepts/RUNE_LIBRARY_REAPER.md](concepts/RUNE_LIBRARY_REAPER.md). In short:
- Silver-grey floor-length hair, ghostly pale skin, amber eyes.
- A black reaper cloak that dissolves into smoke.
- A black corset and mourning skirt hung with keys and hourglasses.
- A huge **rune scythe** with a spirit-flame **lantern** under the blade.
- The floating **Ledger of the Forgotten**, a raven, and drifting scholar spirits.
- **Palette:** black, charcoal, aged bronze, parchment, muted amber, and spectral blue.

## Identity

She is not Death, but she **collects**.
- **Souls:** every foe that dies near her releases a soul that drifts into her lantern.
- **Reach:** her scythe reaps at mid reach, wider than the spear and slower than the axe.
- **Magic:** spectral, with the lantern's flame, chains, hourglasses, and the spirits of dead scholars.
- **Toughness:** between the casters and the melee heroes.

## Signature mechanics

**Souls (decided).** Souls live in `p.mana`, shown as 0-10 souls.
- She starts with none and gathers them from deaths near her:
  - 1 per foe;
  - 3 per boss, or per foe that dies while written in her Ledger.
- Her bigger skills spend souls, and the Reaping Scythe is free.
- Souls don't drain away; she is patient.
- Mana potions become **ink**: a soul each.

**The rune blade (decided).**
- Each scythe hit on the foe she's hunting lights one more rune on the blade, up to 7.
- Switching prey starts over.
- At 7 the whole blade burns blue, and her next Reaping Scythe cleaves for **triple damage** in a wider arc.

## Skill tree (char levels 1 / 6 / 12 / 18)

| Tier | Skill | What it does |
|---|---|---|
| 1 | **Reaping Scythe** | A wide mid-reach sweep. Free. Lights the runes. |
| 1 | **Spirit Lantern** | The lantern's pale flame flies out as a homing spirit bolt. 1 soul. |
| 1 | **Patient Archivist** (passive) | Gathers souls from farther; each soul gathered heals her a little; +1 soul capacity every 2 ranks. |
| 2 | **Ledger Mark** | Writes a foe's name in the Ledger: it takes more damage from her, and gives 3 souls if it dies marked. |
| 2 | **Scholar Spirits** | Ghosts of dead scholars fight beside her for a while. |
| 2 | **Shadow Step** | Dissolves into black smoke and steps to a nearby spot. Her movement skill. |
| 3 | **Chains of the Archive** | Spectral chains burst from the floor and bind every foe in an area. |
| 3 | **Hourglass** | Slows time in an area: foes in it crawl. |
| 3 | **Rune Blade** (passive) | Each lit rune adds damage, and the blazing cleave hits harder. |
| 4 | **Soul Harvest** | A great reaping circle: everything around her is cut, and the badly wounded are reaped outright. |
| 4 | **Open the Ledger** | Her ultimate. The grimoire opens: for a while she hits harder, and every foe that dies near her gives double souls and rises as a scholar spirit. |

## Art plan (PixelLab, `tools/reaper_art.py`, when generations return)

- **Portrait:** from the concept, keyed onto a forbidden-library background.
- **Sprite (56 px):** the scythe, hood and cloak.
- **Animations:** walk, sweep, cast, and spin (Soul Harvest).
- **Extras:** a scholar spirit, plus the souls, lantern flame, chains, hourglass and Ledger drawn in code.
- **Stand-in until then:** the Act 3 cultist (hooded, robed, carrying a tome), tinted charcoal and silver.

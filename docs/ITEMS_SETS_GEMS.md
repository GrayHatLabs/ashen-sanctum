# Sets, gems and jewelers (2026-10-06)

**Decided with the user:**
- **Sets:** one per hero, plus a few shared sets.
- **Gems:** D2 style.
- **Jewelers:** one in every town.

## Set items (green)

- **Look:** set items have a green name and a green inventory cell.
- **Bonuses:** each set has 3 or 4 pieces. Wearing 2, 3 or 4 different pieces adds bonuses on top of each piece's own stats.
- **Where to see them:**
  - the tooltip lists every bonus;
  - the inventory totals show how many pieces of each set you wear (for example `EMBERWEAVE 3/4`).
- **Drops:**
  - a set roll falls between unique and rare (about 1% of drops, more with magic find and from champions and bosses);
  - only pieces whose set level is within 3 of the area's item level drop;
  - half of all set drops come from **your own hero's set**.
- **Who can wear them:** anyone. A hero set is only themed for its hero.

| Set | For | Level | Pieces |
|---|---|---|---|
| EMBERWEAVE | Sorceress | 12 | spire (staff), circlet, robe, sash |
| NIGHTBLOOD | Vampire | 14 | cowl, shroud, talons, signet |
| BRASSWORK | Inventor | 14 | goggles, grips, treads, mainspring |
| OATHSWORN | Valkyrie | 16 | winghelm, mail, greaves |
| BLOODHOWL | Berserker | 16 | mask, hide, fists |
| GRAVESONG | Reaper | 20 | veil, gown, band, locket |
| THORNMOTHER | Druid | 20 | crown, bark, roots, vine |
| WANDERER'S | shared | 4 | sandals, cord, mitts |
| GILDED HAND | shared | 12 | ring, gloves, chain (magic find and gold) |
| ASHEN REGALIA | shared | 30 | scepter, helm, mail, torc (+2 skills with all four) |

## Gems and sockets (D2 style)

- **Gem kinds:** ruby, sapphire, topaz, emerald, amethyst, diamond and skull.
- **Grades:** chipped, flawed, (plain), flawless and perfect.
- **What a gem gives** depends on where it's set:

| Gem | In a weapon | In armor (helm, body, gloves, boots, belt) | In jewelry |
|---|---|---|---|
| Ruby | % damage | life | life after each kill |
| Sapphire | mana after each kill | mana | mana regeneration |
| Topaz | extra gold | magic find | extra gold |
| Emerald | faster cast | slower stamina drain | faster run/walk |
| Amethyst | life | armor | slower hunger |
| Diamond | mana regeneration | armor and life | magic find |
| Skull | life and mana after kills | life and mana regeneration | life and mana |

- **Sockets:**
  - white gear sometimes drops with 1 to 3 sockets (weapons and body armor 3, helms 2, everything else 1);
  - blue gear does now and then, with up to 2;
  - small dots under an icon show its sockets, filled with the colours of their gems.
- **Setting a gem:** choose the gem (ENTER/A, or click it twice), then choose the item (worn or in the bag).
- **Gem drops:**
  - any monster can drop a gem (4%; champions 15%, elites 30%);
  - every boss drops one;
  - deeper areas drop better grades (chipped to flawless; perfect only by joining).

## Jewelers

| Town | Jeweler |
|---|---|
| Hollowmere | Master Odo |
| Kaldholm | Ingrid Stonehand |
| Mournhold | Silas Greave |
| The Last Escapement | the Lapidary (a clockwork lens-eyed automaton) |

Each jeweler offers two things:
- **Join my gems:** every three gems of one kind and grade become one of the next grade, while your gold lasts. It costs 30, 80, 200 or 500 gold for the result.
- **Sockets:** opens the inventory at the jeweler's bench, where choosing an item:
  - with gems in it takes them out unharmed (25 gold per gem grade);
  - if it's plain white gear without sockets, cuts 1 to the slot's maximum sockets (60 + 15 per item level gold).

## Code

- **`src/items.rs`:**
  - `Rarity::Set`, `Gem`, `GEM_TABLE`, `gem_item`, `roll_gem`, `max_sockets`;
  - `SETS`, `set_item`, `set_of`, `hero_set_piece`;
  - `Item.sockets` and `Item.gems`, and `Gear::{sets_worn, socket, unsocket, next_combine, combine_one}`.
- **Saving:** the save line gains a 7th field, `sockets;kind.grade,...`. Older saves load without sockets.
- **`src/inventory.rs`:** `InvUi.jewel` (the bench) and `InvUi.holding` (a gem in hand), plus the gem and socket icons.
- **`src/story.rs`:** `Role::Jeweler(act)`, `Act::Combine` and `Act::Jewel`.
- **`src/world.rs`:** the jewelers' places in each town.
- **Art:** `tools/jeweler_art.py` (art repo) and gem icons in `tools/items_art.py`. Code-drawn stand-ins until those are imported.

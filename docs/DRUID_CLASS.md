# The Druid: seventh playable class, a plague summoner (design, 2026-10-06)

**Decided with the user:**
- **Name:** Druid.
- **Playstyle:** a summoner.
- **Resource:** mana plus a **Decay / Bloom** balance.
- **Big summon:** the **Thorn Warden** (renamed from "Plague Warden", which is already the name of an Act 1 boss).

**Look:** the user's Thorn Plague Druid, verbatim in [docs/concepts/THORN_PLAGUE_DRUID.md](concepts/THORN_PLAGUE_DRUID.md). In short:
- Tangled chestnut hair grown through with ivy and blackthorn.
- A living blackthorn crown like antlers, and yellow-green eyes.
- A raven-feather mantle and a bark-and-leather corset.
- A skirt that turns into roots, and a plague-thorn staff with a glowing green orb.
- Bottles and a plague-doctor mask on her belt.
- Rats, a moss-grown wolf and a raven follow her.

## Identity

She is neither healer nor destroyer; **she keeps the balance**.
- **Fighting:** she lets her creatures fight (rats, her moss wolf, and at last the Thorn Warden). Meanwhile she spreads plague (spore clouds, thorns, bursting fungi) and mends her own with growth magic.
- **Damage:** poison. Foes keep taking damage for a while after she hits them.

## Signature mechanic: Decay and Bloom (decided)

She uses normal mana, plus a **balance** between Decay and Bloom (-1 to +1), shown as a bar under her mana globe.
- **Moving the balance:**
  - Her **plague** skills (spores, thorns, fungi, pestilence) push it toward **Decay**.
  - Her **growth** skills (summons, healing) push it toward **Bloom**.
- **Each side feeds the other:**
  - Leaning toward Bloom makes her plague hit harder, up to +30%.
  - Leaning toward Decay makes her creatures and healing stronger, up to +30%.
  - So she plays best alternating, rot then growth, like the cycle she keeps.
- **Drift:** the balance drifts slowly back to the middle.

## Skill tree (char levels 1 / 6 / 12 / 18)

| Tier | Skill | What it does | Pushes toward |
|---|---|---|---|
| 1 | **Spore Cloud** | Throws a cloud of glowing spores that poisons everything in it. | Decay |
| 1 | **Rat Swarm** | Calls a swarm of black rats to fight for her for a while. | Bloom |
| 1 | **Green Doctor** (passive) | Poison lasts longer and bites deeper; her creatures are tougher. | - |
| 2 | **Thorn Lash** | Blackthorn vines erupt in a line, binding and cutting foes. | Decay |
| 2 | **Moss Wolf** | Her great moss-grown wolf joins her and stays until it falls. | Bloom |
| 2 | **Rejuvenate** | A bloom of growth heals her and her creatures over a few seconds. | Bloom |
| 3 | **Fungal Bloom** | Mushrooms sprout around a spot and burst into poison clouds. | Decay |
| 3 | **Corpse Bloom** | A corpse bursts into spores and rats crawl out of it. | Bloom |
| 3 | **Cycle of Rot** (passive) | The balance bonuses grow, and foes that die poisoned heal her. | - |
| 4 | **Pestilence** | A wave of plague poisons every foe around her; the plague spreads when they die. | Decay |
| 4 | **Thorn Warden** | Summons the Thorn Warden, a great guardian of dead trees, roots, bones and antlers, to fight beside her. | Bloom |

## Art plan (PixelLab, `tools/druid_art.py`)

- **Portrait:** from the concept, on an overgrown-cathedral background.
- **Sprite (56 px):** the staff, crown and mantle. Short, feature-first prompts; "full body"; no "portrait".
- **Animations:**
  - walk;
  - cast (staff raised);
  - summon (hand to the ground).
- **Companions:**
  - the moss wolf (dog template);
  - the Thorn Warden (a big humanoid tree guardian);
  - rats (too small and not humanoid for the character endpoint, so drawn in code).

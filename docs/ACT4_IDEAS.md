# Act 4 ideas: Mechanus (2026-10-05)

The user asked for "a gothic steampunk world based on Mechanus".

## Setting: the Clockwork Dominion

Beyond the vampire lord's castle, a door opens onto **Mechanus**, the plane of perfect law. It's an endless city of brass gears the size of cathedrals, turning in the sky.
- Gothic spires made of iron and stained glass.
- Steam vents and soot-black chapels where prayers are punched into brass cards.
- Copper, verdigris green, soot black and boiler-fire orange, under a tick-tocking sky.

Hook: someone has been *winding the world*. The Ash King's fire, the Rime Wyrm's cold and the vampire lord's curse were all experiments, run to test whether the mortal world could be made **orderly**.

- **Town:** "The Last Escapement", a refuge of runaway clockwork servants who grew souls, plus a few human tinkerers.
- **Overland:** "The Grinding Fields". Gear-tooth bridges over a void, conveyor roads, steam geysers, and great pendulums sweeping across the paths as a hazard.
- **Dungeons:**
  1. **The Foundry of Souls:** molten brass, conveyor floors that carry you.
  2. **The Choir Engine:** a cathedral-organ factory. Its pipes blast sound waves.
  3. **The Archive of Gears:** an infinite punch-card library; it rearranges its rooms as clock hands sweep.
  4. **The Heart of the Clock:** the final citadel, inside the giant clock.

## Bad guys

**Regular monsters**
- **Cog-Hounds:** brass wolves with gear teeth, hunting in packs. Steam vents from them when hit.
- **Inquisitor Automatons:** gothic clockwork monks with censers that leak scalding steam.
- **Gearwraiths:** ghostly souls trapped in cage-like brass frames. They phase, and burst free when killed.
- **Spring-Heeled Jacks:** spindly sprinting mechanicals that leap from the shadows to strike, then bound away.
- **Boiler Brutes:** hulking furnace-bellied golems. They're slow but explode if you kill them while they glow red.
- **Clockwork Crows:** swarms, the mechanical cousins of Act 3's crows.
- **Modron-inspired "Ordinals":** cube, pyramid and sphere drones in rank formations. They march in lockstep, and kill the leader to break the formation. (Own designs, not D&D's modrons.)

**Heralds (one per dungeon)**
- **The Forgemother:** a matriarch of molten brass. She pours slag rivers and rebuilds fallen automatons.
- **The Cantor:** a cathedral organ on spider legs. Sound waves, a rhythm you can dodge, and its pipes as weak points.
- **The Archivist:** a many-armed librarian of punch cards. It "files" you by teleporting you into side rooms, and summons copies of past bosses from its records.

## The end bad guy: options

1. **The Clockmaker** (recommended)
   - Who: a tall gothic figure in a stovepipe hat and soot-black coat, the god-engineer of Mechanus. Half his body is an exposed golden clockwork heart.
   - Why: he has been winding the world, and the earlier acts were his experiments.
   - Fight:
     - **Phase 1:** he duels with time. Rewind pulses snap you back to where you stood 3 seconds ago, and clock-hand blades sweep the arena.
     - **Phase 2:** he climbs into the great clock's escapement, and the whole arena becomes the boss. Gears grind, pendulums swing, and you smash the mainspring.
2. **Primus Ferrum, the Iron Saint**
   - Who: a colossal cathedral-sized automaton saint of absolute law, all stained glass and brass halo.
   - Fight: judgement beams, and it "decrees" rules mid-fight. For example "NO MAGIC FOR 5 SECONDS" or "STANDING STILL IS FORBIDDEN".
3. **The Orrery King**
   - Who: a mad astronomer-king fused to a giant clockwork orrery.
   - Fight: the planets orbit as rotating hazards, and he speeds them up as he weakens.

**Mechanic idea for the act:** *time*. Clocks you can strike to slow an area, rewind pulses, and timed doors. Combined with Mechanus's order, there could be "law zones" where only one type of damage works.

## Built (2026-10-05)

The user picked **the Clockmaker**, unlocked **after Count Vardak** (a gear gate on the castle grounds).
As built: Tally / Madame Vesper / Brother Piston in the Last Escapement; Forgemother, Cantor, Archivist
hold the three Winding Keys; the Clockmaker duels (blade spirals, rewind) then pilots his great engine
(pendulum slams, ordinals). Not built yet: crows, ordinal formations. Built 2026-10-06: stop-clocks and law zones (`src/clockwork.rs`; laws judge distance and stillness, as the game has no damage types).

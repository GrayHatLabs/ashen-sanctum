# Art upgrade plan: toward a Diablo II: Resurrected look

Status: **plan only** (2026-10-13). Nothing here is built yet.

Decided so far:
- **Full hi-res painted look**, not upgraded pixel art.
- **Characters and monsters are 3D models rendered to sprites**, the way D2 and D2R were made.

## Where the game is today

| | Now | D2R-like target |
|---|---|---|
| Internal resolution | 640×360 (desktop), 640×480 (RG35XX H), scaled up nearest-neighbour | 1280×720 (or 1920×1080) native, smooth scaling |
| Renderer | CPU, one ARGB framebuffer | GPU (textures, blending, shaders) |
| Tiles | 32×16 diamonds, PixelLab, 1–3 floors and one wall per theme, many recoloured stone | 64×32 (or 96×48), painted or rendered, 6–12 floors, wall kits, decals |
| Characters | ~48 px, PixelLab, 5 drawn directions mirrored to 8, 6-frame anims, single-frame idle | 128–160 px, rendered from 3D, 8 (or 16) true directions, 12–20-frame anims, breathing idle, hit, death |
| Alpha | 1-bit (cut-out edges) | 8-bit (soft edges, smoke, glows) |
| Lighting | brightness only, per pixel on the CPU, no shadows | coloured lights, soft sprite shadows, normal-mapped sprites lit by torches and spells |
| Assets | 13 MB, zlib blobs inside the .exe, all decoded at start-up | hundreds of MB; packed atlases on disk, loaded per act |

Source: art pipeline survey (src/gfx.rs, src/iso.rs, src/art.rs, src/render.rs, tools/pack.py, scripts/import_art.py).

## The big recommendation: a Classic / Remastered toggle

D2R's best feature: one key swaps between the old and new graphics. The game logic here already runs in world
units (tiles), not pixels, so **two art sets can share one game**:
- **Classic**: today's 640-wide pixel art. Stays the handheld default (RG35XX, Odin), so no extra memory there.
- **Remastered**: the new hi-res art on desktop (and Odin 2 at 1080p if it holds up).

This means the upgrade can roll out **one act or class at a time**. Anything not yet remade falls back to
upscaled classic art (the stand-in system in art.rs already does this for missing art).

## Phase A: the engine work (before any new art)

1. **GPU renderer.** Move drawing from the CPU framebuffer to SDL2 textures (or wgpu): sprites as textures,
   alpha blending, lights as additive passes. The CPU path can't push 4× the pixels on handhelds.
   The Classic renderer can stay as is.
2. **Resolution-independent drawing.** The tile size (`iso::TW/TH`), UI layout and fonts are read from the
   active art set instead of being constants. The HUD gets a hi-res layout (D2R's HUD is a 1080p painting).
3. **Asset format v2.** 8-bit alpha, plus optional normal and shadow layers per sprite. Atlases go to disk as
   compressed packs (zstd or WebP), loaded per act, not embedded in the .exe (Defender already flagged a big .exe once).
4. **Lighting v2.** Coloured point lights (torches warm, magic cold), soft shadows under everything, sprites lit
   by their normal maps, light blocked by walls (optional; costly).
5. **A pilot screen:** one hi-res room, one hero and one monster, with a toggle to compare against Classic.

## Phase B: the art pipeline

**Characters and monsters (3D → sprites)**
1. Model: image/text-to-3D (Meshy or Tripo) from the existing concept art and portraits, so the heroes keep
   their look (Valkyrie: still no red).
2. Rig and animate: auto-rig plus a motion library (Mixamo is free; Meshy has rigging too). Custom moves
   (Rapier lunge, flintlock shot, Whirlwind) get keyframed or picked from the library.
3. Render: a Blender script with a fixed isometric camera matching the game's projection, a three-point light rig,
   8 directions × each animation. Each frame outputs colour, normals and a shadow pass.
4. Pack: the existing pack.py approach, extended for the new layers and atlases.

**Environments**
- Floors: painted seamless isometric textures (image models with tileable prompts), several per theme, plus
  decal overlays (cracks, bones, blood, puddles, moss) so no two screens repeat.
- Walls and props: 3D kits rendered from the same camera (best consistency with the characters), or painted
  with image models and cleaned up for the simpler props.
- Water and lava keep their animated shaders, at the higher resolution.

**UI**: HUD, globes, panels and icons repainted at 1080p (OpenAI image tools, as for the current portraits and HUD).

## Phase C: rollout order

1. Pilot: the Sorceress, 5 Act 1 monsters, the Ashlands tiles, the Ashen Sanctum. Review before going on.
2. The rest of Act 1 and all 8 heroes.
3. Acts 2–7, one act per milestone: tiles, props, monsters, then the act's bosses.
4. UI and portraits.

## Size of the job

- **Characters:** 8 heroes, ~120 monster types, ~30 bosses, about 160 models. Many monsters share a rig, so
  animation sets can be reused.
- **Frames:** each hero needs 8 directions × ~70 frames (idle, walk, run, attack, cast, hit, death, class extras),
  about 560 frames per hero. At 160 px that's ~10–15 MB compressed per hero, and a few hundred MB for the whole game.
- **Costs:**
  - A 3D generation service subscription for a few months.
  - Blender (free) and Mixamo (free).
  - Image-model costs for tiles and UI, similar to the portrait work.
- **Time:** phase A is several sessions. The pilot is a few more. Each act after that is a milestone of its own.

## Decisions still open (one at a time)

1. Target resolution: 1280×720 (lighter, 2× today) or 1920×1080 (true D2R, 3×).
2. GPU route: SDL2's renderer (simplest, enough for sprites and additive lights) or wgpu (real shaders, normal-mapped lighting).
3. Handhelds: Classic only, or a scaled-down Remastered pack for the Odin 2.
4. 3D service: Meshy vs Tripo (try both on one hero and one monster).
5. Directions: 8 like today, or 16 like D2 for smoother turning (2× the frames).
6. Should the pilot happen before or after the PortMaster release?

## Risks

- **Consistency:** AI 3D models vary. Fix one camera and light rig and a shared material look early, and
  review the pilot hard.
- **Handheld performance:** keep Classic as the handheld default until Remastered is measured on the Odin 2.
- **Scope:** the toggle and per-act rollout keep the game shippable at every step.
- **Licensing:** check the terms of whichever 3D service is used against CC BY-NC content and the PortMaster release.

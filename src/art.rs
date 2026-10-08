//! Generated art (PixelLab), embedded at build time by `scripts/import_art.py`
//! into `art_gen.rs`. Anything missing falls back to the code-drawn sprites in
//! `sprites.rs`, so the game always runs.
//!
//! Character sheets: one row per (animation, direction); rows for an animation
//! start at `row` and run through the 8 directions in PixelLab order
//! (south, south-east, east, north-east, north, north-west, west, south-west).
use crate::art_gen::{CHARS, ITEMS, TILES};
use crate::gfx::{mix, rgb, Sprite, BLACK};
use crate::world::Theme;
use crate::sprites;

pub struct AnimDef {
    pub name: &'static str,
    pub row: u32,
    pub frames: u32,
    pub fps: u32,
}

pub struct CharDef {
    pub name: &'static str,
    pub cell: (i32, i32),
    pub anchor: (i32, i32),
    pub data: &'static [u8],
    pub anims: &'static [AnimDef],
}

pub struct ItemDef {
    pub name: &'static str,
    pub data: &'static [u8],
}

pub struct TileDef {
    pub name: &'static str,
    pub anchor: (i32, i32),
    pub data: &'static [u8],
}

/// Decode a blob written by import_art.py: u32 w, u32 h, then w*h ARGB pixels.
pub fn decode(b: &[u8]) -> Sprite {
    let rd = |i: usize| u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]);
    let (w, h) = (rd(0) as i32, rd(4) as i32);
    let mut s = Sprite::new(w, h);
    for (i, p) in s.px.iter_mut().enumerate() {
        *p = rd(8 + i * 4);
    }
    s
}

fn crop(s: &Sprite, x: i32, y: i32, w: i32, h: i32) -> Sprite {
    let mut out = Sprite::new(w, h);
    for yy in 0..h {
        for xx in 0..w {
            out.px[(yy * w + xx) as usize] = s.get(x + xx, y + yy);
        }
    }
    out
}

pub struct Anim {
    pub name: String,
    /// frames[dir][frame]
    pub frames: Vec<Vec<Sprite>>,
    pub fps: u32,
}

pub struct CharArt {
    pub anims: Vec<Anim>,
    /// Height of the figure above its feet (for health bars, hit boxes).
    pub height: i32,
}

impl CharArt {
    pub fn anim(&self, name: &str) -> Option<&Anim> {
        self.anims.iter().find(|a| a.name == name)
    }

    /// Looping frame at time `t` seconds.
    pub fn frame(&self, name: &str, dir: usize, t: f32) -> &Sprite {
        let a = self.anim(name).or_else(|| self.anim("idle")).unwrap_or(&self.anims[0]);
        let fr = &a.frames[dir % a.frames.len()];
        let i = (t * a.fps as f32) as usize % fr.len();
        &fr[i]
    }

    /// One-shot frame at progress `p` in 0..1.
    pub fn frame_at(&self, name: &str, dir: usize, p: f32) -> &Sprite {
        let a = self.anim(name).or_else(|| self.anim("idle")).unwrap_or(&self.anims[0]);
        let fr = &a.frames[dir % a.frames.len()];
        let i = ((p.clamp(0.0, 0.999)) * fr.len() as f32) as usize;
        &fr[i.min(fr.len() - 1)]
    }

    pub fn has(&self, name: &str) -> bool {
        self.anim(name).is_some()
    }
}

pub struct Art {
    chars: Vec<(String, CharArt)>,
    /// Floor tiles per theme (dungeon themes are tinted copies of the stone set).
    floors: Vec<Vec<Sprite>>,
    walls: Vec<Sprite>,
    /// Overworld ground: grass variants, dirt, road.
    grass: Vec<Sprite>,
    dirt: Sprite,
    road: Sprite,
    /// Act 2 overland ground: snow variants, lake ice, snowy road.
    snow: Vec<Sprite>,
    lake: Sprite,
    snow_road: Sprite,
    /// Act 3 overland ground: blue-grey earth variants, glowing moss, muddy road.
    mist_earth: Vec<Sprite>,
    moss: Sprite,
    mist_road: Sprite,
    /// Act 4 overland ground: brass plates, verdigris copper, conveyor road.
    brass: Vec<Sprite>,
    verdigris: Sprite,
    conveyor: Sprite,
    /// Act 5 overland ground: sea-floor sand, the low tide flats, wreck-plank roads.
    sea_sand: Vec<Sprite>,
    flats: Sprite,
    deck: Sprite,
    /// Act 6 overland ground: cloud marble, pale sky grass, chain bridges.
    marble: Vec<Sprite>,
    sky_grass: Sprite,
    bridge: Sprite,
    /// Item and prop sprites (anchored at the bottom centre, where they sit on the floor).
    items: Vec<(&'static str, Sprite)>,
    /// Code-drawn props for any prop sprite that hasn't been generated.
    fallback_props: Vec<(&'static str, Sprite)>,
    missing: Sprite,
}

/// Dungeon theme colour grading: (tint colour, amount, brightness).
fn theme_grade(t: Theme) -> (u32, f32, f32) {
    match t {
        Theme::Overworld | Theme::Crypt | Theme::Tundra | Theme::IceCaves => (0, 0.0, 1.0),
        Theme::Warrens => (0x6a4a24, 0.38, 0.95),
        Theme::Catacombs => (0x4a3a78, 0.38, 0.95),
        Theme::Sanctum => (0x6a1408, 0.42, 0.8),
        Theme::Mines => (0x587890, 0.3, 0.88),
        Theme::Rime => (0x80b0e0, 0.25, 0.95),
        Theme::Glacier => (0x1a3a78, 0.3, 0.85),
        Theme::Mistwood | Theme::Castle | Theme::Mechanus => (0, 0.0, 1.0),
        Theme::Foundry => (0x6a3010, 0.4, 0.85),
        Theme::Choir => (0x3a2a40, 0.4, 0.8),
        Theme::Archive => (0x4a3a20, 0.4, 0.8),
        Theme::Clock => (0x1a1a20, 0.3, 0.85),
        Theme::Chapel => (0x305848, 0.4, 0.8),
        Theme::Gallows => (0x283a30, 0.45, 0.75),
        Theme::Barrow => (0x4a4838, 0.35, 0.8),
        Theme::Deep => (0, 0.0, 1.0),
        Theme::Wreck => (0x2a2018, 0.3, 0.8),
        Theme::Reef => (0x3a2a48, 0.25, 0.9),
        Theme::Trench => (0x081820, 0.5, 0.7),
        Theme::Drowned => (0x103038, 0.35, 0.85),
        Theme::Heavens => (0, 0.0, 1.0),
        Theme::Seraph => (0x3a3020, 0.25, 0.9),
        Theme::Spire => (0x201838, 0.4, 0.8),
        Theme::Wheel => (0x4a3a10, 0.3, 0.9),
        Theme::Zenith => (0x2a1008, 0.4, 0.8),
    }
}

fn grade(s: &Sprite, (col, a, bright): (u32, f32, f32)) -> Sprite {
    let mut out = s.clone();
    for p in out.px.iter_mut() {
        if *p != 0 {
            let c = if a > 0.0 { mix(*p, rgb(col), a) } else { *p };
            *p = mix(BLACK, c, bright);
        }
    }
    out
}

impl Art {
    pub fn load() -> Self {
        let mut chars: Vec<(String, CharArt)> = CHARS
            .iter()
            .map(|d| {
                let img = decode(d.data);
                let (cw, ch) = d.cell;
                let anims = d
                    .anims
                    .iter()
                    .map(|a| {
                        let frames = (0..8)
                            .map(|dir| {
                                (0..a.frames as i32)
                                    .map(|f| {
                                        let mut s = crop(&img, f * cw, (a.row as i32 + dir) * ch, cw, ch);
                                        s.ax = d.anchor.0;
                                        s.ay = d.anchor.1;
                                        s
                                    })
                                    .collect()
                            })
                            .collect();
                        Anim { name: a.name.to_string(), frames, fps: a.fps.max(1) }
                    })
                    .collect();
                let height = d.anchor.1 - top_of(&img, cw, ch);
                (d.name.to_string(), CharArt { anims, height })
            })
            .collect();
        for name in ["mage", "zombie", "skeleton"] {
            if !chars.iter().any(|c| c.0 == name) {
                chars.push((name.to_string(), sprites::fallback_char(name)));
            }
        }
        let tile = |t: &TileDef| {
            let mut s = decode(t.data);
            s.ax = t.anchor.0;
            s.ay = t.anchor.1;
            s
        };
        let named = |prefix: &str| -> Vec<Sprite> { TILES.iter().filter(|t| t.name.starts_with(prefix)).map(tile).collect() };
        let exact = |name: &str| -> Option<Sprite> { TILES.iter().find(|t| t.name == name).map(tile) };
        let mut stone = named("floor");
        if stone.is_empty() {
            stone = (0..3).map(sprites::fallback_floor).collect();
        }
        let wall = named("wall").into_iter().next().unwrap_or_else(sprites::fallback_wall);
        let palisade = exact("palisade").unwrap_or_else(|| grade(&sprites::fallback_wall(), (0x6a4a24, 0.6, 0.9)));
        // Act 2: snowy palisade, ice walls and ice floors (stand-ins graded from the stone set).
        let palisade_snow = exact("palisade_snow").unwrap_or_else(|| grade(&palisade, (0xe8f0ff, 0.45, 1.05)));
        let ice_wall = exact("ice_wall").unwrap_or_else(|| grade(&wall, (0x9ad0ff, 0.5, 1.0)));
        let ice1: Vec<Sprite> = exact("ice_floor1").map(|s| vec![s]).unwrap_or_else(|| stone.iter().map(|s| grade(s, (0xa8d8ff, 0.45, 1.0))).collect());
        let ice2: Vec<Sprite> = exact("ice_floor2").map(|s| vec![s]).unwrap_or_else(|| stone.iter().map(|s| grade(s, (0xc0d8f0, 0.35, 1.0))).collect());
        let castle_floor: Vec<Sprite> = exact("castle_floor").map(|s| vec![s]).unwrap_or_else(|| stone.iter().map(|s| grade(s, (0x501020, 0.3, 0.7))).collect());
        let castle_wall = exact("castle_wall").unwrap_or_else(|| grade(&wall, (0x301020, 0.35, 0.7)));
        let fence = exact("palisade_mist").unwrap_or_else(|| grade(&palisade, (0x304030, 0.4, 0.8)));
        let brass_floor: Vec<Sprite> = ["brass_plate1", "brass_plate2"].iter().filter_map(|n| exact(n)).collect();
        let brass_floor = if brass_floor.is_empty() { stone.iter().map(|s| grade(s, (0x8a6a30, 0.45, 0.8))).collect() } else { brass_floor };
        let grate = exact("grate_glow").unwrap_or_else(|| grade(&stone[0], (0x803010, 0.5, 0.8)));
        let clock_floor = exact("clock_floor").unwrap_or_else(|| grade(&stone[0], (0x101018, 0.5, 0.8)));
        let brass_wall = exact("brass_wall").unwrap_or_else(|| grade(&wall, (0x7a5a28, 0.45, 0.8)));
        let iron_fence = exact("fence_iron").unwrap_or_else(|| grade(&palisade, (0x2a2a2a, 0.6, 0.8)));
        // Act 5: sand, coral, wreck decks and the drowned temple (stand-ins graded from the stone set).
        let sand_floor: Vec<Sprite> = ["sea_sand1", "sea_sand2"].iter().filter_map(|n| exact(n)).collect();
        let sand_floor = if sand_floor.is_empty() { stone.iter().map(|s| grade(s, (0x2a4450, 0.5, 0.8))).collect() } else { sand_floor };
        let coral_floor = exact("coral_floor").unwrap_or_else(|| grade(&stone[0], (0xc08090, 0.4, 0.9)));
        let deck_floor = exact("wreck_deck").unwrap_or_else(|| grade(&stone[0], (0x4a3020, 0.55, 0.8)));
        let temple_floor = exact("sanctum_floor").unwrap_or_else(|| grade(&stone[0], (0x8ab0b0, 0.35, 0.85)));
        let temple_wall = exact("sanctum_wall").unwrap_or_else(|| grade(&wall, (0x4a7a78, 0.4, 0.8)));
        // The reef walls came out a loud purple: pulled toward dark sea-grey, a hint of violet left (2026-10-07).
        let coral_wall = exact("coral_wall").map(|s| grade(&s, (0x2e3e4a, 0.55, 0.85))).unwrap_or_else(|| grade(&wall, (0x3a3a50, 0.45, 0.8)));
        // Act 6: marble, storm stone and the burnt sanctum (stand-ins graded from the stone set).
        let marble_floor: Vec<Sprite> = ["cloud_marble1", "cloud_marble2"].iter().filter_map(|n| exact(n)).collect();
        let marble_floor = if marble_floor.is_empty() { stone.iter().map(|s| grade(s, (0xe8e0d0, 0.5, 1.1))).collect() } else { marble_floor };
        let choir_floor = exact("choir_floor").unwrap_or_else(|| grade(&stone[0], (0x3a3020, 0.45, 0.85)));
        let zenith_floor = exact("zenith_floor").unwrap_or_else(|| grade(&stone[0], (0x201008, 0.6, 0.75)));
        let marble_wall = exact("marble_wall").unwrap_or_else(|| grade(&wall, (0xe0d8c8, 0.5, 1.1)));
        let storm_wall = exact("storm_wall").unwrap_or_else(|| grade(&wall, (0x30204a, 0.5, 0.8)));
        let mut floors = vec![];
        let mut walls = vec![];
        for t in Theme::ALL {
            let set: Vec<Sprite> = match t {
                Theme::IceCaves | Theme::Glacier => ice1.iter().chain(ice2.iter().take(1)).map(|s| grade(s, theme_grade(t))).collect(),
                Theme::Rime | Theme::Mines => ice2.iter().chain(stone.iter().take(1)).map(|s| grade(s, theme_grade(t))).collect(),
                Theme::Castle => castle_floor.iter().chain(stone.iter().take(1)).map(|s| grade(s, (0x300818, 0.25, 0.8))).collect(),
                Theme::Foundry => brass_floor.iter().chain(std::iter::once(&grate)).map(|s| grade(s, theme_grade(t))).collect(),
                Theme::Choir | Theme::Archive => brass_floor.iter().map(|s| grade(s, theme_grade(t))).collect(),
                Theme::Clock => std::iter::once(&clock_floor).chain(brass_floor.iter().take(1)).map(|s| grade(s, theme_grade(t))).collect(),
                Theme::Wreck => std::iter::once(&deck_floor).chain(std::iter::once(&deck_floor)).chain(sand_floor.iter().take(1)).map(|s| grade(s, theme_grade(t))).collect(),
                Theme::Reef => std::iter::once(&coral_floor).chain(sand_floor.iter()).map(|s| grade(s, theme_grade(t))).collect(),
                Theme::Trench => sand_floor.iter().map(|s| grade(s, theme_grade(t))).collect(),
                Theme::Seraph => std::iter::once(&choir_floor).chain(marble_floor.iter()).map(|s| grade(s, theme_grade(t))).collect(),
                Theme::Spire => marble_floor.iter().map(|s| grade(s, theme_grade(t))).collect(),
                Theme::Wheel => marble_floor.iter().chain(std::iter::once(&choir_floor)).map(|s| grade(s, theme_grade(t))).collect(),
                Theme::Zenith => std::iter::once(&zenith_floor).chain(std::iter::once(&zenith_floor)).chain(marble_floor.iter().take(1)).map(|s| grade(s, theme_grade(t))).collect(),
                Theme::Drowned => std::iter::once(&temple_floor).chain(std::iter::once(&temple_floor)).chain(std::iter::once(&coral_floor)).map(|s| grade(s, theme_grade(t))).collect(),
                _ => stone.iter().map(|s| grade(s, theme_grade(t))).collect(),
            };
            floors.push(set);
            walls.push(match t {
                Theme::Overworld => palisade.clone(),
                Theme::Tundra => palisade_snow.clone(),
                Theme::Mistwood => fence.clone(),
                Theme::Castle => castle_wall.clone(),
                Theme::Mechanus => iron_fence.clone(),
                Theme::Deep | Theme::Reef => coral_wall.clone(),
                Theme::Trench => grade(&coral_wall, theme_grade(t)),
                Theme::Wreck => grade(&wall, (0x3a2818, 0.5, 0.75)),
                Theme::Drowned => temple_wall.clone(),
                Theme::Heavens | Theme::Seraph | Theme::Wheel => grade(&marble_wall, theme_grade(t)),
                Theme::Spire => storm_wall.clone(),
                Theme::Zenith => grade(&storm_wall, (0x401008, 0.5, 0.75)),
                Theme::Foundry | Theme::Choir | Theme::Archive | Theme::Clock => grade(&brass_wall, theme_grade(t)),
                Theme::IceCaves | Theme::Rime | Theme::Glacier => grade(&ice_wall, theme_grade(t)),
                _ => grade(&wall, theme_grade(t)),
            });
        }
        let mut grass = named("grass");
        if grass.is_empty() {
            grass = (0..2).map(|v| sprites::fallback_ground(v, 0x2e4a22, 0x46682e)).collect();
        }
        let dirt = named("dirt").into_iter().next().unwrap_or_else(|| sprites::fallback_ground(0, 0x5a4428, 0x705838));
        let road = named("road").into_iter().next().unwrap_or_else(|| sprites::fallback_ground(1, 0x585450, 0x7a746c));
        // Plain snow three times as often as the tufted variant.
        let snow: Vec<Sprite> = ["snow1", "snow1", "snow1", "snow2"].iter().filter_map(|n| exact(n)).collect();
        let snow = if snow.is_empty() { (0..2).map(|v| sprites::fallback_ground(v, 0xc8d4e0, 0xf4f8ff)).collect() } else { snow };
        let lake = exact("lake_ice").unwrap_or_else(|| sprites::fallback_ground(2, 0x7aa8c8, 0xb8d8f0));
        let snow_road = exact("snow_road").unwrap_or_else(|| sprites::fallback_ground(3, 0x8a8478, 0xb8b4a8));
        let mist_earth: Vec<Sprite> = ["mist_earth1", "mist_earth1", "mist_earth2"].iter().filter_map(|n| exact(n)).collect();
        let mist_earth = if mist_earth.is_empty() { (0..2).map(|v| sprites::fallback_ground(v, 0x2a3440, 0x3a4652)).collect() } else { mist_earth };
        let moss = exact("mist_moss").unwrap_or_else(|| sprites::fallback_ground(4, 0x2a6a30, 0x50e060));
        let mist_road = exact("mist_road").unwrap_or_else(|| sprites::fallback_ground(5, 0x3a3630, 0x524a40));
        let brass: Vec<Sprite> = ["brass_plate1", "brass_plate1", "brass_plate2"].iter().filter_map(|n| exact(n)).collect();
        let brass = if brass.is_empty() { (0..2).map(|v| sprites::fallback_ground(v, 0x4a3a20, 0x6a5430)).collect() } else { brass };
        let verdigris = exact("verdigris_floor").unwrap_or_else(|| sprites::fallback_ground(6, 0x2a5a4a, 0x407a64));
        let conveyor = exact("conveyor_road").unwrap_or_else(|| sprites::fallback_ground(7, 0x2a2a2a, 0x4a4440));
        let sea_sand: Vec<Sprite> = ["sea_sand1", "sea_sand1", "sea_sand2"].iter().filter_map(|n| exact(n)).collect();
        let sea_sand = if sea_sand.is_empty() { (0..2).map(|v| sprites::fallback_ground(v, 0x1e3440, 0x2c4a58)).collect() } else { sea_sand };
        // The flats: the same sand, darker and wetter.
        let flats = grade(&sea_sand[0], (0x0a2a34, 0.45, 0.75));
        let deck = exact("wreck_deck").unwrap_or_else(|| sprites::fallback_ground(8, 0x3a2818, 0x5a4028));
        let marble: Vec<Sprite> = ["cloud_marble1", "cloud_marble1", "cloud_marble2"].iter().filter_map(|n| exact(n)).collect();
        let marble = if marble.is_empty() { (0..2).map(|v| sprites::fallback_ground(v, 0xc8c0b0, 0xe8e0d0)).collect() } else { marble };
        let sky_grass = exact("sky_grass").unwrap_or_else(|| sprites::fallback_ground(9, 0xa09058, 0xc8b878));
        let bridge = exact("chain_bridge").unwrap_or_else(|| sprites::fallback_ground(10, 0x5a4028, 0x7a5a38));
        let items: Vec<(&'static str, Sprite)> = ITEMS
            .iter()
            .map(|d| {
                let mut s = decode(d.data);
                s.ax = s.w / 2;
                s.ay = s.h - 1;
                (d.name, s)
            })
            .collect();
        let fallback_props = sprites::PROP_NAMES.iter().filter(|n| !items.iter().any(|i| i.0 == **n)).map(|n| (*n, sprites::fallback_prop(n))).collect();
        let missing = sprites::fallback_prop("rock1");
        Art { chars, floors, walls, grass, dirt, road, snow, lake, snow_road, mist_earth, moss, mist_road, brass, verdigris, conveyor, sea_sand, flats, deck, marble, sky_grass, bridge, items, fallback_props, missing }
    }

    pub fn floor(&self, theme: Theme, ground: u8, var: usize) -> &Sprite {
        if theme == Theme::Overworld {
            return match ground {
                1 => &self.dirt,
                2 => &self.road,
                _ => &self.grass[var % self.grass.len()],
            };
        }
        if theme == Theme::Tundra {
            return match ground {
                1 => &self.lake,
                2 => &self.snow_road,
                _ => &self.snow[var % self.snow.len()],
            };
        }
        if theme == Theme::Heavens {
            return match ground {
                1 => &self.sky_grass,
                2 => &self.bridge,
                _ => &self.marble[var % self.marble.len()],
            };
        }
        if theme == Theme::Deep {
            return match ground {
                1 => &self.flats,
                2 => &self.deck,
                _ => &self.sea_sand[var % self.sea_sand.len()],
            };
        }
        if theme == Theme::Mistwood {
            return match ground {
                1 => &self.moss,
                2 => &self.mist_road,
                _ => &self.mist_earth[var % self.mist_earth.len()],
            };
        }
        if theme == Theme::Mechanus {
            return match ground {
                1 => &self.verdigris,
                2 => &self.conveyor,
                _ => &self.brass[var % self.brass.len()],
            };
        }
        let set = &self.floors[theme.index()];
        &set[var % set.len()]
    }

    pub fn wall(&self, theme: Theme) -> &Sprite {
        &self.walls[theme.index()]
    }

    pub fn item(&self, name: &str) -> Option<&Sprite> {
        self.items.iter().find(|i| i.0 == name).map(|i| &i.1)
    }

    /// A prop sprite, generated or code-drawn.
    pub fn prop(&self, name: &str) -> &Sprite {
        self.item(name).or_else(|| self.fallback_props.iter().find(|p| p.0 == name).map(|p| &p.1)).unwrap_or(&self.missing)
    }

    pub fn has_char(&self, name: &str) -> bool {
        self.chars.iter().any(|c| c.0 == name)
    }

    pub fn char(&self, name: &str) -> &CharArt {
        &self.chars.iter().find(|c| c.0 == name).expect("character art").1
    }

    /// A character's sheet, or a stand-in (another sheet, scale, tint colour, tint amount)
    /// while its own art hasn't been generated yet.
    pub fn char_art(&self, name: &str) -> (&CharArt, f32, u32, f32) {
        if self.has_char(name) {
            return (self.char(name), 1.0, 0, 0.0);
        }
        let (base, scale, tint, a) = match name {
            "wolf" => ("zombie", 0.8, 0x606060, 0.5),
            "goblin" => ("zombie", 0.7, 0x40a030, 0.5),
            "archer" => ("skeleton", 1.0, 0x303050, 0.3),
            "npc_elder" => ("mage", 0.95, 0x406030, 0.5),
            "npc_merchant" => ("mage", 1.0, 0x805030, 0.5),
            "npc_healer" => ("mage", 1.0, 0xe0e0f0, 0.55),
            "npc_guard" => ("mage", 1.05, 0x8090a0, 0.55),
            "npc_villager" => ("mage", 0.95, 0x6a5030, 0.55),
            "boss_bone" => ("skeleton", 1.6, 0xe0d8c0, 0.3),
            "boss_plague" => ("zombie", 1.7, 0x60a020, 0.4),
            "boss_hex" => ("skeleton", 1.4, 0x8040c0, 0.45),
            "boss_ashking" => ("mage", 1.8, 0x400808, 0.55),
            // ---- Act 2 stand-ins ----
            "frost_wolf" => ("wolf", 1.0, 0xe0f0ff, 0.55),
            "raider" => ("skeleton", 1.05, 0x8a6040, 0.6),
            "yeti" => ("zombie", 1.35, 0xf0f4ff, 0.6),
            "ice_troll" => ("goblin", 1.45, 0x70a8d8, 0.55),
            "ice_wraith" => ("boss_hex", 0.6, 0xa0d8ff, 0.55),
            "npc_captain" => ("npc_guard", 1.0, 0xc8b070, 0.3),
            "npc_trader" => ("npc_merchant", 1.05, 0x6a4a30, 0.4),
            // ---- jewelers (stand-ins until their art) ----
            // ---- the Inquisitor (until her PixelLab art exists): a dark hooded figure ----
            "inquisitor_hero" => ("mage", 1.0, 0x1a1414, 0.55),
            "clock_crow" => ("vbat", 0.95, 0xb08840, 0.55),
            "npc_jeweler0" => ("npc_merchant", 0.95, 0x7040a0, 0.45),
            "npc_jeweler1" => ("npc_trader", 1.0, 0x4080c0, 0.45),
            "npc_jeweler2" => ("npc_widow", 1.0, 0x30a060, 0.45),
            "npc_jeweler3" => ("npc_vesper", 1.0, 0xc0a040, 0.45),
            "npc_jeweler4" => ("npc_merchant", 1.0, 0x60c0c0, 0.45),
            "npc_jeweler5" => ("npc_merchant", 1.0, 0xe0c060, 0.45),
            "npc_seer" => ("npc_elder", 1.0, 0x9090a0, 0.4),
            "npc_fisher" => ("npc_villager", 1.0, 0x506878, 0.4),
            "boss_giant" => ("boss_bone", 1.25, 0x6090c0, 0.5),
            "boss_yeti" => ("boss_plague", 1.0, 0xf0f4ff, 0.6),
            "boss_witch" => ("boss_hex", 1.0, 0x80d0ff, 0.5),
            "boss_dragon" => ("boss_ashking", 1.25, 0xe0f0ff, 0.6),
            // ---- Act 3 stand-ins ----
            "ghoul" => ("zombie", 0.95, 0x506850, 0.5),
            "werewolf" => ("wolf", 1.35, 0x303030, 0.5),
            "banshee" => ("boss_hex", 0.6, 0xc0ffd0, 0.6),
            "wisp" => ("boss_hex", 0.35, 0x40ff80, 0.8),
            "cultist" => ("archer", 1.0, 0x501018, 0.6),
            "vbat" => ("wisp", 0.9, 0x180810, 0.75),
            "npc_hunter" => ("npc_guard", 1.0, 0x4a3828, 0.5),
            "npc_widow" => ("npc_elder", 1.0, 0x101010, 0.6),
            "npc_priest" => ("npc_healer", 1.0, 0x101018, 0.6),
            "npc_peasant" => ("npc_villager", 1.0, 0x405060, 0.4),
            "boss_ossric" => ("boss_bone", 1.0, 0x80a070, 0.35),
            "boss_grimhilde" => ("boss_hex", 1.0, 0x40e080, 0.45),
            "boss_malgrave" => ("boss_bone", 1.05, 0x101820, 0.55),
            "boss_vardak" => ("boss_ashking", 0.9, 0x500818, 0.55),
            "boss_vardak_bat" => ("boss_dragon", 0.8, 0x100808, 0.7),
            // ---- the Valkyrie (until her PixelLab art exists) ----
            "valkyrie" => ("mage", 1.05, 0x3a5070, 0.8),
            "valkyrie_horse" => ("frost_wolf", 1.7, 0x101418, 0.75),
            "einherjar" => ("skeleton", 1.05, 0x90d0ff, 0.65),
            // ---- the Berserker (until her PixelLab art exists): the raider has an axe ----
            "berserker" => ("raider", 1.08, 0x3a2418, 0.35),
            "dire_wolf" => ("wolf", 1.4, 0x161414, 0.65),
            // ---- the Reaper (until her PixelLab art exists): the hooded cultist with a tome ----
            "reaper" => ("cultist", 1.05, 0x343a44, 0.72),
            "scholar_spirit" => ("npc_priest", 1.0, 0x9ad8ff, 0.6),
            // ---- the Druid (until her PixelLab art exists) ----
            "druid" => ("npc_widow", 1.0, 0x3a4a28, 0.5),
            "plague_rat" => ("wolf", 0.45, 0x241e1a, 0.7),
            "moss_wolf" => ("wolf", 1.25, 0x3a5a30, 0.55),
            "thorn_warden" => ("boss_giant", 1.05, 0x2a3a20, 0.6),
            // ---- Act 4 stand-ins ----
            "brass_scarab" => ("boss_plague", 0.45, 0xc89040, 0.65),
            "inquisitor" => ("archer", 1.0, 0x302820, 0.6),
            "gearwraith" => ("boss_hex", 0.6, 0x80b0ff, 0.6),
            "spring_jack" => ("skeleton", 1.15, 0xb08030, 0.55),
            "boiler_brute" => ("yeti", 1.0, 0x402818, 0.6),
            "ordinal" => ("boss_hex", 0.35, 0xd0a040, 0.7),
            "ordinal_prism" => ("boss_hex", 0.45, 0x80c0ff, 0.65),
            "ordinal_marshal" => ("boss_hex", 0.65, 0xffd060, 0.6),
            "npc_tally" => ("npc_guard", 1.0, 0xc0a060, 0.55),
            "npc_vesper" => ("npc_merchant", 1.0, 0x6a4020, 0.4),
            "npc_oiler" => ("npc_healer", 1.0, 0x606060, 0.5),
            "npc_servant" => ("npc_villager", 0.9, 0xb08840, 0.55),
            "boss_forgemother" => ("boss_plague", 1.0, 0xff8020, 0.55),
            "boss_cantor" => ("boss_giant", 1.0, 0x8a6a30, 0.6),
            "boss_archivist" => ("boss_hex", 1.0, 0x403020, 0.55),
            "boss_clockmaker" => ("boss_ashking", 0.9, 0x302010, 0.5),
            "boss_clockmaker_engine" => ("boss_giant", 1.3, 0xc09040, 0.6),
            // ---- Act 5 stand-ins (until the PixelLab art is approved) ----
            "drowned_sailor" => ("zombie", 1.05, 0x406060, 0.55),
            "merrow" => ("goblin", 1.3, 0x1a6a70, 0.6),
            "anglerlurk" => ("wolf", 1.2, 0x183040, 0.7),
            "jelly_drift" => ("wisp", 1.2, 0xa060e0, 0.6),
            "shellguard" => ("boss_plague", 0.6, 0x804030, 0.55),
            "siren" => ("banshee", 1.0, 0x40c0b0, 0.55),
            "ink_horror" => ("ghoul", 1.1, 0x201030, 0.7),
            "npc_ysolde" => ("npc_guard", 1.0, 0x3a4a5a, 0.45),
            "npc_nessa" => ("npc_merchant", 1.0, 0x2a5a6a, 0.45),
            "npc_coral" => ("npc_healer", 1.0, 0x40a080, 0.5),
            "npc_diver" => ("npc_villager", 1.0, 0x9a7a40, 0.5),
            "boss_dregmoor" => ("boss_bone", 1.15, 0x2a4a50, 0.55),
            "boss_nacre" => ("boss_witch", 1.1, 0xd0e8f0, 0.5),
            // ---- side content ----
            "boss_wellwitch" => ("boss_witch", 1.0, 0x70b050, 0.55),
            "hoarder" => ("goblin", 0.85, 0xffd040, 0.45),
            "boss_angler" => ("yeti", 1.4, 0x102838, 0.7),
            "boss_leviathan" => ("boss_dragon", 1.3, 0x105060, 0.6),
            // ---- Act 6 stand-ins (until the PixelLab art is approved) ----
            "fallen_seraph" => ("raider", 1.05, 0xe8e0c0, 0.5),
            "ophanim" => ("boss_hex", 0.5, 0xffd060, 0.65),
            "storm_drake" => ("frost_wolf", 1.3, 0x5030a0, 0.6),
            "ash_harpy" => ("banshee", 0.95, 0x808080, 0.6),
            "gilded_sentinel" => ("boss_giant", 0.85, 0xe0d8c0, 0.6),
            "sun_zealot" => ("cultist", 1.0, 0xf0d890, 0.55),
            "thunderbird" => ("boss_dragon", 0.5, 0x304080, 0.6),
            "npc_seraphine" => ("npc_guard", 1.0, 0xe8e0d0, 0.45),
            "npc_bram" => ("npc_merchant", 1.0, 0x30508a, 0.45),
            "npc_aurel" => ("npc_healer", 1.0, 0xf0e8c0, 0.5),
            "npc_deckhand" => ("npc_villager", 1.0, 0x8a6a40, 0.45),
            "boss_vael" => ("boss_ashking", 1.0, 0x302818, 0.5),
            "boss_tempest" => ("boss_dragon", 1.2, 0x402080, 0.6),
            "boss_ophan" => ("boss_hex", 1.4, 0xffd060, 0.6),
            "boss_solanthos" => ("boss_ashking", 1.25, 0x201008, 0.55),
            "vampire" => ("mage", 1.0, 0x501060, 0.5),
            "inventor" => ("mage", 1.0, 0x704820, 0.5),
            "steam_suit" => ("boss_giant", 0.7, 0xb08840, 0.5),
            _ => ("mage", 1.0, 0, 0.0),
        };
        let base = if self.has_char(base) { base } else { "mage" };
        (self.char(base), scale, rgb(tint), a)
    }
}

/// Topmost opaque row in the first cell of a sheet.
fn top_of(img: &Sprite, cw: i32, ch: i32) -> i32 {
    for y in 0..ch {
        for x in 0..cw {
            if img.get(x, y) != 0 {
                return y;
            }
        }
    }
    0
}

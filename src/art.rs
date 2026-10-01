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
    /// Item and prop sprites (anchored at the bottom centre, where they sit on the floor).
    items: Vec<(&'static str, Sprite)>,
    /// Code-drawn props for any prop sprite that hasn't been generated.
    fallback_props: Vec<(&'static str, Sprite)>,
    missing: Sprite,
}

/// Dungeon theme colour grading: (tint colour, amount, brightness).
fn theme_grade(t: Theme) -> (u32, f32, f32) {
    match t {
        Theme::Overworld | Theme::Crypt => (0, 0.0, 1.0),
        Theme::Warrens => (0x6a4a24, 0.38, 0.95),
        Theme::Catacombs => (0x4a3a78, 0.38, 0.95),
        Theme::Sanctum => (0x6a1408, 0.42, 0.8),
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
        let mut stone = named("floor");
        if stone.is_empty() {
            stone = (0..3).map(sprites::fallback_floor).collect();
        }
        let wall = named("wall").into_iter().next().unwrap_or_else(sprites::fallback_wall);
        let palisade = named("palisade").into_iter().next().unwrap_or_else(|| grade(&sprites::fallback_wall(), (0x6a4a24, 0.6, 0.9)));
        let mut floors = vec![];
        let mut walls = vec![];
        for t in Theme::ALL {
            floors.push(stone.iter().map(|s| grade(s, theme_grade(t))).collect());
            walls.push(if t == Theme::Overworld { palisade.clone() } else { grade(&wall, theme_grade(t)) });
        }
        let mut grass = named("grass");
        if grass.is_empty() {
            grass = (0..2).map(|v| sprites::fallback_ground(v, 0x2e4a22, 0x46682e)).collect();
        }
        let dirt = named("dirt").into_iter().next().unwrap_or_else(|| sprites::fallback_ground(0, 0x5a4428, 0x705838));
        let road = named("road").into_iter().next().unwrap_or_else(|| sprites::fallback_ground(1, 0x585450, 0x7a746c));
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
        Art { chars, floors, walls, grass, dirt, road, items, fallback_props, missing }
    }

    pub fn floor(&self, theme: Theme, ground: u8, var: usize) -> &Sprite {
        if theme == Theme::Overworld {
            return match ground {
                1 => &self.dirt,
                2 => &self.road,
                _ => &self.grass[var % self.grass.len()],
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
            // The Plague Warden is meant to be enormous.
            let scale = if name == "boss_plague" { 1.3 } else { 1.0 };
            return (self.char(name), scale, 0, 0.0);
        }
        let (base, scale, tint, a) = match name {
            "wolf" => ("zombie", 0.8, 0x606060, 0.5),
            "imp" => ("zombie", 0.7, 0xc03020, 0.5),
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
            _ => ("mage", 1.0, 0, 0.0),
        };
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

//! Generated art (PixelLab), embedded at build time by `scripts/import_art.py`
//! into `art_gen.rs`. Anything missing falls back to the code-drawn sprites in
//! `sprites.rs`, so the game always runs.
//!
//! Character sheets: one row per (animation, direction); rows for an animation
//! start at `row` and run through the 8 directions in PixelLab order
//! (south, south-east, east, north-east, north, north-west, west, south-west).
use crate::art_gen::{CHARS, ITEMS, TILES};
use crate::gfx::Sprite;
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
    pub floors: Vec<Sprite>,
    pub wall: Sprite,
    /// Item sprites (anchored at the bottom centre, where they sit on the floor).
    items: Vec<(&'static str, Sprite)>,
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
        let mut floors: Vec<Sprite> = TILES.iter().filter(|t| t.name.starts_with("floor")).map(tile).collect();
        if floors.is_empty() {
            floors = (0..3).map(sprites::fallback_floor).collect();
        }
        let wall = match TILES.iter().find(|t| t.name.starts_with("wall")) {
            Some(t) => tile(t),
            None => sprites::fallback_wall(),
        };
        let items = ITEMS
            .iter()
            .map(|d| {
                let mut s = decode(d.data);
                s.ax = s.w / 2;
                s.ay = s.h - 1;
                (d.name, s)
            })
            .collect();
        Art { chars, floors, wall, items }
    }

    pub fn item(&self, name: &str) -> Option<&Sprite> {
        self.items.iter().find(|i| i.0 == name).map(|i| &i.1)
    }

    pub fn char(&self, name: &str) -> &CharArt {
        &self.chars.iter().find(|c| c.0 == name).expect("character art").1
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

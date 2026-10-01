//! Code-drawn placeholder sprites, used when no generated art is embedded.
use crate::art::{Anim, CharArt};
use crate::gfx::{mix, rgb, Sprite, BLACK};

fn hash(x: i32, y: i32, s: i32) -> u32 {
    let mut h = (x as u32).wrapping_mul(374_761_393) ^ (y as u32).wrapping_mul(668_265_263) ^ (s as u32).wrapping_mul(2_246_822_519);
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    h ^ (h >> 16)
}

/// 32x16 diamond floor tile with a stone texture. Anchor = diamond centre.
pub fn fallback_floor(v: i32) -> Sprite {
    let mut s = Sprite::new(32, 17);
    let base = [rgb(0x3a3834), rgb(0x34322f), rgb(0x3d3a35)][v as usize % 3];
    for y in 0..17 {
        for x in 0..32 {
            let dx = (x as f32 + 0.5 - 16.0).abs() / 16.0;
            let dy = (y as f32 + 0.5 - 8.5).abs() / 8.5;
            if dx + dy > 1.0 {
                continue;
            }
            let n = (hash(x / 3, y / 2, v) % 20) as f32 / 100.0;
            let mut c = mix(base, rgb(0x5a554c), n);
            if dx + dy > 0.9 {
                c = mix(c, BLACK, 0.45);
            }
            s.set(x, y, c);
        }
    }
    s.ax = 16;
    s.ay = 8;
    s
}

/// Wall block: 32 wide, 16px top diamond and a 48px face. Anchor = footprint centre.
pub fn fallback_wall() -> Sprite {
    let face = 40;
    let mut s = Sprite::new(32, 16 + face);
    for y in 0..s.h {
        for x in 0..32 {
            let fx = x as f32 + 0.5 - 16.0;
            // Top diamond centred at y=8.
            let top = (fx.abs() / 16.0) + ((y as f32 + 0.5 - 8.0).abs() / 8.0) <= 1.0;
            // Faces: between the top diamond's lower edges and the same edges shifted down by `face`.
            let edge = 8.0 + 8.0 * (1.0 - fx.abs() / 16.0);
            let side = (y as f32) >= edge - 0.5 && (y as f32) < edge + face as f32;
            let n = (hash(x / 4, (y + (x / 8) * 3) / 3, 7) % 16) as f32 / 100.0;
            let c = if top {
                mix(rgb(0x6a645a), rgb(0x857d70), n)
            } else if side {
                let mortar = (y - (edge as i32)) % 8 == 7 || ((x + ((y - edge as i32) / 8) * 4) % 8 == 0);
                let b = if fx < 0.0 { rgb(0x4a453e) } else { rgb(0x35312c) };
                if mortar {
                    mix(b, BLACK, 0.4)
                } else {
                    mix(b, rgb(0x6a645a), n)
                }
            } else {
                continue;
            };
            s.set(x, y, c);
        }
    }
    s.ax = 16;
    s.ay = 8 + face;
    s
}

fn figure(body: u32, trim: u32, head: u32, staff: bool, dir: usize, phase: f32) -> Sprite {
    let mut s = Sprite::new(40, 56);
    let bob = (phase * std::f32::consts::TAU).sin();
    // Legs / robe hem
    let step = bob * 2.5;
    s.ellipse(17.0 - step * 0.3, 50.0, 3.0, 4.0, mix(body, BLACK, 0.4));
    s.ellipse(23.0 + step * 0.3, 50.0, 3.0, 4.0, mix(body, BLACK, 0.4));
    s.ellipse(20.0, 38.0, 8.0, 12.0, body);
    s.ellipse(20.0, 36.0, 4.0, 10.0, mix(body, trim, 0.25));
    s.fill(12, 34, 16, 2, trim);
    s.ellipse(20.0, 20.0, 5.5, 6.0, head);
    // Facing hint: eyes/face on the side we're looking toward.
    let (fx, show_face) = match dir {
        0 => (0.0, true),
        1 => (2.0, true),
        2 => (4.0, true),
        7 => (-2.0, true),
        6 => (-4.0, true),
        _ => (0.0, false),
    };
    if show_face {
        s.fill((18.0 + fx) as i32, 20, 1, 1, rgb(0xffa030));
        s.fill((21.0 + fx) as i32, 20, 1, 1, rgb(0xffa030));
    }
    if staff {
        s.fill(30, 12, 2, 40, rgb(0x5a3a1c));
        s.ellipse(31.0, 11.0, 2.5, 2.5, rgb(0xffb040));
    }
    s.outline(BLACK);
    s.ax = 20;
    s.ay = 53;
    s
}

pub fn fallback_char(name: &str) -> CharArt {
    let (body, trim, head, staff) = match name {
        "mage" => (rgb(0x7a1a2a), rgb(0xc8a040), rgb(0xd8b090), true),
        "zombie" => (rgb(0x4a3a30), rgb(0x3a4a30), rgb(0x8a9a78), false),
        _ => (rgb(0x2a2c3a), rgb(0xd8c8a0), rgb(0xe0d4b0), false),
    };
    let mk = |frames: usize| -> Vec<Vec<Sprite>> {
        (0..8).map(|d| (0..frames).map(|f| figure(body, trim, head, staff, d, f as f32 / frames as f32)).collect()).collect()
    };
    CharArt {
        anims: vec![
            Anim { name: "idle".into(), frames: mk(1), fps: 1 },
            Anim { name: "walk".into(), frames: mk(4), fps: 8 },
        ],
        height: 44,
    }
}

/// Fireball skill icon for the HUD (24x24).
pub fn fireball_icon() -> Sprite {
    let mut s = Sprite::new(24, 24);
    s.fill(0, 0, 24, 24, rgb(0x1a1410));
    for y in 0..24 {
        for x in 0..24 {
            let (dx, dy) = (x as f32 - 13.0, y as f32 - 11.0);
            let d = (dx * dx + dy * dy).sqrt();
            let tail = x < 13 && (dy - (13.0 - x as f32) * 0.35).abs() < (x as f32) * 0.25;
            if d < 6.0 {
                s.set(x, y, mix(rgb(0xfff0a0), rgb(0xff6010), d / 6.0));
            } else if tail {
                s.set(x, y, mix(rgb(0xff8020), rgb(0x801808), (13 - x) as f32 / 13.0));
            }
        }
    }
    s.ax = 0;
    s.ay = 0;
    s
}

/// Flat ground diamond (grass, dirt, road) for the overworld.
pub fn fallback_ground(v: i32, base: u32, hi: u32) -> Sprite {
    let mut s = Sprite::new(32, 17);
    for y in 0..17 {
        for x in 0..32 {
            let dx = (x as f32 + 0.5 - 16.0).abs() / 16.0;
            let dy = (y as f32 + 0.5 - 8.5).abs() / 8.5;
            if dx + dy > 1.0 {
                continue;
            }
            let n = (hash(x / 2, y, v + 11) % 30) as f32 / 100.0;
            s.set(x, y, mix(rgb(base), rgb(hi), n));
        }
    }
    s.ax = 16;
    s.ay = 8;
    s
}

/// Props that have code-drawn stand-ins.
pub const PROP_NAMES: [&str; 16] = [
    "tree_oak", "tree_pine", "tree_dead", "rock1", "bush1", "house1", "house2", "tent1", "campfire", "well", "ent_crypt",
    "ent_warrens", "ent_catacombs", "ent_sanctum", "stairs_down", "stairs_up",
];

/// Simple stand-in shapes for props (anchored at the bottom centre).
pub fn fallback_prop(name: &str) -> Sprite {
    let mut s;
    match name {
        "tree_oak" | "tree_pine" | "tree_dead" | "bush1" => {
            let (w, h) = if name == "bush1" { (24, 20) } else { (40, 64) };
            s = Sprite::new(w, h);
            if name != "bush1" {
                s.fill(w / 2 - 3, h - 22, 6, 22, rgb(0x4a3020));
            }
            let leaf = match name {
                "tree_pine" => rgb(0x1e4028),
                "tree_dead" => rgb(0x5a5048),
                _ => rgb(0x2e5a24),
            };
            if name == "tree_pine" {
                for k in 0..4 {
                    s.ellipse(w as f32 / 2.0, (8 + k * 11) as f32, 6.0 + k as f32 * 4.0, 7.0, mix(leaf, rgb(0x3a7038), k as f32 * 0.1));
                }
            } else if name != "tree_dead" {
                s.ellipse(w as f32 / 2.0, h as f32 * 0.4, w as f32 / 2.0 - 1.0, h as f32 * 0.38, leaf);
                s.ellipse(w as f32 / 2.0 - 4.0, h as f32 * 0.32, w as f32 / 4.0, h as f32 * 0.18, mix(leaf, rgb(0x5a8a40), 0.4));
            }
        }
        "rock1" => {
            s = Sprite::new(28, 20);
            s.ellipse(14.0, 12.0, 12.0, 8.0, rgb(0x5a5a58));
            s.ellipse(11.0, 9.0, 6.0, 4.0, rgb(0x7a7a74));
        }
        "house1" | "house2" | "tent1" => {
            s = Sprite::new(96, 84);
            let wall = if name == "house2" { rgb(0x6a6458) } else { rgb(0x8a6a40) };
            s.fill(16, 40, 64, 40, wall);
            let roof = if name == "tent1" { rgb(0xa03030) } else { rgb(0x6a5030) };
            for y in 0..36 {
                let half = 10 + y * 38 / 36;
                s.fill(48 - half, 8 + y, half * 2, 1, roof);
            }
            s.fill(42, 58, 12, 22, rgb(0x3a2818));
        }
        "campfire" => {
            s = Sprite::new(24, 14);
            s.ellipse(12.0, 9.0, 10.0, 4.0, rgb(0x505050));
            s.ellipse(12.0, 8.0, 6.0, 3.0, rgb(0x4a3020));
        }
        "well" => {
            s = Sprite::new(32, 40);
            s.ellipse(16.0, 32.0, 13.0, 7.0, rgb(0x6a6a64));
            s.ellipse(16.0, 31.0, 8.0, 4.0, rgb(0x101820));
            s.fill(4, 8, 3, 24, rgb(0x5a3a20));
            s.fill(25, 8, 3, 24, rgb(0x5a3a20));
            s.fill(2, 6, 28, 4, rgb(0x6a4a28));
        }
        "stairs_down" => {
            s = Sprite::new(40, 21);
            for y in 0..21 {
                for x in 0..40 {
                    let d = ((x as f32 - 20.0) / 20.0).abs() + ((y as f32 - 10.0) / 10.0).abs();
                    if d <= 1.0 {
                        s.set(x, y, mix(rgb(0x050404), rgb(0x3a3630), (d * 1.2).min(1.0)));
                    }
                }
            }
        }
        _ => {
            // Dungeon entrances and stairs up: a stone arch with a dark doorway.
            s = Sprite::new(72, 72);
            let stone = match name {
                "ent_warrens" => rgb(0x5a4a30),
                "ent_catacombs" => rgb(0x4a3a68),
                "ent_sanctum" => rgb(0x3a2020),
                _ => rgb(0x5a5850),
            };
            s.fill(8, 12, 56, 58, stone);
            s.ellipse(36.0, 40.0, 16.0, 18.0, rgb(0x080606));
            s.fill(20, 40, 32, 30, rgb(0x080606));
        }
    }
    s.outline(BLACK);
    s.ax = s.w / 2;
    s.ay = s.h - 1;
    s
}

/// 24x24 skill icons (drawn at their top-left corner).
pub fn skill_icon(s: crate::skills::Skill) -> Sprite {
    use crate::skills::Skill;
    if s == Skill::Fireball {
        return fireball_icon();
    }
    let mut im = Sprite::new(24, 24);
    im.fill(0, 0, 24, 24, rgb(0x1a1410));
    let fire = |d: f32| mix(rgb(0xfff0a0), rgb(0xc02808), d.clamp(0.0, 1.0));
    match s {
        Skill::Inferno => {
            // Staff tip at the left, a cone of flame to the right.
            for y in 0..24 {
                for x in 3..23 {
                    let t = (x - 3) as f32 / 19.0;
                    let half = 1.0 + t * 9.0;
                    let dy = (y as f32 - 12.0).abs();
                    if dy < half && (hash(x, y, 3) % 7 != 0 || t < 0.3) {
                        im.set(x, y, fire(t * 0.8 + dy / half * 0.4));
                    }
                }
            }
            im.fill(0, 11, 4, 2, rgb(0x6a4020));
        }
        Skill::FireNova => {
            for y in 0..24 {
                for x in 0..24 {
                    let d = ((x as f32 - 11.5).powi(2) + (y as f32 - 11.5).powi(2)).sqrt();
                    if (7.0..10.5).contains(&d) {
                        im.set(x, y, fire((d - 7.0) / 3.5));
                    } else if d < 2.5 {
                        im.set(x, y, rgb(0xfff4c0));
                    }
                }
            }
        }
        Skill::Warmth => {
            // A glowing ember heart.
            for y in 0..24 {
                for x in 0..24 {
                    let (fx, fy) = ((x as f32 - 11.5) / 8.0, (y as f32 - 10.0) / 8.0);
                    let heart = (fx * fx + fy * fy - 1.0).powi(3) - fx * fx * fy.powi(3) * -1.0;
                    if heart < 0.0 {
                        let d = (fx * fx + fy * fy).sqrt();
                        im.set(x, y, fire(d));
                    }
                }
            }
        }
        Skill::FireWall => {
            // A row of flame tongues on a dark ground line.
            for x in 2..22 {
                let h = 8 + (hash(x / 3, 0, 9) % 9) as i32;
                for y in 0..h {
                    let t = y as f32 / h as f32;
                    if (x + y) % 5 != 0 || t < 0.5 {
                        im.set(x, 20 - y, fire(t));
                    }
                }
            }
            im.fill(1, 20, 22, 2, rgb(0x3a2010));
        }
        Skill::Blaze => {
            // Footprints of fire trailing behind.
            for (k, (cx, cy)) in [(5, 18), (10, 13), (15, 9), (20, 5)].iter().enumerate() {
                let r = 2.0 + k as f32 * 0.6;
                for y in 0..24 {
                    for x in 0..24 {
                        let d = (((x - cx) as f32).powi(2) + ((y - cy) as f32).powi(2)).sqrt();
                        if d < r {
                            im.set(x, y, fire(d / r * 0.6 + (3 - k) as f32 * 0.12));
                        }
                    }
                }
            }
        }
        Skill::Combust => {
            // A burst: star of flame with a hot core.
            for y in 0..24 {
                for x in 0..24 {
                    let (dx, dy) = (x as f32 - 11.5, y as f32 - 11.5);
                    let d = (dx * dx + dy * dy).sqrt();
                    let a = dy.atan2(dx);
                    let spike = 6.0 + 4.5 * (a * 4.0).cos().abs();
                    if d < spike {
                        im.set(x, y, fire(d / spike));
                    }
                }
            }
        }
        Skill::Fireball => {}
    }
    im.ax = 0;
    im.ay = 0;
    im
}

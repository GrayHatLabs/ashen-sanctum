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

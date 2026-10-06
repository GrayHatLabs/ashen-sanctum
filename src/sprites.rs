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
pub const PROP_NAMES: [&str; 55] = [
    "tree_oak", "tree_pine", "tree_dead", "rock1", "bush1", "house1", "house2", "tent1", "campfire", "well", "ent_crypt",
    "ent_warrens", "ent_catacombs", "ent_sanctum", "stairs_down", "stairs_up", "tree_snowpine", "tree_snowdead", "rock_snow",
    "ice_crystal", "longhouse1", "longhouse2", "stall_furs", "ent_mines", "ent_caves", "ent_temple", "ent_glacier", "pass_gate",
    "tree_twisted", "tree_mistpine", "glow_shrooms", "gravestone", "cottage_mist", "cottage_mist2", "gallows", "merchant_cart",
    "ent_chapel", "ent_gallows", "ent_barrow", "ent_castle", "pass_mist",
    "gear_tower", "steam_pipes", "steam_vent", "gas_lamp", "cog_pile", "workshop1", "workshop2", "clock_tower", "pendulum",
    "ent_foundry", "ent_choir", "ent_archive", "ent_clock", "gear_gate",
];

/// Simple stand-in shapes for props (anchored at the bottom centre).
pub fn fallback_prop(name: &str) -> Sprite {
    let mut s;
    // Act 2 props reuse the Act 1 shapes, frosted.
    let frost = |base: &str, tint: u32, a: f32| {
        let mut f = fallback_prop(base);
        for p in f.px.iter_mut() {
            if *p != 0 && *p != BLACK {
                *p = mix(*p, rgb(tint), a);
            }
        }
        f
    };
    match name {
        "tree_snowpine" => return frost("tree_pine", 0xf0f8ff, 0.45),
        "tree_twisted" => return frost("tree_dead", 0x304838, 0.5),
        "tree_mistpine" => return frost("tree_pine", 0x203038, 0.5),
        "glow_shrooms" => return frost("bush1", 0x40e060, 0.6),
        "gravestone" => return frost("rock1", 0x707880, 0.3),
        "cottage_mist" | "cottage_mist2" => return frost("house2", 0x303840, 0.35),
        "gallows" | "merchant_cart" => return frost("tent1", 0x40302a, 0.6),
        "ent_chapel" | "ent_gallows" | "ent_barrow" | "pass_mist" => return frost("ent_crypt", 0x305040, 0.45),
        "ent_castle" => return frost("ent_crypt", 0x401018, 0.5),
        "gear_tower" | "clock_tower" | "pendulum" => return frost("tree_pine", 0xb08830, 0.75),
        "steam_pipes" | "gas_lamp" => return frost("well", 0x8a5a30, 0.6),
        "steam_vent" | "cog_pile" => return frost("rock1", 0x8a6a30, 0.6),
        "workshop1" | "workshop2" => return frost("house1", 0x3a2a20, 0.5),
        "ent_foundry" => return frost("ent_crypt", 0xc05010, 0.5),
        "ent_choir" | "ent_archive" | "gear_gate" => return frost("ent_crypt", 0xb08830, 0.5),
        "ent_clock" => return frost("ent_crypt", 0xe0c060, 0.5),
        "tree_snowdead" => return frost("tree_dead", 0xe8f0f8, 0.4),
        "rock_snow" => return frost("rock1", 0xf0f4ff, 0.4),
        "longhouse1" | "longhouse2" => return frost("house1", 0xe8f0ff, 0.25),
        "stall_furs" => return frost("tent1", 0x8a6a48, 0.5),
        "ent_mines" | "pass_gate" => return frost("ent_crypt", 0xd0e0f0, 0.3),
        "ent_caves" | "ent_temple" | "ent_glacier" => return frost("ent_crypt", 0x90c8f0, 0.5),
        "ice_crystal" => {
            s = Sprite::new(20, 30);
            for k in 0..3 {
                let (x, h) = (4 + k * 6, 14 + (k % 2) * 12);
                s.fill(x, 30 - h, 5, h, mix(rgb(0x8ac8f0), rgb(0xe0f4ff), k as f32 * 0.3));
            }
        }
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

/// The inventor's icons: brass and teal on soot black.
fn inventor_icon(s: crate::skills::Skill, im: &mut Sprite) {
    use crate::skills::Skill;
    im.fill(0, 0, 24, 24, rgb(0x14120e));
    let brass = rgb(0xc89a40);
    let dark = rgb(0x6a4a20);
    let teal = rgb(0x50e0d0);
    let gear = |im: &mut Sprite, cx: f32, cy: f32, r: f32, c: u32| {
        for y in 0..24 {
            for x in 0..24 {
                let (dx, dy) = (x as f32 - cx, y as f32 - cy);
                let d = (dx * dx + dy * dy).sqrt();
                let a = dy.atan2(dx);
                let tooth = if (a * 4.0 / std::f32::consts::PI).rem_euclid(2.0) < 1.0 { 1.5 } else { 0.0 };
                if d < r + tooth && d > r * 0.45 {
                    im.set(x, y, c);
                }
            }
        }
    };
    match s {
        Skill::RayPistol => {
            im.fill(4, 9, 12, 5, brass);
            im.fill(5, 14, 4, 7, dark);
            im.fill(16, 10, 5, 3, teal);
            im.set(22, 11, rgb(0xe0fffa));
        }
        Skill::ClockBomb => {
            im.ellipse(11.0, 14.0, 7.0, 7.0, brass);
            im.ellipse(11.0, 14.0, 4.0, 4.0, rgb(0xe8d8b0));
            im.fill(11, 11, 1, 4, BLACK);
            im.fill(15, 4, 2, 5, dark);
            im.set(17, 3, rgb(0xff8030));
        }
        Skill::Tinkerer => {
            gear(im, 9.0, 10.0, 6.0, brass);
            gear(im, 16.0, 16.0, 4.5, dark);
        }
        Skill::ArcCoil => {
            let pts = [(3, 4), (10, 9), (7, 13), (15, 15), (12, 19), (21, 21)];
            for w in pts.windows(2) {
                let (a, b) = (w[0], w[1]);
                for k in 0..=10 {
                    let t = k as f32 / 10.0;
                    im.set(a.0 + ((b.0 - a.0) as f32 * t) as i32, a.1 + ((b.1 - a.1) as f32 * t) as i32, teal);
                }
            }
        }
        Skill::Turret => {
            im.fill(7, 10, 10, 7, brass);
            im.fill(15, 12, 7, 3, dark);
            im.fill(9, 17, 2, 5, dark);
            im.fill(14, 17, 2, 5, dark);
            im.set(11, 12, teal);
        }
        Skill::Grapple => {
            for k in 0..14 {
                im.set(3 + k, 20 - k, dark);
            }
            im.fill(16, 3, 2, 7, brass);
            im.fill(13, 3, 8, 2, brass);
            im.set(13, 5, brass);
            im.set(20, 5, brass);
        }
        Skill::TeslaField => {
            im.ellipse(12.0, 12.0, 9.0, 9.0, rgb(0x103830));
            im.ellipse(12.0, 12.0, 3.0, 3.0, teal);
            for k in 0..8 {
                let a = k as f32 * std::f32::consts::FRAC_PI_4;
                im.set((12.0 + a.cos() * 8.0) as i32, (12.0 + a.sin() * 8.0) as i32, rgb(0xe0fffa));
            }
        }
        Skill::Spider => {
            im.ellipse(12.0, 13.0, 5.0, 4.0, brass);
            for k in 0..4 {
                let y = 9 + k * 3;
                im.fill(2, y, 6, 1, dark);
                im.fill(16, y, 6, 1, dark);
            }
            im.set(10, 11, teal);
            im.set(14, 11, teal);
        }
        Skill::Overclock => {
            gear(im, 12.0, 12.0, 8.0, brass);
            im.fill(12, 6, 1, 7, BLACK);
            im.fill(12, 12, 5, 1, BLACK);
        }
        Skill::AirshipStrike => {
            im.ellipse(12.0, 8.0, 10.0, 5.0, brass);
            im.fill(9, 13, 6, 3, dark);
            for k in 0..3 {
                im.fill(7 + k * 5, 19 + (k % 2) * 2, 2, 2, rgb(0x2a2018));
            }
        }
        Skill::SteamSuit => {
            im.fill(7, 6, 10, 11, brass);
            im.fill(9, 8, 6, 4, teal);
            im.fill(5, 9, 2, 8, dark);
            im.fill(17, 9, 3, 4, dark);
            im.fill(8, 17, 3, 5, dark);
            im.fill(13, 17, 3, 5, dark);
        }
        _ => {}
    }
}

/// The vampire's icons: crimson and violet on near-black.
fn vampire_icon(s: crate::skills::Skill, im: &mut Sprite) {
    use crate::skills::Skill;
    im.fill(0, 0, 24, 24, rgb(0x140a10));
    let blood = |d: f32| mix(rgb(0xff6070), rgb(0x600010), d.clamp(0.0, 1.0));
    let violet = |d: f32| mix(rgb(0xe0b0ff), rgb(0x502070), d.clamp(0.0, 1.0));
    let bat = |im: &mut Sprite, cx: i32, cy: i32, w: i32, c: u32| {
        im.fill(cx - 1, cy - 1, 3, 3, c);
        for k in 1..=w {
            let lift = (k as f32 * 0.6) as i32 - if k == w { 1 } else { 0 };
            im.set(cx - 1 - k, cy - lift + k / 2, c);
            im.set(cx + 1 + k, cy - lift + k / 2, c);
            im.set(cx - 1 - k, cy - lift + k / 2 - 1, c);
            im.set(cx + 1 + k, cy - lift + k / 2 - 1, c);
        }
        im.set(cx - 1, cy - 2, c);
        im.set(cx + 1, cy - 2, c);
    };
    match s {
        Skill::BloodLance => {
            for k in 0..18 {
                let t = k as f32 / 18.0;
                im.fill(3 + k, 20 - k, 2, 2, blood(1.0 - t));
            }
            im.ellipse(20.0, 4.0, 2.5, 2.5, rgb(0xffd0d8));
        }
        Skill::Rake => {
            for c in 0..3 {
                for k in 0..16 {
                    im.set(5 + c * 5 + k / 4, 4 + k, blood(k as f32 / 16.0));
                    im.set(6 + c * 5 + k / 4, 4 + k, blood(k as f32 / 16.0 + 0.2));
                }
            }
        }
        Skill::Thirst => {
            for y in 3..21 {
                let half = if y < 11 { (y - 3) as f32 * 0.7 } else { 6.0 - ((y - 11) as f32 * 0.6).powi(2) * 0.3 };
                for x in 0..24 {
                    if (x as f32 - 11.5).abs() < half.max(0.5) {
                        im.set(x, y, blood((y - 3) as f32 / 18.0));
                    }
                }
            }
            im.set(9, 13, rgb(0xffd0d8));
        }
        Skill::BatSwarm => {
            bat(im, 7, 8, 5, violet(0.4));
            bat(im, 16, 12, 5, violet(0.2));
            bat(im, 9, 18, 4, violet(0.6));
        }
        Skill::Mesmerize => {
            im.ellipse(12.0, 12.0, 10.0, 6.0, violet(0.8));
            im.ellipse(12.0, 12.0, 4.5, 4.5, rgb(0xd02030));
            im.ellipse(12.0, 12.0, 1.5, 3.0, rgb(0x100008));
        }
        Skill::MistStep => {
            for k in 0..4 {
                im.ellipse(6.0 + k as f32 * 4.0, 14.0 - k as f32 * 2.0, 4.0, 2.5, mix(rgb(0x8a7a9a), rgb(0xd8d0e8), k as f32 / 3.0));
            }
        }
        Skill::CrimsonNova => {
            for y in 0..24 {
                for x in 0..24 {
                    let d = ((x as f32 - 11.5).powi(2) + (y as f32 - 11.5).powi(2)).sqrt();
                    if (d - 8.5).abs() < 2.2 {
                        im.set(x, y, blood((d - 6.3) / 4.4));
                    }
                }
            }
            im.ellipse(12.0, 12.0, 2.0, 2.0, rgb(0xffd0d8));
        }
        Skill::Thrall => {
            im.ellipse(12.0, 10.0, 7.0, 7.0, rgb(0xd8d0c0));
            im.fill(8, 15, 9, 5, rgb(0xd8d0c0));
            im.ellipse(9.0, 10.0, 1.8, 2.0, violet(0.0));
            im.ellipse(15.0, 10.0, 1.8, 2.0, violet(0.0));
            for x in [9, 11, 13, 15] {
                im.fill(x, 17, 1, 3, rgb(0x140a10));
            }
        }
        Skill::NightMastery => {
            im.ellipse(12.0, 12.0, 9.0, 9.0, rgb(0xe8e0f0));
            im.ellipse(15.0, 10.0, 8.0, 8.0, rgb(0x140a10));
            im.set(5, 4, rgb(0xffffff));
            im.set(19, 19, rgb(0xd0c0ff));
        }
        Skill::BloodMoon => {
            im.ellipse(12.0, 10.0, 8.0, 8.0, blood(0.3));
            im.ellipse(9.0, 8.0, 2.0, 1.5, blood(0.0));
            im.fill(2, 20, 20, 2, blood(0.8));
        }
        Skill::Embrace => {
            bat(im, 12, 11, 10, violet(0.3));
            im.fill(11, 9, 3, 6, violet(0.5));
            im.set(11, 10, rgb(0xff2030));
            im.set(13, 10, rgb(0xff2030));
        }
        _ => {}
    }
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
        Skill::Meteor => {
            // A rock with a flaming tail, falling to the lower right.
            for t in 0..14 {
                let (x, y) = (2 + t, 2 + t);
                for w in -2..=2 {
                    if (t + w) % 3 != 0 {
                        im.set(x + w, y - w, fire(1.0 - t as f32 / 14.0));
                    }
                }
            }
            im.ellipse(17.0, 17.0, 4.5, 4.5, rgb(0x3a2a20));
            im.ellipse(16.0, 16.0, 2.0, 2.0, rgb(0xffc060));
        }
        Skill::Mastery => {
            // A flame inside a gold ring.
            for y in 0..24 {
                for x in 0..24 {
                    let (dx, dy) = (x as f32 - 11.5, y as f32 - 11.5);
                    let d = (dx * dx + dy * dy).sqrt();
                    if (9.0..11.0).contains(&d) {
                        im.set(x, y, rgb(0xd8a040));
                    }
                    let fl = (dx.abs() / 5.0) + ((dy + 2.0) / 7.0).abs();
                    if fl < 1.0 && dy < 6.0 {
                        im.set(x, y, fire(fl));
                    }
                }
            }
        }
        Skill::Hydra => {
            // Three flame heads on necks.
            for (k, hx) in [6, 12, 18].iter().enumerate() {
                let top = if k == 1 { 3 } else { 6 };
                for y in top..20 {
                    im.set(*hx, y, mix(rgb(0xc04010), rgb(0x701808), (y - top) as f32 / 16.0));
                    im.set(hx + 1, y, mix(rgb(0xc04010), rgb(0x701808), (y - top) as f32 / 16.0));
                }
                im.ellipse(*hx as f32 + 1.0, top as f32 + 1.0, 3.0, 2.5, rgb(0xffb040));
                im.set(hx + 2, top, rgb(0xfff0a0));
            }
            im.fill(3, 19, 18, 3, rgb(0x5a2010));
        }
        Skill::Phoenix => {
            // Spread wings of fire.
            for y in 0..24 {
                for x in 0..24 {
                    let (dx, dy) = ((x as f32 - 11.5).abs(), y as f32);
                    let wing = dy > 4.0 + dx * 0.3 && dy < 10.0 + dx * 0.6 && dx < 11.0;
                    let body = dx < 2.0 && (6.0..20.0).contains(&dy);
                    if wing || body {
                        im.set(x, y, fire(if body { 0.1 } else { dx / 11.0 }));
                    }
                }
            }
        }
        Skill::Fireball => {}
        s if crate::inventor::is_inventor(s) => inventor_icon(s, &mut im),
        s if crate::valkyrie::is_valkyrie(s) => valkyrie_icon(s, &mut im),
        s if crate::berserker::is_berserker(s) => berserker_icon(s, &mut im),
        s if crate::reaper::is_reaper(s) => reaper_icon(s, &mut im),
        s if crate::druid::is_druid(s) => druid_icon(s, &mut im),
        _ => vampire_icon(s, &mut im),
    }
    im.ax = 0;
    im.ay = 0;
    im
}

/// The valkyrie's icons: steel and glacier blue on midnight, no red.
fn valkyrie_icon(s: crate::skills::Skill, im: &mut Sprite) {
    use crate::skills::Skill;
    im.fill(0, 0, 24, 24, rgb(0x0c1018));
    let ice = |d: f32| mix(rgb(0xe8faff), rgb(0x2a5a90), d.clamp(0.0, 1.0));
    let steel = rgb(0x8a96a8);
    // A spear from (x0, y0) to (x1, y1) with an ice blade at the far end.
    let spear = |im: &mut Sprite, x0: i32, y0: i32, x1: i32, y1: i32| {
        let n = (x1 - x0).abs().max((y1 - y0).abs()).max(1);
        for t in 0..=n {
            let x = x0 + (x1 - x0) * t / n;
            let y = y0 + (y1 - y0) * t / n;
            let blade = t * 4 > n * 3;
            im.set(x, y, if blade { ice(0.1) } else { steel });
            if blade {
                im.set(x + 1, y, ice(0.4));
            }
        }
    };
    let ring = |im: &mut Sprite, cx: f32, cy: f32, r0: f32, r1: f32| {
        for y in 0..24 {
            for x in 0..24 {
                let d = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt();
                if d > r0 && d < r1 && hash(x, y, 9) % 4 != 0 {
                    im.set(x, y, ice((d - r0) / (r1 - r0)));
                }
            }
        }
    };
    match s {
        Skill::RuneSpear => spear(im, 3, 20, 20, 3),
        Skill::RimeSweep => {
            ring(im, 4.0, 20.0, 12.0, 17.0);
            spear(im, 4, 20, 14, 10);
        }
        Skill::Northborn => {
            // A shield-like crest with a rune.
            for y in 4..21 {
                let half = if y < 14 { 8 } else { 8 - (y - 14) };
                for x in 12 - half..12 + half {
                    im.set(x, y, if (x + y) % 5 == 0 { rgb(0x3a4a60) } else { rgb(0x2a3448) });
                }
            }
            for y in 7..18 {
                im.set(12, y, ice(0.1));
            }
            for k in 0..3 {
                im.set(11 - k, 9 + k, ice(0.2));
                im.set(13 + k, 9 + k, ice(0.2));
            }
        }
        Skill::RavenStrike => {
            // A diving raven with a blue eye.
            for k in 0..9 {
                im.set(12 - k, 10 - k / 2, rgb(0x202030));
                im.set(12 + k, 10 - k / 2, rgb(0x202030));
                im.set(12 - k, 11 - k / 2, rgb(0x14141c));
                im.set(12 + k, 11 - k / 2, rgb(0x14141c));
            }
            im.fill(10, 10, 5, 7, rgb(0x0c0c14));
            im.fill(11, 17, 3, 3, rgb(0x0c0c14));
            im.set(11, 11, ice(0.0));
        }
        Skill::GlacierLeap => {
            // An arc of travel and an icy landing burst.
            for t in 0..18 {
                let x = 3 + t;
                let y = 18 - ((t as f32 / 17.0 * std::f32::consts::PI).sin() * 13.0) as i32;
                im.set(x, y, ice(0.3));
            }
            ring(im, 20.0, 19.0, 1.0, 4.0);
        }
        Skill::FrostBrand => {
            // A snowflake.
            for k in 0..6 {
                let a = k as f32 * std::f32::consts::PI / 3.0;
                for t in 0..9 {
                    im.set(12 + (a.cos() * t as f32) as i32, 12 + (a.sin() * t as f32) as i32, ice(t as f32 / 10.0));
                }
            }
        }
        Skill::RuneJavelin => {
            spear(im, 2, 14, 21, 9);
            for x in 2..10 {
                im.set(x, 17, ice(0.6));
            }
        }
        Skill::WintersWrath => {
            ring(im, 12.0, 12.0, 6.0, 10.0);
            spear(im, 6, 12, 18, 12);
        }
        Skill::Einherjar => {
            // Two ghostly warriors with round shields.
            for (cx, sh) in [(8, 0.3f32), (16, 0.5)] {
                im.fill(cx - 2, 6, 4, 4, ice(sh));
                im.fill(cx - 3, 10, 6, 9, ice(sh + 0.1));
                im.fill(cx - 5, 11, 3, 5, rgb(0x5a7a9a));
            }
        }
        Skill::ValkyrieRide => {
            // A charging horse's head and a spear point ahead.
            im.fill(5, 9, 9, 6, rgb(0x14161c));
            im.fill(11, 6, 5, 6, rgb(0x14161c));
            im.fill(14, 8, 3, 3, rgb(0x20242c));
            im.set(13, 7, ice(0.0));
            im.fill(8, 7, 4, 2, ice(0.2));
            spear(im, 2, 18, 22, 14);
        }
        Skill::Fimbulwinter => {
            for y in 0..24 {
                for x in 0..24 {
                    if hash(x, y, 31) % 9 == 0 {
                        im.set(x, y, ice((hash(x, y, 5) % 10) as f32 / 10.0));
                    }
                }
            }
            for k in 0..9 {
                im.set(12 - k, 9 + k / 3, rgb(0x101018));
                im.set(12 + k, 9 + k / 3, rgb(0x101018));
                im.set(12 - k, 10 + k / 3, ice(k as f32 / 9.0));
                im.set(12 + k, 10 + k / 3, ice(k as f32 / 9.0));
            }
        }
        _ => {}
    }
}

/// The berserker's icons: iron, leather and dried crimson on dark earth. No magic glow.
fn berserker_icon(s: crate::skills::Skill, im: &mut Sprite) {
    use crate::skills::Skill;
    im.fill(0, 0, 24, 24, rgb(0x14100c));
    let iron = rgb(0x8a8e94);
    let dark = rgb(0x4a4c50);
    let wood = rgb(0x6a4a2a);
    let blood = rgb(0x8a1810);
    // A big axe: haft from (x0, y0) toward (x1, y1), head at the far end.
    let axe = |im: &mut Sprite, x0: i32, y0: i32, x1: i32, y1: i32| {
        let n = (x1 - x0).abs().max((y1 - y0).abs()).max(1);
        for t in 0..=n {
            im.set(x0 + (x1 - x0) * t / n, y0 + (y1 - y0) * t / n, wood);
        }
        for dy in -4..=4i32 {
            let w = 4 - dy.abs() / 2;
            for dx in 0..w {
                im.set(x1 + dx - 1, y1 + dy, if dx == w - 1 { iron } else { dark });
            }
        }
    };
    match s {
        Skill::Cleave => {
            axe(im, 4, 20, 15, 7);
            for k in 0..9 {
                let a = 3.4 + k as f32 * 0.17;
                im.set(12 + (a.cos() * 9.0) as i32, 12 + (a.sin() * 9.0) as i32, rgb(0xb0a890));
            }
        }
        Skill::Rend => {
            axe(im, 4, 21, 13, 9);
            for k in 0..3 {
                for y in 6..19 {
                    im.set(16 + k * 2 + (y % 3 == 0) as i32, y, blood);
                }
            }
        }
        Skill::IronHide => {
            for y in 4..21 {
                let half = if y < 14 { 8 } else { 8 - (y - 14) };
                for x in 12 - half..12 + half {
                    im.set(x, y, if (x * 7 + y * 3) % 6 == 0 { dark } else { rgb(0x5a5c62) });
                }
            }
            for k in 0..5 {
                im.set(8 + k * 2, 9 + (k % 2), iron);
            }
        }
        Skill::LeapSlam => {
            for t in 0..14 {
                im.set(3 + t, 16 - ((t as f32 / 13.0 * std::f32::consts::PI).sin() * 10.0) as i32, rgb(0x9a9080));
            }
            axe(im, 14, 8, 18, 16);
            for x in 12..23 {
                im.set(x, 21, rgb(0x6a5030));
            }
        }
        Skill::DireWolf => {
            // A howling wolf's head.
            im.fill(7, 11, 9, 8, rgb(0x2a2626));
            im.fill(12, 6, 5, 6, rgb(0x2a2626));
            im.fill(15, 4, 4, 3, rgb(0x2a2626));
            im.set(9, 9, rgb(0x2a2626));
            im.set(8, 8, rgb(0x2a2626));
            im.set(14, 8, rgb(0xd0b060));
            for k in 0..3 {
                im.set(19 + k, 3 - k / 2, rgb(0x9a9080));
            }
        }
        Skill::Bloodlust => {
            for y in 4..21 {
                for x in 4..21 {
                    let (dx, dy) = (x as f32 - 12.0, y as f32 - 13.0);
                    let heart = (dx.abs() * 0.8 + dy) < 6.0 && ((dx - 3.0).powi(2) + (dy + 3.0).powi(2) < 14.0 || (dx + 3.0).powi(2) + (dy + 3.0).powi(2) < 14.0 || dy > -3.0);
                    if heart {
                        im.set(x, y, if (x + y) % 4 == 0 { rgb(0xb02818) } else { blood });
                    }
                }
            }
        }
        Skill::WarCry => {
            // An open-mouthed shout and sound arcs.
            im.fill(6, 8, 6, 8, rgb(0x8a6a50));
            im.fill(9, 12, 3, 3, rgb(0x200808));
            for r in [6.0f32, 9.0] {
                for k in 0..7 {
                    let a = -0.8 + k as f32 * 0.27;
                    im.set(11 + (a.cos() * r) as i32, 12 + (a.sin() * r) as i32, rgb(0xe0a060));
                }
            }
        }
        Skill::Whirlwind => {
            for k in 0..40 {
                let a = k as f32 * 0.4;
                let r = 2.0 + k as f32 * 0.22;
                im.set(12 + (a.cos() * r) as i32, 12 + (a.sin() * r) as i32, if k % 3 == 0 { iron } else { rgb(0x9a9080) });
            }
        }
        Skill::Executioner => {
            axe(im, 3, 21, 14, 8);
            // A skull.
            im.fill(15, 14, 6, 5, rgb(0xd8d0b8));
            im.fill(16, 19, 4, 2, rgb(0xd8d0b8));
            im.set(16, 16, BLACK);
            im.set(19, 16, BLACK);
        }
        Skill::HurlAxe => {
            for k in 0..4 {
                let a = k as f32 * std::f32::consts::FRAC_PI_2 + 0.4;
                for t in 0..7 {
                    im.set(12 + (a.cos() * t as f32) as i32, 12 + (a.sin() * t as f32) as i32, if t > 4 { iron } else { wood });
                }
            }
            for x in 2..8 {
                im.set(x, 20, rgb(0x9a9080));
            }
        }
        Skill::Berserk => {
            for y in 0..24 {
                for x in 0..24 {
                    let d = ((x as f32 - 12.0).powi(2) + (y as f32 - 12.0).powi(2)).sqrt();
                    if d < 10.0 && hash(x, y, 13) % 3 == 0 {
                        im.set(x, y, if d < 5.0 { rgb(0xe03018) } else { blood });
                    }
                }
            }
            im.fill(9, 9, 6, 6, rgb(0x200808));
            im.set(10, 11, rgb(0xff6040));
            im.set(13, 11, rgb(0xff6040));
        }
        _ => {}
    }
}

/// The reaper's icons: spectral blue, aged bronze and parchment on charcoal.
fn reaper_icon(s: crate::skills::Skill, im: &mut Sprite) {
    use crate::skills::Skill;
    im.fill(0, 0, 24, 24, rgb(0x101216));
    let spirit = |d: f32| mix(rgb(0xe0f4ff), rgb(0x2a4a70), d.clamp(0.0, 1.0));
    let bronze = rgb(0xa07840);
    let parch = rgb(0xd8ccb0);
    let wood = rgb(0x2a2220);
    // The scythe: a long haft and a curved blade at the top.
    let scythe = |im: &mut Sprite, glow: bool| {
        for t in 0..18 {
            im.set(5 + t / 2, 21 - t, wood);
        }
        for k in 0..12 {
            let a = 3.6 + k as f32 * 0.16;
            let (x, y) = (14 + (a.cos() * 9.0) as i32, 8 + (a.sin() * 5.0) as i32);
            im.set(x, y, if glow { spirit(k as f32 / 14.0) } else { rgb(0x6a7280) });
            im.set(x, y + 1, rgb(0x30343a));
        }
    };
    match s {
        Skill::ReapingScythe => scythe(im, true),
        Skill::SpiritLantern => {
            im.fill(9, 6, 6, 1, bronze);
            im.fill(8, 7, 8, 11, rgb(0x2a2a30));
            im.fill(9, 8, 6, 9, spirit(0.3));
            im.fill(11, 10, 2, 4, spirit(0.0));
            im.fill(11, 3, 2, 3, bronze);
        }
        Skill::PatientArchivist => {
            // An hourglass with a key.
            for y in 4..20 {
                let w = ((y as i32 - 12).abs() / 2).max(1);
                for x in 12 - w..12 + w {
                    im.set(x, y, if y > 12 { rgb(0xd0a050) } else { rgb(0x3a4048) });
                }
            }
            im.fill(7, 3, 10, 1, bronze);
            im.fill(7, 20, 10, 1, bronze);
        }
        Skill::LedgerMark => {
            im.fill(5, 6, 7, 12, parch);
            im.fill(12, 6, 7, 12, parch);
            im.fill(11, 5, 2, 14, rgb(0x2a1a10));
            for k in 0..4 {
                im.fill(6, 8 + k * 2, 5, 1, rgb(0x5a6a80));
                im.fill(13, 8 + k * 2, 4 - k % 2, 1, spirit(0.2));
            }
        }
        Skill::ScholarSpirits => {
            for (cx, sh) in [(8, 0.2f32), (16, 0.45)] {
                im.fill(cx - 2, 5, 4, 4, spirit(sh));
                im.fill(cx - 3, 9, 6, 10, spirit(sh + 0.15));
                im.fill(cx + 2, 11, 3, 4, parch);
            }
        }
        Skill::ShadowStep => {
            for y in 0..24 {
                for x in 0..24 {
                    let d = ((x as f32 - 12.0).powi(2) + (y as f32 - 12.0).powi(2)).sqrt();
                    if d < 9.0 && hash(x, y, 21) % 3 == 0 {
                        im.set(x, y, if d < 4.0 { rgb(0x0c0c10) } else { rgb(0x2a2c34) });
                    }
                }
            }
            for x in 4..20 {
                im.set(x, 12, spirit(0.5));
            }
        }
        Skill::ChainsOfArchive => {
            for k in 0..8 {
                let (x, y) = (4 + k * 2, 20 - k * 2);
                im.fill(x, y, 3, 2, if k % 2 == 0 { rgb(0x8a96a8) } else { spirit(0.3) });
                let (x2, y2) = (20 - k * 2, 20 - k * 2);
                im.fill(x2 - 2, y2, 3, 2, if k % 2 == 1 { rgb(0x8a96a8) } else { spirit(0.3) });
            }
        }
        Skill::Hourglass => {
            for y in 3..21 {
                let w = ((y as i32 - 12).abs() * 2 / 3).max(1);
                for x in 12 - w..12 + w {
                    im.set(x, y, if (y > 13 && y > 20 - w) || (y < 11 && y > 4 + (12 - w)) { rgb(0xe0b060) } else { rgb(0x3a4048) });
                }
            }
            im.fill(6, 2, 12, 1, bronze);
            im.fill(6, 21, 12, 1, bronze);
            im.set(12, 12, rgb(0xe0b060));
        }
        Skill::RuneBlade => {
            scythe(im, false);
            for k in 0..7 {
                let a = 3.6 + k as f32 * 0.28;
                im.set(14 + (a.cos() * 8.0) as i32, 9 + (a.sin() * 4.0) as i32, spirit(0.0));
            }
        }
        Skill::SoulHarvest => {
            for k in 0..48 {
                let a = k as f32 / 48.0 * std::f32::consts::TAU;
                im.set(12 + (a.cos() * 9.0) as i32, 12 + (a.sin() * 9.0) as i32, spirit((k % 12) as f32 / 12.0));
            }
            scythe(im, true);
        }
        Skill::OpenLedger => {
            im.fill(3, 8, 9, 11, parch);
            im.fill(12, 8, 9, 11, parch);
            im.fill(11, 7, 2, 13, rgb(0x2a1a10));
            for y in 0..8 {
                for x in 0..24 {
                    if hash(x, y, 33) % 6 == 0 {
                        im.set(x, y, spirit((y as f32) / 8.0));
                    }
                }
            }
        }
        _ => {}
    }
}

/// The druid's icons: sickly green, moss, bark and bone on dark earth.
fn druid_icon(s: crate::skills::Skill, im: &mut Sprite) {
    use crate::skills::Skill;
    im.fill(0, 0, 24, 24, rgb(0x10120c));
    let spore = |d: f32| mix(rgb(0xd8f080), rgb(0x305010), d.clamp(0.0, 1.0));
    let bark = rgb(0x3a2a1a);
    let blob = |im: &mut Sprite, cx: f32, cy: f32, r: f32, seed: i32| {
        for y in 0..24 {
            for x in 0..24 {
                let d = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt();
                if d < r && hash(x, y, seed) % 3 != 0 {
                    im.set(x, y, spore(d / r));
                }
            }
        }
    };
    let rat = |im: &mut Sprite, x: i32, y: i32| {
        im.fill(x, y, 6, 3, rgb(0x1a1814));
        im.fill(x + 5, y - 1, 2, 2, rgb(0x1a1814));
        im.set(x + 6, y - 1, rgb(0xc04030));
        for k in 0..4 {
            im.set(x - 1 - k, y + 2 + k / 2, rgb(0x6a5a50));
        }
    };
    match s {
        Skill::SporeCloud => blob(im, 12.0, 12.0, 9.0, 3),
        Skill::RatSwarm => {
            rat(im, 4, 8);
            rat(im, 12, 13);
            rat(im, 5, 18);
        }
        Skill::GreenDoctor => {
            // The beaked plague mask.
            im.fill(5, 7, 8, 8, rgb(0x2a2420));
            for k in 0..10 {
                im.fill(12 + k, 10 + k / 3, 1, 3 - k / 4, rgb(0x3a3028));
            }
            im.fill(7, 9, 2, 2, rgb(0x90e040));
            im.fill(10, 9, 2, 2, rgb(0x90e040));
        }
        Skill::ThornLash => {
            for t in 0..20 {
                let x = 2 + t;
                let y = 18 - t / 2 + if t % 4 < 2 { 1 } else { -1 };
                im.set(x, y, bark);
                im.set(x, y + 1, bark);
                if t % 3 == 0 {
                    im.set(x, y - 2, rgb(0x5a5040));
                }
            }
        }
        Skill::MossWolf => {
            im.fill(5, 11, 11, 7, rgb(0x3a4a30));
            im.fill(14, 7, 6, 6, rgb(0x3a4a30));
            im.set(18, 9, rgb(0xc0e060));
            for k in 0..5 {
                im.set(6 + k * 2, 11, rgb(0x70a040));
            }
        }
        Skill::Rejuvenate => {
            for k in 0..3 {
                let x = 6 + k * 6;
                for y in 8..20 {
                    im.set(x, y, rgb(0x4a8a30));
                }
                im.fill(x - 2, 7, 3, 2, rgb(0x80d050));
                im.fill(x + 1, 10, 3, 2, rgb(0x80d050));
            }
        }
        Skill::FungalBloom => {
            for (x, h) in [(6, 8), (12, 12), (18, 9)] {
                im.fill(x - 1, 22 - h, 2, h, rgb(0xd8d0b0));
                im.fill(x - 3, 22 - h - 2, 7, 3, rgb(0x7a3a80));
                im.set(x - 1, 22 - h - 2, rgb(0xe0f080));
            }
        }
        Skill::CorpseBloom => {
            im.fill(6, 15, 12, 4, rgb(0x6a5a4a));
            blob(im, 12.0, 11.0, 6.0, 9);
            rat(im, 3, 20);
        }
        Skill::CycleOfRot => {
            for k in 0..40 {
                let a = k as f32 * std::f32::consts::TAU / 40.0;
                let c = if k < 20 { rgb(0x6a6a18) } else { rgb(0x4a9a30) };
                im.set(12 + (a.cos() * 8.0) as i32, 12 + (a.sin() * 8.0) as i32, c);
            }
            im.fill(11, 8, 2, 8, bark);
            im.fill(9, 8, 2, 2, rgb(0x80d050));
        }
        Skill::Pestilence => {
            blob(im, 12.0, 12.0, 11.0, 17);
            im.fill(9, 9, 6, 6, rgb(0xd8d0b0));
            im.set(10, 11, BLACK);
            im.set(13, 11, BLACK);
        }
        Skill::ThornWarden => {
            // A tree-guardian with antlers and a green heart.
            im.fill(8, 9, 8, 12, bark);
            im.fill(9, 4, 6, 5, rgb(0xd8d0b0));
            for k in 0..4 {
                im.set(8 - k, 4 - k, bark);
                im.set(15 + k, 4 - k, bark);
            }
            im.fill(11, 13, 2, 2, rgb(0x90f040));
            im.fill(5, 10, 3, 7, bark);
            im.fill(16, 10, 3, 7, bark);
        }
        _ => {}
    }
}

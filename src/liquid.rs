//! Water and lava: pools in the open ground of the generated maps. A pool's tiles block walking (they're
//! `Tile::Prop`, so pathing, the map check and everything that looks for open ground already keep off them)
//! but not sight or missiles: you can shoot across. Drawn animated in the floor pass (render.rs); lava lights
//! the dark around it.
use crate::dungeon::{Tile, LAVA, WATER};
use crate::gfx::{mix, rgb, Screen};
use crate::rng::Rng;
use crate::world::{Level, LevelId, Theme};

/// What a level gets: (liquid, how many pools). Overland water in the Ashlands, the Mistwood and the Churn
/// (the Churn's are lava too); lava in the Sanctum, the catacombs and the foundries; still water in the crypts.
fn plan(lv: &Level) -> Option<(u8, u8, i32)> {
    // (first liquid, second liquid (alternating), pools)
    match (lv.id, lv.theme) {
        (LevelId::Overworld | LevelId::Area(0, _), _) => Some((WATER, WATER, 4)),
        (LevelId::Mistwood | LevelId::Area(2, _), _) => Some((WATER, WATER, 5)),
        (LevelId::Churn | LevelId::Area(6, _), _) => Some((WATER, LAVA, 4)),
        (_, Theme::Sanctum | Theme::Catacombs | Theme::Foundry) => Some((LAVA, LAVA, 3)),
        (_, Theme::Crypt | Theme::Warrens | Theme::Barrow) => Some((WATER, WATER, 2)),
        _ => None,
    }
}

/// Lays the pools: each a ragged oval of 2-4 tiles across in open ground (a tile of open floor all round it,
/// so it never closes a corridor), clear of the ways in, the start, people, the town and the boss; and none
/// is kept if it would cut off any ground you could reach before.
pub fn place(lv: &mut Level, seed: u64) {
    let Some((a, b, pools)) = plan(lv) else { return };
    let mut rng = Rng::new(seed ^ 0x11c1_d5ea);
    let (w, h) = (lv.d.w, lv.d.h);
    let mut reach = crate::world::reachable(lv);
    let mut open: i32 = reach.iter().filter(|&&r| r).count() as i32;
    let mut avoid: Vec<(f32, f32, f32)> = vec![(lv.start.0, lv.start.1, 6.0)];
    avoid.extend(lv.portals.iter().map(|p| (p.x, p.y, 5.0)));
    avoid.extend(lv.npcs.iter().map(|n| (n.x, n.y, 5.0)));
    avoid.extend(lv.mobs.iter().filter(|m| m.boss).map(|m| (m.x, m.y, crate::world::ARENA_R as f32 + 3.0)));
    let big = lv.id.overland();
    let mut made = 0;
    for tries in 0..600 {
        if made >= pools {
            break;
        }
        let kind = if made % 2 == 0 { a } else { b };
        // Bigger on open ground; tried large first, smaller as the tries run on.
        let shrink = 1.0 - tries as f32 / 1200.0;
        let (lo, hi) = if big { (5, 10) } else { (3, 6) };
        let (rx, ry) = ((rng.range(lo, hi) as f32 * 0.5 + 0.5) * shrink, (rng.range(lo, hi) as f32 * 0.4 + 0.5) * shrink);
        let (cx, cy) = (rng.range(7, w - 7), rng.range(7, h - 7));
        if !reach[(cy * w + cx) as usize] {
            continue;
        }
        let (fx, fy) = (cx as f32 + 0.5, cy as f32 + 0.5);
        if avoid.iter().any(|&(x, y, r)| (x - fx).powi(2) + (y - fy).powi(2) < (r + rx.max(ry)).powi(2)) {
            continue;
        }
        if let Some((x0, y0, x1, y1)) = lv.safe {
            if fx > x0 - 6.0 && fx < x1 + 6.0 && fy > y0 - 6.0 && fy < y1 + 6.0 {
                continue;
            }
        }
        let wob = rng.range(0, 1000) as f32 * 0.01;
        let mut tiles = vec![];
        for y in cy - 6..=cy + 6 {
            for x in cx - 6..=cx + 6 {
                let (dx, dy) = ((x - cx) as f32 / rx, (y - cy) as f32 / ry);
                let ang = dy.atan2(dx);
                if dx * dx + dy * dy < 1.0 + 0.22 * (ang * 3.0 + wob).sin() + 0.12 * (ang * 5.0 - wob * 1.7).sin() {
                    tiles.push((x, y));
                }
            }
        }
        // Open floor under it and a tile all round, off the roads.
        let clear = tiles.iter().all(|&(x, y)| {
            (-1..=1).all(|oy| (-1..=1).all(|ox| lv.d.get(x + ox, y + oy) == Tile::Floor && lv.d.ground_at(x + ox, y + oy) != 2))
        });
        if tiles.len() < 3 || !clear {
            continue;
        }
        for &(x, y) in &tiles {
            lv.d.set_liquid(x, y, kind);
        }
        let after = crate::world::reachable(lv);
        let left = after.iter().filter(|&&r| r).count() as i32;
        if left != open - tiles.len() as i32 {
            // It cut something off: take it back.
            for &(x, y) in &tiles {
                lv.d.clear_liquid(x, y);
            }
            continue;
        }
        open = left;
        reach = after;
        avoid.push((fx, fy, rx.max(ry) + 2.0));
        made += 1;
    }
}

/// One liquid tile, drawn over its floor: the colours flow in world space, so pools read as one surface;
/// a wet (or crusted) rim where it meets dry ground. `shore` says which sides are dry: +x, -x, +y, -y.
pub fn draw_tile(scr: &mut Screen, kind: u8, murky: bool, sx: i32, sy: i32, tx: i32, ty: i32, shore: [bool; 4], tick: u32, floor_h: i32) {
    let t = tick as f32 / 60.0;
    let (shx, shy) = scr.shake;
    for row in -8i32..=8 {
        // A pixel wider than the tile's diamond, so neighbouring tiles overlap and no seam shows.
        let half = 17 - row.abs() * 2;
        let y = sy + row + shy;
        if y < 0 || y >= floor_h {
            continue;
        }
        for dx in -half..half + 1 {
            let x = sx + dx + shx;
            if x < 0 || x >= scr.w {
                continue;
            }
            // Where in the tile (each -0.5..0.5), and in the world.
            let (fx, fy) = (dx as f32 + 0.5, row as f32);
            let (u, v) = ((fx / 16.0 + fy / 8.0) * 0.5, (fy / 8.0 - fx / 16.0) * 0.5);
            let (wx, wy) = (tx as f32 + 0.5 + u, ty as f32 + 0.5 + v);
            let mut edge: f32 = 1.0;
            for (k, d) in [0.5 - u, 0.5 + u, 0.5 - v, 0.5 + v].iter().enumerate() {
                if shore[k] {
                    edge = edge.min(*d);
                }
            }
            // The bank wanders: a wavy line in world space, so the shore isn't the tile grid's straight edge.
            let bank = 0.16 + 0.12 * ((wx * 3.7 + wy * 1.3).sin() * 0.6 + (wy * 4.3 - wx * 0.9).sin() * 0.4);
            if edge < bank {
                continue;
            }
            let edge = edge - bank;
            let c = if kind == LAVA {
                // Dark crust over a glowing flow: bright where the slow cracks run, pulsing.
                let n = (wx * 2.3 + (wy * 1.9 + t * 0.35).sin() * 1.6).sin() * (wy * 2.1 - t * 0.25 + (wx * 1.4).sin() * 1.3).sin();
                let crack = (1.0 - n.abs()).powi(5);
                let pulse = 0.8 + 0.2 * (t * 1.7 + wx * 0.6 + wy * 0.4).sin();
                let mut c = mix(rgb(0x3a0e06), rgb(0xff5a10), (crack * pulse).min(1.0));
                if crack > 0.8 {
                    c = mix(c, rgb(0xffe080), ((crack - 0.8) * 5.0 * pulse).min(1.0));
                }
                // A cooled black crust at the bank, glowing just inside it.
                if edge < 0.06 {
                    c = mix(rgb(0x140806), rgb(0x60200a), edge / 0.06);
                } else if edge < 0.14 {
                    c = mix(c, rgb(0xff8020), (1.0 - (edge - 0.06) / 0.08) * 0.5);
                }
                c
            } else {
                // Deep in the middle, shallow at the rim, the swell moving across, glints on the crests.
                let wv = (wx * 2.1 + t * 1.2).sin() + (wy * 2.6 - t * 0.9).sin() + ((wx + wy) * 1.3 + t * 0.6).sin();
                let (deep, shallow, glint) = if murky {
                    (rgb(0x1e4a40), rgb(0x48745c), rgb(0xb8e0c8))
                } else {
                    (rgb(0x123848), rgb(0x2e6a80), rgb(0xb0e4f4))
                };
                let depth = (edge / 0.35).clamp(0.0, 1.0);
                let mut c = mix(shallow, deep, depth * 0.75 + (wv / 3.0 * 0.5 + 0.5) * 0.25);
                // Long soft bands of reflected sky, and bright glints on the crests.
                let band = ((wx * 0.9 - wy * 0.5 + t * 0.3).sin() * 0.5 + 0.5).powi(8);
                c = mix(c, glint, band * 0.25);
                if wv > 2.3 {
                    c = mix(c, glint, ((wv - 2.3) * 1.6).min(0.85));
                }
                // A pale foam line at the water's edge, then the wet dark bank.
                if edge < 0.035 {
                    c = mix(c, rgb(0x1c1610), 0.7);
                } else if edge < 0.07 {
                    c = mix(c, glint, 0.35);
                }
                c
            };
            let i = (y * scr.w + x) as usize;
            scr.px[i] = c;
        }
    }
}

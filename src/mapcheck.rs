//! Checks that randomly generated maps are always playable (every map is generated from a seed: each hero's
//! world, each difficulty and each rekindling makes different ones). For every level of every act, over many
//! seeds and all three difficulties: every way in and out (stairs, doors, roads, passes, town portals), every
//! boss, townsperson, shrine and pickup can be reached on foot from where you arrive (airship docks count
//! as a ride to the other dock), and no monster stands inside a wall.
//!
//! `cargo test --release maps_are_playable` runs a few seeds; `MAPCHECK_SEEDS=200 cargo test --release
//! maps_are_playable -- --nocapture` runs a long sweep.
#![cfg(test)]

use crate::dungeon::Tile;
use crate::world::{build_at, near_seen, reachable, Level};

/// Everything wrong with one generated level.
fn problems(lv: &Level) -> Vec<String> {
    let seen = reachable(lv);
    let mut out = vec![];
    for p in &lv.portals {
        if !near_seen(lv, &seen, p.x, p.y) {
            out.push(format!("can't reach the {:?} at {:.0},{:.0}", p.kind, p.x, p.y));
        }
    }
    for m in lv.mobs.iter().filter(|m| m.boss) {
        if !near_seen(lv, &seen, m.x, m.y) {
            out.push(format!("can't reach the boss {:?} at {:.0},{:.0}", m.kind, m.x, m.y));
        }
    }
    for n in &lv.npcs {
        if !near_seen(lv, &seen, n.x, n.y) {
            out.push(format!("can't reach {} at {:.0},{:.0}", n.name, n.x, n.y));
        }
    }
    for k in &lv.pickups {
        if !near_seen(lv, &seen, k.x, k.y) {
            out.push(format!("can't reach a {:?} at {:.0},{:.0}", std::mem::discriminant(&k.kind), k.x, k.y));
        }
    }
    let stuck = lv.mobs.iter().filter(|m| !crate::breakables::is_prop(m.kind) && lv.d.get(m.x.floor() as i32, m.y.floor() as i32) != Tile::Floor).count();
    if stuck > 0 {
        out.push(format!("{stuck} monster(s) standing inside walls"));
    }
    out
}

#[test]
fn maps_are_playable() {
    let seeds: u64 = std::env::var("MAPCHECK_SEEDS").ok().and_then(|s| s.parse().ok()).unwrap_or(4);
    let mut bad = vec![];
    let mut checked = 0;
    for seed in 0..seeds {
        let s = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0x51ED;
        for difficulty in 0..3 {
            for id in crate::levels::all_ids() {
                // Hand-made town maps don't change with the seed: one check is enough.
                if id.town() && (seed > 0 || difficulty > 0) {
                    continue;
                }
                let lv = build_at(id, s, difficulty);
                checked += 1;
                for p in problems(&lv) {
                    bad.push(format!("{} (seed {s:#x}, difficulty {difficulty}): {p}", crate::levels::file_name(id)));
                }
            }
        }
    }
    println!("checked {checked} generated levels, {} problems", bad.len());
    for b in bad.iter().take(40) {
        println!("  {b}");
    }
    assert!(bad.is_empty(), "{} problems in generated maps, first: {}", bad.len(), bad[0]);
}

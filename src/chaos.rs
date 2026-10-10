//! Act 7, the Churning Chaos (docs/ACT7_PLAN.md): the realm under the lid Solanthos's light kept shut.
//!
//! The act's rule is instability, the user's picks:
//! - **Chaos surges**: every 40-60 seconds the Churn shudders (a warning), then whole patches of the land
//!   around you reshape: walls melt and rise, and the ground turns to lava, ice or water. The roads between
//!   the ways in and out never close, and the ground right around you never changes under your feet.
//! - **Anchor stones**: obelisks of old stone in every area. Touch one and it lights, and the land within
//!   ten tiles of it stops shifting for good.
//! - **Stillness**: a meter that fills while you stand still, and faster near a lit anchor. When a surge comes
//!   with the meter full, it's spent and the surge passes you by.
//!
//! The ground here is a patchwork of every world the ash has fallen on (the Drift of Worlds), plus the
//! Churn's own: ground codes below, drawn by art.rs.
use crate::dungeon::Tile;
use crate::game::{Game, PKind, Sfx, DT};
use crate::gfx::rgb;
use crate::world::LevelId;

/// Act 7 ground codes (Theme::Churn).
pub const G_CHAOS: u8 = 0;
pub const G_ASH: u8 = 1;
pub const G_ROAD: u8 = 2;
pub const G_SNOW: u8 = 3;
pub const G_MIST: u8 = 4;
pub const G_BRASS: u8 = 5;
pub const G_SAND: u8 = 6;
pub const G_LAVA: u8 = 7;
pub const G_ICE: u8 = 8;
pub const G_WATER: u8 = 9;
pub const G_MARBLE: u8 = 10;

/// Surge rhythm (seconds): calm between, and the warning.
pub const CALM: (f32, f32) = (40.0, 60.0);
pub const WARN: f32 = 3.0;
/// A lit anchor stills the land this far around it (tiles).
pub const ANCHOR_R: f32 = 10.0;
/// Stillness: per second standing still, extra near a lit anchor; a surge skips this far around you.
pub const STILL_RATE: f32 = 10.0;
pub const STILL_NEAR: f32 = 6.0;
pub const STILL_SKIP: f32 = 14.0;

/// An anchor stone.
#[derive(Clone, Debug)]
pub struct Anchor {
    pub level: LevelId,
    pub x: f32,
    pub y: f32,
    pub lit: bool,
    /// It's given Sister Ferro its lump of chaos ore (churnside.rs).
    pub ore: bool,
}

/// The act an Act 7 ground belongs to (its props and its look), 6 for the Churn's own.
pub fn ground_act(g: u8) -> u8 {
    match g {
        G_ASH => 0,
        G_SNOW => 1,
        G_MIST => 2,
        G_BRASS => 3,
        G_SAND => 4,
        G_MARBLE => 5,
        _ => 6,
    }
}

/// The grounds a surge can turn an area's land into.
pub fn surge_grounds(n: u8) -> &'static [u8] {
    match n {
        2 => &[G_ASH, G_SNOW, G_MIST, G_BRASS, G_SAND, G_MARBLE, G_CHAOS],
        3 => &[G_LAVA, G_ICE, G_WATER, G_CHAOS, G_CHAOS],
        4 => &[G_WATER, G_SAND, G_CHAOS],
        5 => &[G_MARBLE, G_CHAOS, G_CHAOS],
        6 => &[G_CHAOS, G_CHAOS, G_LAVA],
        _ => &[G_CHAOS, G_CHAOS, G_MARBLE, G_LAVA],
    }
}

/// Is this an area of the Churn (where the land surges)?
pub fn churning(id: LevelId) -> bool {
    matches!(id, LevelId::Area(6, _) | LevelId::Dungeon(crate::world::CATHEDRAL | crate::world::EYE, _))
}

impl Game {
    /// Arriving in an area of the Churn: its anchor stones (once), and the roads that must never close.
    pub(crate) fn chaos_enter(&mut self) {
        if !churning(self.level) {
            return;
        }
        if self.feats.surge_t <= 0.0 {
            self.feats.surge_t = self.rng.rf(CALM.0, CALM.1);
        }
        self.feats.surge_warn = 0.0;
        if !self.feats.chaos_placed.contains(&self.level) {
            self.feats.chaos_placed.push(self.level);
            let mut placed: Vec<(f32, f32)> = vec![];
            for _ in 0..800 {
                if placed.len() >= 4 {
                    break;
                }
                let x = self.rng.range(8, self.d.w - 8) as f32 + 0.5;
                let y = self.rng.range(8, self.d.h - 8) as f32 + 0.5;
                if self.d.blocked(x, y, 1.0) || placed.iter().any(|&(px, py)| (px - x).powi(2) + (py - y).powi(2) < 400.0) {
                    continue;
                }
                if self.portals.iter().any(|p| (p.x - x).powi(2) + (p.y - y).powi(2) < 100.0) {
                    continue;
                }
                if self.d.path((self.p.x as i32, self.p.y as i32), (x as i32, y as i32), 30_000).is_none() {
                    continue;
                }
                placed.push((x, y));
                self.feats.anchors.push(Anchor { level: self.level, x, y, lit: false, ore: false });
            }
        }
        self.lifelines();
    }

    /// Marks the roads between the ways in and out, and to the anchors: a surge never walls them up.
    fn lifelines(&mut self) {
        let (w, h) = (self.d.w, self.d.h);
        let mut keep = vec![false; (w * h) as usize];
        let mark = |x: i32, y: i32, r: i32, keep: &mut Vec<bool>| {
            for yy in y - r..=y + r {
                for xx in x - r..=x + r {
                    if xx >= 0 && yy >= 0 && xx < w && yy < h {
                        keep[(yy * w + xx) as usize] = true;
                    }
                }
            }
        };
        let start = (self.start.0 as i32, self.start.1 as i32);
        let mut goals: Vec<(i32, i32)> = self.portals.iter().map(|p| (p.x as i32, p.y as i32)).collect();
        goals.extend(self.feats.anchors.iter().filter(|a| a.level == self.level).map(|a| (a.x as i32, a.y as i32)));
        let wp = self.waypoint;
        if wp != (0.0, 0.0) {
            goals.push((wp.0 as i32, wp.1 as i32));
        }
        mark(start.0, start.1, 4, &mut keep);
        for g in goals {
            mark(g.0, g.1, 4, &mut keep);
            if let Some(path) = self.d.path(start, g, 60_000) {
                for (px, py) in path {
                    mark(px as i32, py as i32, 1, &mut keep);
                }
            }
        }
        self.feats.lifeline = (Some(self.level), keep);
    }

    // ------------------------------------------------------------------ every tick

    pub(crate) fn update_chaos(&mut self) {
        if !churning(self.level) {
            return;
        }
        // Stillness: standing still, and near a lit anchor.
        let near_anchor = self.feats.anchors.iter().any(|a| a.lit && a.level == self.level && (a.x - self.p.x).powi(2) + (a.y - self.p.y).powi(2) < ANCHOR_R * ANCHOR_R);
        if !self.p.moving && self.p.cast_t <= 0.0 {
            self.feats.stillness = (self.feats.stillness + STILL_RATE * DT).min(100.0);
        }
        if near_anchor {
            self.feats.stillness = (self.feats.stillness + STILL_NEAR * DT).min(100.0);
        }
        // Touch an anchor to light it.
        let (px, py) = (self.p.x, self.p.y);
        let here = self.level;
        if let Some(a) = self.feats.anchors.iter_mut().find(|a| !a.lit && a.level == here && (a.x - px).powi(2) + (a.y - py).powi(2) < 1.4 * 1.4) {
            a.lit = true;
            let (ax, ay) = (a.x, a.y);
            self.feats.stillness = (self.feats.stillness + 30.0).min(100.0);
            self.save_due = true;
            for _ in 0..24 {
                self.spray_at(ax, ay, PKind::Holy, 10.0);
            }
            self.sfx.push(Sfx::Cast);
            self.floater(ax, ay, "THE ANCHOR HOLDS. THE LAND HERE IS STILL".into(), rgb(0xffe0a0));
        }
        // The ground: lava burns, ice and water chill.
        let g = self.d.ground_at(px as i32, py as i32);
        if self.d.get(px as i32, py as i32) == Tile::Floor {
            match g {
                G_LAVA => {
                    self.hurt_player(5.0 * self.tier * DT);
                    if self.tick % 20 == 0 {
                        self.spray_at(px, py, PKind::Fire, 6.0);
                    }
                }
                G_ICE | G_WATER => self.p.chill = self.p.chill.max(if g == G_ICE { 0.4 } else { 0.25 }),
                _ => {}
            }
        }
        // The surge's clock.
        if self.feats.surge_warn > 0.0 {
            self.feats.surge_warn -= DT;
            if self.feats.surge_warn <= 0.0 {
                self.surge();
                self.feats.surge_t = self.rng.rf(CALM.0, CALM.1);
            }
        } else {
            self.feats.surge_t -= DT;
            if self.feats.surge_t <= 0.0 {
                self.feats.surge_warn = WARN;
                self.say("THE CHURN SHUDDERS... A SURGE IS COMING".into());
                self.sfx.push(Sfx::Descend);
            }
        }
    }

    /// The land reshapes in patches around you.
    pub(crate) fn surge(&mut self) {
        // The Unfinished Cathedral builds and unbuilds itself like the land outside (as the Breach).
        let n = match self.level {
            LevelId::Area(_, n) => n,
            LevelId::Dungeon(..) => 1,
            _ => return,
        };
        if self.feats.lifeline.0 != Some(self.level) {
            self.lifelines();
        }
        let (px, py) = (self.p.x, self.p.y);
        let skip_r = if self.feats.stillness >= 100.0 {
            self.feats.stillness = 0.0;
            self.floater(px, py, "YOUR STILLNESS HOLDS. THE SURGE PASSES YOU BY".into(), rgb(0xc0e0ff));
            STILL_SKIP
        } else {
            3.5
        };
        let grounds = surge_grounds(n);
        let lit: Vec<(f32, f32)> = self.feats.anchors.iter().filter(|a| a.lit && a.level == self.level).map(|a| (a.x, a.y)).collect();
        let (w, h) = (self.d.w, self.d.h);
        let mut changed = 0;
        for _ in 0..6 {
            let a = self.rng.f() * std::f32::consts::TAU;
            let r = self.rng.rf(6.0, 18.0);
            let (cx, cy) = ((px + a.cos() * r) as i32, (py + a.sin() * r) as i32);
            let ground = grounds[self.rng.range(0, grounds.len() as i32) as usize];
            for y in cy - 3..=cy + 3 {
                for x in cx - 3..=cx + 3 {
                    if x < 3 || y < 3 || x >= w - 3 || y >= h - 3 {
                        continue;
                    }
                    let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
                    if (fx - px).powi(2) + (fy - py).powi(2) < skip_r * skip_r {
                        continue;
                    }
                    if lit.iter().any(|&(ax, ay)| (ax - fx).powi(2) + (ay - fy).powi(2) < ANCHOR_R * ANCHOR_R) {
                        continue;
                    }
                    let t = self.d.get(x, y);
                    if t == Tile::Prop || t == Tile::Void {
                        continue;
                    }
                    let kept = self.feats.lifeline.1.get((y * w + x) as usize).copied().unwrap_or(true);
                    let wall = !kept && self.rng.chance(0.28);
                    self.d.set(x, y, if wall { Tile::Wall } else { Tile::Floor });
                    if !wall {
                        self.d.set_ground(x, y, ground);
                    }
                    changed += 1;
                }
            }
            for _ in 0..6 {
                self.spray_at(cx as f32 + 0.5, cy as f32 + 0.5, PKind::Magic, 12.0);
            }
        }
        // Nothing left inside a new wall: monsters and things on the floor step out to the nearest open tile.
        for i in 0..self.mobs.len() {
            let (mx, my, r) = (self.mobs[i].x, self.mobs[i].y, self.mobs[i].r);
            if self.mobs[i].alive() && self.d.blocked(mx, my, r * 0.5) {
                if let Some((nx, ny)) = self.open_near(mx, my) {
                    (self.mobs[i].x, self.mobs[i].y) = (nx, ny);
                }
            }
        }
        for k in 0..self.pickups.len() {
            let (x, y) = (self.pickups[k].x, self.pickups[k].y);
            if self.d.blocked(x, y, 0.1) {
                if let Some((nx, ny)) = self.open_near(x, y) {
                    (self.pickups[k].x, self.pickups[k].y) = (nx, ny);
                }
            }
        }
        // Never shut in: if the way back to the roads is gone, the Churn opens a path.
        let (sx, sy) = (px as i32, py as i32);
        let to = self.nearest_kept(sx, sy);
        if let Some((tx, ty)) = to {
            if self.d.path((sx, sy), (tx, ty), 8_000).is_none() {
                let (mut x, mut y) = (sx, sy);
                while (x, y) != (tx, ty) {
                    x += (tx - x).signum();
                    y += (ty - y).signum();
                    if self.d.get(x, y) == Tile::Wall {
                        self.d.set(x, y, Tile::Floor);
                    }
                }
            }
        }
        self.p.path.clear();
        self.shake = self.shake.max(0.6);
        self.sfx.push(Sfx::Boom);
        self.feats.surges += 1;
        self.surge_stirs((px, py));
        if changed > 0 {
            self.say("THE LAND HEAVES AND RESHAPES ITSELF".into());
        }
    }

    /// The nearest open tile to (x, y).
    fn open_near(&self, x: f32, y: f32) -> Option<(f32, f32)> {
        for r in 1..8 {
            for dy in -r..=r {
                for dx in -r..=r {
                    let (nx, ny) = (x.floor() + dx as f32 + 0.5, y.floor() + dy as f32 + 0.5);
                    if !self.d.blocked(nx, ny, 0.35) {
                        return Some((nx, ny));
                    }
                }
            }
        }
        None
    }

    /// The nearest tile of the roads that never close.
    fn nearest_kept(&self, x: i32, y: i32) -> Option<(i32, i32)> {
        let w = self.d.w;
        let keep = &self.feats.lifeline.1;
        for r in 0..40i32 {
            for dy in -r..=r {
                for dx in -r..=r {
                    if dx.abs() != r && dy.abs() != r {
                        continue;
                    }
                    let (nx, ny) = (x + dx, y + dy);
                    if nx >= 0 && ny >= 0 && nx < w && ny < self.d.h && keep.get((ny * w + nx) as usize).copied().unwrap_or(false) && self.d.get(nx, ny) == Tile::Floor {
                        return Some((nx, ny));
                    }
                }
            }
        }
        None
    }
}

/// The stillness meter, under the top bar, while you're out in the Churn.
pub fn draw_chaos_hud(g: &Game, scr: &mut crate::gfx::Screen) {
    use crate::gfx::Align;
    if !churning(g.level) {
        return;
    }
    let w = scr.w;
    let bw = 120;
    let (x, y) = (w / 2 - bw / 2, 84);
    let f = g.feats.stillness / 100.0;
    scr.fill(x - 1, y - 1, bw + 2, 6, rgb(0x100c18));
    scr.fill(x, y, (bw as f32 * f) as i32, 4, if f >= 1.0 { rgb(0xc0e0ff) } else { rgb(0x6080c0) });
    let label = if f >= 1.0 { "STILL: THE NEXT SURGE WILL PASS YOU BY" } else { "STILLNESS" };
    scr.text(label, w / 2, y + 7, rgb(0x9090c0), Align::Center, 1);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Game;

    fn at(id: LevelId) -> Game {
        let mut g = Game::new(7, 360);
        g.p.base_hp = 50000.0;
        g.p.recalc();
        g.p.hp = g.p.max_hp;
        g.debug_goto(id);
        g
    }

    #[test]
    fn surges_reshape_the_land_but_never_the_roads_or_your_feet() {
        let mut g = at(LevelId::Area(6, 3));
        let before: Vec<(Tile, u8)> = (0..g.d.w * g.d.h).map(|i| (g.d.tiles[i as usize], g.d.ground[i as usize])).collect();
        let (px, py) = (g.p.x as i32, g.p.y as i32);
        for _ in 0..4 {
            g.surge();
        }
        let after: Vec<(Tile, u8)> = (0..g.d.w * g.d.h).map(|i| (g.d.tiles[i as usize], g.d.ground[i as usize])).collect();
        assert!(before != after, "the land changed");
        assert_eq!(g.d.get(px, py), Tile::Floor, "not under your feet");
        // Every way out is still reachable.
        let ways: Vec<(i32, i32)> = g.portals.iter().map(|p| (p.x as i32, p.y as i32)).collect();
        for w in ways {
            assert!(g.d.path((px, py), w, 80_000).is_some(), "the way to {w:?} stays open");
        }
    }

    #[test]
    fn a_lit_anchor_stills_the_land_and_stillness_turns_a_surge_aside() {
        let mut g = at(LevelId::Area(6, 1));
        let a = g.feats.anchors.iter().find(|a| a.level == g.level).cloned().expect("anchors stand in the Breach");
        (g.p.x, g.p.y) = (a.x, a.y);
        g.update_chaos();
        assert!(g.feats.anchors.iter().any(|b| b.lit && (b.x - a.x).abs() < 0.01));
        let snap = |g: &Game| -> Vec<Tile> {
            let mut v = vec![];
            for y in (a.y as i32 - 8)..=(a.y as i32 + 8) {
                for x in (a.x as i32 - 8)..=(a.x as i32 + 8) {
                    if (x as f32 + 0.5 - a.x).powi(2) + (y as f32 + 0.5 - a.y).powi(2) < 9.0 * 9.0 {
                        v.push(g.d.get(x, y));
                    }
                }
            }
            v
        };
        let before = snap(&g);
        // Stand still a while: the meter fills.
        g.p.moving = false;
        for _ in 0..(12.0 / DT) as i32 {
            g.update_chaos();
            g.feats.surge_t = 99.0;
        }
        assert!(g.feats.stillness >= 100.0);
        g.surge();
        assert_eq!(g.feats.stillness, 0.0, "spent");
        g.surge();
        assert_eq!(before, snap(&g), "nothing moves around a lit anchor");
    }
}

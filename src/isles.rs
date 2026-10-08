//! Act 6's set pieces (docs/SIDE_CONTENT_PLAN.md 2c), the user's picks:
//!
//! - **Island Hopping**: out in the Drifting Isles (and once in the Halo Isles) a line of stepping stones leads
//!   over the open sky to a little island with a sky-pirate's strongbox. Each stone crumbles a moment after you
//!   step on it and grows back a few seconds later: keep moving, and mind the wind.
//! - **Storm Chase** (the Stormfields): a lightning relic on a pedestal. Carry it into the storm cells that roam
//!   the fields to charge it (they zap you while you do), then set it back on its pedestal.
//! - **Fallen Star**: a star has come down in one of the Skyreach's areas (a different one every playthrough).
//!   Star-spawn guard the crater; break the three lumps of star-metal for its treasures.
//! - **The Last Choir**: three lost singers wait in three areas. Find them all and a sanctum of light opens in
//!   the Halo Isles, held by three guardians.
//!
//! Also here: Bram's three lost cargo crates and the Weeping Seraph's three tears (side.rs quests).
use crate::dungeon::Tile;
use crate::game::{Drop, Game, PKind, Pickup, Sfx, DT};
use crate::gfx::rgb;
use crate::mobs::{Kind, Mob, Rank};
use crate::rng::Rng;
use crate::story::Dialog;
use crate::world::LevelId;

pub const STORMFIELDS: LevelId = LevelId::Area(5, 2);
pub const HALO: LevelId = LevelId::Area(5, 4);
pub const DRIFTING: LevelId = LevelId::Area(5, 6);
/// Stepping stones: how long one holds after you step on it, and how long it takes to grow back (seconds).
pub const CRUMBLE: f32 = 0.9;
pub const REGROW: f32 = 7.0;
/// A storm cell's radius (tiles), and the relic's charge per second inside one / its loss outside.
pub const CELL_R: f32 = 3.0;
pub const CHARGE: f32 = 9.0;
pub const DRAIN: f32 = 3.0;
pub const GUARDIAN: &str = "SANCTUM GUARDIAN";

/// One stepping stone over the sky.
#[derive(Clone, Debug)]
pub struct Stone {
    pub level: LevelId,
    pub x: i32,
    pub y: i32,
    /// 0 solid, 1 cracking, 2 gone (growing back).
    pub st: u8,
    pub t: f32,
}

/// Where things lie this playthrough: the fallen star's area, the three singers' and Bram's three crates.
pub fn rolls(world_seed: u64, difficulty: u8) -> (u8, [u8; 3], [u8; 3]) {
    let mut rng = Rng::new(world_seed ^ 0x51CA5E ^ difficulty as u64 * 104_729);
    let shuffled = |rng: &mut Rng| {
        let mut a = vec![1u8, 2, 3, 5, 6];
        for i in (1..a.len()).rev() {
            a.swap(i, rng.range(0, i as i32 + 1) as usize);
        }
        a
    };
    let star = [1u8, 2, 3, 5][rng.range(0, 4) as usize];
    let s = shuffled(&mut rng);
    let c = shuffled(&mut rng);
    (star, [s[0], s[1], s[2]], [c[0], c[1], c[2]])
}

pub const SINGERS: [(&str, &str); 3] = [
    ("ALTO, THE LOST SINGER", "I SANG IN THE LAST CHOIR, BEFORE THE SUN FELL. I'VE BEEN HUMMING MY PART ALONE FOR A HUNDRED YEARS. FIND THE OTHERS. WHEN WE SING TOGETHER, THE SANCTUM OPENS."),
    ("TENOR, THE LOST SINGER", "YOU HEARD ME? NO ONE HEARS ME ANY MORE. THERE WERE THREE OF US LEFT. IF THE THREE OF US SING AGAIN, THE HIDDEN SANCTUM IN THE HALO ISLES WILL REMEMBER US."),
    ("TREBLE, THE LOST SINGER", "SHH. LISTEN. THERE: THE OTHERS ARE SINGING TOO, FAR AWAY. GO. I'LL KEEP THE NOTE UNTIL YOU'VE FOUND US ALL."),
];

impl Game {
    /// Act 6 set pieces as you arrive.
    pub(crate) fn isles_enter(&mut self) {
        let here = self.level;
        let LevelId::Area(5, n) = here else { return };
        // The sanctum's guardians stand whenever you come back with all three singers found.
        if here == HALO && self.feats.singers == 7 && self.feats.sanctum == 0 {
            self.place_sanctum();
        }
        if self.feats.isles_placed.contains(&here) {
            return;
        }
        self.feats.isles_placed.push(here);
        let hops = if here == DRIFTING { 3 } else if here == HALO { 1 } else { 0 };
        for _ in 0..hops {
            self.carve_hop();
        }
        if here == STORMFIELDS && self.feats.relic == 0 {
            if let Some((x, y)) = self.reef_spot_pub(14.0, 1.0) {
                self.feats.relic_spot = (x, y);
                self.pickups.push(Pickup { x, y, kind: Drop::StormRelic, t: 1.0 });
            }
        }
        let (star, singers, cargo) = rolls(self.world_seed, self.quest.difficulty);
        if n == star && !self.feats.star_done {
            self.place_star();
        }
        for k in 0..3 {
            if singers[k] == n && self.feats.singers & (1 << k) == 0 {
                if let Some((x, y)) = self.reef_spot_pub(16.0, 0.8) {
                    self.pickups.push(Pickup { x, y, kind: Drop::Singer(k as u8), t: 1.0 });
                }
            }
            if cargo[k] == n && self.feats.cargo & (1 << k) == 0 {
                if let Some((x, y)) = self.reef_spot_pub(12.0, 0.6) {
                    self.pickups.push(Pickup { x, y, kind: Drop::Cargo(k as u8), t: 1.0 });
                }
            }
        }
    }

    /// A spot far from the hero that you can walk to (reef.rs's search, on any ground).
    fn reef_spot_pub(&mut self, min: f32, room: f32) -> Option<(f32, f32)> {
        let (sx, sy) = (self.p.x, self.p.y);
        for _ in 0..600 {
            let x = self.rng.range(6, self.d.w - 6) as f32 + 0.5;
            let y = self.rng.range(6, self.d.h - 6) as f32 + 0.5;
            if ((x - sx).powi(2) + (y - sy).powi(2)).sqrt() < min || self.d.blocked(x, y, room) {
                continue;
            }
            if self.portals.iter().any(|p| (p.x - x).powi(2) + (p.y - y).powi(2) < 64.0) {
                continue;
            }
            if self.d.path((sx as i32, sy as i32), (x as i32, y as i32), 40_000).is_some() {
                return Some((x, y));
            }
        }
        None
    }

    /// Lays a line of stepping stones from an island's edge out over the sky to a new little island, with a
    /// strongbox on it. Returns false if no edge had room.
    pub(crate) fn carve_hop(&mut self) -> bool {
        let (sx, sy) = (self.p.x as i32, self.p.y as i32);
        let dirs = [(1, 0), (-1, 0), (0, 1), (0, -1)];
        for _ in 0..3000 {
            let x = self.rng.range(4, self.d.w - 4);
            let y = self.rng.range(4, self.d.h - 4);
            let (dx, dy) = dirs[self.rng.range(0, 4) as usize];
            if self.d.get(x, y) != Tile::Floor || self.d.get(x + dx, y + dy) != Tile::Void {
                continue;
            }
            // Open sky all the way out, with room on both sides, and the map edge well beyond.
            let (px, py) = (-dy, dx);
            let clear = (1..=15).all(|k| {
                (-3..=3).all(|j| {
                    let (tx, ty) = (x + dx * k + px * j, y + dy * k + py * j);
                    tx > 1 && ty > 1 && tx < self.d.w - 2 && ty < self.d.h - 2 && self.d.get(tx, ty) == Tile::Void
                })
            });
            if !clear || self.d.path((sx, sy), (x, y), 40_000).is_none() {
                continue;
            }
            for k in 1..=9 {
                for j in 0..=1 {
                    let (tx, ty) = (x + dx * k + px * j, y + dy * k + py * j);
                    self.d.set(tx, ty, Tile::Floor);
                    self.feats.stones.push(Stone { level: self.level, x: tx, y: ty, st: 0, t: 0.0 });
                }
            }
            for k in 10..=14 {
                for j in -2..=2 {
                    self.d.set(x + dx * k + px * j, y + dy * k + py * j, Tile::Floor);
                }
            }
            let (cx, cy) = ((x + dx * 12) as f32 + 0.5, (y + dy * 12) as f32 + 0.5);
            self.pickups.push(Pickup { x: cx, y: cy, kind: Drop::TideChest, t: 1.0 });
            return true;
        }
        false
    }

    fn place_star(&mut self) {
        let Some((x, y)) = self.reef_spot_pub(18.0, 2.4) else { return };
        self.feats.star_spot = (x, y);
        self.feats.star_level = Some(self.level);
        for k in 0..3 {
            let a = k as f32 * 2.1 + 0.4;
            let (mx, my) = (x + a.cos() * 1.5, y + a.sin() * 1.5);
            let (mx, my) = if self.d.blocked(mx, my, 0.4) { (x, y) } else { (mx, my) };
            let mut m = Mob::new(Kind::StarMetal, mx, my, self.tier, &mut self.rng);
            m.max_hp *= 1.0 + self.tier * 0.4;
            m.hp = m.max_hp;
            self.mobs.push(m);
        }
        for k in 0..5 {
            let a = k as f32 * 1.26;
            let (mx, my) = (x + a.cos() * 4.0, y + a.sin() * 4.0);
            if self.d.blocked(mx, my, 0.4) {
                continue;
            }
            let mut m = Mob::new(Kind::Ophanim, mx, my, self.tier * 1.1, &mut self.rng);
            m.promote(Rank::Champion, crate::mobs::M_FAST, Some("STAR-SPAWN".into()));
            self.mobs.push(m);
        }
    }

    fn place_sanctum(&mut self) {
        let Some((x, y)) = self.reef_spot_pub(14.0, 2.0) else { return };
        self.feats.sanctum = 1;
        self.feats.sanctum_spot = (x, y);
        for k in 0..3 {
            let a = k as f32 * 2.1;
            let (mx, my) = (x + a.cos() * 2.0, y + a.sin() * 2.0);
            let (mx, my) = if self.d.blocked(mx, my, 0.4) { (x, y) } else { (mx, my) };
            let mut m = Mob::new(Kind::FallenSeraph, mx, my, self.tier * 1.15, &mut self.rng);
            m.promote(Rank::Elite, crate::mobs::M_STRONG | crate::mobs::M_FIERY, Some(GUARDIAN.into()));
            m.max_hp *= 1.5;
            m.hp = m.max_hp;
            self.mobs.push(m);
        }
        self.say("THE LAST CHOIR'S SONG HAS OPENED THE SANCTUM. ITS GUARDIANS STAND READY".into());
    }

    // ------------------------------------------------------------------ every tick

    pub(crate) fn update_isles(&mut self) {
        if !matches!(self.level, LevelId::Area(5, _)) {
            return;
        }
        self.update_stones();
        if self.level == STORMFIELDS && self.feats.relic < 3 {
            self.update_relic_storm();
        }
        if self.level == HALO && self.feats.sanctum == 1 {
            let left = self.mobs.iter().filter(|m| m.alive() && m.name.as_deref() == Some(GUARDIAN)).count();
            if left == 0 {
                self.open_sanctum();
            }
        }
    }

    fn update_stones(&mut self) {
        let (px, py) = (self.p.x.floor() as i32, self.p.y.floor() as i32);
        let here = self.level;
        let mut fall = false;
        let mut gone = vec![];
        for s in self.feats.stones.iter_mut().filter(|s| s.level == here) {
            match s.st {
                0 if s.x == px && s.y == py => {
                    s.st = 1;
                    s.t = CRUMBLE;
                }
                1 => {
                    s.t -= DT;
                    if s.t <= 0.0 {
                        s.st = 2;
                        s.t = REGROW;
                        gone.push((s.x, s.y));
                    }
                }
                2 => {
                    s.t -= DT;
                    if s.t <= 0.0 {
                        s.st = 0;
                        self.d.set(s.x, s.y, Tile::Floor);
                    }
                }
                _ => {}
            }
        }
        for (x, y) in gone {
            self.d.set(x, y, Tile::Void);
            for _ in 0..4 {
                self.spray_at(x as f32 + 0.5, y as f32 + 0.5, PKind::Smoke, 6.0);
            }
            if (x, y) == (px, py) {
                fall = true;
            }
            for i in 0..self.mobs.len() {
                let m = &self.mobs[i];
                if m.alive() && !m.boss && m.x.floor() as i32 == x && m.y.floor() as i32 == y {
                    self.mob_falls(i);
                }
            }
        }
        if fall {
            self.player_falls();
        }
    }

    fn update_relic_storm(&mut self) {
        // The storm cells drift across the fields, from one patch of solid ground to another.
        if self.feats.cells.is_empty() {
            for _ in 0..3 {
                let (x, y) = self.ground_spot();
                let (tx, ty) = self.ground_spot();
                self.feats.cells.push((x, y, tx, ty));
            }
        }
        for k in 0..self.feats.cells.len() {
            let c = self.feats.cells[k];
            let (dx, dy) = (c.2 - c.0, c.3 - c.1);
            let l = (dx * dx + dy * dy).sqrt();
            if l < 0.5 {
                let (tx, ty) = self.ground_spot();
                (self.feats.cells[k].2, self.feats.cells[k].3) = (tx, ty);
            } else {
                self.feats.cells[k].0 += dx / l * 1.1 * DT;
                self.feats.cells[k].1 += dy / l * 1.1 * DT;
            }
        }
        let (px, py) = (self.p.x, self.p.y);
        let inside = self.feats.cells.iter().any(|c| (c.0 - px).powi(2) + (c.1 - py).powi(2) < CELL_R * CELL_R);
        match self.feats.relic {
            1 => {
                let before = self.feats.charge;
                if inside {
                    self.feats.charge = (self.feats.charge + CHARGE * DT).min(100.0);
                    self.feats.zap_t -= DT;
                    if self.feats.zap_t <= 0.0 {
                        self.feats.zap_t = 1.3;
                        let a = self.rng.f() * std::f32::consts::TAU;
                        let r = self.rng.rf(0.0, 1.6);
                        let burst = 14.0 * self.tier.powf(0.8);
                        self.hazards.push(crate::mobs::Hazard { x: px + a.cos() * r, y: py + a.sin() * r, r: 1.0, warn: 0.7, live: 0.0, dps: 0.0, burst, t: 0.0, fired: false, kind: crate::mobs::HazardKind::Nova });
                    }
                } else {
                    self.feats.charge = (self.feats.charge - DRAIN * DT).max(0.0);
                }
                if (before / 25.0).floor() < (self.feats.charge / 25.0).floor() {
                    let pct = (self.feats.charge / 25.0).floor() as i32 * 25;
                    self.floater(px, py, format!("THE RELIC HUMS: {pct}%"), rgb(0x90d0ff));
                }
                if self.feats.charge >= 100.0 {
                    self.feats.relic = 2;
                    self.save_due = true;
                    self.sfx.push(Sfx::Cast);
                    self.say("THE RELIC BLAZES WITH LIGHTNING! SET IT BACK ON ITS PEDESTAL".into());
                }
            }
            2 => {
                let (rx, ry) = self.feats.relic_spot;
                if (rx - px).powi(2) + (ry - py).powi(2) < 1.6 * 1.6 {
                    self.feats.relic = 3;
                    self.save_due = true;
                    if let Some(u) = crate::items::boss_unique("stormheart") {
                        self.pickups.push(Pickup { x: rx, y: ry + 0.8, kind: Drop::Item(Box::new(u)), t: 0.0 });
                    }
                    self.pickups.push(Pickup { x: rx + 0.8, y: ry, kind: Drop::Gold((80.0 + 40.0 * self.tier) as i32), t: 0.0 });
                    for _ in 0..16 {
                        self.spray_at(rx, ry, PKind::Magic, 10.0);
                    }
                    self.shake = 0.6;
                    self.sfx.push(Sfx::Boom);
                    self.say("THE PEDESTAL DRINKS THE STORM. SOMETHING IS LEFT BEHIND IN THE CRACKLE".into());
                }
            }
            _ => {}
        }
    }

    /// A random open floor tile's middle (anywhere on the map).
    fn ground_spot(&mut self) -> (f32, f32) {
        for _ in 0..400 {
            let x = self.rng.range(4, self.d.w - 4);
            let y = self.rng.range(4, self.d.h - 4);
            if !self.d.blocked(x as f32 + 0.5, y as f32 + 0.5, 1.0) {
                return (x as f32 + 0.5, y as f32 + 0.5);
            }
        }
        (self.p.x, self.p.y)
    }

    /// Picked up the storm relic.
    pub(crate) fn take_relic(&mut self) {
        self.feats.relic = 1;
        self.feats.charge = 0.0;
        self.save_due = true;
        let mut d = Dialog::new(
            "THE STORM RELIC",
            &["A COLD GLASS SPHERE WITH A DEAD SPARK INSIDE. THE PEDESTAL IS CARVED: 'CARRY ME INTO THE STORM. BRING ME HOME FULL.' THE STORM CELLS ROAMING THE FIELDS WILL CHARGE IT, AND THEY WILL BITE."],
        );
        d.refresh_options();
        self.dialog = Some(d);
    }

    /// A lump of star-metal broken open.
    pub(crate) fn star_metal(&mut self, x: f32, y: f32) {
        let ilvl = crate::items::ilvl_for(self.tier) + 3;
        let rar = if self.rng.chance(0.5) { crate::items::Rarity::Rare } else { crate::items::Rarity::Magic };
        self.pickups.push(Pickup { x, y, kind: Drop::Item(Box::new(crate::items::roll(ilvl, rar, &mut self.rng))), t: 0.0 });
        self.pickups.push(Pickup { x: x + 0.6, y, kind: Drop::Gold((30.0 + 20.0 * self.tier) as i32), t: 0.0 });
        for _ in 0..12 {
            self.spray_at(x, y, PKind::Magic, 10.0);
        }
        self.floater(x, y, "STAR-METAL!".into(), rgb(0xc0a0ff));
        let left = self.mobs.iter().filter(|m| m.alive() && m.kind == Kind::StarMetal).count();
        if left == 0 && !self.feats.star_done {
            self.feats.star_done = true;
            self.save_due = true;
            if let Some(u) = crate::items::boss_unique("starfall") {
                self.pickups.push(Pickup { x, y: y + 0.8, kind: Drop::Item(Box::new(u)), t: 0.0 });
            }
            self.say("THE HEART OF THE FALLEN STAR LIES BARE".into());
        }
    }

    /// Found one of the Last Choir's singers.
    pub(crate) fn find_singer(&mut self, k: u8) {
        self.feats.singers |= 1 << k;
        self.save_due = true;
        let (name, line) = SINGERS[k as usize % 3];
        let mut lines = vec![line.to_string()];
        if self.feats.singers == 7 {
            lines.push("(FAR AWAY, THREE VOICES RISE TOGETHER. IN THE HALO ISLES, A DOOR OF LIGHT OPENS.)".into());
        } else {
            lines.push(format!("(SINGERS FOUND: {} OF 3)", self.feats.singers.count_ones()));
        }
        let refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
        let mut d = Dialog::new(name, &refs);
        d.refresh_options();
        self.dialog = Some(d);
        self.sfx.push(Sfx::Cast);
        if self.feats.singers == 7 && self.level == HALO && self.feats.sanctum == 0 {
            self.place_sanctum();
        }
    }

    fn open_sanctum(&mut self) {
        self.feats.sanctum = 2;
        self.save_due = true;
        let (x, y) = self.feats.sanctum_spot;
        let ilvl = crate::items::ilvl_for(self.tier) + 4;
        if let Some(u) = crate::items::boss_unique("hymn") {
            self.pickups.push(Pickup { x, y: y + 0.8, kind: Drop::Item(Box::new(u)), t: 0.0 });
        }
        for dx in [-1.0f32, 1.0] {
            let it = crate::items::roll(ilvl, crate::items::Rarity::Rare, &mut self.rng);
            self.pickups.push(Pickup { x: x + dx, y, kind: Drop::Item(Box::new(it)), t: 0.0 });
        }
        for k in 0..5 {
            let a = k as f32 * 1.3;
            self.pickups.push(Pickup { x: x + a.cos() * 1.4, y: y + a.sin() * 1.4, kind: Drop::Gold((50.0 + 25.0 * self.tier) as i32), t: 0.0 });
        }
        for _ in 0..20 {
            self.spray_at(x, y, PKind::Magic, 12.0);
        }
        self.sfx.push(Sfx::Boom);
        self.say("THE HIDDEN SANCTUM OPENS. THE LAST CHOIR SINGS ONE FINAL HYMN".into());
    }

    /// Picked up one of Bram's crates.
    pub(crate) fn find_cargo(&mut self, k: u8) {
        self.feats.cargo |= 1 << k;
        self.save_due = true;
        let n = self.feats.cargo.count_ones();
        let (x, y) = (self.p.x, self.p.y);
        self.floater(x, y, format!("BRAM'S CARGO ({n} OF 3)"), rgb(0xe0c080));
        if n >= 3 {
            self.side_progress(crate::side::Goal::Cargo);
        }
    }

    /// One of the sky's super uniques fell: it weeps a tear of light, for the Weeping Seraph.
    pub(crate) fn tear_check(&mut self, superu: u8, x: f32, y: f32) {
        let Some(b) = crate::side::TEARS.iter().position(|&s| s as u8 + 1 == superu) else { return };
        self.feats.tears |= 1 << b;
        self.save_due = true;
        self.floater(x, y, "A TEAR OF LIGHT FALLS".into(), rgb(0xfff0a0));
        if self.feats.tears.count_ones() >= 3 {
            self.side_progress(crate::side::Goal::Tears);
        }
    }

    /// The fallen star's crater, on this level.
    pub fn star_here(&self) -> Option<(f32, f32)> {
        (self.feats.star_level == Some(self.level)).then_some(self.feats.star_spot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Game;

    fn at(id: LevelId) -> Game {
        let mut g = Game::new(7, 360);
        g.p.base_hp = 5000.0;
        g.p.recalc();
        g.p.hp = g.p.max_hp;
        g.debug_goto(id);
        g
    }

    #[test]
    fn stepping_stones_crumble_behind_you_and_grow_back() {
        let g = at(DRIFTING);
        assert!(g.feats.stones.iter().any(|s| s.level == DRIFTING), "a hop is laid in the Drifting Isles");
        let mut g = g;
        let s = g.feats.stones.iter().find(|s| s.level == DRIFTING).cloned().unwrap();
        assert_eq!(g.d.get(s.x, s.y), Tile::Floor);
        (g.p.x, g.p.y) = (s.x as f32 + 0.5, s.y as f32 + 0.5);
        g.last_safe = (g.p.x + 50.0, g.p.y);
        for _ in 0..((CRUMBLE / DT) as i32 + 3) {
            g.update_stones();
        }
        assert_eq!(g.d.get(s.x, s.y), Tile::Void, "it crumbled");
        assert!((g.p.x - s.x as f32 - 0.5).abs() > 1.0, "and you fell back to safe ground");
        for _ in 0..((REGROW / DT) as i32 + 3) {
            g.update_stones();
        }
        assert_eq!(g.d.get(s.x, s.y), Tile::Floor, "it grew back");
        assert!(g.pickups.iter().any(|k| matches!(k.kind, Drop::TideChest)), "a strongbox waits on the far island");
    }

    #[test]
    fn the_storm_relic_charges_in_a_cell_and_pays_out_at_its_pedestal() {
        let mut g = at(STORMFIELDS);
        assert!(g.pickups.iter().any(|k| matches!(k.kind, Drop::StormRelic)));
        g.take_relic();
        g.dialog = None;
        g.update_relic_storm();
        let c = g.feats.cells[0];
        for _ in 0..((100.0 / CHARGE / DT) as i32 + 10) {
            (g.p.x, g.p.y) = (g.feats.cells[0].0, g.feats.cells[0].1);
            g.update_relic_storm();
        }
        let _ = c;
        assert_eq!(g.feats.relic, 2, "charged");
        (g.p.x, g.p.y) = g.feats.relic_spot;
        g.update_relic_storm();
        assert_eq!(g.feats.relic, 3);
        assert!(g.pickups.iter().any(|k| matches!(&k.kind, Drop::Item(it) if it.rarity == crate::items::Rarity::Unique)), "the storm's unique");
    }

    #[test]
    fn the_fallen_star_lands_somewhere_and_gives_up_its_heart() {
        let (star, singers, cargo) = rolls(7, 0);
        assert!([1, 2, 3, 5].contains(&star));
        assert!(singers.iter().all(|a| *a != 4) && cargo.iter().all(|a| *a != 4));
        let mut g = at(LevelId::Area(5, star));
        let n = g.mobs.iter().filter(|m| m.kind == Kind::StarMetal).count();
        assert_eq!(n, 3);
        for _ in 0..3 {
            let i = g.mobs.iter().position(|m| m.kind == Kind::StarMetal && m.alive()).unwrap();
            let (x, y) = (g.mobs[i].x, g.mobs[i].y);
            g.mobs[i].hp = 0.0;
            g.mobs[i].state = crate::mobs::MobState::Dead(1.0);
            g.star_metal(x, y);
        }
        assert!(g.feats.star_done);
    }

    #[test]
    fn three_singers_open_the_sanctum_and_its_guardians_guard_it() {
        let mut g = at(HALO);
        g.find_singer(0);
        g.find_singer(1);
        g.dialog = None;
        assert_eq!(g.feats.sanctum, 0);
        g.find_singer(2);
        assert_eq!(g.feats.sanctum, 1, "the guardians stand");
        assert_eq!(g.mobs.iter().filter(|m| m.alive() && m.name.as_deref() == Some(GUARDIAN)).count(), 3);
        for m in g.mobs.iter_mut().filter(|m| m.name.as_deref() == Some(GUARDIAN)) {
            m.hp = 0.0;
            m.state = crate::mobs::MobState::Dead(1.0);
        }
        g.update_isles();
        assert_eq!(g.feats.sanctum, 2);
    }
}

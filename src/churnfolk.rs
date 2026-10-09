//! Act 7's monsters and herald bosses (docs/ACT7_PLAN.md, stage 2). Their tricks run here, a tick at a time,
//! like the sky's (sky.rs), so they can bend the land itself:
//!
//! - **Chaos toads**, in four colours: red ones leap and slam, blue ones spit frost, green ones take your shape,
//!   grey ones are champions with swords. A surge can turn a toad's colour, and reroll a champion's powers.
//! - **The Unmade**: two earlier monsters, half fused (drawn as both at once).
//! - **Chaos knights**: zealots of a law no one else keeps; they charge and bring their swords down.
//! - **Riftmaws**: mouths in the ground. Stand too near and they swallow you, and spit you out somewhere else.
//! - **Chaos matter**: blobs that split in two when they die, twice.
//!
//! The heralds hold the three **Anchor Keys**:
//! - **The Architect of Nothing** (the Unfinished Cathedral, whose floors surge like the land outside) raises
//!   walls around you and throws bricks.
//! - **Grumbleguts the Many-Hued** (the Toad King's Warren) changes colour, and so how it fights, at each third
//!   of its life: red leaps, blue freezes, green breeds.
//! - **The Mirror Abbot** (the Hall of Mirrors) fights with your own class's attacks, and sends copies of you.
use crate::dungeon::Tile;
use crate::game::{Game, PKind, Sfx, DT};
use crate::gfx::rgb;
use crate::mobs::{Hazard, HazardKind, Kind, Mob, MobState, Rank, Shot, ShotKind};
use crate::world::LevelId;

/// Toad colours (`form`): red, blue, green, grey; 4 is a green toad wearing your shape.
pub const TOAD_TINTS: [u32; 5] = [0xc03020, 0x3060d0, 0x40a040, 0x8a8a90, 0x40a040];
/// The Unmade's halves (art sheets), by `form`.
pub const UNMADE: [(&str, &str); 4] = [("ghoul", "ice_troll"), ("werewolf", "boiler_brute"), ("merrow", "fallen_seraph"), ("banshee", "sentinel")];

/// Is this an Act 7 level (an area, Stillhold, or one of its dungeons)?
pub fn churn_level(id: LevelId) -> bool {
    id.act() == 6
}

impl Game {
    /// New monsters in an Act 7 level get their colours and halves; riftmaws open in the Tangle and the Mire.
    pub(crate) fn churnfolk_enter(&mut self) {
        if !churn_level(self.level) {
            return;
        }
        for i in 0..self.mobs.len() {
            let m = &self.mobs[i];
            if m.cue != 0 {
                continue;
            }
            let k = m.kind;
            let r = self.rng.range(0, 4) as u8;
            let m = &mut self.mobs[i];
            match k {
                Kind::ChaosToad => {
                    m.cue = 1;
                    m.form = r;
                    if r == 3 && m.rank == Rank::Normal {
                        m.promote(Rank::Champion, crate::mobs::M_STRONG, None);
                    }
                }
                Kind::Unmade => {
                    m.cue = 1;
                    m.form = r;
                }
                _ => {}
            }
        }
        if let LevelId::Area(6, n @ (3 | 4)) = self.level {
            if !self.feats.maws_placed.contains(&self.level) {
                self.feats.maws_placed.push(self.level);
                let want = if n == 4 { 4 } else { 3 };
                let mut placed = 0;
                for _ in 0..600 {
                    if placed >= want {
                        break;
                    }
                    let x = self.rng.range(8, self.d.w - 8) as f32 + 0.5;
                    let y = self.rng.range(8, self.d.h - 8) as f32 + 0.5;
                    let far = (x - self.p.x).powi(2) + (y - self.p.y).powi(2) > 400.0;
                    if !far || self.d.blocked(x, y, 0.9) || self.portals.iter().any(|p| (p.x - x).powi(2) + (p.y - y).powi(2) < 64.0) {
                        continue;
                    }
                    let mut m = Mob::new(Kind::Riftmaw, x, y, self.tier, &mut self.rng);
                    m.cue = 1;
                    self.mobs.push(m);
                    placed += 1;
                }
            }
        }
    }

    /// A surge stirs the monsters too: toads change colour, and champions' powers are rerolled.
    pub(crate) fn surge_stirs(&mut self, (px, py): (f32, f32)) {
        for i in 0..self.mobs.len() {
            let m = &self.mobs[i];
            if !m.alive() || m.boss || (m.x - px).powi(2) + (m.y - py).powi(2) > 400.0 {
                continue;
            }
            let (kind, rank) = (m.kind, m.rank);
            let r = self.rng.range(0, 4) as u8;
            let bits = [crate::mobs::M_FAST, crate::mobs::M_STRONG, crate::mobs::M_STONE, crate::mobs::M_FIERY, crate::mobs::M_VAMPIRE, crate::mobs::M_MANABURN];
            let a = bits[self.rng.range(0, 6) as usize];
            let b = bits[self.rng.range(0, 6) as usize];
            let m = &mut self.mobs[i];
            if kind == Kind::ChaosToad && m.form < 4 {
                m.form = r;
            }
            if matches!(rank, Rank::Champion | Rank::Elite) {
                m.mods = if rank == Rank::Elite { a | b } else { a };
            }
        }
    }

    // ------------------------------------------------------------------ every tick

    pub(crate) fn update_churnfolk(&mut self) {
        if !churn_level(self.level) {
            return;
        }
        // Walls the Architect raised fall again.
        if !self.feats.temp_walls.is_empty() {
            let mut keep = vec![];
            for (x, y, t) in std::mem::take(&mut self.feats.temp_walls) {
                if t - DT <= 0.0 {
                    if self.d.get(x, y) == Tile::Wall {
                        self.d.set(x, y, Tile::Floor);
                    }
                    self.spray_at(x as f32 + 0.5, y as f32 + 0.5, PKind::Smoke, 4.0);
                } else {
                    keep.push((x, y, t - DT));
                }
            }
            self.feats.temp_walls = keep;
        }
        let (px, py) = (self.p.x, self.p.y);
        for i in 0..self.mobs.len() {
            let m = &self.mobs[i];
            if !m.alive() || m.charm > 0.0 || !matches!(m.kind, Kind::ChaosToad | Kind::ChaosKnight | Kind::Riftmaw | Kind::Architect | Kind::Grumbleguts | Kind::MirrorAbbot) {
                continue;
            }
            let (kind, x, y) = (m.kind, m.x, m.y);
            let dist = ((px - x).powi(2) + (py - y).powi(2)).sqrt();
            if !m.boss {
                self.mobs[i].special -= DT;
            }
            if self.mobs[i].special > 0.0 && !matches!(kind, Kind::Grumbleguts) {
                continue;
            }
            match kind {
                Kind::ChaosToad => self.toad_trick(i, dist),
                Kind::ChaosKnight if dist < 7.0 && dist > 2.0 => {
                    self.mobs[i].special = 5.0;
                    self.leap_at(i, 1.3);
                    let burst = 16.0 * self.mobs[i].tier.powf(0.8);
                    self.hazards.push(Hazard { x: px, y: py, r: 1.8, warn: 0.6, live: 0.0, dps: 0.0, burst, t: 0.0, fired: false, kind: HazardKind::Quake });
                    self.floater(x, y, "BY THE LAW!".into(), rgb(0xc0a0ff));
                }
                Kind::Riftmaw => self.riftmaw_trick(i, dist),
                Kind::Architect if dist < 14.0 => self.architect_trick(i),
                Kind::Grumbleguts if dist < 14.0 => self.grumbleguts_trick(i, dist),
                Kind::MirrorAbbot if dist < 14.0 => self.mirror_trick(i),
                _ => {}
            }
        }
    }

    /// Jump mob `i` to `gap` tiles short of you (if it can land there).
    fn leap_at(&mut self, i: usize, gap: f32) {
        let (x, y) = (self.mobs[i].x, self.mobs[i].y);
        let (px, py) = (self.p.x, self.p.y);
        let d = ((px - x).powi(2) + (py - y).powi(2)).sqrt().max(0.01);
        let (nx, ny) = (px - (px - x) / d * gap, py - (py - y) / d * gap);
        if !self.d.blocked(nx, ny, self.mobs[i].r) {
            (self.mobs[i].x, self.mobs[i].y) = (nx, ny);
            for _ in 0..6 {
                self.spray_at(nx, ny, PKind::Smoke, 6.0);
            }
        }
    }

    fn toad_trick(&mut self, i: usize, dist: f32) {
        let (x, y, form, tier) = (self.mobs[i].x, self.mobs[i].y, self.mobs[i].form, self.mobs[i].tier);
        let dmg = 11.0 * tier.powf(0.8);
        match form {
            // Red: a great leap, and a slam where it lands.
            0 if dist < 8.0 && dist > 1.8 => {
                self.mobs[i].special = 4.5;
                self.leap_at(i, 1.0);
                let (lx, ly) = (self.mobs[i].x, self.mobs[i].y);
                self.hazards.push(Hazard { x: lx, y: ly, r: 1.9, warn: 0.5, live: 0.0, dps: 0.0, burst: dmg * 1.3, t: 0.0, fired: false, kind: HazardKind::Quake });
            }
            // Blue: a spray of frost.
            1 if dist < 9.0 => {
                self.mobs[i].special = 3.2;
                let a0 = (self.p.y - y).atan2(self.p.x - x);
                for k in -1..=1 {
                    let a = a0 + k as f32 * 0.2;
                    self.shots.push(Shot { x, y, vx: a.cos() * 6.5, vy: a.sin() * 6.5, life: 2.0, dmg, kind: ShotKind::Ice });
                }
            }
            // Green: it takes your shape.
            2 if dist < 7.0 => {
                self.mobs[i].form = 4;
                self.mobs[i].special = 99.0;
                for _ in 0..10 {
                    self.spray_at(x, y, PKind::Spore, 8.0);
                }
                self.floater(x, y, "IT TAKES YOUR SHAPE!".into(), rgb(0x80e080));
            }
            _ => self.mobs[i].special = 1.0,
        }
    }

    fn riftmaw_trick(&mut self, i: usize, dist: f32) {
        let (x, y) = (self.mobs[i].x, self.mobs[i].y);
        if dist < 1.4 {
            // Swallowed, and spat out somewhere else on the map.
            self.mobs[i].special = 8.0;
            let hit = self.p.max_hp * 0.12;
            self.hurt_player(hit);
            let (sx, sy) = (self.start.0 as i32, self.start.1 as i32);
            for _ in 0..400 {
                let nx = self.rng.range(6, self.d.w - 6) as f32 + 0.5;
                let ny = self.rng.range(6, self.d.h - 6) as f32 + 0.5;
                if (nx - x).powi(2) + (ny - y).powi(2) < 400.0 || self.d.blocked(nx, ny, 0.6) {
                    continue;
                }
                if self.d.path((sx, sy), (nx as i32, ny as i32), 30_000).is_some() {
                    (self.p.x, self.p.y) = (nx, ny);
                    self.p.path.clear();
                    break;
                }
            }
            self.sfx.push(Sfx::Descend);
            self.shake = self.shake.max(0.5);
            self.floater(self.p.x, self.p.y, "SWALLOWED... AND SPAT OUT".into(), rgb(0xc080ff));
        } else if dist < 4.5 {
            self.mobs[i].special = 3.0;
            self.pull = (x, y, 0.9);
            self.floater(x, y, "IT DRINKS IN THE AIR".into(), rgb(0x9060c0));
        } else {
            self.mobs[i].special = 0.5;
        }
    }

    fn architect_trick(&mut self, i: usize) {
        let m = &self.mobs[i];
        let (x, y, tier, enraged) = (m.x, m.y, m.tier, m.enraged);
        let (px, py) = (self.p.x, self.p.y);
        // A ring of new walls around you, with two gaps: get out before it closes on its own.
        self.mobs[i].special = if enraged { 6.0 } else { 8.5 };
        let gap = self.rng.f() * std::f32::consts::TAU;
        for k in 0..28 {
            let a = k as f32 / 28.0 * std::f32::consts::TAU;
            let off = (a - gap).rem_euclid(std::f32::consts::TAU);
            if off < 0.5 || (off - std::f32::consts::PI).abs() < 0.5 {
                continue;
            }
            let (wx, wy) = ((px + a.cos() * 3.2) as i32, (py + a.sin() * 3.2) as i32);
            if self.d.get(wx, wy) == Tile::Floor && !self.mobs.iter().any(|m| m.alive() && m.x as i32 == wx && m.y as i32 == wy) {
                self.d.set(wx, wy, Tile::Wall);
                self.feats.temp_walls.push((wx, wy, 6.0));
            }
        }
        self.floater(x, y, "I BUILD. YOU STAY.".into(), rgb(0xd0c0a0));
        self.sfx.push(Sfx::Boom);
        // And a volley of loose bricks.
        let a0 = (py - y).atan2(px - x);
        let dmg = 14.0 * tier.powf(0.8);
        for k in -2..=2 {
            let a = a0 + k as f32 * 0.2;
            self.shots.push(Shot { x, y, vx: a.cos() * 6.0, vy: a.sin() * 6.0, life: 2.2, dmg, kind: ShotKind::Boulder });
        }
        if self.mobs.iter().filter(|m| m.alive() && m.kind == Kind::ChaosBlob).count() < 4 {
            let mut m = Mob::new(Kind::ChaosBlob, x + 1.5, y, tier * 0.9, &mut self.rng);
            m.state = MobState::Chase;
            m.cue = 1;
            self.mobs.push(m);
        }
    }

    fn grumbleguts_trick(&mut self, i: usize, dist: f32) {
        let m = &self.mobs[i];
        let (x, y, tier) = (m.x, m.y, m.tier);
        let frac = m.hp / m.max_hp;
        let colour = if frac > 0.667 { 0 } else if frac > 0.333 { 1 } else { 2 };
        if colour != self.mobs[i].form {
            self.mobs[i].form = colour;
            self.mobs[i].special = 1.5;
            let word = ["RED!", "BLUE! COLD! COLD!", "GREEN! BREED, MY BROOD!"][colour as usize];
            self.floater(x, y, word.into(), rgb(TOAD_TINTS[colour as usize]));
            self.shake = self.shake.max(0.6);
            return;
        }
        if self.mobs[i].special > 0.0 {
            return;
        }
        let dmg = 16.0 * tier.powf(0.8);
        let (px, py) = (self.p.x, self.p.y);
        match colour {
            0 => {
                self.mobs[i].special = 3.6;
                if dist > 2.0 {
                    self.leap_at(i, 1.4);
                }
                let (lx, ly) = (self.mobs[i].x, self.mobs[i].y);
                self.hazards.push(Hazard { x: lx, y: ly, r: 2.8, warn: 0.7, live: 0.0, dps: 0.0, burst: dmg * 1.5, t: 0.0, fired: false, kind: HazardKind::Quake });
            }
            1 => {
                self.mobs[i].special = 3.2;
                for k in 0..10 {
                    let a = k as f32 / 10.0 * std::f32::consts::TAU;
                    self.shots.push(Shot { x, y, vx: a.cos() * 6.0, vy: a.sin() * 6.0, life: 2.2, dmg, kind: ShotKind::Ice });
                }
                self.hazards.push(Hazard { x: px, y: py, r: 1.6, warn: 0.8, live: 3.0, dps: dmg * 0.4, burst: 0.0, t: 0.0, fired: false, kind: HazardKind::Frost });
            }
            _ => {
                self.mobs[i].special = 6.0;
                self.hazards.push(Hazard { x: px, y: py, r: 1.8, warn: 0.6, live: 4.0, dps: dmg * 0.4, burst: 0.0, t: 0.0, fired: false, kind: HazardKind::Poison });
                if self.mobs.iter().filter(|m| m.alive() && m.kind == Kind::ChaosToad).count() < 5 {
                    for k in 0..2 {
                        let a = k as f32 * 3.1 + 0.4;
                        let mut t = Mob::new(Kind::ChaosToad, x + a.cos() * 1.8, y + a.sin() * 1.8, tier * 0.85, &mut self.rng);
                        t.form = 2;
                        t.cue = 1;
                        t.state = MobState::Chase;
                        self.mobs.push(t);
                    }
                }
            }
        }
    }

    fn mirror_trick(&mut self, i: usize) {
        let class = crate::skills::ALL_CLASSES.iter().position(|c| *c == self.p.skills.class).unwrap_or(0) as u8;
        let (x, y, tier, enraged) = (self.mobs[i].x, self.mobs[i].y, self.mobs[i].tier, self.mobs[i].enraged);
        self.mobs[i].form = class;
        self.mobs[i].special = if enraged { 1.8 } else { 2.6 };
        self.feats.mirror_moves += 1;
        let third = self.feats.mirror_moves % 3 == 0;
        self.hero_attack(i, class, 11.0 * tier.powf(0.8), third);
        // Every so often, two of you step out of the glass.
        if self.feats.mirror_moves % 4 == 0 && self.mobs.iter().filter(|m| m.alive() && m.kind == Kind::MirrorImage).count() < 3 {
            for k in 0..2 {
                let a = k as f32 * 3.1 + self.rng.f();
                let (mx, my) = (x + a.cos() * 2.0, y + a.sin() * 2.0);
                if self.d.blocked(mx, my, 0.35) {
                    continue;
                }
                let mut m = Mob::new(Kind::MirrorImage, mx, my, tier * 0.8, &mut self.rng);
                m.state = MobState::Chase;
                m.cue = 1;
                self.mobs.push(m);
            }
            self.floater(x, y, "YOUR OWN HAND, TURNED AGAINST YOU".into(), rgb(0xd0d8ff));
        }
    }

    /// Chaos matter splits when it dies (twice).
    pub(crate) fn blob_splits(&mut self, i: usize) {
        let (x, y, form, tier) = (self.mobs[i].x, self.mobs[i].y, self.mobs[i].form, self.mobs[i].tier);
        if form >= 2 {
            return;
        }
        for k in 0..2 {
            let a = k as f32 * 3.1 + self.rng.f();
            let (mx, my) = (x + a.cos() * 0.8, y + a.sin() * 0.8);
            let (mx, my) = if self.d.blocked(mx, my, 0.3) { (x, y) } else { (mx, my) };
            let mut m = Mob::new(Kind::ChaosBlob, mx, my, tier, &mut self.rng);
            m.form = form + 1;
            m.max_hp *= 0.5f32.powi(form as i32 + 1);
            m.hp = m.max_hp;
            m.xp *= 0.4;
            m.cue = 1;
            m.state = MobState::Chase;
            self.mobs.push(m);
        }
    }
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
    fn chaos_matter_splits_twice() {
        let mut g = at(LevelId::Area(6, 3));
        g.mobs.clear();
        let mut m = Mob::new(Kind::ChaosBlob, g.p.x + 4.0, g.p.y, g.tier, &mut g.rng);
        m.cue = 1;
        g.mobs.push(m);
        for _ in 0..3 {
            let alive: Vec<usize> = (0..g.mobs.len()).filter(|&i| g.mobs[i].alive()).collect();
            for i in alive {
                g.kill(i);
            }
        }
        assert_eq!(g.mobs.iter().filter(|m| m.kind == Kind::ChaosBlob).count(), 7, "1 big, 2 middling, 4 small");
        assert!(g.mobs.iter().all(|m| !m.alive()));
    }

    #[test]
    fn a_riftmaw_swallows_you_and_spits_you_out_elsewhere() {
        let mut g = at(LevelId::Area(6, 4));
        let i = g.mobs.iter().position(|m| m.kind == Kind::Riftmaw).expect("riftmaws in the Mire");
        (g.p.x, g.p.y) = (g.mobs[i].x + 0.8, g.mobs[i].y);
        g.mobs[i].special = 0.0;
        let before = (g.p.x, g.p.y);
        g.update_churnfolk();
        assert!((g.p.x - before.0).powi(2) + (g.p.y - before.1).powi(2) > 100.0, "spat out far away");
    }

    #[test]
    fn grumbleguts_changes_colour_by_thirds() {
        let mut g = at(LevelId::Dungeon(crate::world::WARREN, 1));
        let i = g.mobs.iter().position(|m| m.kind == Kind::Grumbleguts).expect("the toad king");
        (g.p.x, g.p.y) = (g.mobs[i].x + 3.0, g.mobs[i].y);
        g.mobs[i].form = 0;
        g.mobs[i].hp = g.mobs[i].max_hp * 0.5;
        g.update_churnfolk();
        assert_eq!(g.mobs[i].form, 1, "blue at half");
        g.mobs[i].hp = g.mobs[i].max_hp * 0.2;
        g.update_churnfolk();
        assert_eq!(g.mobs[i].form, 2, "green at the end");
    }

    #[test]
    fn the_architect_walls_you_in_for_a_while() {
        let mut g = at(LevelId::Dungeon(crate::world::CATHEDRAL, 1));
        let i = g.mobs.iter().position(|m| m.kind == Kind::Architect).expect("the architect");
        (g.p.x, g.p.y) = (g.mobs[i].x + 5.0, g.mobs[i].y);
        if g.d.blocked(g.p.x, g.p.y, 0.4) {
            (g.p.x, g.p.y) = (g.mobs[i].x, g.mobs[i].y + 5.0);
        }
        g.mobs[i].special = 0.0;
        g.update_churnfolk();
        assert!(!g.feats.temp_walls.is_empty(), "walls rise");
        for _ in 0..(7.0 / DT) as i32 {
            g.mobs[i].special = 99.0;
            g.update_churnfolk();
        }
        assert!(g.feats.temp_walls.is_empty(), "and fall again");
    }
}

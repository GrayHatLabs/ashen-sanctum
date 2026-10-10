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
//!
//! **Ylgrath the Unshaped** waits on the Unshaped Throne, in the Eye of the Churn. He wears the old bosses' shapes
//! one after another (and fights as each), then, near the end, his own: a storm of everything at once. Beaten
//! without the **Stillpoint** he flees: a fair reward, a gloat that lets slip what he fears, and his throne seals
//! until you fetch the Stillpoint from the heart of the Clockmaker's clock (Act 4). With it he dies for good.
use crate::dungeon::Tile;
use crate::game::{Drop, Game, PKind, Pickup, Sfx, DT};
use crate::gfx::rgb;
use crate::mobs::{Hazard, HazardKind, Kind, Mob, MobState, Rank, Shot, ShotKind};
use crate::world::LevelId;

/// Toad colours (`form`): red, blue, green, grey; 4 is a green toad wearing your shape.
pub const TOAD_TINTS: [u32; 5] = [0xc03020, 0x3060d0, 0x40a040, 0x8a8a90, 0x40a040];
/// The Unmade's halves (art sheets), by `form`.
pub const UNMADE: [(&str, &str); 4] = [("ghoul", "ice_troll"), ("werewolf", "boiler_brute"), ("merrow", "fallen_seraph"), ("banshee", "sentinel")];

/// The old bosses' shapes Ylgrath wears (art sheet, and what he shouts), by `form`; past the end is his own.
pub const YLGRATH_FORMS: [(&str, &str); 6] = [
    ("boss_ashking", "I WEAR YOUR FIRST KING"),
    ("boss_dragon", "COLD, NOW. LIKE THE WYRM"),
    ("boss_vardak", "A LITTLE BLOOD, LIKE THE COUNT"),
    ("boss_clockmaker", "TICK. TOCK. I KNOW THIS ONE"),
    ("boss_leviathan", "FROM THE DEEP, LITTLE THING"),
    ("boss_solanthos", "AND NOW I AM THE SUN"),
];

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
                    // A named toad keeps its own colour.
                    if let Some(c) = crate::side::CHURN_TOADS.iter().position(|&s| s + 1 == m.superu as usize) {
                        m.form = c as u8;
                    } else if r == 3 && m.rank == Rank::Normal {
                        m.promote(Rank::Champion, crate::mobs::M_STRONG, None);
                    }
                }
                // The Other You is as tough as you are.
                Kind::MirrorImage if m.superu as usize == crate::side::OTHER_YOU + 1 => {
                    m.cue = 1;
                    m.max_hp *= 4.0;
                    m.hp = m.max_hp;
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
            if !m.alive() || m.charm > 0.0 || !matches!(m.kind, Kind::ChaosToad | Kind::ChaosKnight | Kind::Riftmaw | Kind::Architect | Kind::Grumbleguts | Kind::MirrorAbbot | Kind::Ylgrath | Kind::DiceSaint) {
                continue;
            }
            let (kind, x, y) = (m.kind, m.x, m.y);
            let dist = ((px - x).powi(2) + (py - y).powi(2)).sqrt();
            if !m.boss {
                self.mobs[i].special -= DT;
            }
            if self.mobs[i].special > 0.0 && !matches!(kind, Kind::Grumbleguts | Kind::Ylgrath) {
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
                Kind::Ylgrath if dist < 16.0 => self.ylgrath_trick(i, dist),
                Kind::DiceSaint if dist < 14.0 => self.dice_trick(i, dist),
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

    /// Ylgrath: a new old shape every so often (and at once when hurt badly), fighting as that boss did; his own
    /// shape in his last fifth, everything at once. With the Stillpoint near, he can't change at all.
    fn ylgrath_trick(&mut self, i: usize, dist: f32) {
        let m = &self.mobs[i];
        let (x, y, tier, enraged) = (m.x, m.y, m.tier, m.enraged);
        let frac = m.hp / m.max_hp;
        let n = YLGRATH_FORMS.len() as u8;
        let still = self.quest.stillpoint >= 2;
        if frac < 0.2 && self.mobs[i].form < n {
            // His own shape at last.
            self.mobs[i].form = n;
            self.mobs[i].invuln = 1.5;
            self.mobs[i].special = 2.0;
            self.shake = self.shake.max(1.0);
            self.sfx.push(Sfx::Boom);
            let word = if still { "THE QUIET... WHAT IS THAT QUIET?" } else { "ENOUGH MASKS. SEE ME!" };
            self.floater(x, y, word.into(), rgb(0xd0a0ff));
            return;
        }
        if self.mobs[i].form < n && self.mobs[i].special2 <= 0.0 {
            // A new shape: never the same one twice running. (Stillness makes him slower to change.)
            self.mobs[i].special2 = if still { 16.0 } else if enraged { 9.0 } else { 12.0 };
            let old = self.mobs[i].form;
            let mut f = self.rng.range(0, n as i32) as u8;
            if f == old {
                f = (f + 1) % n;
            }
            self.mobs[i].form = f;
            self.mobs[i].invuln = 0.8;
            self.mobs[i].special = 1.6;
            for _ in 0..16 {
                self.spray_at(x, y, PKind::Smoke, 10.0);
            }
            self.floater(x, y, YLGRATH_FORMS[f as usize].1.into(), rgb(0xc090ff));
            return;
        }
        if self.mobs[i].special > 0.0 {
            return;
        }
        let dmg = 17.0 * tier.powf(0.8);
        let (px, py) = (self.p.x, self.p.y);
        let a0 = (py - y).atan2(px - x);
        let form = if self.mobs[i].form < n { self.mobs[i].form } else { self.rng.range(0, n as i32) as u8 };
        let quick = if enraged { 0.75 } else { 1.0 };
        match form {
            // The Ash King: an ash nova under you, and a fan of ash.
            0 => {
                self.mobs[i].special = 3.2 * quick;
                self.hazards.push(Hazard { x: px, y: py, r: 2.2, warn: 0.9, live: 0.0, dps: 0.0, burst: dmg * 1.6, t: 0.0, fired: false, kind: HazardKind::Nova });
                for k in -2..=2 {
                    let a = a0 + k as f32 * 0.18;
                    self.shots.push(Shot { x, y, vx: a.cos() * 6.5, vy: a.sin() * 6.5, life: 2.2, dmg, kind: ShotKind::Ash });
                }
            }
            // The Rime Wyrm: freezing ground, and a ring of ice.
            1 => {
                self.mobs[i].special = 3.4 * quick;
                self.hazards.push(Hazard { x: px, y: py, r: 1.8, warn: 0.7, live: 3.5, dps: dmg * 0.45, burst: 0.0, t: 0.0, fired: false, kind: HazardKind::Frost });
                for k in 0..12 {
                    let a = k as f32 / 12.0 * std::f32::consts::TAU;
                    self.shots.push(Shot { x, y, vx: a.cos() * 5.5, vy: a.sin() * 5.5, life: 2.4, dmg: dmg * 0.8, kind: ShotKind::Ice });
                }
            }
            // Count Vardak: blood bolts, and he steps through the air to you.
            2 => {
                self.mobs[i].special = 3.0 * quick;
                if dist > 3.0 {
                    self.leap_at(i, 1.8);
                }
                for k in -1..=1 {
                    let a = a0 + k as f32 * 0.25;
                    self.shots.push(Shot { x, y, vx: a.cos() * 7.5, vy: a.sin() * 7.5, life: 2.0, dmg: dmg * 1.1, kind: ShotKind::Blood });
                }
            }
            // The Clockmaker: a spiral of cogs.
            3 => {
                self.mobs[i].special = 3.0 * quick;
                let turn = self.rng.f() * std::f32::consts::TAU;
                for k in 0..10 {
                    let a = turn + k as f32 / 10.0 * std::f32::consts::TAU;
                    self.shots.push(Shot { x, y, vx: a.cos() * 6.0, vy: a.sin() * 6.0, life: 2.4, dmg, kind: ShotKind::Gear });
                }
            }
            // The Leviathan: the deep drags you in, and a pressure beam.
            4 => {
                self.mobs[i].special = 3.6 * quick;
                self.pull = (x, y, 0.8);
                self.shots.push(Shot { x, y, vx: a0.cos() * 9.0, vy: a0.sin() * 9.0, life: 1.8, dmg: dmg * 1.6, kind: ShotKind::Tide });
            }
            // Solanthos: falling stars, and spears of light.
            _ => {
                self.mobs[i].special = 3.4 * quick;
                for k in 0..3 {
                    let a = self.rng.f() * std::f32::consts::TAU;
                    let r = if k == 0 { 0.0 } else { self.rng.rf(1.2, 2.6) };
                    self.hazards.push(Hazard { x: px + a.cos() * r, y: py + a.sin() * r, r: 1.0, warn: 1.0, live: 0.0, dps: 0.0, burst: dmg * 1.4, t: 0.0, fired: false, kind: HazardKind::Nova });
                }
                for k in -1..=1 {
                    let a = a0 + k as f32 * 0.12;
                    self.shots.push(Shot { x, y, vx: a.cos() * 8.0, vy: a.sin() * 8.0, life: 2.0, dmg, kind: ShotKind::Light });
                }
            }
        }
        // In his own shape, the storm comes quicker, and spits out chaos matter.
        if self.mobs[i].form >= n {
            self.mobs[i].special *= 0.6;
            if self.rng.chance(0.3) && self.mobs.iter().filter(|m| m.alive() && m.kind == Kind::ChaosBlob).count() < 4 {
                let mut b = Mob::new(Kind::ChaosBlob, x + 1.2, y + 0.6, tier * 0.9, &mut self.rng);
                b.state = MobState::Chase;
                b.cue = 1;
                self.mobs.push(b);
            }
        }
    }

    /// Ylgrath beaten without the Stillpoint (kill() asks first): he comes apart and flees, leaving a fair
    /// reward (a rare, gold, a page of the Clockmaker's notebook; no unique), and his throne seals until you
    /// carry the Stillpoint. Returns whether he fled.
    pub(crate) fn ylgrath_flees(&mut self, i: usize) -> bool {
        if self.mobs[i].kind != Kind::Ylgrath || self.quest.stillpoint >= 2 || self.in_rift() {
            return false;
        }
        let (x, y) = (self.mobs[i].x, self.mobs[i].y);
        let m = &mut self.mobs[i];
        m.hp = 0.0;
        m.state = MobState::Dead(0.0);
        m.form = YLGRATH_FORMS.len() as u8;
        self.stats.bosses += 1;
        self.gain_xp(self.mobs[i].xp * 0.5);
        self.shake = 1.0;
        self.sfx.push(Sfx::Descend);
        for _ in 0..50 {
            self.spray_at(x, y, PKind::Smoke, 30.0);
        }
        // His spawn scatters with him.
        for m in self.mobs.iter_mut().filter(|m| m.alive() && matches!(m.kind, Kind::ChaosBlob)) {
            m.hp = 0.0;
            m.state = MobState::Dead(0.0);
        }
        let ilvl = crate::items::ilvl_for(self.tier) + 2;
        let it = crate::items::roll(ilvl, crate::items::Rarity::Rare, &mut self.rng);
        self.pickups.push(Pickup { x: x + 0.7, y, kind: Drop::Item(Box::new(it)), t: 0.0 });
        for k in 0..3 {
            self.pickups.push(Pickup { x: x + k as f32 * 0.5 - 0.5, y: y + 0.6, kind: Drop::Gold((40.0 + 12.0 * self.tier) as i32), t: 0.0 });
        }
        if let Some(page) = crate::side::LORE.iter().position(|l| l.1 == "A PAGE OF THE CLOCKMAKER'S NOTEBOOK") {
            self.pickups.push(Pickup { x: x - 0.7, y, kind: Drop::Page(page as u8), t: 0.0 });
        }
        // A way home where he stood.
        let spot = crate::world::nearest_open_on(&self.d, x, y).map_or((self.p.x, self.p.y), |(tx, ty)| (tx as f32 + 0.5, ty as f32 + 0.5));
        self.portals.push(crate::world::Portal { x: spot.0, y: spot.1, kind: crate::world::PortalKind::TownPortal });
        self.quest.stillpoint = 1;
        self.save_due = true;
        self.dialog = Some(crate::story::Dialog::new("YLGRATH THE UNSHAPED", &crate::story::YLGRATH_FLEES));
        self.say("YLGRATH FLEES. HIS THRONE CLOSES BEHIND HIM".into());
        true
    }

    /// The Stillpoint waits on the last floor of the Heart of the Clock, once Ylgrath has let slip where it is:
    /// as far from the stair as you can walk, with some of his spawn sent to guard it.
    pub(crate) fn stillpoint_enter(&mut self) {
        // Back on his throne with the Stillpoint: he's there, even if this floor was kept from when he fled.
        let throne = LevelId::Dungeon(crate::world::EYE, crate::world::DUNGEONS[crate::world::EYE].floors - 1);
        if self.level == throne && self.quest.stillpoint == 2 && self.quest.stage7 < 3 && !self.mobs.iter().any(|m| m.kind == Kind::Ylgrath && m.alive()) {
            if let Some(m) = self.mobs.iter_mut().find(|m| m.kind == Kind::Ylgrath) {
                m.hp = m.max_hp;
                m.state = MobState::Idle;
                m.form = 0;
                m.special = 2.0;
                m.special2 = 6.0;
                m.enraged = false;
            }
        }
        let heart = crate::world::HEART;
        if self.quest.stillpoint != 1 || self.level != LevelId::Dungeon(heart, crate::world::DUNGEONS[heart].floors - 1) {
            return;
        }
        if self.pickups.iter().any(|k| matches!(k.kind, Drop::Stillpoint)) {
            return;
        }
        let Some((x, y)) = crate::world::farthest_walk(&self.d, (self.p.x as i32, self.p.y as i32)) else { return };
        let (x, y) = (x as f32 + 0.5, y as f32 + 0.5);
        self.pickups.push(Pickup { x, y, kind: Drop::Stillpoint, t: 1.0 });
        // As tough as the Churn's own (the Heart's tier is about two thirds of the throne's).
        let tier = self.tier * 1.5;
        for k in 0..4 {
            let a = k as f32 * 1.57 + 0.5;
            let (mx, my) = (x + a.cos() * 2.5, y + a.sin() * 2.5);
            if self.d.blocked(mx, my, 0.4) {
                continue;
            }
            let mut m = Mob::new(Kind::Unmade, mx, my, tier, &mut self.rng);
            m.form = k as u8 % 4;
            m.cue = 1;
            self.mobs.push(m);
        }
    }

    /// Picking up the Stillpoint: Ylgrath's throne opens again, rebuilt (he's back on it).
    pub(crate) fn take_stillpoint(&mut self, x: f32, y: f32) {
        self.quest.stillpoint = 2;
        self.parked.retain(|id, _| !matches!(*id, LevelId::Dungeon(k, _) if k == crate::world::EYE));
        self.sfx.push(Sfx::Descend);
        self.floater(x, y, "THE STILLPOINT".into(), rgb(0xb0d8ff));
        self.say("THE STILLPOINT. THE WORLD GOES QUIET AROUND IT. YLGRATH'S THRONE WILL OPEN FOR YOU NOW".into());
        self.save_due = true;
    }

    /// For the survey bot (snapshot.rs): it can't cross acts on its own, so the Stillpoint is handed over.
    pub fn debug_grant_stillpoint(&mut self) {
        let (x, y) = (self.p.x, self.p.y);
        self.take_stillpoint(x, y);
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
    fn ylgrath_wears_the_old_bosses_then_his_own_shape() {
        let mut g = at(LevelId::Dungeon(crate::world::EYE, crate::world::DUNGEONS[crate::world::EYE].floors - 1));
        let i = g.mobs.iter().position(|m| m.kind == Kind::Ylgrath).expect("ylgrath on his throne");
        (g.p.x, g.p.y) = (g.mobs[i].x + 4.0, g.mobs[i].y);
        let mut seen = std::collections::HashSet::new();
        for _ in 0..30 {
            g.mobs[i].special2 = 0.0;
            g.update_churnfolk();
            seen.insert(g.mobs[i].form);
            assert!((g.mobs[i].form as usize) < YLGRATH_FORMS.len());
            // And he fights in each shape.
            g.mobs[i].special = 0.0;
            g.mobs[i].special2 = 5.0;
            let before = g.shots.len() + g.hazards.len();
            g.update_churnfolk();
            assert!(g.shots.len() + g.hazards.len() > before || g.pull.2 > 0.0, "an attack in form {}", g.mobs[i].form);
        }
        assert!(seen.len() >= 4, "many shapes: {seen:?}");
        g.mobs[i].hp = g.mobs[i].max_hp * 0.1;
        g.update_churnfolk();
        assert_eq!(g.mobs[i].form as usize, YLGRATH_FORMS.len(), "his own shape at the end");
    }

    #[test]
    fn ylgrath_is_back_on_a_kept_throne_once_you_carry_the_stillpoint() {
        let throne = LevelId::Dungeon(crate::world::EYE, crate::world::DUNGEONS[crate::world::EYE].floors - 1);
        let mut g = at(throne);
        g.quest.stage7 = 2;
        assert!(g.debug_kill_boss());
        assert_eq!(g.quest.stillpoint, 1, "he fled");
        g.dialog = None;
        g.debug_grant_stillpoint();
        // Out and back: the floor is kept, his corpse with it, but he's there again.
        g.debug_goto(LevelId::Area(6, 6));
        g.debug_goto(throne);
        assert!(g.mobs.iter().any(|m| m.kind == Kind::Ylgrath && m.alive()), "back on his throne");
        assert!(g.debug_kill_boss());
        assert_eq!(g.quest.stage7, 3, "and dies for good");
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

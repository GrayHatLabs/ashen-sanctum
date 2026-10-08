//! Act 4's time mechanics: brass **stop-clocks** you strike (walk into them) to slow every
//! foe around them for a while, and **law zones** where Mechanus decrees how foes may be hurt.
//! Both appear in the Grinding Fields and the clockwork dungeons (not in town).
use crate::game::{Game, Sfx, DT};
use crate::gfx::{mix, rgb, Align, Screen};
use crate::iso;
use crate::mobs::MobState;
use crate::render::{blend_ellipse, ring};
use crate::rng::Rng;

/// How long a struck clock holds time back, and how long it then takes to rewind.
pub const CLOCK_FIELD: f32 = 8.0;
pub const CLOCK_REWIND: f32 = 14.0;
/// Radius of the slowed area (tiles).
pub const CLOCK_R: f32 = 4.5;
/// Walking this close strikes a ready clock.
const STRIKE_R: f32 = 1.1;
pub const LAW_R: f32 = 4.0;
/// Damage a forbidden blow still does inside a law zone.
pub const LAW_LEAK: f32 = 0.1;

pub struct TimeClock {
    pub x: f32,
    pub y: f32,
    /// Seconds of slowed time left.
    pub field: f32,
    /// Seconds until it can be struck again.
    pub cool: f32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Law {
    /// Only blows struck from close by (within 3 tiles) wound here.
    Blade,
    /// Only blows from afar (4 tiles or more) wound here.
    Arrow,
    /// Only foes standing still (attacking, stunned, frozen) can be hurt here.
    Stillness,
}

impl Law {
    pub fn name(self) -> &'static str {
        match self {
            Law::Blade => "LAW OF THE BLADE",
            Law::Arrow => "LAW OF THE ARROW",
            Law::Stillness => "LAW OF STILLNESS",
        }
    }
    pub fn rule(self) -> &'static str {
        match self {
            Law::Blade => "ONLY CLOSE BLOWS WOUND HERE",
            Law::Arrow => "ONLY BLOWS FROM AFAR WOUND HERE",
            Law::Stillness => "ONLY THE STILL CAN BE HURT HERE",
        }
    }
    fn col(self) -> u32 {
        match self {
            Law::Blade => 0xe06040,
            Law::Arrow => 0x50b0e0,
            Law::Stillness => 0xe0c050,
        }
    }
}

pub struct LawZone {
    pub x: f32,
    pub y: f32,
    pub law: Law,
}

impl Game {
    /// Puts clocks and law zones on a clockwork level (the same places on every visit).
    pub(crate) fn place_clockwork(&mut self) {
        self.clocks.clear();
        self.laws.clear();
        self.law_here = None;
        if !self.theme.clockwork() {
            return;
        }
        let tag: u64 = format!("{:?}", self.level).bytes().fold(7, |a, b| a.wrapping_mul(31).wrapping_add(b as u64));
        let mut rng = Rng::new(self.world_seed() ^ tag ^ 0xc10c);
        let (n_clocks, n_laws) = if self.level.overland() { (5, 3) } else { (3, 2) };
        let mut taken: Vec<(f32, f32)> = vec![self.start];
        taken.extend(self.portals.iter().map(|p| (p.x, p.y)));
        let spot = |g: &Game, rng: &mut Rng, clear: f32, taken: &mut Vec<(f32, f32)>| -> Option<(f32, f32)> {
            for _ in 0..400 {
                let x = rng.rf(2.0, g.d.w as f32 - 2.0).floor() + 0.5;
                let y = rng.rf(2.0, g.d.h as f32 - 2.0).floor() + 0.5;
                if g.d.blocked(x, y, clear) || g.safe_contains(x, y, 6.0) {
                    continue;
                }
                if taken.iter().any(|&(tx, ty)| (tx - x).powi(2) + (ty - y).powi(2) < 100.0) {
                    continue;
                }
                taken.push((x, y));
                return Some((x, y));
            }
            None
        };
        for _ in 0..n_clocks {
            if let Some((x, y)) = spot(self, &mut rng, 0.6, &mut taken) {
                self.clocks.push(TimeClock { x, y, field: 0.0, cool: 0.0 });
            }
        }
        let laws = [Law::Blade, Law::Arrow, Law::Stillness];
        for k in 0..n_laws {
            if let Some((x, y)) = spot(self, &mut rng, 1.2, &mut taken) {
                self.laws.push(LawZone { x, y, law: laws[(k + rng.range(0, 3) as usize) % 3] });
            }
        }
    }

    pub(crate) fn update_clockwork(&mut self) {
        if self.clocks.is_empty() && self.laws.is_empty() {
            return;
        }
        let (px, py) = (self.p.x, self.p.y);
        let mut struck = false;
        for c in self.clocks.iter_mut() {
            c.field = (c.field - DT).max(0.0);
            c.cool = (c.cool - DT).max(0.0);
            if c.cool <= 0.0 && (c.x - px).powi(2) + (c.y - py).powi(2) < STRIKE_R * STRIKE_R {
                c.field = CLOCK_FIELD;
                c.cool = CLOCK_FIELD + CLOCK_REWIND;
                struck = true;
            }
        }
        if struck {
            self.sfx.push(Sfx::Descend);
            self.shake = self.shake.max(0.3);
            self.say("THE CLOCK STOPS. TIME DRAGS AROUND IT".into());
        }
        // Inside a running field, foes crawl: they move at a third and wind up their blows slowly.
        for c in self.clocks.iter().filter(|c| c.field > 0.0) {
            for m in self.mobs.iter_mut().filter(|m| m.alive() && m.charm <= 0.0) {
                if (m.x - c.x).powi(2) + (m.y - c.y).powi(2) < CLOCK_R * CLOCK_R {
                    m.slow_t = m.slow_t.max(0.1);
                    m.cd += DT * if m.boss { 0.3 } else { 0.6 };
                    if let MobState::Attack(t) = m.state {
                        m.state = MobState::Attack(t + DT * 0.5);
                    }
                }
            }
        }
        // Announce a law when you step into its zone.
        let here = self.laws.iter().position(|z| (z.x - px).powi(2) + (z.y - py).powi(2) < LAW_R * LAW_R);
        if here != self.law_here {
            if let Some(i) = here {
                let law = self.laws[i].law;
                self.say(format!("{}: {}", law.name(), law.rule()));
            }
            self.law_here = here;
        }
    }

    /// How much of a blow on monster `i` the laws allow (1 outside any zone).
    pub(crate) fn law_scale(&self, i: usize) -> f32 {
        // A writ from Magistrate Korvel's court (gears.rs): the laws don't bind you.
        if self.feats.writ {
            return 1.0;
        }
        let m = &self.mobs[i];
        let Some(z) = self.laws.iter().find(|z| (z.x - m.x).powi(2) + (z.y - m.y).powi(2) < LAW_R * LAW_R) else { return 1.0 };
        let d = ((m.x - self.p.x).powi(2) + (m.y - self.p.y).powi(2)).sqrt();
        let allowed = match z.law {
            Law::Blade => d <= 3.0,
            Law::Arrow => d >= 4.0,
            Law::Stillness => !matches!(m.state, MobState::Chase) || m.stun > 0.0 || m.frozen > 0.0,
        };
        if allowed {
            1.0
        } else {
            LAW_LEAK
        }
    }

    /// Floor marks: law circles and the clocks' slowed areas (drawn under everything else).
    pub(crate) fn draw_clockwork_floor(&self, scr: &mut Screen, to_scr: &dyn Fn(f32, f32) -> (i32, i32)) {
        let half = iso::TW * 0.5;
        for z in &self.laws {
            let (sx, sy) = to_scr(z.x, z.y);
            let (rx, ry) = ((LAW_R * half) as i32, (LAW_R * half * 0.5) as i32);
            let c = rgb(z.law.col());
            let pulse = 0.5 + 0.5 * (self.tick as f32 * 0.05).sin();
            blend_ellipse(scr, sx, sy, rx, ry, c, 0.1 + 0.05 * pulse);
            // Two brass rings with glyph ticks between them, slowly turning.
            ring(scr, sx, sy, rx, ry, rgb(0xb08840));
            ring(scr, sx, sy, rx - 6, ry - 3, mix(rgb(0x6a5020), c, 0.5));
            for k in 0..36 {
                let a = k as f32 / 36.0 * std::f32::consts::TAU + self.tick as f32 * 0.004;
                let (gx, gy) = (sx + (a.cos() * (rx - 3) as f32) as i32, sy + (a.sin() * (ry as f32 - 1.5)) as i32);
                let w = if k % 3 == 0 { 3 } else { 1 };
                scr.fill(gx - w / 2, gy, w, 1, if k % 3 == 0 { c } else { rgb(0xd0a850) });
            }
            let label = match z.law {
                Law::Blade => "I",
                Law::Arrow => "II",
                Law::Stillness => "III",
            };
            scr.text(label, sx, sy - 3, mix(rgb(0x2a2014), c, 0.6), Align::Center, 1);
        }
        for c in &self.clocks {
            if c.field <= 0.0 {
                continue;
            }
            let (sx, sy) = to_scr(c.x, c.y);
            let k = (c.field / CLOCK_FIELD).min(1.0);
            let (rx, ry) = ((CLOCK_R * half) as i32, (CLOCK_R * half * 0.5) as i32);
            blend_ellipse(scr, sx, sy, rx, ry, rgb(0x5070c0), 0.18 * k + 0.06);
            ring(scr, sx, sy, rx, ry, mix(rgb(0x304060), rgb(0xa0c0ff), k));
            // The edge of slowed time ripples inward.
            for q in 0..2 {
                let r = 1.0 - ((self.tick as f32 * 0.008 + q as f32 * 0.5) % 1.0);
                ring(scr, sx, sy, (rx as f32 * r) as i32, (ry as f32 * r) as i32, mix(rgb(0x203050), rgb(0xc0d8ff), k * r));
            }
        }
    }

    /// The clocks themselves: tall brass cases with a white face (hand frozen while time drags).
    pub(crate) fn draw_clock(&self, scr: &mut Screen, i: usize, sx: i32, sy: i32) {
        let c = &self.clocks[i];
        let ready = c.cool <= 0.0;
        blend_ellipse(scr, sx, sy, 12, 5, rgb(0x000000), 0.45);
        // A tall longcase clock on a stepped brass plinth, gothic peak on top.
        scr.fill(sx - 9, sy - 4, 18, 4, rgb(0x6a4a1c));
        scr.fill(sx - 7, sy - 44, 14, 41, rgb(0x3a2410));
        scr.fill(sx - 6, sy - 43, 12, 39, rgb(0x7a4a1e));
        scr.fill(sx - 8, sy - 46, 16, 3, rgb(0xb08840));
        for k in 0..6 {
            scr.fill(sx - 5 + k, sy - 52 + k, 10 - 2 * k, 1, rgb(0xb08840));
        }
        scr.fill(sx - 8, sy - 5, 16, 2, rgb(0xb08840));
        // Face.
        scr.disc(sx, sy - 35, 7, rgb(0xc89848));
        scr.disc(sx, sy - 35, 6, rgb(0xf0e8d0));
        for k in 0..12 {
            let a = k as f32 / 12.0 * std::f32::consts::TAU;
            scr.pset(sx + (a.cos() * 5.0) as i32, sy - 35 + (a.sin() * 5.0) as i32, rgb(0x6a5030));
        }
        let a = if c.field > 0.0 { -1.2 } else { self.tick as f32 * 0.05 };
        for k in 0..5 {
            scr.pset(sx + (a.cos() * k as f32) as i32, sy - 35 + (a.sin() * k as f32) as i32, rgb(0x201810));
        }
        scr.pset(sx, sy - 35, rgb(0x201810));
        // Pendulum window: swinging when ready, still when spent.
        let sw = if c.field > 0.0 || !ready { 0.0 } else { (self.tick as f32 * 0.12).sin() * 3.0 };
        scr.fill(sx - 4, sy - 26, 8, 19, rgb(0x1a1008));
        for y in 0..8 {
            scr.pset(sx + (sw * y as f32 / 8.0) as i32, sy - 25 + y, rgb(0x9a7a40));
        }
        scr.disc(sx + sw as i32, sy - 15, 2, rgb(0xe0b050));
        if ready {
            let pulse = 0.5 + 0.5 * (self.tick as f32 * 0.08).sin();
            blend_ellipse(scr, sx, sy - 35, 11, 11, rgb(0xa0c0ff), 0.12 + 0.18 * pulse);
        } else if c.field > 0.0 {
            blend_ellipse(scr, sx, sy - 35, 11, 11, rgb(0x6080ff), 0.3);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{Game, Input};
    use crate::mobs::{Kind, Mob};
    use crate::world::LevelId;

    #[test]
    fn mechanus_has_clocks_that_slow_time_and_laws_that_bind() {
        let mut g = Game::new(5, crate::gfx::SH_WIDE);
        assert!(g.clocks.is_empty() && g.laws.is_empty(), "none in Act 1");
        g.debug_goto(LevelId::Mechanus);
        assert!(g.clocks.len() >= 3, "clocks in the Grinding Fields: {}", g.clocks.len());
        assert!(!g.laws.is_empty());
        for c in &g.clocks {
            assert!(!g.d.blocked(c.x, c.y, 0.3) && !g.safe_contains(c.x, c.y, 2.0));
        }
        // The same places on every visit.
        let at: Vec<(f32, f32)> = g.clocks.iter().map(|c| (c.x, c.y)).collect();
        g.debug_goto(LevelId::Overworld);
        g.debug_goto(LevelId::Mechanus);
        assert_eq!(at, g.clocks.iter().map(|c| (c.x, c.y)).collect::<Vec<_>>());

        // Walking into a clock stops time around it.
        g.mobs.clear();
        let (cx, cy) = (g.clocks[0].x, g.clocks[0].y);
        (g.p.x, g.p.y) = (cx, cy + 0.6);
        let mut m = Mob::new(Kind::Zombie, cx + 2.0, cy, 1.0, &mut g.rng);
        m.cd = 1.0;
        g.mobs.push(m);
        g.update(&Input::default());
        assert!(g.clocks[0].field > 0.0, "struck");
        assert!(g.mobs[0].slow_t > 0.0, "the zombie crawls");
        assert!(g.mobs[0].cd > 1.0 - 2.0 * DT, "its blows wind up slowly");
        // ...and it can't be struck again until it rewinds.
        for _ in 0..((CLOCK_FIELD + 1.0) / DT) as usize {
            g.update(&Input::default());
        }
        assert_eq!(g.clocks[0].field, 0.0);
        assert!(g.clocks[0].cool > 0.0);

        // A law zone: the Law of the Blade lets only close blows wound.
        g.mobs.clear();
        g.laws[0].law = Law::Blade;
        let (zx, zy) = (g.laws[0].x, g.laws[0].y);
        let m = Mob::new(Kind::Zombie, zx, zy, 1.0, &mut g.rng);
        g.mobs.push(m);
        (g.p.x, g.p.y) = (zx + 6.0, zy);
        assert_eq!(g.law_scale(0), LAW_LEAK, "from afar: forbidden");
        (g.p.x, g.p.y) = (zx + 1.5, zy);
        assert_eq!(g.law_scale(0), 1.0, "up close: allowed");
        g.laws[0].law = Law::Arrow;
        assert_eq!(g.law_scale(0), LAW_LEAK);
        // Lure it out of the zone and anything goes.
        g.mobs[0].x = zx + LAW_R + 2.0;
        assert_eq!(g.law_scale(0), 1.0);
        g.laws[0].law = Law::Stillness;
        g.mobs[0].x = zx;
        g.mobs[0].state = MobState::Chase;
        assert_eq!(g.law_scale(0), LAW_LEAK, "it's moving");
        g.mobs[0].stun = 1.0;
        assert_eq!(g.law_scale(0), 1.0, "stunned: still");
    }
}

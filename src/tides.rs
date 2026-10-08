//! Act 5's sea (docs/ACT5_ACT6_PLAN.md): the tide on the Sunken Reach, sirens' pull, ink and the
//! Angler Matriarch's dark.
//!
//! The tide: every so often a bell tolls and the low tide flats (ground 1) flood for a while. You and
//! land monsters wade there (slower); sea monsters swim faster wherever the water is. Roads, the town and
//! high ground stay dry.
use crate::game::{move_circle, Game, Sfx, DT, PLAYER_R};
use crate::gfx::rgb;
use crate::world::{sea_kind, LevelId};

/// Calm, then the bell (a warning), then the flood, then the ebb (seconds).
pub const TIDE_CALM: f32 = 70.0;
pub const TIDE_WARN: f32 = 5.0;
pub const TIDE_HIGH: f32 = 20.0;
pub const TIDE_EBB: f32 = 4.0;
pub const TIDE_CYCLE: f32 = TIDE_CALM + TIDE_WARN + TIDE_HIGH + TIDE_EBB;
/// Wading in the flood (you and land monsters), and sea monsters in it.
pub const WADE: f32 = 0.7;
pub const SWIM: f32 = 1.3;
/// A siren's pull, tiles per second.
pub const PULL_SPEED: f32 = 3.2;

/// The tide comes in on Act 5's outdoor maps (the Sunken Reach and its areas).
pub fn tidal(id: LevelId) -> bool {
    id.act() == 4 && id.overland()
}

/// Water depth 0..1 at a point in the tide's cycle.
pub fn tide_level(t: f32) -> f32 {
    let t = t.rem_euclid(TIDE_CYCLE);
    if t < TIDE_CALM {
        0.0
    } else if t < TIDE_CALM + TIDE_WARN {
        (t - TIDE_CALM) / TIDE_WARN * 0.3
    } else if t < TIDE_CALM + TIDE_WARN + TIDE_HIGH {
        (0.3 + (t - TIDE_CALM - TIDE_WARN) / 1.5).min(1.0)
    } else {
        1.0 - (t - TIDE_CALM - TIDE_WARN - TIDE_HIGH) / TIDE_EBB
    }
}

impl Game {
    /// Is (x, y) under water right now?
    pub fn flooded(&self, x: f32, y: f32) -> bool {
        self.tide > 0.5 && tidal(self.level) && self.d.ground_at(x.floor() as i32, y.floor() as i32) == 1
    }

    /// The tide's state for the HUD: (label, level 0..1), or None off the Sunken Reach.
    pub fn tide_gauge(&self) -> Option<(&'static str, f32)> {
        if !tidal(self.level) {
            return None;
        }
        let t = self.tide_t.rem_euclid(TIDE_CYCLE);
        let label = if t < TIDE_CALM {
            "LOW TIDE"
        } else if t < TIDE_CALM + TIDE_WARN {
            "THE TIDE BELL!"
        } else if t < TIDE_CALM + TIDE_WARN + TIDE_HIGH {
            "HIGH TIDE"
        } else {
            "EBBING"
        };
        Some((label, self.tide))
    }

    /// How much the light is cut (1 = not at all): ink in your eyes, or the Angler's dark.
    pub fn light_scale(&self) -> f32 {
        let blind = 1.0 - 0.55 * (self.blind_t / 3.5).min(1.0);
        let dark = if self.dark_t > 0.0 { 0.35 } else { 1.0 };
        // An Eclipse rift.
        let eclipse = if self.rift_has(crate::endgame::RiftMod::Eclipse) { 0.55 } else { 1.0 };
        // An ash storm (features.rs).
        blind.min(dark).min(eclipse).min(self.storm_light())
    }

    pub(crate) fn update_deep(&mut self) {
        // Ink and darkness fade; the light is rebuilt while they change.
        let dimmed = self.blind_t > 0.0 || self.dark_t > 0.0;
        self.blind_t = (self.blind_t - DT).max(0.0);
        self.dark_t = (self.dark_t - DT).max(0.0);
        if dimmed {
            self.light_ready = false;
        }
        // A siren's (or the Leviathan's whirlpool's) pull.
        if self.pull.2 > 0.0 {
            self.pull.2 -= DT;
            let (dx, dy) = (self.pull.0 - self.p.x, self.pull.1 - self.p.y);
            let l = (dx * dx + dy * dy).sqrt();
            if l > 1.2 {
                let (mut x, mut y) = (self.p.x, self.p.y);
                move_circle(&self.d, &mut x, &mut y, dx / l * PULL_SPEED * DT, dy / l * PULL_SPEED * DT, PLAYER_R);
                (self.p.x, self.p.y) = (x, y);
            }
        }
        // The tide (the Sunken Reach only; the clock keeps running while you're away).
        self.tide_t += DT;
        if !tidal(self.level) {
            self.tide = 0.0;
            for m in self.mobs.iter_mut() {
                m.tide = 1.0;
            }
            return;
        }
        let before = self.tide;
        self.tide = tide_level(self.tide_t);
        let t = self.tide_t.rem_euclid(TIDE_CYCLE);
        if t >= TIDE_CALM && t - DT < TIDE_CALM {
            self.sfx.push(Sfx::Descend);
            self.say("THE TIDE BELL TOLLS. GET TO HIGH GROUND!".into());
        }
        if before <= 0.5 && self.tide > 0.5 {
            self.say("THE TIDE IS IN. THE FLATS ARE FLOODED".into());
        }
        let level = self.tide;
        let wet: Vec<bool> = self.mobs.iter().map(|m| level > 0.5 && self.d.ground_at(m.x.floor() as i32, m.y.floor() as i32) == 1).collect();
        for (m, wet) in self.mobs.iter_mut().zip(wet) {
            m.tide = match (wet, sea_kind(m.kind)) {
                (true, true) => SWIM,
                (true, false) => WADE,
                _ => 1.0,
            };
        }
    }

    /// A monster's cue: 3 pulls you toward it (a siren's song, the whirlpool), 4 puts the lights out.
    pub(crate) fn deep_cue(&mut self, i: usize, cue: u8) {
        let (x, y) = (self.mobs[i].x, self.mobs[i].y);
        match cue {
            3 => {
                self.pull = (x, y, 1.4);
                self.floater(self.p.x, self.p.y, "DRAWN IN...".into(), rgb(0x60e0d0));
            }
            4 => {
                self.dark_t = 7.0;
                self.light_ready = false;
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Game;
    use crate::mobs::{Kind, Mob};

    #[test]
    fn the_tide_comes_in_floods_the_flats_and_goes_out() {
        assert_eq!(tide_level(10.0), 0.0);
        assert!(tide_level(TIDE_CALM + TIDE_WARN + 5.0) >= 1.0);
        assert_eq!(tide_level(TIDE_CYCLE + 10.0), 0.0, "and round again");
        let mut g = Game::new(7, 360);
        g.debug_goto(LevelId::Deep);
        // A low tile and a dry one.
        let (w, h) = (g.d.w, g.d.h);
        let mut low = None;
        let mut dry = None;
        for y in 10..h - 10 {
            for x in 10..w - 10 {
                if g.d.walkable(x, y) {
                    match g.d.ground_at(x, y) {
                        1 if low.is_none() => low = Some((x as f32 + 0.5, y as f32 + 0.5)),
                        0 if dry.is_none() => dry = Some((x as f32 + 0.5, y as f32 + 0.5)),
                        _ => {}
                    }
                }
            }
        }
        let (low, dry) = (low.unwrap(), dry.unwrap());
        g.mobs.clear();
        g.mobs.push(Mob::new(Kind::Merrow, low.0, low.1, 9.0, &mut g.rng));
        g.mobs.push(Mob::new(Kind::Drowned, low.0, low.1, 9.0, &mut g.rng));
        g.mobs.push(Mob::new(Kind::Merrow, dry.0, dry.1, 9.0, &mut g.rng));
        g.tide_t = 0.0;
        g.update_deep();
        assert!(!g.flooded(low.0, low.1));
        assert!(g.mobs.iter().all(|m| m.tide == 1.0));
        g.tide_t = TIDE_CALM + TIDE_WARN + 5.0;
        g.update_deep();
        assert!(g.flooded(low.0, low.1) && !g.flooded(dry.0, dry.1));
        assert_eq!(g.mobs[0].tide, SWIM, "the merrow swims in the flood");
        assert_eq!(g.mobs[1].tide, WADE, "the drowned sailor wades");
        assert_eq!(g.mobs[2].tide, 1.0, "high ground stays dry");
        assert_eq!(g.tide_gauge().unwrap().0, "HIGH TIDE");
    }

    #[test]
    fn a_siren_draws_you_in_and_ink_dims_the_light() {
        let mut g = Game::new(7, 360);
        g.debug_goto(LevelId::Deep);
        let (px, py) = (g.p.x, g.p.y);
        g.mobs.clear();
        g.mobs.push(Mob::new(Kind::Siren, px + 5.0, py, 9.0, &mut g.rng));
        g.deep_cue(0, 3);
        for _ in 0..30 {
            g.update_deep();
        }
        assert!(g.p.x > px + 0.5 || (g.p.x - px).abs() < 0.01 && g.d.blocked(px + 0.6, py, PLAYER_R), "pulled toward her");
        g.blind_t = 3.5;
        assert!(g.light_scale() < 0.5);
        g.blind_t = 0.0;
        g.deep_cue(0, 4);
        assert!(g.light_scale() <= 0.35, "the lights go out");
    }
}

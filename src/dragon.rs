//! The Ember Wyrm's hoard (Act 1, off the Cinder Hills): a heist, not a fight.
//!
//! Vaurath, a great fire dragon, has made its lair in a cave in Emberpeak Pass, asleep on a hoard of gold.
//! Gold piles go into a **hoard sack** that slows you a little and only becomes yours once you get out of
//! the cave; die in there and the dragon has it back. **Noise** wakes it: grabbing gold, fighting near it,
//! running and standing close. Awake, it hunts you through its cave with fire; it never leaves it, and the
//! next time you sneak in it's asleep again (and whole). Killing it is possible, for a hero far stronger
//! than Act 1 expects.
use crate::game::{Drop, Game, PKind, Pickup, Sfx, DT};
use crate::gfx::rgb;
use crate::mobs::{Kind, MobState};
use crate::rng::Rng;
use crate::world::{Level, LevelId, WYRM};

/// Noise to wake it, and what makes noise.
pub const WAKE: f32 = 100.0;
pub const NOISE_GRAB: f32 = 22.0;
pub const NOISE_CAST: f32 = 5.0;
/// Per second: standing close, close-ish, running anywhere nearby.
pub const NOISE_CLOSE: f32 = 14.0;
pub const NOISE_NEAR: f32 = 4.0;
pub const NOISE_RUN: f32 = 6.0;
/// Per second, when you're quiet.
pub const NOISE_FADE: f32 = 5.0;
/// The sack slows you by up to this much (a full sack: SACK_HEAVY gold or more).
pub const SACK_SLOW: f32 = 0.25;
pub const SACK_HEAVY: f32 = 4000.0;
/// Escaping with this much finishes Captain Rolf's dare (side.rs).
pub const HEIST_GOAL: i32 = 500;

pub fn in_lair(id: LevelId) -> bool {
    matches!(id, LevelId::Dungeon(k, _) if k == WYRM)
}

/// Opens up the dragon's room into a great cavern and heaps its hoard around it (world::build_at).
pub fn place(lv: &mut Level, seed: u64, difficulty: u8) {
    if !in_lair(lv.id) {
        return;
    }
    let Some((bx, by)) = lv.mobs.iter().find(|m| m.kind == Kind::FireWyrm).map(|m| (m.x, m.y)) else { return };
    let mut rng = Rng::new(seed ^ 0x5EA1_0DD5);
    let (w, h) = (lv.d.w, lv.d.h);
    let (cx, cy) = (bx as i32, by as i32);
    for y in (cy - 10).max(2)..=(cy + 10).min(h - 3) {
        for x in (cx - 11).max(2)..=(cx + 11).min(w - 3) {
            let r = (((x - cx) as f32 / 11.0).powi(2) + ((y - cy) as f32 / 9.0).powi(2)).sqrt();
            if r < 1.0 - rng.f() * 0.08 {
                lv.d.set(x, y, crate::dungeon::Tile::Floor);
            }
        }
    }
    // Nothing else sleeps in the lair.
    lv.mobs.retain(|m| m.kind == Kind::FireWyrm || crate::breakables::is_prop(m.kind) || (m.x - bx).powi(2) + (m.y - by).powi(2) > 196.0);
    let scale = 1.0 + difficulty as f32 * 1.5;
    let mut placed = 0;
    for _ in 0..400 {
        if placed >= 14 {
            break;
        }
        let a = rng.f() * std::f32::consts::TAU;
        let r = rng.rf(2.4, 7.5);
        let (x, y) = (bx + a.cos() * r * 1.2, by + a.sin() * r);
        if lv.d.blocked(x, y, 0.4) || lv.pickups.iter().any(|k| (k.x - x).powi(2) + (k.y - y).powi(2) < 1.4) {
            continue;
        }
        let gold = (rng.rf(90.0, 210.0) * scale) as i32;
        lv.pickups.push(Pickup { x, y, kind: Drop::Hoard(gold), t: 1.0 });
        placed += 1;
    }
}

impl Game {
    /// How much the hoard sack slows you (1 = not at all).
    pub fn sack_slow(&self) -> f32 {
        1.0 - SACK_SLOW * (self.p.sack as f32 / SACK_HEAVY).min(1.0)
    }

    fn wyrm(&self) -> Option<usize> {
        self.mobs.iter().position(|m| m.kind == Kind::FireWyrm && m.alive())
    }

    /// For the HUD: (noise 0..1, awake, gold in the sack), in the lair.
    pub fn wyrm_hud(&self) -> Option<(f32, bool, i32)> {
        if !in_lair(self.level) {
            return None;
        }
        let awake = self.wyrm().map_or(false, |i| !self.mobs[i].asleep);
        Some(((self.wyrm_noise / WAKE).min(1.0), awake, self.p.sack))
    }

    /// Every tick: the sleeping dragon and the noise you make, the sack, and getting out alive.
    pub(crate) fn update_wyrm(&mut self) {
        // Out of the lair with the sack: it's yours.
        if !in_lair(self.level) {
            self.wyrm_in = false;
            if self.p.sack > 0 && !matches!(self.state, crate::game::State::Dead(_)) {
                let n = self.p.sack;
                self.p.gold += n;
                self.p.sack = 0;
                self.sfx.push(Sfx::Pickup);
                self.say(format!("YOU ESCAPED THE WYRM WITH {n} GOLD!"));
                if n >= HEIST_GOAL {
                    self.side_progress(crate::side::Goal::Heist);
                }
                self.save_due = true;
            }
            return;
        }
        let Some(i) = self.wyrm() else {
            self.wyrm_in = true;
            return;
        };
        // Sneaking in again: it's asleep, and whole.
        if !self.wyrm_in {
            self.wyrm_in = true;
            self.wyrm_noise = 0.0;
            let m = &mut self.mobs[i];
            m.asleep = true;
            m.hp = m.max_hp;
            m.state = MobState::Idle;
            m.enraged = false;
        }
        // Dead in the lair: the hoard is the dragon's again.
        if matches!(self.state, crate::game::State::Dead(_)) {
            if self.p.sack > 0 {
                let (x, y) = (self.mobs[i].x + 1.5, self.mobs[i].y + 1.0);
                self.pickups.push(Pickup { x, y, kind: Drop::Hoard(self.p.sack), t: 1.0 });
                self.p.sack = 0;
            }
            return;
        }
        let (mx, my) = (self.mobs[i].x, self.mobs[i].y);
        let dist = ((mx - self.p.x).powi(2) + (my - self.p.y).powi(2)).sqrt();
        if self.mobs[i].asleep {
            let hurt = self.mobs[i].hp < self.mobs[i].max_hp;
            let mut add = 0.0;
            if dist < 5.0 {
                add += NOISE_CLOSE;
            } else if dist < 10.0 {
                add += NOISE_NEAR;
            }
            if self.p.moving && self.p.running && dist < 16.0 {
                add += NOISE_RUN;
            }
            if self.stats.casts > self.wyrm_casts && dist < 16.0 {
                self.wyrm_noise += NOISE_CAST * (self.stats.casts - self.wyrm_casts) as f32;
            }
            self.wyrm_noise = (self.wyrm_noise + (add - if add == 0.0 { NOISE_FADE } else { 0.0 }) * DT).max(0.0);
            // Hurt it and it's awake, at once.
            if hurt {
                self.wyrm_noise = WAKE;
            }
            let m = &mut self.mobs[i];
            m.stun = m.stun.max(0.2);
            m.state = MobState::Idle;
            if self.tick % 150 == 0 {
                self.floater(mx + 0.6, my - 0.6, "Z z z".into(), rgb(0xc8b8a0));
            }
            if self.wyrm_noise >= WAKE {
                let m = &mut self.mobs[i];
                m.asleep = false;
                m.stun = 0.0;
                m.state = MobState::Chase;
                self.shake = 1.2;
                self.sfx.push(Sfx::Boom);
                for _ in 0..30 {
                    self.spray_at(mx, my, PKind::Fire, 30.0);
                }
                self.say("VAURATH WAKES! RUN!".into());
                self.floater(mx, my - 1.5, "WHO DARES?!".into(), rgb(0xff6020));
            }
        } else if dist > 30.0 && self.wyrm_noise > 0.0 {
            // Lost you: it settles back down, slowly.
            self.wyrm_noise = (self.wyrm_noise - NOISE_FADE * DT).max(0.0);
        }
        self.wyrm_casts = self.stats.casts;
    }

    /// Picked up a pile of the hoard: into the sack, and it clinks.
    pub(crate) fn grab_hoard(&mut self, n: i32) {
        self.p.sack += n;
        self.wyrm_noise += NOISE_GRAB;
        let (x, y) = (self.p.x, self.p.y);
        self.floater(x, y, format!("+{n} GOLD IN THE SACK"), rgb(0xffd040));
        // Now and then a gem rolls out of the heap.
        if self.rng.chance(0.12) {
            let ilvl = crate::items::ilvl_for(self.tier) + 4;
            let gem = crate::items::gem_item(crate::items::roll_gem(ilvl, &mut self.rng));
            self.pickups.push(Pickup { x: x + 0.4, y: y + 0.3, kind: Drop::Item(Box::new(gem)), t: 0.0 });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Game;

    fn lair() -> Game {
        let mut g = Game::new(7, 360);
        g.debug_goto(LevelId::Dungeon(WYRM, 0));
        g
    }

    #[test]
    fn the_wyrm_sleeps_on_its_hoard_until_you_make_noise() {
        let mut g = lair();
        let i = g.mobs.iter().position(|m| m.kind == Kind::FireWyrm).expect("the wyrm is home");
        let piles = g.pickups.iter().filter(|k| matches!(k.kind, Drop::Hoard(_))).count();
        assert!(piles >= 10, "a hoard to steal ({piles} piles)");
        g.update_wyrm();
        assert!(g.mobs[i].asleep, "asleep when you come in");
        // Standing still far away is quiet.
        for _ in 0..120 {
            g.update_wyrm();
        }
        assert!(g.mobs[i].asleep);
        // Grabbing gold makes noise; enough of it wakes it.
        for _ in 0..5 {
            g.grab_hoard(100);
        }
        g.update_wyrm();
        assert!(!g.mobs[i].asleep, "awake");
        assert_eq!(g.p.sack, 500);
    }

    #[test]
    fn the_sack_is_yours_only_once_you_get_out() {
        let mut g = lair();
        g.update_wyrm();
        g.grab_hoard(150);
        assert!(g.sack_slow() < 1.0, "the sack is heavy");
        let gold = g.p.gold;
        g.debug_goto(crate::areas::dungeon_home(WYRM));
        g.update_wyrm();
        assert_eq!(g.p.gold, gold + 150);
        assert_eq!(g.p.sack, 0);
        // Die in there and it's lost.
        g.debug_goto(LevelId::Dungeon(WYRM, 0));
        g.update_wyrm();
        g.grab_hoard(300);
        g.state = crate::game::State::Dead(0.0);
        g.update_wyrm();
        assert_eq!(g.p.sack, 0);
        assert!(g.pickups.iter().any(|k| matches!(k.kind, Drop::Hoard(300))), "back on the heap");
    }

    #[test]
    fn hitting_it_wakes_it_and_it_sleeps_again_next_time() {
        let mut g = lair();
        g.update_wyrm();
        let i = g.mobs.iter().position(|m| m.kind == Kind::FireWyrm).unwrap();
        g.mobs[i].hp -= 10.0;
        g.update_wyrm();
        assert!(!g.mobs[i].asleep);
        assert!(g.mobs[i].max_hp > 5000.0, "far too tough for Act 1");
        g.debug_goto(crate::areas::dungeon_home(WYRM));
        g.update_wyrm();
        g.debug_goto(LevelId::Dungeon(WYRM, 0));
        g.update_wyrm();
        let i = g.mobs.iter().position(|m| m.kind == Kind::FireWyrm).unwrap();
        assert!(g.mobs[i].asleep && g.mobs[i].hp == g.mobs[i].max_hp);
    }
}

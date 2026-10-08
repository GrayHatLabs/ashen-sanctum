//! Act 6's sky (docs/ACT5_ACT6_PLAN.md): the wind, the edges of the islands, the airship docks, and
//! the tricks of the sky's monsters.
//!
//! The wind: now and then streaks warn of a gust, then it blows for a moment, pushing you and every
//! monster (not bosses) along. The edges are real: whatever is blown or knocked off an island falls.
//! You lose some life and catch hold of the last solid ground you stood on; a monster is simply gone
//! (no XP, no loot).
use crate::dungeon::Tile;
use crate::game::{move_circle, Game, PKind, Particle, Sfx, DT, PLAYER_R};
use crate::gfx::rgb;
use crate::mobs::{Hazard, HazardKind, Kind, MobState};
use crate::world::{PortalKind, Theme};

/// The wind's rhythm (seconds): calm between gusts, the warning streaks, the gust itself.
pub const CALM: (f32, f32) = (7.0, 12.0);
pub const WARN: f32 = 1.2;
pub const GUST: f32 = 2.5;
/// Gust strength, tiles per second.
pub const GUST_SPEED: f32 = 2.3;
/// Falling off an island costs this share of your life.
pub const FALL_COST: f32 = 0.15;

impl Game {
    /// Does the wind blow here?
    pub fn windy(&self) -> bool {
        (self.level.act() == 5 && self.level.overland()) || self.theme == Theme::Spire || self.rift_has(crate::endgame::RiftMod::Gale)
    }

    /// Is (x, y) open sky?
    pub fn sky_at(&self, x: f32, y: f32) -> bool {
        self.theme.sky() && self.d.get(x.floor() as i32, y.floor() as i32) == Tile::Void
    }

    pub(crate) fn update_sky(&mut self) {
        self.sky_tricks();
        if !self.theme.sky() && !self.windy() {
            self.wind_gust = 0.0;
            self.wind_warn = 0.0;
            return;
        }
        // The last solid ground you stood on, well clear of any edge (where a fall puts you back).
        let (px, py) = (self.p.x, self.p.y);
        if (-2..=2).all(|dy| (-2..=2).all(|dx| !self.sky_at(px + dx as f32 * 0.6, py + dy as f32 * 0.6))) {
            self.last_safe = (px, py);
        }
        if !self.windy() {
            return;
        }
        // The wind's clock: calm, warning streaks, then the gust.
        if self.wind_gust > 0.0 {
            self.wind_gust -= DT;
            self.blow();
        } else if self.wind_warn > 0.0 {
            self.wind_warn -= DT;
            if self.wind_warn <= 0.0 {
                self.wind_gust = GUST;
                self.sfx.push(Sfx::Cast);
            }
        } else {
            self.wind_t -= DT;
            if self.wind_t <= 0.0 {
                self.wind_t = self.rng.rf(CALM.0, CALM.1);
                let a = self.rng.f() * std::f32::consts::TAU;
                self.wind_dir = (a.cos(), a.sin());
                self.wind_warn = WARN;
            }
        }
    }

    /// One tick of a gust: you and the monsters are pushed downwind; anything pushed over an edge falls.
    fn blow(&mut self) {
        let (wx, wy) = self.wind_dir;
        let step = GUST_SPEED * DT;
        if !self.in_safe(self.p.x, self.p.y) && !matches!(self.state, crate::game::State::Dead(_)) {
            if self.sky_at(self.p.x + wx * (PLAYER_R + 0.25), self.p.y + wy * (PLAYER_R + 0.25)) {
                self.player_falls();
            } else {
                let (mut x, mut y) = (self.p.x, self.p.y);
                move_circle(&self.d, &mut x, &mut y, wx * step, wy * step, PLAYER_R);
                (self.p.x, self.p.y) = (x, y);
            }
        }
        for i in 0..self.mobs.len() {
            let m = &self.mobs[i];
            if !m.alive() || m.boss || crate::breakables::is_prop(m.kind) {
                continue;
            }
            let (mx, my, r) = (m.x, m.y, m.r);
            if self.sky_at(mx + wx * (r + 0.25), my + wy * (r + 0.25)) {
                self.mob_falls(i);
            } else {
                let (mut x, mut y) = (mx, my);
                move_circle(&self.d, &mut x, &mut y, wx * step, wy * step, r);
                (self.mobs[i].x, self.mobs[i].y) = (x, y);
            }
        }
        // Wind-borne ash.
        if self.tick % 2 == 0 {
            let (px, py) = (self.p.x + self.rng.rf(-6.0, 6.0), self.p.y + self.rng.rf(-6.0, 6.0));
            self.parts.push(Particle { x: px, y: py, z: self.rng.rf(10.0, 40.0), vx: wx * 9.0, vy: wy * 9.0, vz: 0.0, life: 0.6, max: 0.6, kind: PKind::Smoke });
        }
    }

    /// You go over the edge: some life lost, and you catch hold of the last solid ground.
    pub(crate) fn player_falls(&mut self) {
        let lose = self.p.max_hp * FALL_COST;
        self.p.hp = (self.p.hp - lose).max(1.0);
        (self.p.x, self.p.y) = self.last_safe;
        self.p.path.clear();
        self.shake = self.shake.max(0.5);
        self.sfx.push(Sfx::Hurt);
        let (x, y) = (self.p.x, self.p.y);
        self.floater(x, y, "YOU FALL... AND CATCH HOLD".into(), rgb(0xffd080));
    }

    /// A monster blown or knocked off an island: gone, with no XP and no loot.
    pub(crate) fn mob_falls(&mut self, i: usize) {
        let m = &mut self.mobs[i];
        m.hp = 0.0;
        m.state = MobState::Dead(10.0);
        let (x, y) = (m.x, m.y);
        for k in 0..10 {
            let a = k as f32 / 10.0 * std::f32::consts::TAU;
            self.parts.push(Particle { x, y, z: 10.0, vx: a.cos() * 1.5, vy: a.sin() * 1.5, vz: -40.0, life: 0.6, max: 0.6, kind: PKind::Smoke });
        }
        self.floater(x, y, "FELL!".into(), rgb(0xc0d0ff));
        self.stats.kills_fell += 1;
    }

    /// Knocked from (x0, y0) by `dist` tiles: does the blow carry monster `i` over an edge? (Then it falls.)
    pub(crate) fn knocked_off(&mut self, i: usize, (x0, y0): (f32, f32), dist: f32) -> bool {
        let m = &self.mobs[i];
        if !self.theme.sky() || m.boss || !m.alive() {
            return false;
        }
        let (dx, dy) = (m.x - x0, m.y - y0);
        let l = (dx * dx + dy * dy).sqrt().max(0.01);
        let (ux, uy) = (dx / l, dy / l);
        let reach = dist + m.r + 0.2;
        let over = (1..=4).any(|k| self.sky_at(m.x + ux * reach * k as f32 / 4.0, m.y + uy * reach * k as f32 / 4.0));
        if over {
            self.mob_falls(i);
        }
        over
    }

    /// A shove (a harpy's talons, a gale): you're pushed `dist` tiles away from (x0, y0), maybe over an edge.
    pub(crate) fn shove_player(&mut self, (x0, y0): (f32, f32), dist: f32) {
        let (dx, dy) = (self.p.x - x0, self.p.y - y0);
        let l = (dx * dx + dy * dy).sqrt().max(0.01);
        let (ux, uy) = (dx / l, dy / l);
        if self.theme.sky() && (1..=4).any(|k| self.sky_at(self.p.x + ux * (dist + PLAYER_R) * k as f32 / 4.0, self.p.y + uy * (dist + PLAYER_R) * k as f32 / 4.0)) {
            self.player_falls();
            return;
        }
        let (mut x, mut y) = (self.p.x, self.p.y);
        move_circle(&self.d, &mut x, &mut y, ux * dist, uy * dist, PLAYER_R);
        (self.p.x, self.p.y) = (x, y);
    }

    /// The airship: from one dock to its partner on the same map.
    pub(crate) fn fly_airship(&mut self, n: u8) {
        let (px, py) = (self.p.x, self.p.y);
        let other = self
            .portals
            .iter()
            .filter(|p| p.kind == PortalKind::Dock(n) && (p.x - px).powi(2) + (p.y - py).powi(2) > 4.0)
            .map(|p| (p.x, p.y))
            .next();
        if let Some((x, y)) = other {
            // Step off beside the dock, not onto it.
            let spot = [(0.0f32, 1.5f32), (1.5, 0.0), (-1.5, 0.0), (0.0, -1.5)].into_iter().map(|(dx, dy)| (x + dx, y + dy)).find(|&(x, y)| !self.d.blocked(x, y, PLAYER_R));
            (self.p.x, self.p.y) = spot.unwrap_or((x, y + 1.0));
            self.p.path.clear();
            self.portal_cd = 2.0;
            self.sfx.push(Sfx::Descend);
            self.say("THE AIRSHIP CARRIES YOU ACROSS THE CLOUDS".into());
        }
    }

    /// A gale from the Tempest Drake (its cue 5): a strong gust straight away from it.
    pub(crate) fn gale_from(&mut self, i: usize) {
        let (x, y) = (self.mobs[i].x, self.mobs[i].y);
        let (dx, dy) = (self.p.x - x, self.p.y - y);
        let l = (dx * dx + dy * dy).sqrt().max(0.01);
        self.wind_dir = (dx / l, dy / l);
        self.wind_warn = 0.0;
        self.wind_gust = 1.6;
    }

    /// The sky monsters' tricks: seraphs dive, sentinels slam, zealots heal, thunderbirds call lightning.
    fn sky_tricks(&mut self) {
        let (px, py) = (self.p.x, self.p.y);
        let mut heals = vec![];
        for i in 0..self.mobs.len() {
            let m = &mut self.mobs[i];
            if !m.alive() || m.boss || m.charm > 0.0 || !matches!(m.kind, Kind::FallenSeraph | Kind::Sentinel | Kind::Zealot | Kind::Thunderbird) {
                continue;
            }
            m.special -= DT;
            if m.state != MobState::Chase || m.special > 0.0 {
                continue;
            }
            let dist = ((m.x - px).powi(2) + (m.y - py).powi(2)).sqrt();
            match m.kind {
                Kind::FallenSeraph if (2.5..6.5).contains(&dist) => {
                    m.special = 4.0;
                    m.rush = 0.5;
                }
                Kind::Sentinel if dist < 2.4 => {
                    m.special = 5.0;
                    let h = Hazard { x: m.x, y: m.y, r: 2.2, warn: 0.9, live: 0.0, dps: 0.0, burst: 20.0 * m.tier.powf(0.8), t: 0.0, fired: false, kind: HazardKind::Quake };
                    self.hazards.push(h);
                }
                Kind::Zealot => {
                    m.special = 6.0;
                    heals.push((m.x, m.y));
                }
                Kind::Thunderbird if dist < 10.0 => {
                    m.special = 5.0;
                    let tier = m.tier;
                    for k in 0..3 {
                        let a = k as f32 * 2.1 + self.tick as f32 * 0.1;
                        let r = if k == 0 { 0.0 } else { 1.6 };
                        self.hazards.push(Hazard { x: px + a.cos() * r, y: py + a.sin() * r, r: 0.9, warn: 1.0, live: 0.0, dps: 0.0, burst: 18.0 * tier.powf(0.8), t: 0.0, fired: false, kind: HazardKind::Quake });
                    }
                }
                _ => {}
            }
        }
        for (x, y) in heals {
            let mut any = false;
            for m in self.mobs.iter_mut().filter(|m| m.alive() && m.charm <= 0.0 && (m.x - x).powi(2) + (m.y - y).powi(2) < 25.0) {
                if m.hp < m.max_hp {
                    m.hp = (m.hp + m.max_hp * 0.12).min(m.max_hp);
                    any = true;
                }
            }
            if any {
                for k in 0..12 {
                    let a = k as f32 / 12.0 * std::f32::consts::TAU;
                    self.parts.push(Particle { x: x + a.cos() * 1.5, y: y + a.sin() * 1.5, z: 6.0, vx: 0.0, vy: 0.0, vz: 30.0, life: 0.6, max: 0.6, kind: PKind::Holy });
                }
                self.floater(x, y, "THE SUN MENDS THEM".into(), rgb(0xffe080));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Game;
    use crate::mobs::Mob;

    /// A floor tile next to the sky on the Skyreach, and the direction out over the edge.
    fn edge(g: &Game) -> ((f32, f32), (f32, f32)) {
        for y in 4..g.d.h - 4 {
            for x in 4..g.d.w - 4 {
                if g.d.walkable(x, y) && !g.d.blocked(x as f32 + 0.5, y as f32 + 0.5, 0.45) && g.d.get(x + 1, y) == Tile::Void && g.d.get(x + 2, y) == Tile::Void {
                    return ((x as f32 + 0.5, y as f32 + 0.5), (1.0, 0.0));
                }
            }
        }
        panic!("no edge");
    }

    #[test]
    fn gusts_blow_monsters_off_the_edge_and_you_catch_hold() {
        let mut g = Game::new(7, 360);
        g.debug_goto(crate::world::LevelId::Heavens);
        let ((x, y), dir) = edge(&g);
        g.mobs.clear();
        g.mobs.push(Mob::new(Kind::Harpy, x, y, 12.0, &mut g.rng));
        // You stand safe inland; then a gust over the edge.
        let safe = g.start;
        (g.p.x, g.p.y) = safe;
        g.update_sky();
        assert_eq!(g.last_safe, safe);
        g.wind_dir = dir;
        g.wind_gust = 1.0;
        for _ in 0..30 {
            g.blow();
        }
        assert!(!g.mobs[0].alive(), "the harpy is blown off the island");
        assert_eq!(g.stats.kills_fell, 1);
        // Now you stand on the edge.
        (g.p.x, g.p.y) = (x, y);
        g.p.hp = g.p.max_hp;
        g.blow();
        assert_eq!((g.p.x, g.p.y), safe, "you catch hold of solid ground");
        assert!(g.p.hp < g.p.max_hp && g.p.hp > g.p.max_hp * 0.8, "it costs some life");
    }

    #[test]
    fn knockbacks_send_monsters_over_and_the_airship_flies() {
        let mut g = Game::new(7, 360);
        g.debug_goto(crate::world::LevelId::Heavens);
        let ((x, y), _) = edge(&g);
        g.mobs.clear();
        g.mobs.push(Mob::new(Kind::FallenSeraph, x, y, 12.0, &mut g.rng));
        assert!(g.knocked_off(0, (x - 1.0, y), 1.0), "knocked toward the sky, it falls");
        assert!(!g.mobs[0].alive());
        // The airship: from the town's dock to the far island's.
        let docks: Vec<(f32, f32)> = g.portals.iter().filter(|p| p.kind == PortalKind::Dock(0)).map(|p| (p.x, p.y)).collect();
        assert_eq!(docks.len(), 2);
        (g.p.x, g.p.y) = docks[0];
        g.fly_airship(0);
        assert!((g.p.x - docks[1].0).abs() < 2.0 && (g.p.y - docks[1].1).abs() < 2.0, "you land at the other dock");
    }

    #[test]
    fn zealots_heal_their_kin() {
        let mut g = Game::new(7, 360);
        g.debug_goto(crate::world::LevelId::Heavens);
        let (x, y) = (g.p.x + 4.0, g.p.y);
        g.mobs.clear();
        let mut z = Mob::new(Kind::Zealot, x, y, 12.0, &mut g.rng);
        z.state = MobState::Chase;
        z.special = 0.0;
        g.mobs.push(z);
        let mut s = Mob::new(Kind::FallenSeraph, x + 1.0, y, 12.0, &mut g.rng);
        s.hp = s.max_hp * 0.5;
        g.mobs.push(s);
        g.sky_tricks();
        assert!(g.mobs[1].hp > g.mobs[1].max_hp * 0.55, "the sun mends it");
    }
}

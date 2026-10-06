//! The Inventor, the third playable class (docs/INVENTOR_CLASS.md): a steampunk gunslinger with
//! gadgets. She runs on **heat** instead of mana. Under the hood `p.mana` is her remaining heat
//! capacity (so every cost / fallback rule is shared with the casters):
//! - Firing heats her up; cooling refills it.
//! - Hitting zero overheats her: weapons lock for 2 s, and only a weak shot fires.
//! - Y / E vents steam, which dumps all heat and blasts foes back. Mana potions are coolant when
//!   the vent isn't ready.
//! - Above 70% heat she hits harder.
use crate::game::{Game, Light, PKind, Particle, Sfx, DT};
use crate::gfx::rgb;
use crate::mobs::MobState;
use crate::skills::{Skill, Skills, MAX_RANK};

pub fn is_inventor(s: Skill) -> bool {
    crate::skills::INVENTOR.contains(&s)
}

fn up(r: u8) -> f32 {
    (r.max(1) - 1) as f32
}

// ------------------------------------------------------------------ numbers per rank

pub const OVERHEAT_LOCK: f32 = 2.0;
pub const VENT_CD: f32 = 8.0;
pub const VENT_RADIUS: f32 = 2.6;
/// Heat capacity cooled per second (before Tinkerer).
pub const COOLING: f32 = 9.0;

pub fn ray_dmg(r: u8) -> (f32, f32) {
    let k = 1.0 + 0.15 * up(r);
    (6.0 * k, 10.0 * k)
}
pub fn ray_heat(r: u8) -> f32 {
    4.0 + 0.2 * up(r)
}
pub fn bomb_dmg(r: u8) -> f32 {
    22.0 + 8.0 * up(r)
}
pub fn bomb_heat(r: u8) -> f32 {
    12.0 + 0.6 * up(r)
}
pub const BOMB_FUSE: f32 = 1.0;
pub const BOMB_RADIUS: f32 = 2.0;
/// Tinkerer: faster cooling and longer-lasting gadgets.
pub fn tinker_cool(r: u8) -> f32 {
    1.0 + 0.06 * r as f32
}
pub fn tinker_time(r: u8) -> f32 {
    1.0 + 0.08 * r as f32
}
pub fn arc_dmg(r: u8) -> f32 {
    14.0 + 5.0 * up(r)
}
pub fn arc_jumps(r: u8) -> usize {
    3 + r as usize / 3
}
pub fn arc_heat(r: u8) -> f32 {
    10.0 + 0.5 * up(r)
}
pub fn turret_dmg(r: u8) -> f32 {
    7.0 + 2.5 * up(r)
}
pub const TURRET_TIME: f32 = 12.0;
pub fn turret_heat(r: u8) -> f32 {
    18.0 + 0.8 * up(r)
}
pub fn turret_max(r: u8) -> usize {
    1 + r as usize / 5
}
pub const GRAPPLE_RANGE: f32 = 7.0;
pub const GRAPPLE_CD: f32 = 3.0;
pub fn tesla_dps(r: u8) -> f32 {
    14.0 + 5.0 * up(r)
}
pub const TESLA_TIME: f32 = 6.0;
pub const TESLA_RADIUS: f32 = 2.6;
pub fn tesla_heat(r: u8) -> f32 {
    20.0 + 0.8 * up(r)
}
pub fn spider_dmg(r: u8) -> f32 {
    10.0 + 4.0 * up(r)
}
pub const SPIDER_TIME: f32 = 30.0;
pub fn spider_heat(r: u8) -> f32 {
    25.0 + 1.0 * up(r)
}
pub fn airship_dmg(r: u8) -> f32 {
    30.0 + 10.0 * up(r)
}
pub const AIRSHIP_CD: f32 = 15.0;
pub fn airship_heat(r: u8) -> f32 {
    35.0 + 1.0 * up(r)
}
pub fn suit_time(r: u8) -> f32 {
    10.0 + 0.4 * r as f32
}
pub const SUIT_CD: f32 = 40.0;
pub fn suit_heat(r: u8) -> f32 {
    40.0 + 1.0 * up(r)
}

pub fn mana_cost(s: Skill, r: u8) -> f32 {
    match s {
        Skill::RayPistol => ray_heat(r),
        Skill::ClockBomb => bomb_heat(r),
        Skill::ArcCoil => arc_heat(r),
        Skill::Turret => turret_heat(r),
        Skill::Grapple => 8.0,
        Skill::TeslaField => tesla_heat(r),
        Skill::Spider => spider_heat(r),
        Skill::AirshipStrike => airship_heat(r),
        Skill::SteamSuit => suit_heat(r),
        _ => 0.0,
    }
}

pub fn cooldown_of(s: Skill) -> f32 {
    match s {
        Skill::Grapple => GRAPPLE_CD,
        Skill::AirshipStrike => AIRSHIP_CD,
        Skill::SteamSuit => SUIT_CD,
        _ => 0.0,
    }
}

pub fn describe(s: Skill, r: u8, power: f32, sk: &Skills) -> Vec<String> {
    let power = power * if s == Skill::Overclock { 1.0 } else { sk.fire_mult() };
    let at = |r: u8| -> String {
        match s {
            Skill::RayPistol => {
                let (a, b) = ray_dmg(r);
                format!("{}-{} DAMAGE, {:.0} HEAT", (a * power) as i32, (b * power) as i32, ray_heat(r))
            }
            Skill::ClockBomb => format!("{} DAMAGE, RADIUS {:.1}, {:.0} HEAT", (bomb_dmg(r) * power) as i32, BOMB_RADIUS, bomb_heat(r)),
            Skill::Tinkerer => format!("{:.0}% FASTER COOLING, GADGETS LAST {:.0}% LONGER", (tinker_cool(r) - 1.0) * 100.0, (tinker_time(r) - 1.0) * 100.0),
            Skill::ArcCoil => format!("{} DAMAGE, CHAINS TO {} FOES, {:.0} HEAT", (arc_dmg(r) * power) as i32, arc_jumps(r), arc_heat(r)),
            Skill::Turret => format!("{} DAMAGE PER SHOT, UP TO {} SWIVEL GUNS, {:.0} HEAT", (turret_dmg(r) * power) as i32, turret_max(r), turret_heat(r)),
            Skill::Grapple => format!("{:.0} TILES, {:.0} SEC COOLDOWN, 8 HEAT", GRAPPLE_RANGE, GRAPPLE_CD),
            Skill::TeslaField => format!("{} DAMAGE/SEC FOR {:.0} SEC, SLOWS, {:.0} HEAT", (tesla_dps(r) * power) as i32, TESLA_TIME, tesla_heat(r)),
            Skill::Spider => format!("{} DAMAGE PER BITE FOR {:.0} SEC, {:.0} HEAT", (spider_dmg(r) * power) as i32, SPIDER_TIME, spider_heat(r)),
            Skill::Overclock => format!("+{}% AETHER DAMAGE, FLINTLOCK {}% FASTER", 8 * r.max(1) as i32, 4 * r.max(1) as i32),
            Skill::AirshipStrike => format!("{} DAMAGE PER BOMB, {:.0} HEAT, {:.0} SEC COOLDOWN", (airship_dmg(r) * power) as i32, airship_heat(r), AIRSHIP_CD),
            Skill::SteamSuit => format!("{:.0} SEC, HALF DAMAGE TAKEN, {:.0} HEAT, {:.0} SEC COOLDOWN", suit_time(r), suit_heat(r), SUIT_CD),
            _ => String::new(),
        }
    };
    let what = match s {
        Skill::RayPistol => "FAST AETHER SHOTS FROM YOUR BRASS FLINTLOCK. A PIRATE'S BREAD AND BUTTER.",
        Skill::ClockBomb => "LOBS A FUSED POWDER KEG. IT BURSTS A SECOND LATER AND HURLS FOES BACK.",
        Skill::Tinkerer => "PASSIVE. YOUR GEAR COOLS FASTER AND YOUR GADGETS LAST LONGER.",
        Skill::ArcCoil => "A BOLT OF AETHER LIGHTNING THAT LEAPS FROM FOE TO FOE.",
        Skill::Turret => "PLANTS A BRASS SWIVEL GUN THAT SHOOTS NEARBY FOES.",
        Skill::Grapple => "FIRES A BOARDING HOOK. ON THE GROUND YOU SWING THERE; ON A FOE YOU HAUL IT ABOARD.",
        Skill::TeslaField => "A CRACKLING FIELD AROUND YOU SHOCKS AND STAGGERS EVERYTHING NEARBY.",
        Skill::Spider => "WINDS UP A CLOCKWORK SPIDER THAT FOLLOWS YOU AND BITES YOUR FOES.",
        Skill::Overclock => "PASSIVE. MORE AETHER DAMAGE, AND THE FLINTLOCK FIRES FASTER.",
        Skill::AirshipStrike => "YOUR PIRATE AIRSHIP SWEEPS OVERHEAD AND FIRES A BROADSIDE DOWN A LINE TOWARD THE TARGET.",
        Skill::SteamSuit => "CLIMB INTO YOUR STEAM SUIT: HALF DAMAGE TAKEN, FASTER, AND YOUR SKILLS RUN COLD.",
        _ => "",
    };
    let mut v = vec![what.to_string()];
    if r > 0 {
        v.push(format!("RANK {r}: {}", at(r)));
    }
    if r < MAX_RANK {
        v.push(format!("NEXT RANK: {}", at(r + 1)));
    }
    v
}

// ------------------------------------------------------------------ effects

pub struct BombFx {
    pub x0: f32,
    pub y0: f32,
    pub x: f32,
    pub y: f32,
    pub t: f32,
    pub dmg: f32,
}
pub const BOMB_FLIGHT: f32 = 0.4;

/// A lightning chain (visual): the points it passed through.
pub struct ArcFx {
    pub pts: Vec<(f32, f32)>,
    pub t: f32,
}

pub struct TurretFx {
    pub x: f32,
    pub y: f32,
    pub t: f32,
    pub life: f32,
    pub shot: f32,
    pub dmg: f32,
}

pub struct SpiderFx {
    pub x: f32,
    pub y: f32,
    pub t: f32,
    pub life: f32,
    pub bite: f32,
    pub dmg: f32,
    pub moving: bool,
}

pub struct AirshipFx {
    pub x: f32,
    pub y: f32,
    pub dx: f32,
    pub dy: f32,
    pub t: f32,
    pub drops: u32,
    pub dmg: f32,
}
pub const AIRSHIP_SPEED: f32 = 9.0;
pub const AIRSHIP_DROPS: u32 = 7;

impl Game {
    pub(crate) fn is_inventor(&self) -> bool {
        self.p.skills.class == crate::skills::Class::Inventor
    }

    /// Heat 0..1 (the inventor's gauge).
    pub fn heat(&self) -> f32 {
        1.0 - (self.p.mana / self.p.max_mana.max(1.0)).clamp(0.0, 1.0)
    }

    /// After every inventor cast: hitting the top of the gauge locks her guns.
    pub(crate) fn check_overheat(&mut self) {
        if self.p.mana <= 0.5 && self.p.overheat <= 0.0 {
            self.p.mana = 0.0;
            self.p.overheat = OVERHEAT_LOCK;
            self.say("OVERHEATED!".into());
            self.sfx.push(Sfx::Hurt);
            for _ in 0..30 {
                let (x, y) = (self.p.x, self.p.y);
                self.spray_at(x, y, PKind::Smoke, 30.0);
            }
        }
    }

    pub(crate) fn cast_inventor(&mut self, s: Skill, tx: f32, ty: f32, r: u8) {
        if self.p.cast_cd > 0.0 {
            return;
        }
        match s {
            Skill::RayPistol => {
                self.cast_fireball(tx, ty, false);
                let oc = self.p.skills.rank(Skill::Overclock) as f32;
                let t = 0.22 * (1.0 - 0.04 * oc).max(0.5);
                self.cast_pose(t);
            }
            Skill::ClockBomb => {
                self.p.mana -= bomb_heat(r);
                self.cast_pose(0.35);
                self.sfx.push(Sfx::Swing);
                let dmg = bomb_dmg(r) * self.fire_power();
                self.bombs.push(BombFx { x0: self.p.x, y0: self.p.y, x: tx, y: ty, t: 0.0, dmg });
            }
            Skill::ArcCoil => self.arc_coil(tx, ty, r),
            Skill::Turret => {
                let n = self.turrets.len();
                if n >= turret_max(r) {
                    self.turrets.remove(0);
                }
                let (dx, dy) = (tx - self.p.x, ty - self.p.y);
                let l = (dx * dx + dy * dy).sqrt().max(0.01);
                let d = l.min(3.0);
                let (mut x, mut y) = (self.p.x + dx / l * d, self.p.y + dy / l * d);
                if self.d.blocked(x, y, 0.3) {
                    (x, y) = (self.p.x, self.p.y);
                }
                self.p.mana -= turret_heat(r);
                self.cast_pose(0.4);
                let life = TURRET_TIME * tinker_time(self.p.skills.rank(Skill::Tinkerer));
                let dmg = turret_dmg(r) * self.fire_power();
                self.turrets.push(TurretFx { x, y, t: 0.0, life, shot: 0.5, dmg });
                self.sfx.push(Sfx::Pickup);
            }
            Skill::Grapple => self.grapple(tx, ty),
            Skill::TeslaField => {
                self.p.mana -= tesla_heat(r);
                self.cast_pose(0.3);
                self.p.tesla_t = TESLA_TIME * tinker_time(self.p.skills.rank(Skill::Tinkerer));
                self.p.tesla_dps = tesla_dps(r) * self.fire_power();
                self.sfx.push(Sfx::Cast);
            }
            Skill::Spider => {
                self.p.mana -= spider_heat(r);
                self.cast_pose(0.45);
                let life = SPIDER_TIME * tinker_time(self.p.skills.rank(Skill::Tinkerer));
                let dmg = spider_dmg(r) * self.fire_power();
                self.spiders.clear();
                self.spiders.push(SpiderFx { x: self.p.x + 0.8, y: self.p.y, t: 0.0, life, bite: 0.0, dmg, moving: false });
                self.sfx.push(Sfx::Pickup);
                self.say("CLOCKWORK SPIDER".into());
            }
            Skill::AirshipStrike => {
                self.p.mana -= airship_heat(r);
                self.cast_pose(0.4);
                self.p.skills.cooldown[Skill::AirshipStrike as usize] = AIRSHIP_CD;
                let (dx, dy) = (tx - self.p.x, ty - self.p.y);
                let l = (dx * dx + dy * dy).sqrt().max(0.01);
                let (ux, uy) = (dx / l, dy / l);
                // It flies in from behind you and over the target.
                let dmg = airship_dmg(r) * self.fire_power();
                self.airships.push(AirshipFx { x: self.p.x - ux * 3.0, y: self.p.y - uy * 3.0, dx: ux, dy: uy, t: 0.0, drops: 0, dmg });
                self.say("BROADSIDE! ALL HANDS!".into());
            }
            Skill::SteamSuit => {
                self.p.mana -= suit_heat(r);
                self.cast_pose(0.6);
                self.p.skills.cooldown[Skill::SteamSuit as usize] = SUIT_CD;
                self.p.suit_t = suit_time(r);
                self.sfx.push(Sfx::Descend);
                self.say("STEAM SUIT".into());
                for _ in 0..40 {
                    let (x, y) = (self.p.x, self.p.y);
                    self.spray_at(x, y, PKind::Smoke, 20.0);
                }
            }
            _ => {}
        }
        self.check_overheat();
    }

    fn arc_coil(&mut self, tx: f32, ty: f32, r: u8) {
        self.p.mana -= arc_heat(r);
        self.cast_pose(0.3);
        self.sfx.push(Sfx::Cast);
        let mut pts = vec![(self.p.x, self.p.y)];
        let mut hit: Vec<usize> = vec![];
        let mut from = (tx, ty);
        let mut reach = 7.0f32;
        let mut dmg = arc_dmg(r) * self.fire_power();
        for _ in 0..=arc_jumps(r) {
            let next = (0..self.mobs.len())
                .filter(|&i| self.mobs[i].alive() && self.mobs[i].charm <= 0.0 && !hit.contains(&i))
                .map(|i| (i, (self.mobs[i].x - from.0).powi(2) + (self.mobs[i].y - from.1).powi(2)))
                .filter(|&(_, d2)| d2 < reach * reach)
                .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
                .map(|v| v.0);
            let Some(i) = next else { break };
            let (mx, my) = (self.mobs[i].x, self.mobs[i].y);
            pts.push((mx, my));
            hit.push(i);
            self.hit_mob(i, dmg, 0.0, 0.15, None, true);
            from = (mx, my);
            reach = 3.5;
            dmg *= 0.85;
        }
        if pts.len() == 1 {
            pts.push((tx, ty));
        }
        self.arcs.push(ArcFx { pts, t: 0.0 });
    }

    fn grapple(&mut self, tx: f32, ty: f32) {
        self.p.mana -= 8.0;
        self.cast_pose(0.3);
        self.p.skills.cooldown[Skill::Grapple as usize] = GRAPPLE_CD;
        self.sfx.push(Sfx::Swing);
        let (px, py) = (self.p.x, self.p.y);
        // A foe near the target: yank it to you.
        let foe = (0..self.mobs.len())
            .filter(|&i| self.mobs[i].alive() && self.mobs[i].charm <= 0.0 && !self.mobs[i].boss)
            .find(|&i| (self.mobs[i].x - tx).powi(2) + (self.mobs[i].y - ty).powi(2) < 1.2);
        let (dx, dy) = (tx - px, ty - py);
        let l = (dx * dx + dy * dy).sqrt().max(0.01);
        let (ux, uy) = (dx / l, dy / l);
        if let Some(i) = foe.filter(|&i| (self.mobs[i].x - px).powi(2) + (self.mobs[i].y - py).powi(2) < GRAPPLE_RANGE * GRAPPLE_RANGE) {
            let (mx, my) = (self.mobs[i].x, self.mobs[i].y);
            let (nx, ny) = (px + ux * 1.1, py + uy * 1.1);
            if !self.d.blocked(nx, ny, self.mobs[i].r) {
                self.mobs[i].x = nx;
                self.mobs[i].y = ny;
            }
            self.mobs[i].stun = self.mobs[i].stun.max(0.8);
            if self.mobs[i].state == MobState::Idle {
                self.mobs[i].state = MobState::Chase;
            }
            self.arcs.push(ArcFx { pts: vec![(px, py), (mx, my)], t: 0.0 });
            return;
        }
        // Otherwise zip along the line until a wall or a monster.
        let mut best = (px, py);
        let mut d = 0.25;
        while d <= l.min(GRAPPLE_RANGE) {
            let (x, y) = (px + ux * d, py + uy * d);
            if self.d.blocked(x, y, crate::game::PLAYER_R) || self.mobs.iter().any(|m| m.alive() && (m.x - x).powi(2) + (m.y - y).powi(2) < (m.r + 0.3).powi(2)) {
                break;
            }
            best = (x, y);
            d += 0.25;
        }
        self.arcs.push(ArcFx { pts: vec![(px, py), best], t: 0.0 });
        (self.p.x, self.p.y) = best;
        self.p.path.clear();
        self.p.goal = None;
    }

    /// Y / E: vent steam (dumps all heat, blasts foes back), or drink coolant if the vent isn't ready.
    pub(crate) fn vent(&mut self) -> bool {
        if self.p.vent_cd > 0.0 {
            return false;
        }
        self.p.vent_cd = VENT_CD;
        self.p.mana = self.p.max_mana;
        self.p.overheat = 0.0;
        self.sfx.push(Sfx::Boom);
        self.say("VENTED".into());
        let (px, py) = (self.p.x, self.p.y);
        for k in 0..40 {
            let a = k as f32 / 40.0 * std::f32::consts::TAU;
            self.parts.push(Particle { x: px, y: py, z: 12.0, vx: a.cos() * 5.0, vy: a.sin() * 5.0, vz: 20.0, life: 0.7, max: 0.7, kind: PKind::Smoke });
        }
        let near: Vec<usize> = (0..self.mobs.len())
            .filter(|&i| self.mobs[i].alive() && (self.mobs[i].x - px).powi(2) + (self.mobs[i].y - py).powi(2) < VENT_RADIUS * VENT_RADIUS)
            .collect();
        let dmg = 6.0 * self.fire_power();
        for i in near {
            self.hit_mob(i, dmg, 0.0, 0.4, Some((px, py, 1.6)), true);
        }
        true
    }

    /// Bombs, arcs, turrets, the spider, airships, the Tesla field and the suit, every tick.
    pub(crate) fn update_inventor(&mut self) {
        self.p.overheat = (self.p.overheat - DT).max(0.0);
        self.p.vent_cd = (self.p.vent_cd - DT).max(0.0);
        self.p.suit_t = (self.p.suit_t - DT).max(0.0);
        for a in self.arcs.iter_mut() {
            a.t += DT;
        }
        self.arcs.retain(|a| a.t < 0.25);
        // Bombs fly, then tick, then burst.
        let mut booms = vec![];
        for b in self.bombs.iter_mut() {
            b.t += DT;
            if b.t >= BOMB_FLIGHT + BOMB_FUSE {
                booms.push((b.x, b.y, b.dmg));
            }
        }
        self.bombs.retain(|b| b.t < BOMB_FLIGHT + BOMB_FUSE);
        for (x, y, dmg) in booms {
            self.blast(x, y, BOMB_RADIUS, dmg, 0.0);
        }
        // Turrets shoot the nearest foe in sight.
        let mut shots = vec![];
        for tu in self.turrets.iter_mut() {
            tu.t += DT;
            tu.shot -= DT;
            if tu.shot <= 0.0 {
                let target = self
                    .mobs
                    .iter()
                    .filter(|m| m.alive() && m.charm <= 0.0)
                    .map(|m| (m.x, m.y, (m.x - tu.x).powi(2) + (m.y - tu.y).powi(2)))
                    .filter(|t| t.2 < 49.0 && self.d.los(tu.x, tu.y, t.0, t.1))
                    .min_by(|a, b| a.2.partial_cmp(&b.2).unwrap());
                if let Some((mx, my, _)) = target {
                    tu.shot = 0.55;
                    shots.push((tu.x, tu.y, mx, my, tu.dmg));
                }
            }
        }
        self.turrets.retain(|t| t.t < t.life);
        for (x, y, mx, my, dmg) in shots {
            let (dx, dy) = (mx - x, my - y);
            let l = (dx * dx + dy * dy).sqrt().max(0.01);
            self.balls.push(crate::game::Fireball { x: x + dx / l * 0.4, y: y + dy / l * 0.4, vx: dx / l * 12.0, vy: dy / l * 12.0, life: 0.8, dmg, ember: true });
            self.sfx.push(Sfx::Cast);
        }
        // The spider follows you, and jumps on the nearest foe.
        let (px, py) = (self.p.x, self.p.y);
        let mut bites = vec![];
        for sp in self.spiders.iter_mut() {
            sp.t += DT;
            sp.bite -= DT;
            let target = self
                .mobs
                .iter()
                .enumerate()
                .filter(|(_, m)| m.alive() && m.charm <= 0.0)
                .map(|(i, m)| (i, m.x, m.y, (m.x - sp.x).powi(2) + (m.y - sp.y).powi(2)))
                .filter(|t| t.3 < 36.0)
                .min_by(|a, b| a.3.partial_cmp(&b.3).unwrap());
            let (gx, gy, close) = match target {
                Some((i, mx, my, d2)) => {
                    if d2 < 0.8 && sp.bite <= 0.0 {
                        sp.bite = 0.7;
                        bites.push((i, sp.dmg));
                    }
                    (mx, my, 0.6)
                }
                None => (px, py, 1.2),
            };
            let (dx, dy) = (gx - sp.x, gy - sp.y);
            let l = (dx * dx + dy * dy).sqrt();
            sp.moving = l > close;
            if sp.moving {
                let step = (5.5 * DT).min(l - close);
                let (mut x, mut y) = (sp.x, sp.y);
                crate::game::move_circle(&self.d, &mut x, &mut y, dx / l * step, dy / l * step, 0.25);
                sp.x = x;
                sp.y = y;
            }
            // Never lose it: if it falls far behind, it skitters back to you.
            if (sp.x - px).powi(2) + (sp.y - py).powi(2) > 400.0 {
                sp.x = px;
                sp.y = py;
            }
        }
        self.spiders.retain(|s| s.t < s.life);
        for (i, dmg) in bites {
            if self.mobs[i].alive() {
                self.hit_mob(i, dmg, 0.0, 0.1, None, true);
            }
        }
        // Airships fly the line, dropping bombs.
        let mut drops = vec![];
        for a in self.airships.iter_mut() {
            a.t += DT;
            a.x += a.dx * AIRSHIP_SPEED * DT;
            a.y += a.dy * AIRSHIP_SPEED * DT;
            let due = ((a.t - 0.3) / 0.18).floor() as i32;
            while (a.drops as i32) < due.min(AIRSHIP_DROPS as i32) {
                a.drops += 1;
                drops.push((a.x, a.y, a.dmg));
            }
        }
        self.airships.retain(|a| a.t < 2.2);
        for (x, y, dmg) in drops {
            self.blast(x, y, 1.4, dmg, 0.0);
        }
        // Tesla field: shocks everything around you twice a second.
        if self.p.tesla_t > 0.0 {
            let before = self.p.tesla_t;
            self.p.tesla_t -= DT;
            if (before * 2.0).floor() != (self.p.tesla_t * 2.0).floor() {
                let dmg = self.p.tesla_dps * 0.5;
                let near: Vec<usize> = (0..self.mobs.len())
                    .filter(|&i| {
                        let m = &self.mobs[i];
                        m.alive() && m.charm <= 0.0 && (m.x - px).powi(2) + (m.y - py).powi(2) < (TESLA_RADIUS + m.r).powi(2)
                    })
                    .collect();
                for i in near {
                    let (mx, my) = (self.mobs[i].x, self.mobs[i].y);
                    self.hit_mob(i, dmg, 0.0, 0.25, None, false);
                    self.arcs.push(ArcFx { pts: vec![(px, py), (mx, my)], t: 0.1 });
                }
            }
        }
        if self.p.overheat > 0.0 && self.tick % 3 == 0 {
            self.parts.push(Particle { x: px, y: py, z: 30.0, vx: 0.0, vy: 0.0, vz: 25.0, life: 0.6, max: 0.6, kind: PKind::Smoke });
        }
        let _ = Light { x: 0.0, y: 0.0, r: 0.0, s: 0.0, life: 0.0, max: 0.0 };
        let _ = rgb(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skills::Class;

    #[test]
    fn the_inventor_has_her_own_tree() {
        let sk = Skills::new(Class::Inventor);
        assert_eq!(sk.rank(Skill::RayPistol), 1);
        assert_eq!(sk.actives(), vec![Skill::RayPistol]);
        assert_eq!(sk.tree().len(), 11);
        for s in sk.tree() {
            assert!(is_inventor(*s));
            assert!(!describe(*s, 1, 1.0, &sk).is_empty());
        }
        let back = Skills::load_text(&sk.save_text()).unwrap();
        assert_eq!(back.class, Class::Inventor);
    }
}

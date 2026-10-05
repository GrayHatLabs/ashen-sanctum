//! The Valkyrie, the fourth playable class and the first melee hero (docs/VALKYRIE_CLASS.md):
//! a frost spear-warrior. Her hits are half steel, half frost: frost builds up on a foe until it
//! freezes solid, and frozen foes she kills shatter into shards. She runs on Valor instead of mana:
//! it starts empty, her Rune Spear hits and the blows she takes fill it, her skills spend it, and
//! it drains away out of combat. At full Valor her runes blaze (+15% damage).
use crate::game::{move_circle, Game, Light, PKind, Particle, Sfx, DT, PLAYER_R};
use crate::gfx::rgb;
use crate::mobs::{Kind, MobState};
use crate::skills::{Nova, Skill, Skills, MAX_RANK};

pub fn is_valkyrie(s: Skill) -> bool {
    crate::skills::VALKYRIE.contains(&s)
}

// ------------------------------------------------------------------ Valor

/// Valor from each foe a Rune Spear thrust hits (at most two foes count).
pub const VALOR_PER_HIT: f32 = 7.0;
/// Valor per point of damage she takes.
pub const VALOR_PER_HURT: f32 = 0.8;
/// How long after a hit she counts as "in combat" (Valor doesn't drain).
pub const FIGHT_TIME: f32 = 4.0;
/// Valor lost per second out of combat.
pub const VALOR_DECAY: f32 = 8.0;
/// Damage bonus while her Valor is full.
pub const BLAZING: f32 = 1.15;

// ------------------------------------------------------------------ numbers per rank

fn up(r: u8) -> f32 {
    (r.max(1) - 1) as f32
}
pub fn spear_dmg(r: u8) -> (f32, f32) {
    let k = 1.0 + 0.16 * up(r);
    (9.0 * k, 15.0 * k)
}
pub const SPEAR_REACH: f32 = 2.0;
pub fn sweep_dmg(r: u8) -> (f32, f32) {
    let k = 1.0 + 0.15 * up(r);
    (8.0 * k, 13.0 * k)
}
pub const SWEEP_REACH: f32 = 2.1;
pub fn sweep_valor(r: u8) -> f32 {
    16.0 + 0.4 * up(r)
}
/// Northborn: share of incoming damage she shrugs off.
pub fn northborn_dr(r: u8) -> f32 {
    if r == 0 {
        0.0
    } else {
        (0.06 + 0.02 * up(r)).min(0.3)
    }
}
pub fn raven_dmg(r: u8) -> f32 {
    7.0 + 2.5 * up(r)
}
pub fn mark_bonus(r: u8) -> f32 {
    0.25 + 0.03 * up(r)
}
pub const MARK_TIME: f32 = 6.0;
pub fn raven_valor(r: u8) -> f32 {
    12.0 + 0.3 * up(r)
}
pub fn leap_dist(r: u8) -> f32 {
    5.0 + 0.2 * r as f32
}
pub fn leap_dmg(r: u8) -> f32 {
    14.0 + 5.0 * up(r)
}
pub fn leap_cd(r: u8) -> f32 {
    (4.0 - 0.15 * r as f32).max(2.0)
}
pub const LEAP_RADIUS: f32 = 2.0;
pub fn leap_valor(r: u8) -> f32 {
    14.0 + 0.4 * up(r)
}
/// Frost Brand: how long a frozen foe stays frozen.
pub fn freeze_time(r: u8) -> f32 {
    1.6 + 0.12 * r as f32
}
pub fn shatter_dmg(r: u8) -> f32 {
    10.0 + 4.0 * up(r)
}
pub const SHATTER_RADIUS: f32 = 2.0;
pub fn javelin_dmg(r: u8) -> (f32, f32) {
    let k = 1.0 + 0.17 * up(r);
    (14.0 * k, 22.0 * k)
}
pub const JAVELIN_RANGE: f32 = 8.0;
pub fn javelin_valor(r: u8) -> f32 {
    20.0 + 0.5 * up(r)
}
pub fn wrath_dmg(r: u8) -> f32 {
    16.0 + 6.0 * up(r)
}
pub const WRATH_RADIUS: f32 = 2.4;
pub fn wrath_valor(r: u8) -> f32 {
    24.0 + 0.6 * up(r)
}
pub fn einherjar_count(r: u8) -> usize {
    2 + r as usize / 4
}
pub fn einherjar_time(r: u8) -> f32 {
    20.0 + 2.0 * r as f32
}
pub fn einherjar_valor(r: u8) -> f32 {
    30.0 + 0.8 * up(r)
}
pub fn charge_dmg(r: u8) -> f32 {
    25.0 + 8.0 * up(r)
}
pub const CHARGE_DIST: f32 = 8.0;
pub const CHARGE_SPEED: f32 = 15.0;
pub const CHARGE_CD: f32 = 12.0;
pub fn charge_valor(r: u8) -> f32 {
    35.0 + 1.0 * up(r)
}
pub fn fimbul_dps(r: u8) -> f32 {
    14.0 + 5.0 * up(r)
}
pub const FIMBUL_RADIUS: f32 = 3.5;
pub const FIMBUL_TIME: f32 = 6.0;
pub const FIMBUL_CD: f32 = 30.0;
pub fn fimbul_valor(r: u8) -> f32 {
    50.0 + 1.0 * up(r)
}

pub fn mana_cost(s: Skill, r: u8) -> f32 {
    match s {
        Skill::RimeSweep => sweep_valor(r),
        Skill::RavenStrike => raven_valor(r),
        Skill::GlacierLeap => leap_valor(r),
        Skill::RuneJavelin => javelin_valor(r),
        Skill::WintersWrath => wrath_valor(r),
        Skill::Einherjar => einherjar_valor(r),
        Skill::ValkyrieRide => charge_valor(r),
        Skill::Fimbulwinter => fimbul_valor(r),
        _ => 0.0,
    }
}

pub fn cooldown_of(s: Skill) -> f32 {
    match s {
        Skill::GlacierLeap => 2.0,
        Skill::ValkyrieRide => CHARGE_CD,
        Skill::Fimbulwinter => FIMBUL_CD,
        _ => 0.0,
    }
}

/// Melee skills: clicking a foe out of reach walks her in first.
pub fn reach_of(s: Skill) -> Option<f32> {
    match s {
        Skill::RuneSpear => Some(SPEAR_REACH),
        Skill::RimeSweep => Some(SWEEP_REACH),
        Skill::WintersWrath => Some(WRATH_RADIUS),
        _ => None,
    }
}

/// Her hits are half steel, half frost: creatures of the cold shrug off the frost half,
/// creatures of fire take extra from it.
pub fn frost_taken(k: Kind) -> f32 {
    let frost = if crate::mobs::def(k).cold {
        0.2
    } else if matches!(k, Kind::AshKing | Kind::Forgemother | Kind::BoilerBrute) {
        1.5
    } else {
        1.0
    };
    0.5 + 0.5 * frost
}

pub fn describe(s: Skill, r: u8, power: f32, sk: &Skills) -> Vec<String> {
    let power = power * sk.fire_mult();
    let at = |r: u8| -> String {
        match s {
            Skill::RuneSpear => {
                let (a, b) = spear_dmg(r);
                format!("{}-{} DAMAGE, REACH {:.0}, BUILDS VALOR", (a * power) as i32, (b * power) as i32, SPEAR_REACH)
            }
            Skill::RimeSweep => {
                let (a, b) = sweep_dmg(r);
                format!("{}-{} DAMAGE, CHILLS, {:.0} VALOR", (a * power) as i32, (b * power) as i32, sweep_valor(r))
            }
            Skill::Northborn => format!("TAKE {:.0}% LESS DAMAGE, CHILL WEARS OFF TWICE AS FAST", northborn_dr(r.max(1)) * 100.0),
            Skill::RavenStrike => format!(
                "{} DAMAGE, MARKED FOES TAKE +{:.0}% FROM YOU FOR {:.0} SEC, {:.0} VALOR",
                (raven_dmg(r) * power) as i32,
                mark_bonus(r) * 100.0,
                MARK_TIME,
                raven_valor(r)
            ),
            Skill::GlacierLeap => format!(
                "{:.1} TILES, {} DAMAGE ON LANDING, {:.1} SEC COOLDOWN, {:.0} VALOR",
                leap_dist(r),
                (leap_dmg(r) * power) as i32,
                leap_cd(r),
                leap_valor(r)
            ),
            Skill::FrostBrand => format!("FROZEN FOR {:.1} SEC, SHATTER {} DAMAGE", freeze_time(r.max(1)), (shatter_dmg(r.max(1)) * power) as i32),
            Skill::RuneJavelin => {
                let (a, b) = javelin_dmg(r);
                format!("{}-{} DAMAGE, PIERCES, RETURNS, {:.0} VALOR", (a * power) as i32, (b * power) as i32, javelin_valor(r))
            }
            Skill::WintersWrath => format!("{} DAMAGE ALL AROUND, CHILLS, {:.0} VALOR", (wrath_dmg(r) * power) as i32, wrath_valor(r)),
            Skill::Einherjar => format!("{} WARRIORS FOR {:.0} SEC, {:.0} VALOR", einherjar_count(r), einherjar_time(r), einherjar_valor(r)),
            Skill::ValkyrieRide => format!(
                "{} DAMAGE, {:.0} TILE CHARGE, {:.0} SEC COOLDOWN, {:.0} VALOR",
                (charge_dmg(r) * power) as i32,
                CHARGE_DIST,
                CHARGE_CD,
                charge_valor(r)
            ),
            Skill::Fimbulwinter => format!(
                "{} DAMAGE/SEC FOR {:.0} SEC, FREEZES, {:.0} SEC COOLDOWN, {:.0} VALOR",
                (fimbul_dps(r) * power) as i32,
                FIMBUL_TIME,
                FIMBUL_CD,
                fimbul_valor(r)
            ),
            _ => String::new(),
        }
    };
    let what = match s {
        Skill::RuneSpear => "A FAST REACHING THRUST THROUGH EVERYTHING IN A SHORT LINE. EVERY HIT BUILDS VALOR.",
        Skill::RimeSweep => "A WIDE FROST ARC IN FRONT OF YOU: HITS EVERY FOE IN IT AND CHILLS THEM.",
        Skill::Northborn => "PASSIVE. THE COLD NORTH HARDENED YOU: YOU SHRUG OFF PART OF EVERY BLOW, AND FROST BARELY SLOWS YOU.",
        Skill::RavenStrike => "YOUR RAVEN DIVES AT A FOE AND MARKS IT. YOU HIT MARKED FOES HARDER.",
        Skill::GlacierLeap => "YOUR ICE WINGS SPREAD: LEAP TO A SPOT AND LAND IN A FREEZING SHOCKWAVE.",
        Skill::FrostBrand => "PASSIVE. ENOUGH FROST FREEZES A FOE SOLID. FROZEN FOES YOU KILL SHATTER INTO SHARDS THAT HIT THEIR FRIENDS.",
        Skill::RuneJavelin => "HURL A SPECTRAL COPY OF YOUR SPEAR: IT PIERCES A LINE OF FOES AND FLIES BACK TO YOU.",
        Skill::WintersWrath => "WHIRL THE SPEAR IN A FULL CIRCLE, STRIKING EVERYTHING AROUND YOU.",
        Skill::Einherjar => "SPECTRAL WARRIORS RISE FROM THE SNOW AND FIGHT BESIDE YOU FOR A WHILE.",
        Skill::ValkyrieRide => "YOUR FROST WARHORSE CHARGES IN: YOU TRAMPLE A LONG LINE OF FOES AND THROW THEM ASIDE.",
        Skill::Fimbulwinter => "THE GREAT WINTER. YOUR WINGS SPREAD AND A KILLING BLIZZARD RAGES AROUND YOU, FREEZING ALL INSIDE.",
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

/// Her raven, diving at a foe.
pub struct RavenFx {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub life: f32,
    pub dmg: f32,
    pub mark: f32,
    /// The foe it was sent at (it still homes on the nearest one if that dies).
    pub target: Option<usize>,
}

/// A spectral javelin: out through a line of foes, then back to her hand.
pub struct JavelinFx {
    pub x: f32,
    pub y: f32,
    pub ux: f32,
    pub uy: f32,
    pub gone: f32,
    pub back: bool,
    pub dmg: f32,
    pub hit: Vec<usize>,
}

/// The warhorse charge in progress.
#[derive(Clone)]
pub struct Charge {
    pub ux: f32,
    pub uy: f32,
    pub left: f32,
    pub dmg: f32,
    pub hit: Vec<usize>,
}

/// Angle between two directions, in -PI..PI.
fn angle_off(a: f32, b: f32) -> f32 {
    (a - b + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI
}

impl Game {
    pub(crate) fn is_valkyrie(&self) -> bool {
        self.p.skills.class == crate::skills::Class::Valkyrie
    }

    /// Valor full: her runes blaze.
    pub(crate) fn blazing(&self) -> bool {
        self.is_valkyrie() && self.p.mana >= self.p.max_mana - 0.01
    }

    pub(crate) fn gain_valor(&mut self, v: f32) {
        let was = self.blazing();
        self.p.mana = (self.p.mana + v).min(self.p.max_mana);
        self.p.fight_t = FIGHT_TIME;
        if !was && self.blazing() {
            let (x, y) = (self.p.x, self.p.y);
            self.floater(x, y, "VALOR!".into(), rgb(0xa0e0ff));
        }
    }

    /// Hostile, living monsters (indices) that a test says yes to.
    fn foes_where(&self, f: impl Fn(&crate::mobs::Mob) -> bool) -> Vec<usize> {
        (0..self.mobs.len()).filter(|&i| self.mobs[i].alive() && self.mobs[i].charm <= 0.0 && f(&self.mobs[i])).collect()
    }

    /// Foes in a line from her toward (tx, ty).
    fn in_line(&self, tx: f32, ty: f32, reach: f32, width: f32) -> Vec<usize> {
        let (px, py) = (self.p.x, self.p.y);
        let (dx, dy) = (tx - px, ty - py);
        let l = (dx * dx + dy * dy).sqrt().max(0.01);
        let (ux, uy) = (dx / l, dy / l);
        self.foes_where(|m| {
            let (mx, my) = (m.x - px, m.y - py);
            let along = mx * ux + my * uy;
            let side = (mx * -uy + my * ux).abs();
            along > -0.2 && along < reach + m.r && side < width + m.r
        })
    }

    /// Foes in a cone in front of her.
    fn in_cone(&self, tx: f32, ty: f32, reach: f32, half: f32) -> Vec<usize> {
        let (px, py) = (self.p.x, self.p.y);
        let a0 = (ty - py).atan2(tx - px);
        self.foes_where(|m| {
            let (dx, dy) = (m.x - px, m.y - py);
            dx * dx + dy * dy < (reach + m.r).powi(2) && (dx * dx + dy * dy < 0.36 || angle_off(dy.atan2(dx), a0).abs() < half)
        })
    }

    fn in_circle(&self, x: f32, y: f32, r: f32) -> Vec<usize> {
        self.foes_where(|m| (m.x - x).powi(2) + (m.y - y).powi(2) < (r + m.r).powi(2))
    }

    /// One of her blows: the mark bonus, the hit, then frost (which may freeze it).
    pub(crate) fn valk_hit(&mut self, i: usize, dmg: f32, frost: f32, stun: f32, knock: Option<(f32, f32, f32)>) {
        let r = self.p.skills.rank(Skill::RavenStrike);
        let dmg = if self.mobs[i].marked > 0.0 { dmg * (1.0 + mark_bonus(r.max(1))) } else { dmg };
        self.hit_mob(i, dmg, 0.0, stun, knock, true);
        if self.mobs[i].alive() {
            self.add_frost(i, frost);
        }
    }

    /// Builds frost on a foe; with Frost Brand, a full load freezes it solid.
    pub(crate) fn add_frost(&mut self, i: usize, frost: f32) {
        let brand = self.p.skills.rank(Skill::FrostBrand);
        let m = &mut self.mobs[i];
        if m.frozen > 0.0 {
            return;
        }
        m.frost = (m.frost + frost).min(1.0);
        if brand > 0 && m.frost >= 1.0 {
            let t = freeze_time(brand) * if m.boss { 0.25 } else { 1.0 };
            m.frozen = t;
            m.stun = m.stun.max(t);
            m.frost = 0.0;
            let (x, y) = (m.x, m.y);
            self.floater(x, y, "FROZEN".into(), rgb(0xa0e8ff));
            for _ in 0..10 {
                self.spray_at(x, y, PKind::Frost, 16.0);
            }
        }
    }

    /// A frozen foe she kills bursts into ice shards (called from `kill`).
    pub(crate) fn shatter(&mut self, x: f32, y: f32) {
        let r = self.p.skills.rank(Skill::FrostBrand).max(1);
        let dmg = shatter_dmg(r) * self.fire_power();
        self.sfx.push(Sfx::Boom);
        self.novas.push(Nova { x, y, r: SHATTER_RADIUS, t: 0.0, blood: false, frost: true });
        for k in 0..24 {
            let a = k as f32 / 24.0 * std::f32::consts::TAU;
            self.parts.push(Particle { x, y, z: 14.0, vx: a.cos() * 6.0, vy: a.sin() * 6.0, vz: 20.0, life: 0.45, max: 0.45, kind: PKind::Frost });
        }
        self.floater(x, y, "SHATTER".into(), rgb(0xd0f4ff));
        for i in self.in_circle(x, y, SHATTER_RADIUS) {
            if self.mobs[i].alive() {
                self.hit_mob(i, dmg, 0.0, 0.2, Some((x, y, 0.6)), true);
            }
        }
    }

    /// Her pose for the next cast (thrust / sweep / whirl / throw / cast).
    fn valk_pose(&mut self, pose: &'static str, t: f32) {
        self.cast_pose(t);
        self.p.pose = pose;
    }

    /// Rune Spear: free, and the source of her Valor. Also her out-of-Valor attack.
    pub(crate) fn rune_spear(&mut self, tx: f32, ty: f32) {
        if self.p.cast_cd > 0.0 {
            return;
        }
        let r = self.p.skills.rank(Skill::RuneSpear).max(1);
        self.valk_pose("attack", 0.32);
        self.sfx.push(Sfx::Swing);
        self.stats.casts += 1;
        let (px, py) = (self.p.x, self.p.y);
        let (dx, dy) = (tx - px, ty - py);
        let l = (dx * dx + dy * dy).sqrt().max(0.01);
        for k in 0..8 {
            let d = 0.4 + k as f32 * 0.22;
            self.parts.push(Particle { x: px + dx / l * d, y: py + dy / l * d, z: 18.0, vx: dx / l, vy: dy / l, vz: 0.0, life: 0.18, max: 0.18, kind: PKind::Frost });
        }
        let (lo, hi) = spear_dmg(r);
        let power = self.fire_power();
        let hits = self.in_line(tx, ty, SPEAR_REACH, 0.45);
        for (n, i) in hits.into_iter().enumerate() {
            let dmg = self.rng.rf(lo, hi) * power;
            self.valk_hit(i, dmg, 0.18, 0.15, Some((px, py, 0.25)));
            if n < 2 {
                self.gain_valor(VALOR_PER_HIT);
            }
        }
    }

    pub(crate) fn cast_valkyrie(&mut self, s: Skill, tx: f32, ty: f32, r: u8) {
        if self.p.cast_cd > 0.0 {
            return;
        }
        match s {
            Skill::RuneSpear => self.rune_spear(tx, ty),
            Skill::RimeSweep => self.rime_sweep(tx, ty, r),
            Skill::RavenStrike => self.raven_strike(tx, ty, r),
            Skill::GlacierLeap => self.glacier_leap(tx, ty, r),
            Skill::RuneJavelin => self.rune_javelin(tx, ty, r),
            Skill::WintersWrath => self.winters_wrath(r),
            Skill::Einherjar => self.einherjar(r),
            Skill::ValkyrieRide => self.valkyrie_ride(tx, ty, r),
            Skill::Fimbulwinter => self.fimbulwinter(r),
            _ => {}
        }
    }

    fn rime_sweep(&mut self, tx: f32, ty: f32, r: u8) {
        self.p.mana -= sweep_valor(r);
        self.valk_pose("sweep", 0.4);
        self.sfx.push(Sfx::Swing);
        let (px, py) = (self.p.x, self.p.y);
        let a0 = (ty - py).atan2(tx - px);
        for k in 0..16 {
            let a = a0 - 0.95 + k as f32 * 0.127;
            self.parts.push(Particle { x: px + a.cos() * 1.4, y: py + a.sin() * 1.4, z: 14.0, vx: a.cos() * 2.0, vy: a.sin() * 2.0, vz: 6.0, life: 0.3, max: 0.3, kind: PKind::Frost });
        }
        let (lo, hi) = sweep_dmg(r);
        let power = self.fire_power();
        for i in self.in_cone(tx, ty, SWEEP_REACH, 0.95) {
            let dmg = self.rng.rf(lo, hi) * power;
            self.valk_hit(i, dmg, 0.4, 0.25, Some((px, py, 0.4)));
        }
    }

    fn raven_strike(&mut self, tx: f32, ty: f32, r: u8) {
        let target = self.foes_where(|_| true).into_iter().min_by(|&a, &b| {
            let da = (self.mobs[a].x - tx).powi(2) + (self.mobs[a].y - ty).powi(2);
            let db = (self.mobs[b].x - tx).powi(2) + (self.mobs[b].y - ty).powi(2);
            da.partial_cmp(&db).unwrap()
        });
        let Some(i) = target.filter(|&i| (self.mobs[i].x - tx).powi(2) + (self.mobs[i].y - ty).powi(2) < 16.0) else {
            self.say("NO FOE FOR THE RAVEN THERE".into());
            self.p.cast_cd = 0.3;
            return;
        };
        self.p.mana -= raven_valor(r);
        self.valk_pose("cast", 0.3);
        self.sfx.push(Sfx::Cast);
        let (px, py) = (self.p.x, self.p.y);
        let (dx, dy) = (self.mobs[i].x - px, self.mobs[i].y - py);
        let l = (dx * dx + dy * dy).sqrt().max(0.01);
        self.ravens.push(RavenFx { x: px, y: py, vx: dx / l * 6.0 - dy / l * 3.0, vy: dy / l * 6.0 + dx / l * 3.0, life: 3.0, dmg: raven_dmg(r) * self.fire_power(), mark: MARK_TIME, target: Some(i) });
    }

    fn glacier_leap(&mut self, tx: f32, ty: f32, r: u8) {
        let (px, py) = (self.p.x, self.p.y);
        let (dx, dy) = (tx - px, ty - py);
        let l = (dx * dx + dy * dy).sqrt().max(0.01);
        let (ux, uy) = (dx / l, dy / l);
        // Over foes, never through walls; land on the last open spot up to the target.
        let mut best = None;
        let mut d = 0.5;
        while d <= l.min(leap_dist(r)) + 0.01 {
            let (x, y) = (px + ux * d, py + uy * d);
            if !self.d.walkable(x.floor() as i32, y.floor() as i32) {
                break;
            }
            if !self.d.blocked(x, y, PLAYER_R) {
                best = Some((x, y));
            }
            d += 0.25;
        }
        let Some((lx, ly)) = best else {
            self.say("NOWHERE TO LAND".into());
            self.p.cast_cd = 0.3;
            return;
        };
        self.p.mana -= leap_valor(r);
        self.p.skills.cooldown[Skill::GlacierLeap as usize] = leap_cd(r);
        self.valk_pose("cast", 0.3);
        self.p.wings_t = 0.5;
        for k in 0..20 {
            let t = k as f32 / 20.0;
            let (x, y) = (px + (lx - px) * t, py + (ly - py) * t);
            self.parts.push(Particle { x, y, z: 10.0 + (t * std::f32::consts::PI).sin() * 30.0, vx: 0.0, vy: 0.0, vz: 4.0, life: 0.5, max: 0.5, kind: PKind::Frost });
        }
        (self.p.x, self.p.y) = (lx, ly);
        self.p.path.clear();
        self.p.goal = None;
        self.sfx.push(Sfx::Boom);
        self.shake = self.shake.max(0.5);
        self.novas.push(Nova { x: lx, y: ly, r: LEAP_RADIUS, t: 0.0, blood: false, frost: true });
        self.lights.push(Light { x: lx, y: ly, r: 160.0, s: 0.8, life: 0.3, max: 0.3 });
        let dmg = leap_dmg(r) * self.fire_power();
        for i in self.in_circle(lx, ly, LEAP_RADIUS) {
            self.valk_hit(i, dmg, 0.5, 0.4, Some((lx, ly, 0.7)));
        }
    }

    fn rune_javelin(&mut self, tx: f32, ty: f32, r: u8) {
        self.p.mana -= javelin_valor(r);
        self.valk_pose("throw", 0.35);
        self.sfx.push(Sfx::Cast);
        let (px, py) = (self.p.x, self.p.y);
        let (dx, dy) = (tx - px, ty - py);
        let l = (dx * dx + dy * dy).sqrt().max(0.01);
        let (lo, hi) = javelin_dmg(r);
        let dmg = self.rng.rf(lo, hi) * self.fire_power();
        self.javelins.push(JavelinFx { x: px + dx / l * 0.4, y: py + dy / l * 0.4, ux: dx / l, uy: dy / l, gone: 0.0, back: false, dmg, hit: vec![] });
    }

    fn winters_wrath(&mut self, r: u8) {
        self.p.mana -= wrath_valor(r);
        self.valk_pose("whirl", 0.5);
        self.sfx.push(Sfx::Swing);
        let (px, py) = (self.p.x, self.p.y);
        self.novas.push(Nova { x: px, y: py, r: WRATH_RADIUS, t: 0.0, blood: false, frost: true });
        for k in 0..30 {
            let a = k as f32 / 30.0 * std::f32::consts::TAU;
            self.parts.push(Particle { x: px + a.cos() * 1.6, y: py + a.sin() * 1.6, z: 16.0, vx: -a.sin() * 5.0, vy: a.cos() * 5.0, vz: 2.0, life: 0.35, max: 0.35, kind: PKind::Frost });
        }
        let dmg = wrath_dmg(r) * self.fire_power();
        for i in self.in_circle(px, py, WRATH_RADIUS) {
            self.valk_hit(i, dmg, 0.35, 0.3, Some((px, py, 0.6)));
        }
    }

    fn einherjar(&mut self, r: u8) {
        let alive = self.mobs.iter().filter(|m| m.alive() && m.kind == Kind::Einherjar).count();
        let room = einherjar_count(r).saturating_sub(alive);
        if room == 0 {
            self.say(format!("ALL {} EINHERJAR ALREADY FIGHT FOR YOU", einherjar_count(r)));
            self.p.cast_cd = 0.3;
            return;
        }
        self.p.mana -= einherjar_valor(r);
        self.valk_pose("cast", 0.5);
        self.sfx.push(Sfx::Descend);
        let (px, py) = (self.p.x, self.p.y);
        let power = self.fire_power().sqrt();
        let tier = self.tier.max(1.0);
        for k in 0..room {
            let a = k as f32 / room as f32 * std::f32::consts::TAU + 0.5;
            let (mut x, mut y) = (px + a.cos() * 1.3, py + a.sin() * 1.3);
            if self.d.blocked(x, y, 0.35) {
                (x, y) = (px, py);
            }
            let mut m = crate::mobs::Mob::new(Kind::Einherjar, x, y, tier, &mut self.rng);
            m.charm = einherjar_time(r);
            m.thrall = true;
            m.xp = 0.0;
            m.state = MobState::Chase;
            m.dmg = (m.dmg.0 * power, m.dmg.1 * power);
            self.mobs.push(m);
            for _ in 0..14 {
                self.spray_at(x, y, PKind::Frost, 10.0);
            }
        }
        self.floater(px, py, "EINHERJAR, RISE!".into(), rgb(0xa0e0ff));
    }

    fn valkyrie_ride(&mut self, tx: f32, ty: f32, r: u8) {
        let (px, py) = (self.p.x, self.p.y);
        let (dx, dy) = (tx - px, ty - py);
        let l = (dx * dx + dy * dy).sqrt().max(0.01);
        self.p.mana -= charge_valor(r);
        self.p.skills.cooldown[Skill::ValkyrieRide as usize] = CHARGE_CD;
        self.sfx.push(Sfx::Boom);
        self.say("RIDE!".into());
        self.p.charge = Some(Charge { ux: dx / l, uy: dy / l, left: CHARGE_DIST, dmg: charge_dmg(r) * self.fire_power(), hit: vec![] });
        self.p.dir = crate::iso::dir8(dx, dy);
        self.p.path.clear();
        self.p.goal = None;
    }

    fn fimbulwinter(&mut self, r: u8) {
        self.p.mana -= fimbul_valor(r);
        self.p.skills.cooldown[Skill::Fimbulwinter as usize] = FIMBUL_CD;
        self.valk_pose("cast", 0.6);
        self.p.fimbul_t = FIMBUL_TIME;
        self.p.fimbul_dps = fimbul_dps(r) * self.fire_power();
        self.p.wings_t = FIMBUL_TIME;
        self.sfx.push(Sfx::Descend);
        self.say("FIMBULWINTER".into());
    }

    /// Valor drain, frost on monsters, the raven, javelins, the charge and the blizzard, every tick.
    pub(crate) fn update_valkyrie(&mut self) {
        // Frost thaws, the frozen thaw out, marks fade (for everyone, so a respec leaves no stuck ice).
        for m in self.mobs.iter_mut() {
            m.frost = (m.frost - 0.12 * DT).max(0.0);
            m.frozen = (m.frozen - DT).max(0.0);
            m.marked = (m.marked - DT).max(0.0);
        }
        self.p.wings_t = (self.p.wings_t - DT).max(0.0);
        if !self.is_valkyrie() {
            return;
        }
        self.p.fight_t = (self.p.fight_t - DT).max(0.0);
        if self.p.fight_t <= 0.0 {
            let decay = VALOR_DECAY / (1.0 + self.p.bonus.frac(crate::items::Stat::ManaRegen, 200));
            self.p.mana = (self.p.mana - decay * DT).max(0.0);
        }
        // The raven: homes in on its foe, pecks it and marks it.
        let mut raven_hits = vec![];
        for rv in self.ravens.iter_mut() {
            rv.life -= DT;
            let tgt = rv.target.filter(|&i| i < self.mobs.len() && self.mobs[i].alive()).or_else(|| {
                (0..self.mobs.len()).filter(|&i| self.mobs[i].alive() && self.mobs[i].charm <= 0.0).min_by(|&a, &b| {
                    let da = (self.mobs[a].x - rv.x).powi(2) + (self.mobs[a].y - rv.y).powi(2);
                    let db = (self.mobs[b].x - rv.x).powi(2) + (self.mobs[b].y - rv.y).powi(2);
                    da.partial_cmp(&db).unwrap()
                })
            });
            if let Some(i) = tgt {
                let (dx, dy) = (self.mobs[i].x - rv.x, self.mobs[i].y - rv.y);
                let l = (dx * dx + dy * dy).sqrt().max(0.01);
                rv.vx += (dx / l * 10.0 - rv.vx) * 5.0 * DT;
                rv.vy += (dy / l * 10.0 - rv.vy) * 5.0 * DT;
                if l < self.mobs[i].r + 0.3 {
                    raven_hits.push((i, rv.dmg, rv.mark));
                    rv.life = 0.0;
                }
            }
            rv.x += rv.vx * DT;
            rv.y += rv.vy * DT;
        }
        self.ravens.retain(|rv| rv.life > 0.0);
        for (i, dmg, mark) in raven_hits {
            if self.mobs[i].alive() {
                self.mobs[i].marked = mark;
                let (x, y) = (self.mobs[i].x, self.mobs[i].y);
                self.floater(x, y, "MARKED".into(), rgb(0xa0e0ff));
                self.hit_mob(i, dmg, 0.0, 0.3, None, true);
            }
        }
        // Javelins: out, then home.
        let (px, py) = (self.p.x, self.p.y);
        let mut jav_hits = vec![];
        for j in self.javelins.iter_mut() {
            let step = 16.0 * DT;
            if j.back {
                let (dx, dy) = (px - j.x, py - j.y);
                let l = (dx * dx + dy * dy).sqrt().max(0.01);
                j.ux = dx / l;
                j.uy = dy / l;
                if l < 0.6 {
                    j.gone = -1.0;
                    continue;
                }
            }
            let (nx, ny) = (j.x + j.ux * step, j.y + j.uy * step);
            if !j.back && (!self.d.walkable(nx.floor() as i32, ny.floor() as i32) || j.gone >= JAVELIN_RANGE) {
                j.back = true;
                j.hit.clear();
                continue;
            }
            j.x = nx;
            j.y = ny;
            j.gone += step;
            for (i, m) in self.mobs.iter().enumerate() {
                if m.alive() && m.charm <= 0.0 && !j.hit.contains(&i) && (m.x - j.x).powi(2) + (m.y - j.y).powi(2) < (m.r + 0.35).powi(2) {
                    j.hit.push(i);
                    jav_hits.push((i, j.dmg));
                }
            }
        }
        self.javelins.retain(|j| j.gone >= 0.0);
        for (i, dmg) in jav_hits {
            if self.mobs[i].alive() {
                self.valk_hit(i, dmg, 0.2, 0.2, None);
            }
        }
        // The warhorse charge: carried along the line, trampling and throwing foes aside.
        if let Some(mut c) = self.p.charge.take() {
            let step = (CHARGE_SPEED * DT).min(c.left);
            let (mut x, mut y) = (self.p.x, self.p.y);
            move_circle(&self.d, &mut x, &mut y, c.ux * step, c.uy * step, PLAYER_R);
            let moved = ((x - self.p.x).powi(2) + (y - self.p.y).powi(2)).sqrt();
            (self.p.x, self.p.y) = (x, y);
            c.left -= step;
            self.parts.push(Particle { x, y, z: 4.0, vx: -c.ux * 2.0, vy: -c.uy * 2.0, vz: 10.0, life: 0.4, max: 0.4, kind: PKind::Frost });
            let near: Vec<usize> = self.in_circle(x, y, 1.0).into_iter().filter(|i| !c.hit.contains(i)).collect();
            for i in near {
                c.hit.push(i);
                // Thrown to the side of the charge.
                let side = if (self.mobs[i].x - x) * -c.uy + (self.mobs[i].y - y) * c.ux >= 0.0 { 1.0 } else { -1.0 };
                let from = (self.mobs[i].x - c.uy * side, self.mobs[i].y + c.ux * side);
                self.valk_hit(i, c.dmg, 0.3, 0.6, Some((from.0, from.1, 1.6)));
            }
            if c.left > 0.01 && moved > step * 0.4 {
                self.p.charge = Some(c);
            }
        }
        // Fimbulwinter: a killing blizzard around her.
        if self.p.fimbul_t > 0.0 {
            self.p.fimbul_t -= DT;
            let (px, py) = (self.p.x, self.p.y);
            for _ in 0..3 {
                let a = self.rng.f() * std::f32::consts::TAU;
                let rr = self.rng.f() * FIMBUL_RADIUS;
                self.parts.push(Particle {
                    x: px + a.cos() * rr,
                    y: py + a.sin() * rr,
                    z: 40.0,
                    vx: -a.sin() * 4.0,
                    vy: a.cos() * 4.0,
                    vz: -30.0,
                    life: 0.6,
                    max: 0.6,
                    kind: PKind::Frost,
                });
            }
            let dps = self.p.fimbul_dps;
            for i in self.in_circle(px, py, FIMBUL_RADIUS) {
                let kind = self.mobs[i].kind;
                self.mobs[i].hp -= dps * DT * self.taken(kind);
                if self.mobs[i].state == MobState::Idle {
                    self.mobs[i].state = MobState::Chase;
                }
                self.add_frost(i, 0.6 * DT);
                if self.mobs[i].hp <= 0.0 && self.mobs[i].alive() {
                    self.kill(i);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skills::Class;

    #[test]
    fn the_valkyrie_has_her_own_tree() {
        let sk = Skills::new(Class::Valkyrie);
        assert_eq!(sk.rank(Skill::RuneSpear), 1);
        assert_eq!(sk.rank(Skill::Fireball), 0);
        assert_eq!(sk.actives(), vec![Skill::RuneSpear]);
        assert_eq!(sk.tree().len(), 11);
        for s in sk.tree() {
            assert!(is_valkyrie(*s));
            assert!(!describe(*s, 1, 1.0, &sk).is_empty());
        }
        let mut sk = Skills { points: 3, ..Skills::new(Class::Valkyrie) };
        assert!(sk.learn(Skill::RimeSweep, 1));
        let back = Skills::load_text(&sk.save_text()).unwrap();
        assert_eq!(back.class, Class::Valkyrie);
        assert_eq!(back.rank(Skill::RimeSweep), 1);
        assert_eq!(back.primary, Skill::RuneSpear);
    }
}

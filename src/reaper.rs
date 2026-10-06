//! The Reaper, the sixth playable class (docs/REAPER_CLASS.md): an immortal guardian of a forbidden
//! library with a rune scythe, a spirit lantern and the Ledger of the Forgotten. She runs on Souls:
//! every foe that dies near her releases one, which drifts into her lantern; her bigger skills spend
//! them (the Reaping Scythe is free). Her scythe's runes light one by one as she hunts the same prey;
//! at seven the blade blazes and her next sweep cleaves for triple damage.
use crate::game::{Game, Light, PKind, Particle, Sfx, DT, PLAYER_R};
use crate::gfx::rgb;
use crate::mobs::{Kind, MobState};
use crate::skills::{Nova, Skill, Skills, MAX_RANK};

pub fn is_reaper(s: Skill) -> bool {
    crate::skills::REAPER.contains(&s)
}

// ------------------------------------------------------------------ souls and runes

/// One soul in "mana" units (the globe shows souls).
pub const SOUL: f32 = 10.0;
/// Souls rise from deaths this close to her (Patient Archivist reaches further).
pub const GATHER: f32 = 9.0;
pub const RUNES: u8 = 7;

// ------------------------------------------------------------------ numbers per rank

fn up(r: u8) -> f32 {
    (r.max(1) - 1) as f32
}
pub fn scythe_dmg(r: u8) -> (f32, f32) {
    let k = 1.0 + 0.16 * up(r);
    (10.0 * k, 16.0 * k)
}
pub const SCYTHE_REACH: f32 = 2.4;
pub fn lantern_dmg(r: u8) -> (f32, f32) {
    let k = 1.0 + 0.16 * up(r);
    (10.0 * k, 15.0 * k)
}
/// Patient Archivist: extra soul capacity (in souls), reach, and life per soul gathered.
pub fn archivist_cap(r: u8) -> f32 {
    (r / 2) as f32
}
pub fn archivist_reach(r: u8) -> f32 {
    if r == 0 {
        1.0
    } else {
        1.5 + 0.05 * r as f32
    }
}
pub fn archivist_heal(r: u8) -> f32 {
    if r == 0 {
        0.0
    } else {
        0.02 + 0.003 * up(r)
    }
}
pub fn ledger_bonus(r: u8) -> f32 {
    0.3 + 0.03 * up(r)
}
pub fn ledger_time(r: u8) -> f32 {
    8.0 + 0.4 * r as f32
}
pub fn scholars(r: u8) -> usize {
    2 + r as usize / 4
}
pub fn scholar_time(r: u8) -> f32 {
    18.0 + 2.0 * r as f32
}
pub fn step_dist(r: u8) -> f32 {
    5.0 + 0.2 * r as f32
}
pub fn step_cd(r: u8) -> f32 {
    (3.0 - 0.12 * r as f32).max(1.5)
}
pub fn chain_dmg(r: u8) -> f32 {
    12.0 + 5.0 * up(r)
}
pub fn chain_time(r: u8) -> f32 {
    2.5 + 0.15 * r as f32
}
pub const CHAIN_RADIUS: f32 = 2.4;
pub fn glass_time(r: u8) -> f32 {
    5.0 + 0.3 * r as f32
}
pub const GLASS_RADIUS: f32 = 3.0;
/// Rune Blade: damage per lit rune, and the blazing multiplier.
pub fn rune_bonus(r: u8) -> f32 {
    0.03 + 0.01 * r as f32
}
pub fn blaze_mult(r: u8) -> f32 {
    3.0 + 0.2 * r as f32
}
pub fn harvest_dmg(r: u8) -> f32 {
    28.0 + 9.0 * up(r)
}
pub fn harvest_cut(r: u8) -> f32 {
    (0.2 + 0.01 * r as f32).min(0.35)
}
pub const HARVEST_RADIUS: f32 = 3.5;
pub const HARVEST_CD: f32 = 15.0;
pub fn ledger_open_time(r: u8) -> f32 {
    8.0 + 0.4 * r as f32
}
pub const OPEN_CD: f32 = 40.0;

/// Soul costs (in mana units: 10 = one soul).
pub fn mana_cost(s: Skill, r: u8) -> f32 {
    let _ = r;
    match s {
        Skill::SpiritLantern => SOUL,
        Skill::LedgerMark => SOUL,
        Skill::ScholarSpirits => 3.0 * SOUL,
        Skill::ShadowStep => 0.0,
        Skill::ChainsOfArchive => 3.0 * SOUL,
        Skill::Hourglass => 3.0 * SOUL,
        Skill::SoulHarvest => 5.0 * SOUL,
        Skill::OpenLedger => 6.0 * SOUL,
        _ => 0.0,
    }
}

pub fn cooldown_of(s: Skill) -> f32 {
    match s {
        Skill::ShadowStep => 2.0,
        Skill::SoulHarvest => HARVEST_CD,
        Skill::OpenLedger => OPEN_CD,
        _ => 0.0,
    }
}

pub fn reach_of(s: Skill) -> Option<f32> {
    match s {
        Skill::ReapingScythe => Some(SCYTHE_REACH),
        _ => None,
    }
}

pub fn describe(s: Skill, r: u8, power: f32, sk: &Skills) -> Vec<String> {
    let power = power * sk.fire_mult();
    let at = |r: u8| -> String {
        match s {
            Skill::ReapingScythe => {
                let (a, b) = scythe_dmg(r);
                format!("{}-{} DAMAGE, REACH {:.1}, FREE, LIGHTS THE RUNES", (a * power) as i32, (b * power) as i32, SCYTHE_REACH)
            }
            Skill::SpiritLantern => {
                let (a, b) = lantern_dmg(r);
                format!("{}-{} DAMAGE, HOMING, 1 SOUL", (a * power) as i32, (b * power) as i32)
            }
            Skill::PatientArchivist => format!(
                "GATHERS SOULS {:.0}% FARTHER, +{:.0} MAX SOULS, EACH SOUL HEALS {:.1}% LIFE",
                (archivist_reach(r.max(1)) - 1.0) * 100.0,
                archivist_cap(r.max(1)),
                archivist_heal(r.max(1)) * 100.0
            ),
            Skill::LedgerMark => format!("+{:.0}% DAMAGE FROM YOU FOR {:.0} SEC, 3 SOULS IF IT DIES, 1 SOUL", ledger_bonus(r) * 100.0, ledger_time(r)),
            Skill::ScholarSpirits => format!("{} SPIRITS FOR {:.0} SEC, 3 SOULS", scholars(r), scholar_time(r)),
            Skill::ShadowStep => format!("{:.1} TILES, {:.1} SEC COOLDOWN, FREE", step_dist(r), step_cd(r)),
            Skill::ChainsOfArchive => format!("{} DAMAGE, BOUND FOR {:.1} SEC, 3 SOULS", (chain_dmg(r) * power) as i32, chain_time(r)),
            Skill::Hourglass => format!("FOES CRAWL FOR {:.1} SEC, 3 SOULS", glass_time(r)),
            Skill::RuneBlade => format!("+{:.0}% DAMAGE PER LIT RUNE, BLAZING CLEAVE X{:.1}", rune_bonus(r.max(1)) * 100.0, blaze_mult(r.max(1))),
            Skill::SoulHarvest => format!(
                "{} DAMAGE ALL AROUND, REAPS FOES BELOW {:.0}% LIFE, {:.0} SEC COOLDOWN, 5 SOULS",
                (harvest_dmg(r) * power) as i32,
                harvest_cut(r) * 100.0,
                HARVEST_CD
            ),
            Skill::OpenLedger => format!("{:.1} SEC, +30% DAMAGE, DOUBLE SOULS, THE DEAD RISE, {:.0} SEC COOLDOWN, 6 SOULS", ledger_open_time(r), OPEN_CD),
            _ => String::new(),
        }
    };
    let what = match s {
        Skill::ReapingScythe => "A WIDE SWEEP OF THE RUNE SCYTHE. EVERY HIT ON THE SAME PREY LIGHTS ANOTHER RUNE; AT SEVEN THE BLADE BLAZES.",
        Skill::SpiritLantern => "THE LANTERN'S PALE FLAME FLIES OUT AND HUNTS THE NEAREST FOE.",
        Skill::PatientArchivist => "PASSIVE. YOU HAVE WAITED A THOUSAND YEARS. SOULS FIND YOU FROM FARTHER, AND MEND YOU.",
        Skill::LedgerMark => "WRITE A FOE'S NAME IN THE LEDGER: IT TAKES MORE FROM YOU, AND ITS DEATH IS WORTH THREE SOULS.",
        Skill::ScholarSpirits => "THE GHOSTS OF DEAD SCHOLARS RISE TO FIGHT BESIDE YOU FOR A WHILE.",
        Skill::ShadowStep => "DISSOLVE INTO BLACK SMOKE AND STEP TO A SPOT NEARBY.",
        Skill::ChainsOfArchive => "SPECTRAL CHAINS BURST FROM THE FLOOR AND BIND EVERY FOE IN AN AREA.",
        Skill::Hourglass => "TURN AN HOURGLASS: TIME CRAWLS FOR THE FOES CAUGHT IN ITS SAND.",
        Skill::RuneBlade => "PASSIVE. EVERY LIT RUNE SHARPENS THE BLADE, AND THE BLAZING CLEAVE CUTS DEEPER.",
        Skill::SoulHarvest => "A GREAT REAPING CIRCLE. THE BADLY WOUNDED ARE TAKEN OUTRIGHT.",
        Skill::OpenLedger => "THE LEDGER OF THE FORGOTTEN OPENS. THE DEAD ARE RECORDED TWICE, AND RISE TO SERVE YOU.",
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

/// A soul drifting from a corpse into her lantern.
pub struct SoulFx {
    pub x: f32,
    pub y: f32,
    pub t: f32,
    pub n: f32,
}

/// The lantern's spirit flame, hunting a foe.
pub struct LanternFx {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub life: f32,
    pub dmg: f32,
}

/// Chains holding an area (drawn while they last).
pub struct ChainsFx {
    pub x: f32,
    pub y: f32,
    pub t: f32,
    pub held: Vec<usize>,
}

/// An hourglass slowing an area.
pub struct GlassFx {
    pub x: f32,
    pub y: f32,
    pub t: f32,
    pub max: f32,
}

impl Game {
    pub(crate) fn is_reaper(&self) -> bool {
        self.p.skills.class == crate::skills::Class::Reaper
    }

    pub(crate) fn soul_cap(&self) -> f32 {
        (10.0 + archivist_cap(self.p.skills.rank(Skill::PatientArchivist))) * SOUL
    }

    /// A foe died at (x, y): if she's near, its soul rises toward her lantern.
    pub(crate) fn reaper_kill(&mut self, x: f32, y: f32, boss: bool, marked: bool) {
        let reach = GATHER * archivist_reach(self.p.skills.rank(Skill::PatientArchivist));
        if (x - self.p.x).powi(2) + (y - self.p.y).powi(2) > reach * reach {
            return;
        }
        let mut n = if boss || marked { 3.0 } else { 1.0 };
        if self.p.ledger_t > 0.0 {
            n *= 2.0;
        }
        self.souls.push(SoulFx { x, y, t: 0.0, n });
    }

    /// Her blows: the Ledger and the lit runes add up.
    fn reap_hit(&mut self, i: usize, dmg: f32, stun: f32, knock: Option<(f32, f32, f32)>) {
        let mut dmg = dmg;
        if self.mobs[i].marked > 0.0 {
            dmg *= 1.0 + ledger_bonus(self.p.skills.rank(Skill::LedgerMark).max(1));
        }
        if self.p.ledger_t > 0.0 {
            dmg *= 1.3;
        }
        dmg *= 1.0 + rune_bonus(self.p.skills.rank(Skill::RuneBlade)) * self.p.runes as f32;
        self.hit_mob(i, dmg, 0.0, stun, knock, true);
    }

    /// Reaping Scythe: free, lights the runes, and her out-of-souls attack.
    pub(crate) fn reaping_scythe(&mut self, tx: f32, ty: f32) {
        if self.p.cast_cd > 0.0 {
            return;
        }
        let r = self.p.skills.rank(Skill::ReapingScythe).max(1);
        let blazing = self.p.runes >= RUNES;
        self.cast_pose(0.42);
        self.p.pose = if blazing { "spin" } else { "attack" };
        self.sfx.push(if blazing { Sfx::Boom } else { Sfx::Swing });
        self.stats.casts += 1;
        let (px, py) = (self.p.x, self.p.y);
        let a0 = (ty - py).atan2(tx - px);
        let half = if blazing { 1.6 } else { 1.05 };
        for k in 0..14 {
            let a = a0 - half + k as f32 * (2.0 * half / 13.0);
            self.parts.push(Particle { x: px + a.cos() * 1.8, y: py + a.sin() * 1.8, z: 18.0, vx: -a.sin() * 3.0, vy: a.cos() * 3.0, vz: 0.0, life: 0.2, max: 0.2, kind: PKind::Frost });
        }
        let (lo, hi) = scythe_dmg(r);
        let power = self.fire_power();
        let mult = if blazing { blaze_mult(self.p.skills.rank(Skill::RuneBlade).max(1)) } else { 1.0 };
        let hits = self.in_cone(tx, ty, SCYTHE_REACH + if blazing { 0.6 } else { 0.0 }, half);
        // Her prey: the hit foe nearest where she aimed.
        let prey = hits.iter().copied().min_by(|&a, &b| {
            let da = (self.mobs[a].x - tx).powi(2) + (self.mobs[a].y - ty).powi(2);
            let db = (self.mobs[b].x - tx).powi(2) + (self.mobs[b].y - ty).powi(2);
            da.partial_cmp(&db).unwrap()
        });
        for i in hits {
            let dmg = self.rng.rf(lo, hi) * power * mult;
            self.reap_hit(i, dmg, 0.2, Some((px, py, 0.3)));
        }
        if blazing {
            self.p.runes = 0;
            self.lights.push(Light { x: px, y: py, r: 180.0, s: 0.9, life: 0.3, max: 0.3 });
            self.floater(px, py, "THE BLADE BURNS".into(), rgb(0x9ad8ff));
        } else if let Some(i) = prey {
            if self.p.rune_prey == Some(i) {
                self.p.runes = (self.p.runes + 1).min(RUNES);
            } else {
                self.p.rune_prey = Some(i);
                self.p.runes = 1;
            }
            self.p.rune_t = 5.0;
        }
    }

    pub(crate) fn cast_reaper(&mut self, s: Skill, tx: f32, ty: f32, r: u8) {
        if self.p.cast_cd > 0.0 {
            return;
        }
        match s {
            Skill::ReapingScythe => self.reaping_scythe(tx, ty),
            Skill::SpiritLantern => self.spirit_lantern(tx, ty, r),
            Skill::LedgerMark => self.ledger_mark(tx, ty, r),
            Skill::ScholarSpirits => self.scholar_spirits(r),
            Skill::ShadowStep => self.shadow_step(tx, ty, r),
            Skill::ChainsOfArchive => self.chains(tx, ty, r),
            Skill::Hourglass => self.hourglass(tx, ty, r),
            Skill::SoulHarvest => self.soul_harvest(r),
            Skill::OpenLedger => self.open_ledger(r),
            _ => {}
        }
    }

    fn spirit_lantern(&mut self, tx: f32, ty: f32, r: u8) {
        self.p.mana -= SOUL;
        self.cast_pose(0.3);
        self.p.pose = "cast";
        self.sfx.push(Sfx::Cast);
        let (px, py) = (self.p.x, self.p.y);
        let (dx, dy) = (tx - px, ty - py);
        let l = (dx * dx + dy * dy).sqrt().max(0.01);
        let (lo, hi) = lantern_dmg(r);
        let dmg = self.rng.rf(lo, hi) * self.fire_power();
        self.lanterns.push(LanternFx { x: px + dx / l * 0.5, y: py + dy / l * 0.5, vx: dx / l * 8.0, vy: dy / l * 8.0, life: 2.5, dmg });
    }

    fn ledger_mark(&mut self, tx: f32, ty: f32, r: u8) {
        let target = self.foes_where(|_| true).into_iter().min_by(|&a, &b| {
            let da = (self.mobs[a].x - tx).powi(2) + (self.mobs[a].y - ty).powi(2);
            let db = (self.mobs[b].x - tx).powi(2) + (self.mobs[b].y - ty).powi(2);
            da.partial_cmp(&db).unwrap()
        });
        let Some(i) = target.filter(|&i| (self.mobs[i].x - tx).powi(2) + (self.mobs[i].y - ty).powi(2) < 9.0) else {
            self.say("NO NAME TO WRITE THERE".into());
            self.p.cast_cd = 0.3;
            return;
        };
        self.p.mana -= SOUL;
        self.cast_pose(0.3);
        self.p.pose = "cast";
        self.sfx.push(Sfx::Pickup);
        self.mobs[i].marked = ledger_time(r);
        let (x, y) = (self.mobs[i].x, self.mobs[i].y);
        self.floater(x, y, "RECORDED".into(), rgb(0xe0c080));
    }

    fn scholar_spirits(&mut self, r: u8) {
        let alive = self.mobs.iter().filter(|m| m.alive() && m.kind == Kind::Scholar).count();
        let room = scholars(r).saturating_sub(alive);
        if room == 0 {
            self.say("THE SCHOLARS ARE ALL HERE".into());
            self.p.cast_cd = 0.3;
            return;
        }
        self.p.mana -= 3.0 * SOUL;
        self.cast_pose(0.5);
        self.p.pose = "cast";
        self.sfx.push(Sfx::Descend);
        let (px, py) = (self.p.x, self.p.y);
        for k in 0..room {
            let a = k as f32 / room as f32 * std::f32::consts::TAU + 0.3;
            self.raise_scholar(px + a.cos() * 1.3, py + a.sin() * 1.3, scholar_time(r));
        }
    }

    /// A scholar spirit at (x, y), on her side for `t` seconds.
    pub(crate) fn raise_scholar(&mut self, x: f32, y: f32, t: f32) {
        let (mut x, mut y) = (x, y);
        if self.d.blocked(x, y, 0.3) {
            (x, y) = (self.p.x, self.p.y);
        }
        let mut m = crate::mobs::Mob::new(Kind::Scholar, x, y, self.tier.max(1.0), &mut self.rng);
        m.charm = t;
        m.thrall = true;
        m.xp = 0.0;
        m.state = MobState::Chase;
        let k = self.fire_power().sqrt();
        m.dmg = (m.dmg.0 * k, m.dmg.1 * k);
        self.mobs.push(m);
        for _ in 0..12 {
            self.spray_at(x, y, PKind::Frost, 10.0);
        }
    }

    fn shadow_step(&mut self, tx: f32, ty: f32, r: u8) {
        let (px, py) = (self.p.x, self.p.y);
        let (dx, dy) = (tx - px, ty - py);
        let l = (dx * dx + dy * dy).sqrt().max(0.01);
        let (ux, uy) = (dx / l, dy / l);
        let mut best = None;
        let mut d = 0.5;
        while d <= l.min(step_dist(r)) + 0.01 {
            let (x, y) = (px + ux * d, py + uy * d);
            if !self.d.walkable(x.floor() as i32, y.floor() as i32) {
                break;
            }
            if !self.d.blocked(x, y, PLAYER_R) {
                best = Some((x, y));
            }
            d += 0.25;
        }
        let Some((x, y)) = best else {
            self.p.cast_cd = 0.3;
            return;
        };
        self.p.skills.cooldown[Skill::ShadowStep as usize] = step_cd(r);
        for _ in 0..20 {
            self.parts.push(Particle { x: px, y: py, z: 10.0 + self.rng.f() * 30.0, vx: self.rng.rf(-1.0, 1.0), vy: self.rng.rf(-1.0, 1.0), vz: 10.0, life: 0.7, max: 0.7, kind: PKind::Smoke });
        }
        (self.p.x, self.p.y) = (x, y);
        self.p.mist = 0.3;
        self.p.path.clear();
        self.p.goal = None;
        self.sfx.push(Sfx::Swing);
        for _ in 0..12 {
            self.parts.push(Particle { x, y, z: 10.0 + self.rng.f() * 30.0, vx: self.rng.rf(-1.0, 1.0), vy: self.rng.rf(-1.0, 1.0), vz: 10.0, life: 0.5, max: 0.5, kind: PKind::Smoke });
        }
    }

    fn chains(&mut self, tx: f32, ty: f32, r: u8) {
        self.p.mana -= 3.0 * SOUL;
        self.cast_pose(0.4);
        self.p.pose = "cast";
        self.sfx.push(Sfx::Boom);
        let dmg = chain_dmg(r) * self.fire_power();
        let held = self.in_circle(tx, ty, CHAIN_RADIUS);
        for &i in &held {
            let t = chain_time(r) * if self.mobs[i].boss { 0.25 } else { 1.0 };
            self.reap_hit(i, dmg, 0.0, None);
            if self.mobs[i].alive() {
                self.mobs[i].stun = self.mobs[i].stun.max(t);
            }
        }
        self.chains_fx.push(ChainsFx { x: tx, y: ty, t: chain_time(r), held });
    }

    fn hourglass(&mut self, tx: f32, ty: f32, r: u8) {
        self.p.mana -= 3.0 * SOUL;
        self.cast_pose(0.4);
        self.p.pose = "cast";
        self.sfx.push(Sfx::Descend);
        self.glasses.push(GlassFx { x: tx, y: ty, t: glass_time(r), max: glass_time(r) });
    }

    fn soul_harvest(&mut self, r: u8) {
        self.p.mana -= 5.0 * SOUL;
        self.p.skills.cooldown[Skill::SoulHarvest as usize] = HARVEST_CD;
        self.cast_pose(0.5);
        self.p.pose = "spin";
        self.sfx.push(Sfx::Boom);
        self.shake = self.shake.max(0.5);
        let (px, py) = (self.p.x, self.p.y);
        self.novas.push(Nova { x: px, y: py, r: HARVEST_RADIUS, t: 0.0, blood: false, frost: true });
        let dmg = harvest_dmg(r) * self.fire_power();
        let cut = harvest_cut(r);
        for i in self.in_circle(px, py, HARVEST_RADIUS) {
            let m = &self.mobs[i];
            if !m.boss && m.hp < m.max_hp * cut {
                // Reaped outright.
                let (x, y) = (m.x, m.y);
                self.mobs[i].hp = 0.0;
                self.floater(x, y, "REAPED".into(), rgb(0x9ad8ff));
                self.kill(i);
            } else {
                self.reap_hit(i, dmg, 0.3, Some((px, py, 0.6)));
            }
        }
    }

    fn open_ledger(&mut self, r: u8) {
        self.p.mana -= 6.0 * SOUL;
        self.p.skills.cooldown[Skill::OpenLedger as usize] = OPEN_CD;
        self.cast_pose(0.6);
        self.p.pose = "cast";
        self.p.ledger_t = ledger_open_time(r);
        self.sfx.push(Sfx::Descend);
        self.say("THE LEDGER OF THE FORGOTTEN OPENS".into());
    }

    /// Souls, the lantern flame, chains, hourglasses, the runes and the Ledger, every tick.
    pub(crate) fn update_reaper(&mut self) {
        // Hourglasses slow every foe in their sand (for everyone, so nothing gets stuck slow).
        for m in self.mobs.iter_mut() {
            m.slow_t = (m.slow_t - DT).max(0.0);
        }
        for g in self.glasses.iter_mut() {
            g.t -= DT;
        }
        self.glasses.retain(|g| g.t > 0.0);
        let glasses: Vec<(f32, f32)> = self.glasses.iter().map(|g| (g.x, g.y)).collect();
        for m in self.mobs.iter_mut() {
            if m.alive() && m.charm <= 0.0 && glasses.iter().any(|&(x, y)| (m.x - x).powi(2) + (m.y - y).powi(2) < GLASS_RADIUS * GLASS_RADIUS) {
                m.slow_t = 0.2;
            }
        }
        for c in self.chains_fx.iter_mut() {
            c.t -= DT;
        }
        self.chains_fx.retain(|c| c.t > 0.0);
        if !self.is_reaper() {
            return;
        }
        self.p.ledger_t = (self.p.ledger_t - DT).max(0.0);
        self.p.rune_t = (self.p.rune_t - DT).max(0.0);
        if self.p.rune_t <= 0.0 && self.p.runes > 0 && self.p.runes < RUNES {
            self.p.runes = 0;
            self.p.rune_prey = None;
        }
        // Souls drift in, faster as they near her.
        let (px, py) = (self.p.x, self.p.y);
        let cap = self.soul_cap();
        let heal = archivist_heal(self.p.skills.rank(Skill::PatientArchivist));
        let mut got = 0.0;
        for s in self.souls.iter_mut() {
            s.t += DT;
            let (dx, dy) = (px - s.x, py - s.y);
            let l = (dx * dx + dy * dy).sqrt();
            if l < 0.5 {
                got += s.n;
                s.t = -1.0;
                continue;
            }
            let sp = (3.0 + s.t * 8.0).min(14.0) * DT;
            s.x += dx / l * sp.min(l);
            s.y += dy / l * sp.min(l);
        }
        self.souls.retain(|s| s.t >= 0.0);
        if got > 0.0 {
            self.p.mana = (self.p.mana + got * SOUL).min(cap);
            self.p.hp = (self.p.hp + self.p.max_hp * heal * got).min(self.p.max_hp);
            self.sfx.push(Sfx::Pickup);
        }
        // The lantern's flame hunts the nearest foe.
        let mut hits = vec![];
        for f in self.lanterns.iter_mut() {
            f.life -= DT;
            let near = self
                .mobs
                .iter()
                .enumerate()
                .filter(|(_, m)| m.alive() && m.charm <= 0.0)
                .map(|(i, m)| (i, (m.x - f.x).powi(2) + (m.y - f.y).powi(2), m.r))
                .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
            if let Some((i, d2, mr)) = near {
                let l = d2.sqrt().max(0.01);
                if l < 8.0 {
                    let (dx, dy) = ((self.mobs[i].x - f.x) / l, (self.mobs[i].y - f.y) / l);
                    f.vx += (dx * 9.0 - f.vx) * 4.0 * DT;
                    f.vy += (dy * 9.0 - f.vy) * 4.0 * DT;
                }
                if l < mr + 0.3 {
                    hits.push((i, f.dmg));
                    f.life = 0.0;
                }
            }
            let (nx, ny) = (f.x + f.vx * DT, f.y + f.vy * DT);
            if !self.d.walkable(nx.floor() as i32, ny.floor() as i32) {
                f.life = 0.0;
            }
            f.x = nx;
            f.y = ny;
        }
        self.lanterns.retain(|f| f.life > 0.0);
        for (i, dmg) in hits {
            if self.mobs[i].alive() {
                self.reap_hit(i, dmg, 0.15, None);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skills::Class;

    #[test]
    fn the_reaper_has_her_own_tree() {
        let sk = Skills::new(Class::Reaper);
        assert_eq!(sk.rank(Skill::ReapingScythe), 1);
        assert_eq!(sk.actives(), vec![Skill::ReapingScythe]);
        assert_eq!(sk.tree().len(), 11);
        for s in sk.tree() {
            assert!(is_reaper(*s));
            assert!(!describe(*s, 1, 1.0, &sk).is_empty());
        }
        let mut sk = Skills { points: 3, ..Skills::new(Class::Reaper) };
        assert!(sk.learn(Skill::SpiritLantern, 1));
        let back = Skills::load_text(&sk.save_text()).unwrap();
        assert_eq!(back.class, Class::Reaper);
        assert_eq!(back.rank(Skill::SpiritLantern), 1);
    }
}

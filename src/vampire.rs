//! The Vampire, the second playable class (docs/VAMPIRE_CLASS.md): a blood caster with claws.
//! Her damage is blood, not fire. Almost everything she does steals life, but the bloodless
//! (skeletons, wraiths, the Wardens) can't be drained. Mist Step is hers alone.
use crate::game::{Game, Light, PKind, Particle, Sfx, DT};
use crate::gfx::rgb;
use crate::mobs::{Kind, MobState};
use crate::skills::{Nova, Skill, Skills, MAX_RANK};

/// Blood hunger: how much blood (food bar) each point of blood damage on a living foe gives,
/// and what one bite gives on top.
pub const BLOOD_PER_DAMAGE: f32 = 0.2;
pub const BITE_BLOOD: f32 = 12.0;

pub fn is_vampire(s: Skill) -> bool {
    crate::skills::VAMPIRE.contains(&s)
}

// ------------------------------------------------------------------ numbers per rank

fn up(r: u8) -> f32 {
    (r.max(1) - 1) as f32
}
pub fn lance_dmg(r: u8) -> (f32, f32) {
    let k = 1.0 + 0.15 * up(r);
    (8.0 * k, 13.0 * k)
}
pub fn lance_mana(r: u8) -> f32 {
    4.0 + 0.3 * up(r)
}
pub fn rake_dmg(r: u8) -> (f32, f32) {
    let k = 1.0 + 0.18 * up(r);
    (10.0 * k, 16.0 * k)
}
pub fn rake_mana(r: u8) -> f32 {
    2.0 + 0.1 * up(r)
}
/// Thirst: share of all damage dealt that comes back as life.
pub fn thirst_steal(r: u8) -> f32 {
    if r == 0 {
        0.04
    } else {
        0.07 + 0.012 * up(r)
    }
}
pub fn bats(r: u8) -> usize {
    3 + r as usize / 3
}
pub fn bat_dmg(r: u8) -> f32 {
    6.0 + 2.5 * up(r)
}
pub fn bat_mana(r: u8) -> f32 {
    10.0 + 0.6 * up(r)
}
pub fn mesmerize_time(r: u8) -> f32 {
    4.0 + 0.5 * r as f32
}
pub fn mesmerize_mana(r: u8) -> f32 {
    12.0 + 0.5 * up(r)
}
pub fn mist_dist(r: u8) -> f32 {
    4.0 + 0.15 * r as f32
}
pub fn mist_cd(r: u8) -> f32 {
    (3.0 - 0.12 * r as f32).max(1.5)
}
pub fn nova_dmg(r: u8) -> f32 {
    18.0 + 7.0 * up(r)
}
pub fn nova_radius(r: u8) -> f32 {
    2.6 + 0.08 * r as f32
}
pub fn nova_mana(r: u8) -> f32 {
    14.0 + 0.7 * up(r)
}
pub fn thrall_time(r: u8) -> f32 {
    25.0 + 3.0 * r as f32
}
pub fn thrall_max(r: u8) -> usize {
    1 + r as usize / 4
}
pub fn thrall_mana(r: u8) -> f32 {
    20.0 + 1.0 * up(r)
}
pub fn moon_dps(r: u8) -> f32 {
    10.0 + 4.0 * up(r)
}
pub const MOON_RADIUS: f32 = 3.0;
pub const MOON_TIME: f32 = 8.0;
pub const MOON_CD: f32 = 20.0;
pub fn moon_mana(r: u8) -> f32 {
    35.0 + 1.0 * up(r)
}
pub fn embrace_time(r: u8) -> f32 {
    7.0 + 0.3 * r as f32
}
pub fn screech_dmg(r: u8) -> f32 {
    15.0 + 5.0 * up(r)
}
pub const SCREECH_RADIUS: f32 = 2.6;
pub const EMBRACE_CD: f32 = 30.0;
pub fn embrace_mana(r: u8) -> f32 {
    40.0 + 1.0 * up(r)
}

pub fn mana_cost(s: Skill, r: u8) -> f32 {
    match s {
        Skill::BloodLance => lance_mana(r),
        Skill::Rake => rake_mana(r),
        Skill::BatSwarm => bat_mana(r),
        Skill::Mesmerize => mesmerize_mana(r),
        Skill::MistStep => 6.0,
        Skill::CrimsonNova => nova_mana(r),
        Skill::Thrall => thrall_mana(r),
        Skill::BloodMoon => moon_mana(r),
        Skill::Embrace => embrace_mana(r),
        _ => 0.0,
    }
}

pub fn cooldown_of(s: Skill) -> f32 {
    match s {
        Skill::MistStep => 2.0,
        Skill::BloodMoon => MOON_CD,
        Skill::Embrace => EMBRACE_CD,
        _ => 0.0,
    }
}

/// Undead and spirits have no blood: no life to steal, and the drain bites less.
pub fn bloodless(k: Kind) -> bool {
    matches!(
        k,
        Kind::Skeleton
            | Kind::Archer
            | Kind::BoneWarden
            | Kind::HexWarden
            | Kind::AshKing
            | Kind::IceWraith
            | Kind::RimeWitch
            | Kind::Banshee
            | Kind::Wisp
            | Kind::Ossric
            | Kind::Grimhilde
            | Kind::Malgrave
            | Kind::Scarab
            | Kind::Inquisitor
            | Kind::Gearwraith
            | Kind::SpringJack
            | Kind::BoilerBrute
            | Kind::Ordinal
            | Kind::Prism
            | Kind::Marshal
            | Kind::Forgemother
            | Kind::Cantor
            | Kind::Archivist
    )
}

/// Blood damage multiplier against a monster.
pub fn blood_taken(k: Kind) -> f32 {
    if bloodless(k) {
        0.85
    } else {
        1.0
    }
}

pub fn describe(s: Skill, r: u8, power: f32, sk: &Skills) -> Vec<String> {
    let power = power * if s == Skill::NightMastery { 1.0 } else { sk.fire_mult() };
    let at = |r: u8| -> String {
        match s {
            Skill::BloodLance => {
                let (a, b) = lance_dmg(r);
                format!("{}-{} DAMAGE, {:.0} MANA", (a * power) as i32, (b * power) as i32, lance_mana(r))
            }
            Skill::Rake => {
                let (a, b) = rake_dmg(r);
                format!("{}-{} DAMAGE, STEALS 3X LIFE, {:.0} MANA", (a * power) as i32, (b * power) as i32, rake_mana(r))
            }
            Skill::Thirst => format!("{:.0}% OF DAMAGE DEALT RETURNS AS LIFE", thirst_steal(r) * 100.0),
            Skill::BatSwarm => format!("{} BATS, {} DAMAGE EACH, {:.0} MANA", bats(r), (bat_dmg(r) * power) as i32, bat_mana(r)),
            Skill::Mesmerize => format!("FIGHTS FOR YOU FOR {:.1} SEC, {:.0} MANA", mesmerize_time(r), mesmerize_mana(r)),
            Skill::MistStep => format!("{:.1} TILES, {:.1} SEC COOLDOWN, 6 MANA", mist_dist(r), mist_cd(r)),
            Skill::CrimsonNova => format!("{} DAMAGE, RADIUS {:.1}, {:.0} MANA", (nova_dmg(r) * power) as i32, nova_radius(r), nova_mana(r)),
            Skill::Thrall => format!("UP TO {} THRALLS FOR {:.0} SEC, {:.0} MANA", thrall_max(r), thrall_time(r), thrall_mana(r)),
            Skill::NightMastery => format!("+{}% BLOOD DAMAGE, +{}% LIFE STEAL", 8 * r.max(1) as i32, r.max(1)),
            Skill::BloodMoon => format!(
                "{} DAMAGE/SEC FOR {:.0} SEC, {:.0} MANA, {:.0} SEC COOLDOWN",
                (moon_dps(r) * power) as i32,
                MOON_TIME,
                moon_mana(r),
                MOON_CD
            ),
            Skill::Embrace => format!(
                "{:.1} SEC, SCREECH {} DAMAGE/SEC, {:.0} MANA, {:.0} SEC COOLDOWN",
                embrace_time(r),
                (screech_dmg(r) * power) as i32,
                embrace_mana(r),
                EMBRACE_CD
            ),
            _ => String::new(),
        }
    };
    let what = match s {
        Skill::BloodLance => "A CRIMSON BOLT THAT DRINKS FROM WHAT IT HITS.",
        Skill::Rake => "A FAST CLAWED SLASH AT EVERYTHING IN FRONT OF YOU. STEALS THREE TIMES AS MUCH LIFE.",
        Skill::Thirst => "PASSIVE. PART OF ALL DAMAGE YOU DEAL RETURNS AS LIFE. BLOODLESS FOES (SKELETONS, WRAITHS) GIVE NOTHING.",
        Skill::BatSwarm => "A SWARM OF BATS BURSTS FROM YOUR CAPE AND HUNTS THE NEAREST FOES.",
        Skill::Mesmerize => "YOUR GAZE TURNS A FOE: IT FIGHTS FOR YOU UNTIL THE SPELL BREAKS. BOSSES RESIST.",
        Skill::MistStep => "BECOME MIST AND SLIP THROUGH ENEMIES TO A SPOT AHEAD. NOTHING CAN TOUCH YOU WHILE YOU ARE MIST.",
        Skill::CrimsonNova => "A RING OF BLOOD BURSTS FROM YOU, DRAINING EVERYTHING NEARBY.",
        Skill::Thrall => "RAISES A FALLEN FOE NEARBY TO FIGHT AT YOUR SIDE UNTIL IT CRUMBLES.",
        Skill::NightMastery => "PASSIVE. ALL YOUR BLOOD MAGIC HITS HARDER AND STEALS MORE LIFE.",
        Skill::BloodMoon => "A BLOOD MOON HANGS OVER THE TARGET: FOES BENEATH IT BLEED, AND YOU DRINK FROM EVERY ONE.",
        Skill::Embrace => "BECOME A GREAT BAT: FASTER, SCREECHING AT EVERYTHING AROUND YOU, AND YOUR SKILLS COST NO MANA.",
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

/// A bat from Bat Swarm: flies at the nearest foe.
pub struct BatFx {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub life: f32,
    pub dmg: f32,
}

/// Blood Moon's field.
pub struct BloodField {
    pub x: f32,
    pub y: f32,
    pub t: f32,
    pub dps: f32,
}

impl Game {
    /// Life steal from blood damage dealt to a monster of this kind. It also feeds her blood
    /// hunger (the vampire's food bar): only creatures with blood feed her.
    pub(crate) fn drain(&mut self, kind: Kind, dealt: f32, mult: f32) {
        if self.p.skills.class != crate::skills::Class::Vampire || bloodless(kind) || dealt <= 0.0 {
            return;
        }
        let sk = &self.p.skills;
        let steal = thirst_steal(sk.rank(Skill::Thirst)) + 0.01 * sk.rank(Skill::NightMastery) as f32;
        self.p.hp = (self.p.hp + dealt * steal * mult).min(self.p.max_hp);
        self.feed(dealt * BLOOD_PER_DAMAGE * mult);
    }

    /// Fills the vampire's blood hunger.
    pub(crate) fn feed(&mut self, blood: f32) {
        self.p.food = (self.p.food + blood).min(crate::game::MAX_FOOD);
        if blood > 0.0 {
            self.p.hunger_msg = self.p.hunger_msg.min(4.0);
        }
    }

    /// The vampire's free bite (her out-of-mana attack): drinks deeply from one foe in front of her.
    pub(crate) fn bite(&mut self, tx: f32, ty: f32) {
        self.cast_pose(0.3);
        self.sfx.push(Sfx::Swing);
        let (px, py) = (self.p.x, self.p.y);
        let a0 = (ty - py).atan2(tx - px);
        let target = (0..self.mobs.len())
            .filter(|&i| {
                let m = &self.mobs[i];
                let (dx, dy) = (m.x - px, m.y - py);
                let da = (dy.atan2(dx) - a0 + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
                m.alive() && m.charm <= 0.0 && dx * dx + dy * dy < (1.3 + m.r).powi(2) && da.abs() < 1.1
            })
            .min_by(|&a, &b| {
                let da = (self.mobs[a].x - px).powi(2) + (self.mobs[a].y - py).powi(2);
                let db = (self.mobs[b].x - px).powi(2) + (self.mobs[b].y - py).powi(2);
                da.partial_cmp(&db).unwrap()
            });
        let Some(i) = target else { return };
        let kind = self.mobs[i].kind;
        let (mx, my) = (self.mobs[i].x, self.mobs[i].y);
        let dmg = self.rng.rf(6.0, 10.0) * self.p.power.sqrt();
        self.hit_mob(i, dmg, 0.0, 0.3, None, true);
        if bloodless(kind) {
            self.floater(mx, my, "NO BLOOD".into(), rgb(0xa0a0a0));
        } else {
            // A bite drinks far more than magic.
            self.drain(kind, dmg, 3.0);
            self.feed(BITE_BLOOD);
            for _ in 0..12 {
                self.spray_at(mx, my, PKind::Blood, 18.0);
            }
        }
    }

    pub(crate) fn cast_vampire(&mut self, s: Skill, tx: f32, ty: f32, r: u8) {
        if self.p.cast_cd > 0.0 {
            return;
        }
        match s {
            Skill::BloodLance => self.cast_fireball(tx, ty, false),
            Skill::Rake => self.rake(tx, ty, r),
            Skill::BatSwarm => self.bat_swarm(r),
            Skill::Mesmerize => self.mesmerize(tx, ty, r),
            Skill::MistStep => self.mist_step(tx, ty, r),
            Skill::CrimsonNova => self.crimson_nova(r),
            Skill::Thrall => self.raise_thrall(r),
            Skill::BloodMoon => self.blood_moon(tx, ty, r),
            Skill::Embrace => self.embrace(r),
            _ => {}
        }
    }

    fn rake(&mut self, tx: f32, ty: f32, r: u8) {
        self.p.mana -= rake_mana(r);
        self.cast_pose(0.3);
        self.sfx.push(Sfx::Swing);
        let (px, py) = (self.p.x, self.p.y);
        let a0 = (ty - py).atan2(tx - px);
        let (lo, hi) = rake_dmg(r);
        let power = self.fire_power();
        let hits: Vec<usize> = (0..self.mobs.len())
            .filter(|&i| {
                let m = &self.mobs[i];
                let (dx, dy) = (m.x - px, m.y - py);
                let d = (dx * dx + dy * dy).sqrt();
                let da = (dy.atan2(dx) - a0 + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
                m.alive() && m.charm <= 0.0 && d < 1.5 + m.r && da.abs() < 1.0
            })
            .collect();
        for k in 0..10 {
            let a = a0 - 0.9 + k as f32 * 0.2;
            self.parts.push(Particle { x: px + a.cos() * 1.0, y: py + a.sin() * 1.0, z: 18.0, vx: a.cos(), vy: a.sin(), vz: 0.0, life: 0.2, max: 0.2, kind: PKind::Blood });
        }
        for i in hits {
            let dmg = self.rng.rf(lo, hi) * power;
            let kind = self.mobs[i].kind;
            self.hit_mob(i, dmg, 0.0, 0.25, Some((px, py, 0.4)), true);
            // Rake steals three times as much (hit_mob already drained once).
            self.drain(kind, dmg, 2.0);
        }
    }

    fn bat_swarm(&mut self, r: u8) {
        self.p.mana -= bat_mana(r);
        self.cast_pose(0.35);
        self.sfx.push(Sfx::Cast);
        let n = bats(r);
        let dmg = bat_dmg(r) * self.fire_power();
        for k in 0..n {
            let a = k as f32 / n as f32 * std::f32::consts::TAU + self.rng.f();
            self.bats.push(BatFx { x: self.p.x, y: self.p.y, vx: a.cos() * 4.0, vy: a.sin() * 4.0, life: 3.5, dmg });
        }
    }

    fn mesmerize(&mut self, tx: f32, ty: f32, r: u8) {
        let target = (0..self.mobs.len())
            .filter(|&i| self.mobs[i].alive() && self.mobs[i].charm <= 0.0)
            .min_by(|&a, &b| {
                let da = (self.mobs[a].x - tx).powi(2) + (self.mobs[a].y - ty).powi(2);
                let db = (self.mobs[b].x - tx).powi(2) + (self.mobs[b].y - ty).powi(2);
                da.partial_cmp(&db).unwrap()
            })
            .filter(|&i| (self.mobs[i].x - tx).powi(2) + (self.mobs[i].y - ty).powi(2) < 9.0);
        let Some(i) = target else {
            self.say("NO ONE TO MESMERIZE THERE".into());
            self.p.cast_cd = 0.3;
            return;
        };
        self.p.mana -= mesmerize_mana(r);
        self.cast_pose(0.35);
        if self.mobs[i].boss {
            self.say(format!("{} RESISTS YOUR GAZE", self.mobs[i].label()));
            return;
        }
        let (x, y) = (self.mobs[i].x, self.mobs[i].y);
        self.mobs[i].charm = mesmerize_time(r);
        self.mobs[i].state = MobState::Chase;
        self.sfx.push(Sfx::Pickup);
        self.floater(x, y, "MESMERIZED".into(), rgb(0xd070ff));
        for _ in 0..20 {
            self.spray_at(x, y, PKind::Magic, 24.0);
        }
    }

    fn mist_step(&mut self, tx: f32, ty: f32, r: u8) {
        self.p.mana -= 6.0;
        self.p.skills.cooldown[Skill::MistStep as usize] = mist_cd(r);
        let (px, py) = (self.p.x, self.p.y);
        let (dx, dy) = (tx - px, ty - py);
        let l = (dx * dx + dy * dy).sqrt().max(0.01);
        let (ux, uy) = (dx / l, dy / l);
        // Slide through enemies (not walls) as far as the mist reaches.
        let mut best = (px, py);
        let mut d = 0.25;
        while d <= mist_dist(r) {
            let (x, y) = (px + ux * d, py + uy * d);
            if !self.d.walkable(x.floor() as i32, y.floor() as i32) {
                break;
            }
            if !self.d.blocked(x, y, crate::game::PLAYER_R) {
                best = (x, y);
            }
            d += 0.25;
        }
        for k in 0..24 {
            let t = k as f32 / 24.0;
            let (x, y) = (px + (best.0 - px) * t, py + (best.1 - py) * t);
            self.parts.push(Particle { x, y, z: 12.0 + self.rng.f() * 20.0, vx: 0.0, vy: 0.0, vz: 8.0, life: 0.6, max: 0.6, kind: PKind::Smoke });
        }
        (self.p.x, self.p.y) = best;
        self.p.mist = 0.35;
        self.p.path.clear();
        self.p.goal = None;
        self.sfx.push(Sfx::Swing);
    }

    fn crimson_nova(&mut self, r: u8) {
        let (px, py) = (self.p.x, self.p.y);
        self.p.mana -= nova_mana(r);
        self.cast_pose(0.45);
        let radius = nova_radius(r);
        self.novas.push(Nova { x: px, y: py, r: radius, t: 0.0, blood: true });
        self.lights.push(Light { x: px, y: py, r: 220.0, s: 1.0, life: 0.4, max: 0.4 });
        self.shake = self.shake.max(0.4);
        self.sfx.push(Sfx::Boom);
        for k in 0..50 {
            let a = k as f32 / 50.0 * std::f32::consts::TAU;
            let sp = radius / crate::skills::NOVA_TIME;
            self.parts.push(Particle { x: px, y: py, z: 10.0, vx: a.cos() * sp, vy: a.sin() * sp, vz: 10.0, life: 0.4, max: 0.4, kind: PKind::Blood });
        }
        let dmg = nova_dmg(r) * self.fire_power();
        let hits: Vec<usize> = (0..self.mobs.len())
            .filter(|&i| {
                let m = &self.mobs[i];
                m.alive() && (m.x - px).powi(2) + (m.y - py).powi(2) < (radius + m.r).powi(2) && self.d.los(px, py, m.x, m.y)
            })
            .collect();
        for i in hits {
            self.hit_mob(i, dmg, 0.0, 0.3, Some((px, py, 0.8)), true);
        }
    }

    fn raise_thrall(&mut self, r: u8) {
        let alive = self.mobs.iter().filter(|m| m.alive() && m.thrall).count();
        if alive >= thrall_max(r) {
            self.say(format!("YOU CAN HOLD ONLY {} THRALL(S)", thrall_max(r)));
            self.p.cast_cd = 0.3;
            return;
        }
        let (px, py) = (self.p.x, self.p.y);
        let corpse = (0..self.mobs.len())
            .filter(|&i| matches!(self.mobs[i].state, MobState::Dead(_)) && !self.mobs[i].boss && !self.mobs[i].thrall)
            .map(|i| (i, (self.mobs[i].x - px).powi(2) + (self.mobs[i].y - py).powi(2)))
            .filter(|&(_, d2)| d2 < 49.0)
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .map(|v| v.0);
        let Some(i) = corpse else {
            self.say("NO CORPSE NEARBY TO RAISE".into());
            self.p.cast_cd = 0.3;
            return;
        };
        self.p.mana -= thrall_mana(r);
        self.cast_pose(0.5);
        self.sfx.push(Sfx::Descend);
        let power = self.fire_power();
        let m = &mut self.mobs[i];
        m.state = MobState::Chase;
        m.hp = m.max_hp;
        m.charm = thrall_time(r);
        m.thrall = true;
        m.xp = 0.0;
        m.burn = 0.0;
        m.dmg = (m.dmg.0 * power.sqrt(), m.dmg.1 * power.sqrt());
        let (x, y) = (m.x, m.y);
        self.floater(x, y, "RISE".into(), rgb(0xd070ff));
        for _ in 0..30 {
            self.spray_at(x, y, PKind::Magic, 10.0);
        }
    }

    fn blood_moon(&mut self, tx: f32, ty: f32, r: u8) {
        self.p.mana -= moon_mana(r);
        self.cast_pose(0.5);
        self.p.skills.cooldown[Skill::BloodMoon as usize] = MOON_CD;
        self.fields.push(BloodField { x: tx, y: ty, t: 0.0, dps: moon_dps(r) * self.fire_power() });
        self.sfx.push(Sfx::Descend);
        self.say("BLOOD MOON".into());
    }

    fn embrace(&mut self, r: u8) {
        self.p.mana -= embrace_mana(r);
        self.cast_pose(0.5);
        self.p.skills.cooldown[Skill::Embrace as usize] = EMBRACE_CD;
        self.p.embrace_t = embrace_time(r);
        self.p.embrace_rank = r;
        self.sfx.push(Sfx::Descend);
        self.say("THE COUNTESS'S EMBRACE".into());
        for _ in 0..40 {
            self.spray_at(self.p.x, self.p.y, PKind::Magic, 24.0);
        }
    }

    /// Bats, Blood Moon fields and the bat form, every tick.
    pub(crate) fn update_vampire(&mut self) {
        self.p.mist = (self.p.mist - DT).max(0.0);
        // Bats home in on the nearest hostile monster.
        let mut hits = vec![];
        let mobs: Vec<(usize, f32, f32, f32)> =
            self.mobs.iter().enumerate().filter(|(_, m)| m.alive() && m.charm <= 0.0).map(|(i, m)| (i, m.x, m.y, m.r)).collect();
        for b in self.bats.iter_mut() {
            b.life -= DT;
            if let Some(&(i, mx, my, mr)) = mobs.iter().min_by(|a, c| {
                let da = (a.1 - b.x).powi(2) + (a.2 - b.y).powi(2);
                let dc = (c.1 - b.x).powi(2) + (c.2 - b.y).powi(2);
                da.partial_cmp(&dc).unwrap()
            }) {
                let (dx, dy) = (mx - b.x, my - b.y);
                let l = (dx * dx + dy * dy).sqrt().max(0.01);
                if l < 9.0 {
                    b.vx += (dx / l * 9.0 - b.vx) * 4.0 * DT;
                    b.vy += (dy / l * 9.0 - b.vy) * 4.0 * DT;
                }
                if l < mr + 0.25 {
                    hits.push((i, b.dmg));
                    b.life = 0.0;
                }
            }
            b.x += b.vx * DT;
            b.y += b.vy * DT;
        }
        self.bats.retain(|b| b.life > 0.0);
        for (i, dmg) in hits {
            if self.mobs[i].alive() {
                self.hit_mob(i, dmg, 0.0, 0.1, None, true);
            }
        }
        // Blood Moon: everything under it bleeds; you drink from each.
        let mut field_hits = vec![];
        for f in self.fields.iter_mut() {
            f.t += DT;
            for (i, m) in self.mobs.iter().enumerate() {
                if m.alive() && m.charm <= 0.0 && (m.x - f.x).powi(2) + (m.y - f.y).powi(2) < MOON_RADIUS * MOON_RADIUS {
                    field_hits.push((i, f.dps * DT));
                }
            }
        }
        self.fields.retain(|f| f.t < MOON_TIME);
        for (i, dmg) in field_hits {
            let kind = self.mobs[i].kind;
            self.mobs[i].hp -= dmg * blood_taken(kind);
            if self.mobs[i].state == MobState::Idle {
                self.mobs[i].state = MobState::Chase;
            }
            self.drain(kind, dmg, 2.0);
            if self.mobs[i].hp <= 0.0 && self.mobs[i].alive() {
                self.kill(i);
            }
        }
        // Bat form: screech every second.
        if self.p.embrace_t > 0.0 {
            let before = self.p.embrace_t;
            self.p.embrace_t -= DT;
            if (before.fract() < DT) || self.p.embrace_t <= 0.0 {
                let (px, py) = (self.p.x, self.p.y);
                let dmg = screech_dmg(self.p.embrace_rank) * self.fire_power();
                let near: Vec<usize> = (0..self.mobs.len())
                    .filter(|&i| {
                        let m = &self.mobs[i];
                        m.alive() && m.charm <= 0.0 && (m.x - px).powi(2) + (m.y - py).powi(2) < (SCREECH_RADIUS + m.r).powi(2)
                    })
                    .collect();
                self.novas.push(Nova { x: px, y: py, r: SCREECH_RADIUS, t: 0.0, blood: true });
                for i in near {
                    self.hit_mob(i, dmg, 0.0, 0.2, None, false);
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
    fn the_vampire_has_her_own_tree() {
        let sk = Skills::new(Class::Vampire);
        assert_eq!(sk.rank(Skill::BloodLance), 1);
        assert_eq!(sk.rank(Skill::Fireball), 0);
        assert_eq!(sk.actives(), vec![Skill::BloodLance]);
        assert_eq!(sk.tree().len(), 11);
        for s in sk.tree() {
            assert!(is_vampire(*s));
            assert!(!describe(*s, 1, 1.0, &sk).is_empty());
        }
        // Save / load keeps the class and her ranks.
        let mut sk = Skills { points: 3, ..Skills::new(Class::Vampire) };
        assert!(sk.learn(Skill::Rake, 1));
        let back = Skills::load_text(&sk.save_text()).unwrap();
        assert_eq!(back.class, Class::Vampire);
        assert_eq!(back.rank(Skill::Rake), 1);
        assert_eq!(back.primary, Skill::BloodLance);
        // Respec keeps Blood Lance.
        let n = sk.respec();
        assert_eq!(n, 1);
        assert_eq!(sk.rank(Skill::BloodLance), 1);
        assert_eq!(sk.class, Class::Vampire);
    }
}

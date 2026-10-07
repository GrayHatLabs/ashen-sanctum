//! The Druid, the seventh playable class (docs/DRUID_CLASS.md): a plague summoner who keeps nature's
//! balance. Her creatures fight (rats, her moss wolf, the Thorn Warden) while she spreads poison and
//! mends her own. Besides mana she has a balance between Decay and Bloom: plague skills push it toward
//! Decay, growth skills toward Bloom, and each side empowers the other's skills.
use crate::game::{Game, Light, PKind, Particle, Sfx, DT};
use crate::gfx::rgb;
use crate::mobs::{Kind, MobState};
use crate::skills::{Nova, Skill, Skills, MAX_RANK};

pub fn is_druid(s: Skill) -> bool {
    crate::skills::DRUID.contains(&s)
}

// ------------------------------------------------------------------ the balance

/// How far each side can empower the other (Cycle of Rot raises it).
pub fn balance_bonus(cycle: u8) -> f32 {
    0.3 + 0.03 * cycle as f32
}
/// Balance drift back to the middle, per second.
pub const DRIFT: f32 = 0.05;

// ------------------------------------------------------------------ numbers per rank

fn up(r: u8) -> f32 {
    (r.max(1) - 1) as f32
}
pub fn spore_dps(r: u8) -> f32 {
    6.0 + 2.2 * up(r)
}
pub const SPORE_RADIUS: f32 = 1.8;
pub const SPORE_TIME: f32 = 4.0;
pub fn spore_mana(r: u8) -> f32 {
    7.0 + 0.3 * up(r)
}
pub fn rats(r: u8) -> usize {
    3 + r as usize / 3
}
pub fn rat_time(r: u8) -> f32 {
    15.0 + 1.0 * r as f32
}
pub fn rat_mana(r: u8) -> f32 {
    12.0 + 0.5 * up(r)
}
/// Green Doctor: poison lasts longer and bites deeper.
pub fn doctor_poison(r: u8) -> f32 {
    1.0 + 0.1 * r as f32
}
pub fn lash_dmg(r: u8) -> f32 {
    12.0 + 5.0 * up(r)
}
pub fn lash_root(r: u8) -> f32 {
    1.4 + 0.08 * r as f32
}
pub const LASH_LEN: f32 = 6.0;
pub fn lash_mana(r: u8) -> f32 {
    14.0 + 0.5 * up(r)
}
pub fn wolf_mana(r: u8) -> f32 {
    25.0 + 1.0 * up(r)
}
pub fn heal_total(r: u8) -> f32 {
    40.0 + 14.0 * up(r)
}
pub const HEAL_TIME: f32 = 4.0;
pub const HEAL_RADIUS: f32 = 5.0;
pub fn heal_mana(r: u8) -> f32 {
    18.0 + 0.6 * up(r)
}
pub fn fungi(r: u8) -> usize {
    3 + r as usize / 4
}
pub fn fungus_dps(r: u8) -> f32 {
    9.0 + 3.0 * up(r)
}
pub fn fungal_mana(r: u8) -> f32 {
    22.0 + 0.7 * up(r)
}
pub fn corpse_dmg(r: u8) -> f32 {
    16.0 + 6.0 * up(r)
}
pub fn corpse_mana(r: u8) -> f32 {
    16.0 + 0.6 * up(r)
}
/// Cycle of Rot: life back when a poisoned foe dies (share of max life).
pub fn rot_heal(r: u8) -> f32 {
    if r == 0 {
        0.0
    } else {
        0.01 + 0.003 * up(r)
    }
}
pub fn pest_dps(r: u8) -> f32 {
    14.0 + 5.0 * up(r)
}
pub const PEST_RADIUS: f32 = 6.0;
pub const PEST_TIME: f32 = 8.0;
pub const PEST_CD: f32 = 20.0;
pub fn pest_mana(r: u8) -> f32 {
    45.0 + 1.0 * up(r)
}
pub fn warden_time(r: u8) -> f32 {
    30.0 + 2.0 * r as f32
}
pub const WARDEN_CD: f32 = 35.0;
pub fn warden_mana(r: u8) -> f32 {
    50.0 + 1.0 * up(r)
}

pub fn mana_cost(s: Skill, r: u8) -> f32 {
    match s {
        Skill::SporeCloud => spore_mana(r),
        Skill::RatSwarm => rat_mana(r),
        Skill::ThornLash => lash_mana(r),
        Skill::MossWolf => wolf_mana(r),
        Skill::Rejuvenate => heal_mana(r),
        Skill::FungalBloom => fungal_mana(r),
        Skill::CorpseBloom => corpse_mana(r),
        Skill::Pestilence => pest_mana(r),
        Skill::ThornWarden => warden_mana(r),
        _ => 0.0,
    }
}

pub fn cooldown_of(s: Skill) -> f32 {
    match s {
        Skill::Pestilence => PEST_CD,
        Skill::ThornWarden => WARDEN_CD,
        _ => 0.0,
    }
}

pub fn describe(s: Skill, r: u8, power: f32, sk: &Skills) -> Vec<String> {
    let power = power * sk.fire_mult();
    let at = |r: u8| -> String {
        match s {
            Skill::SporeCloud => format!("{} POISON/SEC FOR {:.0} SEC, {:.0} MANA", (spore_dps(r) * power) as i32, SPORE_TIME, spore_mana(r)),
            Skill::RatSwarm => format!("{} RATS FOR {:.0} SEC, {:.0} MANA", rats(r), rat_time(r), rat_mana(r)),
            Skill::GreenDoctor => format!("POISON +{:.0}%, YOUR CREATURES TOUGHER", (doctor_poison(r.max(1)) - 1.0) * 100.0),
            Skill::ThornLash => format!("{} DAMAGE, BINDS {:.1} SEC, {:.0} MANA", (lash_dmg(r) * power) as i32, lash_root(r), lash_mana(r)),
            Skill::MossWolf => format!("A MOSS WOLF THAT STAYS UNTIL IT FALLS, {:.0} MANA", wolf_mana(r)),
            Skill::Rejuvenate => format!("HEALS {} OVER {:.0} SEC, ALLIES TOO, {:.0} MANA", heal_total(r) as i32, HEAL_TIME, heal_mana(r)),
            Skill::FungalBloom => format!("{} MUSHROOMS, {} POISON/SEC EACH, {:.0} MANA", fungi(r), (fungus_dps(r) * power) as i32, fungal_mana(r)),
            Skill::CorpseBloom => format!("{} SPORE DAMAGE, 3 RATS CRAWL OUT, {:.0} MANA", (corpse_dmg(r) * power) as i32, corpse_mana(r)),
            Skill::CycleOfRot => format!(
                "BALANCE BONUS UP TO +{:.0}%, POISONED DEATHS HEAL {:.1}% LIFE",
                balance_bonus(r.max(1)) * 100.0,
                rot_heal(r.max(1)) * 100.0
            ),
            Skill::Pestilence => format!(
                "{} POISON/SEC ON EVERY FOE NEARBY, SPREADS, {:.0} SEC COOLDOWN, {:.0} MANA",
                (pest_dps(r) * power) as i32,
                PEST_CD,
                pest_mana(r)
            ),
            Skill::ThornWarden => format!("THE WARDEN FIGHTS FOR {:.0} SEC, {:.0} SEC COOLDOWN, {:.0} MANA", warden_time(r), WARDEN_CD, warden_mana(r)),
            _ => String::new(),
        }
    };
    let what = match s {
        Skill::SporeCloud => "A CLOUD OF GLOWING SPORES THAT POISONS EVERYTHING INSIDE IT. PUSHES TOWARD DECAY.",
        Skill::RatSwarm => "BLACK RATS SWARM OUT OF THE GROUND TO FIGHT FOR YOU. PUSHES TOWARD BLOOM.",
        Skill::GreenDoctor => "PASSIVE. ONLY YOU KNOW WHICH BOTTLE IS POISON AND WHICH IS MEDICINE.",
        Skill::ThornLash => "BLACKTHORN VINES ERUPT IN A LINE, CUTTING AND BINDING FOES. PUSHES TOWARD DECAY.",
        Skill::MossWolf => "YOUR GREAT MOSS-GROWN WOLF JOINS YOU AND STAYS UNTIL IT FALLS. PUSHES TOWARD BLOOM.",
        Skill::Rejuvenate => "GREEN SHOOTS: YOU AND YOUR CREATURES HEAL OVER A FEW SECONDS. PUSHES TOWARD BLOOM.",
        Skill::FungalBloom => "MUSHROOMS SPROUT AROUND A SPOT AND BURST INTO POISON CLOUDS. PUSHES TOWARD DECAY.",
        Skill::CorpseBloom => "A CORPSE BURSTS INTO SPORES AND RATS CRAWL OUT OF IT. PUSHES TOWARD BLOOM.",
        Skill::CycleOfRot => "PASSIVE. DEATH FEEDS GROWTH: THE BALANCE MATTERS MORE, AND POISONED DEATHS MEND YOU.",
        Skill::Pestilence => "A WAVE OF PLAGUE POISONS EVERY FOE AROUND YOU, AND IT SPREADS WHEN THEY DIE. PUSHES TOWARD DECAY.",
        Skill::ThornWarden => "THE THORN WARDEN RISES: A GUARDIAN OF DEAD TREES, ROOTS AND BONE. PUSHES TOWARD BLOOM.",
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

/// A cloud of poison (Spore Cloud, a burst mushroom).
pub struct CloudFx {
    pub x: f32,
    pub y: f32,
    pub r: f32,
    pub t: f32,
    pub dps: f32,
}

/// A mushroom about to burst.
pub struct FungusFx {
    pub x: f32,
    pub y: f32,
    pub t: f32,
    pub dps: f32,
}

/// Thorn Lash's vines, drawn while they last.
pub struct VineFx {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
    pub t: f32,
}

impl Game {
    pub(crate) fn is_druid(&self) -> bool {
        self.p.skills.class == crate::skills::Class::Druid
    }

    /// Plague damage multiplier (stronger toward Bloom) and growth multiplier (stronger toward Decay).
    pub(crate) fn plague_mult(&self) -> f32 {
        1.0 + balance_bonus(self.p.skills.rank(Skill::CycleOfRot)) * self.p.balance.max(0.0)
    }
    pub(crate) fn growth_mult(&self) -> f32 {
        1.0 + balance_bonus(self.p.skills.rank(Skill::CycleOfRot)) * (-self.p.balance).max(0.0)
    }

    fn shift_balance(&mut self, d: f32) {
        self.p.balance = (self.p.balance + d).clamp(-1.0, 1.0);
    }

    /// Poisons a foe: the stronger dose wins, and the timer refreshes.
    pub(crate) fn poison(&mut self, i: usize, dps: f32, t: f32) {
        let k = doctor_poison(self.p.skills.rank(Skill::GreenDoctor));
        let m = &mut self.mobs[i];
        if !m.alive() || m.charm > 0.0 {
            return;
        }
        m.poison = m.poison.max(dps * k);
        m.poison_t = m.poison_t.max(t * k);
        if m.state == MobState::Idle {
            m.state = MobState::Chase;
        }
    }

    /// One of her allies, at (x, y), for `t` seconds (`t` <= 0: until it falls).
    pub(crate) fn grow_ally(&mut self, kind: Kind, x: f32, y: f32, t: f32) -> usize {
        let (mut x, mut y) = (x, y);
        if self.d.blocked(x, y, 0.3) {
            (x, y) = (self.p.x, self.p.y);
        }
        let mut m = crate::mobs::Mob::new(kind, x, y, self.tier.max(1.0), &mut self.rng);
        let tough = self.growth_mult() * (1.0 + 0.1 * self.p.skills.rank(Skill::GreenDoctor) as f32);
        m.max_hp *= tough * (1.0 + 0.05 * self.p.clvl as f32);
        m.hp = m.max_hp;
        let k = self.fire_power().sqrt() * self.growth_mult();
        m.dmg = (m.dmg.0 * k, m.dmg.1 * k);
        m.charm = if t > 0.0 { t } else { 1.0e9 };
        m.thrall = t > 0.0;
        m.xp = 0.0;
        m.state = MobState::Chase;
        self.mobs.push(m);
        for _ in 0..10 {
            self.spray_at(x, y, PKind::Spore, 8.0);
        }
        self.mobs.len() - 1
    }

    /// Out of mana: a free little puff of spores at the target (her ember spark).
    pub(crate) fn spore_puff(&mut self, tx: f32, ty: f32) {
        if self.p.cast_cd > 0.0 {
            return;
        }
        self.cast_pose(0.3);
        self.p.pose = "cast";
        self.p.cast_cd = 0.45;
        self.sfx.push(Sfx::Cast);
        self.stats.casts += 1;
        let dps = 3.0 * self.p.power.sqrt() * self.plague_mult();
        self.clouds.push(CloudFx { x: tx, y: ty, r: 1.0, t: 1.5, dps });
    }

    pub(crate) fn cast_druid(&mut self, s: Skill, tx: f32, ty: f32, r: u8) {
        if self.p.cast_cd > 0.0 {
            return;
        }
        match s {
            Skill::SporeCloud => self.spore_cloud(tx, ty, r),
            Skill::RatSwarm => self.rat_swarm(r),
            Skill::ThornLash => self.thorn_lash(tx, ty, r),
            Skill::MossWolf => self.moss_wolf(r),
            Skill::Rejuvenate => self.rejuvenate(r),
            Skill::FungalBloom => self.fungal_bloom(tx, ty, r),
            Skill::CorpseBloom => self.corpse_bloom(r),
            Skill::Pestilence => self.pestilence(r),
            Skill::ThornWarden => self.thorn_warden(r),
            _ => {}
        }
    }

    fn spore_cloud(&mut self, tx: f32, ty: f32, r: u8) {
        self.p.mana -= spore_mana(r);
        self.cast_pose(0.35);
        self.p.pose = "cast";
        self.sfx.push(Sfx::Cast);
        let dps = spore_dps(r) * self.fire_power() * self.plague_mult();
        self.clouds.push(CloudFx { x: tx, y: ty, r: SPORE_RADIUS, t: SPORE_TIME, dps });
        self.shift_balance(-0.12);
    }

    fn rat_swarm(&mut self, r: u8) {
        self.p.mana -= rat_mana(r);
        self.cast_pose(0.4);
        self.p.pose = "summon";
        self.sfx.push(Sfx::Descend);
        let (px, py) = (self.p.x, self.p.y);
        let n = rats(r);
        for k in 0..n {
            let a = k as f32 / n as f32 * std::f32::consts::TAU + self.rng.f();
            self.grow_ally(Kind::Rat, px + a.cos() * 1.2, py + a.sin() * 1.2, rat_time(r));
        }
        self.shift_balance(0.15);
    }

    fn thorn_lash(&mut self, tx: f32, ty: f32, r: u8) {
        self.p.mana -= lash_mana(r);
        self.cast_pose(0.4);
        self.p.pose = "cast";
        self.sfx.push(Sfx::Boom);
        let (px, py) = (self.p.x, self.p.y);
        let (dx, dy) = (tx - px, ty - py);
        let l = (dx * dx + dy * dy).sqrt().max(0.01);
        let (ux, uy) = (dx / l, dy / l);
        // The vines run until a wall stops them.
        let mut len = 0.0;
        while len < LASH_LEN {
            let (x, y) = (px + ux * (len + 0.5), py + uy * (len + 0.5));
            if !self.d.walkable(x.floor() as i32, y.floor() as i32) {
                break;
            }
            len += 0.5;
        }
        self.vines.push(VineFx { x0: px, y0: py, x1: px + ux * len, y1: py + uy * len, t: 1.2 });
        let dmg = lash_dmg(r) * self.fire_power() * self.plague_mult();
        let root = lash_root(r);
        for i in self.in_line(px + ux * len, py + uy * len, len, 0.5) {
            let t = if self.mobs[i].boss { root * 0.25 } else { root };
            self.hit_mob(i, dmg, 0.0, t, None, true);
            if self.mobs[i].alive() {
                self.poison(i, dmg * 0.25, 3.0);
            }
        }
        self.shift_balance(-0.15);
    }

    fn moss_wolf(&mut self, r: u8) {
        if self.mobs.iter().any(|m| m.alive() && m.kind == Kind::MossWolf) {
            self.say("YOUR WOLF IS ALREADY HERE".into());
            self.p.cast_cd = 0.3;
            return;
        }
        self.p.mana -= wolf_mana(r);
        self.cast_pose(0.5);
        self.p.pose = "summon";
        self.sfx.push(Sfx::Descend);
        let (px, py) = (self.p.x, self.p.y);
        let i = self.grow_ally(Kind::MossWolf, px + 1.0, py + 0.5, 0.0);
        let k = 1.0 + 0.12 * r as f32;
        let m = &mut self.mobs[i];
        m.max_hp *= k;
        m.hp = m.max_hp;
        m.dmg = (m.dmg.0 * k, m.dmg.1 * k);
        self.floater(px, py, "THE WOLF COMES".into(), rgb(0x90d070));
        self.shift_balance(0.2);
    }

    fn rejuvenate(&mut self, r: u8) {
        self.p.mana -= heal_mana(r);
        self.cast_pose(0.4);
        self.p.pose = "cast";
        self.sfx.push(Sfx::Drink);
        self.p.regrow_t = HEAL_TIME;
        self.p.regrow_rate = heal_total(r) * self.growth_mult() / HEAL_TIME;
        let (px, py) = (self.p.x, self.p.y);
        self.novas.push(Nova { x: px, y: py, r: HEAL_RADIUS, t: 0.0, blood: false, frost: false });
        self.floater(px, py, "REJUVENATE".into(), rgb(0x90e070));
        self.shift_balance(0.2);
    }

    fn fungal_bloom(&mut self, tx: f32, ty: f32, r: u8) {
        self.p.mana -= fungal_mana(r);
        self.cast_pose(0.4);
        self.p.pose = "summon";
        self.sfx.push(Sfx::Cast);
        let n = fungi(r);
        let dps = fungus_dps(r) * self.fire_power() * self.plague_mult();
        for k in 0..n {
            let a = k as f32 / n as f32 * std::f32::consts::TAU + self.rng.f() * 0.5;
            let rr = if k == 0 { 0.0 } else { 1.4 };
            let (x, y) = (tx + a.cos() * rr, ty + a.sin() * rr);
            if self.d.walkable(x.floor() as i32, y.floor() as i32) {
                self.fungi.push(FungusFx { x, y, t: 0.0, dps });
            }
        }
        self.shift_balance(-0.2);
    }

    fn corpse_bloom(&mut self, r: u8) {
        let (px, py) = (self.p.x, self.p.y);
        let corpse = (0..self.mobs.len())
            .filter(|&i| matches!(self.mobs[i].state, MobState::Dead(_)) && !self.mobs[i].boss && !self.mobs[i].thrall && !crate::breakables::is_prop(self.mobs[i].kind))
            .map(|i| (i, (self.mobs[i].x - px).powi(2) + (self.mobs[i].y - py).powi(2)))
            .filter(|&(_, d2)| d2 < 64.0)
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .map(|v| v.0);
        let Some(c) = corpse else {
            self.say("NO CORPSE NEARBY TO BLOOM".into());
            self.p.cast_cd = 0.3;
            return;
        };
        self.p.mana -= corpse_mana(r);
        self.cast_pose(0.45);
        self.p.pose = "summon";
        self.sfx.push(Sfx::Boom);
        let (x, y) = (self.mobs[c].x, self.mobs[c].y);
        // The corpse is used up.
        self.mobs[c].state = MobState::Dead(99.0);
        self.mobs[c].thrall = true;
        self.novas.push(Nova { x, y, r: 2.2, t: 0.0, blood: false, frost: false });
        for _ in 0..30 {
            self.spray_at(x, y, PKind::Spore, 14.0);
        }
        let dmg = corpse_dmg(r) * self.fire_power() * self.plague_mult();
        for i in self.in_circle(x, y, 2.2) {
            self.hit_mob(i, dmg, 0.0, 0.2, Some((x, y, 0.5)), true);
            if self.mobs[i].alive() {
                self.poison(i, dmg * 0.2, 4.0);
            }
        }
        for k in 0..3 {
            let a = k as f32 * 2.1;
            self.grow_ally(Kind::Rat, x + a.cos() * 0.8, y + a.sin() * 0.8, rat_time(r));
        }
        self.shift_balance(0.15);
    }

    fn pestilence(&mut self, r: u8) {
        self.p.mana -= pest_mana(r);
        self.p.skills.cooldown[Skill::Pestilence as usize] = PEST_CD;
        self.cast_pose(0.6);
        self.p.pose = "cast";
        self.sfx.push(Sfx::Boom);
        self.say("PESTILENCE".into());
        let (px, py) = (self.p.x, self.p.y);
        self.novas.push(Nova { x: px, y: py, r: PEST_RADIUS, t: 0.0, blood: false, frost: false });
        self.lights.push(Light { x: px, y: py, r: 200.0, s: 0.8, life: 0.5, max: 0.5 });
        let dps = pest_dps(r) * self.fire_power() * self.plague_mult();
        for i in self.in_circle(px, py, PEST_RADIUS) {
            self.poison(i, dps, PEST_TIME);
            self.mobs[i].plagued = true;
        }
        self.shift_balance(-0.3);
    }

    fn thorn_warden(&mut self, r: u8) {
        self.p.mana -= warden_mana(r);
        self.p.skills.cooldown[Skill::ThornWarden as usize] = WARDEN_CD;
        self.cast_pose(0.7);
        self.p.pose = "summon";
        self.sfx.push(Sfx::Boom);
        self.shake = self.shake.max(0.6);
        let (px, py) = (self.p.x, self.p.y);
        let i = self.grow_ally(Kind::ThornWarden, px + 1.5, py, warden_time(r));
        let k = 1.0 + 0.1 * r as f32;
        let m = &mut self.mobs[i];
        m.max_hp *= k;
        m.hp = m.max_hp;
        m.dmg = (m.dmg.0 * k, m.dmg.1 * k);
        self.say("THE THORN WARDEN RISES".into());
        self.shift_balance(0.3);
    }

    /// A poisoned foe died: Cycle of Rot heals her, and Pestilence passes the plague on.
    pub(crate) fn druid_kill(&mut self, x: f32, y: f32, poisoned: bool, plagued: bool, dps: f32) {
        if poisoned {
            let h = rot_heal(self.p.skills.rank(Skill::CycleOfRot));
            self.p.hp = (self.p.hp + self.p.max_hp * h).min(self.p.max_hp);
        }
        if plagued {
            let near = self.in_circle(x, y, 2.5);
            for i in near {
                self.poison(i, dps * 0.8, PEST_TIME * 0.6);
                self.mobs[i].plagued = true;
            }
            for _ in 0..12 {
                self.spray_at(x, y, PKind::Spore, 10.0);
            }
        }
    }

    /// Poison, clouds, mushrooms, vines, the balance and her healing, every tick.
    pub(crate) fn update_druid(&mut self) {
        // Poison ticks on every monster (so nothing stays poisoned after a respec).
        let mut died = vec![];
        for (i, m) in self.mobs.iter_mut().enumerate() {
            if m.poison_t > 0.0 && m.alive() {
                m.poison_t -= DT;
                m.hp -= m.poison * DT;
                if m.hp <= 0.0 {
                    died.push(i);
                }
                if m.poison_t <= 0.0 {
                    m.poison = 0.0;
                    m.plagued = false;
                }
            }
        }
        for i in died {
            if self.mobs[i].alive() {
                self.kill(i);
            }
        }
        if self.tick % 6 == 0 {
            let spots: Vec<(f32, f32)> = self.mobs.iter().filter(|m| m.alive() && m.poison_t > 0.0).map(|m| (m.x, m.y)).collect();
            for (x, y) in spots {
                self.parts.push(Particle { x: x + self.rng.rf(-0.3, 0.3), y, z: 20.0, vx: 0.0, vy: 0.0, vz: 10.0, life: 0.6, max: 0.6, kind: PKind::Spore });
            }
        }
        // Clouds poison what's inside.
        let mut doses = vec![];
        for c in self.clouds.iter_mut() {
            c.t -= DT;
            for (i, m) in self.mobs.iter().enumerate() {
                if m.alive() && m.charm <= 0.0 && (m.x - c.x).powi(2) + (m.y - c.y).powi(2) < (c.r + m.r).powi(2) {
                    doses.push((i, c.dps));
                }
            }
        }
        self.clouds.retain(|c| c.t > 0.0);
        for (i, dps) in doses {
            self.poison(i, dps, 1.0);
        }
        // Mushrooms burst after a moment.
        let mut bursts = vec![];
        for f in self.fungi.iter_mut() {
            f.t += DT;
            if f.t >= 1.5 {
                bursts.push((f.x, f.y, f.dps));
            }
        }
        self.fungi.retain(|f| f.t < 1.5);
        for (x, y, dps) in bursts {
            self.clouds.push(CloudFx { x, y, r: 1.4, t: 3.0, dps });
            self.sfx.push(Sfx::Boom);
            for _ in 0..16 {
                self.spray_at(x, y, PKind::Spore, 8.0);
            }
        }
        for v in self.vines.iter_mut() {
            v.t -= DT;
        }
        self.vines.retain(|v| v.t > 0.0);
        if !self.is_druid() {
            return;
        }
        // The balance drifts back to the middle.
        let b = self.p.balance;
        self.p.balance = if b > 0.0 { (b - DRIFT * DT).max(0.0) } else { (b + DRIFT * DT).min(0.0) };
        // Rejuvenate: she and her creatures heal.
        if self.p.regrow_t > 0.0 {
            self.p.regrow_t -= DT;
            let h = self.p.regrow_rate * DT;
            self.p.hp = (self.p.hp + h).min(self.p.max_hp);
            let (px, py) = (self.p.x, self.p.y);
            for m in self.mobs.iter_mut() {
                if m.alive() && m.charm > 0.0 && (m.x - px).powi(2) + (m.y - py).powi(2) < HEAL_RADIUS * HEAL_RADIUS {
                    m.hp = (m.hp + h * 2.0).min(m.max_hp);
                }
            }
            if self.tick % 4 == 0 {
                self.spray_at(px, py, PKind::Spore, 4.0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skills::Class;

    #[test]
    fn the_druid_has_her_own_tree() {
        let sk = Skills::new(Class::Druid);
        assert_eq!(sk.rank(Skill::SporeCloud), 1);
        assert_eq!(sk.actives(), vec![Skill::SporeCloud]);
        assert_eq!(sk.tree().len(), 11);
        for s in sk.tree() {
            assert!(is_druid(*s));
            assert!(!describe(*s, 1, 1.0, &sk).is_empty());
        }
        let mut sk = Skills { points: 3, ..Skills::new(Class::Druid) };
        assert!(sk.learn(Skill::RatSwarm, 1));
        let back = Skills::load_text(&sk.save_text()).unwrap();
        assert_eq!(back.class, Class::Druid);
        assert_eq!(back.rank(Skill::RatSwarm), 1);
    }
}

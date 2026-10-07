//! The Berserker, the fifth playable class and the second melee hero (docs/BERSERKER_CLASS.md):
//! a gothic barbarian warlord with a giant two-handed axe and a dire wolf at her side. No magic.
//! She runs on Rage: it starts empty, the blows she takes and the foes she kills fill it, her
//! skills spend it (Cleave is free), and it fades out of combat. The lower her life, the harder
//! she hits (up to +40%). Her axe makes foes bleed and can sunder them; Executioner beheads.
use crate::game::{move_circle, Game, Light, PKind, Particle, Sfx, DT, PLAYER_R};
use crate::gfx::rgb;
use crate::mobs::{Kind, MobState};
use crate::skills::{Nova, Skill, Skills, MAX_RANK};

pub fn is_berserker(s: Skill) -> bool {
    crate::skills::BERSERKER.contains(&s)
}

// ------------------------------------------------------------------ Rage

/// Rage per point of damage she takes.
pub const RAGE_PER_HURT: f32 = 1.0;
/// Rage per kill.
pub const RAGE_PER_KILL: f32 = 10.0;
/// Rage lost per second out of combat.
pub const RAGE_DECAY: f32 = 7.0;
/// Damage bonus at (nearly) no life left.
pub const PAIN_BONUS: f32 = 0.4;

// ------------------------------------------------------------------ numbers per rank

fn up(r: u8) -> f32 {
    (r.max(1) - 1) as f32
}
pub fn cleave_dmg(r: u8) -> (f32, f32) {
    let k = 1.0 + 0.16 * up(r);
    (11.0 * k, 18.0 * k)
}
pub const CLEAVE_REACH: f32 = 1.8;
pub fn rend_dmg(r: u8) -> (f32, f32) {
    let k = 1.0 + 0.15 * up(r);
    (8.0 * k, 12.0 * k)
}
pub fn rend_bleed(r: u8) -> f32 {
    6.0 + 2.5 * up(r)
}
pub const BLEED_TIME: f32 = 4.0;
pub const SUNDER_TIME: f32 = 6.0;
/// Sundered foes take this much more from her.
pub const SUNDER_BONUS: f32 = 0.2;
pub fn rend_rage(r: u8) -> f32 {
    12.0 + 0.3 * up(r)
}
/// Iron Hide: damage shrugged off while she's below half life.
pub fn iron_dr(r: u8) -> f32 {
    if r == 0 {
        0.0
    } else {
        (0.1 + 0.03 * up(r)).min(0.4)
    }
}
pub fn slam_dist(r: u8) -> f32 {
    5.0 + 0.2 * r as f32
}
pub fn slam_dmg(r: u8) -> f32 {
    16.0 + 6.0 * up(r)
}
pub const SLAM_RADIUS: f32 = 2.0;
pub fn slam_cd(r: u8) -> f32 {
    (4.0 - 0.15 * r as f32).max(2.0)
}
pub fn slam_rage(r: u8) -> f32 {
    15.0 + 0.4 * up(r)
}
/// The wolf: its life and bite, by Dire Wolf rank (0 = untrained) and her level.
pub fn wolf_hp(r: u8, clvl: u32) -> f32 {
    60.0 + 12.0 * clvl as f32 + 25.0 * r as f32
}
pub fn wolf_dmg(r: u8, clvl: u32) -> (f32, f32) {
    let k = 1.0 + 0.06 * clvl as f32 + 0.15 * r as f32;
    (4.0 * k, 7.0 * k)
}
pub const HOWL_RADIUS: f32 = 4.0;
pub const HOWL_CD: f32 = 10.0;
pub fn howl_rage(r: u8) -> f32 {
    15.0 + 0.4 * up(r)
}
/// Bloodlust: life back per kill (share of max life), haste per stack.
pub fn lust_heal(r: u8) -> f32 {
    if r == 0 {
        0.0
    } else {
        0.03 + 0.005 * up(r)
    }
}
pub const LUST_STACKS: u8 = 5;
pub const LUST_TIME: f32 = 4.0;
pub fn warcry_bonus(r: u8) -> f32 {
    0.25 + 0.03 * up(r)
}
pub const WARCRY_TIME: f32 = 8.0;
pub const WARCRY_RADIUS: f32 = 5.0;
pub const WARCRY_CD: f32 = 12.0;
pub fn warcry_rage(r: u8) -> f32 {
    20.0 + 0.5 * up(r)
}
pub fn whirl_dmg(r: u8) -> f32 {
    9.0 + 3.5 * up(r)
}
pub fn whirl_time(r: u8) -> f32 {
    2.0 + 0.1 * r as f32
}
pub const WHIRL_RADIUS: f32 = 1.9;
pub fn whirl_rage(r: u8) -> f32 {
    25.0 + 0.6 * up(r)
}
/// Executioner: extra damage against foes below 30% life, and the chance to behead.
pub fn exec_bonus(r: u8) -> f32 {
    if r == 0 {
        0.0
    } else {
        0.5 + 0.1 * up(r)
    }
}
pub fn behead_chance(r: u8) -> f32 {
    if r == 0 {
        0.0
    } else {
        (0.25 + 0.03 * up(r)).min(0.6)
    }
}
pub fn axe_dmg(r: u8) -> (f32, f32) {
    let k = 1.0 + 0.17 * up(r);
    (22.0 * k, 34.0 * k)
}
pub const AXE_RANGE: f32 = 7.0;
pub fn axe_rage(r: u8) -> f32 {
    30.0 + 0.8 * up(r)
}
pub fn berserk_time(r: u8) -> f32 {
    8.0 + 0.3 * r as f32
}
pub const BERSERK_CD: f32 = 35.0;
pub const EXHAUST_TIME: f32 = 3.0;
pub fn berserk_rage(r: u8) -> f32 {
    50.0 + 1.0 * up(r)
}

pub fn mana_cost(s: Skill, r: u8) -> f32 {
    match s {
        Skill::Rend => rend_rage(r),
        Skill::LeapSlam => slam_rage(r),
        Skill::DireWolf => howl_rage(r),
        Skill::WarCry => warcry_rage(r),
        Skill::Whirlwind => whirl_rage(r),
        Skill::HurlAxe => axe_rage(r),
        Skill::Berserk => berserk_rage(r),
        _ => 0.0,
    }
}

pub fn cooldown_of(s: Skill) -> f32 {
    match s {
        Skill::LeapSlam => 2.0,
        Skill::DireWolf => HOWL_CD,
        Skill::WarCry => WARCRY_CD,
        Skill::Berserk => BERSERK_CD,
        _ => 0.0,
    }
}

/// Melee skills: clicking a foe out of reach walks her in first.
pub fn reach_of(s: Skill) -> Option<f32> {
    match s {
        Skill::Cleave => Some(CLEAVE_REACH),
        Skill::Rend => Some(CLEAVE_REACH),
        _ => None,
    }
}

pub fn describe(s: Skill, r: u8, power: f32, sk: &Skills) -> Vec<String> {
    let power = power * sk.fire_mult();
    let at = |r: u8| -> String {
        match s {
            Skill::Cleave => {
                let (a, b) = cleave_dmg(r);
                format!("{}-{} DAMAGE IN A WIDE ARC, FREE", (a * power) as i32, (b * power) as i32)
            }
            Skill::Rend => {
                let (a, b) = rend_dmg(r);
                format!(
                    "{}-{} DAMAGE, BLEEDS {} DAMAGE/SEC, SUNDERS, {:.0} RAGE",
                    (a * power) as i32,
                    (b * power) as i32,
                    (rend_bleed(r) * power) as i32,
                    rend_rage(r)
                )
            }
            Skill::IronHide => format!("TAKE {:.0}% LESS DAMAGE BELOW HALF LIFE", iron_dr(r.max(1)) * 100.0),
            Skill::LeapSlam => format!(
                "{:.1} TILES, {} DAMAGE, KNOCKS DOWN, {:.1} SEC COOLDOWN, {:.0} RAGE",
                slam_dist(r),
                (slam_dmg(r) * power) as i32,
                slam_cd(r),
                slam_rage(r)
            ),
            Skill::DireWolf => {
                let (a, b) = wolf_dmg(r, 18);
                format!("WOLF BITES {}-{} (AT CHAR LEVEL 18), HOWL SCATTERS FOES, {:.0} RAGE", a as i32, b as i32, howl_rage(r))
            }
            Skill::Bloodlust => format!("EACH KILL HEALS {:.1}% LIFE AND STACKS FASTER ATTACKS (UP TO {})", lust_heal(r.max(1)) * 100.0, LUST_STACKS),
            Skill::WarCry => format!("+{:.0}% DAMAGE FOR {:.0} SEC, FOES FLEE, {:.0} RAGE", warcry_bonus(r) * 100.0, WARCRY_TIME, warcry_rage(r)),
            Skill::Whirlwind => format!("{} DAMAGE PER TURN FOR {:.1} SEC, {:.0} RAGE", (whirl_dmg(r) * power) as i32, whirl_time(r), whirl_rage(r)),
            Skill::Executioner => format!(
                "+{:.0}% DAMAGE BELOW 30% LIFE, {:.0}% CHANCE TO BEHEAD",
                exec_bonus(r.max(1)) * 100.0,
                behead_chance(r.max(1)) * 100.0
            ),
            Skill::HurlAxe => {
                let (a, b) = axe_dmg(r);
                format!("{}-{} DAMAGE, PIERCES, RETURNS, {:.0} RAGE", (a * power) as i32, (b * power) as i32, axe_rage(r))
            }
            Skill::Berserk => format!(
                "{:.1} SEC: CAN'T DIE, FASTER, HITS HEAL; THEN EXHAUSTED. {:.0} SEC COOLDOWN, {:.0} RAGE",
                berserk_time(r),
                BERSERK_CD,
                berserk_rage(r)
            ),
            _ => String::new(),
        }
    };
    let what = match s {
        Skill::Cleave => "A HEAVY AXE SWING THROUGH EVERYTHING IN A WIDE ARC. FREE: YOUR RAGE COMES FROM PAIN AND KILLS.",
        Skill::Rend => "A SAVAGE CHOP: THE TARGET AND THOSE BESIDE IT BLEED, AND THEIR ARMOR IS SUNDERED.",
        Skill::IronHide => "PASSIVE. THE CLOSER TO DEATH, THE HARDER YOU ARE TO KILL.",
        Skill::LeapSlam => "LEAP TO A SPOT AND BRING THE AXE DOWN: EVERYTHING AROUND THE LANDING IS KNOCKED DOWN.",
        Skill::DireWolf => "YOUR DIRE WOLF FIGHTS BESIDE YOU ALWAYS. EACH RANK MAKES IT STRONGER; CAST IT TO HOWL AND SCATTER FOES.",
        Skill::Bloodlust => "PASSIVE. EVERY KILL HEALS YOU AND QUICKENS YOUR AXE FOR A FEW SECONDS.",
        Skill::WarCry => "A BATTLE SHOUT: FOES NEARBY FLEE IN TERROR AND YOU HIT HARDER.",
        Skill::Whirlwind => "SPIN WITH THE AXE OUT, MOVING AS YOU SPIN, SHREDDING EVERYTHING YOU PASS.",
        Skill::Executioner => "PASSIVE. FINISH THE WOUNDED: HUGE DAMAGE TO FOES NEAR DEATH, AND A KILLING BLOW MAY TAKE THEIR HEAD.",
        Skill::HurlAxe => "THROW THE GIANT AXE: IT SPINS THROUGH A LINE OF FOES AND COMES BACK TO YOUR HAND.",
        Skill::Berserk => "THE RED MIST. FOR A WHILE YOU CANNOT DIE, YOU SWING FASTER AND EVERY HIT HEALS. AFTERWARD YOU ARE SPENT.",
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

/// The thrown axe: out through a line of foes, then back to her hand.
pub struct AxeFx {
    pub x: f32,
    pub y: f32,
    pub ux: f32,
    pub uy: f32,
    pub gone: f32,
    pub back: bool,
    pub dmg: f32,
    pub hit: Vec<usize>,
    pub spin: f32,
}

impl Game {
    pub(crate) fn is_berserker(&self) -> bool {
        self.p.skills.class == crate::skills::Class::Berserker
    }

    /// Her damage multiplier on top of power: pain, the war cry, berserk, exhaustion.
    pub(crate) fn fury(&self) -> f32 {
        if !self.is_berserker() {
            return 1.0;
        }
        let missing = 1.0 - (self.p.hp / self.p.max_hp).clamp(0.0, 1.0);
        let mut k = 1.0 + PAIN_BONUS * missing;
        if self.p.warcry_t > 0.0 {
            k *= 1.0 + warcry_bonus(self.p.skills.rank(Skill::WarCry).max(1));
        }
        if self.p.exhaust_t > 0.0 {
            k *= 0.7;
        }
        k
    }

    pub(crate) fn gain_rage(&mut self, v: f32) {
        self.p.mana = (self.p.mana + v).min(self.p.max_mana);
        self.p.fight_t = crate::valkyrie::FIGHT_TIME;
    }

    /// One of her blows: sunder and Executioner bonuses, the hit, blood, and the beheading.
    pub(crate) fn axe_hit(&mut self, i: usize, dmg: f32, stun: f32, knock: Option<(f32, f32, f32)>) {
        let m = &self.mobs[i];
        let low = m.hp < m.max_hp * 0.3;
        let (kind, boss, mx, my) = (m.kind, m.boss, m.x, m.y);
        let exec = self.p.skills.rank(Skill::Executioner);
        let mut dmg = dmg;
        if m.sunder > 0.0 {
            dmg *= 1.0 + SUNDER_BONUS;
        }
        if low {
            dmg *= 1.0 + exec_bonus(exec);
        }
        self.hit_mob(i, dmg, 0.0, stun, knock, true);
        if !crate::vampire::bloodless(kind) {
            for _ in 0..5 {
                self.spray_at(mx, my, PKind::Blood, 16.0);
            }
        }
        if self.p.berserk_t > 0.0 {
            self.p.hp = (self.p.hp + dmg * 0.08).min(self.p.max_hp);
        }
        // A killing blow on the wounded can take the head (D2-style gore).
        if !self.mobs[i].alive() && low && !boss && exec > 0 && self.rng.f() < behead_chance(exec) {
            self.behead(mx, my, kind);
        }
    }

    fn behead(&mut self, x: f32, y: f32, kind: Kind) {
        self.floater(x, y, "BEHEADED!".into(), rgb(0xd04030));
        self.shake = self.shake.max(0.4);
        let pk = if crate::vampire::bloodless(kind) { PKind::Bone } else { PKind::Blood };
        for k in 0..30 {
            let a = k as f32 / 30.0 * std::f32::consts::TAU;
            self.parts.push(Particle { x, y, z: 30.0, vx: a.cos() * 3.0, vy: a.sin() * 3.0, vz: 40.0 + (k % 5) as f32 * 8.0, life: 0.7, max: 0.7, kind: pk });
        }
        // The head flies.
        self.parts.push(Particle { x, y, z: 34.0, vx: self.rng.rf(-2.0, 2.0), vy: self.rng.rf(-2.0, 2.0), vz: 70.0, life: 1.0, max: 1.0, kind: PKind::Bone });
        self.decals.push(crate::game::Decal { x, y, r: 0.6, col: rgb(0x400808), a: 0.6 });
        // The others lose their nerve.
        for j in self.in_circle(x, y, 3.5) {
            if !self.mobs[j].boss {
                self.mobs[j].flee = self.mobs[j].flee.max(2.0);
            }
        }
    }

    fn bz_pose(&mut self, pose: &'static str, t: f32) {
        self.cast_pose(t / self.p.haste.max(0.5));
        self.p.pose = pose;
    }

    /// Cleave: free, and her out-of-rage attack.
    pub(crate) fn cleave(&mut self, tx: f32, ty: f32) {
        if self.p.cast_cd > 0.0 {
            return;
        }
        let r = self.p.skills.rank(Skill::Cleave).max(1);
        self.bz_pose("attack", 0.4);
        self.sfx.push(Sfx::Swing);
        self.stats.casts += 1;
        let (px, py) = (self.p.x, self.p.y);
        let a0 = (ty - py).atan2(tx - px);
        for k in 0..12 {
            let a = a0 - 1.1 + k as f32 * 0.2;
            self.parts.push(Particle { x: px + a.cos() * 1.3, y: py + a.sin() * 1.3, z: 18.0, vx: -a.sin() * 2.0, vy: a.cos() * 2.0, vz: 0.0, life: 0.15, max: 0.15, kind: PKind::Smoke });
        }
        let (lo, hi) = cleave_dmg(r);
        let power = self.fire_power();
        for i in self.in_cone(tx, ty, CLEAVE_REACH, 1.1) {
            let dmg = self.rng.rf(lo, hi) * power;
            self.axe_hit(i, dmg, 0.25, Some((px, py, 0.35)));
        }
    }

    pub(crate) fn cast_berserker(&mut self, s: Skill, tx: f32, ty: f32, r: u8) {
        if self.p.cast_cd > 0.0 {
            return;
        }
        match s {
            Skill::Cleave => self.cleave(tx, ty),
            Skill::Rend => self.rend(tx, ty, r),
            Skill::LeapSlam => self.leap_slam(tx, ty, r),
            Skill::DireWolf => self.howl(r),
            Skill::WarCry => self.war_cry(r),
            Skill::Whirlwind => self.whirlwind(tx, ty, r),
            Skill::HurlAxe => self.hurl_axe(tx, ty, r),
            Skill::Berserk => self.berserk(r),
            _ => {}
        }
    }

    fn rend(&mut self, tx: f32, ty: f32, r: u8) {
        self.p.mana -= rend_rage(r);
        self.bz_pose("chop", 0.45);
        self.sfx.push(Sfx::Swing);
        let (px, py) = (self.p.x, self.p.y);
        let (lo, hi) = rend_dmg(r);
        let power = self.fire_power();
        let bleed = rend_bleed(r) * power;
        for i in self.in_cone(tx, ty, CLEAVE_REACH, 0.7) {
            let dmg = self.rng.rf(lo, hi) * power;
            self.axe_hit(i, dmg, 0.3, None);
            if self.mobs[i].alive() {
                let m = &mut self.mobs[i];
                m.bleed = m.bleed.max(bleed);
                m.bleed_t = BLEED_TIME;
                m.sunder = SUNDER_TIME;
            }
        }
        let _ = (px, py);
    }

    fn leap_slam(&mut self, tx: f32, ty: f32, r: u8) {
        let (px, py) = (self.p.x, self.p.y);
        let (dx, dy) = (tx - px, ty - py);
        let l = (dx * dx + dy * dy).sqrt().max(0.01);
        let (ux, uy) = (dx / l, dy / l);
        let mut best = None;
        let mut d = 0.5;
        while d <= l.min(slam_dist(r)) + 0.01 {
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
        self.p.mana -= slam_rage(r);
        self.p.skills.cooldown[Skill::LeapSlam as usize] = slam_cd(r);
        self.bz_pose("chop", 0.35);
        for k in 0..16 {
            let t = k as f32 / 16.0;
            let (x, y) = (px + (lx - px) * t, py + (ly - py) * t);
            self.parts.push(Particle { x, y, z: 10.0 + (t * std::f32::consts::PI).sin() * 34.0, vx: 0.0, vy: 0.0, vz: 4.0, life: 0.4, max: 0.4, kind: PKind::Smoke });
        }
        (self.p.x, self.p.y) = (lx, ly);
        self.p.path.clear();
        self.p.goal = None;
        self.sfx.push(Sfx::Boom);
        self.shake = self.shake.max(0.7);
        self.novas.push(Nova { x: lx, y: ly, r: SLAM_RADIUS, t: 0.0, blood: false, frost: false });
        self.decals.push(crate::game::Decal { x: lx, y: ly, r: 0.9, col: rgb(0x201810), a: 0.5 });
        let dmg = slam_dmg(r) * self.fire_power();
        for i in self.in_circle(lx, ly, SLAM_RADIUS) {
            // Knocked down: a long stagger.
            self.axe_hit(i, dmg, 1.2, Some((lx, ly, 0.8)));
        }
    }

    fn howl(&mut self, r: u8) {
        let Some(w) = self.mobs.iter().position(|m| m.kind == Kind::DireWolf && m.alive()) else {
            self.say("YOUR WOLF ISN'T HERE".into());
            self.p.cast_cd = 0.3;
            return;
        };
        self.p.mana -= howl_rage(r);
        self.p.skills.cooldown[Skill::DireWolf as usize] = HOWL_CD;
        self.bz_pose("cast", 0.3);
        self.sfx.push(Sfx::Descend);
        let (wx, wy) = (self.mobs[w].x, self.mobs[w].y);
        self.floater(wx, wy, "AWOOOO!".into(), rgb(0xd8c8a0));
        self.novas.push(Nova { x: wx, y: wy, r: HOWL_RADIUS, t: 0.0, blood: false, frost: false });
        for i in self.in_circle(wx, wy, HOWL_RADIUS) {
            if !self.mobs[i].boss {
                self.mobs[i].flee = self.mobs[i].flee.max(2.5 + 0.1 * r as f32);
            }
        }
        // The howl fires the wolf up: it bites harder for a while.
        self.p.howl_t = 6.0;
    }

    fn war_cry(&mut self, r: u8) {
        self.p.mana -= warcry_rage(r);
        self.p.skills.cooldown[Skill::WarCry as usize] = WARCRY_CD;
        self.bz_pose("cast", 0.4);
        self.sfx.push(Sfx::Boom);
        let (px, py) = (self.p.x, self.p.y);
        self.floater(px, py, "WAR CRY!".into(), rgb(0xe0a060));
        self.shake = self.shake.max(0.4);
        self.p.warcry_t = WARCRY_TIME;
        self.novas.push(Nova { x: px, y: py, r: WARCRY_RADIUS, t: 0.0, blood: true, frost: false });
        for i in self.in_circle(px, py, WARCRY_RADIUS) {
            if !self.mobs[i].boss {
                self.mobs[i].flee = self.mobs[i].flee.max(2.5);
            }
        }
    }

    fn whirlwind(&mut self, tx: f32, ty: f32, r: u8) {
        self.p.mana -= whirl_rage(r);
        let t = whirl_time(r);
        self.p.whirl_t = t;
        self.p.whirl_to = (tx, ty);
        self.p.whirl_tick = 0.0;
        self.p.whirl_dmg = whirl_dmg(r) * self.fire_power();
        self.cast_pose(t);
        self.p.pose = "whirl";
        self.sfx.push(Sfx::Swing);
    }

    fn hurl_axe(&mut self, tx: f32, ty: f32, r: u8) {
        self.p.mana -= axe_rage(r);
        self.bz_pose("throw", 0.4);
        self.sfx.push(Sfx::Swing);
        let (px, py) = (self.p.x, self.p.y);
        let (dx, dy) = (tx - px, ty - py);
        let l = (dx * dx + dy * dy).sqrt().max(0.01);
        let (lo, hi) = axe_dmg(r);
        let dmg = self.rng.rf(lo, hi) * self.fire_power();
        self.axes.push(AxeFx { x: px + dx / l * 0.4, y: py + dy / l * 0.4, ux: dx / l, uy: dy / l, gone: 0.0, back: false, dmg, hit: vec![], spin: 0.0 });
    }

    fn berserk(&mut self, r: u8) {
        self.p.mana -= berserk_rage(r);
        self.p.skills.cooldown[Skill::Berserk as usize] = BERSERK_CD;
        self.bz_pose("cast", 0.4);
        self.p.berserk_t = berserk_time(r);
        self.sfx.push(Sfx::Boom);
        self.say("BERSERK!".into());
        let (px, py) = (self.p.x, self.p.y);
        self.lights.push(Light { x: px, y: py, r: 200.0, s: 1.0, life: 0.5, max: 0.5 });
        for _ in 0..40 {
            self.spray_at(px, py, PKind::Blood, 24.0);
        }
    }

    /// A kill with her in the fight: rage, and Bloodlust.
    pub(crate) fn berserker_kill(&mut self) {
        self.gain_rage(RAGE_PER_KILL);
        let r = self.p.skills.rank(Skill::Bloodlust);
        if r > 0 {
            self.p.hp = (self.p.hp + self.p.max_hp * lust_heal(r)).min(self.p.max_hp);
            self.p.lust = (self.p.lust + 1).min(LUST_STACKS);
            self.p.lust_t = LUST_TIME;
        }
    }

    /// Keeps her dire wolf beside her (respawning it on a new level, or after it falls).
    fn keep_wolf(&mut self) {
        let r = self.p.skills.rank(Skill::DireWolf);
        let clvl = self.p.clvl;
        let w = self.mobs.iter().position(|m| m.kind == Kind::DireWolf && m.alive());
        match w {
            Some(i) => {
                let (px, py) = (self.p.x, self.p.y);
                let m = &mut self.mobs[i];
                m.charm = 1.0e9;
                // A howl fires it up.
                let (lo, hi) = wolf_dmg(r, clvl);
                let k = if self.p.howl_t > 0.0 { 1.5 } else { 1.0 };
                m.dmg = (lo * k, hi * k);
                // Lost far behind (a leap, a long run): it lopes back to her side.
                if (m.x - px).powi(2) + (m.y - py).powi(2) > 18.0 * 18.0 {
                    m.x = px + 1.0;
                    m.y = py + 0.5;
                    m.path.clear();
                }
            }
            None if self.p.wolf_cd <= 0.0 => {
                let (px, py) = (self.p.x, self.p.y);
                let mut spot = (px, py);
                for (dx, dy) in [(1.0, 0.6), (-1.0, 0.6), (0.6, -1.0), (-0.6, -1.0), (0.0, 1.2)] {
                    if !self.d.blocked(px + dx, py + dy, 0.35) {
                        spot = (px + dx, py + dy);
                        break;
                    }
                }
                let mut m = crate::mobs::Mob::new(Kind::DireWolf, spot.0, spot.1, 1.0, &mut self.rng);
                m.max_hp = wolf_hp(r, clvl);
                m.hp = m.max_hp;
                m.dmg = wolf_dmg(r, clvl);
                m.charm = 1.0e9;
                m.xp = 0.0;
                m.state = MobState::Chase;
                self.mobs.push(m);
            }
            None => {}
        }
    }

    /// Rage decay, bleeding, the wolf, Bloodlust, the whirlwind, thrown axes, berserk, every tick.
    pub(crate) fn update_berserker(&mut self) {
        // Bleeding and sundering run out on every monster.
        let mut bled = vec![];
        for (i, m) in self.mobs.iter_mut().enumerate() {
            m.sunder = (m.sunder - DT).max(0.0);
            if m.bleed_t > 0.0 && m.alive() {
                m.bleed_t -= DT;
                m.hp -= m.bleed * DT;
                bled.push(i);
                if m.bleed_t <= 0.0 {
                    m.bleed = 0.0;
                }
            }
        }
        for i in bled {
            if self.mobs[i].hp <= 0.0 && self.mobs[i].alive() {
                self.kill(i);
            } else if self.tick % 8 == 0 {
                let (x, y) = (self.mobs[i].x, self.mobs[i].y);
                self.spray_at(x, y, PKind::Blood, 6.0);
            }
        }
        if !self.is_berserker() {
            return;
        }
        self.p.fight_t = (self.p.fight_t - DT).max(0.0);
        if self.p.fight_t <= 0.0 && self.p.berserk_t <= 0.0 && !self.has_power(crate::items::P_UNDYING) {
            let decay = RAGE_DECAY / (1.0 + self.p.bonus.frac(crate::items::Stat::ManaRegen, 200));
            self.p.mana = (self.p.mana - decay * DT).max(0.0);
        }
        self.p.wolf_cd = (self.p.wolf_cd - DT).max(0.0);
        self.p.howl_t = (self.p.howl_t - DT).max(0.0);
        self.p.warcry_t = (self.p.warcry_t - DT).max(0.0);
        self.p.lust_t = (self.p.lust_t - DT).max(0.0);
        if self.p.lust_t <= 0.0 {
            self.p.lust = 0;
        }
        if self.p.berserk_t > 0.0 {
            self.p.berserk_t -= DT;
            if self.p.berserk_t <= 0.0 {
                self.p.exhaust_t = EXHAUST_TIME;
                let (x, y) = (self.p.x, self.p.y);
                self.floater(x, y, "EXHAUSTED".into(), rgb(0xa09080));
            }
        }
        self.p.exhaust_t = (self.p.exhaust_t - DT).max(0.0);
        // Attack speed: Bloodlust stacks and Berserk.
        self.p.haste = 1.0 + 0.06 * self.p.lust as f32 + if self.p.berserk_t > 0.0 { 0.5 } else { 0.0 };
        self.keep_wolf();
        // Whirlwind: she spins toward where she aimed, hitting all around a few times a second.
        if self.p.whirl_t > 0.0 {
            self.p.whirl_t -= DT;
            self.p.whirl_tick -= DT;
            let (px, py) = (self.p.x, self.p.y);
            let (tx, ty) = self.p.whirl_to;
            let (dx, dy) = (tx - px, ty - py);
            let l = (dx * dx + dy * dy).sqrt();
            if l > 0.3 {
                let (mut x, mut y) = (px, py);
                move_circle(&self.d, &mut x, &mut y, dx / l * 3.2 * DT, dy / l * 3.2 * DT, PLAYER_R);
                (self.p.x, self.p.y) = (x, y);
            }
            self.p.dir = ((self.tick / 3) % 8) as usize;
            if self.p.whirl_tick <= 0.0 {
                self.p.whirl_tick = 0.25;
                let (px, py) = (self.p.x, self.p.y);
                let dmg = self.p.whirl_dmg;
                for k in 0..10 {
                    let a = k as f32 / 10.0 * std::f32::consts::TAU + self.tick as f32 * 0.4;
                    self.parts.push(Particle { x: px + a.cos() * 1.4, y: py + a.sin() * 1.4, z: 16.0, vx: -a.sin() * 5.0, vy: a.cos() * 5.0, vz: 0.0, life: 0.18, max: 0.18, kind: PKind::Smoke });
                }
                for i in self.in_circle(px, py, WHIRL_RADIUS) {
                    self.axe_hit(i, dmg, 0.15, Some((px, py, 0.3)));
                }
            }
            if self.p.whirl_t <= 0.0 {
                self.p.cast_t = 0.0;
            }
        }
        // Thrown axes: out, then home.
        let (px, py) = (self.p.x, self.p.y);
        let mut axe_hits = vec![];
        for a in self.axes.iter_mut() {
            a.spin += DT * 20.0;
            let step = 13.0 * DT;
            if a.back {
                let (dx, dy) = (px - a.x, py - a.y);
                let l = (dx * dx + dy * dy).sqrt().max(0.01);
                a.ux = dx / l;
                a.uy = dy / l;
                if l < 0.6 {
                    a.gone = -1.0;
                    continue;
                }
            }
            let (nx, ny) = (a.x + a.ux * step, a.y + a.uy * step);
            if !a.back && (!self.d.walkable(nx.floor() as i32, ny.floor() as i32) || a.gone >= AXE_RANGE) {
                a.back = true;
                a.hit.clear();
                continue;
            }
            a.x = nx;
            a.y = ny;
            a.gone += step;
            for (i, m) in self.mobs.iter().enumerate() {
                if m.alive() && m.charm <= 0.0 && !a.hit.contains(&i) && (m.x - a.x).powi(2) + (m.y - a.y).powi(2) < (m.r + 0.45).powi(2) {
                    a.hit.push(i);
                    axe_hits.push((i, a.dmg));
                }
            }
        }
        self.axes.retain(|a| a.gone >= 0.0);
        for (i, dmg) in axe_hits {
            if self.mobs[i].alive() {
                self.axe_hit(i, dmg, 0.3, None);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skills::Class;

    #[test]
    fn the_berserker_has_her_own_tree() {
        let sk = Skills::new(Class::Berserker);
        assert_eq!(sk.rank(Skill::Cleave), 1);
        assert_eq!(sk.actives(), vec![Skill::Cleave]);
        assert_eq!(sk.tree().len(), 11);
        for s in sk.tree() {
            assert!(is_berserker(*s));
            assert!(!describe(*s, 1, 1.0, &sk).is_empty());
        }
        let mut sk = Skills { points: 3, ..Skills::new(Class::Berserker) };
        assert!(sk.learn(Skill::Rend, 1));
        let back = Skills::load_text(&sk.save_text()).unwrap();
        assert_eq!(back.class, Class::Berserker);
        assert_eq!(back.rank(Skill::Rend), 1);
    }
}

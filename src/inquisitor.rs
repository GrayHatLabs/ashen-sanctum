//! The Inquisitor, the eighth playable class (docs/INQUISITOR_CLASS.md): a gothic hunter of the
//! cursed who swings the Censer of Judgment on several feet of chain. Reach melee: her censer hits
//! from 2.6 tiles away, and every skill is a chain (lash, hook, binding chains, the spinning censer,
//! chains linking her branded prey). She runs on Judgment: it starts empty, her censer blows fill
//! it (more against cursed and branded foes), her skills spend it, and it fades out of combat.
//! Cursed foes (the unholy, and every boss) suffer more from her, and the stronger the curse, the
//! stronger she gets.
use crate::game::{move_circle, Game, Light, PKind, Particle, Sfx, DT, PLAYER_R};
use crate::gfx::rgb;
use crate::mobs::{Kind, Mob, MobState, Rank};
use crate::skills::{Nova, Skill, Skills, MAX_RANK};

pub fn is_inquisitor(s: Skill) -> bool {
    crate::skills::INQUISITOR.contains(&s)
}

// ------------------------------------------------------------------ the cursed

/// The unholy kinds (undead, ghosts, witches' servants, vampires, werewolves, cultists, wraiths),
/// plus every boss.
pub fn kind_cursed(k: Kind) -> bool {
    unholy(k) || crate::mobs::def(k).boss
}

fn unholy(k: Kind) -> bool {
    matches!(
        k,
        Kind::Zombie
            | Kind::Skeleton
            | Kind::Archer
            | Kind::IceWraith
            | Kind::Ghoul
            | Kind::Werewolf
            | Kind::Banshee
            | Kind::Wisp
            | Kind::Cultist
            | Kind::Bat
            | Kind::Gearwraith
    )
}

/// How cursed a monster is (0 = not at all): the unholy 1, champions and elites more, bosses most.
pub fn curse_of(m: &Mob) -> f32 {
    let unholy = unholy(m.kind);
    if m.boss {
        1.5
    } else if unholy {
        if m.rank != Rank::Normal {
            1.25
        } else {
            1.0
        }
    } else {
        0.0
    }
}

// ------------------------------------------------------------------ holy fire

/// Holy fire: a share of each censer blow burns on for a few seconds, twice as hard on the cursed.
pub const HOLY_SHARE: f32 = 0.12;
pub const HOLY_TIME: f32 = 3.0;
/// When a cursed foe dies burning, the flame leaps this far to another foe.
pub const HOLY_LEAP: f32 = 4.0;

// ------------------------------------------------------------------ Judgment

/// Judgment per censer hit: plain, on a cursed foe, on a branded cursed foe.
pub const JUDGE_HIT: f32 = 3.0;
pub const JUDGE_CURSED: f32 = 6.0;
pub const JUDGE_BRANDED: f32 = 9.0;
/// Judgment lost per second out of combat.
pub const JUDGE_DECAY: f32 = 6.0;
/// Zeal: faster for each branded cursed foe within this many tiles.
pub const ZEAL_RANGE: f32 = 10.0;
pub const ZEAL_PER: f32 = 0.05;
pub const ZEAL_MAX: f32 = 0.25;

// ------------------------------------------------------------------ numbers per rank

fn up(r: u8) -> f32 {
    (r.max(1) - 1) as f32
}
/// The censer whips out on its chain this far (Zealotry adds to it). The user wanted long whipping
/// strikes (2026-10-06; was 2.6, then 3.6): docs/INQUISITOR_WHIP_PLAN.md.
pub const CENSER_REACH: f32 = 5.5;
/// The whip's timing: drawn back over her shoulder, then unrolling out to the tip (seconds, before haste).
pub const WHIP_WIND: f32 = 0.12;
pub const WHIP_LASH: f32 = 0.18;
/// The tip (this share of the reach and beyond) hits this much harder.
pub const WHIP_TIP: f32 = 0.75;
pub const WHIP_TIP_BONUS: f32 = 1.5;
/// Every third strike in a row (each within this long of the last) is an overhead crack: further, wider at the tip.
pub const WHIP_COMBO_T: f32 = 1.5;
pub const OVERHEAD_REACH: f32 = 1.0;
pub fn censer_dmg(r: u8) -> (f32, f32) {
    let k = 1.0 + 0.16 * up(r);
    (9.0 * k, 14.0 * k)
}
pub fn brand_time(r: u8) -> f32 {
    10.0 + 0.6 * up(r)
}
/// Branded foes take this much more from her (cursed ones, this times their curse on top).
pub fn brand_bonus(r: u8) -> f32 {
    0.2 + 0.02 * up(r)
}
pub const BRAND_RANGE: f32 = 9.0;
pub const BRAND_SPLASH: f32 = 1.4;
pub fn brand_cost(r: u8) -> f32 {
    8.0 + 0.2 * up(r)
}
/// Zealotry: more damage against the cursed, and a longer chain.
pub fn zeal_bonus(r: u8) -> f32 {
    if r == 0 {
        0.0
    } else {
        0.15 + 0.04 * up(r)
    }
}
pub fn zeal_reach(r: u8) -> f32 {
    if r == 0 {
        0.0
    } else {
        0.2 + 0.05 * up(r)
    }
}
pub fn lash_dmg(r: u8) -> (f32, f32) {
    let k = 1.0 + 0.15 * up(r);
    (14.0 * k, 22.0 * k)
}
pub const LASH_REACH: f32 = 7.5;
pub fn lash_cost(r: u8) -> f32 {
    14.0 + 0.3 * up(r)
}
pub const HOOK_RANGE: f32 = 8.0;
pub fn hook_dmg(r: u8) -> f32 {
    12.0 + 4.0 * up(r)
}
pub fn hook_cost(r: u8) -> f32 {
    12.0 + 0.2 * up(r)
}
pub const HOOK_CD: f32 = 2.0;
/// Iron Halo: damage the cursed do to her is cut, and her pain feeds Judgment.
pub fn halo_dr(r: u8) -> f32 {
    if r == 0 {
        0.0
    } else {
        (0.1 + 0.03 * up(r)).min(0.4)
    }
}
pub fn halo_judge(r: u8) -> f32 {
    if r == 0 {
        0.0
    } else {
        0.5 + 0.05 * up(r)
    }
}
pub fn sweep_dmg(r: u8) -> f32 {
    10.0 + 3.5 * up(r)
}
pub fn sweep_time(r: u8) -> f32 {
    2.2 + 0.1 * r as f32
}
pub const SWEEP_RADIUS: f32 = 2.8;
pub fn sweep_cost(r: u8) -> f32 {
    24.0 + 0.5 * up(r)
}
pub fn bind_time(r: u8) -> f32 {
    2.2 + 0.15 * up(r)
}
pub const BIND_RADIUS: f32 = 2.4;
pub const BIND_RANGE: f32 = 8.0;
pub fn bind_cost(r: u8) -> f32 {
    18.0 + 0.4 * up(r)
}
pub const BIND_CD: f32 = 6.0;
/// Chain Links: the share of her damage to one branded foe that spills to the others.
pub fn link_share(r: u8) -> f32 {
    if r == 0 {
        0.0
    } else {
        (0.2 + 0.03 * up(r)).min(0.5)
    }
}
pub const LINK_RANGE: f32 = 7.0;
pub fn purify_dmg(r: u8) -> f32 {
    26.0 + 8.0 * up(r)
}
pub const PURIFY_RADIUS: f32 = 4.5;
pub fn purify_cost(r: u8) -> f32 {
    35.0 + 0.6 * up(r)
}
pub const PURIFY_CD: f32 = 8.0;
/// Chance a possessed (cursed, not boss) foe breaks free and fights for her for a few seconds.
pub fn free_chance(r: u8) -> f32 {
    (0.2 + 0.02 * up(r)).min(0.45)
}
pub fn judgment_time(r: u8) -> f32 {
    8.0 + 0.3 * r as f32
}
pub const JUDGMENT_CD: f32 = 40.0;
pub fn judgment_cost(r: u8) -> f32 {
    60.0 + 1.0 * up(r)
}
/// Final Judgment: damage per branded foe nearby (capped), and haste.
pub const JUDGMENT_PER_BRAND: f32 = 0.15;
pub const JUDGMENT_MAX: f32 = 0.75;
pub const JUDGMENT_HASTE: f32 = 0.4;

pub fn mana_cost(s: Skill, r: u8) -> f32 {
    match s {
        Skill::BrandOfJudgment => brand_cost(r),
        Skill::ChainLash => lash_cost(r),
        Skill::Hook => hook_cost(r),
        Skill::CenserSweep => sweep_cost(r),
        Skill::BindingChains => bind_cost(r),
        Skill::Purification => purify_cost(r),
        Skill::FinalJudgment => judgment_cost(r),
        _ => 0.0,
    }
}

pub fn cooldown_of(s: Skill) -> f32 {
    match s {
        Skill::Hook => HOOK_CD,
        Skill::BindingChains => BIND_CD,
        Skill::Purification => PURIFY_CD,
        Skill::FinalJudgment => JUDGMENT_CD,
        _ => 0.0,
    }
}

/// Reach skills: clicking a foe out of reach walks her in first.
pub fn reach_of(s: Skill, zealotry: u8) -> Option<f32> {
    match s {
        Skill::CenserStrike => Some(CENSER_REACH + zeal_reach(zealotry)),
        _ => None,
    }
}

pub fn describe(s: Skill, r: u8, power: f32, sk: &Skills) -> Vec<String> {
    let power = power * sk.fire_mult();
    let at = |r: u8| -> String {
        match s {
            Skill::CenserStrike => {
                let (a, b) = censer_dmg(r);
                format!("{}-{} DAMAGE, A {:.1}-TILE WHIP, THE TIP HARDER + HOLY FIRE, FREE", (a * power) as i32, (b * power) as i32, CENSER_REACH)
            }
            Skill::BrandOfJudgment => format!(
                "BRANDED FOR {:.0} SEC: +{:.0}% DAMAGE FROM YOU (MORE IF CURSED), {:.0} JUDGMENT",
                brand_time(r),
                brand_bonus(r) * 100.0,
                brand_cost(r)
            ),
            Skill::Zealotry => format!("+{:.0}% DAMAGE TO THE CURSED, +{:.2} TILES OF CHAIN", zeal_bonus(r.max(1)) * 100.0, zeal_reach(r.max(1))),
            Skill::ChainLash => {
                let (a, b) = lash_dmg(r);
                format!("{}-{} DAMAGE IN A {:.1}-TILE LINE, STAGGERS, {:.0} JUDGMENT", (a * power) as i32, (b * power) as i32, LASH_REACH, lash_cost(r))
            }
            Skill::Hook => format!("{} DAMAGE, DRAGS A FOE TO YOU (OR YOU TO A BOSS), {:.0} JUDGMENT", (hook_dmg(r) * power) as i32, hook_cost(r)),
            Skill::IronHalo => format!(
                "CURSED FOES DO {:.0}% LESS TO YOU; EACH BLOW GIVES {:.1} JUDGMENT PER 10 DAMAGE",
                halo_dr(r.max(1)) * 100.0,
                halo_judge(r.max(1)) * 10.0
            ),
            Skill::CenserSweep => format!("{} DAMAGE PER TURN FOR {:.1} SEC, SETS ALL ALIGHT, {:.0} JUDGMENT", (sweep_dmg(r) * power) as i32, sweep_time(r), sweep_cost(r)),
            Skill::BindingChains => format!("PINS FOES IN PLACE FOR {:.1} SEC, {:.0} JUDGMENT", bind_time(r), bind_cost(r)),
            Skill::ChainLinks => format!("{:.0}% OF DAMAGE TO A BRANDED FOE SPILLS TO THE OTHERS", link_share(r.max(1)) * 100.0),
            Skill::Purification => format!(
                "{} DAMAGE (DOUBLE TO THE CURSED), BRANDS ALL, {:.0}% TO FREE THE POSSESSED, {:.0} JUDGMENT",
                (purify_dmg(r) * power) as i32,
                free_chance(r) * 100.0,
                purify_cost(r)
            ),
            Skill::FinalJudgment => format!(
                "{:.1} SEC: +{:.0}% PER BRAND (UP TO {:.0}%), FASTER, HITS BURN ALL BRANDED. {:.0} JUDGMENT",
                judgment_time(r),
                JUDGMENT_PER_BRAND * 100.0,
                JUDGMENT_MAX * 100.0,
                judgment_cost(r)
            ),
            _ => String::new(),
        }
    };
    let what = match s {
        Skill::CenserStrike => "SWING THE CENSER OF JUDGMENT ON ITS CHAIN, FAR BEYOND A BLADE'S REACH. ITS HOLY FIRE BURNS ON, TWICE AS HOT ON THE CURSED, AND LEAPS FROM THEIR CORPSES. FREE.",
        Skill::BrandOfJudgment => "BURN A SEAL INTO A FOE AND THOSE BESIDE IT. THE BRANDED CANNOT HIDE, AND SUFFER MORE FROM YOU.",
        Skill::Zealotry => "PASSIVE. YOU WERE MADE TO HUNT THE CURSED: YOU HIT THEM HARDER, AND YOUR CHAIN RUNS LONGER.",
        Skill::ChainLash => "CRACK THE CHAIN OUT IN A LONG STRAIGHT LINE, CUTTING AND STAGGERING EVERYTHING ALONG IT.",
        Skill::Hook => "THROW THE CHAIN: THE FIRST FOE IT CATCHES IS DRAGGED TO YOUR FEET. A BOSS IS TOO HEAVY, SO YOU ARE PULLED TO IT.",
        Skill::IronHalo => "PASSIVE. THE HALO TURNS AWAY THE CURSED, AND EVERY WOUND HARDENS YOUR JUDGMENT.",
        Skill::CenserSweep => "SPIN THE CENSER AROUND YOU IN GREAT CIRCLES OF HOLY FIRE.",
        Skill::BindingChains => "CHAINS BURST FROM THE FLOOR AND PIN EVERY FOE THERE IN PLACE.",
        Skill::ChainLinks => "PASSIVE. YOUR BRANDS ARE CHAINED TOGETHER: WHAT ONE BRANDED FOE SUFFERS, THE OTHERS SHARE.",
        Skill::Purification => "SLAM THE CENSER INTO THE FLOOR: A WAVE OF BURNING INCENSE. MINOR CURSES BREAK, THE POSSESSED MAY TURN ON THEIR KIN.",
        Skill::FinalJudgment => "YOUR HALO IGNITES AND EVERY BRAND BLAZES. FOR A WHILE YOU ARE JUDGMENT ITSELF: THE MORE YOU HAVE BRANDED, THE MORE TERRIBLE.",
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

/// A chain drawn between two points for a moment (lash, hook, censer strike).
pub struct ChainLinkFx {
    /// A whip: the chain unrolls in a curving lash and cracks at the end (else it's drawn whole at once).
    pub whip: bool,
    /// A whip's wind-up (drawn back over her shoulder) and lash (unrolling) times; the crack lingers after.
    pub wind: f32,
    pub lash: f32,
    /// Which way the lash curls (+1 / -1: forehand and backhand), and how high it arcs (an overhead crack).
    pub side: f32,
    pub lift: f32,
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
    pub t: f32,
    pub max: f32,
}

/// A Censer Strike in flight (her whip): the damage travels out along the chain.
pub struct Whip {
    pub x: f32,
    pub y: f32,
    pub ux: f32,
    pub uy: f32,
    pub reach: f32,
    pub t: f32,
    pub wind: f32,
    pub lash: f32,
    pub lo: f32,
    pub hi: f32,
    pub power: f32,
    pub overhead: bool,
    pub hit: Vec<usize>,
    pub cracked: bool,
}

/// Binding chains holding a spot.
pub struct BindFx {
    pub x: f32,
    pub y: f32,
    pub t: f32,
    pub held: Vec<usize>,
}

impl Game {
    pub(crate) fn is_inquisitor(&self) -> bool {
        self.p.skills.class == crate::skills::Class::Inquisitor
    }

    /// Her damage multiplier on top of power: Final Judgment, by the brands around her.
    pub(crate) fn judgment(&self) -> f32 {
        if !self.is_inquisitor() || self.p.judge_t <= 0.0 {
            return 1.0;
        }
        1.0 + (self.brands_near(ZEAL_RANGE + 2.0) as f32 * JUDGMENT_PER_BRAND).min(JUDGMENT_MAX)
    }

    fn brands_near(&self, r: f32) -> usize {
        let (px, py) = (self.p.x, self.p.y);
        self.mobs.iter().filter(|m| m.alive() && m.charm <= 0.0 && m.brand_t > 0.0 && (m.x - px).powi(2) + (m.y - py).powi(2) < r * r).count()
    }

    pub(crate) fn gain_judgment(&mut self, v: f32) {
        self.p.mana = (self.p.mana + v).min(self.p.max_mana);
        self.p.fight_t = crate::valkyrie::FIGHT_TIME;
    }

    fn brand(&mut self, i: usize, t: f32) {
        let m = &mut self.mobs[i];
        if m.brand_t <= 0.0 {
            let (x, y) = (m.x, m.y);
            m.brand_t = t;
            self.floater(x, y, "BRANDED".into(), rgb(0xffb040));
        } else {
            m.brand_t = m.brand_t.max(t);
        }
    }

    /// One of her blows: brand and curse bonuses, the hit, holy fire on the cursed, Judgment, and
    /// Chain Links spilling some of it to every other branded foe.
    pub(crate) fn inq_hit(&mut self, i: usize, dmg: f32, stun: f32, knock: Option<(f32, f32, f32)>, censer: bool) {
        let m = &self.mobs[i];
        if !m.alive() || m.charm > 0.0 {
            return;
        }
        let curse = curse_of(m);
        let branded = m.brand_t > 0.0;
        let (mx, my) = (m.x, m.y);
        let mut dmg = dmg;
        if curse > 0.0 {
            dmg *= 1.0 + zeal_bonus(self.p.skills.rank(Skill::Zealotry)) * curse;
        }
        if branded {
            let b = brand_bonus(self.p.skills.rank(Skill::BrandOfJudgment).max(1));
            dmg *= 1.0 + b + b * curse;
        }
        self.hit_mob(i, dmg, 0.0, stun, knock, true);
        // Holy fire: everything her censer touches burns white-gold; the cursed burn twice as hard.
        if censer {
            self.ignite_holy(i, dmg * HOLY_SHARE * (1.0 + curse));
        }
        for _ in 0..3 {
            self.spray_at(mx, my, PKind::Holy, 14.0);
        }
        if censer {
            self.gain_judgment(if branded && curse > 0.0 {
                JUDGE_BRANDED
            } else if curse > 0.0 {
                JUDGE_CURSED
            } else {
                JUDGE_HIT
            });
        }
        // Chain Links (and Final Judgment): the other branded foes share the pain.
        let links = self.p.skills.rank(Skill::ChainLinks);
        let share = if self.p.judge_t > 0.0 && censer { 1.0 } else { link_share(links) };
        if branded && share > 0.0 {
            let others: Vec<usize> = (0..self.mobs.len())
                .filter(|&j| j != i)
                .filter(|&j| {
                    let o = &self.mobs[j];
                    o.alive() && o.charm <= 0.0 && o.brand_t > 0.0 && (o.x - mx).powi(2) + (o.y - my).powi(2) < LINK_RANGE * LINK_RANGE
                })
                .collect();
            for j in others {
                let (ox, oy) = (self.mobs[j].x, self.mobs[j].y);
                self.links.push(ChainLinkFx { whip: false, wind: 0.0, lash: 0.0, side: 1.0, lift: 0.0, x0: mx, y0: my, x1: ox, y1: oy, t: 0.0, max: 0.25 });
                self.hit_mob(j, dmg * share, 0.0, 0.0, None, true);
            }
        }
    }

    /// Sets a foe alight with holy fire (the stronger flame wins; the timer refreshes).
    pub(crate) fn ignite_holy(&mut self, i: usize, dps: f32) {
        if let Some(m) = self.mobs.get_mut(i).filter(|m| m.alive()) {
            m.holy = m.holy.max(dps);
            m.holy_t = HOLY_TIME;
        }
    }

    /// A cursed foe that dies in holy fire passes the flame to the nearest foe.
    pub(crate) fn holy_leap(&mut self, i: usize) {
        let m = &self.mobs[i];
        if m.holy_t <= 0.0 || curse_of(m) <= 0.0 {
            return;
        }
        let (x, y, dps) = (m.x, m.y, m.holy);
        let next = self
            .foes_where(|o| (o.x - x).powi(2) + (o.y - y).powi(2) < HOLY_LEAP * HOLY_LEAP)
            .into_iter()
            .filter(|&j| j != i && self.mobs[j].holy_t <= 0.0)
            .min_by(|&a, &b| {
                let da = (self.mobs[a].x - x).powi(2) + (self.mobs[a].y - y).powi(2);
                let db = (self.mobs[b].x - x).powi(2) + (self.mobs[b].y - y).powi(2);
                da.partial_cmp(&db).unwrap()
            });
        if let Some(j) = next {
            let (nx, ny) = (self.mobs[j].x, self.mobs[j].y);
            self.ignite_holy(j, dps);
            for k in 0..10 {
                let t = k as f32 / 10.0;
                self.parts.push(Particle { x: x + (nx - x) * t, y: y + (ny - y) * t, z: 22.0 + (t * std::f32::consts::PI).sin() * 16.0, vx: 0.0, vy: 0.0, vz: 6.0, life: 0.35, max: 0.35, kind: PKind::Holy });
            }
        }
        self.mobs[i].holy_t = 0.0;
    }

    fn iq_pose(&mut self, pose: &'static str, t: f32) {
        self.cast_pose(t / self.p.haste.max(0.5));
        self.p.pose = pose;
    }

    /// The Censer Strike: free, her basic blow, and her out-of-Judgment attack.
    pub(crate) fn censer_strike(&mut self, tx: f32, ty: f32) {
        if self.p.cast_cd > 0.0 {
            return;
        }
        let r = self.p.skills.rank(Skill::CenserStrike).max(1);
        // Forehand, backhand, then an overhead crack.
        self.whip_combo = if self.whip_combo_t > 0.0 { self.whip_combo % 3 + 1 } else { 1 };
        self.whip_combo_t = WHIP_COMBO_T;
        let overhead = self.whip_combo == 3;
        let full = CENSER_REACH + zeal_reach(self.p.skills.rank(Skill::Zealotry)) + if overhead { OVERHEAD_REACH } else { 0.0 };
        let haste = self.p.haste.max(0.5);
        self.iq_pose("attack", 0.5);
        self.sfx.push(Sfx::Swing);
        self.stats.casts += 1;
        let (px, py) = (self.p.x, self.p.y);
        let (dx, dy) = (tx - px, ty - py);
        let l = (dx * dx + dy * dy).sqrt().max(0.01);
        let (ux, uy) = (dx / l, dy / l);
        // The chain stops at the first wall.
        let mut reach = 0.5;
        while reach < full && self.d.walkable((px + ux * reach).floor() as i32, (py + uy * reach).floor() as i32) {
            reach += 0.25;
        }
        let reach = reach.min(full);
        let (wind, lash) = (WHIP_WIND / haste, WHIP_LASH / haste);
        let side = if self.whip_combo == 2 { -1.0 } else { 1.0 };
        let lift = if overhead { 1.0 } else { 0.0 };
        self.links.push(ChainLinkFx { whip: true, wind, lash, side, lift, x0: px, y0: py, x1: px + ux * reach, y1: py + uy * reach, t: 0.0, max: wind + lash + 0.2 });
        let (lo, hi) = censer_dmg(r);
        self.whips.push(Whip { x: px, y: py, ux, uy, reach, t: 0.0, wind, lash, lo, hi, power: self.fire_power(), overhead, hit: vec![], cracked: false });
    }

    /// The whips in flight: each foe along the chain is struck as the wave passes it, the tip hardest; then the crack.
    pub(crate) fn update_whips(&mut self) {
        self.whip_combo_t = (self.whip_combo_t - DT).max(0.0);
        let mut k = 0;
        while k < self.whips.len() {
            self.whips[k].t += DT;
            let w = &self.whips[k];
            if w.t < w.wind {
                k += 1;
                continue;
            }
            let p = ((w.t - w.wind) / w.lash).min(1.0);
            let front = w.reach * p;
            let (x, y, ux, uy, reach, overhead) = (w.x, w.y, w.ux, w.uy, w.reach, w.overhead);
            let hit = w.hit.clone();
            let struck = self.foes_where(|m| {
                let (mx, my) = (m.x - x, m.y - y);
                let along = mx * ux + my * uy;
                let side = (mx * -uy + my * ux).abs();
                let width = if overhead && along > reach * WHIP_TIP { 1.1 } else if along > reach * WHIP_TIP { 0.7 } else { 0.45 };
                along > -0.2 && along < front + m.r && along < reach + m.r && side < width + m.r
            });
            // Embers off the censer as it flies.
            let (tx, ty) = (x + ux * front, y + uy * front);
            if self.tick % 2 == 0 {
                self.parts.push(Particle { x: tx, y: ty, z: 14.0 + 20.0 * if overhead { 1.0 - p } else { 0.0 }, vx: ux * 2.0, vy: uy * 2.0, vz: 8.0, life: 0.35, max: 0.35, kind: PKind::Holy });
            }
            for i in struck.into_iter().filter(|i| !hit.contains(i)) {
                let m = &self.mobs[i];
                let along = (m.x - x) * ux + (m.y - y) * uy;
                let w = &self.whips[k];
                let mut dmg = self.rng.rf(w.lo, w.hi) * w.power;
                if along > reach * WHIP_TIP {
                    dmg *= WHIP_TIP_BONUS;
                }
                self.whips[k].hit.push(i);
                self.inq_hit(i, dmg, 0.2, Some((x, y, 0.25)), true);
            }
            if p >= 1.0 && !self.whips[k].cracked {
                // The crack: a white-gold burst at the tip.
                self.whips[k].cracked = true;
                // Chained Judgment (ancient power): the crack leaps to the nearest foe the whip missed.
                if self.has_power(crate::items::P_CHAINED) {
                    let hit = self.whips[k].hit.clone();
                    let (lo, hi, pw) = (self.whips[k].lo, self.whips[k].hi, self.whips[k].power);
                    let next = self
                        .foes_where(|m| (m.x - tx).powi(2) + (m.y - ty).powi(2) < 16.0)
                        .into_iter()
                        .find(|j| !hit.contains(j));
                    if let Some(j) = next {
                        let dmg = self.rng.rf(lo, hi) * pw * WHIP_TIP_BONUS;
                        self.inq_hit(j, dmg, 0.2, None, true);
                    }
                }
                self.sfx.push(Sfx::Hit);
                self.shake = self.shake.max(if overhead { 0.3 } else { 0.12 });
                self.lights.push(Light { x: tx, y: ty, r: if overhead { 80.0 } else { 50.0 }, s: 0.8, life: 0.2, max: 0.2 });
                for n in 0..(if overhead { 16 } else { 8 }) {
                    let a = n as f32 / if overhead { 16.0 } else { 8.0 } * std::f32::consts::TAU;
                    let v = if overhead { 7.0 } else { 4.5 };
                    self.parts.push(Particle { x: tx, y: ty, z: 14.0, vx: a.cos() * v, vy: a.sin() * v, vz: 6.0, life: 0.3, max: 0.3, kind: PKind::Holy });
                }
            }
            if self.whips[k].t > self.whips[k].wind + self.whips[k].lash + 0.05 {
                self.whips.swap_remove(k);
            } else {
                k += 1;
            }
        }
    }

    pub(crate) fn cast_inquisitor(&mut self, s: Skill, tx: f32, ty: f32, r: u8) {
        if self.p.cast_cd > 0.0 {
            return;
        }
        match s {
            Skill::CenserStrike => self.censer_strike(tx, ty),
            Skill::BrandOfJudgment => self.brand_of_judgment(tx, ty, r),
            Skill::ChainLash => self.chain_lash(tx, ty, r),
            Skill::Hook => self.hook(tx, ty, r),
            Skill::CenserSweep => self.censer_sweep(r),
            Skill::BindingChains => self.binding_chains(tx, ty, r),
            Skill::Purification => self.purification(r),
            Skill::FinalJudgment => self.final_judgment(r),
            _ => {}
        }
    }

    /// The foe nearest a point (within `r`), for aimed skills.
    fn foe_near(&self, x: f32, y: f32, r: f32) -> Option<usize> {
        self.foes_where(|m| (m.x - x).powi(2) + (m.y - y).powi(2) < (r + m.r).powi(2))
            .into_iter()
            .min_by(|&a, &b| {
                let da = (self.mobs[a].x - x).powi(2) + (self.mobs[a].y - y).powi(2);
                let db = (self.mobs[b].x - x).powi(2) + (self.mobs[b].y - y).powi(2);
                da.partial_cmp(&db).unwrap()
            })
    }

    fn brand_of_judgment(&mut self, tx: f32, ty: f32, r: u8) {
        let (px, py) = (self.p.x, self.p.y);
        let target = self.foe_near(tx, ty, 1.2).filter(|&i| {
            let m = &self.mobs[i];
            (m.x - px).powi(2) + (m.y - py).powi(2) < BRAND_RANGE * BRAND_RANGE && self.d.los(px, py, m.x, m.y)
        });
        let Some(i) = target else {
            self.say("NOTHING THERE TO BRAND".into());
            self.p.cast_cd = 0.3;
            return;
        };
        self.p.mana -= brand_cost(r);
        self.iq_pose("cast", 0.35);
        self.sfx.push(Sfx::Cast);
        let (mx, my) = (self.mobs[i].x, self.mobs[i].y);
        let t = brand_time(r);
        for j in self.in_circle(mx, my, BRAND_SPLASH) {
            self.brand(j, t);
        }
        self.brand(i, t);
        self.lights.push(Light { x: mx, y: my, r: 70.0, s: 0.8, life: 0.4, max: 0.4 });
        for k in 0..14 {
            let a = k as f32 / 14.0 * std::f32::consts::TAU;
            self.parts.push(Particle { x: mx + a.cos() * 0.5, y: my + a.sin() * 0.5, z: 20.0, vx: 0.0, vy: 0.0, vz: 10.0, life: 0.5, max: 0.5, kind: PKind::Holy });
        }
        self.gain_judgment(4.0);
    }

    fn chain_lash(&mut self, tx: f32, ty: f32, r: u8) {
        self.p.mana -= lash_cost(r);
        self.iq_pose("attack", 0.45);
        self.sfx.push(Sfx::Swing);
        let (px, py) = (self.p.x, self.p.y);
        let (dx, dy) = (tx - px, ty - py);
        let l = (dx * dx + dy * dy).sqrt().max(0.01);
        let (ux, uy) = (dx / l, dy / l);
        // The lash stops at the first wall.
        let mut reach = 0.5;
        while reach < LASH_REACH && self.d.walkable((px + ux * reach).floor() as i32, (py + uy * reach).floor() as i32) {
            reach += 0.25;
        }
        self.links.push(ChainLinkFx { whip: true, wind: 0.05, lash: 0.16, side: 1.0, lift: 0.0, x0: px, y0: py, x1: px + ux * reach, y1: py + uy * reach, t: 0.0, max: 0.36 });
        let (lo, hi) = lash_dmg(r);
        let power = self.fire_power();
        for i in self.in_line(tx, ty, reach, 0.45) {
            let dmg = self.rng.rf(lo, hi) * power;
            self.inq_hit(i, dmg, 0.5, None, false);
        }
    }

    fn hook(&mut self, tx: f32, ty: f32, r: u8) {
        let (px, py) = (self.p.x, self.p.y);
        let (dx, dy) = (tx - px, ty - py);
        let l = (dx * dx + dy * dy).sqrt().max(0.01);
        let (ux, uy) = (dx / l, dy / l);
        // The first foe along the throw, within range and in sight.
        let first = self
            .in_line(px + ux * HOOK_RANGE, py + uy * HOOK_RANGE, HOOK_RANGE, 0.5)
            .into_iter()
            .filter(|&i| self.d.los(px, py, self.mobs[i].x, self.mobs[i].y))
            .min_by(|&a, &b| {
                let da = (self.mobs[a].x - px).powi(2) + (self.mobs[a].y - py).powi(2);
                let db = (self.mobs[b].x - px).powi(2) + (self.mobs[b].y - py).powi(2);
                da.partial_cmp(&db).unwrap()
            });
        let Some(i) = first else {
            self.say("THE CHAIN FINDS NOTHING".into());
            self.p.cast_cd = 0.3;
            return;
        };
        self.p.mana -= hook_cost(r);
        self.p.skills.cooldown[Skill::Hook as usize] = HOOK_CD;
        self.iq_pose("attack", 0.4);
        self.sfx.push(Sfx::Swing);
        let (mx, my, boss, mr) = (self.mobs[i].x, self.mobs[i].y, self.mobs[i].boss, self.mobs[i].r);
        self.links.push(ChainLinkFx { whip: true, wind: 0.05, lash: 0.16, side: 1.0, lift: 0.0, x0: px, y0: py, x1: mx, y1: my, t: 0.0, max: 0.35 });
        let d = ((mx - px).powi(2) + (my - py).powi(2)).sqrt().max(0.01);
        let (hx, hy) = ((mx - px) / d, (my - py) / d);
        if boss {
            // Too heavy: she is hauled to it.
            let (mut x, mut y) = (px, py);
            move_circle(&self.d, &mut x, &mut y, hx * (d - mr - 0.9).max(0.0), hy * (d - mr - 0.9).max(0.0), PLAYER_R);
            (self.p.x, self.p.y) = (x, y);
            self.p.path.clear();
            self.p.goal = None;
        } else {
            // Dragged to her feet.
            let (mut x, mut y) = (mx, my);
            move_circle(&self.d, &mut x, &mut y, -hx * (d - 1.0).max(0.0), -hy * (d - 1.0).max(0.0), mr);
            (self.mobs[i].x, self.mobs[i].y) = (x, y);
            self.mobs[i].path.clear();
        }
        let dmg = hook_dmg(r) * self.fire_power();
        self.inq_hit(i, dmg, if boss { 0.2 } else { 1.0 }, None, false);
    }

    fn censer_sweep(&mut self, r: u8) {
        self.p.mana -= sweep_cost(r);
        let t = sweep_time(r);
        self.p.whirl_t = t;
        self.p.whirl_tick = 0.0;
        self.p.whirl_dmg = sweep_dmg(r) * self.fire_power();
        self.cast_pose(t);
        self.p.pose = "spin";
        self.sfx.push(Sfx::Swing);
    }

    fn binding_chains(&mut self, tx: f32, ty: f32, r: u8) {
        let (px, py) = (self.p.x, self.p.y);
        let (mut cx, mut cy) = (tx, ty);
        let (dx, dy) = (cx - px, cy - py);
        let l = (dx * dx + dy * dy).sqrt();
        if l > BIND_RANGE {
            cx = px + dx / l * BIND_RANGE;
            cy = py + dy / l * BIND_RANGE;
        }
        if !self.d.walkable(cx.floor() as i32, cy.floor() as i32) || !self.d.los(px, py, cx, cy) {
            self.say("THE CHAINS CAN'T RISE THERE".into());
            self.p.cast_cd = 0.3;
            return;
        }
        self.p.mana -= bind_cost(r);
        self.p.skills.cooldown[Skill::BindingChains as usize] = BIND_CD;
        self.iq_pose("cast", 0.4);
        self.sfx.push(Sfx::Boom);
        let t = bind_time(r);
        let held = self.in_circle(cx, cy, BIND_RADIUS);
        for &i in &held {
            let m = &mut self.mobs[i];
            // Bosses strain free sooner.
            m.stun = m.stun.max(if m.boss { t * 0.3 } else { t });
            m.path.clear();
        }
        self.binds.push(BindFx { x: cx, y: cy, t, held });
        self.decals.push(crate::game::Decal { x: cx, y: cy, r: 1.0, col: rgb(0x2a2018), a: 0.45 });
    }

    fn purification(&mut self, r: u8) {
        self.p.mana -= purify_cost(r);
        self.p.skills.cooldown[Skill::Purification as usize] = PURIFY_CD;
        self.iq_pose("attack", 0.5);
        self.sfx.push(Sfx::Boom);
        self.shake = self.shake.max(0.8);
        let (px, py) = (self.p.x, self.p.y);
        self.floater(px, py, "PURIFY!".into(), rgb(0xffd070));
        self.novas.push(Nova { x: px, y: py, r: PURIFY_RADIUS, t: 0.0, blood: false, frost: false });
        self.lights.push(Light { x: px, y: py, r: 220.0, s: 1.2, life: 0.6, max: 0.6 });
        let dmg = purify_dmg(r) * self.fire_power();
        let t = brand_time(self.p.skills.rank(Skill::BrandOfJudgment).max(1));
        let free = free_chance(r);
        for i in self.in_circle(px, py, PURIFY_RADIUS) {
            let curse = curse_of(&self.mobs[i]);
            self.brand(i, t);
            self.inq_hit(i, if curse > 0.0 { dmg * 2.0 } else { dmg }, 0.4, Some((px, py, 0.6)), false);
            if !self.mobs[i].alive() || self.mobs[i].boss {
                continue;
            }
            // Minor curses break: champions and elites lose their powers.
            self.mobs[i].mods = 0;
            // The possessed may come to their senses and turn on their kin for a while.
            if curse > 0.0 && self.rng.f() < free {
                let (mx, my) = (self.mobs[i].x, self.mobs[i].y);
                self.mobs[i].charm = 4.0;
                self.mobs[i].brand_t = 0.0;
                self.floater(mx, my, "FREED!".into(), rgb(0xfff0c0));
            }
        }
    }

    fn final_judgment(&mut self, r: u8) {
        self.p.mana -= judgment_cost(r);
        self.p.skills.cooldown[Skill::FinalJudgment as usize] = JUDGMENT_CD;
        self.iq_pose("cast", 0.5);
        self.p.judge_t = judgment_time(r);
        self.sfx.push(Sfx::Boom);
        self.say("FINAL JUDGMENT!".into());
        let (px, py) = (self.p.x, self.p.y);
        self.lights.push(Light { x: px, y: py, r: 260.0, s: 1.5, life: 0.8, max: 0.8 });
        // Every brand blazes anew.
        let t = brand_time(self.p.skills.rank(Skill::BrandOfJudgment).max(1));
        for m in self.mobs.iter_mut() {
            if m.brand_t > 0.0 && m.alive() {
                m.brand_t = m.brand_t.max(t);
            }
        }
    }

    /// Judgment fading, Zeal, Final Judgment, the spinning censer, brands and chains, every tick.
    pub(crate) fn update_inquisitor(&mut self) {
        // Brands burn out, and holy fire burns, on every monster (whoever is playing).
        let mut burning = vec![];
        for (i, m) in self.mobs.iter_mut().enumerate() {
            m.brand_t = (m.brand_t - DT).max(0.0);
            if m.holy_t > 0.0 && m.alive() {
                m.holy_t -= DT;
                m.hp -= m.holy * DT;
                burning.push(i);
                if m.holy_t <= 0.0 {
                    m.holy = 0.0;
                }
            }
        }
        for i in burning {
            if self.mobs[i].hp <= 0.0 && self.mobs[i].alive() {
                self.kill(i);
            } else if self.tick % 3 == 0 {
                let m = &self.mobs[i];
                let (x, y) = (m.x + self.rng.rf(-0.25, 0.25), m.y + self.rng.rf(-0.15, 0.15));
                let z = self.rng.rf(6.0, 30.0);
                self.parts.push(Particle { x, y, z, vx: 0.0, vy: 0.0, vz: 22.0, life: 0.5, max: 0.5, kind: PKind::Holy });
            }
        }
        self.update_whips();
        for c in self.links.iter_mut() {
            c.t += DT;
        }
        self.links.retain(|c| c.t < c.max);
        for b in self.binds.iter_mut() {
            b.t -= DT;
        }
        self.binds.retain(|b| b.t > 0.0);
        if !self.is_inquisitor() {
            return;
        }
        self.p.fight_t = (self.p.fight_t - DT).max(0.0);
        if self.p.fight_t <= 0.0 && self.p.judge_t <= 0.0 {
            let decay = JUDGE_DECAY / (1.0 + self.p.bonus.frac(crate::items::Stat::ManaRegen, 200));
            self.p.mana = (self.p.mana - decay * DT).max(0.0);
        }
        self.p.judge_t = (self.p.judge_t - DT).max(0.0);
        // Zeal: faster among branded cursed prey; Final Judgment faster still.
        let (px, py) = (self.p.x, self.p.y);
        let prey = self.mobs.iter().filter(|m| m.alive() && m.charm <= 0.0 && m.brand_t > 0.0 && curse_of(m) > 0.0 && (m.x - px).powi(2) + (m.y - py).powi(2) < ZEAL_RANGE * ZEAL_RANGE).count();
        self.p.haste = 1.0 + (prey as f32 * ZEAL_PER).min(ZEAL_MAX) + if self.p.judge_t > 0.0 { JUDGMENT_HASTE } else { 0.0 };
        // The censer sweeping around her: a few hits a second on everything near.
        if self.p.whirl_t > 0.0 {
            self.p.whirl_t -= DT;
            self.p.whirl_tick -= DT;
            self.p.dir = ((self.tick / 4) % 8) as usize;
            if self.p.whirl_tick <= 0.0 {
                self.p.whirl_tick = 0.3;
                let dmg = self.p.whirl_dmg;
                let a = self.tick as f32 * 0.5;
                let (ex, ey) = (px + a.cos() * SWEEP_RADIUS, py + a.sin() * SWEEP_RADIUS);
                self.links.push(ChainLinkFx { whip: false, wind: 0.0, lash: 0.0, side: 1.0, lift: 0.0, x0: px, y0: py, x1: ex, y1: ey, t: 0.0, max: 0.3 });
                for k in 0..12 {
                    let a = k as f32 / 12.0 * std::f32::consts::TAU + self.tick as f32 * 0.3;
                    self.parts.push(Particle { x: px + a.cos() * 2.2, y: py + a.sin() * 2.2, z: 14.0, vx: -a.sin() * 4.0, vy: a.cos() * 4.0, vz: 4.0, life: 0.25, max: 0.25, kind: PKind::Holy });
                }
                for i in self.in_circle(px, py, SWEEP_RADIUS) {
                    self.inq_hit(i, dmg, 0.25, Some((px, py, 0.2)), true);
                }
            }
            if self.p.whirl_t <= 0.0 {
                self.p.cast_t = 0.0;
            }
        }
        // Freed foes that come back to themselves... are cursed again.
        for m in self.mobs.iter_mut() {
            if m.charm > 0.0 && m.charm < 0.05 && !m.thrall && m.kind != Kind::DireWolf {
                m.state = MobState::Chase;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{Game, Input};
    use crate::skills::Class;

    fn inq_game() -> Game {
        let mut g = Game::new(5, crate::gfx::SH_WIDE);
        g.debug_goto(crate::world::LevelId::Overworld);
        g.set_class(Class::Inquisitor);
        g.mobs.clear();
        g.p.base_hp = 5000.0;
        g.p.recalc();
        g.p.hp = g.p.max_hp;
        g
    }

    fn foe(g: &mut Game, kind: Kind, dx: f32, dy: f32) -> usize {
        let mut m = Mob::new(kind, g.p.x + dx, g.p.y + dy, 1.0, &mut g.rng);
        m.max_hp = 5000.0;
        m.hp = 5000.0;
        m.state = MobState::Idle;
        g.mobs.push(m);
        g.mobs.len() - 1
    }

    /// Puts her in open floor with room around her.
    fn clear_spot(g: &mut Game) {
        for _ in 0..2000 {
            let (x, y) = (g.rng.rf(3.0, g.d.w as f32 - 3.0), g.rng.rf(3.0, g.d.h as f32 - 3.0));
            if !g.safe_contains(x, y, 8.0) && (0..16).all(|k| {
                let a = k as f32 / 16.0 * std::f32::consts::TAU;
                [1.0f32, 2.5, 4.0, 6.0].iter().all(|r| !g.d.blocked(x + a.cos() * r, y + a.sin() * r, 0.4))
            }) {
                (g.p.x, g.p.y) = (x, y);
                return;
            }
        }
        panic!("no open floor");
    }

    #[test]
    fn the_inquisitor_has_her_own_tree() {
        let sk = Skills::new(Class::Inquisitor);
        assert_eq!(sk.rank(Skill::CenserStrike), 1);
        assert_eq!(sk.actives(), vec![Skill::CenserStrike]);
        assert_eq!(sk.tree().len(), 11);
        for s in sk.tree() {
            assert!(is_inquisitor(*s));
            assert!(!describe(*s, 1, 1.0, &sk).is_empty());
        }
        let mut sk = Skills { points: 3, ..Skills::new(Class::Inquisitor) };
        assert!(sk.learn(Skill::BrandOfJudgment, 1));
        let back = Skills::load_text(&sk.save_text()).unwrap();
        assert_eq!(back.class, Class::Inquisitor);
        assert_eq!(back.rank(Skill::BrandOfJudgment), 1);
    }

    #[test]
    fn the_censer_reaches_far_builds_judgment_and_burns_the_cursed() {
        let mut g = inq_game();
        clear_spot(&mut g);
        assert_eq!(g.p.mana, 0.0, "Judgment starts empty");
        let z = foe(&mut g, Kind::Zombie, 2.3, 0.0);
        let w = foe(&mut g, Kind::Wolf, -2.3, 0.0);
        let far = foe(&mut g, Kind::Skeleton, 0.0, 3.3);
        g.censer_strike(g.p.x, g.p.y + 4.0);
        settle(&mut g);
        assert!(g.mobs[far].hp < 5000.0, "the chain whips out 3.3 tiles");
        g.p.mana = 0.0;
        g.p.cast_cd = 0.0;
        g.whip_combo_t = 0.0;
        g.censer_strike(g.p.x + 3.0, g.p.y);
        settle(&mut g);
        assert!(g.mobs[z].hp < 5000.0, "the chain reaches 2.3 tiles");
        assert!(g.mobs[z].holy_t > 0.0, "holy fire takes it");
        let j = g.p.mana;
        assert!((j - JUDGE_CURSED).abs() < 0.01, "a cursed hit: {j}");
        g.p.cast_cd = 0.0;
        g.whip_combo_t = 0.0;
        g.censer_strike(g.p.x - 3.0, g.p.y);
        settle(&mut g);
        assert!(g.mobs[w].hp < 5000.0);
        assert!(g.mobs[w].holy < g.mobs[z].holy, "the cursed burn twice as hard");
        assert!((g.p.mana - j - JUDGE_HIT).abs() < 0.01);
        assert_eq!(curse_of(&g.mobs[w]), 0.0);
    }

    /// Lets her whips fly out and land.
    fn settle(g: &mut Game) {
        for _ in 0..40 {
            g.update_whips();
        }
    }

    #[test]
    fn the_whip_is_long_lands_late_and_cracks_hardest_at_the_tip() {
        let mut g = inq_game();
        clear_spot(&mut g);
        let near = foe(&mut g, Kind::Wolf, 1.5, 0.0);
        let tip = foe(&mut g, Kind::Wolf, 5.0, 0.0);
        let beyond = foe(&mut g, Kind::Wolf, 0.0, 7.0);
        g.censer_strike(g.p.x + 6.0, g.p.y);
        assert_eq!(g.mobs[tip].hp, 5000.0, "nothing is hit before the chain gets there");
        settle(&mut g);
        let (dn, dt) = (5000.0 - g.mobs[near].hp, 5000.0 - g.mobs[tip].hp);
        assert!(dn > 0.0 && dt > 0.0, "the chain strikes all along its length");
        assert!(dt > dn * 1.2, "the tip cracks hardest: {dt} vs {dn}");
        // Straight up: 7 tiles is past her reach.
        g.p.cast_cd = 0.0;
        g.whip_combo_t = 0.0;
        g.censer_strike(g.p.x, g.p.y + 8.0);
        settle(&mut g);
        assert_eq!(g.mobs[beyond].hp, 5000.0, "7 tiles is past the whip");
    }

    #[test]
    fn every_third_lash_is_an_overhead_crack_that_reaches_further() {
        let mut g = inq_game();
        clear_spot(&mut g);
        let far = foe(&mut g, Kind::Wolf, 6.2, 0.0);
        for k in 1..=3 {
            g.p.cast_cd = 0.0;
            g.censer_strike(g.p.x + 8.0, g.p.y);
            assert_eq!(g.whip_combo, k);
            settle(&mut g);
            g.whip_combo_t = WHIP_COMBO_T;
            if k < 3 {
                assert_eq!(g.mobs[far].hp, 5000.0, "6.2 tiles is out of a plain lash's reach");
            }
        }
        assert!(g.mobs[far].hp < 5000.0, "the overhead crack reaches further");
        assert_eq!(g.links.last().map(|c| c.lift), Some(1.0));
    }

    #[test]
    fn brands_hurt_more_link_together_and_hasten_her() {
        let mut g = inq_game();
        clear_spot(&mut g);
        let a = foe(&mut g, Kind::Skeleton, 2.0, 0.0);
        let b = foe(&mut g, Kind::Skeleton, 2.0, 2.5);
        let c = foe(&mut g, Kind::Skeleton, 2.0, -2.5);
        // Unbranded baseline.
        let before = g.mobs[a].hp;
        g.inq_hit(a, 10.0, 0.0, None, false);
        let plain = before - g.mobs[a].hp;
        // Brand all three, then hit one: more damage, and Chain Links spill to the others.
        g.p.skills.rank[Skill::ChainLinks as usize] = 1;
        for i in [a, b, c] {
            g.brand(i, 10.0);
        }
        let (hb, hc) = (g.mobs[b].hp, g.mobs[c].hp);
        let before = g.mobs[a].hp;
        g.inq_hit(a, 10.0, 0.0, None, false);
        let branded = before - g.mobs[a].hp;
        assert!(branded > plain * 1.3, "branded cursed: {branded} vs {plain}");
        assert!(g.mobs[b].hp < hb && g.mobs[c].hp < hc, "the links share the pain");
        // Zeal: three branded cursed foes near her.
        g.update(&Input::default());
        assert!((g.p.haste - (1.0 + 3.0 * ZEAL_PER)).abs() < 0.01, "haste {}", g.p.haste);
    }

    #[test]
    fn the_hook_drags_foes_in_and_her_to_bosses() {
        let mut g = inq_game();
        clear_spot(&mut g);
        g.p.mana = 100.0;
        let z = foe(&mut g, Kind::Zombie, 5.0, 0.0);
        g.hook(g.p.x + 6.0, g.p.y, 1);
        let d = ((g.mobs[z].x - g.p.x).powi(2) + (g.mobs[z].y - g.p.y).powi(2)).sqrt();
        assert!(d < 1.6, "dragged to her feet: {d}");
        assert!(g.mobs[z].stun > 0.5);
        // A boss pulls her instead.
        g.mobs.clear();
        g.p.skills.cooldown[Skill::Hook as usize] = 0.0;
        g.p.cast_cd = 0.0;
        let (sx, sy) = (g.p.x, g.p.y);
        let k = foe(&mut g, Kind::BoneWarden, 5.0, 0.0);
        g.mobs[k].boss = true;
        let (bx, by) = (g.mobs[k].x, g.mobs[k].y);
        g.hook(g.p.x + 6.0, g.p.y, 1);
        assert_eq!((g.mobs[k].x, g.mobs[k].y), (bx, by), "too heavy to move");
        assert!(g.p.x > sx + 2.0, "she flies to it: from {sx},{sy} to {},{}", g.p.x, g.p.y);
    }

    #[test]
    fn chains_bind_lash_sweep_purify_and_judge() {
        let mut g = inq_game();
        clear_spot(&mut g);
        g.p.mana = 100.0;
        let (px, py) = (g.p.x, g.p.y);
        // Binding chains pin a group.
        let a = foe(&mut g, Kind::Ghoul, 4.0, 0.3);
        let b = foe(&mut g, Kind::Ghoul, 4.0, -0.3);
        g.binding_chains(px + 4.0, py, 1);
        assert!(g.mobs[a].stun > 2.0 && g.mobs[b].stun > 2.0);
        // The lash cuts a line.
        g.p.cast_cd = 0.0;
        let far = foe(&mut g, Kind::Wolf, 5.0, 0.0);
        g.chain_lash(px + 6.0, py, 1);
        assert!(g.mobs[far].hp < 5000.0, "the lash reaches 5 tiles");
        // The sweep spins around her.
        g.p.cast_cd = 0.0;
        g.p.mana = 100.0;
        let near = foe(&mut g, Kind::Wolf, -1.5, 1.0);
        g.censer_sweep(1);
        for _ in 0..30 {
            g.update(&Input::default());
        }
        assert!(g.mobs[near].hp < 5000.0);
        // Purification: brands all, strips a champion's powers.
        g.p.cast_cd = 0.0;
        g.p.mana = 100.0;
        let champ = foe(&mut g, Kind::Skeleton, 1.0, -1.5);
        g.mobs[champ].mods = 3;
        g.purification(1);
        assert_eq!(g.mobs[champ].mods, 0);
        assert!(g.mobs[champ].brand_t > 0.0 || g.mobs[champ].charm > 0.0);
        // Final Judgment: stronger with every brand near.
        g.p.cast_cd = 0.0;
        g.p.mana = 100.0;
        g.final_judgment(1);
        assert!(g.p.judge_t > 0.0);
        assert!(g.judgment() > 1.0);
    }

    #[test]
    fn holy_fire_burns_on_and_leaps_from_the_cursed_dead() {
        let mut g = inq_game();
        clear_spot(&mut g);
        let a = foe(&mut g, Kind::Zombie, 2.0, 0.0);
        let b = foe(&mut g, Kind::Wolf, 4.5, 0.0);
        g.mobs[a].hp = 30.0;
        g.ignite_holy(a, 50.0);
        let hp_b = g.mobs[b].hp;
        for _ in 0..60 {
            g.update(&Input::default());
        }
        assert!(!g.mobs[a].alive(), "burned away");
        assert!(g.mobs[b].holy_t > 0.0 || g.mobs[b].hp < hp_b, "the flame leapt to the wolf");
    }

    #[test]
    fn her_halo_turns_away_the_cursed() {
        let mut g = inq_game();
        clear_spot(&mut g);
        g.p.skills.rank[Skill::IronHalo as usize] = 5;
        let hp = g.p.hp;
        g.hurt_by(25.0, Some(Kind::Zombie));
        let cursed = hp - g.p.hp;
        let hp = g.p.hp;
        g.hurt_by(25.0, Some(Kind::Wolf));
        let plain = hp - g.p.hp;
        assert!(cursed < plain, "{cursed} vs {plain}");
        assert!(g.p.mana > 0.0, "pain feeds Judgment");
    }
}

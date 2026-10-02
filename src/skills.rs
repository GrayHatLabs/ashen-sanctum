//! Fire skills: D2-style skill points, ranks, two skill slots, and the skill tree screen.
//! The whole plan in docs/SKILLS_PLAN.md is built: Fireball, Inferno, Warmth, Fire Nova, Fire Wall,
//! Blaze, Combust, Meteor, Fire Mastery, Hydra and Ash Phoenix.
use crate::game::{move_circle, Game, Input, Light, PKind, Particle, Sfx, CAST_TIME, DT, HUD_H};
use crate::gfx::{rgb, Align, Fx, Screen, BLACK};
use crate::mobs::MobState;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Skill {
    Fireball,
    Inferno,
    Warmth,
    FireNova,
    FireWall,
    Blaze,
    Combust,
    Meteor,
    Mastery,
    Hydra,
    Phoenix,
}

pub const ALL: [Skill; 11] = [
    Skill::Fireball,
    Skill::Inferno,
    Skill::Warmth,
    Skill::FireNova,
    Skill::FireWall,
    Skill::Blaze,
    Skill::Combust,
    Skill::Meteor,
    Skill::Mastery,
    Skill::Hydra,
    Skill::Phoenix,
];
/// Character level needed for each tree row.
pub const TIER_LEVELS: [u32; 4] = [1, 6, 12, 18];
pub const MAX_RANK: u8 = 10;

pub struct Def {
    pub name: &'static str,
    /// Save-file name.
    pub key: &'static str,
    pub level: u32,
    pub prereq: Option<Skill>,
    pub passive: bool,
    /// Position in the tree: (tier row, column).
    pub cell: (i32, i32),
}

pub fn def(s: Skill) -> Def {
    match s {
        Skill::Fireball => Def { name: "FIREBALL", key: "fireball", level: 1, prereq: None, passive: false, cell: (0, 0) },
        Skill::Inferno => Def { name: "INFERNO", key: "inferno", level: 1, prereq: None, passive: false, cell: (0, 1) },
        Skill::Warmth => Def { name: "WARMTH", key: "warmth", level: 1, prereq: None, passive: true, cell: (0, 2) },
        Skill::FireNova => Def { name: "FIRE NOVA", key: "firenova", level: 6, prereq: Some(Skill::Inferno), passive: false, cell: (1, 1) },
        Skill::FireWall => Def { name: "FIRE WALL", key: "firewall", level: 6, prereq: Some(Skill::Fireball), passive: false, cell: (1, 0) },
        Skill::Blaze => Def { name: "BLAZE", key: "blaze", level: 6, prereq: Some(Skill::Warmth), passive: false, cell: (1, 2) },
        Skill::Combust => Def { name: "COMBUST", key: "combust", level: 12, prereq: Some(Skill::FireWall), passive: false, cell: (2, 0) },
        Skill::Meteor => Def { name: "METEOR", key: "meteor", level: 12, prereq: Some(Skill::FireNova), passive: false, cell: (2, 1) },
        Skill::Mastery => Def { name: "FIRE MASTERY", key: "mastery", level: 12, prereq: Some(Skill::Blaze), passive: true, cell: (2, 2) },
        Skill::Hydra => Def { name: "HYDRA", key: "hydra", level: 18, prereq: Some(Skill::Combust), passive: false, cell: (3, 0) },
        Skill::Phoenix => Def { name: "ASH PHOENIX", key: "phoenix", level: 18, prereq: Some(Skill::Meteor), passive: false, cell: (3, 1) },
    }
}

/// Your skills: ranks, unspent points and what's in the two slots.
#[derive(Clone, Debug)]
pub struct Skills {
    pub rank: [u8; ALL.len()],
    pub points: u32,
    /// Left-click / pad A.
    pub primary: Skill,
    /// Right-click / pad X (1-4 or R1 to change).
    pub secondary: Skill,
    /// Seconds left before each skill can be cast again (Hydra, Ash Phoenix).
    pub cooldown: [f32; ALL.len()],
    /// + to all fire skills from gear (only skills you have learned).
    pub bonus: u8,
    /// Extra fire damage from gear (0.25 = +25%).
    pub gear_fire: f32,
}

impl Default for Skills {
    fn default() -> Self {
        // You start knowing Fireball, with one point to spend.
        let mut rank = [0; ALL.len()];
        rank[Skill::Fireball as usize] = 1;
        Skills { rank, points: 1, primary: Skill::Fireball, secondary: Skill::Fireball, cooldown: [0.0; ALL.len()], bonus: 0, gear_fire: 0.0 }
    }
}

impl Skills {
    /// Effective rank: learned points plus gear's + to skills.
    pub fn rank(&self, s: Skill) -> u8 {
        match self.rank[s as usize] {
            0 => 0,
            r => r + self.bonus,
        }
    }

    /// Points you put in yourself.
    pub fn learned(&self, s: Skill) -> u8 {
        self.rank[s as usize]
    }

    /// Why you can't put a point into this skill right now (None = you can).
    pub fn blocker(&self, s: Skill, clvl: u32) -> Option<String> {
        let d = def(s);
        if self.learned(s) >= MAX_RANK {
            return Some("MAXED".into());
        }
        if clvl < d.level {
            return Some(format!("NEEDS CHAR LEVEL {}", d.level));
        }
        if let Some(p) = d.prereq {
            if self.rank(p) == 0 {
                return Some(format!("NEEDS {}", def(p).name));
            }
        }
        if self.points == 0 {
            return Some("NO SKILL POINTS".into());
        }
        None
    }

    pub fn learn(&mut self, s: Skill, clvl: u32) -> bool {
        if self.blocker(s, clvl).is_some() {
            return false;
        }
        self.rank[s as usize] += 1;
        self.points -= 1;
        true
    }

    /// Active skills you know, in tree order (for 1-4 and cycling).
    pub fn actives(&self) -> Vec<Skill> {
        ALL.iter().copied().filter(|s| !def(*s).passive && self.rank(*s) > 0).collect()
    }

    /// Refund everything except Fireball's first rank.
    pub fn respec(&mut self) -> u32 {
        let spent: u32 = self.rank.iter().map(|r| *r as u32).sum::<u32>() - 1;
        *self = Skills { points: self.points + spent, bonus: self.bonus, gear_fire: self.gear_fire, ..Skills::default() };
        spent
    }

    /// Fire damage multiplier from Fire Mastery.
    pub fn fire_mult(&self) -> f32 {
        1.0 + 0.08 * self.rank(Skill::Mastery) as f32 + self.gear_fire
    }

    /// Burn duration multiplier from Fire Mastery.
    pub fn burn_mult(&self) -> f32 {
        1.0 + 0.1 * self.rank(Skill::Mastery) as f32
    }

    /// Mana regeneration multiplier from Warmth.
    pub fn regen_mult(&self) -> f32 {
        match self.rank(Skill::Warmth) {
            0 => 1.0,
            r => 1.3 + 0.12 * (r - 1) as f32,
        }
    }

    pub fn save_text(&self) -> String {
        let ranks: Vec<String> = ALL.iter().map(|s| format!("{}:{}", def(*s).key, self.learned(*s))).collect();
        format!("skills={}\npoints={}\nprimary={}\nsecondary={}\n", ranks.join(","), self.points, def(self.primary).key, def(self.secondary).key)
    }

    pub fn load_text(text: &str) -> Option<Skills> {
        let get = |k: &str| text.lines().find_map(|l| l.strip_prefix(k).and_then(|r| r.strip_prefix('='))).map(str::trim);
        let by_key = |k: &str| ALL.iter().copied().find(|s| def(*s).key == k);
        let mut sk = Skills {
            rank: [0; ALL.len()],
            points: get("points")?.parse().ok()?,
            primary: Skill::Fireball,
            secondary: Skill::Fireball,
            cooldown: [0.0; ALL.len()],
            bonus: 0,
            gear_fire: 0.0,
        };
        for part in get("skills")?.split(',') {
            let (k, v) = part.split_once(':')?;
            if let (Some(s), Ok(r)) = (by_key(k), v.parse::<u8>()) {
                sk.rank[s as usize] = r.min(MAX_RANK);
            }
        }
        sk.rank[Skill::Fireball as usize] = sk.learned(Skill::Fireball).max(1);
        let ranks = sk.rank;
        let slot = |k: &str| get(k).and_then(by_key).filter(|s| !def(*s).passive && ranks[*s as usize] > 0).unwrap_or(Skill::Fireball);
        let (p, q) = (slot("primary"), slot("secondary"));
        sk.primary = p;
        sk.secondary = q;
        Some(sk)
    }
}

// ------------------------------------------------------------------ numbers per rank

pub fn fireball_dmg(r: u8) -> (f32, f32) {
    let k = 1.0 + 0.15 * (r.max(1) - 1) as f32;
    (9.0 * k, 15.0 * k)
}
/// Meteor ranks boost Fireball by 6% each (synergy).
pub fn fireball_synergy(meteor_rank: u8) -> f32 {
    1.0 + 0.06 * meteor_rank as f32
}
pub fn meteor_dmg(r: u8) -> f32 {
    40.0 + 15.0 * (r.max(1) - 1) as f32
}
pub fn meteor_radius(r: u8) -> f32 {
    1.8 + 0.06 * r as f32
}
pub fn meteor_mana(r: u8) -> f32 {
    22.0 + 1.2 * (r.max(1) - 1) as f32
}
pub const METEOR_DELAY: f32 = 1.0;
pub fn hydra_dmg(r: u8) -> f32 {
    10.0 + 4.0 * (r.max(1) - 1) as f32
}
pub fn hydra_mana(r: u8) -> f32 {
    30.0 + 1.0 * (r.max(1) - 1) as f32
}
pub const HYDRA_TIME: f32 = 10.0;
pub const HYDRA_CD: f32 = 12.0;
pub fn phoenix_time(r: u8) -> f32 {
    6.0 + 0.3 * r as f32
}
pub fn phoenix_burst(r: u8) -> f32 {
    60.0 + 20.0 * (r.max(1) - 1) as f32
}
pub fn phoenix_mana(r: u8) -> f32 {
    40.0 + 1.0 * (r.max(1) - 1) as f32
}
pub const PHOENIX_CD: f32 = 30.0;
pub const PHOENIX_RADIUS: f32 = 3.6;
pub fn fireball_mana(r: u8) -> f32 {
    5.0 + 0.3 * (r.max(1) - 1) as f32
}
pub fn inferno_dps(r: u8) -> f32 {
    16.0 + 5.0 * (r.max(1) - 1) as f32
}
pub fn inferno_range(r: u8) -> f32 {
    2.8 + 0.15 * r as f32
}
pub fn inferno_mana(r: u8) -> f32 {
    8.0 + 0.6 * (r.max(1) - 1) as f32
}
pub fn nova_dmg(r: u8) -> f32 {
    14.0 + 6.0 * (r.max(1) - 1) as f32
}
pub fn nova_radius(r: u8) -> f32 {
    2.4 + 0.08 * r as f32
}
pub fn nova_mana(r: u8) -> f32 {
    10.0 + 0.6 * (r.max(1) - 1) as f32
}

pub fn wall_dps(r: u8) -> f32 {
    20.0 + 6.0 * (r.max(1) - 1) as f32
}
pub fn wall_len(r: u8) -> f32 {
    3.0 + 0.2 * r as f32
}
pub fn wall_time(r: u8) -> f32 {
    4.0 + 0.3 * r as f32
}
pub fn wall_mana(r: u8) -> f32 {
    14.0 + 0.8 * (r.max(1) - 1) as f32
}
pub fn blaze_dps(r: u8) -> f32 {
    12.0 + 4.0 * (r.max(1) - 1) as f32
}
pub fn blaze_time(r: u8) -> f32 {
    6.0 + 0.5 * r as f32
}
/// Fire Mastery lengthens Blaze by half a second per rank (synergy).
pub fn blaze_time_with(r: u8, mastery: u8) -> f32 {
    blaze_time(r) + 0.5 * mastery as f32
}
pub fn blaze_mana(r: u8) -> f32 {
    12.0 + 0.6 * (r.max(1) - 1) as f32
}
/// Combust's base damage per burning enemy; Fire Wall ranks add 8% each (synergy).
pub fn combust_dmg(r: u8, wall_rank: u8) -> f32 {
    (25.0 + 10.0 * (r.max(1) - 1) as f32) * (1.0 + 0.08 * wall_rank as f32)
}
pub fn combust_mana(r: u8) -> f32 {
    16.0 + 1.0 * (r.max(1) - 1) as f32
}
/// How much harder an enemy that has been burning for `burned` seconds combusts.
pub fn combust_mult(burned: f32) -> f32 {
    1.0 + burned.min(4.0) * 0.25
}
pub const COMBUST_RANGE: f32 = 8.0;
/// Fire patches left by Blaze last this long.
pub const PATCH_TIME: f32 = 2.0;

/// Mana cost to start a cast (Inferno: one tick of channelling).
pub fn mana_cost(s: Skill, r: u8) -> f32 {
    match s {
        Skill::Fireball => fireball_mana(r),
        Skill::Inferno => inferno_mana(r) * DT,
        Skill::FireNova => nova_mana(r),
        Skill::FireWall => wall_mana(r),
        Skill::Blaze => blaze_mana(r),
        Skill::Combust => combust_mana(r),
        Skill::Meteor => meteor_mana(r),
        Skill::Hydra => hydra_mana(r),
        Skill::Phoenix => phoenix_mana(r),
        Skill::Warmth | Skill::Mastery => 0.0,
    }
}

/// Cooldown after casting (0 = limited only by mana and cast time).
pub fn cooldown_of(s: Skill) -> f32 {
    match s {
        Skill::Hydra => HYDRA_CD,
        Skill::Phoenix => PHOENIX_CD,
        _ => 0.0,
    }
}

/// Description lines for the tree: what it does, and this rank vs the next.
pub fn describe(s: Skill, r: u8, power: f32, sk: &Skills) -> Vec<String> {
    let wall_rank = sk.rank(Skill::FireWall);
    let power = power * if s == Skill::Mastery { 1.0 } else { sk.fire_mult() };
    let at = |r: u8| -> String {
        match s {
            Skill::Fireball => {
                let (a, b) = fireball_dmg(r);
                let k = power * fireball_synergy(sk.rank(Skill::Meteor));
                format!("{}-{} DAMAGE, {:.0} MANA", (a * k) as i32, (b * k) as i32, fireball_mana(r))
            }
            Skill::Inferno => format!("{} DAMAGE/SEC, RANGE {:.1}, {:.0} MANA/SEC", (inferno_dps(r) * power) as i32, inferno_range(r), inferno_mana(r)),
            Skill::Warmth => format!("+{:.0}% MANA REGENERATION", (1.3 + 0.12 * (r.max(1) - 1) as f32 - 1.0) * 100.0),
            Skill::FireNova => format!("{} DAMAGE, RADIUS {:.1}, {:.0} MANA", (nova_dmg(r) * power) as i32, nova_radius(r), nova_mana(r)),
            Skill::FireWall => format!(
                "{} DAMAGE/SEC, {:.1} LONG, {:.1} SEC, {:.0} MANA",
                (wall_dps(r) * power) as i32,
                wall_len(r),
                wall_time(r),
                wall_mana(r)
            ),
            Skill::Blaze => format!(
                "{} DAMAGE/SEC TRAIL FOR {:.1} SEC, {:.0} MANA",
                (blaze_dps(r) * power) as i32,
                blaze_time_with(r, sk.rank(Skill::Mastery)),
                blaze_mana(r)
            ),
            Skill::Meteor => format!("{} DAMAGE, RADIUS {:.1}, {:.0} MANA", (meteor_dmg(r) * power) as i32, meteor_radius(r), meteor_mana(r)),
            Skill::Mastery => format!(
                "+{}% FIRE DAMAGE, BURNS LAST {}% LONGER",
                (0.08 * r.max(1) as f32 * 100.0).round() as i32,
                (0.1 * r.max(1) as f32 * 100.0).round() as i32
            ),
            Skill::Hydra => format!(
                "{} DAMAGE PER BOLT FOR {:.0} SEC, {:.0} MANA, {:.0} SEC COOLDOWN",
                (hydra_dmg(r) * power) as i32,
                HYDRA_TIME,
                hydra_mana(r),
                HYDRA_CD
            ),
            Skill::Phoenix => format!(
                "{:.1} SEC, {} DAMAGE BURST, {:.0} MANA, {:.0} SEC COOLDOWN",
                phoenix_time(r),
                (phoenix_burst(r) * power) as i32,
                phoenix_mana(r),
                PHOENIX_CD
            ),
            Skill::Combust => format!(
                "{}+ DAMAGE PER BURNING FOE (UP TO 2X), {:.0} MANA",
                (combust_dmg(r, wall_rank) * power) as i32,
                combust_mana(r)
            ),
        }
    };
    let what = match s {
        Skill::Fireball => "HURLS AN EXPLODING BALL OF FIRE THAT SPLASHES AND SETS FOES BURNING.",
        Skill::Inferno => "HOLD TO BREATHE A CONE OF FLAME FROM YOUR STAFF. DRAINS MANA WHILE HELD.",
        Skill::Warmth => "PASSIVE. THE FLAME WITHIN RESTORES YOUR MANA FASTER.",
        Skill::FireNova => "A RING OF FIRE BURSTS OUT FROM YOU, BURNING AND HURLING BACK EVERYTHING NEARBY.",
        Skill::FireWall => "RAISES A LINE OF FLAMES ACROSS THE TARGET SPOT. ANYTHING THAT STANDS IN IT BURNS.",
        Skill::Blaze => "FOR A WHILE YOU LEAVE BURNING GROUND BEHIND YOU AS YOU MOVE. RUN, AND LET THEM FOLLOW.",
        Skill::Combust => "EVERY BURNING FOE IN SIGHT EXPLODES. THE LONGER THEY HAVE BURNED, THE BIGGER THE BLAST. FIRE WALL RANKS ADD 8% EACH.",
        Skill::Meteor => "A SHADOW FALLS ON THE TARGET, THEN A METEOR CRASHES DOWN AND LEAVES THE GROUND BURNING. METEOR RANKS ADD 6% TO FIREBALL.",
        Skill::Mastery => "PASSIVE. ALL YOUR FIRE HITS HARDER AND BURNS LONGER. EACH RANK ALSO LENGTHENS BLAZE.",
        Skill::Hydra => "SUMMONS A FIRE HYDRA THAT SPITS FIREBALLS AT NEARBY FOES.",
        Skill::Phoenix => "FIERY WINGS: YOU MOVE FASTER AND YOUR SKILLS COST NO MANA. WHEN IT ENDS, YOU EXPLODE IN FLAME.",
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

/// Expanding ring left by Fire Nova (visual only; damage is instant).
pub struct Nova {
    pub x: f32,
    pub y: f32,
    pub r: f32,
    pub t: f32,
}

pub const NOVA_TIME: f32 = 0.4;

/// A line of flames from Fire Wall: tile-centred segments that burn whatever stands on them.
pub struct FireWallFx {
    pub segs: Vec<(f32, f32)>,
    pub t: f32,
    pub life: f32,
    pub dps: f32,
    pub tick: f32,
}

/// A meteor on its way down: the shadow grows until it lands.
pub struct MeteorFx {
    pub x: f32,
    pub y: f32,
    pub t: f32,
    pub r: f32,
    pub dmg: f32,
}

/// A fire hydra: a burning turret that spits fireballs.
pub struct HydraFx {
    pub x: f32,
    pub y: f32,
    pub t: f32,
    pub shot: f32,
    pub dmg: f32,
}

/// A burning patch of ground left by Blaze.
pub struct FirePatch {
    pub x: f32,
    pub y: f32,
    pub t: f32,
    pub dps: f32,
}

/// The skill tree screen.
pub struct TreeUi {
    pub sel: usize,
    /// Clickable rectangles from the last draw: (x, y, w, h, action).
    pub rects: Vec<(i32, i32, i32, i32, TreeAct)>,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TreeAct {
    Select(usize),
    Learn,
    Primary,
    Secondary,
    Close,
}

impl Game {
    // ------------------------------------------------------------------ casting

    /// Casts a skill toward (tx, ty). `held` is true while the button stays down (Inferno).
    /// Returns false when nothing was cast.
    pub(crate) fn cast_skill(&mut self, s: Skill, tx: f32, ty: f32) -> bool {
        let r = self.p.skills.rank(s);
        if r == 0 || def(s).passive {
            return false;
        }
        if self.p.skills.cooldown[s as usize] > 0.0 {
            if self.p.cast_cd <= 0.0 {
                self.say(format!("{} IS NOT READY ({:.0}S)", def(s).name, self.p.skills.cooldown[s as usize].ceil()));
                self.p.cast_cd = 0.4;
            }
            return true;
        }
        // Ash Phoenix: everything is free while it lasts (mana is restored after the cast).
        let phoenix = self.p.phoenix_t > 0.0;
        let mana_before = self.p.mana;
        if phoenix {
            self.p.mana = self.p.max_mana.max(mana_cost(s, r));
        }
        // Out of mana: the free Ember Bolt keeps you fighting.
        if self.p.mana < mana_cost(s, r) {
            if self.p.cast_cd <= 0.0 {
                self.cast_fireball(tx, ty, true);
            }
            return true;
        }
        match s {
            Skill::Fireball => {
                if self.p.cast_cd <= 0.0 {
                    self.cast_fireball(tx, ty, false);
                }
            }
            Skill::Inferno => self.channel_inferno(tx, ty, r),
            Skill::FireNova if self.p.cast_cd <= 0.0 => self.fire_nova(r),
            Skill::FireWall if self.p.cast_cd <= 0.0 => self.fire_wall(tx, ty, r),
            Skill::Blaze if self.p.cast_cd <= 0.0 => self.blaze(r),
            Skill::Combust if self.p.cast_cd <= 0.0 => self.combust(r),
            Skill::Meteor if self.p.cast_cd <= 0.0 => self.meteor(tx, ty, r),
            Skill::Hydra if self.p.cast_cd <= 0.0 => self.hydra(tx, ty, r),
            Skill::Phoenix if self.p.cast_cd <= 0.0 => self.phoenix(r),
            _ => {}
        }
        if phoenix {
            self.p.mana = mana_before;
        }
        true
    }

    /// Damage multiplier for fire skills: levels, seals and Fire Mastery.
    pub(crate) fn fire_power(&self) -> f32 {
        self.p.power * self.p.skills.fire_mult()
    }

    fn meteor(&mut self, tx: f32, ty: f32, r: u8) {
        let (px, py) = (self.p.x, self.p.y);
        let (mut cx, mut cy) = (tx, ty);
        let (dx, dy) = (cx - px, cy - py);
        let l = (dx * dx + dy * dy).sqrt();
        if l > 9.0 {
            cx = px + dx / l * 9.0;
            cy = py + dy / l * 9.0;
        }
        if !self.d.walkable(cx.floor() as i32, cy.floor() as i32) || !self.d.los(px, py, cx, cy) {
            self.say("CAN'T CALL A METEOR THERE".into());
            self.p.cast_cd = 0.3;
            return;
        }
        self.p.mana -= meteor_mana(r);
        self.cast_pose(0.5);
        self.meteors.push(MeteorFx { x: cx, y: cy, t: 0.0, r: meteor_radius(r), dmg: meteor_dmg(r) * self.fire_power() });
        self.sfx.push(Sfx::Cast);
    }

    fn hydra(&mut self, tx: f32, ty: f32, r: u8) {
        let (px, py) = (self.p.x, self.p.y);
        let (dx, dy) = (tx - px, ty - py);
        let l = (dx * dx + dy * dy).sqrt().max(0.001);
        // A couple of tiles toward the target, on open floor.
        let (mut hx, mut hy) = (px + dx / l * 1.6, py + dy / l * 1.6);
        if self.d.blocked(hx, hy, 0.35) {
            hx = px;
            hy = py;
        }
        self.p.mana -= hydra_mana(r);
        self.cast_pose(0.5);
        self.p.skills.cooldown[Skill::Hydra as usize] = HYDRA_CD;
        // One hydra at a time: a new one replaces the old.
        self.hydras.clear();
        self.hydras.push(HydraFx { x: hx, y: hy, t: 0.0, shot: 0.3, dmg: hydra_dmg(r) * self.fire_power() });
        self.sfx.push(Sfx::Boom);
        for _ in 0..24 {
            self.spray_at(hx, hy, PKind::Fire, 6.0);
        }
    }

    fn phoenix(&mut self, r: u8) {
        self.p.mana -= phoenix_mana(r);
        self.cast_pose(0.5);
        self.p.skills.cooldown[Skill::Phoenix as usize] = PHOENIX_CD;
        self.p.phoenix_t = phoenix_time(r);
        self.p.phoenix_rank = r;
        self.sfx.push(Sfx::Descend);
        self.say("ASH PHOENIX".into());
        self.lights.push(Light { x: self.p.x, y: self.p.y, r: 220.0, s: 1.2, life: 0.6, max: 0.6 });
        for _ in 0..40 {
            self.spray_at(self.p.x, self.p.y, PKind::Fire, 24.0);
        }
    }

    /// Meteors landing, hydras shooting, Ash Phoenix ticking down (and its final burst).
    pub(crate) fn update_big_fire(&mut self) {
        for c in self.p.skills.cooldown.iter_mut() {
            *c = (*c - DT).max(0.0);
        }
        // Meteors.
        let mut landed = vec![];
        for m in self.meteors.iter_mut() {
            m.t += DT;
            if m.t >= METEOR_DELAY {
                landed.push((m.x, m.y, m.r, m.dmg));
            }
        }
        self.meteors.retain(|m| m.t < METEOR_DELAY);
        for (x, y, r, dmg) in landed {
            self.blast(x, y, r, dmg, 3.0);
            self.shake = self.shake.max(0.8);
            // Burning ground where it struck.
            let dps = dmg * 0.25;
            for k in 0..7 {
                let a = k as f32 / 7.0 * std::f32::consts::TAU;
                let rr = if k == 0 { 0.0 } else { r * 0.55 };
                let (px, py) = (x + a.cos() * rr, y + a.sin() * rr);
                if self.d.walkable(px.floor() as i32, py.floor() as i32) {
                    self.patches.push(FirePatch { x: px, y: py, t: -1.0, dps });
                }
            }
        }
        // Hydras.
        let mut shots = vec![];
        for h in self.hydras.iter_mut() {
            h.t += DT;
            h.shot -= DT;
            if h.shot <= 0.0 {
                h.shot = 0.8;
                shots.push((h.x, h.y, h.dmg));
            }
        }
        self.hydras.retain(|h| h.t < HYDRA_TIME);
        for (hx, hy, dmg) in shots {
            let target = self
                .mobs
                .iter()
                .filter(|m| m.alive() && (m.x - hx).powi(2) + (m.y - hy).powi(2) < 64.0 && self.d.los(hx, hy, m.x, m.y))
                .min_by(|a, b| ((a.x - hx).powi(2) + (a.y - hy).powi(2)).partial_cmp(&((b.x - hx).powi(2) + (b.y - hy).powi(2))).unwrap())
                .map(|m| (m.x, m.y));
            if let Some((mx, my)) = target {
                let (dx, dy) = (mx - hx, my - hy);
                let l = (dx * dx + dy * dy).sqrt().max(0.01);
                self.balls.push(crate::game::Fireball { x: hx, y: hy, vx: dx / l * 9.0, vy: dy / l * 9.0, life: 1.0, dmg, ember: false });
                self.sfx.push(Sfx::Cast);
            }
        }
        // Ash Phoenix.
        if self.p.phoenix_t > 0.0 {
            self.p.phoenix_t -= DT;
            if self.tick % 2 == 0 {
                let (a, b) = (self.rng.f() - 0.5, self.rng.f());
                self.parts.push(Particle { x: self.p.x + a * 0.6, y: self.p.y + 0.1, z: 20.0 + b * 20.0, vx: 0.0, vy: 0.0, vz: 25.0, life: 0.6, max: 0.6, kind: PKind::Fire });
            }
            if self.p.phoenix_t <= 0.0 {
                self.p.phoenix_t = 0.0;
                let dmg = phoenix_burst(self.p.phoenix_rank) * self.fire_power();
                let (x, y) = (self.p.x, self.p.y);
                self.novas.push(Nova { x, y, r: PHOENIX_RADIUS, t: 0.0 });
                self.blast(x, y, PHOENIX_RADIUS, dmg, 2.5);
                self.shake = self.shake.max(0.9);
            }
        }
    }

    /// A big fire explosion: damages and knocks back everything in radius `r`.
    fn blast(&mut self, x: f32, y: f32, r: f32, dmg: f32, burn: f32) {
        self.sfx.push(Sfx::Boom);
        self.lights.push(Light { x, y, r: 240.0, s: 1.5, life: 0.5, max: 0.5 });
        self.decals.push(crate::game::Decal { x, y, r: r * 0.7, col: rgb(0x100804), a: 0.6 });
        for _ in 0..50 {
            let a = self.rng.f() * std::f32::consts::TAU;
            let sp = self.rng.rf(1.0, r * 2.5);
            let (vz, life) = (self.rng.rf(20.0, 90.0), self.rng.rf(0.3, 0.8));
            self.parts.push(Particle { x, y, z: 10.0, vx: a.cos() * sp, vy: a.sin() * sp, vz, life, max: life, kind: PKind::Fire });
        }
        let hits: Vec<usize> = (0..self.mobs.len())
            .filter(|&i| {
                let m = &self.mobs[i];
                m.alive() && (m.x - x).powi(2) + (m.y - y).powi(2) < (r + m.r).powi(2)
            })
            .collect();
        for i in hits {
            self.hit_mob(i, dmg, burn, 0.3, Some((x, y, 0.8)), true);
        }
    }

    fn cast_pose(&mut self, t: f32) {
        self.p.cast_cd = t;
        self.p.cast_t = t;
        self.p.cast_len = t;
    }

    fn fire_wall(&mut self, tx: f32, ty: f32, r: u8) {
        let (px, py) = (self.p.x, self.p.y);
        // Not further than 7 tiles, and not through walls.
        let (mut cx, mut cy) = (tx, ty);
        let (dx, dy) = (cx - px, cy - py);
        let l = (dx * dx + dy * dy).sqrt().max(0.001);
        if l > 7.0 {
            cx = px + dx / l * 7.0;
            cy = py + dy / l * 7.0;
        }
        if !self.d.los(px, py, cx, cy) || !self.d.walkable(cx.floor() as i32, cy.floor() as i32) {
            self.say("NO ROOM FOR A WALL THERE".into());
            self.p.cast_cd = 0.3;
            return;
        }
        self.p.mana -= wall_mana(r);
        self.cast_pose(0.45);
        // Perpendicular to the cast, one segment every half tile on walkable ground.
        let (ux, uy) = (dx / l, dy / l);
        let (nx, ny) = (-uy, ux);
        let half = wall_len(r) * 0.5;
        let mut segs = vec![];
        let mut k = -half;
        while k <= half + 0.01 {
            let (sx, sy) = (cx + nx * k, cy + ny * k);
            if self.d.walkable(sx.floor() as i32, sy.floor() as i32) {
                segs.push((sx, sy));
            }
            k += 0.5;
        }
        self.fire_walls.push(FireWallFx { segs, t: 0.0, life: wall_time(r), dps: wall_dps(r) * self.fire_power(), tick: 0.0 });
        self.lights.push(Light { x: cx, y: cy, r: 180.0, s: 1.0, life: 0.4, max: 0.4 });
        self.sfx.push(Sfx::Boom);
    }

    fn blaze(&mut self, r: u8) {
        self.p.mana -= blaze_mana(r);
        self.cast_pose(0.3);
        self.p.blaze_t = blaze_time_with(r, self.p.skills.rank(Skill::Mastery));
        self.p.blaze_from = (self.p.x, self.p.y);
        self.sfx.push(Sfx::Cast);
        self.say("BLAZE".into());
        for _ in 0..20 {
            self.spray_at(self.p.x, self.p.y, PKind::Fire, 4.0);
        }
    }

    fn combust(&mut self, r: u8) {
        let (px, py) = (self.p.x, self.p.y);
        let burning: Vec<usize> = (0..self.mobs.len())
            .filter(|&i| {
                let m = &self.mobs[i];
                m.alive() && m.burn > 0.0 && (m.x - px).powi(2) + (m.y - py).powi(2) < COMBUST_RANGE * COMBUST_RANGE && self.d.los(px, py, m.x, m.y)
            })
            .collect();
        if burning.is_empty() {
            self.say("NOTHING IS BURNING".into());
            self.p.cast_cd = 0.3;
            return;
        }
        self.p.mana -= combust_mana(r);
        self.cast_pose(0.4);
        self.sfx.push(Sfx::Boom);
        self.shake = self.shake.max(0.5);
        let base = combust_dmg(r, self.p.skills.rank(Skill::FireWall)) * self.fire_power();
        for i in burning {
            let (mx, my, burned) = (self.mobs[i].x, self.mobs[i].y, self.mobs[i].burned);
            self.lights.push(Light { x: mx, y: my, r: 120.0, s: 1.2, life: 0.3, max: 0.3 });
            for _ in 0..14 {
                self.spray_at(mx, my, PKind::Fire, 16.0);
            }
            // The fire is spent in the blast.
            self.mobs[i].burn = 0.0;
            self.mobs[i].burned = 0.0;
            self.hit_mob(i, base * combust_mult(burned), 0.0, 0.2, Some((mx, my - 0.01, 0.0)), true);
        }
    }

    /// Fire Wall flames and Blaze patches: burn what stands in them; Blaze lays new patches.
    pub(crate) fn update_fire_ground(&mut self) {
        // Blaze trail.
        if self.p.blaze_t > 0.0 {
            self.p.blaze_t = (self.p.blaze_t - DT).max(0.0);
            let (fx, fy) = self.p.blaze_from;
            if (self.p.x - fx).powi(2) + (self.p.y - fy).powi(2) > 0.45 * 0.45 {
                let dps = blaze_dps(self.p.skills.rank(Skill::Blaze)) * self.fire_power();
                self.patches.push(FirePatch { x: fx, y: fy, t: 0.0, dps });
                self.p.blaze_from = (self.p.x, self.p.y);
            }
        }
        // Flames (particles) and damage ticks every 0.2 s.
        let mut hits: Vec<(f32, f32, f32, f32)> = vec![]; // (x, y, radius, damage)
        for w in self.fire_walls.iter_mut() {
            w.t += DT;
            w.tick += DT;
            if w.tick >= 0.2 {
                w.tick -= 0.2;
                for &(x, y) in &w.segs {
                    hits.push((x, y, 0.5, w.dps * 0.2));
                }
            }
        }
        self.fire_walls.retain(|w| w.t < w.life && !w.segs.is_empty());
        let tick = (self.tick % 12) == 0;
        for p in self.patches.iter_mut() {
            p.t += DT;
            if tick {
                hits.push((p.x, p.y, 0.45, p.dps * 0.2));
            }
        }
        self.patches.retain(|p| p.t < PATCH_TIME);
        if self.patches.len() > 200 {
            let n = self.patches.len() - 200;
            self.patches.drain(0..n);
        }
        let mut sparks: Vec<(f32, f32, f32)> = vec![];
        for w in &self.fire_walls {
            let fade = ((w.life - w.t) / 0.6).min(1.0);
            for &(x, y) in &w.segs {
                if self.rng.f() < 0.5 * fade {
                    sparks.push((x, y, 30.0));
                }
            }
        }
        for p in &self.patches {
            if self.rng.f() < 0.25 * (1.0 - p.t / PATCH_TIME) {
                sparks.push((p.x, p.y, 14.0));
            }
        }
        for (x, y, h) in sparks {
            let (a, b, c) = (self.rng.f() - 0.5, self.rng.f() - 0.5, self.rng.f());
            let life = 0.3 + c * 0.35;
            self.parts.push(Particle { x: x + a * 0.4, y: y + b * 0.4, z: 2.0 + c * 6.0, vx: 0.0, vy: 0.0, vz: h + c * h, life, max: life, kind: PKind::Fire });
        }
        if hits.is_empty() {
            return;
        }
        for i in 0..self.mobs.len() {
            if !self.mobs[i].alive() {
                continue;
            }
            let (mx, my, mr) = (self.mobs[i].x, self.mobs[i].y, self.mobs[i].r);
            // One hit per source type per tick: the strongest that touches this monster.
            let best = hits
                .iter()
                .filter(|(x, y, r, _)| (mx - x).powi(2) + (my - y).powi(2) < (r + mr).powi(2))
                .map(|h| h.3)
                .fold(0.0f32, f32::max);
            if best > 0.0 && self.mobs[i].alive() {
                self.hit_mob(i, best, 1.2, 0.0, None, false);
            }
        }
    }

    fn channel_inferno(&mut self, tx: f32, ty: f32, r: u8) {
        let (px, py) = (self.p.x, self.p.y);
        let (dx, dy) = (tx - px, ty - py);
        let l = (dx * dx + dy * dy).sqrt().max(0.001);
        let (ux, uy) = (dx / l, dy / l);
        self.p.mana -= inferno_mana(r) * DT;
        // Hold the staff-forward pose while channelling.
        self.p.cast_t = CAST_TIME * 0.5;
        self.p.cast_len = CAST_TIME;
        self.p.inferno = 0.1;
        self.p.inferno_t += DT;
        let range = inferno_range(r);
        // Flames.
        for _ in 0..5 {
            let (a, b, c) = (self.rng.f() - 0.5, self.rng.f() - 0.5, self.rng.f());
            let sp = range / 0.32 * (0.7 + c * 0.4);
            let (sx, sy) = (ux * sp + (-uy) * a * sp * 0.45, uy * sp + ux * a * sp * 0.45);
            let life = 0.3 + c * 0.08;
            self.parts.push(Particle { x: px + ux * 0.5, y: py + uy * 0.5, z: 22.0 + b * 6.0, vx: sx, vy: sy, vz: -8.0, life, max: life, kind: PKind::Fire });
        }
        self.lights.push(Light { x: px + ux * range * 0.55, y: py + uy * range * 0.55, r: 120.0, s: 0.8, life: 0.05, max: 0.05 });
        if (self.p.inferno_t / 0.25) as i32 != ((self.p.inferno_t - DT) / 0.25) as i32 {
            self.sfx.push(Sfx::Cast);
        }
        // Damage in ticks of 0.1 s.
        if (self.p.inferno_t / 0.1) as i32 == ((self.p.inferno_t - DT) / 0.1) as i32 {
            return;
        }
        let dmg = inferno_dps(r) * self.fire_power() * 0.1;
        let cos_half = (26.0f32).to_radians().cos();
        let targets: Vec<usize> = (0..self.mobs.len())
            .filter(|&i| {
                let m = &self.mobs[i];
                if !m.alive() {
                    return false;
                }
                let (mx, my) = (m.x - px, m.y - py);
                let d = (mx * mx + my * my).sqrt();
                d < range + m.r && d > 0.01 && (mx * ux + my * uy) / d > cos_half && self.d.los(px, py, m.x, m.y)
            })
            .collect();
        let mut any = false;
        for i in targets {
            any = true;
            self.hit_mob(i, dmg, 1.0, 0.0, None, false);
        }
        if any && (self.p.inferno_t / 0.4) as i32 != ((self.p.inferno_t - 0.1) / 0.4) as i32 {
            self.sfx.push(Sfx::Hit);
        }
    }

    fn fire_nova(&mut self, r: u8) {
        let (px, py) = (self.p.x, self.p.y);
        self.p.mana -= nova_mana(r);
        self.p.cast_cd = 0.5;
        self.p.cast_t = 0.5;
        self.p.cast_len = 0.5;
        let radius = nova_radius(r);
        self.novas.push(Nova { x: px, y: py, r: radius, t: 0.0 });
        self.lights.push(Light { x: px, y: py, r: 260.0, s: 1.3, life: 0.45, max: 0.45 });
        self.shake = self.shake.max(0.4);
        self.sfx.push(Sfx::Boom);
        for k in 0..60 {
            let a = k as f32 / 60.0 * std::f32::consts::TAU + self.rng.f() * 0.1;
            let sp = radius / NOVA_TIME * self.rng.rf(0.85, 1.05);
            let life = NOVA_TIME * self.rng.rf(0.9, 1.15);
            self.parts.push(Particle { x: px, y: py, z: 8.0 + self.rng.f() * 10.0, vx: a.cos() * sp, vy: a.sin() * sp, vz: 6.0, life, max: life, kind: PKind::Fire });
        }
        let dmg = nova_dmg(r) * self.fire_power();
        let hits: Vec<usize> = (0..self.mobs.len())
            .filter(|&i| {
                let m = &self.mobs[i];
                m.alive() && (m.x - px).powi(2) + (m.y - py).powi(2) < (radius + m.r).powi(2) && self.d.los(px, py, m.x, m.y)
            })
            .collect();
        for i in hits {
            self.hit_mob(i, dmg, 2.0, 0.3, Some((px, py, 1.2)), true);
        }
    }

    /// Damages one monster: burning (seconds), stagger, optional knockback (from x, y, distance).
    pub(crate) fn hit_mob(&mut self, i: usize, dmg: f32, burn: f32, stun: f32, knock: Option<(f32, f32, f32)>, show: bool) {
        let burn = burn * self.p.skills.burn_mult();
        let m = &mut self.mobs[i];
        let (mx, my, boss, r) = (m.x, m.y, m.boss, m.r);
        m.hp -= dmg;
        m.flash = m.flash.max(if show { 0.12 } else { 0.04 });
        m.burn = m.burn.max(burn);
        m.stun = m.stun.max(if boss { stun * 0.2 } else { stun });
        if m.state == MobState::Idle {
            m.state = MobState::Chase;
        }
        if let (MobState::Attack(_), false, true) = (m.state, boss, stun > 0.0) {
            m.state = MobState::Chase;
            m.cd = m.cd.max(0.3);
        }
        if let (Some((kx0, ky0, dist)), false) = (knock, boss) {
            let (kx, ky) = (mx - kx0, my - ky0);
            let kl = (kx * kx + ky * ky).sqrt().max(0.01);
            let (mut nx, mut ny) = (mx, my);
            move_circle(&self.d, &mut nx, &mut ny, kx / kl * dist, ky / kl * dist, r);
            m.x = nx;
            m.y = ny;
        }
        self.focus = Some(i);
        self.focus_t = 3.0;
        self.stats.hits += 1;
        if show {
            self.floater(mx, my, format!("{}", dmg.round() as i32), rgb(0xffb040));
        }
        if self.mobs[i].hp <= 0.0 && self.mobs[i].alive() {
            self.kill(i);
        }
    }

    pub(crate) fn update_novas(&mut self) {
        for n in self.novas.iter_mut() {
            n.t += DT;
        }
        self.novas.retain(|n| n.t < NOVA_TIME + 0.15);
    }

    // ------------------------------------------------------------------ slots

    /// Number keys pick the secondary skill; R1 cycles it.
    pub(crate) fn update_slots(&mut self, inp: &Input) {
        let actives = self.p.skills.actives();
        let pick = if let Some(k) = inp.slot {
            actives.get(k as usize).copied()
        } else if inp.cycle && !actives.is_empty() {
            let i = actives.iter().position(|s| *s == self.p.skills.secondary).map_or(0, |i| (i + 1) % actives.len());
            Some(actives[i])
        } else {
            None
        };
        if let Some(s) = pick {
            if s != self.p.skills.secondary {
                self.p.skills.secondary = s;
                self.say(format!("SECONDARY SKILL: {}", def(s).name));
            }
        }
    }

    // ------------------------------------------------------------------ the tree screen

    pub(crate) fn open_tree(&mut self) {
        let sel = ALL.iter().position(|s| *s == self.p.skills.secondary).unwrap_or(0);
        self.tree = Some(TreeUi { sel, rects: vec![] });
        self.dialog = None;
    }

    /// Skill tree controls (the world is paused while it's open).
    pub(crate) fn update_tree(&mut self, inp: &Input, pressed_confirm: bool, click: bool) {
        let prev = self.prev.clone();
        let edge = |now: f32, before: f32, neg: bool| if neg { now < -0.5 && before >= -0.5 } else { now > 0.5 && before <= 0.5 };
        let Some(ui) = self.tree.as_mut() else { return };
        let mut act: Option<TreeAct> = None;
        // Arrow / stick navigation across the grid.
        let (r0, c0) = def(ALL[ui.sel]).cell;
        let mut target = None;
        if edge(inp.move_x, prev.move_x, false) {
            target = Some((r0, c0 + 1));
        } else if edge(inp.move_x, prev.move_x, true) {
            target = Some((r0, c0 - 1));
        } else if edge(inp.move_y, prev.move_y, false) {
            target = Some((r0 + 1, c0));
        } else if edge(inp.move_y, prev.move_y, true) {
            target = Some((r0 - 1, c0));
        }
        if let Some((tr, tc)) = target {
            // Nearest skill in that direction.
            let best = ALL
                .iter()
                .enumerate()
                .filter(|(_, s)| {
                    let (r, c) = def(**s).cell;
                    (tr != r0 && r == tr) || (tc != c0 && r == r0 && (c - c0).signum() == (tc - c0).signum())
                })
                .min_by_key(|(_, s)| {
                    let (r, c) = def(**s).cell;
                    (r - tr).abs() * 10 + (c - tc).abs()
                })
                .map(|(i, _)| i);
            if let Some(i) = best {
                ui.sel = i;
            }
        }
        if let Some((mx, my)) = inp.mouse {
            let hit = ui.rects.iter().find(|&&(x, y, w, h, _)| mx >= x && mx < x + w && my >= y && my < y + h).map(|r| r.4);
            if let (true, Some(a)) = (click, hit) {
                act = Some(a);
            }
        }
        if pressed_confirm {
            act = Some(TreeAct::Learn);
        }
        if inp.potion_hp {
            act = Some(TreeAct::Primary);
        }
        if inp.potion_mp {
            act = Some(TreeAct::Secondary);
        }
        if inp.skills || inp.run_toggle || inp.cancel {
            act = Some(TreeAct::Close);
        }
        let s = ALL[self.tree.as_ref().unwrap().sel];
        match act {
            Some(TreeAct::Select(i)) => self.tree.as_mut().unwrap().sel = i,
            Some(TreeAct::Learn) => {
                if self.p.skills.learn(s, self.p.clvl) {
                    self.sfx.push(Sfx::Pickup);
                    // A freshly learned active skill goes into the secondary slot.
                    if self.p.skills.rank(s) == 1 && !def(s).passive {
                        self.p.skills.secondary = s;
                    }
                    self.save_due = true;
                } else if let Some(why) = self.p.skills.blocker(s, self.p.clvl) {
                    self.say(why);
                }
            }
            Some(TreeAct::Primary) | Some(TreeAct::Secondary) => {
                if def(s).passive {
                    self.say("PASSIVE SKILLS ARE ALWAYS ON".into());
                } else if self.p.skills.rank(s) == 0 {
                    self.say("LEARN IT FIRST".into());
                } else if act == Some(TreeAct::Primary) {
                    self.p.skills.primary = s;
                    self.sfx.push(Sfx::Pickup);
                } else {
                    self.p.skills.secondary = s;
                    self.sfx.push(Sfx::Pickup);
                }
            }
            Some(TreeAct::Close) => self.tree = None,
            None => {}
        }
    }

    pub(crate) fn draw_tree(&mut self, scr: &mut Screen) {
        let Some(ui) = self.tree.as_ref() else { return };
        let sel = ui.sel;
        let top = self.view_h - HUD_H;
        let (pw, ph) = (440, (top - 24).min(300));
        let (x0, y0) = (scr.w / 2 - pw / 2, (top - ph) / 2);
        scr.blend(0, 0, scr.w, top, BLACK, 0.45);
        scr.blend(x0, y0, pw, ph, rgb(0x0c0a08), 0.93);
        for (x, y, w, h) in [(x0, y0, pw, 1), (x0, y0 + ph - 1, pw, 1), (x0, y0, 1, ph), (x0 + pw - 1, y0, 1, ph)] {
            scr.fill(x, y, w, h, rgb(0x8a7050));
        }
        let sk = &self.p.skills;
        scr.text("FIRE SKILLS", x0 + 10, y0 + 8, rgb(0xffd080), Align::Left, 1);
        let pts_col = if sk.points > 0 && (self.tick / 20) % 2 == 0 { rgb(0xffe080) } else { rgb(0xd8b878) };
        scr.text(&format!("SKILL POINTS: {}", sk.points), x0 + pw - 10, y0 + 8, pts_col, Align::Right, 1);
        let mut rects = vec![];
        // Tier rows.
        let (cw, ch) = (128, 34);
        for tier in 0..TIER_LEVELS.len() as i32 {
            let ty = y0 + 24 + tier * (ch + 8);
            let lvl = TIER_LEVELS[tier as usize];
            let col = if self.p.clvl >= lvl { rgb(0x9a8a78) } else { rgb(0x5a4a40) };
            scr.text(&format!("LV{lvl}"), x0 + 8, ty + 14, col, Align::Left, 1);
        }
        for (i, s) in ALL.iter().enumerate() {
            let d = def(*s);
            let (row, colm) = d.cell;
            let (cx, cy) = (x0 + 40 + colm * (cw + 6), y0 + 24 + row * (ch + 8));
            let r = sk.rank(*s);
            let locked = self.p.clvl < d.level || d.prereq.map_or(false, |p| sk.rank(p) == 0);
            let bg = if i == sel { rgb(0x4a3010) } else if r > 0 { rgb(0x2a2016) } else { rgb(0x161210) };
            scr.fill(cx, cy, cw, ch, bg);
            scr.fill(cx, cy, cw, 1, if i == sel { rgb(0xffcf70) } else { rgb(0x4a3e30) });
            scr.fill(cx, cy + ch - 1, cw, 1, if i == sel { rgb(0xffcf70) } else { rgb(0x4a3e30) });
            let icon = crate::sprites::skill_icon(*s);
            scr.blit(&icon, cx + 5, cy + 5, Fx { tint: BLACK, tint_a: if locked { 0.6 } else { 0.0 }, ..Fx::default() });
            let ncol = if locked { rgb(0x6a5a4a) } else if r > 0 { rgb(0xffe0a0) } else { rgb(0xb0a090) };
            scr.text(d.name, cx + 36, cy + 6, ncol, Align::Left, 1);

            let cd = sk.cooldown[i];
            let lr = sk.learned(*s);
            let plus = if r > lr { format!(" +{}", r - lr) } else { String::new() };
            let sub = if locked {
                format!("LV{}", d.level)
            } else if cd > 0.0 {
                format!("{lr}/{MAX_RANK}{plus}  {:.0}S", cd.ceil())
            } else {
                format!("{lr}/{MAX_RANK}{plus}")
            };
            scr.text(&sub, cx + 36, cy + 19, if locked { rgb(0x6a5a4a) } else { rgb(0x9a8a78) }, Align::Left, 1);
            let mut tag = String::new();
            if !d.passive && r > 0 && sk.primary == *s {
                tag.push('L');
            }
            if !d.passive && r > 0 && sk.secondary == *s {
                tag.push('R');
            }
            if d.passive && r > 0 {
                tag.push('P');
            }
            if !tag.is_empty() {
                scr.text(&tag, cx + cw - 6, cy + 19, rgb(0x80c0ff), Align::Right, 1);
            }
            rects.push((cx, cy, cw, ch, TreeAct::Select(i)));
        }
        // Prerequisite lines (each skill sits right below the one it needs).
        for s in ALL {
            if let Some(p) = def(s).prereq {
                let (a, b) = (def(p).cell, def(s).cell);
                let lx = x0 + 40 + a.1 * (cw + 6) + cw / 2;
                let lit = sk.rank(p) > 0;
                scr.fill(lx, y0 + 24 + a.0 * (ch + 8) + ch, 2, (b.0 - a.0) * (ch + 8) - ch, if lit { rgb(0xd88a30) } else { rgb(0x5a4030) });
            }
        }
        // Details of the selected skill.
        let s = ALL[sel];
        let dy = y0 + 24 + TIER_LEVELS.len() as i32 * (ch + 8) + 4;
        scr.fill(x0 + 8, dy - 4, pw - 16, 1, rgb(0x4a3e30));
        let mut ly = dy;
        for line in describe(s, sk.rank(s), self.p.power, sk) {
            for l in crate::story::wrap(&line, 68) {
                scr.text(&l, x0 + 12, ly, rgb(0xd8ccb8), Align::Left, 1);
                ly += 10;
            }
        }
        if let Some(why) = sk.blocker(s, self.p.clvl) {
            if why != "NO SKILL POINTS" || sk.rank(s) == 0 {
                scr.text(&why, x0 + 12, ly + 2, rgb(0xc06040), Align::Left, 1);
            }
        }
        // Buttons.
        let by = y0 + ph - 22;
        let mut bx = x0 + 10;
        let can = sk.blocker(s, self.p.clvl).is_none();
        let buttons: [(&str, TreeAct, bool); 4] = [
            ("LEARN (ENTER/A)", TreeAct::Learn, can),
            ("PRIMARY (Q/L1)", TreeAct::Primary, !def(s).passive && sk.rank(s) > 0),
            ("SECONDARY (E/Y)", TreeAct::Secondary, !def(s).passive && sk.rank(s) > 0),
            ("CLOSE (K)", TreeAct::Close, true),
        ];
        for (label, a, on) in buttons {
            let w = crate::gfx::text_width(label, 1) + 12;
            scr.fill(bx, by, w, 15, if on { rgb(0x5a3a10) } else { rgb(0x221c16) });
            scr.text(label, bx + 6, by + 4, if on { rgb(0xffe0a0) } else { rgb(0x6a5a4a) }, Align::Left, 1);
            rects.push((bx, by, w, 15, a));
            bx += w + 6;
        }
        self.tree.as_mut().unwrap().rects = rects;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn learning_respects_points_level_and_prerequisites() {
        let mut sk = Skills::default();
        assert_eq!(sk.rank(Skill::Fireball), 1);
        assert_eq!(sk.points, 1);
        assert!(sk.blocker(Skill::FireNova, 6).is_some(), "needs Inferno");
        assert!(sk.learn(Skill::Inferno, 1));
        assert!(!sk.learn(Skill::Warmth, 1), "no points left");
        sk.points = 3;
        assert!(!sk.learn(Skill::FireNova, 5), "needs level 6");
        assert!(sk.learn(Skill::FireNova, 6));
        assert_eq!(sk.actives(), vec![Skill::Fireball, Skill::Inferno, Skill::FireNova]);
        let refunded = sk.respec();
        assert_eq!(refunded, 2);
        assert_eq!(sk.points, 4);
        assert_eq!(sk.rank(Skill::Inferno), 0);
        assert_eq!(sk.rank(Skill::Fireball), 1);
    }

    #[test]
    fn step_two_skills_need_their_prerequisites() {
        let mut sk = Skills { points: 10, ..Skills::default() };
        assert!(sk.blocker(Skill::FireWall, 5).is_some(), "needs level 6");
        assert!(sk.learn(Skill::FireWall, 6), "Fireball is known from the start");
        assert!(sk.blocker(Skill::Blaze, 6).is_some(), "needs Warmth");
        assert!(sk.learn(Skill::Warmth, 6) && sk.learn(Skill::Blaze, 6));
        assert!(!sk.learn(Skill::Combust, 11), "needs level 12");
        assert!(sk.learn(Skill::Combust, 12));
        assert!(combust_dmg(1, 5) > combust_dmg(1, 0) * 1.3, "Fire Wall synergy");
        assert!(combust_mult(4.0) > combust_mult(0.5));
    }

    #[test]
    fn mastery_scales_fire_and_ultimates_need_level_18() {
        let mut sk = Skills { points: 20, ..Skills::default() };
        assert_eq!(sk.fire_mult(), 1.0);
        sk.rank[Skill::Mastery as usize] = 5;
        assert!((sk.fire_mult() - 1.4).abs() < 1e-5);
        assert!(sk.burn_mult() > 1.4);
        assert!(blaze_time_with(1, 5) > blaze_time(1) + 2.0);
        sk.rank[Skill::Combust as usize] = 1;
        assert!(sk.blocker(Skill::Hydra, 17).is_some());
        assert!(sk.learn(Skill::Hydra, 18));
        assert!(sk.blocker(Skill::Phoenix, 18).is_some(), "needs Meteor");
        assert!(cooldown_of(Skill::Hydra) > 0.0 && cooldown_of(Skill::Fireball) == 0.0);
        assert!(fireball_synergy(5) > 1.25);
    }

    #[test]
    fn skills_save_and_load() {
        let mut sk = Skills::default();
        sk.points = 5;
        sk.learn(Skill::Inferno, 1);
        sk.learn(Skill::Warmth, 1);
        sk.secondary = Skill::Inferno;
        let back = Skills::load_text(&sk.save_text()).unwrap();
        assert_eq!(back.rank, sk.rank);
        assert_eq!(back.points, 3);
        assert_eq!(back.secondary, Skill::Inferno);
        assert_eq!(back.primary, Skill::Fireball);
    }
}

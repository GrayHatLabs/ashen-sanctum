//! Fire skills: D2-style skill points, ranks, two skill slots, and the skill tree screen.
//! See docs/SKILLS_PLAN.md for the full plan (this is step 1: Fireball, Inferno, Warmth, Fire Nova).
use crate::game::{move_circle, Game, Input, Light, PKind, Particle, Sfx, CAST_TIME, DT, HUD_H};
use crate::gfx::{rgb, Align, Fx, Screen, BLACK};
use crate::mobs::MobState;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Skill {
    Fireball,
    Inferno,
    Warmth,
    FireNova,
}

pub const ALL: [Skill; 4] = [Skill::Fireball, Skill::Inferno, Skill::Warmth, Skill::FireNova];
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
}

impl Default for Skills {
    fn default() -> Self {
        // You start knowing Fireball, with one point to spend.
        let mut rank = [0; ALL.len()];
        rank[Skill::Fireball as usize] = 1;
        Skills { rank, points: 1, primary: Skill::Fireball, secondary: Skill::Fireball }
    }
}

impl Skills {
    pub fn rank(&self, s: Skill) -> u8 {
        self.rank[s as usize]
    }

    /// Why you can't put a point into this skill right now (None = you can).
    pub fn blocker(&self, s: Skill, clvl: u32) -> Option<String> {
        let d = def(s);
        if self.rank(s) >= MAX_RANK {
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
        *self = Skills { points: self.points + spent, ..Skills::default() };
        spent
    }

    /// Mana regeneration multiplier from Warmth.
    pub fn regen_mult(&self) -> f32 {
        match self.rank(Skill::Warmth) {
            0 => 1.0,
            r => 1.3 + 0.12 * (r - 1) as f32,
        }
    }

    pub fn save_text(&self) -> String {
        let ranks: Vec<String> = ALL.iter().map(|s| format!("{}:{}", def(*s).key, self.rank(*s))).collect();
        format!("skills={}\npoints={}\nprimary={}\nsecondary={}\n", ranks.join(","), self.points, def(self.primary).key, def(self.secondary).key)
    }

    pub fn load_text(text: &str) -> Option<Skills> {
        let get = |k: &str| text.lines().find_map(|l| l.strip_prefix(k).and_then(|r| r.strip_prefix('='))).map(str::trim);
        let by_key = |k: &str| ALL.iter().copied().find(|s| def(*s).key == k);
        let mut sk = Skills { rank: [0; ALL.len()], points: get("points")?.parse().ok()?, primary: Skill::Fireball, secondary: Skill::Fireball };
        for part in get("skills")?.split(',') {
            let (k, v) = part.split_once(':')?;
            if let (Some(s), Ok(r)) = (by_key(k), v.parse::<u8>()) {
                sk.rank[s as usize] = r.min(MAX_RANK);
            }
        }
        sk.rank[Skill::Fireball as usize] = sk.rank(Skill::Fireball).max(1);
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

/// Mana cost to start a cast (Inferno: one second of channelling).
pub fn mana_cost(s: Skill, r: u8) -> f32 {
    match s {
        Skill::Fireball => fireball_mana(r),
        Skill::Inferno => inferno_mana(r) * DT,
        Skill::FireNova => nova_mana(r),
        Skill::Warmth => 0.0,
    }
}

/// Description lines for the tree: what it does, and this rank vs the next.
pub fn describe(s: Skill, r: u8, power: f32) -> Vec<String> {
    let at = |r: u8| -> String {
        match s {
            Skill::Fireball => {
                let (a, b) = fireball_dmg(r);
                format!("{}-{} DAMAGE, {:.0} MANA", (a * power) as i32, (b * power) as i32, fireball_mana(r))
            }
            Skill::Inferno => format!("{} DAMAGE/SEC, RANGE {:.1}, {:.0} MANA/SEC", (inferno_dps(r) * power) as i32, inferno_range(r), inferno_mana(r)),
            Skill::Warmth => format!("+{:.0}% MANA REGENERATION", (1.3 + 0.12 * (r.max(1) - 1) as f32 - 1.0) * 100.0),
            Skill::FireNova => format!("{} DAMAGE, RADIUS {:.1}, {:.0} MANA", (nova_dmg(r) * power) as i32, nova_radius(r), nova_mana(r)),
        }
    };
    let what = match s {
        Skill::Fireball => "HURLS AN EXPLODING BALL OF FIRE THAT SPLASHES AND SETS FOES BURNING.",
        Skill::Inferno => "HOLD TO BREATHE A CONE OF FLAME FROM YOUR STAFF. DRAINS MANA WHILE HELD.",
        Skill::Warmth => "PASSIVE. THE FLAME WITHIN RESTORES YOUR MANA FASTER.",
        Skill::FireNova => "A RING OF FIRE BURSTS OUT FROM YOU, BURNING AND HURLING BACK EVERYTHING NEARBY.",
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
            Skill::FireNova => {
                if self.p.cast_cd <= 0.0 {
                    self.fire_nova(r);
                }
            }
            Skill::Warmth => {}
        }
        true
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
        let dmg = inferno_dps(r) * self.p.power * 0.1;
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
        let dmg = nova_dmg(r) * self.p.power;
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
        let (cw, ch) = (128, 38);
        for tier in 0..2 {
            let ty = y0 + 24 + tier * (ch + 8);
            let lvl = [1, 6][tier as usize];
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
            scr.blit(&icon, cx + 7, cy + 7, Fx { tint: BLACK, tint_a: if locked { 0.6 } else { 0.0 }, ..Fx::default() });
            let ncol = if locked { rgb(0x6a5a4a) } else if r > 0 { rgb(0xffe0a0) } else { rgb(0xb0a090) };
            scr.text(d.name, cx + 36, cy + 7, ncol, Align::Left, 1);
            let sub = if locked { format!("LV{}", d.level) } else { format!("{r}/{MAX_RANK}") };
            scr.text(&sub, cx + 36, cy + 20, if locked { rgb(0x6a5a4a) } else { rgb(0x9a8a78) }, Align::Left, 1);
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
                scr.text(&tag, cx + cw - 6, cy + 20, rgb(0x80c0ff), Align::Right, 1);
            }
            rects.push((cx, cy, cw, ch, TreeAct::Select(i)));
        }
        // Prerequisite line (Inferno -> Fire Nova).
        let (a, b) = (def(Skill::Inferno).cell, def(Skill::FireNova).cell);
        let lx = x0 + 40 + a.1 * (cw + 6) + cw / 2;
        scr.fill(lx, y0 + 24 + a.0 * (ch + 8) + ch, 2, (b.0 - a.0) * (ch + 8) - ch, rgb(0x8a6030));
        // Details of the selected skill.
        let s = ALL[sel];
        let dy = y0 + 24 + 2 * (ch + 8) + 4;
        scr.fill(x0 + 8, dy - 4, pw - 16, 1, rgb(0x4a3e30));
        let mut ly = dy;
        for line in describe(s, sk.rank(s), self.p.power) {
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

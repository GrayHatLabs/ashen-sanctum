//! Monsters: definitions, AI, enemy projectiles and boss ground hazards.
use crate::game::{move_circle, Game, PKind, Particle, Sfx, State, DT, PLAYER_R};
use crate::gfx::rgb;
use crate::iso;
use crate::rng::Rng;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Kind {
    Zombie,
    Skeleton,
    Wolf,
    Imp,
    Archer,
    BoneWarden,
    PlagueWarden,
    HexWarden,
    AshKing,
}

pub struct Def {
    /// Art sheet name.
    pub art: &'static str,
    pub label: &'static str,
    pub hp: f32,
    pub speed: f32,
    pub dmg: (f32, f32),
    pub windup: f32,
    pub cooldown: f32,
    pub xp: f32,
    /// Collision radius (tiles).
    pub r: f32,
    /// Melee reach (tiles).
    pub reach: f32,
    pub boss: bool,
    pub ranged: bool,
}

pub fn def(k: Kind) -> Def {
    let d = |art, label, hp, speed, dmg, windup, cooldown, xp| Def {
        art,
        label,
        hp,
        speed,
        dmg,
        windup,
        cooldown,
        xp,
        r: 0.32,
        reach: 0.85,
        boss: false,
        ranged: false,
    };
    match k {
        Kind::Zombie => d("zombie", "ROTTING ZOMBIE", 34.0, 1.25, (5.0, 9.0), 0.55, 1.5, 14.0),
        Kind::Skeleton => d("skeleton", "RISEN SKELETON", 20.0, 2.3, (3.0, 6.0), 0.35, 1.0, 10.0),
        Kind::Wolf => Def { r: 0.3, ..d("wolf", "DIRE WOLF", 16.0, 3.6, (3.0, 5.0), 0.3, 0.9, 9.0) },
        Kind::Imp => Def { r: 0.26, ..d("imp", "ASHEN IMP", 12.0, 2.8, (2.0, 5.0), 0.3, 0.9, 7.0) },
        Kind::Archer => Def { ranged: true, ..d("archer", "SKELETON ARCHER", 16.0, 1.9, (4.0, 7.0), 0.55, 1.7, 13.0) },
        Kind::BoneWarden => Def {
            r: 0.55,
            reach: 1.3,
            boss: true,
            ..d("boss_bone", "THE BONE WARDEN", 320.0, 1.7, (12.0, 18.0), 0.7, 1.6, 300.0)
        },
        Kind::PlagueWarden => Def {
            r: 0.6,
            reach: 1.3,
            boss: true,
            ..d("boss_plague", "THE PLAGUE WARDEN", 420.0, 1.1, (14.0, 22.0), 0.8, 1.8, 380.0)
        },
        Kind::HexWarden => Def {
            r: 0.5,
            boss: true,
            ranged: true,
            ..d("boss_hex", "THE HEX WARDEN", 360.0, 1.8, (8.0, 12.0), 0.6, 2.2, 450.0)
        },
        Kind::AshKing => Def {
            r: 0.6,
            reach: 1.4,
            boss: true,
            ..d("boss_ashking", "THE ASH KING", 700.0, 1.6, (18.0, 26.0), 0.7, 1.6, 1500.0)
        },
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum MobState {
    Idle,
    Chase,
    /// Wind-up before a swing or shot; it lands when the timer runs out.
    Attack(f32),
    Dead(f32),
}

pub struct Mob {
    pub kind: Kind,
    pub x: f32,
    pub y: f32,
    pub hp: f32,
    pub max_hp: f32,
    pub speed: f32,
    pub dmg: (f32, f32),
    pub windup: f32,
    pub cooldown: f32,
    pub cd: f32,
    pub r: f32,
    pub reach: f32,
    pub boss: bool,
    pub ranged: bool,
    pub xp: f32,
    pub state: MobState,
    pub dir: usize,
    pub anim_t: f32,
    pub moving: bool,
    pub flash: f32,
    pub stun: f32,
    pub burn: f32,
    pub path: Vec<(f32, f32)>,
    pub repath: f32,
    pub wander: (f32, f32, f32),
    /// Imps run away from you for this long after a packmate dies.
    pub flee: f32,
    /// Boss ability timers.
    pub special: f32,
    pub special2: f32,
    pub enraged: bool,
    /// Where it lives (overworld monsters return home instead of chasing you forever).
    pub home: (f32, f32),
    pub tier: f32,
}

impl Mob {
    pub fn new(kind: Kind, x: f32, y: f32, tier: f32, rng: &mut Rng) -> Self {
        let d = def(kind);
        let hp = d.hp * tier;
        let k = tier.powf(0.8);
        Mob {
            kind,
            x,
            y,
            hp,
            max_hp: hp,
            speed: d.speed * if d.boss { 1.0 } else { rng.rf(0.9, 1.1) },
            dmg: (d.dmg.0 * k, d.dmg.1 * k),
            windup: d.windup,
            cooldown: d.cooldown,
            cd: 0.0,
            r: d.r,
            reach: d.reach,
            boss: d.boss,
            ranged: d.ranged,
            xp: d.xp * tier,
            state: MobState::Idle,
            dir: rng.range(0, 8) as usize,
            anim_t: rng.f(),
            moving: false,
            flash: 0.0,
            stun: 0.0,
            burn: 0.0,
            path: vec![],
            repath: 0.0,
            wander: (0.0, 0.0, rng.f() * 2.0),
            flee: 0.0,
            special: 4.0,
            special2: 6.0,
            enraged: false,
            home: (x, y),
            tier,
        }
    }

    pub fn alive(&self) -> bool {
        !matches!(self.state, MobState::Dead(_))
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ShotKind {
    Arrow,
    Hex,
    Ash,
}

pub struct Shot {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub life: f32,
    pub dmg: f32,
    pub kind: ShotKind,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum HazardKind {
    /// Plague Warden's lingering poison pool.
    Poison,
    /// Ash King's telegraphed ash nova.
    Nova,
}

pub struct Hazard {
    pub x: f32,
    pub y: f32,
    pub r: f32,
    /// Warning time before it becomes dangerous.
    pub warn: f32,
    /// How long it stays dangerous (0 = a single burst).
    pub live: f32,
    pub dps: f32,
    pub burst: f32,
    pub t: f32,
    pub fired: bool,
    pub kind: HazardKind,
}

impl Game {
    pub(crate) fn update_mobs(&mut self) {
        let (px, py) = (self.p.x, self.p.y);
        let player_alive = !matches!(self.state, State::Dead(_));
        let player_safe = self.in_safe(px, py);
        let safe = self.safe;
        let in_safe = |x: f32, y: f32, pad: f32| safe.map_or(false, |(x0, y0, x1, y1)| x > x0 - pad && x < x1 + pad && y > y0 - pad && y < y1 + pad);
        let overworld = self.level == crate::world::LevelId::Overworld;
        let n = self.mobs.len();
        let mut hits: Vec<f32> = vec![];
        let mut aggro_at: Vec<(f32, f32)> = vec![];
        let mut spawns: Vec<(Kind, f32, f32, f32)> = vec![];
        let mut shots: Vec<(f32, f32, f32, f32, f32, ShotKind)> = vec![];
        let mut hazards: Vec<Hazard> = vec![];
        let mut texts: Vec<(f32, f32, &'static str)> = vec![];
        let summons = self.mobs.iter().filter(|m| m.alive() && !m.boss && m.home.0 < -999.0).count();
        for i in 0..n {
            let (tick, rv, rv2) = (self.tick, self.rng.f(), self.rng.f());
            let m = &mut self.mobs[i];
            m.flash = (m.flash - DT).max(0.0);
            m.cd = (m.cd - DT).max(0.0);
            m.flee = (m.flee - DT).max(0.0);
            if let MobState::Dead(t) = m.state {
                m.state = MobState::Dead(t + DT);
                continue;
            }
            if m.burn > 0.0 {
                m.burn -= DT;
                m.hp -= 3.0 * m.tier.max(1.0) * DT;
                if tick % 4 == 0 {
                    self.parts.push(Particle {
                        x: m.x + rv * 0.4 - 0.2,
                        y: m.y + 0.1,
                        z: 10.0 + rv2 * 24.0,
                        vx: 0.0,
                        vy: 0.0,
                        vz: 30.0,
                        life: 0.4,
                        max: 0.4,
                        kind: PKind::Fire,
                    });
                }
                if m.hp <= 0.0 {
                    m.hp = 0.0;
                    continue;
                }
            }
            if m.stun > 0.0 {
                m.stun -= DT;
                m.moving = false;
                continue;
            }
            let (dx, dy) = (px - m.x, py - m.y);
            let dist = (dx * dx + dy * dy).sqrt();
            m.moving = false;
            let r = m.r;
            match m.state {
                MobState::Idle => {
                    let range = if m.boss { 10.0 } else { 8.5 };
                    if player_alive && !player_safe && dist < range && self.d.los(m.x, m.y, px, py) {
                        m.state = MobState::Chase;
                        aggro_at.push((m.x, m.y));
                        continue;
                    }
                    if m.boss {
                        continue;
                    }
                    // Head home if it wandered off, otherwise shuffle about.
                    let (hx, hy) = (m.home.0 - m.x, m.home.1 - m.y);
                    let home_d = (hx * hx + hy * hy).sqrt();
                    if m.home.0 > -999.0 && home_d > 6.0 {
                        let (ux, uy) = (hx / home_d, hy / home_d);
                        let (mut x, mut y) = (m.x, m.y);
                        move_circle(&self.d, &mut x, &mut y, ux * m.speed * 0.6 * DT, uy * m.speed * 0.6 * DT, r);
                        m.x = x;
                        m.y = y;
                        m.dir = iso::dir8(ux, uy);
                        m.moving = true;
                        m.anim_t += DT * 0.6;
                        continue;
                    }
                    m.wander.2 -= DT;
                    if m.wander.2 <= 0.0 {
                        let a = rv * std::f32::consts::TAU;
                        m.wander = (a.cos(), a.sin(), 1.0 + rv2 * 2.5);
                        if rv2 < 0.5 {
                            m.wander.0 = 0.0;
                            m.wander.1 = 0.0;
                        }
                    }
                    if m.wander.0 != 0.0 || m.wander.1 != 0.0 {
                        let (wx, wy) = (m.wander.0, m.wander.1);
                        let (mut x, mut y) = (m.x, m.y);
                        move_circle(&self.d, &mut x, &mut y, wx * m.speed * 0.35 * DT, wy * m.speed * 0.35 * DT, r);
                        if !in_safe(x, y, 2.0) {
                            m.x = x;
                            m.y = y;
                        }
                        m.dir = iso::dir8(wx, wy);
                        m.moving = true;
                        m.anim_t += DT * 0.5;
                    }
                }
                MobState::Chase => {
                    // Give up: you died, reached town, or led an overworld monster too far from home.
                    let too_far = overworld && m.home.0 > -999.0 && ((m.x - m.home.0).powi(2) + (m.y - m.home.1).powi(2)).sqrt() > 22.0;
                    if !player_alive || (player_safe && !m.boss) || too_far {
                        m.state = MobState::Idle;
                        m.path.clear();
                        continue;
                    }
                    if m.boss {
                        boss_specials(m, dist, (px, py), &mut self.rng, &mut spawns, &mut shots, &mut hazards, &mut texts, summons, &self.d);
                    }
                    let los = self.d.los(m.x, m.y, px, py);
                    // Where to go this tick.
                    let mut away = false;
                    if m.flee > 0.0 {
                        away = true;
                    } else if m.ranged {
                        let keep = if m.kind == Kind::HexWarden { 3.5 } else { 2.8 };
                        if dist < keep {
                            away = true;
                        } else if dist <= 8.5 && los {
                            m.dir = iso::dir8(dx, dy);
                            if m.cd <= 0.0 {
                                m.state = MobState::Attack(m.windup);
                            }
                            continue;
                        }
                    } else if dist < m.reach {
                        if m.cd <= 0.0 {
                            m.state = MobState::Attack(m.windup);
                            m.dir = iso::dir8(dx, dy);
                        }
                        continue;
                    }
                    let (tx, ty) = if away {
                        (m.x - dx, m.y - dy)
                    } else if los {
                        m.path.clear();
                        (px, py)
                    } else {
                        m.repath -= DT;
                        if m.repath <= 0.0 || m.path.is_empty() {
                            m.repath = 0.6 + (i % 7) as f32 * 0.05;
                            m.path = self.d.path((m.x as i32, m.y as i32), (px as i32, py as i32), 2500).unwrap_or_default();
                        }
                        while let Some(&(nx, ny)) = m.path.first() {
                            if (nx - m.x).powi(2) + (ny - m.y).powi(2) < 0.04 {
                                m.path.remove(0);
                            } else {
                                break;
                            }
                        }
                        m.path.first().copied().unwrap_or((px, py))
                    };
                    let (ddx, ddy) = (tx - m.x, ty - m.y);
                    let l = (ddx * ddx + ddy * ddy).sqrt().max(0.001);
                    let (ux, uy) = (ddx / l, ddy / l);
                    let speed = m.speed * if m.enraged { 1.25 } else { 1.0 } * if m.flee > 0.0 { 1.1 } else { 1.0 };
                    let (mut x, mut y) = (m.x, m.y);
                    move_circle(&self.d, &mut x, &mut y, ux * speed * DT, uy * speed * DT, r);
                    m.x = x;
                    m.y = y;
                    m.dir = iso::dir8(ux, uy);
                    m.moving = true;
                    m.anim_t += DT * speed / 1.6;
                }
                MobState::Attack(t) => {
                    let t = t - DT;
                    if t > 0.0 {
                        m.state = MobState::Attack(t);
                        continue;
                    }
                    m.state = MobState::Chase;
                    m.cd = m.cooldown * if m.enraged { 0.75 } else { 1.0 };
                    let dmg = m.dmg.0 + (m.dmg.1 - m.dmg.0) * rv;
                    if m.ranged {
                        let l = dist.max(0.01);
                        let (ux, uy) = (dx / l, dy / l);
                        match m.kind {
                            Kind::HexWarden => {
                                for k in [-1.0f32, 0.0, 1.0] {
                                    let a = uy.atan2(ux) + k * 0.28;
                                    shots.push((m.x, m.y, a.cos() * 7.0, a.sin() * 7.0, dmg, ShotKind::Hex));
                                }
                            }
                            _ => shots.push((m.x, m.y, ux * 9.0, uy * 9.0, dmg, ShotKind::Arrow)),
                        }
                    } else if player_alive && dist < m.reach + 0.4 {
                        hits.push(dmg);
                    } else {
                        self.sfx.push(Sfx::Swing);
                    }
                }
                MobState::Dead(_) => {}
            }
        }
        // Pack aggro: a monster that notices you alerts its friends.
        for (ax, ay) in aggro_at {
            for m in self.mobs.iter_mut() {
                if m.state == MobState::Idle && !m.boss && (m.x - ax).powi(2) + (m.y - ay).powi(2) < 25.0 {
                    m.state = MobState::Chase;
                }
            }
        }
        for (kind, x, y, tier) in spawns {
            if self.d.blocked(x, y, 0.3) {
                continue;
            }
            let mut m = Mob::new(kind, x, y, tier, &mut self.rng);
            m.state = MobState::Chase;
            m.home = (-1000.0, -1000.0); // summoned: no home to return to
            for _ in 0..8 {
                self.spray_at(x, y, PKind::Smoke, 4.0);
            }
            self.mobs.push(m);
        }
        for (x, y, vx, vy, dmg, kind) in shots {
            self.shots.push(Shot { x, y, vx, vy, life: 2.0, dmg, kind });
            self.sfx.push(if kind == ShotKind::Arrow { Sfx::Swing } else { Sfx::Cast });
        }
        self.hazards.extend(hazards);
        for (x, y, t) in texts {
            self.floater(x, y, t.into(), rgb(0xff7050));
        }
        self.separate();
        // Burn deaths.
        for i in 0..self.mobs.len() {
            if self.mobs[i].hp <= 0.0 && self.mobs[i].alive() {
                self.kill(i);
            }
        }
        for dmg in hits {
            self.hurt_player(dmg);
        }
    }

    /// Pushes overlapping monsters apart and off the player.
    fn separate(&mut self) {
        let (px, py) = (self.p.x, self.p.y);
        let n = self.mobs.len();
        for i in 0..n {
            if !self.mobs[i].alive() {
                continue;
            }
            for j in i + 1..n {
                if !self.mobs[j].alive() {
                    continue;
                }
                let (dx, dy) = (self.mobs[j].x - self.mobs[i].x, self.mobs[j].y - self.mobs[i].y);
                let d2 = dx * dx + dy * dy;
                let min = self.mobs[i].r + self.mobs[j].r;
                if d2 < min * min && d2 > 1e-6 {
                    let d = d2.sqrt();
                    let push = (min - d) * 0.5;
                    let (ux, uy) = (dx / d * push, dy / d * push);
                    let (ri, rj) = (self.mobs[i].r, self.mobs[j].r);
                    let (mut x, mut y) = (self.mobs[i].x, self.mobs[i].y);
                    move_circle(&self.d, &mut x, &mut y, -ux, -uy, ri);
                    self.mobs[i].x = x;
                    self.mobs[i].y = y;
                    let (mut x, mut y) = (self.mobs[j].x, self.mobs[j].y);
                    move_circle(&self.d, &mut x, &mut y, ux, uy, rj);
                    self.mobs[j].x = x;
                    self.mobs[j].y = y;
                }
            }
            let (dx, dy) = (self.mobs[i].x - px, self.mobs[i].y - py);
            let d2 = dx * dx + dy * dy;
            let min = self.mobs[i].r + PLAYER_R;
            if d2 < min * min && d2 > 1e-6 {
                let d = d2.sqrt();
                let r = self.mobs[i].r;
                let (mut x, mut y) = (self.mobs[i].x, self.mobs[i].y);
                move_circle(&self.d, &mut x, &mut y, dx / d * (min - d), dy / d * (min - d), r);
                self.mobs[i].x = x;
                self.mobs[i].y = y;
            }
        }
    }

    /// Enemy arrows and bolts.
    pub(crate) fn update_shots(&mut self) {
        let (px, py) = (self.p.x, self.p.y);
        let mut hits = vec![];
        for s in self.shots.iter_mut() {
            s.life -= DT;
            for _ in 0..3 {
                s.x += s.vx * DT / 3.0;
                s.y += s.vy * DT / 3.0;
                if !self.d.walkable(s.x.floor() as i32, s.y.floor() as i32) {
                    s.life = 0.0;
                    break;
                }
                if (s.x - px).powi(2) + (s.y - py).powi(2) < (PLAYER_R + 0.12).powi(2) {
                    hits.push(s.dmg);
                    s.life = 0.0;
                    break;
                }
            }
        }
        self.shots.retain(|s| s.life > 0.0);
        for d in hits {
            self.hurt_player(d);
        }
    }

    /// Poison pools and ash novas.
    pub(crate) fn update_hazards(&mut self) {
        let (px, py) = (self.p.x, self.p.y);
        let mut dmg = 0.0;
        let mut burst = 0.0;
        for h in self.hazards.iter_mut() {
            h.t += DT;
            let inside = (h.x - px).powi(2) + (h.y - py).powi(2) < h.r * h.r;
            if h.t >= h.warn {
                if h.burst > 0.0 && !h.fired {
                    h.fired = true;
                    if inside {
                        burst += h.burst;
                    }
                }
                if h.dps > 0.0 && h.t < h.warn + h.live && inside {
                    dmg += h.dps * DT;
                }
            }
        }
        let novas: Vec<(f32, f32)> = self.hazards.iter().filter(|h| h.kind == HazardKind::Nova && h.fired && h.t - DT < h.warn).map(|h| (h.x, h.y)).collect();
        for (x, y) in novas {
            self.sfx.push(Sfx::Boom);
            self.shake = self.shake.max(0.6);
            for _ in 0..40 {
                self.spray_at(x, y, PKind::Fire, 3.0);
            }
        }
        self.hazards.retain(|h| h.t < h.warn + h.live.max(0.3));
        if burst > 0.0 {
            self.hurt_player(burst);
        }
        if dmg > 0.0 {
            // Poison ticks quietly (no flash spam), but it still kills.
            self.p.hp -= dmg;
            self.stats.damage_taken += dmg;
            if self.p.hp <= 0.0 {
                self.hurt_player(0.0);
            }
        }
    }
}

/// Boss abilities, run while the boss is chasing you.
#[allow(clippy::too_many_arguments)]
fn boss_specials(
    m: &mut Mob,
    dist: f32,
    (px, py): (f32, f32),
    rng: &mut Rng,
    spawns: &mut Vec<(Kind, f32, f32, f32)>,
    shots: &mut Vec<(f32, f32, f32, f32, f32, ShotKind)>,
    hazards: &mut Vec<Hazard>,
    texts: &mut Vec<(f32, f32, &'static str)>,
    summons: usize,
    d: &crate::dungeon::Dungeon,
) {
    m.special -= DT;
    m.special2 -= DT;
    if !m.enraged && m.hp < m.max_hp * 0.5 {
        m.enraged = true;
        texts.push((m.x, m.y, "ENRAGED!"));
    }
    let (mx, my) = (m.x, m.y);
    let around = |rng: &mut Rng, n: usize, kind: Kind, tier: f32, out: &mut Vec<(Kind, f32, f32, f32)>| {
        for k in 0..n {
            let a = k as f32 / n as f32 * std::f32::consts::TAU + rng.f();
            out.push((kind, mx + a.cos() * 1.6, my + a.sin() * 1.6, tier));
        }
    };
    match m.kind {
        Kind::BoneWarden => {
            if m.special <= 0.0 {
                m.special = 9.0;
                if summons < 6 {
                    around(rng, 3, Kind::Skeleton, m.tier, spawns);
                    texts.push((m.x, m.y, "RISE, MY BROTHERS!"));
                }
            }
        }
        Kind::PlagueWarden => {
            if m.special <= 0.0 && dist < 9.0 {
                m.special = if m.enraged { 3.0 } else { 4.5 };
                hazards.push(Hazard {
                    x: px,
                    y: py,
                    r: 1.1,
                    warn: 0.9,
                    live: 5.0,
                    dps: 7.0 * m.tier,
                    burst: 0.0,
                    t: 0.0,
                    fired: false,
                    kind: HazardKind::Poison,
                });
            }
        }
        Kind::HexWarden => {
            if m.special <= 0.0 && dist < 2.5 {
                // Blink away to a spot it can still see you from.
                m.special = 6.0;
                for _ in 0..30 {
                    let a = rng.f() * std::f32::consts::TAU;
                    let rr = rng.rf(5.0, 7.0);
                    let (nx, ny) = (px + a.cos() * rr, py + a.sin() * rr);
                    if !d.blocked(nx, ny, m.r) && d.los(nx, ny, px, py) {
                        m.x = nx;
                        m.y = ny;
                        m.path.clear();
                        texts.push((m.x, m.y, "BLINK"));
                        break;
                    }
                }
            }
            if m.special2 <= 0.0 && m.enraged {
                m.special2 = 12.0;
                around(rng, 3, Kind::Archer, m.tier, spawns);
            }
        }
        Kind::AshKing => {
            if m.special <= 0.0 && dist < 10.0 {
                m.special = if m.enraged { 2.0 } else { 2.8 };
                let n = if m.enraged { 7 } else { 5 };
                let base = (py - m.y).atan2(px - m.x);
                let dmg = 9.0 * m.tier.powf(0.8);
                for k in 0..n {
                    let a = base + (k as f32 - (n - 1) as f32 * 0.5) * 0.22;
                    shots.push((m.x, m.y, a.cos() * 7.5, a.sin() * 7.5, dmg, ShotKind::Ash));
                }
            }
            if m.special2 <= 0.0 && dist < 6.0 {
                m.special2 = 8.0;
                hazards.push(Hazard {
                    x: m.x,
                    y: m.y,
                    r: 2.8,
                    warn: 1.2,
                    live: 0.0,
                    dps: 0.0,
                    burst: 22.0 * m.tier.powf(0.8),
                    t: 0.0,
                    fired: false,
                    kind: HazardKind::Nova,
                });
                texts.push((m.x, m.y, "BURN!"));
                if m.enraged && summons < 8 {
                    around(rng, 4, Kind::Imp, m.tier, spawns);
                }
            }
        }
        _ => {}
    }
}

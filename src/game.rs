//! Game state, simulation and rendering. No SDL here, so the game also runs
//! headless for snapshots and self-tests.
use crate::art::Art;
use crate::dungeon::{Dungeon, Tile};
use crate::gfx::{mix, rgb, Align, Fx, Screen, Sprite, BLACK, WHITE};
use crate::iso;
use crate::rng::Rng;
use crate::sprites;

pub const DT: f32 = 1.0 / 60.0;
pub const HUD_H: i32 = 52;
const MAP_W: i32 = 72;
const MAP_H: i32 = 72;

const PLAYER_R: f32 = 0.3;
const MOB_R: f32 = 0.32;
const PLAYER_SPEED: f32 = 3.6;
const FIREBALL_COST: f32 = 5.0;
const FIREBALL_SPEED: f32 = 10.0;
const CAST_TIME: f32 = 0.32;
/// Face height (pixels) of cut-down front walls.
const LOW_WALL: i32 = 8;

/// Everything the game reads from the outside world for one tick.
#[derive(Default, Clone)]
pub struct Input {
    /// Keyboard / left stick movement in screen space (-1..1).
    pub move_x: f32,
    pub move_y: f32,
    /// Right stick aim in screen space.
    pub aim_x: f32,
    pub aim_y: f32,
    /// Mouse position in framebuffer pixels (None when the mouse hasn't been used).
    pub mouse: Option<(i32, i32)>,
    pub lmb: bool,
    pub rmb: bool,
    /// Shift held: left click casts in place instead of moving.
    pub stand: bool,
    /// Pad / keyboard cast button held (casts at the aim direction or the nearest foe).
    pub cast: bool,
    pub confirm: bool,
    pub potion_hp: bool,
    pub potion_mp: bool,
    /// Toggle the automap (one-shot).
    pub map: bool,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Sfx {
    Cast,
    Boom,
    Hit,
    Hurt,
    Die,
    Swing,
    Pickup,
    Drink,
    Descend,
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Zombie,
    Skeleton,
}

impl Kind {
    fn name(self) -> &'static str {
        match self {
            Kind::Zombie => "zombie",
            Kind::Skeleton => "skeleton",
        }
    }
    fn label(self) -> &'static str {
        match self {
            Kind::Zombie => "ROTTING ZOMBIE",
            Kind::Skeleton => "RISEN SKELETON",
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum MobState {
    Idle,
    Chase,
    /// Wind-up before a swing; the hit lands when the timer runs out.
    Attack(f32),
    Dead(f32),
}

struct Mob {
    kind: Kind,
    x: f32,
    y: f32,
    hp: f32,
    max_hp: f32,
    speed: f32,
    dmg: (i32, i32),
    windup: f32,
    cooldown: f32,
    cd: f32,
    state: MobState,
    dir: usize,
    anim_t: f32,
    moving: bool,
    flash: f32,
    stun: f32,
    burn: f32,
    path: Vec<(f32, f32)>,
    repath: f32,
    wander: (f32, f32, f32),
}

struct Fireball {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    life: f32,
    dmg: f32,
}

#[derive(Clone, Copy)]
enum PKind {
    /// Additive fire spark.
    Fire,
    Smoke,
    Bone,
    Blood,
}

struct Particle {
    x: f32,
    y: f32,
    z: f32,
    vx: f32,
    vy: f32,
    vz: f32,
    life: f32,
    max: f32,
    kind: PKind,
}

struct Floater {
    x: f32,
    y: f32,
    t: f32,
    text: String,
    col: u32,
}

#[derive(Clone, Copy, PartialEq)]
enum Drop {
    Health,
    Mana,
    Gold(i32),
}

struct Pickup {
    x: f32,
    y: f32,
    kind: Drop,
    t: f32,
}

struct Decal {
    x: f32,
    y: f32,
    r: f32,
    col: u32,
    a: f32,
}

struct Light {
    x: f32,
    y: f32,
    r: f32,
    s: f32,
    life: f32,
    max: f32,
}

pub struct Player {
    pub x: f32,
    pub y: f32,
    pub hp: f32,
    pub max_hp: f32,
    pub mana: f32,
    pub max_mana: f32,
    dir: usize,
    anim_t: f32,
    moving: bool,
    cast_t: f32,
    cast_cd: f32,
    path: Vec<(f32, f32)>,
    goal: Option<(f32, f32)>,
    repath: f32,
    flash: f32,
    pub hp_pots: i32,
    pub mp_pots: i32,
    pub gold: i32,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum State {
    Playing,
    Dead(f32),
    Cleared,
}

pub struct Game {
    rng: Rng,
    pub art: Art,
    pub d: Dungeon,
    pub p: Player,
    mobs: Vec<Mob>,
    balls: Vec<Fireball>,
    parts: Vec<Particle>,
    floaters: Vec<Floater>,
    pickups: Vec<Pickup>,
    decals: Vec<Decal>,
    lights: Vec<Light>,
    pub state: State,
    pub depth: u32,
    pub kills: u32,
    pub tick: u32,
    pub sfx: Vec<Sfx>,
    shake: f32,
    /// Mob last hit or hovered (for the top-of-screen health bar).
    focus: Option<usize>,
    focus_t: f32,
    hover: Option<usize>,
    icon: Sprite,
    banner_t: f32,
    view_h: i32,
    light_ready: bool,
    /// Tiles the player has seen (for the automap).
    explored: Vec<bool>,
    pub show_map: bool,
    pub stats: Stats,
}

#[derive(Default, Debug, Clone)]
pub struct Stats {
    pub casts: u32,
    pub hits: u32,
    pub damage_taken: f32,
    pub descents: u32,
}

impl Game {
    pub fn new(seed: u64, view_h: i32) -> Self {
        let mut rng = Rng::new(seed);
        let d = Dungeon::generate(&mut rng, MAP_W, MAP_H);
        let p = Player {
            x: 0.0,
            y: 0.0,
            hp: 70.0,
            max_hp: 70.0,
            mana: 50.0,
            max_mana: 50.0,
            dir: 0,
            anim_t: 0.0,
            moving: false,
            cast_t: 0.0,
            cast_cd: 0.0,
            path: vec![],
            goal: None,
            repath: 0.0,
            flash: 0.0,
            hp_pots: 3,
            mp_pots: 3,
            gold: 0,
        };
        let mut g = Game {
            rng,
            art: Art::load(),
            d,
            p,
            mobs: vec![],
            balls: vec![],
            parts: vec![],
            floaters: vec![],
            pickups: vec![],
            decals: vec![],
            lights: vec![],
            state: State::Playing,
            depth: 1,
            kills: 0,
            tick: 0,
            sfx: vec![],
            shake: 0.0,
            focus: None,
            focus_t: 0.0,
            hover: None,
            icon: sprites::fireball_icon(),
            banner_t: 6.0,
            view_h,
            light_ready: false,
            explored: vec![],
            show_map: false,
            stats: Stats::default(),
        };
        g.populate();
        g
    }

    /// Places the player in the first room and fills the other rooms with monster packs.
    fn populate(&mut self) {
        let r0 = self.d.rooms[0];
        let (cx, cy) = r0.center();
        self.p.x = cx as f32 + 0.5;
        self.p.y = cy as f32 + 0.5;
        self.p.path.clear();
        self.p.goal = None;
        self.explored = vec![false; (self.d.w * self.d.h) as usize];
        self.mobs.clear();
        self.balls.clear();
        self.pickups.clear();
        self.decals.clear();
        self.parts.clear();
        self.floaters.clear();
        self.lights.clear();
        let scale = 1.0 + 0.3 * (self.depth - 1) as f32;
        let rooms = self.d.rooms.clone();
        for r in rooms.iter().skip(1) {
            let n = self.rng.range(2, 5) + (self.depth as i32 - 1).min(3);
            let kind = if self.rng.chance(0.5) { Kind::Zombie } else { Kind::Skeleton };
            for _ in 0..n {
                for _try in 0..20 {
                    let x = self.rng.range(r.x + 1, r.x + r.w - 1) as f32 + 0.5;
                    let y = self.rng.range(r.y + 1, r.y + r.h - 1) as f32 + 0.5;
                    if self.d.blocked(x, y, MOB_R) || self.mobs.iter().any(|m| (m.x - x).abs() + (m.y - y).abs() < 1.0) {
                        continue;
                    }
                    let kind = if self.rng.chance(0.2) { if kind == Kind::Zombie { Kind::Skeleton } else { Kind::Zombie } } else { kind };
                    self.mobs.push(Mob::new(kind, x, y, scale, &mut self.rng));
                    break;
                }
            }
        }
    }

    fn descend(&mut self) {
        self.depth += 1;
        self.stats.descents += 1;
        let seed = self.rng.next_u32() as u64;
        let mut r = Rng::new(seed);
        self.d = Dungeon::generate(&mut r, MAP_W, MAP_H);
        self.populate();
        self.state = State::Playing;
        self.banner_t = 3.0;
        self.p.hp = (self.p.hp + self.p.max_hp * 0.35).min(self.p.max_hp);
        self.p.mana = self.p.max_mana;
        self.sfx.push(Sfx::Descend);
    }

    fn restart(&mut self) {
        self.depth = 0;
        self.kills = 0;
        self.p.max_hp = 70.0;
        self.p.hp = 70.0;
        self.p.mana = self.p.max_mana;
        self.p.hp_pots = 3;
        self.p.mp_pots = 3;
        self.p.gold = 0;
        self.descend();
    }

    /// Nearest living monster for the test bot: (x, y, distance, in line of sight).
    pub fn bot_target(&self) -> Option<(f32, f32, f32, bool)> {
        self.mobs
            .iter()
            .filter(|m| !matches!(m.state, MobState::Dead(_)))
            .map(|m| (m.x, m.y, ((m.x - self.p.x).powi(2) + (m.y - self.p.y).powi(2)).sqrt()))
            .min_by(|a, b| a.2.partial_cmp(&b.2).unwrap())
            .map(|(x, y, d)| (x, y, d, self.d.los(self.p.x, self.p.y, x, y)))
    }

    /// Next waypoint on the A* path from the player to (tx, ty).
    pub fn bot_step(&self, tx: f32, ty: f32) -> Option<(f32, f32)> {
        let path = self.d.path((self.p.x as i32, self.p.y as i32), (tx as i32, ty as i32), 6000)?;
        path.first().copied()
    }

    /// True while fire is on screen (for picking interesting snapshot frames).
    pub fn fire_active(&self) -> bool {
        !self.balls.is_empty() && !self.lights.is_empty()
    }

    pub fn alive_mobs(&self) -> usize {
        self.mobs.iter().filter(|m| !matches!(m.state, MobState::Dead(_))).count()
    }

    // ------------------------------------------------------------------ update

    pub fn update(&mut self, inp: &Input) {
        self.tick += 1;
        self.banner_t = (self.banner_t - DT).max(0.0);
        self.shake = (self.shake - DT * 6.0).max(0.0);
        match self.state {
            State::Dead(t) => {
                self.state = State::Dead(t + DT);
                if t > 1.5 && inp.confirm {
                    self.restart();
                }
            }
            State::Cleared => {
                if inp.confirm {
                    self.descend();
                    return;
                }
            }
            State::Playing => {}
        }
        if inp.map {
            self.show_map = !self.show_map;
        }
        if self.state == State::Playing || self.state == State::Cleared {
            self.update_player(inp);
        }
        self.explore();
        self.update_mobs();
        self.update_balls();
        self.update_world();
        if self.state == State::Playing && self.alive_mobs() == 0 {
            self.state = State::Cleared;
        }
    }

    /// Mouse position in world coordinates.
    fn mouse_world(&self, m: (i32, i32)) -> (f32, f32) {
        let (ox, oy) = self.cam_origin();
        let (wx, wy) = iso::to_world(m.0 as f32 - ox, m.1 as f32 - oy);
        (wx + self.p.x, wy + self.p.y)
    }

    /// Screen position of the player's feet.
    fn cam_origin(&self) -> (f32, f32) {
        (crate::gfx::SW as f32 * 0.5, ((self.view_h - HUD_H) as f32 * 0.5 + 18.0).round())
    }

    fn update_player(&mut self, inp: &Input) {
        let dead = matches!(self.state, State::Dead(_));
        if dead {
            return;
        }
        let p = &mut self.p;
        p.flash = (p.flash - DT).max(0.0);
        p.cast_cd = (p.cast_cd - DT).max(0.0);
        p.cast_t = (p.cast_t - DT).max(0.0);
        p.mana = (p.mana + 2.2 * DT).min(p.max_mana);
        p.hp = (p.hp + 0.4 * DT).min(p.max_hp);

        if inp.potion_hp && self.p.hp_pots > 0 && self.p.hp < self.p.max_hp {
            self.p.hp_pots -= 1;
            self.p.hp = (self.p.hp + 40.0).min(self.p.max_hp);
            self.sfx.push(Sfx::Drink);
            self.floater(self.p.x, self.p.y, "+40".into(), rgb(0xff5050));
        }
        if inp.potion_mp && self.p.mp_pots > 0 && self.p.mana < self.p.max_mana {
            self.p.mp_pots -= 1;
            self.p.mana = (self.p.mana + 35.0).min(self.p.max_mana);
            self.sfx.push(Sfx::Drink);
            self.floater(self.p.x, self.p.y, "+35".into(), rgb(0x5080ff));
        }

        // Hovered monster (mouse picking against sprite boxes).
        self.hover = inp.mouse.and_then(|m| self.pick_mob(m));

        // ---- casting ----
        let mut cast_at: Option<(f32, f32)> = None;
        if let Some(m) = inp.mouse {
            let target = self.hover.map(|i| (self.mobs[i].x, self.mobs[i].y)).unwrap_or_else(|| self.mouse_world(m));
            if inp.rmb || (inp.lmb && (inp.stand || self.hover.is_some())) {
                cast_at = Some(target);
            }
        }
        if inp.cast {
            let aim = inp.aim_x * inp.aim_x + inp.aim_y * inp.aim_y;
            if aim > 0.09 {
                let (dx, dy) = iso::screen_dir_to_world(inp.aim_x, inp.aim_y);
                cast_at = Some((self.p.x + dx * 6.0, self.p.y + dy * 6.0));
            } else if let Some(i) = self.nearest_visible_mob(10.0) {
                cast_at = Some((self.mobs[i].x, self.mobs[i].y));
            } else if let Some(m) = inp.mouse.filter(|_| inp.move_x == 0.0 && inp.move_y == 0.0) {
                cast_at = Some(self.mouse_world(m));
            } else {
                let (dx, dy) = dir_vec(self.p.dir);
                cast_at = Some((self.p.x + dx * 6.0, self.p.y + dy * 6.0));
            }
        }
        if let Some((tx, ty)) = cast_at {
            self.p.path.clear();
            self.p.goal = None;
            let (dx, dy) = (tx - self.p.x, ty - self.p.y);
            if dx * dx + dy * dy > 0.01 {
                self.p.dir = iso::dir8(dx, dy);
            }
            if self.p.cast_cd <= 0.0 {
                if self.p.mana >= FIREBALL_COST {
                    self.cast_fireball(tx, ty);
                } else if self.p.cast_cd <= 0.0 {
                    self.p.cast_cd = 0.4;
                    self.floater(self.p.x, self.p.y, "NO MANA".into(), rgb(0x7090ff));
                }
            }
        }

        // ---- movement ----
        let mut mv = (0.0f32, 0.0f32);
        let key = inp.move_x * inp.move_x + inp.move_y * inp.move_y;
        if key > 0.04 {
            let (dx, dy) = iso::screen_dir_to_world(inp.move_x, inp.move_y);
            let mag = key.sqrt().min(1.0);
            mv = (dx * mag, dy * mag);
            self.p.path.clear();
            self.p.goal = None;
        } else if cast_at.is_none() {
            if let (Some(m), true) = (inp.mouse, inp.lmb && !inp.stand && self.hover.is_none()) {
                let goal = self.mouse_world(m);
                self.p.repath -= DT;
                let changed = self.p.goal.map_or(true, |g| (g.0 - goal.0).abs() + (g.1 - goal.1).abs() > 0.5);
                if changed || self.p.repath <= 0.0 {
                    self.p.goal = Some(goal);
                    self.p.repath = 0.25;
                    let from = (self.p.x.floor() as i32, self.p.y.floor() as i32);
                    let to = (goal.0.floor() as i32, goal.1.floor() as i32);
                    self.p.path = self.d.path(from, to, 4000).unwrap_or_default();
                    if let Some(last) = self.p.path.last_mut() {
                        *last = goal;
                    }
                }
            }
            if let Some(&(nx, ny)) = self.p.path.first() {
                let (dx, dy) = (nx - self.p.x, ny - self.p.y);
                let l = (dx * dx + dy * dy).sqrt();
                if l < 0.12 {
                    self.p.path.remove(0);
                } else {
                    mv = (dx / l, dy / l);
                }
            } else if let Some(goal) = self.p.goal.filter(|_| inp.lmb) {
                // Clicked somewhere unreachable: walk straight at it and slide along walls.
                let (dx, dy) = (goal.0 - self.p.x, goal.1 - self.p.y);
                let l = (dx * dx + dy * dy).sqrt();
                if l > 0.15 {
                    mv = (dx / l, dy / l);
                }
            }
        }
        let casting = self.p.cast_t > 0.0;
        let speed = if casting { PLAYER_SPEED * 0.25 } else { PLAYER_SPEED };
        self.p.moving = mv.0 != 0.0 || mv.1 != 0.0;
        if self.p.moving {
            if !casting {
                self.p.dir = iso::dir8(mv.0, mv.1);
            }
            let (mut x, mut y) = (self.p.x, self.p.y);
            move_circle(&self.d, &mut x, &mut y, mv.0 * speed * DT, mv.1 * speed * DT, PLAYER_R);
            self.p.x = x;
            self.p.y = y;
            self.p.anim_t += DT;
        } else {
            self.p.anim_t = 0.0;
        }

        // Pickups
        let (px, py) = (self.p.x, self.p.y);
        let mut got = vec![];
        self.pickups.retain(|k| {
            if (k.x - px).powi(2) + (k.y - py).powi(2) < 0.5 && k.t > 0.3 {
                got.push(k.kind);
                false
            } else {
                true
            }
        });
        for k in got {
            self.sfx.push(Sfx::Pickup);
            match k {
                Drop::Health => {
                    self.p.hp_pots += 1;
                    self.floater(px, py, "HEALING POTION".into(), rgb(0xff6060));
                }
                Drop::Mana => {
                    self.p.mp_pots += 1;
                    self.floater(px, py, "MANA POTION".into(), rgb(0x6090ff));
                }
                Drop::Gold(n) => {
                    self.p.gold += n;
                    self.floater(px, py, format!("{n} GOLD"), rgb(0xe8c050));
                }
            }
        }
    }

    /// Marks tiles around the player (within the light radius, in line of sight) as seen.
    fn explore(&mut self) {
        if self.tick % 6 != 0 {
            return;
        }
        let (px, py) = (self.p.x, self.p.y);
        let r = 9;
        for ty in py as i32 - r..=py as i32 + r {
            for tx in px as i32 - r..=px as i32 + r {
                if tx < 0 || ty < 0 || tx >= self.d.w || ty >= self.d.h {
                    continue;
                }
                let i = (ty * self.d.w + tx) as usize;
                if self.explored[i] {
                    continue;
                }
                let (cx, cy) = (tx as f32 + 0.5, ty as f32 + 0.5);
                if (cx - px).powi(2) + (cy - py).powi(2) > (r * r) as f32 {
                    continue;
                }
                // Walls are seen when the floor next to them is visible.
                let (lx, ly) = (cx + (px - cx).signum() * 0.6, cy + (py - cy).signum() * 0.6);
                if self.d.los(px, py, cx, cy) || self.d.los(px, py, lx, ly) {
                    self.explored[i] = true;
                }
            }
        }
    }

    fn cast_fireball(&mut self, tx: f32, ty: f32) {
        let p = &mut self.p;
        p.mana -= FIREBALL_COST;
        p.cast_cd = CAST_TIME;
        p.cast_t = CAST_TIME;
        self.stats.casts += 1;
        let (dx, dy) = (tx - p.x, ty - p.y);
        let l = (dx * dx + dy * dy).sqrt().max(0.001);
        let (ux, uy) = (dx / l, dy / l);
        let dmg = self.rng.rf(9.0, 15.0) * (1.0 + 0.12 * (self.depth - 1) as f32);
        self.balls.push(Fireball { x: p.x + ux * 0.45, y: p.y + uy * 0.45, vx: ux * FIREBALL_SPEED, vy: uy * FIREBALL_SPEED, life: 1.1, dmg });
        self.sfx.push(Sfx::Cast);
    }

    fn nearest_visible_mob(&self, range: f32) -> Option<usize> {
        self.mobs
            .iter()
            .enumerate()
            .filter(|(_, m)| !matches!(m.state, MobState::Dead(_)))
            .map(|(i, m)| (i, (m.x - self.p.x).powi(2) + (m.y - self.p.y).powi(2)))
            .filter(|&(i, d2)| d2 < range * range && self.d.los(self.p.x, self.p.y, self.mobs[i].x, self.mobs[i].y))
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .map(|v| v.0)
    }

    fn pick_mob(&self, m: (i32, i32)) -> Option<usize> {
        let (ox, oy) = self.cam_origin();
        let mut best: Option<(usize, f32)> = None;
        for (i, mob) in self.mobs.iter().enumerate() {
            if matches!(mob.state, MobState::Dead(_)) {
                continue;
            }
            let (sx, sy) = iso::to_screen(mob.x - self.p.x, mob.y - self.p.y);
            let (sx, sy) = (sx + ox, sy + oy);
            let h = self.art.char(mob.kind.name()).height as f32;
            let (mx, my) = (m.0 as f32, m.1 as f32);
            if (mx - sx).abs() < 12.0 && my < sy + 4.0 && my > sy - h {
                let depth = mob.x + mob.y;
                if best.map_or(true, |b| depth > b.1) {
                    best = Some((i, depth));
                }
            }
        }
        best.map(|b| b.0)
    }

    fn update_mobs(&mut self) {
        let (px, py) = (self.p.x, self.p.y);
        let player_alive = !matches!(self.state, State::Dead(_));
        let n = self.mobs.len();
        let mut hits: Vec<f32> = vec![];
        let mut aggro_at: Vec<(f32, f32)> = vec![];
        for i in 0..n {
            let (tick, rng_v) = (self.tick, self.rng.f());
            let m = &mut self.mobs[i];
            m.flash = (m.flash - DT).max(0.0);
            m.cd = (m.cd - DT).max(0.0);
            if let MobState::Dead(t) = m.state {
                m.state = MobState::Dead(t + DT);
                continue;
            }
            // Burning damage over time.
            if m.burn > 0.0 {
                m.burn -= DT;
                m.hp -= 3.0 * DT;
                if tick % 4 == 0 {
                    self.parts.push(Particle {
                        x: m.x + rng_v * 0.4 - 0.2,
                        y: m.y + 0.1,
                        z: 10.0 + rng_v * 24.0,
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
            match m.state {
                MobState::Idle => {
                    if player_alive && dist < 8.5 && self.d.los(m.x, m.y, px, py) {
                        m.state = MobState::Chase;
                        aggro_at.push((m.x, m.y));
                    } else {
                        // Shuffle about a little.
                        m.wander.2 -= DT;
                        if m.wander.2 <= 0.0 {
                            let a = rng_v * std::f32::consts::TAU;
                            m.wander = (a.cos(), a.sin(), 1.0 + rng_v * 2.5);
                            if rng_v < 0.5 {
                                m.wander.0 = 0.0;
                                m.wander.1 = 0.0;
                            }
                        }
                        if m.wander.0 != 0.0 || m.wander.1 != 0.0 {
                            let (wx, wy) = (m.wander.0, m.wander.1);
                            let (mut x, mut y) = (m.x, m.y);
                            move_circle(&self.d, &mut x, &mut y, wx * m.speed * 0.35 * DT, wy * m.speed * 0.35 * DT, MOB_R);
                            m.x = x;
                            m.y = y;
                            m.dir = iso::dir8(wx, wy);
                            m.moving = true;
                            m.anim_t += DT * 0.5;
                        }
                    }
                }
                MobState::Chase => {
                    if !player_alive {
                        m.state = MobState::Idle;
                        continue;
                    }
                    if dist < 0.85 {
                        if m.cd <= 0.0 {
                            m.state = MobState::Attack(m.windup);
                            m.dir = iso::dir8(dx, dy);
                        }
                        continue;
                    }
                    let (tx, ty) = if self.d.los(m.x, m.y, px, py) {
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
                    let (mut x, mut y) = (m.x, m.y);
                    move_circle(&self.d, &mut x, &mut y, ux * m.speed * DT, uy * m.speed * DT, MOB_R);
                    m.x = x;
                    m.y = y;
                    m.dir = iso::dir8(ux, uy);
                    m.moving = true;
                    m.anim_t += DT * m.speed / 1.6;
                }
                MobState::Attack(t) => {
                    let t = t - DT;
                    if t <= 0.0 {
                        m.state = MobState::Chase;
                        m.cd = m.cooldown;
                        if player_alive && dist < 1.25 {
                            hits.push(self.rng.range(m.dmg.0, m.dmg.1 + 1) as f32);
                        } else {
                            self.sfx.push(Sfx::Swing);
                        }
                    } else {
                        m.state = MobState::Attack(t);
                    }
                }
                MobState::Dead(_) => {}
            }
        }
        // Pack aggro: a monster that notices you alerts its friends.
        for (ax, ay) in aggro_at {
            for m in self.mobs.iter_mut() {
                if m.state == MobState::Idle && (m.x - ax).powi(2) + (m.y - ay).powi(2) < 25.0 {
                    m.state = MobState::Chase;
                }
            }
        }
        // Separation so packs don't stack on one spot.
        for i in 0..n {
            if matches!(self.mobs[i].state, MobState::Dead(_)) {
                continue;
            }
            for j in i + 1..n {
                if matches!(self.mobs[j].state, MobState::Dead(_)) {
                    continue;
                }
                let (dx, dy) = (self.mobs[j].x - self.mobs[i].x, self.mobs[j].y - self.mobs[i].y);
                let d2 = dx * dx + dy * dy;
                let min = MOB_R * 2.0;
                if d2 < min * min && d2 > 1e-6 {
                    let d = d2.sqrt();
                    let push = (min - d) * 0.5;
                    let (ux, uy) = (dx / d * push, dy / d * push);
                    let (mut x, mut y) = (self.mobs[i].x, self.mobs[i].y);
                    move_circle(&self.d, &mut x, &mut y, -ux, -uy, MOB_R);
                    self.mobs[i].x = x;
                    self.mobs[i].y = y;
                    let (mut x, mut y) = (self.mobs[j].x, self.mobs[j].y);
                    move_circle(&self.d, &mut x, &mut y, ux, uy, MOB_R);
                    self.mobs[j].x = x;
                    self.mobs[j].y = y;
                }
            }
            // Keep off the player.
            let (dx, dy) = (self.mobs[i].x - px, self.mobs[i].y - py);
            let d2 = dx * dx + dy * dy;
            let min = MOB_R + PLAYER_R;
            if d2 < min * min && d2 > 1e-6 {
                let d = d2.sqrt();
                let (mut x, mut y) = (self.mobs[i].x, self.mobs[i].y);
                move_circle(&self.d, &mut x, &mut y, dx / d * (min - d), dy / d * (min - d), MOB_R);
                self.mobs[i].x = x;
                self.mobs[i].y = y;
            }
        }
        // Burn deaths.
        for i in 0..n {
            if self.mobs[i].hp <= 0.0 && !matches!(self.mobs[i].state, MobState::Dead(_)) {
                self.kill(i);
            }
        }
        for dmg in hits {
            self.hurt_player(dmg);
        }
    }

    fn hurt_player(&mut self, dmg: f32) {
        if matches!(self.state, State::Dead(_)) {
            return;
        }
        self.p.hp -= dmg;
        self.stats.damage_taken += dmg;
        self.p.flash = 0.2;
        self.shake = self.shake.max(0.5);
        self.sfx.push(Sfx::Hurt);
        self.floater(self.p.x, self.p.y, format!("{}", dmg as i32), rgb(0xff4040));
        if self.p.hp <= 0.0 {
            self.p.hp = 0.0;
            self.state = State::Dead(0.0);
            self.sfx.push(Sfx::Die);
            for _ in 0..30 {
                self.spray(self.p.x, self.p.y, PKind::Blood, 20.0);
            }
        }
    }

    fn update_balls(&mut self) {
        let mut booms = vec![];
        for b in self.balls.iter_mut() {
            b.life -= DT;
            let steps = 3;
            let mut hit = b.life <= 0.0;
            for _ in 0..steps {
                if hit {
                    break;
                }
                b.x += b.vx * DT / steps as f32;
                b.y += b.vy * DT / steps as f32;
                if !self.d.walkable(b.x.floor() as i32, b.y.floor() as i32) {
                    hit = true;
                    b.x -= b.vx * DT / steps as f32;
                    b.y -= b.vy * DT / steps as f32;
                }
                if self.mobs.iter().any(|m| !matches!(m.state, MobState::Dead(_)) && (m.x - b.x).powi(2) + (m.y - b.y).powi(2) < 0.45 * 0.45) {
                    hit = true;
                }
            }
            if hit {
                booms.push((b.x, b.y, b.dmg));
                b.life = -1.0;
            }
        }
        // Trail sparks.
        for i in 0..self.balls.len() {
            let (bx, by, vx, vy) = (self.balls[i].x, self.balls[i].y, self.balls[i].vx, self.balls[i].vy);
            for _ in 0..2 {
                let (r1, r2, r3) = (self.rng.f() - 0.5, self.rng.f() - 0.5, self.rng.f());
                self.parts.push(Particle {
                    x: bx + r1 * 0.15,
                    y: by + r2 * 0.15,
                    z: 22.0 + r3 * 4.0,
                    vx: -vx * 0.08 + r1,
                    vy: -vy * 0.08 + r2,
                    vz: 8.0 + r3 * 10.0,
                    life: 0.35,
                    max: 0.35,
                    kind: PKind::Fire,
                });
            }
        }
        self.balls.retain(|b| b.life > -0.5);
        for (x, y, dmg) in booms {
            self.explode(x, y, dmg);
        }
    }

    fn explode(&mut self, x: f32, y: f32, dmg: f32) {
        self.sfx.push(Sfx::Boom);
        self.shake = self.shake.max(0.35);
        self.lights.push(Light { x, y, r: 150.0, s: 1.3, life: 0.35, max: 0.35 });
        self.decals.push(Decal { x, y, r: 0.55 + self.rng.f() * 0.2, col: rgb(0x100804), a: 0.55 });
        if self.decals.len() > 120 {
            self.decals.remove(0);
        }
        for _ in 0..26 {
            let a = self.rng.f() * std::f32::consts::TAU;
            let s = self.rng.rf(1.0, 4.5);
            let (vz, life) = (self.rng.rf(10.0, 70.0), self.rng.rf(0.3, 0.7));
            self.parts.push(Particle { x, y, z: 18.0, vx: a.cos() * s, vy: a.sin() * s, vz, life, max: life, kind: PKind::Fire });
        }
        for _ in 0..6 {
            let a = self.rng.f() * std::f32::consts::TAU;
            let life = self.rng.rf(0.6, 1.1);
            self.parts.push(Particle { x, y, z: 16.0, vx: a.cos() * 0.6, vy: a.sin() * 0.6, vz: 18.0, life, max: life, kind: PKind::Smoke });
        }
        let mut hit_any = false;
        for i in 0..self.mobs.len() {
            let m = &self.mobs[i];
            if matches!(m.state, MobState::Dead(_)) {
                continue;
            }
            let d2 = (m.x - x).powi(2) + (m.y - y).powi(2);
            if d2 > 1.3 * 1.3 {
                continue;
            }
            let dmg = if d2 < 0.6 * 0.6 { dmg } else { dmg * 0.5 };
            hit_any = true;
            let (mx, my) = (m.x, m.y);
            let m = &mut self.mobs[i];
            m.hp -= dmg;
            m.flash = 0.12;
            m.burn = 2.0;
            m.stun = m.stun.max(0.15);
            if m.state == MobState::Idle {
                m.state = MobState::Chase;
            }
            if let MobState::Attack(_) = m.state {
                // Getting hit interrupts the swing (D2 hit recovery).
                m.state = MobState::Chase;
                m.cd = m.cd.max(0.3);
            }
            // Knockback.
            let (kx, ky) = (mx - x, my - y);
            let kl = (kx * kx + ky * ky).sqrt().max(0.01);
            let (mut nx, mut ny) = (mx, my);
            move_circle(&self.d, &mut nx, &mut ny, kx / kl * 0.25, ky / kl * 0.25, MOB_R);
            m.x = nx;
            m.y = ny;
            self.focus = Some(i);
            self.focus_t = 3.0;
            self.floater(mx, my, format!("{}", dmg.round() as i32), rgb(0xffc040));
            if self.mobs[i].hp <= 0.0 {
                self.kill(i);
            }
        }
        if hit_any {
            self.stats.hits += 1;
            self.sfx.push(Sfx::Hit);
        }
    }

    fn kill(&mut self, i: usize) {
        let (x, y, kind) = (self.mobs[i].x, self.mobs[i].y, self.mobs[i].kind);
        self.mobs[i].state = MobState::Dead(0.0);
        self.mobs[i].hp = 0.0;
        self.kills += 1;
        self.sfx.push(Sfx::Die);
        let pk = if kind == Kind::Skeleton { PKind::Bone } else { PKind::Blood };
        for _ in 0..14 {
            self.spray(x, y, pk, 22.0);
        }
        if kind == Kind::Zombie {
            self.decals.push(Decal { x, y, r: 0.4, col: rgb(0x301008), a: 0.5 });
        }
        let r = self.rng.f();
        let drop = if r < 0.14 {
            Some(Drop::Health)
        } else if r < 0.26 {
            Some(Drop::Mana)
        } else if r < 0.6 {
            Some(Drop::Gold(self.rng.range(3, 12) * self.depth as i32))
        } else {
            None
        };
        if let Some(k) = drop {
            self.pickups.push(Pickup { x, y, kind: k, t: 0.0 });
        }
    }

    fn spray(&mut self, x: f32, y: f32, kind: PKind, z: f32) {
        let a = self.rng.f() * std::f32::consts::TAU;
        let s = self.rng.rf(0.5, 2.5);
        let life = self.rng.rf(0.5, 1.2);
        let vz = self.rng.rf(20.0, 60.0);
        self.parts.push(Particle { x, y, z, vx: a.cos() * s, vy: a.sin() * s, vz, life, max: life, kind });
    }

    fn floater(&mut self, x: f32, y: f32, text: String, col: u32) {
        self.floaters.push(Floater { x, y, t: 0.0, text, col });
    }

    fn update_world(&mut self) {
        for p in self.parts.iter_mut() {
            p.life -= DT;
            p.x += p.vx * DT;
            p.y += p.vy * DT;
            p.z += p.vz * DT;
            match p.kind {
                PKind::Fire => p.vz += 12.0 * DT,
                PKind::Smoke => p.vz = 14.0,
                PKind::Bone | PKind::Blood => {
                    p.vz -= 160.0 * DT;
                    if p.z < 0.0 {
                        p.z = 0.0;
                        p.vz = 0.0;
                        p.vx *= 0.5;
                        p.vy *= 0.5;
                    }
                }
            }
        }
        self.parts.retain(|p| p.life > 0.0);
        if self.parts.len() > 1500 {
            let n = self.parts.len() - 1500;
            self.parts.drain(0..n);
        }
        for f in self.floaters.iter_mut() {
            f.t += DT;
        }
        self.floaters.retain(|f| f.t < 1.0);
        for k in self.pickups.iter_mut() {
            k.t += DT;
        }
        for l in self.lights.iter_mut() {
            l.life -= DT;
        }
        self.lights.retain(|l| l.life > 0.0);
        self.focus_t -= DT;
        if self.focus_t <= 0.0 {
            self.focus = None;
        }
    }

    // ------------------------------------------------------------------ draw

    pub fn draw(&mut self, scr: &mut Screen) {
        let (ox, oy) = self.cam_origin();
        if !self.light_ready {
            scr.build_base_light(ox as i32, oy as i32 - 14, 250.0, 0.10);
            self.light_ready = true;
        }
        let sh = if self.shake > 0.0 { ((self.tick as f32 * 1.7).sin() * self.shake * 4.0) as i32 } else { 0 };
        scr.shake = (sh, (sh as f32 * 0.5) as i32);
        scr.clear(BLACK);
        let (px, py) = (self.p.x, self.p.y);
        let to_scr = |x: f32, y: f32| -> (i32, i32) {
            let (sx, sy) = iso::to_screen(x - px, y - py);
            ((sx + ox).round() as i32, (sy + oy).round() as i32)
        };
        let view_h = self.view_h;

        // Visible tile range: invert the four screen corners.
        let corners = [(0.0, -40.0), (scr.w as f32, -40.0), (0.0, view_h as f32 + 60.0), (scr.w as f32, view_h as f32 + 60.0)];
        let (mut x0, mut y0, mut x1, mut y1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
        for (cx, cy) in corners {
            let (wx, wy) = iso::to_world(cx - ox, cy - oy);
            let (wx, wy) = (wx + px, wy + py);
            x0 = x0.min(wx.floor() as i32 - 1);
            y0 = y0.min(wy.floor() as i32 - 1);
            x1 = x1.max(wx.ceil() as i32 + 1);
            y1 = y1.max(wy.ceil() as i32 + 4);
        }
        x0 = x0.max(0);
        y0 = y0.max(0);
        x1 = x1.min(self.d.w - 1);
        y1 = y1.min(self.d.h - 1);

        // 1. Floors.
        for ty in y0..=y1 {
            for tx in x0..=x1 {
                if self.d.get(tx, ty) == Tile::Floor || self.d.get(tx, ty) == Tile::Wall {
                    let (sx, sy) = to_scr(tx as f32 + 0.5, ty as f32 + 0.5);
                    if sx < -20 || sx > scr.w + 20 || sy < -12 || sy > view_h + 12 {
                        continue;
                    }
                    let v = self.d.var[(ty * self.d.w + tx) as usize] as usize;
                    let f = &self.art.floors[v % self.art.floors.len()];
                    scr.blit(f, sx, sy, Fx::default());
                }
            }
        }
        // Scorch marks and blood.
        for dc in &self.decals {
            let (sx, sy) = to_scr(dc.x, dc.y);
            let r = dc.r * iso::TW * 0.5;
            for yy in -(r as i32 / 2)..=(r as i32 / 2) {
                for xx in -(r as i32)..=(r as i32) {
                    let d = (xx as f32 / r).powi(2) + (yy as f32 * 2.0 / r).powi(2);
                    if d < 1.0 && ((xx * 7 + yy * 13) & 3) != 0 {
                        let (x, y) = (sx + xx + scr.shake.0, sy + yy + scr.shake.1);
                        if x >= 0 && y >= 0 && x < scr.w && y < view_h {
                            let i = (y * scr.w + x) as usize;
                            scr.px[i] = mix(scr.px[i], dc.col, dc.a * (1.0 - d));
                        }
                    }
                }
            }
        }
        // Shadows & pickups sit on the floor.
        for k in &self.pickups {
            let (sx, sy) = to_scr(k.x, k.y);
            draw_pickup(scr, k, sx, sy, self.tick);
        }

        // 2. Depth-sorted walls and actors.
        enum D {
            Wall(i32, i32),
            Mob(usize),
            Player,
            Ball(usize),
        }
        let mut list: Vec<(f32, D)> = vec![];
        for ty in y0..=y1 {
            for tx in x0..=x1 {
                if self.d.get(tx, ty) == Tile::Wall {
                    list.push((tx as f32 + ty as f32 + 1.0, D::Wall(tx, ty)));
                }
            }
        }
        for (i, m) in self.mobs.iter().enumerate() {
            let depth = m.x + m.y - if matches!(m.state, MobState::Dead(_)) { 0.6 } else { 0.0 };
            if (m.x - px).abs() < 30.0 && (m.y - py).abs() < 30.0 {
                list.push((depth, D::Mob(i)));
            }
        }
        list.push((px + py, D::Player));
        for (i, b) in self.balls.iter().enumerate() {
            list.push((b.x + b.y, D::Ball(i)));
        }
        list.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        let (psx, psy) = to_scr(px, py);
        let player_depth = px + py;
        for (depth, item) in &list {
            match *item {
                D::Wall(tx, ty) => {
                    let (sx, sy) = to_scr(tx as f32 + 0.5, ty as f32 + 0.5);
                    let w = &self.art.wall;
                    let top = sy - w.ay;
                    if sx < -20 || sx > scr.w + 20 || top > view_h || sy + 10 < 0 {
                        continue;
                    }
                    // Walls with floor behind them (the camera side of a room) are cut down
                    // to a low ledge so they never hide the room, like D2's front walls.
                    let low = [(-1, 0), (0, -1), (-1, -1)].iter().any(|(dx, dy)| self.d.get(tx + dx, ty + dy) == Tile::Floor);
                    if low {
                        let drop = (w.ay - 8 - LOW_WALL).max(0);
                        scr.blit(w, sx, sy + drop, Fx { cut: w.h - drop, ..Fx::default() });
                        continue;
                    }
                    // See-through walls between the camera and the player.
                    let front = *depth > player_depth + 0.3;
                    let over = (sx - psx).abs() < 26 && psy - 44 < sy + 8 && psy > top;
                    let fx = Fx { dither: front && over, ..Fx::default() };
                    scr.blit(w, sx, sy, fx);
                }
                D::Mob(i) => self.draw_mob(scr, i, to_scr(self.mobs[i].x, self.mobs[i].y)),
                D::Player => self.draw_player(scr, (psx, psy)),
                D::Ball(i) => {
                    let b = &self.balls[i];
                    let (sx, sy) = to_scr(b.x, b.y);
                    // Shadow on the floor.
                    blend_ellipse(scr, sx, sy, 5, 2, BLACK, 0.45);
                }
            }
        }

        // 3. Lighting.
        scr.begin_light();
        for b in &self.balls {
            let (sx, sy) = to_scr(b.x, b.y);
            scr.add_light(sx, sy, 110.0, 0.9);
        }
        for l in &self.lights {
            let (sx, sy) = to_scr(l.x, l.y);
            scr.add_light(sx, sy, l.r, l.s * (l.life / l.max));
        }
        if self.p.cast_t > 0.0 {
            scr.add_light(psx, psy - 20, 70.0, 0.4 * self.p.cast_t / CAST_TIME);
        }
        for m in &self.mobs {
            if m.burn > 0.0 && !matches!(m.state, MobState::Dead(_)) {
                let (sx, sy) = to_scr(m.x, m.y);
                scr.add_light(sx, sy - 10, 50.0, 0.35);
            }
        }
        scr.apply_light(view_h);

        // 4. Unlit, additive fire on top.
        for b in &self.balls {
            let (sx, sy) = to_scr(b.x, b.y);
            let fl = ((self.tick as f32) * 0.9).sin() * 1.5;
            scr.glow(sx, sy - 22, 26.0 + fl, rgb(0xff5010), 0.9);
            scr.glow(sx, sy - 22, 12.0, rgb(0xffd060), 1.0);
            scr.disc(sx + scr.shake.0, sy - 22 + scr.shake.1, 3, rgb(0xfff4c0));
        }
        for l in &self.lights {
            let (sx, sy) = to_scr(l.x, l.y);
            let k = l.life / l.max;
            scr.glow(sx, sy - 16, 50.0 * (1.2 - k * 0.5), rgb(0xff6010), 1.2 * k);
            scr.glow(sx, sy - 16, 22.0 * (1.3 - k * 0.5), rgb(0xffe080), 1.2 * k);
        }
        for p in &self.parts {
            let (sx, sy) = to_scr(p.x, p.y);
            let (sx, sy) = (sx + scr.shake.0, sy - p.z as i32 + scr.shake.1);
            if sx < 0 || sy < 0 || sx >= scr.w || sy >= view_h {
                continue;
            }
            let k = p.life / p.max;
            match p.kind {
                PKind::Fire => {
                    let c = if k > 0.6 { rgb(0xffe890) } else if k > 0.3 { rgb(0xff9030) } else { rgb(0xc03010) };
                    let i = (sy * scr.w + sx) as usize;
                    scr.px[i] = crate::gfx::add(scr.px[i], c, 0.5 + k);
                    if k > 0.5 {
                        scr.pset(sx + 1, sy, mix(scr.px[i], c, 0.6));
                    }
                }
                PKind::Smoke => {
                    let r = (4.0 + (1.0 - k) * 6.0) as i32;
                    blend_ellipse(scr, sx - scr.shake.0, sy - scr.shake.1, r, r * 2 / 3, rgb(0x201c18), 0.3 * k);
                }
                PKind::Bone => {
                    scr.fill(sx, sy, 2, 1, rgb(0xb0a888));
                }
                PKind::Blood => {
                    scr.fill(sx, sy, 1, 1, rgb(0x801010));
                }
            }
        }
        scr.shake = (0, 0);

        // Health bars over wounded, chasing monsters.
        for m in &self.mobs {
            if matches!(m.state, MobState::Dead(_)) || m.hp >= m.max_hp {
                continue;
            }
            let (sx, sy) = to_scr(m.x, m.y);
            let h = self.art.char(m.kind.name()).height;
            let w = 22;
            let f = ((m.hp / m.max_hp) * w as f32).ceil() as i32;
            scr.fill(sx - w / 2 - 1, sy - h - 6, w + 2, 4, BLACK);
            scr.fill(sx - w / 2, sy - h - 5, f, 2, rgb(0xc02020));
        }
        for f in &self.floaters {
            let (sx, sy) = to_scr(f.x, f.y);
            let rise = (f.t * 30.0) as i32;
            let col = if f.t > 0.7 { mix(f.col, BLACK, (f.t - 0.7) / 0.3) } else { f.col };
            scr.text(&f.text, sx, sy - 58 - rise, col, Align::Center, 1);
        }

        if self.show_map {
            self.draw_map(scr);
        }
        self.draw_hud(scr);
    }

    /// D2-style automap overlay: explored walls projected isometrically, centred on the player.
    fn draw_map(&self, scr: &mut Screen) {
        let (cx, cy) = (scr.w / 2, (self.view_h - HUD_H) / 2);
        let (px, py) = (self.p.x, self.p.y);
        let proj = |x: f32, y: f32| -> (i32, i32) { (cx + ((x - px) - (y - py)) as i32 * 3, cy + ((x - px) + (y - py)) as i32 * 3 / 2) };
        let wall = rgb(0xc8b088);
        let floor = rgb(0x3a3024);
        for ty in 0..self.d.h {
            for tx in 0..self.d.w {
                let i = (ty * self.d.w + tx) as usize;
                if !self.explored[i] {
                    continue;
                }
                let (sx, sy) = proj(tx as f32, ty as f32);
                if sy >= self.view_h - HUD_H {
                    continue;
                }
                match self.d.get(tx, ty) {
                    Tile::Wall => scr.fill(sx, sy, 3, 2, wall),
                    Tile::Floor => {
                        let j = (sy * scr.w + sx) as usize;
                        if sx >= 0 && sy >= 0 && sx < scr.w && j < scr.px.len() {
                            scr.px[j] = mix(scr.px[j], floor, 0.8);
                        }
                    }
                    Tile::Void => {}
                }
            }
        }
        // Foes you've seen the room of (and every foe once only a few remain).
        let few = self.alive_mobs() <= 5;
        for m in &self.mobs {
            if matches!(m.state, MobState::Dead(_)) {
                continue;
            }
            let seen = self.explored[(m.y as i32 * self.d.w + m.x as i32) as usize];
            if seen || few {
                let (sx, sy) = proj(m.x, m.y);
                scr.fill(sx - 1, sy - 1, 3, 3, rgb(0xe02020));
            }
        }
        let (sx, sy) = proj(px, py);
        scr.fill(sx - 1, sy - 2, 3, 4, WHITE);
        scr.text("MAP", scr.w - 30, 8, rgb(0xc8b088), Align::Center, 1);
    }

    fn draw_player(&self, scr: &mut Screen, (sx, sy): (i32, i32)) {
        let art = self.art.char("mage");
        blend_ellipse(scr, sx, sy, 11, 4, BLACK, 0.5);
        let dead_t = if let State::Dead(t) = self.state { Some(t) } else { None };
        let spr = if self.p.cast_t > 0.0 && art.has("cast") {
            art.frame_at("cast", self.p.dir, 1.0 - self.p.cast_t / CAST_TIME)
        } else if self.p.moving {
            art.frame("walk", self.p.dir, self.p.anim_t)
        } else {
            art.frame("idle", self.p.dir, 0.0)
        };
        let mut fx = Fx::default();
        if self.p.flash > 0.0 {
            fx.tint = rgb(0xff2020);
            fx.tint_a = 0.5;
        }
        if let Some(t) = dead_t {
            fx.tint = rgb(0x400000);
            fx.tint_a = (t * 0.8).min(0.7);
            fx.cut = (spr.h as f32 * (1.0 - (t * 0.6).min(0.6))) as i32;
            scr.blit(spr, sx, sy + (t.min(1.0) * 14.0) as i32, fx);
            return;
        }
        scr.blit(spr, sx, sy, fx);
    }

    fn draw_mob(&self, scr: &mut Screen, i: usize, (sx, sy): (i32, i32)) {
        let m = &self.mobs[i];
        let art = self.art.char(m.kind.name());
        let mut fx = Fx::default();
        let spr = match m.state {
            MobState::Dead(t) => {
                if art.has("death") {
                    let s = art.frame_at("death", m.dir, (t / 0.7).min(0.999));
                    if t > 6.0 {
                        fx.alpha = (1.0 - (t - 6.0)).max(0.05);
                        if t > 7.0 {
                            return;
                        }
                    }
                    s
                } else {
                    // Collapse: flash, then sink into the floor and fade.
                    if t > 1.6 {
                        return;
                    }
                    fx.tint = if m.kind == Kind::Zombie { rgb(0x300808) } else { rgb(0x202020) };
                    fx.tint_a = (t * 1.5).min(0.8);
                    let s = art.frame("idle", m.dir, 0.0);
                    fx.cut = (s.ay as f32 - (t / 1.6) * s.ay as f32 * 0.9) as i32;
                    let sink = (t / 1.6 * art.height as f32 * 0.9) as i32;
                    scr.blit(s, sx, sy + sink, fx);
                    return;
                }
            }
            MobState::Attack(t) => art.frame_at("attack", m.dir, 1.0 - t / m.windup),
            _ if m.moving => art.frame("walk", m.dir, m.anim_t),
            _ => art.frame("idle", m.dir, 0.0),
        };
        if !matches!(m.state, MobState::Dead(_)) {
            blend_ellipse(scr, sx, sy, 10, 4, BLACK, 0.45);
        }
        if m.flash > 0.0 {
            fx.tint = WHITE;
            fx.tint_a = 0.7;
        } else if m.burn > 0.0 && (self.tick / 4) % 2 == 0 {
            fx.tint = rgb(0xff6020);
            fx.tint_a = 0.25;
        }
        if self.hover == Some(i) && fx.tint_a == 0.0 {
            fx.tint = rgb(0xffe0a0);
            fx.tint_a = 0.15;
        }
        scr.blit(spr, sx, sy, fx);
    }

    fn draw_hud(&self, scr: &mut Screen) {
        let (w, h) = (scr.w, self.view_h);
        let top = h - HUD_H;
        // Stone panel.
        for y in top..h {
            let k = (y - top) as f32 / HUD_H as f32;
            scr.fill(0, y, w, 1, mix(rgb(0x2a2520), rgb(0x141210), k));
        }
        scr.fill(0, top, w, 1, rgb(0x5a4a38));
        scr.fill(0, top + 1, w, 1, rgb(0x0a0806));
        // Globes.
        let gy = h - 28;
        globe(scr, 34, gy, 26, self.p.hp / self.p.max_hp, rgb(0xb01818), rgb(0xff6050));
        globe(scr, w - 34, gy, 26, self.p.mana / self.p.max_mana, rgb(0x1830b0), rgb(0x6090ff));
        scr.text(&format!("{}/{}", self.p.hp.ceil() as i32, self.p.max_hp as i32), 34, gy - 4, WHITE, Align::Center, 1);
        scr.text(&format!("{}/{}", self.p.mana.floor() as i32, self.p.max_mana as i32), w - 34, gy - 4, WHITE, Align::Center, 1);
        // Skill slot.
        let ix = w / 2 - 12;
        let iy = top + 8;
        scr.fill(ix - 2, iy - 2, 28, 28, rgb(0x5a4a38));
        scr.blit(&self.icon, ix, iy, Fx::default());
        if self.p.mana < FIREBALL_COST {
            scr.blend(ix, iy, 24, 24, rgb(0x000040), 0.6);
        }
        scr.text("FIREBALL", w / 2, iy + 28, rgb(0xd8b878), Align::Center, 1);
        // Potions.
        let bx = 80;
        potion(scr, bx, top + 12, rgb(0xc02020));
        scr.text(&format!("X{}", self.p.hp_pots), bx + 14, top + 16, WHITE, Align::Left, 1);
        scr.text("Q", bx + 2, top + 32, rgb(0x908070), Align::Left, 1);
        potion(scr, bx + 44, top + 12, rgb(0x2040c0));
        scr.text(&format!("X{}", self.p.mp_pots), bx + 58, top + 16, WHITE, Align::Left, 1);
        scr.text("E", bx + 46, top + 32, rgb(0x908070), Align::Left, 1);
        scr.text(&format!("GOLD {}", self.p.gold), w - 80, top + 12, rgb(0xe8c050), Align::Right, 1);
        scr.text(&format!("LEVEL {}", self.depth), w - 80, top + 24, rgb(0xb0a090), Align::Right, 1);
        scr.text(&format!("FOES {}", self.alive_mobs()), w - 80, top + 36, rgb(0xb0a090), Align::Right, 1);

        // Monster name + health bar (D2 style, top centre).
        if let Some(i) = self.hover.or(self.focus) {
            let m = &self.mobs[i];
            if !matches!(m.state, MobState::Dead(_)) {
                let bw = 150;
                let f = (m.hp / m.max_hp * bw as f32) as i32;
                scr.fill(w / 2 - bw / 2 - 1, 5, bw + 2, 14, BLACK);
                scr.fill(w / 2 - bw / 2, 6, f, 12, rgb(0x801010));
                scr.text(m.kind.label(), w / 2, 8, WHITE, Align::Center, 1);
            }
        }

        if self.banner_t > 0.0 {
            let a = (self.banner_t.min(1.0)).clamp(0.0, 1.0);
            let c = mix(BLACK, rgb(0xd8a048), a);
            let title = if self.depth == 1 { "ASHEN SANCTUM" } else { "YOU DESCEND DEEPER" };
            scr.text(title, w / 2, top / 2 - 60, c, Align::Center, 3);
            scr.text(&format!("SANCTUM LEVEL {}", self.depth), w / 2, top / 2 - 30, mix(BLACK, rgb(0xb0a090), a), Align::Center, 1);
            if self.depth == 1 {
                let hint = mix(BLACK, rgb(0x8a7a68), a);
                scr.text("LEFT CLICK: MOVE / ATTACK    RIGHT CLICK: FIREBALL    Q / E: POTIONS", w / 2, top - 34, hint, Align::Center, 1);
                scr.text("PAD: LEFT STICK MOVE    RIGHT STICK OR A: FIREBALL    L1 / Y: POTIONS", w / 2, top - 22, hint, Align::Center, 1);
            }
        }
        match self.state {
            State::Dead(t) => {
                scr.blend(0, 0, w, top, BLACK, (t * 0.4).min(0.5));
                scr.text("YOU HAVE DIED", w / 2, top / 2 - 20, rgb(0xc02020), Align::Center, 3);
                if t > 1.5 {
                    scr.text("PRESS ENTER OR START TO RISE AGAIN", w / 2, top / 2 + 14, rgb(0xb0a090), Align::Center, 1);
                }
            }
            State::Cleared => {
                scr.text("THE LEVEL IS CLEANSED", w / 2, 30, rgb(0xd8a048), Align::Center, 2);
                scr.text("PRESS ENTER OR START TO DESCEND", w / 2, 52, rgb(0xb0a090), Align::Center, 1);
            }
            State::Playing => {}
        }
    }
}

impl Mob {
    fn new(kind: Kind, x: f32, y: f32, scale: f32, rng: &mut Rng) -> Self {
        let (hp, speed, dmg, windup, cooldown) = match kind {
            Kind::Zombie => (34.0, 1.25, (5, 9), 0.55, 1.5),
            Kind::Skeleton => (20.0, 2.3, (3, 6), 0.35, 1.0),
        };
        let hp = hp * scale;
        let dmg = ((dmg.0 as f32 * scale) as i32, (dmg.1 as f32 * scale) as i32);
        Mob {
            kind,
            x,
            y,
            hp,
            max_hp: hp,
            speed: speed * rng.rf(0.9, 1.1),
            dmg,
            windup,
            cooldown,
            cd: 0.0,
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
        }
    }
}

/// World-space unit vector for a sprite direction.
fn dir_vec(dir: usize) -> (f32, f32) {
    // Screen-space direction for each index, then back to world.
    let a = (90.0 - dir as f32 * 45.0).to_radians();
    iso::screen_dir_to_world(a.cos(), a.sin())
}

/// Moves a circle with wall sliding (x then y).
fn move_circle(d: &Dungeon, x: &mut f32, y: &mut f32, dx: f32, dy: f32, r: f32) {
    let steps = ((dx.abs().max(dy.abs())) / 0.2).ceil().max(1.0) as i32;
    let (sx, sy) = (dx / steps as f32, dy / steps as f32);
    for _ in 0..steps {
        if !d.blocked(*x + sx, *y, r) {
            *x += sx;
        }
        if !d.blocked(*x, *y + sy, r) {
            *y += sy;
        }
    }
}

fn blend_ellipse(scr: &mut Screen, cx: i32, cy: i32, rx: i32, ry: i32, c: u32, a: f32) {
    let (cx, cy) = (cx + scr.shake.0, cy + scr.shake.1);
    for y in -ry..=ry {
        for x in -rx..=rx {
            let d = (x as f32 / rx as f32).powi(2) + (y as f32 / ry.max(1) as f32).powi(2);
            if d <= 1.0 {
                let (px, py) = (cx + x, cy + y);
                if px >= 0 && py >= 0 && px < scr.w && py < scr.h {
                    let i = (py * scr.w + px) as usize;
                    scr.px[i] = mix(scr.px[i], c, a * (1.0 - d * 0.5));
                }
            }
        }
    }
}

fn globe(scr: &mut Screen, cx: i32, cy: i32, r: i32, frac: f32, col: u32, hi: u32) {
    let level = cy + r - (2.0 * r as f32 * frac.clamp(0.0, 1.0)) as i32;
    scr.disc(cx, cy, r + 2, rgb(0x5a4a38));
    scr.disc(cx, cy, r + 1, BLACK);
    for y in -r..=r {
        for x in -r..=r {
            if x * x + y * y > r * r {
                continue;
            }
            let (px, py) = (cx + x, cy + y);
            let c = if py >= level {
                let shade = 1.0 - ((x + r / 3) as f32).hypot((y + r / 3) as f32) / (r as f32 * 1.6);
                mix(mix(col, BLACK, 0.5), hi, shade.clamp(0.0, 1.0) * 0.6)
            } else {
                rgb(0x0c0a0a)
            };
            scr.pset(px, py, c);
        }
    }
    // Glass highlight.
    scr.disc(cx - r / 3, cy - r / 2, 3, mix(rgb(0xffffff), col, 0.5));
}

fn potion(scr: &mut Screen, x: i32, y: i32, col: u32) {
    scr.fill(x + 3, y, 4, 3, rgb(0x806040));
    scr.disc(x + 5, y + 9, 5, BLACK);
    scr.disc(x + 5, y + 9, 4, col);
    scr.pset(x + 3, y + 7, rgb(0xffffff));
}

fn draw_pickup(scr: &mut Screen, k: &Pickup, sx: i32, sy: i32, tick: u32) {
    let bob = (((tick as f32) * 0.1 + k.x).sin() * 1.5) as i32;
    let pop = if k.t < 0.3 { ((0.3 - k.t) * 40.0) as i32 } else { 0 };
    blend_ellipse(scr, sx, sy, 5, 2, BLACK, 0.5);
    match k.kind {
        Drop::Health => potion(scr, sx - 5, sy - 16 - pop + bob, rgb(0xc02020)),
        Drop::Mana => potion(scr, sx - 5, sy - 16 - pop + bob, rgb(0x2040c0)),
        Drop::Gold(_) => {
            for (dx, dy) in [(-3, 0), (2, -1), (0, -3), (-1, 1)] {
                scr.fill(sx + dx, sy - 3 + dy - pop, 3, 2, rgb(0xe8c050));
                scr.pset(sx + dx, sy - 3 + dy - pop, rgb(0xfff0a0));
            }
        }
    }
}

//! Random side quests ("errands"): every outdoor area rolls a task of its own, different each playthrough (each
//! hero's world, each difficulty). They start when you first walk into the area and pay out on the spot.
//!
//! The kinds: a **bounty** on a named champion, a **cull** of one kind of monster, a **captive** to free from
//! their guards, **herbs** to gather, a **lost heirloom** on a corpse, a **bone totem** raising the dead, a
//! **ward stone** to hold against three waves, and a **treasure map** with an X to dig at. A third of them
//! roll a twist: timed, or double gold.
use crate::game::{Decal, Drop, Game, PKind, Pickup, Sfx, DT};
use crate::gfx::rgb;
use crate::mobs::{Kind, Mob, MobState, Rank};
use crate::rng::Rng;
use crate::story::{Npc, Role};
use crate::world::LevelId;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ErrandKind {
    Bounty,
    Cull,
    Captive,
    Herbs,
    Heirloom,
    Totem,
    Siege,
    Treasure,
}

pub const KINDS: [ErrandKind; 8] = [
    ErrandKind::Bounty,
    ErrandKind::Cull,
    ErrandKind::Captive,
    ErrandKind::Herbs,
    ErrandKind::Heirloom,
    ErrandKind::Totem,
    ErrandKind::Siege,
    ErrandKind::Treasure,
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Twist {
    None,
    /// Done within this many seconds of starting, or it fails.
    Timed(u32),
    /// Twice the gold.
    Greedy,
}

/// Mob tags (`Mob::errand`): the target itself, its guards, a siege wave.
pub const TAG_TARGET: u8 = 1;
pub const TAG_GUARD: u8 = 2;
pub const TAG_WAVE: u8 = 3;

/// Open, done (paid), failed.
pub const E_OPEN: u8 = 0;
pub const E_DONE: u8 = 1;
pub const E_FAILED: u8 = 2;

#[derive(Clone, Debug)]
pub struct Errand {
    pub level: LevelId,
    pub kind: ErrandKind,
    pub twist: Twist,
    pub state: u8,
    /// Started (you've been to the area) and its things put in place.
    pub placed: bool,
    pub name: String,
    /// The monster kind it's about (cull, bounty), the count needed and got (cull, herbs, siege waves).
    pub mob: Kind,
    pub need: u32,
    pub got: u32,
    /// Where it happens (the captive, the heirloom, the totem, the ward stone, the X).
    pub spot: (f32, f32),
    /// Seconds since it started (for timed ones and siege waves).
    pub t: f32,
    pub wave_t: f32,
}

impl Errand {
    pub fn title(&self) -> String {
        let twist = match self.twist {
            Twist::Timed(s) => {
                let left = (s as f32 - self.t).max(0.0) as u32;
                format!("  ({}:{:02} LEFT)", left / 60, left % 60)
            }
            Twist::Greedy => "  (DOUBLE GOLD)".into(),
            Twist::None => String::new(),
        };
        let what = match self.kind {
            ErrandKind::Bounty => format!("BOUNTY: SLAY {}", self.name),
            ErrandKind::Cull => format!("CULL {} {} ({}/{})", self.need, plural(self.mob), self.got.min(self.need), self.need),
            ErrandKind::Captive => format!("FREE {} FROM THE MONSTERS", self.name),
            ErrandKind::Herbs => format!("GATHER {} ({}/{})", self.name, self.got, self.need),
            ErrandKind::Heirloom => format!("FIND {}", self.name),
            ErrandKind::Totem => "BREAK THE BONE TOTEM".into(),
            ErrandKind::Siege => {
                if self.got == 0 {
                    "HOLD THE WARD STONE (STAND ON IT)".into()
                } else {
                    format!("HOLD THE WARD STONE (WAVE {}/3)", self.got.min(3))
                }
            }
            ErrandKind::Treasure => {
                if self.got == 0 {
                    "A TREASURE MAP LIES SOMEWHERE NEAR THE ROAD".into()
                } else {
                    "DIG AT THE X ON YOUR MAP".into()
                }
            }
        };
        if self.kind == ErrandKind::Cull || self.kind == ErrandKind::Herbs || self.state != E_OPEN {
            return what;
        }
        what + &twist
    }

    /// What you're told when it starts.
    fn intro(&self) -> String {
        match self.kind {
            ErrandKind::Bounty => format!("A BOUNTY: {} PREYS ON TRAVELLERS HERE", self.name),
            ErrandKind::Cull => format!("TOO MANY {} IN THESE PARTS. CULL {} OF THEM", plural(self.mob), self.need),
            ErrandKind::Captive => format!("SOMEONE IS CRYING FOR HELP: {} IS HELD BY MONSTERS", self.name),
            ErrandKind::Herbs => format!("{} GROWS HERE. GATHER {} FOR THE HEALERS", self.name, self.need),
            ErrandKind::Heirloom => format!("A TRAVELLER DIED HERE WITH {}", self.name),
            ErrandKind::Totem => "A BONE TOTEM CALLS THE DEAD OUT OF THE GROUND HERE. BREAK IT".into(),
            ErrandKind::Siege => "A WARD STONE STANDS HERE. HOLD IT AND IT WILL BLESS YOU".into(),
            ErrandKind::Treasure => "THERE'S TALK OF BURIED TREASURE HERE. FIND THE MAP".into(),
        }
    }
}

fn plural(k: Kind) -> String {
    let l = crate::mobs::def(k).label;
    let l = l.strip_prefix("THE ").unwrap_or(l);
    if l.ends_with('S') {
        l.into()
    } else if l.ends_with('Y') && !l.ends_with("EY") {
        format!("{}IES", &l[..l.len() - 1])
    } else if l.ends_with("WOLF") {
        format!("{}VES", &l[..l.len() - 1])
    } else {
        format!("{l}S")
    }
}

const BOUNTY_A: &[&str] = &["GRIMSCALE", "OLD SNAGGLE", "BLACKTOOTH", "RUSTJAW", "MOTHER MAW", "SKINFLAY", "ASHEYE", "HOLLOWBACK", "THREE-FINGERS", "GUTSPILL"];
const BOUNTY_B: &[&str] = &["THE BUTCHER", "THE CRUEL", "THE GLUTTON", "THE GRAVE-ROBBER", "THE SLY", "THE UNKIND", "THE STALKER", "THE RAVENOUS"];
const CAPTIVES: &[&str] = &["A TRAPPER", "A PILGRIM", "A MERCHANT'S BOY", "A WOODCUTTER", "A LOST SCOUT", "A TINKER", "A SHEPHERDESS"];
const HERBS: &[&str] = &["EMBERROOT", "GHOSTCAP", "BLOODTHISTLE", "ASHWEED", "WARDLEAF", "STARMOSS"];
const HEIRLOOMS: &[&str] = &["A SIGNET RING", "A SILVER LOCKET", "A FAMILY SWORD-HILT", "A WEDDING BAND", "AN OLD MEDAL", "A CARVED PIPE"];

/// Which errands an act has: one per area, or two on an act that's still one big overland.
fn levels_of(act: usize) -> Vec<LevelId> {
    let areas: Vec<LevelId> = crate::areas::AREAS.iter().filter(|a| a.act as usize == act).map(|a| LevelId::Area(a.act, a.n)).collect();
    if areas.is_empty() {
        vec![LevelId::land(act), LevelId::land(act)]
    } else {
        areas
    }
}

/// Rolls an act's errands for this world and difficulty (no two alike until all eight have come up).
pub fn roll_act(act: usize, world_seed: u64, difficulty: u8) -> Vec<Errand> {
    let mut rng = Rng::new(world_seed ^ 0xE77A_4D00 ^ (difficulty as u64 * 7919 + act as u64 * 131));
    let mut kinds = KINDS.to_vec();
    for i in (1..kinds.len()).rev() {
        kinds.swap(i, rng.range(0, i as i32 + 1) as usize);
    }
    levels_of(act)
        .into_iter()
        .enumerate()
        .map(|(i, level)| {
            let kind = kinds[i % kinds.len()];
            let r = rng.f();
            let twist = if r < 0.17 && matches!(kind, ErrandKind::Bounty | ErrandKind::Cull | ErrandKind::Herbs | ErrandKind::Totem) {
                Twist::Timed(rng.range(4, 7) as u32 * 60)
            } else if r < 0.34 {
                Twist::Greedy
            } else {
                Twist::None
            };
            let pick = |rng: &mut Rng, list: &[&str]| list[rng.range(0, list.len() as i32) as usize].to_string();
            let name = match kind {
                ErrandKind::Bounty => format!("{} {}", pick(&mut rng, BOUNTY_A), pick(&mut rng, BOUNTY_B)),
                ErrandKind::Captive => pick(&mut rng, CAPTIVES),
                ErrandKind::Herbs => pick(&mut rng, HERBS),
                ErrandKind::Heirloom => pick(&mut rng, HEIRLOOMS),
                _ => String::new(),
            };
            Errand { level, kind, twist, state: E_OPEN, placed: false, name, mob: Kind::Zombie, need: 0, got: 0, spot: (0.0, 0.0), t: 0.0, wave_t: 0.0 }
        })
        .collect()
}

impl Game {
    /// This level's errand (the first open one, or the first).
    fn errand_here(&self) -> Option<usize> {
        let mine: Vec<usize> = (0..self.errands.len()).filter(|&i| self.errands[i].level == self.level).collect();
        mine.iter().copied().find(|&i| self.errands[i].state == E_OPEN).or(mine.first().copied())
    }

    /// The quest-log line of this level's errand.
    pub fn errand_log(&self) -> Option<(String, u32)> {
        let i = self.errand_here()?;
        let e = &self.errands[i];
        if !e.placed {
            return None;
        }
        Some(match e.state {
            E_OPEN => (e.title(), rgb(0xa8b8d8)),
            E_DONE => (format!("{}: DONE", e.title()), rgb(0x608060)),
            _ => (format!("{}: FAILED", e.title()), rgb(0x806050)),
        })
    }

    /// For the journal: this act's errands you've come across.
    pub fn errand_journal(&self) -> Vec<(String, u32)> {
        let act = self.level.act();
        self.errands
            .iter()
            .filter(|e| e.level.act() == act && e.placed)
            .map(|e| {
                let state = match e.state {
                    E_OPEN => ("", 0xa8b8d8),
                    E_DONE => ("  DONE", 0x80a080),
                    _ => ("  FAILED", 0x806050),
                };
                (format!("  {}{}", e.title(), state.0), rgb(state.1))
            })
            .collect()
    }

    /// The X on the map (a treasure errand with its map found).
    pub fn errand_x(&self) -> Option<(f32, f32)> {
        let i = self.errand_here()?;
        let e = &self.errands[i];
        (e.kind == ErrandKind::Treasure && e.state == E_OPEN && e.got == 1).then_some(e.spot)
    }

    /// The ward stone or the X (drawn on the ground).
    pub fn errand_marker(&self) -> Option<(ErrandKind, (f32, f32), bool)> {
        let i = self.errand_here()?;
        let e = &self.errands[i];
        match e.kind {
            ErrandKind::Siege if e.placed => Some((e.kind, e.spot, e.state == E_OPEN)),
            ErrandKind::Treasure if e.placed && e.got == 1 && e.state == E_OPEN => Some((e.kind, e.spot, true)),
            _ => None,
        }
    }

    /// Entering a level: roll the act's errands the first time, and set this level's in motion.
    pub(crate) fn errand_enter(&mut self) {
        if !self.level.overland() || self.level == LevelId::Overworld && crate::areas::AREAS.iter().any(|a| a.act == 0) {
            return;
        }
        let act = self.level.act();
        if !self.errands.iter().any(|e| e.level.act() == act) {
            let mut fresh = roll_act(act, self.world_seed, self.quest.difficulty);
            for e in fresh.iter_mut() {
                if self.errands_done.contains(&(e.level, e.kind)) {
                    e.state = E_DONE;
                }
            }
            self.errands.extend(fresh);
        }
        let here: Vec<usize> = (0..self.errands.len()).filter(|&i| self.errands[i].level == self.level && !self.errands[i].placed).collect();
        for (n, i) in here.into_iter().enumerate() {
            if n > 0 && self.errands.iter().filter(|e| e.level == self.level && e.placed && e.state == E_OPEN).count() > 0 {
                // A second errand on the same overland waits for the first.
                break;
            }
            self.place_errand(i);
        }
    }

    /// Places an errand now (snapshots).
    pub fn debug_place_errand(&mut self, i: usize) {
        self.errands[i].placed = false;
        self.place_errand(i);
    }

    /// A random open spot far from where you came in, that you can walk to.
    fn errand_spot(&mut self, min: f32) -> Option<(f32, f32)> {
        let (sx, sy) = (self.p.x, self.p.y);
        for _ in 0..300 {
            let x = self.rng.range(4, self.d.w - 4) as f32 + 0.5;
            let y = self.rng.range(4, self.d.h - 4) as f32 + 0.5;
            if ((x - sx).powi(2) + (y - sy).powi(2)).sqrt() < min || self.d.blocked(x, y, 0.8) || self.in_safe(x, y) {
                continue;
            }
            if self.portals.iter().any(|p| (p.x - x).powi(2) + (p.y - y).powi(2) < 25.0) {
                continue;
            }
            if self.d.path((sx as i32, sy as i32), (x as i32, y as i32), 30_000).is_some() {
                return Some((x, y));
            }
        }
        None
    }

    fn spawn_tagged(&mut self, kind: Kind, (x, y): (f32, f32), n: usize, tag: u8, champion: bool) {
        for k in 0..n {
            let a = k as f32 / n.max(1) as f32 * std::f32::consts::TAU;
            let (mx, my) = (x + a.cos() * 1.5, y + a.sin() * 1.5);
            let (mx, my) = if self.d.blocked(mx, my, 0.35) { (x, y) } else { (mx, my) };
            let mut m = Mob::new(kind, mx, my, self.tier, &mut self.rng);
            if champion && k == 0 {
                let mods = crate::mobs::roll_mods(1, &mut self.rng);
                m.promote(Rank::Champion, mods, None);
            }
            m.errand = tag;
            self.mobs.push(m);
        }
    }

    fn place_errand(&mut self, i: usize) {
        if self.errands[i].state != E_OPEN {
            self.errands[i].placed = true;
            return;
        }
        let kinds = self.local_kinds();
        let common = kinds.first().copied().unwrap_or(Kind::Zombie);
        let pick_kind = |g: &mut Game| if kinds.is_empty() { Kind::Zombie } else { kinds[g.rng.range(0, kinds.len() as i32) as usize] };
        let Some(spot) = self.errand_spot(22.0).or_else(|| self.errand_spot(10.0)) else { return };
        let kind = self.errands[i].kind;
        // (A name for the kinds that need one, if it was rolled as another kind.)
        if self.errands[i].name.is_empty() {
            let list: &[&str] = match kind {
                ErrandKind::Captive => CAPTIVES,
                ErrandKind::Herbs => HERBS,
                ErrandKind::Heirloom => HEIRLOOMS,
                _ => BOUNTY_A,
            };
            let mut name = list[self.rng.range(0, list.len() as i32) as usize].to_string();
            if kind == ErrandKind::Bounty {
                name = format!("{name} {}", BOUNTY_B[self.rng.range(0, BOUNTY_B.len() as i32) as usize]);
            }
            self.errands[i].name = name;
        }
        match kind {
            ErrandKind::Bounty => {
                let k = pick_kind(self);
                let mods = crate::mobs::roll_mods(2, &mut self.rng);
                let mut m = Mob::new(k, spot.0, spot.1, self.tier * 1.2, &mut self.rng);
                let name = self.errands[i].name.clone();
                m.promote(Rank::Elite, mods, Some(name));
                m.max_hp *= 1.3;
                m.hp = m.max_hp;
                m.errand = TAG_TARGET;
                self.mobs.push(m);
                self.spawn_tagged(k, spot, 3, TAG_GUARD, false);
                self.errands[i].mob = k;
            }
            ErrandKind::Cull => {
                let alive = self.mobs.iter().filter(|m| m.alive() && m.kind == common && m.charm <= 0.0).count() as u32;
                self.errands[i].mob = common;
                self.errands[i].need = alive.clamp(6, 15);
                if alive < 6 {
                    // Not enough of them about: a few more turn up.
                    self.spawn_tagged(common, spot, 6, 0, false);
                }
            }
            ErrandKind::Captive => {
                let name = CAPTIVES.iter().find(|c| **c == self.errands[i].name).copied().unwrap_or("A CAPTIVE");
                self.npcs.push(Npc::new(name, Role::Captive, "npc_villager", spot.0, spot.1, 2));
                let k = pick_kind(self);
                self.spawn_tagged(k, spot, 5, TAG_GUARD, true);
            }
            ErrandKind::Herbs => {
                self.errands[i].need = 5;
                let mut n = 0;
                for _ in 0..40 {
                    if n >= 5 {
                        break;
                    }
                    if let Some(s) = self.errand_spot(12.0) {
                        self.pickups.push(Pickup { x: s.0, y: s.1, kind: Drop::Herb, t: 1.0 });
                        n += 1;
                    }
                }
            }
            ErrandKind::Heirloom => {
                self.decals.push(Decal { x: spot.0, y: spot.1, r: 0.6, col: rgb(0x301810), a: 0.6 });
                self.pickups.push(Pickup { x: spot.0, y: spot.1, kind: Drop::Heirloom, t: 1.0 });
                let k = pick_kind(self);
                self.spawn_tagged(k, (spot.0 + 2.0, spot.1 + 1.0), 4, TAG_GUARD, true);
            }
            ErrandKind::Totem => {
                let mut m = Mob::new(Kind::Totem, spot.0, spot.1, self.tier, &mut self.rng);
                m.errand = TAG_TARGET;
                self.mobs.push(m);
                self.errands[i].mob = pick_kind(self);
            }
            ErrandKind::Siege => {
                self.errands[i].mob = pick_kind(self);
            }
            ErrandKind::Treasure => {
                // The map lies near the road in; the X is far off.
                let near = self.errand_spot(5.0).filter(|s| ((s.0 - self.p.x).powi(2) + (s.1 - self.p.y).powi(2)).sqrt() < 20.0).or_else(|| self.errand_spot(4.0));
                if let Some(m) = near {
                    self.pickups.push(Pickup { x: m.0, y: m.1, kind: Drop::Clue, t: 1.0 });
                }
            }
        }
        let e = &mut self.errands[i];
        e.spot = spot;
        e.placed = true;
        e.t = 0.0;
        let intro = e.intro();
        self.say(intro);
    }

    /// Every tick: timers, the captive's guards, the totem's dead, the siege waves, the X.
    pub(crate) fn update_errands(&mut self) {
        let Some(i) = self.errand_here() else { return };
        if !self.errands[i].placed || self.errands[i].state != E_OPEN {
            return;
        }
        self.errands[i].t += DT;
        if let Twist::Timed(s) = self.errands[i].twist {
            if self.errands[i].t > s as f32 {
                self.errands[i].state = E_FAILED;
                self.say(format!("TOO SLOW: {}", self.errands[i].title()));
                return;
            }
        }
        let (px, py) = (self.p.x, self.p.y);
        let spot = self.errands[i].spot;
        let near = ((spot.0 - px).powi(2) + (spot.1 - py).powi(2)).sqrt();
        match self.errands[i].kind {
            ErrandKind::Captive => {
                let guards = self.mobs.iter().filter(|m| m.errand == TAG_GUARD && m.alive()).count();
                if guards == 0 && near < 14.0 {
                    if let Some(n) = self.npcs.iter().position(|n| n.role == Role::Captive) {
                        let (x, y) = (self.npcs[n].x, self.npcs[n].y);
                        self.floater(x, y, "THANK YOU! I'LL TELL THEM IN TOWN!".into(), rgb(0x80ff80));
                        self.npcs.remove(n);
                    }
                    self.errand_done(i);
                }
            }
            ErrandKind::Totem => {
                // The totem raises the dead while you're near it.
                if let Some(t) = self.mobs.iter().position(|m| m.kind == Kind::Totem && m.alive()) {
                    self.mobs[t].special += DT;
                    let raised = self.mobs.iter().filter(|m| m.errand == TAG_GUARD && m.alive()).count();
                    if near < 14.0 && self.mobs[t].special > 6.0 && raised < 8 {
                        self.mobs[t].special = 0.0;
                        let k = self.errands[i].mob;
                        self.spawn_tagged(k, spot, 2, TAG_GUARD, false);
                        for m in self.mobs.iter_mut().filter(|m| m.errand == TAG_GUARD && m.state == MobState::Idle) {
                            m.state = MobState::Chase;
                        }
                        for _ in 0..10 {
                            self.spray_at(spot.0, spot.1, PKind::Bone, 14.0);
                        }
                    }
                }
            }
            ErrandKind::Siege => {
                let e = &self.errands[i];
                let wave_alive = self.mobs.iter().filter(|m| m.errand == TAG_WAVE && m.alive()).count();
                if e.got == 0 {
                    if near < 1.6 {
                        self.errands[i].got = 1;
                        self.errands[i].wave_t = 0.0;
                        self.siege_wave(i);
                        self.say("THE WARD STONE FLARES. HERE THEY COME!".into());
                    }
                } else if wave_alive == 0 {
                    self.errands[i].wave_t += DT;
                    if self.errands[i].wave_t > 2.0 {
                        if self.errands[i].got >= 3 {
                            self.errand_done(i);
                        } else {
                            self.errands[i].got += 1;
                            self.errands[i].wave_t = 0.0;
                            self.siege_wave(i);
                        }
                    }
                }
            }
            ErrandKind::Treasure => {
                if self.errands[i].got == 1 && near < 1.3 {
                    self.say("YOU DIG... AND HIT A BURIED CHEST!".into());
                    self.sfx.push(Sfx::Boom);
                    for _ in 0..14 {
                        self.spray_at(spot.0, spot.1, PKind::Smoke, 8.0);
                    }
                    self.errand_done(i);
                }
            }
            _ => {}
        }
    }

    fn siege_wave(&mut self, i: usize) {
        let (spot, kind, wave) = (self.errands[i].spot, self.errands[i].mob, self.errands[i].got);
        for k in 0..3 {
            let a = k as f32 * 2.1 + wave as f32;
            let (x, y) = (spot.0 + a.cos() * 8.0, spot.1 + a.sin() * 8.0);
            let at = if self.d.blocked(x, y, 0.5) { (spot.0 + a.cos() * 4.0, spot.1 + a.sin() * 4.0) } else { (x, y) };
            let at = if self.d.blocked(at.0, at.1, 0.5) { spot } else { at };
            self.spawn_tagged(kind, at, 2, TAG_WAVE, k == 0 && wave == 3);
        }
        for m in self.mobs.iter_mut().filter(|m| m.errand == TAG_WAVE) {
            m.state = MobState::Chase;
        }
        self.shake = self.shake.max(0.4);
    }

    /// A monster died: a bounty's target, a cull's count, the totem.
    pub(crate) fn errand_kill(&mut self, kind: Kind, tag: u8) {
        let Some(i) = self.errand_here() else { return };
        if !self.errands[i].placed || self.errands[i].state != E_OPEN {
            return;
        }
        match self.errands[i].kind {
            ErrandKind::Bounty | ErrandKind::Totem if tag == TAG_TARGET => self.errand_done(i),
            ErrandKind::Cull if kind == self.errands[i].mob => {
                self.errands[i].got += 1;
                if self.errands[i].got >= self.errands[i].need {
                    self.errand_done(i);
                }
            }
            _ => {}
        }
    }

    /// Picked up an errand's thing: an herb, the heirloom, the treasure map.
    pub(crate) fn errand_pick(&mut self, d: &Drop) {
        let Some(i) = self.errand_here() else { return };
        if self.errands[i].state != E_OPEN {
            return;
        }
        let (x, y) = (self.p.x, self.p.y);
        match d {
            Drop::Herb => {
                self.errands[i].got += 1;
                let (got, need, name) = (self.errands[i].got, self.errands[i].need, self.errands[i].name.clone());
                self.floater(x, y, format!("{name} {got}/{need}"), rgb(0x80f080));
                if got >= need {
                    self.errand_done(i);
                }
            }
            Drop::Heirloom => {
                let name = self.errands[i].name.clone();
                self.floater(x, y, name, rgb(0xe0c0ff));
                self.errand_done(i);
            }
            Drop::Clue => {
                self.errands[i].got = 1;
                self.say("A TREASURE MAP! AN X IS MARKED ON IT (SEE YOUR MAP)".into());
            }
            _ => {}
        }
    }

    /// An errand is done: it pays out where you stand.
    fn errand_done(&mut self, i: usize) {
        let e = &mut self.errands[i];
        if e.state != E_OPEN {
            return;
        }
        e.state = E_DONE;
        let (level, kind, twist) = (e.level, e.kind, e.twist);
        let title = e.title();
        self.errands_done.push((level, kind));
        self.save_due = true;
        let tier = self.tier;
        let gold = ((40.0 + 30.0 * tier) * if twist == Twist::Greedy { 2.0 } else { 1.0 }) as i32;
        let (x, y) = (self.p.x, self.p.y);
        let ilvl = crate::items::ilvl_for(tier) + 2;
        let rare = matches!(kind, ErrandKind::Bounty | ErrandKind::Siege | ErrandKind::Treasure) || self.rng.chance(0.3);
        let item = if rare { crate::items::roll(ilvl, crate::items::Rarity::Rare, &mut self.rng) } else { crate::items::roll(ilvl, crate::items::Rarity::Magic, &mut self.rng) };
        let loot = [Drop::Gold(gold), Drop::Item(Box::new(item)), if self.rng.chance(0.5) { Drop::Health } else { Drop::Mana }];
        for (k, d) in loot.into_iter().enumerate() {
            let a = k as f32 * 2.1;
            self.pickups.push(Pickup { x: x + a.cos() * 0.8, y: y + a.sin() * 0.8, kind: d, t: 0.0 });
        }
        self.gain_xp(60.0 * tier * (1.0 + self.quest.difficulty as f32));
        self.sfx.push(Sfx::Descend);
        for _ in 0..20 {
            self.spray_at(x, y, PKind::Magic, 18.0);
        }
        self.say(format!("TASK DONE: {title}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Game;

    #[test]
    fn errands_change_with_every_playthrough() {
        let a = roll_act(0, 7, 0);
        let b = roll_act(0, 8, 0);
        let c = roll_act(0, 7, 1);
        assert_eq!(a.len(), crate::areas::AREAS.iter().filter(|a| a.act == 0).count(), "one per area");
        let kinds = |v: &[Errand]| v.iter().map(|e| e.kind).collect::<Vec<_>>();
        assert!(kinds(&a) != kinds(&b) || kinds(&a) != kinds(&c), "a new hero or difficulty rolls new tasks");
        // Eight areas, eight different kinds.
        let mut k = kinds(&a);
        k.truncate(8);
        k.sort_by_key(|k| *k as u8);
        k.dedup();
        assert_eq!(k.len(), 8);
        assert_eq!(roll_act(5, 7, 0).len(), crate::areas::AREAS.iter().filter(|a| a.act == 5).count());
    }

    #[test]
    fn every_kind_of_errand_can_be_finished() {
        for kind in KINDS {
            let mut g = Game::new(7, 360);
            g.debug_goto(LevelId::Area(0, 2));
            let i = g.errand_here().expect("the area has an errand");
            // Force this kind and place it afresh.
            g.errands[i].kind = kind;
            g.errands[i].twist = Twist::None;
            g.errands[i].state = E_OPEN;
            g.errands[i].got = 0;
            g.mobs.retain(|m| m.errand == 0);
            g.npcs.retain(|n| n.role != Role::Captive);
            g.place_errand(i);
            assert!(g.errands[i].placed, "{kind:?} placed");
            let gold = g.pickups.len();
            match kind {
                ErrandKind::Bounty | ErrandKind::Totem => {
                    let t = g.mobs.iter().position(|m| m.errand == TAG_TARGET).unwrap();
                    g.kill(t);
                }
                ErrandKind::Cull => {
                    let k = g.errands[i].mob;
                    for _ in 0..g.errands[i].need {
                        g.errand_kill(k, 0);
                    }
                }
                ErrandKind::Captive => {
                    for m in g.mobs.iter_mut().filter(|m| m.errand == TAG_GUARD) {
                        m.state = MobState::Dead(0.0);
                    }
                    (g.p.x, g.p.y) = g.errands[i].spot;
                    g.update_errands();
                }
                ErrandKind::Herbs => {
                    for _ in 0..5 {
                        g.errand_pick(&Drop::Herb);
                    }
                }
                ErrandKind::Heirloom => g.errand_pick(&Drop::Heirloom),
                ErrandKind::Siege => {
                    (g.p.x, g.p.y) = g.errands[i].spot;
                    for _ in 0..4 {
                        g.update_errands();
                        for m in g.mobs.iter_mut().filter(|m| m.errand == TAG_WAVE) {
                            m.state = MobState::Dead(0.0);
                        }
                        for _ in 0..150 {
                            g.update_errands();
                        }
                    }
                }
                ErrandKind::Treasure => {
                    g.errand_pick(&Drop::Clue);
                    assert!(g.errand_x().is_some(), "the X is on the map");
                    (g.p.x, g.p.y) = g.errands[i].spot;
                    g.update_errands();
                }
            }
            assert_eq!(g.errands[i].state, E_DONE, "{kind:?} done");
            assert!(g.pickups.len() > gold, "{kind:?} pays out");
        }
    }
}

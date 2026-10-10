//! Act 7's side content (docs/ACT7_PLAN.md, stage 4), the user's picks:
//!
//! - **The Probability Vault** (off the Drift of Worlds): every coffer is a gamble (gold, a rare, now and then a
//!   unique, or a mimic). Its keeper, **the Dice-Saint**, rolls a die for every attack.
//! - **Chaos Roulette** (the Elemental Tangle): a shrine that spins a blessing for three minutes, or a curse, or
//!   once in a while a jackpot. It can be spun again after a minute.
//! - **The Wandering Room**: a pocket of some other world that drifts from area to area across the Churn. Reach
//!   it before it flickers away (once a minute) and its treasure is yours, once per difficulty.
//! - **Gravity Wells** (the Drift of Worlds and the Shattered Monastery): down points somewhere else near them.
//!   They pull at you, and bend every bolt, arrow, axe and spear that flies past.
//! - **Echo Hunt**: one earlier act's last boss has left an echo somewhere in the Churn (a different one, in a
//!   different area, every playthrough). It fights like the original and drops the original's unique.
//!
//! Also here: Abbot Quiet's three lost words and Sister Ferro's chaos ore (side.rs quests).
use crate::game::{Drop, Game, PKind, Pickup, Sfx, DT};
use crate::gfx::rgb;
use crate::mobs::{Hazard, HazardKind, Kind, Mob, MobState, Rank, Shot, ShotKind};
use crate::rng::Rng;
use crate::side::{Blessing, Goal, S_GIVEN};
use crate::story::Dialog;
use crate::world::{LevelId, Prop, PropKind};

pub const DRIFT: LevelId = LevelId::Area(6, 2);
pub const TANGLE: LevelId = LevelId::Area(6, 3);
pub const MONASTERY: LevelId = LevelId::Area(6, 5);
/// How far a gravity well reaches (tiles), and how hard it pulls at things flying past (tiles/s²).
pub const WELL_R: f32 = 5.0;
pub const WELL_PULL: f32 = 14.0;
/// Seconds between spins of the roulette; how long a roulette blessing lasts.
pub const ROULETTE_CD: f32 = 60.0;
pub const ROULETTE_TIME: f32 = 180.0;
/// Seconds before the Wandering Room drifts on (while you're anywhere in the Churn), and how close is caught.
pub const WANDER_GAP: f32 = 60.0;
pub const WANDER_R: f32 = 2.2;
/// The old act bosses an echo can be of.
pub const ECHOES: [Kind; 6] = [Kind::AshKing, Kind::WhiteDragon, Kind::Vardak, Kind::Clockmaker, Kind::Leviathan, Kind::Solanthos];

/// Abbot Quiet's three words, by index (side.rs quest "THE ABBOT'S LOST VOICE").
pub const WORDS: [(&str, &str); 3] = [
    ("A WORD: \"STILL\"", "A SMOOTH GREY PEBBLE, AND WHEN YOU HOLD IT YOU HEAR THE ABBOT'S VOICE, YOUNGER, SAY ONE WORD: STILL."),
    ("A WORD: \"HOLD\"", "A KNOT OF OLD ROPE. THE ABBOT'S VOICE, FROM SOMEWHERE INSIDE IT: HOLD."),
    ("A WORD: \"HOME\"", "A CHIPPED BOWL FROM STILLHOLD'S KITCHEN. TAP IT AND IT RINGS: HOME."),
];

/// Where things lie this playthrough: the three words' areas, the echo's area and boss.
pub fn rolls(world_seed: u64, difficulty: u8) -> ([u8; 3], u8, Kind) {
    let mut rng = Rng::new(world_seed ^ 0xC4A05 ^ difficulty as u64 * 7_919);
    let mut a = vec![1u8, 2, 3, 4, 5, 6];
    for i in (1..a.len()).rev() {
        a.swap(i, rng.range(0, i as i32 + 1) as usize);
    }
    let echo_area = [1u8, 2, 3, 4, 5][rng.range(0, 5) as usize];
    let echo = ECHOES[rng.range(0, ECHOES.len() as i32) as usize];
    ([a[0], a[1], a[2]], echo_area, echo)
}

/// The index of the side quest with this goal (side.rs SIDES).
fn quest_of(goal: Goal) -> Option<usize> {
    crate::side::SIDES.iter().position(|s| s.goal == goal)
}

impl Game {
    /// Act 7 side content as you arrive.
    pub(crate) fn churnside_enter(&mut self) {
        let here = self.level;
        // The vault's coffers (each floor once).
        if let LevelId::Dungeon(crate::world::VAULT, _) = here {
            if !self.feats.coffers_placed.contains(&here) {
                self.feats.coffers_placed.push(here);
                self.place_coffers();
            }
        }
        let LevelId::Area(6, n) = here else { return };
        // The Wandering Room, if it's drifted here.
        if !self.feats.wander_done {
            if self.feats.wander_area == 0 {
                self.feats.wander_area = 1 + self.rng.range(0, 6) as u8;
                self.feats.wander_t = WANDER_GAP;
            }
            if self.feats.wander_area == n {
                self.place_wander_room();
            }
        }
        if self.feats.churn_placed.contains(&here) {
            return;
        }
        self.feats.churn_placed.push(here);
        if here == TANGLE {
            self.place_roulette();
        }
        if here == DRIFT || here == MONASTERY {
            for _ in 0..3 {
                if let Some((x, y)) = self.churn_spot(14.0, 2.0) {
                    self.feats.wells.push((here, x, y));
                }
            }
        }
        let (words, echo_area, echo) = rolls(self.world_seed, self.quest.difficulty);
        for k in 0..3 {
            if words[k] == n && self.feats.words & (1 << k) == 0 {
                if let Some((x, y)) = self.churn_spot(16.0, 0.8) {
                    self.pickups.push(Pickup { x, y, kind: Drop::Word(k as u8), t: 1.0 });
                }
            }
        }
        if n == echo_area && !self.feats.echo_done {
            self.place_echo(echo);
        }
    }

    /// A spot far from the hero that you can walk to, away from the ways in and out.
    pub(crate) fn churn_spot(&mut self, min: f32, room: f32) -> Option<(f32, f32)> {
        let (sx, sy) = (self.p.x, self.p.y);
        for _ in 0..600 {
            let x = self.rng.range(6, self.d.w - 6) as f32 + 0.5;
            let y = self.rng.range(6, self.d.h - 6) as f32 + 0.5;
            if ((x - sx).powi(2) + (y - sy).powi(2)).sqrt() < min || self.d.blocked(x, y, room) {
                continue;
            }
            if self.portals.iter().any(|p| (p.x - x).powi(2) + (p.y - y).powi(2) < 64.0) {
                continue;
            }
            if self.d.path((sx as i32, sy as i32), (x as i32, y as i32), 40_000).is_some() {
                return Some((x, y));
            }
        }
        None
    }

    // ------------------------------------------------------------------ the Probability Vault

    fn place_coffers(&mut self) {
        let n = 6 + self.rng.range(0, 3);
        for _ in 0..n {
            let Some((x, y)) = self.churn_spot(6.0, 0.6) else { continue };
            let mut m = Mob::new(Kind::Coffer, x, y, self.tier, &mut self.rng);
            m.cue = 1;
            self.mobs.push(m);
        }
    }

    /// A coffer broken open: a gamble.
    pub(crate) fn coffer_opens(&mut self, x: f32, y: f32) {
        let ilvl = crate::items::ilvl_for(self.tier) + 3;
        let r = self.rng.f();
        for _ in 0..10 {
            self.spray_at(x, y, PKind::Magic, 10.0);
        }
        if r < 0.22 {
            // A mimic: the coffer had teeth.
            let mut m = Mob::new(Kind::ChaosToad, x, y, self.tier * 1.1, &mut self.rng);
            m.form = 3;
            m.cue = 1;
            m.promote(Rank::Champion, crate::mobs::M_STRONG | crate::mobs::M_FAST, Some("MIMIC".into()));
            m.state = MobState::Chase;
            self.mobs.push(m);
            self.floater(x, y, "THE COFFER HAD TEETH!".into(), rgb(0xff6060));
            self.sfx.push(Sfx::Boom);
        } else if r < 0.62 {
            for k in 0..4 {
                let a = k as f32 * 1.57 + 0.3;
                self.pickups.push(Pickup { x: x + a.cos() * 0.7, y: y + a.sin() * 0.7, kind: Drop::Gold((20.0 + 14.0 * self.tier) as i32), t: 0.0 });
            }
            self.floater(x, y, "GOLD!".into(), rgb(0xffd060));
        } else if r < 0.96 {
            let it = crate::items::roll(ilvl, crate::items::Rarity::Rare, &mut self.rng);
            self.pickups.push(Pickup { x, y, kind: Drop::Item(Box::new(it)), t: 0.0 });
            self.floater(x, y, "A FAIR ROLL".into(), rgb(0xffe080));
        } else {
            let it = crate::items::drop(ilvl, 600, true, &mut self.rng);
            self.pickups.push(Pickup { x, y, kind: Drop::Item(Box::new(it)), t: 0.0 });
            self.floater(x, y, "JACKPOT!".into(), rgb(0xc080ff));
            self.sfx.push(Sfx::Descend);
        }
    }

    /// The Dice-Saint rolls a die for every attack (twice when enraged), and the die decides.
    pub(crate) fn dice_trick(&mut self, i: usize, dist: f32) {
        let (x, y, tier, enraged) = (self.mobs[i].x, self.mobs[i].y, self.mobs[i].tier, self.mobs[i].enraged);
        self.mobs[i].special = if enraged { 2.0 } else { 2.8 };
        let dmg = 15.0 * tier.powf(0.8);
        let (px, py) = (self.p.x, self.p.y);
        let a0 = (py - y).atan2(px - x);
        for _ in 0..if enraged { 2 } else { 1 } {
            let roll = self.rng.range(1, 7);
            let words = ["", "SNAKE EYES...", "TWO!", "THREE!", "FOUR!", "FIVE!", "SIX! THE HOUSE WINS!"][roll as usize];
            self.floater(x, y - 0.6, format!("ROLLS A {roll}: {words}"), rgb(0xf0e0c0));
            match roll {
                // Snake eyes: he fumbles, and is open for a moment.
                1 => {
                    self.mobs[i].stun = self.mobs[i].stun.max(1.6);
                }
                // Two: a pair of chaos toads.
                2 => {
                    if self.mobs.iter().filter(|m| m.alive() && m.kind == Kind::ChaosToad).count() < 5 {
                        for k in 0..2 {
                            let a = k as f32 * 3.1 + 0.6;
                            let (mx, my) = (x + a.cos() * 1.6, y + a.sin() * 1.6);
                            if self.d.blocked(mx, my, 0.4) {
                                continue;
                            }
                            let mut t = Mob::new(Kind::ChaosToad, mx, my, tier * 0.85, &mut self.rng);
                            t.form = self.rng.range(0, 3) as u8;
                            t.cue = 1;
                            t.state = MobState::Chase;
                            self.mobs.push(t);
                        }
                    }
                }
                // Three: three novas around you.
                3 => {
                    for k in 0..3 {
                        let a = self.rng.f() * std::f32::consts::TAU;
                        let r = if k == 0 { 0.0 } else { self.rng.rf(1.2, 2.4) };
                        self.hazards.push(Hazard { x: px + a.cos() * r, y: py + a.sin() * r, r: 1.1, warn: 0.9, live: 0.0, dps: 0.0, burst: dmg * 1.3, t: 0.0, fired: false, kind: HazardKind::Nova });
                    }
                }
                // Four: a ring of dice.
                4 => {
                    for k in 0..8 {
                        let a = a0 + k as f32 / 8.0 * std::f32::consts::TAU;
                        self.shots.push(Shot { x, y, vx: a.cos() * 6.0, vy: a.sin() * 6.0, life: 2.4, dmg, kind: ShotKind::Boulder });
                    }
                }
                // Five: he steps beside you and brings his dice down.
                5 if dist > 2.0 => {
                    let d = dist.max(0.01);
                    let (nx, ny) = (px - (px - x) / d * 1.3, py - (py - y) / d * 1.3);
                    if !self.d.blocked(nx, ny, self.mobs[i].r) {
                        (self.mobs[i].x, self.mobs[i].y) = (nx, ny);
                    }
                    self.hazards.push(Hazard { x: px, y: py, r: 1.8, warn: 0.6, live: 0.0, dps: 0.0, burst: dmg * 1.5, t: 0.0, fired: false, kind: HazardKind::Quake });
                }
                5 => {
                    self.hazards.push(Hazard { x: px, y: py, r: 1.8, warn: 0.6, live: 0.0, dps: 0.0, burst: dmg * 1.5, t: 0.0, fired: false, kind: HazardKind::Quake });
                }
                // Six: a fan of dice, and the house takes a little back.
                _ => {
                    for k in -3..=3 {
                        let a = a0 + k as f32 * 0.15;
                        self.shots.push(Shot { x, y, vx: a.cos() * 7.5, vy: a.sin() * 7.5, life: 2.0, dmg: dmg * 1.1, kind: ShotKind::Boulder });
                    }
                    let m = &mut self.mobs[i];
                    m.hp = (m.hp + m.max_hp * 0.04).min(m.max_hp);
                }
            }
        }
    }

    // ------------------------------------------------------------------ Chaos Roulette

    fn place_roulette(&mut self) {
        let Some((x, y)) = self.churn_spot(10.0, 1.6) else { return };
        let (tx, ty) = (x as i32, y as i32);
        self.d.set(tx, ty, crate::dungeon::Tile::Prop);
        self.props.push(Prop::on(PropKind::Shrine(7), tx, ty, 1, 1));
        self.feats.roulette = Some((self.level, tx as f32 + 0.5, ty as f32 + 0.5));
    }

    /// Spin the wheel: a blessing for three minutes (most likely), a curse, or a jackpot.
    fn spin_roulette(&mut self, x: f32, y: f32) {
        self.feats.roulette_cd = ROULETTE_CD;
        self.feats.spins += 1;
        self.sfx.push(Sfx::Descend);
        for _ in 0..20 {
            self.spray_at(x, y, PKind::Magic, 22.0);
        }
        let r = self.rng.f();
        if r < 0.55 {
            let picks = [Blessing::Armor, Blessing::Combat, Blessing::Mana, Blessing::Experience, Blessing::Skill, Blessing::Haste];
            let b = picks[self.rng.range(0, picks.len() as i32) as usize];
            let was = self.p.blessing;
            self.p.blessing = b.id();
            self.p.bless_t = ROULETTE_TIME;
            if b == Blessing::Skill || Blessing::from_id(was) == Some(Blessing::Skill) {
                self.p.recalc();
            }
            self.floater(x, y, format!("THE WHEEL STOPS ON {}: {}", b.name().trim_end_matches(" SHRINE"), b.short()), rgb(b.col()));
        } else if r < 0.92 {
            match self.rng.range(0, 4) {
                0 => {
                    self.hurt_player(self.p.max_hp * 0.3);
                    self.floater(x, y, "THE WHEEL STOPS ON FIRE: IT BURNS".into(), rgb(0xff6030));
                }
                1 => {
                    self.p.mana = 0.0;
                    self.floater(x, y, "THE WHEEL STOPS ON VOID: YOUR MANA IS GONE".into(), rgb(0x8060c0));
                }
                2 => {
                    let lose = (self.p.gold / 10).min(5000);
                    self.p.gold -= lose;
                    self.floater(x, y, format!("THE WHEEL STOPS ON THE HOUSE: IT TAKES {lose} GOLD"), rgb(0xc0a040));
                }
                _ => {
                    let tier = self.tier;
                    for k in 0..4 {
                        let a = k as f32 * 1.57 + 0.8;
                        let (mx, my) = (self.p.x + a.cos() * 2.5, self.p.y + a.sin() * 2.5);
                        if self.d.blocked(mx, my, 0.4) {
                            continue;
                        }
                        let mut m = Mob::new(Kind::Unmade, mx, my, tier, &mut self.rng);
                        m.form = k as u8 % 4;
                        m.cue = 1;
                        m.state = MobState::Chase;
                        self.mobs.push(m);
                    }
                    self.floater(x, y, "THE WHEEL STOPS ON COMPANY: THE UNMADE".into(), rgb(0xa060e0));
                }
            }
        } else {
            let ilvl = crate::items::ilvl_for(self.tier) + 4;
            let it = crate::items::roll(ilvl, crate::items::Rarity::Rare, &mut self.rng);
            self.pickups.push(Pickup { x: self.p.x, y: self.p.y + 0.5, kind: Drop::Item(Box::new(it)), t: 0.0 });
            for k in 0..5 {
                let a = k as f32 * 1.26;
                self.pickups.push(Pickup { x: self.p.x + a.cos(), y: self.p.y + a.sin(), kind: Drop::Gold((40.0 + 25.0 * self.tier) as i32), t: 0.0 });
            }
            self.floater(x, y, "JACKPOT!".into(), rgb(0xffe060));
            self.shake = 0.5;
        }
    }

    // ------------------------------------------------------------------ the Wandering Room

    fn place_wander_room(&mut self) {
        if let Some((x, y)) = self.churn_spot(16.0, 2.4) {
            self.feats.wander_spot = Some((self.level, x, y));
            self.say("THE WANDERING ROOM HAS DRIFTED HERE. CATCH IT BEFORE IT MOVES ON".into());
        }
    }

    fn wander_on(&mut self) {
        let old = self.feats.wander_area;
        let mut n = 1 + self.rng.range(0, 6) as u8;
        if n == old {
            n = n % 6 + 1;
        }
        self.feats.wander_area = n;
        self.feats.wander_t = WANDER_GAP;
        if let Some((lv, x, y)) = self.feats.wander_spot.take() {
            if lv == self.level {
                for _ in 0..16 {
                    self.spray_at(x, y, PKind::Magic, 18.0);
                }
                let name = crate::areas::def(6, n).name;
                self.say(format!("THE WANDERING ROOM FLICKERS AWAY... TOWARD {name}"));
            }
        }
        if self.level == LevelId::Area(6, n) {
            self.place_wander_room();
        }
    }

    fn catch_wander_room(&mut self, x: f32, y: f32) {
        self.feats.wander_done = true;
        self.feats.wander_spot = None;
        self.save_due = true;
        let ilvl = crate::items::ilvl_for(self.tier) + 4;
        if let Some(u) = crate::items::boss_unique("wander") {
            self.pickups.push(Pickup { x, y: y + 0.8, kind: Drop::Item(Box::new(u)), t: 0.0 });
        }
        for dx in [-1.0f32, 1.0] {
            let it = crate::items::roll(ilvl, crate::items::Rarity::Rare, &mut self.rng);
            self.pickups.push(Pickup { x: x + dx, y, kind: Drop::Item(Box::new(it)), t: 0.0 });
        }
        for k in 0..6 {
            let a = k as f32 * 1.05;
            self.pickups.push(Pickup { x: x + a.cos() * 1.5, y: y + a.sin() * 1.5, kind: Drop::Gold((40.0 + 20.0 * self.tier) as i32), t: 0.0 });
        }
        for _ in 0..24 {
            self.spray_at(x, y, PKind::Magic, 14.0);
        }
        self.sfx.push(Sfx::Boom);
        self.say("YOU CAUGHT THE WANDERING ROOM! WHATEVER WORLD IT CAME FROM LEFT ITS TREASURE BEHIND".into());
    }

    // ------------------------------------------------------------------ the Echo Hunt

    fn place_echo(&mut self, kind: Kind) {
        let Some((x, y)) = self.churn_spot(20.0, 1.6) else { return };
        let mut m = Mob::new(kind, x, y, self.tier, &mut self.rng);
        m.max_hp *= crate::world::boss_life(6) * 0.6;
        m.hp = m.max_hp;
        m.name = Some(format!("ECHO OF {}", crate::mobs::def(kind).label));
        m.cue = 0;
        self.mobs.push(m);
    }

    /// An echo fell (kill() asks for every boss): the hunt is over for this difficulty.
    pub(crate) fn echo_check(&mut self, i: usize) {
        if self.mobs[i].name.as_deref().map_or(false, |n| n.starts_with("ECHO OF")) && !self.feats.echo_done {
            self.feats.echo_done = true;
            self.save_due = true;
            let (x, y) = (self.mobs[i].x, self.mobs[i].y);
            self.floater(x, y, "THE ECHO FADES, AND LEAVES SOMETHING REAL".into(), rgb(0xc0a0ff));
        }
    }

    // ------------------------------------------------------------------ words and ore

    /// Picked up one of Abbot Quiet's lost words.
    pub(crate) fn find_word(&mut self, k: u8) {
        self.feats.words |= 1 << k;
        self.save_due = true;
        let (title, line) = WORDS[k as usize % 3];
        let n = self.feats.words.count_ones();
        let tail = if n >= 3 { "ALL THREE WORDS. BRING THEM TO ABBOT QUIET".to_string() } else { format!("(WORDS FOUND: {n} OF 3)") };
        let mut d = Dialog::new(title, &[line, &tail]);
        d.refresh_options();
        self.dialog = Some(d);
        self.sfx.push(Sfx::Cast);
        if n >= 3 {
            self.side_progress(Goal::Words);
        }
    }

    /// Picked up a lump of chaos ore by an anchor stone.
    pub(crate) fn find_ore(&mut self) {
        self.feats.ore = (self.feats.ore + 1).min(3);
        self.save_due = true;
        let n = self.feats.ore;
        let (x, y) = (self.p.x, self.p.y);
        self.floater(x, y, format!("CHAOS ORE ({n} OF 3)"), rgb(0xc090ff));
        if n >= 3 {
            self.side_progress(Goal::Ore);
        }
    }

    // ------------------------------------------------------------------ every tick

    pub(crate) fn update_churnside(&mut self) {
        if self.level.act() != 6 {
            return;
        }
        let (px, py) = (self.p.x, self.p.y);
        // Chaos ore: while Sister Ferro wants it, each anchor stone sheds one lump when you come near.
        if let Some(q) = quest_of(Goal::Ore) {
            let pending = self.pickups.iter().filter(|k| matches!(k.kind, Drop::Ore)).count() as u8;
            if self.quest.side[q] == S_GIVEN && self.feats.ore + pending < 3 {
                let here = self.level;
                if let Some(a) = self.feats.anchors.iter_mut().find(|a| !a.ore && a.level == here && (a.x - px).powi(2) + (a.y - py).powi(2) < 2.5 * 2.5) {
                    a.ore = true;
                    let (ax, ay) = (a.x, a.y);
                    self.pickups.push(Pickup { x: ax + 0.8, y: ay + 0.4, kind: Drop::Ore, t: 0.0 });
                    self.floater(ax, ay, "A LUMP OF CHAOS ORE FLAKES OFF THE ANCHOR".into(), rgb(0xc090ff));
                }
            }
        }
        // The roulette.
        self.feats.roulette_cd -= DT;
        if let Some((lv, x, y)) = self.feats.roulette {
            if lv == self.level && self.feats.roulette_cd <= 0.0 && (x - px).powi(2) + (y - py).powi(2) < 1.4 * 1.4 {
                self.spin_roulette(x, y);
            }
        }
        // The Wandering Room drifts on while you're out in the Churn.
        if !self.feats.wander_done && self.level.overland() && !self.level.town() {
            if let Some((lv, x, y)) = self.feats.wander_spot {
                if lv == self.level && (x - px).powi(2) + (y - py).powi(2) < WANDER_R * WANDER_R {
                    self.catch_wander_room(x, y);
                    return;
                }
            }
            self.feats.wander_t -= DT;
            if self.feats.wander_t <= 0.0 {
                self.wander_on();
            }
        }
        // Gravity wells.
        let here = self.level;
        let wells: Vec<(f32, f32)> = self.feats.wells.iter().filter(|w| w.0 == here).map(|w| (w.1, w.2)).collect();
        for (wx, wy) in wells {
            let pull = |x: f32, y: f32| -> Option<(f32, f32)> {
                let (dx, dy) = (wx - x, wy - y);
                let d = (dx * dx + dy * dy).sqrt();
                (d < WELL_R && d > 0.3).then(|| (dx / d * WELL_PULL * DT, dy / d * WELL_PULL * DT))
            };
            for s in self.shots.iter_mut() {
                if let Some((ax, ay)) = pull(s.x, s.y) {
                    s.vx += ax;
                    s.vy += ay;
                }
            }
            for b in self.balls.iter_mut() {
                if let Some((ax, ay)) = pull(b.x, b.y) {
                    b.vx += ax;
                    b.vy += ay;
                }
            }
            for j in self.javelins.iter_mut() {
                if let Some((ax, ay)) = pull(j.x, j.y) {
                    let (ux, uy) = (j.ux + ax * 0.08, j.uy + ay * 0.08);
                    let l = (ux * ux + uy * uy).sqrt().max(0.01);
                    (j.ux, j.uy) = (ux / l, uy / l);
                }
            }
            for a in self.axes.iter_mut() {
                if let Some((ax, ay)) = pull(a.x, a.y) {
                    let (ux, uy) = (a.ux + ax * 0.08, a.uy + ay * 0.08);
                    let l = (ux * ux + uy * uy).sqrt().max(0.01);
                    (a.ux, a.uy) = (ux / l, uy / l);
                }
            }
            // And you: a slow drag toward it.
            let (dx, dy) = (wx - px, wy - py);
            let d = (dx * dx + dy * dy).sqrt();
            if d < WELL_R && d > 0.8 {
                let step = 0.9 * DT * (1.0 - d / WELL_R + 0.3);
                let (nx, ny) = (self.p.x + dx / d * step, self.p.y + dy / d * step);
                if !self.d.blocked(nx, ny, 0.3) {
                    (self.p.x, self.p.y) = (nx, ny);
                }
            }
        }
    }

    /// The Wandering Room, if it's on this level.
    pub fn wander_here(&self) -> Option<(f32, f32)> {
        self.feats.wander_spot.filter(|w| w.0 == self.level).map(|w| (w.1, w.2))
    }

    /// This level's gravity wells.
    pub fn wells_here(&self) -> Vec<(f32, f32)> {
        self.feats.wells.iter().filter(|w| w.0 == self.level).map(|w| (w.1, w.2)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Game;

    fn at(id: LevelId) -> Game {
        let mut g = Game::new(7, 360);
        g.p.base_hp = 50000.0;
        g.p.recalc();
        g.p.hp = g.p.max_hp;
        g.debug_goto(id);
        g
    }

    #[test]
    fn the_vault_is_full_of_gambles_and_the_dice_saint_rolls() {
        let mut g = at(LevelId::Dungeon(crate::world::VAULT, 0));
        let n = g.mobs.iter().filter(|m| m.kind == Kind::Coffer).count();
        assert!(n >= 6, "coffers: {n}");
        let mut outcomes = (0, 0);
        for _ in 0..60 {
            let before = (g.pickups.len(), g.mobs.len());
            let (x, y) = (g.p.x, g.p.y);
            g.coffer_opens(x, y);
            if g.mobs.len() > before.1 {
                outcomes.0 += 1;
            } else if g.pickups.len() > before.0 {
                outcomes.1 += 1;
            }
        }
        assert!(outcomes.0 > 0 && outcomes.1 > 0, "mimics and treasure: {outcomes:?}");
        let mut g = at(LevelId::Dungeon(crate::world::VAULT, crate::world::DUNGEONS[crate::world::VAULT].floors - 1));
        let i = g.mobs.iter().position(|m| m.kind == Kind::DiceSaint).expect("the dice-saint");
        (g.p.x, g.p.y) = (g.mobs[i].x + 4.0, g.mobs[i].y);
        for _ in 0..12 {
            g.mobs[i].special = 0.0;
            g.update_churnfolk();
        }
        assert!(g.floaters.iter().any(|f| f.text.starts_with("ROLLS A")), "he rolls");
    }

    #[test]
    fn the_roulette_spins_and_rests_a_minute() {
        let mut g = at(TANGLE);
        let (lv, x, y) = g.feats.roulette.expect("the roulette stands in the Tangle");
        assert_eq!(lv, TANGLE);
        (g.p.x, g.p.y) = (x + 1.0, y);
        g.update_churnside();
        assert_eq!(g.feats.spins, 1);
        g.update_churnside();
        assert_eq!(g.feats.spins, 1, "not again for a minute");
        g.feats.roulette_cd = 0.0;
        g.update_churnside();
        assert_eq!(g.feats.spins, 2);
    }

    #[test]
    fn the_wandering_room_drifts_until_you_catch_it() {
        let mut g = at(TANGLE);
        g.feats.wander_area = 3;
        g.feats.wander_spot = None;
        g.debug_goto(LevelId::Area(6, 2));
        g.debug_goto(TANGLE);
        let (x, y) = g.wander_here().expect("it's here");
        // It drifts off if you dawdle.
        g.feats.wander_t = 0.0;
        g.update_churnside();
        assert_ne!(g.feats.wander_area, 3);
        assert!(g.wander_here().is_none());
        // Catch it.
        g.feats.wander_spot = Some((TANGLE, x, y));
        (g.p.x, g.p.y) = (x, y);
        g.update_churnside();
        assert!(g.feats.wander_done);
        assert!(g.pickups.iter().any(|k| matches!(&k.kind, Drop::Item(it) if it.rarity == crate::items::Rarity::Unique)));
    }

    #[test]
    fn gravity_wells_bend_shots_and_tug_at_you() {
        let mut g = at(DRIFT);
        let (wx, wy) = *g.wells_here().first().expect("wells in the Drift");
        g.shots.push(Shot { x: wx - 3.0, y: wy - 2.0, vx: 6.0, vy: 0.0, life: 2.0, dmg: 1.0, kind: ShotKind::Arrow });
        g.update_churnside();
        let s = g.shots.last().unwrap();
        assert!(s.vy > 0.0, "bent toward the well");
        (g.p.x, g.p.y) = (wx + 3.0, wy);
        if g.d.blocked(g.p.x, g.p.y, 0.3) {
            return;
        }
        let before = g.p.x;
        for _ in 0..30 {
            g.update_churnside();
        }
        assert!(g.p.x < before, "pulled in");
    }

    #[test]
    fn an_echo_of_an_old_boss_haunts_one_area() {
        let (_, area, kind) = rolls(7, 0);
        let g = at(LevelId::Area(6, area));
        let m = g.mobs.iter().find(|m| m.name.as_deref().map_or(false, |n| n.starts_with("ECHO OF"))).expect("the echo");
        assert_eq!(m.kind, kind);
        // Every echo fights without trouble in the Churn.
        for k in ECHOES {
            let mut g = at(LevelId::Area(6, area));
            g.mobs.retain(|m| !m.boss);
            g.place_echo(k);
            let i = g.mobs.len() - 1;
            (g.p.x, g.p.y) = (g.mobs[i].x + 3.0, g.mobs[i].y);
            for _ in 0..600 {
                g.update(&crate::game::Input::default());
                g.p.hp = g.p.max_hp;
            }
            g.mobs[i].hp = 0.0;
            g.kill(i);
            assert!(g.feats.echo_done);
        }
    }

    #[test]
    fn stillhold_offers_every_act7_quest_and_the_supers_wait_at_home() {
        let mut g = at(LevelId::Churn);
        g.quest.stage7 = 1;
        for s in crate::side::SIDES.iter().filter(|s| s.act == 6) {
            assert!(g.debug_talk(s.giver), "{} stands in Stillhold", s.giver_name);
            let d = g.dialog.take().unwrap();
            assert!(d.options.iter().chain(d.last_options.iter()).any(|o| o.0 == s.ask), "{} offers {}", s.giver_name, s.name);
        }
        for (i, s) in crate::side::SUPERS.iter().enumerate().filter(|(_, s)| s.home.act() == 6) {
            g.debug_goto(s.home);
            assert!(g.mobs.iter().any(|m| m.superu as usize == i + 1 && m.alive()), "{} in its area", s.name);
            if s.kind == Kind::ChaosToad {
                let m = g.mobs.iter().find(|m| m.superu as usize == i + 1).unwrap();
                let c = crate::side::CHURN_TOADS.iter().position(|&k| k == i).unwrap();
                assert_eq!(m.form as usize, c, "{} keeps its colour", s.name);
            }
        }
    }

    #[test]
    fn three_words_and_three_ores_finish_their_quests() {
        let mut g = at(LevelId::Churn);
        let words = quest_of(Goal::Words).unwrap();
        let ore = quest_of(Goal::Ore).unwrap();
        g.quest.side[words] = S_GIVEN;
        g.quest.side[ore] = S_GIVEN;
        for k in 0..3 {
            g.find_word(k);
            g.dialog = None;
        }
        assert_eq!(g.quest.side[words], crate::side::S_DONE);
        // Ore comes off the anchors while the quest is open.
        g.debug_goto(LevelId::Area(6, 1));
        let a = g.feats.anchors.iter().find(|a| a.level == g.level).cloned().unwrap();
        (g.p.x, g.p.y) = (a.x + 1.5, a.y);
        g.update_churnside();
        assert!(g.pickups.iter().any(|k| matches!(k.kind, Drop::Ore)));
        for _ in 0..3 {
            g.find_ore();
        }
        assert_eq!(g.quest.side[ore], crate::side::S_DONE);
    }
}

//! The endgame (docs/ENDGAME_PLAN.md): the Rekindling brazier in every town (boss runs), the Ash Rifts
//! (endless numbered challenge maps from the Riftwarden in Windward Anchorage), and Embers of mastery
//! (small capped bonuses for every level past 50).
use crate::game::{Drop, Game, Pickup, Sfx, DT};
use crate::gfx::rgb;
use crate::mobs::{HazardKind, Kind, Mob, MobState};
use crate::story::{Act, Dialog, Role};
use crate::world::{self, LevelId, DUNGEONS};

/// Rift modifiers: a few per rift, more the higher the tier.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RiftMod {
    /// Monsters 30% faster.
    Swift,
    /// Half again as many monsters.
    Teeming,
    /// Monsters heal on every blow they land.
    Vampiric,
    /// You take a quarter more, and the rift pays half again as much gold.
    Fragile,
    /// Fire breaks out under your feet.
    Burning,
    /// Your light shrinks.
    Eclipse,
    /// The Skyreach's wind blows in the rift.
    Gale,
    /// Monsters have half again as much life.
    Armored,
}

pub const MODS: [RiftMod; 8] = [
    RiftMod::Swift,
    RiftMod::Teeming,
    RiftMod::Vampiric,
    RiftMod::Fragile,
    RiftMod::Burning,
    RiftMod::Eclipse,
    RiftMod::Gale,
    RiftMod::Armored,
];

pub fn mod_name(m: RiftMod) -> &'static str {
    match m {
        RiftMod::Swift => "SWIFT",
        RiftMod::Teeming => "TEEMING",
        RiftMod::Vampiric => "VAMPIRIC",
        RiftMod::Fragile => "FRAGILE",
        RiftMod::Burning => "BURNING GROUND",
        RiftMod::Eclipse => "ECLIPSE",
        RiftMod::Gale => "GALE",
        RiftMod::Armored => "ARMORED",
    }
}

/// Clear a rift's guardian within this long to unlock the next tier (seconds).
pub const RIFT_TIME: f32 = 600.0;
/// The share of a rift's monsters to slay before its guardian comes.
pub const RIFT_FILL: f32 = 0.6;
/// Embers: the four tracks' caps (each Ember: +2% damage, +2% life, +3% gold and magic find, +1% speed).
pub const EMBER_CAP: u8 = 25;
pub const EMBER_TRACKS: [&str; 4] = ["DAMAGE", "LIFE", "FORTUNE", "SPEED"];
/// Levels past this earn Embers.
pub const EMBER_LEVEL: u32 = 50;

/// An Ash Rift in progress.
#[derive(Clone, Debug)]
pub struct RiftRun {
    pub tier: u16,
    pub mods: Vec<RiftMod>,
    pub progress: f32,
    pub needed: f32,
    pub time: f32,
    /// The guardian has come (and the bar is full).
    pub guardian: bool,
    pub done: bool,
    pub burn_t: f32,
}

/// The area tier of a rift (before Nightmare and Hell): past Act 6's at tier 1, then steadily harder.
pub fn rift_area_tier(tier: u16) -> f32 {
    12.0 + tier as f32 * 1.2
}

/// The chance a rift guardian drops an ancient item.
pub fn ancient_chance(tier: u16) -> f32 {
    (0.04 + tier as f32 * 0.015).min(0.5)
}

impl Game {
    // ------------------------------------------------------------------ the Rekindling

    /// Has this boss's story part already happened (its relic taken, or its act finished)? Then killing it
    /// again (a rekindled act) gives only its loot.
    pub(crate) fn boss_story_done(&self, kind: Kind) -> bool {
        let Some(k) = DUNGEONS.iter().position(|d| d.boss == kind) else { return false };
        let act = DUNGEONS[k].act;
        let slot = k - act * 4;
        if slot < 3 {
            self.quest.tokens_of(act)[slot]
        } else {
            self.quest.stage_of(act) >= 3
        }
    }

    /// The act's bosses come back: its dungeons are rebuilt with fresh layouts and monsters.
    pub(crate) fn rekindle(&mut self, act: usize) {
        self.p.rekindles[act.min(5)] += 1;
        self.parked.retain(|id, _| !matches!(*id, LevelId::Dungeon(k, _) if DUNGEONS[k].act == act));
        self.sfx.push(Sfx::Boom);
        self.shake = self.shake.max(0.4);
        let names: Vec<&str> = DUNGEONS.iter().filter(|d| d.act == act).map(|d| crate::mobs::def(d.boss).label).collect();
        self.say(format!("THE BRAZIER ROARS. {} WALK AGAIN", names.join(", ")));
        self.save_due = true;
    }

    /// The seed a level is built from: dungeons of a rekindled act get a new one each time.
    pub(crate) fn level_seed(&self, id: LevelId) -> u64 {
        match id {
            LevelId::Dungeon(k, _) => {
                let n = self.p.rekindles[DUNGEONS[k].act.min(5)] as u64;
                self.world_seed ^ n.wrapping_mul(0x9E37_79B9_7F4A_7C15)
            }
            LevelId::Rift(t) => self.world_seed ^ (t as u64).wrapping_mul(0x51ED_270B) ^ self.p.rift_runs as u64 * 0x2545_F491,
            _ => self.world_seed,
        }
    }

    /// Every town gets its brazier; Windward Anchorage its Riftwarden once Solanthos has fallen (on any
    /// difficulty). Added when a town is entered, so hand-made level files get them too.
    pub(crate) fn add_endgame_npcs(&mut self) {
        if !self.level.town() {
            return;
        }
        let act = self.level.act();
        let (cx, cy) = self.town_start;
        let place = |g: &mut Game, role: Role, name: &'static str, art: &'static str, want: (f32, f32)| {
            if g.npcs.iter().any(|n| n.role == role) {
                return;
            }
            let spot = (0..24)
                .map(|k| {
                    let a = k as f32 * 0.9;
                    let r = 0.6 + k as f32 * 0.25;
                    (want.0 + a.cos() * r, want.1 + a.sin() * r)
                })
                .find(|&(x, y)| !g.d.blocked(x, y, 0.45) && g.npcs.iter().all(|n| (n.x - x).powi(2) + (n.y - y).powi(2) > 2.0));
            if let Some((x, y)) = spot {
                g.npcs.push(crate::story::Npc::new(name, role, art, x, y, 0));
            }
        };
        place(self, Role::Brazier(act as u8), "THE REKINDLING BRAZIER", "brazier_rekindle", (cx - 3.0, cy + 2.5));
        if self.level == LevelId::Heavens && (self.quest.stage6 >= 3 || self.quest.difficulty > 0) {
            place(self, Role::Riftwarden, "THE RIFTWARDEN", "npc_riftwarden", (cx + 3.0, cy + 2.5));
        }
    }

    /// The brazier's and the Riftwarden's words (they need the hero's endgame state, not just the story).
    pub(crate) fn endgame_dialog(&self, role: Role) -> Option<Dialog> {
        match role {
            Role::Brazier(a) => {
                let act = a as usize;
                let last = DUNGEONS.iter().filter(|d| d.act == act).last().map(|d| crate::mobs::def(d.boss).label).unwrap_or("");
                if self.quest.stage_of(act) < 3 {
                    return Some(Dialog::new("THE REKINDLING BRAZIER", &[&format!("THE BRAZIER IS COLD. IT WILL TAKE FLAME WHEN {last} FALLS.")]));
                }
                let mut d = Dialog::new(
                    "THE REKINDLING BRAZIER",
                    &["THE BRAZIER BURNS WITH THE LAST EMBERS OF THIS LAND'S MASTERS. FEED IT, AND THEY RETURN: NEW HALLS, NEW SERVANTS, THE SAME OLD TREASURES."],
                );
                d.last_options = vec![("REKINDLE THIS LAND (ITS BOSSES RETURN)".into(), Act::Rekindle(a)), ("NOT NOW".into(), Act::Close)];
                Some(d)
            }
            Role::Riftwarden => {
                let best = self.p.rift_best;
                let mut d = Dialog::new(
                    "THE RIFTWARDEN",
                    &[
                        "THE SUN IS OUT, BUT ITS ASH STILL DRIFTS BETWEEN THE WORLDS, AND IT GATHERS INTO RIFTS. EACH ONE DEEPER THAN THE LAST.",
                        &format!(
                            "CLEAR A RIFT'S GUARDIAN WITHIN TEN MINUTES AND THE NEXT TIER OPENS. YOUR DEEPEST: TIER {best}. EMBERS TO SPEND: {}.",
                            self.p.ember_points
                        ),
                    ],
                );
                let top = best + 1;
                let mut opts: Vec<(String, Act)> = vec![(format!("OPEN TIER {top}"), Act::OpenRift(top))];
                if best >= 1 {
                    opts.push((format!("OPEN TIER {best}"), Act::OpenRift(best)));
                }
                if best >= 5 {
                    opts.push((format!("OPEN TIER {}", best - 4), Act::OpenRift(best - 4)));
                }
                if self.p.ember_points > 0 {
                    for (k, name) in EMBER_TRACKS.iter().enumerate() {
                        let have = self.p.embers[k];
                        if have < EMBER_CAP {
                            opts.push((format!("EMBER: {name} ({have}/{EMBER_CAP})"), Act::Ember(k as u8)));
                        }
                    }
                }
                opts.push(("LEAVE".into(), Act::Close));
                d.last_options = opts;
                Some(d)
            }
            _ => None,
        }
    }

    // ------------------------------------------------------------------ Ash Rifts

    pub fn in_rift(&self) -> bool {
        matches!(self.level, LevelId::Rift(_))
    }

    pub fn rift_has(&self, m: RiftMod) -> bool {
        self.in_rift() && self.rift.as_ref().map_or(false, |r| r.mods.contains(&m))
    }

    /// Opens an Ash Rift of this tier: modifiers rolled, a fresh map, and in you go.
    pub(crate) fn open_rift(&mut self, tier: u16) {
        let tier = tier.clamp(1, self.p.rift_best + 1);
        self.p.rift_runs += 1;
        let n = (1 + tier as usize / 5).min(4);
        let mut mods: Vec<RiftMod> = vec![];
        while mods.len() < n {
            let m = MODS[self.rng.range(0, MODS.len() as i32) as usize];
            if !mods.contains(&m) {
                mods.push(m);
            }
        }
        self.parked.retain(|id, _| !matches!(id, LevelId::Rift(_)));
        self.rift = Some(RiftRun { tier, mods: mods.clone(), progress: 0.0, needed: 1.0, time: 0.0, guardian: false, done: false, burn_t: 3.0 });
        let here = self.level;
        self.go_to(LevelId::Rift(tier), Some(here));
        // Teeming: half again as many, Swift and Armored: tougher, faster.
        if mods.contains(&RiftMod::Teeming) {
            let extra: Vec<(Kind, f32, f32, f32)> = self.mobs.iter().filter(|m| !m.boss).step_by(2).map(|m| (m.kind, m.x + 0.7, m.y, m.tier)).collect();
            for (kind, x, y, tier) in extra {
                if !self.d.blocked(x, y, 0.35) {
                    let m = Mob::new(kind, x, y, tier, &mut self.rng);
                    self.mobs.push(m);
                }
            }
        }
        for m in self.mobs.iter_mut() {
            if mods.contains(&RiftMod::Swift) {
                m.speed *= 1.3;
            }
            if mods.contains(&RiftMod::Armored) {
                m.max_hp *= 1.5;
                m.hp = m.max_hp;
            }
        }
        let count = self.mobs.iter().filter(|m| m.alive() && !crate::breakables::is_prop(m.kind)).count();
        if let Some(r) = self.rift.as_mut() {
            r.needed = (count as f32 * RIFT_FILL).max(10.0);
        }
        let list: Vec<&str> = mods.iter().map(|m| mod_name(*m)).collect();
        self.say(format!("ASH RIFT, TIER {tier}: {}", list.join(", ")));
    }

    /// A monster slain in a rift fills the bar (champions and elites more); a full bar calls the guardian.
    pub(crate) fn rift_kill(&mut self, rank: crate::mobs::Rank, boss: bool) {
        if !self.in_rift() || boss {
            return;
        }
        let Some(r) = self.rift.as_mut() else { return };
        r.progress += match rank {
            crate::mobs::Rank::Normal | crate::mobs::Rank::Minion => 1.0,
            _ => 3.0,
        };
        if r.progress >= r.needed && !r.guardian {
            r.guardian = true;
            let tier = r.tier;
            self.spawn_guardian(tier);
        }
    }

    fn spawn_guardian(&mut self, tier: u16) {
        // Any of the six acts' bosses, risen from the ash, stronger with every tier.
        let bosses: Vec<Kind> = DUNGEONS.iter().map(|d| d.boss).collect();
        let kind = bosses[self.rng.range(0, bosses.len() as i32) as usize];
        let (px, py) = (self.p.x, self.p.y);
        let spot = (0..32)
            .map(|k| {
                let a = k as f32 * 0.7;
                let r = 4.0 + (k % 4) as f32;
                (px + a.cos() * r, py + a.sin() * r)
            })
            .find(|&(x, y)| !self.d.blocked(x, y, 0.8) && self.d.los(px, py, x, y))
            .unwrap_or((px + 1.0, py));
        let area = rift_area_tier(tier) * [1.0, 2.0, 3.2][self.quest.difficulty.min(2) as usize];
        let mut m = Mob::new(kind, spot.0, spot.1, area * 0.5 + 0.5, &mut self.rng);
        m.max_hp *= world::boss_life(5) * (1.0 + tier as f32 * 0.1) * [1.0, 3.5, 8.0][self.quest.difficulty.min(2) as usize];
        m.hp = m.max_hp;
        m.home = (-1000.0, -2000.0);
        m.state = MobState::Chase;
        m.name = Some("RIFT GUARDIAN".into());
        if self.rift_has(RiftMod::Swift) {
            m.speed *= 1.3;
        }
        let label = crate::mobs::def(kind).label;
        self.mobs.push(m);
        self.lights.push(crate::game::Light { x: spot.0, y: spot.1, r: 200.0, s: 1.2, life: 1.0, max: 1.0 });
        self.shake = self.shake.max(0.6);
        self.sfx.push(Sfx::Boom);
        self.say(format!("THE RIFT GUARDIAN RISES FROM THE ASH: {label}"));
    }

    /// The guardian is down: the rift's prize, and (in time) the next tier.
    pub(crate) fn rift_cleared(&mut self, x: f32, y: f32) {
        let Some(r) = self.rift.as_mut() else { return };
        if r.done || !r.guardian {
            return;
        }
        r.done = true;
        let (tier, time) = (r.tier, r.time);
        let fragile = r.mods.contains(&RiftMod::Fragile);
        let gold = (400 + 150 * tier as i32) * if fragile { 3 } else { 2 } / 2;
        self.pickups.push(Pickup { x, y: y + 0.8, kind: Drop::Gold(gold), t: 0.0 });
        let ilvl = 50;
        let mf = self.p.bonus.get(crate::items::Stat::Magic) + self.ember_fortune();
        for k in 0..(2 + tier as usize / 4).min(6) {
            let it = crate::items::drop(ilvl, mf, true, &mut self.rng);
            self.pickups.push(Pickup { x: x + k as f32 * 0.5 - 1.0, y: y - 0.5, kind: Drop::Item(Box::new(it)), t: 0.0 });
        }
        if self.rng.chance(ancient_chance(tier)) {
            if let Some(it) = crate::items::roll_ancient(self.p.skills.class, &mut self.rng) {
                self.pickups.push(Pickup { x, y, kind: Drop::Item(Box::new(it)), t: 0.0 });
                self.say("AN ANCIENT! IT GLOWS LIKE A COAL IN THE ASH".into());
            }
        }
        if time <= RIFT_TIME {
            if tier > self.p.rift_best {
                self.p.rift_best = tier;
            }
            self.say(format!("RIFT TIER {tier} CLEARED IN {}:{:02}. TIER {} IS OPEN", time as i32 / 60, time as i32 % 60, tier + 1));
        } else {
            self.say(format!("RIFT TIER {tier} CLEARED, BUT TOO LATE TO OPEN THE NEXT ({}:{:02})", time as i32 / 60, time as i32 % 60));
        }
        self.save_due = true;
    }

    /// Each tick in a rift: its clock, and Burning Ground.
    pub(crate) fn update_rift(&mut self) {
        if !self.in_rift() {
            return;
        }
        let burning = self.rift_has(RiftMod::Burning);
        let tier = self.tier;
        let (px, py) = (self.p.x, self.p.y);
        let Some(r) = self.rift.as_mut() else { return };
        if !r.done {
            r.time += DT;
        }
        if burning && !r.done {
            r.burn_t -= DT;
            if r.burn_t <= 0.0 {
                r.burn_t = 4.0;
                let a = self.rng.f() * std::f32::consts::TAU;
                let d = self.rng.rf(0.0, 1.5);
                self.hazards.push(crate::mobs::Hazard { x: px + a.cos() * d, y: py + a.sin() * d, r: 1.2, warn: 1.0, live: 3.0, dps: 4.0 * tier, burst: 0.0, t: 0.0, fired: false, kind: HazardKind::Slag });
            }
        }
    }

    /// The rift for the HUD: (tier, bar 0..1, seconds, guardian come, done).
    pub fn rift_hud(&self) -> Option<(u16, f32, f32, bool, bool)> {
        if !self.in_rift() {
            return None;
        }
        self.rift.as_ref().map(|r| (r.tier, (r.progress / r.needed).min(1.0), r.time, r.guardian, r.done))
    }

    // ------------------------------------------------------------------ Embers of mastery

    /// A level past 50 earns an Ember.
    pub(crate) fn ember_level_up(&mut self) {
        if self.p.clvl > EMBER_LEVEL {
            self.p.ember_points += 1;
            self.say(format!("AN EMBER OF MASTERY ({} TO SPEND AT THE RIFTWARDEN)", self.p.ember_points));
        }
    }

    pub(crate) fn spend_ember(&mut self, track: u8) {
        let k = (track as usize).min(3);
        if self.p.ember_points == 0 || self.p.embers[k] >= EMBER_CAP {
            return;
        }
        self.p.ember_points -= 1;
        self.p.embers[k] += 1;
        self.p.recalc();
        self.sfx.push(Sfx::Pickup);
        self.say(format!("EMBER OF {}: {}/{EMBER_CAP}", EMBER_TRACKS[k], self.p.embers[k]));
        self.save_due = true;
    }

    /// Embers' damage multiplier.
    pub fn ember_damage(&self) -> f32 {
        1.0 + 0.02 * self.p.embers[0] as f32
    }

    /// Embers' gold and magic find (percent).
    pub fn ember_fortune(&self) -> i32 {
        3 * self.p.embers[2] as i32
    }

    /// Embers' move speed multiplier.
    pub fn ember_speed(&self) -> f32 {
        1.0 + 0.01 * self.p.embers[3] as f32
    }
}

/// The colour of the rift bar and the endgame text.
pub const RIFT_COL: u32 = 0xff8a30;

/// Draws the rift's bar, clock and modifiers (top centre).
pub fn draw_rift_hud(g: &Game, scr: &mut crate::gfx::Screen) {
    use crate::gfx::Align;
    let Some((tier, bar, time, guardian, done)) = g.rift_hud() else { return };
    let w = scr.w;
    let bw = 220;
    let x = w / 2 - bw / 2;
    let y = 62;
    scr.fill(x - 1, y - 1, bw + 2, 8, rgb(0x100806));
    scr.fill(x, y, (bw as f32 * bar) as i32, 6, rgb(RIFT_COL));
    scr.fill(x, y, (bw as f32 * bar) as i32, 1, rgb(0xffd090));
    let left = (crate::endgame::RIFT_TIME - time).max(0.0);
    let clock = if done {
        "CLEARED".to_string()
    } else if left > 0.0 {
        format!("{}:{:02}", left as i32 / 60, left as i32 % 60)
    } else {
        "OUT OF TIME".into()
    };
    let label = if guardian && !done { format!("TIER {tier}  GUARDIAN!  {clock}") } else { format!("ASH RIFT TIER {tier}  {clock}") };
    scr.text(&label, w / 2, y + 10, rgb(0xffc080), Align::Center, 1);
    if let Some(r) = g.rift.as_ref() {
        let mods: Vec<&str> = r.mods.iter().map(|m| mod_name(*m)).collect();
        scr.text(&mods.join("  "), w / 2, y + 21, rgb(0xa08070), Align::Center, 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{Game, Input};

    fn hero() -> Game {
        let mut g = Game::new(5, crate::gfx::SH_WIDE);
        g.act_start(5);
        g
    }

    #[test]
    fn the_brazier_brings_an_acts_bosses_back_without_their_relics_twice() {
        let mut g = hero();
        // Act 1 is done for an act 6 hero: the Bone Crypt's boss is dead and its seal taken.
        g.debug_goto(LevelId::Dungeon(1, 1));
        assert!(g.debug_kill_boss());
        g.debug_collect_all();
        let pts = g.p.skills.points;
        g.debug_goto(LevelId::Overworld);
        assert!(g.npcs.iter().any(|n| n.role == Role::Brazier(0)), "Hollowmere has its brazier");
        let d = g.endgame_dialog(Role::Brazier(0)).unwrap();
        assert!(d.last_options.iter().any(|o| o.1 == Act::Rekindle(0)));
        g.rekindle(0);
        g.debug_goto(LevelId::Dungeon(1, 1));
        assert!(g.mobs.iter().any(|m| m.boss && m.alive()), "the Plague Warden walks again");
        assert!(g.debug_kill_boss());
        g.debug_collect_all();
        assert_eq!(g.p.skills.points, pts, "no second seal");
        // The cold brazier of an unfinished act.
        g.quest.stage6 = 0;
        let d = g.endgame_dialog(Role::Brazier(5)).unwrap();
        assert!(!d.last_options.iter().any(|o| matches!(o.1, Act::Rekindle(_))));
    }

    #[test]
    fn a_rift_fills_calls_its_guardian_and_opens_the_next_tier() {
        let mut g = hero();
        g.quest.stage6 = 3;
        g.debug_goto(LevelId::Heavens);
        assert!(g.npcs.iter().any(|n| n.role == Role::Riftwarden), "the Riftwarden waits in the Anchorage");
        g.open_rift(1);
        assert_eq!(g.level, LevelId::Rift(1));
        let r = g.rift.clone().unwrap();
        assert_eq!(r.mods.len(), 1);
        assert!(g.mobs.len() > 20, "a rift full of monsters: {}", g.mobs.len());
        // Fill the bar.
        for _ in 0..(r.needed as usize + 1) {
            g.rift_kill(crate::mobs::Rank::Normal, false);
        }
        let guardian = g.mobs.iter().position(|m| m.boss && m.name.as_deref() == Some("RIFT GUARDIAN")).expect("the guardian rises");
        g.p.hp = 1e9;
        g.mobs[guardian].hp = 0.0;
        g.kill(guardian);
        assert!(g.rift.as_ref().unwrap().done);
        assert_eq!(g.p.rift_best, 1, "tier 2 is open");
        assert!(g.pickups.iter().any(|p| matches!(p.kind, Drop::Item(_))));
        // Saved and loaded.
        g.debug_goto(LevelId::Heavens);
        let text = crate::save::to_text(&g);
        let mut h = Game::new(5, crate::gfx::SH_WIDE);
        crate::save::apply(&mut h, &text);
        assert_eq!(h.p.rift_best, 1);
        let _ = Input::default();
    }

    #[test]
    fn ancients_carry_a_power_that_works_and_survives_a_save() {
        let mut g = hero();
        g.set_class(crate::skills::Class::Sorceress);
        let mut rng = crate::rng::Rng::new(3);
        // Roll until the sorceress's own power comes up.
        let it = (0..400)
            .filter_map(|_| crate::items::roll_ancient(g.p.skills.class, &mut rng))
            .find(|it| it.stat(crate::items::Stat::Power) == 1 << crate::items::P_FIRESTORM)
            .expect("firestorm rolls");
        assert_eq!(it.rarity, crate::items::Rarity::Ancient);
        assert!(it.name.starts_with("ANCIENT "));
        assert!(it.lines().iter().any(|l| l.contains("FIRESTORM")));
        g.p.gear.bag[0] = Some(it);
        g.p.clvl = 99;
        g.p.gear.equip(0, 99).unwrap();
        g.p.recalc();
        assert!(g.has_power(crate::items::P_FIRESTORM));
        let before = g.balls.len();
        let (x, y) = (g.p.x + 3.0, g.p.y);
        g.p.cast_cd = 0.0;
        g.cast_fireball(x, y, false);
        assert_eq!(g.balls.len() - before, 3, "the fireball splits in three");
        let text = crate::save::to_text(&g);
        let mut h = Game::new(5, crate::gfx::SH_WIDE);
        crate::save::apply(&mut h, &text);
        assert!(h.has_power(crate::items::P_FIRESTORM), "the power survives a save");
    }

    #[test]
    fn embers_come_past_fifty_and_make_you_stronger() {
        let mut g = hero();
        g.p.clvl = 50;
        let xp = crate::game::xp_to_next(50);
        g.debug_gain_xp(xp + 1.0);
        assert_eq!(g.p.clvl, 51);
        assert_eq!(g.p.ember_points, 1);
        let life = g.p.max_hp;
        g.spend_ember(1);
        assert_eq!(g.p.embers[1], 1);
        assert!(g.p.max_hp > life, "more life");
        assert_eq!(g.p.ember_points, 0);
        g.spend_ember(0);
        assert_eq!(g.p.embers[0], 0, "no Ember left to spend");
    }
}

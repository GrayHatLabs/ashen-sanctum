//! The all-act systems (docs/SIDE_CONTENT_PLAN.md 2c, the user's picks):
//!
//! - **The bestiary**: every monster kind you slay is counted. At 50, 250 and 1000 kills you know that kind
//!   well: +4% damage against it per tier, and at the last tier +10% experience from it too. The map shows
//!   this act's monsters and your count.
//! - **The rival**: another adventurer turns up in town now and then and challenges you. It's opt-in: accept
//!   and they wait for you in one of the act's areas. Beat them and they leave, and a new rival comes later.
//!   Not easy to farm (the user): one rival at a time, a long wait (an hour of play) before the next, only a
//!   rival's first defeat pays their unique, and if they beat you they take some of your gold.
//! - **Merchant caravans**: now and then a caravan is camped by the road in an area, selling better goods.
//!   Sometimes raiders fall on it as you arrive; drive them off and the merchant gives you a better price.
//! - **The arena**: the Arena Master in each town opens a challenge for every act you've reached: five waves
//!   of that act's monsters, then its champion, against the clock. The best times are kept on a board shared
//!   by all your heroes.
use std::collections::HashMap;

use crate::game::{Drop, Game, PKind, Pickup, Sfx, DT};
use crate::gfx::rgb;
use crate::mobs::{def, Kind, Mob, Rank, ShotKind};
use crate::story::{Act, Dialog, Npc, Role};
use crate::world::LevelId;

/// Kills for each bestiary tier.
pub const TIERS: [u32; 3] = [50, 250, 1000];
/// Play time (seconds) before the first rival, after a rival is beaten, and after you turn one away.
pub const RIVAL_FIRST: f32 = 1200.0;
pub const RIVAL_GAP: f32 = 3600.0;
pub const RIVAL_DECLINED: f32 = 900.0;
/// A caravan comes along at most this often (seconds), and this likely each time you enter an area.
pub const CARAVAN_GAP: f32 = 600.0;
pub const CARAVAN_CHANCE: f32 = 0.35;
pub const RAIDER: &str = "CARAVAN RAIDER";
pub const ARENA_WAVES: u8 = 5;

/// The rivals, in the order they come: name, class sheet, what they say, their unique (items.rs key).
pub const RIVALS: [(&str, &str, &str); 8] = [
    ("VESNA THE RED", "mage", "SO YOU'RE THE ONE THEY'RE ALL TALKING ABOUT. I'M NOT IMPRESSED. MEET ME OUT THERE AND WE'LL SEE WHO THE SONGS ARE REALLY ABOUT."),
    ("LORD CASIMIR", "vampire", "YOUR REPUTATION SMELLS... DELICIOUS. I HAVE NOT HAD A WORTHY MEAL IN A CENTURY. COME, LET US DINE."),
    ("COGWRIGHT DAX", "inventor", "I BUILT A MACHINE THAT TELLS ME WHO'S THE BEST ADVENTURER ALIVE. IT SAYS YOU. IT'S WRONG. LET ME PROVE IT."),
    ("BRYNHILD", "valkyrie", "THE RAVENS SAY YOU ARE A WARRIOR. THE RAVENS ARE OFTEN WRONG. FACE ME, AND LET THE SKY DECIDE."),
    ("GORM BLOODAXE", "berserker", "YOU! YES, YOU! I HEARD YOU KILLED A DRAGON. I'VE KILLED TWO. FIGHT ME!"),
    ("MORWENNA", "reaper", "YOUR NAME IS IN MY LEDGER, IN PENCIL. LET'S SEE IF I CAN MAKE IT INK."),
    ("ROWAN ASHLEAF", "druid", "THE WILD DOESN'T CARE FOR HEROES. NEITHER DO I. BUT IT WOULD BE GOOD TO KNOW WHICH OF US IS STRONGER."),
    ("SISTER CASSIA", "inquisitor_hero", "THE ORDER SENT ME TO TEST YOU, HERETIC OR SAINT. I HOPE YOU'RE A HERETIC. IT'S MORE FUN."),
];

/// The arena's challenges, by act.
pub const ARENAS: [&str; 6] = ["THE ASH PIT", "THE FROST RING", "THE BLOOD COURT", "THE GEAR PIT", "THE DROWNED RING", "THE SUN COURT"];

/// Caravan merchants' names.
const MERCHANTS: [&str; 4] = ["HESKETH THE TRAVELLER", "MAMA OLUWA", "THE SILK BROTHERS", "PEDDLER QUILL"];

#[derive(Clone, Debug)]
pub struct ArenaRun {
    pub act: u8,
    pub wave: u8,
    pub time: f32,
    /// Seconds until the next wave comes in.
    pub pause: f32,
    pub done: bool,
}

/// What the all-act systems remember. Per hero, across difficulties (the rival's wait never resets).
#[derive(Clone, Debug, Default)]
pub struct Extras {
    /// Bestiary: kills by monster name.
    pub kills: HashMap<String, u32>,
    /// The rival: time left until the next one comes (seconds of play), who's next, 0 waiting / 1 in town /
    /// 2 challenge accepted, where they wait, and whose unique has been paid (bits).
    pub rival_cd: f32,
    pub rival_next: u8,
    pub rival_state: u8,
    pub rival_area: Option<LevelId>,
    pub rivals_paid: u8,
    pub rival_t: f32,
    pub rival_moves: u32,
    /// Caravans: time until the next can come, where the current one is, raiders 0 none / 1 attacking / 2 beaten.
    pub caravan_cd: f32,
    pub caravan_level: Option<LevelId>,
    pub caravan_name: &'static str,
    pub caravan_ambush: u8,
    /// The arena: the run in progress, your best times by act (0 none), first clears (bits per difficulty).
    pub arena: Option<ArenaRun>,
    pub arena_best: [f32; 6],
    pub arena_cleared: [u8; 6],
    pub started: bool,
}

/// Bestiary tier for this many kills.
pub fn tier_of(n: u32) -> u8 {
    TIERS.iter().filter(|&&t| n >= t).count() as u8
}

/// Does this kind count for the bestiary (real monsters, not crates, rocks or rivals)?
pub fn counts(k: Kind) -> bool {
    !crate::breakables::is_prop(k) && def(k).dmg.1 > 0.0 && k != Kind::Rival
}

/// The monsters of an act (its areas and dungeons), for the bestiary page.
pub fn act_monsters(act: usize) -> Vec<Kind> {
    let mut out: Vec<Kind> = vec![];
    for a in crate::areas::AREAS.iter().filter(|a| a.act as usize == act) {
        out.extend(a.monsters.iter().copied());
    }
    for d in crate::world::DUNGEONS.iter().filter(|d| d.act == act) {
        out.extend(d.monsters.iter().copied());
        out.push(d.boss);
    }
    let mut seen = vec![];
    out.retain(|k| {
        let keep = counts(*k) && !seen.contains(k);
        seen.push(*k);
        keep
    });
    out
}

/// "1:23.4"
pub fn fmt_time(t: f32) -> String {
    if t <= 0.0 {
        return "--".into();
    }
    format!("{}:{:04.1}", (t / 60.0) as u32, t % 60.0)
}

/// The arena's best-time board (shared by all heroes): (act, seconds, hero, class) lines.
pub fn board_path() -> std::path::PathBuf {
    if cfg!(test) {
        return std::env::temp_dir().join("ashensanctum_arena_test.txt");
    }
    crate::save::dir().join("arena.txt")
}

pub fn read_board() -> Vec<(u8, f32, String, String)> {
    let text = std::fs::read_to_string(board_path()).unwrap_or_default();
    text.lines()
        .filter_map(|l| {
            let mut it = l.split('|');
            Some((it.next()?.parse().ok()?, it.next()?.parse().ok()?, it.next()?.to_string(), it.next()?.to_string()))
        })
        .collect()
}

/// Adds a time to the board (keeping each act's best five); returns its place (1-5), or None.
pub fn post_time(act: u8, t: f32, hero: &str, class: &str) -> Option<usize> {
    let mut b = read_board();
    b.push((act, t, hero.replace('|', " "), class.into()));
    let mut out: Vec<(u8, f32, String, String)> = vec![];
    let mut place = None;
    for a in 0..6u8 {
        let mut rows: Vec<_> = b.iter().filter(|r| r.0 == a).cloned().collect();
        rows.sort_by(|x, y| x.1.partial_cmp(&y.1).unwrap_or(std::cmp::Ordering::Equal));
        rows.truncate(5);
        if a == act {
            place = rows.iter().position(|r| r.1 == t && r.2 == hero.replace('|', " ")).map(|p| p + 1);
        }
        out.extend(rows);
    }
    let text: String = out.iter().map(|r| format!("{}|{}|{}|{}\n", r.0, r.1, r.2, r.3)).collect();
    if let Some(d) = board_path().parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let _ = std::fs::write(board_path(), text);
    place
}

impl Game {
    // ------------------------------------------------------------------ bestiary

    /// Damage bonus against this kind from the bestiary.
    pub fn bestiary_damage(&self, k: Kind) -> f32 {
        let n = self.ex.kills.get(def(k).label).copied().unwrap_or(0);
        1.0 + 0.04 * tier_of(n) as f32
    }

    /// Counts a kill; returns the experience multiplier for it.
    pub(crate) fn bestiary_kill(&mut self, k: Kind, x: f32, y: f32) -> f32 {
        if !counts(k) {
            return 1.0;
        }
        let label = def(k).label;
        let n = self.ex.kills.entry(label.to_string()).or_insert(0);
        *n += 1;
        let n = *n;
        if TIERS.contains(&n) {
            let t = tier_of(n);
            let extra = if t == 3 { ", +10% EXPERIENCE" } else { "" };
            self.floater(x, y - 1.0, format!("BESTIARY: {label} - TIER {t} (+{}% DAMAGE{extra})", 4 * t), rgb(0xe0d0a0));
            self.save_due = true;
        }
        if tier_of(n) >= 3 {
            1.1
        } else {
            1.0
        }
    }

    /// The map's bestiary panel: this act's monsters, your kills and their tier.
    pub fn bestiary_lines(&self) -> Vec<(String, u32)> {
        let act = self.level.act();
        let mut out = vec![];
        let total: u32 = self.ex.kills.values().sum();
        out.push((format!("BESTIARY  ({total} SLAIN)"), rgb(0xe8d8b0)));
        for k in act_monsters(act) {
            let label = def(k).label;
            let n = self.ex.kills.get(label).copied().unwrap_or(0);
            let t = tier_of(n);
            let stars = if t > 0 { format!("T{t}") } else { String::new() };
            let (name, col) = if n == 0 { ("???".to_string(), 0x6a5a48) } else { (label.to_string(), [0xa89878, 0xc8b890, 0xe0c060, 0xffd080][t as usize]) };
            out.push((format!("  {name:<22} {n:>5} {stars}"), rgb(col)));
        }
        if self.ex.rival_state == 2 {
            if let Some(a) = self.ex.rival_area {
                out.push((String::new(), 0));
                let (name, ..) = RIVALS[self.ex.rival_next as usize % 8];
                out.push((format!("RIVAL: {name}"), rgb(0xff9060)));
                out.push((format!("  WAITS IN {}", Game::waypoint_name(a)), rgb(0xd08060)));
            }
        }
        out
    }

    // ------------------------------------------------------------------ arriving somewhere

    pub(crate) fn extras_enter(&mut self) {
        if !self.ex.started {
            self.ex.started = true;
            if self.ex.rival_cd <= 0.0 && self.ex.rival_state == 0 && self.ex.rival_next == 0 {
                self.ex.rival_cd = RIVAL_FIRST;
            }
        }
        if self.level.town() {
            self.town_extras();
        }
        if let LevelId::Area(..) = self.level {
            if self.ex.rival_state == 2 && self.ex.rival_area == Some(self.level) && !self.mobs.iter().any(|m| m.kind == Kind::Rival && m.alive()) {
                self.place_rival();
            }
            if self.p.clvl >= 5 && self.ex.caravan_cd <= 0.0 && self.ex.caravan_level != Some(self.level) && self.rng.chance(CARAVAN_CHANCE) {
                self.place_caravan();
            }
        }
        if let LevelId::Arena(a) = self.level {
            self.ex.arena = Some(ArenaRun { act: a, wave: 0, time: 0.0, pause: 3.0, done: false });
            self.say(format!("{}: FIVE WAVES, THEN THE CHAMPION. THE CLOCK STARTS WITH THE FIRST", ARENAS[a as usize % 6]));
        } else {
            self.ex.arena = None;
        }
    }

    /// The Arena Master in every town, and the rival when they're waiting there.
    fn town_extras(&mut self) {
        if !self.npcs.iter().any(|n| n.role == Role::ArenaMaster) {
            let spot = self.free_town_spot(&[(-6.5, 0.0), (0.0, 6.5), (-5.0, -4.0), (6.5, 0.0), (5.0, -4.0), (0.0, -6.0)]);
            self.npcs.push(Npc::new("THE ARENA MASTER", Role::ArenaMaster, "npc_guard", spot.0, spot.1, 2));
        }
        self.npcs.retain(|n| n.role != Role::Rival);
        if self.ex.rival_state == 1 {
            let (name, sheet, _) = RIVALS[self.ex.rival_next as usize % 8];
            let spot = self.free_town_spot(&[(5.5, -1.5), (-5.5, -1.5), (2.5, 6.0), (-2.5, -6.0), (7.0, 3.0)]);
            self.npcs.push(Npc::new(name, Role::Rival, sheet, spot.0, spot.1, 6));
        }
    }

    /// The first of these spots (from the town square) that's open and not crowded by townsfolk or props.
    fn free_town_spot(&self, offsets: &[(f32, f32)]) -> (f32, f32) {
        let (tx, ty) = self.town_start;
        offsets
            .iter()
            .map(|(dx, dy)| (tx + dx, ty + dy))
            .find(|&(x, y)| {
                !self.d.blocked(x, y, 0.8)
                    && self.npcs.iter().all(|n| (n.x - x).powi(2) + (n.y - y).powi(2) > 6.0)
                    && self.portals.iter().all(|p| (p.x - x).powi(2) + (p.y - y).powi(2) > 6.0)
            })
            .unwrap_or((tx + 1.0, ty + 1.0))
    }

    // ------------------------------------------------------------------ every tick

    pub(crate) fn update_extras(&mut self) {
        // The rival's clock runs while you play (anywhere), so quitting and reloading doesn't bring one sooner.
        if self.ex.rival_state == 0 && self.p.clvl >= 6 {
            self.ex.rival_cd -= DT;
            if self.ex.rival_cd <= 0.0 {
                self.ex.rival_state = 1;
                self.save_due = true;
                let (name, ..) = RIVALS[self.ex.rival_next as usize % 8];
                self.say(format!("A RIVAL ADVENTURER, {name}, IS ASKING FOR YOU IN TOWN"));
                if self.level.town() {
                    self.town_extras();
                }
            }
        }
        if self.ex.caravan_cd > 0.0 {
            self.ex.caravan_cd -= DT;
        }
        if self.ex.rival_state == 2 && self.ex.rival_area == Some(self.level) {
            self.rival_fight();
        }
        if self.ex.caravan_ambush == 1 && self.ex.caravan_level == Some(self.level) {
            let left = self.mobs.iter().filter(|m| m.alive() && m.name.as_deref() == Some(RAIDER)).count();
            if left == 0 {
                self.ex.caravan_ambush = 2;
                self.say(format!("{} THANKS YOU: \"FOR YOU, FRIEND, A THIRD OFF EVERYTHING\"", self.ex.caravan_name));
            }
        }
        self.update_arena();
    }

    // ------------------------------------------------------------------ the rival

    pub(crate) fn rival_dialog(&mut self) -> Dialog {
        let (name, _, line) = RIVALS[self.ex.rival_next as usize % 8];
        let mut d = Dialog::new(name, &[line]);
        d.options = vec![("ACCEPT THE CHALLENGE".into(), Act::Rival(1)), ("NOT TODAY".into(), Act::Rival(0))];
        d
    }

    pub(crate) fn rival_answer(&mut self, yes: bool) {
        let (name, ..) = RIVALS[self.ex.rival_next as usize % 8];
        self.dialog = None;
        self.npcs.retain(|n| n.role != Role::Rival);
        self.save_due = true;
        if !yes {
            self.ex.rival_state = 0;
            self.ex.rival_cd = RIVAL_DECLINED;
            self.say(format!("{name}: \"SUIT YOURSELF. I'LL BE BACK, AND I WON'T ASK SO NICELY.\""));
            return;
        }
        let act = self.level.act() as u8;
        let areas: Vec<u8> = crate::areas::AREAS.iter().filter(|a| a.act == act).map(|a| a.n).collect();
        let n = areas[self.rng.range(0, areas.len() as i32) as usize];
        let at = LevelId::Area(act, n);
        self.ex.rival_state = 2;
        self.ex.rival_area = Some(at);
        self.say(format!("{name}: \"I'LL BE WAITING IN {}. DON'T KEEP ME WAITING.\"", Game::waypoint_name(at)));
    }

    fn place_rival(&mut self) {
        let k = self.ex.rival_next as usize % 8;
        let (name, ..) = RIVALS[k];
        let (sx, sy) = (self.p.x, self.p.y);
        let mut spot = None;
        let mut tries = 0;
        for _ in 0..600 {
            let x = self.rng.range(6, self.d.w - 6) as f32 + 0.5;
            let y = self.rng.range(6, self.d.h - 6) as f32 + 0.5;
            let far = ((x - sx).powi(2) + (y - sy).powi(2)).sqrt();
            if far < 14.0 || self.d.blocked(x, y, 1.2) {
                continue;
            }
            tries += 1;
            if tries > 20 {
                break;
            }
            if self.d.path((sx as i32, sy as i32), (x as i32, y as i32), 40_000).is_some() {
                spot = Some((x, y));
                break;
            }
        }
        let Some((x, y)) = spot else { return };
        // A rival is about your match: tougher than the area's elites, scaled to your level.
        let tier = self.tier.max(1.0 + self.p.clvl as f32 * 0.27);
        let mut m = Mob::new(Kind::Rival, x, y, tier, &mut self.rng);
        m.form = k as u8;
        let mods = [crate::mobs::M_FIERY, crate::mobs::M_VAMPIRE, crate::mobs::M_STONE, crate::mobs::M_FAST, crate::mobs::M_STRONG, crate::mobs::M_MANABURN, crate::mobs::M_STONE, crate::mobs::M_FIERY][k];
        m.promote(Rank::Elite, mods, Some(name.into()));
        m.max_hp *= 2.0;
        m.hp = m.max_hp;
        self.mobs.push(m);
        self.ex.rival_t = 3.0;
        // Clear the ground around them: this is a duel.
        for o in self.mobs.iter_mut() {
            if o.kind != Kind::Rival && o.alive() && (o.x - x).powi(2) + (o.y - y).powi(2) < 36.0 {
                o.x += if o.x < x { -6.0 } else { 6.0 };
                if self.d.blocked(o.x, o.y, o.r) {
                    o.x = if o.x < x { o.x + 6.0 } else { o.x - 6.0 };
                }
            }
        }
    }

    /// The rival fights like a hero of their class.
    fn rival_fight(&mut self) {
        let Some(i) = self.mobs.iter().position(|m| m.kind == Kind::Rival && m.alive()) else { return };
        let (x, y, form) = (self.mobs[i].x, self.mobs[i].y, self.mobs[i].form);
        let (px, py) = (self.p.x, self.p.y);
        let dist = ((px - x).powi(2) + (py - y).powi(2)).sqrt();
        if dist > 14.0 {
            return;
        }
        let hurt = self.mobs[i].hp < self.mobs[i].max_hp * 0.5;
        self.ex.rival_t -= DT;
        if self.ex.rival_t > 0.0 {
            return;
        }
        self.ex.rival_t = if hurt { 1.6 } else { 2.3 };
        self.ex.rival_moves += 1;
        let tier = self.mobs[i].tier;
        let dmg = 12.0 * tier.powf(0.8);
        let a0 = (py - y).atan2(px - x);
        let fan = |g: &mut Game, n: i32, spread: f32, speed: f32, kind: ShotKind| {
            for k in -(n / 2)..=(n / 2) {
                let a = a0 + k as f32 * spread;
                g.shots.push(crate::mobs::Shot { x, y, vx: a.cos() * speed, vy: a.sin() * speed, life: 2.0, dmg, kind });
            }
        };
        let third = self.ex.rival_moves % 3 == 0;
        match form {
            // Vesna: fireball fans.
            0 => fan(self, 5, 0.16, 7.5, ShotKind::Ash),
            // Casimir: blood bolts, and he drinks.
            1 => {
                fan(self, 3, 0.2, 7.0, ShotKind::Blood);
                let m = &mut self.mobs[i];
                m.hp = (m.hp + m.max_hp * 0.03).min(m.max_hp);
            }
            // Dax: cogs, and brass scarabs.
            2 => {
                fan(self, 4, 0.22, 7.0, ShotKind::Gear);
                if third {
                    self.rival_minions(Kind::Scarab, 2, x, y, tier);
                }
            }
            // Brynhild: rune spears of ice.
            3 => fan(self, 3, 0.1, 9.0, ShotKind::Ice),
            // Gorm: leaps in and slams.
            4 => {
                if dist > 2.0 {
                    let (nx, ny) = (px - (px - x) / dist * 1.4, py - (py - y) / dist * 1.4);
                    if !self.d.blocked(nx, ny, 0.4) {
                        (self.mobs[i].x, self.mobs[i].y) = (nx, ny);
                    }
                }
                self.hazards.push(crate::mobs::Hazard { x: px, y: py, r: 2.2, warn: 0.8, live: 0.0, dps: 0.0, burst: dmg * 1.6, t: 0.0, fired: false, kind: crate::mobs::HazardKind::Quake });
                self.floater(x, y, "RAAAH!".into(), rgb(0xff6040));
            }
            // Morwenna: grave-fire, and the dead rise.
            5 => {
                fan(self, 3, 0.18, 7.0, ShotKind::Necro);
                if third {
                    self.rival_minions(Kind::Skeleton, 3, x, y, tier);
                }
            }
            // Rowan: spore ground, and wolves.
            6 => {
                self.hazards.push(crate::mobs::Hazard { x: px, y: py, r: 1.6, warn: 0.6, live: 4.0, dps: dmg * 0.5, burst: 0.0, t: 0.0, fired: false, kind: crate::mobs::HazardKind::Poison });
                if third {
                    self.rival_minions(Kind::Wolf, 2, x, y, tier);
                }
            }
            // Cassia: holy fire in a ring round you.
            _ => {
                for k in 0..4 {
                    let a = k as f32 * std::f32::consts::FRAC_PI_2 + a0;
                    self.hazards.push(crate::mobs::Hazard { x: px + a.cos() * 1.8, y: py + a.sin() * 1.8, r: 1.0, warn: 0.9, live: 0.0, dps: 0.0, burst: dmg * 1.2, t: 0.0, fired: false, kind: crate::mobs::HazardKind::Nova });
                }
                self.hazards.push(crate::mobs::Hazard { x: px, y: py, r: 1.0, warn: 1.1, live: 0.0, dps: 0.0, burst: dmg, t: 0.0, fired: false, kind: crate::mobs::HazardKind::Nova });
            }
        }
    }

    fn rival_minions(&mut self, kind: Kind, n: usize, x: f32, y: f32, tier: f32) {
        for k in 0..n {
            let a = k as f32 * 2.4 + self.rng.f();
            let (mx, my) = (x + a.cos() * 1.5, y + a.sin() * 1.5);
            if !self.d.blocked(mx, my, 0.35) {
                let mut m = Mob::new(kind, mx, my, tier * 0.8, &mut self.rng);
                m.state = crate::mobs::MobState::Chase;
                self.mobs.push(m);
            }
        }
    }

    /// The rival is beaten: they yield and leave. Only their first defeat pays their unique.
    pub(crate) fn rival_beaten(&mut self, x: f32, y: f32) {
        let k = self.ex.rival_next as usize % 8;
        let (name, ..) = RIVALS[k];
        let ilvl = crate::items::ilvl_for(self.tier) + 3;
        if self.ex.rivals_paid & (1 << k) == 0 {
            self.ex.rivals_paid |= 1 << k;
            if let Some(u) = crate::items::boss_unique(&format!("rival{k}")) {
                self.pickups.push(Pickup { x, y: y + 0.6, kind: Drop::Item(Box::new(u)), t: 0.0 });
            }
            let it = crate::items::roll(ilvl, crate::items::Rarity::Rare, &mut self.rng);
            self.pickups.push(Pickup { x: x + 0.7, y, kind: Drop::Item(Box::new(it)), t: 0.0 });
            self.say(format!("{name} YIELDS: \"FINE! YOU'RE BETTER. TAKE THIS, AND TELL NO ONE.\""));
        } else {
            self.pickups.push(Pickup { x, y, kind: Drop::Gold((20.0 + 10.0 * self.tier) as i32), t: 0.0 });
            self.say(format!("{name} YIELDS, AND LEAVES WITHOUT A WORD"));
        }
        self.ex.rival_state = 0;
        self.ex.rival_area = None;
        self.ex.rival_cd = RIVAL_GAP;
        self.ex.rival_next = (self.ex.rival_next + 1) % 8;
        self.save_due = true;
        // The rival's helpers scatter.
        for m in self.mobs.iter_mut().filter(|m| m.alive() && matches!(m.kind, Kind::Scarab | Kind::Skeleton | Kind::Wolf) && (m.x - x).powi(2) + (m.y - y).powi(2) < 64.0) {
            m.hp = 0.0;
            m.state = crate::mobs::MobState::Dead(0.0);
        }
    }

    /// You fell to the rival: they take some of your gold, and wait to go again.
    pub(crate) fn rival_won(&mut self) {
        if self.ex.rival_state != 2 || self.ex.rival_area != Some(self.level) {
            return;
        }
        let Some(i) = self.mobs.iter().position(|m| m.kind == Kind::Rival && m.alive()) else { return };
        let m = &mut self.mobs[i];
        m.hp = m.max_hp;
        let take = (self.p.gold / 10).min(2000 * (1 + self.quest.difficulty as i32));
        self.p.gold -= take;
        let (name, ..) = RIVALS[self.ex.rival_next as usize % 8];
        self.say(format!("{name} TAKES {take} GOLD FROM YOUR PURSE: \"THANKS FOR THE PRACTICE.\""));
    }

    // ------------------------------------------------------------------ caravans

    pub(crate) fn place_caravan(&mut self) {
        let (sx, sy) = (self.p.x, self.p.y);
        let mut spot = None;
        let mut tries = 0;
        for _ in 0..600 {
            let x = self.rng.range(8, self.d.w - 8);
            let y = self.rng.range(8, self.d.h - 8);
            let far = ((x as f32 - sx).powi(2) + (y as f32 - sy).powi(2)).sqrt();
            if !(10.0..30.0).contains(&far) || (-1..=4).any(|dy| (-1..=4).any(|dx| !self.d.walkable(x + dx, y + dy))) {
                continue;
            }
            if self.portals.iter().any(|p| (p.x - x as f32).powi(2) + (p.y - y as f32).powi(2) < 64.0) {
                continue;
            }
            // Paths are dear: give up after a few tries rather than search the whole map again and again.
            tries += 1;
            if tries > 20 {
                break;
            }
            if self.d.path((sx as i32, sy as i32), (x, y + 3), 20_000).is_some() {
                spot = Some((x, y));
                break;
            }
        }
        let Some((x, y)) = spot else { return };
        // The last caravan has moved on.
        if let Some(old) = self.ex.caravan_level.take() {
            if let Some(lv) = self.parked.get_mut(&old) {
                lv.npcs.retain(|n| !matches!(n.role, Role::Caravan | Role::CaravanGuard));
            }
        }
        self.set_prop(crate::world::PropKind::Cart, x, y, 3, 2);
        let name = MERCHANTS[self.rng.range(0, MERCHANTS.len() as i32) as usize];
        let (fx, fy) = (x as f32 + 1.5, y as f32 + 3.0);
        self.npcs.push(Npc::new(name, Role::Caravan, "npc_trader", fx, fy, 0));
        self.npcs.push(Npc::new("CARAVAN GUARD", Role::CaravanGuard, "npc_guard", fx - 2.0, fy + 0.5, 1));
        self.npcs.push(Npc::new("CARAVAN GUARD", Role::CaravanGuard, "npc_guard", fx + 2.5, fy - 0.5, 3));
        self.ex.caravan_level = Some(self.level);
        self.ex.caravan_name = name;
        self.ex.caravan_cd = CARAVAN_GAP;
        self.ex.caravan_ambush = 0;
        self.stock(Role::Caravan, 5, false);
        // Raiders, sometimes, falling on it as you come up the road.
        if self.rng.chance(0.4) {
            let pool: Vec<Kind> = crate::areas::AREAS.iter().filter(|a| LevelId::Area(a.act, a.n) == self.level).flat_map(|a| a.monsters.iter().copied()).collect();
            if !pool.is_empty() {
                let kind = pool[self.rng.range(0, pool.len() as i32) as usize];
                for k in 0..5 {
                    let a = k as f32 * 1.25;
                    let (mx, my) = (fx + a.cos() * 4.0, fy + a.sin() * 4.0);
                    if self.d.blocked(mx, my, 0.4) {
                        continue;
                    }
                    let mut m = Mob::new(kind, mx, my, self.tier, &mut self.rng);
                    m.promote(if k == 0 { Rank::Champion } else { Rank::Normal }, if k == 0 { crate::mobs::M_FAST } else { 0 }, Some(RAIDER.into()));
                    self.mobs.push(m);
                }
                self.ex.caravan_ambush = 1;
                self.say(format!("RAIDERS ARE ATTACKING {}'S CARAVAN!", name));
                return;
            }
        }
        self.say(format!("A MERCHANT CARAVAN IS CAMPED NEARBY: {name}"));
    }

    // ------------------------------------------------------------------ the arena

    pub(crate) fn arena_dialog(&mut self) -> Dialog {
        let mut d = Dialog::new(
            "THE ARENA MASTER",
            &["STEP INTO THE RING, IF YOU DARE. FIVE WAVES OF THE WORST THIS LAND HAS, THEN ITS CHAMPION, AND THE CLOCK RUNNING. THE BEST TIMES GO UP ON MY BOARD FOR ALL TO SEE."],
        );
        let reached = self.level.act().max(self.highest_act());
        let mut opts = vec![];
        for a in 0..=reached.min(5) {
            let best = self.ex.arena_best[a];
            opts.push((format!("{} (ACT {})  BEST {}", ARENAS[a], a + 1, fmt_time(best)), Act::Arena(a as u8)));
        }
        opts.push(("THE BOARD OF BEST TIMES".into(), Act::Arena(99)));
        opts.push(("FAREWELL".into(), Act::Close));
        d.options = opts;
        d
    }

    /// The furthest act this hero has reached (on this difficulty).
    fn highest_act(&self) -> usize {
        let q = &self.quest;
        [q.stage2, q.stage3, q.stage4, q.stage5, q.stage6].iter().filter(|&&s| s > 0).count()
    }

    pub(crate) fn arena_choice(&mut self, a: u8) {
        if a == 99 {
            let board = read_board();
            let mut lines: Vec<String> = vec![];
            for (k, name) in ARENAS.iter().enumerate() {
                let rows: Vec<String> = board.iter().filter(|r| r.0 as usize == k).take(3).map(|r| format!("{} {} ({})", fmt_time(r.1), r.2, r.3)).collect();
                if !rows.is_empty() {
                    lines.push(format!("{name}: {}", rows.join(", ")));
                }
            }
            if lines.is_empty() {
                lines.push("THE BOARD IS EMPTY. BE THE FIRST.".into());
            }
            let refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
            let mut d = Dialog::new("THE BOARD OF BEST TIMES", &refs);
            d.refresh_options();
            self.dialog = Some(d);
            return;
        }
        self.dialog = None;
        self.parked.retain(|id, _| !matches!(id, LevelId::Arena(_)));
        let here = self.level;
        self.go_to(LevelId::Arena(a), Some(here));
    }

    fn update_arena(&mut self) {
        let Some(mut run) = self.ex.arena.clone() else { return };
        if run.done {
            return;
        }
        if run.wave > 0 {
            run.time += DT;
        }
        let alive = self.mobs.iter().filter(|m| m.alive() && !crate::breakables::is_prop(m.kind)).count();
        if alive == 0 {
            if run.wave >= ARENA_WAVES + 1 {
                run.done = true;
                self.ex.arena = Some(run.clone());
                self.arena_won(run.act, run.time);
                return;
            }
            run.pause -= DT;
            if run.pause <= 0.0 {
                run.wave += 1;
                run.pause = 2.5;
                self.ex.arena = Some(run.clone());
                self.arena_wave(run.act, run.wave);
                return;
            }
        }
        self.ex.arena = Some(run);
    }

    fn arena_wave(&mut self, act: u8, wave: u8) {
        let pool: Vec<Kind> = crate::world::DUNGEONS.iter().filter(|d| d.act == act as usize).take(4).flat_map(|d| d.monsters.iter().copied()).filter(|k| counts(*k)).collect();
        let (cx, cy) = (18.5f32, 18.5f32);
        let champion = wave == ARENA_WAVES + 1;
        let n = if champion { 4 } else { 5 + 2 * wave as usize };
        for k in 0..n {
            let a = k as f32 / n as f32 * std::f32::consts::TAU + wave as f32;
            let (x, y) = (cx + a.cos() * 9.0, cy + a.sin() * 9.0);
            if self.d.blocked(x, y, 0.4) {
                continue;
            }
            let kind = pool[self.rng.range(0, pool.len() as i32) as usize];
            let mut m = Mob::new(kind, x, y, self.tier, &mut self.rng);
            m.state = crate::mobs::MobState::Chase;
            if champion && k == 0 {
                m.promote(Rank::Elite, crate::mobs::M_STRONG | crate::mobs::M_FAST, Some("THE ARENA CHAMPION".into()));
                m.max_hp *= 4.0;
                m.hp = m.max_hp;
            } else if wave >= 3 && k % 4 == 0 {
                m.promote(Rank::Champion, crate::mobs::M_STRONG, None);
            }
            self.mobs.push(m);
        }
        let label = if champion { "THE CHAMPION!".to_string() } else { format!("WAVE {wave} OF {ARENA_WAVES}") };
        self.say(label);
        self.sfx.push(Sfx::Cast);
    }

    fn arena_won(&mut self, act: u8, t: f32) {
        let a = act as usize % 6;
        let first = self.ex.arena_cleared[a] & (1 << self.quest.difficulty) == 0;
        self.ex.arena_cleared[a] |= 1 << self.quest.difficulty;
        let best = self.ex.arena_best[a];
        let pb = best <= 0.0 || t < best;
        if pb {
            self.ex.arena_best[a] = t;
        }
        let place = post_time(act, t, &self.hero_name.clone(), self.p.skills.class.name());
        let (x, y) = (self.p.x, self.p.y + 1.0);
        if first {
            let ilvl = crate::items::ilvl_for(self.tier) + 3;
            let it = crate::items::roll(ilvl, crate::items::Rarity::Rare, &mut self.rng);
            self.pickups.push(Pickup { x, y, kind: Drop::Item(Box::new(it)), t: 0.0 });
            self.pickups.push(Pickup { x: x + 0.8, y, kind: Drop::Gold(150 * (a as i32 + 1) * (self.quest.difficulty as i32 + 1)), t: 0.0 });
        } else if pb {
            self.pickups.push(Pickup { x, y, kind: Drop::Gold(40 * (a as i32 + 1)), t: 0.0 });
        }
        for _ in 0..16 {
            self.spray_at(x, y, PKind::Magic, 10.0);
        }
        self.save_due = true;
        let rank = place.map(|p| format!("  #{p} ON THE BOARD")).unwrap_or_default();
        let new = if pb { "  A NEW BEST!" } else { "" };
        self.say(format!("{} CLEARED IN {}{new}{rank}", ARENAS[a], fmt_time(t)));
    }

}

/// The arena's clock and wave, under the top bar.
pub fn draw_arena_hud(g: &Game, scr: &mut crate::gfx::Screen) {
    use crate::gfx::Align;
    let Some(r) = g.ex.arena.as_ref() else { return };
    let w = scr.w;
    let wave = if r.done {
        "CLEARED".to_string()
    } else if r.wave == 0 {
        "GET READY".to_string()
    } else if r.wave > ARENA_WAVES {
        "THE CHAMPION".to_string()
    } else {
        format!("WAVE {}/{ARENA_WAVES}", r.wave)
    };
    let label = format!("{}  {wave}  {}", ARENAS[r.act as usize % 6], fmt_time(r.time.max(0.01)));
    scr.text(&label, w / 2, 62, rgb(0xffc080), Align::Center, 1);
    let best = g.ex.arena_best[r.act as usize % 6];
    if best > 0.0 {
        scr.text(&format!("YOUR BEST {}", fmt_time(best)), w / 2, 73, rgb(0xa08070), Align::Center, 1);
    }
}

/// The save line for the all-act systems.
pub fn to_line(e: &Extras) -> String {
    let kills: Vec<String> = e.kills.iter().map(|(k, n)| format!("{k}:{n}")).collect();
    let best: Vec<String> = e.arena_best.iter().map(|t| format!("{t:.2}")).collect();
    let clr: Vec<String> = e.arena_cleared.iter().map(|c| c.to_string()).collect();
    format!(
        "bestiary={}\nrival={},{},{},{},{}\ncaravan={}\narena={};{}\n",
        kills.join("|"),
        e.rival_cd as i32,
        e.rival_next,
        e.rival_state,
        e.rival_area.map(crate::levels::id_string).unwrap_or_default(),
        e.rivals_paid,
        e.caravan_cd as i32,
        best.join(","),
        clr.join(",")
    )
}

pub fn from_lines(get: &dyn Fn(&str) -> Option<String>, e: &mut Extras) {
    if let Some(v) = get("bestiary") {
        for part in v.split('|') {
            if let Some((k, n)) = part.rsplit_once(':') {
                if let Ok(n) = n.parse() {
                    e.kills.insert(k.to_string(), n);
                }
            }
        }
    }
    if let Some(v) = get("rival") {
        let f: Vec<&str> = v.split(',').collect();
        if f.len() >= 5 {
            e.rival_cd = f[0].parse().unwrap_or(RIVAL_FIRST as i32) as f32;
            e.rival_next = f[1].parse().unwrap_or(0) % 8;
            e.rival_state = f[2].parse().unwrap_or(0).min(2);
            e.rival_area = crate::levels::parse_id(f[3]);
            if e.rival_state == 2 && e.rival_area.is_none() {
                e.rival_state = 1;
            }
            e.rivals_paid = f[4].parse().unwrap_or(0);
            e.started = true;
        }
    }
    if let Some(v) = get("caravan") {
        e.caravan_cd = v.parse::<i32>().unwrap_or(0) as f32;
    }
    if let Some(v) = get("arena") {
        if let Some((b, c)) = v.split_once(';') {
            for (k, t) in b.split(',').enumerate().take(6) {
                e.arena_best[k] = t.parse().unwrap_or(0.0);
            }
            for (k, t) in c.split(',').enumerate().take(6) {
                e.arena_cleared[k] = t.parse().unwrap_or(0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Game;

    fn at(id: LevelId) -> Game {
        let mut g = Game::new(7, 360);
        g.p.base_hp = 5000.0;
        g.p.recalc();
        g.p.hp = g.p.max_hp;
        g.debug_goto(id);
        g
    }

    #[test]
    fn the_bestiary_counts_kills_and_pays_by_tier() {
        let mut g = at(LevelId::Area(0, 1));
        assert_eq!(g.bestiary_damage(Kind::Zombie), 1.0);
        for _ in 0..50 {
            g.bestiary_kill(Kind::Zombie, 1.0, 1.0);
        }
        assert!((g.bestiary_damage(Kind::Zombie) - 1.04).abs() < 1e-4);
        for _ in 0..950 {
            g.bestiary_kill(Kind::Zombie, 1.0, 1.0);
        }
        assert!((g.bestiary_damage(Kind::Zombie) - 1.12).abs() < 1e-4);
        assert_eq!(g.bestiary_kill(Kind::Zombie, 1.0, 1.0), 1.1);
        assert!(!counts(Kind::Crate) && !counts(Kind::Rival));
        let mut e = Extras::default();
        from_lines(&|k: &str| to_line(&g.ex).lines().find_map(|l| l.strip_prefix(k).and_then(|r| r.strip_prefix('=')).map(String::from)), &mut e);
        assert_eq!(e.kills.get("RISEN ZOMBIE").or(e.kills.get(def(Kind::Zombie).label)).copied(), Some(1001));
    }

    #[test]
    fn a_rival_comes_waits_in_an_area_and_once_beaten_stays_away_a_long_time() {
        let mut g = at(LevelId::Overworld);
        g.p.clvl = 10;
        g.ex.rival_cd = 0.01;
        g.update_extras();
        assert_eq!(g.ex.rival_state, 1);
        g.town_extras();
        assert!(g.npcs.iter().any(|n| n.role == Role::Rival), "the rival waits in town");
        g.rival_answer(true);
        assert_eq!(g.ex.rival_state, 2);
        let at_ = g.ex.rival_area.unwrap();
        g.debug_goto(at_);
        let i = g.mobs.iter().position(|m| m.kind == Kind::Rival && m.alive()).expect("the rival is there");
        // You lose: they take gold and wait.
        g.p.gold = 1000;
        g.rival_won();
        assert_eq!(g.p.gold, 900);
        let (x, y) = (g.mobs[i].x, g.mobs[i].y);
        g.mobs[i].hp = 0.0;
        g.mobs[i].state = crate::mobs::MobState::Dead(0.0);
        g.rival_beaten(x, y);
        assert_eq!(g.ex.rival_state, 0);
        assert!(g.ex.rival_cd >= RIVAL_GAP, "a long wait for the next");
        assert_eq!(g.ex.rivals_paid, 1);
        // The same rival beaten again (a later cycle) pays no unique.
        g.ex.rival_next = 0;
        let before = g.pickups.iter().filter(|k| matches!(&k.kind, Drop::Item(it) if it.rarity == crate::items::Rarity::Unique)).count();
        g.rival_beaten(x, y);
        let after = g.pickups.iter().filter(|k| matches!(&k.kind, Drop::Item(it) if it.rarity == crate::items::Rarity::Unique)).count();
        assert_eq!(before, after);
    }

    #[test]
    fn caravans_trade_and_raiders_attack() {
        let mut g = at(LevelId::Area(0, 2));
        g.ex.caravan_cd = 0.0;
        g.place_caravan();
        assert!(g.npcs.iter().any(|n| n.role == Role::Caravan));
        assert!(g.feature_dialog(Role::Caravan).is_some());
        if g.ex.caravan_ambush == 1 {
            for m in g.mobs.iter_mut().filter(|m| m.name.as_deref() == Some(RAIDER)) {
                m.hp = 0.0;
                m.state = crate::mobs::MobState::Dead(0.0);
            }
            g.update_extras();
            assert_eq!(g.ex.caravan_ambush, 2);
        }
    }

    #[test]
    fn the_arena_runs_waves_then_posts_a_time() {
        let mut g = at(LevelId::Overworld);
        g.town_extras();
        assert!(g.npcs.iter().any(|n| n.role == Role::ArenaMaster));
        g.arena_choice(0);
        assert_eq!(g.level, LevelId::Arena(0));
        for _ in 0..4000 {
            for m in g.mobs.iter_mut() {
                m.hp = 0.0;
                m.state = crate::mobs::MobState::Dead(0.0);
            }
            g.update_extras();
            if g.ex.arena.as_ref().map_or(false, |r| r.done) {
                break;
            }
        }
        assert!(g.ex.arena.as_ref().unwrap().done);
        assert!(g.ex.arena_best[0] > 0.0);
        assert!(g.ex.arena_cleared[0] & 1 != 0);
    }
}

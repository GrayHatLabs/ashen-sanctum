//! Game state and simulation. No SDL here, so the game also runs headless for
//! snapshots and self-tests. Rendering lives in `render.rs`, monsters in `mobs.rs`,
//! maps in `world.rs` and the story in `story.rs`.
use crate::art::Art;
use crate::dungeon::{Dungeon, Tile};
use crate::gfx::rgb;
use crate::iso;
use crate::mobs::{Hazard, Kind, Mob, MobState, Shot};
use crate::rng::Rng;
use crate::story::{self, Act, Dialog, Npc, Quest, Role, Ware};
use crate::world::{self, Level, LevelId, Portal, PortalKind, Prop, Theme, SANCTUM};
#[cfg(test)]
use crate::world::DUNGEONS;
use crate::world::DUNGEONS as DUNGEONS_LIST;
use std::collections::HashMap;

pub const DT: f32 = 1.0 / 60.0;
pub const HUD_H: i32 = 52;

pub const PLAYER_R: f32 = 0.3;
pub const WALK_SPEED: f32 = 4.3;
pub const RUN_SPEED: f32 = 6.5;
pub const FIREBALL_COST: f32 = 5.0;
const FIREBALL_SPEED: f32 = 10.0;
pub const CAST_TIME: f32 = 0.32;
/// Free fallback bolt when out of mana.
const EMBER_SPEED: f32 = 13.0;
const EMBER_CAST_TIME: f32 = 0.4;
/// Stamina (D2 style): drains while running, refills while walking or standing.
pub const MAX_STAMINA: f32 = 100.0;
const STAMINA_DRAIN: f32 = 14.0;
const STAMINA_REGEN: f32 = 10.0;
/// After running dry you walk until stamina is back to this much.
pub const WINDED_UNTIL: f32 = 20.0;
/// Food: drains over time (faster while running); empty = starving.
pub const MAX_FOOD: f32 = 100.0;
const FOOD_DRAIN: f32 = 0.35;
const FOOD_DRAIN_RUN: f32 = 0.8;
const STARVE_DPS: f32 = 1.2;
/// Apple, bread, roast: food restored, bonus life.
pub const FOODS: [(&str, f32, f32); 3] = [("APPLE", 20.0, 0.0), ("BREAD", 35.0, 0.0), ("ROAST", 60.0, 10.0)];
/// Chance a normal monster drops a piece of equipment.
pub const ITEM_DROP: f32 = 0.1;
/// Talking range to people in town.
const TALK_RANGE: f32 = 1.8;

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
    /// In town (and in conversations) it talks / chooses instead.
    pub cast: bool,
    pub confirm: bool,
    pub potion_hp: bool,
    pub potion_mp: bool,
    /// Toggle the automap (one-shot).
    pub map: bool,
    /// Toggle run / walk (one-shot).
    pub run_toggle: bool,
    /// Secondary skill held (right mouse is `rmb`; this is pad X / Space).
    pub cast2: bool,
    /// Number key 1-4: pick the secondary skill (one-shot, 0-based).
    pub slot: Option<u8>,
    /// Pad R1: cycle the secondary skill (one-shot).
    pub cycle: bool,
    /// K / hold SELECT: open the skill tree (one-shot).
    pub skills: bool,
    /// I / START: open or close the inventory (one-shot).
    pub inv: bool,
    /// `--cheats` only: F9 gains a level, F10 drops loot (one-shot).
    pub cheat_level: bool,
    pub cheat_loot: bool,
    /// Esc: close a conversation or the map; with nothing open, quit.
    pub cancel: bool,
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
    Eat,
}

pub struct Fireball {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub life: f32,
    pub dmg: f32,
    /// Ember bolt (free, weak, single target, no burn).
    pub ember: bool,
}

#[derive(Clone, Copy, PartialEq)]
pub enum PKind {
    /// Additive fire spark.
    Fire,
    Smoke,
    Bone,
    Blood,
    /// Violet magic (seals, hex bolts, portals).
    Magic,
}

pub struct Particle {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub vx: f32,
    pub vy: f32,
    pub vz: f32,
    pub life: f32,
    pub max: f32,
    pub kind: PKind,
}

pub struct Floater {
    pub x: f32,
    pub y: f32,
    pub t: f32,
    pub text: String,
    pub col: u32,
}

#[derive(Clone, PartialEq, Debug)]
pub enum Drop {
    Health,
    Mana,
    Gold(i32),
    /// Index into FOODS.
    Food(usize),
    /// A Warden's seal (0 bone, 1 plague, 2 hex).
    Seal(usize),
    /// Equipment.
    Item(Box<crate::items::Item>),
}

pub struct Pickup {
    pub x: f32,
    pub y: f32,
    pub kind: Drop,
    pub t: f32,
}

pub struct Decal {
    pub x: f32,
    pub y: f32,
    pub r: f32,
    pub col: u32,
    pub a: f32,
}

pub struct Light {
    pub x: f32,
    pub y: f32,
    pub r: f32,
    pub s: f32,
    pub life: f32,
    pub max: f32,
}

pub struct Player {
    pub x: f32,
    pub y: f32,
    pub hp: f32,
    pub max_hp: f32,
    pub mana: f32,
    pub max_mana: f32,
    pub dir: usize,
    pub anim_t: f32,
    pub moving: bool,
    pub cast_t: f32,
    pub cast_cd: f32,
    /// Length of the current cast animation (fireball or ember).
    pub cast_len: f32,
    pub stamina: f32,
    /// Run toggle (D2's run/walk button).
    pub running: bool,
    /// Ran out of stamina: walking until it recovers.
    pub winded: bool,
    pub food: f32,
    pub hunger_msg: f32,
    pub path: Vec<(f32, f32)>,
    pub goal: Option<(f32, f32)>,
    pub repath: f32,
    pub flash: f32,
    pub hp_pots: i32,
    pub mp_pots: i32,
    pub gold: i32,
    /// Character level and experience.
    pub clvl: u32,
    pub xp: f32,
    /// Fireball damage multiplier (levels and seals).
    pub power: f32,
    /// Walking toward someone to talk to them (mouse click on an NPC).
    pub talk_to: Option<usize>,
    pub skills: crate::skills::Skills,
    /// Inferno is being channelled (counts down when you let go).
    pub inferno: f32,
    pub inferno_t: f32,
    /// Blaze time left, and where the last fire patch was dropped.
    pub blaze_t: f32,
    pub blaze_from: (f32, f32),
    /// Ash Phoenix time left, and the rank it was cast at (for the final burst).
    pub phoenix_t: f32,
    pub phoenix_rank: u8,
    /// Life and mana from levels and seals (gear adds on top: `recalc`).
    pub base_hp: f32,
    pub base_mana: f32,
    pub gear: crate::items::Gear,
    /// Totals of the worn gear's stats.
    pub bonus: crate::items::Bonus,
}

impl Player {
    fn new() -> Self {
        Player {
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
            cast_len: CAST_TIME,
            stamina: MAX_STAMINA,
            running: true,
            winded: false,
            food: MAX_FOOD,
            hunger_msg: 0.0,
            path: vec![],
            goal: None,
            repath: 0.0,
            flash: 0.0,
            hp_pots: 3,
            mp_pots: 3,
            gold: 40,
            clvl: 1,
            xp: 0.0,
            power: 1.0,
            talk_to: None,
            skills: crate::skills::Skills::default(),
            inferno: 0.0,
            inferno_t: 0.0,
            blaze_t: 0.0,
            blaze_from: (0.0, 0.0),
            phoenix_t: 0.0,
            phoenix_rank: 1,
            base_hp: 70.0,
            base_mana: 50.0,
            gear: crate::items::Gear::default(),
            bonus: crate::items::Bonus::default(),
        }
    }

    /// The gear you start with: a plain gnarled staff in hand.
    pub fn starting_gear(&mut self) {
        let staff = crate::items::Item {
            base: crate::items::base_by_key("gnarled").unwrap(),
            rarity: crate::items::Rarity::Normal,
            ilvl: 1,
            name: "GNARLED STAFF".into(),
            stats: vec![(crate::items::Stat::Fire, 5)],
            req: 1,
        };
        self.gear.worn[0] = Some(staff);
        self.recalc();
    }

    /// Applies the worn gear: max life / mana, +skills and fire damage.
    pub fn recalc(&mut self) {
        use crate::items::Stat;
        self.bonus = self.gear.bonus();
        self.max_hp = self.base_hp + self.bonus.get(Stat::Life) as f32;
        self.max_mana = self.base_mana + self.bonus.get(Stat::Mana) as f32;
        self.hp = self.hp.min(self.max_hp);
        self.mana = self.mana.min(self.max_mana);
        self.skills.bonus = self.bonus.get(Stat::Skills).clamp(0, 5) as u8;
        self.skills.gear_fire = self.bonus.frac(Stat::Fire, 300);
    }

    /// Damage taken after armor (armor 50 halves it).
    pub fn armored(&self, dmg: f32) -> f32 {
        dmg * 50.0 / (50.0 + self.bonus.get(crate::items::Stat::Armor).max(0) as f32)
    }
}

/// Experience needed to reach the next character level.
pub fn xp_to_next(clvl: u32) -> f32 {
    60.0 * (clvl as f32).powf(1.6)
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum State {
    Playing,
    Dead(f32),
    /// The Ash King is dead: show the epilogue.
    Victory(f32),
}

#[derive(Default, Debug, Clone)]
pub struct Stats {
    pub casts: u32,
    pub hits: u32,
    pub damage_taken: f32,
    pub embers: u32,
    pub eaten: u32,
    pub starve_damage: f32,
    pub run_time: f32,
    pub levels_entered: u32,
    pub deaths: u32,
    pub talks: u32,
    pub bosses: u32,
}

pub struct Game {
    pub(crate) rng: Rng,
    pub art: Art,
    pub p: Player,
    // ---- the current level (swapped in and out of `parked`) ----
    pub level: LevelId,
    pub level_name: String,
    pub theme: Theme,
    pub tier: f32,
    pub d: Dungeon,
    pub(crate) mobs: Vec<Mob>,
    pub(crate) pickups: Vec<Pickup>,
    pub(crate) decals: Vec<Decal>,
    pub(crate) explored: Vec<bool>,
    pub(crate) props: Vec<Prop>,
    pub(crate) portals: Vec<Portal>,
    pub(crate) npcs: Vec<Npc>,
    pub(crate) safe: Option<(f32, f32, f32, f32)>,
    pub(crate) start: (f32, f32),
    /// The overworld's wake-up spot (town square), remembered while you're in a dungeon.
    town_start: (f32, f32),
    parked: HashMap<LevelId, Level>,
    world_seed: u64,
    // ---- transient effects ----
    pub(crate) balls: Vec<Fireball>,
    pub(crate) novas: Vec<crate::skills::Nova>,
    pub(crate) fire_walls: Vec<crate::skills::FireWallFx>,
    pub(crate) patches: Vec<crate::skills::FirePatch>,
    pub(crate) meteors: Vec<crate::skills::MeteorFx>,
    pub(crate) hydras: Vec<crate::skills::HydraFx>,
    /// The skill tree screen, while open.
    pub tree: Option<crate::skills::TreeUi>,
    /// The inventory screen, while open.
    pub inv: Option<crate::inventory::InvUi>,
    /// Waypoints you have touched (D2 fast travel), and this level's waypoint.
    pub waypoints: Vec<LevelId>,
    pub(crate) waypoint: (f32, f32),
    /// Stepping onto the waypoint opens the travel menu; step off to re-arm it.
    wp_armed: bool,
    /// Gerta's gear for sale; restocked when you come back from a dungeon.
    pub shop_stock: Vec<Option<crate::items::Item>>,
    pub(crate) shop_stale: bool,
    /// HUD skill buttons from the last draw (x, y, w, h): clicking one opens the tree.
    pub(crate) hud_skill_rects: Vec<(i32, i32, i32, i32)>,
    /// HUD bag button from the last draw: clicking it opens the inventory.
    pub(crate) hud_bag: (i32, i32, i32, i32),
    pub(crate) shots: Vec<Shot>,
    pub(crate) hazards: Vec<Hazard>,
    pub(crate) parts: Vec<Particle>,
    pub(crate) floaters: Vec<Floater>,
    pub(crate) lights: Vec<Light>,
    // ---- story and UI ----
    pub quest: Quest,
    pub dialog: Option<Dialog>,
    /// Clickable dialog option rectangles from the last draw (x, y, w, h).
    pub(crate) dlg_rects: Vec<(i32, i32, i32, i32)>,
    pub(crate) message: Option<(String, f32)>,
    pub state: State,
    pub kills: u32,
    pub tick: u32,
    pub sfx: Vec<Sfx>,
    pub(crate) shake: f32,
    /// Mob last hit or hovered (for the top-of-screen health bar).
    pub(crate) focus: Option<usize>,
    pub(crate) focus_t: f32,
    pub(crate) hover: Option<usize>,
    pub(crate) hover_npc: Option<usize>,
    pub(crate) banner_t: f32,
    pub(crate) level_up_t: f32,
    pub(crate) view_h: i32,
    pub(crate) light_ready: bool,
    portal_cd: f32,
    pub(crate) prev: Input,
    pub show_map: bool,
    pub quit: bool,
    /// Set when the game wants the character saved (town, boss kills); main writes it.
    pub save_due: bool,
    pub stats: Stats,
}

impl Game {
    pub fn new(seed: u64, view_h: i32) -> Self {
        let world_seed = seed;
        let lv = world::build_at(LevelId::Overworld, world_seed, 0);
        let mut g = Game {
            rng: Rng::new(seed),
            art: Art::load(),
            p: Player::new(),
            level: LevelId::Overworld,
            level_name: String::new(),
            theme: Theme::Overworld,
            tier: 1.0,
            d: Dungeon::blank(1, 1, Tile::Void),
            mobs: vec![],
            pickups: vec![],
            decals: vec![],
            explored: vec![],
            props: vec![],
            portals: vec![],
            npcs: vec![],
            safe: None,
            start: (0.0, 0.0),
            town_start: world::town_center(),
            parked: HashMap::new(),
            world_seed,
            balls: vec![],
            novas: vec![],
            fire_walls: vec![],
            patches: vec![],
            meteors: vec![],
            hydras: vec![],
            tree: None,
            inv: None,
            waypoints: vec![LevelId::Overworld],
            waypoint: (0.0, 0.0),
            wp_armed: true,
            shop_stock: vec![],
            shop_stale: true,
            hud_skill_rects: vec![],
            hud_bag: (0, 0, 0, 0),
            shots: vec![],
            hazards: vec![],
            parts: vec![],
            floaters: vec![],
            lights: vec![],
            quest: Quest::default(),
            dialog: None,
            dlg_rects: vec![],
            message: None,
            state: State::Playing,
            kills: 0,
            tick: 0,
            sfx: vec![],
            shake: 0.0,
            focus: None,
            focus_t: 0.0,
            hover: None,
            hover_npc: None,
            banner_t: 7.0,
            level_up_t: 0.0,
            view_h,
            light_ready: false,
            portal_cd: 0.0,
            prev: Input::default(),
            show_map: false,
            quit: false,
            save_due: false,
            stats: Stats::default(),
        };
        g.swap_in(lv);
        (g.p.x, g.p.y) = g.start;
        g.p.starting_gear();
        g
    }

    pub fn world_seed(&self) -> u64 {
        self.world_seed
    }

    // ------------------------------------------------------------------ levels

    fn swap_out(&mut self) -> Level {
        Level {
            id: self.level,
            name: std::mem::take(&mut self.level_name),
            theme: self.theme,
            tier: self.tier,
            d: std::mem::replace(&mut self.d, Dungeon::blank(1, 1, Tile::Void)),
            mobs: std::mem::take(&mut self.mobs),
            pickups: std::mem::take(&mut self.pickups),
            decals: std::mem::take(&mut self.decals),
            explored: std::mem::take(&mut self.explored),
            props: std::mem::take(&mut self.props),
            portals: std::mem::take(&mut self.portals),
            npcs: std::mem::take(&mut self.npcs),
            safe: self.safe.take(),
            start: self.start,
        }
    }

    fn swap_in(&mut self, lv: Level) {
        self.level = lv.id;
        self.level_name = lv.name;
        self.theme = lv.theme;
        self.tier = lv.tier;
        self.d = lv.d;
        self.mobs = lv.mobs;
        self.pickups = lv.pickups;
        self.decals = lv.decals;
        self.explored = lv.explored;
        self.props = lv.props;
        self.portals = lv.portals;
        self.npcs = lv.npcs;
        self.safe = lv.safe;
        self.start = lv.start;
        if lv.id == LevelId::Overworld {
            self.town_start = lv.start;
        }
        self.balls.clear();
        self.novas.clear();
        self.fire_walls.clear();
        self.patches.clear();
        self.meteors.clear();
        self.hydras.clear();
        self.tree = None;
        self.shots.clear();
        self.hazards.clear();
        self.parts.clear();
        self.floaters.clear();
        self.lights.clear();
        self.focus = None;
        self.hover = None;
        self.hover_npc = None;
        self.p.path.clear();
        self.p.goal = None;
        self.p.talk_to = None;
        self.dialog = None;
        self.light_ready = false;
        self.portal_cd = 0.8;
        self.banner_t = 3.0;
        self.waypoint = self.find_waypoint();
        self.wp_armed = true;
        if self.quest.difficulty > 0 {
            self.level_name = format!("{} ({})", self.level_name, story::DIFFICULTIES[self.quest.difficulty as usize]);
        }
        if lv.id != LevelId::Overworld {
            self.shop_stale = true;
        }
    }

    /// Gerta's stock: mostly magic gear around your level, now and then a rare.
    pub(crate) fn restock(&mut self) {
        use crate::items::{self, Rarity, SHOP};
        let ilvl = (self.p.clvl as u8 + 1).clamp(2, 30);
        self.shop_stock = (0..SHOP)
            .map(|k| {
                if k >= 12 {
                    return None;
                }
                let r = if self.rng.chance(0.12) {
                    Rarity::Rare
                } else if self.rng.chance(0.75) {
                    Rarity::Magic
                } else {
                    Rarity::Normal
                };
                let mut it = items::roll(ilvl, r, &mut self.rng);
                it.req = it.req.min(self.p.clvl.max(1) + 2);
                Some(it)
            })
            .collect();
        self.shop_stale = false;
    }

    /// Where this level's waypoint stands: beside the town square, or near a floor's way in.
    fn find_waypoint(&self) -> (f32, f32) {
        let base = match self.level {
            LevelId::Overworld => (self.town_start.0 + 3.0, self.town_start.1 + 2.0),
            LevelId::Dungeon(..) => self.portals.iter().find(|p| p.kind == PortalKind::Up).map(|p| (p.x + 2.0, p.y + 1.0)).unwrap_or(self.start),
        };
        let clear = |x: f32, y: f32| {
            !self.d.blocked(x, y, 0.6)
                && self.portals.iter().all(|p| (p.x - x).powi(2) + (p.y - y).powi(2) > 4.0)
                && self.npcs.iter().all(|n| (n.x - x).powi(2) + (n.y - y).powi(2) > 2.0)
        };
        for r in 0..8i32 {
            for dy in -r..=r {
                for dx in -r..=r {
                    if dx.abs().max(dy.abs()) != r {
                        continue;
                    }
                    let (x, y) = ((base.0 + dx as f32).floor() + 0.5, (base.1 + dy as f32).floor() + 0.5);
                    if clear(x, y) {
                        return (x, y);
                    }
                }
            }
        }
        self.start
    }

    /// Waypoint name in the travel menu.
    pub fn waypoint_name(id: LevelId) -> String {
        match id {
            LevelId::Overworld => "HOLLOWMERE".into(),
            LevelId::Dungeon(k, f) => format!("{} - LEVEL {}", DUNGEONS_LIST[k].name, f + 1),
        }
    }

    /// Touching a waypoint activates it; stepping onto it opens the travel menu.
    fn check_waypoint(&mut self) {
        let (wx, wy) = self.waypoint;
        let d2 = (self.p.x - wx).powi(2) + (self.p.y - wy).powi(2);
        if d2 > 1.6 {
            self.wp_armed = true;
            return;
        }
        if d2 > 0.5 || !self.wp_armed {
            return;
        }
        self.wp_armed = false;
        if !self.waypoints.contains(&self.level) {
            self.waypoints.push(self.level);
            self.sfx.push(Sfx::Descend);
            self.say("WAYPOINT ACTIVATED".into());
            self.save_due = true;
            for _ in 0..24 {
                self.spray_at(wx, wy, PKind::Magic, 6.0);
            }
        }
        let mut options: Vec<(String, Act)> = self
            .waypoints
            .iter()
            .filter(|id| **id != self.level)
            .map(|&id| (Self::waypoint_name(id), Act::Travel(id)))
            .collect();
        if options.is_empty() {
            return;
        }
        options.sort_by_key(|o| match o.1 {
            Act::Travel(LevelId::Overworld) => 0,
            Act::Travel(LevelId::Dungeon(k, f)) => 1 + k * 10 + f,
            _ => 999,
        });
        options.push(("STAY HERE".into(), Act::Close));
        let mut d = Dialog { name: "WAYPOINT", pages: vec!["THE RUNES HUM. WHERE WILL YOU GO?".into()], page: 0, options: vec![], sel: 0, advance_to: None, heals: false, last_options: vec![] };
        d.options = options;
        self.dialog = Some(d);
    }

    /// Rebuilds the overworld at the current difficulty (after loading a Nightmare / Hell save).
    pub(crate) fn rebuild_world(&mut self) {
        self.parked.clear();
        let lv = world::build_at(LevelId::Overworld, self.world_seed, self.quest.difficulty);
        self.swap_in(lv);
        (self.p.x, self.p.y) = self.town_start;
    }

    /// Nightmare / Hell: the world is rebuilt harder, the quests start over, your hero carries on.
    pub(crate) fn next_difficulty(&mut self) {
        let d = (self.quest.difficulty + 1).min(2);
        self.quest = Quest { stage: 1, seals: [false; 3], difficulty: d };
        self.parked.clear();
        self.waypoints = vec![LevelId::Overworld];
        self.shop_stale = true;
        let lv = world::build_at(LevelId::Overworld, self.world_seed, d);
        self.swap_in(lv);
        (self.p.x, self.p.y) = self.town_start;
        self.p.hp = self.p.max_hp;
        self.p.mana = self.p.max_mana;
        self.sfx.push(Sfx::Descend);
        self.say(format!("{} BEGINS. SLAY THE THREE WARDENS AGAIN", story::DIFFICULTIES[d as usize]));
        self.save_due = true;
    }

    /// Waypoint travel: you arrive standing on the other waypoint.
    pub(crate) fn travel(&mut self, id: LevelId) {
        self.dialog = None;
        if id != self.level {
            self.go_to(id, None);
        }
        (self.p.x, self.p.y) = self.waypoint;
        self.wp_armed = false;
        for _ in 0..24 {
            let (x, y) = (self.p.x, self.p.y);
            self.spray_at(x, y, PKind::Magic, 8.0);
        }
    }

    /// Moves to another level and places the player at the matching entrance.
    pub fn go_to(&mut self, id: LevelId, from: Option<LevelId>) {
        let cur = self.swap_out();
        self.parked.insert(cur.id, cur);
        let lv = match self.parked.remove(&id) {
            Some(lv) => lv,
            None => world::build_at(id, self.world_seed, self.quest.difficulty),
        };
        self.swap_in(lv);
        self.stats.levels_entered += 1;
        if id == LevelId::Overworld {
            self.save_due = true;
        }
        self.sfx.push(Sfx::Descend);
        // Where do we arrive?
        let spot = match (id, from) {
            (LevelId::Overworld, Some(LevelId::Dungeon(k, _))) => self.portal_spot(PortalKind::Entrance(k)),
            (LevelId::Dungeon(_, f), Some(LevelId::Dungeon(_, g))) if g > f => self.portal_spot(PortalKind::Down),
            (LevelId::Dungeon(..), _) => self.portal_spot(PortalKind::Up),
            _ => None,
        };
        let (x, y) = spot.unwrap_or(self.start);
        self.p.x = x;
        self.p.y = y;
    }

    /// A walkable spot next to a portal of this kind (so you don't land back on it).
    fn portal_spot(&self, kind: PortalKind) -> Option<(f32, f32)> {
        let p = self.portals.iter().find(|p| p.kind == kind)?;
        for (dx, dy) in [(0.0, 1.2), (1.2, 0.0), (-1.2, 0.0), (0.0, -1.2), (1.0, 1.0), (-1.0, 1.0), (1.0, -1.0), (-1.0, -1.0)] {
            let (x, y) = (p.x + dx, p.y + dy);
            if !self.d.blocked(x, y, PLAYER_R + 0.05) {
                return Some((x, y));
            }
        }
        Some((p.x, p.y))
    }

    pub fn in_safe(&self, x: f32, y: f32) -> bool {
        self.safe_contains(x, y, 0.0)
    }

    pub fn safe_contains(&self, x: f32, y: f32, pad: f32) -> bool {
        match self.safe {
            Some((x0, y0, x1, y1)) => x > x0 - pad && x < x1 + pad && y > y0 - pad && y < y1 + pad,
            None => false,
        }
    }

    // ------------------------------------------------------------------ bot / debug helpers

    /// Nearest living monster for the test bot: (x, y, distance, in line of sight).
    pub fn bot_target(&self) -> Option<(f32, f32, f32, bool)> {
        self.mobs
            .iter()
            .filter(|m| m.alive())
            .map(|m| (m.x, m.y, ((m.x - self.p.x).powi(2) + (m.y - self.p.y).powi(2)).sqrt()))
            .min_by(|a, b| a.2.partial_cmp(&b.2).unwrap())
            .map(|(x, y, d)| (x, y, d, self.d.los(self.p.x, self.p.y, x, y)))
    }

    /// Nearest food on the floor (for the test bot).
    pub fn bot_food(&self) -> Option<(f32, f32)> {
        self.pickups
            .iter()
            .filter(|k| matches!(k.kind, Drop::Food(_) | Drop::Seal(_)))
            .map(|k| (k.x, k.y, (k.x - self.p.x).powi(2) + (k.y - self.p.y).powi(2)))
            .min_by(|a, b| a.2.partial_cmp(&b.2).unwrap())
            .map(|(x, y, _)| (x, y))
    }

    /// Where the bot should head next when nothing is in reach: deeper, or into the next dungeon.
    pub fn bot_portal(&self) -> Option<(f32, f32)> {
        let want = |p: &&Portal| match p.kind {
            PortalKind::Down => true,
            PortalKind::Entrance(k) => k == self.bot_dungeon(),
            PortalKind::TownPortal => true,
            PortalKind::Up => self.mobs.iter().all(|m| !m.boss || !m.alive()) && self.portal_kind_exists_not(PortalKind::Down),
        };
        self.portals.iter().find(want).map(|p| (p.x, p.y))
    }

    fn portal_kind_exists_not(&self, k: PortalKind) -> bool {
        !self.portals.iter().any(|p| p.kind == k)
    }

    /// First dungeon whose seal the bot doesn't have yet (the Sanctum once unlocked).
    fn bot_dungeon(&self) -> usize {
        (0..3).find(|&k| !self.quest.seals[k]).unwrap_or(SANCTUM)
    }

    /// Path from the player to (tx, ty), for the bot.
    pub fn bot_path(&self, tx: f32, ty: f32) -> Option<Vec<(f32, f32)>> {
        self.d.path((self.p.x as i32, self.p.y as i32), (tx as i32, ty as i32), 60_000)
    }

    /// Where an NPC of this role stands (for the bot).
    pub fn bot_npc(&self, role: Role) -> Option<(f32, f32)> {
        self.npcs.iter().find(|n| n.role == role).map(|n| (n.x, n.y))
    }

    /// A living boss within `r` tiles (for the bot).
    pub fn boss_alive_near(&self, r: f32) -> bool {
        self.mobs.iter().any(|m| m.boss && m.alive() && (m.x - self.p.x).powi(2) + (m.y - self.p.y).powi(2) < r * r)
    }

    /// A seal is lying on the floor waiting to be picked up.
    pub fn boss_dead_with_loot(&self) -> bool {
        self.pickups.iter().any(|k| matches!(k.kind, Drop::Seal(_)))
    }

    /// Drops one of each food next to the player (staged snapshot only).
    pub fn debug_food_nearby(&mut self) {
        for i in 0..3 {
            let (x, y) = (self.p.x + 0.8 + i as f32 * 0.7, self.p.y - 0.6);
            self.pickups.push(Pickup { x, y, kind: Drop::Food(i), t: 1.0 });
        }
    }

    /// Jumps straight to a level (tests and snapshots).
    pub fn debug_goto(&mut self, id: LevelId) {
        let from = self.level;
        self.go_to(id, Some(from));
        if let LevelId::Dungeon(..) = id {
            if let Some((x, y)) = self.portal_spot(PortalKind::Up) {
                self.p.x = x;
                self.p.y = y;
            }
        }
    }

    /// Puts the player next to the level's boss (tests and snapshots).
    pub fn debug_near_boss(&mut self) -> bool {
        let Some(b) = self.mobs.iter().find(|m| m.boss && m.alive()) else { return false };
        let (bx, by) = (b.x, b.y);
        for (dx, dy) in [(-3.0, 0.0), (0.0, -3.0), (3.0, 0.0), (0.0, 3.0), (-2.0, -2.0)] {
            if !self.d.blocked(bx + dx, by + dy, PLAYER_R) && self.d.los(bx + dx, by + dy, bx, by) {
                self.p.x = bx + dx;
                self.p.y = by + dy;
                return true;
            }
        }
        false
    }

    /// Moves the player to a free spot about `dist` tiles from (x, y) with line of sight (snapshots).
    pub fn debug_place_near(&mut self, x: f32, y: f32, dist: f32) {
        for k in 0..16 {
            let a = k as f32 / 16.0 * std::f32::consts::TAU;
            let (px, py) = (x + a.cos() * dist, y + a.sin() * dist);
            if !self.d.blocked(px, py, PLAYER_R) && self.d.los(px, py, x, y) {
                self.p.x = px;
                self.p.y = py;
                return;
            }
        }
    }

    /// Kills the level's boss outright (tests).
    pub fn debug_kill_boss(&mut self) -> bool {
        let Some(i) = self.mobs.iter().position(|m| m.boss && m.alive()) else { return false };
        self.mobs[i].hp = 0.0;
        self.kill(i);
        true
    }

    /// Opens a conversation with the first NPC of this role (tests and snapshots).
    pub fn debug_talk(&mut self, role: Role) -> bool {
        match self.npcs.iter().position(|n| n.role == role) {
            Some(i) => {
                self.open_dialog(i);
                true
            }
            None => false,
        }
    }

    /// Picks up every pickup on the level (tests).
    #[cfg(test)]
    pub fn debug_collect_all(&mut self) {
        for k in self.pickups.iter_mut() {
            k.x = self.p.x;
            k.y = self.p.y;
            k.t = 1.0;
        }
        self.p.food = 50.0;
        self.collect_pickups();
    }

    /// True while fire is on screen (for picking interesting snapshot frames).
    /// Which music loop fits where you are.
    pub fn music_track(&self) -> crate::music::Track {
        use crate::music::Track;
        if self.mobs.iter().any(|m| m.boss && m.alive() && m.state != MobState::Idle) {
            Track::Boss
        } else if self.in_safe(self.p.x, self.p.y) || matches!(self.state, State::Victory(_)) {
            Track::Town
        } else if self.level == LevelId::Overworld {
            Track::Wilds
        } else {
            Track::Dungeon
        }
    }

    pub fn fire_active(&self) -> bool {
        !self.balls.is_empty() && !self.lights.is_empty()
    }

    pub fn alive_mobs(&self) -> usize {
        self.mobs.iter().filter(|m| m.alive()).count()
    }

    #[cfg(test)]
    pub fn boss_alive(&self) -> bool {
        self.mobs.iter().any(|m| m.boss && m.alive())
    }

    // ------------------------------------------------------------------ update

    pub fn update(&mut self, inp: &Input) {
        self.tick += 1;
        self.banner_t = (self.banner_t - DT).max(0.0);
        self.level_up_t = (self.level_up_t - DT).max(0.0);
        self.shake = (self.shake - DT * 6.0).max(0.0);
        self.portal_cd = (self.portal_cd - DT).max(0.0);
        if let Some((_, t)) = self.message.as_mut() {
            *t -= DT;
            if *t <= 0.0 {
                self.message = None;
            }
        }
        let pressed = |now: bool, before: bool| now && !before;
        let (p_cast, p_lmb, p_confirm) = (pressed(inp.cast, self.prev.cast), pressed(inp.lmb, self.prev.lmb), inp.confirm);
        match self.state {
            State::Dead(t) => {
                self.state = State::Dead(t + DT);
                if t > 1.5 && (inp.confirm || p_cast) {
                    self.respawn();
                }
            }
            State::Victory(t) => {
                self.state = State::Victory(t + DT);
                if t > 3.0 && (inp.confirm || p_cast || p_lmb) {
                    self.state = State::Playing;
                }
            }
            State::Playing => {}
        }
        if self.tree.is_some() {
            // The world waits while you plan your skills.
            self.update_tree(inp, p_cast || p_confirm, p_lmb);
            self.prev = inp.clone();
            return;
        }
        if self.inv.is_some() {
            self.update_inventory(inp, p_confirm, p_lmb);
            self.prev = inp.clone();
            return;
        }
        if inp.skills && self.dialog.is_none() && self.state == State::Playing {
            self.open_tree();
            self.prev = inp.clone();
            return;
        }
        if self.state == State::Playing && (inp.cheat_level || inp.cheat_loot) {
            self.cheat(inp.cheat_level);
        }
        if inp.inv && self.dialog.is_none() && self.state == State::Playing {
            self.open_inventory();
            self.prev = inp.clone();
            return;
        }
        if inp.cancel {
            if self.dialog.is_some() {
                self.dialog = None;
            } else if self.show_map {
                self.show_map = false;
            } else {
                self.quit = true;
            }
        }
        if inp.map {
            self.show_map = !self.show_map;
        }
        if self.dialog.is_some() {
            self.update_dialog(inp, p_cast || p_confirm, p_lmb);
        } else if self.state == State::Playing {
            self.update_player(inp, p_cast, p_lmb);
            self.check_portals();
            if self.dialog.is_none() && self.state == State::Playing {
                self.check_waypoint();
            }
        }
        self.update_npcs();
        self.explore();
        self.update_mobs();
        self.update_shots();
        self.update_hazards();
        self.update_balls();
        self.update_novas();
        self.update_fire_ground();
        self.update_big_fire();
        self.update_world();
        self.prev = inp.clone();
    }

    /// Mouse position in world coordinates.
    pub(crate) fn mouse_world(&self, m: (i32, i32)) -> (f32, f32) {
        let (ox, oy) = self.cam_origin();
        let (wx, wy) = iso::to_world(m.0 as f32 - ox, m.1 as f32 - oy);
        (wx + self.p.x, wy + self.p.y)
    }

    /// Screen position of the player's feet.
    pub(crate) fn cam_origin(&self) -> (f32, f32) {
        (crate::gfx::SW as f32 * 0.5, ((self.view_h - HUD_H) as f32 * 0.5 + 18.0).round())
    }

    fn respawn(&mut self) {
        self.stats.deaths += 1;
        let lost = self.p.gold / 10;
        self.p.gold -= lost;
        self.p.hp = self.p.max_hp;
        self.p.mana = self.p.max_mana;
        self.p.food = self.p.food.max(60.0);
        self.p.stamina = MAX_STAMINA;
        self.p.winded = false;
        self.state = State::Playing;
        // Monsters that were chasing you lose interest.
        for m in self.mobs.iter_mut() {
            if m.alive() && m.state != MobState::Idle {
                m.state = MobState::Idle;
            }
        }
        let from = self.level;
        if from != LevelId::Overworld {
            self.go_to(LevelId::Overworld, None);
        }
        (self.p.x, self.p.y) = self.town_start;
        if lost > 0 {
            self.say(format!("YOU WAKE IN HOLLOWMERE. LOST {lost} GOLD."));
        } else {
            self.say("YOU WAKE IN HOLLOWMERE.".into());
        }
    }

    /// Greets a returning player (after loading a save).
    pub fn welcome_back(&mut self) {
        self.stats.levels_entered = 1; // skip the first-visit title card
        let msg = format!("WELCOME BACK. CHAR LEVEL {}, {}/3 SEALS", self.p.clvl, self.quest.seal_count());
        self.say(msg);
    }

    pub(crate) fn say(&mut self, text: String) {
        self.message = Some((text, 4.0));
    }

    fn check_portals(&mut self) {
        if self.portal_cd > 0.0 {
            return;
        }
        let (px, py) = (self.p.x, self.p.y);
        let Some(kind) = self.portals.iter().find(|p| (p.x - px).powi(2) + (p.y - py).powi(2) < 0.55 * 0.55).map(|p| p.kind) else { return };
        let here = self.level;
        match kind {
            PortalKind::Entrance(k) => {
                if k == SANCTUM && self.quest.stage < 2 {
                    self.portal_cd = 2.0;
                    let n = self.quest.seal_count();
                    self.say(format!("A WALL OF ASH BARS THE GATE. ({n}/3 SEALS)"));
                    return;
                }
                self.go_to(LevelId::Dungeon(k, 0), Some(here));
            }
            PortalKind::Up => match here {
                LevelId::Dungeon(k, 0) => self.go_to(LevelId::Overworld, Some(LevelId::Dungeon(k, 0))),
                LevelId::Dungeon(k, f) => self.go_to(LevelId::Dungeon(k, f - 1), Some(here)),
                LevelId::Overworld => {}
            },
            PortalKind::Down => {
                if let LevelId::Dungeon(k, f) = here {
                    self.go_to(LevelId::Dungeon(k, f + 1), Some(here));
                }
            }
            PortalKind::TownPortal => {
                self.go_to(LevelId::Overworld, None);
                (self.p.x, self.p.y) = self.town_start;
            }
        }
    }

    // ------------------------------------------------------------------ people & conversations

    fn update_npcs(&mut self) {
        let (px, py) = (self.p.x, self.p.y);
        let talking = self.dialog.is_some();
        for i in 0..self.npcs.len() {
            let r1 = self.rng.f();
            let r2 = self.rng.f();
            let n = &mut self.npcs[i];
            n.moving = false;
            let near = (n.x - px).powi(2) + (n.y - py).powi(2) < 9.0;
            if near || talking {
                // Face the player when they're close.
                if near {
                    n.dir = iso::dir8(px - n.x, py - n.y);
                }
                continue;
            }
            if !matches!(n.role, Role::Villager(_)) {
                continue;
            }
            n.wander.2 -= DT;
            if n.wander.2 <= 0.0 {
                let a = r1 * std::f32::consts::TAU;
                // Stroll, but drift back toward home.
                let (hx, hy) = (n.home.0 - n.x, n.home.1 - n.y);
                let (wx, wy) = if hx * hx + hy * hy > 9.0 { (hx, hy) } else { (a.cos(), a.sin()) };
                let l = (wx * wx + wy * wy).sqrt().max(0.01);
                n.wander = if r2 < 0.45 { (0.0, 0.0, 1.5 + r2 * 3.0) } else { (wx / l, wy / l, 1.0 + r2 * 2.0) };
            }
            if n.wander.0 != 0.0 || n.wander.1 != 0.0 {
                let (wx, wy) = (n.wander.0, n.wander.1);
                let (mut x, mut y) = (n.x, n.y);
                move_circle(&self.d, &mut x, &mut y, wx * 1.1 * DT, wy * 1.1 * DT, 0.3);
                n.x = x;
                n.y = y;
                n.dir = iso::dir8(wx, wy);
                n.moving = true;
                n.anim_t += DT * 0.7;
            }
        }
    }

    pub(crate) fn open_dialog(&mut self, i: usize) {
        let role = self.npcs[i].role;
        let mut d = story::talk(role, &self.quest);
        if d.heals {
            // Aldric can also make you forget your skills, for a price.
            let price = 50 * self.p.clvl as i32;
            d.options = vec![(format!("FORGET MY SKILLS  {price} GOLD"), Act::Respec(price)), ("FAREWELL".into(), Act::Close)];
        }
        if d.heals {
            self.p.hp = self.p.max_hp;
            self.p.mana = self.p.max_mana;
            self.p.stamina = MAX_STAMINA;
            self.p.winded = false;
            self.sfx.push(Sfx::Drink);
        }
        self.stats.talks += 1;
        self.p.path.clear();
        self.p.goal = None;
        self.p.talk_to = None;
        self.dialog = Some(d);
    }

    /// Conversation controls: up/down or the mouse pick an option, cast / confirm / click chooses.
    fn update_dialog(&mut self, inp: &Input, choose: bool, click: bool) {
        let up = inp.move_y < -0.5 && self.prev.move_y >= -0.5;
        let down = inp.move_y > 0.5 && self.prev.move_y <= 0.5;
        let mut act = None;
        {
            let d = self.dialog.as_mut().unwrap();
            let n = d.options.len().max(1);
            if up {
                d.sel = (d.sel + n - 1) % n;
            }
            if down {
                d.sel = (d.sel + 1) % n;
            }
            let mut hovered = None;
            if let Some((mx, my)) = inp.mouse {
                hovered = self.dlg_rects.iter().position(|&(x, y, w, h)| mx >= x && mx < x + w && my >= y && my < y + h);
                if let Some(i) = hovered {
                    if inp.mouse != self.prev.mouse {
                        d.sel = i;
                    }
                }
            }
            if choose || (click && hovered.is_some()) {
                let sel = if click { hovered.unwrap_or(d.sel) } else { d.sel };
                act = d.options.get(sel).map(|o| o.1);
            }
            if inp.run_toggle {
                act = Some(Act::Close);
            }
        }
        match act {
            Some(Act::Next) => {
                let d = self.dialog.as_mut().unwrap();
                d.page += 1;
                d.refresh_options();
            }
            Some(Act::Close) => {
                if let Some(stage) = self.dialog.as_ref().and_then(|d| d.advance_to) {
                    self.advance_quest(stage);
                    self.save_due = true;
                }
                self.dialog = None;
            }
            Some(Act::Buy(w)) => self.buy(w),
            Some(Act::Travel(id)) => self.travel(id),
            Some(Act::NextDifficulty) => {
                self.dialog = None;
                self.next_difficulty();
            }
            Some(Act::Shop) => {
                if self.shop_stale || self.shop_stock.is_empty() {
                    self.restock();
                }
                self.open_inventory();
                self.inv.as_mut().unwrap().shop = true;
            }
            Some(Act::Respec(price)) => {
                if self.p.gold < price {
                    self.say("NOT ENOUGH GOLD".into());
                } else {
                    self.p.gold -= price;
                    let n = self.p.skills.respec();
                    self.sfx.push(Sfx::Descend);
                    self.say(format!("YOUR SKILLS ARE FORGOTTEN: {n} POINTS RETURNED"));
                    self.dialog = None;
                    self.save_due = true;
                }
            }
            None => {}
        }
    }

    fn advance_quest(&mut self, stage: u8) {
        if stage > self.quest.stage {
            self.quest.stage = stage;
            self.sfx.push(Sfx::Pickup);
            match stage {
                1 => self.say("NEW QUEST: SLAY THE THREE WARDENS".into()),
                2 => self.say("THE SANCTUM GATE IS UNSEALED".into()),
                _ => {}
            }
        }
    }

    fn buy(&mut self, w: Ware) {
        let price = w.price();
        if self.p.gold < price {
            self.say("NOT ENOUGH GOLD".into());
            return;
        }
        self.p.gold -= price;
        self.sfx.push(Sfx::Pickup);
        match w {
            Ware::HealthPotion => self.p.hp_pots += 1,
            Ware::ManaPotion => self.p.mp_pots += 1,
            Ware::Bread => self.p.food = (self.p.food + FOODS[1].1).min(MAX_FOOD),
            Ware::Roast => {
                self.p.food = (self.p.food + FOODS[2].1).min(MAX_FOOD);
                self.p.hp = (self.p.hp + FOODS[2].2).min(self.p.max_hp);
            }
        }
        let name = match w {
            Ware::HealthPotion => "HEALING POTION",
            Ware::ManaPotion => "MANA POTION",
            Ware::Bread => "BREAD - EATEN",
            Ware::Roast => "ROAST - EATEN",
        };
        self.say(format!("BOUGHT {name}"));
    }

    fn nearest_npc(&self, range: f32) -> Option<usize> {
        self.npcs
            .iter()
            .enumerate()
            .map(|(i, n)| (i, (n.x - self.p.x).powi(2) + (n.y - self.p.y).powi(2)))
            .filter(|&(_, d2)| d2 < range * range)
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .map(|v| v.0)
    }

    pub(crate) fn pick_npc(&self, m: (i32, i32)) -> Option<usize> {
        let (ox, oy) = self.cam_origin();
        let (mx, my) = (m.0 as f32, m.1 as f32);
        self.npcs.iter().enumerate().find_map(|(i, n)| {
            let (sx, sy) = iso::to_screen(n.x - self.p.x, n.y - self.p.y);
            let (sx, sy) = (sx + ox, sy + oy);
            ((mx - sx).abs() < 12.0 && my < sy + 4.0 && my > sy - 50.0).then_some(i)
        })
    }

    // ------------------------------------------------------------------ player

    fn update_player(&mut self, inp: &Input, p_cast: bool, p_lmb: bool) {
        let p = &mut self.p;
        p.flash = (p.flash - DT).max(0.0);
        use crate::items::Stat;
        // Faster cast rate: cast animations and recoveries run quicker.
        let fcr = 1.0 + p.bonus.frac(Stat::Cast, 60);
        p.cast_cd = (p.cast_cd - DT * fcr).max(0.0);
        p.cast_t = (p.cast_t - DT * fcr).max(0.0);
        let mregen = 1.0 + p.bonus.frac(Stat::ManaRegen, 200);
        p.mana = (p.mana + 2.2 * p.skills.regen_mult() * mregen * DT).min(p.max_mana);
        p.inferno = (p.inferno - DT).max(0.0);
        let starving = p.food <= 0.0;
        if !starving {
            p.hp = (p.hp + (0.4 + p.bonus.get(Stat::LifeRegen) as f32) * DT).min(p.max_hp);
        }
        p.hunger_msg = (p.hunger_msg - DT).max(0.0);
        if inp.run_toggle {
            p.running = !p.running;
        }
        let in_town = self.in_safe(self.p.x, self.p.y);
        if starving && !in_town {
            self.stats.starve_damage += STARVE_DPS * DT;
            self.p.hp -= STARVE_DPS * DT;
            if self.p.hunger_msg <= 0.0 {
                self.p.hunger_msg = 4.0;
                self.floater(self.p.x, self.p.y, "STARVING".into(), rgb(0xff6030));
            }
            if self.p.hp <= 0.0 {
                self.p.hp = 0.0;
                self.state = State::Dead(0.0);
                self.sfx.push(Sfx::Die);
                return;
            }
        } else if self.p.food < 25.0 && self.p.hunger_msg <= 0.0 {
            self.p.hunger_msg = 12.0;
            self.floater(self.p.x, self.p.y, "HUNGRY".into(), rgb(0xe0a040));
        }

        if inp.potion_hp && self.p.hp_pots > 0 && self.p.hp < self.p.max_hp {
            self.p.hp_pots -= 1;
            self.p.hp = (self.p.hp + 40.0 + self.p.max_hp * 0.15).min(self.p.max_hp);
            self.sfx.push(Sfx::Drink);
            self.floater(self.p.x, self.p.y, "HEALED".into(), rgb(0xff5050));
        }
        if inp.potion_mp && self.p.mp_pots > 0 && self.p.mana < self.p.max_mana {
            self.p.mp_pots -= 1;
            self.p.mana = (self.p.mana + 35.0 + self.p.max_mana * 0.15).min(self.p.max_mana);
            self.sfx.push(Sfx::Drink);
            self.floater(self.p.x, self.p.y, "MANA".into(), rgb(0x5080ff));
        }

        // Hovered monster / person (mouse picking against sprite boxes).
        self.hover = inp.mouse.and_then(|m| self.pick_mob(m));
        self.hover_npc = inp.mouse.and_then(|m| self.pick_npc(m));

        self.update_slots(inp);
        if let (Some((mx, my)), true) = (inp.mouse, p_lmb) {
            if self.hud_skill_rects.iter().any(|&(x, y, w, h)| mx >= x && mx < x + w && my >= y && my < y + h) {
                self.open_tree();
                return;
            }
            let (x, y, w, h) = self.hud_bag;
            if mx >= x && mx < x + w && my >= y && my < y + h {
                self.open_inventory();
                return;
            }
        }
        let on_hud = inp.mouse.map_or(false, |(_, my)| my >= self.view_h - HUD_H);

        // ---- talking (town) ----
        if let (Some(i), true) = (self.hover_npc, p_lmb) {
            self.p.talk_to = Some(i);
        }
        if let Some(i) = self.p.talk_to {
            let n = &self.npcs[i];
            if (n.x - self.p.x).powi(2) + (n.y - self.p.y).powi(2) < TALK_RANGE * TALK_RANGE {
                self.open_dialog(i);
                return;
            }
        }
        if in_town && (p_cast || inp.confirm) {
            if let Some(i) = self.nearest_npc(TALK_RANGE) {
                self.open_dialog(i);
                return;
            }
        }

        // ---- casting (not in town) ----
        // Primary skill: left click on a monster / Shift+click / pad A. Secondary: right click / X / Space.
        let mut cast_at: Option<(f32, f32)> = None;
        let mut use_secondary = false;
        if !in_town && !on_hud {
            if let Some(m) = inp.mouse {
                let target = self.hover.map(|i| (self.mobs[i].x, self.mobs[i].y)).unwrap_or_else(|| self.mouse_world(m));
                if inp.rmb {
                    cast_at = Some(target);
                    use_secondary = true;
                } else if inp.lmb && (inp.stand || self.hover.is_some()) {
                    cast_at = Some(target);
                }
            }
            if inp.cast || inp.cast2 {
                use_secondary = inp.cast2 && !inp.cast;
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
        }
        if let Some((tx, ty)) = cast_at {
            self.p.path.clear();
            self.p.goal = None;
            self.p.talk_to = None;
            let (dx, dy) = (tx - self.p.x, ty - self.p.y);
            if dx * dx + dy * dy > 0.01 {
                self.p.dir = iso::dir8(dx, dy);
            }
            let skill = if use_secondary { self.p.skills.secondary } else { self.p.skills.primary };
            self.cast_skill(skill, tx, ty);
        }
        if self.p.inferno <= 0.0 {
            self.p.inferno_t = 0.0;
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
            self.p.talk_to = None;
        } else if cast_at.is_none() {
            let talk_goal = self.p.talk_to.map(|i| (self.npcs[i].x, self.npcs[i].y));
            let want = if let Some(g) = talk_goal {
                Some(g)
            } else if let (Some(m), true) = (inp.mouse, inp.lmb && !inp.stand && self.hover.is_none() && !on_hud) {
                Some(self.mouse_world(m))
            } else {
                None
            };
            if let Some(goal) = want {
                self.p.repath -= DT;
                let changed = self.p.goal.map_or(true, |g| (g.0 - goal.0).abs() + (g.1 - goal.1).abs() > 0.5);
                if changed || self.p.repath <= 0.0 {
                    self.p.goal = Some(goal);
                    self.p.repath = 0.25;
                    let from = (self.p.x.floor() as i32, self.p.y.floor() as i32);
                    let to = (goal.0.floor() as i32, goal.1.floor() as i32);
                    self.p.path = self.d.path(from, to, 6000).unwrap_or_default();
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
            } else if let Some(goal) = self.p.goal.filter(|_| inp.lmb || talk_goal.is_some()) {
                // Clicked somewhere unreachable: walk straight at it and slide along walls.
                let (dx, dy) = (goal.0 - self.p.x, goal.1 - self.p.y);
                let l = (dx * dx + dy * dy).sqrt();
                if l > 0.15 {
                    mv = (dx / l, dy / l);
                }
            }
        }
        let casting = self.p.cast_t > 0.0 && self.p.cast_len >= CAST_TIME;
        self.p.moving = mv.0 != 0.0 || mv.1 != 0.0;
        let run = self.p.running && !self.p.winded && self.p.moving && !casting;
        let starving = self.p.food <= 0.0;
        let gear = self.p.bonus;
        if run {
            self.p.stamina -= STAMINA_DRAIN * (1.0 - gear.frac(crate::items::Stat::Stamina, 75)) * DT;
            self.stats.run_time += DT;
            if self.p.stamina <= 0.0 {
                self.p.stamina = 0.0;
                self.p.winded = true;
            }
        } else {
            let regen = if starving { STAMINA_REGEN * 0.5 } else { STAMINA_REGEN };
            self.p.stamina = (self.p.stamina + regen * DT).min(MAX_STAMINA);
            if self.p.winded && self.p.stamina >= WINDED_UNTIL {
                self.p.winded = false;
            }
        }
        // Hunger doesn't tick in the safety of town.
        if !in_town {
            let drain = if run { FOOD_DRAIN_RUN } else { FOOD_DRAIN } * (1.0 - gear.frac(crate::items::Stat::Hunger, 75));
            self.p.food = (self.p.food - drain * DT).max(0.0);
        }
        let base = if run { RUN_SPEED } else { WALK_SPEED };
        let base = if self.p.phoenix_t > 0.0 { base * 1.4 } else { base };
        let base = base * (1.0 + gear.frac(crate::items::Stat::Move, 50));
        let speed = if casting { base * 0.25 } else { base };
        if self.p.moving {
            if !casting {
                self.p.dir = iso::dir8(mv.0, mv.1);
            }
            let (mut x, mut y) = (self.p.x, self.p.y);
            move_circle(&self.d, &mut x, &mut y, mv.0 * speed * DT, mv.1 * speed * DT, PLAYER_R);
            self.p.x = x;
            self.p.y = y;
            self.p.anim_t += DT * speed / WALK_SPEED;
        } else {
            self.p.anim_t = 0.0;
        }
        self.collect_pickups();
    }

    fn collect_pickups(&mut self) {
        let (px, py) = (self.p.x, self.p.y);
        let full = self.p.food > MAX_FOOD - 8.0;
        let mut bag_free = self.p.gear.free();
        let mut bag_full = false;
        let mut got = vec![];
        self.pickups.retain(|k| {
            let food_but_full = matches!(k.kind, Drop::Food(_)) && full;
            let near = (k.x - px).powi(2) + (k.y - py).powi(2) < 0.5 && k.t > 0.3;
            let no_room = matches!(k.kind, Drop::Item(_)) && bag_free == 0;
            if near && no_room {
                bag_full = true;
            }
            if near && !food_but_full && !no_room {
                if matches!(k.kind, Drop::Item(_)) {
                    bag_free -= 1;
                }
                got.push(k.kind.clone());
                false
            } else {
                true
            }
        });
        if bag_full && self.message.is_none() {
            self.say("YOUR BAG IS FULL. SELL OR DROP SOMETHING (I)".into());
        }
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
                    let n = (n as f32 * (1.0 + self.p.bonus.frac(crate::items::Stat::Gold, 300))).round() as i32;
                    self.p.gold += n;
                    self.floater(px, py, format!("{n} GOLD"), rgb(0xe8c050));
                }
                Drop::Food(i) => {
                    let (name, food, life) = FOODS[i];
                    self.p.food = (self.p.food + food).min(MAX_FOOD);
                    self.p.hp = (self.p.hp + life).min(self.p.max_hp);
                    self.p.hunger_msg = 0.0;
                    self.stats.eaten += 1;
                    self.sfx.pop();
                    self.sfx.push(Sfx::Eat);
                    self.floater(px, py, name.into(), rgb(0xe0b060));
                }
                Drop::Item(it) => {
                    self.floater(px, py, it.name.clone(), it.col());
                    let _ = self.p.gear.add(*it);
                }
                Drop::Seal(i) => {
                    self.quest.seals[i] = true;
                    self.p.skills.points += 1;
                    self.p.base_hp += 15.0;
                    self.p.base_mana += 10.0;
                    self.p.recalc();
                    self.p.power *= 1.15;
                    self.p.hp = self.p.max_hp;
                    self.p.mana = self.p.max_mana;
                    self.sfx.push(Sfx::Descend);
                    let name = ["BONE", "PLAGUE", "HEX"][i];
                    self.floater(px, py, format!("SEAL OF {name}"), rgb(0xc8a0ff));
                    let n = self.quest.seal_count();
                    if n == 3 {
                        self.say("ALL THREE SEALS. RETURN TO ELDER MAREN".into());
                    } else {
                        self.say(format!("THE SEAL'S POWER FLOWS INTO YOU ({n}/3)"));
                    }
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
        let r = if self.level == LevelId::Overworld { 13 } else { 9 };
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
                if self.level == LevelId::Overworld || self.d.los(px, py, cx, cy) || self.d.los(px, py, lx, ly) {
                    self.explored[i] = true;
                }
            }
        }
    }

    pub(crate) fn cast_fireball(&mut self, tx: f32, ty: f32, ember: bool) {
        let p = &mut self.p;
        let fb_rank = p.skills.rank(crate::skills::Skill::Fireball);
        let (len, speed, life) = if ember { (EMBER_CAST_TIME, EMBER_SPEED, 0.7) } else { (CAST_TIME, FIREBALL_SPEED, 1.1) };
        if !ember {
            p.mana -= crate::skills::fireball_mana(fb_rank);
        }
        p.cast_cd = len;
        p.cast_t = len;
        p.cast_len = len;
        let (dx, dy) = (tx - p.x, ty - p.y);
        let l = (dx * dx + dy * dy).sqrt().max(0.001);
        let (ux, uy) = (dx / l, dy / l);
        let (x, y) = (p.x + ux * 0.45, p.y + uy * 0.45);
        let power = p.power * p.skills.fire_mult() * crate::skills::fireball_synergy(p.skills.rank(crate::skills::Skill::Meteor));
        let (lo, hi) = crate::skills::fireball_dmg(fb_rank);
        let dmg = if ember { self.rng.rf(3.0, 5.0) } else { self.rng.rf(lo, hi) } * power;
        self.balls.push(Fireball { x, y, vx: ux * speed, vy: uy * speed, life, dmg, ember });
        if ember {
            self.stats.embers += 1;
        } else {
            self.stats.casts += 1;
        }
        self.sfx.push(Sfx::Cast);
    }

    fn nearest_visible_mob(&self, range: f32) -> Option<usize> {
        self.mobs
            .iter()
            .enumerate()
            .filter(|(_, m)| m.alive())
            .map(|(i, m)| (i, (m.x - self.p.x).powi(2) + (m.y - self.p.y).powi(2)))
            .filter(|&(i, d2)| d2 < range * range && self.d.los(self.p.x, self.p.y, self.mobs[i].x, self.mobs[i].y))
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .map(|v| v.0)
    }

    pub(crate) fn pick_mob(&self, m: (i32, i32)) -> Option<usize> {
        let (ox, oy) = self.cam_origin();
        let mut best: Option<(usize, f32)> = None;
        for (i, mob) in self.mobs.iter().enumerate() {
            if !mob.alive() {
                continue;
            }
            let (sx, sy) = iso::to_screen(mob.x - self.p.x, mob.y - self.p.y);
            let (sx, sy) = (sx + ox, sy + oy);
            let (art, scale, ..) = self.art.char_art(crate::mobs::def(mob.kind).art);
            let h = art.height as f32 * scale;
            let half = if mob.boss { 22.0 } else { 12.0 };
            let (mx, my) = (m.0 as f32, m.1 as f32);
            if (mx - sx).abs() < half && my < sy + 4.0 && my > sy - h {
                let depth = mob.x + mob.y;
                if best.map_or(true, |b| depth > b.1) {
                    best = Some((i, depth));
                }
            }
        }
        best.map(|b| b.0)
    }

    pub(crate) fn hurt_player(&mut self, dmg: f32) {
        if matches!(self.state, State::Dead(_)) {
            return;
        }
        let dmg = self.p.armored(dmg);
        if dmg > 0.0 {
            self.p.hp -= dmg;
            self.stats.damage_taken += dmg;
            self.p.flash = 0.2;
            self.shake = self.shake.max(0.5);
            self.sfx.push(Sfx::Hurt);
            self.floater(self.p.x, self.p.y, format!("{}", dmg as i32), rgb(0xff4040));
        }
        if self.p.hp <= 0.0 {
            self.p.hp = 0.0;
            self.state = State::Dead(0.0);
            self.dialog = None;
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
                if self.mobs.iter().any(|m| m.alive() && (m.x - b.x).powi(2) + (m.y - b.y).powi(2) < (m.r + 0.13).powi(2)) {
                    hit = true;
                }
            }
            if hit {
                booms.push((b.x, b.y, b.dmg, b.ember));
                b.life = -1.0;
            }
        }
        // Trail sparks.
        for i in 0..self.balls.len() {
            let (bx, by, vx, vy) = (self.balls[i].x, self.balls[i].y, self.balls[i].vx, self.balls[i].vy);
            let n = if self.balls[i].ember { 1 } else { 2 };
            for _ in 0..n {
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
        for (x, y, dmg, ember) in booms {
            if ember {
                self.ember_hit(x, y, dmg);
            } else {
                self.explode(x, y, dmg);
            }
        }
    }

    /// Ember Bolt impact: a small puff that hurts only the monster it touched.
    fn ember_hit(&mut self, x: f32, y: f32, dmg: f32) {
        self.lights.push(Light { x, y, r: 70.0, s: 0.7, life: 0.2, max: 0.2 });
        for _ in 0..8 {
            let a = self.rng.f() * std::f32::consts::TAU;
            let s = self.rng.rf(0.8, 2.5);
            let (vz, life) = (self.rng.rf(10.0, 40.0), self.rng.rf(0.2, 0.4));
            self.parts.push(Particle { x, y, z: 20.0, vx: a.cos() * s, vy: a.sin() * s, vz, life, max: life, kind: PKind::Fire });
        }
        let target = self
            .mobs
            .iter()
            .enumerate()
            .filter(|(_, m)| m.alive())
            .map(|(i, m)| (i, (m.x - x).powi(2) + (m.y - y).powi(2), m.r))
            .filter(|&(_, d2, r)| d2 < (r + 0.3).powi(2))
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .map(|v| v.0);
        let Some(i) = target else { return };
        self.stats.hits += 1;
        self.sfx.push(Sfx::Hit);
        let m = &mut self.mobs[i];
        m.hp -= dmg;
        m.flash = 0.1;
        if m.state == MobState::Idle {
            m.state = MobState::Chase;
        }
        let (mx, my) = (m.x, m.y);
        self.focus = Some(i);
        self.focus_t = 3.0;
        self.floater(mx, my, format!("{}", dmg.round() as i32), rgb(0xffa060));
        if self.mobs[i].hp <= 0.0 {
            self.kill(i);
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
            if !m.alive() {
                continue;
            }
            let d2 = (m.x - x).powi(2) + (m.y - y).powi(2);
            let reach = 1.3 + m.r - 0.32;
            if d2 > reach * reach {
                continue;
            }
            let dmg = if d2 < (0.3 + m.r).powi(2) { dmg } else { dmg * 0.5 };
            hit_any = true;
            let (mx, my, boss, r) = (m.x, m.y, m.boss, m.r);
            let m = &mut self.mobs[i];
            m.hp -= dmg;
            m.flash = 0.12;
            m.burn = 2.0 * self.p.skills.burn_mult();
            // Bosses shrug off most of the stagger.
            m.stun = m.stun.max(if boss { 0.03 } else { 0.15 });
            if m.state == MobState::Idle {
                m.state = MobState::Chase;
            }
            if let (MobState::Attack(_), false) = (m.state, boss) {
                // Getting hit interrupts the swing (D2 hit recovery).
                m.state = MobState::Chase;
                m.cd = m.cd.max(0.3);
            }
            if !boss {
                let (kx, ky) = (mx - x, my - y);
                let kl = (kx * kx + ky * ky).sqrt().max(0.01);
                let (mut nx, mut ny) = (mx, my);
                move_circle(&self.d, &mut nx, &mut ny, kx / kl * 0.25, ky / kl * 0.25, r);
                m.x = nx;
                m.y = ny;
            }
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

    pub(crate) fn kill(&mut self, i: usize) {
        let (x, y, kind, boss, xp) = (self.mobs[i].x, self.mobs[i].y, self.mobs[i].kind, self.mobs[i].boss, self.mobs[i].xp);
        self.mobs[i].state = MobState::Dead(0.0);
        self.mobs[i].hp = 0.0;
        self.kills += 1;
        self.sfx.push(Sfx::Die);
        let pk = if matches!(kind, Kind::Skeleton | Kind::Archer | Kind::BoneWarden | Kind::HexWarden) { PKind::Bone } else { PKind::Blood };
        for _ in 0..if boss { 60 } else { 14 } {
            self.spray(x, y, pk, 22.0);
        }
        if matches!(kind, Kind::Zombie | Kind::PlagueWarden | Kind::Wolf) {
            self.decals.push(Decal { x, y, r: if boss { 1.0 } else { 0.4 }, col: rgb(0x301008), a: 0.5 });
        }
        self.gain_xp(xp);
        {
            use crate::items::Stat;
            let p = &mut self.p;
            p.hp = (p.hp + p.bonus.get(Stat::LifeOnKill) as f32).min(p.max_hp);
            p.mana = (p.mana + p.bonus.get(Stat::ManaOnKill) as f32).min(p.max_mana);
        }
        let (rank, mods) = (self.mobs[i].rank, self.mobs[i].mods);
        self.drop_gear(x, y, boss, kind, rank);
        if mods & crate::mobs::M_FIERY != 0 {
            // Fire enchanted: bursts into flame a moment after it dies.
            let dmg = (self.mobs[i].dmg.1 * 1.4).max(8.0);
            self.hazards.push(crate::mobs::Hazard { x, y, r: 1.7, warn: 0.6, live: 0.0, dps: 0.0, burst: dmg, t: 0.0, fired: false, kind: crate::mobs::HazardKind::Nova });
        }
        // Goblins panic when one of their own falls (like D2's Fallen).
        if kind == Kind::Goblin {
            for m in self.mobs.iter_mut() {
                if m.kind == Kind::Goblin && m.alive() && (m.x - x).powi(2) + (m.y - y).powi(2) < 36.0 {
                    m.flee = 2.5;
                }
            }
        }
        if boss {
            self.stats.bosses += 1;
            self.save_due = true;
            self.shake = 1.0;
            self.lights.push(Light { x, y, r: 260.0, s: 1.5, life: 1.2, max: 1.2 });
            match kind {
                Kind::BoneWarden => self.pickups.push(Pickup { x, y, kind: Drop::Seal(0), t: 0.0 }),
                Kind::PlagueWarden => self.pickups.push(Pickup { x, y, kind: Drop::Seal(1), t: 0.0 }),
                Kind::HexWarden => self.pickups.push(Pickup { x, y, kind: Drop::Seal(2), t: 0.0 }),
                Kind::AshKing => {
                    self.quest.stage = 3;
                    self.state = State::Victory(0.0);
                    self.dialog = None;
                }
                _ => {}
            }
            for k in 0..3 {
                self.pickups.push(Pickup { x: x + k as f32 * 0.5 - 0.5, y: y + 0.6, kind: Drop::Gold(self.rng.range(40, 90)), t: 0.0 });
            }
            self.pickups.push(Pickup { x: x - 0.6, y: y - 0.4, kind: Drop::Health, t: 0.0 });
            // A way home.
            let (mut px, mut py) = (x + 1.5, y + 1.5);
            if self.d.blocked(px, py, 0.4) {
                px = x;
                py = y + 1.2;
            }
            self.portals.push(Portal { x: px, y: py, kind: PortalKind::TownPortal });
            let label = crate::mobs::def(kind).label;
            self.say(format!("{label} IS SLAIN"));
            return;
        }
        let r = self.rng.f();
        let drop = if r < 0.12 {
            Some(Drop::Health)
        } else if r < 0.22 {
            Some(Drop::Mana)
        } else if r < 0.32 {
            Some(Drop::Food(if self.rng.chance(0.6) { 0 } else { 1 }))
        } else if r < 0.65 {
            Some(Drop::Gold((self.rng.range(3, 12) as f32 * self.tier) as i32))
        } else {
            None
        };
        if let Some(k) = drop {
            self.pickups.push(Pickup { x, y, kind: k, t: 0.0 });
        }
    }

    /// Test keys (`--cheats`): a character level, or a handful of loot at your feet.
    fn cheat(&mut self, level: bool) {
        if level {
            let need = xp_to_next(self.p.clvl) - self.p.xp;
            self.gain_xp(need.max(0.0) + 0.01);
            return;
        }
        let ilvl = (self.p.clvl as u8).max(crate::items::ilvl_for(self.tier));
        for k in 0..4 {
            let r = [crate::items::Rarity::Magic, crate::items::Rarity::Magic, crate::items::Rarity::Rare, crate::items::Rarity::Rare][k];
            let it = if k == 3 && self.rng.chance(0.3) { crate::items::drop(ilvl, 400, true, &mut self.rng) } else { crate::items::roll(ilvl, r, &mut self.rng) };
            let a = k as f32 * 1.57 + 0.4;
            let (mut x, mut y) = (self.p.x + a.cos() * 1.2, self.p.y + a.sin() * 1.2);
            if self.d.blocked(x, y, 0.2) {
                (x, y) = (self.p.x, self.p.y);
            }
            self.pickups.push(Pickup { x, y, kind: Drop::Item(Box::new(it)), t: 0.0 });
        }
    }

    /// Equipment drops: about one monster in ten drops something; bosses drop their unique
    /// and two magic-or-better items.
    fn drop_gear(&mut self, x: f32, y: f32, boss: bool, kind: Kind, rank: crate::mobs::Rank) {
        use crate::mobs::Rank;
        use crate::items;
        let ilvl = items::ilvl_for(self.tier) + if boss { 3 } else { 0 };
        let mf = self.p.bonus.get(items::Stat::Magic);
        let mut drops = vec![];
        if boss {
            let key = match kind {
                Kind::BoneWarden => "bone",
                Kind::PlagueWarden => "plague",
                Kind::HexWarden => "hex",
                _ => "ashking",
            };
            drops.extend(items::boss_unique(key));
            for _ in 0..2 {
                let mut it = items::drop(ilvl, mf, true, &mut self.rng);
                if it.rarity == items::Rarity::Normal {
                    it = items::roll(ilvl, items::Rarity::Magic, &mut self.rng);
                }
                drops.push(it);
            }
        } else {
            // Champions and elites drop more, and better.
            let (n, chance, boost) = match rank {
                Rank::Normal => (1, ITEM_DROP, false),
                Rank::Minion => (1, 0.2, false),
                Rank::Champion => (1, 0.6, true),
                Rank::Elite => (2, 1.0, true),
            };
            for _ in 0..n {
                if self.rng.chance(chance) {
                    drops.push(items::drop(ilvl + boost as u8, mf, boost, &mut self.rng));
                }
            }
        }
        let n = drops.len();
        for (k, it) in drops.into_iter().enumerate() {
            let a = k as f32 / n.max(1) as f32 * std::f32::consts::TAU + 0.5;
            let r = if n > 1 { 0.9 } else { 0.25 };
            let (mut ix, mut iy) = (x + a.cos() * r, y + a.sin() * r);
            if self.d.blocked(ix, iy, 0.2) {
                (ix, iy) = (x, y);
            }
            self.pickups.push(Pickup { x: ix, y: iy, kind: Drop::Item(Box::new(it)), t: 0.0 });
        }
    }

    fn gain_xp(&mut self, xp: f32) {
        self.p.xp += xp;
        while self.p.xp >= xp_to_next(self.p.clvl) {
            self.p.xp -= xp_to_next(self.p.clvl);
            self.p.clvl += 1;
            self.p.skills.points += 1;
            self.p.base_hp += 8.0;
            self.p.base_mana += 4.0;
            self.p.recalc();
            // Damage grows fast early and slower past level 20 (so Nightmare and Hell still bite).
            self.p.power *= if self.p.clvl <= 20 { 1.07 } else { 1.03 };
            self.p.hp = self.p.max_hp;
            self.p.mana = self.p.max_mana;
            self.level_up_t = 2.5;
            self.sfx.push(Sfx::Descend);
            let (x, y) = (self.p.x, self.p.y);
            self.floater(x, y, format!("LEVEL {}", self.p.clvl), rgb(0xffe080));
            for _ in 0..30 {
                self.spray_at(x, y, PKind::Magic, 20.0);
            }
        }
    }

    pub(crate) fn spray(&mut self, x: f32, y: f32, kind: PKind, z: f32) {
        let a = self.rng.f() * std::f32::consts::TAU;
        let s = self.rng.rf(0.5, 2.5);
        let life = self.rng.rf(0.5, 1.2);
        let vz = self.rng.rf(20.0, 60.0);
        self.parts.push(Particle { x, y, z, vx: a.cos() * s, vy: a.sin() * s, vz, life, max: life, kind });
    }

    /// Particles bursting outward from a point (summons, novas, level-ups).
    pub(crate) fn spray_at(&mut self, x: f32, y: f32, kind: PKind, z: f32) {
        let a = self.rng.f() * std::f32::consts::TAU;
        let s = self.rng.rf(0.5, 3.5);
        let life = self.rng.rf(0.4, 0.9);
        let vz = self.rng.rf(10.0, 50.0);
        self.parts.push(Particle { x, y, z, vx: a.cos() * s, vy: a.sin() * s, vz, life, max: life, kind });
    }

    pub(crate) fn floater(&mut self, x: f32, y: f32, text: String, col: u32) {
        self.floaters.push(Floater { x, y, t: 0.0, text, col });
    }

    fn update_world(&mut self) {
        for p in self.parts.iter_mut() {
            p.life -= DT;
            p.x += p.vx * DT;
            p.y += p.vy * DT;
            p.z += p.vz * DT;
            match p.kind {
                PKind::Fire | PKind::Magic => p.vz += 12.0 * DT,
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
        // Campfires and portals give off sparks.
        if self.tick % 3 == 0 {
            let fires: Vec<(f32, f32)> = self.props.iter().filter(|p| p.kind == world::PropKind::Campfire).map(|p| (p.x, p.y)).collect();
            for (x, y) in fires {
                let (r1, r2) = (self.rng.f() - 0.5, self.rng.f());
                self.parts.push(Particle { x: x + r1 * 0.3, y, z: 6.0, vx: r1 * 0.3, vy: 0.0, vz: 20.0 + r2 * 20.0, life: 0.8, max: 0.8, kind: PKind::Fire });
            }
            let portals: Vec<(f32, f32)> = self.portals.iter().filter(|p| p.kind == PortalKind::TownPortal).map(|p| (p.x, p.y)).collect();
            for (x, y) in portals {
                let (r1, r2) = (self.rng.f() - 0.5, self.rng.f());
                self.parts.push(Particle { x: x + r1 * 0.4, y, z: 4.0 + r2 * 30.0, vx: 0.0, vy: 0.0, vz: 18.0, life: 0.7, max: 0.7, kind: PKind::Magic });
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
}

/// World-space unit vector for a sprite direction.
pub(crate) fn dir_vec(dir: usize) -> (f32, f32) {
    // Screen-space direction for each index, then back to world.
    let a = (90.0 - dir as f32 * 45.0).to_radians();
    iso::screen_dir_to_world(a.cos(), a.sin())
}

/// Moves a circle with wall sliding (x then y).
pub(crate) fn move_circle(d: &Dungeon, x: &mut f32, y: &mut f32, dx: f32, dy: f32, r: f32) {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A game standing at the stairs of the Bone Crypt's first floor, with no monsters.
    fn quiet_game() -> Game {
        let mut g = Game::new(7, crate::gfx::SH_WIDE);
        g.debug_goto(LevelId::Dungeon(0, 0));
        g.mobs.clear();
        g.pickups.clear();
        g.banner_t = 0.0;
        g.portal_cd = 1000.0;
        g
    }

    #[test]
    fn running_is_faster_and_drains_stamina_until_winded() {
        let walk = |running: bool| {
            let mut g = quiet_game();
            g.p.running = running;
            let (x0, y0) = (g.p.x, g.p.y);
            for _ in 0..20 {
                g.update(&Input { move_x: 1.0, ..Input::default() });
            }
            (((g.p.x - x0).powi(2) + (g.p.y - y0).powi(2)).sqrt(), g.p.stamina)
        };
        let (walked, st_walk) = walk(false);
        let (ran, st_run) = walk(true);
        assert!(ran > walked * 1.3, "run {ran} vs walk {walked}");
        assert_eq!(st_walk, MAX_STAMINA);
        assert!(st_run < MAX_STAMINA);

        let mut g = quiet_game();
        g.p.stamina = 1.0;
        for _ in 0..10 {
            g.update(&Input { move_x: 1.0, ..Input::default() });
        }
        assert!(g.p.winded);
        for _ in 0..60 * 3 {
            g.update(&Input::default());
        }
        assert!(!g.p.winded && g.p.stamina >= WINDED_UNTIL);
        g.update(&Input { run_toggle: true, ..Input::default() });
        assert!(!g.p.running);
    }

    #[test]
    fn hunger_starves_and_food_feeds() {
        let mut g = quiet_game();
        g.p.food = 0.0;
        let hp = g.p.hp;
        for _ in 0..60 {
            g.update(&Input::default());
        }
        assert!(g.p.hp < hp - 1.0, "starving hurts");
        g.pickups.push(Pickup { x: g.p.x, y: g.p.y, kind: Drop::Food(2), t: 1.0 });
        g.update(&Input::default());
        assert!(g.p.food > 50.0 && g.pickups.is_empty());
        assert_eq!(g.stats.eaten, 1);
        g.p.food = MAX_FOOD;
        g.pickups.push(Pickup { x: g.p.x, y: g.p.y, kind: Drop::Food(0), t: 1.0 });
        g.update(&Input::default());
        assert_eq!(g.pickups.len(), 1);
        let lv = world::dungeon_floor(0, 0, 11);
        assert!(lv.pickups.iter().filter(|k| matches!(k.kind, Drop::Food(_))).count() >= 3);
    }

    #[test]
    fn ember_bolt_is_free_when_out_of_mana() {
        let mut g = quiet_game();
        g.p.mana = 1.0;
        g.update(&Input { cast: true, ..Input::default() });
        assert_eq!(g.balls.len(), 1);
        assert!(g.balls[0].ember);
        assert!(g.p.mana >= 1.0, "ember costs no mana");
        g.p.mana = 50.0;
        for _ in 0..30 {
            g.update(&Input::default());
        }
        g.update(&Input { cast: true, ..Input::default() });
        assert!(g.balls.iter().any(|b| !b.ember), "fireball again with mana");
        assert!(g.p.mana < 50.0);
    }

    #[test]
    fn stairs_and_entrances_connect_the_world() {
        let mut g = Game::new(3, crate::gfx::SH_WIDE);
        assert_eq!(g.level, LevelId::Overworld);
        assert!(g.in_safe(g.p.x, g.p.y), "start in town");
        // Walk onto the crypt entrance.
        let e = g.portals.iter().find(|p| p.kind == PortalKind::Entrance(0)).map(|p| (p.x, p.y)).unwrap();
        g.p.x = e.0;
        g.p.y = e.1;
        g.portal_cd = 0.0;
        g.update(&Input::default());
        assert_eq!(g.level, LevelId::Dungeon(0, 0));
        assert!(g.d.walkable(g.p.x as i32, g.p.y as i32));
        // Down the stairs, then back up and out.
        let down = g.portals.iter().find(|p| p.kind == PortalKind::Down).map(|p| (p.x, p.y)).unwrap();
        g.mobs.clear();
        g.p.x = down.0;
        g.p.y = down.1;
        g.portal_cd = 0.0;
        g.update(&Input::default());
        assert_eq!(g.level, LevelId::Dungeon(0, 1));
        assert!(g.boss_alive(), "the Bone Warden waits on the bottom floor");
        let up = g.portals.iter().find(|p| p.kind == PortalKind::Up).map(|p| (p.x, p.y)).unwrap();
        g.p.x = up.0;
        g.p.y = up.1;
        g.portal_cd = 0.0;
        g.update(&Input::default());
        assert_eq!(g.level, LevelId::Dungeon(0, 0));
        assert!(g.mobs.is_empty(), "levels remember their state");
        let up = g.portals.iter().find(|p| p.kind == PortalKind::Up).map(|p| (p.x, p.y)).unwrap();
        g.p.x = up.0;
        g.p.y = up.1;
        g.portal_cd = 0.0;
        g.update(&Input::default());
        assert_eq!(g.level, LevelId::Overworld);
        assert!(((g.p.x - e.0).powi(2) + (g.p.y - e.1).powi(2)).sqrt() < 2.5, "back at the crypt door");
    }

    #[test]
    fn the_whole_story_can_be_completed() {
        let mut g = Game::new(5, crate::gfx::SH_WIDE);
        // Meet the elder and read through her story.
        assert!(g.debug_talk(Role::Elder));
        for _ in 0..10 {
            if g.dialog.is_none() {
                break;
            }
            g.update(&Input { confirm: true, ..Input::default() });
            g.update(&Input::default());
        }
        assert!(g.dialog.is_none());
        assert_eq!(g.quest.stage, 1);
        // The Sanctum is sealed.
        let s = g.portals.iter().find(|p| p.kind == PortalKind::Entrance(SANCTUM)).map(|p| (p.x, p.y)).unwrap();
        g.p.x = s.0;
        g.p.y = s.1;
        g.portal_cd = 0.0;
        g.update(&Input::default());
        assert_eq!(g.level, LevelId::Overworld, "ash barrier holds");
        // Slay the three wardens and take their seals.
        for k in 0..3 {
            g.debug_goto(LevelId::Dungeon(k, DUNGEONS[k].floors - 1));
            assert!(g.debug_kill_boss());
            g.debug_collect_all();
            assert!(g.quest.seals[k], "seal {k}");
            assert!(g.portals.iter().any(|p| p.kind == PortalKind::TownPortal));
        }
        assert!(g.p.power > 1.4, "seals and levels made you stronger");
        // Town portal home, then the elder unseals the gate.
        let tp = g.portals.iter().find(|p| p.kind == PortalKind::TownPortal).map(|p| (p.x, p.y)).unwrap();
        g.p.x = tp.0;
        g.p.y = tp.1;
        g.portal_cd = 0.0;
        g.update(&Input::default());
        assert_eq!(g.level, LevelId::Overworld);
        assert!(g.in_safe(g.p.x, g.p.y));
        assert!(g.debug_talk(Role::Elder));
        for _ in 0..10 {
            if g.dialog.is_none() {
                break;
            }
            g.update(&Input { confirm: true, ..Input::default() });
            g.update(&Input::default());
        }
        assert_eq!(g.quest.stage, 2);
        g.p.x = s.0;
        g.p.y = s.1;
        g.portal_cd = 0.0;
        g.update(&Input::default());
        assert_eq!(g.level, LevelId::Dungeon(SANCTUM, 0));
        g.debug_goto(LevelId::Dungeon(SANCTUM, DUNGEONS[SANCTUM].floors - 1));
        assert!(g.debug_kill_boss());
        assert!(matches!(g.state, State::Victory(_)));
        assert_eq!(g.quest.stage, 3);
    }

    #[test]
    fn merchant_sells_and_healer_heals() {
        let mut g = Game::new(5, crate::gfx::SH_WIDE);
        g.p.gold = 100;
        assert!(g.debug_talk(Role::Merchant));
        let pots = g.p.hp_pots;
        g.update(&Input { confirm: true, ..Input::default() }); // first option: healing potion
        assert_eq!(g.p.hp_pots, pots + 1);
        assert_eq!(g.p.gold, 100 - Ware::HealthPotion.price());
        g.update(&Input { cancel: true, ..Input::default() });
        assert!(g.dialog.is_none() && !g.quit);
        g.p.hp = 5.0;
        assert!(g.debug_talk(Role::Healer));
        assert_eq!(g.p.hp, g.p.max_hp);
    }

    #[test]
    fn dying_wakes_you_in_town() {
        let mut g = quiet_game();
        g.p.gold = 100;
        g.hurt_player(10_000.0);
        assert!(matches!(g.state, State::Dead(_)));
        for _ in 0..120 {
            g.update(&Input::default());
        }
        g.update(&Input { confirm: true, ..Input::default() });
        assert_eq!(g.state, State::Playing);
        assert_eq!(g.level, LevelId::Overworld);
        assert!(g.in_safe(g.p.x, g.p.y));
        assert_eq!(g.p.gold, 90);
    }

    /// A quiet dungeon with one zombie placed relative to the player.
    fn with_zombie(dx: f32, dy: f32) -> Game {
        let mut g = quiet_game();
        let mut m = Mob::new(Kind::Zombie, g.p.x + dx, g.p.y + dy, 1.0, &mut g.rng);
        m.hp = 1000.0;
        m.max_hp = 1000.0;
        g.mobs.push(m);
        g
    }

    #[test]
    fn inferno_burns_what_is_in_front() {
        use crate::skills::Skill;
        let mut g = with_zombie(1.5, 0.0);
        let mut behind = Mob::new(Kind::Zombie, g.p.x - 1.5, g.p.y, 1.0, &mut g.rng);
        behind.hp = 1000.0;
        g.mobs.push(behind);
        g.p.skills.rank[Skill::Inferno as usize] = 1;
        g.p.skills.primary = Skill::Inferno;
        g.p.mana = 50.0;
        let (zx, zy) = (g.mobs[0].x, g.mobs[0].y);
        for _ in 0..60 {
            // Aim with the right stick toward the zombie's side of the screen.
            let (sx, sy) = crate::iso::to_screen(zx - g.p.x, zy - g.p.y);
            let l = (sx * sx + sy * sy).sqrt();
            g.mobs[0].x = zx;
            g.mobs[0].y = zy;
            g.mobs[0].state = MobState::Idle;
            g.update(&Input { cast: true, aim_x: sx / l, aim_y: sy / l, ..Input::default() });
        }
        assert!(g.mobs[0].hp < 1000.0 - 10.0, "front zombie hp {}", g.mobs[0].hp);
        assert!(g.mobs[1].hp >= 1000.0 - 0.01, "the one behind is untouched");
        assert!(g.p.mana < 50.0 - 5.0, "inferno drains mana");
    }

    #[test]
    fn fire_nova_hits_and_hurls_back() {
        use crate::skills::Skill;
        let mut g = with_zombie(1.2, 0.0);
        g.p.skills.rank[Skill::FireNova as usize] = 1;
        g.p.skills.secondary = Skill::FireNova;
        g.p.mana = 50.0;
        let d0 = ((g.mobs[0].x - g.p.x).powi(2) + (g.mobs[0].y - g.p.y).powi(2)).sqrt();
        g.update(&Input { cast2: true, ..Input::default() });
        let d1 = ((g.mobs[0].x - g.p.x).powi(2) + (g.mobs[0].y - g.p.y).powi(2)).sqrt();
        assert!(g.mobs[0].hp < 1000.0, "nova hit");
        assert!(d1 > d0 + 0.3, "knocked back {d0} -> {d1}");
        assert!(g.p.mana < 45.0);
    }

    #[test]
    fn warmth_speeds_up_mana_and_points_come_from_levels_and_seals() {
        use crate::skills::Skill;
        let regen = |warmth: u8| {
            let mut g = quiet_game();
            g.p.skills.rank[Skill::Warmth as usize] = warmth;
            g.p.mana = 0.0;
            for _ in 0..60 {
                g.update(&Input::default());
            }
            g.p.mana
        };
        assert!(regen(3) > regen(0) * 1.4);
        let mut g = quiet_game();
        let pts = g.p.skills.points;
        g.gain_xp(xp_to_next(1) + 1.0);
        assert_eq!(g.p.skills.points, pts + 1);
        g.pickups.push(Pickup { x: g.p.x, y: g.p.y, kind: Drop::Seal(0), t: 1.0 });
        g.update(&Input::default());
        assert_eq!(g.p.skills.points, pts + 2);
    }

    #[test]
    fn fire_wall_burns_what_stands_in_it() {
        use crate::skills::Skill;
        let mut g = with_zombie(3.0, 0.0);
        g.p.skills.rank[Skill::FireWall as usize] = 1;
        g.p.skills.primary = Skill::FireWall;
        g.p.mana = 50.0;
        let (zx, zy) = (g.mobs[0].x, g.mobs[0].y);
        let (sx, sy) = crate::iso::to_screen(zx - g.p.x, zy - g.p.y);
        let l = (sx * sx + sy * sy).sqrt();
        // Aim the wall at the zombie (6 tiles out along the stick), keep it pinned there.
        g.cast_skill(Skill::FireWall, zx, zy);
        assert_eq!(g.fire_walls.len(), 1);
        let _ = (sx, sy, l);
        for _ in 0..60 {
            g.mobs[0].x = zx;
            g.mobs[0].y = zy;
            g.mobs[0].stun = 1.0;
            g.update(&Input::default());
        }
        assert!(g.mobs[0].hp < 1000.0 - 15.0, "zombie in the wall took {}", 1000.0 - g.mobs[0].hp);
        for _ in 0..60 * 8 {
            g.update(&Input::default());
        }
        assert!(g.fire_walls.is_empty(), "the wall burns out");
    }

    #[test]
    fn blaze_leaves_burning_ground_behind_you() {
        use crate::skills::Skill;
        let mut g = quiet_game();
        g.p.skills.rank[Skill::Blaze as usize] = 1;
        g.p.mana = 50.0;
        g.cast_skill(Skill::Blaze, g.p.x, g.p.y);
        assert!(g.p.blaze_t > 5.0);
        for _ in 0..40 {
            g.update(&Input { move_x: 1.0, ..Input::default() });
        }
        assert!(g.patches.len() >= 3, "{} patches", g.patches.len());
        // A zombie standing on a patch gets burned.
        let (x, y) = (g.patches[0].x, g.patches[0].y);
        let mut m = Mob::new(Kind::Zombie, x, y, 1.0, &mut g.rng);
        m.hp = 1000.0;
        g.mobs.push(m);
        for _ in 0..30 {
            g.mobs[0].x = x;
            g.mobs[0].y = y;
            g.mobs[0].stun = 1.0;
            g.update(&Input::default());
        }
        assert!(g.mobs[0].hp < 1000.0);
    }

    #[test]
    fn combust_detonates_only_burning_foes_and_longer_burns_hit_harder() {
        use crate::skills::Skill;
        let mut g = with_zombie(2.0, 0.0);
        let mut cold = Mob::new(Kind::Zombie, g.p.x - 2.0, g.p.y, 1.0, &mut g.rng);
        cold.hp = 1000.0;
        g.mobs.push(cold);
        g.p.skills.rank[Skill::Combust as usize] = 1;
        g.p.mana = 50.0;
        g.mobs[0].burn = 2.0;
        g.mobs[0].burned = 3.0;
        g.cast_skill(Skill::Combust, g.p.x, g.p.y);
        let hot_dmg = 1000.0 - g.mobs[0].hp;
        assert!(hot_dmg > crate::skills::combust_dmg(1, 0) * 1.5, "a long burn hits hard: {hot_dmg}");
        assert_eq!(g.mobs[0].burn, 0.0, "the fire is spent");
        assert_eq!(g.mobs[1].hp, 1000.0, "the unlit zombie is untouched");
        // Nothing burning: no mana spent.
        let mana = g.p.mana;
        g.p.cast_cd = 0.0;
        g.cast_skill(Skill::Combust, g.p.x, g.p.y);
        assert_eq!(g.p.mana, mana);
    }

    #[test]
    fn meteor_lands_after_its_shadow_and_burns_the_ground() {
        use crate::skills::Skill;
        let mut g = with_zombie(3.0, 0.0);
        g.p.skills.rank[Skill::Meteor as usize] = 1;
        g.p.mana = 50.0;
        let (zx, zy) = (g.mobs[0].x, g.mobs[0].y);
        g.cast_skill(Skill::Meteor, zx, zy);
        assert_eq!(g.meteors.len(), 1);
        for _ in 0..30 {
            g.mobs[0].x = zx;
            g.mobs[0].y = zy;
            g.mobs[0].stun = 1.0;
            g.update(&Input::default());
        }
        assert_eq!(g.mobs[0].hp, 1000.0, "nothing until it lands");
        for _ in 0..40 {
            g.mobs[0].x = zx;
            g.mobs[0].y = zy;
            g.mobs[0].stun = 1.0;
            g.update(&Input::default());
        }
        assert!(g.mobs[0].hp < 1000.0 - crate::skills::meteor_dmg(1) * 0.9, "it struck: {}", 1000.0 - g.mobs[0].hp);
        assert!(!g.patches.is_empty(), "burning ground");
    }

    #[test]
    fn hydra_spits_at_foes_and_has_a_cooldown() {
        use crate::skills::Skill;
        let mut g = with_zombie(4.0, 0.0);
        g.p.skills.rank[Skill::Hydra as usize] = 1;
        g.p.mana = 60.0;
        let (zx, zy) = (g.mobs[0].x, g.mobs[0].y);
        g.cast_skill(Skill::Hydra, zx, zy);
        assert_eq!(g.hydras.len(), 1);
        let mana = g.p.mana;
        g.p.cast_cd = 0.0;
        g.cast_skill(Skill::Hydra, zx, zy);
        assert_eq!(g.p.mana, mana, "on cooldown: no second cast");
        for _ in 0..60 * 3 {
            g.mobs[0].x = zx;
            g.mobs[0].y = zy;
            g.mobs[0].stun = 1.0;
            g.update(&Input::default());
        }
        assert!(g.mobs[0].hp < 1000.0, "the hydra hit it");
    }

    #[test]
    fn ash_phoenix_makes_casts_free_then_explodes() {
        use crate::skills::Skill;
        let mut g = with_zombie(1.5, 0.0);
        g.p.skills.rank[Skill::Phoenix as usize] = 1;
        g.p.mana = 45.0;
        g.cast_skill(Skill::Phoenix, g.p.x, g.p.y);
        assert!(g.p.phoenix_t > 5.0);
        let mana = g.p.mana;
        for _ in 0..60 {
            g.mobs[0].stun = 1.0;
            g.update(&Input { cast: true, ..Input::default() });
        }
        assert!(g.p.mana >= mana, "free casts while it lasts ({mana} -> {})", g.p.mana);
        assert!(g.p.mana <= g.p.max_mana);
        for _ in 0..60 * 7 {
            g.update(&Input::default());
        }
        assert_eq!(g.p.phoenix_t, 0.0);
        assert!(g.mobs[0].hp < 1000.0 - crate::skills::phoenix_burst(1) * 0.9 || !g.mobs[0].alive(), "the final burst");
    }

    #[test]
    fn skill_tree_learns_and_aldric_respecs() {
        use crate::skills::Skill;
        let mut g = Game::new(5, crate::gfx::SH_WIDE);
        g.update(&Input { skills: true, ..Input::default() });
        assert!(g.tree.is_some());
        g.tree.as_mut().unwrap().sel = Skill::Inferno as usize;
        g.update(&Input { confirm: true, ..Input::default() });
        assert_eq!(g.p.skills.rank(Skill::Inferno), 1);
        assert_eq!(g.p.skills.secondary, Skill::Inferno, "new skill goes in the secondary slot");
        g.update(&Input { skills: true, ..Input::default() });
        assert!(g.tree.is_none());
        g.p.gold = 500;
        assert!(g.debug_talk(Role::Healer));
        g.update(&Input { confirm: true, ..Input::default() }); // first option: forget skills
        assert_eq!(g.p.skills.rank(Skill::Inferno), 0);
        assert_eq!(g.p.skills.points, 1);
        assert!(g.p.gold < 500);
    }

    #[test]
    fn experience_levels_you_up() {
        let mut g = quiet_game();
        let hp = g.p.max_hp;
        g.gain_xp(xp_to_next(1) + 1.0);
        assert_eq!(g.p.clvl, 2);
        assert!(g.p.max_hp > hp);
    }

    #[test]
    fn monsters_drop_gear_you_pick_up_and_wear() {
        use crate::items::{self, Rarity, Stat};
        let mut g = quiet_game();
        assert!(g.p.gear.worn[0].is_some(), "you start with a staff");
        // About one kill in ten drops gear.
        let mut drops = 0;
        for _ in 0..400 {
            g.pickups.clear();
            let m = Mob::new(Kind::Zombie, g.p.x + 3.0, g.p.y, 1.0, &mut g.rng);
            g.mobs.push(m);
            let i = g.mobs.len() - 1;
            g.kill(i);
            drops += g.pickups.iter().filter(|k| matches!(k.kind, Drop::Item(_))).count();
        }
        assert!((20..70).contains(&drops), "{drops} drops in 400 kills");
        // Bosses drop their unique and two magic-or-better items.
        g.pickups.clear();
        let m = Mob::new(Kind::BoneWarden, g.p.x + 3.0, g.p.y, 1.0, &mut g.rng);
        g.mobs.push(m);
        let i = g.mobs.len() - 1;
        g.mobs[i].boss = true;
        g.kill(i);
        let loot: Vec<_> = g.pickups.iter().filter_map(|k| if let Drop::Item(it) = &k.kind { Some(it.clone()) } else { None }).collect();
        assert_eq!(loot.len(), 3);
        assert!(loot.iter().any(|it| it.name == "MARROWGRIP"));
        assert!(loot.iter().all(|it| it.rarity >= Rarity::Magic));
        // Walking over gear picks it up; a full bag leaves it on the floor.
        g.pickups.clear();
        g.state = State::Playing;
        g.pickups.push(Pickup { x: g.p.x, y: g.p.y, kind: Drop::Item(Box::new(items::unique(0))), t: 1.0 });
        g.collect_pickups();
        assert_eq!(g.p.gear.bag[0].as_ref().map(|i| i.name.as_str()), Some("MARROWGRIP"));
        // Wear it from the inventory screen.
        g.p.clvl = 5;
        let hp = g.p.max_hp;
        g.update(&Input { inv: true, ..Input::default() });
        assert!(g.inv.is_some());
        g.inv.as_mut().unwrap().sel = crate::inventory::Cell::Bag(0);
        g.update(&Input { confirm: true, ..Input::default() });
        assert!(g.p.gear.worn[3].is_some());
        assert_eq!(g.p.max_hp, hp + 20.0, "gear life is added");
        assert_eq!(g.p.bonus.get(Stat::Armor), 8);
        g.update(&Input { inv: true, ..Input::default() });
        assert!(g.inv.is_none());
        // Armor softens hits.
        assert!(g.p.armored(10.0) < 10.0);
        for i in 0..items::BAG {
            g.p.gear.bag[i] = Some(items::unique(1));
        }
        g.pickups.push(Pickup { x: g.p.x, y: g.p.y, kind: Drop::Item(Box::new(items::unique(2))), t: 1.0 });
        g.collect_pickups();
        assert_eq!(g.pickups.len(), 1, "no room: it stays on the floor");
    }

    #[test]
    fn champion_and_elite_packs_are_tougher_and_drop_more() {
        use crate::mobs::{Rank, M_FIERY, M_STONE};
        let lv = world::build(LevelId::Dungeon(0, 1), 7);
        let count = |r: Rank| lv.mobs.iter().filter(|m| m.rank == r).count();
        assert!(count(Rank::Elite) == 1, "one elite leader per floor");
        assert!(count(Rank::Minion) >= 1, "with minions");
        assert!(count(Rank::Champion) >= 2, "and a champion pack");
        let e = lv.mobs.iter().find(|m| m.rank == Rank::Elite).unwrap();
        assert!(e.name.is_some() && e.mods.count_ones() == 2);
        let ow = world::build(LevelId::Overworld, 7);
        assert!(ow.mobs.iter().filter(|m| m.rank == Rank::Elite).count() == 3);
        // Promotion multiplies life; stone skin even more.
        let mut g = quiet_game();
        let mut a = Mob::new(Kind::Zombie, g.p.x + 3.0, g.p.y, 1.0, &mut g.rng);
        let hp = a.max_hp;
        a.promote(Rank::Champion, M_STONE, None);
        assert!(a.max_hp > hp * 4.0);
        // An elite always drops two items; a fire enchanted one bursts into flame.
        let mut m = Mob::new(Kind::Zombie, g.p.x + 1.0, g.p.y, 1.0, &mut g.rng);
        m.promote(Rank::Elite, M_FIERY | M_STONE, Some("TEST THE FOUL".into()));
        g.mobs.push(m);
        g.kill(0);
        assert_eq!(g.pickups.iter().filter(|k| matches!(k.kind, Drop::Item(_))).count(), 2);
        assert_eq!(g.hazards.len(), 1);
        let hp = g.p.hp;
        for _ in 0..60 {
            g.update(&Input::default());
        }
        assert!(g.p.hp < hp, "standing in the burst hurts");
    }

    #[test]
    fn gerta_sells_gear_and_restocks_after_a_dungeon() {
        let mut g = Game::new(5, crate::gfx::SH_WIDE);
        g.p.gold = 5000;
        g.debug_talk(Role::Merchant);
        let k = g.dialog.as_ref().unwrap().options.iter().position(|o| o.1 == Act::Shop).unwrap();
        g.dialog.as_mut().unwrap().sel = k;
        g.update(&Input { confirm: true, ..Input::default() });
        assert!(g.inv.as_ref().map_or(false, |u| u.shop), "trading opens the inventory");
        let n = g.shop_stock.iter().flatten().count();
        assert!(n >= 10);
        let first = g.shop_stock[0].clone().unwrap();
        g.update(&Input::default());
        g.inv.as_mut().unwrap().sel = crate::inventory::Cell::Shop(0);
        g.update(&Input { confirm: true, ..Input::default() });
        assert_eq!(g.p.gold, 5000 - first.cost());
        assert_eq!(g.p.gear.bag[0].as_ref(), Some(&first));
        assert!(g.shop_stock[0].is_none());
        // The same stock until you've been to a dungeon.
        g.inv = None;
        let before: Vec<_> = g.shop_stock.clone();
        g.debug_talk(Role::Merchant);
        g.dialog.as_mut().unwrap().sel = k;
        g.update(&Input { confirm: true, ..Input::default() });
        assert_eq!(g.shop_stock, before);
        g.inv = None;
        g.debug_goto(LevelId::Dungeon(0, 0));
        g.debug_goto(LevelId::Overworld);
        g.debug_talk(Role::Merchant);
        g.dialog.as_mut().unwrap().sel = k;
        g.update(&Input { confirm: true, ..Input::default() });
        assert_ne!(g.shop_stock, before, "restocked");
    }

    #[test]
    fn nightmare_follows_the_ash_king() {
        let mut g = Game::new(5, crate::gfx::SH_WIDE);
        let normal_hp: f32 = g.mobs.iter().map(|m| m.max_hp).sum();
        g.quest.stage = 3;
        g.quest.seals = [true; 3];
        g.p.gear.bag[0] = Some(crate::items::unique(0));
        let clvl = g.p.clvl;
        g.debug_talk(Role::Elder);
        for _ in 0..3 {
            let d = g.dialog.as_ref().unwrap();
            if d.options.iter().any(|o| o.1 == Act::NextDifficulty) {
                break;
            }
            g.update(&Input { confirm: true, ..Input::default() });
            g.update(&Input::default());
        }
        let d = g.dialog.as_mut().unwrap();
        d.sel = d.options.iter().position(|o| o.1 == Act::NextDifficulty).expect("elder offers nightmare");
        g.update(&Input { confirm: true, ..Input::default() });
        assert_eq!(g.quest.difficulty, 1);
        assert_eq!((g.quest.stage, g.quest.seal_count()), (1, 0), "quests start over");
        assert!(g.p.gear.bag[0].is_some() && g.p.clvl == clvl, "you keep your hero");
        assert!(g.level_name.contains("NIGHTMARE"));
        let nm_hp: f32 = g.mobs.iter().map(|m| m.max_hp).sum();
        assert!(nm_hp > normal_hp * 2.0, "monsters are much tougher");
        assert!(g.tier > 1.5);
        // Saved and loaded as nightmare.
        let text = crate::save::to_text(&g);
        let mut h = Game::new(5, crate::gfx::SH_WIDE);
        crate::save::apply(&mut h, &text);
        assert_eq!(h.quest.difficulty, 1);
        assert!(h.level_name.contains("NIGHTMARE"));
        h.debug_goto(LevelId::Dungeon(0, 0));
        assert!(h.tier >= 1.99, "dungeons are harder too");
    }

    #[test]
    fn waypoints_activate_and_take_you_back() {
        let mut g = Game::new(5, crate::gfx::SH_WIDE);
        g.debug_goto(LevelId::Dungeon(1, 1));
        let (wx, wy) = g.waypoint;
        assert!(!g.d.blocked(wx, wy, 0.5), "waypoint on open floor");
        g.p.x = wx;
        g.p.y = wy;
        g.update(&Input::default());
        assert!(g.waypoints.contains(&LevelId::Dungeon(1, 1)), "touching activates it");
        let d = g.dialog.as_ref().expect("travel menu");
        assert_eq!(d.options[0].1, Act::Travel(LevelId::Overworld));
        g.update(&Input { confirm: true, ..Input::default() });
        assert_eq!(g.level, LevelId::Overworld);
        assert!((g.p.x - g.waypoint.0).abs() < 0.01 && g.in_safe(g.p.x, g.p.y), "arrive on the town waypoint");
        // Standing on arrival doesn't reopen the menu; stepping off and back on does.
        g.update(&Input::default());
        assert!(g.dialog.is_none());
        g.p.x += 2.0;
        g.update(&Input::default());
        g.p.x -= 2.0;
        g.update(&Input::default());
        let d = g.dialog.as_ref().expect("menu again");
        assert!(d.options.iter().any(|o| o.0 == "THE ROTTING WARRENS - LEVEL 2"));
        // Saved with the character.
        let text = crate::save::to_text(&g);
        let mut h = Game::new(5, crate::gfx::SH_WIDE);
        crate::save::apply(&mut h, &text);
        assert!(h.waypoints.contains(&LevelId::Dungeon(1, 1)));
    }

    #[test]
    fn the_stash_keeps_gear_in_town() {
        let mut g = Game::new(5, crate::gfx::SH_WIDE);
        g.p.gear.bag[2] = Some(crate::items::unique(3));
        g.update(&Input { inv: true, ..Input::default() });
        g.inv.as_mut().unwrap().sel = crate::inventory::Cell::Bag(2);
        g.update(&Input { potion_mp: true, ..Input::default() });
        assert!(g.p.gear.bag[2].is_none());
        assert_eq!(g.p.gear.stash[0].as_ref().map(|i| i.name.as_str()), Some("CROWN OF ASH"));
        g.inv.as_mut().unwrap().sel = crate::inventory::Cell::Stash(0);
        g.update(&Input::default());
        g.update(&Input { confirm: true, ..Input::default() });
        assert!(g.p.gear.stash[0].is_none() && g.p.gear.bag[0].is_some(), "taken back to the bag");
        // Outside town there is no stash.
        g.inv = None;
        g.debug_goto(LevelId::Dungeon(0, 0));
        g.update(&Input { inv: true, ..Input::default() });
        g.inv.as_mut().unwrap().sel = crate::inventory::Cell::Bag(0);
        g.update(&Input { potion_mp: true, ..Input::default() });
        assert!(g.p.gear.bag[0].is_some());
    }

    #[test]
    fn gear_sells_in_town_and_drops_outside() {
        let mut g = Game::new(5, crate::gfx::SH_WIDE);
        assert!(g.in_safe(g.p.x, g.p.y), "you start in Hollowmere");
        g.p.gear.bag[0] = Some(crate::items::unique(4));
        let gold = g.p.gold;
        g.update(&Input { inv: true, ..Input::default() });
        g.inv.as_mut().unwrap().sel = crate::inventory::Cell::Bag(0);
        g.update(&Input { cast2: true, ..Input::default() });
        assert!(g.p.gear.bag[0].is_none());
        assert!(g.p.gold > gold, "sold to Gerta");
        g.inv = None;
        g.debug_goto(LevelId::Dungeon(0, 0));
        g.p.gear.bag[0] = Some(crate::items::unique(4));
        let n = g.pickups.len();
        g.update(&Input { inv: true, ..Input::default() });
        g.inv.as_mut().unwrap().sel = crate::inventory::Cell::Bag(0);
        g.update(&Input { cast2: true, ..Input::default() });
        assert_eq!(g.pickups.len(), n + 1, "dropped on the floor");
        // Taking off the staff loses its fire bonus.
        let f = g.p.skills.fire_mult();
        g.inv.as_mut().unwrap().sel = crate::inventory::Cell::Worn(0);
        g.update(&Input { confirm: true, ..Input::default() });
        assert!(g.p.gear.worn[0].is_none());
        assert!(g.p.skills.fire_mult() < f);
    }
}

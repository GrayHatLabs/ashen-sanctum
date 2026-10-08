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
use crate::world::{self, Level, LevelId, Portal, PortalKind, Prop, Theme, ABYSS, CASTLE, GLACIER, HEART, SANCTUM, ZENITH};
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
/// Chance a common monster drops a gem.
pub const GEM_DROP: f32 = 0.04;
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
    /// Text typed this tick (hero names) and Backspace.
    pub typed: String,
    pub backspace: bool,
    /// Mouse wheel this tick (+ up, - down).
    pub wheel: i32,
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
    /// Ice shards and snow puffs.
    Frost,
    /// The druid's glowing green spores.
    Spore,
    /// The inquisitor's holy fire: white-gold flames that rise slowly.
    Holy,
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
    /// A Frost Herald's rune (0 giant, 1 yeti matriarch, 2 rime witch).
    Rune(usize),
    /// A skeleton lord's grave sigil (0 Ossric, 1 Grimhilde, 2 Malgrave).
    Sigil(usize),
    /// A herald of Mechanus's winding key (0 Forgemother, 1 Cantor, 2 Archivist).
    Key(usize),
    /// A Leviathan pearl from a herald of the deep (0 Dregmoor, 1 Nacre, 2 the Angler Matriarch).
    Pearl(usize),
    /// A sun-shard from a herald of the sky (0 Vael, 1 the Tempest Drake, 2 the Ophan Prime).
    Shard(usize),
    /// A lore page (side.rs LORE index).
    Page(u8),
    /// A heap of the Ember Wyrm's gold: into the hoard sack (dragon.rs).
    Hoard(i32),
    /// Random errands' things (errands.rs): an herb to gather, a lost heirloom, a treasure map.
    Herb,
    Heirloom,
    Clue,
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
    /// Endgame (endgame.rs): how often each act has been rekindled, the deepest Ash Rift cleared in time,
    /// rifts opened, and Embers of mastery (four tracks, and points to spend).
    pub rekindles: [u32; 6],
    pub rift_best: u16,
    pub rift_runs: u32,
    pub embers: [u8; 4],
    pub ember_points: u32,
    /// Side content (side.rs): a shrine's blessing (Blessing id, 0 none) and its time left, wards from
    /// side quests (-5% damage taken each), lore pages found and super uniques slain (bit sets), shrines used.
    pub blessing: u8,
    pub bless_t: f32,
    pub ward: u8,
    pub pages: u64,
    pub supers: u64,
    pub shrines_used: u32,
    /// Gold in the hoard sack (dragon.rs): yours once you're out of the Ember Wyrm's cave.
    pub sack: i32,
    pub x: f32,
    pub y: f32,
    pub hp: f32,
    pub max_hp: f32,
    pub mana: f32,
    pub max_mana: f32,
    pub dir: usize,
    pub anim_t: f32,
    pub moving: bool,
    /// The inventor's last cast was a thrown gadget (throw pose, not the pistol).
    pub throwing: bool,
    /// The valkyrie's current attack pose (attack / sweep / whirl / throw / cast).
    pub pose: &'static str,
    /// The valkyrie: seconds left "in combat" (Valor doesn't drain).
    pub fight_t: f32,
    /// Her ice wings are spread (Glacier Leap, Fimbulwinter).
    pub wings_t: f32,
    /// Ride of the Valkyrie in progress.
    pub charge: Option<crate::valkyrie::Charge>,
    pub fimbul_t: f32,
    pub fimbul_dps: f32,
    // ---- the berserker ----
    /// Attack speed (Bloodlust stacks, Berserk).
    pub haste: f32,
    pub lust: u8,
    pub lust_t: f32,
    pub whirl_t: f32,
    pub whirl_to: (f32, f32),
    pub whirl_tick: f32,
    pub whirl_dmg: f32,
    pub warcry_t: f32,
    pub berserk_t: f32,
    pub exhaust_t: f32,
    // ---- the inquisitor (Censer Sweep reuses the whirl fields) ----
    /// Final Judgment.
    pub judge_t: f32,
    pub wolf_cd: f32,
    pub howl_t: f32,
    // ---- the reaper ----
    /// Runes lit on her scythe (0..7), the prey they were lit on, and how long until they fade.
    pub runes: u8,
    pub rune_prey: Option<usize>,
    pub rune_t: f32,
    /// The Ledger is open.
    pub ledger_t: f32,
    // ---- the druid ----
    /// Decay (-1) .. Bloom (+1).
    pub balance: f32,
    /// Rejuvenate: seconds left and life per second.
    pub regrow_t: f32,
    pub regrow_rate: f32,
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
    /// Chilled by frost: slower moving and casting while this lasts.
    pub chill: f32,
    /// The vampire: Mist Step (untouchable while it lasts) and Countess's Embrace (bat form).
    pub mist: f32,
    pub embrace_t: f32,
    pub embrace_rank: u8,
    /// The inventor: overheat lock, vent cooldown, steam suit and Tesla field timers.
    pub overheat: f32,
    pub vent_cd: f32,
    pub suit_t: f32,
    pub tesla_t: f32,
    pub tesla_dps: f32,
}

impl Player {
    fn new() -> Self {
        Player {
            rekindles: [0; 6],
            rift_best: 0,
            rift_runs: 0,
            embers: [0; 4],
            ember_points: 0,
            blessing: 0,
            bless_t: 0.0,
            ward: 0,
            pages: 0,
            supers: 0,
            shrines_used: 0,
            sack: 0,
            x: 0.0,
            y: 0.0,
            hp: 70.0,
            max_hp: 70.0,
            mana: 50.0,
            max_mana: 50.0,
            dir: 0,
            anim_t: 0.0,
            moving: false,
            throwing: false,
            pose: "cast",
            fight_t: 0.0,
            wings_t: 0.0,
            charge: None,
            fimbul_t: 0.0,
            fimbul_dps: 0.0,
            haste: 1.0,
            lust: 0,
            lust_t: 0.0,
            whirl_t: 0.0,
            whirl_to: (0.0, 0.0),
            whirl_tick: 0.0,
            whirl_dmg: 0.0,
            warcry_t: 0.0,
            berserk_t: 0.0,
            exhaust_t: 0.0,
            judge_t: 0.0,
            wolf_cd: 0.0,
            howl_t: 0.0,
            runes: 0,
            rune_prey: None,
            rune_t: 0.0,
            ledger_t: 0.0,
            balance: 0.0,
            regrow_t: 0.0,
            regrow_rate: 0.0,
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
            chill: 0.0,
            mist: 0.0,
            embrace_t: 0.0,
            embrace_rank: 1,
            overheat: 0.0,
            vent_cd: 0.0,
            suit_t: 0.0,
            tesla_t: 0.0,
            tesla_dps: 0.0,
        }
    }

    /// The gear you start with: the sorceress a gnarled staff, the vampire a plain amulet.
    pub fn starting_gear(&mut self) {
        if self.skills.class == crate::skills::Class::Reaper {
            // Ten souls fill her lantern (gear "+mana" adds room); she starts with none.
            self.base_mana = 100.0;
            self.base_hp += 15.0;
            self.gear = crate::items::Gear::default();
            let amulet = crate::items::Item {
                base: crate::items::base_by_key("amulet").unwrap(),
                rarity: crate::items::Rarity::Normal,
                ilvl: 1,
                name: "ARCHIVE KEY".into(),
                stats: vec![],
                req: 1,
                sockets: 0,
                gems: vec![],
            };
            let slot = crate::items::WORN.iter().position(|s| *s == crate::items::Slot::Amulet).unwrap();
            self.gear.worn[slot] = Some(amulet);
            self.recalc();
            return;
        }
        if self.skills.class == crate::skills::Class::Berserker {
            // Rage fills to 100 from pain and kills; she's big, and wears what she took from the dead.
            self.base_mana = 100.0;
            self.base_hp += 40.0;
            self.gear = crate::items::Gear::default();
            let armor = crate::items::Item {
                base: crate::items::base_by_key("leather").unwrap(),
                rarity: crate::items::Rarity::Normal,
                ilvl: 1,
                name: "WOLF-FUR HIDES".into(),
                stats: vec![(crate::items::Stat::Armor, 8), (crate::items::Stat::Life, 10)],
                req: 1,
                sockets: 0,
                gems: vec![],
            };
            let slot = crate::items::WORN.iter().position(|s| *s == crate::items::Slot::Armor).unwrap();
            self.gear.worn[slot] = Some(armor);
            self.recalc();
            return;
        }
        if self.skills.class == crate::skills::Class::Valkyrie {
            // Valor fills to 100 by fighting; she's the toughest hero, in leather from the start.
            self.base_mana = 100.0;
            self.base_hp += 30.0;
            self.gear = crate::items::Gear::default();
            let armor = crate::items::Item {
                base: crate::items::base_by_key("leather").unwrap(),
                rarity: crate::items::Rarity::Normal,
                ilvl: 1,
                name: "NORTHERN LEATHERS".into(),
                stats: vec![(crate::items::Stat::Armor, 12)],
                req: 1,
                sockets: 0,
                gems: vec![],
            };
            let slot = crate::items::WORN.iter().position(|s| *s == crate::items::Slot::Armor).unwrap();
            self.gear.worn[slot] = Some(armor);
            self.recalc();
            return;
        }
        if self.skills.class == crate::skills::Class::Inventor {
            // Her heat gauge is roomier than a caster's mana pool.
            self.base_mana = 100.0;
            self.gear = crate::items::Gear::default();
            let ring = crate::items::Item {
                base: crate::items::base_by_key("ring").unwrap(),
                rarity: crate::items::Rarity::Normal,
                ilvl: 1,
                name: "BRASS RING".into(),
                stats: vec![],
                req: 1,
                sockets: 0,
                gems: vec![],
            };
            self.gear.worn[6] = Some(ring);
            self.recalc();
            return;
        }
        if self.skills.class == crate::skills::Class::Vampire {
            self.gear = crate::items::Gear::default();
            let amulet = crate::items::Item {
                base: crate::items::base_by_key("amulet").unwrap(),
                rarity: crate::items::Rarity::Normal,
                ilvl: 1,
                name: "AMULET".into(),
                stats: vec![],
                req: 1,
                sockets: 0,
                gems: vec![],
            };
            self.gear.worn[8] = Some(amulet);
            self.recalc();
            return;
        }
        let staff = crate::items::Item {
            base: crate::items::base_by_key("gnarled").unwrap(),
            rarity: crate::items::Rarity::Normal,
            ilvl: 1,
            name: "GNARLED STAFF".into(),
            stats: vec![(crate::items::Stat::Fire, 5)],
            req: 1,
            sockets: 0,
            gems: vec![],
        };
        self.gear.worn[0] = Some(staff);
        self.recalc();
    }

    /// Applies the worn gear: max life / mana, +skills and fire damage.
    pub fn recalc(&mut self) {
        use crate::items::Stat;
        self.bonus = self.gear.bonus();
        self.max_hp = (self.base_hp + self.bonus.get(Stat::Life) as f32) * (1.0 + 0.02 * self.embers[1] as f32);
        self.max_mana = self.base_mana + self.bonus.get(Stat::Mana) as f32;
        self.hp = self.hp.min(self.max_hp);
        self.mana = self.mana.min(self.max_mana);
        // A Skill shrine adds one more.
        let shrine = (self.bless_t > 0.0 && crate::side::Blessing::from_id(self.blessing) == Some(crate::side::Blessing::Skill)) as i32;
        self.skills.bonus = (self.bonus.get(Stat::Skills).clamp(0, 5) + shrine) as u8;
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
    /// Blows that reached a monster (any skill; the test bot watches this).
    pub dealt: u32,
    pub damage_taken: f32,
    pub embers: u32,
    pub eaten: u32,
    pub starve_damage: f32,
    pub run_time: f32,
    pub levels_entered: u32,
    pub deaths: u32,
    pub talks: u32,
    pub bosses: u32,
    /// Monsters blown or knocked off the sky islands (sky.rs).
    pub kills_fell: u32,
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
    pub(crate) town_start: (f32, f32),
    pub(crate) parked: HashMap<LevelId, Level>,
    pub(crate) world_seed: u64,
    // ---- transient effects ----
    pub(crate) balls: Vec<Fireball>,
    pub(crate) novas: Vec<crate::skills::Nova>,
    pub(crate) fire_walls: Vec<crate::skills::FireWallFx>,
    pub(crate) patches: Vec<crate::skills::FirePatch>,
    pub(crate) meteors: Vec<crate::skills::MeteorFx>,
    pub(crate) hydras: Vec<crate::skills::HydraFx>,
    pub(crate) bats: Vec<crate::vampire::BatFx>,
    pub(crate) ravens: Vec<crate::valkyrie::RavenFx>,
    pub(crate) javelins: Vec<crate::valkyrie::JavelinFx>,
    pub(crate) axes: Vec<crate::berserker::AxeFx>,
    pub(crate) souls: Vec<crate::reaper::SoulFx>,
    pub(crate) lanterns: Vec<crate::reaper::LanternFx>,
    pub(crate) chains_fx: Vec<crate::reaper::ChainsFx>,
    pub(crate) glasses: Vec<crate::reaper::GlassFx>,
    pub(crate) sweeps: Vec<crate::reaper::SweepFx>,
    pub(crate) clouds: Vec<crate::druid::CloudFx>,
    pub(crate) fungi: Vec<crate::druid::FungusFx>,
    pub(crate) vines: Vec<crate::druid::VineFx>,
    pub(crate) fields: Vec<crate::vampire::BloodField>,
    pub(crate) bombs: Vec<crate::inventor::BombFx>,
    pub(crate) arcs: Vec<crate::inventor::ArcFx>,
    pub(crate) turrets: Vec<crate::inventor::TurretFx>,
    pub(crate) spiders: Vec<crate::inventor::SpiderFx>,
    pub(crate) airships: Vec<crate::inventor::AirshipFx>,
    /// The skill tree screen, while open.
    pub tree: Option<crate::skills::TreeUi>,
    /// The inventory screen, while open.
    pub inv: Option<crate::inventory::InvUi>,
    /// The hero's name (character select screen, save file).
    pub hero_name: String,
    /// The class select screen for a new character (0 = Sorceress, 1 = Vampire).
    pub choose: Option<usize>,
    pub(crate) choose_rects: Vec<(i32, i32, i32, i32, usize)>,
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
    /// The inquisitor's chains drawn for a moment, and her binding chains.
    pub(crate) links: Vec<crate::inquisitor::ChainLinkFx>,
    /// Her whips in flight, and her strike combo (forehand, backhand, overhead) with its timer.
    pub(crate) whips: Vec<crate::inquisitor::Whip>,
    pub(crate) whip_combo: u8,
    pub(crate) whip_combo_t: f32,
    /// Act 5 (tides.rs): the tide's clock and depth, ink in your eyes, the Angler's dark, and a pull
    /// toward a point (x, y, time left).
    pub(crate) tide_t: f32,
    /// Side content (side.rs): this level's shrines, and seconds until the next event out in the wilds.
    pub(crate) shrines: Vec<crate::side::Shrine>,
    pub(crate) event_cd: f32,
    /// The Ember Wyrm (dragon.rs): noise toward waking it, whether you were in its lair last tick, casts seen.
    pub(crate) wyrm_noise: f32,
    pub(crate) wyrm_in: bool,
    pub(crate) wyrm_casts: u32,
    /// Random errands (errands.rs): rolled per act as you get there; the ones done this difficulty (saved).
    pub(crate) errands: Vec<crate::errands::Errand>,
    pub(crate) errands_done: Vec<(LevelId, crate::errands::ErrandKind)>,
    pub(crate) tide: f32,
    pub(crate) blind_t: f32,
    pub(crate) dark_t: f32,
    pub(crate) pull: (f32, f32, f32),
    /// Act 6 (sky.rs): the wind's clock and direction, the warning streaks and the gust, and the last
    /// solid ground you stood on (a fall puts you back there).
    pub(crate) wind_t: f32,
    pub(crate) wind_dir: (f32, f32),
    pub(crate) wind_warn: f32,
    pub(crate) wind_gust: f32,
    pub(crate) last_safe: (f32, f32),
    /// The Ash Rift in progress (endgame.rs), and when Second Wind last saved you.
    pub(crate) rift: Option<crate::endgame::RiftRun>,
    pub(crate) second_wind_t: f32,
    pub(crate) binds: Vec<crate::inquisitor::BindFx>,
    /// The blow being dealt to you comes from something cursed (her Iron Halo).
    pub(crate) hurt_cursed: bool,
    /// Act 4: stop-clocks and law zones on this level (`clockwork.rs`).
    pub clocks: Vec<crate::clockwork::TimeClock>,
    pub laws: Vec<crate::clockwork::LawZone>,
    /// The law zone you're standing in (announced on entry).
    pub law_here: Option<usize>,
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
    pub(crate) portal_cd: f32,
    /// Where you stood over the last three seconds (the Clockmaker's rewind).
    trail: std::collections::VecDeque<(f32, f32)>,
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
            bats: vec![],
            ravens: vec![],
            javelins: vec![],
            axes: vec![],
            souls: vec![],
            lanterns: vec![],
            chains_fx: vec![],
            glasses: vec![],
            sweeps: vec![],
            clouds: vec![],
            fungi: vec![],
            vines: vec![],
            fields: vec![],
            bombs: vec![],
            arcs: vec![],
            turrets: vec![],
            spiders: vec![],
            airships: vec![],
            tree: None,
            inv: None,
            hero_name: "HERO".into(),
            choose: None,
            choose_rects: vec![],
            waypoints: vec![LevelId::Overworld],
            waypoint: (0.0, 0.0),
            wp_armed: true,
            shop_stock: vec![],
            shop_stale: true,
            hud_skill_rects: vec![],
            hud_bag: (0, 0, 0, 0),
            shots: vec![],
            hazards: vec![],
            links: vec![],
            whips: vec![],
            whip_combo: 0,
            whip_combo_t: 0.0,
            tide_t: 0.0,
            shrines: vec![],
            event_cd: 60.0,
            wyrm_noise: 0.0,
            wyrm_in: false,
            wyrm_casts: 0,
            errands: vec![],
            errands_done: vec![],
            tide: 0.0,
            blind_t: 0.0,
            dark_t: 0.0,
            pull: (0.0, 0.0, 0.0),
            wind_t: 8.0,
            wind_dir: (1.0, 0.0),
            wind_warn: 0.0,
            wind_gust: 0.0,
            last_safe: (0.0, 0.0),
            rift: None,
            second_wind_t: 0.0,
            binds: vec![],
            hurt_cursed: false,
            clocks: vec![],
            laws: vec![],
            law_here: None,
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
            trail: Default::default(),
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
            shrines: std::mem::take(&mut self.shrines),
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
        self.shrines = lv.shrines;
        self.safe = lv.safe;
        self.start = lv.start;
        if lv.id.town() {
            self.town_start = lv.start;
        }
        self.add_endgame_npcs();
        self.balls.clear();
        self.novas.clear();
        self.fire_walls.clear();
        self.patches.clear();
        self.meteors.clear();
        self.hydras.clear();
        self.bats.clear();
        self.fields.clear();
        self.bombs.clear();
        self.arcs.clear();
        self.turrets.clear();
        self.airships.clear();
        // The spider follows you between levels.
        for s in self.spiders.iter_mut() {
            (s.x, s.y) = self.start;
        }
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
        // Parked levels keep their name, so only tag it once.
        let tag = format!(" ({})", story::DIFFICULTIES[self.quest.difficulty as usize]);
        if self.quest.difficulty > 0 && !self.level_name.ends_with(&tag) {
            self.level_name += &tag;
        }
        if !lv.id.town() {
            self.shop_stale = true;
        }
        self.place_clockwork();
    }

    /// Gerta's stock: mostly magic gear around your level, now and then a rare.
    pub(crate) fn restock(&mut self) {
        use crate::items::{self, Rarity, SHOP};
        let ilvl = (self.p.clvl as u8 + 1).clamp(2, 50);
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
            LevelId::Overworld | LevelId::Frostmarch | LevelId::Mistwood | LevelId::Mechanus | LevelId::Deep | LevelId::Heavens => (self.town_start.0 + 3.0, self.town_start.1 + 2.0),
            LevelId::Dungeon(..) => self.portals.iter().find(|p| p.kind == PortalKind::Up).map(|p| (p.x + 2.0, p.y + 1.0)).unwrap_or(self.start),
            LevelId::Rift(_) => self.start,
            // An area's waypoint stands by the road in from town.
            LevelId::Area(..) => (self.start.0 + 2.0, self.start.1 + 1.0),
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
            LevelId::Frostmarch => "KALDHOLM".into(),
            LevelId::Mistwood => "MOURNHOLD".into(),
            LevelId::Mechanus => "THE LAST ESCAPEMENT".into(),
            LevelId::Deep => "BRINEHOLLOW".into(),
            LevelId::Heavens => "WINDWARD ANCHORAGE".into(),
            LevelId::Dungeon(k, f) => format!("{} - LEVEL {}", DUNGEONS_LIST[k].name, f + 1),
            LevelId::Rift(t) => format!("ASH RIFT - TIER {t}"),
            LevelId::Area(a, n) => crate::areas::def(a, n).name.into(),
        }
    }

    /// Touching a waypoint activates it; stepping onto it opens the travel menu.
    fn check_waypoint(&mut self) {
        let (wx, wy) = self.waypoint;
        // A rift is a one-off: its waypoint doesn't join your travel list.
        if self.in_rift() {
            return;
        }
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
            // This act's waypoints, and the other acts' towns (D2 shows one act at a time).
            .filter(|id| id.act() == self.level.act() || id.town())
            .map(|&id| (Self::waypoint_name(id), Act::Travel(id)))
            .collect();
        if options.is_empty() {
            return;
        }
        options.sort_by_key(|o| match o.1 {
            Act::Travel(id @ LevelId::Dungeon(k, f)) => id.act() * 1000 + 100 + k * 10 + f,
            Act::Travel(id @ LevelId::Area(_, n)) => id.act() * 1000 + n as usize,
            Act::Travel(id) => id.act() * 1000,
            _ => 999,
        });
        options.push(("STAY HERE".into(), Act::Close));
        let mut d = Dialog { name: "WAYPOINT", pages: vec!["THE RUNES HUM. WHERE WILL YOU GO?".into()], page: 0, options: vec![], sel: 0, advance_to: None, heals: false, last_options: vec![] };
        d.options = options;
        self.dialog = Some(d);
    }

    /// Makes this a fresh character of a class (new game).
    pub fn set_class(&mut self, class: crate::skills::Class) {
        self.p.skills = crate::skills::Skills::new(class);
        self.p.starting_gear();
        self.p.hp = self.p.max_hp;
        // Valor starts empty: she earns it in the fight.
        self.p.mana = if matches!(class, crate::skills::Class::Valkyrie | crate::skills::Class::Berserker | crate::skills::Class::Reaper | crate::skills::Class::Inquisitor) {
            0.0
        } else {
            self.p.max_mana
        };
    }

    /// Class select controls: left / right (or click) to pick, confirm to start.
    fn update_choose(&mut self, inp: &Input, sel: usize, confirm: bool, click: bool) {
        let mut sel = sel;
        let edge = |now: f32, before: f32, neg: bool| if neg { now < -0.5 && before >= -0.5 } else { now > 0.5 && before <= 0.5 };
        // The carousel wraps around.
        let n = crate::skills::ALL_CLASSES.len();
        if edge(inp.move_x, self.prev.move_x, true) || edge(inp.move_y, self.prev.move_y, true) || inp.wheel > 0 {
            sel = (sel + n - 1) % n;
        }
        if edge(inp.move_x, self.prev.move_x, false) || edge(inp.move_y, self.prev.move_y, false) || inp.wheel < 0 {
            sel = (sel + 1) % n;
        }
        let mut go = confirm;
        if let (Some((mx, my)), true) = (inp.mouse, click) {
            // The topmost card under the mouse (the centre one is drawn last).
            if let Some(&(.., k)) = self.choose_rects.iter().rev().find(|&&(x, y, w, h, _)| mx >= x && mx < x + w && my >= y && my < y + h) {
                go = k == sel;
                sel = k;
            }
        }
        self.choose = Some(sel);
        if go {
            let class = match sel {
                1 => crate::skills::Class::Vampire,
                2 => crate::skills::Class::Inventor,
                3 => crate::skills::Class::Valkyrie,
                4 => crate::skills::Class::Berserker,
                5 => crate::skills::Class::Reaper,
                6 => crate::skills::Class::Druid,
                7 => crate::skills::Class::Inquisitor,
                _ => crate::skills::Class::Sorceress,
            };
            self.set_class(class);
            self.choose = None;
            self.sfx.push(Sfx::Descend);
            self.save_due = true;
        }
    }

    /// `--act2` / `--act3` / `--act4`: a ready character standing in that act's town, as if the
    /// earlier acts were done (level 18 / 26 / 34, their relics' power, gear and gold to match).
    pub fn act_start(&mut self, act: usize) {
        use crate::items::{self, Rarity, WORN};
        let act = act.clamp(1, 5);
        let clvl = [1, 18, 26, 34, 42, 50][act];
        let relics = 3 * act as i32;
        let ilvl = (clvl - 2) as u8;
        let p = &mut self.p;
        p.clvl = clvl;
        p.xp = 0.0;
        p.base_hp = 70.0 + 8.0 * (clvl - 1) as f32 + 15.0 * relics as f32;
        p.base_mana = 50.0 + 4.0 * (clvl - 1) as f32 + 10.0 * relics as f32;
        // The same growth as playing there: 7% a level to 20, 3% after (level_up), and the relics' boosts.
        p.power = (2..=clvl).map(|l| if l <= 20 { 1.07f32 } else { 1.03 }).product::<f32>() * 1.12f32.powi(relics);
        p.skills = crate::skills::Skills::new(p.skills.class);
        p.skills.points = clvl + relics as u32;
        p.gold = 3000 * act as i32;
        p.hp_pots = 8;
        p.mp_pots = 8;
        p.food = MAX_FOOD;
        // Gear: magic or rare pieces around item level 18 in every slot.
        let mut rng = Rng::new(self.world_seed ^ 0xAC72 ^ act as u64);
        for (w, slot) in WORN.iter().enumerate() {
            let base = (0..items::BASES.len())
                .rev()
                .find(|&b| items::BASES[b].slot == *slot && !items::BASES[b].unique_only && items::BASES[b].lvl <= ilvl)
                .unwrap();
            let rarity = if rng.chance(0.4) { Rarity::Rare } else { Rarity::Magic };
            let mut it = items::roll_base(base, ilvl, rarity, &mut rng);
            it.req = it.req.min(clvl);
            p.gear.worn[w] = Some(it);
        }
        p.gear.bag[0] = Some(items::roll(ilvl - 2, Rarity::Rare, &mut rng));
        p.recalc();
        p.hp = p.max_hp;
        p.mana = p.max_mana;
        let done = |n: usize| act > n;
        self.quest = Quest {
            stage: 3,
            seals: [true; 3],
            difficulty: 0,
            stage2: if done(1) { 3 } else { 0 },
            runes: [done(1); 3],
            stage3: if done(2) { 3 } else { 0 },
            sigils: [done(2); 3],
            stage4: if done(3) { 3 } else { 0 },
            keys: [done(3); 3],
            stage5: if done(4) { 3 } else { 0 },
            pearls: [done(4); 3],
            stage6: 0,
            shards: [false; 3],
            side: [0; 18],
        };
        self.waypoints = (0..=act).map(LevelId::land).collect();
        self.go_to(LevelId::land(act), None);
        (self.p.x, self.p.y) = self.town_start;
        self.stats.levels_entered = 1;
        self.say(format!("ACT {} TEST CHARACTER: {} SKILL POINTS TO SPEND (K)", act + 1, self.p.skills.points));
    }

    /// Rebuilds an act's overland at the current difficulty (after loading a save).
    pub(crate) fn rebuild_world(&mut self, act: usize) {
        self.parked.clear();
        let lv = world::build_at(LevelId::land(act), self.world_seed, self.quest.difficulty);
        self.swap_in(lv);
        (self.p.x, self.p.y) = self.town_start;
    }

    /// Nightmare / Hell: the world is rebuilt harder, the quests start over, your hero carries on.
    pub(crate) fn next_difficulty(&mut self) {
        let d = (self.quest.difficulty + 1).min(2);
        self.quest = Quest { stage: 1, seals: [false; 3], difficulty: d, stage2: 0, runes: [false; 3], stage3: 0, sigils: [false; 3], stage4: 0, keys: [false; 3], stage5: 0, pearls: [false; 3], stage6: 0, shards: [false; 3], side: [0; 18] };
        self.parked.clear();
        // New tasks on the new difficulty.
        self.errands.clear();
        self.errands_done.clear();
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
    /// Orders from bosses: the Clockmaker rewinds you, the Archivist files you away elsewhere.
    fn mob_cues(&mut self) {
        for i in 0..self.mobs.len() {
            match std::mem::take(&mut self.mobs[i].cue) {
                1 => {
                    if let Some(&(x, y)) = self.trail.front() {
                        if !self.d.blocked(x, y, PLAYER_R) {
                            (self.p.x, self.p.y) = (x, y);
                            self.trail.clear();
                            self.floater(x, y, "REWOUND!".into(), rgb(0xe0c060));
                            self.sfx.push(Sfx::Descend);
                        }
                    }
                }
                2 => {
                    let (px, py) = (self.p.x, self.p.y);
                    for _ in 0..40 {
                        let a = self.rng.f() * std::f32::consts::TAU;
                        let r = self.rng.rf(6.0, 10.0);
                        let (x, y) = (px + a.cos() * r, py + a.sin() * r);
                        if !self.d.blocked(x, y, PLAYER_R + 0.05) && self.d.path((px as i32, py as i32), (x as i32, y as i32), 3000).is_some() {
                            (self.p.x, self.p.y) = (x, y);
                            self.floater(x, y, "FILED AWAY!".into(), rgb(0x80b0ff));
                            self.sfx.push(Sfx::Descend);
                            break;
                        }
                    }
                }
                c @ (3 | 4) => self.deep_cue(i, c),
                5 => self.gale_from(i),
                _ => {}
            }
        }
    }

    pub fn go_to(&mut self, id: LevelId, from: Option<LevelId>) {
        // The druid's moss wolf follows her (it stays until it falls).
        let wolf = self.mobs.iter().position(|m| m.kind == Kind::MossWolf && m.alive()).map(|i| self.mobs.remove(i));
        self.trail.clear();
        // Her raven, javelins and charge don't follow her between levels.
        self.ravens.clear();
        self.javelins.clear();
        self.axes.clear();
        self.souls.clear();
        self.lanterns.clear();
        self.chains_fx.clear();
        self.sweeps.clear();
        self.clouds.clear();
        self.fungi.clear();
        self.vines.clear();
        self.glasses.clear();
        self.links.clear();
        self.whips.clear();
        self.binds.clear();
        self.p.judge_t = 0.0;
        self.p.rune_prey = None;
        self.p.runes = 0;
        self.p.charge = None;
        self.p.whirl_t = 0.0;
        // Her wolf comes with her (it's made anew beside her on the other side).
        self.mobs.retain(|m| m.kind != Kind::DireWolf);
        let cur = self.swap_out();
        self.parked.insert(cur.id, cur);
        let lv = match self.parked.remove(&id) {
            Some(lv) => lv,
            None => world::build_at(id, self.level_seed(id), self.quest.difficulty),
        };
        self.swap_in(lv);
        self.stats.levels_entered += 1;
        if id.overland() {
            self.save_due = true;
        }
        self.sfx.push(Sfx::Descend);
        // Where do we arrive?
        let spot = match (id, from) {
            (land, Some(LevelId::Dungeon(k, _))) if land.overland() => self.portal_spot(PortalKind::Entrance(k)),
            // From the next area over: at the road back to it.
            (land, Some(other)) if land.overland() && other.overland() && land.act() == other.act() => self.portal_spot(PortalKind::Exit(crate::areas::number(other))),
            (land, Some(other)) if land.overland() && other.overland() => self.portal_spot(PortalKind::Pass(other.act())),
            (LevelId::Dungeon(_, f), Some(LevelId::Dungeon(_, g))) if g > f => self.portal_spot(PortalKind::Down),
            (LevelId::Dungeon(..), _) => self.portal_spot(PortalKind::Up),
            _ => None,
        };
        let (x, y) = spot.unwrap_or(self.start);
        self.p.x = x;
        self.p.y = y;
        // This area's random errand starts as you arrive.
        self.errand_enter();
        if let Some(mut w) = wolf {
            let (wx, wy) = if self.d.blocked(x + 1.0, y, 0.35) { (x, y) } else { (x + 1.0, y) };
            w.x = wx;
            w.y = wy;
            w.path.clear();
            self.mobs.push(w);
        }
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
            // Not your own allies (wolves, thralls, rats...), nor crates and barrels.
            .filter(|m| m.alive() && m.charm <= 0.0 && !crate::breakables::is_prop(m.kind))
            .map(|m| (m.x, m.y, ((m.x - self.p.x).powi(2) + (m.y - self.p.y).powi(2)).sqrt()))
            .min_by(|a, b| a.2.partial_cmp(&b.2).unwrap())
            .map(|(x, y, d)| (x, y, d, self.d.los(self.p.x, self.p.y, x, y)))
    }

    /// Nearest food on the floor (for the test bot).
    pub fn bot_food(&self) -> Option<(f32, f32)> {
        self.pickups
            .iter()
            .filter(|k| matches!(k.kind, Drop::Food(_) | Drop::Seal(_) | Drop::Rune(_) | Drop::Sigil(_) | Drop::Key(_) | Drop::Pearl(_) | Drop::Shard(_)))
            .map(|k| (k.x, k.y, (k.x - self.p.x).powi(2) + (k.y - self.p.y).powi(2)))
            .min_by(|a, b| a.2.partial_cmp(&b.2).unwrap())
            .map(|(x, y, _)| (x, y))
    }

    /// Where the bot should head next when nothing is in reach: deeper, or into the next dungeon.
    pub fn bot_portal(&self) -> Option<(f32, f32)> {
        let at = |kind: PortalKind| self.portals.iter().find(|p| p.kind == kind).map(|p| (p.x, p.y));
        let target = self.bot_dungeon();
        match self.level {
            // Done here (its token is in hand, or it's the wrong dungeon): head back out.
            LevelId::Dungeon(k, _) if k != target => at(PortalKind::TownPortal).or_else(|| at(PortalKind::Up)),
            // Deeper, until the boss is dead; then out.
            LevelId::Dungeon(..) => {
                let boss_dead = self.mobs.iter().all(|m| !m.boss || !m.alive());
                at(PortalKind::Down).or_else(|| if boss_dead { at(PortalKind::TownPortal).or_else(|| at(PortalKind::Up)) } else { None })
            }
            // Outdoors: the door if it's in this area, else the road toward the area that holds it.
            _ => {
                let home = crate::areas::dungeon_home(target);
                match crate::areas::route(self.level, home) {
                    Some(next) if self.level != home => at(PortalKind::Exit(crate::areas::number(next))),
                    _ => at(PortalKind::Entrance(target)),
                }
            }
        }
    }

    /// A herald token (seal, rune, sigil, key) lying on this level.
    pub fn bot_token(&self) -> Option<(f32, f32)> {
        self.pickups.iter().find(|k| matches!(k.kind, Drop::Seal(_) | Drop::Rune(_) | Drop::Sigil(_) | Drop::Key(_) | Drop::Pearl(_) | Drop::Shard(_))).map(|k| (k.x, k.y))
    }

    /// The role of the NPC that talking now would address (for the bot).
    pub fn nearest_npc_role(&self, r: f32) -> Option<Role> {
        self.nearest_npc(r).map(|i| self.npcs[i].role)
    }

    /// Where this level's living boss is (the bot hunts it down on the last floor).
    pub fn bot_boss(&self) -> Option<(f32, f32)> {
        self.mobs.iter().find(|m| m.boss && m.alive()).map(|m| (m.x, m.y))
    }

    /// First dungeon of this act whose herald token the bot doesn't have yet (then the act's
    /// last dungeon). Each act has four dungeons in order: three heralds, then the final one.
    /// For the bot: a waypoint worth taking (one known in an area closer to where it's going), outdoors.
    pub fn bot_waypoint_goal(&self) -> Option<LevelId> {
        if !self.level.overland() {
            return None;
        }
        let home = crate::areas::dungeon_home(self.bot_dungeon());
        if home.act() != self.level.act() {
            return None;
        }
        let here = crate::areas::steps(self.level, home)?;
        self.waypoints
            .iter()
            .filter(|w| w.act() == self.level.act() && w.overland())
            .filter_map(|&w| crate::areas::steps(w, home).map(|s| (w, s)))
            .filter(|&(_, s)| s + 1 < here)
            .min_by_key(|&(_, s)| s)
            .map(|(w, _)| w)
    }

    /// Where this level's waypoint stands, and whether it's been found (for the bot).
    pub fn bot_waypoint(&self) -> ((f32, f32), bool) {
        (self.waypoint, self.waypoints.contains(&self.level))
    }

    fn bot_dungeon(&self) -> usize {
        let a = self.level.act().min(5);
        let got = match a {
            0 => self.quest.seals,
            1 => self.quest.runes,
            2 => self.quest.sigils,
            3 => self.quest.keys,
            4 => self.quest.pearls,
            _ => self.quest.shards,
        };
        let _ = SANCTUM;
        (0..3).find(|&k| !got[k]).map_or(a * 4 + 3, |k| a * 4 + k)
    }

    /// The quest log line for the act you're in.
    pub fn quest_log(&self) -> String {
        if let Some((_, _, _, guardian, done)) = self.rift_hud() {
            return if done {
                "THE RIFT IS CLEARED. TAKE THE WAY HOME AT ITS START".into()
            } else if guardian {
                "SLAY THE RIFT GUARDIAN".into()
            } else {
                "SLAY THE RIFT'S MONSTERS TO CALL ITS GUARDIAN".into()
            };
        }
        match self.level.act() {
            5 => self.quest.log6(),
            4 => self.quest.log5(),
            3 => self.quest.log4(),
            2 => self.quest.log3(),
            1 => self.quest.log2(),
            _ => self.quest.log(),
        }
    }

    /// The act's story-giver, when they have news for the bot (Elder, Captain, Hunter, Tally).
    pub fn bot_story_npc(&self) -> Option<Role> {
        let q = &self.quest;
        match self.level {
            LevelId::Overworld if q.elder_has_news() => Some(Role::Elder),
            LevelId::Frostmarch if q.captain_has_news() => Some(Role::Captain),
            LevelId::Mistwood if q.hunter_has_news() => Some(Role::Hunter),
            LevelId::Mechanus if q.tally_has_news() => Some(Role::Tally),
            LevelId::Deep if q.ysolde_has_news() => Some(Role::Ysolde),
            LevelId::Heavens if q.seraphine_has_news() => Some(Role::Seraphine),
            _ => None,
        }
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
            // Each town has its own tune.
            [Track::Town, Track::Hearth, Track::Vigil, Track::Refuge, Track::Brine, Track::Harbor][self.level.act().min(5)]
        } else if self.level.act() == 5 {
            // The heavens: a soaring choir and wind outside, the storm and the burning sanctum within.
            if self.level.overland() {
                Track::Sky
            } else {
                Track::Storm
            }
        } else if self.level.act() == 4 {
            // The deep: a slow swell of whale-song outside, the pressure drone below.
            if self.level.overland() {
                Track::Tide
            } else {
                Track::Abyss
            }
        } else if self.level.act() == 3 {
            // Mechanus: a ticking harpsichord outside, the engine's clangour in the works.
            if self.level.overland() {
                Track::Gears
            } else {
                Track::Engine
            }
        } else if self.level.act() == 2 {
            // The Mistwood: a haunted waltz outside, the organ in the dungeons and the castle.
            if self.level.overland() {
                Track::Mist
            } else {
                Track::Crypt
            }
        } else if self.level.act() == 1 {
            // The north has its own music: Kaldholm and the snowfields share the windy theme.
            if self.level.overland() {
                Track::Frost
            } else {
                Track::Ice
            }
        } else if self.level.overland() {
            Track::Wilds
        } else {
            Track::Dungeon
        }
    }

    pub fn fire_active(&self) -> bool {
        !self.balls.is_empty() && !self.lights.is_empty()
    }

    pub fn alive_mobs(&self) -> usize {
        self.mobs.iter().filter(|m| m.alive() && !crate::breakables::is_prop(m.kind)).count()
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
        if let Some(sel) = self.choose {
            self.update_choose(inp, sel, p_cast || p_confirm, p_lmb);
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
        self.mob_cues();
        self.trail.push_back((self.p.x, self.p.y));
        if self.trail.len() > 180 {
            self.trail.pop_front();
        }
        self.update_shots();
        self.update_hazards();
        self.update_deep();
        self.update_side();
        self.update_wyrm();
        self.update_errands();
        self.update_sky();
        self.update_rift();
        self.second_wind_t = (self.second_wind_t - DT).max(0.0);
        self.update_clockwork();
        self.update_balls();
        self.update_novas();
        self.update_fire_ground();
        self.update_big_fire();
        self.update_vampire();
        self.update_inventor();
        self.update_valkyrie();
        self.update_berserker();
        self.update_reaper();
        self.update_druid();
        self.update_inquisitor();
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
        // You wake in the town of the act you fell in.
        let home = LevelId::land(self.level.act());
        if self.level != home {
            self.go_to(home, None);
        }
        (self.p.x, self.p.y) = self.town_start;
        let town = Self::waypoint_name(home);
        if lost > 0 {
            self.say(format!("YOU WAKE IN {town}. LOST {lost} GOLD."));
        } else {
            self.say(format!("YOU WAKE IN {town}."));
        }
    }

    /// Greets a returning player (after loading a save).
    pub fn welcome_back(&mut self) {
        self.stats.levels_entered = 1; // skip the first-visit title card
        let msg = format!("WELCOME BACK, {}. CHAR LEVEL {}, {}/3 SEALS", self.p.skills.class.name(), self.p.clvl, self.quest.seal_count());
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
                if k == GLACIER && self.quest.stage2 < 2 {
                    self.portal_cd = 2.0;
                    let n = self.quest.rune_count();
                    self.say(format!("A WALL OF BLACK ICE SEALS THE GLACIER. ({n}/3 RUNES)"));
                    return;
                }
                if k == CASTLE && self.quest.stage3 < 2 {
                    self.portal_cd = 2.0;
                    let n = self.quest.sigil_count();
                    self.say(format!("THE CASTLE GATE WILL NOT MOVE. ({n}/3 SIGILS)"));
                    return;
                }
                if k == HEART && self.quest.stage4 < 2 {
                    self.portal_cd = 2.0;
                    let n = self.quest.key_count();
                    self.say(format!("THREE KEYHOLES IN A DOOR OF GEARS. ({n}/3 KEYS)"));
                    return;
                }
                if k == ZENITH && self.quest.stage6 < 2 {
                    self.portal_cd = 2.0;
                    let n = self.quest.shard_count();
                    self.say(format!("A GATE OF BLACK STONE, THREE SUNBURSTS CARVED IN IT. ({n}/3 SHARDS)"));
                    return;
                }
                if k == ABYSS && self.quest.stage5 < 2 {
                    self.portal_cd = 2.0;
                    let n = self.quest.pearl_count();
                    self.say(format!("THREE HOLLOWS IN THE STONE, SHAPED FOR PEARLS. ({n}/3 PEARLS)"));
                    return;
                }
                self.go_to(LevelId::Dungeon(k, 0), Some(here));
            }
            PortalKind::Dock(n) => self.fly_airship(n),
            PortalKind::Exit(n) => {
                let to = crate::areas::level(here.act(), n);
                let first = !self.waypoints.contains(&to) && !self.parked.contains_key(&to);
                self.go_to(to, Some(here));
                if first {
                    self.banner_t = 3.0;
                }
            }
            PortalKind::Pass(act) => {
                if act == 1 && here.act() == 0 && !self.quest.north_open() {
                    self.portal_cd = 2.0;
                    self.say("THE PASS IS CHOKED WITH ASH AND SNOW. NOT WHILE THE ASH KING LIVES.".into());
                    return;
                }
                if act == 2 && !self.quest.mists_open() {
                    self.portal_cd = 2.0;
                    self.say("A WALL OF MIST. YOU WALK IN... AND OUT AGAIN WHERE YOU STARTED.".into());
                    return;
                }
                if act == 3 && !self.quest.gears_open() {
                    self.portal_cd = 2.0;
                    self.say("A RING OF STILL BRASS GEARS IN THE ROCK. SOMETHING IN THE CASTLE HOLDS THEM.".into());
                    return;
                }
                if act == 5 && !self.quest.skies_open() {
                    self.portal_cd = 2.0;
                    self.say("A STAIR OF FAINT LIGHT. IT WON'T BEAR YOUR WEIGHT WHILE THE LEVIATHAN LIVES.".into());
                    return;
                }
                if act == 4 && !self.quest.deep_open() {
                    self.portal_cd = 2.0;
                    self.say("AN OLD DIVING BELL. ITS CHAINS ARE LOCKED BY THE CLOCK'S OWN GEARS.".into());
                    return;
                }
                // Coming back from the next act lands you at this act's pass (an area, or the overland).
                self.go_to(crate::areas::pass_home(act, here.act()), Some(here));
                if act == 1 && here.act() == 0 && !self.waypoints.contains(&LevelId::Frostmarch) {
                    self.say("THE FROSTMARCH. FIND KALDHOLM, BY THE FROZEN LAKE".into());
                }
                if act == 2 && !self.waypoints.contains(&LevelId::Mistwood) {
                    self.say("THE MISTWOOD. FIND THE VILLAGE OF MOURNHOLD".into());
                }
                if act == 3 && !self.waypoints.contains(&LevelId::Mechanus) {
                    self.say("MECHANUS, THE CLOCKWORK DOMINION. FIND THE LAST ESCAPEMENT".into());
                }
                if act == 5 && !self.waypoints.contains(&LevelId::Heavens) {
                    self.say("THE SHATTERED HEAVENS. FIND WINDWARD ANCHORAGE. MIND THE EDGES".into());
                }
                if act == 4 && !self.waypoints.contains(&LevelId::Deep) {
                    self.say("THE DROWNED DEEP. FIND BRINEHOLLOW, THE TOWN ON STILTS".into());
                }
            }
            PortalKind::Up => match here {
                LevelId::Dungeon(k, 0) => self.go_to(crate::areas::dungeon_home(k), Some(LevelId::Dungeon(k, 0))),
                LevelId::Dungeon(k, f) => self.go_to(LevelId::Dungeon(k, f - 1), Some(here)),
                _ => {}
            },
            PortalKind::Down => {
                if let LevelId::Dungeon(k, f) = here {
                    self.go_to(LevelId::Dungeon(k, f + 1), Some(here));
                }
            }
            PortalKind::TownPortal => {
                // A boss's token left lying here comes with you (stepping in too soon used to strand it).
                let (px, py) = (self.p.x, self.p.y);
                let mut any = false;
                for k in self.pickups.iter_mut() {
                    if matches!(k.kind, Drop::Seal(_) | Drop::Rune(_) | Drop::Sigil(_) | Drop::Key(_) | Drop::Pearl(_) | Drop::Shard(_)) {
                        (k.x, k.y, k.t) = (px, py, 1.0);
                        any = true;
                    }
                }
                if any {
                    self.collect_pickups();
                }
                self.go_to(LevelId::land(here.act()), None);
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
            if !matches!(n.role, Role::Villager(_) | Role::Fisher(_) | Role::Peasant(_) | Role::Servant(_) | Role::Diver(_) | Role::Deckhand(_)) {
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
        let mut d = self.endgame_dialog(role).unwrap_or_else(|| story::talk(role, &self.quest));
        if d.heals {
            // Aldric can also make you forget your skills, for a price.
            let price = 50 * self.p.clvl as i32;
            d.options = vec![(format!("FORGET MY SKILLS  {price} GOLD"), Act::Respec(price)), ("FAREWELL".into(), Act::Close)];
        }
        // Side quests this person has (side.rs).
        self.side_options(role, &mut d);
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
            Some(Act::Combine) => self.combine_gems(),
            Some(Act::Rekindle(a)) => {
                self.dialog = None;
                self.rekindle(a as usize);
            }
            Some(Act::OpenRift(t)) => {
                self.dialog = None;
                self.open_rift(t);
            }
            Some(Act::Ember(k)) => {
                self.spend_ember(k);
                // Talk again: the Riftwarden's list shows the new counts.
                if let Some(d) = self.endgame_dialog(Role::Riftwarden) {
                    let mut d = d;
                    d.page = d.pages.len() - 1;
                    d.refresh_options();
                    self.dialog = Some(d);
                } else {
                    self.dialog = None;
                }
            }
            Some(Act::Jewel) => {
                self.open_inventory();
                self.inv.as_mut().unwrap().jewel = true;
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
            Some(Act::Side(q)) => {
                // Leaving the story conversation still counts as having heard it.
                if let Some(stage) = self.dialog.as_ref().and_then(|d| d.advance_to) {
                    self.advance_quest(stage);
                }
                self.side_talk(q as usize);
            }
            None => {}
        }
    }

    /// The jeweler joins every three alike gems you carry, while your gold lasts.
    fn combine_gems(&mut self) {
        let (mut made, mut spent, mut short) = (vec![], 0, false);
        while let Some(up) = self.p.gear.next_combine() {
            let cost = crate::items::combine_cost(up);
            if self.p.gold < cost {
                short = true;
                break;
            }
            self.p.gold -= cost;
            spent += cost;
            self.p.gear.combine_one();
            made.push(up);
        }
        if made.is_empty() {
            self.say(if short { "NOT ENOUGH GOLD".into() } else { "YOU NEED THREE GEMS OF ONE KIND AND GRADE".into() });
            return;
        }
        self.sfx.push(Sfx::Pickup);
        let last = crate::items::gem_name(*made.last().unwrap());
        self.say(if made.len() == 1 { format!("MADE A {last} FOR {spent} GOLD") } else { format!("MADE {} GEMS FOR {spent} GOLD", made.len()) });
        self.save_due = true;
    }

    fn advance_quest(&mut self, stage: u8) {
        // 51 and 52 are Act 6's stages 1 and 2 (Seraphine).
        if stage > 50 {
            let s6 = stage - 50;
            if s6 > self.quest.stage6 {
                self.quest.stage6 = s6;
                self.sfx.push(Sfx::Pickup);
                match s6 {
                    1 => self.say("NEW QUEST: SLAY THE THREE HERALDS OF THE SKY".into()),
                    2 => self.say("THE TRUE SANCTUM IS OPEN".into()),
                    _ => {}
                }
            }
            return;
        }
        // 41 and 42 are Act 5's stages 1 and 2 (Captain Ysolde).
        if stage > 40 {
            let s5 = stage - 40;
            if s5 > self.quest.stage5 {
                self.quest.stage5 = s5;
                self.sfx.push(Sfx::Pickup);
                match s5 {
                    1 => self.say("NEW QUEST: SLAY THE THREE HERALDS OF THE DEEP".into()),
                    2 => self.say("THE DROWNED SANCTUM IS OPEN".into()),
                    _ => {}
                }
            }
            return;
        }
        // 31 and 32 are Act 4's stages 1 and 2 (Tally).
        if stage > 30 {
            let s4 = stage - 30;
            if s4 > self.quest.stage4 {
                self.quest.stage4 = s4;
                self.sfx.push(Sfx::Pickup);
                match s4 {
                    1 => self.say("NEW QUEST: SILENCE THE THREE HERALDS OF MECHANUS".into()),
                    2 => self.say("THE HEART OF THE CLOCK IS OPEN".into()),
                    _ => {}
                }
            }
            return;
        }
        // 21 and 22 are Act 3's stages 1 and 2 (Abelard).
        if stage > 20 {
            let s3 = stage - 20;
            if s3 > self.quest.stage3 {
                self.quest.stage3 = s3;
                self.sfx.push(Sfx::Pickup);
                match s3 {
                    1 => self.say("NEW QUEST: SLAY THE THREE SKELETON LORDS".into()),
                    2 => self.say("THE GATE OF CASTLE VARDAK IS OPEN".into()),
                    _ => {}
                }
            }
            return;
        }
        // 11 and 12 are Act 2's stages 1 and 2 (Captain Brenna).
        if stage > 10 {
            let s2 = stage - 10;
            if s2 > self.quest.stage2 {
                self.quest.stage2 = s2;
                self.sfx.push(Sfx::Pickup);
                match s2 {
                    1 => self.say("NEW QUEST: SLAY THE THREE FROST HERALDS".into()),
                    2 => self.say("THE GLACIER'S HEART IS UNSEALED".into()),
                    _ => {}
                }
            }
            return;
        }
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
        if self.p.skills.class == crate::skills::Class::Vampire && matches!(w, Ware::Bread | Ware::Roast) {
            self.say("YOU THIRST FOR BLOOD, NOT BREAD".into());
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
        // Haste and Mana shrines (side.rs).
        let (bspeed, bmana) = (self.bless_speed(), self.bless_mana());
        let p = &mut self.p;
        p.flash = (p.flash - DT).max(0.0);
        use crate::items::Stat;
        // Faster cast rate: cast animations and recoveries run quicker (slower while chilled).
        p.chill = (p.chill - DT).max(0.0);
        let fcr = (1.0 + p.bonus.frac(Stat::Cast, 60)) * if p.chill > 0.0 { 0.75 } else { 1.0 } * p.haste * bspeed;
        p.cast_cd = (p.cast_cd - DT * fcr).max(0.0);
        p.cast_t = (p.cast_t - DT * fcr).max(0.0);
        let mregen = 1.0 + p.bonus.frac(Stat::ManaRegen, 200);
        let base_regen = if p.skills.class == crate::skills::Class::Inventor {
            // Cooling (gear "mana regeneration" cools faster too).
            crate::inventor::COOLING * crate::inventor::tinker_cool(p.skills.rank(crate::skills::Skill::Tinkerer))
        } else if matches!(p.skills.class, crate::skills::Class::Valkyrie | crate::skills::Class::Berserker | crate::skills::Class::Reaper | crate::skills::Class::Inquisitor) {
            // Valor, rage and judgment don't regenerate: they fight for it.
            0.0
        } else {
            2.2 * p.skills.regen_mult()
        };
        p.mana = (p.mana + base_regen * mregen * bmana * DT).min(p.max_mana);
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
                let word = if self.p.skills.class == crate::skills::Class::Vampire { "BLOODTHIRSTY" } else { "STARVING" };
                self.floater(self.p.x, self.p.y, word.into(), rgb(0xff6030));
            }
            if self.p.hp <= 0.0 {
                self.p.hp = 0.0;
                self.state = State::Dead(0.0);
                self.sfx.push(Sfx::Die);
                return;
            }
        } else if self.p.food < 25.0 && self.p.hunger_msg <= 0.0 {
            self.p.hunger_msg = 12.0;
            let word = if self.p.skills.class == crate::skills::Class::Vampire { "THIRSTY" } else { "HUNGRY" };
            self.floater(self.p.x, self.p.y, word.into(), rgb(0xe0a040));
        }

        if inp.potion_hp && self.p.hp_pots > 0 && self.p.hp < self.p.max_hp {
            self.p.hp_pots -= 1;
            self.p.hp = (self.p.hp + 40.0 + self.p.max_hp * 0.15).min(self.p.max_hp);
            self.sfx.push(Sfx::Drink);
            self.floater(self.p.x, self.p.y, "HEALED".into(), rgb(0xff5050));
        }
        // The inventor vents steam instead (mana potions are coolant when the vent isn't ready).
        let vented = inp.potion_mp && self.is_inventor() && self.vent();
        if inp.potion_mp && !vented && self.p.mp_pots > 0 && self.p.mana < self.p.max_mana {
            self.p.mp_pots -= 1;
            self.p.mana = (self.p.mana + 35.0 + self.p.max_mana * 0.15).min(self.p.max_mana);
            self.sfx.push(Sfx::Drink);
            let word = if self.is_reaper() {
                "INK"
            } else if self.is_valkyrie() || self.is_berserker() {
                "MEAD"
            } else if self.is_inquisitor() {
                "HOLY OIL"
            } else {
                "MANA"
            };
            self.floater(self.p.x, self.p.y, word.into(), rgb(0x5080ff));
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
        let on_hud = inp.mouse.map_or(false, |(mx, my)| my >= self.view_h - HUD_H || crate::render::on_hud_orb(crate::gfx::SW, self.view_h, mx, my));

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
        // A monster aimed at (clicked, or picked by the pad's auto-target): melee walks to it.
        let mut aimed_mob = self.hover;
        let mut melee_goal: Option<(f32, f32)> = None;
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
                    aimed_mob = Some(i);
                } else if let Some(m) = inp.mouse.filter(|_| inp.move_x == 0.0 && inp.move_y == 0.0) {
                    cast_at = Some(self.mouse_world(m));
                } else {
                    let (dx, dy) = dir_vec(self.p.dir);
                    cast_at = Some((self.p.x + dx * 6.0, self.p.y + dy * 6.0));
                }
            }
        }
        if let Some((tx, ty)) = cast_at {
            self.p.talk_to = None;
            let (dx, dy) = (tx - self.p.x, ty - self.p.y);
            if dx * dx + dy * dy > 0.01 {
                self.p.dir = iso::dir8(dx, dy);
            }
            let skill = if use_secondary { self.p.skills.secondary } else { self.p.skills.primary };
            // The valkyrie's melee: out of reach of the monster she's aiming at, walk in first.
            let reach = if self.is_valkyrie() {
                crate::valkyrie::reach_of(skill)
            } else if self.is_berserker() {
                crate::berserker::reach_of(skill)
            } else if self.is_reaper() {
                crate::reaper::reach_of(skill)
            } else if self.is_inquisitor() {
                crate::inquisitor::reach_of(skill, self.p.skills.rank(crate::skills::Skill::Zealotry))
            } else {
                None
            };
            let far = match (reach, aimed_mob) {
                (Some(reach), Some(i)) => {
                    let m = &self.mobs[i];
                    (m.x - self.p.x).powi(2) + (m.y - self.p.y).powi(2) > (reach + m.r * 0.5).powi(2)
                }
                _ => false,
            };
            if far {
                // Keep the path between ticks while walking in (it's re-planned when the foe moves).
                melee_goal = Some((tx, ty));
            } else {
                self.p.path.clear();
                self.p.goal = None;
                self.cast_skill(skill, tx, ty);
            }
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
        } else if cast_at.is_none() || melee_goal.is_some() {
            let talk_goal = self.p.talk_to.map(|i| (self.npcs[i].x, self.npcs[i].y));
            let want = if let Some(g) = melee_goal {
                Some(g)
            } else if let Some(g) = talk_goal {
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
            } else if let Some(goal) = self.p.goal.filter(|_| inp.lmb || talk_goal.is_some() || melee_goal.is_some()) {
                // Clicked somewhere unreachable: walk straight at it and slide along walls.
                let (dx, dy) = (goal.0 - self.p.x, goal.1 - self.p.y);
                let l = (dx * dx + dy * dy).sqrt();
                if l > 0.15 {
                    mv = (dx / l, dy / l);
                }
            }
        }
        // On the charging warhorse, the horse goes where it goes.
        if self.p.charge.is_some() || self.p.whirl_t > 0.0 {
            mv = (0.0, 0.0);
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
        let base = if self.p.phoenix_t > 0.0 || self.p.embrace_t > 0.0 { base * 1.4 } else if self.p.suit_t > 0.0 { base * 1.2 } else { base };
        let base = base * (1.0 + gear.frac(crate::items::Stat::Move, 50)) * self.ember_speed() * if self.has_power(crate::items::P_ASHWALKER) { 1.2 } else { 1.0 };
        let base = base * self.bless_speed() * self.sack_slow();
        let base = if self.p.chill > 0.0 { base * 0.6 } else { base };
        let base = if self.flooded(self.p.x, self.p.y) { base * crate::tides::WADE } else { base };
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
        // The vampire can't eat: she only feeds on blood.
        let full = self.p.food > MAX_FOOD - 8.0 || self.p.skills.class == crate::skills::Class::Vampire;
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
                    let n = (n as f32 * (1.0 + self.p.bonus.frac(crate::items::Stat::Gold, 300) + self.ember_fortune() as f32 / 100.0)).round() as i32;
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
                Drop::Page(i) => self.read_page(i),
                Drop::Hoard(n) => self.grab_hoard(n),
                Drop::Herb | Drop::Heirloom | Drop::Clue => self.errand_pick(&k),
                Drop::Key(i) => {
                    self.quest.keys[i] = true;
                    self.p.skills.points += 1;
                    self.p.base_hp += 24.0;
                    self.p.base_mana += 16.0;
                    self.p.recalc();
                    self.p.power *= 1.1;
                    self.p.hp = self.p.max_hp;
                    self.p.mana = self.p.max_mana;
                    self.sfx.push(Sfx::Descend);
                    let name = ["THE FORGEMOTHER", "THE CANTOR", "THE ARCHIVIST"][i];
                    self.floater(px, py, format!("WINDING KEY OF {name}"), rgb(0xe0b040));
                    let n = self.quest.key_count();
                    if n == 3 {
                        self.say("ALL THREE KEYS. RETURN TO TALLY".into());
                    } else {
                        self.say(format!("THE KEY TURNS ITSELF IN YOUR HAND, AND YOU FEEL STRONGER ({n}/3)"));
                    }
                }
                Drop::Shard(i) => {
                    self.quest.shards[i] = true;
                    self.p.skills.points += 1;
                    self.p.base_hp += 32.0;
                    self.p.base_mana += 20.0;
                    self.p.recalc();
                    self.p.power *= 1.1;
                    self.p.hp = self.p.max_hp;
                    self.p.mana = self.p.max_mana;
                    self.sfx.push(Sfx::Descend);
                    let name = ["SERAPH-COMMANDER VAEL", "THE TEMPEST DRAKE", "THE OPHAN PRIME"][i];
                    self.floater(px, py, format!("THE SUN-SHARD OF {name}"), rgb(0xffe080));
                    let n = self.quest.shard_count();
                    if n == 3 {
                        self.say("ALL THREE SHARDS. RETURN TO SERAPHINE".into());
                    } else {
                        self.say(format!("THE SHARD BURNS WARM IN YOUR HAND, AND YOU FEEL STRONGER ({n}/3)"));
                    }
                }
                Drop::Pearl(i) => {
                    self.quest.pearls[i] = true;
                    self.p.skills.points += 1;
                    self.p.base_hp += 28.0;
                    self.p.base_mana += 18.0;
                    self.p.recalc();
                    self.p.power *= 1.1;
                    self.p.hp = self.p.max_hp;
                    self.p.mana = self.p.max_mana;
                    self.sfx.push(Sfx::Descend);
                    let name = ["ADMIRAL DREGMOOR", "MOTHER NACRE", "THE ANGLER MATRIARCH"][i];
                    self.floater(px, py, format!("THE PEARL OF {name}"), rgb(0x80e8e0));
                    let n = self.quest.pearl_count();
                    if n == 3 {
                        self.say("ALL THREE PEARLS. RETURN TO CAPTAIN YSOLDE".into());
                    } else {
                        self.say(format!("THE PEARL HUMS IN YOUR HAND, AND YOU FEEL STRONGER ({n}/3)"));
                    }
                }
                Drop::Sigil(i) => {
                    self.quest.sigils[i] = true;
                    self.p.skills.points += 1;
                    self.p.base_hp += 20.0;
                    self.p.base_mana += 14.0;
                    self.p.recalc();
                    self.p.power *= 1.1;
                    self.p.hp = self.p.max_hp;
                    self.p.mana = self.p.max_mana;
                    self.sfx.push(Sfx::Descend);
                    let name = ["OSSRIC", "GRIMHILDE", "MALGRAVE"][i];
                    self.floater(px, py, format!("GRAVE SIGIL OF {name}"), rgb(0x60f080));
                    let n = self.quest.sigil_count();
                    if n == 3 {
                        self.say("ALL THREE SIGILS. RETURN TO ABELARD".into());
                    } else {
                        self.say(format!("THE SIGIL'S GRAVE-COLD POWER FLOWS INTO YOU ({n}/3)"));
                    }
                }
                Drop::Rune(i) => {
                    self.quest.runes[i] = true;
                    self.p.skills.points += 1;
                    self.p.base_hp += 18.0;
                    self.p.base_mana += 12.0;
                    self.p.recalc();
                    self.p.power *= 1.12;
                    self.p.hp = self.p.max_hp;
                    self.p.mana = self.p.max_mana;
                    self.sfx.push(Sfx::Descend);
                    let name = ["THE GIANT", "THE MATRIARCH", "THE WITCH"][i];
                    self.floater(px, py, format!("FROST RUNE OF {name}"), rgb(0x90d0ff));
                    let n = self.quest.rune_count();
                    if n == 3 {
                        self.say("ALL THREE RUNES. RETURN TO CAPTAIN BRENNA".into());
                    } else {
                        self.say(format!("THE RUNE'S COLD POWER FLOWS INTO YOU ({n}/3)"));
                    }
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
        let r = if self.level.overland() { 13 } else { 9 };
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
                if self.level.overland() || self.d.los(px, py, cx, cy) || self.d.los(px, py, lx, ly) {
                    self.explored[i] = true;
                }
            }
        }
    }

    pub(crate) fn cast_fireball(&mut self, tx: f32, ty: f32, ember: bool) {
        let p = &mut self.p;
        let vampire = p.skills.class == crate::skills::Class::Vampire;
        let inventor = p.skills.class == crate::skills::Class::Inventor;
        if inventor {
            // Ray Pistol: fast single-target aether bolts (the weak shot when overheated).
            let r = p.skills.rank(crate::skills::Skill::RayPistol);
            if !ember {
                p.mana -= crate::inventor::ray_heat(r);
            }
            p.cast_cd = 0.22;
            p.cast_t = 0.22;
            p.cast_len = 0.22;
            let (px, py) = (p.x, p.y);
            let (dx, dy) = (tx - px, ty - py);
            let l = (dx * dx + dy * dy).sqrt().max(0.001);
            let (ux, uy) = (dx / l, dy / l);
            let (x, y) = (px + ux * 0.45, py + uy * 0.45);
            let (lo, hi) = crate::inventor::ray_dmg(r);
            let dmg = if ember { self.rng.rf(3.0, 5.0) * self.p.power.sqrt() } else { self.rng.rf(lo, hi) * self.fire_power() };
            self.balls.push(Fireball { x, y, vx: ux * 15.0, vy: uy * 15.0, life: 0.6, dmg, ember: true });
            self.stats.casts += 1;
            self.sfx.push(Sfx::Cast);
            return;
        }
        let fb_rank = p.skills.rank(if vampire { crate::skills::Skill::BloodLance } else { crate::skills::Skill::Fireball });
        let (len, speed, life) = if ember { (EMBER_CAST_TIME, EMBER_SPEED, 0.7) } else { (CAST_TIME, FIREBALL_SPEED, 1.1) };
        if !ember {
            p.mana -= if vampire { crate::vampire::lance_mana(fb_rank) } else { crate::skills::fireball_mana(fb_rank) };
        }
        p.cast_cd = len;
        p.cast_t = len;
        p.cast_len = len;
        let (dx, dy) = (tx - p.x, ty - p.y);
        let l = (dx * dx + dy * dy).sqrt().max(0.001);
        let (ux, uy) = (dx / l, dy / l);
        let (x, y) = (p.x + ux * 0.45, p.y + uy * 0.45);
        let power = p.power * p.skills.fire_mult() * crate::skills::fireball_synergy(p.skills.rank(crate::skills::Skill::Meteor));
        let (lo, hi) = if vampire { crate::vampire::lance_dmg(fb_rank) } else { crate::skills::fireball_dmg(fb_rank) };
        let dmg = if ember { self.rng.rf(3.0, 5.0) } else { self.rng.rf(lo, hi) } * power;
        self.balls.push(Fireball { x, y, vx: ux * speed, vy: uy * speed, life, dmg, ember });
        // Firestorm (the sorceress's ancient power): two more, fanned out.
        if !ember && !vampire && self.has_power(crate::items::P_FIRESTORM) {
            for a in [-0.26f32, 0.26] {
                let (c, s) = (a.cos(), a.sin());
                let (vx, vy) = (ux * c - uy * s, ux * s + uy * c);
                self.balls.push(Fireball { x, y, vx: vx * speed, vy: vy * speed, life, dmg: dmg * 0.6, ember });
            }
        }
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
            .filter(|(_, m)| m.alive() && m.charm <= 0.0)
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

    /// Damage multiplier against a monster: cold creatures burn better, the bloodless resist blood.
    pub(crate) fn taken(&self, kind: Kind) -> f32 {
        match self.p.skills.class {
            crate::skills::Class::Vampire => crate::vampire::blood_taken(kind),
            crate::skills::Class::Inventor => 1.0,
            crate::skills::Class::Valkyrie => crate::valkyrie::frost_taken(kind),
            crate::skills::Class::Berserker | crate::skills::Class::Reaper | crate::skills::Class::Druid | crate::skills::Class::Inquisitor => 1.0,
            crate::skills::Class::Sorceress => crate::mobs::fire_taken(kind),
        }
    }

    /// Frost slows you for a moment.
    pub(crate) fn chill(&mut self, t: f32) {
        // Northborn: frost barely slows her.
        let t = if self.is_valkyrie() && self.p.skills.rank(crate::skills::Skill::Northborn) > 0 { t * 0.5 } else { t };
        if self.p.chill <= 0.0 {
            let (x, y) = (self.p.x, self.p.y);
            self.floater(x, y, "CHILLED".into(), rgb(0x90d0ff));
        }
        self.p.chill = self.p.chill.max(t);
    }

    /// A blow from a monster of this kind (the Iron Halo turns away the cursed).
    pub(crate) fn hurt_by(&mut self, dmg: f32, src: Option<Kind>) {
        self.hurt_cursed = src.map_or(false, crate::inquisitor::kind_cursed);
        self.hurt_player(dmg);
        self.hurt_cursed = false;
    }

    /// Does the hero wear an ancient item with this power (items::POWERS)?
    pub fn has_power(&self, p: usize) -> bool {
        self.p.bonus.get(crate::items::Stat::Power) & (1 << p) != 0
    }

    #[cfg(test)]
    pub(crate) fn debug_gain_xp(&mut self, xp: f32) {
        self.gain_xp(xp);
    }

    pub(crate) fn hurt_player(&mut self, dmg: f32) {
        if matches!(self.state, State::Dead(_)) {
            return;
        }
        // Mist can't be touched.
        let dmg = if self.p.mist > 0.0 { 0.0 } else { self.p.armored(dmg) };
        // The steam suit takes half.
        let dmg = if self.p.suit_t > 0.0 { dmg * 0.5 } else { dmg };
        // An Armor shrine, and side-quest wards (side.rs).
        let dmg = dmg * self.bless_taken();
        // A Fragile rift: a quarter more.
        let dmg = if self.rift_has(crate::endgame::RiftMod::Fragile) { dmg * 1.25 } else { dmg };
        // Second Wind (ancient power): once a minute a killing blow leaves you at half life.
        if dmg >= self.p.hp && self.has_power(crate::items::P_SECOND_WIND) && self.second_wind_t <= 0.0 {
            self.second_wind_t = 60.0;
            self.p.hp = self.p.max_hp * 0.5;
            let (x, y) = (self.p.x, self.p.y);
            self.floater(x, y, "SECOND WIND!".into(), rgb(0xff8a30));
            return;
        }
        // The balance pass (2026-10-07): the Sky Pirate's plating and the Inquisitor's iron and faith take
        // the edge off; both dealt plenty but died far more than the rest late on.
        let dmg = dmg * match self.p.skills.class {
            crate::skills::Class::Inventor => 0.8,
            crate::skills::Class::Inquisitor => 0.85,
            _ => 1.0,
        };
        // The valkyrie: Northborn shrugs off part of it, the warhorse takes half while it charges,
        // and every blow she takes stokes her Valor.
        // The berserker: Iron Hide below half life, and every blow stokes her rage.
        let dmg = if self.is_berserker() {
            let low = self.p.hp < self.p.max_hp * 0.5;
            let dr = if low { crate::berserker::iron_dr(self.p.skills.rank(crate::skills::Skill::IronHide)) } else { 0.0 };
            let dmg = dmg * (1.0 - dr);
            if dmg > 0.0 {
                self.gain_rage(dmg * crate::berserker::RAGE_PER_HURT);
            }
            dmg
        } else {
            dmg
        };
        // The inquisitor: the Iron Halo turns away the cursed, and pain hardens her Judgment.
        let dmg = if self.is_inquisitor() {
            let r = self.p.skills.rank(crate::skills::Skill::IronHalo);
            let dmg = if self.hurt_cursed { dmg * (1.0 - crate::inquisitor::halo_dr(r)) } else { dmg };
            if dmg > 0.0 {
                self.gain_judgment(dmg * crate::inquisitor::halo_judge(r));
            }
            dmg
        } else {
            dmg
        };
        let dmg = if self.is_valkyrie() {
            let dr = crate::valkyrie::northborn_dr(self.p.skills.rank(crate::skills::Skill::Northborn));
            let dmg = dmg * (1.0 - dr) * if self.p.charge.is_some() { 0.5 } else { 1.0 };
            if dmg > 0.0 {
                self.gain_valor(dmg * crate::valkyrie::VALOR_PER_HURT);
            }
            dmg
        } else {
            dmg
        };
        if dmg > 0.0 {
            self.p.hp -= dmg;
            // Berserk: she can't die while the red mist lasts.
            if self.p.berserk_t > 0.0 {
                self.p.hp = self.p.hp.max(1.0);
            }
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
                if self.mobs.iter().any(|m| m.alive() && m.charm <= 0.0 && (m.x - b.x).powi(2) + (m.y - b.y).powi(2) < (m.r + 0.13).powi(2)) {
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
                    kind: if self.p.skills.class == crate::skills::Class::Vampire { PKind::Blood } else { PKind::Fire },
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
            let kind = if self.p.skills.class == crate::skills::Class::Vampire { PKind::Blood } else { PKind::Fire };
            self.parts.push(Particle { x, y, z: 20.0, vx: a.cos() * s, vy: a.sin() * s, vz, life, max: life, kind });
        }
        let target = self
            .mobs
            .iter()
            .enumerate()
            .filter(|(_, m)| m.alive() && m.charm <= 0.0 && m.invuln <= 0.0)
            .map(|(i, m)| (i, (m.x - x).powi(2) + (m.y - y).powi(2), m.r))
            .filter(|&(_, d2, r)| d2 < (r + 0.3).powi(2))
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .map(|v| v.0);
        let Some(i) = target else { return };
        self.stats.hits += 1;
        self.sfx.push(Sfx::Hit);
        let kind = self.mobs[i].kind;
        let dmg = dmg * self.taken(kind) * self.bless_damage();
        self.drain(kind, dmg, 1.0);
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
            let kind = if self.p.skills.class == crate::skills::Class::Vampire { PKind::Blood } else { PKind::Fire };
            self.parts.push(Particle { x, y, z: 18.0, vx: a.cos() * s, vy: a.sin() * s, vz, life, max: life, kind });
        }
        for _ in 0..6 {
            let a = self.rng.f() * std::f32::consts::TAU;
            let life = self.rng.rf(0.6, 1.1);
            self.parts.push(Particle { x, y, z: 16.0, vx: a.cos() * 0.6, vy: a.sin() * 0.6, vz: 18.0, life, max: life, kind: PKind::Smoke });
        }
        let mut hit_any = false;
        let blood = self.p.skills.class == crate::skills::Class::Vampire;
        for i in 0..self.mobs.len() {
            let m = &self.mobs[i];
            if !m.alive() || m.charm > 0.0 || m.invuln > 0.0 {
                continue;
            }
            let d2 = (m.x - x).powi(2) + (m.y - y).powi(2);
            let reach = 1.3 + m.r - 0.32;
            if d2 > reach * reach {
                continue;
            }
            let dmg = if d2 < (0.3 + m.r).powi(2) { dmg } else { dmg * 0.5 };
            hit_any = true;
            let (mx, my, boss, r, kind) = (m.x, m.y, m.boss, m.r, m.kind);
            let dmg = dmg * self.taken(kind) * self.bless_damage();
            self.drain(kind, dmg, 1.0);
            let burn = 2.0 * self.p.skills.burn_mult();
            let m = &mut self.mobs[i];
            m.hp -= dmg;
            m.flash = 0.12;
            if !blood {
                m.burn = burn;
            }
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
        // A crate, barrel or urn: it breaks (no XP, no kill hooks).
        if crate::breakables::is_prop(self.mobs[i].kind) {
            self.break_prop(i);
            return;
        }
        let (x, y, kind, boss, xp) = (self.mobs[i].x, self.mobs[i].y, self.mobs[i].kind, self.mobs[i].boss, self.mobs[i].xp);
        let marked = self.mobs[i].marked > 0.0;
        let (poisoned, plagued, pdps) = (self.mobs[i].poison_t > 0.0, self.mobs[i].plagued, self.mobs[i].poison);
        // The valkyrie kills a frozen foe: it shatters (after it's counted dead, so shards can't re-kill it).
        let shatter = self.is_valkyrie() && self.mobs[i].frozen > 0.0 && self.p.skills.rank(crate::skills::Skill::FrostBrand) > 0;
        self.mobs[i].frozen = 0.0;
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
        // Super uniques, gold-thieves and side quests (side.rs), and random errands (errands.rs).
        self.side_kill(i);
        let tag = self.mobs[i].errand;
        self.errand_kill(kind, tag);
        if shatter {
            self.shatter(x, y);
        }
        // Holy fire leaps from a cursed foe that dies burning.
        self.holy_leap(i);
        if self.is_berserker() && kind != Kind::DireWolf {
            self.berserker_kill();
        }
        if self.is_druid() && !matches!(kind, Kind::Rat | Kind::MossWolf | Kind::ThornWarden) {
            self.druid_kill(x, y, poisoned, plagued, pdps);
        }
        if self.is_reaper() && kind != Kind::Scholar {
            self.reaper_kill(x, y, boss, marked);
            // With the Ledger open, the dead rise to serve her.
            if self.p.ledger_t > 0.0 && (x - self.p.x).powi(2) + (y - self.p.y).powi(2) < 64.0 {
                self.raise_scholar(x, y, 10.0);
            }
        }
        // Boiler brutes blow their boilers when they fall.
        if kind == Kind::BoilerBrute {
            let dmg = (self.mobs[i].dmg.1 * 1.6).max(10.0);
            self.hazards.push(crate::mobs::Hazard { x, y, r: 2.0, warn: 0.7, live: 0.0, dps: 0.0, burst: dmg, t: 0.0, fired: false, kind: crate::mobs::HazardKind::Nova });
            self.floater(x, y, "HISSSS...".into(), rgb(0xff9040));
        }
        if mods & crate::mobs::M_FIERY != 0 {
            // Fire enchanted: bursts into flame a moment after it dies.
            let dmg = (self.mobs[i].dmg.1 * 1.4).max(8.0);
            self.hazards.push(crate::mobs::Hazard { x, y, r: 1.7, warn: 0.6, live: 0.0, dps: 0.0, burst: dmg, t: 0.0, fired: false, kind: crate::mobs::HazardKind::Nova });
        }
        // Break the marshal and its squad forgets its shape.
        if kind == Kind::Marshal {
            let mut any = false;
            for m in self.mobs.iter_mut() {
                if matches!(m.kind, Kind::Ordinal | Kind::Prism) && m.alive() && (m.x - x).powi(2) + (m.y - y).powi(2) < 100.0 {
                    m.stun = m.stun.max(2.5);
                    m.drilled = false;
                    // Broken ranks: no shield wall, and they take extra damage for a while.
                    m.broken = 6.0;
                    any = true;
                }
            }
            if any {
                self.floater(x, y, "DISORDER!".into(), rgb(0x80b0ff));
            }
        }
        // Goblins panic when one of their own falls (like D2's Fallen).
        if kind == Kind::Goblin {
            for m in self.mobs.iter_mut() {
                if m.kind == Kind::Goblin && m.alive() && (m.x - x).powi(2) + (m.y - y).powi(2) < 36.0 {
                    m.flee = 2.5;
                }
            }
        }
        // The Ash Rifts: the bar fills, and a guardian's fall clears the rift.
        let rank = self.mobs[i].rank;
        self.rift_kill(rank, boss);
        if boss && self.in_rift() {
            self.rift_cleared(x, y);
        }
        // Emberstorm (ancient power): the slain burst, scorching those around them.
        if self.has_power(crate::items::P_EMBERSTORM) {
            let blast = self.mobs[i].max_hp * 0.25;
            let near: Vec<usize> = (0..self.mobs.len()).filter(|&j| j != i && self.mobs[j].alive() && self.mobs[j].charm <= 0.0 && (self.mobs[j].x - x).powi(2) + (self.mobs[j].y - y).powi(2) < 6.25).collect();
            for _ in 0..8 {
                self.spray_at(x, y, PKind::Fire, 30.0);
            }
            for j in near {
                self.hit_mob(j, blast, 1.0, 0.0, None, true);
            }
        }
        if boss {
            self.stats.bosses += 1;
            self.save_due = true;
            self.shake = 1.0;
            self.lights.push(Light { x, y, r: 260.0, s: 1.5, life: 1.2, max: 1.2 });
            // A rekindled boss (or a rift guardian) gives its loot again, but not its relic or the ending.
            let story = !self.boss_story_done(kind) && !self.in_rift();
            if story {
            match kind {
                Kind::BoneWarden => self.pickups.push(Pickup { x, y, kind: Drop::Seal(0), t: 0.0 }),
                Kind::PlagueWarden => self.pickups.push(Pickup { x, y, kind: Drop::Seal(1), t: 0.0 }),
                Kind::HexWarden => self.pickups.push(Pickup { x, y, kind: Drop::Seal(2), t: 0.0 }),
                Kind::AshKing => {
                    self.quest.stage = 3;
                    self.state = State::Victory(0.0);
                    self.dialog = None;
                }
                Kind::FrostGiant => self.pickups.push(Pickup { x, y, kind: Drop::Rune(0), t: 0.0 }),
                Kind::YetiMatriarch => self.pickups.push(Pickup { x, y, kind: Drop::Rune(1), t: 0.0 }),
                Kind::RimeWitch => self.pickups.push(Pickup { x, y, kind: Drop::Rune(2), t: 0.0 }),
                Kind::WhiteDragon => {
                    self.quest.stage2 = 3;
                    self.state = State::Victory(0.0);
                    self.dialog = None;
                }
                Kind::Ossric => self.pickups.push(Pickup { x, y, kind: Drop::Sigil(0), t: 0.0 }),
                Kind::Grimhilde => self.pickups.push(Pickup { x, y, kind: Drop::Sigil(1), t: 0.0 }),
                Kind::Malgrave => self.pickups.push(Pickup { x, y, kind: Drop::Sigil(2), t: 0.0 }),
                Kind::Vardak => {
                    self.quest.stage3 = 3;
                    self.state = State::Victory(0.0);
                    self.dialog = None;
                }
                Kind::Forgemother => self.pickups.push(Pickup { x, y, kind: Drop::Key(0), t: 0.0 }),
                Kind::Cantor => self.pickups.push(Pickup { x, y, kind: Drop::Key(1), t: 0.0 }),
                Kind::Archivist => self.pickups.push(Pickup { x, y, kind: Drop::Key(2), t: 0.0 }),
                Kind::Clockmaker => {
                    self.quest.stage4 = 3;
                    self.state = State::Victory(0.0);
                    self.dialog = None;
                }
                Kind::Dregmoor => self.pickups.push(Pickup { x, y, kind: Drop::Pearl(0), t: 0.0 }),
                Kind::Nacre => self.pickups.push(Pickup { x, y, kind: Drop::Pearl(1), t: 0.0 }),
                Kind::Angler => self.pickups.push(Pickup { x, y, kind: Drop::Pearl(2), t: 0.0 }),
                Kind::Leviathan => {
                    self.quest.stage5 = 3;
                    self.state = State::Victory(0.0);
                    self.dialog = None;
                }
                Kind::Vael => self.pickups.push(Pickup { x, y, kind: Drop::Shard(0), t: 0.0 }),
                Kind::Tempest => self.pickups.push(Pickup { x, y, kind: Drop::Shard(1), t: 0.0 }),
                Kind::OphanPrime => self.pickups.push(Pickup { x, y, kind: Drop::Shard(2), t: 0.0 }),
                Kind::Solanthos => {
                    self.quest.stage6 = 3;
                    self.state = State::Victory(0.0);
                    self.dialog = None;
                }
                _ => {}
            }
            }
            for k in 0..3 {
                self.pickups.push(Pickup { x: x + k as f32 * 0.5 - 0.5, y: y + 0.6, kind: Drop::Gold(self.rng.range(40, 90)), t: 0.0 });
            }
            self.pickups.push(Pickup { x: x - 0.6, y: y - 0.4, kind: Drop::Health, t: 0.0 });
            // A way home: beyond the boss as seen from you, so walking over to its token (and loot)
            // never steps you into the portal first.
            let (ax, ay) = (x - self.p.x, y - self.p.y);
            let base = ay.atan2(ax);
            // It must be somewhere you can walk to (a spot past a wall corner once trapped the bot); failing
            // that, where the boss fell.
            let mut spot = (x, y);
            let (hx, hy) = (self.p.x as i32, self.p.y as i32);
            'find: for dist in [2.6f32, 2.0, 3.2, 1.4] {
                for k in [0.0f32, 0.6, -0.6, 1.2, -1.2, 1.8, -1.8, 2.4, -2.4, 3.1] {
                    let a = base + k;
                    let (px, py) = (x + a.cos() * dist, y + a.sin() * dist);
                    if !self.d.blocked(px, py, 0.45) && self.d.los(x, y, px, py) && self.d.path((hx, hy), (px as i32, py as i32), 6000).is_some() {
                        spot = (px, py);
                        break 'find;
                    }
                }
            }
            self.portals.push(Portal { x: spot.0, y: spot.1, kind: PortalKind::TownPortal });
            // The portal's opening shatters any crate, barrel or urn crowding it (one could wall it off).
            let crowding: Vec<usize> = (0..self.mobs.len())
                .filter(|&j| crate::breakables::is_prop(self.mobs[j].kind) && self.mobs[j].alive() && (self.mobs[j].x - spot.0).powi(2) + (self.mobs[j].y - spot.1).powi(2) < 4.0)
                .collect();
            for j in crowding {
                self.mobs[j].hp = 0.0;
                self.kill(j);
            }
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
        let mf = self.p.bonus.get(items::Stat::Magic) + self.ember_fortune();
        let mut drops = vec![];
        if boss {
            let key = match kind {
                Kind::BoneWarden => "bone",
                Kind::PlagueWarden => "plague",
                Kind::HexWarden => "hex",
                Kind::FrostGiant => "giant",
                Kind::YetiMatriarch => "yeti",
                Kind::RimeWitch => "witch",
                Kind::WhiteDragon => "dragon",
                Kind::Ossric => "ossric",
                Kind::Grimhilde => "grimhilde",
                Kind::Malgrave => "malgrave",
                Kind::Vardak => "vardak",
                Kind::Forgemother => "forgemother",
                Kind::Cantor => "cantor",
                Kind::Archivist => "archivist",
                Kind::Clockmaker => "clockmaker",
                Kind::Dregmoor => "dregmoor",
                Kind::Nacre => "nacre",
                Kind::Angler => "angler",
                Kind::Leviathan => "leviathan",
                Kind::Vael => "vael",
                Kind::Tempest => "tempest",
                Kind::OphanPrime => "ophan",
                Kind::Solanthos => "solanthos",
                Kind::WellWitch => "wellwitch",
                Kind::FireWyrm => "firewyrm",
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
        // Gems: now and then from anyone, always from a boss.
        let gem_chance = if boss {
            1.0
        } else {
            match rank {
                Rank::Normal | Rank::Minion => GEM_DROP,
                Rank::Champion => 0.15,
                Rank::Elite => 0.3,
            }
        };
        if self.rng.chance(gem_chance) {
            drops.push(items::gem_item(items::roll_gem(ilvl, &mut self.rng)));
        }
        // Half of all set drops are from your own hero's set.
        let hero = self.p.skills.class.name();
        for it in drops.iter_mut() {
            if it.rarity == items::Rarity::Set && self.rng.chance(0.5) {
                if let Some(mine) = items::hero_set_piece(hero, ilvl, &mut self.rng) {
                    *it = mine;
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

    pub(crate) fn gain_xp(&mut self, xp: f32) {
        // An Experience shrine, and a finished act of lore pages (side.rs).
        let xp = xp * self.bless_xp();
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
            self.ember_level_up();
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
                PKind::Fire | PKind::Magic | PKind::Spore | PKind::Holy => p.vz += 12.0 * DT,
                PKind::Frost => p.vz -= 60.0 * DT,
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
        // Out the north road into the Ashlands, then on to the Barrow Fields.
        walk_onto(&mut g, PortalKind::Exit(1));
        assert_eq!(g.level, LevelId::Area(0, 1));
        assert!(g.portals.iter().any(|p| p.kind == PortalKind::Exit(0) && (p.x - g.p.x).abs() + (p.y - g.p.y).abs() < 3.0), "arrive by the road back");
        walk_onto(&mut g, PortalKind::Exit(2));
        assert_eq!(g.level, LevelId::Area(0, 2));
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
        assert_eq!(g.level, LevelId::Area(0, 2), "out by the crypt's door");
        assert!(g.portals.iter().any(|p| p.kind == PortalKind::Entrance(0) && (p.x - g.p.x).abs() + (p.y - g.p.y).abs() < 3.0));
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
        g.debug_goto(LevelId::Area(0, 6));
        let s = g.portals.iter().find(|p| p.kind == PortalKind::Entrance(SANCTUM)).map(|p| (p.x, p.y)).unwrap();
        g.p.x = s.0;
        g.p.y = s.1;
        g.portal_cd = 0.0;
        g.update(&Input::default());
        assert_eq!(g.level, LevelId::Area(0, 6), "ash barrier holds");
        g.debug_goto(LevelId::Overworld);
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
        g.debug_goto(LevelId::Area(0, 6));
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

    fn read_through(g: &mut Game) {
        for _ in 0..12 {
            if g.dialog.is_none() {
                break;
            }
            g.update(&Input { confirm: true, ..Input::default() });
            g.update(&Input::default());
        }
    }

    fn walk_onto(g: &mut Game, kind: PortalKind) {
        // Doors and the ways on live out in the areas now: go to the one that has it.
        if !g.portals.iter().any(|p| p.kind == kind) {
            let there = match kind {
                PortalKind::Entrance(k) => crate::areas::dungeon_home(k),
                PortalKind::Pass(to) => crate::areas::pass_home(g.level.act(), to),
                _ => g.level,
            };
            g.debug_goto(there);
        }
        let s = g.portals.iter().find(|p| p.kind == kind).map(|p| (p.x, p.y)).expect("portal");
        (g.p.x, g.p.y) = s;
        g.portal_cd = 0.0;
        g.update(&Input::default());
    }

    #[test]
    fn act_two_opens_after_the_ash_king_and_can_be_finished() {
        let mut g = Game::new(5, crate::gfx::SH_WIDE);
        // The pass north (at the far end of the Ashen Steppe) is shut while the Ash King lives.
        g.debug_goto(LevelId::Area(0, 6));
        walk_onto(&mut g, PortalKind::Pass(1));
        assert_eq!(g.level, LevelId::Area(0, 6), "pass closed");
        g.quest.stage = 3;
        g.portal_cd = 0.0;
        g.update(&Input::default());
        assert_eq!(g.level, LevelId::Frostmarch, "through the pass");
        assert!(g.portals.iter().any(|p| p.kind == PortalKind::Pass(0) && (p.x - g.p.x).abs() + (p.y - g.p.y).abs() < 3.0), "arrive by the pass");
        assert!(g.level_name.contains("FROSTMARCH"));
        // Captain Brenna gives the quest.
        assert!(g.debug_talk(Role::Captain));
        read_through(&mut g);
        assert_eq!(g.quest.stage2, 1);
        // The glacier is sealed.
        walk_onto(&mut g, PortalKind::Entrance(GLACIER));
        assert!(g.level.act() == LevelId::Frostmarch.act() && !matches!(g.level, LevelId::Dungeon(..)), "black ice holds");
        // The three heralds and their runes.
        let power = g.p.power;
        for (i, k) in (4..7).enumerate() {
            g.debug_goto(LevelId::Dungeon(k, DUNGEONS[k].floors - 1));
            assert!(g.debug_kill_boss());
            g.debug_collect_all();
            assert!(g.quest.runes[i], "rune {i}");
        }
        assert!(g.p.power > power * 1.3, "runes make you stronger");
        // Town portal back to Kaldholm (not Hollowmere).
        walk_onto(&mut g, PortalKind::TownPortal);
        assert!(g.level.act() == LevelId::Frostmarch.act() && !matches!(g.level, LevelId::Dungeon(..)));
        assert!(g.in_safe(g.p.x, g.p.y));
        assert!(g.debug_talk(Role::Captain));
        read_through(&mut g);
        assert_eq!(g.quest.stage2, 2);
        walk_onto(&mut g, PortalKind::Entrance(GLACIER));
        assert_eq!(g.level, LevelId::Dungeon(GLACIER, 0));
        // Up the stairs comes out on the Frostmarch, by the glacier gate.
        walk_onto(&mut g, PortalKind::Up);
        assert!(g.level.act() == LevelId::Frostmarch.act() && !matches!(g.level, LevelId::Dungeon(..)));
        g.debug_goto(LevelId::Dungeon(GLACIER, DUNGEONS[GLACIER].floors - 1));
        assert!(g.mobs.iter().any(|m| m.kind == crate::mobs::Kind::WhiteDragon));
        assert!(g.debug_kill_boss());
        assert!(matches!(g.state, State::Victory(_)));
        assert_eq!(g.quest.stage2, 3);
        assert!(g.quest.mists_open(), "the road east opens");
    }

    #[test]
    fn act_three_opens_after_the_rime_wyrm_and_can_be_finished() {
        let mut g = Game::new(5, crate::gfx::SH_WIDE);
        g.quest.stage = 3;
        g.quest.seals = [true; 3];
        g.quest.stage2 = 2;
        g.quest.runes = [true; 3];
        g.debug_goto(LevelId::Frostmarch);
        // The mists stay closed until the Rime Wyrm falls.
        walk_onto(&mut g, PortalKind::Pass(2));
        assert!(g.level.act() == LevelId::Frostmarch.act() && !matches!(g.level, LevelId::Dungeon(..)), "the mist pass is closed");
        g.quest.stage2 = 3;
        walk_onto(&mut g, PortalKind::Pass(2));
        assert!(g.level.act() == LevelId::Mistwood.act() && !matches!(g.level, LevelId::Dungeon(..)));
        assert_eq!(g.level.act(), 2);
        assert!(g.in_safe(g.p.x, g.p.y) || g.npcs.iter().any(|n| n.role == Role::Hunter));
        // The hunter sends you after the three heralds.
        assert!(g.quest.hunter_has_news());
        assert!(g.debug_talk(Role::Hunter));
        read_through(&mut g);
        assert_eq!(g.quest.stage3, 1);
        // The castle stays shut without the sigils.
        walk_onto(&mut g, PortalKind::Entrance(CASTLE));
        assert!(g.level.act() == LevelId::Mistwood.act() && !matches!(g.level, LevelId::Dungeon(..)), "the castle gate is sealed");
        for (k, kind) in [(8, crate::mobs::Kind::Ossric), (9, crate::mobs::Kind::Grimhilde), (10, crate::mobs::Kind::Malgrave)] {
            g.debug_goto(LevelId::Dungeon(k, DUNGEONS[k].floors - 1));
            assert!(g.mobs.iter().any(|m| m.kind == kind), "{kind:?} waits below");
            assert!(g.debug_kill_boss());
            g.debug_collect_all();
        }
        assert_eq!(g.quest.sigil_count(), 3, "three sigils");
        g.debug_goto(LevelId::Mistwood);
        assert!(g.quest.hunter_has_news());
        assert!(g.debug_talk(Role::Hunter));
        read_through(&mut g);
        assert_eq!(g.quest.stage3, 2);
        walk_onto(&mut g, PortalKind::Entrance(CASTLE));
        assert_eq!(g.level, LevelId::Dungeon(CASTLE, 0));
        g.debug_goto(LevelId::Dungeon(CASTLE, DUNGEONS[CASTLE].floors - 1));
        assert!(g.mobs.iter().any(|m| m.kind == crate::mobs::Kind::Vardak));
        assert!(g.debug_kill_boss());
        assert!(matches!(g.state, State::Victory(_)));
        assert_eq!(g.quest.stage3, 3);
        assert!(g.quest.gears_open(), "the gear gate turns");
        // Saved mid-Act 3, loaded in the Mistwood.
        g.debug_goto(LevelId::Mistwood);
        let text = crate::save::to_text(&g);
        let mut h = Game::new(5, crate::gfx::SH_WIDE);
        crate::save::apply(&mut h, &text);
        assert_eq!(h.level, LevelId::Mistwood);
        assert_eq!((h.quest.stage3, h.quest.sigil_count()), (3, 3));
    }

    #[test]
    fn act_four_opens_after_vardak_and_can_be_finished() {
        let mut g = Game::new(5, crate::gfx::SH_WIDE);
        g.act_start(2);
        assert!(g.level.act() == LevelId::Mistwood.act() && !matches!(g.level, LevelId::Dungeon(..)));
        // The gear gate is still while the Count lives.
        walk_onto(&mut g, PortalKind::Pass(3));
        assert!(g.level.act() == LevelId::Mistwood.act() && !matches!(g.level, LevelId::Dungeon(..)), "the gears are still");
        g.quest.stage3 = 3;
        walk_onto(&mut g, PortalKind::Pass(3));
        assert!(g.level.act() == LevelId::Mechanus.act() && !matches!(g.level, LevelId::Dungeon(..)));
        assert_eq!(g.level.act(), 3);
        // And back again, arriving by the gear gate.
        walk_onto(&mut g, PortalKind::Pass(2));
        assert!(g.level.act() == LevelId::Mistwood.act() && !matches!(g.level, LevelId::Dungeon(..)));
        let gate = g.portals.iter().find(|p| p.kind == PortalKind::Pass(3)).map(|p| (p.x, p.y)).unwrap();
        assert!((g.p.x - gate.0).abs() + (g.p.y - gate.1).abs() < 3.0, "arrived at the gear gate");
        g.debug_goto(LevelId::Mechanus);
        assert!(g.quest.tally_has_news());
        assert!(g.debug_talk(Role::Tally));
        read_through(&mut g);
        assert_eq!(g.quest.stage4, 1);
        walk_onto(&mut g, PortalKind::Entrance(HEART));
        assert!(g.level.act() == LevelId::Mechanus.act() && !matches!(g.level, LevelId::Dungeon(..)), "the heart is locked");
        for (i, (k, kind)) in [(12, crate::mobs::Kind::Forgemother), (13, crate::mobs::Kind::Cantor), (14, crate::mobs::Kind::Archivist)].into_iter().enumerate() {
            g.debug_goto(LevelId::Dungeon(k, DUNGEONS[k].floors - 1));
            assert!(g.mobs.iter().any(|m| m.kind == kind), "{kind:?} waits below");
            assert!(g.debug_kill_boss());
            g.debug_collect_all();
            assert!(g.quest.keys[i], "key {i}");
        }
        g.debug_goto(LevelId::Mechanus);
        assert!(g.quest.tally_has_news());
        assert!(g.debug_talk(Role::Tally));
        read_through(&mut g);
        assert_eq!(g.quest.stage4, 2);
        walk_onto(&mut g, PortalKind::Entrance(HEART));
        assert_eq!(g.level, LevelId::Dungeon(HEART, 0));
        g.debug_goto(LevelId::Dungeon(HEART, DUNGEONS[HEART].floors - 1));
        assert!(g.debug_kill_boss());
        assert!(matches!(g.state, State::Victory(_)));
        assert_eq!(g.quest.stage4, 3);
        assert!(!g.quest.tally_has_news(), "nightmare waits for later acts");
        assert!(g.quest.deep_open(), "the diving bell is free");
        // Saved in Mechanus, loaded in Mechanus.
        g.debug_goto(LevelId::Mechanus);
        let text = crate::save::to_text(&g);
        let mut h = Game::new(5, crate::gfx::SH_WIDE);
        crate::save::apply(&mut h, &text);
        assert_eq!(h.level, LevelId::Mechanus);
        assert_eq!((h.quest.stage4, h.quest.key_count()), (3, 3));
    }

    #[test]
    fn the_clockmaker_rewinds_you_then_takes_the_engine() {
        let mut g = Game::new(5, crate::gfx::SH_WIDE);
        g.debug_goto(LevelId::Dungeon(HEART, DUNGEONS[HEART].floors - 1));
        let i = g.mobs.iter().position(|m| m.kind == crate::mobs::Kind::Clockmaker).expect("the clockmaker");
        let (bx, by) = (g.mobs[i].x, g.mobs[i].y);
        // Find open ground near him for a walk.
        let start = [(3.0, 0.0), (-3.0, 0.0), (0.0, 3.0), (0.0, -3.0)].iter().map(|(dx, dy)| (bx + dx, by + dy)).find(|&(x, y)| !g.d.blocked(x, y, 0.4)).expect("room");
        (g.p.x, g.p.y) = start;
        g.mobs[i].state = MobState::Chase;
        g.mobs[i].special2 = 5.0;
        g.update(&Input::default());
        // Three seconds ago you stood here; now you've stepped away.
        let back = (g.p.x, g.p.y);
        g.trail = std::iter::repeat(back).take(180).collect();
        let away = [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)].iter().map(|(dx, dy)| (back.0 + dx, back.1 + dy)).find(|&(x, y)| !g.d.blocked(x, y, 0.4)).unwrap();
        (g.p.x, g.p.y) = away;
        // Now he rewinds you.
        g.mobs[i].special2 = 0.0;
        g.p.hp = g.p.max_hp;
        g.update(&Input::default());
        assert!((g.p.x - back.0).abs() + (g.p.y - back.1).abs() < 0.5, "rewound to {back:?}, at {:?}", (g.p.x, g.p.y));
        // Below half life he climbs into the engine.
        g.mobs[i].hp = g.mobs[i].max_hp * 0.4;
        g.p.hp = g.p.max_hp;
        g.update(&Input::default());
        assert_eq!(g.mobs[i].form, 1, "the great engine");
        assert!(g.mobs[i].invuln > 0.0);
    }

    #[test]
    fn ordinal_squads_march_in_step_until_the_marshal_falls() {
        let mut g = Game::new(5, crate::gfx::SH_WIDE);
        g.debug_goto(LevelId::Mechanus);
        let squad_of = |g: &Game, m: usize| -> Vec<usize> {
            let (mx, my) = (g.mobs[m].x, g.mobs[m].y);
            (0..g.mobs.len())
                .filter(|&i| matches!(g.mobs[i].kind, crate::mobs::Kind::Ordinal | crate::mobs::Kind::Prism) && (g.mobs[i].x - mx).powi(2) + (g.mobs[i].y - my).powi(2) < 16.0)
                .collect()
        };
        let m = (0..g.mobs.len()).filter(|&i| g.mobs[i].kind == crate::mobs::Kind::Marshal).max_by_key(|&i| squad_of(&g, i).len()).expect("a marshal");
        let squad = squad_of(&g, m);
        assert!(squad.len() >= 3, "a marshal leads a squad");
        assert!(squad.iter().any(|&i| g.mobs[i].kind == crate::mobs::Kind::Prism));
        g.update(&Input::default());
        assert!(squad.iter().all(|&i| g.mobs[i].drilled), "in step");
        g.mobs[m].hp = 0.0;
        g.kill(m);
        assert!(squad.iter().all(|&i| g.mobs[i].stun > 2.0), "disorder");
    }

    #[test]
    fn ordinal_ranks_hold_a_shield_wall_and_break_with_their_marshal() {
        use crate::mobs::{Kind, Mob, MobState};
        let mut g = Game::new(5, crate::gfx::SH_WIDE);
        g.debug_goto(LevelId::Mechanus);
        // A fresh squad in open ground, you to the east.
        g.mobs.clear();
        let (cx, cy) = g.clocks.first().map(|c| (c.x, c.y)).unwrap_or(g.start);
        let (ox, oy) = (cx + 3.0, cy);
        let mut marshal = Mob::new(Kind::Marshal, ox, oy, 7.0, &mut g.rng);
        marshal.state = MobState::Chase;
        g.mobs.push(marshal);
        for (dx, dy) in [(-1.2f32, 0.8f32), (1.2, 0.8), (0.0, 1.8)] {
            let mut m = Mob::new(Kind::Ordinal, ox - 3.0, oy - 3.0, 7.0, &mut g.rng);
            m.post = (dx, dy);
            m.state = MobState::Chase;
            g.mobs.push(m);
        }
        g.clocks.clear();
        g.laws.clear();
        (g.p.x, g.p.y) = (ox + 6.0, oy);
        g.p.base_hp = 99999.0;
        g.p.recalc();
        // Wait for a marching beat, then let them march.
        while g.tick % 60 >= 40 {
            g.update(&Input::default());
        }
        let before: Vec<f32> = (1..4).map(|i| ((g.mobs[i].x - ox).powi(2) + (g.mobs[i].y - oy).powi(2)).sqrt()).collect();
        for _ in 0..30 {
            g.mobs[0].x = ox;
            g.mobs[0].y = oy;
            g.p.hp = g.p.max_hp;
            g.update(&Input::default());
        }
        for i in 1..4 {
            let d = ((g.mobs[i].x - ox).powi(2) + (g.mobs[i].y - oy).powi(2)).sqrt();
            assert!(d < before[i - 1], "ordinal {i} closes on its post");
            assert!(g.mobs[i].x < ox + 0.5, "the ranks stay behind the marshal (you are east)");
        }
        // Shield wall: in step they take less damage...
        assert!(g.mobs[1].drilled);
        let hp = g.mobs[1].hp;
        g.hit_mob(1, 10.0, 0.0, 0.0, None, false);
        let walled = hp - g.mobs[1].hp;
        let k = crate::skills::class_damage(g.p.skills.class);
        assert!((walled - 7.0 * k).abs() < 0.01, "shield wall: {walled}");
        // ...and with the marshal dead, they're broken and take more.
        g.mobs[0].hp = 0.0;
        g.kill(0);
        g.update(&Input::default());
        assert!(!g.mobs[1].drilled && g.mobs[1].broken > 0.0);
        let hp = g.mobs[1].hp;
        g.hit_mob(1, 10.0, 0.0, 0.0, None, false);
        assert!((hp - g.mobs[1].hp - 13.0 * k).abs() < 0.01);
    }

    #[test]
    fn clockwork_crows_flock_in_mechanus() {
        let mut g = Game::new(5, crate::gfx::SH_WIDE);
        g.debug_goto(LevelId::Mechanus);
        let n = g.mobs.iter().filter(|m| m.kind == crate::mobs::Kind::ClockCrow).count();
        assert!(n >= 6, "a flock or more of crows: {n}");
    }

    #[test]
    fn act_five_opens_after_the_clockmaker_and_can_be_finished() {
        let mut g = Game::new(5, crate::gfx::SH_WIDE);
        g.act_start(3);
        assert!(g.level.act() == LevelId::Mechanus.act() && !matches!(g.level, LevelId::Dungeon(..)));
        // The bell is locked until the Clockmaker stops.
        walk_onto(&mut g, PortalKind::Pass(4));
        assert!(g.level.act() == LevelId::Mechanus.act() && !matches!(g.level, LevelId::Dungeon(..)), "the bell's chains are locked");
        g.quest.stage4 = 3;
        g.quest.keys = [true; 3];
        g.portal_cd = 0.0;
        walk_onto(&mut g, PortalKind::Pass(4));
        assert!(g.level.act() == LevelId::Deep.act() && !matches!(g.level, LevelId::Dungeon(..)));
        assert!(g.quest.ysolde_has_news());
        assert!(g.debug_talk(Role::Ysolde));
        read_through(&mut g);
        assert_eq!(g.quest.stage5, 1);
        // The sanctum is sealed until the pearls are set.
        g.portal_cd = 0.0;
        walk_onto(&mut g, PortalKind::Entrance(ABYSS));
        assert!(g.level.act() == LevelId::Deep.act() && !matches!(g.level, LevelId::Dungeon(..)), "the sanctum is sealed");
        for (i, k) in (16..19).enumerate() {
            g.debug_goto(LevelId::Dungeon(k, DUNGEONS[k].floors - 1));
            assert!(g.debug_kill_boss());
            g.debug_collect_all();
            assert!(g.quest.pearls[i], "pearl {i}");
        }
        g.debug_goto(LevelId::Deep);
        assert!(g.quest.ysolde_has_news());
        assert!(g.debug_talk(Role::Ysolde));
        read_through(&mut g);
        assert_eq!(g.quest.stage5, 2);
        g.portal_cd = 0.0;
        walk_onto(&mut g, PortalKind::Entrance(ABYSS));
        assert_eq!(g.level, LevelId::Dungeon(ABYSS, 0));
        g.debug_goto(LevelId::Dungeon(ABYSS, DUNGEONS[ABYSS].floors - 1));
        assert!(g.debug_kill_boss());
        assert!(matches!(g.state, State::Victory(_)));
        assert_eq!(g.quest.stage5, 3);
        assert!(!g.quest.ysolde_has_news(), "nightmare waits for act 6");
        assert!(g.quest.skies_open(), "the stair of light holds");
        // Saved in the deep, loaded in the deep.
        g.debug_goto(LevelId::Deep);
        let text = crate::save::to_text(&g);
        let mut h = Game::new(5, crate::gfx::SH_WIDE);
        crate::save::apply(&mut h, &text);
        assert_eq!(h.level, LevelId::Deep);
        assert_eq!((h.quest.stage5, h.quest.pearl_count()), (3, 3));
    }

    #[test]
    fn act_six_opens_after_the_leviathan_and_can_be_finished() {
        let mut g = Game::new(5, crate::gfx::SH_WIDE);
        g.act_start(4);
        assert!(g.level.act() == LevelId::Deep.act() && !matches!(g.level, LevelId::Dungeon(..)));
        walk_onto(&mut g, PortalKind::Pass(5));
        assert!(g.level.act() == LevelId::Deep.act() && !matches!(g.level, LevelId::Dungeon(..)), "the stair won't hold you yet");
        g.quest.stage5 = 3;
        g.quest.pearls = [true; 3];
        g.portal_cd = 0.0;
        walk_onto(&mut g, PortalKind::Pass(5));
        assert!(g.level.act() == LevelId::Heavens.act() && !matches!(g.level, LevelId::Dungeon(..)));
        assert!(g.quest.seraphine_has_news());
        assert!(g.debug_talk(Role::Seraphine));
        read_through(&mut g);
        assert_eq!(g.quest.stage6, 1);
        g.portal_cd = 0.0;
        walk_onto(&mut g, PortalKind::Entrance(ZENITH));
        assert!(g.level.act() == LevelId::Heavens.act() && !matches!(g.level, LevelId::Dungeon(..)), "the true sanctum is sealed");
        for (i, k) in (20..23).enumerate() {
            g.debug_goto(LevelId::Dungeon(k, DUNGEONS[k].floors - 1));
            assert!(g.debug_kill_boss());
            g.debug_collect_all();
            assert!(g.quest.shards[i], "shard {i}");
        }
        g.debug_goto(LevelId::Heavens);
        assert!(g.debug_talk(Role::Seraphine));
        read_through(&mut g);
        assert_eq!(g.quest.stage6, 2);
        g.portal_cd = 0.0;
        walk_onto(&mut g, PortalKind::Entrance(ZENITH));
        assert_eq!(g.level, LevelId::Dungeon(ZENITH, 0));
        g.debug_goto(LevelId::Dungeon(ZENITH, DUNGEONS[ZENITH].floors - 1));
        assert!(g.debug_kill_boss());
        assert!(matches!(g.state, State::Victory(_)));
        assert_eq!(g.quest.stage6, 3);
        assert!(g.quest.seraphine_has_news(), "seraphine offers nightmare");
        g.debug_goto(LevelId::Heavens);
        let text = crate::save::to_text(&g);
        let mut h = Game::new(5, crate::gfx::SH_WIDE);
        crate::save::apply(&mut h, &text);
        assert_eq!(h.level, LevelId::Heavens);
        assert_eq!((h.quest.stage6, h.quest.shard_count()), (3, 3));
    }

    #[test]
    fn the_heralds_of_the_sky_fight_back() {
        use crate::mobs::Kind;
        let mut g = Game::new(5, crate::gfx::SH_WIDE);
        g.act_start(5);
        for k in [Kind::Vael, Kind::Tempest, Kind::OphanPrime, Kind::Solanthos] {
            g.debug_goto(LevelId::Heavens);
            g.safe = None;
            g.mobs.clear();
            g.hazards.clear();
            g.shots.clear();
            g.p.hp = 1e9;
            g.p.max_hp = 1e9;
            let (px, py) = (g.p.x, g.p.y);
            let mut m = crate::mobs::Mob::new(k, px + 4.0, py, 12.0, &mut g.rng);
            m.state = crate::mobs::MobState::Chase;
            g.mobs.push(m);
            let mut acted = false;
            for _ in 0..600 {
                g.update(&Input::default());
                if !g.hazards.is_empty() || !g.shots.is_empty() || g.mobs.len() > 1 || g.wind_gust > 0.0 {
                    acted = true;
                    break;
                }
            }
            assert!(acted, "{k:?} never used a special");
        }
    }

    #[test]
    fn the_heralds_of_the_deep_fight_back() {
        use crate::mobs::Kind;
        let mut g = Game::new(5, crate::gfx::SH_WIDE);
        g.act_start(4);
        for k in [Kind::Dregmoor, Kind::Nacre, Kind::Angler, Kind::Leviathan] {
            g.debug_goto(LevelId::Deep);
            g.mobs.clear();
            g.hazards.clear();
            g.shots.clear();
            g.p.hp = 1e9;
            g.p.max_hp = 1e9;
            let (px, py) = (g.p.x, g.p.y);
            g.safe = None;
            let mut m = crate::mobs::Mob::new(k, px + 4.0, py, 10.0, &mut g.rng);
            m.state = crate::mobs::MobState::Chase;
            if k == Kind::Leviathan {
                m.hp = m.max_hp * 0.2;
            }
            g.mobs.push(m);
            let mut acted = false;
            for _ in 0..600 {
                g.update(&Input::default());
                if !g.hazards.is_empty() || !g.shots.is_empty() || g.mobs.len() > 1 || g.dark_t > 0.0 || g.pull.2 > 0.0 {
                    acted = true;
                    break;
                }
            }
            assert!(acted, "{k:?} never used a special");
        }
    }

    #[test]
    fn boiler_brutes_burst_when_they_fall() {
        let mut g = Game::new(5, crate::gfx::SH_WIDE);
        g.debug_goto(LevelId::Mechanus);
        let i = match g.mobs.iter().position(|m| m.kind == crate::mobs::Kind::BoilerBrute) {
            Some(i) => i,
            None => {
                let m = crate::mobs::Mob::new(crate::mobs::Kind::BoilerBrute, g.p.x + 3.0, g.p.y, 7.5, &mut g.rng);
                g.mobs.push(m);
                g.mobs.len() - 1
            }
        };
        let before = g.hazards.len();
        g.mobs[i].hp = 0.0;
        g.kill(i);
        assert!(g.hazards.len() > before);
    }

    #[test]
    fn vardak_changes_form() {
        let mut g = Game::new(5, crate::gfx::SH_WIDE);
        g.debug_goto(LevelId::Dungeon(CASTLE, DUNGEONS[CASTLE].floors - 1));
        let i = g.mobs.iter().position(|m| m.kind == crate::mobs::Kind::Vardak).expect("the count");
        g.p.x = g.mobs[i].x + 3.0;
        g.p.y = g.mobs[i].y;
        g.mobs[i].state = MobState::Chase;
        let max = g.mobs[i].max_hp;
        g.mobs[i].hp = max * 0.6;
        g.mobs[i].special2 = 0.0;
        let mut misted = false;
        for _ in 0..600 {
            g.p.hp = g.p.max_hp;
            g.update(&Input::default());
            if g.mobs.get(i).map_or(false, |m| m.invuln > 0.0) {
                misted = true;
                break;
            }
        }
        assert!(misted, "mist form below two thirds");
        g.mobs[i].invuln = 0.0;
        g.mobs[i].hp = max * 0.2;
        let mut bat = false;
        for _ in 0..600 {
            g.p.hp = g.p.max_hp;
            g.update(&Input::default());
            if g.mobs.get(i).map_or(false, |m| m.form == 1) {
                bat = true;
                break;
            }
        }
        assert!(bat, "the giant bat at the end");
    }

    #[test]
    fn act2_start_makes_a_ready_character_in_kaldholm() {
        let mut g = Game::new(5, crate::gfx::SH_WIDE);
        g.act_start(1);
        assert_eq!(g.level, LevelId::Frostmarch);
        assert!(g.in_safe(g.p.x, g.p.y));
        assert_eq!(g.p.clvl, 18);
        assert_eq!(g.p.skills.points, 21);
        assert!(g.quest.north_open() && g.quest.seal_count() == 3);
        assert!(g.p.gear.worn.iter().all(|w| w.is_some()), "a full set of gear");
        assert!(g.p.max_hp > 250.0 && g.p.power > 4.0);
        // It saves and loads back into Kaldholm.
        let text = crate::save::to_text(&g);
        let mut h = Game::new(5, crate::gfx::SH_WIDE);
        crate::save::apply(&mut h, &text);
        assert_eq!(h.level, LevelId::Frostmarch);
        assert_eq!(h.p.clvl, 18);
    }

    fn vampire_game() -> Game {
        let mut g = quiet_game();
        g.set_class(crate::skills::Class::Vampire);
        g.p.mana = 500.0;
        g.p.max_mana = 500.0;
        g
    }

    #[test]
    fn the_class_screen_makes_a_vampire() {
        let mut g = Game::new(5, crate::gfx::SH_WIDE);
        g.choose = Some(0);
        g.update(&Input { move_x: 1.0, ..Input::default() });
        assert_eq!(g.choose, Some(1));
        g.update(&Input::default());
        g.update(&Input { confirm: true, ..Input::default() });
        assert!(g.choose.is_none());
        assert_eq!(g.p.skills.class, crate::skills::Class::Vampire);
        assert_eq!(g.p.skills.rank(crate::skills::Skill::BloodLance), 1);
        assert!(g.p.gear.worn[0].is_none(), "no staff for her");
        // Saved and loaded as a vampire.
        let text = crate::save::to_text(&g);
        let mut h = Game::new(5, crate::gfx::SH_WIDE);
        crate::save::apply(&mut h, &text);
        assert_eq!(h.p.skills.class, crate::skills::Class::Vampire);
    }

    #[test]
    fn the_vampire_drains_life_but_not_from_the_bloodless() {
        use crate::skills::Skill;
        let mut g = vampire_game();
        for kind in [Kind::Zombie, Kind::Skeleton] {
            let mut m = Mob::new(kind, g.p.x + 2.0, g.p.y, 1.0, &mut g.rng);
            m.max_hp = 5000.0;
            m.hp = 5000.0;
            g.mobs.push(m);
        }
        g.p.hp = 10.0;
        g.hit_mob(0, 200.0, 0.0, 0.0, None, false);
        assert!(g.p.hp > 10.0, "drank from the zombie");
        let hp = g.p.hp;
        g.hit_mob(1, 200.0, 0.0, 0.0, None, false);
        assert_eq!(g.p.hp, hp, "skeletons have no blood");
        assert!(g.mobs[1].hp > g.mobs[0].hp, "and resist blood damage");
        // Blood Lance flies and drains on impact; no burning.
        g.p.hp = 10.0;
        let (zx, zy) = (g.mobs[0].x, g.mobs[0].y);
        assert!(g.cast_skill(Skill::BloodLance, zx, zy));
        for _ in 0..60 {
            g.update(&Input::default());
        }
        assert!(g.p.hp > 10.0);
        assert!(g.mobs[0].burn <= 0.0, "blood doesn't burn");
    }

    #[test]
    fn rake_mist_step_and_bats_work() {
        use crate::skills::Skill;
        let mut g = vampire_game();
        g.p.clvl = 18;
        for s in [Skill::Rake, Skill::MistStep, Skill::BatSwarm] {
            g.p.skills.rank[s as usize] = 3;
        }
        let mut m = Mob::new(Kind::Zombie, g.p.x + 1.0, g.p.y, 1.0, &mut g.rng);
        m.max_hp = 1000.0;
        m.hp = 1000.0;
        g.mobs.push(m);
        let (zx, zy) = (g.mobs[0].x, g.mobs[0].y);
        g.p.cast_cd = 0.0;
        g.cast_skill(Skill::Rake, zx, zy);
        assert!(g.mobs[0].hp < 1000.0, "rake hits what's in front");
        // Mist Step slips through the zombie and can't be hurt for a moment.
        let (x0, y0) = (g.p.x, g.p.y);
        g.p.cast_cd = 0.0;
        g.cast_skill(Skill::MistStep, x0 + 6.0, y0);
        assert!((g.p.x - x0).abs() + (g.p.y - y0).abs() > 2.0, "moved");
        let hp = g.p.hp;
        g.hurt_player(50.0);
        assert_eq!(g.p.hp, hp, "mist can't be touched");
        assert!(g.p.skills.cooldown[Skill::MistStep as usize] > 0.0);
        // Bats find the zombie.
        let hp = g.mobs[0].hp;
        g.p.cast_cd = 0.0;
        g.cast_skill(Skill::BatSwarm, zx, zy);
        assert!(!g.bats.is_empty());
        for _ in 0..120 {
            g.update(&Input::default());
        }
        assert!(g.mobs[0].hp < hp, "bats bit");
    }

    #[test]
    fn mesmerized_and_raised_foes_fight_for_her() {
        use crate::skills::Skill;
        let mut g = vampire_game();
        g.p.clvl = 18;
        for s in [Skill::Mesmerize, Skill::Thrall] {
            g.p.skills.rank[s as usize] = 3;
        }
        let a = Mob::new(Kind::Zombie, g.p.x + 2.0, g.p.y, 1.0, &mut g.rng);
        let mut b = Mob::new(Kind::Zombie, g.p.x + 3.5, g.p.y, 1.0, &mut g.rng);
        b.max_hp = 300.0;
        b.hp = 300.0;
        g.mobs.push(a);
        g.mobs.push(b);
        let (ax, ay) = (g.mobs[0].x, g.mobs[0].y);
        g.p.cast_cd = 0.0;
        g.cast_skill(Skill::Mesmerize, ax, ay);
        assert!(g.mobs[0].charm > 0.0);
        let hp = g.p.hp;
        for _ in 0..240 {
            g.update(&Input::default());
        }
        assert!(g.mobs[1].hp < 300.0, "the mesmerized zombie attacks the other");
        // My spells don't hurt my ally.
        let ally = g.mobs[0].hp;
        g.hit_mob(0, 50.0, 0.0, 0.0, None, false);
        assert_eq!(g.mobs[0].hp, ally);
        let _ = hp;
        // A corpse rises as a thrall, and crumbles when its time is up.
        g.mobs[1].hp = 0.0;
        g.kill(1);
        g.p.cast_cd = 0.0;
        g.p.mana = 500.0;
        g.cast_skill(Skill::Thrall, g.p.x, g.p.y);
        assert!(g.mobs[1].alive() && g.mobs[1].thrall);
        g.mobs[1].charm = 0.05;
        for _ in 0..10 {
            g.update(&Input::default());
        }
        assert!(!g.mobs[1].alive(), "crumbled");
    }

    #[test]
    fn the_vampire_hungers_for_blood_not_bread() {
        let mut g = vampire_game();
        g.p.food = 20.0;
        // Bread on the floor stays there.
        g.pickups.push(Pickup { x: g.p.x, y: g.p.y, kind: Drop::Food(1), t: 1.0 });
        g.collect_pickups();
        assert_eq!(g.p.food, 20.0);
        assert_eq!(g.pickups.len(), 1);
        g.pickups.clear();
        // Out of mana she bites: a zombie feeds her, a skeleton doesn't.
        g.p.mana = 0.0;
        for (k, kind) in [Kind::Zombie, Kind::Skeleton].iter().enumerate() {
            let mut m = Mob::new(*kind, g.p.x + 1.0, g.p.y + k as f32 * 6.0, 1.0, &mut g.rng);
            m.max_hp = 500.0;
            m.hp = 500.0;
            g.mobs.push(m);
        }
        let (zx, zy) = (g.mobs[0].x, g.mobs[0].y);
        g.p.cast_cd = 0.0;
        g.cast_skill(crate::skills::Skill::BloodLance, zx, zy);
        assert!(g.mobs[0].hp < 500.0, "bitten");
        assert!(g.p.food > 20.0 + crate::vampire::BITE_BLOOD * 0.9, "fed: {}", g.p.food);
        let fed = g.p.food;
        g.p.x = g.mobs[1].x - 1.0;
        g.p.y = g.mobs[1].y;
        let (sx, sy) = (g.mobs[1].x, g.mobs[1].y);
        g.p.cast_cd = 0.0;
        g.cast_skill(crate::skills::Skill::BloodLance, sx, sy);
        assert!(g.mobs[1].hp < 500.0, "the skeleton is bitten too");
        assert_eq!(g.p.food, fed, "but has no blood to give");
        // Merchants' bread is refused.
        let gold = g.p.gold;
        g.buy(Ware::Bread);
        assert_eq!(g.p.gold, gold);
    }

    #[test]
    fn blood_moon_and_the_embrace() {
        use crate::skills::Skill;
        let mut g = vampire_game();
        g.p.clvl = 18;
        for s in [Skill::BloodMoon, Skill::Embrace] {
            g.p.skills.rank[s as usize] = 2;
        }
        let mut m = Mob::new(Kind::Zombie, g.p.x + 2.0, g.p.y, 1.0, &mut g.rng);
        m.max_hp = 1000.0;
        m.hp = 1000.0;
        m.dmg = (0.0, 0.0); // only the moon's healing counts here
        g.mobs.push(m);
        g.p.hp = 20.0;
        let (zx, zy) = (g.mobs[0].x, g.mobs[0].y);
        g.p.cast_cd = 0.0;
        g.cast_skill(Skill::BloodMoon, zx, zy);
        for _ in 0..120 {
            g.update(&Input::default());
        }
        assert!(g.mobs[0].hp < 1000.0 && g.p.hp > 20.0, "bleeds them, feeds you");
        g.p.cast_cd = 0.0;
        g.cast_skill(Skill::Embrace, g.p.x, g.p.y);
        assert!(g.p.embrace_t > 0.0);
        let mana = g.p.mana;
        g.p.cast_cd = 0.0;
        g.cast_skill(Skill::BloodLance, zx, zy);
        assert_eq!(g.p.mana, mana, "skills are free as a bat");
    }

    fn inventor_game() -> Game {
        let mut g = quiet_game();
        g.set_class(crate::skills::Class::Inventor);
        g.p.clvl = 18;
        for s in crate::skills::INVENTOR {
            g.p.skills.rank[s as usize] = 3;
        }
        g
    }

    #[test]
    fn the_inventor_runs_on_heat() {
        use crate::skills::Skill;
        let mut g = inventor_game();
        assert_eq!(g.heat(), 0.0, "starts cold");
        let mut m = Mob::new(Kind::Zombie, g.p.x + 4.0, g.p.y, 1.0, &mut g.rng);
        m.max_hp = 9000.0;
        m.hp = 9000.0;
        m.dmg = (0.0, 0.0);
        g.mobs.push(m);
        let (zx, zy) = (g.mobs[0].x, g.mobs[0].y);
        g.p.cast_cd = 0.0;
        g.cast_skill(Skill::RayPistol, zx, zy);
        assert!(g.heat() > 0.0, "firing heats her up");
        for _ in 0..30 {
            g.update(&Input::default());
        }
        assert!(g.mobs[0].hp < 9000.0, "the bolt hit");
        // Fire until she overheats: weapons lock, only the weak shot works.
        for _ in 0..200 {
            g.p.cast_cd = 0.0;
            g.cast_skill(Skill::ClockBomb, zx, zy);
            if g.p.overheat > 0.0 {
                break;
            }
        }
        assert!(g.p.overheat > 0.0, "overheated");
        let bombs = g.bombs.len();
        g.p.cast_cd = 0.0;
        g.cast_skill(Skill::ClockBomb, zx, zy);
        assert_eq!(g.bombs.len(), bombs, "locked: no bomb");
        // Venting dumps the heat (and pushes foes back).
        g.update(&Input { potion_mp: true, ..Input::default() });
        assert_eq!(g.heat(), 0.0);
        assert!(g.p.overheat <= 0.0 && g.p.vent_cd > 0.0);
        // Bombs burst after their fuse.
        let hp = g.mobs[0].hp;
        for _ in 0..120 {
            g.update(&Input::default());
        }
        assert!(g.mobs[0].hp < hp, "boom");
        // Cooling: heat drains on its own.
        g.p.mana = 10.0;
        for _ in 0..60 {
            g.update(&Input::default());
        }
        assert!(g.p.mana > 15.0);
    }

    fn valkyrie_game() -> Game {
        let mut g = quiet_game();
        g.set_class(crate::skills::Class::Valkyrie);
        g.p.clvl = 18;
        for s in crate::skills::VALKYRIE {
            g.p.skills.rank[s as usize] = 3;
        }
        g
    }

    /// A tough, harmless, rooted zombie at (dx, dy) from the player.
    fn dummy(g: &mut Game, dx: f32, dy: f32) -> usize {
        let mut m = Mob::new(Kind::Zombie, g.p.x + dx, g.p.y + dy, 1.0, &mut g.rng);
        m.max_hp = 9000.0;
        m.hp = 9000.0;
        m.dmg = (0.0, 0.0);
        m.speed = 0.0;
        g.mobs.push(m);
        g.mobs.len() - 1
    }

    #[test]
    fn the_valkyrie_fights_for_valor_and_walks_into_reach() {
        use crate::skills::Skill;
        let mut g = valkyrie_game();
        assert_eq!(g.p.mana, 0.0, "valor starts empty");
        let z = dummy(&mut g, 4.5, 0.0);
        let x0 = g.p.x;
        // The pad's attack button: she walks in, then thrusts.
        // (Out of the way of this floor's waypoint, which would open its travel menu.)
        g.waypoint = (-99.0, -99.0);
        for _ in 0..150 {
            g.update(&Input { cast: true, ..Input::default() });
        }
        assert!(g.p.x > x0 + 1.5, "walked into reach");
        assert!(g.mobs[z].hp < 9000.0, "the spear hit");
        assert!(g.p.mana > 0.0, "and built valor");
        // Out of combat, valor drains away.
        let v = g.p.mana;
        g.p.fight_t = 0.0;
        for _ in 0..60 {
            g.update(&Input::default());
        }
        assert!(g.p.mana < v, "valor fades out of combat");
        // Without the valor for a skill, she thrusts her spear instead (spending nothing).
        g.p.mana = 0.0;
        g.p.cast_cd = 0.0;
        let hp = g.mobs[z].hp;
        let (zx, zy) = (g.mobs[z].x, g.mobs[z].y);
        g.cast_skill(Skill::RimeSweep, zx, zy);
        assert!(g.mobs[z].hp < hp && g.p.mana >= 0.0);
        // Full valor: her runes blaze.
        g.p.mana = 0.0;
        let low = g.fire_power();
        g.p.mana = g.p.max_mana;
        assert!(g.blazing() && g.fire_power() > low * 1.1);
        // Blows she takes stoke her valor, and Northborn shrugs some off.
        g.p.mana = 0.0;
        let hp0 = g.p.hp;
        g.hurt_player(20.0);
        assert!(g.p.mana > 0.0, "pain is valor");
        assert!(hp0 - g.p.hp < 20.0, "northborn");
    }

    #[test]
    fn frost_freezes_and_frozen_foes_shatter() {
        use crate::skills::Skill;
        let mut g = valkyrie_game();
        let a = dummy(&mut g, 1.4, 0.0);
        let b = dummy(&mut g, 2.6, 0.0);
        g.mobs[a].frost = 0.95;
        let (ax, ay) = (g.mobs[a].x, g.mobs[a].y);
        g.p.cast_cd = 0.0;
        g.cast_skill(Skill::RuneSpear, ax, ay);
        assert!(g.mobs[a].frozen > 0.0, "frozen solid");
        let hp_b = g.mobs[b].hp;
        g.mobs[a].hp = 1.0;
        g.p.cast_cd = 0.0;
        g.cast_skill(Skill::RuneSpear, ax, ay);
        assert!(!g.mobs[a].alive());
        assert!(g.mobs[b].hp < hp_b, "the shards hit its friend");
    }

    #[test]
    fn the_valkyries_skills_work() {
        use crate::skills::Skill;
        let mut g = valkyrie_game();
        let cast = |g: &mut Game, s: Skill, x: f32, y: f32| {
            g.p.mana = g.p.max_mana;
            g.p.cast_cd = 0.0;
            g.p.skills.cooldown = [0.0; crate::skills::ALL.len()];
            g.cast_skill(s, x, y);
        };
        let z = dummy(&mut g, 5.0, 0.0);
        let (zx, zy) = (g.mobs[z].x, g.mobs[z].y);
        // Raven Strike marks.
        cast(&mut g, Skill::RavenStrike, zx, zy);
        for _ in 0..90 {
            g.update(&Input::default());
        }
        assert!(g.mobs[z].marked > 0.0, "marked by the raven");
        // Glacier Leap lands near and hurts.
        let hp = g.mobs[z].hp;
        let x0 = g.p.x;
        cast(&mut g, Skill::GlacierLeap, zx - 1.0, zy);
        assert!(g.p.x > x0 + 2.0, "leapt");
        assert!(g.mobs[z].hp < hp, "landing shockwave");
        // Winter's Wrath hits all around.
        let w = dummy(&mut g, -1.0, 0.0);
        let hp_w = g.mobs[w].hp;
        let (px, py) = (g.p.x, g.p.y);
        cast(&mut g, Skill::WintersWrath, px, py);
        assert!(g.mobs[w].hp < hp_w);
        // Rune Javelin goes out and comes back.
        let hp = g.mobs[z].hp;
        cast(&mut g, Skill::RuneJavelin, zx, zy);
        assert_eq!(g.javelins.len(), 1);
        for _ in 0..200 {
            g.update(&Input::default());
        }
        assert!(g.javelins.is_empty(), "back in her hand");
        assert!(g.mobs[z].hp < hp);
        // Einherjar fight for her.
        let (px, py) = (g.p.x, g.p.y);
        cast(&mut g, Skill::Einherjar, px, py);
        let n = g.mobs.iter().filter(|m| m.kind == Kind::Einherjar && m.alive() && m.charm > 0.0).count();
        assert_eq!(n, crate::valkyrie::einherjar_count(3));
        // Ride of the Valkyrie: carried forward, trampling.
        let far = dummy(&mut g, -4.0, 0.0);
        let (fx, fy) = (g.mobs[far].x, g.mobs[far].y);
        let (x0, hp) = (g.p.x, g.mobs[far].hp);
        cast(&mut g, Skill::ValkyrieRide, fx - 3.0, fy);
        assert!(g.p.charge.is_some());
        for _ in 0..60 {
            g.update(&Input::default());
        }
        assert!(g.p.charge.is_none(), "the charge ends");
        assert!(g.p.x < x0 - 2.0, "carried along");
        assert!(g.mobs[far].hp < hp, "trampled");
        // Fimbulwinter: a blizzard that hurts and freezes.
        let f = dummy(&mut g, 1.5, 0.0);
        let hp = g.mobs[f].hp;
        let (px, py) = (g.p.x, g.p.y);
        cast(&mut g, Skill::Fimbulwinter, px, py);
        for _ in 0..120 {
            g.update(&Input::default());
        }
        assert!(g.mobs[f].hp < hp, "the blizzard bites");
        assert!(g.p.fimbul_t > 0.0);
    }

    fn berserker_game() -> Game {
        let mut g = quiet_game();
        g.set_class(crate::skills::Class::Berserker);
        g.p.clvl = 18;
        for s in crate::skills::BERSERKER {
            g.p.skills.rank[s as usize] = 3;
        }
        g
    }

    #[test]
    fn the_berserker_rages_from_pain_and_kills_and_keeps_her_wolf() {
        use crate::skills::Skill;
        let mut g = berserker_game();
        assert_eq!(g.p.mana, 0.0, "rage starts empty");
        // The wolf joins her at once and stays on her side.
        g.update(&Input::default());
        let wolves = || g.mobs.iter().filter(|m| m.kind == Kind::DireWolf && m.alive() && m.charm > 0.0).count();
        assert_eq!(wolves(), 1);
        // Cleave is free, hits a wide arc, and her own hits build no rage.
        let a = dummy(&mut g, 1.2, 0.5);
        let b = dummy(&mut g, 1.2, -0.5);
        let (ax, ay) = (g.mobs[a].x, g.mobs[a].y);
        g.p.cast_cd = 0.0;
        g.cast_skill(Skill::Cleave, ax, ay - 0.5);
        assert!(g.mobs[a].hp < 9000.0 && g.mobs[b].hp < 9000.0, "the arc hit both");
        assert_eq!(g.p.mana, 0.0, "hits alone don't make rage");
        // Pain does, and the lower her life the harder she hits.
        let full = g.fire_power();
        g.hurt_player(30.0);
        assert!(g.p.mana >= 25.0, "pain is rage");
        g.p.hp = g.p.max_hp * 0.2;
        assert!(g.fire_power() > full * 1.25, "fury at low life");
        g.p.hp = g.p.max_hp;
        // A kill makes rage and (Bloodlust) heals and quickens her.
        g.p.hp = g.p.max_hp * 0.5;
        let (hp, rage) = (g.p.hp, g.p.mana);
        g.mobs[a].hp = 1.0;
        g.p.cast_cd = 0.0;
        g.cast_skill(Skill::Cleave, ax, ay);
        assert!(!g.mobs[a].alive());
        assert!(g.p.mana > rage && g.p.hp > hp && g.p.lust == 1);
        g.update(&Input::default());
        assert!(g.p.haste > 1.0);
        // Changing level: the wolf comes along (a new one beside her), never two.
        g.debug_goto(LevelId::Dungeon(0, 1));
        for _ in 0..3 {
            g.update(&Input::default());
        }
        assert_eq!(g.mobs.iter().filter(|m| m.kind == Kind::DireWolf && m.alive()).count(), 1);
    }

    #[test]
    fn the_berserkers_skills_work() {
        use crate::skills::Skill;
        let mut g = berserker_game();
        g.waypoint = (-99.0, -99.0);
        let cast = |g: &mut Game, s: Skill, x: f32, y: f32| {
            g.p.mana = g.p.max_mana;
            g.p.cast_cd = 0.0;
            g.p.skills.cooldown = [0.0; crate::skills::ALL.len()];
            g.cast_skill(s, x, y);
        };
        // Rend: bleeding and sundered.
        let z = dummy(&mut g, 1.3, 0.0);
        let (zx, zy) = (g.mobs[z].x, g.mobs[z].y);
        cast(&mut g, Skill::Rend, zx, zy);
        assert!(g.mobs[z].bleed_t > 0.0 && g.mobs[z].sunder > 0.0);
        let hp = g.mobs[z].hp;
        for _ in 0..60 {
            g.update(&Input::default());
        }
        assert!(g.mobs[z].hp < hp, "it bleeds");
        // Executioner: the wounded take far more.
        let fresh = dummy(&mut g, 1.3, 0.6);
        let low = dummy(&mut g, 1.3, -0.6);
        g.mobs[fresh].sunder = 0.0;
        g.mobs[low].hp = g.mobs[low].max_hp * 0.2;
        let (h1, h2) = (g.mobs[fresh].hp, g.mobs[low].hp);
        g.axe_hit(fresh, 100.0, 0.0, None);
        g.axe_hit(low, 100.0, 0.0, None);
        assert!(h2 - g.mobs[low].hp > (h1 - g.mobs[fresh].hp) * 1.4, "executioner");
        // War Cry: foes flee, she hits harder.
        let p0 = g.fire_power();
        let (px, py) = (g.p.x, g.p.y);
        cast(&mut g, Skill::WarCry, px, py);
        assert!(g.p.warcry_t > 0.0 && g.fire_power() > p0);
        assert!(g.mobs[z].flee > 0.0, "it flees the war cry");
        // Leap Slam: there and knocked down.
        let far = dummy(&mut g, 5.0, 0.0);
        let (fx, fy) = (g.mobs[far].x, g.mobs[far].y);
        let x0 = g.p.x;
        cast(&mut g, Skill::LeapSlam, fx - 1.0, fy);
        assert!(g.p.x > x0 + 2.0);
        assert!(g.mobs[far].stun > 0.8, "knocked down");
        // Whirlwind: several hits on everything around, moving as she spins.
        let hp = g.mobs[far].hp;
        let (px, py) = (g.p.x, g.p.y);
        cast(&mut g, Skill::Whirlwind, px + 2.0, py);
        assert!(g.p.whirl_t > 0.0);
        for _ in 0..150 {
            g.update(&Input::default());
        }
        assert!(g.p.whirl_t <= 0.0);
        assert!(hp - g.mobs[far].hp > 0.0, "shredded");
        // Hurl Axe: out and back.
        let (fx, fy) = (g.mobs[far].x, g.mobs[far].y);
        let hp = g.mobs[far].hp;
        cast(&mut g, Skill::HurlAxe, fx, fy);
        assert_eq!(g.axes.len(), 1);
        for _ in 0..200 {
            g.update(&Input::default());
        }
        assert!(g.axes.is_empty() && g.mobs[far].hp < hp);
        // Berserk: can't die, then spent.
        let (px, py) = (g.p.x, g.p.y);
        cast(&mut g, Skill::Berserk, px, py);
        g.hurt_player(g.p.max_hp * 5.0);
        assert!(g.p.hp >= 1.0 && !matches!(g.state, State::Dead(_)), "the red mist holds");
        g.p.berserk_t = 0.01;
        g.update(&Input::default());
        assert!(g.p.exhaust_t > 0.0, "spent afterward");
        // Dire Wolf: the howl scatters foes near the wolf.
        let w = g.mobs.iter().position(|m| m.kind == Kind::DireWolf && m.alive()).unwrap();
        let near = dummy(&mut g, 0.0, 0.0);
        g.mobs[near].x = g.mobs[w].x + 1.0;
        g.mobs[near].y = g.mobs[w].y;
        g.mobs[near].flee = 0.0;
        let (px, py) = (g.p.x, g.p.y);
        cast(&mut g, Skill::DireWolf, px, py);
        assert!(g.mobs[near].flee > 0.0, "howl");
    }

    fn reaper_game() -> Game {
        let mut g = quiet_game();
        g.set_class(crate::skills::Class::Reaper);
        g.p.clvl = 18;
        for s in crate::skills::REAPER {
            g.p.skills.rank[s as usize] = 3;
        }
        g
    }

    #[test]
    fn the_reaper_gathers_souls_and_her_runes_blaze() {
        use crate::skills::Skill;
        let mut g = reaper_game();
        assert_eq!(g.p.mana, 0.0, "no souls yet");
        // A death nearby: its soul drifts into her lantern.
        let a = dummy(&mut g, 1.6, 0.0);
        g.mobs[a].hp = 1.0;
        let (ax, ay) = (g.mobs[a].x, g.mobs[a].y);
        g.p.cast_cd = 0.0;
        g.cast_skill(Skill::ReapingScythe, ax, ay);
        assert!(!g.mobs[a].alive());
        assert_eq!(g.souls.len(), 1);
        for _ in 0..90 {
            g.update(&Input::default());
        }
        assert!(g.souls.is_empty());
        assert_eq!(g.p.mana, crate::reaper::SOUL, "one soul");
        // Hunting the same prey lights the runes; at seven the next sweep blazes for triple.
        let b = dummy(&mut g, 1.6, 0.0);
        let (bx, by) = (g.mobs[b].x, g.mobs[b].y);
        let mut last = 0.0;
        for k in 0..8 {
            g.p.cast_cd = 0.0;
            // (Her sweeps knock it back; keep it in reach.)
            (g.mobs[b].x, g.mobs[b].y) = (bx, by);
            let before = g.mobs[b].hp;
            g.cast_skill(Skill::ReapingScythe, bx, by);
            last = before - g.mobs[b].hp;
            if k == 6 {
                assert_eq!(g.p.runes, crate::reaper::RUNES, "seven runes");
            }
        }
        assert_eq!(g.p.runes, 0, "the blaze spent the runes");
        assert!(last > 25.0, "the blazing cleave hit hard ({last})");
        // Out of souls her skills fall back to the free scythe.
        g.p.mana = 0.0;
        g.p.cast_cd = 0.0;
        let before = g.mobs[b].hp;
        g.cast_skill(Skill::SpiritLantern, bx, by);
        assert!(g.lanterns.is_empty() && g.mobs[b].hp < before);
    }

    #[test]
    fn the_reapers_skills_work() {
        use crate::skills::Skill;
        let mut g = reaper_game();
        g.waypoint = (-99.0, -99.0);
        let cast = |g: &mut Game, s: Skill, x: f32, y: f32| {
            g.p.mana = g.soul_cap();
            g.p.cast_cd = 0.0;
            g.p.skills.cooldown = [0.0; crate::skills::ALL.len()];
            g.cast_skill(s, x, y);
        };
        let z = dummy(&mut g, 4.0, 0.0);
        let (zx, zy) = (g.mobs[z].x, g.mobs[z].y);
        // Spirit Lantern hunts it down.
        let hp = g.mobs[z].hp;
        cast(&mut g, Skill::SpiritLantern, zx, zy);
        for _ in 0..90 {
            g.update(&Input::default());
        }
        assert!(g.mobs[z].hp < hp, "the flame struck");
        // The Ledger: marked, it takes more, and it's worth three souls.
        cast(&mut g, Skill::LedgerMark, zx, zy);
        assert!(g.mobs[z].marked > 0.0);
        g.mobs[z].hp = 1.0;
        g.p.mana = 0.0;
        g.p.cast_cd = 0.0;
        g.p.x = zx - 1.5;
        g.cast_skill(Skill::ReapingScythe, zx, zy);
        assert!(!g.mobs[z].alive());
        assert_eq!(g.souls.last().map(|s| s.n), Some(3.0));
        // Chains bind, the hourglass slows.
        let c = dummy(&mut g, 2.0, 1.0);
        let (cx, cy) = (g.mobs[c].x, g.mobs[c].y);
        cast(&mut g, Skill::ChainsOfArchive, cx, cy);
        assert!(g.mobs[c].stun > 1.0, "bound");
        cast(&mut g, Skill::Hourglass, cx, cy);
        g.update(&Input::default());
        assert!(g.mobs[c].slow_t > 0.0, "time crawls");
        // Scholars fight for her.
        let (px, py) = (g.p.x, g.p.y);
        cast(&mut g, Skill::ScholarSpirits, px, py);
        assert_eq!(g.mobs.iter().filter(|m| m.kind == Kind::Scholar && m.alive() && m.charm > 0.0).count(), crate::reaper::scholars(3));
        // Shadow Step.
        let x0 = g.p.x;
        let py = g.p.y;
        cast(&mut g, Skill::ShadowStep, x0 - 4.0, py);
        assert!(g.p.x < x0 - 1.5, "stepped through the smoke");
        // Soul Harvest reaps the badly wounded outright.
        let w = dummy(&mut g, 1.5, 0.0);
        g.mobs[w].hp = g.mobs[w].max_hp * 0.1;
        let (px, py) = (g.p.x, g.p.y);
        cast(&mut g, Skill::SoulHarvest, px, py);
        assert!(!g.mobs[w].alive(), "reaped");
        // Open the Ledger: the dead rise as scholars, and souls come double.
        let (px, py) = (g.p.x, g.p.y);
        cast(&mut g, Skill::OpenLedger, px, py);
        assert!(g.p.ledger_t > 0.0);
        let d = dummy(&mut g, 1.5, 0.0);
        let before = g.mobs.iter().filter(|m| m.kind == Kind::Scholar).count();
        g.mobs[d].hp = 0.0;
        g.kill(d);
        assert!(g.mobs.iter().filter(|m| m.kind == Kind::Scholar).count() > before, "the dead rise");
        assert_eq!(g.souls.last().map(|s| s.n), Some(2.0), "double souls");
    }

    fn druid_game() -> Game {
        let mut g = quiet_game();
        g.set_class(crate::skills::Class::Druid);
        g.p.clvl = 18;
        // A level-18 mana pool (her ultimates cost 50+).
        g.p.base_mana = 120.0;
        g.p.recalc();
        for s in crate::skills::DRUID {
            g.p.skills.rank[s as usize] = 3;
        }
        g
    }

    #[test]
    fn the_druid_poisons_and_keeps_the_balance() {
        use crate::skills::Skill;
        let mut g = druid_game();
        g.waypoint = (-99.0, -99.0);
        let cast = |g: &mut Game, s: Skill, x: f32, y: f32| {
            g.p.mana = g.p.max_mana;
            g.p.cast_cd = 0.0;
            g.p.skills.cooldown = [0.0; crate::skills::ALL.len()];
            g.cast_skill(s, x, y);
        };
        // Spore Cloud poisons, and pushes toward Decay.
        let z = dummy(&mut g, 3.0, 0.0);
        let (zx, zy) = (g.mobs[z].x, g.mobs[z].y);
        cast(&mut g, Skill::SporeCloud, zx, zy);
        assert!(g.p.balance < 0.0, "decay");
        let hp = g.mobs[z].hp;
        for _ in 0..60 {
            g.update(&Input::default());
        }
        assert!(g.mobs[z].poison_t > 0.0 && g.mobs[z].hp < hp, "poisoned");
        // Toward Decay, her creatures grow stronger; toward Bloom, her plague does.
        g.p.balance = -1.0;
        assert!(g.growth_mult() > 1.25 && (g.plague_mult() - 1.0).abs() < 1e-6);
        g.p.balance = 1.0;
        assert!(g.plague_mult() > 1.25);
        g.p.balance = 0.0;
        // Rat Swarm: rats fight for her, and push toward Bloom.
        let (px, py) = (g.p.x, g.p.y);
        cast(&mut g, Skill::RatSwarm, px, py);
        assert_eq!(g.mobs.iter().filter(|m| m.kind == Kind::Rat && m.alive() && m.charm > 0.0).count(), crate::druid::rats(3));
        assert!(g.p.balance > 0.0, "bloom");
        // Thorn Lash binds.
        let t = dummy(&mut g, 2.0, 0.0);
        let (tx, ty) = (g.mobs[t].x, g.mobs[t].y);
        cast(&mut g, Skill::ThornLash, tx, ty);
        assert!(g.mobs[t].stun > 1.0, "bound by thorns");
        // Rejuvenate heals her over time.
        g.p.hp = g.p.max_hp * 0.3;
        let hp = g.p.hp;
        let (px, py) = (g.p.x, g.p.y);
        cast(&mut g, Skill::Rejuvenate, px, py);
        for _ in 0..240 {
            g.update(&Input::default());
        }
        assert!(g.p.hp > hp + 20.0, "rejuvenated");
        // Fungal Bloom: mushrooms burst into clouds.
        cast(&mut g, Skill::FungalBloom, tx, ty);
        assert!(!g.fungi.is_empty());
        for _ in 0..100 {
            g.update(&Input::default());
        }
        assert!(g.fungi.is_empty() && !g.clouds.is_empty(), "they burst");
    }

    #[test]
    fn the_druids_creatures_and_plague_work() {
        use crate::skills::Skill;
        let mut g = druid_game();
        g.waypoint = (-99.0, -99.0);
        let cast = |g: &mut Game, s: Skill, x: f32, y: f32| {
            g.p.mana = g.p.max_mana;
            g.p.cast_cd = 0.0;
            g.p.skills.cooldown = [0.0; crate::skills::ALL.len()];
            g.cast_skill(s, x, y);
        };
        let (px, py) = (g.p.x, g.p.y);
        // The moss wolf stays, and follows her to the next level.
        cast(&mut g, Skill::MossWolf, px, py);
        assert_eq!(g.mobs.iter().filter(|m| m.kind == Kind::MossWolf && m.alive()).count(), 1);
        g.debug_goto(LevelId::Dungeon(0, 1));
        assert_eq!(g.mobs.iter().filter(|m| m.kind == Kind::MossWolf && m.alive()).count(), 1, "it came along");
        g.mobs.retain(|m| m.kind == Kind::MossWolf);
        // Corpse Bloom: a corpse bursts and rats crawl out.
        let c = dummy(&mut g, 2.0, 0.0);
        g.mobs[c].hp = 0.0;
        g.kill(c);
        let (px, py) = (g.p.x, g.p.y);
        cast(&mut g, Skill::CorpseBloom, px, py);
        assert_eq!(g.mobs.iter().filter(|m| m.kind == Kind::Rat && m.alive()).count(), 3);
        // Pestilence poisons everything around, and the plague spreads when one dies.
        let a = dummy(&mut g, 2.0, 1.0);
        let b = dummy(&mut g, 2.5, 1.5);
        cast(&mut g, Skill::Pestilence, px, py);
        assert!(g.mobs[a].plagued && g.mobs[a].poison_t > 0.0);
        let far = dummy(&mut g, 0.0, 0.0);
        g.mobs[far].x = g.mobs[a].x + 1.0;
        g.mobs[far].y = g.mobs[a].y - 1.0;
        g.mobs[far].poison_t = 0.0;
        g.mobs[a].hp = 0.0;
        g.kill(a);
        assert!(g.mobs[far].poison_t > 0.0, "the plague spread");
        let _ = b;
        // The Thorn Warden rises.
        cast(&mut g, Skill::ThornWarden, px, py);
        assert_eq!(g.mobs.iter().filter(|m| m.kind == Kind::ThornWarden && m.alive() && m.charm > 0.0).count(), 1);
    }

    #[test]
    fn the_inventors_gadgets_work() {
        use crate::skills::Skill;
        let mut g = inventor_game();
        for k in 0..3 {
            let mut m = Mob::new(Kind::Zombie, g.p.x + 3.0 + k as f32 * 1.5, g.p.y, 1.0, &mut g.rng);
            m.max_hp = 9000.0;
            m.hp = 9000.0;
            m.dmg = (0.0, 0.0);
            m.speed = 0.0;
            g.mobs.push(m);
        }
        let (zx, zy) = (g.mobs[0].x, g.mobs[0].y);
        let cast = |g: &mut Game, s: Skill, x: f32, y: f32| {
            g.p.mana = g.p.max_mana;
            g.p.overheat = 0.0;
            g.p.cast_cd = 0.0;
            g.cast_skill(s, x, y);
        };
        cast(&mut g, Skill::ArcCoil, zx, zy);
        assert!(g.mobs.iter().filter(|m| m.hp < 9000.0).count() >= 2, "the arc chained");
        cast(&mut g, Skill::Turret, zx, zy);
        cast(&mut g, Skill::Spider, zx, zy);
        cast(&mut g, Skill::TeslaField, zx, zy);
        assert_eq!((g.turrets.len(), g.spiders.len()), (1, 1));
        let hp: f32 = g.mobs.iter().map(|m| m.hp).sum();
        for _ in 0..240 {
            g.update(&Input::default());
        }
        assert!(g.mobs.iter().map(|m| m.hp).sum::<f32>() < hp, "turret and spider at work");
        cast(&mut g, Skill::AirshipStrike, zx, zy);
        assert_eq!(g.airships.len(), 1);
        for _ in 0..150 {
            g.update(&Input::default());
        }
        assert!(g.airships.is_empty());
        // Grapple: yank a foe to you, or zip to the floor.
        let d0 = (g.mobs[2].x - g.p.x).abs();
        let (fx, fy) = (g.mobs[2].x, g.mobs[2].y);
        cast(&mut g, Skill::Grapple, fx, fy);
        assert!((g.mobs[2].x - g.p.x).abs() < d0, "yanked closer");
        // Steam suit halves damage and makes skills free.
        let (px, py) = (g.p.x, g.p.y);
        cast(&mut g, Skill::SteamSuit, px, py);
        assert!(g.p.suit_t > 0.0);
        let hp = g.p.hp;
        g.hurt_player(20.0);
        assert!(hp - g.p.hp < 10.5);
        let mana = g.p.mana;
        g.p.cast_cd = 0.0;
        g.cast_skill(Skill::ClockBomb, zx, zy);
        assert_eq!(g.p.mana, mana, "free in the suit");
    }

    #[test]
    fn dying_in_the_north_wakes_you_in_kaldholm() {
        let mut g = Game::new(5, crate::gfx::SH_WIDE);
        g.quest.stage = 3;
        g.debug_goto(LevelId::Dungeon(5, 0));
        g.hurt_player(99_999.0);
        for _ in 0..120 {
            g.update(&Input { confirm: true, ..Input::default() });
        }
        assert_eq!(g.level, LevelId::Frostmarch);
        assert!(g.in_safe(g.p.x, g.p.y));
    }

    #[test]
    fn the_cold_chills_you_and_burns_well() {
        use crate::mobs::Rank;
        let mut g = quiet_game();
        // Chill slows you down.
        let x0 = g.p.x;
        for _ in 0..30 {
            g.update(&Input { move_x: 1.0, ..Input::default() });
        }
        let free = g.p.x - x0;
        let mut h = quiet_game();
        let x0 = h.p.x;
        for _ in 0..30 {
            h.chill(5.0);
            h.update(&Input { move_x: 1.0, ..Input::default() });
        }
        assert!((h.p.x - x0) < free * 0.8, "chilled: {} vs {}", h.p.x - x0, free);
        // Creatures of the cold take more fire damage.
        let mut g = quiet_game();
        for kind in [Kind::Zombie, Kind::Yeti] {
            let mut m = Mob::new(kind, g.p.x + 3.0, g.p.y, 1.0, &mut g.rng);
            m.max_hp = 1000.0;
            m.hp = 1000.0;
            m.rank = Rank::Normal;
            g.mobs.push(m);
        }
        g.hit_mob(0, 100.0, 0.0, 0.0, None, false);
        g.hit_mob(1, 100.0, 0.0, 0.0, None, false);
        assert!(g.mobs[1].hp < g.mobs[0].hp, "the yeti burns better");
        // Ice trolls regenerate unless burning.
        let mut t = Mob::new(Kind::IceTroll, g.p.x + 6.0, g.p.y + 6.0, 1.0, &mut g.rng);
        t.hp = t.max_hp * 0.5;
        g.mobs.push(t);
        let hp = g.mobs[2].hp;
        for _ in 0..60 {
            g.update(&Input::default());
        }
        assert!(g.mobs[2].hp > hp, "troll heals");
        g.mobs[2].burn = 5.0;
        let hp = g.mobs[2].hp;
        for _ in 0..60 {
            g.update(&Input::default());
        }
        assert!(g.mobs[2].hp < hp, "but not while burning");
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
        // Bosses drop their unique and two magic-or-better items...
        g.pickups.clear();
        let m = Mob::new(Kind::BoneWarden, g.p.x + 3.0, g.p.y, 1.0, &mut g.rng);
        g.mobs.push(m);
        let i = g.mobs.len() - 1;
        g.mobs[i].boss = true;
        g.kill(i);
        let loot: Vec<_> = g.pickups.iter().filter_map(|k| if let Drop::Item(it) = &k.kind { Some(it.clone()) } else { None }).collect();
        // ...and a gem.
        assert_eq!(loot.len(), 4);
        assert!(loot.iter().any(|it| it.name == "MARROWGRIP"));
        assert_eq!(loot.iter().filter(|it| it.gem().is_some()).count(), 1);
        assert!(loot.iter().filter(|it| it.gem().is_none()).all(|it| it.rarity >= Rarity::Magic));
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
        // (Old Bonejaw, a super unique, lives on this floor too: he's counted apart.)
        let count = |r: Rank| lv.mobs.iter().filter(|m| m.rank == r && m.superu == 0).count();
        assert!(count(Rank::Elite) == 1, "one elite leader per floor");
        assert!(count(Rank::Minion) >= 1, "with minions");
        assert!(count(Rank::Champion) >= 2, "and a champion pack");
        let e = lv.mobs.iter().find(|m| m.rank == Rank::Elite).unwrap();
        assert!(e.name.is_some() && e.mods.count_ones() == 2);
        let ow = world::build(LevelId::Area(0, 2), 7);
        // A super unique clears its spot, which can take an elite pack with it.
        assert!((2..=3).contains(&ow.mobs.iter().filter(|m| m.rank == Rank::Elite && m.superu == 0).count()));
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
    fn each_town_has_its_own_music() {
        use crate::music::Track;
        let mut g = Game::new(5, crate::gfx::SH_WIDE);
        for (id, town, wild) in [
            (LevelId::Overworld, Track::Town, Track::Wilds),
            (LevelId::Frostmarch, Track::Hearth, Track::Frost),
            (LevelId::Mistwood, Track::Vigil, Track::Mist),
            (LevelId::Mechanus, Track::Refuge, Track::Gears),
        ] {
            g.debug_goto(id);
            (g.p.x, g.p.y) = g.start;
            assert!(g.in_safe(g.p.x, g.p.y));
            assert_eq!(g.music_track(), town, "{id:?} town");
            let (x0, y0, ..) = g.safe.unwrap();
            (g.p.x, g.p.y) = (x0 - 12.0, y0 - 12.0);
            assert_eq!(g.music_track(), wild, "{id:?} outside town");
        }
    }

    #[test]
    fn every_town_has_a_jeweler_who_joins_cuts_and_sets_gems() {
        use crate::inventory::Cell;
        use crate::items::{gem_item, Gem, Rarity};
        let mut g = Game::new(5, crate::gfx::SH_WIDE);
        for (k, id) in [LevelId::Overworld, LevelId::Frostmarch, LevelId::Mistwood, LevelId::Mechanus].into_iter().enumerate() {
            g.debug_goto(id);
            let n = g.npcs.iter().find(|n| n.role == Role::Jeweler(k as u8)).expect("a jeweler in town");
            assert!(!g.d.blocked(n.x, n.y, 0.3), "the jeweler stands on open ground in act {}", k + 1);
            assert!(g.in_safe(n.x, n.y));
        }
        g.debug_goto(LevelId::Overworld);
        g.p.gold = 1000;
        g.p.gear = crate::items::Gear::default();
        for _ in 0..3 {
            g.p.gear.add(gem_item(Gem { kind: 0, grade: 1 })).unwrap();
        }
        let mut staff = crate::items::roll_base(crate::items::base_by_key("gnarled").unwrap(), 3, Rarity::Normal, &mut g.rng);
        staff.sockets = 0;
        g.p.gear.bag[5] = Some(staff);
        let pick = |g: &mut Game, a: Act| {
            assert!(g.debug_talk(Role::Jeweler(0)));
            let k = g.dialog.as_ref().unwrap().options.iter().position(|o| o.1 == a).unwrap();
            g.dialog.as_mut().unwrap().sel = k;
            g.update(&Input { confirm: true, ..Input::default() });
            g.update(&Input::default());
        };
        // Three chipped rubies make a flawed one, for a fee.
        pick(&mut g, Act::Combine);
        assert_eq!(g.p.gold, 1000 - crate::items::combine_cost(Gem { kind: 0, grade: 2 }));
        let gi = g.p.gear.bag.iter().position(|c| c.as_ref().and_then(|i| i.gem()) == Some(Gem { kind: 0, grade: 2 })).expect("a flawed ruby");
        g.dialog = None;
        // The bench cuts sockets in the plain staff.
        pick(&mut g, Act::Jewel);
        assert!(g.inv.as_ref().map_or(false, |u| u.jewel));
        g.inv.as_mut().unwrap().sel = Cell::Bag(5);
        g.update(&Input { confirm: true, ..Input::default() });
        g.update(&Input::default());
        let s = g.p.gear.bag[5].as_ref().unwrap().sockets;
        assert!((1..=3).contains(&s), "sockets cut: {s}");
        g.inv = None;
        // Pick up the gem, then the staff: the gem goes in.
        g.open_inventory();
        g.inv.as_mut().unwrap().sel = Cell::Bag(gi);
        g.update(&Input { confirm: true, ..Input::default() });
        g.update(&Input::default());
        assert_eq!(g.inv.as_ref().unwrap().holding, Some(gi));
        g.inv.as_mut().unwrap().sel = Cell::Bag(5);
        g.update(&Input { confirm: true, ..Input::default() });
        g.update(&Input::default());
        assert!(g.p.gear.bag[gi].is_none());
        assert_eq!(g.p.gear.bag[5].as_ref().unwrap().gems, vec![Gem { kind: 0, grade: 2 }]);
        // And the jeweler takes it back out.
        g.inv = None;
        let gold = g.p.gold;
        pick(&mut g, Act::Jewel);
        g.inv.as_mut().unwrap().sel = Cell::Bag(5);
        g.update(&Input { confirm: true, ..Input::default() });
        assert!(g.p.gear.bag[5].as_ref().unwrap().gems.is_empty());
        assert_eq!(g.p.gold, gold - 50);
        assert!(g.p.gear.bag.iter().flatten().any(|i| i.gem() == Some(Gem { kind: 0, grade: 2 })));
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
    fn nightmare_follows_the_rime_wyrm() {
        let mut g = Game::new(5, crate::gfx::SH_WIDE);
        let normal_hp: f32 = world::build_at(LevelId::Area(0, 1), g.world_seed, 0).mobs.iter().map(|m| m.max_hp).sum();
        g.quest.stage = 3;
        g.quest.seals = [true; 3];
        g.p.gear.bag[0] = Some(crate::items::unique(0));
        let clvl = g.p.clvl;
        // The Ash King and the Rime Wyrm only open the next land; nightmare follows Count Vardak.
        g.quest.stage2 = 3;
        g.quest.runes = [true; 3];
        g.debug_goto(LevelId::Frostmarch);
        assert!(g.debug_talk(Role::Captain));
        assert!(!g.dialog.as_ref().unwrap().options.iter().any(|o| o.1 == Act::NextDifficulty), "not after act 2");
        g.dialog = None;
        g.quest.stage3 = 3;
        g.quest.sigils = [true; 3];
        g.debug_goto(LevelId::Mistwood);
        assert!(g.debug_talk(Role::Hunter));
        assert!(!g.dialog.as_ref().unwrap().options.iter().any(|o| o.1 == Act::NextDifficulty), "not after act 3");
        g.dialog = None;
        g.quest.stage4 = 3;
        g.quest.keys = [true; 3];
        g.debug_goto(LevelId::Mechanus);
        assert!(g.debug_talk(Role::Tally));
        assert!(!g.dialog.as_ref().unwrap().options.iter().any(|o| o.1 == Act::NextDifficulty), "not after act 4");
        g.dialog = None;
        g.quest.stage5 = 3;
        g.quest.pearls = [true; 3];
        g.debug_goto(LevelId::Deep);
        assert!(g.debug_talk(Role::Ysolde));
        assert!(!g.dialog.as_ref().unwrap().options.iter().any(|o| o.1 == Act::NextDifficulty), "not after act 5");
        g.dialog = None;
        g.quest.stage6 = 3;
        g.quest.shards = [true; 3];
        g.debug_goto(LevelId::Heavens);
        assert!(g.debug_talk(Role::Seraphine));
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
        assert_eq!((g.quest.stage, g.quest.seal_count(), g.quest.stage2, g.quest.rune_count()), (1, 0, 0, 0), "quests start over");
        assert_eq!((g.quest.stage3, g.quest.sigil_count(), g.quest.stage4, g.quest.key_count()), (0, 0, 0, 0));
        assert_eq!((g.quest.stage5, g.quest.pearl_count(), g.quest.stage6, g.quest.shard_count()), (0, 0, 0, 0));
        assert!(g.p.gear.bag[0].is_some() && g.p.clvl == clvl, "you keep your hero");
        assert!(g.level_name.contains("NIGHTMARE"));
        let nm_hp: f32 = world::build_at(LevelId::Area(0, 1), g.world_seed, 1).mobs.iter().map(|m| m.max_hp).sum();
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
        // Act 2 progress, and you wake in Kaldholm if you saved in the north.
        let mut g = Game::new(5, crate::gfx::SH_WIDE);
        g.quest.stage = 3;
        g.quest.stage2 = 1;
        g.quest.runes = [true, false, true];
        g.debug_goto(LevelId::Frostmarch);
        let text = crate::save::to_text(&g);
        let mut h = Game::new(5, crate::gfx::SH_WIDE);
        crate::save::apply(&mut h, &text);
        assert_eq!((h.quest.stage2, h.quest.runes), (1, [true, false, true]));
        assert_eq!(h.level, LevelId::Frostmarch);
        assert!(h.in_safe(h.p.x, h.p.y));
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

//! Level files (JSON) written by the level editor (`tools/level-editor/index.html`).
//!
//! Lookup order for each map: `levels/<name>.json` on disk (so edits show up without a
//! rebuild), then the same file embedded at build time (`build.rs` embeds `levels/`), then
//! the procedural generator. `--export-levels <dir>` writes the generated maps as starting
//! points. The format is documented in `docs/LEVEL_FORMAT.md`.
use crate::dungeon::{Dungeon, Tile};
use crate::game::{Drop, Pickup};
use crate::mobs::{def, Kind, Mob};
use crate::rng::Rng;
use crate::story::{Npc, Role};
use crate::world::{self, Level, LevelId, Portal, PortalKind, Prop, PropKind, Theme, DUNGEONS};
use serde::{Deserialize, Serialize};

include!(concat!(env!("OUT_DIR"), "/levels_gen.rs"));

pub const FORMAT_VERSION: u32 = 1;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Ent {
    pub kind: String,
    pub x: f32,
    pub y: f32,
    /// Footprint in tiles (props only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub w: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub h: Option<i32>,
    /// Difficulty multiplier (monsters only; defaults to the level's tier).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tier: Option<f32>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LevelFile {
    pub version: u32,
    /// "overworld" or "dungeon:<index>:<floor>" (both from 0).
    pub id: String,
    pub name: String,
    pub theme: String,
    pub tier: f32,
    pub width: i32,
    pub height: i32,
    /// One string per row. '.' floor, '#' wall, 'o' blocked by a prop, ' ' void.
    pub tiles: Vec<String>,
    /// One string per row. 'g' grass, 'd' dirt, 'r' road (overworld art; ignored in dungeons).
    #[serde(default)]
    pub ground: Vec<String>,
    /// Where you wake up / arrive when there's no matching portal.
    pub start: [f32; 2],
    /// Town safe zone: x0, y0, x1, y1.
    #[serde(default)]
    pub safe: Option<[f32; 4]>,
    #[serde(default)]
    pub props: Vec<Ent>,
    #[serde(default)]
    pub portals: Vec<Ent>,
    #[serde(default)]
    pub monsters: Vec<Ent>,
    #[serde(default)]
    pub npcs: Vec<Ent>,
    #[serde(default)]
    pub items: Vec<Ent>,
}

// ---------------------------------------------------------------- names

pub const DUNGEON_SLUGS: [&str; 24] = [
    "bone_crypt",
    "rotting_warrens",
    "hexed_catacombs",
    "ashen_sanctum",
    "frozen_mines",
    "howling_caves",
    "rime_temple",
    "glaciers_heart",
    "sunken_chapel",
    "gallows_catacombs",
    "barrow_of_knights",
    "castle_vardak",
    "foundry_of_souls",
    "choir_engine",
    "archive_of_gears",
    "heart_of_the_clock",
    "wreck_of_the_sovereign",
    "coral_cathedral",
    "midnight_trench",
    "drowned_sanctum",
    "broken_choir",
    "storm_spire",
    "wheel_of_eyes",
    "true_sanctum",
];

/// File name (without .json) for a level.
pub fn file_name(id: LevelId) -> String {
    match id {
        LevelId::Overworld => "overworld".into(),
        LevelId::Frostmarch => "frostmarch".into(),
        LevelId::Mistwood => "mistwood".into(),
        LevelId::Mechanus => "mechanus".into(),
        LevelId::Deep => "deep".into(),
        LevelId::Heavens => "heavens".into(),
        LevelId::Dungeon(k, f) => format!("{}_floor{}", DUNGEON_SLUGS[k], f + 1),
        LevelId::Rift(t) => format!("rift{t}"),
    }
}

pub fn id_string(id: LevelId) -> String {
    match id {
        LevelId::Overworld => "overworld".into(),
        LevelId::Frostmarch => "frostmarch".into(),
        LevelId::Mistwood => "mistwood".into(),
        LevelId::Mechanus => "mechanus".into(),
        LevelId::Deep => "deep".into(),
        LevelId::Heavens => "heavens".into(),
        LevelId::Dungeon(k, f) => format!("dungeon:{k}:{f}"),
        LevelId::Rift(t) => format!("rift:{t}"),
    }
}

pub fn parse_id(s: &str) -> Option<LevelId> {
    if s == "overworld" {
        return Some(LevelId::Overworld);
    }
    if s == "frostmarch" {
        return Some(LevelId::Frostmarch);
    }
    if s == "mistwood" {
        return Some(LevelId::Mistwood);
    }
    if s == "mechanus" {
        return Some(LevelId::Mechanus);
    }
    if s == "deep" {
        return Some(LevelId::Deep);
    }
    if s == "heavens" {
        return Some(LevelId::Heavens);
    }
    if let Some(t) = s.strip_prefix("rift:") {
        return t.parse().ok().map(LevelId::Rift);
    }
    let mut it = s.strip_prefix("dungeon:")?.split(':');
    let k: usize = it.next()?.parse().ok()?;
    let f: usize = it.next()?.parse().ok()?;
    (k < DUNGEONS.len() && f < DUNGEONS[k].floors).then_some(LevelId::Dungeon(k, f))
}

/// Every level in the game, in order.
pub fn all_ids() -> Vec<LevelId> {
    let mut v = vec![LevelId::Overworld, LevelId::Frostmarch, LevelId::Mistwood, LevelId::Mechanus, LevelId::Deep, LevelId::Heavens];
    for (k, d) in DUNGEONS.iter().enumerate() {
        for f in 0..d.floors {
            v.push(LevelId::Dungeon(k, f));
        }
    }
    v
}

/// A level id from a file name or a path to one ("bone_crypt_floor2", "levels/overworld.json").
pub fn id_from_name(name: &str) -> Option<LevelId> {
    let stem = std::path::Path::new(name).file_stem()?.to_str()?;
    all_ids().into_iter().find(|id| file_name(*id) == stem)
}

const THEMES: [(&str, Theme); 30] = [
    ("overworld", Theme::Overworld),
    ("crypt", Theme::Crypt),
    ("warrens", Theme::Warrens),
    ("catacombs", Theme::Catacombs),
    ("sanctum", Theme::Sanctum),
    ("tundra", Theme::Tundra),
    ("mines", Theme::Mines),
    ("icecaves", Theme::IceCaves),
    ("rime", Theme::Rime),
    ("glacier", Theme::Glacier),
    ("mistwood", Theme::Mistwood),
    ("chapel", Theme::Chapel),
    ("gallows", Theme::Gallows),
    ("barrow", Theme::Barrow),
    ("castle", Theme::Castle),
    ("mechanus", Theme::Mechanus),
    ("foundry", Theme::Foundry),
    ("choir", Theme::Choir),
    ("archive", Theme::Archive),
    ("clock", Theme::Clock),
    ("deep", Theme::Deep),
    ("wreck", Theme::Wreck),
    ("reef", Theme::Reef),
    ("trench", Theme::Trench),
    ("drowned", Theme::Drowned),
    ("heavens", Theme::Heavens),
    ("seraph", Theme::Seraph),
    ("spire", Theme::Spire),
    ("wheel", Theme::Wheel),
    ("zenith", Theme::Zenith),
];

const PROPS: [PropKind; 82] = [
    PropKind::TreeOak,
    PropKind::TreePine,
    PropKind::TreeDead,
    PropKind::Rock,
    PropKind::Bush,
    PropKind::House1,
    PropKind::House2,
    PropKind::Stall,
    PropKind::Campfire,
    PropKind::Well,
    PropKind::Entrance(0),
    PropKind::Entrance(1),
    PropKind::Entrance(2),
    PropKind::Entrance(3),
    PropKind::StairsDown,
    PropKind::StairsUp,
    PropKind::SnowPine,
    PropKind::SnowDead,
    PropKind::SnowRock,
    PropKind::IceCrystal,
    PropKind::Longhouse1,
    PropKind::Longhouse2,
    PropKind::FurStall,
    PropKind::Entrance(4),
    PropKind::Entrance(5),
    PropKind::Entrance(6),
    PropKind::Entrance(7),
    PropKind::Pass,
    PropKind::TwistedTree,
    PropKind::MistPine,
    PropKind::GlowShrooms,
    PropKind::Gravestone,
    PropKind::Cottage1,
    PropKind::Cottage2,
    PropKind::Gallows,
    PropKind::Cart,
    PropKind::Entrance(8),
    PropKind::Entrance(9),
    PropKind::Entrance(10),
    PropKind::Entrance(11),
    PropKind::PassMist,
    PropKind::GearTower,
    PropKind::SteamPipes,
    PropKind::SteamVent,
    PropKind::GasLamp,
    PropKind::CogPile,
    PropKind::Workshop1,
    PropKind::Workshop2,
    PropKind::ClockTower,
    PropKind::Pendulum,
    PropKind::Entrance(12),
    PropKind::Entrance(13),
    PropKind::Entrance(14),
    PropKind::Entrance(15),
    PropKind::GearGate,
    PropKind::Kelp,
    PropKind::Coral1,
    PropKind::Coral2,
    PropKind::WreckHull,
    PropKind::WhaleBones,
    PropKind::StiltHouse1,
    PropKind::StiltHouse2,
    PropKind::ShellLamp,
    PropKind::AnchorRock,
    PropKind::DivingBell,
    PropKind::Entrance(16),
    PropKind::Entrance(17),
    PropKind::Entrance(18),
    PropKind::Entrance(19),
    PropKind::AngelStatue,
    PropKind::HaloArch,
    PropKind::SkyLamp,
    PropKind::CloudTree,
    PropKind::MarbleRuin,
    PropKind::SkyHouse1,
    PropKind::SkyHouse2,
    PropKind::AirshipDock,
    PropKind::LightStair,
    PropKind::Entrance(20),
    PropKind::Entrance(21),
    PropKind::Entrance(22),
    PropKind::Entrance(23),
];

const KINDS: [Kind; 66] = [
    Kind::Zombie,
    Kind::Skeleton,
    Kind::Wolf,
    Kind::Goblin,
    Kind::Archer,
    Kind::BoneWarden,
    Kind::PlagueWarden,
    Kind::HexWarden,
    Kind::AshKing,
    Kind::FrostWolf,
    Kind::Raider,
    Kind::Yeti,
    Kind::IceTroll,
    Kind::IceWraith,
    Kind::FrostGiant,
    Kind::YetiMatriarch,
    Kind::RimeWitch,
    Kind::WhiteDragon,
    Kind::Ghoul,
    Kind::Werewolf,
    Kind::Banshee,
    Kind::Wisp,
    Kind::Cultist,
    Kind::Bat,
    Kind::Ossric,
    Kind::Grimhilde,
    Kind::Malgrave,
    Kind::Vardak,
    Kind::Scarab,
    Kind::Inquisitor,
    Kind::Gearwraith,
    Kind::SpringJack,
    Kind::BoilerBrute,
    Kind::Ordinal,
    Kind::Prism,
    Kind::Marshal,
    Kind::Forgemother,
    Kind::Cantor,
    Kind::Archivist,
    Kind::Clockmaker,
    Kind::ClockCrow,
    Kind::Crate,
    Kind::Barrel,
    Kind::Urn,
    Kind::Drowned,
    Kind::Merrow,
    Kind::Anglerlurk,
    Kind::Jelly,
    Kind::Shellguard,
    Kind::Siren,
    Kind::InkHorror,
    Kind::Dregmoor,
    Kind::Nacre,
    Kind::Angler,
    Kind::Leviathan,
    Kind::FallenSeraph,
    Kind::Ophanim,
    Kind::StormDrake,
    Kind::Harpy,
    Kind::Sentinel,
    Kind::Zealot,
    Kind::Thunderbird,
    Kind::Vael,
    Kind::Tempest,
    Kind::OphanPrime,
    Kind::Solanthos,
];

fn portal_name(k: PortalKind) -> String {
    match k {
        PortalKind::Entrance(i) => format!("entrance{i}"),
        PortalKind::Up => "up".into(),
        PortalKind::Down => "down".into(),
        PortalKind::TownPortal => "townportal".into(),
        PortalKind::Pass(a) => format!("pass{a}"),
        PortalKind::Dock(n) => format!("dock{n}"),
    }
}

fn portal_kind(s: &str) -> Option<PortalKind> {
    match s {
        "up" => Some(PortalKind::Up),
        "down" => Some(PortalKind::Down),
        "townportal" => Some(PortalKind::TownPortal),
        "pass0" => Some(PortalKind::Pass(0)),
        "pass1" => Some(PortalKind::Pass(1)),
        "pass2" => Some(PortalKind::Pass(2)),
        "pass3" => Some(PortalKind::Pass(3)),
        "pass4" => Some(PortalKind::Pass(4)),
        "pass5" => Some(PortalKind::Pass(5)),
        "dock0" => Some(PortalKind::Dock(0)),
        _ => s.strip_prefix("entrance")?.parse().ok().filter(|i: &usize| *i < DUNGEONS.len()).map(PortalKind::Entrance),
    }
}

/// NPC kinds: file name, role, display name, art.
const NPCS: [(&str, Role, &str, &str); 44] = [
    ("elder", Role::Elder, "ELDER MAREN", "npc_elder"),
    ("merchant", Role::Merchant, "GERTA", "npc_merchant"),
    ("healer", Role::Healer, "BROTHER ALDRIC", "npc_healer"),
    ("guard", Role::Guard, "CAPTAIN ROLF", "npc_guard"),
    ("villager0", Role::Villager(0), "VILLAGER", "npc_villager"),
    ("villager1", Role::Villager(1), "FARMER", "npc_villager"),
    ("villager2", Role::Villager(2), "VILLAGER", "npc_villager"),
    ("villager3", Role::Villager(3), "FARMER", "npc_villager"),
    ("captain", Role::Captain, "CAPTAIN BRENNA", "npc_captain"),
    ("trader", Role::Trader, "OLD SIGURD", "npc_trader"),
    ("seer", Role::Seer, "MOTHER YLVA", "npc_seer"),
    ("fisher0", Role::Fisher(0), "FISHERMAN", "npc_fisher"),
    ("fisher1", Role::Fisher(1), "FISHERMAN", "npc_fisher"),
    ("fisher2", Role::Fisher(2), "FISHERWIFE", "npc_fisher"),
    ("hunter", Role::Hunter, "ABELARD", "npc_hunter"),
    ("widow", Role::Widow, "WIDOW KASIA", "npc_widow"),
    ("priest", Role::Priest, "FATHER LUCIAN", "npc_priest"),
    ("peasant0", Role::Peasant(0), "PEASANT", "npc_peasant"),
    ("peasant1", Role::Peasant(1), "PEASANT", "npc_peasant"),
    ("peasant2", Role::Peasant(2), "PEASANT", "npc_peasant"),
    ("tally", Role::Tally, "TALLY", "npc_tally"),
    ("vesper", Role::Vesper, "MADAME VESPER", "npc_vesper"),
    ("oiler", Role::Oiler, "BROTHER PISTON", "npc_oiler"),
    ("servant0", Role::Servant(0), "SERVANT", "npc_servant"),
    ("servant1", Role::Servant(1), "SERVANT", "npc_servant"),
    ("servant2", Role::Servant(2), "SERVANT", "npc_servant"),
    ("jeweler0", Role::Jeweler(0), "MASTER ODO", "npc_jeweler0"),
    ("jeweler1", Role::Jeweler(1), "INGRID STONEHAND", "npc_jeweler1"),
    ("jeweler2", Role::Jeweler(2), "SILAS GREAVE", "npc_jeweler2"),
    ("jeweler3", Role::Jeweler(3), "THE LAPIDARY", "npc_jeweler3"),
    ("ysolde", Role::Ysolde, "CAPTAIN YSOLDE MARROW", "npc_ysolde"),
    ("nessa", Role::Nessa, "NESSA THE PEARL-DIVER", "npc_nessa"),
    ("coral", Role::Coral, "BROTHER CORAL", "npc_coral"),
    ("diver0", Role::Diver(0), "DIVER", "npc_diver"),
    ("diver1", Role::Diver(1), "DIVER", "npc_diver"),
    ("diver2", Role::Diver(2), "DIVER", "npc_diver"),
    ("jeweler4", Role::Jeweler(4), "THE PEARL-SETTER", "npc_jeweler4"),
    ("seraphine", Role::Seraphine, "SERAPHINE", "npc_seraphine"),
    ("bram", Role::Bram, "QUARTERMASTER BRAM", "npc_bram"),
    ("aurel", Role::Aurel, "SISTER AUREL", "npc_aurel"),
    ("deckhand0", Role::Deckhand(0), "DECKHAND", "npc_deckhand"),
    ("deckhand1", Role::Deckhand(1), "DECKHAND", "npc_deckhand"),
    ("deckhand2", Role::Deckhand(2), "DECKHAND", "npc_deckhand"),
    ("jeweler5", Role::Jeweler(5), "THE GILDER", "npc_jeweler5"),
];

fn item_name(d: &Drop) -> Option<String> {
    Some(match d {
        Drop::Food(0) => "apple".into(),
        Drop::Food(1) => "bread".into(),
        Drop::Food(_) => "roast".into(),
        Drop::Health => "health_potion".into(),
        Drop::Mana => "mana_potion".into(),
        Drop::Gold(n) => format!("gold{n}"),
        Drop::Seal(_) | Drop::Rune(_) | Drop::Sigil(_) | Drop::Key(_) | Drop::Pearl(_) | Drop::Shard(_) | Drop::Item(_) => return None,
    })
}

fn item_kind(s: &str) -> Option<Drop> {
    Some(match s {
        "apple" => Drop::Food(0),
        "bread" => Drop::Food(1),
        "roast" => Drop::Food(2),
        "health_potion" => Drop::Health,
        "mana_potion" => Drop::Mana,
        _ => Drop::Gold(s.strip_prefix("gold")?.parse().ok()?),
    })
}

// ---------------------------------------------------------------- export

pub fn to_file(lv: &Level) -> LevelFile {
    let d = &lv.d;
    let tiles = (0..d.h)
        .map(|y| {
            (0..d.w)
                .map(|x| match d.get(x, y) {
                    Tile::Floor => '.',
                    Tile::Wall => '#',
                    Tile::Prop => 'o',
                    Tile::Void => ' ',
                })
                .collect()
        })
        .collect();
    let ground = if lv.theme.open() {
        (0..d.h).map(|y| (0..d.w).map(|x| ['g', 'd', 'r'][d.ground_at(x, y).min(2) as usize]).collect()).collect()
    } else {
        vec![]
    };
    let theme = THEMES.iter().find(|t| t.1 == lv.theme).map(|t| t.0).unwrap_or("crypt");
    let props = lv
        .props
        .iter()
        .filter(|p| !matches!(p.kind, PropKind::StairsUp | PropKind::StairsDown))
        .map(|p| Ent { kind: p.kind.art().into(), x: p.foot.0 as f32, y: p.foot.1 as f32, w: Some(p.foot.2), h: Some(p.foot.3), tier: None })
        .collect();
    let ent = |kind: String, x: f32, y: f32| Ent { kind, x: (x * 100.0).round() / 100.0, y: (y * 100.0).round() / 100.0, w: None, h: None, tier: None };
    LevelFile {
        version: FORMAT_VERSION,
        id: id_string(lv.id),
        name: lv.name.clone(),
        theme: theme.into(),
        tier: lv.tier,
        width: d.w,
        height: d.h,
        tiles,
        ground,
        start: [lv.start.0, lv.start.1],
        safe: lv.safe.map(|(a, b, c, e)| [a, b, c, e]),
        props,
        portals: lv.portals.iter().map(|p| ent(portal_name(p.kind), p.x, p.y)).collect(),
        monsters: lv
            .mobs
            .iter()
            .filter(|m| m.alive())
            .map(|m| Ent { tier: Some((m.tier * 1000.0).round() / 1000.0), ..ent(def(m.kind).art.into(), m.x, m.y) })
            .collect(),
        npcs: lv
            .npcs
            .iter()
            .filter_map(|n| NPCS.iter().find(|e| e.1 == n.role).map(|e| ent(e.0.into(), n.x, n.y)))
            .collect(),
        items: lv.pickups.iter().filter_map(|k| item_name(&k.kind).map(|n| ent(n, k.x, k.y))).collect(),
    }
}

// ---------------------------------------------------------------- import

/// Builds a playable level from a file. Unknown kinds are skipped with a warning.
pub fn from_file(f: &LevelFile, seed: u64) -> Result<Level, String> {
    if f.version != FORMAT_VERSION {
        return Err(format!("unsupported version {}", f.version));
    }
    let id = parse_id(&f.id).ok_or_else(|| format!("bad id {:?}", f.id))?;
    let (w, h) = (f.width, f.height);
    if !(8..=512).contains(&w) || !(8..=512).contains(&h) || f.tiles.len() != h as usize {
        return Err(format!("bad size {w}x{h} with {} tile rows", f.tiles.len()));
    }
    let theme = THEMES.iter().find(|t| t.0 == f.theme).map(|t| t.1).ok_or_else(|| format!("bad theme {:?}", f.theme))?;
    let mut d = Dungeon::blank(w, h, Tile::Void);
    let mut rng = Rng::new(seed ^ 0xED17);
    for (y, row) in f.tiles.iter().enumerate() {
        for (x, c) in row.chars().take(w as usize).enumerate() {
            let t = match c {
                '.' => Tile::Floor,
                '#' => Tile::Wall,
                'o' => Tile::Prop,
                _ => Tile::Void,
            };
            d.set(x as i32, y as i32, t);
        }
    }
    for (y, row) in f.ground.iter().enumerate().take(h as usize) {
        for (x, c) in row.chars().take(w as usize).enumerate() {
            d.set_ground(x as i32, y as i32, match c {
                'd' => 1,
                'r' => 2,
                _ => 0,
            });
        }
    }
    for v in d.var.iter_mut() {
        *v = rng.range(0, 100) as u8;
    }
    let mut warn = vec![];
    let mut lv = world::empty_level(id, f.name.clone(), theme, f.tier, d);
    lv.start = (f.start[0], f.start[1]);
    lv.safe = f.safe.map(|s| (s[0], s[1], s[2], s[3]));
    for p in &f.props {
        match PROPS.iter().find(|k| k.art() == p.kind) {
            Some(&kind) => {
                let (x0, y0) = (p.x.floor() as i32, p.y.floor() as i32);
                let (fw, fh) = (p.w.unwrap_or(1).max(1), p.h.unwrap_or(1).max(1));
                for yy in y0..y0 + fh {
                    for xx in x0..x0 + fw {
                        lv.d.set(xx, yy, Tile::Prop);
                    }
                }
                lv.props.push(Prop::on(kind, x0, y0, fw, fh));
            }
            None => warn.push(format!("prop {:?}", p.kind)),
        }
    }
    for p in &f.portals {
        match portal_kind(&p.kind) {
            Some(kind) => {
                lv.portals.push(Portal { x: p.x, y: p.y, kind });
                let (tx, ty) = (p.x.floor() as i32, p.y.floor() as i32);
                match kind {
                    PortalKind::Up if !theme.open() => lv.props.push(Prop::stairs(PropKind::StairsUp, tx, ty)),
                    PortalKind::Down => lv.props.push(Prop::stairs(PropKind::StairsDown, tx, ty)),
                    _ => {}
                }
            }
            None => warn.push(format!("portal {:?}", p.kind)),
        }
    }
    for m in &f.monsters {
        // "imp" is the old name for goblins.
        let name = if m.kind == "imp" { "goblin" } else { m.kind.as_str() };
        match KINDS.iter().find(|k| def(**k).art == name) {
            Some(&kind) => {
                let boss = def(kind).boss;
                // Bosses are scaled like the generator's (half the floor tier, plus half).
                let tier = m.tier.unwrap_or(if boss { f.tier * 0.5 + 0.5 } else { f.tier });
                let mut mob = Mob::new(kind, m.x, m.y, tier, &mut rng);
                if boss {
                    mob.home = (-1000.0, -2000.0);
                    mob.max_hp *= world::boss_life(id.act());
                    mob.hp = mob.max_hp;
                }
                lv.mobs.push(mob);
            }
            None => warn.push(format!("monster {:?}", m.kind)),
        }
    }
    for n in &f.npcs {
        match NPCS.iter().find(|e| e.0 == n.kind) {
            Some(&(_, role, name, art)) => lv.npcs.push(Npc::new(name, role, art, n.x, n.y, 0)),
            None => warn.push(format!("npc {:?}", n.kind)),
        }
    }
    for it in &f.items {
        match item_kind(&it.kind) {
            Some(kind) => lv.pickups.push(Pickup { x: it.x, y: it.y, kind, t: 1.0 }),
            None => warn.push(format!("item {:?}", it.kind)),
        }
    }
    if !warn.is_empty() {
        eprintln!("level {}: skipped unknown {}", f.id, warn.join(", "));
    }
    Ok(lv)
}

// ---------------------------------------------------------------- lookup

/// Where the editable level files live: `$ASHEN_LEVELS`, else `levels/` in the working directory.
pub fn dir() -> std::path::PathBuf {
    std::env::var("ASHEN_LEVELS").map(std::path::PathBuf::from).unwrap_or_else(|_| "levels".into())
}

/// A hand-made version of this level, if there is one (folder first, then embedded).
pub fn load(id: LevelId, seed: u64) -> Option<Level> {
    let name = file_name(id);
    let disk = std::fs::read_to_string(dir().join(format!("{name}.json"))).ok();
    let text = disk.as_deref().or_else(|| EMBEDDED.iter().find(|e| e.0 == name).map(|e| e.1))?;
    let parsed: Result<LevelFile, String> = serde_json::from_str(text).map_err(|e| e.to_string());
    match parsed.and_then(|f| from_file(&f, seed)) {
        Ok(lv) if lv.id == id => Some(lv),
        Ok(_) => {
            eprintln!("level {name}.json: its id doesn't match the file name; using the generator");
            None
        }
        Err(e) => {
            eprintln!("level {name}.json: {e}; using the generator");
            None
        }
    }
}

/// Writes every level as JSON (generated, unless a hand-made one already exists).
pub fn export_all(out: &str, seed: u64) -> std::io::Result<usize> {
    std::fs::create_dir_all(out)?;
    let mut n = 0;
    for id in all_ids() {
        let lv = world::build(id, seed);
        let json = serde_json::to_string_pretty(&to_file(&lv)).expect("serialize level");
        std::fs::write(std::path::Path::new(out).join(format!("{}.json", file_name(id))), json)?;
        n += 1;
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_round_trip_through_json() {
        for id in all_ids() {
            let lv = world::generate(id, 7);
            let f = to_file(&lv);
            let text = serde_json::to_string(&f).unwrap();
            let back: LevelFile = serde_json::from_str(&text).unwrap();
            let lv2 = from_file(&back, 7).unwrap();
            assert_eq!(lv2.id, id);
            assert_eq!((lv2.d.w, lv2.d.h), (lv.d.w, lv.d.h));
            assert_eq!(lv2.d.tiles, lv.d.tiles, "{} tiles", file_name(id));
            if id == LevelId::Overworld {
                assert_eq!(lv2.d.ground, lv.d.ground);
            }
            assert_eq!(lv2.portals.len(), lv.portals.len());
            assert_eq!(lv2.mobs.len(), lv.mobs.len());
            assert_eq!(lv2.npcs.len(), lv.npcs.len());
            assert_eq!(lv2.props.len(), lv.props.len(), "{} props", file_name(id));
            assert_eq!(lv2.start, lv.start);
            for (a, b) in lv.mobs.iter().zip(&lv2.mobs) {
                assert_eq!(a.kind, b.kind);
                assert!((a.max_hp - b.max_hp).abs() < 0.01, "{:?} hp {} vs {}", a.kind, a.max_hp, b.max_hp);
            }
        }
    }

    #[test]
    fn names_and_ids_agree() {
        for id in all_ids() {
            assert_eq!(parse_id(&id_string(id)), Some(id));
            assert_eq!(id_from_name(&format!("levels/{}.json", file_name(id))), Some(id));
        }
        assert_eq!(parse_id("dungeon:24:0"), None);
    }

    #[test]
    fn bad_files_are_rejected() {
        let mut f = to_file(&world::generate(LevelId::Dungeon(0, 0), 1));
        f.theme = "lava".into();
        assert!(from_file(&f, 1).is_err());
        let mut f = to_file(&world::generate(LevelId::Dungeon(0, 0), 1));
        f.tiles.pop();
        assert!(from_file(&f, 1).is_err());
    }
}

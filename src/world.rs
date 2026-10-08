//! The world: Act 1 (the Ashlands overworld with Hollowmere and four dungeons) and Act 2
//! (the snowy Frostmarch with Kaldholm and four ice dungeons, see docs/ACT2_PLAN.md).
//! Each map is a `Level`; levels you leave are parked and come back as you left them.
use crate::dungeon::{Dungeon, Room, Tile};
use crate::game::{Decal, Drop, Pickup};
use crate::mobs::{Kind, Mob};
use crate::rng::Rng;
use crate::story::{Npc, Role};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum LevelId {
    /// Act 1's overland: the Ashlands and Hollowmere.
    Overworld,
    /// Act 2's overland: the Frostmarch and Kaldholm.
    Frostmarch,
    /// Act 3's overland: the Mistwood and Mournhold.
    Mistwood,
    /// Act 4's overland: the Grinding Fields of Mechanus and the Last Escapement.
    Mechanus,
    /// Act 5's overland: the Sunken Reach and Brinehollow, on the floor of the black sea.
    Deep,
    /// Act 6's overland: the Skyreach and Windward Anchorage, islands above the clouds.
    Heavens,
    /// (dungeon index into DUNGEONS, floor from 0)
    Dungeon(usize, usize),
    /// An outdoor area of an act (areas.rs): (act, area from 1). The act's town map keeps its own id.
    Area(u8, u8),
    /// An Ash Rift of this tier (endgame.rs).
    Rift(u16),
}

impl LevelId {
    /// An open-air map with a town (one per act).
    pub fn overland(self) -> bool {
        matches!(self, LevelId::Overworld | LevelId::Frostmarch | LevelId::Mistwood | LevelId::Mechanus | LevelId::Deep | LevelId::Heavens | LevelId::Area(..))
    }

    /// An act's town map (the overland with the town; the wild areas around it don't count).
    pub fn town(self) -> bool {
        matches!(self, LevelId::Overworld | LevelId::Frostmarch | LevelId::Mistwood | LevelId::Mechanus | LevelId::Deep | LevelId::Heavens)
    }

    /// 0 for Act 1 ... 4 for Act 5.
    pub fn act(self) -> usize {
        match self {
            LevelId::Overworld => 0,
            LevelId::Frostmarch => 1,
            LevelId::Mistwood => 2,
            LevelId::Mechanus => 3,
            LevelId::Deep => 4,
            LevelId::Heavens => 5,
            LevelId::Dungeon(k, _) => DUNGEONS[k].act,
            LevelId::Area(a, _) => a as usize,
            // The rifts open from Windward Anchorage, and lead back there.
            LevelId::Rift(_) => 5,
        }
    }

    /// The overland (and town) of an act.
    pub fn land(act: usize) -> LevelId {
        match act {
            0 => LevelId::Overworld,
            1 => LevelId::Frostmarch,
            2 => LevelId::Mistwood,
            3 => LevelId::Mechanus,
            4 => LevelId::Deep,
            _ => LevelId::Heavens,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Theme {
    Overworld,
    Crypt,
    Warrens,
    Catacombs,
    Sanctum,
    /// Act 2 overland: snow.
    Tundra,
    Mines,
    IceCaves,
    Rime,
    Glacier,
    /// Act 3 overland: misty forest.
    Mistwood,
    Chapel,
    Gallows,
    Barrow,
    Castle,
    /// Act 4 overland: brass plates over the void.
    Mechanus,
    Foundry,
    Choir,
    Archive,
    Clock,
    /// Act 5 overland: the sea floor under the bubble.
    Deep,
    Wreck,
    Reef,
    Trench,
    Drowned,
    /// Act 6 overland: marble islands over the clouds.
    Heavens,
    Seraph,
    Spire,
    Wheel,
    Zenith,
}

impl Theme {
    pub const ALL: [Theme; 30] = [
        Theme::Overworld,
        Theme::Crypt,
        Theme::Warrens,
        Theme::Catacombs,
        Theme::Sanctum,
        Theme::Tundra,
        Theme::Mines,
        Theme::IceCaves,
        Theme::Rime,
        Theme::Glacier,
        Theme::Mistwood,
        Theme::Chapel,
        Theme::Gallows,
        Theme::Barrow,
        Theme::Castle,
        Theme::Mechanus,
        Theme::Foundry,
        Theme::Choir,
        Theme::Archive,
        Theme::Clock,
        Theme::Deep,
        Theme::Wreck,
        Theme::Reef,
        Theme::Trench,
        Theme::Drowned,
        Theme::Heavens,
        Theme::Seraph,
        Theme::Spire,
        Theme::Wheel,
        Theme::Zenith,
    ];

    /// Open-air (grass or snow ground, palisade walls).
    pub fn open(self) -> bool {
        matches!(self, Theme::Overworld | Theme::Tundra | Theme::Mistwood | Theme::Mechanus | Theme::Deep | Theme::Heavens)
    }

    /// Act 4 themes (drifting steam and brass sparks).
    pub fn clockwork(self) -> bool {
        matches!(self, Theme::Mechanus | Theme::Foundry | Theme::Choir | Theme::Archive | Theme::Clock)
    }

    /// Act 5 themes (rising bubbles and drifting sea snow).
    pub fn drowned(self) -> bool {
        matches!(self, Theme::Deep | Theme::Wreck | Theme::Reef | Theme::Trench | Theme::Drowned)
    }

    /// Act 6 themes (ash drifting upward, sunbeams).
    pub fn sky(self) -> bool {
        matches!(self, Theme::Heavens | Theme::Seraph | Theme::Spire | Theme::Wheel | Theme::Zenith)
    }

    /// Act 3 themes (fog and drifting wisp motes).
    pub fn misty(self) -> bool {
        matches!(self, Theme::Mistwood | Theme::Chapel | Theme::Gallows | Theme::Barrow | Theme::Castle)
    }

    /// Act 2 themes (snowfall outside, frost motes inside).
    pub fn cold(self) -> bool {
        matches!(self, Theme::Tundra | Theme::Mines | Theme::IceCaves | Theme::Rime | Theme::Glacier)
    }

    pub fn index(self) -> usize {
        self as usize
    }

    /// (light radius in pixels, ambient light 0..1)
    pub fn light(self) -> (f32, f32) {
        match self {
            Theme::Overworld => (360.0, 0.5),
            Theme::Tundra => (380.0, 0.56),
            Theme::Sanctum => (230.0, 0.08),
            // Ice catches the light: a little brighter than stone.
            Theme::IceCaves | Theme::Rime => (270.0, 0.16),
            Theme::Glacier => (250.0, 0.12),
            Theme::Mines => (240.0, 0.1),
            // The Mistwood is gloomy even by day.
            Theme::Mistwood => (300.0, 0.3),
            Theme::Castle => (240.0, 0.1),
            Theme::Chapel | Theme::Gallows | Theme::Barrow => (230.0, 0.08),
            // Mechanus: a sooty amber dusk; the foundry glows.
            Theme::Mechanus => (340.0, 0.36),
            Theme::Foundry => (260.0, 0.16),
            Theme::Clock => (260.0, 0.14),
            Theme::Choir | Theme::Archive => (240.0, 0.1),
            // The deep: dim blue-green, lit by the sea's own glow; the trench is the darkest place in the game.
            Theme::Deep => (320.0, 0.3),
            Theme::Wreck => (240.0, 0.1),
            Theme::Reef => (260.0, 0.14),
            Theme::Trench => (190.0, 0.04),
            Theme::Drowned => (250.0, 0.12),
            // The heavens: bright, warm sunset light; the storm tower and the burnt sanctum darker.
            Theme::Heavens => (420.0, 0.62),
            Theme::Seraph => (280.0, 0.2),
            Theme::Spire => (240.0, 0.12),
            Theme::Wheel => (300.0, 0.22),
            Theme::Zenith => (240.0, 0.1),
            _ => (250.0, 0.10),
        }
    }
}

pub struct DungeonDef {
    pub name: &'static str,
    pub floors: usize,
    pub theme: Theme,
    pub boss: Kind,
    pub monsters: &'static [Kind],
    pub tier: f32,
    /// Overland tile in front of the entrance (on its act's overland).
    pub entrance: (i32, i32),
    /// The act, from 0 (Act 1, the Ashlands) to 4 (Act 5, the Drowned Deep).
    pub act: usize,
}

pub const DUNGEONS: [DungeonDef; 26] = [
    DungeonDef {
        name: "THE BONE CRYPT",
        floors: 2,
        theme: Theme::Crypt,
        boss: Kind::BoneWarden,
        monsters: &[Kind::Skeleton, Kind::Zombie, Kind::Archer],
        tier: 1.0,
        entrance: (20, 92),
        act: 0,
    },
    DungeonDef {
        name: "THE ROTTING WARRENS",
        floors: 2,
        theme: Theme::Warrens,
        boss: Kind::PlagueWarden,
        monsters: &[Kind::Zombie, Kind::Goblin, Kind::Wolf],
        tier: 1.6,
        entrance: (94, 92),
        act: 0,
    },
    DungeonDef {
        name: "THE HEXED CATACOMBS",
        floors: 3,
        theme: Theme::Catacombs,
        boss: Kind::HexWarden,
        monsters: &[Kind::Archer, Kind::Skeleton, Kind::Goblin],
        tier: 2.3,
        entrance: (92, 22),
        act: 0,
    },
    DungeonDef {
        name: "THE ASHEN SANCTUM",
        floors: 3,
        theme: Theme::Sanctum,
        boss: Kind::AshKing,
        monsters: &[Kind::Goblin, Kind::Skeleton, Kind::Archer, Kind::Zombie],
        tier: 3.2,
        entrance: (22, 20),
        act: 0,
    },
    DungeonDef {
        name: "THE FROZEN MINES",
        floors: 2,
        theme: Theme::Mines,
        boss: Kind::FrostGiant,
        monsters: &[Kind::Raider, Kind::IceTroll, Kind::Raider],
        tier: 3.8,
        entrance: (18, 78),
        act: 1,
    },
    DungeonDef {
        name: "THE HOWLING CAVES",
        floors: 2,
        theme: Theme::IceCaves,
        boss: Kind::YetiMatriarch,
        monsters: &[Kind::FrostWolf, Kind::Yeti, Kind::FrostWolf],
        tier: 4.2,
        entrance: (96, 30),
        act: 1,
    },
    DungeonDef {
        name: "THE RIME TEMPLE",
        floors: 3,
        theme: Theme::Rime,
        boss: Kind::RimeWitch,
        monsters: &[Kind::IceWraith, Kind::Raider, Kind::IceTroll],
        tier: 4.6,
        entrance: (20, 24),
        act: 1,
    },
    DungeonDef {
        name: "THE GLACIER'S HEART",
        floors: 3,
        theme: Theme::Glacier,
        boss: Kind::WhiteDragon,
        monsters: &[Kind::IceWraith, Kind::Yeti, Kind::IceTroll, Kind::FrostWolf],
        tier: 5.2,
        entrance: (58, 12),
        act: 1,
    },
    DungeonDef {
        name: "THE SUNKEN CHAPEL",
        floors: 2,
        theme: Theme::Chapel,
        boss: Kind::Ossric,
        monsters: &[Kind::Ghoul, Kind::Skeleton, Kind::Cultist],
        tier: 5.8,
        entrance: (20, 82),
        act: 2,
    },
    DungeonDef {
        name: "THE GALLOWS CATACOMBS",
        floors: 2,
        theme: Theme::Gallows,
        boss: Kind::Grimhilde,
        monsters: &[Kind::Banshee, Kind::Cultist, Kind::Skeleton],
        tier: 6.2,
        entrance: (94, 88),
        act: 2,
    },
    DungeonDef {
        name: "THE BARROW OF KNIGHTS",
        floors: 3,
        theme: Theme::Barrow,
        boss: Kind::Malgrave,
        monsters: &[Kind::Ghoul, Kind::Archer, Kind::Werewolf, Kind::Skeleton],
        tier: 6.6,
        entrance: (90, 24),
        act: 2,
    },
    DungeonDef {
        name: "CASTLE VARDAK",
        floors: 3,
        theme: Theme::Castle,
        boss: Kind::Vardak,
        monsters: &[Kind::Werewolf, Kind::Cultist, Kind::Banshee, Kind::Wisp],
        tier: 7.2,
        entrance: (40, 14),
        act: 2,
    },
    DungeonDef {
        name: "THE FOUNDRY OF SOULS",
        floors: 2,
        theme: Theme::Foundry,
        boss: Kind::Forgemother,
        monsters: &[Kind::BoilerBrute, Kind::Scarab, Kind::Ordinal],
        tier: 7.8,
        entrance: (20, 84),
        act: 3,
    },
    DungeonDef {
        name: "THE CHOIR ENGINE",
        floors: 2,
        theme: Theme::Choir,
        boss: Kind::Cantor,
        monsters: &[Kind::Inquisitor, Kind::Gearwraith, Kind::Ordinal, Kind::Prism],
        tier: 8.2,
        entrance: (92, 86),
        act: 3,
    },
    DungeonDef {
        name: "THE ARCHIVE OF GEARS",
        floors: 3,
        theme: Theme::Archive,
        boss: Kind::Archivist,
        monsters: &[Kind::Gearwraith, Kind::SpringJack, Kind::Inquisitor, Kind::ClockCrow],
        tier: 8.6,
        entrance: (90, 24),
        act: 3,
    },
    DungeonDef {
        name: "THE HEART OF THE CLOCK",
        floors: 3,
        theme: Theme::Clock,
        boss: Kind::Clockmaker,
        monsters: &[Kind::SpringJack, Kind::BoilerBrute, Kind::Scarab, Kind::Prism, Kind::Ordinal],
        tier: 9.2,
        entrance: (54, 14),
        act: 3,
    },
    // ---- Act 5: the Drowned Deep (docs/ACT5_ACT6_PLAN.md) ----
    DungeonDef {
        name: "THE WRECK OF THE SOVEREIGN",
        floors: 2,
        theme: Theme::Wreck,
        boss: Kind::Dregmoor,
        monsters: &[Kind::Drowned, Kind::Merrow, Kind::Shellguard],
        tier: 10.2,
        entrance: (20, 84),
        act: 4,
    },
    DungeonDef {
        name: "THE CORAL CATHEDRAL",
        floors: 2,
        theme: Theme::Reef,
        boss: Kind::Nacre,
        monsters: &[Kind::Siren, Kind::Jelly, Kind::Merrow, Kind::Shellguard],
        tier: 10.6,
        entrance: (92, 86),
        act: 4,
    },
    DungeonDef {
        name: "THE MIDNIGHT TRENCH",
        floors: 3,
        theme: Theme::Trench,
        boss: Kind::Angler,
        monsters: &[Kind::Anglerlurk, Kind::InkHorror, Kind::Jelly, Kind::Drowned],
        tier: 11.0,
        entrance: (90, 24),
        act: 4,
    },
    DungeonDef {
        name: "THE DROWNED SANCTUM",
        floors: 3,
        theme: Theme::Drowned,
        boss: Kind::Leviathan,
        monsters: &[Kind::Merrow, Kind::Siren, Kind::InkHorror, Kind::Shellguard, Kind::Anglerlurk],
        tier: 11.6,
        entrance: (54, 14),
        act: 4,
    },
    // ---- Act 6: the Shattered Heavens (docs/ACT5_ACT6_PLAN.md) ----
    DungeonDef {
        name: "THE BROKEN CHOIR",
        floors: 2,
        theme: Theme::Seraph,
        boss: Kind::Vael,
        monsters: &[Kind::FallenSeraph, Kind::Zealot, Kind::Sentinel],
        tier: 12.2,
        entrance: (20, 84),
        act: 5,
    },
    DungeonDef {
        name: "THE STORM SPIRE",
        floors: 2,
        theme: Theme::Spire,
        boss: Kind::Tempest,
        monsters: &[Kind::StormDrake, Kind::Harpy, Kind::Ophanim],
        tier: 12.6,
        entrance: (92, 86),
        act: 5,
    },
    DungeonDef {
        name: "THE WHEEL OF EYES",
        floors: 3,
        theme: Theme::Wheel,
        boss: Kind::OphanPrime,
        monsters: &[Kind::Ophanim, Kind::FallenSeraph, Kind::Zealot, Kind::Sentinel],
        tier: 13.0,
        entrance: (90, 24),
        act: 5,
    },
    DungeonDef {
        name: "THE TRUE SANCTUM",
        floors: 3,
        theme: Theme::Zenith,
        boss: Kind::Solanthos,
        monsters: &[Kind::FallenSeraph, Kind::Zealot, Kind::Sentinel, Kind::StormDrake, Kind::Ophanim],
        tier: 13.6,
        entrance: (54, 14),
        act: 5,
    },
    // ---- optional dungeons (side.rs) ----
    DungeonDef {
        name: "THE CHARNEL WELL",
        floors: 2,
        theme: Theme::Warrens,
        boss: Kind::WellWitch,
        monsters: &[Kind::Zombie, Kind::Skeleton, Kind::Goblin],
        tier: 1.9,
        entrance: (57, 100),
        act: 0,
    },
    DungeonDef {
        name: "THE WYRM'S HOARD",
        floors: 1,
        theme: Theme::Sanctum,
        boss: Kind::FireWyrm,
        monsters: &[Kind::Goblin],
        tier: 2.2,
        entrance: (0, 0),
        act: 0,
    },
];

/// The Ashen Sanctum (needs all three seals).
pub const SANCTUM: usize = 3;
/// The Glacier's Heart (needs all three frost runes).
pub const GLACIER: usize = 7;
/// Castle Vardak (needs the three grave sigils).
pub const CASTLE: usize = 11;
/// The Heart of the Clock (needs the three winding keys).
pub const HEART: usize = 15;
/// The Drowned Sanctum (needs the three Leviathan pearls).
pub const ABYSS: usize = 19;
/// The True Sanctum, Solanthos's (needs the three sun-shards).
pub const ZENITH: usize = 23;
/// Act 1's optional dungeon (side.rs): under the old well south of Hollowmere.
pub const CHARNEL: usize = 24;
/// The Ember Wyrm's cave in Emberpeak Pass (dragon.rs), off the Cinder Hills.
pub const WYRM: usize = 25;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PortalKind {
    /// Overworld door into a dungeon's first floor.
    Entrance(usize),
    /// Stairs up (to the floor above, or out to the overworld from the first floor).
    Up,
    Down,
    /// Opens where a boss dies: straight back to the act's town.
    TownPortal,
    /// The mountain pass between the acts (to this act).
    Pass(usize),
    /// An airship dock: it flies you to the other dock with the same number on this map.
    Dock(u8),
    /// A road off the edge of the map into area `n` of the same act (0 = the town map; areas.rs).
    Exit(u8),
}

pub struct Portal {
    pub x: f32,
    pub y: f32,
    pub kind: PortalKind,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PropKind {
    TreeOak,
    TreePine,
    TreeDead,
    Rock,
    Bush,
    House1,
    House2,
    Stall,
    Campfire,
    Well,
    Entrance(usize),
    /// A shrine (side.rs), in its act's style.
    Shrine(u8),
    StairsDown,
    StairsUp,
    // ---- Act 2 ----
    SnowPine,
    SnowDead,
    SnowRock,
    IceCrystal,
    Longhouse1,
    Longhouse2,
    FurStall,
    /// The mountain pass between the acts.
    Pass,
    // ---- Act 3 ----
    TwistedTree,
    MistPine,
    GlowShrooms,
    Gravestone,
    Cottage1,
    Cottage2,
    Gallows,
    Cart,
    /// The misty road between the Frostmarch and the Mistwood.
    PassMist,
    // ---- Act 4 ----
    GearTower,
    SteamPipes,
    SteamVent,
    GasLamp,
    CogPile,
    Workshop1,
    Workshop2,
    ClockTower,
    Pendulum,
    /// The gear gate between the Mistwood and Mechanus.
    GearGate,
    // ---- Act 5 ----
    Kelp,
    Coral1,
    Coral2,
    WreckHull,
    WhaleBones,
    StiltHouse1,
    StiltHouse2,
    ShellLamp,
    AnchorRock,
    /// The diving bell between Mechanus and the Drowned Deep.
    DivingBell,
    // ---- Act 6 ----
    AngelStatue,
    HaloArch,
    SkyLamp,
    CloudTree,
    MarbleRuin,
    SkyHouse1,
    SkyHouse2,
    AirshipDock,
    /// The stair of light between the Drowned Deep and the Heavens.
    LightStair,
}

impl PropKind {
    pub fn art(self) -> &'static str {
        match self {
            PropKind::TreeOak => "tree_oak",
            PropKind::TreePine => "tree_pine",
            PropKind::TreeDead => "tree_dead",
            PropKind::Rock => "rock1",
            PropKind::Bush => "bush1",
            PropKind::House1 => "house1",
            PropKind::House2 => "house2",
            PropKind::Stall => "tent1",
            PropKind::Campfire => "campfire",
            PropKind::Well => "well",
            PropKind::Entrance(0) => "ent_crypt",
            PropKind::Entrance(1) => "ent_warrens",
            PropKind::Entrance(2) => "ent_catacombs",
            PropKind::Entrance(3) => "ent_sanctum",
            PropKind::Entrance(4) => "ent_mines",
            PropKind::Entrance(5) => "ent_caves",
            PropKind::Entrance(6) => "ent_temple",
            PropKind::Entrance(7) => "ent_glacier",
            PropKind::Entrance(8) => "ent_chapel",
            PropKind::Entrance(9) => "ent_gallows",
            PropKind::Entrance(10) => "ent_barrow",
            PropKind::Entrance(11) => "ent_castle",
            PropKind::Entrance(12) => "ent_foundry",
            PropKind::Entrance(13) => "ent_choir",
            PropKind::Entrance(14) => "ent_archive",
            PropKind::Entrance(15) => "ent_clock",
            PropKind::Entrance(16) => "ent_wreck",
            PropKind::Entrance(17) => "ent_cathedral",
            PropKind::Entrance(18) => "ent_trench",
            PropKind::Entrance(19) => "ent_drowned",
            PropKind::Entrance(20) => "ent_brokenchoir",
            PropKind::Entrance(21) => "ent_spire",
            PropKind::Entrance(22) => "ent_wheel",
            PropKind::Entrance(CHARNEL) => "ent_charnel",
            PropKind::Entrance(WYRM) => "ent_wyrm",
            PropKind::Entrance(_) => "ent_zenith",
            PropKind::Shrine(a) => crate::side::shrine_art(a),
            PropKind::StairsDown => "stairs_down",
            PropKind::StairsUp => "stairs_up",
            PropKind::SnowPine => "tree_snowpine",
            PropKind::SnowDead => "tree_snowdead",
            PropKind::SnowRock => "rock_snow",
            PropKind::IceCrystal => "ice_crystal",
            PropKind::Longhouse1 => "longhouse1",
            PropKind::Longhouse2 => "longhouse2",
            PropKind::FurStall => "stall_furs",
            PropKind::Pass => "pass_gate",
            PropKind::TwistedTree => "tree_twisted",
            PropKind::MistPine => "tree_mistpine",
            PropKind::GlowShrooms => "glow_shrooms",
            PropKind::Gravestone => "gravestone",
            PropKind::Cottage1 => "cottage_mist",
            PropKind::Cottage2 => "cottage_mist2",
            PropKind::Gallows => "gallows",
            PropKind::Cart => "merchant_cart",
            PropKind::PassMist => "pass_mist",
            PropKind::GearTower => "gear_tower",
            PropKind::SteamPipes => "steam_pipes",
            PropKind::SteamVent => "steam_vent",
            PropKind::GasLamp => "gas_lamp",
            PropKind::CogPile => "cog_pile",
            PropKind::Workshop1 => "workshop1",
            PropKind::Workshop2 => "workshop2",
            PropKind::ClockTower => "clock_tower",
            PropKind::Pendulum => "pendulum",
            PropKind::GearGate => "gear_gate",
            PropKind::Kelp => "kelp1",
            PropKind::Coral1 => "coral1",
            PropKind::Coral2 => "coral2",
            PropKind::WreckHull => "wreck_hull",
            PropKind::WhaleBones => "whale_bones",
            PropKind::StiltHouse1 => "stilt_house1",
            PropKind::StiltHouse2 => "stilt_house2",
            PropKind::ShellLamp => "shell_lamp",
            PropKind::AnchorRock => "anchor_rock",
            PropKind::DivingBell => "diving_bell",
            PropKind::AngelStatue => "angel_statue",
            PropKind::HaloArch => "halo_arch",
            PropKind::SkyLamp => "sky_lamp",
            PropKind::CloudTree => "cloud_tree",
            PropKind::MarbleRuin => "marble_ruin",
            PropKind::SkyHouse1 => "sky_house1",
            PropKind::SkyHouse2 => "sky_house2",
            PropKind::AirshipDock => "airship_dock",
            PropKind::LightStair => "light_stair",
        }
    }

    /// Flat props are drawn with the floor (you walk over them), not depth-sorted.
    pub fn flat(self) -> bool {
        matches!(self, PropKind::StairsDown)
    }
}

pub struct Prop {
    /// World point the sprite's bottom centre sits on.
    pub x: f32,
    pub y: f32,
    /// Depth for sorting against actors (x + y of the footprint centre).
    pub depth: f32,
    pub kind: PropKind,
    /// Blocked tiles: x0, y0, width, height.
    pub foot: (i32, i32, i32, i32),
}

impl Prop {
    /// A prop standing on a rectangle of blocked tiles.
    pub fn on(kind: PropKind, x0: i32, y0: i32, fw: i32, fh: i32) -> Self {
        Prop {
            x: (x0 + fw) as f32 - 0.5,
            y: (y0 + fh) as f32 - 0.5,
            depth: x0 as f32 + y0 as f32 + (fw + fh) as f32 * 0.5,
            kind,
            foot: (x0, y0, fw, fh),
        }
    }

    /// Stairs sprite drawn over a portal tile (walkable, not blocking).
    pub fn stairs(kind: PropKind, tx: i32, ty: i32) -> Self {
        let depth = (tx + ty) as f32 + if kind == PropKind::StairsUp { 0.9 } else { 0.0 };
        Prop { x: tx as f32 + 0.5, y: ty as f32 + 0.5, depth, kind, foot: (tx, ty, 0, 0) }
    }
}

pub struct Level {
    pub id: LevelId,
    pub name: String,
    pub theme: Theme,
    pub tier: f32,
    pub d: Dungeon,
    pub mobs: Vec<Mob>,
    pub pickups: Vec<Pickup>,
    pub decals: Vec<Decal>,
    pub explored: Vec<bool>,
    pub props: Vec<Prop>,
    pub portals: Vec<Portal>,
    pub npcs: Vec<Npc>,
    /// Shrines (side.rs); their props are in `props`.
    pub shrines: Vec<crate::side::Shrine>,
    /// Safe zone (town): x0, y0, x1, y1.
    pub safe: Option<(f32, f32, f32, f32)>,
    /// Where you wake up / arrive when there's no matching portal (town square on the overworld).
    pub start: (f32, f32),
}

impl Level {
    fn new(id: LevelId, name: String, theme: Theme, tier: f32, d: Dungeon) -> Self {
        let n = (d.w * d.h) as usize;
        Level {
            id,
            name,
            theme,
            tier,
            d,
            mobs: vec![],
            pickups: vec![],
            decals: vec![],
            explored: vec![false; n],
            props: vec![],
            portals: vec![],
            npcs: vec![],
            shrines: vec![],
            safe: None,
            start: (0.0, 0.0),
        }
    }

    #[cfg(test)]
    pub fn portal(&self, kind: PortalKind) -> Option<&Portal> {
        self.portals.iter().find(|p| p.kind == kind)
    }
}

/// An empty level to fill in (used by the level loader).
pub fn empty_level(id: LevelId, name: String, theme: Theme, tier: f32, d: Dungeon) -> Level {
    Level::new(id, name, theme, tier, d)
}

/// The procedural version of a level.
pub fn generate(id: LevelId, seed: u64) -> Level {
    match id {
        LevelId::Overworld => overworld(seed),
        LevelId::Frostmarch => frostmarch(seed),
        LevelId::Mistwood => mistwood(seed),
        LevelId::Mechanus => mechanus(seed),
        LevelId::Deep => deep(seed),
        LevelId::Heavens => heavens(seed),
        LevelId::Rift(t) => rift(t, seed),
        LevelId::Dungeon(k, f) => dungeon_floor(k, f, seed),
        LevelId::Area(a, n) => crate::areas::area(crate::areas::def(a, n), seed),
    }
}

/// A level as the game plays it: the hand-made file if there is one, else generated.
#[allow(dead_code)] // the tests' way in (the game goes through build_at)
pub fn build(id: LevelId, seed: u64) -> Level {
    build_at(id, seed, 0)
}

/// A level at a difficulty (0 normal, 1 nightmare, 2 hell): tougher monsters, richer drops,
/// and fresh layouts for the generated levels.
pub fn build_at(id: LevelId, seed: u64, difficulty: u8) -> Level {
    let seed = seed.wrapping_add(difficulty as u64 * 7919);
    let mut lv = crate::levels::load(id, seed).unwrap_or_else(|| generate(id, seed));
    add_elites(&mut lv, seed);
    // Shrines, super uniques and lore pages (side.rs).
    crate::side::place(&mut lv, seed);
    // The Ember Wyrm's cavern and hoard (dragon.rs).
    crate::dragon::place(&mut lv, seed, difficulty);
    let (hp, dmg, xp, tier) = match difficulty {
        0 => (1.0, 1.0, 1.0, 1.0),
        1 => (3.5, 2.0, 2.8, 2.0),
        _ => (8.0, 3.2, 5.5, 3.2),
    };
    if difficulty > 0 {
        lv.tier *= tier;
        for m in lv.mobs.iter_mut() {
            m.max_hp *= hp;
            m.hp = m.max_hp;
            m.dmg = (m.dmg.0 * dmg, m.dmg.1 * dmg);
            m.xp *= xp;
            m.tier *= tier;
        }
    }
    // Crates, barrels and urns to smash (after the elites and difficulty, which don't apply to them).
    crate::breakables::place(&mut lv, seed);
    lv
}

/// Promotes some packs, D2 style: blue champion packs (one modifier each) and elite leaders
/// with a name, two modifiers and minions that share one of them. Works on hand-made levels too.
pub fn add_elites(lv: &mut Level, seed: u64) {
    use crate::mobs::{elite_name, roll_mods, Rank};
    let salt = match lv.id {
        LevelId::Overworld => 0x0e11,
        LevelId::Frostmarch => 0x0f11,
        LevelId::Mistwood => 0x1011,
        LevelId::Mechanus => 0x1111,
        LevelId::Deep => 0x1211,
        LevelId::Heavens => 0x1311,
        LevelId::Rift(t) => 0x1411 + t as u64,
        LevelId::Dungeon(k, f) => 0x0e12 + k as u64 * 16 + f as u64,
        LevelId::Area(a, n) => 0x1511 + a as u64 * 16 + n as u64,
    };
    let mut rng = Rng::new(seed ^ salt.wrapping_mul(0x9e37_79b9));
    let (champs, elites) = match lv.id {
        LevelId::Overworld | LevelId::Frostmarch | LevelId::Mistwood | LevelId::Mechanus | LevelId::Deep | LevelId::Heavens => (5, 3),
        LevelId::Dungeon(_, f) => (1 + (f > 0) as usize, 1),
        LevelId::Rift(t) => (2 + t as usize / 4, 1 + t as usize / 8),
        LevelId::Area(..) => (3, 2),
    };
    let mut order: Vec<usize> = (0..lv.mobs.len()).filter(|&i| !lv.mobs[i].boss).collect();
    for i in (1..order.len()).rev() {
        order.swap(i, rng.range(0, i as i32 + 1) as usize);
    }
    let (mut c, mut e) = (0, 0);
    for &i in &order {
        if c >= champs && e >= elites {
            break;
        }
        if lv.mobs[i].rank != Rank::Normal {
            continue;
        }
        let (x, y) = (lv.mobs[i].x, lv.mobs[i].y);
        let pack: Vec<usize> = (0..lv.mobs.len())
            .filter(|&j| j != i && !lv.mobs[j].boss && lv.mobs[j].rank == Rank::Normal && (lv.mobs[j].x - x).powi(2) + (lv.mobs[j].y - y).powi(2) < 12.0)
            .take(4)
            .collect();
        if e < elites {
            let mods = roll_mods(2, &mut rng);
            let shared = crate::mobs::MODS.iter().map(|m| m.0).find(|m| mods & m != 0).unwrap_or(0);
            lv.mobs[i].promote(Rank::Elite, mods, Some(elite_name(&mut rng)));
            for j in pack {
                lv.mobs[j].promote(Rank::Minion, shared, None);
            }
            e += 1;
        } else {
            let mods = roll_mods(1, &mut rng);
            lv.mobs[i].promote(Rank::Champion, mods, None);
            for j in pack.into_iter().take(2) {
                lv.mobs[j].promote(Rank::Champion, mods, None);
            }
            c += 1;
        }
    }
}

/// Where the pass between the acts starts on each overland (the door tile).
pub const PASS_FROST: (i32, i32) = (56, 104);
/// The misty road east out of the Frostmarch, and where it comes out in the Mistwood.
pub const PASS_FROST_EAST: (i32, i32) = (106, 52);
pub const PASS_MIST: (i32, i32) = (6, 56);
/// The gear gate behind Castle Vardak (opens when the Count dies), and where it comes out on Mechanus.
pub const GEAR_GATE: (i32, i32) = (22, 16);
pub const PASS_GEARS: (i32, i32) = (8, 56);
/// The diving bell at the Last Escapement's east road (down to the deep, once the Clockmaker is dead), and
/// where it comes up on the Sunken Reach.
pub const LIFT_GEARS: (i32, i32) = (100, 56);
pub const LIFT_DEEP: (i32, i32) = (8, 56);
/// The stair of light on the Sunken Reach (once the Leviathan is dead), and where it comes out in the Heavens.
pub const STAIR_DEEP: (i32, i32) = (100, 56);
pub const STAIR_SKY: (i32, i32) = (10, 56);
/// Windward Anchorage: the sky-harbour town.
pub const ANCHORAGE: (i32, i32, i32, i32) = (44, 46, 64, 64);
/// Brinehollow: the stilt town on the sea floor.
pub const BRINEHOLLOW: (i32, i32, i32, i32) = (44, 46, 64, 64);
/// The Last Escapement: the refuge town on Mechanus.
pub const ESCAPEMENT: (i32, i32, i32, i32) = (44, 46, 64, 64);
/// Mournhold: palisade rectangle on the Mistwood.
pub const MOURNHOLD: (i32, i32, i32, i32) = (40, 48, 62, 66);
/// Kaldholm: palisade rectangle (inclusive tile bounds) on the Frostmarch.
pub const KALDHOLM: (i32, i32, i32, i32) = (44, 66, 68, 86);
/// The frozen lake beside Kaldholm: centre and radii.
const LAKE: (f32, f32, f32, f32) = (84.0, 70.0, 15.0, 10.0);

pub const WORLD_W: i32 = 112;
pub const WORLD_H: i32 = 112;
/// Hollowmere: palisade rectangle (inclusive tile bounds).
pub const TOWN: (i32, i32, i32, i32) = (44, 50, 68, 70);

pub fn town_center() -> (f32, f32) {
    (56.5, 61.5)
}

/// Smooth value noise in 0..1 from a seeded lattice.
pub(crate) struct Noise {
    grid: Vec<f32>,
    n: i32,
    cell: f32,
}

impl Noise {
    pub(crate) fn new(rng: &mut Rng, n: i32, cell: f32) -> Self {
        Noise { grid: (0..n * n).map(|_| rng.f()).collect(), n, cell }
    }
    pub(crate) fn at(&self, x: f32, y: f32) -> f32 {
        let (gx, gy) = (x / self.cell, y / self.cell);
        let (x0, y0) = (gx.floor() as i32, gy.floor() as i32);
        let (fx, fy) = (gx - x0 as f32, gy - y0 as f32);
        let g = |x: i32, y: i32| self.grid[(y.rem_euclid(self.n) * self.n + x.rem_euclid(self.n)) as usize];
        let s = |t: f32| t * t * (3.0 - 2.0 * t);
        let a = g(x0, y0) + (g(x0 + 1, y0) - g(x0, y0)) * s(fx);
        let b = g(x0, y0 + 1) + (g(x0 + 1, y0 + 1) - g(x0, y0 + 1)) * s(fx);
        a + (b - a) * s(fy)
    }
}

/// Act 1's town map: Hollowmere and the fields around it (the Ashlands themselves are areas, areas.rs).
pub fn overworld(seed: u64) -> Level {
    crate::areas::hollowmere(seed)
}

/// Kaldholm's town square.
pub fn kaldholm_center() -> (f32, f32) {
    (56.5, 75.5)
}

/// Builds Act 2's overland: Kaldholm by a frozen lake, snowy forests, roads to the ice
/// dungeons, the pass back south, raiders, frost wolves and yetis.
pub fn frostmarch(seed: u64) -> Level {
    let mut rng = Rng::new(seed ^ 0x5A0F_7777);
    let (w, h) = (WORLD_W, WORLD_H);
    let mut d = Dungeon::blank(w, h, Tile::Floor);
    for v in d.var.iter_mut() {
        *v = rng.range(0, 100) as u8;
    }
    let mut lv = Level::new(LevelId::Frostmarch, "THE FROSTMARCH".into(), Theme::Tundra, 3.4, Dungeon::blank(1, 1, Tile::Void));
    let mut keep = vec![false; (w * h) as usize];
    let clear = |keep: &mut Vec<bool>, x: i32, y: i32, r: i32| {
        for yy in y - r..=y + r {
            for xx in x - r..=x + r {
                if xx >= 0 && yy >= 0 && xx < w && yy < h {
                    keep[(yy * w + xx) as usize] = true;
                }
            }
        }
    };
    let prop = |lv: &mut Level, d: &mut Dungeon, kind: PropKind, x0: i32, y0: i32, fw: i32, fh: i32| {
        for y in y0..y0 + fh {
            for x in x0..x0 + fw {
                d.set(x, y, Tile::Prop);
            }
        }
        lv.props.push(Prop::on(kind, x0, y0, fw, fh));
    };

    // ---- the frozen lake (walkable ice: ground 1) ----
    let (lx, ly, lrx, lry) = LAKE;
    for y in 0..h {
        for x in 0..w {
            let (dx, dy) = ((x as f32 - lx) / lrx, (y as f32 - ly) / lry);
            let wob = ((x as f32 * 0.4).sin() + (y as f32 * 0.31).cos()) * 0.06;
            if dx * dx + dy * dy < 1.0 + wob {
                d.set_ground(x, y, 1);
                keep[(y * w + x) as usize] = true;
            }
        }
    }

    // ---- Kaldholm ----
    let (tx0, ty0, tx1, ty1) = KALDHOLM;
    let (mx, my) = ((tx0 + tx1) / 2, (ty0 + ty1) / 2);
    for y in ty0..=ty1 {
        for x in tx0..=tx1 {
            let edge = x == tx0 || x == tx1 || y == ty0 || y == ty1;
            let gate = (y == ty0 || y == ty1) && (mx - 1..=mx + 2).contains(&x) || (x == tx0 || x == tx1) && (my - 1..=my + 2).contains(&y);
            if edge && !gate {
                d.set(x, y, Tile::Wall);
            }
        }
    }
    clear(&mut keep, mx, my, 16);
    for x in tx0..=tx1 {
        for y in my..=my + 1 {
            d.set_ground(x, y, 2);
        }
    }
    for y in ty0..=ty1 {
        for x in mx..=mx + 1 {
            d.set_ground(x, y, 2);
        }
    }
    prop(&mut lv, &mut d, PropKind::Longhouse1, 46, 68, 5, 4);
    prop(&mut lv, &mut d, PropKind::Longhouse2, 62, 68, 4, 4);
    prop(&mut lv, &mut d, PropKind::Longhouse2, 46, 80, 4, 4);
    prop(&mut lv, &mut d, PropKind::Longhouse1, 61, 80, 5, 4);
    prop(&mut lv, &mut d, PropKind::FurStall, 59, 72, 3, 2);
    prop(&mut lv, &mut d, PropKind::Campfire, 58, 78, 1, 1);
    lv.safe = Some((tx0 as f32 - 1.0, ty0 as f32 - 1.0, tx1 as f32 + 2.0, ty1 as f32 + 2.0));
    lv.npcs = vec![
        Npc::new("CAPTAIN BRENNA", Role::Captain, "npc_captain", 57.5, 79.0, 6),
        Npc::new("OLD SIGURD", Role::Trader, "npc_trader", 60.5, 74.8, 0),
        Npc::new("MOTHER YLVA", Role::Seer, "npc_seer", 50.5, 74.5, 2),
        Npc::new("FISHERMAN", Role::Fisher(0), "npc_fisher", 53.5, 78.5, 1),
        Npc::new("FISHERMAN", Role::Fisher(1), "npc_fisher", 64.0, 76.5, 5),
        Npc::new("FISHERWIFE", Role::Fisher(2), "npc_fisher", 52.0, 71.5, 3),
        Npc::new("INGRID STONEHAND", Role::Jeweler(1), "npc_jeweler1", 55.5, 74.5, 1),
    ];

    // ---- roads: to each ice dungeon, and south to the pass ----
    let gates = [(mx, ty1 + 1), (tx1 + 1, my), (mx, ty0 - 1), (tx0 - 1, my)];
    let road = |d: &mut Dungeon, keep: &mut Vec<bool>, (gx, gy): (i32, i32), (ex, ey): (i32, i32), salt: f32| {
        let (mut x, mut y) = (gx as f32, gy as f32);
        let mut guard = 0;
        while ((x - ex as f32).abs() > 0.8 || (y - ey as f32).abs() > 0.8) && guard < 400 {
            guard += 1;
            let (dx, dy) = (ex as f32 - x, ey as f32 - y);
            let l = (dx * dx + dy * dy).sqrt();
            let wob = (guard as f32 * 0.19 + salt).sin() * 0.6;
            x += dx / l + (-dy / l) * wob * 0.5;
            y += dy / l + (dx / l) * wob * 0.5;
            for (ox, oy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                let (rx, ry) = (x as i32 + ox, y as i32 + oy);
                if d.get(rx, ry) == Tile::Floor && d.ground_at(rx, ry) != 1 {
                    d.set_ground(rx, ry, 2);
                }
            }
            clear(keep, x as i32, y as i32, 2);
        }
    };
    for (k, def) in DUNGEONS.iter().enumerate().filter(|(_, d)| d.act == 1) {
        let (ex, ey) = def.entrance;
        let &g = gates.iter().min_by_key(|(gx, gy)| (gx - ex).pow(2) + (gy - ey).pow(2)).unwrap();
        road(&mut d, &mut keep, g, (ex, ey), k as f32);
        clear(&mut keep, ex, ey, 5);
        let (fw, fh) = if k == GLACIER { (4, 3) } else { (3, 3) };
        prop(&mut lv, &mut d, PropKind::Entrance(k), ex - fw / 2, ey - 3, fw, fh);
        lv.portals.push(Portal { x: ex as f32 + 0.5, y: ey as f32 + 0.5, kind: PortalKind::Entrance(k) });
    }
    let (px, py) = PASS_FROST;
    road(&mut d, &mut keep, (mx, ty1 + 1), (px, py), 9.0);
    clear(&mut keep, px, py, 4);
    prop(&mut lv, &mut d, PropKind::Pass, px - 1, py + 1, 3, 2);
    lv.portals.push(Portal { x: px as f32 + 0.5, y: py as f32 + 0.5, kind: PortalKind::Pass(0) });
    // The road east into the mists (Act 3, open after the Rime Wyrm).
    let (ex, ey) = PASS_FROST_EAST;
    road(&mut d, &mut keep, (tx1 + 1, my), (ex, ey), 4.0);
    clear(&mut keep, ex, ey, 4);
    prop(&mut lv, &mut d, PropKind::PassMist, ex + 1, ey - 1, 2, 3);
    lv.portals.push(Portal { x: ex as f32 + 0.5, y: ey as f32 + 0.5, kind: PortalKind::Pass(2) });

    // ---- snowy forests, rocks and ice crystals ----
    let forest = Noise::new(&mut rng, 16, 8.0);
    for y in 0..h {
        for x in 0..w {
            if d.get(x, y) != Tile::Floor || keep[(y * w + x) as usize] {
                continue;
            }
            let border = x < 4 || y < 4 || x >= w - 4 || y >= h - 4;
            let f = forest.at(x as f32, y as f32);
            let r = rng.f();
            let tree = if border { r < 0.85 } else { f > 0.56 && r < 0.55 || r < 0.012 };
            if tree {
                let kind = if rng.chance(0.18) { PropKind::SnowDead } else { PropKind::SnowPine };
                prop(&mut lv, &mut d, kind, x, y, 1, 1);
            } else if r < 0.028 {
                prop(&mut lv, &mut d, PropKind::SnowRock, x, y, 1, 1);
            } else if r < 0.036 && f < 0.4 {
                prop(&mut lv, &mut d, PropKind::IceCrystal, x, y, 1, 1);
            }
        }
    }

    // ---- roaming packs and a little food ----
    let (cx, cy) = kaldholm_center();
    let mut packs = 0;
    for _ in 0..600 {
        if packs >= 30 {
            break;
        }
        let x = rng.range(6, w - 6) as f32 + 0.5;
        let y = rng.range(6, h - 6) as f32 + 0.5;
        let far = ((x - cx).powi(2) + (y - cy).powi(2)).sqrt();
        if far < 20.0
            || d.blocked(x, y, 0.4)
            || DUNGEONS.iter().filter(|d| d.act == 1).any(|def| (def.entrance.0 as f32 - x).abs() + (def.entrance.1 as f32 - y).abs() < 6.0)
            || ((x - px as f32).abs() < 6.0 && y > 96.0)
            || ((x - PASS_FROST_EAST.0 as f32).abs() < 7.0 && (y - PASS_FROST_EAST.1 as f32).abs() < 7.0)
        {
            continue;
        }
        let tier = if far < 34.0 { 3.4 } else { 3.8 };
        let kinds: &[Kind] = if far < 34.0 { &[Kind::FrostWolf, Kind::Raider] } else { &[Kind::FrostWolf, Kind::Raider, Kind::Yeti, Kind::IceTroll] };
        let kind = kinds[rng.range(0, kinds.len() as i32) as usize];
        let n = if kind == Kind::Yeti { rng.range(1, 3) } else { rng.range(3, 6) };
        for _ in 0..n {
            for _try in 0..10 {
                let (mx, my) = (x + rng.rf(-2.0, 2.0), y + rng.rf(-2.0, 2.0));
                if !d.blocked(mx, my, 0.35) {
                    lv.mobs.push(Mob::new(kind, mx, my, tier, &mut rng));
                    break;
                }
            }
        }
        packs += 1;
    }
    let mut food = 0;
    for _ in 0..400 {
        if food >= 10 {
            break;
        }
        let x = rng.range(6, w - 6) as f32 + 0.5;
        let y = rng.range(6, h - 6) as f32 + 0.5;
        if ((x - cx).powi(2) + (y - cy).powi(2)).sqrt() > 16.0 && !d.blocked(x, y, 0.3) {
            lv.pickups.push(Pickup { x, y, kind: Drop::Food(if rng.chance(0.6) { 1 } else { 2 }), t: 1.0 });
            food += 1;
        }
    }
    lv.explored = vec![false; (w * h) as usize];
    lv.d = d;
    lv.start = kaldholm_center();
    lv
}

/// Mournhold's square.
pub fn mournhold_center() -> (f32, f32) {
    (51.5, 57.5)
}

/// Builds Act 3's overland: the frightened village of Mournhold in a misty, glowing forest,
/// roads to the skeleton lords' lairs and Castle Vardak, the road back west to Kaldholm.
pub fn mistwood(seed: u64) -> Level {
    let mut rng = Rng::new(seed ^ 0x3A57_1D55);
    let (w, h) = (WORLD_W, WORLD_H);
    let mut d = Dungeon::blank(w, h, Tile::Floor);
    for v in d.var.iter_mut() {
        *v = rng.range(0, 100) as u8;
    }
    let mut lv = Level::new(LevelId::Mistwood, "THE MISTWOOD".into(), Theme::Mistwood, 5.2, Dungeon::blank(1, 1, Tile::Void));
    let mut keep = vec![false; (w * h) as usize];
    let clear = |keep: &mut Vec<bool>, x: i32, y: i32, r: i32| {
        for yy in y - r..=y + r {
            for xx in x - r..=x + r {
                if xx >= 0 && yy >= 0 && xx < w && yy < h {
                    keep[(yy * w + xx) as usize] = true;
                }
            }
        }
    };
    let prop = |lv: &mut Level, d: &mut Dungeon, kind: PropKind, x0: i32, y0: i32, fw: i32, fh: i32| {
        for y in y0..y0 + fh {
            for x in x0..x0 + fw {
                d.set(x, y, Tile::Prop);
            }
        }
        lv.props.push(Prop::on(kind, x0, y0, fw, fh));
    };

    // ---- glowing moss (ground 1) in patches ----
    let moss = Noise::new(&mut rng, 14, 7.0);
    for y in 0..h {
        for x in 0..w {
            if moss.at(x as f32, y as f32) > 0.68 {
                d.set_ground(x, y, 1);
            }
        }
    }

    // ---- Mournhold ----
    let (tx0, ty0, tx1, ty1) = MOURNHOLD;
    let (mx, my) = ((tx0 + tx1) / 2, (ty0 + ty1) / 2);
    for y in ty0..=ty1 {
        for x in tx0..=tx1 {
            d.set_ground(x, y, 0);
            let edge = x == tx0 || x == tx1 || y == ty0 || y == ty1;
            let gate = (y == ty0 || y == ty1) && (mx - 1..=mx + 2).contains(&x) || (x == tx0 || x == tx1) && (my - 1..=my + 2).contains(&y);
            if edge && !gate {
                d.set(x, y, Tile::Wall);
            }
        }
    }
    clear(&mut keep, mx, my, 14);
    for x in tx0..=tx1 {
        for y in my..=my + 1 {
            d.set_ground(x, y, 2);
        }
    }
    for y in ty0..=ty1 {
        for x in mx..=mx + 1 {
            d.set_ground(x, y, 2);
        }
    }
    prop(&mut lv, &mut d, PropKind::Cottage1, 42, 50, 4, 4);
    prop(&mut lv, &mut d, PropKind::Cottage2, 56, 50, 4, 4);
    prop(&mut lv, &mut d, PropKind::Cottage1, 42, 61, 4, 3);
    prop(&mut lv, &mut d, PropKind::Cottage2, 57, 61, 3, 4);
    prop(&mut lv, &mut d, PropKind::Cart, 54, 55, 3, 2);
    prop(&mut lv, &mut d, PropKind::Gallows, 47, 55, 1, 1);
    prop(&mut lv, &mut d, PropKind::Campfire, 51, 60, 1, 1);
    lv.safe = Some((tx0 as f32 - 1.0, ty0 as f32 - 1.0, tx1 as f32 + 2.0, ty1 as f32 + 2.0));
    lv.npcs = vec![
        Npc::new("ABELARD", Role::Hunter, "npc_hunter", 52.5, 58.8, 6),
        Npc::new("WIDOW KASIA", Role::Widow, "npc_widow", 55.0, 57.8, 0),
        Npc::new("FATHER LUCIAN", Role::Priest, "npc_priest", 46.5, 57.5, 2),
        Npc::new("PEASANT", Role::Peasant(0), "npc_peasant", 49.0, 62.0, 1),
        Npc::new("PEASANT", Role::Peasant(1), "npc_peasant", 58.5, 59.0, 5),
        Npc::new("PEASANT", Role::Peasant(2), "npc_peasant", 45.5, 54.0, 3),
        Npc::new("SILAS GREAVE", Role::Jeweler(2), "npc_jeweler2", 55.0, 60.0, 1),
    ];

    // ---- roads ----
    let gates = [(mx, ty1 + 1), (tx1 + 1, my), (mx, ty0 - 1), (tx0 - 1, my)];
    let road = |d: &mut Dungeon, keep: &mut Vec<bool>, (gx, gy): (i32, i32), (ex, ey): (i32, i32), salt: f32| {
        let (mut x, mut y) = (gx as f32, gy as f32);
        let mut guard = 0;
        while ((x - ex as f32).abs() > 0.8 || (y - ey as f32).abs() > 0.8) && guard < 400 {
            guard += 1;
            let (dx, dy) = (ex as f32 - x, ey as f32 - y);
            let l = (dx * dx + dy * dy).sqrt();
            let wob = (guard as f32 * 0.23 + salt).sin() * 0.7;
            x += dx / l + (-dy / l) * wob * 0.5;
            y += dy / l + (dx / l) * wob * 0.5;
            for (ox, oy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                let (rx, ry) = (x as i32 + ox, y as i32 + oy);
                if d.get(rx, ry) == Tile::Floor {
                    d.set_ground(rx, ry, 2);
                }
            }
            clear(keep, x as i32, y as i32, 2);
        }
    };
    for (k, def) in DUNGEONS.iter().enumerate().filter(|(_, d)| d.act == 2) {
        let (ex, ey) = def.entrance;
        let &g = gates.iter().min_by_key(|(gx, gy)| (gx - ex).pow(2) + (gy - ey).pow(2)).unwrap();
        road(&mut d, &mut keep, g, (ex, ey), k as f32);
        clear(&mut keep, ex, ey, 5);
        let (fw, fh) = if k == CASTLE { (4, 3) } else { (3, 3) };
        prop(&mut lv, &mut d, PropKind::Entrance(k), ex - fw / 2, ey - 3, fw, fh);
        lv.portals.push(Portal { x: ex as f32 + 0.5, y: ey as f32 + 0.5, kind: PortalKind::Entrance(k) });
    }
    let (px, py) = PASS_MIST;
    road(&mut d, &mut keep, (tx0 - 1, my), (px, py), 7.0);
    clear(&mut keep, px, py, 4);
    prop(&mut lv, &mut d, PropKind::PassMist, px - 3, py - 1, 2, 3);
    lv.portals.push(Portal { x: px as f32 + 0.5, y: py as f32 + 0.5, kind: PortalKind::Pass(1) });
    // The gear gate on the castle grounds (to Mechanus, once the Count is dead).
    let (gx, gy) = GEAR_GATE;
    road(&mut d, &mut keep, (mx, ty0 - 1), (gx, gy), 11.0);
    clear(&mut keep, gx, gy, 4);
    prop(&mut lv, &mut d, PropKind::GearGate, gx - 1, gy - 3, 3, 2);
    lv.portals.push(Portal { x: gx as f32 + 0.5, y: gy as f32 + 0.5, kind: PortalKind::Pass(3) });

    // ---- the haunted forest: twisted trees, gravestones, glowing mushrooms ----
    let forest = Noise::new(&mut rng, 16, 8.0);
    for y in 0..h {
        for x in 0..w {
            if d.get(x, y) != Tile::Floor || keep[(y * w + x) as usize] {
                continue;
            }
            let border = x < 4 || y < 4 || x >= w - 4 || y >= h - 4;
            let f = forest.at(x as f32, y as f32);
            let r = rng.f();
            let tree = if border { r < 0.85 } else { f > 0.55 && r < 0.55 || r < 0.012 };
            if tree {
                let kind = if rng.chance(0.55) { PropKind::TwistedTree } else { PropKind::MistPine };
                prop(&mut lv, &mut d, kind, x, y, 1, 1);
            } else if r < 0.02 {
                prop(&mut lv, &mut d, PropKind::Gravestone, x, y, 1, 1);
            } else if r < 0.035 && d.ground_at(x, y) == 1 {
                prop(&mut lv, &mut d, PropKind::GlowShrooms, x, y, 1, 1);
            }
        }
    }

    // ---- roaming packs and a little food ----
    let (cx, cy) = mournhold_center();
    let mut packs = 0;
    for _ in 0..600 {
        if packs >= 30 {
            break;
        }
        let x = rng.range(6, w - 6) as f32 + 0.5;
        let y = rng.range(6, h - 6) as f32 + 0.5;
        let far = ((x - cx).powi(2) + (y - cy).powi(2)).sqrt();
        if far < 20.0
            || d.blocked(x, y, 0.4)
            || DUNGEONS.iter().filter(|d| d.act == 2).any(|def| (def.entrance.0 as f32 - x).abs() + (def.entrance.1 as f32 - y).abs() < 6.0)
            || ((x - px as f32).abs() < 7.0 && (y - py as f32).abs() < 7.0)
            || ((x - GEAR_GATE.0 as f32).abs() < 7.0 && (y - GEAR_GATE.1 as f32).abs() < 7.0)
        {
            continue;
        }
        let tier = if far < 34.0 { 5.2 } else { 5.7 };
        let kinds: &[Kind] = if far < 34.0 { &[Kind::Ghoul, Kind::Wisp, Kind::Wolf] } else { &[Kind::Ghoul, Kind::Werewolf, Kind::Cultist, Kind::Banshee, Kind::Wisp] };
        let kind = kinds[rng.range(0, kinds.len() as i32) as usize];
        let n = if kind == Kind::Werewolf { rng.range(1, 3) } else { rng.range(3, 6) };
        for _ in 0..n {
            for _try in 0..10 {
                let (mx, my) = (x + rng.rf(-2.0, 2.0), y + rng.rf(-2.0, 2.0));
                if !d.blocked(mx, my, 0.35) {
                    lv.mobs.push(Mob::new(kind, mx, my, tier, &mut rng));
                    break;
                }
            }
        }
        packs += 1;
    }
    let mut food = 0;
    for _ in 0..400 {
        if food >= 10 {
            break;
        }
        let x = rng.range(6, w - 6) as f32 + 0.5;
        let y = rng.range(6, h - 6) as f32 + 0.5;
        if ((x - cx).powi(2) + (y - cy).powi(2)).sqrt() > 16.0 && !d.blocked(x, y, 0.3) {
            lv.pickups.push(Pickup { x, y, kind: Drop::Food(if rng.chance(0.5) { 1 } else { 2 }), t: 1.0 });
            food += 1;
        }
    }
    lv.explored = vec![false; (w * h) as usize];
    lv.d = d;
    lv.start = mournhold_center();
    lv
}

/// The Last Escapement's square.
pub fn escapement_center() -> (f32, f32) {
    (54.5, 55.5)
}

/// Builds Act 4's overland: the Grinding Fields of Mechanus, an island of brass plates over the
/// void, with the refuge town of the Last Escapement, roads to the heralds' works and the
/// great clock, and the gear gate back to the Mistwood.
pub fn mechanus(seed: u64) -> Level {
    let mut rng = Rng::new(seed ^ 0x4C0C_C10C);
    let (w, h) = (WORLD_W, WORLD_H);
    let mut d = Dungeon::blank(w, h, Tile::Floor);
    for v in d.var.iter_mut() {
        *v = rng.range(0, 100) as u8;
    }
    let mut lv = Level::new(LevelId::Mechanus, "THE GRINDING FIELDS".into(), Theme::Mechanus, 7.2, Dungeon::blank(1, 1, Tile::Void));
    let mut keep = vec![false; (w * h) as usize];
    let clear = |keep: &mut Vec<bool>, x: i32, y: i32, r: i32| {
        for yy in y - r..=y + r {
            for xx in x - r..=x + r {
                if xx >= 0 && yy >= 0 && xx < w && yy < h {
                    keep[(yy * w + xx) as usize] = true;
                }
            }
        }
    };
    let prop = |lv: &mut Level, d: &mut Dungeon, kind: PropKind, x0: i32, y0: i32, fw: i32, fh: i32| {
        for y in y0..y0 + fh {
            for x in x0..x0 + fw {
                d.set(x, y, Tile::Prop);
            }
        }
        lv.props.push(Prop::on(kind, x0, y0, fw, fh));
    };

    // ---- verdigris copper (ground 1) in patches ----
    let green = Noise::new(&mut rng, 14, 7.0);
    for y in 0..h {
        for x in 0..w {
            if green.at(x as f32, y as f32) > 0.66 {
                d.set_ground(x, y, 1);
            }
        }
    }

    // ---- the Last Escapement ----
    let (tx0, ty0, tx1, ty1) = ESCAPEMENT;
    let (mx, my) = ((tx0 + tx1) / 2, (ty0 + ty1) / 2);
    for y in ty0..=ty1 {
        for x in tx0..=tx1 {
            d.set_ground(x, y, 0);
            let edge = x == tx0 || x == tx1 || y == ty0 || y == ty1;
            let gate = (y == ty0 || y == ty1) && (mx - 1..=mx + 2).contains(&x) || (x == tx0 || x == tx1) && (my - 1..=my + 2).contains(&y);
            if edge && !gate {
                d.set(x, y, Tile::Wall);
            }
        }
    }
    clear(&mut keep, mx, my, 14);
    for x in tx0..=tx1 {
        for y in my..=my + 1 {
            d.set_ground(x, y, 2);
        }
    }
    for y in ty0..=ty1 {
        for x in mx..=mx + 1 {
            d.set_ground(x, y, 2);
        }
    }
    prop(&mut lv, &mut d, PropKind::Workshop1, 46, 48, 4, 4);
    prop(&mut lv, &mut d, PropKind::Workshop2, 58, 48, 4, 4);
    prop(&mut lv, &mut d, PropKind::Workshop1, 46, 59, 4, 3);
    prop(&mut lv, &mut d, PropKind::ClockTower, 59, 59, 2, 2);
    prop(&mut lv, &mut d, PropKind::GasLamp, 52, 53, 1, 1);
    prop(&mut lv, &mut d, PropKind::GasLamp, 57, 58, 1, 1);
    prop(&mut lv, &mut d, PropKind::SteamPipes, 62, 54, 1, 2);
    lv.safe = Some((tx0 as f32 - 1.0, ty0 as f32 - 1.0, tx1 as f32 + 2.0, ty1 as f32 + 2.0));
    lv.npcs = vec![
        Npc::new("TALLY", Role::Tally, "npc_tally", 55.5, 56.8, 6),
        Npc::new("MADAME VESPER", Role::Vesper, "npc_vesper", 58.0, 55.8, 0),
        Npc::new("BROTHER PISTON", Role::Oiler, "npc_oiler", 50.5, 55.5, 2),
        Npc::new("SERVANT", Role::Servant(0), "npc_servant", 52.0, 60.0, 1),
        Npc::new("SERVANT", Role::Servant(1), "npc_servant", 61.5, 57.0, 5),
        Npc::new("SERVANT", Role::Servant(2), "npc_servant", 49.5, 52.0, 3),
        Npc::new("THE LAPIDARY", Role::Jeweler(3), "npc_jeweler3", 54.0, 58.5, 1),
    ];

    // ---- conveyor roads ----
    let gates = [(mx, ty1 + 1), (tx1 + 1, my), (mx, ty0 - 1), (tx0 - 1, my)];
    let road = |d: &mut Dungeon, keep: &mut Vec<bool>, (gx, gy): (i32, i32), (ex, ey): (i32, i32), salt: f32| {
        let (mut x, mut y) = (gx as f32, gy as f32);
        let mut guard = 0;
        while ((x - ex as f32).abs() > 0.8 || (y - ey as f32).abs() > 0.8) && guard < 400 {
            guard += 1;
            let (dx, dy) = (ex as f32 - x, ey as f32 - y);
            let l = (dx * dx + dy * dy).sqrt();
            // Machined roads: straighter than forest paths.
            let wob = (guard as f32 * 0.17 + salt).sin() * 0.35;
            x += dx / l + (-dy / l) * wob * 0.5;
            y += dy / l + (dx / l) * wob * 0.5;
            for (ox, oy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                let (rx, ry) = (x as i32 + ox, y as i32 + oy);
                if d.get(rx, ry) == Tile::Floor {
                    d.set_ground(rx, ry, 2);
                }
            }
            clear(keep, x as i32, y as i32, 2);
        }
    };
    for (k, def) in DUNGEONS.iter().enumerate().filter(|(_, d)| d.act == 3) {
        let (ex, ey) = def.entrance;
        let &g = gates.iter().min_by_key(|(gx, gy)| (gx - ex).pow(2) + (gy - ey).pow(2)).unwrap();
        road(&mut d, &mut keep, g, (ex, ey), k as f32);
        clear(&mut keep, ex, ey, 5);
        let (fw, fh) = if k == HEART { (4, 3) } else { (3, 3) };
        prop(&mut lv, &mut d, PropKind::Entrance(k), ex - fw / 2, ey - 3, fw, fh);
        lv.portals.push(Portal { x: ex as f32 + 0.5, y: ey as f32 + 0.5, kind: PortalKind::Entrance(k) });
    }
    let (px, py) = PASS_GEARS;
    road(&mut d, &mut keep, (tx0 - 1, my), (px, py), 7.0);
    clear(&mut keep, px, py, 4);
    prop(&mut lv, &mut d, PropKind::GearGate, px - 1, py - 3, 3, 2);
    lv.portals.push(Portal { x: px as f32 + 0.5, y: py as f32 + 0.5, kind: PortalKind::Pass(2) });
    // The diving bell down to the Drowned Deep (once the Clockmaker is dead).
    let (lx, ly) = LIFT_GEARS;
    road(&mut d, &mut keep, (tx1 + 1, my), (lx, ly), 9.0);
    clear(&mut keep, lx, ly, 4);
    prop(&mut lv, &mut d, PropKind::DivingBell, lx - 1, ly - 3, 3, 2);
    lv.portals.push(Portal { x: lx as f32 + 0.5, y: ly as f32 + 0.5, kind: PortalKind::Pass(4) });

    // ---- the void: the island's ragged rim and a few chasms (never across a road) ----
    let rim = Noise::new(&mut rng, 12, 6.0);
    let pits = Noise::new(&mut rng, 18, 9.0);
    for y in 0..h {
        for x in 0..w {
            if keep[(y * w + x) as usize] || d.get(x, y) != Tile::Floor {
                continue;
            }
            let edge = x.min(y).min(w - 1 - x).min(h - 1 - y) as f32;
            let ragged = edge < 3.0 + rim.at(x as f32, y as f32) * 4.0;
            if ragged || pits.at(x as f32, y as f32) > 0.78 {
                d.set(x, y, Tile::Void);
            }
        }
    }

    // ---- the works: gear towers, pipes, vents, cog piles, pendulums ----
    let works = Noise::new(&mut rng, 16, 8.0);
    for y in 0..h {
        for x in 0..w {
            if d.get(x, y) != Tile::Floor || keep[(y * w + x) as usize] {
                continue;
            }
            let f = works.at(x as f32, y as f32);
            let r = rng.f();
            if f > 0.6 && r < 0.3 || r < 0.01 {
                let kind = match rng.range(0, 10) {
                    0..=3 => PropKind::GearTower,
                    4..=6 => PropKind::SteamPipes,
                    7 => PropKind::Pendulum,
                    _ => PropKind::CogPile,
                };
                prop(&mut lv, &mut d, kind, x, y, 1, 1);
            } else if r < 0.018 {
                prop(&mut lv, &mut d, PropKind::SteamVent, x, y, 1, 1);
            } else if r < 0.024 {
                prop(&mut lv, &mut d, PropKind::GasLamp, x, y, 1, 1);
            }
        }
    }

    // ---- roaming packs and a little food ----
    let (cx, cy) = escapement_center();
    let mut packs = 0;
    for _ in 0..800 {
        if packs >= 30 {
            break;
        }
        let x = rng.range(6, w - 6) as f32 + 0.5;
        let y = rng.range(6, h - 6) as f32 + 0.5;
        let far = ((x - cx).powi(2) + (y - cy).powi(2)).sqrt();
        if far < 20.0
            || d.blocked(x, y, 0.4)
            || DUNGEONS.iter().filter(|d| d.act == 3).any(|def| (def.entrance.0 as f32 - x).abs() + (def.entrance.1 as f32 - y).abs() < 6.0)
            || ((x - px as f32).abs() < 7.0 && (y - py as f32).abs() < 7.0)
            || ((x - lx as f32).abs() < 7.0 && (y - ly as f32).abs() < 7.0)
        {
            continue;
        }
        let tier = if far < 34.0 { 7.2 } else { 7.7 };
        let kinds: &[Kind] = if far < 34.0 {
            &[Kind::Scarab, Kind::Ordinal, Kind::Inquisitor, Kind::ClockCrow]
        } else {
            &[Kind::Scarab, Kind::SpringJack, Kind::Inquisitor, Kind::BoilerBrute, Kind::Ordinal, Kind::ClockCrow]
        };
        let kind = kinds[rng.range(0, kinds.len() as i32) as usize];
        if kind == Kind::Ordinal {
            // An ordinal squad: the marshal in front, prisms on its flanks, cubits in ranks behind.
            let mut squad = vec![(Kind::Marshal, 0.0, 0.0), (Kind::Prism, -1.2, 0.8), (Kind::Prism, 1.2, 0.8)];
            for k in 0..rng.range(3, 5) {
                squad.push((Kind::Ordinal, -1.2 + (k % 3) as f32 * 1.2, 1.8 + (k / 3) as f32 * 1.0));
            }
            for (kind, dx, dy) in squad {
                let (sx, sy) = (x + dx, y + dy);
                if !d.blocked(sx, sy, 0.35) {
                    let mut m = Mob::new(kind, sx, sy, tier, &mut rng);
                    // Its place in the ranks (across, behind), kept as the squad turns to face you.
                    m.post = (dx, dy);
                    lv.mobs.push(m);
                }
            }
            packs += 1;
            continue;
        }
        let n = if kind == Kind::BoilerBrute {
            rng.range(1, 3)
        } else if kind == Kind::Scarab {
            rng.range(4, 7)
        } else if kind == Kind::ClockCrow {
            rng.range(6, 10)
        } else {
            rng.range(3, 6)
        };
        for _ in 0..n {
            for _try in 0..10 {
                let (mx, my) = (x + rng.rf(-2.0, 2.0), y + rng.rf(-2.0, 2.0));
                if !d.blocked(mx, my, 0.35) {
                    lv.mobs.push(Mob::new(kind, mx, my, tier, &mut rng));
                    break;
                }
            }
        }
        packs += 1;
    }
    let mut food = 0;
    for _ in 0..400 {
        if food >= 10 {
            break;
        }
        let x = rng.range(6, w - 6) as f32 + 0.5;
        let y = rng.range(6, h - 6) as f32 + 0.5;
        if ((x - cx).powi(2) + (y - cy).powi(2)).sqrt() > 16.0 && !d.blocked(x, y, 0.3) {
            lv.pickups.push(Pickup { x, y, kind: Drop::Food(if rng.chance(0.5) { 1 } else { 2 }), t: 1.0 });
            food += 1;
        }
    }
    lv.explored = vec![false; (w * h) as usize];
    lv.d = d;
    lv.start = escapement_center();
    lv
}

/// Windward Anchorage's square.
pub fn anchorage_center() -> (f32, f32) {
    (54.5, 55.5)
}

/// The far island (reached only by airship) and the two docks that join it to the town's island.
pub const DOCK_TOWN: (i32, i32) = (66, 72);
pub const FAR_ISLE: (i32, i32) = (96, 56);
pub const DOCK_FAR: (i32, i32) = (92, 60);

/// Builds Act 6's overland: the Skyreach, islands of marble and pale grass above an endless sea of
/// cloud (the void). Windward Anchorage stands on the great central island; chain bridges run out to
/// the four sky dungeons and the stair of light, and an airship flies to a far island. The edges are
/// real: whatever is knocked or blown off falls (sky.rs).
pub fn heavens(seed: u64) -> Level {
    let mut rng = Rng::new(seed ^ 0x5C1E_A7E5);
    let (w, h) = (WORLD_W, WORLD_H);
    let mut d = Dungeon::blank(w, h, Tile::Void);
    for v in d.var.iter_mut() {
        *v = rng.range(0, 100) as u8;
    }
    let mut lv = Level::new(LevelId::Heavens, "THE SKYREACH".into(), Theme::Heavens, 12.0, Dungeon::blank(1, 1, Tile::Void));
    let prop = |lv: &mut Level, d: &mut Dungeon, kind: PropKind, x0: i32, y0: i32, fw: i32, fh: i32| {
        for y in y0..y0 + fh {
            for x in x0..x0 + fw {
                d.set(x, y, Tile::Prop);
            }
        }
        lv.props.push(Prop::on(kind, x0, y0, fw, fh));
    };
    // ---- the islands: noisy discs of floor (ground 0 marble, ground 1 pale grass) ----
    let edge = Noise::new(&mut rng, 12, 5.0);
    let grass = Noise::new(&mut rng, 14, 7.0);
    let mut isles: Vec<(i32, i32, f32)> = vec![(54, 55, 17.0), STAIR_SKY_ISLE, FAR_ISLE_R];
    for def in DUNGEONS.iter().filter(|d| d.act == 5) {
        isles.push((def.entrance.0, def.entrance.1 - 1, 9.0));
    }
    // A few wild islands for the packs.
    for &(x, y) in &[(30, 30), (78, 34), (32, 66), (74, 92), (50, 92)] {
        isles.push((x, y, rng.rf(6.0, 8.5)));
    }
    for &(cx, cy, r) in &isles {
        for y in (cy as f32 - r - 4.0) as i32..=(cy as f32 + r + 4.0) as i32 {
            for x in (cx as f32 - r - 4.0) as i32..=(cx as f32 + r + 4.0) as i32 {
                if x < 2 || y < 2 || x >= w - 2 || y >= h - 2 {
                    continue;
                }
                let dist = ((x - cx) as f32).hypot((y - cy) as f32);
                if dist < r - 2.0 + edge.at(x as f32, y as f32) * 4.0 {
                    d.set(x, y, Tile::Floor);
                    d.set_ground(x, y, if grass.at(x as f32, y as f32) > 0.55 { 1 } else { 0 });
                }
            }
        }
    }
    // ---- chain bridges (ground 2) from the town island to the others, straight and two wide ----
    let mut keep = vec![false; (w * h) as usize];
    let bridge = |d: &mut Dungeon, keep: &mut Vec<bool>, (ax, ay): (i32, i32), (bx, by): (i32, i32)| {
        let n = ((bx - ax).abs().max((by - ay).abs()) * 2).max(1);
        for k in 0..=n {
            let t = k as f32 / n as f32;
            let (x, y) = ((ax as f32 + (bx - ax) as f32 * t) as i32, (ay as f32 + (by - ay) as f32 * t) as i32);
            for (ox, oy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                d.set(x + ox, y + oy, Tile::Floor);
                d.set_ground(x + ox, y + oy, 2);
                if (0..w).contains(&(x + ox)) && (0..h).contains(&(y + oy)) {
                    keep[((y + oy) * w + x + ox) as usize] = true;
                }
            }
        }
    };
    let hub = (55, 55);
    for &(cx, cy, _) in isles.iter().skip(1) {
        if (cx, cy) == (FAR_ISLE.0, FAR_ISLE.1) {
            continue; // the far island: by airship only
        }
        bridge(&mut d, &mut keep, hub, (cx, cy));
    }
    // ---- Windward Anchorage ----
    let (tx0, ty0, tx1, ty1) = ANCHORAGE;
    let (mx, my) = ((tx0 + tx1) / 2, (ty0 + ty1) / 2);
    for y in ty0..=ty1 {
        for x in tx0..=tx1 {
            d.set(x, y, Tile::Floor);
            d.set_ground(x, y, 0);
            let edge = x == tx0 || x == tx1 || y == ty0 || y == ty1;
            let gate = (y == ty0 || y == ty1) && (mx - 1..=mx + 2).contains(&x) || (x == tx0 || x == tx1) && (my - 1..=my + 2).contains(&y);
            if edge && !gate {
                d.set(x, y, Tile::Wall);
            }
        }
    }
    for x in tx0..=tx1 {
        for y in my..=my + 1 {
            d.set_ground(x, y, 2);
        }
    }
    for y in ty0..=ty1 {
        for x in mx..=mx + 1 {
            d.set_ground(x, y, 2);
        }
    }
    prop(&mut lv, &mut d, PropKind::SkyHouse1, 46, 48, 4, 4);
    prop(&mut lv, &mut d, PropKind::SkyHouse2, 58, 48, 4, 4);
    prop(&mut lv, &mut d, PropKind::SkyHouse1, 46, 59, 4, 3);
    prop(&mut lv, &mut d, PropKind::HaloArch, 59, 59, 2, 2);
    prop(&mut lv, &mut d, PropKind::SkyLamp, 52, 53, 1, 1);
    prop(&mut lv, &mut d, PropKind::SkyLamp, 57, 58, 1, 1);
    lv.safe = Some((tx0 as f32 - 1.0, ty0 as f32 - 1.0, tx1 as f32 + 2.0, ty1 as f32 + 2.0));
    lv.npcs = vec![
        Npc::new("SERAPHINE", Role::Seraphine, "npc_seraphine", 55.5, 56.8, 6),
        Npc::new("QUARTERMASTER BRAM", Role::Bram, "npc_bram", 58.0, 55.8, 0),
        Npc::new("SISTER AUREL", Role::Aurel, "npc_aurel", 50.5, 55.5, 2),
        Npc::new("DECKHAND", Role::Deckhand(0), "npc_deckhand", 52.0, 60.0, 1),
        Npc::new("DECKHAND", Role::Deckhand(1), "npc_deckhand", 61.5, 57.0, 5),
        Npc::new("DECKHAND", Role::Deckhand(2), "npc_deckhand", 49.5, 52.0, 3),
        Npc::new("THE GILDER", Role::Jeweler(5), "npc_jeweler5", 54.0, 58.5, 1),
    ];
    // Clear around the town so nothing grows across its gates.
    for y in ty0 - 3..=ty1 + 3 {
        for x in tx0 - 3..=tx1 + 3 {
            keep[(y * w + x) as usize] = true;
        }
    }
    // ---- dungeon doors, the stair down, the airship docks ----
    let clear = |keep: &mut Vec<bool>, x: i32, y: i32, r: i32| {
        for yy in y - r..=y + r {
            for xx in x - r..=x + r {
                if xx >= 0 && yy >= 0 && xx < w && yy < h {
                    keep[(yy * w + xx) as usize] = true;
                }
            }
        }
    };
    for (k, def) in DUNGEONS.iter().enumerate().filter(|(_, d)| d.act == 5) {
        let (ex, ey) = def.entrance;
        clear(&mut keep, ex, ey, 4);
        let (fw, fh) = if k == ZENITH { (4, 3) } else { (3, 3) };
        for y in ey - 3..=ey + 1 {
            for x in ex - 3..=ex + 3 {
                d.set(x, y, Tile::Floor);
            }
        }
        prop(&mut lv, &mut d, PropKind::Entrance(k), ex - fw / 2, ey - 3, fw, fh);
        lv.portals.push(Portal { x: ex as f32 + 0.5, y: ey as f32 + 0.5, kind: PortalKind::Entrance(k) });
    }
    let (px, py) = STAIR_SKY;
    clear(&mut keep, px, py, 4);
    prop(&mut lv, &mut d, PropKind::LightStair, px - 1, py - 3, 2, 2);
    lv.portals.push(Portal { x: px as f32 + 0.5, y: py as f32 + 0.5, kind: PortalKind::Pass(4) });
    bridge(&mut d, &mut keep, (mx, ty1 + 1), (DOCK_TOWN.0, DOCK_TOWN.1));
    for (dx, dy) in [DOCK_TOWN, DOCK_FAR] {
        for y in dy - 3..=dy + 2 {
            for x in dx - 2..=dx + 2 {
                d.set(x, y, Tile::Floor);
            }
        }
        clear(&mut keep, dx, dy, 3);
        prop(&mut lv, &mut d, PropKind::AirshipDock, dx - 1, dy - 3, 3, 2);
        lv.portals.push(Portal { x: dx as f32 + 0.5, y: dy as f32 + 0.5, kind: PortalKind::Dock(0) });
    }

    // ---- ruins, statues, trees and lamps ----
    let ruins = Noise::new(&mut rng, 16, 8.0);
    for y in 0..h {
        for x in 0..w {
            if d.get(x, y) != Tile::Floor || keep[(y * w + x) as usize] || d.ground_at(x, y) == 2 {
                continue;
            }
            // Not right on an edge: keep a walkable rim.
            if (-1..=1).any(|dy| (-1..=1).any(|dx| d.get(x + dx, y + dy) == Tile::Void)) {
                continue;
            }
            let f = ruins.at(x as f32, y as f32);
            let r = rng.f();
            if f > 0.62 && r < 0.22 || r < 0.006 {
                let kind = match rng.range(0, 10) {
                    0..=3 => PropKind::CloudTree,
                    4..=6 => PropKind::MarbleRuin,
                    7..=8 => PropKind::SkyLamp,
                    _ => PropKind::AngelStatue,
                };
                prop(&mut lv, &mut d, kind, x, y, 1, 1);
            }
        }
    }

    // ---- roaming packs (none in town, on bridges or by the doors), and food ----
    let (cx, cy) = anchorage_center();
    let mut packs = 0;
    for _ in 0..1500 {
        if packs >= 26 {
            break;
        }
        let x = rng.range(4, w - 4) as f32 + 0.5;
        let y = rng.range(4, h - 4) as f32 + 0.5;
        let far = ((x - cx).powi(2) + (y - cy).powi(2)).sqrt();
        if far < 19.0
            || d.blocked(x, y, 0.4)
            || d.ground_at(x as i32, y as i32) == 2
            || DUNGEONS.iter().filter(|d| d.act == 5).any(|def| (def.entrance.0 as f32 - x).abs() + (def.entrance.1 as f32 - y).abs() < 6.0)
            || ((x - px as f32).abs() < 6.0 && (y - py as f32).abs() < 6.0)
            || [DOCK_TOWN, DOCK_FAR].iter().any(|&(dx, dy)| (x - dx as f32).abs() < 5.0 && (y - dy as f32).abs() < 5.0)
        {
            continue;
        }
        let tier = if far < 34.0 { 12.0 } else { 12.4 };
        let far_isle = ((x - FAR_ISLE.0 as f32).powi(2) + (y - FAR_ISLE.1 as f32).powi(2)).sqrt() < 10.0;
        let kinds: &[Kind] = if far_isle {
            &[Kind::Thunderbird, Kind::Sentinel]
        } else if far < 34.0 {
            &[Kind::FallenSeraph, Kind::Harpy, Kind::Zealot, Kind::Ophanim]
        } else {
            &[Kind::FallenSeraph, Kind::Harpy, Kind::Zealot, Kind::Ophanim, Kind::StormDrake, Kind::Sentinel]
        };
        let kind = kinds[rng.range(0, kinds.len() as i32) as usize];
        let n = match kind {
            Kind::Sentinel | Kind::Thunderbird => rng.range(1, 3),
            Kind::Harpy => rng.range(4, 7),
            Kind::Zealot => rng.range(2, 4),
            _ => rng.range(3, 5),
        };
        for _ in 0..n {
            for _try in 0..10 {
                let (mx, my) = (x + rng.rf(-2.0, 2.0), y + rng.rf(-2.0, 2.0));
                if !d.blocked(mx, my, 0.35) && d.ground_at(mx as i32, my as i32) != 2 {
                    lv.mobs.push(Mob::new(kind, mx, my, tier, &mut rng));
                    break;
                }
            }
        }
        packs += 1;
    }
    let mut food = 0;
    for _ in 0..600 {
        if food >= 10 {
            break;
        }
        let x = rng.range(4, w - 4) as f32 + 0.5;
        let y = rng.range(4, h - 4) as f32 + 0.5;
        if ((x - cx).powi(2) + (y - cy).powi(2)).sqrt() > 16.0 && !d.blocked(x, y, 0.3) {
            lv.pickups.push(Pickup { x, y, kind: Drop::Food(if rng.chance(0.5) { 1 } else { 2 }), t: 1.0 });
            food += 1;
        }
    }
    lv.explored = vec![false; (w * h) as usize];
    lv.d = d;
    lv.start = anchorage_center();
    lv
}

const STAIR_SKY_ISLE: (i32, i32, f32) = (12, 56, 8.0);
const FAR_ISLE_R: (i32, i32, f32) = (96, 56, 9.0);

/// Brinehollow's square.
pub fn brinehollow_center() -> (f32, f32) {
    (54.5, 55.5)
}

/// Kinds of the sea: they swim faster when the tide is in (tides.rs).
pub fn sea_kind(k: Kind) -> bool {
    matches!(k, Kind::Merrow | Kind::Jelly | Kind::Anglerlurk | Kind::Siren | Kind::InkHorror | Kind::Shellguard)
}

/// Builds Act 5's overland: the Sunken Reach, the floor of the black sea under a great air bubble,
/// walled in by reef. Low ground (ground 1) floods when the tide comes in. Brinehollow stands on its
/// stilts in the middle; roads of wreck planks run to the four drowned dungeons and the diving bell.
pub fn deep(seed: u64) -> Level {
    let mut rng = Rng::new(seed ^ 0x5EA_F100D);
    let (w, h) = (WORLD_W, WORLD_H);
    let mut d = Dungeon::blank(w, h, Tile::Floor);
    for v in d.var.iter_mut() {
        *v = rng.range(0, 100) as u8;
    }
    let mut lv = Level::new(LevelId::Deep, "THE SUNKEN REACH".into(), Theme::Deep, 9.6, Dungeon::blank(1, 1, Tile::Void));
    let mut keep = vec![false; (w * h) as usize];
    let clear = |keep: &mut Vec<bool>, x: i32, y: i32, r: i32| {
        for yy in y - r..=y + r {
            for xx in x - r..=x + r {
                if xx >= 0 && yy >= 0 && xx < w && yy < h {
                    keep[(yy * w + xx) as usize] = true;
                }
            }
        }
    };
    let prop = |lv: &mut Level, d: &mut Dungeon, kind: PropKind, x0: i32, y0: i32, fw: i32, fh: i32| {
        for y in y0..y0 + fh {
            for x in x0..x0 + fw {
                d.set(x, y, Tile::Prop);
            }
        }
        lv.props.push(Prop::on(kind, x0, y0, fw, fh));
    };

    // ---- tide flats (ground 1): low sand that floods at high tide ----
    let low = Noise::new(&mut rng, 14, 7.0);
    for y in 0..h {
        for x in 0..w {
            if low.at(x as f32, y as f32) > 0.6 {
                d.set_ground(x, y, 1);
            }
        }
    }

    // ---- Brinehollow ----
    let (tx0, ty0, tx1, ty1) = BRINEHOLLOW;
    let (mx, my) = ((tx0 + tx1) / 2, (ty0 + ty1) / 2);
    for y in ty0..=ty1 {
        for x in tx0..=tx1 {
            d.set_ground(x, y, 0);
            let edge = x == tx0 || x == tx1 || y == ty0 || y == ty1;
            let gate = (y == ty0 || y == ty1) && (mx - 1..=mx + 2).contains(&x) || (x == tx0 || x == tx1) && (my - 1..=my + 2).contains(&y);
            if edge && !gate {
                d.set(x, y, Tile::Wall);
            }
        }
    }
    clear(&mut keep, mx, my, 14);
    for x in tx0..=tx1 {
        for y in my..=my + 1 {
            d.set_ground(x, y, 2);
        }
    }
    for y in ty0..=ty1 {
        for x in mx..=mx + 1 {
            d.set_ground(x, y, 2);
        }
    }
    prop(&mut lv, &mut d, PropKind::StiltHouse1, 46, 48, 4, 4);
    prop(&mut lv, &mut d, PropKind::StiltHouse2, 58, 48, 4, 4);
    prop(&mut lv, &mut d, PropKind::StiltHouse1, 46, 59, 4, 3);
    prop(&mut lv, &mut d, PropKind::Coral1, 59, 59, 2, 2);
    prop(&mut lv, &mut d, PropKind::ShellLamp, 52, 53, 1, 1);
    prop(&mut lv, &mut d, PropKind::ShellLamp, 57, 58, 1, 1);
    prop(&mut lv, &mut d, PropKind::Coral2, 62, 54, 1, 2);
    lv.safe = Some((tx0 as f32 - 1.0, ty0 as f32 - 1.0, tx1 as f32 + 2.0, ty1 as f32 + 2.0));
    lv.npcs = vec![
        Npc::new("CAPTAIN YSOLDE MARROW", Role::Ysolde, "npc_ysolde", 55.5, 56.8, 6),
        Npc::new("NESSA THE PEARL-DIVER", Role::Nessa, "npc_nessa", 58.0, 55.8, 0),
        Npc::new("BROTHER CORAL", Role::Coral, "npc_coral", 50.5, 55.5, 2),
        Npc::new("DIVER", Role::Diver(0), "npc_diver", 52.0, 60.0, 1),
        Npc::new("DIVER", Role::Diver(1), "npc_diver", 61.5, 57.0, 5),
        Npc::new("DIVER", Role::Diver(2), "npc_diver", 49.5, 52.0, 3),
        Npc::new("THE PEARL-SETTER", Role::Jeweler(4), "npc_jeweler4", 54.0, 58.5, 1),
    ];

    // ---- plank roads (ground 2), never flooded ----
    let gates = [(mx, ty1 + 1), (tx1 + 1, my), (mx, ty0 - 1), (tx0 - 1, my)];
    let road = |d: &mut Dungeon, keep: &mut Vec<bool>, (gx, gy): (i32, i32), (ex, ey): (i32, i32), salt: f32| {
        let (mut x, mut y) = (gx as f32, gy as f32);
        let mut guard = 0;
        while ((x - ex as f32).abs() > 0.8 || (y - ey as f32).abs() > 0.8) && guard < 400 {
            guard += 1;
            let (dx, dy) = (ex as f32 - x, ey as f32 - y);
            let l = (dx * dx + dy * dy).sqrt();
            let wob = (guard as f32 * 0.13 + salt).sin() * 0.6;
            x += dx / l + (-dy / l) * wob * 0.6;
            y += dy / l + (dx / l) * wob * 0.6;
            for (ox, oy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                let (rx, ry) = (x as i32 + ox, y as i32 + oy);
                if d.get(rx, ry) == Tile::Floor {
                    d.set_ground(rx, ry, 2);
                }
            }
            clear(keep, x as i32, y as i32, 2);
        }
    };
    for (k, def) in DUNGEONS.iter().enumerate().filter(|(_, d)| d.act == 4) {
        let (ex, ey) = def.entrance;
        let &g = gates.iter().min_by_key(|(gx, gy)| (gx - ex).pow(2) + (gy - ey).pow(2)).unwrap();
        road(&mut d, &mut keep, g, (ex, ey), k as f32);
        clear(&mut keep, ex, ey, 5);
        let (fw, fh) = if k == ABYSS { (4, 3) } else { (3, 3) };
        prop(&mut lv, &mut d, PropKind::Entrance(k), ex - fw / 2, ey - 3, fw, fh);
        lv.portals.push(Portal { x: ex as f32 + 0.5, y: ey as f32 + 0.5, kind: PortalKind::Entrance(k) });
    }
    let (px, py) = LIFT_DEEP;
    road(&mut d, &mut keep, (tx0 - 1, my), (px, py), 7.0);
    clear(&mut keep, px, py, 4);
    prop(&mut lv, &mut d, PropKind::DivingBell, px - 1, py - 3, 3, 2);
    lv.portals.push(Portal { x: px as f32 + 0.5, y: py as f32 + 0.5, kind: PortalKind::Pass(3) });
    // The stair of light up to the Heavens (once the Leviathan is dead).
    let (sx, sy) = STAIR_DEEP;
    road(&mut d, &mut keep, (tx1 + 1, my), (sx, sy), 9.0);
    clear(&mut keep, sx, sy, 4);
    prop(&mut lv, &mut d, PropKind::LightStair, sx - 1, sy - 3, 2, 2);
    lv.portals.push(Portal { x: sx as f32 + 0.5, y: sy as f32 + 0.5, kind: PortalKind::Pass(5) });

    // ---- the reef wall: a ragged rim of coral, and a few coral heads (never across a road) ----
    let rim = Noise::new(&mut rng, 12, 6.0);
    let heads = Noise::new(&mut rng, 18, 9.0);
    for y in 0..h {
        for x in 0..w {
            if keep[(y * w + x) as usize] || d.get(x, y) != Tile::Floor {
                continue;
            }
            let edge = x.min(y).min(w - 1 - x).min(h - 1 - y) as f32;
            let ragged = edge < 3.0 + rim.at(x as f32, y as f32) * 4.0;
            if ragged || heads.at(x as f32, y as f32) > 0.8 {
                d.set(x, y, Tile::Wall);
            }
        }
    }

    // ---- kelp forests, coral, wrecks and whale bones ----
    let kelp = Noise::new(&mut rng, 16, 8.0);
    for y in 0..h {
        for x in 0..w {
            if d.get(x, y) != Tile::Floor || keep[(y * w + x) as usize] {
                continue;
            }
            let f = kelp.at(x as f32, y as f32);
            let r = rng.f();
            if f > 0.6 && r < 0.3 || r < 0.008 {
                let kind = match rng.range(0, 10) {
                    0..=4 => PropKind::Kelp,
                    5..=6 => PropKind::Coral1,
                    _ => PropKind::Coral2,
                };
                prop(&mut lv, &mut d, kind, x, y, 1, 1);
            } else if r < 0.0105 && x + 3 < w && y + 2 < h && (0..3).all(|ox| (0..2).all(|oy| d.get(x + ox, y + oy) == Tile::Floor && !keep[((y + oy) * w + x + ox) as usize])) {
                let kind = if rng.chance(0.5) { PropKind::WreckHull } else { PropKind::WhaleBones };
                prop(&mut lv, &mut d, kind, x, y, 3, 2);
            } else if r < 0.013 {
                prop(&mut lv, &mut d, PropKind::ShellLamp, x, y, 1, 1);
            } else if r < 0.015 {
                prop(&mut lv, &mut d, PropKind::Coral2, x, y, 1, 1);
            }
        }
    }

    // ---- roaming packs and a little food ----
    let (cx, cy) = brinehollow_center();
    let mut packs = 0;
    for _ in 0..800 {
        if packs >= 30 {
            break;
        }
        let x = rng.range(6, w - 6) as f32 + 0.5;
        let y = rng.range(6, h - 6) as f32 + 0.5;
        let far = ((x - cx).powi(2) + (y - cy).powi(2)).sqrt();
        if far < 20.0
            || d.blocked(x, y, 0.4)
            || DUNGEONS.iter().filter(|d| d.act == 4).any(|def| (def.entrance.0 as f32 - x).abs() + (def.entrance.1 as f32 - y).abs() < 6.0)
            || ((x - px as f32).abs() < 7.0 && (y - py as f32).abs() < 7.0)
            || ((x - sx as f32).abs() < 7.0 && (y - sy as f32).abs() < 7.0)
        {
            continue;
        }
        let tier = if far < 34.0 { 9.6 } else { 10.0 };
        let kinds: &[Kind] = if far < 34.0 {
            &[Kind::Drowned, Kind::Merrow, Kind::Jelly, Kind::Shellguard]
        } else {
            &[Kind::Drowned, Kind::Merrow, Kind::Siren, Kind::Anglerlurk, Kind::InkHorror, Kind::Jelly, Kind::Shellguard]
        };
        let kind = kinds[rng.range(0, kinds.len() as i32) as usize];
        let n = match kind {
            Kind::Anglerlurk | Kind::Siren => rng.range(1, 3),
            Kind::Shellguard | Kind::InkHorror => rng.range(2, 4),
            Kind::Jelly => rng.range(4, 7),
            _ => rng.range(3, 6),
        };
        for _ in 0..n {
            for _try in 0..10 {
                let (mx, my) = (x + rng.rf(-2.0, 2.0), y + rng.rf(-2.0, 2.0));
                if !d.blocked(mx, my, 0.35) {
                    lv.mobs.push(Mob::new(kind, mx, my, tier, &mut rng));
                    break;
                }
            }
        }
        packs += 1;
    }
    let mut food = 0;
    for _ in 0..400 {
        if food >= 10 {
            break;
        }
        let x = rng.range(6, w - 6) as f32 + 0.5;
        let y = rng.range(6, h - 6) as f32 + 0.5;
        if ((x - cx).powi(2) + (y - cy).powi(2)).sqrt() > 16.0 && !d.blocked(x, y, 0.3) {
            lv.pickups.push(Pickup { x, y, kind: Drop::Food(if rng.chance(0.5) { 1 } else { 2 }), t: 1.0 });
            food += 1;
        }
    }
    lv.explored = vec![false; (w * h) as usize];
    lv.d = d;
    lv.start = brinehollow_center();
    lv
}

/// An Ash Rift (endgame.rs): one big floor in any act's look, its monsters drawn from two or three acts
/// mixed together, and a way home at the start. Its guardian comes when you've slain enough.
pub fn rift(tier: u16, seed: u64) -> Level {
    let mut rng = Rng::new(seed ^ 0xA5_4121F7 ^ tier as u64 * 0x9E37);
    let d = Dungeon::generate(&mut rng, 80, 80);
    let look = &DUNGEONS[rng.range(0, DUNGEONS.len() as i32) as usize];
    let area = crate::endgame::rift_area_tier(tier);
    let mut lv = Level::new(LevelId::Rift(tier), format!("ASH RIFT - TIER {tier}"), look.theme, area, d);
    let rooms: Vec<Room> = lv.d.rooms.clone();
    let (sx, sy) = rooms[0].center();
    lv.start = (sx as f32 + 1.5, sy as f32 + 0.5);
    lv.portals.push(Portal { x: sx as f32 + 0.5, y: sy as f32 + 0.5, kind: PortalKind::TownPortal });
    let mut pool: Vec<Kind> = vec![];
    for _ in 0..3 {
        let def = &DUNGEONS[rng.range(0, DUNGEONS.len() as i32) as usize];
        pool.extend(def.monsters.iter().copied());
    }
    for r in rooms.iter().skip(1) {
        let n = rng.range(3, 6) + (tier as i32 / 4).min(4);
        let main = pool[rng.range(0, pool.len() as i32) as usize];
        for _ in 0..n {
            for _try in 0..20 {
                let x = rng.range(r.x + 1, r.x + r.w - 1) as f32 + 0.5;
                let y = rng.range(r.y + 1, r.y + r.h - 1) as f32 + 0.5;
                if lv.d.blocked(x, y, 0.35) || lv.mobs.iter().any(|m| (m.x - x).abs() + (m.y - y).abs() < 1.0) {
                    continue;
                }
                let kind = if rng.chance(0.35) { pool[rng.range(0, pool.len() as i32) as usize] } else { main };
                lv.mobs.push(Mob::new(kind, x, y, area, &mut rng));
                break;
            }
        }
    }
    lv
}

/// Bosses past Act 1 fell in seconds (balance pass 2026-10-07, scripts/balance-bench.sh): more life by
/// act, so a herald lasts ~8 s and an act's last boss ~20-30 s at that act's power.
pub fn boss_life(act: usize) -> f32 {
    [1.0, 2.5, 3.0, 3.5, 4.0, 4.5][act.min(5)]
}

/// One floor of a dungeon: stairs up in the first room, stairs down (or the boss)
/// in the last, monster packs in between.
pub fn dungeon_floor(k: usize, floor: usize, seed: u64) -> Level {
    let def = &DUNGEONS[k];
    let mut rng = Rng::new(seed ^ ((k as u64 + 1) * 7919 + floor as u64 * 104_729));
    let d = Dungeon::generate(&mut rng, 72, 72);
    let tier = def.tier + floor as f32 * 0.25;
    let last = floor + 1 == def.floors;
    let name = format!("{} - LEVEL {}", def.name, floor + 1);
    let mut lv = Level::new(LevelId::Dungeon(k, floor), name, def.theme, tier, d);
    let rooms: Vec<Room> = lv.d.rooms.clone();
    let (sx, sy) = rooms[0].center();
    lv.start = (sx as f32 + 1.5, sy as f32 + 0.5);
    lv.portals.push(Portal { x: sx as f32 + 0.5, y: sy as f32 + 0.5, kind: PortalKind::Up });
    lv.props.push(Prop::stairs(PropKind::StairsUp, sx, sy));
    let end = *rooms.last().unwrap();
    let (ex, ey) = end.center();
    if last {
        let mut boss = Mob::new(def.boss, ex as f32 + 0.5, ey as f32 + 0.5, tier * 0.5 + 0.5, &mut rng);
        boss.max_hp *= boss_life(def.act);
        boss.hp = boss.max_hp;
        boss.home = (-1000.0, -2000.0); // bosses don't wander home
        lv.mobs.push(boss);
    } else {
        lv.portals.push(Portal { x: ex as f32 + 0.5, y: ey as f32 + 0.5, kind: PortalKind::Down });
        lv.props.push(Prop::stairs(PropKind::StairsDown, ex, ey));
    }
    let rooms_with_mobs = rooms.len() - if last { 1 } else { 0 };
    for r in rooms.iter().take(rooms_with_mobs).skip(1) {
        let n = rng.range(2, 5) + floor as i32;
        let main = def.monsters[rng.range(0, def.monsters.len() as i32) as usize];
        for _ in 0..n {
            for _try in 0..20 {
                let x = rng.range(r.x + 1, r.x + r.w - 1) as f32 + 0.5;
                let y = rng.range(r.y + 1, r.y + r.h - 1) as f32 + 0.5;
                if lv.d.blocked(x, y, 0.35) || lv.mobs.iter().any(|m| (m.x - x).abs() + (m.y - y).abs() < 1.0) {
                    continue;
                }
                let kind = if rng.chance(0.25) { def.monsters[rng.range(0, def.monsters.len() as i32) as usize] } else { main };
                lv.mobs.push(Mob::new(kind, x, y, tier, &mut rng));
                break;
            }
        }
    }
    let n_food = 3 + rng.range(0, 3);
    for _ in 0..n_food {
        let r = rooms[rng.range(1, rooms.len() as i32) as usize];
        for _try in 0..20 {
            let x = rng.range(r.x + 1, r.x + r.w - 1) as f32 + 0.5;
            let y = rng.range(r.y + 1, r.y + r.h - 1) as f32 + 0.5;
            if lv.d.blocked(x, y, 0.3) {
                continue;
            }
            let roll = rng.f();
            let kind = if roll < 0.5 { 0 } else if roll < 0.85 { 1 } else { 2 };
            lv.pickups.push(Pickup { x, y, kind: Drop::Food(kind), t: 1.0 });
            break;
        }
    }
    lv
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_skyreach_bridges_anchorage_to_every_sky_dungeon_and_the_stair() {
        let lv = heavens(7);
        let (cx, cy) = anchorage_center();
        let mut kinds = vec![];
        for p in &lv.portals {
            assert!(lv.d.walkable(p.x as i32, p.y as i32), "portal {:?} blocked", p.kind);
            // The far island's dock is reached by airship only.
            let far_dock = p.kind == PortalKind::Dock(0) && (p.x as i32, p.y as i32) == DOCK_FAR;
            if !far_dock {
                assert!(lv.d.path((cx as i32, cy as i32), (p.x as i32, p.y as i32), 100_000).is_some(), "no bridge to {:?}", p.kind);
            }
            kinds.push(p.kind);
        }
        for k in 20..24 {
            assert!(kinds.contains(&PortalKind::Entrance(k)));
        }
        assert!(kinds.contains(&PortalKind::Pass(4)));
        assert_eq!(kinds.iter().filter(|k| **k == PortalKind::Dock(0)).count(), 2, "two airship docks");
        // The far island is off the bridges: walking there is impossible.
        let (fx, fy) = FAR_ISLE;
        let far = (fy - 3..=fy + 3).flat_map(|y| (fx - 3..=fx + 3).map(move |x| (x, y))).find(|&(x, y)| lv.d.walkable(x, y) && lv.d.get(x, y) != Tile::Prop);
        if let Some((x, y)) = far {
            assert!(lv.d.path((cx as i32, cy as i32), (x, y), 100_000).is_none(), "the far island is by airship only");
        }
        for n in &lv.npcs {
            assert!(lv.d.walkable(n.x as i32, n.y as i32), "{} stands in a wall", n.name);
        }
        assert!(lv.mobs.len() > 40, "{} monsters", lv.mobs.len());
        assert!((0..lv.d.h).flat_map(|y| (0..lv.d.w).map(move |x| (x, y))).filter(|&(x, y)| lv.d.get(x, y) == Tile::Void).count() > 3000, "there's plenty of sky");
        let (x0, y0, x1, y1) = lv.safe.unwrap();
        assert!(lv.mobs.iter().all(|m| !(m.x > x0 && m.x < x1 && m.y > y0 && m.y < y1)), "monsters spawned in town");
        // The deep has the stair up.
        let deep = deep(7);
        let stair = deep.portal(PortalKind::Pass(5)).expect("the stair of light");
        let (bx, by) = brinehollow_center();
        assert!(deep.d.path((bx as i32, by as i32), (stair.x as i32, stair.y as i32), 100_000).is_some());
    }

    #[test]
    fn the_sunken_reach_connects_brinehollow_to_every_drowned_dungeon_and_the_bell() {
        let lv = deep(7);
        let (cx, cy) = brinehollow_center();
        let mut kinds = vec![];
        for p in &lv.portals {
            assert!(lv.d.walkable(p.x as i32, p.y as i32), "portal {:?} blocked", p.kind);
            assert!(lv.d.path((cx as i32, cy as i32), (p.x as i32, p.y as i32), 100_000).is_some(), "no road to {:?}", p.kind);
            kinds.push(p.kind);
        }
        for k in 16..20 {
            assert!(kinds.contains(&PortalKind::Entrance(k)));
        }
        assert!(kinds.contains(&PortalKind::Pass(3)));
        for n in &lv.npcs {
            assert!(lv.d.walkable(n.x as i32, n.y as i32), "{} stands in a wall", n.name);
        }
        assert!(lv.mobs.len() > 50);
        let (x0, y0, x1, y1) = lv.safe.unwrap();
        assert!(lv.mobs.iter().all(|m| !(m.x > x0 && m.x < x1 && m.y > y0 && m.y < y1)), "monsters spawned in town");
        // Mechanus has the bell down.
        let mech = mechanus(7);
        let bell = mech.portal(PortalKind::Pass(4)).expect("the diving bell");
        let (cx, cy) = escapement_center();
        assert!(mech.d.path((cx as i32, cy as i32), (bell.x as i32, bell.y as i32), 100_000).is_some());
    }

    #[test]
    fn every_entrance_is_reachable_from_town() {
        let lv = overworld(7);
        let (cx, cy) = town_center();
        assert!(lv.d.walkable(cx as i32, cy as i32));
        for p in &lv.portals {
            assert!(lv.d.walkable(p.x as i32, p.y as i32), "portal {:?} blocked", p.kind);
            assert!(lv.d.path((cx as i32, cy as i32), (p.x as i32, p.y as i32), 100_000).is_some(), "no road to {:?}", p.kind);
        }
        for n in &lv.npcs {
            assert!(lv.d.walkable(n.x as i32, n.y as i32), "{} stands in a wall", n.name);
        }
        // Hollowmere's map is quiet; the monsters are out in the areas (areas.rs tests those).
        assert!(lv.mobs.is_empty());
        assert!(lv.portal(PortalKind::Exit(1)).is_some(), "the north road");
    }

    #[test]
    fn the_frostmarch_connects_kaldholm_to_every_ice_dungeon_and_the_pass() {
        let lv = frostmarch(7);
        let (cx, cy) = kaldholm_center();
        assert!(lv.d.walkable(cx as i32, cy as i32));
        let mut kinds = vec![];
        for p in &lv.portals {
            assert!(lv.d.walkable(p.x as i32, p.y as i32), "portal {:?} blocked", p.kind);
            assert!(lv.d.path((cx as i32, cy as i32), (p.x as i32, p.y as i32), 100_000).is_some(), "no road to {:?}", p.kind);
            kinds.push(p.kind);
        }
        for k in 4..8 {
            assert!(kinds.contains(&PortalKind::Entrance(k)));
        }
        assert!(kinds.contains(&PortalKind::Pass(0)));
        for n in &lv.npcs {
            assert!(lv.d.walkable(n.x as i32, n.y as i32), "{} stands in a wall", n.name);
        }
        assert!(lv.mobs.len() > 50 && lv.mobs.iter().all(|m| crate::mobs::def(m.kind).cold));
        let (x0, y0, x1, y1) = lv.safe.unwrap();
        assert!(lv.mobs.iter().all(|m| !(m.x > x0 && m.x < x1 && m.y > y0 && m.y < y1)), "monsters spawned in town");
        // And the Ashen Steppe (Act 1's last area) has the pass north.
        let steppe = generate(LevelId::Area(0, 6), 7);
        let pass = steppe.portal(PortalKind::Pass(1)).expect("pass north");
        let s = (steppe.start.0 as i32, steppe.start.1 as i32);
        assert!(steppe.d.path(s, (pass.x as i32, pass.y as i32), 100_000).is_some());
    }

    #[test]
    fn mechanus_connects_the_escapement_to_every_works_and_the_gear_gate() {
        let lv = mechanus(7);
        let (cx, cy) = escapement_center();
        assert!(lv.d.walkable(cx as i32, cy as i32));
        let mut kinds = vec![];
        for p in &lv.portals {
            assert!(lv.d.walkable(p.x as i32, p.y as i32), "portal {:?} blocked", p.kind);
            assert!(lv.d.path((cx as i32, cy as i32), (p.x as i32, p.y as i32), 100_000).is_some(), "no road to {:?}", p.kind);
            kinds.push(p.kind);
        }
        for k in 12..16 {
            assert!(kinds.contains(&PortalKind::Entrance(k)));
        }
        assert!(kinds.contains(&PortalKind::Pass(2)));
        for n in &lv.npcs {
            assert!(lv.d.walkable(n.x as i32, n.y as i32), "{} stands in a wall", n.name);
        }
        assert!(lv.mobs.len() > 50);
        let (x0, y0, x1, y1) = lv.safe.unwrap();
        assert!(lv.mobs.iter().all(|m| !(m.x > x0 && m.x < x1 && m.y > y0 && m.y < y1)), "monsters spawned in town");
        assert!(lv.d.tiles.iter().any(|t| *t == Tile::Void), "an island over the void");
        // The Mistwood has the gear gate, reachable from Mournhold.
        let mw = mistwood(7);
        let gate = mw.portal(PortalKind::Pass(3)).expect("gear gate");
        let (cx, cy) = mournhold_center();
        assert!(mw.d.path((cx as i32, cy as i32), (gate.x as i32, gate.y as i32), 100_000).is_some());
    }

    #[test]
    fn dungeon_floors_have_stairs_and_a_boss_at_the_bottom() {
        for (k, def) in DUNGEONS.iter().enumerate() {
            for f in 0..def.floors {
                let lv = dungeon_floor(k, f, 99);
                let up = lv.portal(PortalKind::Up).expect("stairs up");
                assert!(lv.d.walkable(up.x as i32, up.y as i32));
                let last = f + 1 == def.floors;
                assert_eq!(lv.portal(PortalKind::Down).is_some(), !last);
                assert_eq!(lv.mobs.iter().filter(|m| m.boss).count(), if last { 1 } else { 0 });
                if last {
                    let b = lv.mobs.iter().find(|m| m.boss).unwrap();
                    assert_eq!(b.kind, def.boss);
                    assert!(lv.d.path((up.x as i32, up.y as i32), (b.x as i32, b.y as i32), 20_000).is_some());
                }
            }
        }
    }
}

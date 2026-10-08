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
    Goblin,
    Archer,
    BoneWarden,
    PlagueWarden,
    HexWarden,
    AshKing,
    // ---- Act 2: the Frostmarch ----
    FrostWolf,
    Raider,
    Yeti,
    IceTroll,
    IceWraith,
    FrostGiant,
    YetiMatriarch,
    RimeWitch,
    WhiteDragon,
    // ---- Act 3: the Mistwood ----
    Ghoul,
    Werewolf,
    Banshee,
    Wisp,
    Cultist,
    /// Count Vardak's bats.
    Bat,
    Ossric,
    Grimhilde,
    Malgrave,
    Vardak,
    // ---- Act 4: Mechanus ----
    Scarab,
    Inquisitor,
    Gearwraith,
    SpringJack,
    BoilerBrute,
    /// The Ordinals: law-bound clockwork shapes that march in squads. Cubits are the rank and file,
    /// prisms the gunners, and a marshal keeps them in step (kill it and they fall into disorder).
    Ordinal,
    Prism,
    Marshal,
    /// Clockwork crows: fast, fragile, in jittering flocks.
    ClockCrow,
    Forgemother,
    Cantor,
    Archivist,
    Clockmaker,
    // ---- Act 5: the Drowned Deep ----
    /// Drowned sailors: their grip drags you (a chill).
    Drowned,
    /// Fish-folk spearmen: they lunge, and swim faster in the flood.
    Merrow,
    /// Hides behind its lure until you come close, then strikes.
    Anglerlurk,
    /// Floating jellyfish: shock bolts.
    Jelly,
    /// Crab knights behind a great claw: half damage unless stunned, frozen or slowed.
    Shellguard,
    /// Her song pulls you toward her.
    Siren,
    /// Its ink blinds you (your light shrinks).
    InkHorror,
    Dregmoor,
    Nacre,
    Angler,
    Leviathan,
    // ---- Act 6: the Shattered Heavens (sky.rs has their tricks) ----
    /// Broken-winged angels: they dive at you.
    FallenSeraph,
    /// Wheels of eyes: fast beams of light.
    Ophanim,
    /// Small storm drakes: forked lightning.
    StormDrake,
    /// Ash harpies: their talons knock you back (mind the edges).
    Harpy,
    /// Stone angels that wake as you pass: slow, very tough, ground slams.
    Sentinel,
    /// Sun-zealots: they heal the monsters around them.
    Zealot,
    /// Rare thunderbirds: they call lightning down on you.
    Thunderbird,
    Vael,
    Tempest,
    OphanPrime,
    Solanthos,
    /// The valkyrie's spectral warriors (always on her side).
    Einherjar,
    /// The berserker's dire wolf (always on her side).
    DireWolf,
    /// The reaper's scholar spirits (on her side for a while).
    Scholar,
    /// The druid's creatures.
    Rat,
    MossWolf,
    ThornWarden,
    // ---- side content (side.rs) ----
    /// The Charnel Well's witch (Act 1's optional dungeon).
    WellWitch,
    /// A gold-thief laden with loot (an event): it flees, and gets away in the end.
    Hoarder,
    /// Vaurath, the Ember Wyrm asleep on its hoard (dragon.rs): a heist, not a fight.
    FireWyrm,
    /// A bone totem raising the dead (a random errand, errands.rs): it doesn't move or fight.
    Totem,
    /// Ash elementals that roam in an ash storm (features.rs).
    AshElemental,
    /// A block of ice with someone frozen inside (features.rs: the frozen merchant, Brenna's scouts).
    IceBlock,
    /// Hrolf Ice-Beard, raider-king of the Icebound Longship (Act 2's optional dungeon).
    Hrolf,
    // ---- Act 3's side content (mist.rs) ----
    /// The Gravedigger, under Widow Kasia's graveyard.
    Gravedigger,
    /// Lady Elspeth's ghost in the Hollow Manor.
    Elspeth,
    /// One of Count Vardak's brides.
    Bride,
    /// The Wailing Shade that haunts you through the Mistwood.
    Shade,
    // ---- Act 4's side content (gears.rs) ----
    /// The brass knight you rebuild from five gears: it fights for you.
    Automaton,
    /// The Junk Golem: rebuilds itself from scrap twice.
    JunkGolem,
    /// A scrap heap in the Scrapyard: smash it for the lottery.
    ScrapPile,
    // ---- Act 5's side content (reef.rs) ----
    /// Old Barnacle, the giant crab knight of the Pearl Grotto.
    Barnacle,
    /// The kraken's arm in the Bone Reef: it grabs and slams, it doesn't move.
    KrakenArm,
    // ---- Act 6's side content (isles.rs) ----
    /// The Astronomer, the fallen seraph of the Fallen Observatory: he throws constellations.
    Astronomer,
    /// A lump of star-metal in a fallen star's crater: break it open.
    StarMetal,
    /// A rival adventurer (extras.rs): `form` is which one, drawn as a hero of their class.
    Rival,
    // ---- breakables (breakables.rs): no mind, smashed by any hit; the act is in `Mob::form` ----
    Crate,
    Barrel,
    Urn,
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
    /// Creature of the cold: takes extra fire damage.
    pub cold: bool,
    /// Its hits chill / numb you (frost creatures, ghouls).
    pub chills: bool,
}

/// Fire damage multiplier: creatures of the cold burn better.
pub fn fire_taken(k: Kind) -> f32 {
    if def(k).cold {
        1.25
    } else {
        1.0
    }
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
        cold: false,
        chills: false,
    };
    match k {
        Kind::Zombie => d("zombie", "ROTTING ZOMBIE", 34.0, 1.25, (5.0, 9.0), 0.55, 1.5, 14.0),
        Kind::Skeleton => d("skeleton", "RISEN SKELETON", 20.0, 2.3, (3.0, 6.0), 0.35, 1.0, 10.0),
        Kind::Wolf => Def { r: 0.3, ..d("wolf", "DIRE WOLF", 16.0, 3.6, (3.0, 5.0), 0.3, 0.9, 9.0) },
        Kind::Goblin => Def { r: 0.26, ..d("goblin", "GOBLIN", 12.0, 2.8, (2.0, 5.0), 0.3, 0.9, 7.0) },
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
        Kind::FrostWolf => Def { r: 0.3, cold: true, chills: true, ..d("frost_wolf", "WINTER WOLF", 18.0, 3.8, (4.0, 6.0), 0.3, 0.9, 11.0) },
        Kind::Raider => Def { cold: true, chills: true, ..d("raider", "NORTHERN RAIDER", 30.0, 2.6, (6.0, 10.0), 0.45, 1.2, 15.0) },
        Kind::Yeti => Def { r: 0.42, reach: 1.0, cold: true, chills: true, ..d("yeti", "YETI", 55.0, 1.6, (9.0, 14.0), 0.65, 1.7, 24.0) },
        Kind::IceTroll => Def { r: 0.36, cold: true, chills: true, ..d("ice_troll", "ICE TROLL", 40.0, 2.2, (6.0, 9.0), 0.4, 1.0, 20.0) },
        Kind::IceWraith => Def { ranged: true, cold: true, chills: true, ..d("ice_wraith", "ICE WRAITH", 22.0, 2.4, (5.0, 8.0), 0.55, 1.8, 16.0) },
        Kind::FrostGiant => Def {
            r: 0.65,
            reach: 1.5,
            boss: true,
            cold: true, chills: true,
            ..d("boss_giant", "THE FROST GIANT OVERSEER", 420.0, 1.4, (16.0, 24.0), 0.8, 1.8, 500.0)
        },
        Kind::YetiMatriarch => Def {
            r: 0.6,
            reach: 1.4,
            boss: true,
            cold: true, chills: true,
            ..d("boss_yeti", "THE YETI MATRIARCH", 400.0, 1.9, (14.0, 20.0), 0.6, 1.5, 550.0)
        },
        Kind::RimeWitch => Def {
            r: 0.5,
            boss: true,
            ranged: true,
            cold: true, chills: true,
            ..d("boss_witch", "THE RIME WITCH", 360.0, 1.8, (10.0, 14.0), 0.6, 2.0, 650.0)
        },
        Kind::WhiteDragon => Def {
            r: 0.9,
            reach: 1.9,
            boss: true,
            cold: true, chills: true,
            ..d("boss_dragon", "VORTHRAX THE RIME WYRM", 1100.0, 1.5, (22.0, 32.0), 0.8, 1.7, 2500.0)
        },
        Kind::Ghoul => Def { chills: true, ..d("ghoul", "GHOUL", 45.0, 3.0, (8.0, 12.0), 0.35, 1.0, 24.0) },
        Kind::Werewolf => Def { r: 0.42, reach: 1.0, ..d("werewolf", "WEREWOLF", 70.0, 3.6, (12.0, 18.0), 0.4, 1.1, 34.0) },
        Kind::Banshee => Def { ranged: true, ..d("banshee", "BANSHEE", 40.0, 2.2, (8.0, 12.0), 0.6, 2.0, 30.0) },
        Kind::Wisp => Def { r: 0.25, ranged: true, ..d("wisp", "WILL-O'-WISP", 20.0, 4.0, (5.0, 8.0), 0.4, 1.6, 18.0) },
        Kind::Cultist => Def { ranged: true, ..d("cultist", "CULTIST", 45.0, 2.2, (9.0, 13.0), 0.6, 1.9, 28.0) },
        Kind::Bat => Def { r: 0.22, ..d("vbat", "VAMPIRE BAT", 14.0, 4.6, (4.0, 6.0), 0.25, 0.8, 6.0) },
        Kind::Ossric => Def {
            r: 0.6,
            reach: 1.5,
            boss: true,
            ..d("boss_ossric", "LORD OSSRIC, THE BONE BARON", 520.0, 1.6, (18.0, 26.0), 0.7, 1.6, 900.0)
        },
        Kind::Grimhilde => Def {
            r: 0.5,
            boss: true,
            ranged: true,
            ..d("boss_grimhilde", "DUCHESS GRIMHILDE", 480.0, 1.8, (12.0, 16.0), 0.6, 2.0, 1000.0)
        },
        Kind::Malgrave => Def {
            r: 0.6,
            reach: 1.6,
            boss: true,
            ..d("boss_malgrave", "SIR MALGRAVE, THE DEATH KNIGHT", 620.0, 1.5, (20.0, 30.0), 0.8, 1.7, 1100.0)
        },
        Kind::Vardak => Def {
            r: 0.55,
            reach: 1.5,
            boss: true,
            ..d("boss_vardak", "COUNT VARDAK", 1400.0, 2.0, (24.0, 34.0), 0.6, 1.5, 4000.0)
        },
        Kind::Scarab => Def { r: 0.3, ..d("brass_scarab", "BRASS SCARAB", 42.0, 3.8, (8.0, 12.0), 0.3, 0.8, 24.0) },
        Kind::Inquisitor => Def { ranged: true, ..d("inquisitor", "CENSER AUTOMATON", 60.0, 2.1, (10.0, 14.0), 0.6, 1.9, 32.0) },
        Kind::Gearwraith => d("gearwraith", "GEARWRAITH", 40.0, 3.0, (9.0, 13.0), 0.4, 1.2, 28.0),
        Kind::SpringJack => d("spring_jack", "SPRING-HEELED JACK", 45.0, 3.4, (11.0, 16.0), 0.3, 1.0, 30.0),
        Kind::BoilerBrute => Def { r: 0.5, reach: 1.1, ..d("boiler_brute", "BOILER BRUTE", 120.0, 1.7, (16.0, 24.0), 0.7, 1.6, 48.0) },
        Kind::Ordinal => Def { r: 0.25, ranged: true, ..d("ordinal", "ORDINAL CUBIT", 24.0, 3.0, (5.0, 8.0), 0.4, 1.4, 16.0) },
        Kind::Prism => Def { r: 0.28, ranged: true, ..d("ordinal_prism", "ORDINAL PRISM", 34.0, 2.6, (6.0, 9.0), 0.5, 1.8, 22.0) },
        Kind::Marshal => Def { r: 0.4, ranged: true, ..d("ordinal_marshal", "ORDINAL MARSHAL", 90.0, 2.4, (9.0, 13.0), 0.6, 2.2, 45.0) },
        Kind::ClockCrow => Def { r: 0.2, ..d("clock_crow", "CLOCKWORK CROW", 14.0, 4.8, (3.0, 5.0), 0.2, 0.7, 7.0) },
        Kind::Forgemother => Def {
            r: 0.6,
            reach: 1.5,
            boss: true,
            ..d("boss_forgemother", "THE FORGEMOTHER", 700.0, 1.5, (20.0, 30.0), 0.7, 1.6, 1400.0)
        },
        Kind::Cantor => Def {
            r: 0.7,
            reach: 1.5,
            boss: true,
            ranged: true,
            ..d("boss_cantor", "THE CANTOR", 760.0, 1.3, (16.0, 24.0), 0.6, 1.8, 1500.0)
        },
        Kind::Archivist => Def {
            r: 0.55,
            boss: true,
            ranged: true,
            ..d("boss_archivist", "THE ARCHIVIST", 680.0, 1.8, (14.0, 20.0), 0.6, 1.8, 1600.0)
        },
        Kind::Rat => Def { r: 0.2, ..d("plague_rat", "PLAGUE RAT", 14.0, 4.5, (3.0, 5.0), 0.25, 0.7, 0.0) },
        Kind::MossWolf => Def { r: 0.36, ..d("moss_wolf", "MOSS WOLF", 80.0, 4.4, (5.0, 8.0), 0.3, 0.8, 0.0) },
        Kind::ThornWarden => Def { r: 0.6, reach: 1.4, ..d("thorn_warden", "THORN WARDEN", 300.0, 2.2, (16.0, 24.0), 0.6, 1.4, 0.0) },
        Kind::WellWitch => Def {
            r: 0.5,
            boss: true,
            ranged: true,
            ..d("boss_wellwitch", "THE WELL-WITCH", 380.0, 1.6, (9.0, 13.0), 0.6, 2.0, 420.0)
        },
        Kind::FireWyrm => Def {
            r: 1.0,
            reach: 2.2,
            boss: true,
            ..d("boss_firewyrm", "VAURATH THE EMBER WYRM", 9000.0, 2.6, (45.0, 65.0), 0.8, 1.6, 9000.0)
        },
        Kind::Barnacle => Def {
            r: 0.75,
            reach: 1.6,
            boss: true,
            ..d("boss_barnacle", "OLD BARNACLE", 1300.0, 1.4, (28.0, 40.0), 0.8, 1.8, 1900.0)
        },
        Kind::Astronomer => Def {
            r: 0.6,
            reach: 1.6,
            ranged: true,
            boss: true,
            ..d("boss_astronomer", "THE ASTRONOMER", 1500.0, 2.0, (26.0, 36.0), 0.6, 1.6, 2400.0)
        },
        Kind::Rival => Def { r: 0.35, reach: 1.3, ..d("mage", "RIVAL", 110.0, 3.0, (4.0, 7.0), 0.45, 1.2, 900.0) },
        Kind::StarMetal => Def { r: 0.45, reach: 0.0, ..d("star_metal", "STAR-METAL", 60.0, 0.0, (0.0, 0.0), 9.0, 99.0, 30.0) },
        Kind::KrakenArm => Def { r: 0.8, reach: 2.2, ..d("kraken_arm", "THE KRAKEN'S ARM", 220.0, 0.0, (20.0, 30.0), 0.7, 2.2, 600.0) },
        Kind::Automaton => Def { r: 0.38, reach: 1.0, ..d("automaton", "THE BRASS KNIGHT", 420.0, 3.8, (14.0, 20.0), 0.4, 0.9, 0.0) },
        Kind::JunkGolem => Def {
            r: 0.7,
            reach: 1.6,
            boss: true,
            ..d("boss_junkgolem", "THE JUNK GOLEM", 900.0, 1.6, (24.0, 34.0), 0.8, 1.7, 1400.0)
        },
        Kind::ScrapPile => Def { r: 0.45, reach: 0.0, ..d("scrap_pile", "SCRAP HEAP", 30.0, 0.0, (0.0, 0.0), 9.0, 99.0, 0.0) },
        Kind::Gravedigger => Def {
            r: 0.6,
            reach: 1.5,
            boss: true,
            ..d("boss_gravedigger", "THE GRAVEDIGGER", 560.0, 1.6, (18.0, 26.0), 0.8, 1.7, 1000.0)
        },
        Kind::Elspeth => Def {
            r: 0.45,
            boss: true,
            ranged: true,
            ..d("boss_elspeth", "LADY ELSPETH", 480.0, 2.0, (10.0, 15.0), 0.6, 1.8, 900.0)
        },
        Kind::Bride => Def { r: 0.32, ..d("vampire_bride", "VAMPIRE BRIDE", 90.0, 3.4, (9.0, 14.0), 0.35, 0.9, 120.0) },
        Kind::Shade => Def { r: 0.36, ranged: true, ..d("shade", "THE WAILING SHADE", 260.0, 3.2, (7.0, 11.0), 0.5, 1.4, 300.0) },
        Kind::AshElemental => Def { r: 0.34, ..d("ash_elemental", "ASH ELEMENTAL", 38.0, 2.6, (5.0, 9.0), 0.45, 1.2, 28.0) },
        Kind::IceBlock => Def { r: 0.45, reach: 0.0, cold: true, ..d("ice_block", "BLOCK OF ICE", 150.0, 0.0, (0.0, 0.0), 9.0, 99.0, 20.0) },
        Kind::Hrolf => Def {
            r: 0.65,
            reach: 1.5,
            boss: true,
            cold: true,
            chills: true,
            ..d("boss_hrolf", "HROLF ICE-BEARD", 520.0, 1.8, (18.0, 26.0), 0.7, 1.6, 900.0)
        },
        Kind::Totem => Def { r: 0.4, reach: 0.0, ..d("totem", "BONE TOTEM", 120.0, 0.0, (0.0, 0.0), 9.0, 99.0, 40.0) },
        Kind::Hoarder => Def { r: 0.26, ..d("hoarder", "GOLD-THIEF", 60.0, 3.3, (1.0, 2.0), 0.4, 1.5, 40.0) },
        Kind::Crate => Def { r: 0.34, ..d("crate", "CRATE", 4.0, 0.0, (0.0, 0.0), 1.0, 9.0, 0.0) },
        Kind::Barrel => Def { r: 0.34, ..d("barrel", "BARREL", 4.0, 0.0, (0.0, 0.0), 1.0, 9.0, 0.0) },
        Kind::Urn => Def { r: 0.3, ..d("urn", "URN", 4.0, 0.0, (0.0, 0.0), 1.0, 9.0, 0.0) },
        Kind::Scholar => Def { r: 0.3, reach: 2.4, ..d("scholar_spirit", "SCHOLAR SPIRIT", 50.0, 3.0, (7.0, 11.0), 0.35, 1.1, 0.0) },
        Kind::DireWolf => Def { r: 0.36, ..d("dire_wolf", "DIRE WOLF", 80.0, 4.6, (4.0, 7.0), 0.3, 0.8, 0.0) },
        Kind::Einherjar => Def { r: 0.32, ..d("einherjar", "EINHERJAR", 60.0, 3.0, (8.0, 12.0), 0.35, 1.0, 0.0) },
        Kind::Clockmaker => Def {
            r: 0.55,
            reach: 1.5,
            boss: true,
            ..d("boss_clockmaker", "THE CLOCKMAKER", 1800.0, 2.1, (26.0, 36.0), 0.6, 1.4, 6000.0)
        },
        Kind::Drowned => Def { chills: true, ..d("drowned_sailor", "DROWNED SAILOR", 70.0, 1.5, (11.0, 16.0), 0.55, 1.5, 34.0) },
        Kind::Merrow => d("merrow", "MERROW SPEARMAN", 58.0, 3.0, (12.0, 17.0), 0.4, 1.1, 36.0),
        Kind::Anglerlurk => Def { r: 0.4, reach: 1.0, ..d("anglerlurk", "ANGLERLURK", 90.0, 2.4, (16.0, 24.0), 0.35, 1.4, 44.0) },
        Kind::Jelly => Def { r: 0.28, ranged: true, ..d("jelly_drift", "JELLY DRIFT", 30.0, 1.6, (7.0, 10.0), 0.5, 1.8, 20.0) },
        Kind::Shellguard => Def { r: 0.42, reach: 1.0, ..d("shellguard", "SHELLGUARD", 130.0, 1.5, (14.0, 20.0), 0.6, 1.6, 50.0) },
        Kind::Siren => Def { ranged: true, ..d("siren", "SIREN", 55.0, 2.2, (10.0, 14.0), 0.55, 2.0, 40.0) },
        Kind::InkHorror => Def { r: 0.4, ranged: true, ..d("ink_horror", "INK HORROR", 80.0, 1.8, (10.0, 15.0), 0.6, 2.2, 42.0) },
        Kind::Dregmoor => Def {
            r: 0.6,
            reach: 1.6,
            boss: true,
            chills: true,
            ..d("boss_dregmoor", "ADMIRAL DREGMOOR", 900.0, 1.6, (24.0, 34.0), 0.7, 1.6, 2200.0)
        },
        Kind::Nacre => Def {
            r: 0.5,
            boss: true,
            ranged: true,
            ..d("boss_nacre", "MOTHER NACRE, THE SIREN QUEEN", 820.0, 1.9, (16.0, 22.0), 0.6, 1.8, 2400.0)
        },
        Kind::Angler => Def {
            r: 0.7,
            reach: 1.6,
            boss: true,
            ..d("boss_angler", "THE ANGLER MATRIARCH", 1000.0, 2.0, (26.0, 38.0), 0.5, 1.5, 2600.0)
        },
        Kind::FallenSeraph => d("fallen_seraph", "FALLEN SERAPH", 80.0, 2.8, (15.0, 21.0), 0.45, 1.2, 46.0),
        Kind::Ophanim => Def { r: 0.36, ranged: true, ..d("ophanim", "OPHANIM", 60.0, 2.0, (11.0, 15.0), 0.5, 1.9, 44.0) },
        Kind::StormDrake => Def { r: 0.4, ranged: true, ..d("storm_drake", "STORM DRAKE", 90.0, 2.6, (12.0, 17.0), 0.5, 2.0, 50.0) },
        Kind::Harpy => Def { r: 0.3, ..d("ash_harpy", "ASH HARPY", 46.0, 4.2, (9.0, 13.0), 0.3, 0.9, 30.0) },
        Kind::Sentinel => Def { r: 0.55, reach: 1.3, ..d("gilded_sentinel", "GILDED SENTINEL", 260.0, 1.3, (24.0, 34.0), 0.8, 2.0, 90.0) },
        Kind::Zealot => Def { ranged: true, ..d("sun_zealot", "SUN-ZEALOT", 64.0, 2.2, (11.0, 15.0), 0.55, 2.0, 44.0) },
        Kind::Thunderbird => Def { r: 0.5, ranged: true, ..d("thunderbird", "THUNDERBIRD", 200.0, 3.0, (16.0, 24.0), 0.5, 1.8, 120.0) },
        Kind::Vael => Def {
            r: 0.6,
            reach: 1.6,
            boss: true,
            ..d("boss_vael", "SERAPH-COMMANDER VAEL", 1100.0, 2.2, (28.0, 38.0), 0.55, 1.5, 3200.0)
        },
        Kind::Tempest => Def {
            r: 0.8,
            reach: 1.8,
            boss: true,
            ranged: true,
            ..d("boss_tempest", "THE TEMPEST DRAKE", 1200.0, 1.8, (22.0, 30.0), 0.6, 1.8, 3400.0)
        },
        Kind::OphanPrime => Def {
            r: 0.8,
            boss: true,
            ranged: true,
            ..d("boss_ophan", "THE OPHAN PRIME", 1150.0, 1.2, (18.0, 26.0), 0.6, 1.9, 3600.0)
        },
        Kind::Solanthos => Def {
            r: 0.7,
            reach: 1.8,
            boss: true,
            ..d("boss_solanthos", "SOLANTHOS, THE BURNT-OUT SUN", 3200.0, 1.8, (34.0, 48.0), 0.6, 1.6, 14000.0)
        },
        Kind::Leviathan => Def {
            r: 0.9,
            reach: 2.2,
            boss: true,
            ..d("boss_leviathan", "THE LEVIATHAN", 2600.0, 1.2, (30.0, 44.0), 0.7, 1.7, 9000.0)
        },
    }
}

/// D2-style elite monsters: blue champion packs, and gold-named elites with minions.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Rank {
    Normal,
    Champion,
    Elite,
    Minion,
}

/// Elite modifiers (bit flags).
pub const M_FAST: u8 = 1;
pub const M_STRONG: u8 = 2;
pub const M_STONE: u8 = 4;
pub const M_VAMPIRE: u8 = 8;
pub const M_MANABURN: u8 = 16;
pub const M_FIERY: u8 = 32;
pub const MODS: [(u8, &str); 6] = [
    (M_FAST, "FAST"),
    (M_STRONG, "STRONG"),
    (M_STONE, "STONE SKIN"),
    (M_VAMPIRE, "VAMPIRIC"),
    (M_MANABURN, "MANA BURN"),
    (M_FIERY, "FIRE ENCHANTED"),
];

pub fn mod_text(mods: u8) -> String {
    MODS.iter().filter(|(m, _)| mods & m != 0).map(|(_, n)| *n).collect::<Vec<_>>().join(", ")
}

const ELITE_A: &[&str] = &["GRIMTOOTH", "ROTGUT", "BLOODMAW", "ASHCLAW", "BONEGNAW", "GLOOMFANG", "SOOTHIDE", "CINDERSKULL", "MARROWKIN", "DREADSPINE"];
const ELITE_B: &[&str] = &["THE FOUL", "THE HUNGRY", "THE CRUEL", "THE BURNT", "THE DEFILER", "THE WICKED", "THE UNCLEAN", "THE VILE"];

pub fn elite_name(rng: &mut Rng) -> String {
    format!("{} {}", ELITE_A[rng.range(0, ELITE_A.len() as i32) as usize], ELITE_B[rng.range(0, ELITE_B.len() as i32) as usize])
}

/// `n` different random modifiers.
pub fn roll_mods(n: usize, rng: &mut Rng) -> u8 {
    let mut m = 0u8;
    while (m.count_ones() as usize) < n.min(MODS.len()) {
        m |= MODS[rng.range(0, MODS.len() as i32) as usize].0;
    }
    m
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
    /// How long it has been burning without a break (Combust hits harder).
    pub burned: f32,
    pub path: Vec<(f32, f32)>,
    pub repath: f32,
    pub wander: (f32, f32, f32),
    /// Goblins run away from you for this long after a packmate dies.
    pub flee: f32,
    /// Boss ability timers.
    pub special: f32,
    pub special2: f32,
    pub enraged: bool,
    /// Where it lives (overworld monsters return home instead of chasing you forever).
    pub home: (f32, f32),
    pub tier: f32,
    pub rank: Rank,
    pub mods: u8,
    /// Elite leaders have their own names.
    pub name: Option<String>,
    /// Mesmerized or raised by the vampire: fights for you while this lasts.
    pub charm: f32,
    /// A raised thrall crumbles when its charm runs out.
    pub thrall: bool,
    /// The valkyrie's frost on it (0..1: slows it; full freezes it with Frost Brand).
    pub frost: f32,
    /// Frozen solid (can't move or act; shatters if she kills it).
    pub frozen: f32,
    /// Her raven's mark: she hits it harder while this lasts.
    pub marked: f32,
    /// The berserker's Rend: bleeding (damage per second, seconds left) and sundered armor.
    pub bleed: f32,
    pub bleed_t: f32,
    pub sunder: f32,
    /// Caught in the reaper's hourglass: crawling.
    pub slow_t: f32,
    /// The druid's poison: damage per second and seconds left; plagued foes pass it on when they die.
    pub poison: f32,
    pub poison_t: f32,
    pub plagued: bool,
    /// Untouchable while this lasts (Count Vardak's mist form).
    pub invuln: f32,
    /// Boss form (Count Vardak: 1 = giant bat).
    pub form: u8,
    /// Charging (Sir Malgrave, spring-heeled jacks): moves much faster while this lasts.
    pub rush: f32,
    /// Speed from the tide (tides.rs): sea kinds swim faster in the flood, land kinds wade.
    pub tide: f32,
    /// A one-off order to the game (the Clockmaker: 1 = rewind the player; the Archivist:
    /// 2 = file the player away elsewhere). The game clears it.
    pub cue: u8,
    /// An ordinal marching in step with a living marshal (faster, shielded).
    pub drilled: bool,
    /// Its place in a squad's ranks: (across, behind the marshal) in tiles; (0, 0) = no place.
    pub post: (f32, f32),
    /// Seconds a squad stays broken after its marshal falls (takes extra damage).
    pub broken: f32,
    /// The inquisitor's Brand of Judgment (seconds left).
    pub brand_t: f32,
    /// Her holy fire: damage per second, seconds left. It ignores fire resistance.
    pub holy: f32,
    pub holy_t: f32,
    /// A super unique (side.rs SUPERS index + 1; 0 = not one), and whether it has shouted its line yet.
    pub superu: u8,
    pub shouted: bool,
    /// Asleep (the Ember Wyrm on its hoard, dragon.rs): drawn curled up.
    pub asleep: bool,
    /// A random errand's own (errands.rs: 1 its target, 2 a guard, 3 a siege wave).
    pub errand: u8,
    /// Neutral (features.rs): a group that stands by until one of them is hurt (0: hostile).
    pub neutral: u8,
}

impl Mob {
    pub fn new(kind: Kind, x: f32, y: f32, tier: f32, rng: &mut Rng) -> Self {
        let d = def(kind);
        // Past Act 3 (tier 6) heroes' power outgrows a straight line (balance pass 2026-10-07,
        // scripts/balance-bench.sh): monsters get that much tougher again, and a little harder-hitting.
        let late = (tier / 6.0).max(1.0);
        let hp = d.hp * tier * late;
        let k = tier.powf(0.8) * late.powf(0.6);
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
            burned: 0.0,
            path: vec![],
            repath: 0.0,
            wander: (0.0, 0.0, rng.f() * 2.0),
            flee: 0.0,
            special: 4.0,
            special2: 6.0,
            enraged: false,
            home: (x, y),
            tier,
            rank: Rank::Normal,
            mods: 0,
            name: None,
            charm: 0.0,
            thrall: false,
            frost: 0.0,
            frozen: 0.0,
            marked: 0.0,
            bleed: 0.0,
            bleed_t: 0.0,
            sunder: 0.0,
            slow_t: 0.0,
            poison: 0.0,
            poison_t: 0.0,
            plagued: false,
            invuln: 0.0,
            form: 0,
            rush: 0.0,
            tide: 1.0,
            cue: 0,
            drilled: false,
            post: (0.0, 0.0),
            broken: 0.0,
            brand_t: 0.0,
            holy: 0.0,
            holy_t: 0.0,
            superu: 0,
            shouted: false,
            asleep: false,
            errand: 0,
            neutral: 0,
        }
    }

    /// Turns a normal monster into a champion, an elite leader or one of its minions.
    pub fn promote(&mut self, rank: Rank, mods: u8, name: Option<String>) {
        let (hp, dmg, xp) = match rank {
            Rank::Normal => (1.0, 1.0, 1.0),
            Rank::Champion => (3.0, 1.5, 3.0),
            Rank::Elite => (4.0, 1.6, 5.0),
            Rank::Minion => (1.8, 1.2, 1.5),
        };
        let hp = hp * if mods & M_STONE != 0 { 1.6 } else { 1.0 };
        let dmg = dmg * if mods & M_STRONG != 0 { 1.6 } else { 1.0 };
        self.rank = rank;
        self.mods = mods;
        self.name = name;
        self.max_hp *= hp;
        self.hp = self.max_hp;
        self.dmg = (self.dmg.0 * dmg, self.dmg.1 * dmg);
        self.xp *= xp;
        if mods & M_FAST != 0 {
            self.speed *= 1.45;
            self.cooldown *= 0.7;
            self.windup *= 0.8;
        } else if rank != Rank::Minion {
            self.speed *= 1.1;
        }
    }

    pub fn label(&self) -> String {
        if crate::breakables::is_prop(self.kind) {
            return crate::breakables::label(self.kind, self.form).to_string();
        }
        match &self.name {
            Some(n) => n.clone(),
            None => def(self.kind).label.to_string(),
        }
    }

    /// Name colour on the health bar (D2: blue champions, gold elites).
    pub fn name_col(&self) -> u32 {
        match self.rank {
            Rank::Champion => 0x7090ff,
            Rank::Elite => 0xd8a850,
            _ => 0xffffff,
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
    /// Ice bolts (wraiths, the Rime Witch): they chill you.
    Ice,
    /// The frost giant's thrown ice boulder.
    Boulder,
    /// Green grave-fire (banshees, wisps, cultists, the lich duchess).
    Necro,
    /// Lord Ossric's bone spears.
    Bone,
    /// Count Vardak's blood bolts.
    Blood,
    /// Scalding steam (inquisitors, the Cantor's sound waves).
    Steam,
    /// Spinning brass cogs and clock hands (the Clockmaker).
    Gear,
    /// Blue arcs from ordinals and the Archivist.
    Spark,
    /// A siren's song: teal notes.
    Song,
    /// Ink: it blinds you for a moment.
    Ink,
    /// Admiral Dregmoor's cannonballs.
    Cannon,
    /// The Leviathan's pressure beam.
    Tide,
    /// Spears and beams of light (seraphs, the ophanim, Vael, Solanthos).
    Light,
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
    /// Freezing ground (Rime Witch, dragon breath): hurts and chills while you stand in it.
    Frost,
    /// A telegraphed icicle falling from the ceiling.
    Icicle,
    /// The frost giant's ground slam.
    Quake,
    /// The Forgemother's molten slag pool.
    Slag,
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
        let overworld = self.level.overland();
        let n = self.mobs.len();
        let mut hits: Vec<(f32, usize)> = vec![];
        let mut aggro_at: Vec<(f32, f32)> = vec![];
        let mut spawns: Vec<(Kind, f32, f32, f32)> = vec![];
        let mut shots: Vec<(f32, f32, f32, f32, f32, ShotKind)> = vec![];
        let mut hazards: Vec<Hazard> = vec![];
        let mut texts: Vec<(f32, f32, &'static str)> = vec![];
        let summons = self.mobs.iter().filter(|m| m.alive() && !m.boss && m.home.0 < -999.0).count();
        // Ordinals near a living marshal march in step.
        // Each marshal faces you; its squad keeps its ranks behind it.
        let marshals: Vec<(f32, f32, f32, f32)> = self
            .mobs
            .iter()
            .filter(|m| m.kind == Kind::Marshal && m.alive())
            .map(|m| {
                let (fx, fy) = (px - m.x, py - m.y);
                let l = (fx * fx + fy * fy).sqrt().max(0.01);
                (m.x, m.y, fx / l, fy / l)
            })
            .collect();
        for m in self.mobs.iter_mut().filter(|m| matches!(m.kind, Kind::Ordinal | Kind::Prism)) {
            m.drilled = m.broken <= 0.0 && marshals.iter().any(|&(x, y, ..)| (m.x - x).powi(2) + (m.y - y).powi(2) < 64.0);
        }
        // The lockstep beat: march for 40 ticks, halt (and fire together) for 20.
        let halt = self.tick % 60 >= 40;
        // Charmed monsters hunt the hostile ones.
        let foes: Vec<(usize, f32, f32, f32)> =
            self.mobs.iter().enumerate().filter(|(_, m)| m.alive() && m.charm <= 0.0 && !crate::breakables::is_prop(m.kind)).map(|(i, m)| (i, m.x, m.y, m.r)).collect();
        let mut ally_hits: Vec<(usize, f32)> = vec![];
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
                m.burned += DT;
                m.burn -= DT;
                m.hp -= 3.0 * m.tier.max(1.0) * fire_taken(m.kind) * DT;
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
            } else {
                m.burned = 0.0;
                // Ice trolls and werewolves knit back together unless they're burning.
                if matches!(m.kind, Kind::IceTroll | Kind::Werewolf) && m.hp < m.max_hp {
                    m.hp = (m.hp + m.max_hp * 0.03 * DT).min(m.max_hp);
                }
            }
            // Breakables have no mind: nothing below applies to them.
            if crate::breakables::is_prop(m.kind) {
                m.moving = false;
                continue;
            }
            // Gearwraiths flicker out of phase now and then.
            if m.kind == Kind::Gearwraith && m.alive() && m.invuln <= 0.0 && rv < DT / 4.0 {
                m.invuln = 0.8;
            }
            m.invuln = (m.invuln - DT).max(0.0);
            m.rush = (m.rush - DT).max(0.0);
            m.broken = (m.broken - DT).max(0.0);
            if m.stun > 0.0 {
                m.stun -= DT;
                m.moving = false;
                continue;
            }
            // Neutral ones stand by (features.rs turns the group on you when one is hurt).
            if m.neutral > 0 {
                m.moving = false;
                continue;
            }
            if m.charm > 0.0 && m.alive() {
                m.charm -= DT;
                if m.charm <= 0.0 {
                    m.charm = 0.0;
                    if m.thrall {
                        // The thrall crumbles back into a corpse.
                        m.hp = 0.0;
                        m.state = MobState::Dead(0.0);
                    } else {
                        m.state = MobState::Chase;
                    }
                    continue;
                }
                // Hunt the nearest hostile monster, or stay near you.
                let target = foes.iter().filter(|f| f.0 != i).min_by(|a, b| {
                    let da = (a.1 - m.x).powi(2) + (a.2 - m.y).powi(2);
                    let db = (b.1 - m.x).powi(2) + (b.2 - m.y).powi(2);
                    da.partial_cmp(&db).unwrap()
                });
                let (tx, ty, reach) = match target {
                    Some(&(_, fx, fy, fr)) if (fx - m.x).powi(2) + (fy - m.y).powi(2) < 100.0 => (fx, fy, m.reach + fr),
                    _ => (px, py, 2.0),
                };
                let (dx, dy) = (tx - m.x, ty - m.y);
                let dist = (dx * dx + dy * dy).sqrt().max(0.01);
                m.dir = iso::dir8(dx, dy);
                if dist > reach {
                    let (mut x, mut y) = (m.x, m.y);
                    move_circle(&self.d, &mut x, &mut y, dx / dist * m.speed * DT, dy / dist * m.speed * DT, m.r);
                    m.x = x;
                    m.y = y;
                    m.moving = true;
                    m.anim_t += DT;
                } else if let (Some(&(j, ..)), true) = (target, m.cd <= 0.0) {
                    m.cd = m.cooldown;
                    ally_hits.push((j, m.dmg.0 + (m.dmg.1 - m.dmg.0) * rv));
                }
                continue;
            }
            let (dx, dy) = (px - m.x, py - m.y);
            let dist = (dx * dx + dy * dy).sqrt();
            m.moving = false;
            let r = m.r;
            match m.state {
                MobState::Idle => {
                    let range = if m.boss { 10.0 } else if m.kind == Kind::Anglerlurk { 3.5 } else { 8.5 };
                    if player_alive && !player_safe && dist < range && self.d.los(m.x, m.y, px, py) {
                        m.state = MobState::Chase;
                        if m.kind == Kind::Anglerlurk {
                            // Out from behind the lure.
                            m.rush = 0.6;
                            texts.push((m.x, m.y, "!"));
                        }
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
                    if m.kind == Kind::SpringJack && m.rush <= 0.0 && (2.5..7.0).contains(&dist) && rv < DT / 1.5 {
                        m.rush = 0.45;
                    }
                    if m.kind == Kind::Merrow && m.rush <= 0.0 && (2.0..5.5).contains(&dist) && rv < DT / 2.5 {
                        m.rush = 0.35;
                    }
                    // A siren's song draws you in.
                    if m.kind == Kind::Siren {
                        m.special -= DT;
                        if m.special <= 0.0 && (3.0..9.0).contains(&dist) && self.d.los(m.x, m.y, px, py) {
                            m.special = 7.0 + rv * 3.0;
                            m.cue = 3;
                            texts.push((m.x, m.y, "~ COME... ~"));
                        }
                    }
                    // Crows close in and circle you, pecking as they pass.
                    if m.kind == Kind::ClockCrow && dist < m.reach + 1.2 {
                        if dist < m.reach + 0.3 && m.cd <= 0.0 {
                            m.state = MobState::Attack(m.windup);
                        }
                        let side = if i % 2 == 0 { 1.0 } else { -1.0 };
                        let want = 1.0 + (i % 3) as f32 * 0.35;
                        let (ux, uy) = (dx / dist.max(0.01), dy / dist.max(0.01));
                        let pull = (dist - want).clamp(-1.0, 1.0);
                        let (vx, vy) = (-uy * side + ux * pull, ux * side + uy * pull);
                        let sp = m.speed * 0.7 * if m.slow_t > 0.0 { 0.35 } else { 1.0 };
                        let (mut x, mut y) = (m.x, m.y);
                        move_circle(&self.d, &mut x, &mut y, vx * sp * DT, vy * sp * DT, r);
                        m.x = x;
                        m.y = y;
                        m.dir = iso::dir8(vx, vy);
                        m.moving = true;
                        continue;
                    }
                    // Ordinal squads march in step: to their places in the ranks on the beat, then
                    // halt and fire together. The marshal halts with them.
                    let in_ranks = m.drilled && m.post != (0.0, 0.0);
                    if (in_ranks || m.kind == Kind::Marshal) && halt && dist < 9.5 {
                        m.dir = iso::dir8(dx, dy);
                        if m.cd <= 0.0 && dist <= 9.0 && self.d.los(m.x, m.y, px, py) {
                            m.state = MobState::Attack(m.windup);
                        }
                        continue;
                    }
                    if in_ranks {
                        let near = marshals.iter().min_by(|a, b| {
                            let da = (a.0 - m.x).powi(2) + (a.1 - m.y).powi(2);
                            let db = (b.0 - m.x).powi(2) + (b.1 - m.y).powi(2);
                            da.partial_cmp(&db).unwrap()
                        });
                        if let Some(&(mx, my, fx, fy)) = near {
                            let (sx, sy) = (mx - fx * m.post.1 - fy * m.post.0, my - fy * m.post.1 + fx * m.post.0);
                            let (gx, gy) = (sx - m.x, sy - m.y);
                            let gl = (gx * gx + gy * gy).sqrt();
                            if gl < 0.15 {
                                m.dir = iso::dir8(dx, dy);
                                continue;
                            }
                            let sp = m.speed * 1.3 * if m.slow_t > 0.0 { 0.35 } else { 1.0 } * (1.0 - 0.35 * m.frost);
                            let step = (sp * DT).min(gl);
                            let (mut x, mut y) = (m.x, m.y);
                            move_circle(&self.d, &mut x, &mut y, gx / gl * step, gy / gl * step, r);
                            m.x = x;
                            m.y = y;
                            m.dir = iso::dir8(gx, gy);
                            m.moving = true;
                            m.anim_t += DT * sp / 1.6;
                            continue;
                        }
                    }
                    let los = self.d.los(m.x, m.y, px, py);
                    // Where to go this tick.
                    let mut away = false;
                    if m.flee > 0.0 {
                        away = true;
                    } else if m.ranged {
                        let keep = if matches!(m.kind, Kind::HexWarden | Kind::RimeWitch | Kind::Grimhilde) { 3.5 } else { 2.8 };
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
                    let (mut ux, mut uy) = (ddx / l, ddy / l);
                    if m.kind == Kind::ClockCrow {
                        // Crows jink from side to side as they come.
                        let a = (tick as f32 * 0.12 + i as f32 * 1.7).sin() * 0.9;
                        (ux, uy) = (ux * a.cos() - uy * a.sin(), ux * a.sin() + uy * a.cos());
                    }
                    let speed = m.speed * if m.enraged { 1.25 } else { 1.0 } * if m.flee > 0.0 { 1.1 } else { 1.0 } * if m.rush > 0.0 { 3.2 } else { 1.0 } * if m.drilled { 1.3 } else { 1.0 } * (1.0 - 0.35 * m.frost) * if m.slow_t > 0.0 { 0.35 } else { 1.0 } * m.tide;
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
                            Kind::RimeWitch => {
                                let n = if m.enraged { 5 } else { 3 };
                                for k in 0..n {
                                    let a = uy.atan2(ux) + (k as f32 - (n - 1) as f32 * 0.5) * 0.24;
                                    shots.push((m.x, m.y, a.cos() * 7.5, a.sin() * 7.5, dmg, ShotKind::Ice));
                                }
                            }
                            Kind::IceWraith => shots.push((m.x, m.y, ux * 7.5, uy * 7.5, dmg, ShotKind::Ice)),
                            Kind::Grimhilde => {
                                let n = if m.enraged { 5 } else { 3 };
                                for k in 0..n {
                                    let a = uy.atan2(ux) + (k as f32 - (n - 1) as f32 * 0.5) * 0.24;
                                    shots.push((m.x, m.y, a.cos() * 7.0, a.sin() * 7.0, dmg, ShotKind::Necro));
                                }
                            }
                            Kind::Banshee | Kind::Wisp | Kind::Cultist | Kind::WellWitch | Kind::Elspeth | Kind::Shade => shots.push((m.x, m.y, ux * 7.0, uy * 7.0, dmg, ShotKind::Necro)),
                            Kind::Inquisitor => {
                                for k in 0..3 {
                                    let a = uy.atan2(ux) + (k as f32 - 1.0) * 0.18;
                                    shots.push((m.x, m.y, a.cos() * 6.0, a.sin() * 6.0, dmg * 0.6, ShotKind::Steam));
                                }
                            }
                            Kind::Ordinal | Kind::Archivist => shots.push((m.x, m.y, ux * 9.5, uy * 9.5, dmg, ShotKind::Spark)),
                            Kind::Prism => {
                                for k in 0..3 {
                                    let a = uy.atan2(ux) + (k as f32 - 1.0) * 0.22;
                                    shots.push((m.x, m.y, a.cos() * 9.0, a.sin() * 9.0, dmg * 0.7, ShotKind::Spark));
                                }
                            }
                            Kind::Marshal => shots.push((m.x, m.y, ux * 6.5, uy * 6.5, dmg, ShotKind::Gear)),
                            Kind::Jelly => shots.push((m.x, m.y, ux * 8.0, uy * 8.0, dmg, ShotKind::Spark)),
                            Kind::Ophanim => {
                                // Three quick beams, one blow's worth between them (they were 1.5x, and a pack of
                                // five shredded ranged heroes: balance pass 2026-10-07).
                                for k in 0..3 {
                                    shots.push((m.x, m.y, ux * (10.0 + k as f32), uy * (10.0 + k as f32), dmg * 0.34, ShotKind::Light));
                                }
                            }
                            Kind::StormDrake | Kind::Thunderbird => {
                                for k in [-1.0f32, 0.0, 1.0] {
                                    let a = uy.atan2(ux) + k * 0.2;
                                    shots.push((m.x, m.y, a.cos() * 9.0, a.sin() * 9.0, dmg * 0.7, ShotKind::Spark));
                                }
                            }
                            Kind::Zealot => shots.push((m.x, m.y, ux * 7.0, uy * 7.0, dmg, ShotKind::Ash)),
                            Kind::Tempest => {
                                let n = if m.enraged { 7 } else { 5 };
                                for k in 0..n {
                                    let a = uy.atan2(ux) + (k as f32 - (n - 1) as f32 * 0.5) * 0.18;
                                    shots.push((m.x, m.y, a.cos() * 9.5, a.sin() * 9.5, dmg, ShotKind::Spark));
                                }
                            }
                            Kind::OphanPrime => {
                                for k in 0..4 {
                                    shots.push((m.x, m.y, ux * (9.0 + k as f32 * 1.2), uy * (9.0 + k as f32 * 1.2), dmg * 0.5, ShotKind::Light));
                                }
                            }
                            Kind::Siren => shots.push((m.x, m.y, ux * 6.5, uy * 6.5, dmg, ShotKind::Song)),
                            Kind::InkHorror => shots.push((m.x, m.y, ux * 6.0, uy * 6.0, dmg, ShotKind::Ink)),
                            Kind::Nacre => {
                                let n = if m.enraged { 5 } else { 3 };
                                for k in 0..n {
                                    let a = uy.atan2(ux) + (k as f32 - (n - 1) as f32 * 0.5) * 0.26;
                                    shots.push((m.x, m.y, a.cos() * 7.0, a.sin() * 7.0, dmg, ShotKind::Song));
                                }
                            }
                            Kind::Cantor => {
                                // A ring of sound with a gap that turns: dodge into the gap.
                                let gap = (tick as f32 * 0.02).rem_euclid(std::f32::consts::TAU);
                                for k in 0..16 {
                                    let a = k as f32 / 16.0 * std::f32::consts::TAU;
                                    let off = (a - gap).rem_euclid(std::f32::consts::TAU);
                                    if off < 0.8 {
                                        continue;
                                    }
                                    shots.push((m.x, m.y, a.cos() * 5.5, a.sin() * 5.5, dmg, ShotKind::Steam));
                                }
                            }
                            _ => shots.push((m.x, m.y, ux * 9.0, uy * 9.0, dmg, ShotKind::Arrow)),
                        }
                    } else if player_alive && dist < m.reach + 0.4 {
                        hits.push((dmg, i));
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
        for (j, dmg) in ally_hits {
            if self.mobs[j].alive() {
                self.mobs[j].hp -= dmg;
                self.mobs[j].flash = 0.1;
                if self.mobs[j].state == MobState::Idle {
                    self.mobs[j].state = MobState::Chase;
                }
                if self.mobs[j].hp <= 0.0 {
                    self.kill(j);
                }
            }
        }
        for (dmg, i) in hits {
            let before = self.p.hp;
            let src = self.mobs[i].kind;
            self.hurt_by(dmg, Some(src));
            if def(self.mobs[i].kind).chills {
                self.chill(1.4);
            }
            // A Vampiric rift: monsters heal on every blow they land.
            if self.rift_has(crate::endgame::RiftMod::Vampiric) {
                let m = &mut self.mobs[i];
                m.hp = (m.hp + dmg * 2.0).min(m.max_hp);
            }
            if src == Kind::Harpy {
                let (hx, hy) = (self.mobs[i].x, self.mobs[i].y);
                self.shove_player((hx, hy), 1.3);
            }
            let dealt = (before - self.p.hp).max(0.0);
            let mods = self.mobs[i].mods;
            if mods & M_VAMPIRE != 0 {
                let m = &mut self.mobs[i];
                m.hp = (m.hp + dealt * 1.5).min(m.max_hp);
            }
            if mods & M_MANABURN != 0 {
                self.p.mana = (self.p.mana - dealt * 1.5).max(0.0);
                let (x, y) = (self.p.x, self.p.y);
                self.spray_at(x, y, PKind::Magic, 12.0);
            }
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
                    // Breakables don't budge: whatever bumps into one is pushed off it.
                    let (pi, pj) = (crate::breakables::is_prop(self.mobs[i].kind), crate::breakables::is_prop(self.mobs[j].kind));
                    if pi && pj {
                        continue;
                    }
                    let d = d2.sqrt();
                    let push = (min - d) * if pi || pj { 1.0 } else { 0.5 };
                    let (ux, uy) = (dx / d * push, dy / d * push);
                    let (ri, rj) = (self.mobs[i].r, self.mobs[j].r);
                    if !pi {
                        let (mut x, mut y) = (self.mobs[i].x, self.mobs[i].y);
                        move_circle(&self.d, &mut x, &mut y, -ux, -uy, ri);
                        self.mobs[i].x = x;
                        self.mobs[i].y = y;
                    }
                    if !pj {
                        let (mut x, mut y) = (self.mobs[j].x, self.mobs[j].y);
                        move_circle(&self.d, &mut x, &mut y, ux, uy, rj);
                        self.mobs[j].x = x;
                        self.mobs[j].y = y;
                    }
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
                let size = if matches!(s.kind, ShotKind::Boulder | ShotKind::Cannon) { 0.4 } else { 0.12 };
                if (s.x - px).powi(2) + (s.y - py).powi(2) < (PLAYER_R + size).powi(2) {
                    hits.push((s.dmg, s.kind));
                    s.life = 0.0;
                    break;
                }
            }
        }
        self.shots.retain(|s| s.life > 0.0);
        for (d, kind) in hits {
            // Necromantic and hex bolts come from cursed hands.
            self.hurt_cursed = matches!(kind, ShotKind::Necro | ShotKind::Hex);
            self.hurt_player(d);
            self.hurt_cursed = false;
            match kind {
                ShotKind::Ice => self.chill(2.0),
                ShotKind::Boulder => {
                    self.chill(1.0);
                    self.shake = self.shake.max(0.6);
                }
                ShotKind::Cannon => self.shake = self.shake.max(0.5),
                ShotKind::Ink => {
                    self.blind_t = 3.5;
                    self.floater(px, py, "BLINDED!".into(), rgb(0x9070c0));
                }
                _ => {}
            }
        }
    }

    /// Poison pools and ash novas.
    pub(crate) fn update_hazards(&mut self) {
        let (px, py) = (self.p.x, self.p.y);
        let mut dmg = 0.0;
        let mut burst = 0.0;
        let mut frozen = false;
        let mut quakes = vec![];
        for h in self.hazards.iter_mut() {
            h.t += DT;
            let inside = (h.x - px).powi(2) + (h.y - py).powi(2) < h.r * h.r;
            if h.kind == HazardKind::Frost && inside && h.t >= h.warn && h.t < h.warn + h.live {
                frozen = true;
            }
            if matches!(h.kind, HazardKind::Icicle | HazardKind::Quake) && h.t >= h.warn && !h.fired {
                quakes.push((h.x, h.y, h.kind));
            }
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
        for (x, y, kind) in quakes {
            self.sfx.push(if kind == HazardKind::Quake { Sfx::Boom } else { Sfx::Hit });
            self.shake = self.shake.max(if kind == HazardKind::Quake { 0.8 } else { 0.3 });
            for _ in 0..if kind == HazardKind::Quake { 30 } else { 10 } {
                self.spray_at(x, y, PKind::Frost, 3.0);
            }
        }
        if frozen {
            self.chill(0.5);
        }
        self.hazards.retain(|h| h.t < h.warn + h.live.max(0.3));
        if burst > 0.0 {
            // Ground hazards are a boss's (or its curse's) doing.
            self.hurt_cursed = true;
            self.hurt_player(burst);
            self.hurt_cursed = false;
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
        Kind::Astronomer => {
            // He draws a constellation around you, and its stars fall; a fan of starlight; his ophanim.
            if m.special <= 0.0 && dist < 14.0 {
                m.special = if m.enraged { 3.0 } else { 4.2 };
                let n = if m.enraged { 7 } else { 5 };
                let a0 = rng.f() * std::f32::consts::TAU;
                hazards.push(Hazard { x: px, y: py, r: 1.0, warn: 1.1, live: 0.0, dps: 0.0, burst: 22.0 * m.tier.powf(0.8), t: 0.0, fired: false, kind: HazardKind::Nova });
                for k in 0..n {
                    let a = a0 + k as f32 / n as f32 * std::f32::consts::TAU;
                    hazards.push(Hazard { x: px + a.cos() * 2.6, y: py + a.sin() * 2.6, r: 1.0, warn: 1.1, live: 0.0, dps: 0.0, burst: 22.0 * m.tier.powf(0.8), t: 0.0, fired: false, kind: HazardKind::Nova });
                }
                texts.push((m.x, m.y, "THE STARS ALIGN!"));
            } else if m.special2 <= 0.0 && dist < 12.0 {
                m.special2 = if m.enraged { 2.6 } else { 3.6 };
                let base = (py - m.y).atan2(px - m.x);
                for k in -2..=2 {
                    let a = base + k as f32 * 0.18;
                    shots.push((m.x, m.y, a.cos() * 7.5, a.sin() * 7.5, 13.0 * m.tier.powf(0.8), ShotKind::Ash));
                }
                if summons < 4 && rng.chance(0.3) {
                    around(rng, 2, Kind::Ophanim, m.tier, spawns);
                }
            }
        }
        Kind::Barnacle => {
            // A claw slam, and the grotto's merrow come running.
            if m.special <= 0.0 && dist < 3.2 {
                m.special = if m.enraged { 3.0 } else { 4.4 };
                hazards.push(Hazard { x: m.x, y: m.y, r: 2.6, warn: 0.9, live: 0.0, dps: 0.0, burst: 26.0 * m.tier.powf(0.8), t: 0.0, fired: false, kind: HazardKind::Quake });
                texts.push((m.x, m.y, "CLACK!"));
            }
            if m.special2 <= 0.0 {
                m.special2 = if m.enraged { 8.0 } else { 12.0 };
                if summons < 5 {
                    around(rng, 2, Kind::Merrow, m.tier, spawns);
                }
            }
        }
        Kind::JunkGolem => {
            // Both fists down, and loose scarabs skittering out of the scrap.
            if m.special <= 0.0 && dist < 3.4 {
                m.special = if m.enraged { 3.0 } else { 4.4 };
                hazards.push(Hazard { x: m.x, y: m.y, r: 2.8, warn: 0.9, live: 0.0, dps: 0.0, burst: 24.0 * m.tier.powf(0.8), t: 0.0, fired: false, kind: HazardKind::Quake });
                texts.push((m.x, m.y, "CRUSH!"));
            }
            if m.special2 <= 0.0 && m.boss {
                m.special2 = if m.enraged { 8.0 } else { 11.0 };
                if summons < 6 {
                    around(rng, 3, Kind::Scarab, m.tier, spawns);
                }
            }
        }
        Kind::Gravedigger => {
            // The shovel comes down; the graves give up their dead.
            if m.special <= 0.0 && dist < 3.2 {
                m.special = if m.enraged { 3.2 } else { 4.6 };
                hazards.push(Hazard { x: m.x, y: m.y, r: 2.6, warn: 0.9, live: 0.0, dps: 0.0, burst: 20.0 * m.tier.powf(0.8), t: 0.0, fired: false, kind: HazardKind::Quake });
                texts.push((m.x, m.y, "DIG! DIG!"));
            }
            if m.special2 <= 0.0 {
                m.special2 = if m.enraged { 8.0 } else { 11.0 };
                if summons < 6 {
                    around(rng, 3, Kind::Ghoul, m.tier, spawns);
                    texts.push((m.x, m.y, "UP YOU GET, FRESH ONES!"));
                }
            }
        }
        Kind::Elspeth => {
            // Will-o'-wisps from the manor's cold hearth.
            if m.special2 <= 0.0 {
                m.special2 = 9.0;
                if summons < 4 {
                    around(rng, 2, Kind::Wisp, m.tier, spawns);
                    texts.push((m.x, m.y, "LEAVE MY HOUSE!"));
                }
            }
        }
        Kind::Hrolf => {
            // A ground slam with his frozen axe, and ice shards shaken from the frozen rigging.
            if m.special <= 0.0 && dist < 3.0 {
                m.special = if m.enraged { 3.5 } else { 5.0 };
                hazards.push(Hazard { x: m.x, y: m.y, r: 2.4, warn: 0.9, live: 0.0, dps: 0.0, burst: 18.0 * m.tier.powf(0.8), t: 0.0, fired: false, kind: HazardKind::Quake });
                texts.push((m.x, m.y, "FOR THE JARLS OF OLD!"));
            }
            if m.special2 <= 0.0 && dist < 10.0 {
                m.special2 = if m.enraged { 4.5 } else { 6.5 };
                for _ in 0..4 {
                    let (a, rr) = (rng.f() * std::f32::consts::TAU, rng.rf(0.0, 2.4));
                    let (x, y) = (px + a.cos() * rr, py + a.sin() * rr);
                    if !d.blocked(x, y, 0.2) {
                        hazards.push(Hazard { x, y, r: 0.9, warn: 1.1, live: 0.0, dps: 0.0, burst: 13.0 * m.tier.powf(0.8), t: 0.0, fired: false, kind: HazardKind::Icicle });
                    }
                }
            }
            if m.enraged && summons < 4 && rng.chance(DT / 9.0) {
                around(rng, 2, Kind::Raider, m.tier, spawns);
                texts.push((m.x, m.y, "TO ME, MY CREW!"));
            }
        }
        Kind::FireWyrm => {
            // A torrent of fire down its line of sight, burning pools where it lands, and a tail sweep up close.
            if m.special <= 0.0 && dist < 10.0 {
                m.special = if m.enraged { 2.6 } else { 3.6 };
                let a = (py - m.y).atan2(px - m.x);
                for k in 0..8 {
                    let dd = 1.8 + k as f32 * 1.1;
                    for side in [-1.0f32, 0.0, 1.0] {
                        if k < 2 && side != 0.0 {
                            continue;
                        }
                        let aa = a + side * 0.16 * (k as f32 * 0.5 + 0.5);
                        let (x, y) = (m.x + aa.cos() * dd, m.y + aa.sin() * dd);
                        if d.blocked(x, y, 0.1) {
                            continue;
                        }
                        hazards.push(Hazard { x, y, r: 0.75 + k as f32 * 0.06, warn: 0.6 + k as f32 * 0.05, live: 2.5, dps: 30.0 * m.tier.powf(0.8), burst: 0.0, t: 0.0, fired: false, kind: HazardKind::Slag });
                    }
                }
                texts.push((m.x, m.y, "FIRE!"));
            }
            if m.special2 <= 0.0 && dist < 3.4 {
                m.special2 = if m.enraged { 2.5 } else { 3.5 };
                hazards.push(Hazard { x: m.x, y: m.y, r: 3.0, warn: 0.7, live: 0.0, dps: 0.0, burst: 40.0 * m.tier.powf(0.8), t: 0.0, fired: false, kind: HazardKind::Quake });
                texts.push((m.x, m.y, "TAIL SWEEP!"));
            }
        }
        Kind::WellWitch => {
            // Pools of foul well-water under you, and the drowned dead climbing out after her.
            if m.special <= 0.0 && dist < 9.0 {
                m.special = if m.enraged { 3.5 } else { 5.0 };
                hazards.push(Hazard { x: px, y: py, r: 1.2, warn: 0.9, live: 5.0, dps: 6.0 * m.tier, burst: 0.0, t: 0.0, fired: false, kind: HazardKind::Poison });
            }
            if m.special2 <= 0.0 {
                m.special2 = if m.enraged { 8.0 } else { 11.0 };
                if summons < 6 {
                    around(rng, 3, Kind::Zombie, m.tier, spawns);
                    texts.push((m.x, m.y, "UP, MY DROWNED DARLINGS!"));
                }
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
        Kind::FrostGiant => {
            // Ground slam around itself, and an ice boulder thrown at range.
            if m.special <= 0.0 && dist < 3.2 {
                m.special = if m.enraged { 3.5 } else { 5.0 };
                hazards.push(Hazard { x: m.x, y: m.y, r: 2.6, warn: 1.0, live: 0.0, dps: 0.0, burst: 20.0 * m.tier.powf(0.8), t: 0.0, fired: false, kind: HazardKind::Quake });
                texts.push((m.x, m.y, "THE GROUND SHAKES!"));
            }
            if m.special2 <= 0.0 && (3.0..11.0).contains(&dist) {
                m.special2 = if m.enraged { 3.0 } else { 4.5 };
                let a = (py - m.y).atan2(px - m.x);
                shots.push((m.x, m.y, a.cos() * 6.0, a.sin() * 6.0, 15.0 * m.tier.powf(0.8), ShotKind::Boulder));
            }
            if m.enraged && summons < 4 && rng.chance(DT / 10.0) {
                around(rng, 2, Kind::Raider, m.tier, spawns);
                texts.push((m.x, m.y, "TO ME, RAIDERS!"));
            }
        }
        Kind::YetiMatriarch => {
            // Shakes icicles loose over you, and calls her brood.
            if m.special <= 0.0 && dist < 10.0 {
                m.special = if m.enraged { 4.0 } else { 6.0 };
                for _ in 0..if m.enraged { 6 } else { 4 } {
                    let (a, rr) = (rng.f() * std::f32::consts::TAU, rng.rf(0.0, 2.4));
                    let (x, y) = (px + a.cos() * rr, py + a.sin() * rr);
                    if !d.blocked(x, y, 0.2) {
                        hazards.push(Hazard { x, y, r: 0.9, warn: 1.1, live: 0.0, dps: 0.0, burst: 13.0 * m.tier.powf(0.8), t: 0.0, fired: false, kind: HazardKind::Icicle });
                    }
                }
                texts.push((m.x, m.y, "ROOAAAR!"));
            }
            if m.special2 <= 0.0 {
                m.special2 = 11.0;
                if summons < 6 {
                    if rng.chance(0.5) {
                        around(rng, 2, Kind::Yeti, m.tier * 0.8, spawns);
                    } else {
                        around(rng, 3, Kind::FrostWolf, m.tier, spawns);
                    }
                    texts.push((m.x, m.y, "THE BROOD COMES!"));
                }
            }
        }
        Kind::Ossric => {
            // Volleys of bone spears, and the dead rise around him.
            if m.special <= 0.0 && dist < 10.0 {
                m.special = if m.enraged { 2.6 } else { 3.6 };
                let base = (py - m.y).atan2(px - m.x);
                let n = if m.enraged { 5 } else { 3 };
                for k in 0..n {
                    let a = base + (k as f32 - (n - 1) as f32 * 0.5) * 0.2;
                    shots.push((m.x, m.y, a.cos() * 8.5, a.sin() * 8.5, 11.0 * m.tier.powf(0.8), ShotKind::Bone));
                }
            }
            if m.special2 <= 0.0 {
                m.special2 = 10.0;
                if summons < 6 {
                    around(rng, 3, Kind::Skeleton, m.tier, spawns);
                    texts.push((m.x, m.y, "RISE, MY VASSALS!"));
                }
            }
        }
        Kind::Grimhilde => {
            if m.special <= 0.0 && dist < 2.5 {
                m.special = 5.0;
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
            if m.special2 <= 0.0 && dist < 10.0 {
                m.special2 = if m.enraged { 3.5 } else { 5.0 };
                hazards.push(Hazard { x: px, y: py, r: 1.3, warn: 0.8, live: 4.5, dps: 7.0 * m.tier, burst: 0.0, t: 0.0, fired: false, kind: HazardKind::Poison });
                if m.enraged && summons < 4 {
                    around(rng, 2, Kind::Banshee, m.tier, spawns);
                }
            }
        }
        Kind::Malgrave => {
            // Shield charge from range, ground slam up close, the dead knights answer.
            if m.special <= 0.0 && (3.0..9.0).contains(&dist) {
                m.special = if m.enraged { 4.5 } else { 6.0 };
                m.rush = 0.9;
                texts.push((m.x, m.y, "CHARGE!"));
            }
            if m.special2 <= 0.0 && dist < 3.0 {
                m.special2 = if m.enraged { 3.5 } else { 5.0 };
                hazards.push(Hazard { x: m.x, y: m.y, r: 2.6, warn: 0.9, live: 0.0, dps: 0.0, burst: 22.0 * m.tier.powf(0.8), t: 0.0, fired: false, kind: HazardKind::Quake });
            }
            if m.enraged && summons < 4 && rng.chance(DT / 9.0) {
                around(rng, 2, Kind::Archer, m.tier, spawns);
                texts.push((m.x, m.y, "TO ME, MY KNIGHTS!"));
            }
        }
        Kind::Vardak => {
            let frac = m.hp / m.max_hp;
            // Phase 3: the giant bat.
            if frac < 0.33 && m.form == 0 {
                m.form = 1;
                m.speed *= 1.6;
                m.reach = 1.7;
                m.invuln = 1.0;
                texts.push((m.x, m.y, "THE COUNT TAKES WING!"));
            }
            if m.form == 1 {
                if m.special <= 0.0 && dist < 5.0 {
                    m.special = 3.5;
                    hazards.push(Hazard { x: m.x, y: m.y, r: 3.0, warn: 0.8, live: 0.0, dps: 0.0, burst: 22.0 * m.tier.powf(0.8), t: 0.0, fired: false, kind: HazardKind::Nova });
                    texts.push((m.x, m.y, "SCREEEE!"));
                }
                if m.special2 <= 0.0 && summons < 6 {
                    m.special2 = 9.0;
                    around(rng, 4, Kind::Bat, m.tier, spawns);
                }
                return;
            }
            // Phases 1 and 2: blood bolt fans.
            if m.special <= 0.0 && dist < 10.0 {
                m.special = if frac < 0.66 { 2.4 } else { 3.0 };
                let base = (py - m.y).atan2(px - m.x);
                let n = if frac < 0.66 { 7 } else { 5 };
                for k in 0..n {
                    let a = base + (k as f32 - (n - 1) as f32 * 0.5) * 0.2;
                    shots.push((m.x, m.y, a.cos() * 8.0, a.sin() * 8.0, 12.0 * m.tier.powf(0.8), ShotKind::Blood));
                }
            }
            if m.special2 <= 0.0 {
                if frac < 0.66 {
                    // Phase 2: mist. Untouchable, reappears elsewhere, wolves answer.
                    m.special2 = 9.0;
                    m.invuln = 2.5;
                    for _ in 0..30 {
                        let a = rng.f() * std::f32::consts::TAU;
                        let rr = rng.rf(4.0, 7.0);
                        let (nx, ny) = (px + a.cos() * rr, py + a.sin() * rr);
                        if !d.blocked(nx, ny, m.r) && d.los(nx, ny, px, py) {
                            m.x = nx;
                            m.y = ny;
                            m.path.clear();
                            break;
                        }
                    }
                    texts.push((m.x, m.y, "MIST..."));
                    if summons < 6 {
                        around(rng, 3, Kind::Wolf, m.tier, spawns);
                    }
                } else {
                    m.special2 = 10.0;
                    if summons < 6 {
                        around(rng, 4, Kind::Bat, m.tier, spawns);
                        texts.push((m.x, m.y, "CHILDREN OF THE NIGHT!"));
                    }
                }
            }
        }
        Kind::Forgemother => {
            // Slag pools under you, and she rebuilds her fallen children.
            if m.special <= 0.0 && dist < 10.0 {
                m.special = if m.enraged { 3.0 } else { 4.5 };
                hazards.push(Hazard { x: px, y: py, r: 1.5, warn: 0.9, live: 5.0, dps: 9.0 * m.tier, burst: 0.0, t: 0.0, fired: false, kind: HazardKind::Slag });
                texts.push((m.x, m.y, "POUR!"));
            }
            if m.special2 <= 0.0 {
                m.special2 = if m.enraged { 8.0 } else { 11.0 };
                if summons < 6 {
                    around(rng, 2, Kind::Scarab, m.tier, spawns);
                    around(rng, 2, Kind::Ordinal, m.tier, spawns);
                    texts.push((m.x, m.y, "REBUILD!"));
                }
            }
            if m.enraged && m.special2 > 4.0 && m.special2 < 4.0 + DT * 1.5 && dist < 3.0 {
                hazards.push(Hazard { x: m.x, y: m.y, r: 2.6, warn: 0.9, live: 0.0, dps: 0.0, burst: 24.0 * m.tier.powf(0.8), t: 0.0, fired: false, kind: HazardKind::Quake });
            }
        }
        Kind::Cantor => {
            // The organ's deep chord: a slam around it, and the choir answers.
            if m.special <= 0.0 && dist < 3.5 {
                m.special = if m.enraged { 3.5 } else { 5.0 };
                hazards.push(Hazard { x: m.x, y: m.y, r: 3.2, warn: 1.0, live: 0.0, dps: 0.0, burst: 22.0 * m.tier.powf(0.8), t: 0.0, fired: false, kind: HazardKind::Quake });
                texts.push((m.x, m.y, "FORTISSIMO!"));
            }
            if m.special2 <= 0.0 {
                m.special2 = if m.enraged { 9.0 } else { 12.0 };
                if summons < 5 {
                    around(rng, 3, Kind::Inquisitor, m.tier, spawns);
                }
            }
        }
        Kind::Archivist => {
            // Files you away to another shelf, and summons records of old foes.
            if m.special <= 0.0 && dist < 3.0 {
                m.special = if m.enraged { 5.0 } else { 7.0 };
                m.cue = 2;
                texts.push((m.x, m.y, "FILED!"));
            }
            if m.special2 <= 0.0 {
                m.special2 = if m.enraged { 10.0 } else { 14.0 };
                if summons < 4 {
                    let past = [Kind::BoneWarden, Kind::HexWarden, Kind::Ossric, Kind::RimeWitch];
                    let k = past[rng.range(0, 4) as usize];
                    around(rng, 1, k, m.tier * 0.35, spawns);
                    texts.push((m.x, m.y, "FROM THE RECORDS..."));
                }
            }
        }
        Kind::Clockmaker => {
            let frac = m.hp / m.max_hp;
            // Phase 2: he climbs into the great engine.
            if frac < 0.5 && m.form == 0 {
                m.form = 1;
                m.speed *= 0.7;
                m.reach = 2.0;
                m.r = 0.8;
                m.invuln = 1.5;
                texts.push((m.x, m.y, "BEHOLD MY GREAT WORK!"));
            }
            if m.form == 1 {
                // The pendulum: a line of slams sweeping across you.
                if m.special <= 0.0 && dist < 9.0 {
                    m.special = if frac < 0.25 { 3.0 } else { 4.2 };
                    let (ux, uy) = ((px - m.x) / dist.max(0.01), (py - m.y) / dist.max(0.01));
                    for k in -3..=3 {
                        let (x, y) = (px + -uy * k as f32 * 1.2, py + ux * k as f32 * 1.2);
                        let warn = 0.7 + (k + 3) as f32 * 0.12;
                        hazards.push(Hazard { x, y, r: 0.9, warn, live: 0.0, dps: 0.0, burst: 20.0 * m.tier.powf(0.8), t: 0.0, fired: false, kind: HazardKind::Quake });
                    }
                    texts.push((m.x, m.y, "TICK... TOCK..."));
                }
                if m.special2 <= 0.0 {
                    m.special2 = 10.0;
                    if summons < 6 {
                        around(rng, 4, Kind::Ordinal, m.tier, spawns);
                    }
                }
                return;
            }
            // Phase 1: the duel. Clock-hand blades in a spiral, and rewinds.
            if m.special <= 0.0 && dist < 10.0 {
                m.special = if m.enraged { 2.2 } else { 3.0 };
                let base = (py - m.y).atan2(px - m.x);
                for k in 0..8 {
                    let a = base + k as f32 * std::f32::consts::TAU / 8.0;
                    shots.push((m.x, m.y, a.cos() * 6.5, a.sin() * 6.5, 12.0 * m.tier.powf(0.8), ShotKind::Gear));
                }
            }
            if m.special2 <= 0.0 && dist < 12.0 {
                m.special2 = 8.0;
                m.cue = 1;
                texts.push((m.x, m.y, "NO. AGAIN."));
            }
        }
        Kind::Dregmoor => {
            // Broadsides, the anchor hurled down a line at you, and his drowned crew.
            if m.special <= 0.0 && (3.0..11.0).contains(&dist) {
                m.special = if m.enraged { 3.0 } else { 4.2 };
                let base = (py - m.y).atan2(px - m.x);
                for k in -2..=2 {
                    let a = base + k as f32 * 0.2;
                    shots.push((m.x, m.y, a.cos() * 7.5, a.sin() * 7.5, 14.0 * m.tier.powf(0.8), ShotKind::Cannon));
                }
                texts.push((m.x, m.y, "FIRE THE BROADSIDE!"));
            }
            if m.special2 <= 0.0 && dist < 7.0 {
                m.special2 = if m.enraged { 5.0 } else { 7.0 };
                let (ux, uy) = ((px - m.x) / dist.max(0.01), (py - m.y) / dist.max(0.01));
                for k in 1..=6 {
                    let (x, y) = (m.x + ux * k as f32 * 1.1, m.y + uy * k as f32 * 1.1);
                    hazards.push(Hazard { x, y, r: 0.85, warn: 0.6 + k as f32 * 0.08, live: 0.0, dps: 0.0, burst: 22.0 * m.tier.powf(0.8), t: 0.0, fired: false, kind: HazardKind::Quake });
                }
                texts.push((m.x, m.y, "DROP ANCHOR!"));
                if summons < 6 {
                    around(rng, if m.enraged { 3 } else { 2 }, Kind::Drowned, m.tier, spawns);
                }
            }
        }
        Kind::Nacre => {
            // Her song pulls you in, coral spears burst up around you, and her daughters answer.
            if m.special <= 0.0 && (2.5..11.0).contains(&dist) {
                m.special = if m.enraged { 5.0 } else { 7.0 };
                m.cue = 3;
                texts.push((m.x, m.y, "~ COME TO MOTHER ~"));
            }
            if m.special2 <= 0.0 && dist < 10.0 {
                m.special2 = if m.enraged { 3.5 } else { 5.0 };
                for k in 0..3 {
                    let a = k as f32 / 3.0 * std::f32::consts::TAU + rng.f();
                    hazards.push(Hazard { x: px + a.cos() * 1.3, y: py + a.sin() * 1.3, r: 0.9, warn: 0.8, live: 0.0, dps: 0.0, burst: 18.0 * m.tier.powf(0.8), t: 0.0, fired: false, kind: HazardKind::Quake });
                }
                hazards.push(Hazard { x: px, y: py, r: 0.8, warn: 1.0, live: 0.0, dps: 0.0, burst: 18.0 * m.tier.powf(0.8), t: 0.0, fired: false, kind: HazardKind::Quake });
                if summons < 4 && rng.chance(0.5) {
                    around(rng, 2, Kind::Siren, m.tier, spawns);
                }
            }
        }
        Kind::Angler => {
            // She puts the lights out; only her lure shines. Then she lunges from the dark.
            if m.special <= 0.0 && dist < 12.0 {
                m.special = if m.enraged { 12.0 } else { 16.0 };
                m.cue = 4;
                texts.push((m.x, m.y, "THE LIGHTS GO OUT..."));
            }
            if m.special2 <= 0.0 && (2.5..9.0).contains(&dist) {
                m.special2 = if m.enraged { 3.0 } else { 4.5 };
                m.rush = 0.8;
            }
            if m.enraged && summons < 4 && rng.chance(DT / 8.0) {
                around(rng, 2, Kind::Anglerlurk, m.tier, spawns);
            }
        }
        Kind::Vael => {
            // Volleys of light-spears, a diving charge, and the fallen of his host.
            if m.special <= 0.0 && (2.5..11.0).contains(&dist) {
                m.special = if m.enraged { 2.6 } else { 3.6 };
                let base = (py - m.y).atan2(px - m.x);
                for k in -2..=2 {
                    let a = base + k as f32 * 0.16;
                    shots.push((m.x, m.y, a.cos() * 10.0, a.sin() * 10.0, 13.0 * m.tier.powf(0.8), ShotKind::Light));
                }
                texts.push((m.x, m.y, "KNEEL!"));
            }
            if m.special2 <= 0.0 && (3.0..9.0).contains(&dist) {
                m.special2 = if m.enraged { 4.0 } else { 6.0 };
                m.rush = 0.7;
                texts.push((m.x, m.y, "FROM ON HIGH!"));
                if summons < 5 {
                    around(rng, 2, Kind::FallenSeraph, m.tier, spawns);
                }
            }
        }
        Kind::Tempest => {
            // Storm rings around you, and the gale of its wings (cue 5).
            if m.special <= 0.0 && dist < 11.0 {
                m.special = if m.enraged { 3.4 } else { 4.6 };
                for k in 0..10 {
                    let a = k as f32 / 10.0 * std::f32::consts::TAU;
                    hazards.push(Hazard { x: px + a.cos() * 2.2, y: py + a.sin() * 2.2, r: 0.8, warn: 0.9, live: 0.0, dps: 0.0, burst: 20.0 * m.tier.powf(0.8), t: 0.0, fired: false, kind: HazardKind::Quake });
                }
                texts.push((m.x, m.y, "THE STORM RINGS!"));
            }
            if m.special2 <= 0.0 && dist < 12.0 {
                m.special2 = if m.enraged { 6.0 } else { 8.0 };
                m.cue = 5;
                texts.push((m.x, m.y, "GALE!"));
            }
        }
        Kind::OphanPrime => {
            // Turning beams: a ring of light that turns a little each time; more eyes open as it weakens.
            if m.special <= 0.0 && dist < 13.0 {
                let frac = m.hp / m.max_hp;
                m.special = if frac < 0.33 { 1.2 } else if frac < 0.66 { 1.7 } else { 2.3 };
                m.form = m.form.wrapping_add(1);
                let turn = m.form as f32 * 0.21;
                let n = if frac < 0.33 { 16 } else { 12 };
                for k in 0..n {
                    let a = turn + k as f32 / n as f32 * std::f32::consts::TAU;
                    shots.push((m.x, m.y, a.cos() * 7.0, a.sin() * 7.0, 11.0 * m.tier.powf(0.8), ShotKind::Light));
                }
            }
            if m.special2 <= 0.0 {
                m.special2 = 12.0;
                if summons < 4 {
                    around(rng, 2, Kind::Ophanim, m.tier, spawns);
                    texts.push((m.x, m.y, "ITS EYES OPEN..."));
                }
            }
        }
        Kind::Solanthos => {
            let frac = m.hp / m.max_hp;
            if frac < 0.66 && m.form == 0 {
                m.form = 1;
                m.invuln = 1.5;
                m.cue = 4;
                texts.push((m.x, m.y, "THE LIGHT GOES OUT..."));
            }
            if frac < 0.33 && m.form == 1 {
                m.form = 2;
                m.invuln = 1.5;
                texts.push((m.x, m.y, "ECLIPSE!"));
            }
            match m.form {
                0 => {
                    // Radiant: solar flares, and stars falling where you stand.
                    if m.special <= 0.0 && dist < 13.0 {
                        m.special = if m.enraged { 2.4 } else { 3.2 };
                        let base = (py - m.y).atan2(px - m.x);
                        for k in -3..=3 {
                            let a = base + k as f32 * 0.14;
                            shots.push((m.x, m.y, a.cos() * 8.0, a.sin() * 8.0, 14.0 * m.tier.powf(0.8), ShotKind::Ash));
                        }
                    }
                    if m.special2 <= 0.0 && dist < 14.0 {
                        m.special2 = if m.enraged { 3.0 } else { 4.0 };
                        for k in 0..3 {
                            let a = rng.f() * std::f32::consts::TAU;
                            let r = if k == 0 { 0.0 } else { rng.rf(1.2, 2.6) };
                            hazards.push(Hazard { x: px + a.cos() * r, y: py + a.sin() * r, r: 1.0, warn: 1.0, live: 0.0, dps: 0.0, burst: 24.0 * m.tier.powf(0.8), t: 0.0, fired: false, kind: HazardKind::Nova });
                        }
                        texts.push((m.x, m.y, "FALL, STARS!"));
                    }
                }
                1 => {
                    // Ash: in the dark, ember-wraiths rise, and ash burns underfoot.
                    if m.special <= 0.0 && dist < 13.0 {
                        m.special = if m.enraged { 3.0 } else { 4.0 };
                        hazards.push(Hazard { x: px, y: py, r: 1.6, warn: 0.9, live: 4.0, dps: 10.0 * m.tier, burst: 0.0, t: 0.0, fired: false, kind: HazardKind::Slag });
                    }
                    if m.special2 <= 0.0 {
                        m.special2 = 9.0;
                        m.cue = 4;
                        if summons < 6 {
                            around(rng, 4, Kind::Wisp, m.tier, spawns);
                            texts.push((m.x, m.y, "RISE, MY EMBERS!"));
                        }
                    }
                }
                _ => {
                    // Eclipse: turning beams, and the floor breaking away all around.
                    if m.special <= 0.0 && dist < 14.0 {
                        m.special = if m.enraged { 1.4 } else { 1.9 };
                        m.anim_t += 0.37;
                        let turn = m.anim_t * 0.6;
                        for k in 0..10 {
                            let a = turn + k as f32 / 10.0 * std::f32::consts::TAU;
                            shots.push((m.x, m.y, a.cos() * 7.5, a.sin() * 7.5, 13.0 * m.tier.powf(0.8), ShotKind::Light));
                        }
                    }
                    if m.special2 <= 0.0 {
                        m.special2 = if m.enraged { 2.4 } else { 3.2 };
                        for _ in 0..6 {
                            let a = rng.f() * std::f32::consts::TAU;
                            let r = rng.rf(0.0, 4.5);
                            hazards.push(Hazard { x: px + a.cos() * r, y: py + a.sin() * r, r: 0.9, warn: 1.1, live: 0.0, dps: 0.0, burst: 20.0 * m.tier.powf(0.8), t: 0.0, fired: false, kind: HazardKind::Quake });
                        }
                    }
                }
            }
        }
        Kind::Leviathan => {
            let frac = m.hp / m.max_hp;
            if frac < 0.66 && m.form == 0 {
                m.form = 1;
                m.invuln = 1.5;
                texts.push((m.x, m.y, "THE TIDE RISES!"));
            }
            if frac < 0.33 && m.form == 1 {
                m.form = 2;
                m.invuln = 1.5;
                m.speed *= 0.5;
                texts.push((m.x, m.y, "THE LEVIATHAN RISES FROM THE DEEP!"));
            }
            match m.form {
                0 => {
                    // Its coils sweep the floor: three lines of slams rolling across you, one after another.
                    if m.special <= 0.0 && dist < 12.0 {
                        m.special = if m.enraged { 3.4 } else { 4.4 };
                        let (ux, uy) = ((px - m.x) / dist.max(0.01), (py - m.y) / dist.max(0.01));
                        for row in -1..=1 {
                            for k in -4..=4 {
                                let (x, y) = (px + ux * row as f32 * 1.8 + -uy * k as f32 * 1.0, py + uy * row as f32 * 1.8 + ux * k as f32 * 1.0);
                                let warn = 0.8 + (row + 1) as f32 * 0.45;
                                hazards.push(Hazard { x, y, r: 0.7, warn, live: 0.0, dps: 0.0, burst: 20.0 * m.tier.powf(0.8), t: 0.0, fired: false, kind: HazardKind::Quake });
                            }
                        }
                        texts.push((m.x, m.y, "THE COILS!"));
                    }
                }
                1 => {
                    // The tidal wave: a ring crashing in on you with one gap to dodge through, and the whirlpool's pull.
                    if m.special <= 0.0 && dist < 14.0 {
                        m.special = if m.enraged { 4.0 } else { 5.0 };
                        let gap = rng.f() * std::f32::consts::TAU;
                        for k in 0..14 {
                            let a = k as f32 / 14.0 * std::f32::consts::TAU;
                            let off = (a - gap).rem_euclid(std::f32::consts::TAU);
                            if off < 0.9 {
                                continue;
                            }
                            hazards.push(Hazard { x: px + a.cos() * 2.4, y: py + a.sin() * 2.4, r: 1.1, warn: 1.1, live: 0.0, dps: 0.0, burst: 24.0 * m.tier.powf(0.8), t: 0.0, fired: false, kind: HazardKind::Quake });
                        }
                        hazards.push(Hazard { x: px, y: py, r: 1.2, warn: 1.1, live: 0.0, dps: 0.0, burst: 24.0 * m.tier.powf(0.8), t: 0.0, fired: false, kind: HazardKind::Quake });
                        texts.push((m.x, m.y, "TIDAL WAVE!"));
                    }
                    if m.special2 <= 0.0 {
                        m.special2 = 9.0;
                        m.cue = 3;
                        if summons < 6 {
                            around(rng, 3, Kind::Merrow, m.tier, spawns);
                        }
                    }
                }
                _ => {
                    // The pressure beam: a jet of water sweeping toward you.
                    if m.special <= 0.0 && dist < 14.0 {
                        m.special = if m.enraged { 2.2 } else { 3.0 };
                        let base = (py - m.y).atan2(px - m.x);
                        for k in 0..9 {
                            let a = base + (k as f32 - 4.0) * 0.06;
                            let v = 7.0 + k as f32 * 0.6;
                            shots.push((m.x, m.y, a.cos() * v, a.sin() * v, 12.0 * m.tier.powf(0.8), ShotKind::Tide));
                        }
                    }
                    if m.special2 <= 0.0 {
                        m.special2 = 7.0;
                        m.cue = 3;
                        if summons < 6 {
                            around(rng, 2, Kind::InkHorror, m.tier, spawns);
                        }
                    }
                }
            }
        }
        Kind::RimeWitch => {
            if m.special <= 0.0 && dist < 2.5 {
                m.special = 5.0;
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
            if m.special2 <= 0.0 && dist < 10.0 {
                m.special2 = if m.enraged { 4.0 } else { 6.0 };
                hazards.push(Hazard { x: px, y: py, r: 1.4, warn: 0.8, live: 4.0, dps: 6.0 * m.tier, burst: 0.0, t: 0.0, fired: false, kind: HazardKind::Frost });
                if m.enraged && summons < 4 {
                    around(rng, 2, Kind::IceWraith, m.tier, spawns);
                }
            }
        }
        Kind::WhiteDragon => {
            // Frost breath: a cone of freezing ground toward you.
            if m.special <= 0.0 && dist < 9.0 {
                m.special = if m.enraged { 3.2 } else { 4.5 };
                let a = (py - m.y).atan2(px - m.x);
                for k in 0..6 {
                    let dd = 1.6 + k as f32 * 1.1;
                    for side in [-1.0f32, 0.0, 1.0] {
                        if k < 2 && side != 0.0 {
                            continue;
                        }
                        let aa = a + side * 0.14 * (k as f32 * 0.5 + 0.5);
                        let (x, y) = (m.x + aa.cos() * dd, m.y + aa.sin() * dd);
                        if d.blocked(x, y, 0.1) {
                            continue;
                        }
                        hazards.push(Hazard {
                            x,
                            y,
                            r: 0.7 + k as f32 * 0.07,
                            warn: 0.7 + k as f32 * 0.05,
                            live: 1.6,
                            dps: 16.0 * m.tier.powf(0.8),
                            burst: 0.0,
                            t: 0.0,
                            fired: false,
                            kind: HazardKind::Frost,
                        });
                    }
                }
                texts.push((m.x, m.y, "FROST BREATH!"));
            }
            // Icicles shaken from the cavern roof.
            if m.special2 <= 0.0 && dist < 12.0 {
                m.special2 = if m.enraged { 5.0 } else { 7.5 };
                for _ in 0..if m.enraged { 8 } else { 5 } {
                    let (a, rr) = (rng.f() * std::f32::consts::TAU, rng.rf(0.0, 3.0));
                    let (x, y) = (px + a.cos() * rr, py + a.sin() * rr);
                    if !d.blocked(x, y, 0.2) {
                        hazards.push(Hazard { x, y, r: 1.0, warn: 1.2, live: 0.0, dps: 0.0, burst: 16.0 * m.tier.powf(0.8), t: 0.0, fired: false, kind: HazardKind::Icicle });
                    }
                }
            }
            if m.enraged && summons < 4 && rng.chance(DT / 12.0) {
                around(rng, 2, Kind::IceWraith, m.tier, spawns);
                texts.push((m.x, m.y, "THE COLD ANSWERS ME"));
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
                    around(rng, 4, Kind::Goblin, m.tier, spawns);
                }
            }
        }
        _ => {}
    }
}

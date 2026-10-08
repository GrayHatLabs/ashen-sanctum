//! Side content (docs/SIDE_CONTENT_PLAN.md): shrines, super uniques, side quests, the optional dungeons,
//! random events and lore pages. Nothing here is on the main story's path.
//!
//! - **Shrines**: glowing altars on overlands, dungeon floors and in the rifts. Touch one for a 90 second
//!   blessing (or an instant gift); a used shrine goes dark.
//! - **Super uniques**: named monsters with a fixed home, fixed powers, their own line and loot (a rare,
//!   sometimes their own unique, and a lore page).
//! - **Side quests**: three per act, given by townsfolk, saved per difficulty (like D2, the rewards can be
//!   earned again on Nightmare and Hell).
//! - **Events**: now and then out in the wilds an ambush, a fleeing gold-thief or a fallen adventurer.
//! - **Lore pages**: five per act; all five give +5% XP in that act.
use crate::game::{Decal, Drop, Game, Light, PKind, Pickup, Sfx};
use crate::gfx::rgb;
use crate::mobs::{Kind, Mob, MobState, Rank, M_FAST, M_FIERY, M_STONE, M_STRONG, M_VAMPIRE};
use crate::rng::Rng;
use crate::story::{Act, Dialog, Role};
use crate::world::{Level, LevelId, Prop, PropKind};

// ------------------------------------------------------------------ shrines

/// What a shrine gives. The timed ones last BLESS_TIME seconds.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Blessing {
    Armor,
    Combat,
    Mana,
    Experience,
    Refill,
    Skill,
    Gem,
    Haste,
}

pub const BLESSINGS: [Blessing; 8] = [
    Blessing::Armor,
    Blessing::Combat,
    Blessing::Mana,
    Blessing::Experience,
    Blessing::Refill,
    Blessing::Skill,
    Blessing::Gem,
    Blessing::Haste,
];
pub const BLESS_TIME: f32 = 90.0;

impl Blessing {
    pub fn name(self) -> &'static str {
        match self {
            Blessing::Armor => "ARMOR SHRINE",
            Blessing::Combat => "COMBAT SHRINE",
            Blessing::Mana => "MANA SHRINE",
            Blessing::Experience => "EXPERIENCE SHRINE",
            Blessing::Refill => "REFILL SHRINE",
            Blessing::Skill => "SKILL SHRINE",
            Blessing::Gem => "GEM SHRINE",
            Blessing::Haste => "HASTE SHRINE",
        }
    }

    /// What it does, for the floater and the HUD.
    pub fn short(self) -> &'static str {
        match self {
            Blessing::Armor => "-25% DAMAGE TAKEN",
            Blessing::Combat => "+30% DAMAGE",
            Blessing::Mana => "MANA REGENERATES 3X",
            Blessing::Experience => "+40% EXPERIENCE",
            Blessing::Refill => "LIFE AND MANA RESTORED",
            Blessing::Skill => "+1 TO ALL SKILLS",
            Blessing::Gem => "A GEM",
            Blessing::Haste => "+25% SPEED",
        }
    }

    pub fn col(self) -> u32 {
        match self {
            Blessing::Armor => 0xc0c8d8,
            Blessing::Combat => 0xff7040,
            Blessing::Mana => 0x6090ff,
            Blessing::Experience => 0xffe080,
            Blessing::Refill => 0xff6080,
            Blessing::Skill => 0xc080ff,
            Blessing::Gem => 0x60f0c0,
            Blessing::Haste => 0x80f080,
        }
    }

    pub fn id(self) -> u8 {
        BLESSINGS.iter().position(|b| *b == self).unwrap() as u8 + 1
    }

    pub fn from_id(id: u8) -> Option<Blessing> {
        BLESSINGS.get((id as usize).wrapping_sub(1)).copied()
    }
}

/// A shrine on a level: its prop's footprint tile, what it gives, and whether it's been used.
#[derive(Clone, Copy, Debug)]
pub struct Shrine {
    pub x: i32,
    pub y: i32,
    pub kind: Blessing,
    pub used: bool,
}

impl Shrine {
    /// Where you stand to touch it (the middle of its tile).
    pub fn at(&self) -> (f32, f32) {
        (self.x as f32 + 0.5, self.y as f32 + 0.5)
    }
}

/// The shrine art of an act (one style per act; later acts use the ash shrine until theirs exists).
pub fn shrine_art(act: u8) -> &'static str {
    match act {
        1 => "shrine_frost",
        2 => "shrine_mist",
        3 => "shrine_gear",
        4 => "shrine_coral",
        _ => "shrine_ash",
    }
}

// ------------------------------------------------------------------ super uniques

pub struct SuperDef {
    pub name: &'static str,
    pub kind: Kind,
    /// Where it lives, and the tile it stands on ((0, 0): a random spot far from the start).
    pub home: LevelId,
    pub spot: (i32, i32),
    pub mods: u8,
    /// Its gang: (kind, how many).
    pub gang: (Kind, usize),
    pub tint: u32,
    /// What it shouts when it sees you.
    pub line: &'static str,
    /// Its own unique (the items::UNIQUES boss key), dropped now and then.
    pub unique: &'static str,
    /// The lore page it carries (index into LORE), if any.
    pub page: Option<u8>,
    /// Something of its camp, set down beside it (Skrat's stolen cart).
    pub camp: Option<PropKind>,
}

pub const SUPERS: &[SuperDef] = &[
    // ---- Act 1: the Ashlands ----
    SuperDef {
        name: "SKRAT ONE-EAR",
        kind: Kind::Goblin,
        home: LevelId::Area(0, 8),
        spot: (0, 0),
        mods: M_FAST | M_STRONG,
        gang: (Kind::Goblin, 5),
        tint: 0x80d040,
        line: "SKRAT'S LOOT! ALL OF IT SKRAT'S!",
        unique: "skrat",
        page: Some(1),
        camp: Some(PropKind::Cart),
    },
    SuperDef {
        name: "OLD BONEJAW",
        kind: Kind::Archer,
        home: LevelId::Dungeon(0, 1),
        spot: (0, 0),
        mods: M_STONE | M_FIERY,
        gang: (Kind::Skeleton, 3),
        tint: 0xf0e0b0,
        line: "MORE BONES FOR THE PILE.",
        unique: "bonejaw",
        page: Some(0),
        camp: None,
    },
    SuperDef {
        name: "THE HOLLOW SHEPHERD",
        kind: Kind::Zombie,
        home: LevelId::Area(0, 7),
        spot: (0, 0),
        mods: M_FIERY | M_VAMPIRE,
        gang: (Kind::Zombie, 5),
        tint: 0xa0b080,
        line: "THE FLOCK... MUST... FEED...",
        unique: "shepherd",
        page: Some(2),
        camp: None,
    },
    // ---- Act 2: the Frostmarch ----
    SuperDef {
        name: "GRIMFANG THE WHITE",
        kind: Kind::FrostWolf,
        home: LevelId::Area(1, 2),
        spot: (0, 0),
        mods: M_FAST | M_STRONG,
        gang: (Kind::FrostWolf, 4),
        tint: 0xf0f8ff,
        line: "(A HOWL THAT FREEZES YOUR BLOOD)",
        unique: "grimfang",
        page: Some(6),
        camp: None,
    },
    SuperDef {
        name: "THE FROZEN BRIDE",
        kind: Kind::IceWraith,
        home: LevelId::Area(1, 3),
        spot: (0, 0),
        mods: crate::mobs::M_MANABURN | M_FAST,
        gang: (Kind::IceWraith, 2),
        tint: 0xb0d8ff,
        line: "WHERE IS MY GROOM? WHERE IS HE?",
        unique: "frozenbride",
        page: Some(7),
        camp: None,
    },
    // (The Jarl isn't placed with the others: features.rs brings him out of his hall.)
    SuperDef {
        name: "JARL HROGAR",
        kind: Kind::Raider,
        home: LevelId::Area(1, 99),
        spot: (0, 0),
        mods: M_STRONG | M_STONE,
        gang: (Kind::Raider, 0),
        tint: 0xd09060,
        line: "COME ON, THEN!",
        unique: "jarl",
        page: Some(8),
        camp: None,
    },
    // ---- Act 3: the Mistwood ----
    SuperDef {
        name: "THE PALE HUNTSMAN",
        kind: Kind::Werewolf,
        home: LevelId::Area(2, 3),
        spot: (0, 0),
        mods: crate::mobs::M_FAST | crate::mobs::M_VAMPIRE,
        gang: (Kind::Werewolf, 2),
        tint: 0xe8e0d0,
        line: "I SMELL YOUR FEAR, LITTLE HUNTER...",
        unique: "pale",
        page: Some(10),
        camp: None,
    },
    SuperDef {
        name: "SISTER MOURNWAIL",
        kind: Kind::Banshee,
        home: LevelId::Area(2, 2),
        spot: (0, 0),
        mods: M_STRONG | crate::mobs::M_MANABURN,
        gang: (Kind::Banshee, 2),
        tint: 0xc0d0ff,
        line: "SING WITH US... SING WITH US FOREVER...",
        unique: "mournwail",
        page: Some(11),
        camp: None,
    },
    SuperDef {
        name: "BLACKMOOR THE GIBBET-HANGED",
        kind: Kind::Cultist,
        home: LevelId::Area(2, 4),
        spot: (0, 0),
        mods: M_STONE | M_FIERY,
        gang: (Kind::Cultist, 3),
        tint: 0x707060,
        line: "THEY HANGED ME TWICE. IT DIDN'T TAKE.",
        unique: "blackmoor",
        page: Some(12),
        camp: Some(PropKind::Gallows),
    },
    // Count Vardak's brides (mist.rs hides them in three areas).
    SuperDef { name: "LUCRETIA, VARDAK'S BRIDE", kind: Kind::Bride, home: LevelId::Area(2, 99), spot: (0, 0), mods: M_FAST | M_VAMPIRE, gang: (Kind::Cultist, 0), tint: 0xf0d0e0, line: "MY LORD WILL DRINK YOU DRY.", unique: "bride", page: None, camp: None },
    SuperDef { name: "MORGANA, VARDAK'S BRIDE", kind: Kind::Bride, home: LevelId::Area(2, 99), spot: (0, 0), mods: M_STRONG | M_VAMPIRE, gang: (Kind::Cultist, 0), tint: 0xd8c0f0, line: "OH, A GUEST. HOW... APPETISING.", unique: "bride", page: None, camp: None },
    SuperDef { name: "ISOLDE, VARDAK'S BRIDE", kind: Kind::Bride, home: LevelId::Area(2, 99), spot: (0, 0), mods: M_STONE | M_VAMPIRE, gang: (Kind::Cultist, 0), tint: 0xe0e8ff, line: "YOU'LL NEVER REACH THE CASTLE.", unique: "bride", page: None, camp: None },
    // ---- Act 4: Mechanus ----
    SuperDef {
        name: "MAINSPRING",
        kind: Kind::BoilerBrute,
        home: LevelId::Area(3, 2),
        spot: (0, 0),
        mods: M_STONE | M_FIERY,
        gang: (Kind::Scarab, 4),
        tint: 0xffa060,
        line: "PRESSURE... RISING...",
        unique: "mainspring",
        page: Some(15),
        camp: None,
    },
    SuperDef {
        name: "GRAND INQUISITOR HALVANE",
        kind: Kind::Inquisitor,
        home: LevelId::Area(3, 5),
        spot: (0, 0),
        mods: M_STRONG | M_FIERY,
        gang: (Kind::Inquisitor, 3),
        tint: 0xfff0c0,
        line: "THE CLOCK-LAW IS ABSOLUTE. KNEEL!",
        unique: "halvane",
        page: Some(16),
        camp: None,
    },
    SuperDef {
        name: "TICK-TOCK JACK",
        kind: Kind::SpringJack,
        home: LevelId::Area(3, 3),
        spot: (0, 0),
        mods: M_FAST | M_STRONG,
        gang: (Kind::ClockCrow, 4),
        tint: 0xc0ffe0,
        line: "TICK, TOCK! CATCH ME IF YOU CAN!",
        unique: "ticktock",
        page: Some(17),
        camp: None,
    },
    // ---- Act 5: the Drowned Deep ----
    SuperDef { name: "BOSUN KRAKE", kind: Kind::Drowned, home: LevelId::Area(4, 1), spot: (0, 0), mods: M_STRONG | crate::mobs::M_VAMPIRE, gang: (Kind::Drowned, 4), tint: 0x80a090, line: "ALL HANDS... ALL HANDS ON DECK...", unique: "krake", page: Some(20), camp: None },
    SuperDef { name: "LIRAEL", kind: Kind::Siren, home: LevelId::Area(4, 4), spot: (0, 0), mods: crate::mobs::M_MANABURN | M_FAST, gang: (Kind::Merrow, 3), tint: 0xa0f0e0, line: "COME CLOSER, SWEET ONE. CLOSER...", unique: "lirael", page: Some(21), camp: None },
    SuperDef { name: "MAW", kind: Kind::Anglerlurk, home: LevelId::Area(4, 5), spot: (0, 0), mods: M_STRONG | M_STONE, gang: (Kind::Anglerlurk, 2), tint: 0x406080, line: "(A LIGHT BOBS IN THE DARK... AND THEN THE TEETH)", unique: "maw", page: Some(22), camp: None },
    // Captain Ysolde's drowned crew (side.rs quest: lay all three to rest).
    SuperDef { name: "MATE HOLLIS", kind: Kind::Drowned, home: LevelId::Area(4, 2), spot: (0, 0), mods: M_STRONG, gang: (Kind::Drowned, 2), tint: 0xb0c8d0, line: "CAPTAIN? IS THAT YOU, CAPTAIN?", unique: "sovereign", page: None, camp: None },
    SuperDef { name: "COOK BRANNIGAN", kind: Kind::Drowned, home: LevelId::Area(4, 3), spot: (0, 0), mods: M_FIERY, gang: (Kind::Drowned, 2), tint: 0xb0c8d0, line: "SUPPER'S GETTING COLD, LADS...", unique: "sovereign", page: None, camp: None },
    SuperDef { name: "BOY TOBIAS", kind: Kind::Drowned, home: LevelId::Area(4, 5), spot: (0, 0), mods: M_FAST, gang: (Kind::Drowned, 2), tint: 0xb0c8d0, line: "I'M SORRY, CAPTAIN, I'M SORRY, I DIDN'T MEAN TO...", unique: "sovereign", page: None, camp: None },
    // The Sunken Galleon's siren choir (reef.rs places them).
    SuperDef { name: "CORALIE OF THE CHOIR", kind: Kind::Siren, home: LevelId::Area(4, 99), spot: (0, 0), mods: crate::mobs::M_MANABURN, gang: (Kind::Siren, 0), tint: 0xffc0e0, line: "SING WITH US...", unique: "choir", page: None, camp: None },
    SuperDef { name: "MARIS OF THE CHOIR", kind: Kind::Siren, home: LevelId::Area(4, 99), spot: (0, 0), mods: M_FAST, gang: (Kind::Siren, 0), tint: 0xc0e0ff, line: "...DOWN, DOWN, DOWN...", unique: "choir", page: None, camp: None },
    SuperDef { name: "LYRA OF THE CHOIR", kind: Kind::Siren, home: LevelId::Area(4, 99), spot: (0, 0), mods: M_STONE, gang: (Kind::Siren, 0), tint: 0xe0ffc0, line: "...INTO THE DARK WATER...", unique: "choir", page: None, camp: None },
];

/// Jarl Hrogar's index in SUPERS (features.rs).
pub const JARL: usize = 5;
/// Ysolde's drowned crew, and the Sunken Galleon's choir, in SUPERS (reef.rs).
pub const GHOSTS: [usize; 3] = [18, 19, 20];
pub const CHOIR: [usize; 3] = [21, 22, 23];
/// Vardak's brides in SUPERS (mist.rs places them).
pub const BRIDES: [usize; 3] = [9, 10, 11];

/// Super uniques are drawn this much bigger than their kind.
pub const SUPER_SCALE: f32 = 1.25;

// ------------------------------------------------------------------ side quests

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Reward {
    SkillPoint,
    /// Permanent extra life.
    Life(u32),
    /// A rare item, and some gold.
    Rare(i32),
    /// Permanent -5% damage taken (stacks).
    Ward,
    /// A socket punched into your weapon (or armour, if the weapon is full).
    Socket,
    /// This unique item (items::UNIQUES boss key).
    Unique(&'static str),
    Respec,
    Gold(i32),
}

impl Reward {
    pub fn text(self) -> String {
        match self {
            Reward::SkillPoint => "A SKILL POINT".into(),
            Reward::Life(n) => format!("+{n} LIFE, FOR GOOD"),
            Reward::Rare(g) => format!("A RARE ITEM AND {g} GOLD"),
            Reward::Ward => "-5% DAMAGE TAKEN, FOR GOOD".into(),
            Reward::Socket => "A SOCKET IN YOUR GEAR".into(),
            Reward::Unique(k) => crate::items::boss_unique(k).map(|u| u.name).unwrap_or_default(),
            Reward::Respec => "A FREE RESPEC".into(),
            Reward::Gold(g) => format!("{g} GOLD"),
        }
    }
}

/// What finishes a side quest.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Goal {
    /// Kill this boss (an optional dungeon's).
    Boss(Kind),
    /// Kill this super unique (index into SUPERS).
    Super(usize),
    /// Get out of the Ember Wyrm's cave with a sack of its gold (dragon.rs).
    Heist,
    /// Thaw Brenna's three frozen scouts (features.rs).
    Scouts,
    /// Ring the three bells of Mournhold (mist.rs).
    Bells,
    /// Lay Captain Ysolde's three drowned crewmen to rest (reef.rs).
    Ghosts,
}

pub struct SideDef {
    /// Short name, for the quest log and the journal.
    pub name: &'static str,
    pub act: u8,
    pub giver: Role,
    pub giver_name: &'static str,
    /// The option that starts the conversation about it.
    pub ask: &'static str,
    pub offer: &'static [&'static str],
    pub remind: &'static str,
    pub thanks: &'static [&'static str],
    pub goal: Goal,
    pub reward: Reward,
    /// The quest log line while it's open.
    pub todo: &'static str,
}

/// Side quest states (Quest::side): not given, given, done (reward waiting), rewarded.
pub const S_NONE: u8 = 0;
pub const S_GIVEN: u8 = 1;
pub const S_DONE: u8 = 2;
pub const S_PAID: u8 = 3;

pub const SIDES: &[SideDef] = &[
    // ---- Act 1 ----
    SideDef {
        name: "THE WELL RUNS RED",
        act: 0,
        giver: Role::Healer,
        giver_name: "BROTHER ALDRIC",
        ask: "ASK ABOUT THE OLD WELL",
        offer: &[
            "THE OLD WELL OUT IN THE ASHLANDS RAN DRY THE NIGHT THE ASH FELL. NOW IT RUNS AGAIN... RED, AND IT STINKS OF THE GRAVE.",
            "THE FARMERS SAY A WITCH WAS DROWNED IN IT, LONG AGO. I THINK SHE HAS FOUND HER WAY BACK UP. GO DOWN THE CHARNEL WELL AND END HER, BEFORE SHE POISONS US ALL.",
        ],
        remind: "THE CHARNEL WELL IS IN THE ASHLANDS, NORTH OF THE VILLAGE, OFF THE ROAD. THE WITCH WAITS AT THE BOTTOM.",
        thanks: &["THE WATER RUNS CLEAR AGAIN. I CAN TEACH YOU SOMETHING FOR THAT: SIT, AND LET YOUR MIND GO STILL..."],
        goal: Goal::Boss(Kind::WellWitch),
        reward: Reward::SkillPoint,
        todo: "SLAY THE WELL-WITCH IN THE CHARNEL WELL (THE ASHLANDS)",
    },
    SideDef {
        name: "GERTA'S CARAVAN",
        act: 0,
        giver: Role::Merchant,
        giver_name: "GERTA",
        ask: "ASK ABOUT HER SUPPLIES",
        offer: &[
            "MY SUPPLY CART NEVER CAME. GOBLINS DRAGGED IT OFF THE BARROW FIELDS INTO A GULCH TO THE EAST. THEIR CHIEF IS A ONE-EARED RUNT CALLED SKRAT, AND HE THINKS EVERYTHING SHINY IS HIS.",
            "BRING HIM DOWN AND I'LL MAKE IT WORTH YOUR WHILE. I KEEP MY BEST PIECE UNDER THE COUNTER.",
        ],
        remind: "SKRAT ONE-EAR. HIS GULCH IS EAST OFF THE BARROW FIELDS, PAST THE ASHLANDS. HE'LL BE SITTING ON MY CART.",
        thanks: &["SKRAT'S DEAD? HA! HERE, AS PROMISED: THE PIECE FROM UNDER THE COUNTER, AND A PURSE FOR YOUR TROUBLE."],
        goal: Goal::Super(0),
        reward: Reward::Rare(300),
        todo: "SLAY SKRAT ONE-EAR (SKRAT'S GULCH, EAST OF THE BARROW FIELDS)",
    },
    SideDef {
        name: "THE HOLLOW SHEPHERD",
        act: 0,
        giver: Role::Villager(1),
        giver_name: "FARMER",
        ask: "ASK ABOUT HIS FLOCK",
        offer: &[
            "MY FLOCK... THE OLD SHEPHERD WENT OUT TO THE VALE WEST OF THE ASHLANDS WHEN THE ASH FELL. HE CAME BACK WRONG, AND THE SHEEP CAME BACK WITH HIM. THEY DON'T BLEAT ANY MORE.",
            "PUT HIM TO REST. PLEASE. HE WAS MY FATHER.",
        ],
        remind: "SHEPHERD'S VALE, WEST OFF THE ASHLANDS. YOU'LL KNOW HIM BY THE FLOCK AROUND HIM.",
        thanks: &["THANK YOU. I'LL BURY HIM PROPERLY. TAKE THIS: MY MOTHER'S CHARM. IT KEPT HIM ALIVE THROUGH THREE WINTERS."],
        goal: Goal::Super(2),
        reward: Reward::Life(20),
        todo: "PUT THE HOLLOW SHEPHERD TO REST (SHEPHERD'S VALE, WEST OF THE ASHLANDS)",
    },
    SideDef {
        name: "THE EMBER WYRM",
        act: 0,
        giver: Role::Guard,
        giver_name: "CAPTAIN ROLF",
        ask: "ASK ABOUT THE SMOKE IN THE HILLS",
        offer: &[
            "SEE THAT SMOKE OVER THE CINDER HILLS? A DRAGON. VAURATH, THE EMBER WYRM. IT CAME DOWN FROM THE NORTH WITH THE ASH AND CRAWLED INTO A CAVE IN EMBERPEAK PASS, AND IT SLEEPS ON A HILL OF GOLD.",
            "DON'T FIGHT IT. I MEAN IT. BUT A QUIET PAIR OF HANDS... WALK, DON'T RUN. DON'T CAST. GRAB WHAT YOU CAN CARRY AND GET OUT BEFORE IT OPENS ITS EYES. BRING BACK FIVE HUNDRED IN GOLD AND I'LL DRINK TO YOUR NAME.",
        ],
        remind: "EMBERPEAK PASS, NORTH OFF THE CINDER HILLS. WALK SOFT, GRAB FAST, RUN WHEN IT WAKES. THE GOLD ISN'T YOURS TILL YOU'RE OUT.",
        thanks: &["YOU WALKED INTO A DRAGON'S BEDROOM AND WALKED OUT RICH! HERE, THE GUARD'S OLD CHARM. IT'S TURNED A FEW BLADES IN ITS TIME."],
        goal: Goal::Heist,
        reward: Reward::Ward,
        todo: "STEAL 500 GOLD FROM THE EMBER WYRM AND GET OUT (EMBERPEAK PASS, NORTH OF THE CINDER HILLS)",
    },
    // ---- Act 2 ----
    SideDef {
        name: "SIGURD'S AXE",
        act: 1,
        giver: Role::Trader,
        giver_name: "OLD SIGURD",
        ask: "ASK ABOUT THE SHIP IN THE ICE",
        offer: &[
            "MY FATHER SAILED WITH HROLF ICE-BEARD, BACK WHEN HROLF WAS A MAN. THE WINTER TOOK THEIR LONGSHIP IN THE FJORD, AND HROLF WOULDN'T LET IT GO. HE'S STILL ABOARD, THEY SAY. AND MY FATHER'S AXE WITH HIM.",
            "THE ICEBOUND LONGSHIP, IN THE RAIDERS' FJORD EAST OF THE FROZEN SHORE. PUT HROLF DOWN AND I'LL SHOW YOU WHAT AN OLD SMITH CAN STILL DO.",
        ],
        remind: "THE LONGSHIP'S FROZEN IN THE RAIDERS' FJORD, EAST OFF THE FROZEN SHORE. HROLF WAITS BELOW DECKS.",
        thanks: &["HROLF'S DEAD, AND THE AXE IS HOME. GIVE ME YOUR GEAR A MOMENT... THERE. A SOCKET, CUT CLEAN. MY FATHER WOULD HAVE LIKED YOU."],
        goal: Goal::Boss(Kind::Hrolf),
        reward: Reward::Socket,
        todo: "SLAY HROLF ICE-BEARD IN THE ICEBOUND LONGSHIP (THE RAIDERS' FJORD)",
    },
    SideDef {
        name: "THE LOST PATROL",
        act: 1,
        giver: Role::Captain,
        giver_name: "CAPTAIN BRENNA",
        ask: "ASK ABOUT HER MISSING SCOUTS",
        offer: &["THREE OF MY SCOUTS NEVER CAME BACK. A HUNTER SAW ONE STANDING STILL AS A STATUE, ICE ALL OVER HIM, WITH WRAITHS CIRCLING. THE WRAITHS KEEP THEM FROZEN. BREAK THE ICE AND BRING MY PEOPLE HOME. ONE ON THE FROZEN SHORE, ONE IN THE RIME WOODS, ONE OUT ON THE WHITE WASTE."],
        remind: "THE FROZEN SHORE, THE RIME WOODS, THE WHITE WASTE. LOOK FOR ICE WITH A MAN INSIDE IT, AND WRAITHS AROUND IT.",
        thanks: &["ALL THREE, HOME AND WARM. TAKE THIS: THE CAPTAIN'S CLOAK-PIN. IT'S TURNED MORE THAN ONE BLADE."],
        goal: Goal::Scouts,
        reward: Reward::Ward,
        todo: "THAW BRENNA'S THREE SCOUTS (FROZEN SHORE, RIME WOODS, WHITE WASTE)",
    },
    SideDef {
        name: "THE FROZEN BRIDE",
        act: 1,
        giver: Role::Seer,
        giver_name: "MOTHER YLVA",
        ask: "ASK ABOUT THE WEEPING IN THE WOODS",
        offer: &["A BRIDE WALKED INTO THE RIME WOODS ON HER WEDDING NIGHT, LOOKING FOR A GROOM WHO NEVER CAME. THE COLD KEPT HER. NOW SHE WALKS THERE STILL, AND ANYONE SHE TOUCHES FORGETS. LAY HER TO REST, AND I'LL HELP YOU FORGET SOMETHING OF YOUR OWN."],
        remind: "THE RIME WOODS. LISTEN FOR THE WEEPING.",
        thanks: &["SHE'S AT PEACE. NOW, SIT. CLOSE YOUR EYES. LET YOUR SKILLS RUN OUT OF YOU LIKE MELTWATER... AND LEARN THEM AGAIN AS YOU WILL."],
        goal: Goal::Super(4),
        reward: Reward::Respec,
        todo: "LAY THE FROZEN BRIDE TO REST (THE RIME WOODS)",
    },
    // ---- Act 3 ----
    SideDef {
        name: "A HUSBAND'S GRAVE",
        act: 2,
        giver: Role::Widow,
        giver_name: "WIDOW KASIA",
        ask: "ASK ABOUT HER HUSBAND",
        offer: &["I WENT TO LAY FLOWERS ON MY HUSBAND'S GRAVE AND FOUND IT DUG UP. EMPTY. HALF THE GRAVES IN THE BLIGHTED FIELDS ARE THE SAME, AND THERE'S A CELLAR DOOR IN THE OLD MOUND THERE THAT WASN'T THERE LAST SPRING. SOMEONE IS DIGGING. PLEASE. MAKE IT STOP."],
        remind: "THE CELLAR DOOR IN THE BLIGHTED FIELDS. WHATEVER DIGS DOWN THERE, END IT.",
        thanks: &["THE GRAVEDIGGER IS DEAD? THEN MY HUSBAND CAN SLEEP. TAKE THIS... IT WAS HIS. HE'D WANT IT KEEPING SOMEONE ALIVE."],
        goal: Goal::Boss(Kind::Gravedigger),
        reward: Reward::Life(20),
        todo: "SLAY THE GRAVEDIGGER IN HIS CELLAR (THE BLIGHTED FIELDS)",
    },
    SideDef {
        name: "THE BELLS OF MOURNHOLD",
        act: 2,
        giver: Role::Priest,
        giver_name: "FATHER LUCIAN",
        ask: "ASK ABOUT THE SILENT BELLS",
        offer: &["THREE BELLS STAND OUT IN THE MIST, OLDER THAN THE VILLAGE. WHEN THEY RANG, THE DEAD STAYED DOWN. THEN THE BELL-RINGERS DIED, AND THE DEAD CROWDED ROUND THE BELLS. RING THEM AGAIN: THE BLIGHTED FIELDS, THE GALLOWS MOOR, THE BARROW HILLS."],
        remind: "THE BLIGHTED FIELDS, THE GALLOWS MOOR, THE BARROW HILLS. CLEAR THE DEAD FROM EACH BELL AND RING IT.",
        thanks: &["I HEARD THEM FROM HERE, ALL THREE. LISTEN... THE MIST IS QUIETER ALREADY. KNEEL, CHILD. A BLESSING, AND SOMETHING MORE: KNOWLEDGE."],
        goal: Goal::Bells,
        reward: Reward::SkillPoint,
        todo: "RING THE THREE BELLS (BLIGHTED FIELDS, GALLOWS MOOR, BARROW HILLS)",
    },
    SideDef {
        name: "THE PALE HUNTSMAN",
        act: 2,
        giver: Role::Hunter,
        giver_name: "ABELARD",
        ask: "ASK ABOUT HIS RIVAL",
        offer: &["THERE WAS A HUNTER BETTER THAN ME. WE HUNTED THE WOLVES OF THE HOLLOW WOOD TOGETHER, UNTIL ONE BIT HIM. NOW HE HUNTS FOR THEM. PALE AS BONE, FAST AS ANYTHING. END HIM, AND I'LL GIVE YOU THE CHARM WE MADE TOGETHER, YEARS AGO."],
        remind: "THE HOLLOW WOOD. FOLLOW THE KILLS; HE LEAVES THEM WHERE YOU'LL SEE.",
        thanks: &["HE'S GONE, THEN. GOOD. HE'D HAVE WANTED IT. HERE: THE HUNTER'S CHARM. IT NEVER MISSED FOR US."],
        goal: Goal::Super(6),
        reward: Reward::Unique("abelard"),
        todo: "SLAY THE PALE HUNTSMAN (THE HOLLOW WOOD)",
    },
    // ---- Act 4 ----
    SideDef {
        name: "OIL FOR THE SAINT",
        act: 3,
        giver: Role::Oiler,
        giver_name: "BROTHER PISTON",
        ask: "ASK ABOUT THE HOLY OIL",
        offer: &["THE HOLY OIL OF THE ESCAPEMENT, STOLEN! SCAVENGERS DRAGGED THE CASKS INTO THE SCRAPHEAP LABYRINTH, PAST THE SCRAPYARD, AND SOMETHING DOWN THERE IS BUILT OUT OF EVERYTHING THEY EVER STOLE. BRING BACK THE OIL. THE SAINT'S GEARS RUN DRY."],
        remind: "THE SCRAPHEAP LABYRINTH, IN THE SCRAPYARD EAST OF THE GEARFIELDS. MIND THE GOLEM: IT DOESN'T STAY BROKEN.",
        thanks: &["THE OIL! THE SAINT TURNS AGAIN. MADAME VESPER OWES ME A FAVOUR, AND YOU'RE IT: SHE'S MADE YOU SOMETHING. AND A PURSE, FROM THE COLLECTION PLATE."],
        goal: Goal::Boss(Kind::JunkGolem),
        reward: Reward::Rare(500),
        todo: "DESTROY THE JUNK GOLEM IN THE SCRAPHEAP LABYRINTH (THE SCRAPYARD)",
    },
    SideDef {
        name: "TALLY'S COUNT",
        act: 3,
        giver: Role::Tally,
        giver_name: "TALLY",
        ask: "ASK ABOUT THE MISSING LEDGER",
        offer: &["MY LEDGER! EVERY GEAR IN MECHANUS, COUNTED AND ACCOUNTED, AND A SPRING-HEELED LUNATIC SNATCHED IT RIGHT OFF MY DESK. TICK-TOCK JACK, THEY CALL HIM. HE BOUNCES ROUND THE COGWORKS LAUGHING AT HIS OWN JOKES. BRING IT BACK. I'LL PAY. I ALWAYS PAY. IT'S IN THE LEDGER."],
        remind: "TICK-TOCK JACK, IN THE COGWORKS. HE'S FAST. BE FASTER.",
        thanks: &["MY LEDGER. NOT A PAGE MISSING. FIFTEEN HUNDRED GOLD, AS AGREED. I'VE WRITTEN IT DOWN."],
        goal: Goal::Super(14),
        reward: Reward::Gold(1500),
        todo: "CATCH TICK-TOCK JACK (THE COGWORKS)",
    },
    SideDef {
        name: "THE GRAND INQUISITOR",
        act: 3,
        giver: Role::Vesper,
        giver_name: "MADAME VESPER",
        ask: "ASK ABOUT THE PREACHER IN THE FIELDS",
        offer: &["HALVANE. A GRAND INQUISITOR WHO DECIDED THE CLOCK-LAW WASN'T STRICT ENOUGH. HE PREACHES ON THE CLOCKFACE PLAIN, AND HIS ZEALOTS 'CORRECT' ANYONE WHO WALKS OUT OF STEP. SILENCE HIM, DARLING. I'LL MAKE IT WORTH YOUR WHILE."],
        remind: "THE CLOCKFACE PLAIN. FOLLOW THE SOUND OF SERMONS AND SCREAMING.",
        thanks: &["HALVANE, SILENCED. HOW RESTFUL. HERE: A LITTLE CLOCKWORK OF MY OWN. IT TURNS BLADES ASIDE."],
        goal: Goal::Super(13),
        reward: Reward::Ward,
        todo: "SILENCE GRAND INQUISITOR HALVANE (THE CLOCKFACE PLAIN)",
    },
    // ---- Act 5 ----
    SideDef {
        name: "THE BLACK PEARL",
        act: 4,
        giver: Role::Nessa,
        giver_name: "NESSA THE PEARL-DIVER",
        ask: "ASK ABOUT THE GROTTO",
        offer: &["THERE'S A GROTTO IN THE KELP SHALLOWS WHERE THE BLACK PEARLS GROW. YOU CAN ONLY GET IN AT LOW TIDE, AND SOMETHING LIVES IN THERE NOW: OLD BARNACLE, A CRAB THE SIZE OF A COTTAGE. THE BIGGEST BLACK PEARL IN THE DEEP IS UNDER HIM. KILL HIM AND IT'S YOURS... WELL. WE'LL SEE."],
        remind: "THE PEARL GROTTO, IN THE KELP SHALLOWS. WAIT FOR THE TIDE TO GO OUT, OR THE MOUTH IS UNDER WATER.",
        thanks: &["OLD BARNACLE, DEAD? I'VE DIVED FOR TWENTY YEARS AND NEVER SEEN ANYONE DO THAT. I'LL TEACH YOU A DIVER'S TRICK OR TWO FOR IT. BREATHE IN..."],
        goal: Goal::Boss(Kind::Barnacle),
        reward: Reward::SkillPoint,
        todo: "SLAY OLD BARNACLE IN THE PEARL GROTTO (THE KELP SHALLOWS, AT LOW TIDE)",
    },
    SideDef {
        name: "GHOSTS OF THE SOVEREIGN",
        act: 4,
        giver: Role::Ysolde,
        giver_name: "CAPTAIN YSOLDE MARROW",
        ask: "ASK ABOUT HER LOST CREW",
        offer: &["THE SOVEREIGN WENT DOWN WITH MY CREW ABOARD. THREE OF THEM STILL WALK THE REEF: HOLLIS, MY MATE, IN THE CORAL GARDENS; BRANNIGAN, THE COOK, ON THE BONE REEF; AND YOUNG TOBIAS, OUT ON THE ABYSSAL PLAIN. THEY DON'T KNOW THEY'RE DEAD. TELL THEM. GENTLY, IF YOU CAN."],
        remind: "HOLLIS IN THE CORAL GARDENS, BRANNIGAN ON THE BONE REEF, TOBIAS ON THE ABYSSAL PLAIN. LET THEM REST.",
        thanks: &["ALL THREE. THANK YOU. THEY WERE GOOD MEN. TOBIAS WAS ONLY FOURTEEN. TAKE MY OLD CHARM: IT KEPT ME ALIVE WHEN THE SOVEREIGN DIDN'T."],
        goal: Goal::Ghosts,
        reward: Reward::Life(20),
        todo: "LAY YSOLDE'S DROWNED CREW TO REST (CORAL GARDENS, BONE REEF, ABYSSAL PLAIN)",
    },
    SideDef {
        name: "THE SIREN'S PRICE",
        act: 4,
        giver: Role::Coral,
        giver_name: "BROTHER CORAL",
        ask: "ASK ABOUT HIS APPRENTICE",
        offer: &["MY APPRENTICE HEARD SINGING OVER THE TRENCH RIM AND WALKED OUT OF THE BUBBLE AFTER IT. LIRAEL, THE SIREN WHO SINGS THERE. SHE KEEPS WHAT SHE CATCHES. BRING HIM BACK, IF HE CAN STILL BE BROUGHT."],
        remind: "THE TRENCH RIM. YOU'LL HEAR HER BEFORE YOU SEE HER. STOP YOUR EARS IF YOU CAN.",
        thanks: &["HE'S HOME, SHIVERING, ALIVE. THE TIDE BLESS YOU. A TIDE-PRIEST'S BLESSING ISN'T NOTHING: IT TURNS BLADES LIKE WATER TURNS STONES."],
        goal: Goal::Super(16),
        reward: Reward::Ward,
        todo: "SLAY LIRAEL THE SIREN (THE TRENCH RIM)",
    },
];

// ------------------------------------------------------------------ lore

/// Lore pages: (act, title, text). Five per act; the index is the page id.
pub const LORE: &[(u8, &str, &str)] = &[
    (0, "A WARDEN'S OATH", "WE THREE SWORE ON THE ASH ALTAR: BONE, PLAGUE AND HEX, TO GUARD THE SANCTUM UNTIL THE KING WAKES. NONE OF US ASKED WHAT HE WOULD BE WHEN HE DID."),
    (0, "SKRAT'S TALLY", "SKRAT'S: 1 CART. 2 BARRELS ALE. 1 SHINY HAT. 3 HORSES (ATE). 1 EAR (MINE, LOST). ALL SKRAT'S. TOUCH AND DIE."),
    (0, "THE SHEPHERD'S PRAYER", "LORD OF THE GREEN FIELDS, KEEP MY FLOCK FROM THE ASH. I WILL FEED THEM. I WILL ALWAYS FEED THEM. WHATEVER THEY HUNGER FOR."),
    (0, "THE DROWNING", "THEY TIED STONES TO HER FEET AND DROPPED HER IN THE WELL. SHE DID NOT SCREAM. SHE LAUGHED, AND SAID SHE WOULD BE THIRSTY WHEN SHE CAME BACK."),
    (0, "MAREN'S LETTER", "HOLLOWMERE WAS BUILT ON A BURIAL GROUND. EVERY ELDER KNOWS IT. I PRAY THE DEAD UNDER US STAY QUIET, BUT THE ASH HAS WOKEN EVERYTHING ELSE."),
    (1, "THE RAIDER'S SAGA", "WE SAILED WITH HROLF WHEN THE SEA STILL MOVED. THEN THE COLD CAME DOWN FROM THE NORTH ALL AT ONCE, AND THE FJORD BECAME A FLOOR. HROLF SAID WE WOULD WAIT FOR THE THAW. THAT WAS FORTY WINTERS AGO."),
    (1, "GRIMFANG", "THE WHITE WOLF TOOK MY DOGS, THEN MY SHEEP, THEN MY BROTHER. IT DOES NOT HUNT TO EAT. IT HUNTS TO TEACH THE PACK."),
    (1, "THE BRIDE'S VEIL", "SHE WORE HER MOTHER'S VEIL INTO THE WOODS. THEY FOUND IT ON A BRANCH IN SPRING, FROZEN STIFF, AND HER FOOTPRINTS GOING ON INTO THE TREES. JUST HERS."),
    (1, "THE JARL'S BOAST", "HROGAR CLAIMS HE TOOK HIS HORN FROM A FROST GIANT'S CORPSE. HIS MEN SAY HE WON IT AT DICE. THE GIANT IS NOT AVAILABLE FOR COMMENT."),
    (1, "THE LONGSHIP'S LOG", "DAY 3 OF THE ICE. THE CAPTAIN WILL NOT LEAVE THE SHIP. DAY 40. THE CAPTAIN'S BEARD HAS FROZEN TO HIS CHEST. HE LAUGHED. DAY ???. THE CAPTAIN DOES NOT SLEEP NOW. NONE OF US DO."),
    (2, "THE HUNTSMAN'S LAST NOTE", "IT BIT ME AT DUSK. ABELARD THINKS I DON'T KNOW WHAT THAT MEANS. I KNOW. I'LL GO INTO THE WOOD BEFORE THE MOON IS FULL, SO HE NEVER HAS TO DO IT."),
    (2, "A HYMN, UNFINISHED", "SISTERS OF THE MOOR, SING FOR THE DEAD, SING SO THEY SLEEP, SING SO THEY... (THE REST IS SCRATCHED OUT, OVER AND OVER, UNTIL THE PAGE TEARS.)"),
    (2, "THE SENTENCE", "BLACKMOOR, FOR WORSHIPPING THE COUNT IN THE COUNT'S OWN DUNGEON, IS TO HANG BY THE NECK UNTIL DEAD. ADDENDUM: AGAIN. ADDENDUM: WE HAVE RUN OUT OF ROPE."),
    (2, "THE GRAVEDIGGER'S LEDGER", "TWELVE FROM THE FIELDS. NINE FROM THE MOOR. THE COUNT PAYS A SILVER A HEAD AND ASKS NO QUESTIONS, AND I ASK HIM NONE ABOUT WHAT HE DOES WITH THEM."),
    (2, "ELSPETH'S DIARY", "MY LITTLE ONE IS SICK AGAIN. THE COUNT SAYS HE CAN CURE HER, FOR A PRICE. I WILL PAY ANYTHING. I WILL PAY ANYTHING. I PAID."),
    (3, "PRESSURE LOG, BOILER 9", "PRESSURE NOMINAL. PRESSURE HIGH. PRESSURE HIGH. PRESSURE HIGH. PRESSURE (THE NEEDLE HAS BEEN BENT PAST THE LAST MARK BY SOMETHING VERY STRONG.)"),
    (3, "HALVANE'S SERMON", "THE CLOCK DOES NOT FORGIVE. THE CLOCK DOES NOT HURRY. THE CLOCK DOES NOT CARE IF YOU ARE TIRED. BE AS THE CLOCK, AND YOU WILL NEVER BE LATE FOR YOUR JUDGMENT."),
    (3, "A JOKE, WRITTEN ON A GEAR", "WHAT DID THE CLOCKMAKER SAY TO THE THIEF? NOTHING. HE NEVER SAW ME. TICK TOCK. (SIGNED: J.)"),
    (3, "SCAVENGER'S MAP", "THE HEAP KEEPS GROWING. WE BRING IT SCRAP AND IT EATS IT AND STANDS UP TALLER. YESTERDAY IT HAD ARMS. TODAY IT HAD OPINIONS."),
    (3, "AN APPRENTICE'S NOTE", "IF YOU FIND THIS, THE VAULT IN THE ARCHIVE STACKS ONLY OPENS WHEN THE CLOCK BESIDE IT IS STOPPED. I GOT IN. I DID NOT THINK ABOUT GETTING OUT."),
    (4, "THE BOSUN'S ROLL CALL", "HOLLIS. BRANNIGAN. TOBIAS. SALT. MARROW. (THE NAMES ARE CARVED INTO A PLANK, AND SCRATCHED OUT, AND CARVED AGAIN, OVER AND OVER.)"),
    (4, "LIRAEL'S VERSE", "I DO NOT DROWN THEM. THEY COME TO ME. I SING AND THEY WALK INTO THE DARK WATER SMILING, AND IS THAT NOT KINDER THAN THE SEA?"),
    (4, "A DIVER'S WARNING", "IF YOU SEE A LIGHT BOBBING IN THE DEEP TRENCH, DO NOT SWIM TOWARD IT. THE LIGHT IS NOT A LANTERN. THE LIGHT IS BAIT."),
    (4, "NESSA'S DIVE LOG", "GROTTO AGAIN TODAY. THE BLACK PEARL IS STILL THERE, UNDER THE BIG ONE. I COULD REACH IT IF HE WOULD ONLY MOVE. HE NEVER MOVES."),
    (4, "THE SOVEREIGN'S LAST ENTRY", "THE LEVIATHAN ROSE BENEATH US AT THE SECOND BELL. CAPTAIN ORDERED ALL HANDS TO THE BOATS. WE DID NOT HAVE ENOUGH BOATS."),
];

pub fn pages_of(act: usize) -> impl Iterator<Item = usize> {
    LORE.iter().enumerate().filter(move |(_, l)| l.0 as usize == act).map(|(i, _)| i)
}

// ------------------------------------------------------------------ events

/// Seconds between events out in the wilds (a range).
pub const EVENT_GAP: (f32, f32) = (80.0, 150.0);
/// A gold-thief escapes after this long.
pub const HOARDER_TIME: f32 = 22.0;

// ------------------------------------------------------------------ placing it all on a level

/// A random open tile far from the level's start (and from portals), reachable from it.
fn far_spot(lv: &Level, rng: &mut Rng, min: f32) -> Option<(i32, i32)> {
    let (sx, sy) = (lv.start.0 as i32, lv.start.1 as i32);
    for _ in 0..400 {
        let x = rng.range(3, lv.d.w - 3);
        let y = rng.range(3, lv.d.h - 3);
        let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
        let far = ((fx - lv.start.0).powi(2) + (fy - lv.start.1).powi(2)).sqrt();
        if far < min || lv.d.blocked(fx, fy, 0.9) {
            continue;
        }
        if lv.portals.iter().any(|p| (p.x - fx).powi(2) + (p.y - fy).powi(2) < 16.0) {
            continue;
        }
        if let Some((x0, y0, x1, y1)) = lv.safe {
            if fx > x0 - 3.0 && fx < x1 + 3.0 && fy > y0 - 3.0 && fy < y1 + 3.0 {
                continue;
            }
        }
        if lv.d.path((sx, sy), (x, y), 20_000).is_some() {
            return Some((x, y));
        }
    }
    None
}

/// The nearest open tile to (x, y).
fn open_near(lv: &Level, (x, y): (i32, i32)) -> Option<(i32, i32)> {
    for r in 0..8 {
        for dy in -r..=r {
            for dx in -r..=r {
                let (tx, ty) = (x + dx, y + dy);
                if !lv.d.blocked(tx as f32 + 0.5, ty as f32 + 0.5, 0.45) {
                    return Some((tx, ty));
                }
            }
        }
    }
    None
}

/// Puts a level's side content in place: shrines, super uniques and lore pages lying about.
/// Called by world::build_at after the elites and before the difficulty scaling.
pub fn place(lv: &mut Level, seed: u64) {
    let salt = crate::levels::file_name(lv.id).bytes().fold(0x51de_u64, |h, b| h.wrapping_mul(31).wrapping_add(b as u64));
    let mut rng = Rng::new(seed ^ salt.wrapping_mul(0x2545_f491));
    // ---- shrines: 2-3 on an overland, 1-2 on a dungeon floor or in a rift ----
    let n = if lv.id.overland() { rng.range(2, 4) } else { rng.range(1, 3) };
    let act = lv.id.act() as u8;
    for _ in 0..n {
        let Some((x, y)) = far_spot(lv, &mut rng, 10.0) else { break };
        if lv.shrines.iter().any(|s| (s.x - x).abs() + (s.y - y).abs() < 12) {
            continue;
        }
        let kind = BLESSINGS[rng.range(0, BLESSINGS.len() as i32) as usize];
        lv.d.set(x, y, crate::dungeon::Tile::Prop);
        lv.props.push(Prop::on(PropKind::Shrine(act), x, y, 1, 1));
        lv.shrines.push(Shrine { x, y, kind, used: false });
    }
    if matches!(lv.id, LevelId::Rift(_)) {
        return;
    }
    // ---- super uniques and their gangs ----
    for (i, s) in SUPERS.iter().enumerate() {
        if s.home != lv.id {
            continue;
        }
        let spot = if s.spot == (0, 0) { far_spot(lv, &mut rng, 18.0) } else { open_near(lv, s.spot) };
        let Some((x, y)) = spot else { continue };
        let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
        let tier = lv.tier * if lv.id.overland() { 1.2 } else { 1.0 };
        // Clear the spot of other monsters, so it's its own fight.
        lv.mobs.retain(|m| m.boss || crate::breakables::is_prop(m.kind) || (m.x - fx).powi(2) + (m.y - fy).powi(2) > 36.0);
        if let Some(kind) = s.camp {
            if let Some((cx, cy)) = open_near(lv, (x + 3, y - 2)) {
                if (0..3).all(|dx| (0..2).all(|dy| !lv.d.blocked((cx + dx) as f32 + 0.5, (cy + dy) as f32 + 0.5, 0.45))) {
                    for dy in 0..2 {
                        for dx in 0..3 {
                            lv.d.set(cx + dx, cy + dy, crate::dungeon::Tile::Prop);
                        }
                    }
                    lv.props.push(Prop::on(kind, cx, cy, 3, 2));
                }
            }
        }
        let mut m = Mob::new(s.kind, fx, fy, tier, &mut rng);
        m.promote(Rank::Elite, s.mods, Some(s.name.into()));
        // Tougher than an ordinary elite leader.
        m.max_hp *= 1.5;
        m.hp = m.max_hp;
        m.xp *= 1.5;
        m.superu = i as u8 + 1;
        lv.mobs.push(m);
        for k in 0..s.gang.1 {
            let a = k as f32 / s.gang.1 as f32 * std::f32::consts::TAU;
            let (gx, gy) = (fx + a.cos() * 1.4, fy + a.sin() * 1.4);
            if lv.d.blocked(gx, gy, 0.35) {
                continue;
            }
            let mut g = Mob::new(s.gang.0, gx, gy, tier, &mut rng);
            g.promote(Rank::Minion, s.mods & (M_FAST | M_FIERY), None);
            lv.mobs.push(g);
        }
    }
    // ---- the optional dungeons' pages lie on their first floor ----
    if let LevelId::Dungeon(k, 0) = lv.id {
        if let Some(page) = OPTIONAL.iter().find(|o| o.0 == k).map(|o| o.1) {
            if let Some((x, y)) = far_spot(lv, &mut rng, 12.0) {
                lv.pickups.push(Pickup { x: x as f32 + 0.5, y: y as f32 + 0.5, kind: Drop::Page(page), t: 1.0 });
            }
        }
    }
}

/// The optional dungeons: (dungeon index, the lore page on its first floor).
pub const OPTIONAL: &[(usize, u8)] = &[(crate::world::CHARNEL, 3), (crate::world::LONGSHIP, 9), (crate::world::CELLAR, 13), (crate::world::MANOR, 14), (crate::world::SCRAPHEAP, 18), (crate::world::GROTTO, 23)];

// ------------------------------------------------------------------ the game side

impl Game {
    /// A shrine blessing in force: (kind, seconds left).
    pub fn blessing(&self) -> Option<(Blessing, f32)> {
        Blessing::from_id(self.p.blessing).filter(|_| self.p.bless_t > 0.0).map(|b| (b, self.p.bless_t))
    }

    pub fn blessed(&self, b: Blessing) -> bool {
        self.blessing().map_or(false, |(k, _)| k == b)
    }

    /// Damage dealt (Combat shrine).
    pub fn bless_damage(&self) -> f32 {
        let shrine = if self.blessed(Blessing::Combat) { 1.3 } else { 1.0 };
        // The Bog Witch's Wolf's Heart (mist.rs).
        shrine * self.pact_damage()
    }

    /// Damage taken: the Armor shrine, and the wards earned from side quests.
    pub fn bless_taken(&self) -> f32 {
        let armor = if self.blessed(Blessing::Armor) { 0.75 } else { 1.0 };
        armor * (1.0 - 0.05 * self.p.ward.min(6) as f32)
    }

    /// Experience: the Experience shrine, and a finished act of lore pages.
    pub fn bless_xp(&self) -> f32 {
        let shrine = if self.blessed(Blessing::Experience) { 1.4 } else { 1.0 };
        let act = self.level.act();
        let lore = if pages_of(act).count() > 0 && pages_of(act).all(|i| self.p.pages & (1 << i) != 0) { 1.05 } else { 1.0 };
        shrine * lore
    }

    /// Movement and cast speed (Haste shrine).
    pub fn bless_speed(&self) -> f32 {
        if self.blessed(Blessing::Haste) {
            1.25
        } else {
            1.0
        }
    }

    /// Mana regeneration (Mana shrine).
    pub fn bless_mana(&self) -> f32 {
        if self.blessed(Blessing::Mana) {
            3.0
        } else {
            1.0
        }
    }

    /// Every tick: shrines you touch, the blessing running out, events, gold-thieves, supers' shouts.
    pub(crate) fn update_side(&mut self) {
        let dt = crate::game::DT;
        if self.p.bless_t > 0.0 {
            self.p.bless_t -= dt;
            if self.p.bless_t <= 0.0 {
                let was = self.p.blessing;
                self.p.blessing = 0;
                if Blessing::from_id(was) == Some(Blessing::Skill) {
                    self.p.recalc();
                }
            }
        }
        // ---- shrines: walk up to one ----
        let (px, py) = (self.p.x, self.p.y);
        if let Some(i) = self.shrines.iter().position(|s| !s.used && {
            let (sx, sy) = s.at();
            (sx - px).powi(2) + (sy - py).powi(2) < 1.3 * 1.3
        }) {
            self.use_shrine(i);
        }
        // ---- super uniques shout when they see you ----
        for i in 0..self.mobs.len() {
            let m = &self.mobs[i];
            if m.superu == 0 || !m.alive() || m.shouted || m.state == MobState::Idle {
                continue;
            }
            let (x, y, line) = (m.x, m.y, SUPERS[m.superu as usize - 1].line);
            self.mobs[i].shouted = true;
            self.floater(x, y - 1.0, line.into(), rgb(0xffd080));
        }
        // ---- the gold-thief runs, and gets away in the end ----
        for i in 0..self.mobs.len() {
            if self.mobs[i].kind != Kind::Hoarder || !self.mobs[i].alive() {
                continue;
            }
            self.mobs[i].flee = 1.0;
            if self.mobs[i].state == MobState::Idle {
                self.mobs[i].state = MobState::Chase;
            }
            self.mobs[i].special += dt;
            if self.mobs[i].special > HOARDER_TIME {
                let (x, y) = (self.mobs[i].x, self.mobs[i].y);
                self.mobs[i].state = MobState::Dead(10.0);
                self.mobs[i].hp = 0.0;
                for _ in 0..16 {
                    self.spray_at(x, y, PKind::Magic, 20.0);
                }
                self.say("THE GOLD-THIEF GOT AWAY...".into());
            }
        }
        // ---- events out in the wilds ----
        let wild = !self.in_safe(px, py) && !self.in_rift() && !matches!(self.state, crate::game::State::Dead(_));
        if wild {
            self.event_cd -= dt;
            if self.event_cd <= 0.0 {
                self.event_cd = self.rng.rf(EVENT_GAP.0, EVENT_GAP.1);
                let r = self.rng.f();
                let ok = if r < 0.4 {
                    self.event_ambush()
                } else if r < 0.7 {
                    self.event_hoarder()
                } else {
                    self.event_fallen()
                };
                if !ok {
                    // No room here: try again soon.
                    self.event_cd = 10.0;
                }
            }
        }
    }

    fn use_shrine(&mut self, i: usize) {
        self.shrines[i].used = true;
        let kind = self.shrines[i].kind;
        let (x, y) = self.shrines[i].at();
        self.p.shrines_used += 1;
        self.sfx.push(Sfx::Descend);
        self.lights.push(Light { x, y, r: 160.0, s: 1.2, life: 1.0, max: 1.0 });
        for _ in 0..24 {
            self.spray_at(x, y, PKind::Magic, 26.0);
        }
        self.floater(x, y, format!("{}: {}", kind.name(), kind.short()), kind.col());
        match kind {
            Blessing::Refill => {
                self.p.hp = self.p.max_hp;
                self.p.mana = self.p.max_mana;
            }
            Blessing::Gem => {
                let ilvl = crate::items::ilvl_for(self.tier);
                let gem = crate::items::gem_item(crate::items::roll_gem(ilvl.saturating_add(4), &mut self.rng));
                self.pickups.push(Pickup { x: self.p.x, y: self.p.y + 0.3, kind: Drop::Item(Box::new(gem)), t: 0.0 });
            }
            _ => {
                let was = self.p.blessing;
                self.p.blessing = kind.id();
                self.p.bless_t = BLESS_TIME;
                if kind == Blessing::Skill || Blessing::from_id(was) == Some(Blessing::Skill) {
                    self.p.recalc();
                }
            }
        }
    }

    /// A random open spot `r` tiles (give or take) from the hero that the hero can walk to.
    fn spot_near(&mut self, r: f32) -> Option<(f32, f32)> {
        let (px, py) = (self.p.x, self.p.y);
        for _ in 0..40 {
            let a = self.rng.f() * std::f32::consts::TAU;
            let rr = self.rng.rf(r * 0.8, r * 1.2);
            let (x, y) = (px + a.cos() * rr, py + a.sin() * rr);
            if self.d.blocked(x, y, 0.5) || self.in_safe(x, y) {
                continue;
            }
            if self.d.path((px as i32, py as i32), (x as i32, y as i32), 4000).is_some() {
                return Some((x, y));
            }
        }
        None
    }

    /// The monsters that live here (for ambushes): the level's own, not bosses or breakables.
    pub(crate) fn local_kinds(&self) -> Vec<Kind> {
        let mut kinds: Vec<Kind> = vec![];
        for m in &self.mobs {
            if !m.boss && m.superu == 0 && m.charm <= 0.0 && !crate::breakables::is_prop(m.kind) && m.kind != Kind::Hoarder && def_is_wild(m.kind) && !kinds.contains(&m.kind) {
                kinds.push(m.kind);
            }
        }
        kinds
    }

    /// The ground shakes and two packs close in, each led by a champion.
    fn event_ambush(&mut self) -> bool {
        let kinds = self.local_kinds();
        if kinds.is_empty() {
            return false;
        }
        let mut spawned = 0;
        for _ in 0..2 {
            let Some((x, y)) = self.spot_near(7.5) else { continue };
            let kind = kinds[self.rng.range(0, kinds.len() as i32) as usize];
            let mods = crate::mobs::roll_mods(1, &mut self.rng);
            for k in 0..3 {
                let a = k as f32 * 2.1;
                let (mx, my) = (x + a.cos() * 0.9, y + a.sin() * 0.9);
                if self.d.blocked(mx, my, 0.35) {
                    continue;
                }
                let mut m = Mob::new(kind, mx, my, self.tier, &mut self.rng);
                // A champion leads each pack; the rest are ordinary (six champions at once were a wall early on).
                if k == 0 {
                    m.promote(Rank::Champion, mods, None);
                }
                m.state = MobState::Chase;
                self.mobs.push(m);
                spawned += 1;
            }
        }
        if spawned == 0 {
            return false;
        }
        self.shake = self.shake.max(0.6);
        self.sfx.push(Sfx::Boom);
        self.say("AMBUSH!".into());
        true
    }

    /// A gold-thief, laden with loot, bolts when it sees you.
    fn event_hoarder(&mut self) -> bool {
        let Some((x, y)) = self.spot_near(8.0) else { return false };
        let mut m = Mob::new(Kind::Hoarder, x, y, self.tier, &mut self.rng);
        m.state = MobState::Chase;
        m.flee = 1.0;
        self.mobs.push(m);
        self.say("A GOLD-THIEF! CATCH IT BEFORE IT GETS AWAY".into());
        true
    }

    /// A fallen adventurer: a pack to loot, and maybe a page.
    fn event_fallen(&mut self) -> bool {
        let Some((x, y)) = self.spot_near(6.0) else { return false };
        self.decals.push(Decal { x, y, r: 0.6, col: rgb(0x301810), a: 0.6 });
        let ilvl = crate::items::ilvl_for(self.tier);
        let gold = (20.0 + 15.0 * self.tier) as i32;
        let mut loot = vec![Drop::Gold(gold), if self.rng.chance(0.5) { Drop::Health } else { Drop::Mana }];
        let mf = self.p.bonus.get(crate::items::Stat::Magic);
        loot.push(Drop::Item(Box::new(crate::items::drop(ilvl + 1, mf + 50, true, &mut self.rng))));
        if let Some(page) = self.missing_page(self.level.act()) {
            loot.push(Drop::Page(page as u8));
        }
        let n = loot.len();
        for (k, kind) in loot.into_iter().enumerate() {
            let a = k as f32 / n as f32 * std::f32::consts::TAU;
            self.pickups.push(Pickup { x: x + a.cos() * 0.5, y: y + a.sin() * 0.5, kind, t: 0.0 });
        }
        self.say("A FALLEN ADVENTURER LIES NEARBY. HIS PACK IS STILL FULL".into());
        true
    }

    /// A lore page of this act you haven't found yet (fallen adventurers carry them).
    pub fn missing_page(&self, act: usize) -> Option<usize> {
        // Pages carried by super uniques or lying in optional dungeons are found there, not on the fallen.
        let fixed: Vec<usize> = SUPERS.iter().filter_map(|s| s.page.map(|p| p as usize)).chain(OPTIONAL.iter().map(|o| o.1 as usize)).collect();
        pages_of(act).find(|&i| self.p.pages & (1 << i) == 0 && !fixed.contains(&i))
    }

    /// A monster died: super uniques' loot and quests, the gold-thief's hoard, optional bosses' quests.
    pub(crate) fn side_kill(&mut self, i: usize) {
        let (x, y, kind, superu) = (self.mobs[i].x, self.mobs[i].y, self.mobs[i].kind, self.mobs[i].superu);
        if kind == Kind::Hoarder {
            // Its hoard bursts out.
            for k in 0..7 {
                let a = k as f32 / 7.0 * std::f32::consts::TAU;
                let g = (8.0 + 6.0 * self.tier) as i32;
                self.pickups.push(Pickup { x: x + a.cos() * 0.9, y: y + a.sin() * 0.9, kind: Drop::Gold(g), t: 0.0 });
            }
            let ilvl = crate::items::ilvl_for(self.tier);
            self.pickups.push(Pickup { x, y, kind: Drop::Item(Box::new(crate::items::gem_item(crate::items::roll_gem(ilvl + 3, &mut self.rng)))), t: 0.0 });
            if self.rng.chance(0.35) {
                self.pickups.push(Pickup { x, y: y + 0.4, kind: Drop::Item(Box::new(crate::items::roll(ilvl + 2, crate::items::Rarity::Rare, &mut self.rng))), t: 0.0 });
            }
            self.say("THE GOLD-THIEF'S HOARD SPILLS OUT!".into());
            return;
        }
        if superu > 0 {
            let s = &SUPERS[superu as usize - 1];
            self.p.supers |= 1 << (superu - 1);
            let ilvl = crate::items::ilvl_for(self.tier) + 2;
            let mut drops = vec![Drop::Item(Box::new(crate::items::roll(ilvl, crate::items::Rarity::Rare, &mut self.rng)))];
            if self.rng.chance(0.2) {
                if let Some(u) = crate::items::boss_unique(s.unique) {
                    drops.push(Drop::Item(Box::new(u)));
                }
            }
            if let Some(page) = s.page {
                if self.p.pages & (1 << page) == 0 {
                    drops.push(Drop::Page(page));
                }
            }
            drops.push(Drop::Gold((30.0 + 20.0 * self.tier) as i32));
            let n = drops.len();
            for (k, kind) in drops.into_iter().enumerate() {
                let a = k as f32 / n as f32 * std::f32::consts::TAU + 0.3;
                self.pickups.push(Pickup { x: x + a.cos() * 0.8, y: y + a.sin() * 0.8, kind, t: 0.0 });
            }
            self.floater(x, y, format!("{} IS SLAIN", s.name), rgb(0xffd080));
            self.side_progress(Goal::Super(superu as usize - 1));
        }
        if self.mobs[i].boss {
            self.side_progress(Goal::Boss(kind));
        }
    }

    /// A side quest's goal was reached.
    pub(crate) fn side_progress(&mut self, goal: Goal) {
        for (q, s) in SIDES.iter().enumerate() {
            if s.goal == goal && self.quest.side[q] < S_DONE {
                self.quest.side[q] = S_DONE;
                self.save_due = true;
                self.say(format!("{}: DONE. RETURN TO {}", s.name, s.giver_name));
            }
        }
    }

    /// Adds the side quest options of this person to a conversation.
    pub(crate) fn side_options(&self, role: Role, d: &mut Dialog) {
        let act = self.level.act() as u8;
        for (q, s) in SIDES.iter().enumerate() {
            if s.giver != role || s.act != act {
                continue;
            }
            let label = match self.quest.side[q] {
                S_NONE => s.ask.to_string(),
                S_GIVEN => format!("{} (OPEN)", s.name),
                S_DONE => format!("{}: IT'S DONE", s.name),
                _ => continue,
            };
            let opt = (label, Act::Side(q as u8));
            // Just above FAREWELL, so the person's own options (shop, healing...) keep their place.
            let list = if d.pages.len() > 1 { &mut d.last_options } else { &mut d.options };
            if list.is_empty() || list.iter().all(|o| o.1 == Act::Next) {
                *list = vec![("FAREWELL".into(), Act::Close)];
            }
            let at = if list.last().map_or(false, |o| o.1 == Act::Close) { list.len() - 1 } else { list.len() };
            list.insert(at, opt);
        }
        d.refresh_options();
    }

    /// Talking about a side quest: the offer, a reminder, or the thanks and the reward.
    pub(crate) fn side_talk(&mut self, q: usize) {
        let s = &SIDES[q];
        let mut d = match self.quest.side[q] {
            S_NONE => {
                self.quest.side[q] = S_GIVEN;
                self.save_due = true;
                let mut d = Dialog::new(s.giver_name, s.offer);
                d.last_options = vec![("I'LL SEE TO IT".into(), Act::Close)];
                d
            }
            S_GIVEN => Dialog::new(s.giver_name, &[s.remind]),
            _ => {
                self.quest.side[q] = S_PAID;
                self.save_due = true;
                let text = self.give_reward(s.reward);
                let mut pages: Vec<String> = s.thanks.iter().map(|t| t.to_string()).collect();
                pages.push(format!("{}: {}", s.name, text));
                let refs: Vec<&str> = pages.iter().map(|p| p.as_str()).collect();
                Dialog::new(s.giver_name, &refs)
            }
        };
        d.refresh_options();
        self.dialog = Some(d);
    }

    fn give_reward(&mut self, r: Reward) -> String {
        self.sfx.push(Sfx::Descend);
        match r {
            Reward::SkillPoint => self.p.skills.points += 1,
            Reward::Life(n) => {
                self.p.base_hp += n as f32;
                self.p.recalc();
                self.p.hp = self.p.max_hp;
            }
            Reward::Rare(g) => {
                self.p.gold += g;
                let ilvl = crate::items::ilvl_for(self.tier) + 3;
                let it = crate::items::roll(ilvl, crate::items::Rarity::Rare, &mut self.rng);
                // A full bag: it lands at your feet.
                if let Err(it) = self.p.gear.add(it) {
                    self.pickups.push(Pickup { x: self.p.x, y: self.p.y + 0.4, kind: Drop::Item(Box::new(it)), t: 0.0 });
                }
            }
            Reward::Ward => self.p.ward = (self.p.ward + 1).min(6),
            Reward::Unique(k) => {
                if let Some(u) = crate::items::boss_unique(k) {
                    if let Err(u) = self.p.gear.add(u) {
                        self.pickups.push(Pickup { x: self.p.x, y: self.p.y + 0.4, kind: Drop::Item(Box::new(u)), t: 0.0 });
                    }
                }
            }
            Reward::Socket => {
                // The weapon first, then the armour.
                for slot in 0..self.p.gear.worn.len() {
                    if let Some(it) = self.p.gear.worn[slot].as_mut() {
                        if it.sockets < crate::items::max_sockets(it.slot()) && matches!(it.slot(), crate::items::Slot::Weapon | crate::items::Slot::Armor) {
                            it.sockets += 1;
                            break;
                        }
                    }
                }
            }
            Reward::Respec => {
                self.p.skills.respec();
            }
            Reward::Gold(g) => self.p.gold += g,
        }
        r.text()
    }

    /// Picked up a lore page: read it.
    pub(crate) fn read_page(&mut self, page: u8) {
        let i = page as usize;
        let Some(&(act, title, text)) = LORE.get(i) else { return };
        let new = self.p.pages & (1 << i) == 0;
        self.p.pages |= 1 << i;
        if new {
            let xp = 40.0 * (1.0 + act as f32) * (1.0 + self.quest.difficulty as f32);
            self.gain_xp(xp);
            self.save_due = true;
        }
        let have = pages_of(act as usize).filter(|&k| self.p.pages & (1 << k) != 0).count();
        let all = pages_of(act as usize).count();
        let mut pages = vec![text.to_string()];
        if new && have == all {
            pages.push(format!("ALL {all} PAGES OF THIS ACT FOUND: +5% EXPERIENCE HERE, FOR GOOD"));
        } else {
            pages.push(format!("LORE PAGE {have}/{all} OF THIS ACT (SEE THE MAP: TAB / M)"));
        }
        let refs: Vec<&str> = pages.iter().map(|p| p.as_str()).collect();
        let mut d = Dialog::new(title, &refs);
        d.refresh_options();
        self.dialog = Some(d);
    }

    /// The quest log line of this act's open side quests (shown under the main quest).
    pub fn side_log(&self) -> Option<String> {
        let act = self.level.act() as u8;
        SIDES.iter().enumerate().find(|(q, s)| s.act == act && matches!(self.quest.side[*q], S_GIVEN | S_DONE)).map(|(q, s)| {
            if self.quest.side[q] == S_DONE {
                format!("{}: RETURN TO {}", s.name, s.giver_name)
            } else {
                s.todo.to_string()
            }
        })
    }

    /// The journal (drawn beside the map): this act's pages, side quests and super uniques, and shrines.
    pub fn journal(&self) -> Vec<(String, u32)> {
        let act = self.level.act();
        let mut out = vec![];
        let have = pages_of(act).filter(|&k| self.p.pages & (1 << k) != 0).count();
        out.push((format!("LORE PAGES  {have}/{}", pages_of(act).count()), rgb(0xe8d8b0)));
        for k in pages_of(act) {
            let found = self.p.pages & (1 << k) != 0;
            out.push((if found { format!("  {}", LORE[k].1) } else { "  ???".into() }, if found { rgb(0xc8b890) } else { rgb(0x6a5a48) }));
        }
        out.push((String::new(), 0));
        out.push(("SIDE QUESTS".into(), rgb(0xe8d8b0)));
        for (q, s) in SIDES.iter().enumerate().filter(|(_, s)| s.act as usize == act) {
            let (state, col) = match self.quest.side[q] {
                S_NONE => ("NOT FOUND YET", 0x6a5a48),
                S_GIVEN => ("OPEN", 0xe0c060),
                S_DONE => ("DONE: RETURN", 0x80e080),
                _ => ("COMPLETE", 0x80a080),
            };
            let name = if self.quest.side[q] == S_NONE { "???" } else { s.name };
            out.push((format!("  {name}  {state}"), rgb(col)));
        }
        out.push((String::new(), 0));
        out.push(("SUPER UNIQUES".into(), rgb(0xe8d8b0)));
        for (i, s) in SUPERS.iter().enumerate().filter(|(_, s)| s.home.act() == act) {
            let slain = self.p.supers & (1 << i) != 0;
            out.push((if slain { format!("  {}  SLAIN", s.name) } else { "  ???".into() }, if slain { rgb(0xd8a850) } else { rgb(0x6a5a48) }));
        }
        // The Wailing Shade (mist.rs).
        if act == 2 && self.feats.shade > 0 {
            let (t, c) = match (self.feats.shade, self.feats.shade_known) {
                (2, _) => ("THE WAILING SHADE  AT REST".to_string(), 0x80a080),
                (_, true) => (format!("THE WAILING SHADE  ITS GRAVE: {}", Game::waypoint_name(crate::mist::tomb_area(self.world_seed, self.quest.difficulty))), 0xc0d8ff),
                _ => ("THE WAILING SHADE  HAUNTS YOU (WOUND IT)".to_string(), 0xc0d8ff),
            };
            out.push((String::new(), 0));
            out.push((t, rgb(c)));
        }
        let errands = self.errand_journal();
        if !errands.is_empty() {
            out.push((String::new(), 0));
            out.push(("AREA TASKS".into(), rgb(0xe8d8b0)));
            out.extend(errands);
        }
        out.push((String::new(), 0));
        out.push((format!("SHRINES USED  {}", self.p.shrines_used), rgb(0xa8a0c0)));
        if self.p.ward > 0 {
            out.push((format!("WARDS  -{}% DAMAGE TAKEN", 5 * self.p.ward), rgb(0xa8a0c0)));
        }
        out
    }
}

/// The monster kinds an ambush can bring (not the heroes' companions).
fn def_is_wild(k: Kind) -> bool {
    !matches!(k, Kind::Einherjar | Kind::DireWolf | Kind::Scholar | Kind::Rat | Kind::MossWolf | Kind::ThornWarden | Kind::Bat)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Game;

    #[test]
    fn shrines_bless_you_and_go_dark() {
        let mut g = Game::new(7, 360);
        g.debug_goto(LevelId::Dungeon(0, 0));
        assert!(!g.shrines.is_empty(), "a dungeon floor has a shrine");
        let s0 = g.shrines[0];
        assert!(g.props.iter().any(|p| matches!(p.kind, PropKind::Shrine(_)) && (p.foot.0, p.foot.1) == (s0.x, s0.y)), "its prop is drawn");
        g.shrines[0].kind = Blessing::Combat;
        let (x, y) = g.shrines[0].at();
        (g.p.x, g.p.y) = (x + 0.8, y);
        g.update_side();
        assert!(g.shrines[0].used);
        assert!(g.blessed(Blessing::Combat));
        assert!(g.bless_damage() > 1.2);
        g.p.bless_t = 0.01;
        g.update_side();
        assert!(g.blessing().is_none(), "it wears off");
        // A used shrine can't be used again.
        g.update_side();
        assert!(g.blessing().is_none());
    }

    #[test]
    fn super_uniques_live_where_they_should_and_pay_out() {
        let mut g = Game::new(7, 360);
        g.debug_goto(LevelId::Area(0, 8));
        let i = g.mobs.iter().position(|m| m.superu == 1).expect("Skrat lives in his gulch");
        assert!(g.props.iter().any(|p| p.kind == PropKind::Cart), "on Gerta's cart");
        assert_eq!(g.mobs[i].label(), "SKRAT ONE-EAR");
        assert!(g.mobs.iter().filter(|m| m.kind == Kind::Goblin && m.rank == Rank::Minion).count() >= 3, "with his gang");
        // Gerta's quest: take it, kill him, collect.
        g.quest.side[1] = S_GIVEN;
        let gold = g.p.gold;
        let before = g.pickups.len();
        g.kill(i);
        assert!(g.pickups.len() > before + 1, "a rare, gold and a page");
        assert!(g.pickups.iter().any(|k| matches!(k.kind, Drop::Page(1))));
        assert_eq!(g.quest.side[1], S_DONE);
        assert!(g.p.supers & 1 != 0, "in the journal");
        g.side_talk(1);
        assert_eq!(g.quest.side[1], S_PAID);
        assert!(g.p.gold >= gold + 300);
    }

    #[test]
    fn side_quests_are_offered_and_rewarded() {
        let mut g = Game::new(7, 360);
        g.debug_goto(LevelId::Overworld);
        let i = g.npcs.iter().position(|n| n.role == Role::Healer).unwrap();
        g.open_dialog(i);
        let d = g.dialog.as_ref().unwrap();
        let all: Vec<_> = d.options.iter().chain(d.last_options.iter()).collect();
        assert!(all.iter().any(|o| o.1 == Act::Side(0)), "Aldric asks about the well");
        g.side_talk(0);
        assert_eq!(g.quest.side[0], S_GIVEN);
        assert!(g.side_log().unwrap().contains("WELL-WITCH"));
        let pts = g.p.skills.points;
        g.side_progress(Goal::Boss(Kind::WellWitch));
        g.side_talk(0);
        assert_eq!(g.p.skills.points, pts + 1, "a skill point");
    }

    #[test]
    fn lore_pages_teach_and_complete_an_act() {
        let mut g = Game::new(7, 360);
        g.debug_goto(LevelId::Overworld);
        let xp = g.p.xp + g.p.clvl as f32 * 1000.0;
        g.read_page(0);
        assert!(g.p.pages & 1 != 0);
        assert!(g.p.xp + g.p.clvl as f32 * 1000.0 > xp);
        assert!(g.dialog.is_some());
        assert_eq!(g.bless_xp(), 1.0);
        for k in pages_of(0) {
            g.read_page(k as u8);
        }
        assert!(g.bless_xp() > 1.0, "all five: +5% XP in Act 1");
    }

    #[test]
    fn events_happen_in_the_wilds() {
        let mut g = Game::new(7, 360);
        g.debug_goto(LevelId::Dungeon(1, 0));
        let n = g.mobs.len();
        assert!(g.event_ambush());
        assert!(g.mobs.len() > n);
        assert!(g.event_hoarder());
        let h = g.mobs.iter().position(|m| m.kind == Kind::Hoarder).unwrap();
        let before = g.pickups.len();
        g.kill(h);
        assert!(g.pickups.len() >= before + 8, "its hoard spills out");
        assert!(g.event_fallen());
    }
}

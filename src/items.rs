//! Equipment, D2 style: base types per slot, white / magic (blue) / rare (yellow) /
//! unique (gold) items with random affixes, the bag and the worn gear, and the stat
//! totals the game reads (`Bonus`). No SDL here; the inventory screen is in `inventory.rs`.
use crate::rng::Rng;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Slot {
    Weapon,
    Helm,
    Armor,
    Gloves,
    Boots,
    Belt,
    Ring,
    Amulet,
    /// Gems go in sockets, never on your body.
    Gem,
}

/// Worn slots, in paper-doll order (two rings).
pub const WORN: [Slot; 9] = [Slot::Weapon, Slot::Helm, Slot::Armor, Slot::Gloves, Slot::Boots, Slot::Belt, Slot::Ring, Slot::Ring, Slot::Amulet];
pub const BAG: usize = 30;
pub const BAG_COLS: usize = 10;
/// Gerta's shelf (two rows).
pub const SHOP: usize = 20;
/// The stash chest in Hollowmere (three rows).
pub const STASH: usize = 30;

pub fn slot_name(s: Slot) -> &'static str {
    match s {
        Slot::Weapon => "STAFF",
        Slot::Helm => "HELM",
        Slot::Armor => "ARMOR",
        Slot::Gloves => "GLOVES",
        Slot::Boots => "BOOTS",
        Slot::Belt => "BELT",
        Slot::Ring => "RING",
        Slot::Amulet => "AMULET",
        Slot::Gem => "GEM",
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord)]
pub enum Rarity {
    Normal,
    Magic,
    Rare,
    Unique,
    /// Part of a set (green): more bonuses the more pieces you wear.
    Set,
    /// Ancient (endgame.rs): a unique's stats rolled stronger, plus an ancient power. Very rare.
    Ancient,
}

/// Name colour per rarity (D2: white, blue, yellow, gold).
pub fn rarity_col(r: Rarity) -> u32 {
    match r {
        Rarity::Normal => 0xd8d8d0,
        Rarity::Magic => 0x7090ff,
        Rarity::Rare => 0xf0e060,
        Rarity::Unique => 0xc89850,
        Rarity::Set => 0x40d040,
        Rarity::Ancient => 0xff5a20,
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Stat {
    Life,
    Mana,
    /// % fire damage.
    Fire,
    /// Damage reduction rating.
    Armor,
    /// Life per second.
    LifeRegen,
    /// % mana regeneration.
    ManaRegen,
    /// % faster casting.
    Cast,
    /// % faster walking and running.
    Move,
    /// % slower stamina drain.
    Stamina,
    /// % slower hunger.
    Hunger,
    /// + to all fire skills you know.
    Skills,
    /// % extra gold.
    Gold,
    /// % better chance of magic items.
    Magic,
    LifeOnKill,
    ManaOnKill,
    /// Ancient powers (bit flags, see POWERS): OR-ed together, not added.
    Power,
}

pub const STATS: [Stat; 16] = [
    Stat::Skills,
    Stat::Fire,
    Stat::Life,
    Stat::Mana,
    Stat::Armor,
    Stat::LifeRegen,
    Stat::ManaRegen,
    Stat::Cast,
    Stat::Move,
    Stat::Stamina,
    Stat::Hunger,
    Stat::LifeOnKill,
    Stat::ManaOnKill,
    Stat::Gold,
    Stat::Magic,
    Stat::Power,
];

pub fn stat_key(s: Stat) -> &'static str {
    match s {
        Stat::Life => "life",
        Stat::Mana => "mana",
        Stat::Fire => "fire",
        Stat::Armor => "armor",
        Stat::LifeRegen => "regen",
        Stat::ManaRegen => "mregen",
        Stat::Cast => "cast",
        Stat::Move => "move",
        Stat::Stamina => "stamina",
        Stat::Hunger => "hunger",
        Stat::Skills => "skills",
        Stat::Gold => "gold",
        Stat::Magic => "magic",
        Stat::LifeOnKill => "lifekill",
        Stat::ManaOnKill => "manakill",
        Stat::Power => "power",
    }
}

/// One line of an item's description.
pub fn stat_text(s: Stat, v: i32) -> String {
    match s {
        Stat::Life => format!("+{v} TO LIFE"),
        Stat::Mana => format!("+{v} TO MANA"),
        Stat::Fire => format!("+{v}% FIRE DAMAGE"),
        Stat::Armor => format!("+{v} ARMOR"),
        Stat::LifeRegen => format!("REPLENISH LIFE +{v}/S"),
        Stat::ManaRegen => format!("+{v}% MANA REGENERATION"),
        Stat::Cast => format!("+{v}% FASTER CAST RATE"),
        Stat::Move => format!("+{v}% FASTER RUN/WALK"),
        Stat::Stamina => format!("{v}% SLOWER STAMINA DRAIN"),
        Stat::Hunger => format!("{v}% SLOWER HUNGER"),
        Stat::Skills => format!("+{v} TO FIRE SKILLS"),
        Stat::Gold => format!("+{v}% EXTRA GOLD"),
        Stat::Magic => format!("+{v}% BETTER CHANCE OF MAGIC ITEMS"),
        Stat::LifeOnKill => format!("+{v} LIFE AFTER EACH KILL"),
        Stat::ManaOnKill => format!("+{v} MANA AFTER EACH KILL"),
        Stat::Power => (0..POWERS.len())
            .filter(|i| v & (1 << i) != 0)
            .map(|i| format!("ANCIENT POWER - {}: {}", POWERS[i].0, POWERS[i].1))
            .collect::<Vec<_>>()
            .join(" / "),
    }
}

/// Ancient powers: (name, what it does, the hero it's for, or None for anyone). Bit i of Stat::Power.
pub const POWERS: [(&str, &str, Option<crate::skills::Class>); 16] = [
    ("EMBERSTORM", "SLAIN FOES BURST, SCORCHING THOSE AROUND THEM", None),
    ("BLOODTHIRST", "4% OF YOUR DAMAGE HEALS YOU", None),
    ("ASHWALKER", "+20% FASTER RUN/WALK", None),
    ("GIANTSLAYER", "+40% DAMAGE TO BOSSES", None),
    ("THE LONG NIGHT", "+50% DAMAGE BELOW A THIRD OF YOUR LIFE", None),
    ("SECOND WIND", "ONCE A MINUTE A KILLING BLOW LEAVES YOU AT HALF LIFE", None),
    ("STORMCALLER", "SOME BLOWS ARC TO A NEARBY FOE", None),
    ("PHOENIX ASH", "COOLDOWNS RECOVER 30% FASTER", None),
    ("FIRESTORM", "YOUR FIREBALLS SPLIT IN THREE", Some(crate::skills::Class::Sorceress)),
    ("BLOODLORD", "YOUR DRAINS HEAL TWICE AS MUCH", Some(crate::skills::Class::Vampire)),
    ("ENDLESS BOILER", "YOUR GUNS NEVER LOCK FROM OVERHEATING", Some(crate::skills::Class::Inventor)),
    ("VALHALLA'S CALL", "VALOR FILLS TWICE AS FAST", Some(crate::skills::Class::Valkyrie)),
    ("UNDYING RAGE", "RAGE NEVER FADES", Some(crate::skills::Class::Berserker)),
    ("SOUL HARVEST", "CARRY FIVE MORE SOULS", Some(crate::skills::Class::Reaper)),
    ("COLOSSI", "YOUR CREATURES HAVE HALF AGAIN AS MUCH LIFE AND DAMAGE", Some(crate::skills::Class::Druid)),
    ("CHAINED JUDGMENT", "THE WHIP'S CRACK STRIKES A SECOND FOE", Some(crate::skills::Class::Inquisitor)),
];

pub const P_EMBERSTORM: usize = 0;
pub const P_BLOODTHIRST: usize = 1;
pub const P_ASHWALKER: usize = 2;
pub const P_GIANTSLAYER: usize = 3;
pub const P_LONG_NIGHT: usize = 4;
pub const P_SECOND_WIND: usize = 5;
pub const P_STORMCALLER: usize = 6;
pub const P_PHOENIX_ASH: usize = 7;
pub const P_FIRESTORM: usize = 8;
pub const P_BLOODLORD: usize = 9;
pub const P_BOILER: usize = 10;
pub const P_VALHALLA: usize = 11;
pub const P_UNDYING: usize = 12;
pub const P_HARVEST: usize = 13;
pub const P_COLOSSI: usize = 14;
pub const P_CHAINED: usize = 15;

/// An ancient item for this hero: one of the uniques, its stats 25-50% stronger, plus an ancient power (one
/// anyone can use, or the hero's own).
pub fn roll_ancient(class: crate::skills::Class, rng: &mut Rng) -> Option<Item> {
    let i = rng.range(0, UNIQUES.len() as i32) as usize;
    let mut it = unique(i);
    let k = rng.rf(1.25, 1.5);
    for (s, v) in it.stats.iter_mut() {
        if *s != Stat::Skills {
            *v = ((*v as f32) * k).round() as i32;
        }
    }
    let ok: Vec<usize> = (0..POWERS.len()).filter(|&p| POWERS[p].2.map_or(true, |c| c == class)).collect();
    let p = ok[rng.range(0, ok.len() as i32) as usize];
    it.stats.push((Stat::Power, 1 << p));
    it.rarity = Rarity::Ancient;
    it.name = format!("ANCIENT {}", it.name);
    Some(it)
}

/// Totals of every stat on your worn gear.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Bonus([i32; STATS.len()]);

impl Bonus {
    pub fn get(&self, s: Stat) -> i32 {
        self.0[s as usize]
    }
    /// `get` as a 0..1 fraction (percent stats), capped.
    pub fn frac(&self, s: Stat, cap: i32) -> f32 {
        self.get(s).min(cap) as f32 / 100.0
    }
}

// ------------------------------------------------------------------ base types

pub struct Base {
    pub key: &'static str,
    pub name: &'static str,
    pub slot: Slot,
    /// Art name of the inventory icon (`icon_*`, code-drawn fallback otherwise).
    pub icon: &'static str,
    /// Lowest item level it drops at.
    pub lvl: u8,
    /// Built-in stat (staffs: fire damage, body gear: armor), rolled lo..=hi.
    pub implicit: Option<(Stat, i32, i32)>,
    /// Only made as a unique.
    pub unique_only: bool,
}

const fn b(key: &'static str, name: &'static str, slot: Slot, icon: &'static str, lvl: u8, implicit: Option<(Stat, i32, i32)>) -> Base {
    Base { key, name, slot, icon, lvl, implicit, unique_only: false }
}

pub static BASES: &[Base] = &[
    b("gnarled", "GNARLED STAFF", Slot::Weapon, "icon_staff_gnarled", 1, Some((Stat::Fire, 5, 10))),
    b("ashwood", "ASHWOOD STAFF", Slot::Weapon, "icon_staff_ash", 4, Some((Stat::Fire, 10, 18))),
    b("runed", "RUNED STAFF", Slot::Weapon, "icon_staff_runed", 9, Some((Stat::Fire, 18, 28))),
    b("ember", "EMBER STAFF", Slot::Weapon, "icon_staff_ember", 14, Some((Stat::Fire, 28, 40))),
    b("hood", "HOOD", Slot::Helm, "icon_helm_hood", 1, Some((Stat::Armor, 3, 5))),
    b("circlet", "CIRCLET", Slot::Helm, "icon_helm_circlet", 6, Some((Stat::Armor, 4, 7))),
    b("horned", "HORNED HELM", Slot::Helm, "icon_helm_horned", 10, Some((Stat::Armor, 9, 14))),
    b("robe", "ROBE", Slot::Armor, "icon_armor_robe", 1, Some((Stat::Armor, 4, 8))),
    b("leather", "LEATHER ARMOR", Slot::Armor, "icon_armor_leather", 4, Some((Stat::Armor, 10, 16))),
    b("chain", "CHAIN MAIL", Slot::Armor, "icon_armor_chain", 10, Some((Stat::Armor, 20, 30))),
    b("cgloves", "CLOTH GLOVES", Slot::Gloves, "icon_gloves_cloth", 1, Some((Stat::Armor, 1, 3))),
    b("gauntlets", "GAUNTLETS", Slot::Gloves, "icon_gloves_leather", 8, Some((Stat::Armor, 4, 7))),
    b("cboots", "SOFT BOOTS", Slot::Boots, "icon_boots_cloth", 1, Some((Stat::Armor, 1, 3))),
    b("iboots", "IRON BOOTS", Slot::Boots, "icon_boots_heavy", 8, Some((Stat::Armor, 4, 7))),
    b("sash", "SASH", Slot::Belt, "icon_belt_sash", 1, Some((Stat::Armor, 1, 2))),
    b("hbelt", "HEAVY BELT", Slot::Belt, "icon_belt_leather", 8, Some((Stat::Armor, 3, 5))),
    b("ring", "RING", Slot::Ring, "icon_ring", 1, None),
    b("amulet", "AMULET", Slot::Amulet, "icon_amulet", 1, None),
    Base { key: "crown", name: "CROWN", slot: Slot::Helm, icon: "icon_helm_crown", lvl: 99, implicit: Some((Stat::Armor, 16, 16)), unique_only: true },
    // Gems (never rolled as gear; see `gem_item`).
    Base { key: "gem_ruby", name: "RUBY", slot: Slot::Gem, icon: "icon_gem_ruby", lvl: 99, implicit: None, unique_only: true },
    Base { key: "gem_sapphire", name: "SAPPHIRE", slot: Slot::Gem, icon: "icon_gem_sapphire", lvl: 99, implicit: None, unique_only: true },
    Base { key: "gem_topaz", name: "TOPAZ", slot: Slot::Gem, icon: "icon_gem_topaz", lvl: 99, implicit: None, unique_only: true },
    Base { key: "gem_emerald", name: "EMERALD", slot: Slot::Gem, icon: "icon_gem_emerald", lvl: 99, implicit: None, unique_only: true },
    Base { key: "gem_amethyst", name: "AMETHYST", slot: Slot::Gem, icon: "icon_gem_amethyst", lvl: 99, implicit: None, unique_only: true },
    Base { key: "gem_diamond", name: "DIAMOND", slot: Slot::Gem, icon: "icon_gem_diamond", lvl: 99, implicit: None, unique_only: true },
    Base { key: "gem_skull", name: "SKULL", slot: Slot::Gem, icon: "icon_gem_skull", lvl: 99, implicit: None, unique_only: true },
];

pub fn base_by_key(k: &str) -> Option<usize> {
    BASES.iter().position(|b| b.key == k)
}

// ------------------------------------------------------------------ affixes

struct Affix {
    stat: Stat,
    prefix: bool,
    /// Names by roll strength (weak .. strong).
    names: &'static [&'static str],
    /// Value range at item level 1; scales up with item level.
    lo: i32,
    hi: i32,
    /// Max value whatever the item level.
    cap: i32,
    /// Lowest item level.
    lvl: u8,
    slots: &'static [Slot],
}

use Slot::*;
const ANY: &[Slot] = &[Weapon, Helm, Armor, Gloves, Boots, Belt, Ring, Amulet];

static AFFIXES: &[Affix] = &[
    Affix { stat: Stat::Fire, prefix: true, names: &["SMOLDERING", "BURNING", "BLAZING"], lo: 4, hi: 8, cap: 40, lvl: 1, slots: &[Weapon, Helm, Ring, Amulet, Gloves] },
    Affix { stat: Stat::Mana, prefix: true, names: &["LAPIS", "COBALT", "SAPPHIRE"], lo: 5, hi: 10, cap: 60, lvl: 1, slots: ANY },
    Affix { stat: Stat::Armor, prefix: true, names: &["STURDY", "STRONG", "GLORIOUS"], lo: 2, hi: 5, cap: 30, lvl: 1, slots: &[Helm, Armor, Gloves, Boots, Belt] },
    Affix { stat: Stat::Skills, prefix: true, names: &["ARCANE", "ARCANE", "ARCANE"], lo: 1, hi: 1, cap: 1, lvl: 8, slots: &[Weapon, Helm, Amulet] },
    Affix { stat: Stat::Gold, prefix: true, names: &["GILDED", "GILDED", "GOLDEN"], lo: 10, hi: 25, cap: 80, lvl: 1, slots: &[Gloves, Boots, Belt, Ring, Amulet] },
    Affix { stat: Stat::Magic, prefix: true, names: &["LUCKY", "FORTUNATE", "FORTUNATE"], lo: 5, hi: 12, cap: 35, lvl: 3, slots: &[Helm, Gloves, Boots, Ring, Amulet] },
    Affix { stat: Stat::Life, prefix: false, names: &["OF THE FOX", "OF THE WOLF", "OF THE BEAR"], lo: 5, hi: 10, cap: 60, lvl: 1, slots: ANY },
    Affix { stat: Stat::LifeRegen, prefix: false, names: &["OF REGROWTH", "OF REGENERATION", "OF REGENERATION"], lo: 1, hi: 2, cap: 6, lvl: 2, slots: &[Helm, Armor, Belt, Ring, Amulet] },
    Affix { stat: Stat::ManaRegen, prefix: false, names: &["OF FOCUS", "OF CLARITY", "OF THE MIND"], lo: 10, hi: 20, cap: 60, lvl: 1, slots: &[Weapon, Helm, Ring, Amulet] },
    Affix { stat: Stat::Cast, prefix: false, names: &["OF THE APPRENTICE", "OF THE MAGUS", "OF THE MAGUS"], lo: 10, hi: 10, cap: 20, lvl: 3, slots: &[Weapon, Ring, Amulet, Gloves] },
    Affix { stat: Stat::Move, prefix: false, names: &["OF PACING", "OF HASTE", "OF SPEED"], lo: 8, hi: 12, cap: 30, lvl: 2, slots: &[Boots] },
    Affix { stat: Stat::Stamina, prefix: false, names: &["OF ENDURANCE", "OF ENDURANCE", "OF THE OX"], lo: 20, hi: 35, cap: 60, lvl: 1, slots: &[Armor, Boots, Belt] },
    Affix { stat: Stat::Hunger, prefix: false, names: &["OF PLENTY", "OF PLENTY", "OF THE FEAST"], lo: 15, hi: 30, cap: 50, lvl: 1, slots: &[Belt, Amulet, Armor] },
    Affix { stat: Stat::LifeOnKill, prefix: false, names: &["OF THE LEECH", "OF THE LEECH", "OF THE VAMPIRE"], lo: 2, hi: 4, cap: 12, lvl: 4, slots: &[Weapon, Gloves, Ring] },
    Affix { stat: Stat::ManaOnKill, prefix: false, names: &["OF WITS", "OF WITS", "OF THE SAGE"], lo: 2, hi: 3, cap: 10, lvl: 4, slots: &[Weapon, Helm, Ring, Amulet] },
];

const RARE_A: &[&str] = &["ASH", "DOOM", "GRIM", "EMBER", "CINDER", "GLOOM", "PYRE", "SOOT", "STORM", "WRAITH", "BLOOD", "DUSK", "HAVOC", "RUNE"];

fn rare_noun(s: Slot) -> &'static [&'static str] {
    match s {
        Weapon => &["SPIRE", "BRAND", "BRANCH", "ROD"],
        Helm => &["VISAGE", "CROWN", "HOOD", "MASK"],
        Armor => &["SHROUD", "HIDE", "MANTLE", "WRAP"],
        Gloves => &["GRASP", "CLAW", "FIST", "HOLD"],
        Boots => &["STRIDE", "TREAD", "SPUR", "WALK"],
        Belt => &["COIL", "CORD", "LOCK", "CLASP"],
        Ring => &["BAND", "LOOP", "COIL", "EYE"],
        Amulet => &["HEART", "CHARM", "EYE", "TOKEN"],
        Gem => &["STONE"],
    }
}

// ------------------------------------------------------------------ items

#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub base: usize,
    pub rarity: Rarity,
    pub ilvl: u8,
    pub name: String,
    pub stats: Vec<(Stat, i32)>,
    /// Character level needed to wear it.
    pub req: u32,
    /// Open and filled sockets (D2 style: gems go in, and give a bonus by the kind of item).
    pub sockets: u8,
    pub gems: Vec<Gem>,
}

// ------------------------------------------------------------------ gems

/// A gem: one of seven kinds, graded chipped (1) to perfect (5).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Gem {
    pub kind: u8,
    pub grade: u8,
}

pub const GEM_KINDS: [&str; 7] = ["RUBY", "SAPPHIRE", "TOPAZ", "EMERALD", "AMETHYST", "DIAMOND", "SKULL"];
const GEM_KEYS: [&str; 7] = ["gem_ruby", "gem_sapphire", "gem_topaz", "gem_emerald", "gem_amethyst", "gem_diamond", "gem_skull"];
pub const GEM_GRADES: [&str; 5] = ["CHIPPED", "FLAWED", "", "FLAWLESS", "PERFECT"];
pub const TOP_GRADE: u8 = 5;

pub fn gem_col(kind: u8) -> u32 {
    [0xe02030, 0x3060f0, 0xf0c020, 0x20c050, 0xa040e0, 0xf0f0ff, 0xd8d0b8][kind as usize % 7]
}

pub fn gem_name(g: Gem) -> String {
    let k = GEM_KINDS[g.kind as usize % 7];
    match GEM_GRADES[(g.grade.clamp(1, TOP_GRADE) - 1) as usize] {
        "" => k.to_string(),
        q => format!("{q} {k}"),
    }
}

/// Where a gem sits decides what it gives: 0 weapon, 1 armor (helm, body, gloves, boots, belt), 2 jewelry.
fn gem_group(s: Slot) -> usize {
    match s {
        Weapon => 0,
        Ring | Amulet => 2,
        _ => 1,
    }
}

/// (stat, value per grade) for each kind in a weapon / armor / jewelry.
type GemLine = &'static [(Stat, [i32; 5])];
static GEM_TABLE: [[GemLine; 3]; 7] = [
    // Ruby: damage, life, life after kills.
    [&[(Stat::Fire, [4, 6, 9, 12, 16])], &[(Stat::Life, [6, 10, 15, 22, 30])], &[(Stat::LifeOnKill, [1, 1, 2, 3, 4])]],
    // Sapphire: mana.
    [&[(Stat::ManaOnKill, [1, 1, 2, 3, 4])], &[(Stat::Mana, [6, 10, 15, 22, 30])], &[(Stat::ManaRegen, [5, 8, 12, 16, 20])]],
    // Topaz: treasure.
    [&[(Stat::Gold, [8, 12, 18, 25, 35])], &[(Stat::Magic, [5, 8, 12, 18, 24])], &[(Stat::Gold, [10, 15, 22, 30, 40])]],
    // Emerald: speed.
    [&[(Stat::Cast, [3, 4, 6, 8, 10])], &[(Stat::Stamina, [8, 12, 18, 25, 35])], &[(Stat::Move, [2, 3, 5, 7, 9])]],
    // Amethyst: toughness.
    [&[(Stat::Life, [5, 8, 12, 16, 20])], &[(Stat::Armor, [3, 5, 8, 12, 16])], &[(Stat::Hunger, [8, 12, 18, 25, 30])]],
    // Diamond: a little of everything.
    [&[(Stat::ManaRegen, [5, 8, 12, 16, 20])], &[(Stat::Armor, [2, 3, 5, 7, 10]), (Stat::Life, [2, 3, 5, 7, 10])], &[(Stat::Magic, [3, 5, 8, 11, 15])]],
    // Skull: life and mana together.
    [
        &[(Stat::LifeOnKill, [1, 1, 1, 2, 2]), (Stat::ManaOnKill, [1, 1, 1, 2, 2])],
        &[(Stat::LifeRegen, [1, 1, 1, 2, 2]), (Stat::ManaRegen, [3, 5, 7, 10, 12])],
        &[(Stat::Life, [3, 5, 8, 11, 15]), (Stat::Mana, [3, 5, 8, 11, 15])],
    ],
];

/// What a gem gives set in an item of this slot.
pub fn gem_stats(g: Gem, slot: Slot) -> Vec<(Stat, i32)> {
    let gi = (g.grade.clamp(1, TOP_GRADE) - 1) as usize;
    GEM_TABLE[g.kind as usize % 7][gem_group(slot)].iter().map(|(s, v)| (*s, v[gi])).collect()
}

/// A loose gem, as a bag item.
pub fn gem_item(g: Gem) -> Item {
    let base = base_by_key(GEM_KEYS[g.kind as usize % 7]).expect("gem base");
    Item { base, rarity: Rarity::Normal, ilvl: g.grade, name: gem_name(g), stats: vec![], req: 1, sockets: 0, gems: vec![] }
}

/// A gem dropped by a monster of this item level: better gems deeper in.
pub fn roll_gem(ilvl: u8, rng: &mut Rng) -> Gem {
    let mut grade = 1 + ilvl / 9;
    if rng.chance(0.25) {
        grade += 1;
    }
    Gem { kind: rng.range(0, 7) as u8, grade: grade.clamp(1, TOP_GRADE - 1) }
}

/// What a jeweler charges to make this gem from three of the grade below.
pub fn combine_cost(up: Gem) -> i32 {
    [0, 30, 80, 200, 500][(up.grade.clamp(1, TOP_GRADE) - 1) as usize]
}

/// What a jeweler charges to cut sockets in a plain item.
pub fn socket_cost(it: &Item) -> i32 {
    60 + it.ilvl as i32 * 15
}

/// What a jeweler charges to prise the gems out of an item, unharmed.
pub fn unsocket_cost(it: &Item) -> i32 {
    it.gems.iter().map(|g| g.grade as i32 * 25).sum()
}

/// Most sockets an item of this slot can have.
pub fn max_sockets(s: Slot) -> u8 {
    match s {
        Weapon | Armor => 3,
        Helm => 2,
        Gem => 0,
        _ => 1,
    }
}

// ------------------------------------------------------------------ sets

pub struct SetPiece {
    pub name: &'static str,
    pub base: &'static str,
    pub stats: &'static [(Stat, i32)],
}

pub struct SetDef {
    pub name: &'static str,
    /// The hero it was made for (None: a shared set). Anyone may wear it.
    pub hero: Option<&'static str>,
    pub req: u32,
    pub pieces: &'static [SetPiece],
    /// Bonuses for wearing at least this many pieces.
    pub bonus: &'static [(u8, &'static [(Stat, i32)])],
}

const fn sp(name: &'static str, base: &'static str, stats: &'static [(Stat, i32)]) -> SetPiece {
    SetPiece { name, base, stats }
}

pub static SETS: &[SetDef] = &[
    SetDef {
        name: "EMBERWEAVE",
        hero: Some("SORCERESS"),
        req: 12,
        pieces: &[
            sp("EMBERWEAVE SPIRE", "runed", &[(Stat::Fire, 20), (Stat::Mana, 20)]),
            sp("EMBERWEAVE CIRCLET", "circlet", &[(Stat::Armor, 6), (Stat::ManaRegen, 25)]),
            sp("EMBERWEAVE ROBE", "robe", &[(Stat::Armor, 12), (Stat::Life, 20)]),
            sp("EMBERWEAVE SASH", "sash", &[(Stat::Armor, 3), (Stat::Cast, 10)]),
        ],
        bonus: &[(2, &[(Stat::Fire, 15)]), (3, &[(Stat::Cast, 15), (Stat::Mana, 30)]), (4, &[(Stat::Skills, 1), (Stat::Fire, 25)])],
    },
    SetDef {
        name: "NIGHTBLOOD",
        hero: Some("VAMPIRE"),
        req: 14,
        pieces: &[
            sp("NIGHTBLOOD COWL", "hood", &[(Stat::Armor, 6), (Stat::LifeOnKill, 3)]),
            sp("NIGHTBLOOD SHROUD", "leather", &[(Stat::Armor, 16), (Stat::Life, 25)]),
            sp("NIGHTBLOOD TALONS", "gauntlets", &[(Stat::Armor, 6), (Stat::Fire, 12)]),
            sp("NIGHTBLOOD SIGNET", "ring", &[(Stat::LifeOnKill, 3), (Stat::ManaOnKill, 2)]),
        ],
        bonus: &[(2, &[(Stat::LifeOnKill, 4)]), (3, &[(Stat::Life, 40)]), (4, &[(Stat::Skills, 1), (Stat::LifeRegen, 4)])],
    },
    SetDef {
        name: "BRASSWORK",
        hero: Some("SKY PIRATE"),
        req: 14,
        pieces: &[
            sp("BRASSWORK GOGGLES", "horned", &[(Stat::Armor, 12), (Stat::Magic, 15)]),
            sp("BRASSWORK GRIPS", "gauntlets", &[(Stat::Armor, 6), (Stat::Cast, 10)]),
            sp("BRASSWORK TREADS", "iboots", &[(Stat::Armor, 6), (Stat::Move, 12)]),
            sp("BRASSWORK MAINSPRING", "amulet", &[(Stat::Mana, 30), (Stat::ManaRegen, 20)]),
        ],
        bonus: &[(2, &[(Stat::Cast, 10)]), (3, &[(Stat::Fire, 25), (Stat::Gold, 30)]), (4, &[(Stat::Skills, 1), (Stat::Mana, 40)])],
    },
    SetDef {
        name: "OATHSWORN",
        hero: Some("VALKYRIE"),
        req: 16,
        pieces: &[
            sp("OATHSWORN WINGHELM", "horned", &[(Stat::Armor, 14), (Stat::Life, 20)]),
            sp("OATHSWORN MAIL", "chain", &[(Stat::Armor, 28), (Stat::LifeRegen, 3)]),
            sp("OATHSWORN GREAVES", "iboots", &[(Stat::Armor, 8), (Stat::Stamina, 40)]),
        ],
        bonus: &[(2, &[(Stat::Armor, 25), (Stat::Life, 30)]), (3, &[(Stat::Skills, 1), (Stat::Fire, 30)])],
    },
    SetDef {
        name: "BLOODHOWL",
        hero: Some("BERSERKER"),
        req: 16,
        pieces: &[
            sp("BLOODHOWL MASK", "horned", &[(Stat::Armor, 12), (Stat::LifeOnKill, 4)]),
            sp("BLOODHOWL HIDE", "leather", &[(Stat::Armor, 18), (Stat::Life, 35)]),
            sp("BLOODHOWL FISTS", "gauntlets", &[(Stat::Armor, 6), (Stat::Fire, 18)]),
        ],
        bonus: &[(2, &[(Stat::LifeOnKill, 6), (Stat::Move, 10)]), (3, &[(Stat::Skills, 1), (Stat::Life, 50)])],
    },
    SetDef {
        name: "GRAVESONG",
        hero: Some("REAPER"),
        req: 20,
        pieces: &[
            sp("GRAVESONG VEIL", "circlet", &[(Stat::Armor, 8), (Stat::ManaOnKill, 3)]),
            sp("GRAVESONG GOWN", "robe", &[(Stat::Armor, 14), (Stat::Mana, 35)]),
            sp("GRAVESONG BAND", "ring", &[(Stat::LifeOnKill, 3), (Stat::Fire, 12)]),
            sp("GRAVESONG LOCKET", "amulet", &[(Stat::Life, 30), (Stat::ManaRegen, 25)]),
        ],
        bonus: &[(2, &[(Stat::ManaOnKill, 4)]), (3, &[(Stat::Fire, 25), (Stat::Cast, 10)]), (4, &[(Stat::Skills, 1), (Stat::Life, 50)])],
    },
    SetDef {
        name: "THORNMOTHER",
        hero: Some("DRUID"),
        req: 20,
        pieces: &[
            sp("THORNMOTHER CROWN", "circlet", &[(Stat::Armor, 8), (Stat::LifeRegen, 3)]),
            sp("THORNMOTHER BARK", "leather", &[(Stat::Armor, 18), (Stat::Life, 30)]),
            sp("THORNMOTHER ROOTS", "cboots", &[(Stat::Armor, 4), (Stat::Move, 12)]),
            sp("THORNMOTHER VINE", "sash", &[(Stat::Armor, 3), (Stat::Hunger, 35)]),
        ],
        bonus: &[(2, &[(Stat::LifeRegen, 3)]), (3, &[(Stat::Mana, 40), (Stat::Fire, 20)]), (4, &[(Stat::Skills, 1), (Stat::Life, 40)])],
    },
    // ---- shared sets ----
    SetDef {
        name: "WANDERER'S",
        hero: None,
        req: 4,
        pieces: &[
            sp("WANDERER'S SANDALS", "cboots", &[(Stat::Armor, 3), (Stat::Move, 10)]),
            sp("WANDERER'S CORD", "sash", &[(Stat::Armor, 2), (Stat::Hunger, 25)]),
            sp("WANDERER'S MITTS", "cgloves", &[(Stat::Armor, 3), (Stat::Gold, 20)]),
        ],
        bonus: &[(2, &[(Stat::Stamina, 40)]), (3, &[(Stat::Life, 20), (Stat::Magic, 20)])],
    },
    SetDef {
        name: "GILDED HAND",
        hero: None,
        req: 12,
        pieces: &[
            sp("GILDED HAND RING", "ring", &[(Stat::Gold, 30), (Stat::Mana, 15)]),
            sp("GILDED HAND GLOVES", "gauntlets", &[(Stat::Armor, 6), (Stat::Magic, 15)]),
            sp("GILDED HAND CHAIN", "amulet", &[(Stat::Gold, 30), (Stat::Life, 20)]),
        ],
        bonus: &[(2, &[(Stat::Magic, 25)]), (3, &[(Stat::Gold, 60), (Stat::Magic, 25)])],
    },
    SetDef {
        name: "ASHEN REGALIA",
        hero: None,
        req: 30,
        pieces: &[
            sp("ASHEN REGALIA SCEPTER", "ember", &[(Stat::Fire, 40), (Stat::Cast, 15)]),
            sp("ASHEN REGALIA HELM", "horned", &[(Stat::Armor, 16), (Stat::Life, 40)]),
            sp("ASHEN REGALIA MAIL", "chain", &[(Stat::Armor, 34), (Stat::Mana, 40)]),
            sp("ASHEN REGALIA TORC", "amulet", &[(Stat::Skills, 1), (Stat::LifeOnKill, 6)]),
        ],
        bonus: &[(2, &[(Stat::Life, 50)]), (3, &[(Stat::Fire, 40), (Stat::LifeRegen, 5)]), (4, &[(Stat::Skills, 2), (Stat::Magic, 40)])],
    },
];

pub fn set_item(set: usize, piece: usize) -> Item {
    let d = &SETS[set];
    let p = &d.pieces[piece];
    let base = base_by_key(p.base).expect("set base");
    let ilvl = (d.req as u8).saturating_add(2);
    let mut stats = vec![];
    if let Some((s, lo, hi)) = BASES[base].implicit {
        stats.push((s, (lo + hi) / 2));
    }
    stats.extend_from_slice(p.stats);
    Item { base, rarity: Rarity::Set, ilvl, name: p.name.to_string(), stats, req: d.req, sockets: 0, gems: vec![] }
}

/// A piece of the set made for this hero that can drop at this item level.
pub fn hero_set_piece(hero: &str, ilvl: u8, rng: &mut Rng) -> Option<Item> {
    let si = SETS.iter().position(|d| d.hero == Some(hero) && d.req <= ilvl as u32 + 3)?;
    Some(set_item(si, rng.range(0, SETS[si].pieces.len() as i32) as usize))
}

/// The set an item belongs to (by name).
pub fn set_of(it: &Item) -> Option<usize> {
    if it.rarity != Rarity::Set {
        return None;
    }
    SETS.iter().position(|d| d.pieces.iter().any(|p| p.name == it.name))
}

impl Item {
    pub fn base(&self) -> &'static Base {
        &BASES[self.base]
    }
    pub fn slot(&self) -> Slot {
        self.base().slot
    }
    pub fn col(&self) -> u32 {
        rarity_col(self.rarity)
    }
    /// A stat's total, gems included.
    pub fn stat(&self, s: Stat) -> i32 {
        self.all_stats().iter().filter(|(t, _)| *t == s).map(|(_, v)| *v).sum()
    }
    /// Its own stats plus whatever its gems give.
    pub fn all_stats(&self) -> Vec<(Stat, i32)> {
        let mut v = self.stats.clone();
        for g in &self.gems {
            v.extend(gem_stats(*g, self.slot()));
        }
        v
    }
    /// The gem this item is, if it's a loose gem.
    pub fn gem(&self) -> Option<Gem> {
        let k = GEM_KEYS.iter().position(|k| *k == self.base().key)?;
        Some(Gem { kind: k as u8, grade: self.ilvl.clamp(1, TOP_GRADE) })
    }
    /// What Gerta charges for it.
    pub fn cost(&self) -> i32 {
        self.price() * 4
    }

    /// What Gerta pays for it.
    pub fn price(&self) -> i32 {
        if let Some(g) = self.gem() {
            return [10, 25, 60, 140, 320][(g.grade - 1) as usize];
        }
        let mult = match self.rarity {
            Rarity::Normal => 1,
            Rarity::Magic => 3,
            Rarity::Rare => 6,
            Rarity::Set => 10,
            Rarity::Unique => 12,
            Rarity::Ancient => 25,
        };
        (4 + self.ilvl as i32 * 2) * mult + self.gems.iter().map(|g| gem_item(*g).price()).sum::<i32>()
    }
    /// Tooltip lines (after the name).
    pub fn lines(&self) -> Vec<String> {
        let mut v = vec![];
        if let Some(g) = self.gem() {
            for (slot, what) in [(Weapon, "WEAPON"), (Armor, "ARMOR"), (Ring, "JEWELRY")] {
                let st: Vec<String> = gem_stats(g, slot).iter().map(|(s, n)| stat_text(*s, *n)).collect();
                v.push(format!("{what}: {}", st.join(", ")));
            }
            v.push("SET IT IN AN ITEM WITH A FREE SOCKET.".into());
            if g.grade < TOP_GRADE {
                v.push("A JEWELER JOINS THREE INTO ONE BETTER GEM.".into());
            }
            return v;
        }
        if self.rarity != Rarity::Normal || self.name != self.base().name {
            v.push(self.base().name.to_string());
        }
        for s in STATS {
            let n = self.stat(s);
            if n != 0 {
                v.push(stat_text(s, n));
            }
        }
        if self.sockets > 0 {
            v.push(format!("SOCKETED ({}/{})", self.gems.len(), self.sockets));
            for g in &self.gems {
                v.push(format!("  {}", gem_name(*g)));
            }
        }
        if let Some(si) = set_of(self) {
            let d = &SETS[si];
            v.push(format!("{} SET ({} PIECES)", d.name, d.pieces.len()));
            if let Some(h) = d.hero {
                v.push(format!("  MADE FOR THE {h}"));
            }
            for (n, b) in d.bonus {
                let st: Vec<String> = b.iter().map(|(s, x)| stat_text(*s, *x)).collect();
                v.push(format!("({n}) {}", st.join(", ")));
            }
        }
        if self.req > 1 {
            v.push(format!("REQUIRED LEVEL: {}", self.req));
        }
        v
    }
}

/// Item level for a monster in an area of this tier.
pub fn ilvl_for(tier: f32) -> u8 {
    (tier * 4.5).round().clamp(1.0, 50.0) as u8
}

fn roll_affix(a: &Affix, ilvl: u8, rng: &mut Rng) -> (i32, &'static str) {
    let scale = 1.0 + ilvl as f32 * 0.09;
    let lo = (a.lo as f32 * scale).round() as i32;
    let hi = ((a.hi as f32 * scale).round() as i32).max(lo);
    let v = rng.range(lo, hi + 1).min(a.cap).max(1);
    // Stronger rolls (relative to what this level can roll) get the stronger name.
    let top = (a.hi as f32 * (1.0 + 30.0 * 0.09)).min(a.cap as f32);
    let k = ((v as f32 / top) * a.names.len() as f32) as usize;
    (v, a.names[k.min(a.names.len() - 1)])
}

/// A random base that can drop at this item level (newest bases are likelier).
fn pick_base(ilvl: u8, rng: &mut Rng) -> usize {
    let ok: Vec<usize> = (0..BASES.len()).filter(|&i| !BASES[i].unique_only && BASES[i].lvl <= ilvl).collect();
    // Pick a slot first so rings and amulets are as common as staffs.
    let slots = [Weapon, Helm, Armor, Gloves, Boots, Belt, Ring, Amulet];
    let s = slots[rng.range(0, slots.len() as i32) as usize];
    let mut of: Vec<usize> = ok.iter().copied().filter(|&i| BASES[i].slot == s).collect();
    of.sort_by_key(|&i| BASES[i].lvl);
    // Two in three: the best base for this level; otherwise any.
    if rng.chance(0.66) {
        *of.last().unwrap()
    } else {
        of[rng.range(0, of.len() as i32) as usize]
    }
}

pub fn roll(ilvl: u8, rarity: Rarity, rng: &mut Rng) -> Item {
    let base = pick_base(ilvl, rng);
    roll_base(base, ilvl, rarity, rng)
}

pub fn roll_base(base: usize, ilvl: u8, rarity: Rarity, rng: &mut Rng) -> Item {
    let bd = &BASES[base];
    let mut stats = vec![];
    if let Some((s, lo, hi)) = bd.implicit {
        stats.push((s, rng.range(lo, hi + 1)));
    }
    let (np, ns) = match rarity {
        Rarity::Normal | Rarity::Unique | Rarity::Set | Rarity::Ancient => (0, 0),
        Rarity::Magic => {
            let r = rng.f();
            if r < 0.4 {
                (1, 1)
            } else if r < 0.7 {
                (1, 0)
            } else {
                (0, 1)
            }
        }
        Rarity::Rare => {
            let n = rng.range(3, 6);
            let p = rng.range(1, 4).min(n - 1);
            (p, n - p)
        }
    };
    let mut used = vec![];
    let mut pre_name = None;
    let mut suf_name = None;
    for (want, prefix) in [(np, true), (ns, false)] {
        for _ in 0..want {
            let pool: Vec<&Affix> = AFFIXES.iter().filter(|a| a.prefix == prefix && a.lvl <= ilvl && a.slots.contains(&bd.slot) && !used.contains(&a.stat)).collect();
            if pool.is_empty() {
                break;
            }
            let a = pool[rng.range(0, pool.len() as i32) as usize];
            used.push(a.stat);
            let (v, nm) = roll_affix(a, ilvl, rng);
            stats.push((a.stat, v));
            if prefix {
                pre_name.get_or_insert(nm);
            } else {
                suf_name.get_or_insert(nm);
            }
        }
    }
    let name = match rarity {
        Rarity::Normal | Rarity::Unique | Rarity::Set | Rarity::Ancient => bd.name.to_string(),
        Rarity::Magic => {
            let mut n = String::new();
            if let Some(p) = pre_name {
                n.push_str(p);
                n.push(' ');
            }
            n.push_str(bd.name);
            if let Some(s) = suf_name {
                n.push(' ');
                n.push_str(s);
            }
            n
        }
        Rarity::Rare => {
            let nouns = rare_noun(bd.slot);
            format!("{} {}", RARE_A[rng.range(0, RARE_A.len() as i32) as usize], nouns[rng.range(0, nouns.len() as i32) as usize])
        }
    };
    let req = match rarity {
        Rarity::Normal => bd.lvl as u32,
        _ => (ilvl as u32).saturating_sub(2).max(bd.lvl as u32).max(1),
    };
    // White and blue gear sometimes drops with sockets.
    let max = max_sockets(bd.slot);
    let sockets = match rarity {
        Rarity::Normal if max > 0 && rng.chance(0.2) => rng.range(1, max as i32 + 1) as u8,
        Rarity::Magic if max > 0 && rng.chance(0.08) => rng.range(1, max.min(2) as i32 + 1) as u8,
        _ => 0,
    };
    Item { base, rarity, ilvl, name, stats, req, sockets, gems: vec![] }
}

/// Rarity of a monster drop. `mf` is your magic-find %; `boost` is 1 for bosses.
pub fn roll_rarity(mf: i32, boost: bool, rng: &mut Rng) -> Rarity {
    let mf = mf as f32;
    let k = if boost { 4.0 } else { 1.0 };
    let r = rng.f();
    if r < 0.006 * k * (1.0 + mf / 250.0) {
        Rarity::Unique
    } else if r < 0.016 * k * (1.0 + mf / 200.0) {
        Rarity::Set
    } else if r < 0.07 * k * (1.0 + mf / 150.0) {
        Rarity::Rare
    } else if r < 0.36 * k * (1.0 + mf / 100.0) {
        Rarity::Magic
    } else {
        Rarity::Normal
    }
}

// ------------------------------------------------------------------ uniques

pub struct UniqueDef {
    pub name: &'static str,
    pub base: &'static str,
    pub req: u32,
    pub stats: &'static [(Stat, i32)],
    /// Only dropped by this boss (None: anywhere, rarely).
    pub boss: Option<&'static str>,
}

pub static UNIQUES: &[UniqueDef] = &[
    UniqueDef { name: "MARROWGRIP", base: "gauntlets", req: 5, stats: &[(Stat::Armor, 8), (Stat::Life, 20), (Stat::LifeOnKill, 5), (Stat::Cast, 15)], boss: Some("bone") },
    UniqueDef { name: "ROTMOTHER COWL", base: "hood", req: 7, stats: &[(Stat::Armor, 10), (Stat::Life, 25), (Stat::LifeRegen, 3), (Stat::Hunger, 40)], boss: Some("plague") },
    UniqueDef { name: "HEXBINDER", base: "ring", req: 10, stats: &[(Stat::Skills, 1), (Stat::Mana, 30), (Stat::ManaRegen, 40), (Stat::ManaOnKill, 3)], boss: Some("hex") },
    UniqueDef { name: "CROWN OF ASH", base: "crown", req: 14, stats: &[(Stat::Skills, 2), (Stat::Fire, 30), (Stat::Life, 40), (Stat::Mana, 40)], boss: Some("ashking") },
    // ---- side content (side.rs): the super uniques' and the Well-Witch's own ----
    UniqueDef { name: "SKRAT'S LUCKY EAR", base: "amulet", req: 4, stats: &[(Stat::Gold, 40), (Stat::Magic, 25), (Stat::Move, 10)], boss: Some("skrat") },
    UniqueDef { name: "BONEJAW'S GRIN", base: "ring", req: 5, stats: &[(Stat::LifeOnKill, 4), (Stat::Armor, 6), (Stat::Cast, 10)], boss: Some("bonejaw") },
    UniqueDef { name: "THE SHEPHERD'S GIRDLE", base: "hbelt", req: 5, stats: &[(Stat::Life, 30), (Stat::LifeRegen, 3), (Stat::Hunger, 30)], boss: Some("shepherd") },
    UniqueDef { name: "BARNACLE-PLATE", base: "chain", req: 40, stats: &[(Stat::Armor, 55), (Stat::Life, 80), (Stat::LifeRegen, 8), (Stat::Hunger, 30)], boss: Some("barnacle") },
    UniqueDef { name: "THE BOSUN'S WHISTLE", base: "amulet", req: 38, stats: &[(Stat::Life, 50), (Stat::LifeOnKill, 8), (Stat::Move, 12), (Stat::Stamina, 60)], boss: Some("krake") },
    UniqueDef { name: "LIRAEL'S CORAL", base: "ring", req: 40, stats: &[(Stat::Skills, 1), (Stat::Mana, 50), (Stat::ManaOnKill, 6)], boss: Some("lirael") },
    UniqueDef { name: "THE LURE", base: "amulet", req: 41, stats: &[(Stat::Fire, 45), (Stat::Magic, 40), (Stat::Life, 40)], boss: Some("maw") },
    UniqueDef { name: "YSOLDE'S COMPASS", base: "ring", req: 39, stats: &[(Stat::Move, 15), (Stat::Life, 35), (Stat::Gold, 50)], boss: Some("sovereign") },
    UniqueDef { name: "THE CHOIR'S PEARL", base: "amulet", req: 42, stats: &[(Stat::Skills, 1), (Stat::Mana, 70), (Stat::ManaRegen, 50), (Stat::Cast, 20)], boss: Some("choir") },
    UniqueDef { name: "KRAKEN-SUCKER GRIPS", base: "gauntlets", req: 42, stats: &[(Stat::Armor, 30), (Stat::LifeOnKill, 10), (Stat::Life, 50), (Stat::Cast, 15)], boss: Some("kraken") },
    UniqueDef { name: "THE GOLEM'S HEART", base: "chain", req: 32, stats: &[(Stat::Armor, 45), (Stat::Life, 70), (Stat::Fire, 30), (Stat::LifeRegen, 6)], boss: Some("junkgolem") },
    UniqueDef { name: "MAINSPRING'S VALVE", base: "hbelt", req: 32, stats: &[(Stat::Armor, 24), (Stat::Life, 60), (Stat::Fire, 25), (Stat::Stamina, 60)], boss: Some("mainspring") },
    UniqueDef { name: "HALVANE'S VERDICT", base: "crown", req: 34, stats: &[(Stat::Skills, 1), (Stat::Armor, 28), (Stat::Life, 50), (Stat::Cast, 20)], boss: Some("halvane") },
    UniqueDef { name: "TICK-TOCK'S HEELS", base: "cboots", req: 32, stats: &[(Stat::Move, 30), (Stat::Armor, 16), (Stat::Stamina, 80), (Stat::Cast, 10)], boss: Some("ticktock") },
    UniqueDef { name: "THE HUNTER'S CHARM", base: "amulet", req: 26, stats: &[(Stat::Fire, 30), (Stat::Cast, 15), (Stat::Life, 40), (Stat::LifeOnKill, 5)], boss: Some("abelard") },
    UniqueDef { name: "PALE CLAW", base: "gauntlets", req: 26, stats: &[(Stat::Armor, 18), (Stat::LifeOnKill, 8), (Stat::Move, 10), (Stat::Cast, 10)], boss: Some("pale") },
    UniqueDef { name: "THE MOURNING VEIL", base: "hood", req: 26, stats: &[(Stat::Armor, 20), (Stat::Mana, 50), (Stat::ManaOnKill, 5), (Stat::ManaRegen, 30)], boss: Some("mournwail") },
    UniqueDef { name: "HANGMAN'S KNOT", base: "hbelt", req: 27, stats: &[(Stat::Armor, 20), (Stat::Life, 55), (Stat::Hunger, 40), (Stat::LifeRegen, 5)], boss: Some("blackmoor") },
    UniqueDef { name: "BRIDE'S BLOODSTONE", base: "ring", req: 27, stats: &[(Stat::LifeOnKill, 6), (Stat::Life, 30), (Stat::Cast, 10)], boss: Some("bride") },
    UniqueDef { name: "VARDAK'S WEDDING BAND", base: "ring", req: 28, stats: &[(Stat::Skills, 1), (Stat::Life, 40), (Stat::LifeOnKill, 8), (Stat::Magic, 30)], boss: Some("vardakring") },
    UniqueDef { name: "THE GRAVEDIGGER'S SPADE-HANDLE", base: "gnarled", req: 27, stats: &[(Stat::Fire, 45), (Stat::Life, 40), (Stat::Skills, 1)], boss: Some("gravedigger") },
    UniqueDef { name: "ELSPETH'S LOCKET", base: "amulet", req: 28, stats: &[(Stat::Skills, 1), (Stat::Mana, 60), (Stat::Life, 40), (Stat::ManaRegen, 40)], boss: Some("elspeth") },
    UniqueDef { name: "THE SHADE'S LANTERN", base: "amulet", req: 28, stats: &[(Stat::Fire, 35), (Stat::Magic, 40), (Stat::Gold, 40), (Stat::Life, 30)], boss: Some("shade") },
    UniqueDef { name: "HROGAR'S WAR HORN", base: "amulet", req: 20, stats: &[(Stat::Life, 40), (Stat::Armor, 15), (Stat::LifeOnKill, 6), (Stat::Move, 10)], boss: Some("jarl") },
    UniqueDef { name: "GRIMFANG'S PELT", base: "leather", req: 18, stats: &[(Stat::Armor, 22), (Stat::Move, 15), (Stat::Life, 30), (Stat::Stamina, 40)], boss: Some("grimfang") },
    UniqueDef { name: "THE BRIDE'S VEIL", base: "circlet", req: 21, stats: &[(Stat::Skills, 1), (Stat::Mana, 45), (Stat::ManaRegen, 40), (Stat::Cast, 15)], boss: Some("frozenbride") },
    UniqueDef { name: "ICE-BEARD'S HAUBERK", base: "chain", req: 22, stats: &[(Stat::Armor, 35), (Stat::Life, 50), (Stat::LifeRegen, 5), (Stat::Hunger, 30)], boss: Some("hrolf") },
    UniqueDef { name: "EMBERSCALE", base: "chain", req: 18, stats: &[(Stat::Armor, 30), (Stat::Life, 60), (Stat::Fire, 40), (Stat::LifeRegen, 6), (Stat::Gold, 50)], boss: Some("firewyrm") },
    UniqueDef { name: "WELL-WITCH'S CHARM", base: "amulet", req: 6, stats: &[(Stat::Skills, 1), (Stat::Mana, 25), (Stat::ManaRegen, 30), (Stat::Life, 10)], boss: Some("wellwitch") },
    UniqueDef { name: "CINDERWALKERS", base: "cboots", req: 3, stats: &[(Stat::Armor, 6), (Stat::Move, 20), (Stat::Stamina, 50), (Stat::Fire, 10)], boss: None },
    UniqueDef { name: "EMBERHEART", base: "amulet", req: 6, stats: &[(Stat::Fire, 25), (Stat::ManaRegen, 30), (Stat::Life, 15)], boss: None },
    UniqueDef { name: "THE KINDLING", base: "gnarled", req: 1, stats: &[(Stat::Fire, 35), (Stat::Mana, 20), (Stat::Cast, 10)], boss: None },
    // ---- Act 2 ----
    UniqueDef { name: "OVERSEER'S GIRDLE", base: "hbelt", req: 16, stats: &[(Stat::Armor, 12), (Stat::Life, 45), (Stat::Stamina, 50), (Stat::LifeRegen, 4)], boss: Some("giant") },
    UniqueDef { name: "MATRIARCH'S MANTLE", base: "chain", req: 18, stats: &[(Stat::Armor, 34), (Stat::Life, 40), (Stat::Hunger, 50), (Stat::LifeOnKill, 6)], boss: Some("yeti") },
    UniqueDef { name: "RIME CIRCLET", base: "circlet", req: 20, stats: &[(Stat::Skills, 1), (Stat::Mana, 55), (Stat::ManaRegen, 60), (Stat::Cast, 20)], boss: Some("witch") },
    UniqueDef { name: "WYRMHEART", base: "amulet", req: 22, stats: &[(Stat::Skills, 2), (Stat::Fire, 45), (Stat::Life, 50), (Stat::Magic, 30)], boss: Some("dragon") },
    // ---- Act 3 ----
    UniqueDef { name: "BONE BARON'S MANTLE", base: "chain", req: 24, stats: &[(Stat::Armor, 40), (Stat::Life, 60), (Stat::LifeOnKill, 8), (Stat::Skills, 1)], boss: Some("ossric") },
    UniqueDef { name: "GRAVEFIRE CIRCLET", base: "circlet", req: 26, stats: &[(Stat::Skills, 1), (Stat::Fire, 40), (Stat::Mana, 60), (Stat::ManaOnKill, 5)], boss: Some("grimhilde") },
    UniqueDef { name: "DEATHKNIGHT GAUNTLETS", base: "gauntlets", req: 28, stats: &[(Stat::Armor, 18), (Stat::Life, 50), (Stat::Cast, 20), (Stat::LifeRegen, 5)], boss: Some("malgrave") },
    UniqueDef { name: "VARDAK'S SIGNET", base: "ring", req: 30, stats: &[(Stat::Skills, 2), (Stat::Fire, 35), (Stat::LifeOnKill, 10), (Stat::Magic, 40)], boss: Some("vardak") },
    // ---- Act 4 ----
    UniqueDef { name: "FORGEMOTHER'S APRON", base: "leather", req: 32, stats: &[(Stat::Armor, 55), (Stat::Life, 80), (Stat::Fire, 30), (Stat::LifeRegen, 6)], boss: Some("forgemother") },
    UniqueDef { name: "CANTOR'S HYMNAL", base: "circlet", req: 34, stats: &[(Stat::Skills, 2), (Stat::Mana, 80), (Stat::Cast, 20)], boss: Some("cantor") },
    UniqueDef { name: "THE ARCHIVIST'S INDEX", base: "amulet", req: 36, stats: &[(Stat::Skills, 1), (Stat::Magic, 60), (Stat::ManaOnKill, 8), (Stat::Life, 50)], boss: Some("archivist") },
    UniqueDef { name: "THE CLOCKMAKER'S HEART", base: "amulet", req: 38, stats: &[(Stat::Skills, 3), (Stat::Fire, 50), (Stat::Cast, 25), (Stat::LifeOnKill, 12)], boss: Some("clockmaker") },
    // ---- Act 5 ----
    UniqueDef { name: "DREGMOOR'S BICORNE", base: "circlet", req: 40, stats: &[(Stat::Armor, 40), (Stat::Life, 90), (Stat::Gold, 60), (Stat::LifeRegen, 8)], boss: Some("dregmoor") },
    UniqueDef { name: "THE NACRE CROWN", base: "circlet", req: 41, stats: &[(Stat::Skills, 2), (Stat::Mana, 100), (Stat::Cast, 25), (Stat::Magic, 30)], boss: Some("nacre") },
    UniqueDef { name: "THE ANGLER'S LURE", base: "amulet", req: 42, stats: &[(Stat::Skills, 1), (Stat::Magic, 80), (Stat::LifeOnKill, 10), (Stat::Move, 15)], boss: Some("angler") },
    UniqueDef { name: "THE LEVIATHAN'S EYE", base: "ring", req: 44, stats: &[(Stat::Skills, 3), (Stat::Life, 100), (Stat::Fire, 50), (Stat::ManaOnKill, 10)], boss: Some("leviathan") },
    // ---- Act 6 ----
    UniqueDef { name: "VAEL'S BROKEN HALO", base: "circlet", req: 48, stats: &[(Stat::Skills, 2), (Stat::Life, 110), (Stat::Armor, 50), (Stat::LifeOnKill, 12)], boss: Some("vael") },
    UniqueDef { name: "THE STORM'S EYE", base: "ring", req: 49, stats: &[(Stat::Skills, 2), (Stat::Cast, 30), (Stat::Move, 15), (Stat::Mana, 90)], boss: Some("tempest") },
    UniqueDef { name: "THE THOUSAND-EYED WHEEL", base: "amulet", req: 50, stats: &[(Stat::Skills, 2), (Stat::Magic, 100), (Stat::ManaOnKill, 12), (Stat::Life, 80)], boss: Some("ophan") },
    UniqueDef { name: "THE LAST EMBER OF THE SUN", base: "amulet", req: 52, stats: &[(Stat::Skills, 4), (Stat::Fire, 80), (Stat::Cast, 30), (Stat::LifeOnKill, 16)], boss: Some("solanthos") },
    UniqueDef { name: "SKALD'S STRIDE", base: "iboots", req: 16, stats: &[(Stat::Armor, 12), (Stat::Move, 25), (Stat::Stamina, 40), (Stat::Gold, 40)], boss: None },
];

pub fn unique(i: usize) -> Item {
    let u = &UNIQUES[i];
    let base = base_by_key(u.base).expect("unique base");
    Item { base, rarity: Rarity::Unique, ilvl: u.req as u8 + 2, name: u.name.to_string(), stats: u.stats.to_vec(), req: u.req, sockets: 0, gems: vec![] }
}

pub fn boss_unique(boss: &str) -> Option<Item> {
    UNIQUES.iter().position(|u| u.boss == Some(boss)).map(unique)
}

/// A random drop: a world unique when the roll says unique (or a rare if none fits).
pub fn drop(ilvl: u8, mf: i32, boost: bool, rng: &mut Rng) -> Item {
    match roll_rarity(mf, boost, rng) {
        Rarity::Unique => {
            let ok: Vec<usize> = (0..UNIQUES.len()).filter(|&i| UNIQUES[i].boss.is_none() && UNIQUES[i].req <= ilvl as u32 + 3).collect();
            if ok.is_empty() {
                roll(ilvl, Rarity::Rare, rng)
            } else {
                unique(ok[rng.range(0, ok.len() as i32) as usize])
            }
        }
        Rarity::Set => {
            let ok: Vec<(usize, usize)> =
                (0..SETS.len()).filter(|&i| SETS[i].req <= ilvl as u32 + 3).flat_map(|i| (0..SETS[i].pieces.len()).map(move |p| (i, p))).collect();
            if ok.is_empty() {
                roll(ilvl, Rarity::Rare, rng)
            } else {
                let (i, p) = ok[rng.range(0, ok.len() as i32) as usize];
                set_item(i, p)
            }
        }
        r => roll(ilvl, r, rng),
    }
}

// ------------------------------------------------------------------ the bag and worn gear

#[derive(Clone, Debug)]
pub struct Gear {
    pub worn: [Option<Item>; WORN.len()],
    pub bag: Vec<Option<Item>>,
    pub stash: Vec<Option<Item>>,
}

impl Default for Gear {
    fn default() -> Self {
        Gear { worn: Default::default(), bag: vec![None; BAG], stash: vec![None; STASH] }
    }
}

impl Gear {
    pub fn bonus(&self) -> Bonus {
        let mut b = Bonus::default();
        for it in self.worn.iter().flatten() {
            for (s, v) in it.all_stats() {
                if s == Stat::Power {
                    b.0[s as usize] |= v;
                } else {
                    b.0[s as usize] += v;
                }
            }
        }
        for (si, n) in self.sets_worn() {
            for (need, bonus) in SETS[si].bonus {
                if n >= *need as usize {
                    for &(s, v) in *bonus {
                        b.0[s as usize] += v;
                    }
                }
            }
        }
        b
    }

    /// Sets you wear pieces of: (set, how many different pieces).
    pub fn sets_worn(&self) -> Vec<(usize, usize)> {
        let mut out: Vec<(usize, usize)> = vec![];
        let mut names: Vec<&str> = vec![];
        for it in self.worn.iter().flatten() {
            let Some(si) = set_of(it) else { continue };
            if names.contains(&it.name.as_str()) {
                continue;
            }
            names.push(&it.name);
            match out.iter_mut().find(|o| o.0 == si) {
                Some(o) => o.1 += 1,
                None => out.push((si, 1)),
            }
        }
        out
    }

    fn cell_mut(&mut self, worn: bool, i: usize) -> Option<&mut Item> {
        if worn {
            self.worn.get_mut(i)?.as_mut()
        } else {
            self.bag.get_mut(i)?.as_mut()
        }
    }

    /// Sets the gem in bag cell `gem` into an item (worn or in the bag).
    pub fn socket(&mut self, gem: usize, worn: bool, i: usize) -> Result<String, String> {
        let g = self.bag.get(gem).and_then(|c| c.as_ref()).and_then(Item::gem).ok_or_else(|| "THAT IS NOT A GEM".to_string())?;
        if !worn && i == gem {
            return Err(String::new());
        }
        let it = self.cell_mut(worn, i).ok_or_else(String::new)?;
        if it.gem().is_some() {
            return Err("GEMS GO IN GEAR, NOT IN GEMS".into());
        }
        if it.gems.len() >= it.sockets as usize {
            return Err(if it.sockets == 0 { "IT HAS NO SOCKETS. A JEWELER CAN ADD SOME".into() } else { "NO FREE SOCKET".into() });
        }
        it.gems.push(g);
        let name = it.name.clone();
        self.bag[gem] = None;
        Ok(format!("{} SET IN {name}", gem_name(g)))
    }

    /// Takes every gem out of an item and puts them in the bag.
    pub fn unsocket(&mut self, worn: bool, i: usize) -> Result<usize, String> {
        let n = self.cell_mut(worn, i).map_or(0, |it| it.gems.len());
        if n == 0 {
            return Err("NO GEMS TO TAKE OUT".into());
        }
        if self.free() < n {
            return Err("YOUR BAG IS FULL".into());
        }
        let gems = std::mem::take(&mut self.cell_mut(worn, i).unwrap().gems);
        for g in gems {
            let _ = self.add(gem_item(g));
        }
        Ok(n)
    }

    /// The gem `combine_one` would make next (three alike, lowest grade first).
    pub fn next_combine(&self) -> Option<Gem> {
        let mut best: Option<Gem> = None;
        for kind in 0..7u8 {
            for grade in 1..TOP_GRADE {
                let g = Gem { kind, grade };
                if self.bag.iter().flatten().filter(|it| it.gem() == Some(g)).count() >= 3 && best.map_or(true, |b| grade < b.grade) {
                    best = Some(g);
                }
            }
        }
        best.map(|g| Gem { kind: g.kind, grade: g.grade + 1 })
    }

    /// Joins three gems of one kind and grade into one of the next grade.
    pub fn combine_one(&mut self) -> Option<Gem> {
        let up = self.next_combine()?;
        let g = Gem { kind: up.kind, grade: up.grade - 1 };
        let mut left = 3;
        for c in self.bag.iter_mut() {
            if left > 0 && c.as_ref().and_then(Item::gem) == Some(g) {
                *c = None;
                left -= 1;
            }
        }
        let _ = self.add(gem_item(up));
        Some(up)
    }

    /// Puts an item in the first free bag cell. Gives it back when the bag is full.
    pub fn add(&mut self, it: Item) -> Result<usize, Item> {
        match self.bag.iter().position(Option::is_none) {
            Some(i) => {
                self.bag[i] = Some(it);
                Ok(i)
            }
            None => Err(it),
        }
    }

    pub fn free(&self) -> usize {
        self.bag.iter().filter(|c| c.is_none()).count()
    }

    /// The worn slot an item would go to (an empty ring slot first); None for gems.
    pub fn target(&self, it: &Item) -> Option<usize> {
        let s = it.slot();
        let mut slots = (0..WORN.len()).filter(|&i| WORN[i] == s);
        let first = slots.next()?;
        Some(match slots.next() {
            Some(second) if self.worn[first].is_some() && self.worn[second].is_none() => second,
            _ => first,
        })
    }

    /// Wears the item in bag cell `i`, swapping whatever was worn into its place.
    pub fn equip(&mut self, i: usize, clvl: u32) -> Result<(), String> {
        let Some(it) = self.bag[i].take() else { return Err(String::new()) };
        if it.slot() == Slot::Gem {
            self.bag[i] = Some(it);
            return Err("GEMS GO IN SOCKETS".into());
        }
        if it.req > clvl {
            let why = format!("NEEDS CHAR LEVEL {}", it.req);
            self.bag[i] = Some(it);
            return Err(why);
        }
        let Some(t) = self.target(&it) else {
            self.bag[i] = Some(it);
            return Err("GEMS GO IN SOCKETS".into());
        };
        self.bag[i] = self.worn[t].take();
        self.worn[t] = Some(it);
        Ok(())
    }

    /// Takes off worn slot `w` into the bag.
    pub fn unequip(&mut self, w: usize) -> Result<(), String> {
        let Some(it) = self.worn[w].take() else { return Err(String::new()) };
        match self.add(it) {
            Ok(_) => Ok(()),
            Err(it) => {
                self.worn[w] = Some(it);
                Err("YOUR BAG IS FULL".into())
            }
        }
    }

    pub fn save_text(&self) -> String {
        let mut s = String::new();
        let enc = |it: &Item| {
            let st: Vec<String> = it.stats.iter().map(|(k, v)| format!("{}:{v}", stat_key(*k))).collect();
            let gems: Vec<String> = it.gems.iter().map(|g| format!("{}.{}", g.kind, g.grade)).collect();
            format!("{}|{}|{}|{}|{}|{}|{};{}", BASES[it.base].key, it.rarity as u8, it.ilvl, it.req, it.name, st.join(","), it.sockets, gems.join(","))
        };
        for (i, it) in self.worn.iter().enumerate() {
            if let Some(it) = it {
                s += &format!("worn{i}={}\n", enc(it));
            }
        }
        for (i, it) in self.bag.iter().enumerate() {
            if let Some(it) = it {
                s += &format!("bag{i}={}\n", enc(it));
            }
        }
        for (i, it) in self.stash.iter().enumerate() {
            if let Some(it) = it {
                s += &format!("stash{i}={}\n", enc(it));
            }
        }
        s
    }

    pub fn load_text(text: &str) -> Gear {
        let mut g = Gear::default();
        let dec = |v: &str| -> Option<Item> {
            let f: Vec<&str> = v.trim().split('|').collect();
            let rarity = match f.get(1)?.parse::<u8>().ok()? {
                0 => Rarity::Normal,
                1 => Rarity::Magic,
                2 => Rarity::Rare,
                4 => Rarity::Set,
                5 => Rarity::Ancient,
                _ => Rarity::Unique,
            };
            let mut stats = vec![];
            for part in f.get(5)?.split(',').filter(|p| !p.is_empty()) {
                let (k, n) = part.split_once(':')?;
                let s = STATS.iter().copied().find(|s| stat_key(*s) == k)?;
                stats.push((s, n.parse().ok()?));
            }
            let (mut sockets, mut gems) = (0, vec![]);
            if let Some((n, gs)) = f.get(6).and_then(|x| x.split_once(';')) {
                sockets = n.parse().unwrap_or(0);
                for g in gs.split(',').filter(|p| !p.is_empty()) {
                    let (k, q) = g.split_once('.')?;
                    gems.push(Gem { kind: k.parse::<u8>().ok()?.min(6), grade: q.parse::<u8>().ok()?.clamp(1, TOP_GRADE) });
                }
            }
            Some(Item { base: base_by_key(f[0])?, rarity, ilvl: f.get(2)?.parse().ok()?, req: f.get(3)?.parse().ok()?, name: f.get(4)?.to_string(), stats, sockets, gems })
        };
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else { continue };
            if let Some(i) = k.strip_prefix("worn").and_then(|n| n.parse::<usize>().ok()).filter(|&i| i < WORN.len()) {
                g.worn[i] = dec(v);
            } else if let Some(i) = k.strip_prefix("bag").and_then(|n| n.parse::<usize>().ok()).filter(|&i| i < BAG) {
                g.bag[i] = dec(v);
            } else if let Some(i) = k.strip_prefix("stash").and_then(|n| n.parse::<usize>().ok()).filter(|&i| i < STASH) {
                g.stash[i] = dec(v);
            }
        }
        g
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rolled_items_follow_their_rarity() {
        let mut rng = Rng::new(7);
        for ilvl in [1u8, 5, 12, 20] {
            for _ in 0..200 {
                let n = roll(ilvl, Rarity::Normal, &mut rng);
                assert!(n.stats.len() <= 1, "white items only have their base stat");
                assert!(n.base().lvl <= ilvl);
                let m = roll(ilvl, Rarity::Magic, &mut rng);
                let extra = m.stats.len() - m.base().implicit.is_some() as usize;
                assert!((1..=2).contains(&extra), "magic: 1-2 affixes, got {:?}", m);
                let r = roll(ilvl, Rarity::Rare, &mut rng);
                let extra = r.stats.len() - r.base().implicit.is_some() as usize;
                assert!((2..=5).contains(&extra), "rare: several affixes, got {:?}", r);
                // No stat appears twice as an affix.
                let mut seen = vec![];
                for (s, _) in r.stats.iter().skip(r.base().implicit.is_some() as usize) {
                    assert!(!seen.contains(s));
                    seen.push(*s);
                }
                assert!(r.name.split(' ').count() == 2);
            }
        }
    }

    #[test]
    fn deeper_items_roll_higher() {
        let mut rng = Rng::new(3);
        let avg = |ilvl: u8, rng: &mut Rng| -> f32 {
            (0..400).map(|_| roll(ilvl, Rarity::Rare, rng).stat(Stat::Life) as f32).sum::<f32>() / 400.0
        };
        assert!(avg(20, &mut rng) > avg(2, &mut rng) * 1.5);
    }

    #[test]
    fn magic_find_finds_more_magic() {
        let mut rng = Rng::new(9);
        let count = |mf: i32, rng: &mut Rng| (0..4000).filter(|_| roll_rarity(mf, false, rng) != Rarity::Normal).count();
        let (a, b) = (count(0, &mut rng), count(100, &mut rng));
        assert!(b as f32 > a as f32 * 1.4, "{a} vs {b}");
        assert!(a > 1000 && a < 2000, "about a third of drops are magic or better: {a}");
    }

    #[test]
    fn equipping_swaps_and_sums_stats() {
        let mut g = Gear::default();
        let ring = unique(2); // Hexbinder, needs level 10
        g.add(ring.clone()).unwrap();
        assert!(g.equip(0, 5).is_err(), "too low a level");
        g.equip(0, 10).unwrap();
        assert_eq!(g.bonus().get(Stat::Skills), 1);
        // A second ring goes in the empty ring slot.
        let mut rng = Rng::new(1);
        let other = roll_base(base_by_key("ring").unwrap(), 5, Rarity::Magic, &mut rng);
        g.add(other).unwrap();
        g.equip(0, 10).unwrap();
        assert!(g.worn[6].is_some() && g.worn[7].is_some());
        // A third ring swaps out the first.
        g.add(roll_base(base_by_key("ring").unwrap(), 5, Rarity::Magic, &mut rng)).unwrap();
        g.equip(0, 10).unwrap();
        assert_eq!(g.bag[0].as_ref().map(|i| i.name.as_str()), Some("HEXBINDER"));
        assert_eq!(g.bonus().get(Stat::Skills), 0);
        g.unequip(8).unwrap_err(); // nothing in the amulet slot
        g.unequip(7).unwrap();
        assert!(g.worn[7].is_none());
    }

    #[test]
    fn gear_saves_and_loads() {
        let mut rng = Rng::new(5);
        let mut g = Gear::default();
        g.worn[0] = Some(roll(10, Rarity::Rare, &mut rng));
        g.worn[3] = Some(unique(0));
        g.bag[4] = Some(roll(3, Rarity::Magic, &mut rng));
        g.bag[29] = Some(roll(1, Rarity::Normal, &mut rng));
        g.stash[17] = Some(unique(3));
        let h = Gear::load_text(&g.save_text());
        assert_eq!(h.worn, g.worn);
        assert_eq!(h.bag, g.bag);
        assert_eq!(h.stash, g.stash);
        assert_eq!(h.bonus(), g.bonus());
    }

    #[test]
    fn gems_socket_combine_and_save() {
        let mut g = Gear::default();
        let mut rng = Rng::new(4);
        let mut staff = roll_base(base_by_key("runed").unwrap(), 10, Rarity::Normal, &mut rng);
        staff.sockets = 2;
        let fire = staff.stat(Stat::Fire);
        g.add(staff).unwrap();
        for _ in 0..3 {
            g.add(gem_item(Gem { kind: 0, grade: 1 })).unwrap();
        }
        assert!(g.equip(1, 30).is_err(), "gems can't be worn");
        // Three chipped rubies make a flawed one.
        assert_eq!(g.combine_one(), Some(Gem { kind: 0, grade: 2 }));
        assert_eq!(g.combine_one(), None);
        let gi = g.bag.iter().position(|c| c.as_ref().map_or(false, |i| i.gem().is_some())).unwrap();
        g.socket(gi, false, 0).unwrap();
        let s = g.bag[0].as_ref().unwrap();
        assert_eq!(s.stat(Stat::Fire), fire + 6, "a flawed ruby in a weapon: +6% damage");
        assert_eq!(g.free(), BAG - 1);
        g.equip(0, 30).unwrap();
        assert_eq!(g.bonus().get(Stat::Fire), fire + 6);
        let h = Gear::load_text(&g.save_text());
        assert_eq!(h.worn, g.worn);
        // Taking it out gives the gem back.
        g.unsocket(true, 0).unwrap();
        assert_eq!(g.bonus().get(Stat::Fire), fire);
        assert!(g.bag.iter().flatten().any(|i| i.gem() == Some(Gem { kind: 0, grade: 2 })));
    }

    #[test]
    fn set_bonuses_grow_with_pieces() {
        let mut g = Gear::default();
        let si = SETS.iter().position(|d| d.name == "WANDERER'S").unwrap();
        g.add(set_item(si, 0)).unwrap();
        g.equip(0, 10).unwrap();
        let one = g.bonus();
        assert_eq!(one.get(Stat::Stamina), 0);
        g.add(set_item(si, 1)).unwrap();
        g.equip(0, 10).unwrap();
        assert_eq!(g.bonus().get(Stat::Stamina), 40, "two pieces: the first bonus");
        g.add(set_item(si, 2)).unwrap();
        g.equip(0, 10).unwrap();
        assert_eq!(g.sets_worn(), vec![(si, 3)]);
        assert_eq!(g.bonus().get(Stat::Magic), 20);
        let h = Gear::load_text(&g.save_text());
        assert_eq!(h.bonus(), g.bonus());
        for d in SETS {
            for p in d.pieces {
                assert!(base_by_key(p.base).is_some(), "{}", p.name);
            }
        }
    }

    #[test]
    fn every_base_slot_drops_from_level_one() {
        let mut rng = Rng::new(2);
        let mut slots = vec![];
        for _ in 0..500 {
            let s = roll(1, Rarity::Magic, &mut rng).slot();
            if !slots.contains(&s) {
                slots.push(s);
            }
        }
        assert_eq!(slots.len(), 8);
        for u in UNIQUES {
            assert!(base_by_key(u.base).is_some(), "{}", u.name);
        }
    }
}

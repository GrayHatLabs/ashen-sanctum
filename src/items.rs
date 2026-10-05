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
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord)]
pub enum Rarity {
    Normal,
    Magic,
    Rare,
    Unique,
}

/// Name colour per rarity (D2: white, blue, yellow, gold).
pub fn rarity_col(r: Rarity) -> u32 {
    match r {
        Rarity::Normal => 0xd8d8d0,
        Rarity::Magic => 0x7090ff,
        Rarity::Rare => 0xf0e060,
        Rarity::Unique => 0xc89850,
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
}

pub const STATS: [Stat; 15] = [
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
    }
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
    pub fn stat(&self, s: Stat) -> i32 {
        self.stats.iter().filter(|(t, _)| *t == s).map(|(_, v)| *v).sum()
    }
    /// What Gerta charges for it.
    pub fn cost(&self) -> i32 {
        self.price() * 4
    }

    /// What Gerta pays for it.
    pub fn price(&self) -> i32 {
        let mult = match self.rarity {
            Rarity::Normal => 1,
            Rarity::Magic => 3,
            Rarity::Rare => 6,
            Rarity::Unique => 12,
        };
        (4 + self.ilvl as i32 * 2) * mult
    }
    /// Tooltip lines (after the name).
    pub fn lines(&self) -> Vec<String> {
        let mut v = vec![];
        if self.rarity != Rarity::Normal || self.name != self.base().name {
            v.push(self.base().name.to_string());
        }
        for s in STATS {
            let n = self.stat(s);
            if n != 0 {
                v.push(stat_text(s, n));
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
    (tier * 4.5).round().clamp(1.0, 30.0) as u8
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
        Rarity::Normal | Rarity::Unique => (0, 0),
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
        Rarity::Normal | Rarity::Unique => bd.name.to_string(),
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
    Item { base, rarity, ilvl, name, stats, req }
}

/// Rarity of a monster drop. `mf` is your magic-find %; `boost` is 1 for bosses.
pub fn roll_rarity(mf: i32, boost: bool, rng: &mut Rng) -> Rarity {
    let mf = mf as f32;
    let k = if boost { 4.0 } else { 1.0 };
    let r = rng.f();
    if r < 0.006 * k * (1.0 + mf / 250.0) {
        Rarity::Unique
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
    UniqueDef { name: "SKALD'S STRIDE", base: "iboots", req: 16, stats: &[(Stat::Armor, 12), (Stat::Move, 25), (Stat::Stamina, 40), (Stat::Gold, 40)], boss: None },
];

pub fn unique(i: usize) -> Item {
    let u = &UNIQUES[i];
    let base = base_by_key(u.base).expect("unique base");
    Item { base, rarity: Rarity::Unique, ilvl: u.req as u8 + 2, name: u.name.to_string(), stats: u.stats.to_vec(), req: u.req }
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
            for &(s, v) in &it.stats {
                b.0[s as usize] += v;
            }
        }
        b
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

    /// The worn slot an item would go to (an empty ring slot first).
    pub fn target(&self, it: &Item) -> usize {
        let s = it.slot();
        let mut slots = (0..WORN.len()).filter(|&i| WORN[i] == s);
        let first = slots.next().unwrap();
        match slots.next() {
            Some(second) if self.worn[first].is_some() && self.worn[second].is_none() => second,
            _ => first,
        }
    }

    /// Wears the item in bag cell `i`, swapping whatever was worn into its place.
    pub fn equip(&mut self, i: usize, clvl: u32) -> Result<(), String> {
        let Some(it) = self.bag[i].take() else { return Err(String::new()) };
        if it.req > clvl {
            let why = format!("NEEDS CHAR LEVEL {}", it.req);
            self.bag[i] = Some(it);
            return Err(why);
        }
        let t = self.target(&it);
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
            format!("{}|{}|{}|{}|{}|{}", BASES[it.base].key, it.rarity as u8, it.ilvl, it.req, it.name, st.join(","))
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
                _ => Rarity::Unique,
            };
            let mut stats = vec![];
            for part in f.get(5)?.split(',').filter(|p| !p.is_empty()) {
                let (k, n) = part.split_once(':')?;
                let s = STATS.iter().copied().find(|s| stat_key(*s) == k)?;
                stats.push((s, n.parse().ok()?));
            }
            Some(Item { base: base_by_key(f[0])?, rarity, ilvl: f.get(2)?.parse().ok()?, req: f.get(3)?.parse().ok()?, name: f.get(4)?.to_string(), stats })
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

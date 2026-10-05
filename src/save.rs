//! Saving and loading the character, D2 style: your hero, gold, potions and quest
//! progress persist; the world (monsters, loot) is fresh each time you load.
use crate::game::Game;
use std::path::PathBuf;

/// `$XDG_DATA_HOME/ashensanctum/save.txt` (or `~/.local/share/...`); next to the
/// executable when neither is set (handheld ports).
static FILE_NAME: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

/// Use another save file in the save folder: a hero's slot (`heroes/<slot>.txt`), or
/// `save_act2.txt` for the `--act2` test character.
pub fn use_file(name: &str) {
    *FILE_NAME.lock().unwrap() = Some(name.to_string());
}

/// The save folder.
pub fn dir() -> PathBuf {
    if let Ok(p) = std::env::var("ASHEN_SAVE") {
        return PathBuf::from(p).parent().map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    }
    let base = std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|_| std::env::var("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        .unwrap_or_else(|_| PathBuf::from("."));
    base.join("ashensanctum")
}

pub fn path() -> PathBuf {
    let name = FILE_NAME.lock().unwrap().clone();
    if let (Ok(p), None) = (std::env::var("ASHEN_SAVE"), &name) {
        return PathBuf::from(p);
    }
    dir().join(name.unwrap_or_else(|| "save.txt".into()))
}

// ------------------------------------------------------------------ heroes (one save per character)

/// A saved hero, as the character select screen shows it.
#[derive(Clone, Debug, PartialEq)]
pub struct HeroInfo {
    /// File stem in heroes/.
    pub slot: String,
    pub name: String,
    pub class: crate::skills::Class,
    pub clvl: u32,
    /// 0 = Act 1, 1 = Act 2.
    pub act: usize,
    pub difficulty: u8,
}

pub fn heroes_dir() -> PathBuf {
    dir().join("heroes")
}

/// Reads the hero summary out of a save file's text.
pub fn hero_info(slot: &str, text: &str) -> Option<HeroInfo> {
    let get = |k: &str| text.lines().find_map(|l| l.strip_prefix(k).and_then(|r| r.strip_prefix('='))).map(str::trim);
    let class = match get("class") {
        Some("vampire") => crate::skills::Class::Vampire,
        Some("inventor") => crate::skills::Class::Inventor,
        Some("valkyrie") => crate::skills::Class::Valkyrie,
        Some("berserker") => crate::skills::Class::Berserker,
        _ => crate::skills::Class::Sorceress,
    };
    Some(HeroInfo {
        slot: slot.to_string(),
        name: get("name").filter(|n| !n.is_empty()).unwrap_or("HERO").to_string(),
        class,
        clvl: get("clvl")?.parse::<f64>().ok()? as u32,
        act: get("act").and_then(|a| a.parse().ok()).unwrap_or(0),
        difficulty: get("difficulty").and_then(|a| a.parse().ok()).unwrap_or(0),
    })
}

/// Every saved hero, sorted by name. Copies an old single save.txt in as the first hero.
pub fn list_heroes() -> Vec<HeroInfo> {
    let hd = heroes_dir();
    let old = dir().join("save.txt");
    let empty = std::fs::read_dir(&hd).map(|mut r| r.next().is_none()).unwrap_or(true);
    if empty && old.exists() {
        let _ = std::fs::create_dir_all(&hd);
        let _ = std::fs::copy(&old, hd.join("hero1.txt"));
    }
    let mut v: Vec<HeroInfo> = std::fs::read_dir(&hd)
        .map(|r| {
            r.flatten()
                .filter_map(|e| {
                    let p = e.path();
                    if p.extension()? != "txt" {
                        return None;
                    }
                    let slot = p.file_stem()?.to_str()?.to_string();
                    hero_info(&slot, &std::fs::read_to_string(&p).ok()?)
                })
                .collect()
        })
        .unwrap_or_default();
    v.sort_by(|a, b| a.name.cmp(&b.name).then(a.slot.cmp(&b.slot)));
    v
}

/// Points saving at a hero's slot.
pub fn use_hero(slot: &str) {
    let _ = std::fs::create_dir_all(heroes_dir());
    use_file(&format!("heroes/{slot}.txt"));
}

/// A free slot name for a new hero.
pub fn new_slot(name: &str) -> String {
    let base: String = name.chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>().to_lowercase();
    let base = if base.is_empty() { "hero".to_string() } else { base };
    let mut slot = base.clone();
    let mut k = 2;
    while heroes_dir().join(format!("{slot}.txt")).exists() {
        slot = format!("{base}{k}");
        k += 1;
    }
    slot
}

pub fn delete_hero(slot: &str) {
    let _ = std::fs::remove_file(heroes_dir().join(format!("{slot}.txt")));
}

pub fn to_text(g: &Game) -> String {
    let p = &g.p;
    let q = &g.quest;
    let bits = |b: &[bool; 3]| b.iter().map(|s| if *s { '1' } else { '0' }).collect::<String>();
    let mut s = format!("name={}\n", g.hero_name);
    s += &format!(
        "version=1\nseed={}\nclvl={}\nxp={}\nmax_hp={}\nmax_mana={}\npower={}\ngold={}\nhp_pots={}\nmp_pots={}\nfood={}\nrunning={}\nstage={}\nseals={}\nkills={}\n",
        g.world_seed(),
        p.clvl,
        p.xp,
        p.base_hp,
        p.base_mana,
        p.power,
        p.gold,
        p.hp_pots,
        p.mp_pots,
        p.food,
        p.running as u8,
        q.stage,
        bits(&q.seals),
        g.kills
    );
    s += &format!("difficulty={}\n", q.difficulty);
    s += &format!("stage2={}\nrunes={}\n", q.stage2, bits(&q.runes));
    s += &format!("stage3={}\nsigils={}\n", q.stage3, bits(&q.sigils));
    s += &format!("stage4={}\nkeys={}\n", q.stage4, bits(&q.keys));
    s += &format!("act={}\n", g.level.act());
    s += &p.skills.save_text();
    s += &format!("waypoints={}\n", g.waypoints.iter().map(|w| crate::levels::id_string(*w)).collect::<Vec<_>>().join(","));
    s += &p.gear.save_text();
    s
}

/// Applies a saved character to a freshly created game. Returns false on a bad file.
pub fn apply(g: &mut Game, text: &str) -> bool {
    let get = |k: &str| text.lines().find_map(|l| l.strip_prefix(k).and_then(|r| r.strip_prefix('='))).map(str::trim);
    let num = |k: &str| get(k).and_then(|v| v.parse::<f64>().ok());
    if get("version") != Some("1") {
        return false;
    }
    let (Some(clvl), Some(max_hp), Some(max_mana)) = (num("clvl"), num("max_hp"), num("max_mana")) else { return false };
    g.p.clvl = clvl as u32;
    g.p.xp = num("xp").unwrap_or(0.0) as f32;
    // max_hp / max_mana in the file are life and mana before gear.
    g.p.base_hp = max_hp as f32;
    g.p.base_mana = max_mana as f32;
    g.p.power = num("power").unwrap_or(1.0) as f32;
    g.p.gold = num("gold").unwrap_or(0.0) as i32;
    g.p.hp_pots = num("hp_pots").unwrap_or(3.0) as i32;
    g.p.mp_pots = num("mp_pots").unwrap_or(3.0) as i32;
    g.p.food = (num("food").unwrap_or(100.0) as f32).max(40.0);
    g.p.running = num("running").unwrap_or(1.0) != 0.0;
    g.quest.stage = num("stage").unwrap_or(0.0) as u8;
    if let Some(n) = get("name") {
        g.hero_name = n.to_string();
    }
    g.quest.difficulty = (num("difficulty").unwrap_or(0.0) as u8).min(2);
    g.quest.stage2 = (num("stage2").unwrap_or(0.0) as u8).min(3);
    if let Some(s) = get("runes") {
        for (i, c) in s.chars().take(3).enumerate() {
            g.quest.runes[i] = c == '1';
        }
    }
    g.quest.stage3 = (num("stage3").unwrap_or(0.0) as u8).min(3);
    if let Some(s) = get("sigils") {
        for (i, c) in s.chars().take(3).enumerate() {
            g.quest.sigils[i] = c == '1';
        }
    }
    g.quest.stage4 = (num("stage4").unwrap_or(0.0) as u8).min(3);
    if let Some(s) = get("keys") {
        for (i, c) in s.chars().take(3).enumerate() {
            g.quest.keys[i] = c == '1';
        }
    }
    let act = match num("act").unwrap_or(0.0) as usize {
        3 if g.quest.gears_open() => 3,
        2 if g.quest.mists_open() => 2,
        1 if g.quest.north_open() => 1,
        _ => 0,
    };
    if let Some(s) = get("seals") {
        for (i, c) in s.chars().take(3).enumerate() {
            g.quest.seals[i] = c == '1';
        }
    }
    g.kills = num("kills").unwrap_or(0.0) as u32;
    // Older saves have no skills: hand out the points they would have earned.
    g.p.skills = crate::skills::Skills::load_text(text).unwrap_or_else(|| {
        let mut sk = crate::skills::Skills::default();
        sk.points = g.p.clvl + g.quest.seal_count() as u32;
        sk
    });
    // Saves from before equipment existed keep the starting staff.
    if text.lines().any(|l| l.starts_with("worn") || l.starts_with("bag") || l.starts_with("stash")) {
        g.p.gear = crate::items::Gear::load_text(text);
    }
    if let Some(w) = get("waypoints") {
        for id in w.split(',').filter_map(crate::levels::parse_id) {
            if !g.waypoints.contains(&id) {
                g.waypoints.push(id);
            }
        }
    }
    // You wake in the town of the act you saved in.
    if g.quest.difficulty > 0 || act > 0 {
        g.rebuild_world(act.min(3));
    }
    g.p.recalc();
    g.p.hp = g.p.max_hp;
    g.p.mana = g.p.max_mana;
    true
}

pub fn seed_of(text: &str) -> Option<u64> {
    text.lines().find_map(|l| l.strip_prefix("seed=")).and_then(|v| v.trim().parse().ok())
}

pub fn write(g: &Game) {
    let p = path();
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(&p, to_text(g));
}

pub fn read() -> Option<String> {
    std::fs::read_to_string(path()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_saved_character_comes_back() {
        let mut g = Game::new(42, crate::gfx::SH_WIDE);
        g.p.clvl = 7;
        g.p.base_hp = 133.0;
        g.p.gear.bag[3] = Some(crate::items::unique(1));
        g.p.gold = 321;
        g.p.power = 1.8;
        g.quest.stage = 1;
        g.quest.seals = [true, false, true];
        let text = to_text(&g);
        assert_eq!(seed_of(&text), Some(42));
        let mut h = Game::new(42, crate::gfx::SH_WIDE);
        assert!(apply(&mut h, &text));
        assert_eq!(h.p.clvl, 7);
        assert_eq!(h.p.max_hp, 133.0);
        assert_eq!(h.p.hp, 133.0);
        assert_eq!(h.p.gear.bag[3].as_ref().map(|i| i.name.as_str()), Some("ROTMOTHER COWL"));
        assert_eq!(h.p.gear.worn[0].as_ref().map(|i| i.name.as_str()), Some("GNARLED STAFF"));
        assert_eq!(h.p.gold, 321);
        assert!((h.p.power - 1.8).abs() < 1e-5);
        assert_eq!(h.quest.stage, 1);
        assert_eq!(h.quest.seals, [true, false, true]);
        assert!(!apply(&mut h, "garbage"));
    }
}

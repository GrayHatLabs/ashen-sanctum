//! Saving and loading the character, D2 style: your hero, gold, potions and quest
//! progress persist; the world (monsters, loot) is fresh each time you load.
use crate::game::Game;
use std::path::PathBuf;

/// `$XDG_DATA_HOME/ashensanctum/save.txt` (or `~/.local/share/...`); next to the
/// executable when neither is set (handheld ports).
pub fn path() -> PathBuf {
    if let Ok(p) = std::env::var("ASHEN_SAVE") {
        return PathBuf::from(p);
    }
    let base = std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|_| std::env::var("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        .unwrap_or_else(|_| PathBuf::from("."));
    base.join("ashensanctum").join("save.txt")
}

pub fn to_text(g: &Game) -> String {
    let p = &g.p;
    let q = &g.quest;
    let seals: String = q.seals.iter().map(|s| if *s { '1' } else { '0' }).collect();
    format!(
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
        seals,
        g.kills
    ) + &p.skills.save_text()
        + &format!("waypoints={}
", g.waypoints.iter().map(|w| crate::levels::id_string(*w)).collect::<Vec<_>>().join(","))
        + &p.gear.save_text()
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
    if text.lines().any(|l| l.starts_with("worn") || l.starts_with("bag")) {
        g.p.gear = crate::items::Gear::load_text(text);
    }
    if let Some(w) = get("waypoints") {
        for id in w.split(',').filter_map(crate::levels::parse_id) {
            if !g.waypoints.contains(&id) {
                g.waypoints.push(id);
            }
        }
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

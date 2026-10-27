//! Gamepad buttons (the GAMEPAD BUTTONS screen in the help menu): what each pad button does. START (bag, and
//! confirm), SELECT (tap: map, hold: eat or drink; SELECT + START: the quit menu) and the D-pad (move) stay as
//! they are; the rest can be given any action, or none. Saved in the save folder (controls.txt).
use std::sync::Mutex;

/// A pad button that can be remapped (by its printed label on a handheld; main.rs `by_label`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PadBtn {
    A,
    B,
    X,
    Y,
    L1,
    R1,
    L2,
    R2,
    L3,
    R3,
}

pub const PAD_BTNS: [PadBtn; 10] = [PadBtn::A, PadBtn::B, PadBtn::X, PadBtn::Y, PadBtn::L1, PadBtn::R1, PadBtn::L2, PadBtn::R2, PadBtn::L3, PadBtn::R3];

impl PadBtn {
    pub fn name(self) -> &'static str {
        ["A", "B", "X", "Y", "L1", "R1", "L2", "R2", "L3 (LEFT STICK CLICK)", "R3 (RIGHT STICK CLICK)"][self as usize]
    }
    fn key(self) -> &'static str {
        ["a", "b", "x", "y", "l1", "r1", "l2", "r2", "l3", "r3"][self as usize]
    }
}

/// What a pad button can do.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PadAct {
    Primary,
    Secondary,
    NextSkill,
    SkillTree,
    Run,
    HealPotion,
    ManaPotion,
    Back,
}

pub const PAD_ACTS: [PadAct; 8] = [PadAct::Primary, PadAct::Secondary, PadAct::NextSkill, PadAct::SkillTree, PadAct::Run, PadAct::HealPotion, PadAct::ManaPotion, PadAct::Back];

impl PadAct {
    pub fn name(self) -> &'static str {
        [
            "MAIN SKILL (AND TALK, CONFIRM)",
            "SECOND SKILL (AND DROP / SELL IN THE BAG)",
            "SWITCH SECOND SKILL",
            "SKILL TREE",
            "RUN / WALK",
            "HEALING POTION",
            "MANA POTION",
            "BACK (CLOSE THE BAG, MAP, TALK)",
        ][self as usize]
    }
    fn key(self) -> &'static str {
        ["primary", "secondary", "next_skill", "skill_tree", "run", "heal_potion", "mana_potion", "back"][self as usize]
    }
}

/// The out-of-the-box layout: run / walk on a stick click (it was on B, too easy to hit by accident).
pub const DEFAULTS: [Option<PadAct>; 10] = [
    Some(PadAct::Primary),    // A
    Some(PadAct::Back),       // B
    Some(PadAct::Secondary),  // X
    Some(PadAct::ManaPotion), // Y
    Some(PadAct::HealPotion), // L1
    Some(PadAct::NextSkill),  // R1
    Some(PadAct::SkillTree),  // L2
    Some(PadAct::Primary),    // R2
    Some(PadAct::Run),        // L3
    Some(PadAct::SkillTree),  // R3
];

static BINDS: Mutex<[Option<PadAct>; 10]> = Mutex::new(DEFAULTS);

/// Event notices (game.rs `notice`): on, or off (then just the one-line message). Saved with the buttons.
static NOTICES: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

pub fn notices() -> bool {
    NOTICES.load(std::sync::atomic::Ordering::Relaxed)
}

pub fn set_notices(on: bool) {
    NOTICES.store(on, std::sync::atomic::Ordering::Relaxed);
    save();
}

/// The pad rumbles when you're hit and on big blows (main.rs).
static RUMBLE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

pub fn rumble() -> bool {
    RUMBLE.load(std::sync::atomic::Ordering::Relaxed)
}

pub fn set_rumble(on: bool) {
    RUMBLE.store(on, std::sync::atomic::Ordering::Relaxed);
    save();
}

/// Large text: conversations, help, notices and messages at double size (small handheld screens).
static LARGE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Players 1-8, like Diablo II's /players: the game acts as if that many heroes were in it. Each one past
/// the first: monsters +50% life, more of them in a pack, +35% experience, more drops and gold.
/// (Online play will set it from the real count.)
static PLAYERS: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(1);

thread_local! {
    /// The party's size for the world on this thread (set by hosting, the server's rooms and the test bot);
    /// otherwise the Players setting. Each server room has its own thread, so its own count.
    static HERE: std::cell::Cell<Option<u8>> = const { std::cell::Cell::new(None) };
}

pub fn players() -> u8 {
    HERE.with(|h| h.get()).unwrap_or_else(|| PLAYERS.load(std::sync::atomic::Ordering::Relaxed)).clamp(1, 8)
}

pub fn set_players(n: u8) {
    PLAYERS.store(n.clamp(1, 8), std::sync::atomic::Ordering::Relaxed);
    save();
}

/// For the test bot (ASHEN_PLAYERS): set, but don't save over the player's settings.
pub fn set_players_quiet(n: u8) {
    HERE.with(|h| h.set(Some(n.clamp(1, 8))));
}

/// Back to the Players setting on this thread.
pub fn clear_players_here() {
    HERE.with(|h| h.set(None));
}

/// The next setting round (1, 2, ... 8, 1).
pub fn cycle_players() {
    set_players(players() % 8 + 1);
}

/// What the extra players do: (monster life, experience, drop chance, extra monster chance per pack member).
pub fn players_scale(n: u8) -> (f32, f32, f32, f32) {
    let k = (n.clamp(1, 8) - 1) as f32;
    (1.0 + 0.5 * k, 1.0 + 0.35 * k, 1.0 + 0.12 * k, 0.15 * k)
}

pub fn large_text() -> bool {
    LARGE.load(std::sync::atomic::Ordering::Relaxed)
}

pub fn set_large_text(on: bool) {
    LARGE.store(on, std::sync::atomic::Ordering::Relaxed);
    save();
}

/// The minimap in the corner (render.rs `draw_minimap`): on, or off.
static MINIMAP: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

pub fn minimap() -> bool {
    MINIMAP.load(std::sync::atomic::Ordering::Relaxed)
}

pub fn set_minimap(on: bool) {
    MINIMAP.store(on, std::sync::atomic::Ordering::Relaxed);
    save();
}

/// What this button does now.
pub fn get(b: PadBtn) -> Option<PadAct> {
    BINDS.lock().map(|m| m[b as usize]).unwrap_or(DEFAULTS[b as usize])
}

/// Gives a button an action (or none), and saves the layout.
pub fn set(b: PadBtn, a: Option<PadAct>) {
    if let Ok(mut m) = BINDS.lock() {
        m[b as usize] = a;
    }
    save();
}

pub fn reset() {
    if let Ok(mut m) = BINDS.lock() {
        *m = DEFAULTS;
    }
    save();
}

/// The buttons that do this, for the help pages ("L1", "A OR R2"; "NO BUTTON" when nothing does).
pub fn label(a: PadAct) -> String {
    let names: Vec<&str> = PAD_BTNS.iter().filter(|&&b| get(b) == Some(a)).map(|b| b.name().split(' ').next().unwrap_or("")).collect();
    if names.is_empty() {
        "NO BUTTON (SET ONE IN HELP > GAMEPAD BUTTONS)".into()
    } else {
        names.join(" OR ")
    }
}

fn path() -> std::path::PathBuf {
    crate::save::dir().join("controls.txt")
}

/// Reads the saved layout (at startup); a missing or broken file keeps the defaults.
pub fn load() {
    let Ok(text) = std::fs::read_to_string(path()) else { return };
    let mut m = DEFAULTS;
    for line in text.lines() {
        let Some((b, a)) = line.split_once('=') else { continue };
        if b.trim() == "notices" {
            NOTICES.store(a.trim() != "off", std::sync::atomic::Ordering::Relaxed);
            continue;
        }
        if b.trim() == "players" {
            PLAYERS.store(a.trim().parse::<u8>().unwrap_or(1).clamp(1, 8), std::sync::atomic::Ordering::Relaxed);
            continue;
        }
        if b.trim() == "large_text" {
            LARGE.store(a.trim() == "on", std::sync::atomic::Ordering::Relaxed);
            continue;
        }
        if b.trim() == "rumble" {
            RUMBLE.store(a.trim() != "off", std::sync::atomic::Ordering::Relaxed);
            continue;
        }
        if b.trim() == "minimap" {
            MINIMAP.store(a.trim() != "off", std::sync::atomic::Ordering::Relaxed);
            continue;
        }
        let Some(b) = PAD_BTNS.iter().find(|x| x.key() == b.trim()) else { continue };
        m[*b as usize] = PAD_ACTS.iter().find(|x| x.key() == a.trim()).copied();
    }
    if let Ok(mut g) = BINDS.lock() {
        *g = m;
    }
}

fn save() {
    // Tests change bindings too: they mustn't touch a player's saved layout.
    if cfg!(test) {
        return;
    }
    let Ok(m) = BINDS.lock() else { return };
    let mut text: String = PAD_BTNS.iter().map(|b| format!("{}={}\n", b.key(), m[*b as usize].map_or("none", |a| a.key()))).collect();
    text += &format!("notices={}\n", if notices() { "on" } else { "off" });
    text += &format!("minimap={}\n", if minimap() { "on" } else { "off" });
    text += &format!("rumble={}\n", if rumble() { "on" } else { "off" });
    text += &format!("large_text={}\n", if large_text() { "on" } else { "off" });
    text += &format!("players={}\n", players());
    let _ = std::fs::create_dir_all(crate::save::dir());
    let _ = std::fs::write(path(), text);
}

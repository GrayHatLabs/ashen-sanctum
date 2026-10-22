//! The messages, and the checks every one from the network passes before the game sees it.
//!
//! Decoding (postcard) is bounded by the frame it comes in, never panics, and refuses trailing bytes.
//! A joiner sends `NetInput`: intentions only (a direction, a point to aim at, which buttons are down),
//! never results. `NetInput::check` turns one into a game `Input` or refuses it.
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

/// Bumped whenever a message changes shape; both ends must agree.
pub const PROTOCOL: u32 = 1;
/// The TCP port a host listens on unless told otherwise.
pub const DEFAULT_PORT: u16 = 4747;
/// Hero names: 1-16 of A-Z, 0-9, space, ' and -.
pub const MAX_NAME: usize = 16;
pub const MAX_CHAT: usize = 120;
/// A hero's save, sent when joining.
pub const MAX_HERO: usize = 96 * 1024;
/// How far from its hero a joiner may aim (tiles): the screen is about 20 across.
pub const MAX_AIM: f32 = 24.0;

/// Joiner → host.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ToHost {
    /// The first message, and only then.
    Hello { protocol: u32, version: String, name: String, hero: String, password: String },
    Input(NetInput),
    Chat(String),
    Bye,
    /// In the bag: use / toss / store / sort (`BAG_*`) the item in a cell (`CELL_*`, index).
    Bag { op: u8, cell: (u8, u16) },
    /// Set the gem in bag cell `from` into the item in `to`.
    Gem { from: u16, to: (u8, u16) },
    /// In the skill tree: learn / make primary / make secondary (`SKILL_*`) a skill (its number in skills::ALL).
    Skill { op: u8, skill: u16 },
}

pub const BAG_USE: u8 = 0;
pub const BAG_TOSS: u8 = 1;
pub const BAG_STORE: u8 = 2;
pub const BAG_SORT: u8 = 3;
pub const CELL_WORN: u8 = 0;
pub const CELL_BAG: u8 = 1;
pub const CELL_STASH: u8 = 2;
pub const SKILL_LEARN: u8 = 0;
pub const SKILL_PRIMARY: u8 = 1;
pub const SKILL_SECONDARY: u8 = 2;

/// A bag cell from the wire, if it's a real one (the shop isn't: that needs the trader).
pub fn cell_in(c: (u8, u16)) -> Option<crate::inventory::Cell> {
    use crate::inventory::Cell;
    let i = c.1 as usize;
    match c.0 {
        CELL_WORN if i < crate::items::WORN.len() => Some(Cell::Worn(i)),
        CELL_BAG if i < crate::items::BAG => Some(Cell::Bag(i)),
        CELL_STASH if i < crate::items::STASH => Some(Cell::Stash(i)),
        _ => None,
    }
}

pub fn cell_out(c: crate::inventory::Cell) -> Option<(u8, u16)> {
    use crate::inventory::Cell;
    match c {
        Cell::Worn(i) => Some((CELL_WORN, i as u16)),
        Cell::Bag(i) => Some((CELL_BAG, i as u16)),
        Cell::Stash(i) => Some((CELL_STASH, i as u16)),
        Cell::Shop(_) => None,
    }
}

/// Host → joiner.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ToClient {
    /// You're in: your hero's number in the party (1-7).
    Welcome { you: u8, host_name: String },
    /// Not let in, and why (shown to the player).
    Refused(String),
    Chat { from: String, text: String },
    /// Closing the connection (the host quit, or kicked you), and why.
    Bye(String),
    /// The level you're in now (net/snap.rs, compressed).
    Level(Vec<u8>),
    /// What's near you this moment (net/snap.rs, compressed).
    Snap(Vec<u8>),
    /// Your hero as the host has it (its save text), for your own save.
    Hero(String),
}

/// One tick of a joiner's controls.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Default)]
pub struct NetInput {
    /// Increases every tick (so stale or replayed input can be told apart).
    pub seq: u32,
    /// Movement, in screen space, as the keys or the stick give it (-1..1 each way).
    pub move_x: f32,
    pub move_y: f32,
    /// The right stick (aim), -1..1 each way.
    pub aim_x: f32,
    pub aim_y: f32,
    /// Where the mouse points, in world tiles (the host turns it into its own screen coordinates).
    pub target: Option<(f32, f32)>,
    /// Which buttons are down: the `B_*` bits.
    pub buttons: u16,
    /// A skill slot picked this tick.
    pub slot: Option<u8>,
}

pub const B_LMB: u16 = 1;
pub const B_RMB: u16 = 1 << 1;
pub const B_STAND: u16 = 1 << 2;
pub const B_CAST: u16 = 1 << 3;
pub const B_CAST2: u16 = 1 << 4;
pub const B_POTION_HP: u16 = 1 << 5;
pub const B_POTION_MP: u16 = 1 << 6;
pub const B_RUN: u16 = 1 << 7;
pub const B_CYCLE: u16 = 1 << 8;
pub const B_EAT: u16 = 1 << 9;
pub const B_SCROLL: u16 = 1 << 10;
const B_ALL: u16 = (1 << 11) - 1;
/// Skill slots on the bar.
pub const MAX_SLOT: u8 = 8;

impl NetInput {
    /// The controls a joiner may use, from this tick's `Input` (the menus and cheats stay at home).
    pub fn from_input(seq: u32, inp: &crate::game::Input, target: Option<(f32, f32)>) -> Self {
        let bit = |on: bool, b: u16| if on { b } else { 0 };
        NetInput {
            seq,
            move_x: inp.move_x,
            move_y: inp.move_y,
            aim_x: inp.aim_x,
            aim_y: inp.aim_y,
            target,
            buttons: bit(inp.lmb, B_LMB)
                | bit(inp.rmb, B_RMB)
                | bit(inp.stand, B_STAND)
                | bit(inp.cast, B_CAST)
                | bit(inp.cast2, B_CAST2)
                | bit(inp.potion_hp, B_POTION_HP)
                | bit(inp.potion_mp, B_POTION_MP)
                | bit(inp.run_toggle, B_RUN)
                | bit(inp.cycle, B_CYCLE)
                | bit(inp.eat, B_EAT)
                | bit(inp.scroll, B_SCROLL),
            slot: inp.slot,
        }
    }

    /// Checks a joiner's input against the hero it drives (standing at `hero`): every number finite and in
    /// range, only known buttons, the aim point near the hero. Returns the game's input (without the mouse:
    /// the host places it from `target`) and the checked target, or why it was refused.
    pub fn check(&self, hero: (f32, f32)) -> Result<(crate::game::Input, Option<(f32, f32)>), &'static str> {
        let finite = [self.move_x, self.move_y, self.aim_x, self.aim_y].iter().all(|v| v.is_finite());
        if !finite {
            return Err("not a number");
        }
        if self.buttons & !B_ALL != 0 {
            return Err("unknown button");
        }
        if self.slot.map_or(false, |s| s >= MAX_SLOT) {
            return Err("no such slot");
        }
        let target = match self.target {
            Some((x, y)) if !x.is_finite() || !y.is_finite() => return Err("not a number"),
            Some((x, y)) if (x - hero.0).powi(2) + (y - hero.1).powi(2) > MAX_AIM * MAX_AIM => None,
            t => t,
        };
        // A direction is at most full tilt, however it's sent.
        let unit = |x: f32, y: f32| {
            let l = (x * x + y * y).sqrt();
            if l > 1.0 {
                (x / l, y / l)
            } else {
                (x, y)
            }
        };
        let (mx, my) = unit(self.move_x, self.move_y);
        let (ax, ay) = unit(self.aim_x, self.aim_y);
        let on = |b: u16| self.buttons & b != 0;
        let inp = crate::game::Input {
            move_x: mx,
            move_y: my,
            aim_x: ax,
            aim_y: ay,
            lmb: on(B_LMB),
            rmb: on(B_RMB),
            stand: on(B_STAND),
            cast: on(B_CAST),
            cast2: on(B_CAST2),
            potion_hp: on(B_POTION_HP),
            potion_mp: on(B_POTION_MP),
            run_toggle: on(B_RUN),
            cycle: on(B_CYCLE),
            eat: on(B_EAT),
            scroll: on(B_SCROLL),
            slot: self.slot,
            ..Default::default()
        };
        Ok((inp, target))
    }
}

/// A hero's name as the game shows it: upper case, 1-16 of A-Z 0-9 space ' -, no space at either end.
pub fn clean_name(s: &str) -> Option<String> {
    let t: String = s.trim().chars().map(|c| c.to_ascii_uppercase()).collect();
    let ok = !t.is_empty() && t.chars().count() <= MAX_NAME && t.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || matches!(c, ' ' | '\'' | '-'));
    ok.then_some(t)
}

/// A chat line as the game's font can show it: upper case, printable, at most 120 characters.
/// Anything else (control characters, other alphabets) is dropped. None if nothing's left.
pub fn clean_chat(s: &str) -> Option<String> {
    let t: String = s
        .chars()
        .map(|c| c.to_ascii_uppercase())
        .filter(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || " .,!?'-:;()+/*#%&=\"^".contains(*c))
        .take(MAX_CHAT)
        .collect();
    let t = t.trim().to_string();
    (!t.is_empty()).then_some(t)
}

pub fn encode<T: Serialize>(m: &T) -> Option<Vec<u8>> {
    postcard::to_stdvec(m).ok()
}

/// Decodes one message, refusing anything malformed or with bytes left over.
pub fn decode<'a, T: Deserialize<'a>>(buf: &'a [u8]) -> Option<T> {
    match postcard::take_from_bytes::<T>(buf) {
        Ok((m, rest)) if rest.is_empty() => Some(m),
        _ => None,
    }
}

/// A joiner's hello, checked: the protocol matches, the name is clean, the save is a sane size.
/// Returns the clean name.
pub fn check_hello(protocol: u32, version: &str, name: &str, hero: &str) -> Result<String, String> {
    if protocol != PROTOCOL || version != env!("CARGO_PKG_VERSION") {
        return Err(format!("VERSION MISMATCH: THE HOST RUNS {}", env!("CARGO_PKG_VERSION")));
    }
    let name = clean_name(name).ok_or_else(|| "BAD HERO NAME".to_string())?;
    if hero.is_empty() || hero.len() > MAX_HERO {
        return Err("BAD HERO".into());
    }
    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_round_trip() {
        let msgs = vec![
            ToHost::Hello { protocol: PROTOCOL, version: "1.0".into(), name: "ISOLDE".into(), hero: "name=ISOLDE\n".into(), password: String::new() },
            ToHost::Input(NetInput { seq: 9, move_x: 0.5, target: Some((3.0, 4.0)), buttons: B_CAST | B_RUN, ..Default::default() }),
            ToHost::Chat("HELLO".into()),
            ToHost::Bye,
        ];
        for m in msgs {
            let b = encode(&m).unwrap();
            assert_eq!(decode::<ToHost>(&b), Some(m));
        }
    }

    #[test]
    fn garbage_never_decodes_into_trouble() {
        // Thousands of random and mangled buffers: decoding must just fail (or give a valid message),
        // never panic or allocate wildly.
        let mut rng = crate::rng::Rng::new(77);
        let good = encode(&ToHost::Input(NetInput { seq: 1, ..Default::default() })).unwrap();
        for k in 0..20_000 {
            let n = rng.range(0, 300) as usize;
            let mut b: Vec<u8> = (0..n).map(|_| rng.range(0, 256) as u8).collect();
            if k % 3 == 0 {
                // A real message with a few bytes flipped.
                b = good.clone();
                for _ in 0..rng.range(1, 4) {
                    let i = rng.range(0, b.len() as i32) as usize;
                    b[i] = rng.range(0, 256) as u8;
                }
            }
            if let Some(ToHost::Input(i)) = decode::<ToHost>(&b) {
                let _ = i.check((0.0, 0.0));
            }
            let _ = decode::<ToClient>(&b);
        }
        // A huge declared length with no data behind it.
        assert_eq!(decode::<ToHost>(&[2, 0xff, 0xff, 0xff, 0xff, 0x0f]), None);
        // Trailing bytes are refused.
        let mut b = encode(&ToHost::Bye).unwrap();
        b.push(0);
        assert_eq!(decode::<ToHost>(&b), None);
    }

    #[test]
    fn inputs_are_checked_and_clamped() {
        let base = NetInput { seq: 1, ..Default::default() };
        assert!(NetInput { move_x: f32::NAN, ..base }.check((0.0, 0.0)).is_err());
        assert!(NetInput { aim_y: f32::INFINITY, ..base }.check((0.0, 0.0)).is_err());
        assert!(NetInput { buttons: 1 << 15, ..base }.check((0.0, 0.0)).is_err(), "unknown button");
        assert!(NetInput { slot: Some(200), ..base }.check((0.0, 0.0)).is_err());
        assert!(NetInput { target: Some((f32::NAN, 0.0)), ..base }.check((0.0, 0.0)).is_err());
        // Faster than full tilt is full tilt.
        let (inp, _) = NetInput { move_x: 50.0, move_y: 0.0, ..base }.check((0.0, 0.0)).unwrap();
        assert!((inp.move_x - 1.0).abs() < 1e-6);
        // Aiming across the map is ignored, near is kept.
        assert_eq!(NetInput { target: Some((500.0, 0.0)), ..base }.check((0.0, 0.0)).unwrap().1, None);
        assert_eq!(NetInput { target: Some((5.0, 1.0)), ..base }.check((2.0, 2.0)).unwrap().1, Some((5.0, 1.0)));
        // Cheats and menus can't be sent at all: the game's input never has them.
        let (inp, _) = NetInput { buttons: B_CAST, ..base }.check((0.0, 0.0)).unwrap();
        assert!(inp.cast && !inp.cheat_level && !inp.cheat_loot && !inp.inv && !inp.skills && inp.typed.is_empty());
    }

    #[test]
    fn names_and_chat_are_cleaned() {
        assert_eq!(clean_name(" isolde "), Some("ISOLDE".into()));
        assert_eq!(clean_name("O'BRIEN-2"), Some("O'BRIEN-2".into()));
        assert_eq!(clean_name(""), None);
        assert_eq!(clean_name("A VERY LONG NAME INDEED"), None);
        assert_eq!(clean_name("BAD\u{0}NAME"), None);
        assert_eq!(clean_name("../../ETC"), None);
        assert_eq!(clean_chat("hi there!\u{7}\u{1b}[31m"), Some("HI THERE!31M".into()));
        assert_eq!(clean_chat("\u{202e}\n\t"), None);
        assert_eq!(clean_chat(&"X".repeat(500)).map(|s| s.len()), Some(MAX_CHAT));
    }

    #[test]
    fn a_hello_from_another_version_is_refused() {
        assert!(check_hello(PROTOCOL, env!("CARGO_PKG_VERSION"), "ROWAN", "name=ROWAN\n").is_ok());
        assert!(check_hello(PROTOCOL + 1, env!("CARGO_PKG_VERSION"), "ROWAN", "x").is_err());
        assert!(check_hello(PROTOCOL, "0.0.1", "ROWAN", "x").is_err());
        assert!(check_hello(PROTOCOL, env!("CARGO_PKG_VERSION"), "ROWAN", "").is_err());
        assert!(check_hello(PROTOCOL, env!("CARGO_PKG_VERSION"), "ROWAN", &"x".repeat(MAX_HERO + 1)).is_err());
    }
}

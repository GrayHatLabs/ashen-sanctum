//! The front end, Diablo 2 style:
//! - **Title screen:** Play / Options / Quit.
//! - **Character select:** saved heroes, plus create or delete.
//! - **New hero:** class select (three cards), then naming.
//! - **Options.**
//! No SDL here: main feeds it `Input` and acts on what it returns.
use crate::art::Art;
use crate::game::Input;
use crate::gfx::{mix, rgb, Align, Fx, Screen, BLACK};
use crate::save::HeroInfo;
use crate::skills::Class;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Stage {
    Title,
    Heroes,
    Create,
    Name,
    Options,
    /// Asking before deleting this hero (index into `heroes`).
    Delete(usize),
}

#[derive(Clone, Debug, PartialEq)]
pub enum MenuOut {
    /// Continue a saved hero.
    Load(String),
    /// Start a new hero.
    New(Class, String),
    ToggleMusic,
    Quit,
}

/// The classes on the create screen: (name, portrait, two lines, colour, playable, in-game sheet).
const CLASSES: [(&str, &str, &str, &str, u32, bool, &str); 7] = [
    ("SORCERESS", "portrait_sorceress", "FIREBALLS,", "METEORS.", 0xff9040, true, "mage"),
    ("VAMPIRE", "portrait_vampire", "BLOOD MAGIC,", "THRALLS.", 0xd04060, true, "vampire"),
    ("INVENTOR", "portrait_inventor", "AETHER GUNS,", "TURRETS.", 0x40c0b0, true, "inventor"),
    ("VALKYRIE", "portrait_valkyrie", "FROST SPEAR,", "SHATTER.", 0x80d0ff, true, "valkyrie"),
    ("BERSERKER", "portrait_berserker", "GIANT AXE,", "DIRE WOLF.", 0xd07040, true, "berserker"),
    ("REAPER", "portrait_reaper", "RUNE SCYTHE,", "SOULS.", 0x9ad8ff, true, "reaper"),
    ("DRUID", "portrait_druid", "PLAGUE AND", "SUMMONS.", 0x90d050, true, "druid"),
];

const NAMES: [&str; 16] = [
    "MORWEN", "LILITH", "SERAPHINE", "VESPERA", "ISOLDE", "RAVENNA", "NYX", "ELSPETH", "CORDELIA", "MIRELLE", "SABLE", "OPHELIA",
    "ROWENA", "THESSALY", "VALERIA", "EMBERLY",
];
const MAX_HEROES: usize = 8;
const MAX_NAME: usize = 12;

pub struct Menu {
    pub stage: Stage,
    pub sel: usize,
    pub heroes: Vec<HeroInfo>,
    pub class_sel: usize,
    pub name: String,
    pub music_on: bool,
    t: f32,
    msg: Option<(String, f32)>,
    prev: Input,
    /// Clickable rectangles from the last draw: (x, y, w, h, item index).
    rects: Vec<(i32, i32, i32, i32, usize)>,
    /// The hero last highlighted in the list (what the DELETE HERO button acts on).
    hero_sel: usize,
    /// First hero row shown (the list scrolls when it doesn't fit).
    scroll: usize,
    /// Rows that fit on screen (from the last draw).
    rows_fit: usize,
    seed: u32,
}

impl Menu {
    pub fn new(heroes: Vec<HeroInfo>) -> Self {
        Menu {
            stage: Stage::Title,
            sel: 0,
            heroes,
            class_sel: 0,
            name: String::new(),
            music_on: true,
            t: 0.0,
            msg: None,
            prev: Input::default(),
            rects: vec![],
            hero_sel: 0,
            scroll: 0,
            rows_fit: 7,
            seed: 1,
        }
    }

    /// Opens straight on the character select screen (after leaving a game).
    pub fn at_heroes(heroes: Vec<HeroInfo>) -> Self {
        let mut m = Menu::new(heroes);
        m.stage = Stage::Heroes;
        m
    }

    fn items(&self) -> usize {
        match self.stage {
            Stage::Title => 3,
            Stage::Heroes => self.heroes.len() + 3,
            Stage::Create => CLASSES.len(),
            Stage::Name => 3,
            Stage::Options => 2,
            Stage::Delete(_) => 2,
        }
    }

    fn go(&mut self, stage: Stage) {
        self.stage = stage;
        self.sel = 0;
        self.rects.clear();
    }

    fn say(&mut self, s: &str) {
        self.msg = Some((s.to_string(), 2.5));
    }

    fn random_name(&mut self) {
        self.seed = self.seed.wrapping_mul(1_103_515_245).wrapping_add(12_345 + (self.t * 1000.0) as u32);
        let taken: Vec<&str> = self.heroes.iter().map(|h| h.name.as_str()).collect();
        for k in 0..NAMES.len() {
            let n = NAMES[(self.seed as usize / 7 + k) % NAMES.len()];
            if !taken.contains(&n) {
                self.name = n.to_string();
                return;
            }
        }
        self.name = NAMES[self.seed as usize % NAMES.len()].to_string();
    }

    pub fn update(&mut self, inp: &Input) -> Option<MenuOut> {
        let dt = crate::game::DT;
        self.t += dt;
        if let Some((_, t)) = self.msg.as_mut() {
            *t -= dt;
            if *t <= 0.0 {
                self.msg = None;
            }
        }
        let prev = std::mem::replace(&mut self.prev, inp.clone());
        let edge = |now: f32, before: f32, neg: bool| if neg { now < -0.5 && before >= -0.5 } else { now > 0.5 && before <= 0.5 };
        let typing = self.stage == Stage::Name;
        let typed_now = !inp.typed.is_empty();
        let (up, down) = (edge(inp.move_y, prev.move_y, true), edge(inp.move_y, prev.move_y, false));
        let (left, right) = (edge(inp.move_x, prev.move_x, true), edge(inp.move_x, prev.move_x, false));
        // While naming, letter keys type (W/A/S/D, F, E... must not also act).
        let (up, down, left, right) = if typing && typed_now { (false, false, false, false) } else { (up, down, left, right) };
        let mut confirm = inp.confirm || (inp.cast && !prev.cast && !(typing && typed_now));
        let back = inp.cancel || (inp.run_toggle && !typing);
        let n = self.items().max(1);
        let horizontal = self.stage == Stage::Create;
        if (up && !horizontal) || (left && horizontal) {
            self.sel = (self.sel + n - 1) % n;
        }
        if (down && !horizontal) || (right && horizontal) {
            self.sel = (self.sel + 1) % n;
        }
        if let (Some((mx, my)), true) = (inp.mouse, inp.lmb && !prev.lmb) {
            // The topmost (last drawn) thing under the mouse wins: the delete prompt sits over the hero list.
            if let Some(&(.., k)) = self.rects.iter().rev().find(|&&(x, y, w, h, _)| mx >= x && mx < x + w && my >= y && my < y + h) {
                // Click to select, click the selected one again to choose (single click on buttons).
                let button = self.stage == Stage::Heroes && k >= self.heroes.len();
                if k == self.sel || button || !matches!(self.stage, Stage::Heroes | Stage::Create) {
                    confirm = true;
                }
                self.sel = k;
            }
        }
        match self.stage {
            Stage::Title => {
                if back {
                    return Some(MenuOut::Quit);
                }
                if confirm {
                    match self.sel {
                        0 => {
                            let s = if self.heroes.is_empty() { Stage::Create } else { Stage::Heroes };
                            self.go(s);
                        }
                        1 => self.go(Stage::Options),
                        _ => return Some(MenuOut::Quit),
                    }
                }
            }
            Stage::Heroes => {
                if back {
                    self.go(Stage::Title);
                    return None;
                }
                // The mouse wheel scrolls the list; keyboard / pad selection keeps the pick in view.
                let nh = self.heroes.len();
                let max_scroll = nh.saturating_sub(self.rows_fit);
                if inp.wheel > 0 {
                    self.scroll = self.scroll.saturating_sub(inp.wheel as usize);
                } else if inp.wheel < 0 {
                    self.scroll = (self.scroll + (-inp.wheel) as usize).min(max_scroll);
                } else if self.sel < nh {
                    if self.sel < self.scroll {
                        self.scroll = self.sel;
                    } else if self.sel >= self.scroll + self.rows_fit {
                        self.scroll = self.sel + 1 - self.rows_fit;
                    }
                }
                self.scroll = self.scroll.min(max_scroll);
                let k = self.sel;
                if k < self.heroes.len() {
                    self.hero_sel = k;
                }
                // X / Space / Delete / Backspace, or the DELETE HERO button: delete a hero (asks first).
                let del_key = (inp.cast2 && !prev.cast2) || (inp.backspace && !prev.backspace);
                let del_button = confirm && k == self.heroes.len() + 2;
                if (del_key || del_button) && !self.heroes.is_empty() {
                    let h = if k < self.heroes.len() { k } else { self.hero_sel.min(self.heroes.len() - 1) };
                    self.go(Stage::Delete(h));
                    self.sel = 1;
                    return None;
                }
                if del_button {
                    self.say("NO HEROES TO DELETE");
                    return None;
                }
                if confirm {
                    if k < self.heroes.len() {
                        return Some(MenuOut::Load(self.heroes[k].slot.clone()));
                    } else if k == self.heroes.len() {
                        if self.heroes.len() >= MAX_HEROES {
                            self.say("NO ROOM: DELETE A HERO FIRST");
                        } else {
                            self.go(Stage::Create);
                        }
                    } else {
                        self.go(Stage::Title);
                    }
                }
            }
            Stage::Delete(k) => {
                if back {
                    self.go(Stage::Heroes);
                    self.sel = k;
                    return None;
                }
                if confirm {
                    if self.sel == 0 {
                        crate::save::delete_hero(&self.heroes[k].slot);
                        self.heroes.remove(k);
                        self.say("HERO DELETED");
                    }
                    self.go(Stage::Heroes);
                    self.sel = k.min(self.heroes.len());
                }
            }
            Stage::Create => {
                if back {
                    let s = if self.heroes.is_empty() { Stage::Title } else { Stage::Heroes };
                    self.go(s);
                    return None;
                }
                if confirm {
                    if CLASSES[self.sel].5 {
                        self.class_sel = self.sel;
                        self.random_name();
                        self.go(Stage::Name);
                    } else {
                        self.say("THE INVENTOR IS COMING SOON");
                    }
                }
            }
            Stage::Name => {
                for c in inp.typed.chars() {
                    let c = c.to_ascii_uppercase();
                    if (c.is_ascii_alphabetic() || c == ' ' || c == '-') && self.name.len() < MAX_NAME {
                        self.name.push(c);
                    }
                }
                if inp.backspace {
                    self.name.pop();
                }
                if inp.cancel {
                    self.go(Stage::Create);
                    self.sel = self.class_sel;
                    return None;
                }
                // Pad Y / the button: a random name.
                if inp.potion_mp && !typed_now {
                    self.random_name();
                }
                let start = confirm && (self.sel == 0 || inp.confirm);
                if confirm && self.sel == 1 {
                    self.random_name();
                } else if confirm && self.sel == 2 && !inp.confirm {
                    self.go(Stage::Create);
                    self.sel = self.class_sel;
                } else if start {
                    let name = self.name.trim().to_string();
                    if name.is_empty() {
                        self.say("GIVE YOUR HERO A NAME");
                    } else {
                        let class = match self.class_sel {
                            1 => Class::Vampire,
                            2 => Class::Inventor,
                            3 => Class::Valkyrie,
                            4 => Class::Berserker,
                            5 => Class::Reaper,
                            6 => Class::Druid,
                            _ => Class::Sorceress,
                        };
                        return Some(MenuOut::New(class, name));
                    }
                }
            }
            Stage::Options => {
                if back {
                    self.go(Stage::Title);
                    self.sel = 1;
                    return None;
                }
                if confirm {
                    if self.sel == 0 {
                        self.music_on = !self.music_on;
                        return Some(MenuOut::ToggleMusic);
                    }
                    self.go(Stage::Title);
                    self.sel = 1;
                }
            }
        }
        None
    }

    // ------------------------------------------------------------------ drawing

    pub fn draw(&mut self, scr: &mut Screen, art: &Art) {
        let (w, h) = (scr.w, scr.px.len() as i32 / scr.w);
        self.rects.clear();
        // The painted background, scaled to cover the screen, darkened.
        scr.fill(0, 0, w, h, rgb(0x08060a));
        if let Some(bg) = art.item("title_bg") {
            let s = (w as f32 / bg.w as f32).max(h as f32 / bg.h as f32);
            let dim = if self.stage == Stage::Title { 0.25 } else { 0.6 };
            let (dw, dh) = ((bg.w as f32 * s) as i32, (bg.h as f32 * s) as i32);
            scr.blit_scaled(bg, w / 2 - dw / 2 + (bg.ax as f32 * s) as i32, h / 2 - dh / 2 + (bg.ay as f32 * s) as i32, s, Fx { tint: BLACK, tint_a: dim, ..Fx::default() });
        }
        // Drifting embers.
        for i in 0..60u32 {
            let h1 = (i.wrapping_mul(2_654_435_761) >> 8) as f32 / 16_777_216.0;
            let h2 = (i.wrapping_mul(40_503).wrapping_add(977) % 1000) as f32 / 1000.0;
            let x = (h1 * w as f32 + (self.t * 0.7 + h2 * 9.0).sin() * 10.0 + self.t * 6.0).rem_euclid(w as f32) as i32;
            let y = (h as f32 - (h2 * h as f32 * 2.0 + self.t * (14.0 + h1 * 20.0)).rem_euclid(h as f32)) as i32;
            let c = if h2 > 0.5 { rgb(0xff8030) } else { rgb(0xffc060) };
            scr.glow(x, y, 3.0 + h1 * 2.0, c, 0.6);
        }
        match self.stage {
            Stage::Title => self.draw_title(scr, w, h),
            Stage::Heroes | Stage::Delete(_) => self.draw_heroes(scr, art, w, h),
            Stage::Create => self.draw_create(scr, art, w, h),
            Stage::Name => self.draw_name(scr, art, w, h),
            Stage::Options => self.draw_options(scr, w, h),
        }
        if let Some((m, t)) = &self.msg {
            let c = mix(BLACK, rgb(0xffd080), t.min(1.0));
            scr.text(m, w / 2, h - 34, c, Align::Center, 1);
        }
    }

    fn logo(&self, scr: &mut Screen, w: i32, y: i32, sc: i32) {
        let pulse = (self.t * 1.3).sin() * 0.15 + 0.85;
        scr.glow(w / 2, y + 4 * sc, 140.0 * sc as f32 / 4.0, rgb(0xa02008), 0.5 * pulse);
        scr.text("ASHEN SANCTUM", w / 2 + sc, y + sc, BLACK, Align::Center, sc);
        scr.text("ASHEN SANCTUM", w / 2, y, rgb(0xffb040), Align::Center, sc);
    }

    fn button(&mut self, scr: &mut Screen, label: &str, cx: i32, y: i32, wdt: i32, k: usize, enabled: bool) {
        let on = self.sel == k;
        let x = cx - wdt / 2;
        scr.blend(x, y, wdt, 18, if on { rgb(0x5a2a08) } else { BLACK }, if on { 0.85 } else { 0.55 });
        let edge = if on { rgb(0xffb040) } else { rgb(0x5a4a38) };
        for (ex, ey, ew, eh) in [(x, y, wdt, 1), (x, y + 17, wdt, 1), (x, y, 1, 18), (x + wdt - 1, y, 1, 18)] {
            scr.fill(ex, ey, ew, eh, edge);
        }
        let col = if !enabled {
            rgb(0x6a5a4a)
        } else if on {
            rgb(0xffe0a0)
        } else {
            rgb(0xc0a880)
        };
        scr.text(label, cx, y + 5, col, Align::Center, 1);
        self.rects.push((x, y, wdt, 18, k));
    }

    fn draw_title(&mut self, scr: &mut Screen, w: i32, h: i32) {
        // Dark bands behind the logo and the menu so they read over the painting.
        for k in 0..60 {
            let a = 0.7 * (1.0 - (k as f32 - 30.0).abs() / 30.0);
            scr.blend(0, h / 4 - 40 + k, w, 1, BLACK, a);
        }
        scr.blend(w / 2 - 110, h / 2, 220, 92, BLACK, 0.45);
        self.logo(scr, w, h / 4 - 20, 4);
        scr.text("A TALE OF ASH AND ICE", w / 2, h / 4 + 22, rgb(0xc8a070), Align::Center, 1);
        let y0 = h / 2 + 10;
        for (k, label) in ["PLAY", "OPTIONS", "QUIT"].iter().enumerate() {
            self.button(scr, label, w / 2, y0 + k as i32 * 26, 150, k, true);
        }
        scr.text("UP / DOWN + ENTER (PAD: A)", w / 2, h - 14, rgb(0x7a6a5a), Align::Center, 1);
    }

    fn draw_heroes(&mut self, scr: &mut Screen, art: &Art, w: i32, h: i32) {
        scr.text("SELECT YOUR HERO", w / 2, 10, rgb(0xffd080), Align::Center, 2);
        let (lx, ly, lw) = (24, 40, w / 2 - 24);
        let row = 30;
        // The buttons sit in a fixed bar at the bottom; the list scrolls in the space above.
        let bar_y = h - 50;
        self.rows_fit = (((bar_y - 14 - ly) / (row + 2)).max(1)) as usize;
        let shown = self.scroll..(self.scroll + self.rows_fit).min(self.heroes.len());
        if self.scroll > 0 {
            scr.text("- MORE ABOVE -", lx + lw / 2, ly - 10, rgb(0x9a8a70), Align::Center, 1);
        }
        if shown.end < self.heroes.len() {
            scr.text("- MORE BELOW (SCROLL) -", lx + lw / 2, bar_y - 12, rgb(0x9a8a70), Align::Center, 1);
        }
        for (k, hero) in self.heroes.clone().iter().enumerate() {
            if !shown.contains(&k) {
                continue;
            }
            let y = ly + (k - self.scroll) as i32 * (row + 2);
            let on = self.sel == k;
            scr.blend(lx, y, lw, row, if on { rgb(0x3a1a08) } else { BLACK }, if on { 0.85 } else { 0.5 });
            if on {
                scr.fill(lx, y, 2, row, rgb(0xffb040));
            }
            let (pname, col) = class_look(hero.class);
            if let Some(p) = art.item(pname) {
                // A head-and-shoulders crop of the portrait (the top rows only).
                let s = 0.3;
                let fx = Fx { cut: ((row - 2) as f32 / s) as i32, ..Fx::default() };
                scr.blit_scaled(p, lx + 20, y + 1 + (p.ay as f32 * s) as i32, s, fx);
            }
            scr.text(&hero.name, lx + 42, y + 5, if on { rgb(0xffe0a0) } else { rgb(0xd0c0a0) }, Align::Left, 1);
            let act = ["ACT 1", "ACT 2", "ACT 3", "ACT 4"][hero.act.min(3)];
            let diff = crate::story::DIFFICULTIES[hero.difficulty.min(2) as usize];
            let line = format!("LEVEL {} {}  {}  {}", hero.clvl, hero.class.name(), act, diff);
            scr.text(&line, lx + 42, y + 17, col, Align::Left, 1);
            self.rects.push((lx, y, lw, row, k));
        }
        let nh = self.heroes.len();
        let bw = (lw - 8) / 3;
        self.button(scr, "NEW HERO", lx + bw / 2, bar_y, bw, nh, nh < MAX_HEROES);
        self.button(scr, "DELETE HERO", lx + bw + 4 + bw / 2, bar_y, bw, nh + 2, nh > 0);
        self.button(scr, "BACK", lx + 2 * (bw + 4) + bw / 2, bar_y, bw, nh + 1, true);
        // The selected hero, big, on the right.
        if let Some(hero) = self.heroes.get(self.sel.min(self.heroes.len().saturating_sub(1))).filter(|_| self.sel < nh) {
            let (pname, col) = class_look(hero.class);
            let (cx, top) = (w * 3 / 4, 40);
            if let Some(p) = art.item(pname) {
                let s = ((h - 110) as f32 / p.h as f32).min(1.4);
                scr.blit_scaled(p, cx, top + (p.h as f32 * s) as i32, s, Fx::default());
                scr.text(&hero.name, cx, top + (p.h as f32 * s) as i32 + 8, col, Align::Center, 2);
            }
        }
        if let Stage::Delete(k) = self.stage {
            let (bw, bh) = (300, 80);
            let (x, y) = (w / 2 - bw / 2, h / 2 - bh / 2);
            scr.fill(x, y, bw, bh, rgb(0x100808));
            for (ex, ey, ew, eh) in [(x, y, bw, 1), (x, y + bh - 1, bw, 1), (x, y, 1, bh), (x + bw - 1, y, 1, bh)] {
                scr.fill(ex, ey, ew, eh, rgb(0xc04030));
            }
            let name = self.heroes.get(k).map(|h| h.name.clone()).unwrap_or_default();
            scr.text(&format!("DELETE {name} FOREVER?"), w / 2, y + 12, rgb(0xffc0a0), Align::Center, 1);
            self.button(scr, "DELETE", w / 2 - 60, y + 40, 100, 0, true);
            self.button(scr, "KEEP", w / 2 + 60, y + 40, 100, 1, true);
        } else {
            scr.text("ENTER / A: PLAY    DEL / X: DELETE    ESC / B: BACK", w / 2, h - 14, rgb(0x7a6a5a), Align::Center, 1);
        }
    }

    fn draw_create(&mut self, scr: &mut Screen, art: &Art, w: i32, h: i32) {
        scr.text("CHOOSE YOUR HERO", w / 2, 10, rgb(0xffd080), Align::Center, 2);
        let n = CLASSES.len() as i32;
        let gap = 12;
        let pw = (w - 24 - gap * (n - 1)) / n;
        let ph = h - 66;
        for (k, (name, portrait, l1, l2, col, playable, sheet)) in CLASSES.iter().enumerate() {
            let x0 = 12 + k as i32 * (pw + gap);
            let y0 = 34;
            let on = self.sel == k;
            scr.blend(x0, y0, pw, ph, if on { rgb(0x1c1418) } else { BLACK }, 0.85);
            let edge = if on { rgb(*col) } else { rgb(0x3a3026) };
            for (ex, ey, ew, eh) in [(x0, y0, pw, 2), (x0, y0 + ph - 2, pw, 2), (x0, y0, 2, ph), (x0 + pw - 2, y0, 2, ph)] {
                scr.fill(ex, ey, ew, eh, edge);
            }
            let cx = x0 + pw / 2;
            if let Some(s) = art.item(portrait) {
                let scale = ((ph - 62) as f32 / s.h as f32).min((pw - 12) as f32 / s.w as f32);
                let dim = if !playable {
                    0.65
                } else if on {
                    0.0
                } else {
                    0.4
                };
                scr.blit_scaled(s, cx, y0 + 6 + (s.h as f32 * scale) as i32, scale, Fx { tint: BLACK, tint_a: dim, ..Fx::default() });
            } else {
                // No portrait yet: the in-game sprite, big.
                // (A stand-in sheet comes with its own tint; selected cards show it, others are dimmed.)
                let (ca, _, tint, tint_a) = art.char_art(sheet);
                let spr = ca.frame("idle", 0, 0.0);
                let fx = match (on, tint_a > 0.0) {
                    (true, _) => Fx { tint, tint_a, ..Fx::default() },
                    // Dimmed, but still in the stand-in's colours.
                    (false, true) => Fx { tint: crate::gfx::mix(tint, BLACK, 0.5), tint_a: (tint_a + 0.1).min(0.95), ..Fx::default() },
                    (false, false) => Fx { tint: BLACK, tint_a: 0.5, ..Fx::default() },
                };
                scr.blit_scaled(spr, cx, y0 + ph - 62, 2.5, fx);
            }
            let ncol = if !playable {
                rgb(0x6a5a4a)
            } else if on {
                rgb(*col)
            } else {
                rgb(0x9a8a78)
            };
            // Big names when they fit the card, small when six cards share the row.
            // One size for every card: big names only if they all fit.
            let sc = if CLASSES.iter().all(|c| crate::gfx::text_width(c.0, 2) <= pw - 6) { 2 } else { 1 };
            scr.text(name, cx, y0 + ph - 50 + (2 - sc) * 6, ncol, Align::Center, sc);
            if *playable {
                scr.text(l1, cx, y0 + ph - 28, rgb(0xb0a090), Align::Center, 1);
                scr.text(l2, cx, y0 + ph - 17, rgb(0xb0a090), Align::Center, 1);
            } else {
                scr.text("COMING SOON", cx, y0 + ph - 24, rgb(0x40c0b0), Align::Center, 1);
            }
            self.rects.push((x0, y0, pw, ph, k));
        }
        scr.text("LEFT / RIGHT TO CHOOSE, ENTER / A TO PICK, ESC / B: BACK", w / 2, h - 14, rgb(0x7a6a5a), Align::Center, 1);
    }

    fn draw_name(&mut self, scr: &mut Screen, art: &Art, w: i32, h: i32) {
        let (name, portrait, _, _, col, _, _) = CLASSES[self.class_sel];
        scr.text(&format!("NAME YOUR {name}"), w / 2, 10, rgb(0xffd080), Align::Center, 2);
        if let Some(p) = art.item(portrait) {
            let s = ((h - 150) as f32 / p.h as f32).min(1.2);
            scr.blit_scaled(p, w / 2, 34 + (p.h as f32 * s) as i32, s, Fx::default());
        }
        let by = h - 108;
        let bw = 200;
        scr.fill(w / 2 - bw / 2, by, bw, 22, rgb(0x100c0a));
        for (ex, ey, ew, eh) in [(w / 2 - bw / 2, by, bw, 1), (w / 2 - bw / 2, by + 21, bw, 1), (w / 2 - bw / 2, by, 1, 22), (w / 2 + bw / 2 - 1, by, 1, 22)] {
            scr.fill(ex, ey, ew, eh, rgb(col));
        }
        let caret = if (self.t * 2.0) as i32 % 2 == 0 { "_" } else { " " };
        scr.text(&format!("{}{caret}", self.name), w / 2, by + 7, rgb(0xffe0a0), Align::Center, 1);
        let y = by + 30;
        self.button(scr, "BEGIN", w / 2 - 110, y, 100, 0, true);
        self.button(scr, "RANDOM NAME", w / 2, y, 110, 1, true);
        self.button(scr, "BACK", w / 2 + 110, y, 100, 2, true);
        scr.text("TYPE A NAME, ENTER TO BEGIN.  PAD: Y RANDOM NAME, A BEGIN", w / 2, h - 14, rgb(0x7a6a5a), Align::Center, 1);
    }

    fn draw_options(&mut self, scr: &mut Screen, w: i32, _h: i32) {
        self.logo(scr, w, 20, 2);
        scr.text("OPTIONS", w / 2, 52, rgb(0xffd080), Align::Center, 2);
        let music = if self.music_on { "MUSIC: ON" } else { "MUSIC: OFF" };
        self.button(scr, music, w / 2, 80, 180, 0, true);
        self.button(scr, "BACK", w / 2, 106, 180, 1, true);
        let lines = [
            "CONTROLS",
            "MOVE: LEFT CLICK OR WASD / LEFT STICK      RUN: R / B",
            "SKILLS: LEFT / RIGHT CLICK, F / SPACE / A / X      SKILL TREE: K / HOLD SELECT",
            "INVENTORY: I / START      MAP: TAB / SELECT      POTIONS: Q / E / L1 / Y",
            "MUSIC ON / OFF IN GAME: N      ESC: SAVE AND RETURN TO THE HEROES",
        ];
        for (k, l) in lines.iter().enumerate() {
            let col = if k == 0 { rgb(0xd8b878) } else { rgb(0xa09080) };
            scr.text(l, w / 2, 150 + k as i32 * 14, col, Align::Center, 1);
        }
    }
}

fn class_look(c: Class) -> (&'static str, u32) {
    match c {
        Class::Sorceress => ("portrait_sorceress", rgb(0xff9040)),
        Class::Vampire => ("portrait_vampire", rgb(0xd04060)),
        Class::Inventor => ("portrait_inventor", rgb(0x40c0b0)),
        Class::Valkyrie => ("portrait_valkyrie", rgb(0x80d0ff)),
        Class::Berserker => ("portrait_berserker", rgb(0xd07040)),
        Class::Reaper => ("portrait_reaper", rgb(0x9ad8ff)),
        Class::Druid => ("portrait_druid", rgb(0x90d050)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(m: &mut Menu, f: impl Fn(&mut Input)) -> Option<MenuOut> {
        let mut i = Input::default();
        f(&mut i);
        let out = m.update(&i);
        m.update(&Input::default());
        out
    }

    #[test]
    fn play_new_hero_and_continue() {
        // No heroes yet: Play goes straight to class select.
        let mut m = Menu::new(vec![]);
        press(&mut m, |i| i.confirm = true);
        assert_eq!(m.stage, Stage::Create);
        // The vampire, named by typing.
        press(&mut m, |i| i.move_x = 1.0);
        press(&mut m, |i| i.confirm = true);
        assert_eq!(m.stage, Stage::Name);
        assert!(!m.name.is_empty(), "a random name to start");
        for _ in 0..20 {
            press(&mut m, |i| i.backspace = true);
        }
        press(&mut m, |i| i.typed = "carmilla".into());
        let out = press(&mut m, |i| i.confirm = true);
        assert_eq!(out, Some(MenuOut::New(Class::Vampire, "CARMILLA".into())));
        // With heroes: Play lists them; pick one to continue.
        let hero = HeroInfo { slot: "carmilla".into(), name: "CARMILLA".into(), class: Class::Vampire, clvl: 7, act: 0, difficulty: 0 };
        let mut m = Menu::new(vec![hero]);
        press(&mut m, |i| i.confirm = true);
        assert_eq!(m.stage, Stage::Heroes);
        assert_eq!(press(&mut m, |i| i.confirm = true), Some(MenuOut::Load("carmilla".into())));
        // Esc from the title quits.
        let mut m = Menu::new(vec![]);
        assert_eq!(press(&mut m, |i| i.cancel = true), Some(MenuOut::Quit));
    }

    #[test]
    fn the_hero_list_scrolls_and_deletes_with_one_click() {
        let heroes: Vec<HeroInfo> = (0..10)
            .map(|k| HeroInfo { slot: format!("zz_test_{k}"), name: format!("HERO {k}"), clvl: 1, class: Class::Sorceress, act: 0, difficulty: 0 })
            .collect();
        let mut m = Menu::new(heroes);
        m.go(Stage::Heroes);
        let mut scr = crate::gfx::Screen::new(crate::gfx::SH_WIDE);
        let art = crate::art::Art::load();
        m.draw(&mut scr, &art);
        assert!(m.rows_fit < 10, "not all ten fit");
        // Every button is on screen.
        let h = crate::gfx::SH_WIDE;
        for k in 10..13 {
            let r = m.rects.iter().find(|r| r.4 == k).expect("button drawn");
            assert!(r.1 + r.3 <= h, "button {k} is on screen");
        }
        // Scrolling with the wheel shows later heroes.
        m.update(&Input { wheel: -3, ..Input::default() });
        assert!(m.scroll > 0);
        // Highlight hero 2 with a click, then click DELETE HERO once: it asks.
        m.scroll = 0;
        m.draw(&mut scr, &art);
        let r = *m.rects.iter().find(|r| r.4 == 2).unwrap();
        m.update(&Input { mouse: Some((r.0 + 4, r.1 + 4)), lmb: true, ..Input::default() });
        m.update(&Input::default());
        m.draw(&mut scr, &art);
        let d = *m.rects.iter().find(|r| r.4 == 12).unwrap();
        m.update(&Input { mouse: Some((d.0 + 4, d.1 + 4)), lmb: true, ..Input::default() });
        assert_eq!(m.stage, Stage::Delete(2));
        // Click DELETE in the prompt (it sits over the list): the hero is gone and the list shrinks.
        m.update(&Input::default());
        m.draw(&mut scr, &art);
        let yes = *m.rects.iter().rev().find(|r| r.4 == 0).unwrap();
        m.update(&Input { mouse: Some((yes.0 + 4, yes.1 + 4)), lmb: true, ..Input::default() });
        assert_eq!(m.stage, Stage::Heroes);
        assert_eq!(m.heroes.len(), 9);
        assert!(m.heroes.iter().all(|h| h.name != "HERO 2"), "the right hero was deleted");
    }

    #[test]
    fn hero_info_reads_a_save() {
        let text = "name=MORWEN\nversion=1\nclvl=12\nclass=vampire\nact=1\ndifficulty=1\n";
        let h = crate::save::hero_info("morwen", text).unwrap();
        assert_eq!((h.name.as_str(), h.clvl, h.class, h.act, h.difficulty), ("MORWEN", 12, Class::Vampire, 1, 1));
    }
}

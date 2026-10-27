//! The login server's screens (PLAY ONLINE, then USE A SERVER): the address and the server's key, logging in
//! or registering, your heroes on the server, making one, and the games to join or create. It holds the
//! connection (net::client::Lobby); joining a game hands the connection to online.rs.
//!
//! Requests block the frame they're made in (a few hundred milliseconds at worst, Argon2 on the server).
use crate::art::Art;
use crate::game::Input;
use crate::gfx::{rgb, Align, Fx, Screen, BLACK};
use crate::net::client::{JoinError, Link, Lobby};
use crate::net::proto::{FromServer, GameInfo, ServerHero, ToServer};

#[derive(Clone, Copy, PartialEq, Debug)]
enum Screen_ {
    Address,
    Trust,
    Login,
    Heroes,
    NewHero,
    Delete,
    Games,
    Create,
    GamePassword,
}

/// What the screens want from the game.
pub enum LobbyOut {
    /// Back to the title.
    Back,
    /// In a game: play it.
    Play(Link),
}

pub struct LobbyUi {
    screen: Screen_,
    sel: usize,
    t: f32,
    prev: Input,
    msg: Option<(String, f32)>,
    rects: Vec<(i32, i32, i32, i32, usize)>,
    lobby: Option<Lobby>,
    // What's been typed.
    addr: String,
    name: String,
    password: String,
    invite: String,
    /// Remembered (in memory only) to log back in after a game.
    logged_in: bool,
    key: Option<[u8; 32]>,
    heroes: Vec<ServerHero>,
    hero: usize,
    games: Vec<GameInfo>,
    game: usize,
    // A new hero.
    class: usize,
    hero_name: String,
    hardcore: bool,
    // A new game.
    game_name: String,
    game_pw: String,
    /// When the game list was last fetched (it refreshes itself).
    fetched: f32,
}

/// Text fields: which screen item each one is.
fn field(s: Screen_, sel: usize, invite: bool) -> Option<usize> {
    match (s, sel) {
        (Screen_::Address, 0) => Some(0),
        (Screen_::Login, 0) | (Screen_::Login, 1) => Some(sel),
        (Screen_::Login, 2) if invite => Some(2),
        (Screen_::NewHero, 1) => Some(3),
        (Screen_::Create, 0) | (Screen_::Create, 1) => Some(4 + sel),
        (Screen_::GamePassword, 0) => Some(6),
        _ => None,
    }
}

impl LobbyUi {
    pub fn new(addr: String) -> LobbyUi {
        LobbyUi {
            screen: Screen_::Address,
            sel: 0,
            t: 0.0,
            prev: Input::default(),
            msg: None,
            rects: vec![],
            lobby: None,
            addr,
            name: String::new(),
            password: String::new(),
            invite: String::new(),
            logged_in: false,
            key: None,
            heroes: vec![],
            hero: 0,
            games: vec![],
            game: 0,
            class: 0,
            hero_name: String::new(),
            hardcore: false,
            game_name: String::new(),
            game_pw: String::new(),
            fetched: 0.0,
        }
    }

    fn say(&mut self, s: &str) {
        self.msg = Some((s.to_string(), 3.0));
    }

    fn go(&mut self, s: Screen_) {
        self.screen = s;
        self.sel = 0;
    }

    fn invite_field(&self) -> bool {
        self.lobby.as_ref().map_or(false, |l| l.invite)
    }

    fn items(&self) -> usize {
        match self.screen {
            Screen_::Address => 3,
            Screen_::Trust => 2,
            Screen_::Login => 5 + self.invite_field() as usize,
            Screen_::Heroes => self.heroes.len() + 3,
            Screen_::NewHero => 5,
            Screen_::Delete => 2,
            Screen_::Games => self.games.len() + 3,
            Screen_::Create => 4,
            Screen_::GamePassword => 3,
        }
    }

    fn text_mut(&mut self, f: usize) -> &mut String {
        match f {
            0 => &mut self.addr,
            1 => &mut self.name,
            2 => &mut self.password,
            3 => &mut self.hero_name,
            4 => &mut self.game_name,
            5 => &mut self.game_pw,
            _ => &mut self.game_pw,
        }
    }

    /// After a game ends: back to the hero list, logging in again (the game had the connection).
    pub fn back_from_game(&mut self) {
        self.lobby = None;
        if self.logged_in && self.connect() && self.log_in(false) {
            self.go(Screen_::Heroes);
        } else {
            self.go(Screen_::Address);
        }
    }

    fn ask(&mut self, m: &ToServer) -> Option<FromServer> {
        let r = self.lobby.as_mut()?.ask(m);
        match r {
            Ok(FromServer::Refused(why)) => {
                self.say(&why);
                None
            }
            Ok(a) => Some(a),
            Err(why) => {
                self.say(&why);
                self.lobby = None;
                self.go(Screen_::Address);
                None
            }
        }
    }

    /// Connects (asking about the key the first time). True when connected.
    fn connect(&mut self) -> bool {
        let dir = crate::save::dir();
        match crate::net::client::lobby(&self.addr, &dir) {
            Ok(l) => {
                self.lobby = Some(l);
                true
            }
            Err(JoinError::Unknown(k)) => {
                self.key = Some(k);
                self.go(Screen_::Trust);
                false
            }
            Err(JoinError::Changed(_)) => {
                self.say("THE SERVER'S KEY HAS CHANGED: NOT CONNECTING");
                false
            }
            Err(JoinError::Refused(w)) | Err(JoinError::Unreachable(w)) => {
                self.say(&format!("COULDN'T CONNECT: {w}"));
                false
            }
        }
    }

    fn log_in(&mut self, register: bool) -> bool {
        let m = if register {
            ToServer::Register { name: self.name.clone(), password: self.password.clone(), invite: self.invite.clone() }
        } else {
            ToServer::Login { name: self.name.clone(), password: self.password.clone() }
        };
        match self.ask(&m) {
            Some(FromServer::LoggedIn { .. }) => {
                self.logged_in = true;
                self.refresh_heroes();
                true
            }
            _ => false,
        }
    }

    fn refresh_heroes(&mut self) {
        if let Some(FromServer::Heroes(h)) = self.ask(&ToServer::Heroes) {
            self.heroes = h;
            self.heroes.truncate(crate::server::accounts::HERO_SLOTS as usize);
            self.hero = self.hero.min(self.heroes.len().saturating_sub(1));
        }
    }

    fn refresh_games(&mut self) {
        self.fetched = self.t;
        if let Some(FromServer::Games(g)) = self.ask(&ToServer::Games) {
            self.games = g;
            self.games.truncate(64);
        }
    }

    fn enter(&mut self, m: ToServer) -> Option<LobbyOut> {
        let lobby = self.lobby.take()?;
        match lobby.enter(&m) {
            Ok(link) => Some(LobbyOut::Play(link)),
            Err((back, why)) => {
                self.say(&why);
                self.lobby = back;
                if self.lobby.is_none() {
                    self.go(Screen_::Address);
                }
                None
            }
        }
    }

    pub fn update(&mut self, inp: &Input) -> Option<LobbyOut> {
        let dt = crate::game::DT;
        self.t += dt;
        if let Some((_, t)) = self.msg.as_mut() {
            *t -= dt;
            if *t <= 0.0 {
                self.msg = None;
            }
        }
        if let Some(l) = self.lobby.as_mut() {
            if !l.keep_alive() {
                self.lobby = None;
                self.say("LOST THE CONNECTION");
                self.go(Screen_::Address);
            }
        }
        if self.screen == Screen_::Games && self.t - self.fetched > 5.0 {
            self.refresh_games();
        }
        let prev = std::mem::replace(&mut self.prev, inp.clone());
        let typing = field(self.screen, self.sel, self.invite_field()).is_some() && !inp.typed.is_empty();
        let edge = |now: f32, before: f32, neg: bool| if neg { now < -0.5 && before >= -0.5 } else { now > 0.5 && before <= 0.5 };
        let (up, down) = if typing { (false, false) } else { (edge(inp.move_y, prev.move_y, true), edge(inp.move_y, prev.move_y, false)) };
        let (left, right) = if typing { (false, false) } else { (edge(inp.move_x, prev.move_x, true), edge(inp.move_x, prev.move_x, false)) };
        let n = self.items().max(1);
        if up {
            self.sel = (self.sel + n - 1) % n;
        }
        if down {
            self.sel = (self.sel + 1) % n;
        }
        let mut confirm = inp.confirm || (inp.cast && !prev.cast && !typing);
        if let (Some((mx, my)), true) = (inp.mouse, inp.lmb && !prev.lmb) {
            if let Some(&(.., k)) = self.rects.iter().rev().find(|&&(x, y, w, h, _)| mx >= x && mx < x + w && my >= y && my < y + h) {
                self.sel = k;
                confirm = true;
            }
        }
        // Typing into the field picked.
        if let Some(f) = field(self.screen, self.sel, self.invite_field()) {
            let (max, ok): (usize, fn(char) -> bool) = match f {
                0 => (64, |c| c.is_ascii_alphanumeric() || ".:-[]".contains(c)),
                1 => (16, |c| c.is_ascii_alphanumeric() || c == '_' || c == '-'),
                3 => (12, |c| c.is_ascii_alphabetic() || c == ' ' || c == '-' || c == '\''),
                4 => (24, |c| c.is_ascii_alphanumeric() || " '-!".contains(c)),
                _ => (128, |c| c.is_ascii_graphic() || c == ' '),
            };
            let text = self.text_mut(f);
            for c in inp.typed.chars() {
                if ok(c) && text.chars().count() < max {
                    text.push(if f == 3 || f == 4 { c.to_ascii_uppercase() } else { c });
                }
            }
            if inp.backspace {
                text.pop();
            }
            // Enter in a field moves on to the next item.
            if inp.confirm {
                self.sel = (self.sel + 1) % n;
                return None;
            }
        }
        let back = inp.cancel || (inp.back && !typing);
        match self.screen {
            Screen_::Address => {
                if back || (confirm && self.sel == 2) {
                    return Some(LobbyOut::Back);
                }
                if confirm && self.sel == 1 {
                    if self.addr.is_empty() {
                        self.say("TYPE THE SERVER'S ADDRESS");
                    } else if self.connect() {
                        self.go(Screen_::Login);
                    }
                }
            }
            Screen_::Trust => {
                if back || (confirm && self.sel == 1) {
                    self.go(Screen_::Address);
                } else if confirm {
                    if let Some(k) = self.key.take() {
                        let _ = crate::net::tls::trust(&crate::save::dir(), &crate::net::tls::host_key(&self.addr), &k);
                        if self.connect() {
                            self.go(Screen_::Login);
                        }
                    }
                }
            }
            Screen_::Login => {
                let base = 2 + self.invite_field() as usize;
                if back || (confirm && self.sel == base + 2) {
                    self.lobby = None;
                    self.go(Screen_::Address);
                } else if confirm && (self.sel == base || self.sel == base + 1) {
                    if self.log_in(self.sel == base + 1) {
                        self.go(Screen_::Heroes);
                    }
                }
            }
            Screen_::Heroes => {
                let nh = self.heroes.len();
                if back || (confirm && self.sel == nh + 2) {
                    self.lobby = None;
                    self.logged_in = false;
                    self.password.clear();
                    self.go(Screen_::Address);
                } else if confirm && self.sel < nh {
                    self.hero = self.sel;
                    if self.heroes[self.hero].fallen {
                        self.say("THAT HERO HAS FALLEN");
                    } else {
                        self.refresh_games();
                        self.go(Screen_::Games);
                    }
                } else if confirm && self.sel == nh {
                    self.hero_name.clear();
                    self.go(Screen_::NewHero);
                } else if confirm && self.sel == nh + 1 {
                    if nh == 0 {
                        self.say("NO HERO TO DELETE");
                    } else {
                        self.go(Screen_::Delete);
                    }
                } else if self.sel < nh {
                    self.hero = self.sel;
                }
            }
            Screen_::Delete => {
                if back || (confirm && self.sel == 1) {
                    self.go(Screen_::Heroes);
                } else if confirm {
                    if let Some(h) = self.heroes.get(self.hero) {
                        let slot = h.slot;
                        if let Some(FromServer::Heroes(mut list)) = self.ask(&ToServer::DeleteHero { slot }) {
                            list.truncate(crate::server::accounts::HERO_SLOTS as usize);
                            self.heroes = list;
                        }
                    }
                    self.hero = 0;
                    self.go(Screen_::Heroes);
                }
            }
            Screen_::NewHero => {
                let nc = crate::skills::ALL_CLASSES.len();
                if self.sel == 0 && (left || right) {
                    self.class = if left { (self.class + nc - 1) % nc } else { (self.class + 1) % nc };
                }
                if back || (confirm && self.sel == 4) {
                    self.go(Screen_::Heroes);
                } else if confirm && self.sel == 0 {
                    self.class = (self.class + 1) % nc;
                } else if confirm && self.sel == 2 {
                    self.hardcore = !self.hardcore;
                } else if confirm && self.sel == 3 {
                    if self.hero_name.trim().is_empty() {
                        self.say("GIVE YOUR HERO A NAME");
                    } else if let Some(FromServer::Heroes(mut list)) = self.ask(&ToServer::NewHero { class: self.class as u8, name: self.hero_name.clone(), hardcore: self.hardcore }) {
                        list.truncate(crate::server::accounts::HERO_SLOTS as usize);
                        self.heroes = list;
                        self.go(Screen_::Heroes);
                    }
                }
            }
            Screen_::Games => {
                let ng = self.games.len();
                if back || (confirm && self.sel == ng + 2) {
                    self.go(Screen_::Heroes);
                } else if confirm && self.sel < ng {
                    self.game = self.sel;
                    if self.games[self.game].locked {
                        self.game_pw.clear();
                        self.go(Screen_::GamePassword);
                    } else {
                        let (id, slot) = (self.games[self.game].id, self.heroes[self.hero].slot);
                        return self.enter(ToServer::JoinGame { game: id, password: String::new(), hero: slot });
                    }
                } else if confirm && self.sel == ng {
                    self.game_name = format!("{}'S GAME", self.heroes.get(self.hero).map_or("A", |h| h.name.as_str()));
                    self.game_name.truncate(24);
                    self.game_pw.clear();
                    self.go(Screen_::Create);
                } else if confirm && self.sel == ng + 1 {
                    self.refresh_games();
                }
            }
            Screen_::Create => {
                if back || (confirm && self.sel == 3) {
                    self.go(Screen_::Games);
                } else if confirm && self.sel == 2 {
                    let slot = self.heroes.get(self.hero).map_or(0, |h| h.slot);
                    return self.enter(ToServer::CreateGame { name: self.game_name.clone(), password: self.game_pw.clone(), hero: slot });
                }
            }
            Screen_::GamePassword => {
                if back || (confirm && self.sel == 2) {
                    self.go(Screen_::Games);
                } else if confirm && self.sel == 1 {
                    let (id, slot) = (self.games.get(self.game).map_or(0, |g| g.id), self.heroes.get(self.hero).map_or(0, |h| h.slot));
                    return self.enter(ToServer::JoinGame { game: id, password: self.game_pw.clone(), hero: slot });
                }
            }
        }
        None
    }

    fn button(&mut self, scr: &mut Screen, label: &str, cx: i32, y: i32, w: i32, k: usize) {
        let on = self.sel == k;
        let x = cx - w / 2;
        scr.blend(x, y, w, 18, if on { rgb(0x5a2a08) } else { BLACK }, if on { 0.85 } else { 0.55 });
        let edge = if on { rgb(0xffb040) } else { rgb(0x5a4a38) };
        for (ex, ey, ew, eh) in [(x, y, w, 1), (x, y + 17, w, 1), (x, y, 1, 18), (x + w - 1, y, 1, 18)] {
            scr.fill(ex, ey, ew, eh, edge);
        }
        scr.text(label, cx, y + 5, if on { rgb(0xffe0a0) } else { rgb(0xc0a880) }, Align::Center, 1);
        self.rects.push((x, y, w, 18, k));
    }

    pub fn draw(&mut self, scr: &mut Screen, art: &Art) {
        let (w, h) = (scr.w, scr.px.len() as i32 / scr.w);
        self.rects.clear();
        scr.fill(0, 0, w, h, rgb(0x08060a));
        if let Some(bg) = art.item("title_bg") {
            let s = (w as f32 / bg.w as f32).max(h as f32 / bg.h as f32);
            let (dw, dh) = ((bg.w as f32 * s) as i32, (bg.h as f32 * s) as i32);
            scr.blit_scaled(bg, w / 2 - dw / 2 + (bg.ax as f32 * s) as i32, h / 2 - dh / 2 + (bg.ay as f32 * s) as i32, s, Fx { tint: BLACK, tint_a: 0.6, ..Fx::default() });
        }
        let server = self.lobby.as_ref().map(|l| l.server.clone()).unwrap_or_else(|| "A SERVER".into());
        let caret = if (self.t * 2.0) as i32 % 2 == 0 { "_" } else { " " };
        let cur = |s: &Self, k: usize| if s.sel == k && field(s.screen, k, s.invite_field()).is_some() { caret } else { "" };
        let title = match self.screen {
            Screen_::Address | Screen_::Trust => "PLAY ON A SERVER".to_string(),
            Screen_::Login => server.clone(),
            Screen_::Heroes | Screen_::Delete | Screen_::NewHero => "YOUR HEROES".into(),
            _ => "GAMES".into(),
        };
        scr.text(&title, w / 2, 16, rgb(0xffd080), Align::Center, 2);
        let cx = w / 2;
        match self.screen {
            Screen_::Address => {
                let a = format!("ADDRESS: {}{}", self.addr.to_uppercase(), cur(self, 0));
                self.button(scr, &a, cx, 60, 320, 0);
                self.button(scr, "CONNECT", cx, 86, 320, 1);
                self.button(scr, "BACK", cx, 112, 320, 2);
                scr.text("THE SERVER'S ADDRESS, LIKE 192.168.1.50 OR GAME.EXAMPLE.COM (:PORT IF NOT 4747)", cx, 146, rgb(0xa09080), Align::Center, 1);
            }
            Screen_::Trust => {
                let key = self.key.map(|k| crate::net::tls::show(&k)).unwrap_or_default();
                scr.text("A SERVER YOU HAVEN'T USED BEFORE. ITS KEY:", cx, 60, rgb(0xd8c090), Align::Center, 1);
                scr.text(&key, cx, 78, rgb(0xffe0a0), Align::Center, 2);
                scr.text("ASK WHOEVER RUNS IT WHAT THE KEY IS. GO ON ONLY IF IT'S THE SAME.", cx, 104, rgb(0xa09080), Align::Center, 1);
                self.button(scr, "IT MATCHES: TRUST IT", cx, 126, 260, 0);
                self.button(scr, "BACK", cx, 152, 260, 1);
            }
            Screen_::Login => {
                let inv = self.invite_field();
                let n = format!("ACCOUNT: {}{}", self.name, cur(self, 0));
                let p = format!("PASSWORD: {}{}", "*".repeat(self.password.chars().count()), cur(self, 1));
                self.button(scr, &n, cx, 54, 320, 0);
                self.button(scr, &p, cx, 78, 320, 1);
                let mut y = 102;
                if inv {
                    let i = format!("INVITE CODE (NEW ACCOUNTS): {}{}", "*".repeat(self.invite.chars().count()), cur(self, 2));
                    self.button(scr, &i, cx, y, 320, 2);
                    y += 24;
                }
                let base = 2 + inv as usize;
                self.button(scr, "LOG IN", cx, y + 6, 320, base);
                self.button(scr, "REGISTER A NEW ACCOUNT", cx, y + 30, 320, base + 1);
                self.button(scr, "BACK", cx, y + 54, 320, base + 2);
                scr.text("NAMES: 3-16 LETTERS, NUMBERS, _ OR -.  PASSWORDS: AT LEAST 8 CHARACTERS.", cx, y + 86, rgb(0xa09080), Align::Center, 1);
            }
            Screen_::Heroes | Screen_::Delete => {
                let heroes = self.heroes.clone();
                for (k, hh) in heroes.iter().enumerate() {
                    let class = crate::skills::ALL_CLASSES.get(hh.class as usize).map_or("?", |c| c.name());
                    let tag = if hh.fallen { "  FALLEN" } else if hh.hardcore { "  HARDCORE" } else { "" };
                    let label = format!("{}  LEVEL {} {}  ACT {}{tag}", hh.name, hh.clvl, class, hh.act.saturating_add(1));
                    self.button(scr, &label, cx, 46 + k as i32 * 22, 380, k);
                }
                let y = 52 + heroes.len() as i32 * 22;
                let nh = heroes.len();
                self.button(scr, "NEW HERO", cx - 130, y, 120, nh);
                self.button(scr, "DELETE HERO", cx, y, 120, nh + 1);
                self.button(scr, "LOG OUT", cx + 130, y, 120, nh + 2);
                if self.screen == Screen_::Delete {
                    self.rects.clear();
                    let name = heroes.get(self.hero).map_or(String::new(), |h| h.name.clone());
                    scr.blend(cx - 170, 90, 340, 90, BLACK, 0.9);
                    scr.text(&format!("DELETE {name}? (A COPY IS KEPT ON THE SERVER)"), cx, 104, rgb(0xffd080), Align::Center, 1);
                    self.button(scr, "DELETE", cx - 70, 130, 120, 0);
                    self.button(scr, "KEEP", cx + 70, 130, 120, 1);
                } else {
                    scr.text("PICK A HERO TO SEE THE GAMES", cx, y + 30, rgb(0xa09080), Align::Center, 1);
                }
            }
            Screen_::NewHero => {
                let c = crate::skills::ALL_CLASSES[self.class % crate::skills::ALL_CLASSES.len()];
                self.button(scr, &format!("< {} >", c.name()), cx, 54, 300, 0);
                let n = format!("NAME: {}{}", self.hero_name, cur(self, 1));
                self.button(scr, &n, cx, 78, 300, 1);
                self.button(scr, if self.hardcore { "HARDCORE: ON (ONE LIFE)" } else { "HARDCORE: OFF" }, cx, 102, 300, 2);
                self.button(scr, "CREATE", cx, 130, 300, 3);
                self.button(scr, "BACK", cx, 154, 300, 4);
            }
            Screen_::Games => {
                let hero = self.heroes.get(self.hero).map_or(String::new(), |h| h.name.clone());
                scr.text(&format!("PLAYING AS {hero}"), cx, 38, rgb(0xd8c090), Align::Center, 1);
                let games = self.games.clone();
                for (k, g) in games.iter().enumerate().take(8) {
                    let label = format!("{}  {}/8  ACT {}{}{}", g.name, g.players, g.act.saturating_add(1), if g.locked { "  LOCKED" } else { "" }, if g.hardcore { "  HARDCORE" } else { "" });
                    self.button(scr, &label, cx, 54 + k as i32 * 22, 380, k);
                }
                if games.is_empty() {
                    scr.text("NO GAMES YET: CREATE ONE", cx, 60, rgb(0xa09080), Align::Center, 1);
                }
                let y = 64 + games.len().min(8) as i32 * 22;
                let ng = games.len();
                self.button(scr, "CREATE GAME", cx - 130, y, 120, ng);
                self.button(scr, "REFRESH", cx, y, 120, ng + 1);
                self.button(scr, "BACK", cx + 130, y, 120, ng + 2);
            }
            Screen_::Create => {
                let n = format!("GAME NAME: {}{}", self.game_name, cur(self, 0));
                let p = if self.game_pw.is_empty() && self.sel != 1 { "PASSWORD: (NONE)".to_string() } else { format!("PASSWORD: {}{}", "*".repeat(self.game_pw.chars().count()), cur(self, 1)) };
                self.button(scr, &n, cx, 60, 320, 0);
                self.button(scr, &p, cx, 84, 320, 1);
                self.button(scr, "CREATE AND PLAY", cx, 112, 320, 2);
                self.button(scr, "BACK", cx, 136, 320, 3);
                scr.text("THE WORLD IS YOUR HERO'S: ITS ACT AND QUESTS. OTHERS JOIN IT.", cx, 168, rgb(0xa09080), Align::Center, 1);
            }
            Screen_::GamePassword => {
                let name = self.games.get(self.game).map_or(String::new(), |g| g.name.clone());
                scr.text(&format!("{name} IS LOCKED"), cx, 50, rgb(0xd8c090), Align::Center, 1);
                let p = format!("PASSWORD: {}{}", "*".repeat(self.game_pw.chars().count()), cur(self, 0));
                self.button(scr, &p, cx, 66, 320, 0);
                self.button(scr, "JOIN", cx, 92, 320, 1);
                self.button(scr, "BACK", cx, 116, 320, 2);
            }
        }
        if let Some((m, _)) = &self.msg {
            scr.text(m, w / 2, h - 30, rgb(0xffb070), Align::Center, 1);
        }
        scr.text("ENTER / A: CHOOSE    ESC / B: BACK", w / 2, h - 14, rgb(0x7a6a5a), Align::Center, 1);
    }

    /// For the snapshots: a screen filled in.
    #[allow(dead_code)]
    pub fn debug_screen(&mut self, which: &str) {
        match which {
            "login" => {
                self.screen = Screen_::Login;
                self.name = "Rowan".into();
                self.password = "secret password".into();
                self.sel = 2;
            }
            "heroes" => {
                self.screen = Screen_::Heroes;
                self.heroes = vec![
                    ServerHero { slot: 0, name: "ROWAN".into(), class: 5, clvl: 23, act: 2, hardcore: false, fallen: false },
                    ServerHero { slot: 1, name: "ISOLDE".into(), class: 0, clvl: 41, act: 4, hardcore: true, fallen: false },
                ];
            }
            _ => {
                self.screen = Screen_::Games;
                self.heroes = vec![ServerHero { slot: 0, name: "ROWAN".into(), class: 5, clvl: 23, act: 2, hardcore: false, fallen: false }];
                self.games = vec![
                    GameInfo { id: 1, name: "CRYPT RUN".into(), players: 3, locked: false, hardcore: false, act: 0 },
                    GameInfo { id: 2, name: "BOSS RUSH".into(), players: 7, locked: true, hardcore: false, act: 3 },
                ];
                self.fetched = 1e9;
            }
        }
    }
}

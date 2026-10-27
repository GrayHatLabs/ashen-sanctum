//! One game on the login server: a world on its own thread, 60 ticks a second, every hero a player over
//! the network. The first to join leads (hero 0); if the leader leaves, the next takes over. Heroes are
//! the accounts' own: loaded when they join, saved (just the hero, never the world: online::merge_hero)
//! every minute and when they leave.
#![forbid(unsafe_code)]

use super::accounts::Accounts;
use crate::game::{Floater, Game, Input, Sfx, State};
use crate::net::proto::{NetInput, ToClient, ToHost};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const SNAP_EVERY: u32 = 3;
const SAVE_EVERY: u32 = 60 * 60;
const MAX_STRIKES: u32 = 30;
pub const MAX_PLAYERS: usize = 8;
/// A fallen leader gets up again in town after this long (softcore).
const LEADER_RESPAWN: f32 = 6.0;

/// A player joining (from its connection thread): who, which hero, where its frames go, and where the answer
/// (its number in the party and the leader's name, or why not) goes.
pub struct JoinReq {
    pub peer: u32,
    pub account: u64,
    pub slot: u8,
    pub name: String,
    pub hero: String,
    pub out: SyncSender<Vec<u8>>,
    pub reply: SyncSender<Result<(u8, String), String>>,
}

pub enum RoomEvent {
    Join(JoinReq),
    Input { peer: u32, input: NetInput },
    Act { peer: u32, act: ToHost },
    Chat { peer: u32, text: String },
    Gone { peer: u32, why: String },
}

/// What the lobby knows of a room.
pub struct RoomHandle {
    pub id: u32,
    pub name: String,
    /// The game password's hash (None: open).
    pub password: Option<[u8; 32]>,
    pub hardcore: bool,
    pub act: u8,
    pub players: Arc<AtomicU8>,
    /// Which heroes are in it (account, slot): one hero can't be in two places.
    pub heroes: Arc<Mutex<Vec<(u64, u8)>>>,
    pub tx: std::sync::mpsc::Sender<RoomEvent>,
    pub ended: Arc<AtomicBool>,
}

struct Peer {
    peer: u32,
    account: u64,
    slot: u8,
    name: String,
    out: SyncSender<Vec<u8>>,
    strikes: u32,
    chat: (f32, Instant),
    events: (Vec<Sfx>, Vec<Floater>),
    /// The hero as stored when it joined (the world part of it is kept when saving: merge_hero).
    stored: String,
}

struct Room {
    g: Game,
    peers: Vec<Peer>,
    accounts: Arc<Mutex<Accounts>>,
    heroes: Arc<Mutex<Vec<(u64, u8)>>>,
    players: Arc<AtomicU8>,
    sent_level: Option<crate::world::LevelId>,
    /// The leader's held input and aim.
    lead: (Input, Option<(f32, f32)>),
}

/// Merges a checked input into the held one: one-shot buttons not yet used stay pressed until they are.
pub fn hold_input(held: &mut Input, new: Input) {
    let keep = (held.potion_hp, held.potion_mp, held.eat, held.scroll, held.cycle, held.run_toggle, held.slot);
    *held = new;
    held.potion_hp |= keep.0;
    held.potion_mp |= keep.1;
    held.eat |= keep.2;
    held.scroll |= keep.3;
    held.cycle |= keep.4;
    held.run_toggle |= keep.5;
    held.slot = held.slot.or(keep.6);
}

impl Room {
    fn send(&self, i: usize, m: &ToClient) -> bool {
        let Some(p) = self.peers.get(i) else { return false };
        match crate::net::proto::encode(m) {
            Some(b) => !matches!(p.out.try_send(b), Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_))),
            None => true,
        }
    }

    fn tell_all(&self, m: &ToClient) {
        for i in 0..self.peers.len() {
            let _ = self.send(i, m);
        }
    }

    fn index(&self, peer: u32) -> Option<usize> {
        self.peers.iter().position(|p| p.peer == peer)
    }

    /// The hero of player `i`, as save text.
    fn hero_text(&mut self, i: usize) -> String {
        if i == 0 {
            crate::save::to_text(&self.g)
        } else {
            self.g.as_hero(i - 1, |g| crate::save::to_text(g))
        }
    }

    /// Saves player `i`'s hero to its account (only the hero; its own world stays as stored).
    fn save(&mut self, i: usize) {
        let text = self.hero_text(i);
        let p = &self.peers[i];
        if let Some(merged) = crate::online::merge_hero(&p.stored, &text) {
            let a = self.accounts.lock().unwrap_or_else(|e| e.into_inner());
            let _ = a.save_hero(p.account, p.slot, &merged);
        }
    }

    fn join(&mut self, r: JoinReq) {
        let first = self.peers.is_empty();
        let joined = if first {
            // The world is the first hero's: their act and quests.
            let mut g = Game::new(self.g.world_seed(), crate::gfx::SH_WIDE);
            if !crate::save::apply(&mut g, &r.hero) || !crate::online::hero_ok(&g.p) {
                Err("YOUR HERO COULDN'T BE READ")
            } else {
                g.hero_name = r.name.clone();
                self.g = g;
                Ok(0usize)
            }
        } else if self.peers.len() >= MAX_PLAYERS {
            Err("THE GAME IS FULL")
        } else {
            self.g.add_ally_from_save(&r.hero, &r.name).map(|k| k + 1)
        };
        match joined {
            Ok(i) => {
                let leader = if first { r.name.clone() } else { self.peers[0].name.clone() };
                let _ = r.reply.send(Ok((i as u8, leader)));
                self.peers.push(Peer { peer: r.peer, account: r.account, slot: r.slot, name: r.name.clone(), out: r.out, strikes: 0, chat: (5.0, Instant::now()), events: (vec![], vec![]), stored: r.hero });
                self.heroes.lock().unwrap_or_else(|e| e.into_inner()).push((r.account, r.slot));
                if let Some(lv) = self.g.net_level() {
                    let _ = self.send(i, &ToClient::Level(lv));
                }
                self.tell_all(&ToClient::Chat { from: String::new(), text: format!("{} JOINED", r.name) });
                self.rescale();
            }
            Err(why) => {
                let _ = r.reply.send(Err(why.to_string()));
            }
        }
    }

    fn rescale(&self) {
        self.players.store(self.peers.len() as u8, Ordering::Relaxed);
        crate::binds::set_players_quiet(self.peers.len().max(1) as u8);
    }

    /// Player `i` leaves: its hero is saved, and (the leader) the next player leads.
    fn leave(&mut self, i: usize, why: &str) {
        if i >= self.peers.len() {
            return;
        }
        self.save(i);
        if i == 0 {
            if self.peers.len() > 1 {
                self.g.promote_first_ally();
            }
        } else {
            self.g.remove_ally(i - 1);
        }
        let p = self.peers.remove(i);
        self.heroes.lock().unwrap_or_else(|e| e.into_inner()).retain(|h| *h != (p.account, p.slot));
        let leader = self.peers.first().map(|p| p.name.clone()).unwrap_or_default();
        for j in i.min(self.peers.len())..self.peers.len() {
            let _ = self.send(j, &ToClient::Welcome { you: j as u8, host_name: leader.clone() });
        }
        if i == 0 {
            self.lead = (Input::default(), None);
        }
        self.tell_all(&ToClient::Chat { from: String::new(), text: format!("{} {why}", p.name) });
        self.rescale();
    }

    fn strike(&mut self, i: usize) {
        self.peers[i].strikes += 1;
        if self.peers[i].strikes > MAX_STRIKES {
            let _ = self.send(i, &ToClient::Bye("BAD INPUT".into()));
            self.leave(i, "WAS DISCONNECTED");
        }
    }

    fn handle(&mut self, e: RoomEvent) {
        match e {
            RoomEvent::Join(r) => self.join(r),
            RoomEvent::Input { peer, input } => {
                let Some(i) = self.index(peer) else { return };
                let hero = if i == 0 { (self.g.p.x, self.g.p.y) } else { (self.g.allies[i - 1].st.p.x, self.g.allies[i - 1].st.p.y) };
                match input.check(hero) {
                    Ok((inp, target)) if i == 0 => {
                        hold_input(&mut self.lead.0, inp);
                        self.lead.1 = target;
                        self.g.lead_seq = self.g.lead_seq.max(input.seq);
                    }
                    Ok((inp, target)) => {
                        let a = &mut self.g.allies[i - 1];
                        hold_input(&mut a.inp, inp);
                        a.target = target;
                        a.seq = a.seq.max(input.seq);
                    }
                    Err(_) => self.strike(i),
                }
            }
            RoomEvent::Act { peer, act } => {
                let Some(i) = self.index(peer) else { return };
                let out = if i == 0 { self.g.remote_act_as(&act, true) } else { self.g.as_hero(i - 1, |g| g.remote_act(&act)) };
                if let Some((shop, jewel)) = out.opened {
                    let _ = self.send(i, &ToClient::OpenBag { shop, jewel });
                }
                if let Some(s) = out.said {
                    let _ = self.send(i, &ToClient::Say(s));
                }
                if !out.ok {
                    self.strike(i);
                }
            }
            RoomEvent::Chat { peer, text } => {
                let Some(i) = self.index(peer) else { return };
                let (tokens, last) = &mut self.peers[i].chat;
                *tokens = (*tokens + last.elapsed().as_secs_f32() * 2.0).min(5.0);
                *last = Instant::now();
                if *tokens < 1.0 {
                    return;
                }
                *tokens -= 1.0;
                let from = self.peers[i].name.clone();
                self.tell_all(&ToClient::Chat { from, text });
            }
            RoomEvent::Gone { peer, why } => {
                if let Some(i) = self.index(peer) {
                    self.leave(i, &why);
                }
            }
        }
    }

    /// One tick: the leader's input (its mouse placed on its own screen), the world, then what everyone sees.
    fn tick(&mut self) {
        let mut inp = self.lead.0.clone();
        (self.lead.0.potion_hp, self.lead.0.potion_mp, self.lead.0.eat, self.lead.0.scroll, self.lead.0.cycle, self.lead.0.run_toggle, self.lead.0.slot) = (false, false, false, false, false, false, None);
        if let Some((tx, ty)) = self.lead.1 {
            let (ox, oy) = self.g.cam_origin();
            let (sx, sy) = crate::iso::to_screen(tx - self.g.p.x, ty - self.g.p.y);
            inp.mouse = Some(((sx + ox).round() as i32, (sy + oy).round() as i32));
        }
        // A fallen leader gets up in town (softcore); nobody's at the keyboard to press it.
        if let State::Dead(t) = self.g.state {
            if t > LEADER_RESPAWN && !self.g.hardcore {
                self.g.respawn();
            }
        }
        let before = self.g.message.take();
        self.g.update(&inp);
        // The leader's own words go to the leader.
        if let Some((m, _)) = self.g.message.take() {
            let _ = self.send(0, &ToClient::Say(m));
        }
        drop(before);
        self.g.quit = false;
        self.g.exit = false;
        // Hardcore heroes that fall are saved as fallen and leave the game.
        let fallen: Vec<usize> = std::iter::once(self.g.hardcore && matches!(self.g.state, State::Dead(_)))
            .chain(self.g.allies.iter().map(|a| a.st.hardcore && matches!(a.st.state, State::Dead(_))))
            .enumerate()
            .filter(|&(_, f)| f)
            .map(|(i, _)| i)
            .collect();
        for &i in fallen.iter().rev() {
            if i == 0 {
                self.g.fallen = true;
            } else {
                self.g.allies[i - 1].st.fallen = true;
            }
            let _ = self.send(i, &ToClient::Bye("YOUR HERO HAS FALLEN".into()));
            self.leave(i, "HAS FALLEN");
        }
        self.send_out();
    }

    fn send_out(&mut self) {
        if self.peers.is_empty() {
            return;
        }
        if self.sent_level != Some(self.g.level) {
            self.sent_level = Some(self.g.level);
            if let Some(lv) = self.g.net_level() {
                self.tell_all(&ToClient::Level(lv));
            }
        }
        let fresh: Vec<Floater> = self.g.floaters.iter().filter(|f| f.t <= crate::game::DT * 1.5).cloned().collect();
        let sfx: Vec<Sfx> = self.g.sfx.drain(..).collect();
        for p in self.peers.iter_mut() {
            if p.events.0.len() < 32 {
                p.events.0.extend(sfx.iter().copied());
            }
            if p.events.1.len() < 64 {
                p.events.1.extend(fresh.iter().cloned());
            }
        }
        for i in 1..self.peers.len() {
            for s in std::mem::take(&mut self.g.allies[i - 1].said) {
                let _ = self.send(i, &ToClient::Say(s));
            }
        }
        if self.g.tick % SNAP_EVERY == 0 {
            let mut behind = vec![];
            for i in 0..self.peers.len() {
                let (sfx, fl) = std::mem::take(&mut self.peers[i].events);
                if let Some(s) = self.g.net_snapshot(i as u8, &sfx, &fl) {
                    if !self.send(i, &ToClient::Snap(s)) {
                        behind.push(i);
                    }
                }
            }
            // A player that can't keep up (or is gone) leaves.
            for &i in behind.iter().rev() {
                self.leave(i, "LOST CONNECTION");
            }
        }
        if self.g.tick % SAVE_EVERY == 0 {
            for i in 0..self.peers.len() {
                self.save(i);
            }
        }
    }
}

/// Starts a room's thread. It runs until its last player leaves.
pub fn spawn(id: u32, name: String, password: Option<[u8; 32]>, hardcore: bool, act: u8, accounts: Arc<Mutex<Accounts>>, seed: u64) -> std::io::Result<RoomHandle> {
    let (tx, rx) = std::sync::mpsc::channel::<RoomEvent>();
    let players = Arc::new(AtomicU8::new(0));
    let heroes = Arc::new(Mutex::new(vec![]));
    let ended = Arc::new(AtomicBool::new(false));
    {
        let (players, heroes, ended) = (players.clone(), heroes.clone(), ended.clone());
        std::thread::Builder::new().name(format!("room-{id}")).spawn(move || {
            let g = Game::new(seed, crate::gfx::SH_WIDE);
            let mut room = Room { g, peers: vec![], accounts, heroes, players, sent_level: None, lead: (Input::default(), None) };
            run(&mut room, rx);
            // Everyone's saved on the way out.
            for i in 0..room.peers.len() {
                room.save(i);
            }
            ended.store(true, Ordering::Relaxed);
        })?;
    }
    Ok(RoomHandle { id, name, password, hardcore, act, players, heroes, tx, ended })
}

fn run(room: &mut Room, rx: Receiver<RoomEvent>) {
    let step = Duration::from_secs_f32(crate::game::DT);
    let started = Instant::now();
    let mut next = Instant::now();
    loop {
        // Events, without waiting long (the world goes on).
        while let Ok(e) = rx.try_recv() {
            room.handle(e);
        }
        if room.peers.is_empty() {
            // Nobody came within a minute of starting, or the last player left.
            if room.g.tick > 0 || started.elapsed() > Duration::from_secs(60) {
                return;
            }
        } else {
            room.tick();
        }
        next += step;
        let now = Instant::now();
        if next > now {
            std::thread::sleep(next - now);
        } else if now - next > Duration::from_millis(250) {
            // Too far behind (a slow machine): don't try to catch up all at once.
            next = now;
        }
    }
}

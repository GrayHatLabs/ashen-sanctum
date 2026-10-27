//! The login server (docs/ONLINE_PLAN.md, phase 3): accounts, heroes kept on the server, and games anyone
//! logged in can create or join. `ashensanctum --server` runs it with no window (Docker: see docker/).
//!
//! Settings (command line, or the environment for Docker):
//! - `--port N` / `ASHEN_PORT` (4747)
//! - `--data DIR` / `ASHEN_DATA` (./server-data): accounts, heroes and the server's TLS key
//! - `ASHEN_INVITE`: if set, registering needs this code (12 characters or more)
//! - `ASHEN_SERVER_NAME` (ASHEN SANCTUM); `ASHEN_MAX_GAMES` (6); `ASHEN_MAX_PLAYERS` (32 connections, at most 200)
//!
//! Every connection: TLS 1.3 within 10 s, hello, then log in (or register) within 45 s, then heroes and games.
//! Every step has limits: frame size and rate (net::conn), a cap on connections not yet logged in (half of all),
//! failed logins / registrations / game passwords per connection, requests a second, hero and game changes a
//! minute, and one session per account. A hero is reserved in one shared set, under one lock, before it goes
//! anywhere: it can't be in two games, or deleted while playing (security review 2026-10-27).
#![forbid(unsafe_code)]

pub mod accounts;
pub mod room;

use crate::net::conn::{self, Ended};
use crate::net::proto::{self, FromServer, GameInfo, ServerHero, ToHost, ToServer};
use crate::net::tls;
use accounts::{Accounts, HeroFiles};
use room::{JoinReq, RoomEvent, RoomHandle};
use std::collections::{HashMap, HashSet};
use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{self, SyncSender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const PER_ADDRESS: usize = 4;
const HELLO_WITHIN: Duration = Duration::from_secs(10);
/// Log in (or register) within this long of connecting.
const LOGIN_WITHIN: Duration = Duration::from_secs(45);
/// Wrong logins, failed registrations and wrong game passwords on one connection before it's closed (the
/// account and address limits apply as well).
const LOGIN_TRIES: u32 = 3;
const REGISTER_TRIES: u32 = 3;
const GAME_PW_TRIES: u32 = 5;
/// Hero and game changes a connection may make a minute.
const CHANGES_PER_MIN: u32 = 12;
/// Requests a logged-in connection may make: 4 a second, 8 at once.
const REQ_RATE: f32 = 4.0;
const REQ_BURST: f32 = 8.0;
/// The placeholder in docker/.env.example: a server won't start with it.
const PLACEHOLDER_INVITE: &str = "change-me-to-something-long";

pub struct Config {
    pub port: u16,
    pub data: std::path::PathBuf,
    pub invite: Option<String>,
    pub name: String,
    pub max_games: usize,
    pub max_conns: usize,
}

impl Config {
    pub fn from_args(args: &[String]) -> Config {
        let after = |flag: &str| args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).filter(|v| !v.starts_with("--")).cloned();
        let env = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
        Config {
            port: after("--port").or_else(|| env("ASHEN_PORT")).and_then(|p| p.parse().ok()).unwrap_or(proto::DEFAULT_PORT),
            data: after("--data").or_else(|| env("ASHEN_DATA")).unwrap_or_else(|| "server-data".into()).into(),
            invite: env("ASHEN_INVITE"),
            name: env("ASHEN_SERVER_NAME").and_then(|n| proto::clean_game_name(&n)).unwrap_or_else(|| "ASHEN SANCTUM".into()),
            max_games: env("ASHEN_MAX_GAMES").and_then(|n| n.parse().ok()).unwrap_or(6).clamp(1, 64),
            // A thread per connection: Docker allows 256 processes (docker-compose.yml).
            max_conns: env("ASHEN_MAX_PLAYERS").and_then(|n| n.parse().ok()).unwrap_or(32).clamp(2, 200),
        }
    }
}

struct Shared {
    accounts: Arc<Mutex<Accounts>>,
    files: HeroFiles,
    /// Heroes in play or being changed (account, slot): one place at a time.
    busy: Arc<Mutex<HashSet<(u64, u8)>>>,
    /// Accounts logged in now: one session each.
    online: Mutex<HashSet<u64>>,
    rooms: Mutex<Vec<RoomHandle>>,
    next_room: AtomicU32,
    conns: Mutex<HashMap<IpAddr, usize>>,
    total: Mutex<usize>,
    /// Connections not logged in yet (they get at most half the room).
    unauth: Mutex<usize>,
    cfg: Config,
    stop: AtomicBool,
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

fn log(s: &str) {
    // Never passwords, codes or chat: who, what, and when is enough.
    let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    eprintln!("[{t}] {s}");
}

/// Runs the server until it's stopped (Ctrl+C / docker stop). Returns the exit code.
pub fn run(args: &[String]) -> i32 {
    let cfg = Config::from_args(args);
    match serve(cfg, None) {
        Ok(()) => 0,
        Err(e) => {
            log(&format!("server failed: {e}"));
            1
        }
    }
}

/// The server proper. `ready` (tests) hears the port and key once it's listening.
pub fn serve(cfg: Config, ready: Option<SyncSender<(u16, [u8; 32])>>) -> std::io::Result<()> {
    if let Some(code) = &cfg.invite {
        if code == PLACEHOLDER_INVITE || code.chars().count() < 12 {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "ASHEN_INVITE is the example's placeholder or shorter than 12 characters: set your own in docker/.env (or leave it empty for open registering)"));
        }
    }
    std::fs::create_dir_all(&cfg.data)?;
    let id = tls::identity(&cfg.data.join("tls"))?;
    let tls_cfg = tls::server_config(&id).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
    let accounts = Accounts::open(&cfg.data, cfg.invite.clone())?;
    let files = accounts.files();
    let listener = TcpListener::bind(("0.0.0.0", cfg.port))?;
    let port = listener.local_addr()?.port();
    log(&format!("{} listening on port {port}; key {}; invite {}", cfg.name, tls::show(&id.fingerprint), if cfg.invite.is_some() { "required" } else { "not required" }));
    if let Some(r) = ready {
        let _ = r.send((port, id.fingerprint));
    }
    let shared = Arc::new(Shared {
        accounts: Arc::new(Mutex::new(accounts)),
        files,
        busy: Arc::new(Mutex::new(HashSet::new())),
        online: Mutex::new(HashSet::new()),
        rooms: Mutex::new(vec![]),
        next_room: AtomicU32::new(1),
        conns: Mutex::new(HashMap::new()),
        total: Mutex::new(0),
        unauth: Mutex::new(0),
        cfg,
        stop: AtomicBool::new(false),
    });
    let next_peer = AtomicU32::new(1);
    for conn in listener.incoming() {
        if shared.stop.load(Ordering::Relaxed) {
            break;
        }
        let Ok(tcp) = conn else { continue };
        let Ok(addr) = tcp.peer_addr() else { continue };
        {
            let mut total = lock(&shared.total);
            let mut per = lock(&shared.conns);
            let mut unauth = lock(&shared.unauth);
            let here = per.get(&addr.ip()).copied().unwrap_or(0);
            if *total >= shared.cfg.max_conns || here >= PER_ADDRESS || *unauth >= (shared.cfg.max_conns / 2).max(2) {
                continue;
            }
            *total += 1;
            *unauth += 1;
            *per.entry(addr.ip()).or_insert(0) += 1;
        }
        let peer = next_peer.fetch_add(1, Ordering::Relaxed);
        let (cfg, shared2) = (tls_cfg.clone(), shared.clone());
        let ip = addr.ip();
        let spawned = std::thread::Builder::new().name(format!("conn-{peer}")).spawn(move || {
            let mut guard = Guard { shared: &shared2, ip, unauth: true, account: None, game: None, peer };
            // Whatever happens in there (a panic included), the guard gives back what the connection held.
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| connection(peer, tcp, addr, cfg, &mut guard)));
        });
        if spawned.is_err() {
            let mut g = Guard { shared: &shared, ip, unauth: true, account: None, game: None, peer };
            g.account = None;
        }
    }
    Ok(())
}

/// What a connection holds, given back when it ends (however it ends): its slot, its "not logged in" slot,
/// its account's session, and its place in a game.
struct Guard<'a> {
    shared: &'a Shared,
    ip: IpAddr,
    unauth: bool,
    account: Option<u64>,
    /// In a room: its events channel, to say the player's gone.
    game: Option<mpsc::Sender<RoomEvent>>,
    peer: u32,
}

impl Guard<'_> {
    fn logged_in(&mut self, id: u64) {
        if self.unauth {
            let mut u = lock(&self.shared.unauth);
            *u = u.saturating_sub(1);
            self.unauth = false;
        }
        self.account = Some(id);
    }
}

impl Drop for Guard<'_> {
    fn drop(&mut self) {
        if let Some(tx) = self.game.take() {
            let _ = tx.send(RoomEvent::Gone { peer: self.peer, why: "LEFT".into() });
        }
        if self.unauth {
            let mut u = lock(&self.shared.unauth);
            *u = u.saturating_sub(1);
        }
        if let Some(id) = self.account {
            lock(&self.shared.online).remove(&id);
        }
        let mut total = lock(&self.shared.total);
        *total = total.saturating_sub(1);
        let mut per = lock(&self.shared.conns);
        if let Some(n) = per.get_mut(&self.ip) {
            *n = n.saturating_sub(1);
            if *n == 0 {
                per.remove(&self.ip);
            }
        }
    }
}

/// Where a connection is.
enum Stage {
    /// Waiting for the hello.
    Start,
    /// In the lobby (logged in or not).
    Lobby { who: Option<(u64, String)> },
    /// Playing in a room.
    Game { tx: mpsc::Sender<RoomEvent> },
}

/// A connection's counters.
struct Tally {
    bad_logins: u32,
    bad_registers: u32,
    bad_game_pw: u32,
    changes: Vec<Instant>,
    /// Requests allowance (tokens, last refill).
    req: (f32, Instant),
}

/// The lobby's limits on a connection: idle a while, it's closed (the game pings while you're in the menus).
const LOBBY: conn::Limits = conn::Limits { max_frame: proto::MAX_HERO + 1024, frames_per_sec: 150, bytes_per_sec: 64 * 1024, idle: Duration::from_secs(30) };

fn connection(peer: u32, tcp: TcpStream, addr: SocketAddr, cfg: Arc<rustls::ServerConfig>, guard: &mut Guard) {
    let shared = guard.shared;
    let started = Instant::now();
    let setup = || -> std::io::Result<rustls::StreamOwned<rustls::ServerConnection, TcpStream>> {
        tcp.set_nodelay(true)?;
        tcp.set_write_timeout(Some(Duration::from_secs(5)))?;
        let conn = rustls::ServerConnection::new(cfg).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        let mut s = rustls::StreamOwned::new(conn, tcp);
        tls::handshake(&mut s, started + HELLO_WITHIN, &shared.stop)?;
        s.sock.set_read_timeout(Some(Duration::from_millis(10)))?;
        Ok(s)
    };
    let Ok(mut tls) = setup() else { return };
    let (out_tx, out_rx) = mpsc::sync_channel::<Vec<u8>>(256);
    let reply = |m: &FromServer| {
        if let Some(b) = proto::encode(m) {
            let _ = out_tx.try_send(b);
        }
    };
    let mut stage = Stage::Start;
    let mut strikes = 0u32;
    let mut tally = Tally { bad_logins: 0, bad_registers: 0, bad_game_pw: 0, changes: vec![], req: (REQ_BURST, Instant::now()) };
    let stop = AtomicBool::new(false);
    let ended = conn::pump(&mut tls, LOBBY, &out_rx, &stop, |f| {
        // Deadlines: the hello, then logging in (pinging doesn't put it off).
        if matches!(stage, Stage::Start) && started.elapsed() > HELLO_WITHIN {
            return false;
        }
        if matches!(stage, Stage::Lobby { who: None }) && started.elapsed() > LOGIN_WITHIN {
            reply(&FromServer::Refused("LOG IN SOONER".into()));
            return false;
        }
        match &stage {
            Stage::Game { tx } => {
                let Some(m) = proto::decode::<ToHost>(&f) else {
                    strikes += 1;
                    return strikes < 3;
                };
                let ev = match m {
                    ToHost::Input(input) => RoomEvent::Input { peer, input },
                    ToHost::Chat(text) => match proto::clean_chat(&text) {
                        Some(text) => RoomEvent::Chat { peer, text },
                        None => return true,
                    },
                    ToHost::Bye => return false,
                    ToHost::Hello { .. } => return false,
                    act => RoomEvent::Act { peer, act },
                };
                tx.send(ev).is_ok()
            }
            _ => {
                let Some(m) = proto::decode::<ToServer>(&f) else {
                    strikes += 1;
                    return strikes < 3;
                };
                // Logged in, requests are rate-limited too (they read files).
                if matches!(stage, Stage::Lobby { who: Some(_) }) && m != ToServer::Ping {
                    let (tokens, last) = &mut tally.req;
                    *tokens = (*tokens + last.elapsed().as_secs_f32() * REQ_RATE).min(REQ_BURST);
                    *last = Instant::now();
                    if *tokens < 1.0 {
                        return false;
                    }
                    *tokens -= 1.0;
                }
                let go_on = lobby(m, peer, addr.ip(), &mut stage, &mut tally, &out_tx, &reply, guard);
                if let Stage::Game { tx } = &stage {
                    guard.game = Some(tx.clone());
                }
                go_on
            }
        }
    });
    if guard.game.is_some() {
        let why = match ended {
            Ended::Closed | Ended::Stopped => "LEFT",
            Ended::Idle => "TIMED OUT",
            Ended::Misbehaved(_) => "WAS DISCONNECTED",
            Ended::Io(_) => "LOST CONNECTION",
        };
        if let Some(tx) = guard.game.take() {
            let _ = tx.send(RoomEvent::Gone { peer, why: why.into() });
        }
    }
    // A goodbye or refusal still queued goes out, briefly.
    let _ = tls.sock.set_write_timeout(Some(Duration::from_millis(300)));
    while let Ok(f) = out_rx.try_recv() {
        if conn::Wire::send(&mut tls, &f).is_err() {
            break;
        }
    }
    tls.conn.send_close_notify();
    let _ = tls.conn.complete_io(&mut tls.sock);
}

/// Reserves a hero (it's free), or None.
fn reserve(shared: &Shared, id: u64, slot: u8) -> bool {
    lock(&shared.busy).insert((id, slot))
}

fn unreserve(shared: &Shared, id: u64, slot: u8) {
    lock(&shared.busy).remove(&(id, slot));
}

/// A session for this account (one at a time). False if it's already on.
fn take_session(shared: &Shared, id: u64) -> bool {
    lock(&shared.online).insert(id)
}

/// One lobby message. False ends the connection.
#[allow(clippy::too_many_arguments)]
fn lobby(m: ToServer, peer: u32, ip: IpAddr, stage: &mut Stage, tally: &mut Tally, out: &SyncSender<Vec<u8>>, reply: &dyn Fn(&FromServer), guard: &mut Guard) -> bool {
    let shared = guard.shared;
    let refuse = |why: &str| reply(&FromServer::Refused(why.to_string()));
    // Hero and game changes are limited (each new hero builds a world to make it).
    let mut change_ok = || {
        tally.changes.retain(|t| t.elapsed() < Duration::from_secs(60));
        tally.changes.push(Instant::now());
        tally.changes.len() as u32 <= CHANGES_PER_MIN
    };
    match (m, &*stage) {
        (ToServer::Hello { protocol, version }, Stage::Start) => {
            if protocol != proto::PROTOCOL || version != env!("CARGO_PKG_VERSION") {
                refuse(&format!("VERSION MISMATCH: THE SERVER RUNS {}", env!("CARGO_PKG_VERSION")));
                return false;
            }
            let invite = lock(&shared.accounts).needs_invite();
            reply(&FromServer::Hello { name: shared.cfg.name.clone(), invite });
            *stage = Stage::Lobby { who: None };
            true
        }
        (_, Stage::Start) => false,
        (ToServer::Register { name, password, invite }, Stage::Lobby { who: None }) => match accounts::register(&shared.accounts, &name, &password, &invite, ip) {
            Ok(id) if take_session(shared, id) => {
                log(&format!("new account {id} from {ip}"));
                guard.logged_in(id);
                *stage = Stage::Lobby { who: Some((id, name.clone())) };
                reply(&FromServer::LoggedIn { name });
                true
            }
            Ok(_) => {
                refuse("THAT ACCOUNT IS ALREADY PLAYING");
                true
            }
            Err(why) => {
                tally.bad_registers += 1;
                refuse(why);
                tally.bad_registers < REGISTER_TRIES
            }
        },
        (ToServer::Login { name, password }, Stage::Lobby { who: None }) => match accounts::login(&shared.accounts, &name, &password, ip) {
            Ok((id, name)) => {
                if !take_session(shared, id) {
                    refuse("THAT ACCOUNT IS ALREADY PLAYING");
                    return true;
                }
                log(&format!("account {id} logged in from {ip}"));
                guard.logged_in(id);
                *stage = Stage::Lobby { who: Some((id, name.clone())) };
                reply(&FromServer::LoggedIn { name });
                true
            }
            Err(why) => {
                tally.bad_logins += 1;
                refuse(why);
                tally.bad_logins < LOGIN_TRIES
            }
        },
        (ToServer::Ping, _) => true,
        // Everything else needs a login.
        (_, Stage::Lobby { who: None }) => {
            refuse("LOG IN FIRST");
            true
        }
        (ToServer::Heroes, Stage::Lobby { who: Some((id, _)) }) => {
            reply(&FromServer::Heroes(heroes_of(shared, *id)));
            true
        }
        (ToServer::NewHero { class, name, hardcore }, Stage::Lobby { who: Some((id, _)) }) => {
            let id = *id;
            if !change_ok() {
                refuse("SLOW DOWN");
                return true;
            }
            let Some(class) = crate::skills::ALL_CLASSES.get(class as usize).copied() else { return false };
            let Some(name) = proto::clean_name(&name) else {
                refuse("BAD HERO NAME");
                return true;
            };
            // The slot is reserved while it's made (two at once can't take the same one).
            let slot = {
                let mut busy = lock(&shared.busy);
                let s = shared.files.free_slot(id, &busy);
                if let Some(s) = s {
                    busy.insert((id, s));
                }
                s
            };
            let Some(slot) = slot else {
                refuse("ALL YOUR HERO SLOTS ARE FULL");
                return true;
            };
            let mut g = crate::game::Game::new(new_seed(), crate::gfx::SH_WIDE);
            g.set_class(class);
            g.hero_name = name;
            g.hardcore = hardcore;
            let text = crate::save::to_text(&g);
            if shared.files.save_hero(id, slot, &text).is_err() {
                refuse("THE SERVER COULDN'T SAVE THAT");
            }
            unreserve(shared, id, slot);
            reply(&FromServer::Heroes(heroes_of(shared, id)));
            true
        }
        (ToServer::DeleteHero { slot }, Stage::Lobby { who: Some((id, _)) }) => {
            let id = *id;
            if !change_ok() {
                refuse("SLOW DOWN");
                return true;
            }
            if !reserve(shared, id, slot) {
                refuse("THAT HERO IS IN A GAME");
            } else {
                if shared.files.delete_hero(id, slot).is_err() {
                    refuse("NO SUCH HERO");
                }
                unreserve(shared, id, slot);
            }
            reply(&FromServer::Heroes(heroes_of(shared, id)));
            true
        }
        (ToServer::Games, Stage::Lobby { who: Some(_) }) => {
            reply(&FromServer::Games(games(shared)));
            true
        }
        (ToServer::CreateGame { name, password, hero }, Stage::Lobby { who: Some((id, acct)) }) => {
            let (id, acct) = (*id, acct.clone());
            if !change_ok() {
                refuse("SLOW DOWN");
                return true;
            }
            let Some(gname) = proto::clean_game_name(&name) else {
                refuse("GIVE THE GAME A NAME");
                return true;
            };
            let Some(text) = playable(shared, id, hero, &refuse) else { return true };
            let hardcore = text.lines().any(|l| l.trim() == "hardcore=1");
            let act = text.lines().find_map(|l| l.strip_prefix("act=")).and_then(|v| v.trim().parse::<u8>().ok()).unwrap_or(0).min(6);
            let rid = {
                let mut rooms = lock(&shared.rooms);
                rooms.retain(|r| !r.ended.load(Ordering::Relaxed));
                if rooms.len() >= shared.cfg.max_games {
                    drop(rooms);
                    unreserve(shared, id, hero);
                    refuse("THE SERVER IS FULL OF GAMES. JOIN ONE");
                    return true;
                }
                let rid = shared.next_room.fetch_add(1, Ordering::Relaxed);
                let pw = (!password.is_empty()).then(|| tls::hash(password.as_bytes()));
                match room::spawn(rid, gname.clone(), pw, hardcore, act, shared.files.clone(), shared.busy.clone(), new_seed()) {
                    Ok(h) => rooms.push(h),
                    Err(_) => {
                        drop(rooms);
                        unreserve(shared, id, hero);
                        refuse("THE SERVER COULDN'T START A GAME");
                        return true;
                    }
                }
                rid
            };
            log(&format!("account {id} made game {rid}"));
            join(shared, stage, tally, peer, id, &acct, hero, text, rid, &password, out, reply)
        }
        (ToServer::JoinGame { game, password, hero }, Stage::Lobby { who: Some((id, acct)) }) => {
            let (id, acct) = (*id, acct.clone());
            let Some(text) = playable(shared, id, hero, &refuse) else { return true };
            join(shared, stage, tally, peer, id, &acct, hero, text, game, &password, out, reply)
        }
        // A second login, a hello again: not allowed.
        _ => false,
    }
}

/// A hero that can play, reserved for it: it exists, isn't fallen, and isn't anywhere else. The caller gives
/// the reservation back if it doesn't go into a game.
fn playable(shared: &Shared, id: u64, slot: u8, refuse: &dyn Fn(&str)) -> Option<String> {
    if !reserve(shared, id, slot) {
        refuse("THAT HERO IS IN A GAME");
        return None;
    }
    let text = shared.files.hero(id, slot);
    let Some(text) = text else {
        unreserve(shared, id, slot);
        refuse("NO SUCH HERO");
        return None;
    };
    if text.lines().any(|l| l.trim() == "fallen=1") {
        unreserve(shared, id, slot);
        refuse("THAT HERO HAS FALLEN");
        return None;
    }
    Some(text)
}

/// Into a game with a reserved hero: if it doesn't get in, the reservation goes back. False ends the connection.
#[allow(clippy::too_many_arguments)]
fn join(shared: &Shared, stage: &mut Stage, tally: &mut Tally, peer: u32, id: u64, acct: &str, slot: u8, text: String, game: u32, password: &str, out: &SyncSender<Vec<u8>>, reply: &dyn Fn(&FromServer)) -> bool {
    let refuse = |why: &str| {
        unreserve(shared, id, slot);
        reply(&FromServer::Refused(why.to_string()));
    };
    let hero_name = text.lines().find_map(|l| l.strip_prefix("name=")).and_then(proto::clean_name).unwrap_or_else(|| acct.to_ascii_uppercase());
    let hardcore = text.lines().any(|l| l.trim() == "hardcore=1");
    let tx = {
        let rooms = lock(&shared.rooms);
        let Some(r) = rooms.iter().find(|r| r.id == game && !r.ended.load(Ordering::Relaxed)) else {
            drop(rooms);
            refuse("THAT GAME HAS ENDED");
            return true;
        };
        let pw_ok = r.password.map_or(true, |h| tls::hash(password.as_bytes()).iter().zip(h.iter()).fold(0u8, |a, (x, y)| a | (x ^ y)) == 0);
        if !pw_ok {
            drop(rooms);
            tally.bad_game_pw += 1;
            refuse("WRONG GAME PASSWORD");
            return tally.bad_game_pw < GAME_PW_TRIES;
        }
        if r.hardcore != hardcore {
            let why = if r.hardcore { "A HARDCORE GAME: HARDCORE HEROES ONLY" } else { "HARDCORE HEROES PLAY IN HARDCORE GAMES" };
            drop(rooms);
            refuse(why);
            return true;
        }
        if r.players.load(Ordering::Relaxed) as usize >= room::MAX_PLAYERS {
            drop(rooms);
            refuse("THE GAME IS FULL");
            return true;
        }
        r.tx.clone()
    };
    let (rtx, rrx) = mpsc::sync_channel(1);
    let req = JoinReq { peer, account: id, slot, name: hero_name, hero: text, out: out.clone(), reply: rtx };
    if tx.send(RoomEvent::Join(req)).is_err() {
        refuse("THAT GAME HAS ENDED");
        return true;
    }
    match rrx.recv_timeout(Duration::from_secs(10)) {
        Ok(Ok((you, host_name))) => {
            reply(&FromServer::Joined { you, host_name });
            *stage = Stage::Game { tx };
        }
        // The room gave the reservation back itself.
        Ok(Err(why)) => reply(&FromServer::Refused(why)),
        // Too slow: the room won't take it now (its answer can't be delivered), so it's free again.
        Err(_) => refuse("THE GAME DIDN'T ANSWER"),
    }
    true
}

fn heroes_of(shared: &Shared, id: u64) -> Vec<ServerHero> {
    shared
        .files
        .heroes(id)
        .into_iter()
        .filter_map(|(slot, text)| {
            let h = crate::save::hero_info(&slot.to_string(), &text)?;
            let class = crate::skills::ALL_CLASSES.iter().position(|c| *c == h.class)? as u8;
            Some(ServerHero { slot, name: h.name, class, clvl: h.clvl, act: h.act as u8, hardcore: h.hardcore, fallen: h.fallen })
        })
        .collect()
}

fn games(shared: &Shared) -> Vec<GameInfo> {
    let mut rooms = lock(&shared.rooms);
    rooms.retain(|r| !r.ended.load(Ordering::Relaxed));
    rooms.iter().map(|r| GameInfo { id: r.id, name: r.name.clone(), players: r.players.load(Ordering::Relaxed), locked: r.password.is_some(), hardcore: r.hardcore, act: r.act }).collect()
}

/// A fresh world seed (from the OS's random source).
fn new_seed() -> u64 {
    let mut b = [0u8; 8];
    if accounts::random(&mut b) {
        u64::from_le_bytes(b)
    } else {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.as_nanos() as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::client::{lobby, JoinError, Lobby};
    use crate::net::proto::{FromServer, ToServer};

    fn tmp(n: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("ashen_server_{n}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    /// A player: connects (trusting the server's key), registers, and makes a hero.
    fn player(addr: &str, dir: &std::path::Path, fp: [u8; 32], name: &str, class: u8) -> Lobby {
        match lobby(addr, dir) {
            Err(JoinError::Unknown(k)) => {
                assert_eq!(k, fp);
                crate::net::tls::trust(dir, &crate::net::tls::host_key(addr), &k).unwrap();
            }
            Ok(_) => {}
            Err(e) => panic!("{e:?}"),
        }
        let mut l = lobby(addr, dir).expect("connected");
        assert_eq!(l.server, "TEST SERVER");
        assert!(matches!(l.ask(&ToServer::Heroes), Ok(FromServer::Refused(w)) if w == "LOG IN FIRST"));
        assert!(matches!(l.ask(&ToServer::Register { name: name.into(), password: "a long password".into(), invite: String::new() }), Ok(FromServer::LoggedIn { .. })));
        match l.ask(&ToServer::NewHero { class, name: format!("{name}hero"), hardcore: false }) {
            Ok(FromServer::Heroes(h)) => assert_eq!(h.len(), 1),
            other => panic!("{other:?}"),
        }
        l
    }

    #[test]
    fn a_hero_is_reserved_once_and_cant_be_deleted_in_play() {
        let data = tmp("reserve");
        let accounts = Accounts::open(&data, None).unwrap();
        let files = accounts.files();
        files.save_hero(3, 0, "name=ROWAN
class=reaper
clvl=1
").unwrap();
        let shared = Shared {
            accounts: Arc::new(Mutex::new(accounts)),
            files,
            busy: Arc::new(Mutex::new(HashSet::new())),
            online: Mutex::new(HashSet::new()),
            rooms: Mutex::new(vec![]),
            next_room: AtomicU32::new(1),
            conns: Mutex::new(HashMap::new()),
            total: Mutex::new(0),
            unauth: Mutex::new(0),
            cfg: Config { port: 0, data: data.clone(), invite: None, name: "T".into(), max_games: 2, max_conns: 4 },
            stop: AtomicBool::new(false),
        };
        let said = std::cell::RefCell::new(vec![]);
        let refuse = |w: &str| said.borrow_mut().push(w.to_string());
        // Two requests for the same hero at once: only the first gets it.
        assert!(playable(&shared, 3, 0, &refuse).is_some());
        assert!(playable(&shared, 3, 0, &refuse).is_none());
        assert_eq!(said.borrow().last().map(String::as_str), Some("THAT HERO IS IN A GAME"));
        // While it's out, it can't be deleted either.
        assert!(!reserve(&shared, 3, 0));
        unreserve(&shared, 3, 0);
        assert!(reserve(&shared, 3, 0), "free again once given back");
        // And one session per account.
        assert!(take_session(&shared, 3));
        assert!(!take_session(&shared, 3));
        let _ = std::fs::remove_dir_all(&data);
    }

    #[test]
    fn the_example_invite_code_wont_start_a_server() {
        let cfg = Config { port: 0, data: tmp("placeholder"), invite: Some(PLACEHOLDER_INVITE.into()), name: "T".into(), max_games: 1, max_conns: 2 };
        assert!(serve(cfg, None).is_err());
        let cfg = Config { port: 0, data: tmp("short"), invite: Some("short".into()), name: "T".into(), max_games: 1, max_conns: 2 };
        assert!(serve(cfg, None).is_err());
    }

    #[test]
    fn players_register_make_heroes_and_play_together_on_the_server() {
        let data = tmp("data");
        let (rtx, rrx) = std::sync::mpsc::sync_channel(1);
        let cfg = Config { port: 0, data: data.clone(), invite: None, name: "TEST SERVER".into(), max_games: 4, max_conns: 16 };
        std::thread::spawn(move || serve(cfg, Some(rtx)));
        let (port, fp) = rrx.recv_timeout(Duration::from_secs(30)).expect("the server started");
        let addr = format!("127.0.0.1:{port}");
        let cd = tmp("client");
        // A wrong password, then the right one, for an account that exists.
        let mut a = player(&addr, &cd, fp, "Rowan", 5);
        let mut probe = lobby(&addr, &cd).unwrap();
        assert!(matches!(probe.ask(&ToServer::Login { name: "rowan".into(), password: "wrong password".into() }), Ok(FromServer::Refused(w)) if w == accounts::BAD_LOGIN));
        // One session per account: Rowan is already on.
        assert!(matches!(probe.ask(&ToServer::Login { name: "rowan".into(), password: "a long password".into() }), Ok(FromServer::Refused(w)) if w == "THAT ACCOUNT IS ALREADY PLAYING"));
        drop(probe);
        // Rowan makes a game; Ada sees it and joins.
        let link_a = a.enter(&ToServer::CreateGame { name: "CRYPT RUN".into(), password: "pw".into(), hero: 0 }).map_err(|e| e.1).expect("made a game");
        assert_eq!(link_a.you, 0, "the maker leads");
        let mut b = player(&addr, &cd, fp, "Ada", 2);
        let games = match b.ask(&ToServer::Games) {
            Ok(FromServer::Games(g)) => g,
            other => panic!("{other:?}"),
        };
        assert_eq!(games.len(), 1);
        assert!(games[0].locked && games[0].name == "CRYPT RUN");
        let gid = games[0].id;
        let b = match b.enter(&ToServer::JoinGame { game: gid, password: "nope".into(), hero: 0 }) {
            Err((Some(l), why)) => {
                assert_eq!(why, "WRONG GAME PASSWORD");
                l
            }
            _ => panic!("a wrong game password must be refused"),
        };
        let link_b = b.enter(&ToServer::JoinGame { game: gid, password: "pw".into(), hero: 0 }).map_err(|e| e.1).expect("joined");
        assert_eq!(link_b.you, 1);
        // Both play: their worlds come from the server, and Ada's hero walks there.
        let (mut ga, mut gb) = (crate::game::Game::new(1, crate::gfx::SH_WIDE), crate::game::Game::new(2, crate::gfx::SH_WIDE));
        let (mut ja, mut jb) = (crate::online::Joined::new(link_a), crate::online::Joined::new(link_b));
        let idle = crate::game::Input::default();
        for _ in 0..90 {
            assert_eq!(ja.frame(&mut ga, &idle), None);
            assert_eq!(jb.frame(&mut gb, &idle), None);
            std::thread::sleep(Duration::from_millis(17));
        }
        let start = (gb.p.x, gb.p.y);
        let walk = crate::game::Input { move_x: 1.0, ..Default::default() };
        for _ in 0..90 {
            ja.frame(&mut ga, &idle);
            jb.frame(&mut gb, &walk);
            std::thread::sleep(Duration::from_millis(17));
        }
        assert_eq!(ga.level, gb.level, "the same world");
        assert_eq!(gb.p.skills.class, crate::skills::ALL_CLASSES[2]);
        assert!(ga.allies.iter().any(|h| h.st.p.skills.class == crate::skills::ALL_CLASSES[2]), "Rowan sees Ada");
        assert!((gb.p.x - start.0).abs() + (gb.p.y - start.1).abs() > 1.0, "Ada walked");
        // Rowan leaves: Ada leads now.
        drop(ja);
        let mut led = false;
        for _ in 0..120 {
            jb.frame(&mut gb, &idle);
            if jb.me == 0 {
                led = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(17));
        }
        assert!(led, "the next player leads");
        // Rowan's hero was saved back to its account.
        let a_hero = std::fs::read_to_string(data.join("heroes/1/0.txt")).unwrap();
        assert!(a_hero.contains("class=reaper"));
        drop(jb);
        let _ = std::fs::remove_dir_all(&cd);
    }
}

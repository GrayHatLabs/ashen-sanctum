//! The login server (docs/ONLINE_PLAN.md, phase 3): accounts, heroes kept on the server, and games anyone
//! logged in can create or join. `ashensanctum --server` runs it with no window (Docker: see docker/).
//!
//! Settings (command line, or the environment for Docker):
//! - `--port N` / `ASHEN_PORT` (4747)
//! - `--data DIR` / `ASHEN_DATA` (./server-data): accounts, heroes and the server's TLS key
//! - `ASHEN_INVITE`: if set, registering needs this code
//! - `ASHEN_SERVER_NAME` (ASHEN SANCTUM); `ASHEN_MAX_GAMES` (6); `ASHEN_MAX_PLAYERS` (32 connections)
//!
//! Every connection: TLS 1.3 within 10 s, then hello, then log in (or register), then pick heroes and games.
//! Every step has limits: frame size and rate (net::conn), a lobby idle timeout, a cap on failed logins per
//! connection, and on hero and game changes per minute.
#![forbid(unsafe_code)]

pub mod accounts;
pub mod room;

use crate::net::conn::{self, Ended};
use crate::net::proto::{self, FromServer, GameInfo, ServerHero, ToHost, ToServer};
use crate::net::tls;
use accounts::Accounts;
use room::{JoinReq, RoomEvent, RoomHandle};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{self, SyncSender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const PER_ADDRESS: usize = 4;
const HELLO_WITHIN: Duration = Duration::from_secs(10);
/// Wrong logins on one connection before it's closed (the account and address limits apply as well).
const LOGIN_TRIES: u32 = 3;
/// Hero and game changes a connection may make a minute.
const CHANGES_PER_MIN: u32 = 12;

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
            max_conns: env("ASHEN_MAX_PLAYERS").and_then(|n| n.parse().ok()).unwrap_or(32).clamp(1, 512),
        }
    }
}

struct Shared {
    accounts: Arc<Mutex<Accounts>>,
    rooms: Mutex<Vec<RoomHandle>>,
    next_room: AtomicU32,
    conns: Mutex<HashMap<IpAddr, usize>>,
    total: Mutex<usize>,
    cfg: Config,
    stop: AtomicBool,
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
    std::fs::create_dir_all(&cfg.data)?;
    let id = tls::identity(&cfg.data.join("tls"))?;
    let tls_cfg = tls::server_config(&id).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
    let accounts = Arc::new(Mutex::new(Accounts::open(&cfg.data, cfg.invite.clone())?));
    let listener = TcpListener::bind(("0.0.0.0", cfg.port))?;
    let port = listener.local_addr()?.port();
    log(&format!("{} listening on port {port}; key {}; invite {}", cfg.name, tls::show(&id.fingerprint), if cfg.invite.is_some() { "required" } else { "not required" }));
    if let Some(r) = ready {
        let _ = r.send((port, id.fingerprint));
    }
    let shared = Arc::new(Shared { accounts, rooms: Mutex::new(vec![]), next_room: AtomicU32::new(1), conns: Mutex::new(HashMap::new()), total: Mutex::new(0), cfg, stop: AtomicBool::new(false) });
    let next_peer = AtomicU32::new(1);
    for conn in listener.incoming() {
        if shared.stop.load(Ordering::Relaxed) {
            break;
        }
        let Ok(tcp) = conn else { continue };
        let Ok(addr) = tcp.peer_addr() else { continue };
        {
            let mut total = shared.total.lock().unwrap_or_else(|e| e.into_inner());
            let mut per = shared.conns.lock().unwrap_or_else(|e| e.into_inner());
            let here = per.get(&addr.ip()).copied().unwrap_or(0);
            if *total >= shared.cfg.max_conns || here >= PER_ADDRESS {
                continue;
            }
            *total += 1;
            *per.entry(addr.ip()).or_insert(0) += 1;
        }
        let peer = next_peer.fetch_add(1, Ordering::Relaxed);
        let (cfg, shared2) = (tls_cfg.clone(), shared.clone());
        let spawned = std::thread::Builder::new().name(format!("conn-{peer}")).spawn(move || {
            connection(peer, tcp, addr, cfg, &shared2);
            release(&shared2, addr.ip());
        });
        if spawned.is_err() {
            release(&shared, addr.ip());
        }
    }
    Ok(())
}

fn release(shared: &Shared, ip: IpAddr) {
    let mut total = shared.total.lock().unwrap_or_else(|e| e.into_inner());
    *total = total.saturating_sub(1);
    let mut per = shared.conns.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(n) = per.get_mut(&ip) {
        *n = n.saturating_sub(1);
        if *n == 0 {
            per.remove(&ip);
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

/// The lobby's limits on a connection: idle a while, it's closed (the game pings while you're in the menus).
const LOBBY: conn::Limits = conn::Limits { max_frame: proto::MAX_HERO + 1024, frames_per_sec: 150, bytes_per_sec: 64 * 1024, idle: Duration::from_secs(60) };

fn connection(peer: u32, tcp: TcpStream, addr: SocketAddr, cfg: Arc<rustls::ServerConfig>, shared: &Shared) {
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
    let (mut bad_logins, mut strikes) = (0u32, 0u32);
    let mut changes: Vec<Instant> = vec![];
    let stop = AtomicBool::new(false);
    let ended = conn::pump(&mut tls, LOBBY, &out_rx, &stop, |f| {
        if matches!(stage, Stage::Start) && started.elapsed() > HELLO_WITHIN {
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
                lobby(m, peer, addr.ip(), &mut stage, &mut bad_logins, &mut changes, &out_tx, &reply, shared)
            }
        }
    });
    if let Stage::Game { tx } = &stage {
        let why = match ended {
            Ended::Closed | Ended::Stopped => "LEFT",
            Ended::Idle => "TIMED OUT",
            Ended::Misbehaved(_) => "WAS DISCONNECTED",
            Ended::Io(_) => "LOST CONNECTION",
        };
        let _ = tx.send(RoomEvent::Gone { peer, why: why.into() });
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

/// One lobby message. False ends the connection.
#[allow(clippy::too_many_arguments)]
fn lobby(m: ToServer, peer: u32, ip: IpAddr, stage: &mut Stage, bad_logins: &mut u32, changes: &mut Vec<Instant>, out: &SyncSender<Vec<u8>>, reply: &dyn Fn(&FromServer), shared: &Shared) -> bool {
    let refuse = |why: &str| reply(&FromServer::Refused(why.to_string()));
    // Hero and game changes are limited (each new hero builds a world to make it).
    let mut change_ok = || {
        changes.retain(|t| t.elapsed() < Duration::from_secs(60));
        changes.push(Instant::now());
        changes.len() as u32 <= CHANGES_PER_MIN
    };
    match (m, &*stage) {
        (ToServer::Hello { protocol, version }, Stage::Start) => {
            if protocol != proto::PROTOCOL || version != env!("CARGO_PKG_VERSION") {
                refuse(&format!("VERSION MISMATCH: THE SERVER RUNS {}", env!("CARGO_PKG_VERSION")));
                return false;
            }
            let invite = shared.accounts.lock().unwrap_or_else(|e| e.into_inner()).needs_invite();
            reply(&FromServer::Hello { name: shared.cfg.name.clone(), invite });
            *stage = Stage::Lobby { who: None };
            true
        }
        (_, Stage::Start) => false,
        (ToServer::Register { name, password, invite }, Stage::Lobby { who: None }) => {
            match accounts::register(&shared.accounts, &name, &password, &invite, ip) {
                Ok(id) => {
                    log(&format!("new account {id} from {ip}"));
                    *stage = Stage::Lobby { who: Some((id, name.clone())) };
                    reply(&FromServer::LoggedIn { name });
                }
                Err(why) => refuse(why),
            }
            true
        }
        (ToServer::Login { name, password }, Stage::Lobby { who: None }) => match accounts::login(&shared.accounts, &name, &password, ip) {
            Ok((id, name)) => {
                log(&format!("account {id} logged in from {ip}"));
                *stage = Stage::Lobby { who: Some((id, name.clone())) };
                reply(&FromServer::LoggedIn { name });
                true
            }
            Err(why) => {
                *bad_logins += 1;
                refuse(why);
                *bad_logins < LOGIN_TRIES
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
            let slot = shared.accounts.lock().unwrap_or_else(|e| e.into_inner()).free_slot(id);
            let Some(slot) = slot else {
                refuse("ALL YOUR HERO SLOTS ARE FULL");
                return true;
            };
            let mut g = crate::game::Game::new(new_seed(), crate::gfx::SH_WIDE);
            g.set_class(class);
            g.hero_name = name;
            g.hardcore = hardcore;
            let text = crate::save::to_text(&g);
            if shared.accounts.lock().unwrap_or_else(|e| e.into_inner()).save_hero(id, slot, &text).is_err() {
                refuse("THE SERVER COULDN'T SAVE THAT");
            }
            reply(&FromServer::Heroes(heroes_of(shared, id)));
            true
        }
        (ToServer::DeleteHero { slot }, Stage::Lobby { who: Some((id, _)) }) => {
            let id = *id;
            if !change_ok() {
                refuse("SLOW DOWN");
                return true;
            }
            if in_use(shared, id, slot) {
                refuse("THAT HERO IS IN A GAME");
            } else if shared.accounts.lock().unwrap_or_else(|e| e.into_inner()).delete_hero(id, slot).is_err() {
                refuse("NO SUCH HERO");
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
                let mut rooms = shared.rooms.lock().unwrap_or_else(|e| e.into_inner());
                rooms.retain(|r| !r.ended.load(Ordering::Relaxed));
                if rooms.len() >= shared.cfg.max_games {
                    refuse("THE SERVER IS FULL OF GAMES. JOIN ONE");
                    return true;
                }
                let rid = shared.next_room.fetch_add(1, Ordering::Relaxed);
                let pw = (!password.is_empty()).then(|| tls::hash(password.as_bytes()));
                match room::spawn(rid, gname.clone(), pw, hardcore, act, shared.accounts.clone(), new_seed()) {
                    Ok(h) => rooms.push(h),
                    Err(_) => {
                        refuse("THE SERVER COULDN'T START A GAME");
                        return true;
                    }
                }
                rid
            };
            log(&format!("account {id} made game {rid}"));
            join(shared, stage, peer, id, &acct, hero, text, rid, &password, out, reply);
            true
        }
        (ToServer::JoinGame { game, password, hero }, Stage::Lobby { who: Some((id, acct)) }) => {
            let (id, acct) = (*id, acct.clone());
            let Some(text) = playable(shared, id, hero, &refuse) else { return true };
            join(shared, stage, peer, id, &acct, hero, text, game, &password, out, reply);
            true
        }
        // A second login, a hello again: not allowed.
        _ => false,
    }
}

/// A hero that can play: it exists, isn't fallen, and isn't in a game already.
fn playable(shared: &Shared, id: u64, slot: u8, refuse: &dyn Fn(&str)) -> Option<String> {
    let text = shared.accounts.lock().unwrap_or_else(|e| e.into_inner()).hero(id, slot);
    let Some(text) = text else {
        refuse("NO SUCH HERO");
        return None;
    };
    if text.lines().any(|l| l.trim() == "fallen=1") {
        refuse("THAT HERO HAS FALLEN");
        return None;
    }
    if in_use(shared, id, slot) {
        refuse("THAT HERO IS IN A GAME");
        return None;
    }
    Some(text)
}

fn in_use(shared: &Shared, id: u64, slot: u8) -> bool {
    shared.rooms.lock().unwrap_or_else(|e| e.into_inner()).iter().any(|r| !r.ended.load(Ordering::Relaxed) && r.heroes.lock().unwrap_or_else(|e| e.into_inner()).contains(&(id, slot)))
}

#[allow(clippy::too_many_arguments)]
fn join(shared: &Shared, stage: &mut Stage, peer: u32, id: u64, acct: &str, slot: u8, text: String, game: u32, password: &str, out: &SyncSender<Vec<u8>>, reply: &dyn Fn(&FromServer)) {
    let refuse = |why: &str| reply(&FromServer::Refused(why.to_string()));
    let hero_name = text.lines().find_map(|l| l.strip_prefix("name=")).and_then(proto::clean_name).unwrap_or_else(|| acct.to_ascii_uppercase());
    let hardcore = text.lines().any(|l| l.trim() == "hardcore=1");
    let tx = {
        let rooms = shared.rooms.lock().unwrap_or_else(|e| e.into_inner());
        let Some(r) = rooms.iter().find(|r| r.id == game && !r.ended.load(Ordering::Relaxed)) else {
            refuse("THAT GAME HAS ENDED");
            return;
        };
        let pw_ok = r.password.map_or(true, |h| tls::hash(password.as_bytes()).iter().zip(h.iter()).fold(0u8, |a, (x, y)| a | (x ^ y)) == 0);
        if !pw_ok {
            refuse("WRONG GAME PASSWORD");
            return;
        }
        if r.hardcore != hardcore {
            refuse(if r.hardcore { "A HARDCORE GAME: HARDCORE HEROES ONLY" } else { "HARDCORE HEROES PLAY IN HARDCORE GAMES" });
            return;
        }
        if r.players.load(Ordering::Relaxed) as usize >= room::MAX_PLAYERS {
            refuse("THE GAME IS FULL");
            return;
        }
        r.tx.clone()
    };
    let (rtx, rrx) = mpsc::sync_channel(1);
    let req = JoinReq { peer, account: id, slot, name: hero_name, hero: text, out: out.clone(), reply: rtx };
    if tx.send(RoomEvent::Join(req)).is_err() {
        refuse("THAT GAME HAS ENDED");
        return;
    }
    match rrx.recv_timeout(Duration::from_secs(10)) {
        Ok(Ok((you, host_name))) => {
            reply(&FromServer::Joined { you, host_name });
            *stage = Stage::Game { tx };
        }
        Ok(Err(why)) => refuse(&why),
        Err(_) => refuse("THE GAME DIDN'T ANSWER"),
    }
}

fn heroes_of(shared: &Shared, id: u64) -> Vec<ServerHero> {
    let list = shared.accounts.lock().unwrap_or_else(|e| e.into_inner()).heroes(id);
    list.into_iter()
        .filter_map(|(slot, text)| {
            let h = crate::save::hero_info(&slot.to_string(), &text)?;
            let class = crate::skills::ALL_CLASSES.iter().position(|c| *c == h.class)? as u8;
            Some(ServerHero { slot, name: h.name, class, clvl: h.clvl, act: h.act as u8, hardcore: h.hardcore, fallen: h.fallen })
        })
        .collect()
}

fn games(shared: &Shared) -> Vec<GameInfo> {
    let mut rooms = shared.rooms.lock().unwrap_or_else(|e| e.into_inner());
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
        assert!(matches!(probe.ask(&ToServer::Login { name: "rowan".into(), password: "a long password".into() }), Ok(FromServer::LoggedIn { .. })));
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

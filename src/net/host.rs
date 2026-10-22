//! Hosting: a listener on its own thread, and a thread per joiner (TLS, then `conn::pump`). The game thread
//! polls `Host::poll` for what's happened and sends with `Host::send`; nothing here touches the game.
//!
//! Limits: at most 7 joiners (8 heroes with the host), 3 connections per address, 4 still shaking hands.
//! A joiner must say hello within 10 s. Wrong game passwords from an address lock it out for a minute
//! after 5 tries.
#![forbid(unsafe_code)]

use super::conn::{self, Ended};
use super::proto::{self, ToClient, ToHost};
use super::tls;
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub const MAX_JOINERS: usize = 7;
const PER_ADDRESS: usize = 3;
const MAX_HANDSHAKING: usize = 4;
const HELLO_WITHIN: Duration = Duration::from_secs(10);
/// Frames waiting to go out to one joiner; a joiner this far behind is dropped.
const OUT_QUEUE: usize = 256;
const PASSWORD_TRIES: u32 = 5;
const LOCKOUT: Duration = Duration::from_secs(60);

/// What happened, for the game thread.
#[derive(Debug)]
pub enum Event {
    /// A joiner said hello (name already checked and cleaned; the password is checked here, not passed on).
    Hello { peer: u32, name: String, hero: String },
    Input { peer: u32, input: proto::NetInput },
    Chat { peer: u32, text: String },
    /// Gone, and why (shown to the others).
    Gone { peer: u32, why: String },
}

struct Shared {
    /// Connections per address, and how many are still shaking hands.
    per_ip: HashMap<IpAddr, usize>,
    handshaking: usize,
    joined: usize,
    /// Wrong passwords per address: (count, last try).
    bad_pw: HashMap<IpAddr, (u32, Instant)>,
}

pub struct Host {
    pub port: u16,
    pub fingerprint: [u8; 32],
    events: Receiver<Event>,
    links: Arc<Mutex<HashMap<u32, (SyncSender<Vec<u8>>, Arc<AtomicBool>)>>>,
    stop: Arc<AtomicBool>,
}

/// Constant-time equality of two passwords (compared as SHA-256 hashes, so length doesn't show either).
fn same_password(a: &str, b: &str) -> bool {
    let (x, y) = (tls::hash(a.as_bytes()), tls::hash(b.as_bytes()));
    x.iter().zip(y.iter()).fold(0u8, |acc, (p, q)| acc | (p ^ q)) == 0
}

impl Host {
    /// Starts listening on `port` (0 picks a free one, for tests). `password` empty means anyone may join.
    pub fn start(port: u16, password: String, dir: &std::path::Path) -> std::io::Result<Host> {
        let id = tls::identity(dir)?;
        let cfg = tls::server_config(&id).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        let listener = TcpListener::bind(("0.0.0.0", port))?;
        let port = listener.local_addr()?.port();
        listener.set_nonblocking(true)?;
        let (tx, rx) = mpsc::channel();
        let links = Arc::new(Mutex::new(HashMap::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let shared = Arc::new(Mutex::new(Shared { per_ip: HashMap::new(), handshaking: 0, joined: 0, bad_pw: HashMap::new() }));
        let next_id = Arc::new(AtomicU32::new(1));
        {
            let (links, stop) = (links.clone(), stop.clone());
            std::thread::Builder::new().name("host-listen".into()).spawn(move || {
                while !stop.load(Ordering::Relaxed) {
                    match listener.accept() {
                        Ok((tcp, addr)) => {
                            let ip = addr.ip();
                            {
                                let mut s = shared.lock().unwrap_or_else(|e| e.into_inner());
                                let here = *s.per_ip.get(&ip).unwrap_or(&0);
                                if s.joined + s.handshaking >= MAX_JOINERS + MAX_HANDSHAKING || s.handshaking >= MAX_HANDSHAKING || here >= PER_ADDRESS {
                                    drop(tcp);
                                    continue;
                                }
                                s.handshaking += 1;
                                *s.per_ip.entry(ip).or_insert(0) += 1;
                            }
                            let peer = next_id.fetch_add(1, Ordering::Relaxed);
                            let (cfg, tx, links, stop, shared, password) = (cfg.clone(), tx.clone(), links.clone(), stop.clone(), shared.clone(), password.clone());
                            let _ = std::thread::Builder::new().name(format!("host-peer-{peer}")).spawn(move || {
                                let why = serve(peer, tcp, addr, cfg, &tx, &links, &stop, &shared, &password);
                                links.lock().unwrap_or_else(|e| e.into_inner()).remove(&peer);
                                let _ = tx.send(Event::Gone { peer, why });
                            });
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => std::thread::sleep(Duration::from_millis(50)),
                        Err(_) => std::thread::sleep(Duration::from_millis(200)),
                    }
                }
            })?;
        }
        Ok(Host { port, fingerprint: id.fingerprint, events: rx, links, stop })
    }

    /// Everything that's happened since last time.
    pub fn poll(&self) -> Vec<Event> {
        self.events.try_iter().collect()
    }

    /// Sends a message to one joiner. One that can't keep up is dropped.
    pub fn send(&self, peer: u32, m: &ToClient) {
        let Some(bytes) = proto::encode(m) else { return };
        let links = self.links.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((tx, kick)) = links.get(&peer) {
            if let Err(TrySendError::Full(_)) = tx.try_send(bytes) {
                kick.store(true, Ordering::Relaxed);
            }
        }
    }

    /// Ends one joiner's connection (after a goodbye, if it can still be sent).
    pub fn kick(&self, peer: u32, why: &str) {
        self.send(peer, &ToClient::Bye(why.to_string()));
        if let Some((_, kick)) = self.links.lock().unwrap_or_else(|e| e.into_inner()).get(&peer) {
            // Give the goodbye a moment to go out: the peer thread checks the flag after writing.
            kick.store(true, Ordering::Relaxed);
        }
    }

    pub fn peers(&self) -> Vec<u32> {
        self.links.lock().unwrap_or_else(|e| e.into_inner()).keys().copied().collect()
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        let peers = self.peers();
        for p in peers {
            self.kick(p, "THE HOST HAS LEFT");
        }
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// One joiner, start to finish. Returns why it ended (for the others to read).
#[allow(clippy::too_many_arguments)]
fn serve(
    peer: u32,
    tcp: TcpStream,
    addr: SocketAddr,
    cfg: Arc<rustls::ServerConfig>,
    tx: &Sender<Event>,
    links: &Mutex<HashMap<u32, (SyncSender<Vec<u8>>, Arc<AtomicBool>)>>,
    stop: &AtomicBool,
    shared: &Mutex<Shared>,
    password: &str,
) -> String {
    let ip = addr.ip();
    let mut joined = false;
    let release = |joined: bool| {
        let mut s = shared.lock().unwrap_or_else(|e| e.into_inner());
        if joined {
            s.joined = s.joined.saturating_sub(1);
        } else {
            s.handshaking = s.handshaking.saturating_sub(1);
        }
        if let Some(n) = s.per_ip.get_mut(&ip) {
            *n = n.saturating_sub(1);
            if *n == 0 {
                s.per_ip.remove(&ip);
            }
        }
    };
    let setup = || -> std::io::Result<rustls::StreamOwned<rustls::ServerConnection, TcpStream>> {
        tcp.set_nonblocking(false)?;
        tcp.set_nodelay(true)?;
        tcp.set_write_timeout(Some(Duration::from_secs(5)))?;
        // The handshake must finish within the hello deadline.
        tcp.set_read_timeout(Some(HELLO_WITHIN))?;
        let conn = rustls::ServerConnection::new(cfg).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        let mut tls = rustls::StreamOwned::new(conn, tcp);
        while tls.conn.is_handshaking() {
            tls.conn.complete_io(&mut tls.sock)?;
        }
        tls.sock.set_read_timeout(Some(Duration::from_millis(10)))?;
        Ok(tls)
    };
    let mut tls = match setup() {
        Ok(t) => t,
        Err(_) => {
            release(false);
            return "COULDN'T CONNECT".into();
        }
    };
    let (out_tx, out_rx) = mpsc::sync_channel::<Vec<u8>>(OUT_QUEUE);
    let kick = Arc::new(AtomicBool::new(false));
    links.lock().unwrap_or_else(|e| e.into_inner()).insert(peer, (out_tx.clone(), kick.clone()));
    let started = Instant::now();
    let mut said_hello = false;
    let mut strikes = 0u32;
    // Flags the watcher can see: hello received, goodbye received, and "stop now".
    let (hello_seen, bye_seen, stop_either) = (AtomicBool::new(false), AtomicBool::new(false), AtomicBool::new(false));
    let ended = std::thread::scope(|sc| {
        // Stops the pump when the host stops, this joiner is kicked, or no hello came in time.
        let watcher = sc.spawn(|| {
            while !stop_either.load(Ordering::Relaxed) {
                let late = !hello_seen.load(Ordering::Relaxed) && started.elapsed() > HELLO_WITHIN;
                if stop.load(Ordering::Relaxed) || kick.load(Ordering::Relaxed) || late {
                    stop_either.store(true, Ordering::Relaxed);
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        });
        let ended = conn::pump(&mut tls, conn::FROM_JOINER, &out_rx, &stop_either, |f| {
            let Some(m) = proto::decode::<ToHost>(&f) else {
                strikes += 1;
                return strikes < 3;
            };
            match m {
                ToHost::Hello { protocol, version, name, hero, password: pw } if !said_hello => {
                    if started.elapsed() > HELLO_WITHIN {
                        return false;
                    }
                    // Locked out after too many wrong passwords.
                    {
                        let mut s = shared.lock().unwrap_or_else(|e| e.into_inner());
                        if let Some(&(n, t)) = s.bad_pw.get(&ip) {
                            if n >= PASSWORD_TRIES && t.elapsed() < LOCKOUT {
                                drop(s);
                                send_now(&out_tx, &ToClient::Refused("TOO MANY TRIES. WAIT A MINUTE".into()));
                                return false;
                            }
                        }
                        if !same_password(&pw, password) {
                            let e = s.bad_pw.entry(ip).or_insert((0, Instant::now()));
                            e.0 = if e.1.elapsed() > LOCKOUT { 1 } else { e.0 + 1 };
                            e.1 = Instant::now();
                            drop(s);
                            send_now(&out_tx, &ToClient::Refused("WRONG PASSWORD".into()));
                            return false;
                        }
                        s.bad_pw.remove(&ip);
                        if s.joined >= MAX_JOINERS {
                            drop(s);
                            send_now(&out_tx, &ToClient::Refused("THE GAME IS FULL".into()));
                            return false;
                        }
                        s.joined += 1;
                        s.handshaking = s.handshaking.saturating_sub(1);
                    }
                    joined = true;
                    match proto::check_hello(protocol, &version, &name, &hero) {
                        Ok(name) => {
                            said_hello = true;
                            hello_seen.store(true, Ordering::Relaxed);
                            let _ = tx.send(Event::Hello { peer, name, hero });
                            true
                        }
                        Err(why) => {
                            send_now(&out_tx, &ToClient::Refused(why));
                            false
                        }
                    }
                }
                // Nothing else before the hello, and no second hello.
                _ if !said_hello => false,
                ToHost::Hello { .. } => false,
                ToHost::Input(input) => tx.send(Event::Input { peer, input }).is_ok(),
                ToHost::Chat(text) => {
                    if let Some(text) = proto::clean_chat(&text) {
                        let _ = tx.send(Event::Chat { peer, text });
                    }
                    true
                }
                ToHost::Bye => {
                    bye_seen.store(true, Ordering::Relaxed);
                    false
                }
            }
        });
        stop_either.store(true, Ordering::Relaxed);
        let _ = watcher.join();
        ended
    });
    // Flush whatever's left (a goodbye), briefly.
    let _ = tls.sock.set_write_timeout(Some(Duration::from_millis(300)));
    while let Ok(f) = out_rx.try_recv() {
        if conn::Wire::send(&mut tls, &f).is_err() {
            break;
        }
    }
    tls.conn.send_close_notify();
    let _ = tls.conn.complete_io(&mut tls.sock);
    release(joined);
    if cfg!(test) {
        eprintln!("host peer {peer} ended: {ended:?}");
    }
    if bye_seen.load(Ordering::Relaxed) {
        return "LEFT".into();
    }
    match ended {
        Ended::Closed => "LEFT".into(),
        Ended::Idle => "TIMED OUT".into(),
        Ended::Misbehaved(_) => "WAS DISCONNECTED".into(),
        Ended::Io(_) => "LOST CONNECTION".into(),
        Ended::Stopped => "LEFT".into(),
    }
}

/// Queues a message for a joiner (used for refusals, which go out just before the connection closes).
fn send_now(tx: &SyncSender<Vec<u8>>, m: &ToClient) {
    if let Some(b) = proto::encode(m) {
        let _ = tx.try_send(b);
    }
}

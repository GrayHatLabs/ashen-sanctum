//! Joining: connect, check the host's key against the pinned one, say hello, then a thread moves frames
//! both ways. The game thread polls `Link::poll` and sends with `Link::send`.
#![forbid(unsafe_code)]

use super::conn::{self, Ended};
use super::proto::{self, ToClient, ToHost};
use super::tls;
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Why joining didn't work.
#[derive(Debug, Clone, PartialEq)]
pub enum JoinError {
    /// A host never met before: show this fingerprint and ask whether to trust it.
    Unknown([u8; 32]),
    /// The host's key isn't the one trusted before: refuse (someone may be standing in for it).
    Changed([u8; 32]),
    /// The host said no (wrong password, full, another version...).
    Refused(String),
    /// Couldn't reach it, or it went quiet.
    Unreachable(String),
}

pub struct Link {
    pub you: u8,
    pub host_name: String,
    inbound: Receiver<ToClient>,
    out: SyncSender<Vec<u8>>,
    stop: Arc<AtomicBool>,
    /// Set when the connection ends, with why.
    pub ended: Arc<Mutex<Option<String>>>,
}

/// "host" or "host:port" → an address (the default port if none is given).
pub fn resolve(addr: &str) -> Option<SocketAddr> {
    let addr = addr.trim();
    if addr.is_empty() || addr.len() > 255 {
        return None;
    }
    let with_port = if addr.parse::<SocketAddr>().is_ok() || addr.rsplit_once(':').map_or(false, |(_, p)| p.parse::<u16>().is_ok()) {
        addr.to_string()
    } else {
        format!("{addr}:{}", proto::DEFAULT_PORT)
    };
    with_port.to_socket_addrs().ok()?.next()
}

/// Looks a host up before joining: None if its key is already trusted, or the new key to ask about.
pub fn probe(addr: &str, dir: &Path) -> Result<Option<[u8; 32]>, JoinError> {
    let sa = resolve(addr).ok_or_else(|| JoinError::Unreachable("NO SUCH ADDRESS".into()))?;
    let pin = tls::known(dir, &tls::host_key(addr));
    let deadline = Instant::now() + Duration::from_secs(10);
    let (cfg, seen) = tls::client_config(pin).map_err(|e| JoinError::Unreachable(e.to_string()))?;
    let tcp = TcpStream::connect_timeout(&sa, Duration::from_secs(5)).map_err(|_| JoinError::Unreachable("NO ANSWER".into()))?;
    let _ = tcp.set_read_timeout(Some(Duration::from_secs(5)));
    let _ = tcp.set_write_timeout(Some(Duration::from_secs(5)));
    let conn = rustls::ClientConnection::new(cfg, tls::server_name()).map_err(|e| JoinError::Unreachable(e.to_string()))?;
    let mut s = rustls::StreamOwned::new(conn, tcp);
    let never = AtomicBool::new(false);
    {
        if tls::handshake(&mut s, deadline, &never).is_err() {
            let shown = seen.lock().ok().and_then(|g| *g);
            return match (pin, shown) {
                (None, Some(h)) => Ok(Some(h)),
                (Some(p), Some(h)) if p != h => Err(JoinError::Changed(h)),
                _ => Err(JoinError::Unreachable("THE CONNECTION FAILED".into())),
            };
        }
    }
    s.conn.send_close_notify();
    let _ = s.conn.complete_io(&mut s.sock);
    Ok(None)
}

/// What a joiner will accept from the host before the game thread gets to it; beyond that, the host is
/// sending more than any game needs (review 2026-10-22).
const INBOX: usize = 512;

/// The address as it's written in known_hosts.txt.
pub fn key_for(addr: &str) -> Option<String> {
    resolve(addr).map(|_| tls::host_key(addr))
}

/// Joins the game at `addr` with this hero (its save text). `dir` holds known_hosts.txt.
type Tls = rustls::StreamOwned<rustls::ClientConnection, TcpStream>;

/// Connects to a host or server and checks its key against the pinned one (TLS 1.3).
fn connect_tls(addr: &str, dir: &Path, deadline: Instant) -> Result<Tls, JoinError> {
    let sa = resolve(addr).ok_or_else(|| JoinError::Unreachable("NO SUCH ADDRESS".into()))?;
    let pin = tls::known(dir, &tls::host_key(addr));
    let never = AtomicBool::new(false);
    let (cfg, seen) = tls::client_config(pin).map_err(|e| JoinError::Unreachable(e.to_string()))?;
    let tcp = TcpStream::connect_timeout(&sa, Duration::from_secs(5)).map_err(|_| JoinError::Unreachable("NO ANSWER".into()))?;
    let io = |e: std::io::Error| JoinError::Unreachable(e.to_string());
    tcp.set_nodelay(true).map_err(io)?;
    tcp.set_read_timeout(Some(Duration::from_secs(10))).map_err(io)?;
    tcp.set_write_timeout(Some(Duration::from_secs(5))).map_err(io)?;
    let conn = rustls::ClientConnection::new(cfg, tls::server_name()).map_err(|e| JoinError::Unreachable(e.to_string()))?;
    let mut s = rustls::StreamOwned::new(conn, tcp);
    if tls::handshake(&mut s, deadline, &never).is_err() {
        let shown = seen.lock().ok().and_then(|g| *g);
        return Err(match (pin, shown) {
            (None, Some(h)) => JoinError::Unknown(h),
            (Some(p), Some(h)) if p != h => JoinError::Changed(h),
            _ => JoinError::Unreachable("THE CONNECTION FAILED".into()),
        });
    }
    Ok(s)
}

/// The next whole frame, by the deadline however slowly it comes.
fn next_frame(s: &mut Tls, reader: &mut conn::FrameReader, deadline: Instant) -> Result<Vec<u8>, JoinError> {
    let mut buf = vec![0u8; 16 * 1024];
    loop {
        if Instant::now() > deadline {
            return Err(JoinError::Unreachable("NO ANSWER".into()));
        }
        match reader.next() {
            Ok(Some(f)) => return Ok(f),
            Ok(None) => match conn::Wire::recv(s, &mut buf) {
                Ok(0) => return Err(JoinError::Unreachable("THE CONNECTION CLOSED".into())),
                Ok(n) => reader.feed(&buf[..n]),
                Err(e) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut | std::io::ErrorKind::Interrupted) => {}
                Err(_) => return Err(JoinError::Unreachable("THE CONNECTION CLOSED".into())),
            },
            Err(_) => return Err(JoinError::Unreachable("IT SENT NONSENSE".into())),
        }
    }
}

/// From here the connection carries the game: a thread moves frames both ways (what arrived with the last
/// reply, still in `reader`, is handled first).
fn start_link(mut s: Tls, reader: conn::FrameReader, you: u8, host_name: String) -> Result<Link, JoinError> {
    let io = |e: std::io::Error| JoinError::Unreachable(e.to_string());
    s.sock.set_read_timeout(Some(Duration::from_millis(10))).map_err(io)?;
    let (in_tx, in_rx) = mpsc::sync_channel(INBOX);
    let (out_tx, out_rx) = mpsc::sync_channel::<Vec<u8>>(256);
    let stop = Arc::new(AtomicBool::new(false));
    let ended = Arc::new(Mutex::new(None));
    {
        let (stop, ended) = (stop.clone(), ended.clone());
        std::thread::Builder::new()
            .name("join-link".into())
            .spawn(move || {
                let mut strikes = 0;
                let why = match conn::pump_from(&mut s, conn::FROM_HOST, &out_rx, &stop, reader, |f| match proto::decode::<ToClient>(&f) {
                    // Full: the host is sending far more than a game needs. End it.
                    Some(m) => in_tx.try_send(m).is_ok(),
                    None => {
                        strikes += 1;
                        strikes < 3
                    }
                }) {
                    Ended::Closed | Ended::Stopped => "DISCONNECTED",
                    Ended::Idle => "THE HOST STOPPED ANSWERING",
                    Ended::Misbehaved(_) => "THE HOST SENT NONSENSE",
                    Ended::Io(e) => {
                        if cfg!(test) {
                            eprintln!("join link: {e:?}");
                        }
                        "LOST CONNECTION"
                    }
                };
                s.conn.send_close_notify();
                let _ = s.conn.complete_io(&mut s.sock);
                if let Ok(mut e) = ended.lock() {
                    *e = Some(why.to_string());
                }
            })
            .map_err(io)?;
    }
    Ok(Link { you, host_name, inbound: in_rx, out: out_tx, stop, ended })
}

pub fn join(addr: &str, dir: &Path, name: &str, hero: &str, password: &str) -> Result<Link, JoinError> {
    let deadline = Instant::now() + Duration::from_secs(12);
    let mut s = connect_tls(addr, dir, deadline)?;
    let hello = ToHost::Hello { protocol: proto::PROTOCOL, version: env!("CARGO_PKG_VERSION").into(), name: name.into(), hero: hero.into(), password: password.into() };
    let bytes = proto::encode(&hello).ok_or_else(|| JoinError::Unreachable("BAD HERO".into()))?;
    conn::Wire::send(&mut s, &bytes).map_err(|e| JoinError::Unreachable(e.to_string()))?;
    // The answer: welcome, or why not.
    let mut reader = conn::FrameReader::new(super::frame::MAX_TO_CLIENT);
    let f = next_frame(&mut s, &mut reader, deadline)?;
    let (you, host_name) = match proto::decode::<ToClient>(&f) {
        Some(ToClient::Welcome { you, host_name }) if (1..=7).contains(&you) => (you, proto::clean_name(&host_name).unwrap_or_else(|| "HOST".into())),
        Some(ToClient::Refused(why)) | Some(ToClient::Bye(why)) => return Err(JoinError::Refused(proto::clean_chat(&why).unwrap_or_default())),
        _ => return Err(JoinError::Unreachable("THE HOST SENT NONSENSE".into())),
    };
    start_link(s, reader, you, host_name)
}

/// Connected to a login server: requests and their answers, one at a time.
pub struct Lobby {
    s: Tls,
    reader: conn::FrameReader,
    /// The server's name, and whether registering needs an invite code.
    pub server: String,
    pub invite: bool,
    last_sent: Instant,
}

/// Connects to a login server and says hello.
pub fn lobby(addr: &str, dir: &Path) -> Result<Lobby, JoinError> {
    let deadline = Instant::now() + Duration::from_secs(12);
    let s = connect_tls(addr, dir, deadline)?;
    let mut l = Lobby { s, reader: conn::FrameReader::new(super::frame::MAX_TO_CLIENT), server: String::new(), invite: false, last_sent: Instant::now() };
    match l.ask(&proto::ToServer::Hello { protocol: proto::PROTOCOL, version: env!("CARGO_PKG_VERSION").into() }) {
        Ok(proto::FromServer::Hello { name, invite }) => {
            l.server = proto::clean_game_name(&name).unwrap_or_else(|| "SERVER".into());
            l.invite = invite;
            Ok(l)
        }
        Ok(proto::FromServer::Refused(why)) => Err(JoinError::Refused(proto::clean_chat(&why).unwrap_or_default())),
        Ok(_) => Err(JoinError::Unreachable("IT SENT NONSENSE".into())),
        Err(why) => Err(JoinError::Unreachable(why)),
    }
}

impl Lobby {
    /// Sends a request and waits (up to 8 s) for its answer.
    pub fn ask(&mut self, m: &proto::ToServer) -> Result<proto::FromServer, String> {
        let b = proto::encode(m).ok_or_else(|| "TOO LONG".to_string())?;
        conn::Wire::send(&mut self.s, &b).map_err(|_| "LOST CONNECTION".to_string())?;
        self.last_sent = Instant::now();
        let f = next_frame(&mut self.s, &mut self.reader, Instant::now() + Duration::from_secs(8)).map_err(|e| match e {
            JoinError::Unreachable(w) | JoinError::Refused(w) => w,
            _ => "LOST CONNECTION".into(),
        })?;
        proto::decode::<proto::FromServer>(&f).ok_or_else(|| "THE SERVER SENT NONSENSE".to_string())
    }

    /// Keeps an idle connection open (the menus call it every frame; it sends at most every 15 s).
    pub fn keep_alive(&mut self) -> bool {
        if self.last_sent.elapsed() < Duration::from_secs(15) {
            return true;
        }
        self.last_sent = Instant::now();
        proto::encode(&proto::ToServer::Ping).map_or(false, |b| conn::Wire::send(&mut self.s, &b).is_ok())
    }

    /// Joins (or makes) a game: from then on the connection is the game's. On a refusal the lobby comes back.
    pub fn enter(mut self, m: &proto::ToServer) -> Result<Link, (Option<Lobby>, String)> {
        match self.ask(m) {
            Ok(proto::FromServer::Joined { you, host_name }) if you <= 7 => {
                let host = proto::clean_name(&host_name).unwrap_or_else(|| "HOST".into());
                let reader = std::mem::replace(&mut self.reader, conn::FrameReader::new(1));
                start_link(self.s, reader, you, host).map_err(|_| (None, "LOST CONNECTION".to_string()))
            }
            Ok(proto::FromServer::Refused(why)) => Err((Some(self), proto::clean_chat(&why).unwrap_or_default())),
            Ok(_) => Err((Some(self), "THE SERVER SENT NONSENSE".into())),
            Err(why) => Err((None, why)),
        }
    }
}

impl Link {
    pub fn poll(&self) -> Vec<ToClient> {
        self.inbound.try_iter().collect()
    }

    /// Sends a message; false if the connection is gone or hopelessly behind.
    pub fn send(&self, m: &ToHost) -> bool {
        proto::encode(m).map_or(false, |b| self.out.try_send(b).is_ok())
    }

    pub fn ended(&self) -> Option<String> {
        self.ended.lock().ok().and_then(|e| e.clone())
    }
}

impl Drop for Link {
    fn drop(&mut self) {
        let _ = self.send(&ToHost::Bye);
        // A moment for the goodbye to go out.
        std::thread::sleep(Duration::from_millis(30));
        self.stop.store(true, Ordering::Relaxed);
    }
}

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
    let pin = tls::known(dir, &sa.to_string());
    let (cfg, seen) = tls::client_config(pin).map_err(|e| JoinError::Unreachable(e.to_string()))?;
    let tcp = TcpStream::connect_timeout(&sa, Duration::from_secs(5)).map_err(|_| JoinError::Unreachable("NO ANSWER".into()))?;
    let _ = tcp.set_read_timeout(Some(Duration::from_secs(5)));
    let _ = tcp.set_write_timeout(Some(Duration::from_secs(5)));
    let conn = rustls::ClientConnection::new(cfg, tls::server_name()).map_err(|e| JoinError::Unreachable(e.to_string()))?;
    let mut s = rustls::StreamOwned::new(conn, tcp);
    while s.conn.is_handshaking() {
        if s.conn.complete_io(&mut s.sock).is_err() {
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

/// The address as it's written in known_hosts.txt.
pub fn key_for(addr: &str) -> Option<String> {
    resolve(addr).map(|a| a.to_string())
}

/// Joins the game at `addr` with this hero (its save text). `dir` holds known_hosts.txt.
pub fn join(addr: &str, dir: &Path, name: &str, hero: &str, password: &str) -> Result<Link, JoinError> {
    let sa = resolve(addr).ok_or_else(|| JoinError::Unreachable("NO SUCH ADDRESS".into()))?;
    let key = sa.to_string();
    let pin = tls::known(dir, &key);
    let (cfg, seen) = tls::client_config(pin).map_err(|e| JoinError::Unreachable(e.to_string()))?;
    let tcp = TcpStream::connect_timeout(&sa, Duration::from_secs(5)).map_err(|_| JoinError::Unreachable("NO ANSWER".into()))?;
    let io = |e: std::io::Error| JoinError::Unreachable(e.to_string());
    tcp.set_nodelay(true).map_err(io)?;
    tcp.set_read_timeout(Some(Duration::from_secs(10))).map_err(io)?;
    tcp.set_write_timeout(Some(Duration::from_secs(5))).map_err(io)?;
    let conn = rustls::ClientConnection::new(cfg, tls::server_name()).map_err(|e| JoinError::Unreachable(e.to_string()))?;
    let mut tls = rustls::StreamOwned::new(conn, tcp);
    while tls.conn.is_handshaking() {
        if tls.conn.complete_io(&mut tls.sock).is_err() {
            let shown = seen.lock().ok().and_then(|s| *s);
            return Err(match (pin, shown) {
                (None, Some(h)) => JoinError::Unknown(h),
                (Some(p), Some(h)) if p != h => JoinError::Changed(h),
                _ => JoinError::Unreachable("THE CONNECTION FAILED".into()),
            });
        }
    }
    let hello = ToHost::Hello { protocol: proto::PROTOCOL, version: env!("CARGO_PKG_VERSION").into(), name: name.into(), hero: hero.into(), password: password.into() };
    let bytes = proto::encode(&hello).ok_or_else(|| JoinError::Unreachable("BAD HERO".into()))?;
    super::frame::write_frame(&mut tls, &bytes).map_err(io)?;
    // The answer: welcome, or why not.
    let deadline = Instant::now() + Duration::from_secs(10);
    let (you, host_name) = loop {
        if Instant::now() > deadline {
            return Err(JoinError::Unreachable("NO ANSWER".into()));
        }
        let f = super::frame::read_frame(&mut tls, super::frame::MAX_TO_CLIENT).map_err(|_| JoinError::Unreachable("THE HOST CLOSED THE CONNECTION".into()))?;
        match proto::decode::<ToClient>(&f) {
            Some(ToClient::Welcome { you, host_name }) if (1..=7).contains(&you) => break (you, proto::clean_name(&host_name).unwrap_or_else(|| "HOST".into())),
            Some(ToClient::Refused(why)) | Some(ToClient::Bye(why)) => return Err(JoinError::Refused(proto::clean_chat(&why).unwrap_or_default())),
            _ => return Err(JoinError::Unreachable("THE HOST SENT NONSENSE".into())),
        }
    };
    tls.sock.set_read_timeout(Some(Duration::from_millis(10))).map_err(io)?;
    let (in_tx, in_rx) = mpsc::channel();
    let (out_tx, out_rx) = mpsc::sync_channel::<Vec<u8>>(256);
    let stop = Arc::new(AtomicBool::new(false));
    let ended = Arc::new(Mutex::new(None));
    {
        let (stop, ended) = (stop.clone(), ended.clone());
        std::thread::Builder::new()
            .name("join-link".into())
            .spawn(move || {
                let mut strikes = 0;
                let why = match conn::pump(&mut tls, conn::FROM_HOST, &out_rx, &stop, |f| match proto::decode::<ToClient>(&f) {
                    Some(m) => in_tx.send(m).is_ok(),
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
                tls.conn.send_close_notify();
                let _ = tls.conn.complete_io(&mut tls.sock);
                if let Ok(mut e) = ended.lock() {
                    *e = Some(why.to_string());
                }
            })
            .map_err(io)?;
    }
    Ok(Link { you, host_name, inbound: in_rx, out: out_tx, stop, ended })
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

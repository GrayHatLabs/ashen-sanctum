//! Accounts and their heroes, on the server's disk (docs/ONLINE_PLAN.md, phase 3).
//!
//! - Passwords are hashed with Argon2id (OWASP settings, a random salt from the OS); only the hash is kept.
//! - A login for a name that doesn't exist still costs a full hash check, and the answer is the same either
//!   way, so names can't be found by asking or by timing.
//! - Failed logins lock the account, and the address, for a while (growing with repeats); sign-ups are limited
//!   per address and in all. At most two hashes run at once, so a flood of logins can't eat the CPU.
//! - Files are keyed by account number, never by name (no path tricks), and written atomically (a crash mid-write
//!   leaves the old file). A deleted hero is moved to `deleted/`, not erased.
#![forbid(unsafe_code)]

use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::{Algorithm, Argon2, Params, Version};
use std::collections::HashMap;
use std::io;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::{Condvar, Mutex};
use std::time::{Duration, Instant, SystemTime};

pub const MIN_PASSWORD: usize = 8;
pub const MAX_PASSWORD: usize = 128;
/// Heroes per account.
pub const HERO_SLOTS: u8 = 8;
/// Wrong passwords before an account is locked, and for how long at first (it doubles with repeats, to 15 min).
const TRIES: u32 = 5;
const LOCK: Duration = Duration::from_secs(60);
const MAX_LOCK: Duration = Duration::from_secs(15 * 60);
/// Failed logins from one address in ten minutes before it's locked out too.
const IP_TRIES: u32 = 20;
const IP_WINDOW: Duration = Duration::from_secs(600);
/// New accounts per address an hour, and in all.
const SIGNUPS_PER_IP: usize = 3;
const SIGNUPS_PER_HOUR: usize = 30;
/// The one answer for a failed login, whatever the reason.
pub const BAD_LOGIN: &str = "WRONG NAME OR PASSWORD";

/// Argon2id with OWASP's settings (19 MiB, 2 passes, 1 lane).
fn hasher() -> Argon2<'static> {
    let params = Params::new(19 * 1024, 2, 1, None).expect("valid argon2 params");
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
}

/// At most two hashes at a time.
struct Gate {
    busy: Mutex<usize>,
    free: Condvar,
}

static GATE: Gate = Gate { busy: Mutex::new(0), free: Condvar::new() };

fn gated<R>(f: impl FnOnce() -> R) -> R {
    {
        let mut n = GATE.busy.lock().unwrap_or_else(|e| e.into_inner());
        while *n >= 2 {
            n = GATE.free.wait(n).unwrap_or_else(|e| e.into_inner());
        }
        *n += 1;
    }
    let r = f();
    *GATE.busy.lock().unwrap_or_else(|e| e.into_inner()) -= 1;
    GATE.free.notify_one();
    r
}

/// Random bytes from the OS (the TLS library's secure source, ring underneath).
pub fn random(b: &mut [u8]) -> bool {
    rustls::crypto::ring::default_provider().secure_random.fill(b).is_ok()
}

/// A random salt.
fn salt() -> Option<SaltString> {
    let mut b = [0u8; 16];
    if !random(&mut b) {
        return None;
    }
    SaltString::encode_b64(&b).ok()
}

pub fn hash_password(pw: &str) -> Option<String> {
    let s = salt()?;
    gated(|| hasher().hash_password(pw.as_bytes(), &s).ok().map(|h| h.to_string()))
}

pub fn check_password(pw: &str, stored: &str) -> bool {
    let Ok(h) = PasswordHash::new(stored) else { return false };
    gated(|| hasher().verify_password(pw.as_bytes(), &h).is_ok())
}

/// A hash to check against when the name doesn't exist (so it takes as long as a real one).
fn decoy() -> &'static str {
    static D: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    D.get_or_init(|| hash_password("not a real password, just the same work").unwrap_or_default())
}

/// Account names: 3-16 of A-Z a-z 0-9 _ -. Compared without case.
pub fn name_ok(n: &str) -> bool {
    (3..=16).contains(&n.len()) && n.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Passwords: 8-128 bytes, no control characters.
pub fn password_ok(p: &str) -> bool {
    (MIN_PASSWORD..=MAX_PASSWORD).contains(&p.len()) && !p.chars().any(|c| c.is_control())
}

#[derive(Clone)]
struct User {
    id: u64,
    name: String,
    hash: String,
}

/// Failed tries: how many, the last, and how long the current lock is.
#[derive(Default, Clone, Copy)]
struct Fails {
    n: u32,
    last: Option<Instant>,
    lock: Duration,
}

pub struct Accounts {
    dir: PathBuf,
    users: Vec<User>,
    next_id: u64,
    fails: HashMap<String, Fails>,
    ip_fails: HashMap<IpAddr, (u32, Instant)>,
    signups: Vec<(IpAddr, Instant)>,
    /// If set, registering needs this code.
    invite: Option<String>,
}

/// Writes a file all at once: to a temporary name, synced, then renamed over the old one.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    use std::io::Write;
    let tmp = path.with_extension("tmp");
    {
        let mut o = std::fs::OpenOptions::new();
        o.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            o.mode(0o600);
        }
        let mut f = o.open(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
    }
    std::fs::rename(&tmp, path)
}

impl Accounts {
    /// Opens (or starts) the account store in `dir`. `invite`: a code needed to register, if any.
    pub fn open(dir: &Path, invite: Option<String>) -> io::Result<Accounts> {
        std::fs::create_dir_all(dir.join("heroes"))?;
        let mut users = vec![];
        if let Ok(text) = std::fs::read_to_string(dir.join("users.txt")) {
            for l in text.lines() {
                let mut f = l.split('\t');
                if let (Some(id), Some(name), Some(hash)) = (f.next(), f.next(), f.next()) {
                    if let Ok(id) = id.parse::<u64>() {
                        if name_ok(name) && hash.starts_with("$argon2id$") {
                            users.push(User { id, name: name.to_string(), hash: hash.to_string() });
                        }
                    }
                }
            }
        }
        let next_id = users.iter().map(|u| u.id).max().unwrap_or(0) + 1;
        let invite = invite.filter(|c| !c.is_empty());
        Ok(Accounts { dir: dir.to_path_buf(), users, next_id, fails: HashMap::new(), ip_fails: HashMap::new(), signups: vec![], invite })
    }

    pub fn needs_invite(&self) -> bool {
        self.invite.is_some()
    }

    fn save_users(&self) -> io::Result<()> {
        let text: String = self.users.iter().map(|u| format!("{}\t{}\t{}\n", u.id, u.name, u.hash)).collect();
        write_atomic(&self.dir.join("users.txt"), text.as_bytes())
    }

    fn ip_locked(&self, ip: IpAddr) -> bool {
        self.ip_fails.get(&ip).map_or(false, |&(n, t)| n >= IP_TRIES && t.elapsed() < IP_WINDOW)
    }

    fn note_fail(&mut self, key: &str, ip: IpAddr) {
        let f = self.fails.entry(key.to_string()).or_default();
        f.n += 1;
        f.last = Some(Instant::now());
        if f.n >= TRIES {
            f.lock = if f.lock.is_zero() { LOCK } else { (f.lock * 2).min(MAX_LOCK) };
        }
        let e = self.ip_fails.entry(ip).or_insert((0, Instant::now()));
        if e.1.elapsed() > IP_WINDOW {
            *e = (0, Instant::now());
        }
        e.0 += 1;
        e.1 = Instant::now();
        // Old entries go, so the maps can't grow without end.
        self.fails.retain(|_, f| f.last.map_or(false, |t| t.elapsed() < MAX_LOCK * 2));
        self.ip_fails.retain(|_, (_, t)| t.elapsed() < IP_WINDOW);
    }

    fn name_locked(&self, key: &str) -> bool {
        self.fails.get(key).map_or(false, |f| f.n >= TRIES && f.last.map_or(false, |t| t.elapsed() < f.lock))
    }
}

/// Registers a new account. The store is locked only around reading and writing it, never while hashing.
pub fn register(store: &Mutex<Accounts>, name: &str, password: &str, invite: &str, ip: IpAddr) -> Result<u64, &'static str> {
    if !name_ok(name) {
        return Err("NAMES ARE 3-16 LETTERS, NUMBERS, _ OR -");
    }
    if !password_ok(password) {
        return Err("PASSWORDS ARE 8-128 CHARACTERS");
    }
    {
        let mut a = store.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(code) = &a.invite {
            // Compared in constant time, as hashes (so the length doesn't show either).
            let differs = crate::net::tls::hash(code.as_bytes()).iter().zip(crate::net::tls::hash(invite.as_bytes()).iter()).fold(0u8, |acc, (x, y)| acc | (x ^ y));
            if differs != 0 {
                return Err("THAT INVITE CODE ISN'T RIGHT");
            }
        }
        a.signups.retain(|(_, t)| t.elapsed() < Duration::from_secs(3600));
        if a.signups.iter().filter(|(i, _)| *i == ip).count() >= SIGNUPS_PER_IP || a.signups.len() >= SIGNUPS_PER_HOUR {
            return Err("TOO MANY NEW ACCOUNTS. TRY AGAIN LATER");
        }
        if a.users.iter().any(|u| u.name.eq_ignore_ascii_case(name)) {
            return Err("THAT NAME IS TAKEN");
        }
    }
    let hash = hash_password(password).ok_or("THE SERVER COULDN'T SAVE THAT")?;
    let mut a = store.lock().unwrap_or_else(|e| e.into_inner());
    if a.users.iter().any(|u| u.name.eq_ignore_ascii_case(name)) {
        return Err("THAT NAME IS TAKEN");
    }
    let id = a.next_id;
    a.next_id += 1;
    a.users.push(User { id, name: name.to_string(), hash });
    a.signups.push((ip, Instant::now()));
    if a.save_users().is_err() {
        a.users.pop();
        return Err("THE SERVER COULDN'T SAVE THAT");
    }
    let _ = std::fs::create_dir_all(a.dir.join("heroes").join(id.to_string()));
    Ok(id)
}

/// Logs in: the account's number and name, or the one answer for anything wrong.
pub fn login(store: &Mutex<Accounts>, name: &str, password: &str, ip: IpAddr) -> Result<(u64, String), &'static str> {
    let key = name.to_ascii_lowercase();
    let user = {
        let a = store.lock().unwrap_or_else(|e| e.into_inner());
        if a.ip_locked(ip) || a.name_locked(&key) {
            return Err("TOO MANY TRIES. WAIT A WHILE");
        }
        if !name_ok(name) || password.len() > MAX_PASSWORD {
            None
        } else {
            a.users.iter().find(|u| u.name.eq_ignore_ascii_case(name)).cloned()
        }
    };
    // The same work whether or not the name exists.
    let ok = match &user {
        Some(u) => check_password(password, &u.hash),
        None => {
            let _ = check_password(password, decoy());
            false
        }
    };
    let mut a = store.lock().unwrap_or_else(|e| e.into_inner());
    match (ok, user) {
        (true, Some(u)) => {
            a.fails.remove(&key);
            Ok((u.id, u.name))
        }
        _ => {
            a.note_fail(&key, ip);
            Err(BAD_LOGIN)
        }
    }
}

impl Accounts {
    fn hero_dir(&self, id: u64) -> PathBuf {
        self.dir.join("heroes").join(id.to_string())
    }

    /// Every hero of an account: (slot, save text).
    pub fn heroes(&self, id: u64) -> Vec<(u8, String)> {
        (0..HERO_SLOTS).filter_map(|s| self.hero(id, s).map(|t| (s, t))).collect()
    }

    pub fn hero(&self, id: u64, slot: u8) -> Option<String> {
        if slot >= HERO_SLOTS {
            return None;
        }
        let p = self.hero_dir(id).join(format!("{slot}.txt"));
        let len = std::fs::metadata(&p).ok()?.len();
        if len as usize > crate::net::proto::MAX_HERO {
            return None;
        }
        std::fs::read_to_string(p).ok()
    }

    pub fn save_hero(&self, id: u64, slot: u8, text: &str) -> io::Result<()> {
        if slot >= HERO_SLOTS || text.len() > crate::net::proto::MAX_HERO {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "bad hero"));
        }
        let d = self.hero_dir(id);
        std::fs::create_dir_all(&d)?;
        write_atomic(&d.join(format!("{slot}.txt")), text.as_bytes())
    }

    /// Moves a hero to `deleted/` (kept, not erased).
    pub fn delete_hero(&self, id: u64, slot: u8) -> io::Result<()> {
        if slot >= HERO_SLOTS {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "bad slot"));
        }
        let d = self.hero_dir(id);
        let gone = d.join("deleted");
        std::fs::create_dir_all(&gone)?;
        let stamp = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map_or(0, |t| t.as_secs());
        std::fs::rename(d.join(format!("{slot}.txt")), gone.join(format!("{slot}-{stamp}.txt")))
    }

    /// A free hero slot.
    pub fn free_slot(&self, id: u64) -> Option<u8> {
        (0..HERO_SLOTS).find(|&s| !self.hero_dir(id).join(format!("{s}.txt")).exists())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(n: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("ashen_accounts_{n}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn register_log_in_and_the_hash_is_argon2id() {
        let d = tmp("basic");
        let store = Mutex::new(Accounts::open(&d, None).unwrap());
        let ip: IpAddr = "10.0.0.1".parse().unwrap();
        let id = register(&store, "Rowan", "correct horse battery", "", ip).unwrap();
        assert_eq!(register(&store, "ROWAN", "another password", "", ip), Err("THAT NAME IS TAKEN"));
        assert_eq!(register(&store, "../etc", "correct horse battery", "", ip), Err("NAMES ARE 3-16 LETTERS, NUMBERS, _ OR -"));
        assert_eq!(register(&store, "Short", "short", "", ip), Err("PASSWORDS ARE 8-128 CHARACTERS"));
        assert_eq!(login(&store, "rowan", "correct horse battery", ip), Ok((id, "Rowan".to_string())));
        // Wrong password and unknown name: the same answer.
        assert_eq!(login(&store, "rowan", "wrong password!", ip), Err(BAD_LOGIN));
        assert_eq!(login(&store, "nobody", "whatever123", ip), Err(BAD_LOGIN));
        // Only the hash is on disk, and it's Argon2id; it survives a restart.
        let text = std::fs::read_to_string(d.join("users.txt")).unwrap();
        assert!(text.contains("$argon2id$") && !text.contains("correct horse"));
        let again = Mutex::new(Accounts::open(&d, None).unwrap());
        assert!(login(&again, "Rowan", "correct horse battery", ip).is_ok());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn guessing_locks_the_account_and_signups_are_limited() {
        let d = tmp("locks");
        let store = Mutex::new(Accounts::open(&d, None).unwrap());
        let ip: IpAddr = "10.0.0.2".parse().unwrap();
        register(&store, "Isolde", "the real password", "", ip).unwrap();
        for _ in 0..TRIES {
            assert_eq!(login(&store, "isolde", "a bad guess", ip), Err(BAD_LOGIN));
        }
        assert_eq!(login(&store, "isolde", "the real password", ip), Err("TOO MANY TRIES. WAIT A WHILE"), "locked even with the right one");
        // Another address can't make more than three accounts an hour.
        let other: IpAddr = "10.0.0.3".parse().unwrap();
        for n in ["Ada1", "Ada2", "Ada3"] {
            register(&store, n, "password123", "", other).unwrap();
        }
        assert_eq!(register(&store, "Ada4", "password123", "", other), Err("TOO MANY NEW ACCOUNTS. TRY AGAIN LATER"));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn an_invite_code_guards_registering() {
        let d = tmp("invite");
        let store = Mutex::new(Accounts::open(&d, Some("OPEN SESAME".into())).unwrap());
        let ip: IpAddr = "10.0.0.4".parse().unwrap();
        assert_eq!(register(&store, "Brynja", "password123", "guess", ip), Err("THAT INVITE CODE ISN'T RIGHT"));
        assert!(register(&store, "Brynja", "password123", "OPEN SESAME", ip).is_ok());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn heroes_are_kept_per_account_and_deleting_keeps_a_copy() {
        let d = tmp("heroes");
        let a = Accounts::open(&d, None).unwrap();
        a.save_hero(7, 0, "name=ROWAN\n").unwrap();
        assert_eq!(a.hero(7, 0).as_deref(), Some("name=ROWAN\n"));
        assert_eq!(a.hero(8, 0), None, "another account's slot is its own");
        assert_eq!(a.free_slot(7), Some(1));
        assert!(a.save_hero(7, 99, "x").is_err());
        a.delete_hero(7, 0).unwrap();
        assert_eq!(a.hero(7, 0), None);
        assert_eq!(std::fs::read_dir(d.join("heroes/7/deleted")).unwrap().count(), 1, "kept in deleted/");
        let _ = std::fs::remove_dir_all(&d);
    }
}

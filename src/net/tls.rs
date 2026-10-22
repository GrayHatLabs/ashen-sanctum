//! TLS 1.3 for every connection (rustls with ring; no other version, no plaintext fallback).
//!
//! A host has its own key and certificate, made on first use and kept in the save folder (the key readable
//! only by you). Joiners pin it like SSH: the first time, the game shows the host's fingerprint and asks;
//! once trusted, it's remembered in `known_hosts.txt`, and a host that later shows a different key is
//! refused. The pin is checked *and* the host must prove it holds the key (the TLS 1.3 handshake
//! signature is verified as usual), so nobody in between can stand in for it.
#![forbid(unsafe_code)]

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName, UnixTime};
use rustls::{DigitallySignedStruct, SignatureScheme};
use sha2::{Digest, Sha256};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// The name in the host's certificate (joiners connect by address; this is only the label it's issued to).
pub const HOST_NAME: &str = "ashen-sanctum.host";
/// known_hosts.txt is small; anything bigger is ignored rather than read.
const MAX_KNOWN: u64 = 256 * 1024;

fn provider() -> Arc<rustls::crypto::CryptoProvider> {
    Arc::new(rustls::crypto::ring::default_provider())
}

/// SHA-256 of a certificate.
pub fn hash(cert: &[u8]) -> [u8; 32] {
    Sha256::digest(cert).into()
}

/// The fingerprint as players read it: the first 12 bytes in six groups (AB12 CD34 ...).
pub fn show(h: &[u8; 32]) -> String {
    h[..12].chunks(2).map(|c| format!("{:02X}{:02X}", c[0], c[1])).collect::<Vec<_>>().join(" ")
}

fn hex(h: &[u8; 32]) -> String {
    h.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(s: &str) -> Option<[u8; 32]> {
    let s = s.trim();
    if s.len() != 64 || !s.is_ascii() {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, o) in out.iter_mut().enumerate() {
        *o = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
}

/// The host's own certificate and key.
pub struct Identity {
    pub cert: CertificateDer<'static>,
    key: Vec<u8>,
    pub fingerprint: [u8; 32],
}

/// Writes a file only its owner can read (the private key).
fn write_private(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut o = std::fs::OpenOptions::new();
    o.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        o.mode(0o600);
    }
    use std::io::Write;
    let mut f = o.open(path)?;
    f.write_all(bytes)?;
    f.sync_all()
}

/// Loads the host's key and certificate from `dir`, or makes them the first time.
pub fn identity(dir: &Path) -> io::Result<Identity> {
    let (cp, kp) = (dir.join("host_cert.der"), dir.join("host_key.der"));
    if let (Ok(cert), Ok(key)) = (std::fs::read(&cp), std::fs::read(&kp)) {
        if !cert.is_empty() && !key.is_empty() && cert.len() < 64 * 1024 && key.len() < 64 * 1024 {
            let fingerprint = hash(&cert);
            return Ok(Identity { cert: CertificateDer::from(cert), key, fingerprint });
        }
    }
    std::fs::create_dir_all(dir)?;
    let made = rcgen::generate_simple_self_signed(vec![HOST_NAME.to_string()]).map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
    let cert = made.cert.der().to_vec();
    let key = made.key_pair.serialize_der();
    write_private(&kp, &key)?;
    std::fs::write(&cp, &cert)?;
    let fingerprint = hash(&cert);
    Ok(Identity { cert: CertificateDer::from(cert), key, fingerprint })
}

/// The host's TLS settings: TLS 1.3 only, its own certificate, no client certificates.
pub fn server_config(id: &Identity) -> Result<Arc<rustls::ServerConfig>, rustls::Error> {
    let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(id.key.clone()));
    let cfg = rustls::ServerConfig::builder_with_provider(provider())
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .with_no_client_auth()
        .with_single_cert(vec![id.cert.clone()], key)?;
    Ok(Arc::new(cfg))
}

/// Accepts exactly one pinned host key (or, with no pin, none: it only notes the key it was shown,
/// so the player can be asked about it).
#[derive(Debug)]
pub struct Pin {
    expected: Option<[u8; 32]>,
    /// The key the host showed (filled in during the handshake).
    pub seen: Arc<Mutex<Option<[u8; 32]>>>,
    algs: rustls::crypto::WebPkiSupportedAlgorithms,
}

impl ServerCertVerifier for Pin {
    fn verify_server_cert(&self, end_entity: &CertificateDer<'_>, _: &[CertificateDer<'_>], _: &ServerName<'_>, _: &[u8], _: UnixTime) -> Result<ServerCertVerified, rustls::Error> {
        let h = hash(end_entity.as_ref());
        if let Ok(mut s) = self.seen.lock() {
            *s = Some(h);
        }
        match self.expected {
            Some(e) if e == h => Ok(ServerCertVerified::assertion()),
            _ => Err(rustls::Error::General("host key not trusted".into())),
        }
    }

    fn verify_tls12_signature(&self, _: &[u8], _: &CertificateDer<'_>, _: &DigitallySignedStruct) -> Result<HandshakeSignatureValid, rustls::Error> {
        Err(rustls::Error::General("TLS 1.2 is not allowed".into()))
    }

    fn verify_tls13_signature(&self, message: &[u8], cert: &CertificateDer<'_>, dss: &DigitallySignedStruct) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(message, cert, dss, &self.algs)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.algs.supported_schemes()
    }
}

/// A joiner's TLS settings for one host: TLS 1.3 only, that host's key pinned (or none yet).
pub fn client_config(pin: Option<[u8; 32]>) -> Result<(Arc<rustls::ClientConfig>, Arc<Mutex<Option<[u8; 32]>>>), rustls::Error> {
    let p = provider();
    let seen = Arc::new(Mutex::new(None));
    let verifier = Pin { expected: pin, seen: seen.clone(), algs: p.signature_verification_algorithms };
    let cfg = rustls::ClientConfig::builder_with_provider(p)
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(verifier))
        .with_no_client_auth();
    Ok((Arc::new(cfg), seen))
}

pub fn server_name() -> ServerName<'static> {
    ServerName::try_from(HOST_NAME).expect("a valid name")
}

/// Hosts trusted before: "address hash" lines in the save folder.
pub fn known_path(dir: &Path) -> PathBuf {
    dir.join("known_hosts.txt")
}

/// The pinned key for this address, if any.
pub fn known(dir: &Path, addr: &str) -> Option<[u8; 32]> {
    let p = known_path(dir);
    if std::fs::metadata(&p).ok()?.len() > MAX_KNOWN {
        return None;
    }
    let text = std::fs::read_to_string(p).ok()?;
    text.lines().find_map(|l| {
        let (a, h) = l.split_once(' ')?;
        (a == addr).then(|| unhex(h)).flatten()
    })
}

/// Remembers (or replaces) the key for this address.
pub fn trust(dir: &Path, addr: &str, h: &[u8; 32]) -> io::Result<()> {
    if addr.is_empty() || addr.contains(char::is_whitespace) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "bad address"));
    }
    std::fs::create_dir_all(dir)?;
    let p = known_path(dir);
    let old = if std::fs::metadata(&p).map_or(0, |m| m.len()) <= MAX_KNOWN { std::fs::read_to_string(&p).unwrap_or_default() } else { String::new() };
    let mut text: String = old.lines().filter(|l| l.split_once(' ').map_or(false, |(a, _)| a != addr)).map(|l| format!("{l}\n")).collect();
    text += &format!("{addr} {}\n", hex(h));
    std::fs::write(p, text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_hosts_remember_and_replace() {
        let dir = std::env::temp_dir().join(format!("ashen_known_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let (a, b) = ([1u8; 32], [2u8; 32]);
        assert_eq!(known(&dir, "10.0.0.5:4747"), None);
        trust(&dir, "10.0.0.5:4747", &a).unwrap();
        trust(&dir, "host.example:4747", &b).unwrap();
        assert_eq!(known(&dir, "10.0.0.5:4747"), Some(a));
        trust(&dir, "10.0.0.5:4747", &b).unwrap();
        assert_eq!(known(&dir, "10.0.0.5:4747"), Some(b));
        assert_eq!(known(&dir, "host.example:4747"), Some(b));
        assert!(trust(&dir, "bad addr", &a).is_err());
        assert_eq!(show(&[0xab; 32]), "ABAB ABAB ABAB ABAB ABAB ABAB");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_host_identity_is_made_once_and_kept() {
        let dir = std::env::temp_dir().join(format!("ashen_id_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let one = identity(&dir).unwrap();
        let two = identity(&dir).unwrap();
        assert_eq!(one.fingerprint, two.fingerprint);
        assert!(server_config(&one).is_ok());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(dir.join("host_key.der")).unwrap().permissions().mode();
            assert_eq!(mode & 0o077, 0, "the key is private");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}

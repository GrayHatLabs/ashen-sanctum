//! Online play (docs/ONLINE_PLAN.md, phase 2): one player hosts, friends join over TCP.
//!
//! The host runs the world; joiners send only what they want to do (`proto::NetInput`) and get back what
//! they can see. Everything that arrives from the network is untrusted: frames are size-capped before
//! anything is allocated (`frame`), decoding never panics (`proto`), and every input is checked and clamped
//! before the game sees it. The connection is TLS 1.3 (`tls`).
#![forbid(unsafe_code)]

pub mod frame;
pub mod proto;
pub mod tls;
pub mod intern;
pub mod snap;
pub mod conn;
pub mod host;
pub mod client;

#[cfg(test)]
mod tests {
    use super::client::{join, JoinError};
    use super::host::{Event, Host};
    use super::proto::{NetInput, ToClient, ToHost, B_CAST};
    use super::tls;
    use std::time::{Duration, Instant};

    fn tmp(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("ashen_net_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    /// Waits for host events matching `want`, answering hellos with a welcome.
    fn wait_for(host: &Host, mut want: impl FnMut(&Event) -> bool) -> bool {
        let t = Instant::now();
        while t.elapsed() < Duration::from_secs(10) {
            for e in host.poll() {
                if let Event::Hello { peer, .. } = &e {
                    host.send(*peer, &ToClient::Welcome { you: 1, host_name: "HOST".into() });
                }
                if want(&e) {
                    return true;
                }
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        false
    }

    #[test]
    fn join_a_host_over_tls_with_a_pinned_key_and_a_password() {
        let (hd, cd) = (tmp("host"), tmp("client"));
        let host = Host::start(0, "SECRET".into(), &hd).unwrap();
        let addr = format!("127.0.0.1:{}", host.port);
        // First contact: not trusted yet; the fingerprint shown is the host's.
        match join(&addr, &cd, "ROWAN", "name=ROWAN\n", "SECRET") {
            Err(JoinError::Unknown(fp)) => assert_eq!(fp, host.fingerprint),
            Err(e) => panic!("expected an unknown host, got {e:?}"),
            Ok(_) => panic!("expected an unknown host, got in"),
        }
        tls::trust(&cd, &addr, &host.fingerprint).unwrap();
        // The wrong password is refused.
        let wrong = std::thread::spawn({
            let (addr, cd) = (addr.clone(), cd.clone());
            move || join(&addr, &cd, "ROWAN", "name=ROWAN\n", "GUESS").err()
        });
        assert!(matches!(wrong.join().unwrap(), Some(JoinError::Refused(w)) if w == "WRONG PASSWORD"));
        // The right one gets in, and its input reaches the host.
        let ok = std::thread::spawn({
            let (addr, cd) = (addr.clone(), cd.clone());
            move || join(&addr, &cd, "rowan", "name=ROWAN\n", "SECRET")
        });
        assert!(wait_for(&host, |e| matches!(e, Event::Hello { name, .. } if name == "ROWAN")), "hello (name cleaned)");
        let link = ok.join().unwrap().expect("joined");
        assert_eq!(link.you, 1);
        assert!(link.send(&ToHost::Input(NetInput { seq: 1, buttons: B_CAST, ..Default::default() })));
        assert!(wait_for(&host, |e| matches!(e, Event::Input { input, .. } if input.buttons == B_CAST)));
        assert!(link.send(&ToHost::Chat("hello\u{7}".into())));
        assert!(wait_for(&host, |e| matches!(e, Event::Chat { text, .. } if text == "HELLO")));
        drop(link);
        assert!(wait_for(&host, |e| matches!(e, Event::Gone { why, .. } if why == "LEFT")));
        // A host showing another key than the one trusted is refused.
        tls::trust(&cd, &addr, &[9u8; 32]).unwrap();
        assert!(matches!(join(&addr, &cd, "ROWAN", "name=ROWAN\n", "SECRET"), Err(JoinError::Changed(fp)) if fp == host.fingerprint));
        let _ = std::fs::remove_dir_all(&hd);
        let _ = std::fs::remove_dir_all(&cd);
    }

    #[test]
    fn junk_and_silence_are_dropped_without_harm() {
        use std::io::Write;
        let hd = tmp("junk");
        let host = Host::start(0, String::new(), &hd).unwrap();
        // Plain junk instead of TLS.
        let mut s = std::net::TcpStream::connect(("127.0.0.1", host.port)).unwrap();
        let _ = s.write_all(&[0xff; 4096]);
        drop(s);
        // A connection that never says anything still frees its slot (after the hello deadline).
        let _quiet = std::net::TcpStream::connect(("127.0.0.1", host.port)).unwrap();
        // The host keeps working: a real joiner still gets in.
        let cd = tmp("junk_client");
        let addr = format!("127.0.0.1:{}", host.port);
        tls::trust(&cd, &addr, &host.fingerprint).unwrap();
        let ok = std::thread::spawn({
            let (addr, cd) = (addr.clone(), cd.clone());
            move || join(&addr, &cd, "ADA", "name=ADA\n", "").is_ok()
        });
        assert!(wait_for(&host, |e| matches!(e, Event::Hello { name, .. } if name == "ADA")));
        assert!(ok.join().unwrap());
        let _ = std::fs::remove_dir_all(&hd);
        let _ = std::fs::remove_dir_all(&cd);
    }
}

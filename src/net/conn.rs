//! One connection's traffic, on its own thread: frames in (size-capped, rate-limited, with an idle
//! timeout) and frames out (from a bounded queue). Works over any stream, TLS or not (the tests use both).
#![forbid(unsafe_code)]

use super::frame::{write_frame, FrameError};
use std::io::{self, Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::{Duration, Instant};

/// What one side may send the other.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// The biggest frame accepted.
    pub max_frame: usize,
    /// Frames per second, on average over a second.
    pub frames_per_sec: u32,
    /// Bytes per second, on average over a second (a big first frame, the hero's save, is let through).
    pub bytes_per_sec: usize,
    /// Silence longer than this ends the connection.
    pub idle: Duration,
}

/// A joiner (as the host sees it): 60 inputs a second and a little chat; quiet for 20 s and it's gone.
pub const FROM_JOINER: Limits = Limits { max_frame: super::frame::MAX_TO_HOST, frames_per_sec: 150, bytes_per_sec: 64 * 1024, idle: Duration::from_secs(20) };
/// The host (as a joiner sees it): snapshots, and a whole level now and then.
pub const FROM_HOST: Limits = Limits { max_frame: super::frame::MAX_TO_CLIENT, frames_per_sec: 400, bytes_per_sec: 8 * 1024 * 1024, idle: Duration::from_secs(20) };

/// Why a connection ended.
#[derive(Debug)]
pub enum Ended {
    /// The other side closed it.
    Closed,
    /// Nothing heard for too long.
    Idle,
    /// It broke the rules (a bad frame, too much too fast, a message it mustn't send).
    Misbehaved(&'static str),
    Io(io::Error),
    /// We're shutting down, or the game asked to drop it.
    Stopped,
}

/// Splits a byte stream into frames as it arrives (reads can stop anywhere, including mid-length).
pub struct FrameReader {
    buf: Vec<u8>,
    max: usize,
}

impl FrameReader {
    pub fn new(max: usize) -> Self {
        FrameReader { buf: Vec::new(), max }
    }

    pub fn feed(&mut self, data: &[u8]) {
        self.buf.extend_from_slice(data);
    }

    /// The next whole frame, if one has arrived. A bad length is an error at once (nothing is allocated for it).
    pub fn next(&mut self) -> Result<Option<Vec<u8>>, FrameError> {
        if self.buf.len() < 4 {
            return Ok(None);
        }
        let n = u32::from_le_bytes([self.buf[0], self.buf[1], self.buf[2], self.buf[3]]) as usize;
        if n == 0 || n > self.max {
            return Err(FrameError::BadLength(n));
        }
        if self.buf.len() < 4 + n {
            return Ok(None);
        }
        let frame = self.buf[4..4 + n].to_vec();
        self.buf.drain(..4 + n);
        Ok(Some(frame))
    }
}

/// Runs a connection until it ends. Each whole frame goes to `on_frame`, which returns false to end it
/// (a message it mustn't send). Frames queued in `outbound` are written. The stream should have a short
/// read timeout (the loop's tick) and a write timeout (a peer that stops reading is dropped).
pub fn pump<S: Read + Write>(s: &mut S, lim: Limits, outbound: &Receiver<Vec<u8>>, stop: &AtomicBool, mut on_frame: impl FnMut(Vec<u8>) -> bool) -> Ended {
    let mut reader = FrameReader::new(lim.max_frame);
    let mut tmp = vec![0u8; 16 * 1024];
    let mut heard = Instant::now();
    let (mut window, mut frames, mut bytes) = (Instant::now(), 0u32, 0usize);
    let mut first = true;
    loop {
        if stop.load(Ordering::Relaxed) {
            return Ended::Stopped;
        }
        // Out: everything queued (bounded by the queue's size).
        loop {
            match outbound.try_recv() {
                Ok(f) => {
                    if let Err(e) = write_frame(s, &f) {
                        return Ended::Io(e);
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return Ended::Stopped,
            }
        }
        // In.
        match s.read(&mut tmp) {
            Ok(0) => return Ended::Closed,
            Ok(n) => {
                heard = Instant::now();
                reader.feed(&tmp[..n]);
                loop {
                    match reader.next() {
                        Ok(Some(f)) => {
                            if window.elapsed() >= Duration::from_secs(1) {
                                (window, frames, bytes) = (Instant::now(), 0, 0);
                            }
                            frames += 1;
                            // The first frame (the hello, with the hero's save) may be big; after that, the budget.
                            if !first {
                                bytes += f.len();
                            }
                            first = false;
                            if frames > lim.frames_per_sec || bytes > lim.bytes_per_sec {
                                return Ended::Misbehaved("too much, too fast");
                            }
                            if !on_frame(f) {
                                return Ended::Misbehaved("a message it mustn't send");
                            }
                        }
                        Ok(None) => break,
                        Err(_) => return Ended::Misbehaved("a bad frame"),
                    }
                }
            }
            Err(e) if matches!(e.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut | io::ErrorKind::Interrupted) => {}
            Err(e) => return Ended::Io(e),
        }
        if heard.elapsed() > lim.idle {
            return Ended::Idle;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_split_anywhere_come_out_whole() {
        let mut wire = vec![];
        write_frame(&mut wire, b"one").unwrap();
        write_frame(&mut wire, b"second frame").unwrap();
        let mut r = FrameReader::new(100);
        let mut got = vec![];
        // One byte at a time: the worst a network can do.
        for b in wire {
            r.feed(&[b]);
            while let Some(f) = r.next().unwrap() {
                got.push(f);
            }
        }
        assert_eq!(got, vec![b"one".to_vec(), b"second frame".to_vec()]);
        let mut r = FrameReader::new(100);
        r.feed(&1000u32.to_le_bytes());
        assert!(r.next().is_err(), "too big, refused on the length");
    }
}

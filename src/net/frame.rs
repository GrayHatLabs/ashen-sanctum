//! Length-prefixed frames: a little-endian u32 length, then that many bytes. The length is checked
//! against a cap *before* any buffer is allocated, so a peer can't make us allocate gigabytes, and a
//! zero length is refused (it's never sent).
#![forbid(unsafe_code)]

use std::io::{self, Read, Write};

/// The most a joiner may send in one frame (its hero's save is the biggest thing it sends).
pub const MAX_TO_HOST: usize = 128 * 1024;
/// The most the host may send in one frame (a whole level, compressed).
pub const MAX_TO_CLIENT: usize = 4 * 1024 * 1024;

#[derive(Debug)]
pub enum FrameError {
    /// The connection failed or closed (including a read timeout).
    Io(io::Error),
    /// The peer announced a frame that's empty or over the cap: it's misbehaving.
    BadLength(usize),
}

impl From<io::Error> for FrameError {
    fn from(e: io::Error) -> Self {
        FrameError::Io(e)
    }
}

/// Writes one frame. The caller keeps payloads under the receiver's cap.
pub fn write_frame<W: Write>(w: &mut W, payload: &[u8]) -> io::Result<()> {
    let n = u32::try_from(payload.len()).map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "frame too large"))?;
    let mut buf = Vec::with_capacity(4 + payload.len());
    buf.extend_from_slice(&n.to_le_bytes());
    buf.extend_from_slice(payload);
    w.write_all(&buf)?;
    w.flush()
}

/// Reads one frame of at most `max` bytes.
pub fn read_frame<R: Read>(r: &mut R, max: usize) -> Result<Vec<u8>, FrameError> {
    let mut len = [0u8; 4];
    r.read_exact(&mut len)?;
    let n = u32::from_le_bytes(len) as usize;
    if n == 0 || n > max {
        return Err(FrameError::BadLength(n));
    }
    let mut buf = vec![0u8; n];
    r.read_exact(&mut buf)?;
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_round_trip() {
        let mut wire = vec![];
        write_frame(&mut wire, b"hello").unwrap();
        write_frame(&mut wire, &[7u8; 1000]).unwrap();
        let mut r = &wire[..];
        assert_eq!(read_frame(&mut r, 2000).unwrap(), b"hello");
        assert_eq!(read_frame(&mut r, 2000).unwrap(), vec![7u8; 1000]);
        assert!(matches!(read_frame(&mut r, 2000), Err(FrameError::Io(_))), "end of stream");
    }

    #[test]
    fn oversized_and_empty_frames_are_refused_before_allocating() {
        // A peer claiming a 4 GB frame: refused on the length alone.
        let wire = u32::MAX.to_le_bytes();
        assert!(matches!(read_frame(&mut &wire[..], MAX_TO_HOST), Err(FrameError::BadLength(_))));
        let wire = 0u32.to_le_bytes();
        assert!(matches!(read_frame(&mut &wire[..], MAX_TO_HOST), Err(FrameError::BadLength(0))));
        let wire = ((MAX_TO_HOST + 1) as u32).to_le_bytes();
        assert!(matches!(read_frame(&mut &wire[..], MAX_TO_HOST), Err(FrameError::BadLength(_))));
    }

    #[test]
    fn a_truncated_frame_is_an_error_not_a_hang_or_panic() {
        let mut wire = 10u32.to_le_bytes().to_vec();
        wire.extend_from_slice(b"abc");
        assert!(matches!(read_frame(&mut &wire[..], 100), Err(FrameError::Io(_))));
    }
}

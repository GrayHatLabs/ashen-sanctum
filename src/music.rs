//! Synthesised music loops (no asset files): plucked strings (Karplus-Strong), pads,
//! FM bells and drums, with a little reverb. Each track is rendered once, off the main
//! thread, into a seamless loop.
use crate::rng::Rng;
use std::f32::consts::TAU;

pub const RATE: i32 = 22050;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Track {
    Town,
    Wilds,
    Dungeon,
    Boss,
    /// Act 2: the Frostmarch and Kaldholm.
    Frost,
    /// Act 2: the ice dungeons.
    Ice,
}

pub const TRACKS: [Track; 6] = [Track::Town, Track::Wilds, Track::Dungeon, Track::Boss, Track::Frost, Track::Ice];

fn hz(midi: f32) -> f32 {
    440.0 * 2f32.powf((midi - 69.0) / 12.0)
}

/// A loop being written: `len` samples, plus a tail that wraps back onto the start.
struct Buf {
    s: Vec<f32>,
    len: usize,
}

impl Buf {
    fn new(secs: f32) -> Buf {
        let len = (secs * RATE as f32) as usize;
        Buf { s: vec![0.0; len + RATE as usize * 4], len }
    }
    fn at(t: f32) -> usize {
        (t * RATE as f32) as usize
    }

    /// Plucked string (Karplus-Strong). `bright` 0..1, `decay` per-sample damping.
    fn pluck(&mut self, t: f32, midi: f32, gain: f32, bright: f32, decay: f32, rng: &mut Rng) {
        let p = ((RATE as f32 / hz(midi)) as usize).max(2);
        let mut ring: Vec<f32> = (0..p).map(|_| rng.f() * 2.0 - 1.0).collect();
        // Soften the attack for darker tones.
        for _ in 0..((1.0 - bright) * 4.0) as usize {
            let c = ring.clone();
            for i in 0..p {
                ring[i] = (c[i] + c[(i + 1) % p]) * 0.5;
            }
        }
        let start = Buf::at(t);
        let n = (RATE as f32 * 3.5) as usize;
        let mut i = 0;
        for k in 0..n {
            let j = start + k;
            if j >= self.s.len() {
                break;
            }
            let next = (i + 1) % p;
            let v = ring[i];
            ring[i] = (ring[i] + ring[next]) * 0.5 * decay;
            i = next;
            self.s[j] += v * gain;
        }
    }

    /// Soft pad chord: detuned saws through a low-pass, slow swell.
    fn pad(&mut self, t: f32, dur: f32, notes: &[f32], gain: f32, cutoff: f32) {
        let start = Buf::at(t);
        let n = Buf::at(dur + 1.5);
        let mut lp = 0.0f32;
        let mut lp2 = 0.0f32;
        let mut ph: Vec<f32> = vec![0.0; notes.len() * 2];
        let a = (cutoff / RATE as f32 * TAU).min(1.0);
        for k in 0..n {
            let j = start + k;
            if j >= self.s.len() {
                break;
            }
            let tt = k as f32 / RATE as f32;
            let env = (tt / 1.2).min(1.0) * if tt > dur { (1.0 - (tt - dur) / 1.5).max(0.0) } else { 1.0 };
            let mut v = 0.0;
            for (q, m) in notes.iter().enumerate() {
                for d in 0..2 {
                    let f = hz(*m) * if d == 0 { 1.0 } else { 1.004 };
                    let p = &mut ph[q * 2 + d];
                    *p = (*p + f / RATE as f32).fract();
                    v += *p * 2.0 - 1.0;
                }
            }
            lp += (v - lp) * a;
            lp2 += (lp - lp2) * a;
            self.s[j] += lp2 * env * gain / notes.len() as f32;
        }
    }

    /// FM bell.
    fn bell(&mut self, t: f32, midi: f32, gain: f32) {
        let f = hz(midi);
        let start = Buf::at(t);
        let n = Buf::at(4.0);
        for k in 0..n {
            let j = start + k;
            if j >= self.s.len() {
                break;
            }
            let tt = k as f32 / RATE as f32;
            let idx = 3.0 * (-tt * 2.0).exp();
            let v = (TAU * f * tt + idx * (TAU * f * 1.41 * tt).sin()).sin();
            self.s[j] += v * (-tt * 1.1).exp() * gain * (tt * 200.0).min(1.0);
        }
    }

    /// Low drum: falling sine plus a noise click.
    fn drum(&mut self, t: f32, gain: f32, pitch: f32, rng: &mut Rng) {
        let start = Buf::at(t);
        let n = Buf::at(0.6);
        let mut ph = 0.0f32;
        let mut lp = 0.0;
        for k in 0..n {
            let j = start + k;
            if j >= self.s.len() {
                break;
            }
            let tt = k as f32 / RATE as f32;
            ph += (pitch * (1.0 + 1.5 * (-tt * 25.0).exp())) / RATE as f32;
            lp += (rng.f() * 2.0 - 1.0 - lp) * 0.3;
            let v = (ph * TAU).sin() * (-tt * 7.0).exp() + lp * (-tt * 40.0).exp() * 0.5;
            self.s[j] += v * gain;
        }
    }

    /// Envelope for sounds that run the whole loop: fades in over the first 2 s and out over
    /// 2 s past the end, so folding the tail onto the start crossfades the seam.
    fn seam(&self, j: usize) -> f32 {
        let x = 2 * RATE as usize;
        if j < x {
            j as f32 / x as f32
        } else if j >= self.len {
            (1.0 - (j - self.len) as f32 / x as f32).max(0.0)
        } else {
            1.0
        }
    }

    /// Filtered noise wind with slow gusts.
    fn wind(&mut self, gain: f32, rng: &mut Rng) {
        let mut lp = 0.0f32;
        let mut lp2 = 0.0f32;
        for j in 0..self.len + 2 * RATE as usize {
            let tt = j as f32 / RATE as f32;
            let gust = 0.5 + 0.5 * (tt * 0.21).sin() * (tt * 0.13 + 1.0).sin();
            let a = 0.01 + 0.02 * gust;
            lp += (rng.f() * 2.0 - 1.0 - lp) * a;
            lp2 += (lp - lp2) * a;
            let e = self.seam(j);
            self.s[j] += lp2 * gain * (0.4 + gust) * e;
        }
    }

    /// Sustained low drone (two detuned sines with a slow wobble).
    fn drone(&mut self, midi: f32, gain: f32) {
        let f = hz(midi);
        for j in 0..self.len + 2 * RATE as usize {
            let tt = j as f32 / RATE as f32;
            let w = 1.0 + 0.3 * (tt * 0.4).sin();
            let v = (TAU * f * tt).sin() + 0.6 * (TAU * f * 1.502 * tt).sin() * w + 0.3 * (TAU * f * 2.003 * tt).sin();
            let e = self.seam(j);
            self.s[j] += v * gain * e;
        }
    }

    /// Wraps the tail onto the start, adds reverb, normalises.
    fn finish(mut self, wet: f32, peak: f32) -> Vec<f32> {
        // Reverb on the whole buffer (combs + all-pass, Schroeder style).
        let combs = [1557usize, 1617, 1491, 1422];
        let mut out = self.s.clone();
        for &d in &combs {
            let d = d * RATE as usize / 44100;
            let mut line = vec![0.0f32; d];
            let mut i = 0;
            for k in 0..self.s.len() {
                let y = line[i];
                line[i] = self.s[k] + y * 0.8;
                i = (i + 1) % d;
                out[k] += y * wet / combs.len() as f32;
            }
        }
        // Fold the tail onto the loop start.
        let len = self.len;
        for k in len..out.len() {
            out[k % len] += out[k];
        }
        out.truncate(len);
        // Warm it up: two gentle low-passes (~3 kHz) and a DC blocker, run twice around the
        // loop so the filters have settled by the seam.
        let a = (3000.0 / RATE as f32 * TAU).min(1.0);
        let (mut l1, mut l2, mut x1, mut y1) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
        for pass in 0..2 {
            for k in 0..len {
                l1 += (out[k] - l1) * a;
                l2 += (l1 - l2) * a;
                let y = l2 - x1 + 0.995 * y1;
                x1 = l2;
                y1 = y;
                if pass == 1 {
                    out[k] = y;
                }
            }
        }
        // Gentle fades at the seam are not needed: the tail wrap keeps it continuous.
        let m = out.iter().fold(0.0f32, |a, v| a.max(v.abs())).max(1e-6);
        for v in out.iter_mut() {
            *v *= peak / m;
        }
        self.s.clear();
        out
    }
}

/// Renders one track (a few hundred ms to a couple of seconds of CPU).
pub fn render(track: Track) -> Vec<f32> {
    let mut rng = Rng::new(match track {
        Track::Town => 11,
        Track::Wilds => 23,
        Track::Dungeon => 37,
        Track::Boss => 41,
        Track::Frost => 53,
        Track::Ice => 67,
    });
    match track {
        Track::Town => town(&mut rng),
        Track::Wilds => wilds(&mut rng),
        Track::Dungeon => dungeon(&mut rng),
        Track::Boss => boss(&mut rng),
        Track::Frost => frost(&mut rng),
        Track::Ice => ice(&mut rng),
    }
}

/// Hollowmere: a lone plucked guitar in D minor over a soft pad.
fn town(rng: &mut Rng) -> Vec<f32> {
    let bpm = 72.0;
    let beat = 60.0 / bpm;
    // Chords (root midi, intervals for the arpeggio).
    let prog: [(f32, [f32; 4]); 8] = [
        (50.0, [0.0, 7.0, 12.0, 15.0]), // Dm
        (46.0, [0.0, 7.0, 12.0, 16.0]), // Bb
        (48.0, [0.0, 7.0, 12.0, 16.0]), // C
        (45.0, [0.0, 7.0, 12.0, 15.0]), // Am
        (50.0, [0.0, 7.0, 12.0, 15.0]), // Dm
        (43.0, [0.0, 7.0, 12.0, 15.0]), // Gm
        (46.0, [0.0, 7.0, 12.0, 16.0]), // Bb
        (45.0, [0.0, 7.0, 12.0, 16.0]), // A
    ];
    let bars = prog.len() * 2;
    let mut b = Buf::new(bars as f32 * 4.0 * beat);
    let pattern = [0usize, 1, 2, 1, 3, 1, 2, 1];
    let scale = [0.0, 2.0, 3.0, 5.0, 7.0, 8.0, 10.0];
    for bar in 0..bars {
        let (root, iv) = prog[bar % prog.len()];
        let t0 = bar as f32 * 4.0 * beat;
        for (k, &p) in pattern.iter().enumerate() {
            let swing = if k % 2 == 1 { 0.03 } else { 0.0 };
            let g = if k == 0 { 0.5 } else { 0.32 };
            b.pluck(t0 + k as f32 * beat * 0.5 + swing, root + iv[p], g, 0.5, 0.996, rng);
        }
        b.pad(t0, 4.0 * beat, &[root - 12.0, root - 5.0, root + iv[3] - 12.0], 0.12, 500.0);
        // A sparse melody on the second time through.
        if bar >= prog.len() {
            for q in 0..2 {
                if rng.chance(0.75) {
                    let deg = scale[rng.range(0, scale.len() as i32) as usize];
                    let m = 62.0 + deg + if rng.chance(0.3) { 12.0 } else { 0.0 };
                    b.pluck(t0 + q as f32 * 2.0 * beat + beat * 0.02, m, 0.3, 0.8, 0.9975, rng);
                }
            }
        }
    }
    b.finish(0.35, 0.7)
}

/// The Ashlands: wind, a low drone and a few distant plucks.
fn wilds(rng: &mut Rng) -> Vec<f32> {
    let secs = 48.0;
    let mut b = Buf::new(secs);
    b.wind(0.35, rng);
    b.drone(33.0, 0.05); // A1
    let notes = [57.0, 60.0, 64.0, 62.0, 55.0, 59.0, 64.0, 67.0];
    let mut t = 1.0;
    let mut k = 0;
    while t < secs - 1.0 {
        let m = notes[k % notes.len()];
        b.pluck(t, m, 0.35, 0.4, 0.997, rng);
        if rng.chance(0.4) {
            b.pluck(t + 0.42, m + 7.0, 0.2, 0.4, 0.997, rng);
        }
        t += rng.rf(2.2, 4.0);
        k += 1;
    }
    for c in 0..4 {
        let root = [45.0, 41.0, 43.0, 40.0][c];
        b.pad(c as f32 * 12.0, 11.0, &[root, root + 7.0, root + 15.0], 0.08, 350.0);
    }
    b.finish(0.45, 0.6)
}

/// Dungeons: a dark drone, a slow heartbeat and distant bells.
fn dungeon(rng: &mut Rng) -> Vec<f32> {
    let secs = 40.0;
    let mut b = Buf::new(secs);
    b.drone(26.0, 0.07); // D1
    for c in 0..5 {
        let root = [38.0, 39.0, 38.0, 36.0, 37.0][c];
        b.pad(c as f32 * 8.0, 7.5, &[root, root + 3.0, root + 7.0], 0.1, 260.0);
    }
    let mut t = 0.5;
    while t < secs {
        b.drum(t, 0.25, 48.0, rng);
        b.drum(t + 0.28, 0.15, 44.0, rng);
        t += 2.0;
    }
    let bells = [74.0, 75.0, 70.0, 69.0, 77.0, 74.0];
    for (k, m) in bells.iter().enumerate() {
        b.bell(3.0 + k as f32 * 6.3 + rng.rf(0.0, 1.5), *m, 0.12);
    }
    b.finish(0.55, 0.6)
}

/// The Frostmarch: howling wind, open fifths, and a slow harp-like melody in D dorian.
fn frost(rng: &mut Rng) -> Vec<f32> {
    let secs = 56.0;
    let mut b = Buf::new(secs);
    b.wind(0.55, rng);
    b.drone(38.0, 0.035); // D2
    b.drone(45.0, 0.025); // A2: the open fifth
    let chords = [(50.0, 57.0, 62.0), (48.0, 55.0, 64.0), (53.0, 57.0, 60.0), (50.0, 57.0, 65.0)];
    for (c, &(a, bb, cc)) in chords.iter().enumerate() {
        b.pad(c as f32 * 14.0, 13.0, &[a, bb, cc], 0.07, 420.0);
    }
    // A slow melody (D dorian), each note with a soft octave echo.
    let tune = [62.0, 64.0, 65.0, 69.0, 67.0, 65.0, 64.0, 62.0, 60.0, 62.0, 69.0, 71.0, 69.0, 67.0, 64.0, 62.0];
    let mut t = 2.0;
    for (k, m) in tune.iter().enumerate() {
        b.pluck(t, *m, 0.32, 0.55, 0.9975, rng);
        if k % 4 == 3 {
            b.pluck(t + 0.9, m - 12.0, 0.18, 0.4, 0.998, rng);
        }
        t += if k % 4 == 3 { 4.2 } else { 2.6 };
        if t > secs - 2.0 {
            break;
        }
    }
    b.finish(0.5, 0.6)
}

/// Ice dungeons: a deep cold drone, glassy bells and groaning ice.
fn ice(rng: &mut Rng) -> Vec<f32> {
    let secs = 44.0;
    let mut b = Buf::new(secs);
    b.drone(26.0, 0.06);
    for c in 0..4 {
        let root = [38.0, 41.0, 36.0, 39.0][c];
        b.pad(c as f32 * 11.0, 10.5, &[root, root + 7.0, root + 14.0], 0.08, 300.0);
    }
    let bells = [86.0, 81.0, 84.0, 79.0, 88.0, 83.0, 81.0];
    for (k, m) in bells.iter().enumerate() {
        let t = 1.5 + k as f32 * 6.0 + rng.rf(0.0, 1.8);
        b.bell(t, *m, 0.08);
        b.bell(t + 0.18, m - 12.0, 0.05);
    }
    // Ice groans: very low, slowly falling plucks.
    for k in 0..4 {
        b.pluck(4.0 + k as f32 * 10.5, 31.0 + (k % 2) as f32 * 2.0, 0.22, 0.2, 0.9985, rng);
    }
    b.finish(0.6, 0.6)
}

/// Boss fights: drums and a driving low ostinato in E phrygian.
fn boss(rng: &mut Rng) -> Vec<f32> {
    let bpm = 112.0;
    let beat = 60.0 / bpm;
    let bars = 16;
    let mut b = Buf::new(bars as f32 * 4.0 * beat);
    let riff = [40.0, 40.0, 41.0, 40.0, 43.0, 40.0, 41.0, 38.0];
    for bar in 0..bars {
        let t0 = bar as f32 * 4.0 * beat;
        let up = if (bar / 4) % 2 == 1 { 1.0 } else { 0.0 };
        for (k, m) in riff.iter().enumerate() {
            b.pluck(t0 + k as f32 * beat * 0.5, m + up, 0.45, 0.55, 0.993, rng);
            b.pluck(t0 + k as f32 * beat * 0.5, m + up + 12.0, 0.15, 0.5, 0.99, rng);
        }
        for k in 0..4 {
            b.drum(t0 + k as f32 * beat, if k % 2 == 0 { 0.55 } else { 0.35 }, if k % 2 == 0 { 55.0 } else { 70.0 }, rng);
        }
        if bar % 2 == 1 {
            b.drum(t0 + 3.5 * beat, 0.3, 80.0, rng);
        }
        b.pad(t0, 4.0 * beat, &[52.0 + up, 55.0 + up, 59.0 + up], 0.1, 700.0);
        if bar % 4 == 0 {
            b.bell(t0, 76.0 + up, 0.1);
        }
    }
    b.finish(0.3, 0.75)
}

/// 16-bit mono WAV file bytes.
pub fn wav(s: &[f32]) -> Vec<u8> {
    let mut b = Vec::with_capacity(44 + s.len() * 2);
    let data = (s.len() * 2) as u32;
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + data).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&(RATE as u32).to_le_bytes());
    b.extend_from_slice(&(RATE as u32 * 2).to_le_bytes());
    b.extend_from_slice(&2u16.to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&data.to_le_bytes());
    for v in s {
        b.extend_from_slice(&((v.clamp(-1.0, 1.0) * 32000.0) as i16).to_le_bytes());
    }
    b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracks_render_as_quiet_seamless_loops() {
        for t in TRACKS {
            let s = render(t);
            assert!(s.len() > RATE as usize * 20, "{t:?} is a long loop");
            let peak = s.iter().fold(0.0f32, |a, v| a.max(v.abs()));
            assert!(peak > 0.3 && peak <= 0.8, "{t:?} peak {peak}");
            assert!(s.iter().all(|v| v.is_finite()));
            // The seam: the last and first samples are close (no click).
            let (a, z) = (s[0], s[s.len() - 1]);
            assert!((a - z).abs() < 0.2, "{t:?} seam {a} {z}");
        }
    }
}

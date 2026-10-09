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
    /// Act 3: the Mistwood (a haunted waltz).
    Mist,
    /// Act 3: the dungeons and Castle Vardak (a cathedral organ).
    Crypt,
    /// Act 4: the Grinding Fields (a ticking harpsichord).
    Gears,
    /// Act 4: the works and the Heart of the Clock (clanking engine).
    Engine,
    /// Kaldholm: a fireside folk tune over a drone.
    Hearth,
    /// Mournhold: a candlelit lament on a lute, a bell tolling far off.
    Vigil,
    /// The Last Escapement: a gentle music-box waltz, the one warm place in the Dominion.
    Refuge,
    /// Act 5: the Sunken Reach (a slow swell, whale-song).
    Tide,
    /// Act 5: the drowned dungeons (the pressure of the deep).
    Abyss,
    /// Brinehollow: a sea shanty on a squeezebox and a creaking hull.
    Brine,
    /// Act 6: the Skyreach (a soaring choir over the wind).
    Sky,
    /// Act 6: the sky dungeons (storm drums, a burning organ).
    Storm,
    /// Windward Anchorage: a bright harbour waltz on fiddle and bells.
    Harbor,
}

pub const TRACKS: [Track; 19] = [
    Track::Town,
    Track::Wilds,
    Track::Dungeon,
    Track::Boss,
    Track::Frost,
    Track::Ice,
    Track::Mist,
    Track::Crypt,
    Track::Gears,
    Track::Engine,
    Track::Hearth,
    Track::Vigil,
    Track::Refuge,
    Track::Tide,
    Track::Abyss,
    Track::Brine,
    Track::Sky,
    Track::Storm,
    Track::Harbor,
];

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
        Track::Mist => 71,
        Track::Crypt => 83,
        Track::Gears => 97,
        Track::Engine => 101,
        Track::Hearth => 107,
        Track::Vigil => 113,
        Track::Refuge => 127,
        Track::Tide => 131,
        Track::Abyss => 137,
        Track::Brine => 139,
        Track::Sky => 149,
        Track::Storm => 151,
        Track::Harbor => 157,
    });
    match track {
        Track::Town => town(&mut rng),
        Track::Wilds => wilds(&mut rng),
        Track::Dungeon => dungeon(&mut rng),
        Track::Boss => boss(&mut rng),
        Track::Frost => frost(&mut rng),
        Track::Ice => ice(&mut rng),
        Track::Mist => mist(&mut rng),
        Track::Crypt => crypt(&mut rng),
        Track::Gears => gears(&mut rng),
        Track::Engine => engine(&mut rng),
        Track::Hearth => hearth(&mut rng),
        Track::Vigil => vigil(&mut rng),
        Track::Refuge => refuge(&mut rng),
        Track::Tide => tide(&mut rng),
        Track::Abyss => abyss(&mut rng),
        Track::Brine => brine(&mut rng),
        Track::Sky => sky(&mut rng),
        Track::Storm => storm(&mut rng),
        Track::Harbor => harbor(&mut rng),
    }
}

/// The Skyreach: a slow, soaring choir of pads in D major over the wind, with high bells like sunlight.
fn sky(rng: &mut Rng) -> Vec<f32> {
    let secs = 52.0;
    let mut b = Buf::new(secs);
    b.wind(0.09, rng);
    b.drone(38.0, 0.04);
    let chords = [(50.0, 54.0, 57.0), (47.0, 50.0, 54.0), (43.0, 47.0, 50.0), (45.0, 49.0, 52.0)];
    for (k, &(a, c, e)) in chords.iter().enumerate() {
        b.pad(k as f32 * 13.0, 12.5, &[a, c, e, a + 12.0, c + 12.0], 0.09, 1200.0);
    }
    let bells = [86.0, 88.0, 90.0, 85.0, 83.0, 81.0, 86.0, 93.0];
    for (k, m) in bells.iter().enumerate() {
        let t = 1.0 + k as f32 * 6.3 + rng.rf(0.0, 1.2);
        b.bell(t, *m, 0.05);
        b.bell(t + 0.3, m - 5.0, 0.03);
    }
    b.finish(0.6, 0.6)
}

/// The sky dungeons: storm drums rolling under a burning organ in E minor, thunder now and then.
fn storm(rng: &mut Rng) -> Vec<f32> {
    let secs = 48.0;
    let mut b = Buf::new(secs);
    b.drone(28.0, 0.07);
    b.wind(0.06, rng);
    let chords = [(40.0, 43.0, 47.0), (36.0, 40.0, 43.0), (38.0, 42.0, 45.0), (35.0, 39.0, 42.0)];
    for (k, &(a, c, e)) in chords.iter().enumerate() {
        b.pad(k as f32 * 12.0, 11.5, &[a, c, e, a + 12.0], 0.11, 600.0);
    }
    let mut t = 0.3;
    let mut k = 0;
    while t < secs {
        // A rolling tom pattern.
        b.drum(t, 0.25, 48.0, rng);
        b.drum(t + 0.2, 0.12, 60.0, rng);
        if k % 4 == 3 {
            b.drum(t + 0.4, 0.18, 52.0, rng);
            b.drum(t + 0.55, 0.14, 56.0, rng);
        }
        t += 0.8;
        k += 1;
    }
    // Thunder: very low long plucks.
    for k in 0..4 {
        b.pluck(5.0 + k as f32 * 11.0 + rng.rf(0.0, 2.0), 26.0, 0.3, 0.15, 0.9992, rng);
    }
    b.finish(0.6, 0.62)
}

/// Windward Anchorage: a bright 3/4 harbour waltz in G, a fiddle-like pluck over bells and a soft pad.
fn harbor(rng: &mut Rng) -> Vec<f32> {
    let bpm = 104.0;
    let beat = 60.0 / bpm;
    let bars = 16;
    let mut b = Buf::new(bars as f32 * 3.0 * beat);
    let chords: [(f32, [f32; 3]); 4] = [(55.0, [0.0, 4.0, 7.0]), (52.0, [0.0, 3.0, 7.0]), (48.0, [0.0, 4.0, 7.0]), (50.0, [0.0, 4.0, 7.0])];
    let tune = [[79.0, 83.0, 86.0], [84.0, 83.0, 79.0], [76.0, 79.0, 84.0], [81.0, 78.0, 74.0]];
    for bar in 0..bars {
        let (root, iv) = chords[bar % 4];
        let t0 = bar as f32 * 3.0 * beat + 0.05;
        b.pluck(t0, root - 12.0, 0.24, 0.4, 0.996, rng);
        for k in 1..3 {
            for (q, i) in iv.iter().enumerate().skip(1) {
                b.pluck(t0 + k as f32 * beat + q as f32 * 0.01, root + i, 0.07, 0.6, 0.995, rng);
            }
        }
        b.pad(t0, 3.0 * beat, &[root - 12.0, root + iv[1] - 12.0, root + iv[2] - 12.0], 0.05, 900.0);
        let up = if bar >= 8 { 0.0 } else { -12.0 };
        for (k, m) in tune[bar % 4].iter().enumerate() {
            b.pluck(t0 + k as f32 * beat + 0.01, m + up, 0.1, 0.85, 0.994, rng);
        }
        if bar % 4 == 0 {
            b.bell(t0, root + 24.0, 0.04);
        }
    }
    b.finish(0.5, 0.6)
}

/// Brinehollow: a slow sea shanty in D dorian, a squeezebox-like pad, a stamping beat and a creaking hull.
fn brine(rng: &mut Rng) -> Vec<f32> {
    let bpm = 84.0;
    let beat = 60.0 / bpm;
    let bars = 16;
    let mut b = Buf::new(bars as f32 * 4.0 * beat);
    let chords: [(f32, [f32; 3]); 4] = [(50.0, [0.0, 3.0, 7.0]), (48.0, [0.0, 4.0, 7.0]), (50.0, [0.0, 3.0, 7.0]), (45.0, [0.0, 3.0, 7.0])];
    let tune = [
        [74.0, 74.0, 77.0, 76.0],
        [72.0, 72.0, 76.0, 74.0],
        [74.0, 77.0, 79.0, 77.0],
        [76.0, 72.0, 69.0, 69.0],
    ];
    for bar in 0..bars {
        let (root, iv) = chords[bar % 4];
        let t0 = bar as f32 * 4.0 * beat + 0.05;
        // Stamp and clap.
        b.drum(t0, 0.3, 50.0, rng);
        b.drum(t0 + 2.0 * beat, 0.3, 50.0, rng);
        b.drum(t0 + beat, 0.06, 900.0, rng);
        b.drum(t0 + 3.0 * beat, 0.06, 900.0, rng);
        // The squeezebox: a reedy pad that breathes in and out with the bar.
        b.pad(t0, 4.0 * beat, &[root - 12.0, root + iv[1] - 12.0, root + iv[2] - 12.0, root], 0.09, 900.0);
        b.pluck(t0, root - 24.0, 0.3, 0.4, 0.996, rng);
        b.pluck(t0 + 2.0 * beat, root - 17.0, 0.22, 0.4, 0.996, rng);
        // The tune, an octave lower on the second pass.
        let down = if bar >= 8 { 12.0 } else { 0.0 };
        for (k, m) in tune[bar % 4].iter().enumerate() {
            b.pluck(t0 + k as f32 * beat + 0.01, m - down, 0.12, 0.7, 0.994, rng);
        }
        // The hull creaks now and then.
        if bar % 4 == 1 {
            b.pluck(t0 + 2.6 * beat, 33.0, 0.12, 0.15, 0.999, rng);
        }
    }
    b.finish(0.5, 0.6)
}

/// The Sunken Reach: a slow swell of pads rising and falling like the tide, whale-song bells and bubbling plucks.
fn tide(rng: &mut Rng) -> Vec<f32> {
    let secs = 52.0;
    let mut b = Buf::new(secs);
    b.drone(26.0, 0.05);
    b.wind(0.05, rng);
    let chords = [(38.0, 45.0, 53.0), (36.0, 43.0, 52.0), (41.0, 48.0, 55.0), (40.0, 47.0, 55.0)];
    for (k, &(a, c, e)) in chords.iter().enumerate() {
        b.pad(k as f32 * 13.0, 12.5, &[a, c, e, e + 12.0], 0.09, 380.0);
    }
    // Whale-song: long low bells sliding down an interval.
    for k in 0..6 {
        let t = 2.0 + k as f32 * 8.5 + rng.rf(0.0, 1.5);
        let m = [62.0, 60.0, 65.0, 57.0, 64.0, 60.0][k];
        b.bell(t, m, 0.06);
        b.bell(t + 0.8, m - 5.0, 0.045);
    }
    // Bubbles: quick high plucks rising.
    for k in 0..14 {
        let t = rng.rf(0.5, secs - 1.0);
        for j in 0..3 {
            b.pluck(t + j as f32 * 0.06, 84.0 + k as f32 % 5.0 + j as f32 * 2.0, 0.025, 0.9, 0.98, rng);
        }
    }
    b.finish(0.6, 0.6)
}

/// The drowned dungeons: the pressure of the deep, a slow heartbeat, sonar pings and a choir of the drowned.
fn abyss(rng: &mut Rng) -> Vec<f32> {
    let secs = 48.0;
    let mut b = Buf::new(secs);
    b.drone(24.0, 0.08);
    b.drone(31.0, 0.04);
    for c in 0..4 {
        let root = [36.0, 35.0, 33.0, 34.0][c];
        b.pad(c as f32 * 12.0, 11.5, &[root, root + 3.0, root + 10.0], 0.08, 260.0);
    }
    // A slow heartbeat.
    let mut t = 0.4;
    while t < secs {
        b.drum(t, 0.22, 38.0, rng);
        b.drum(t + 0.28, 0.14, 40.0, rng);
        t += 1.6;
    }
    // Pings in the dark.
    for k in 0..7 {
        let t = 3.0 + k as f32 * 6.4 + rng.rf(0.0, 1.0);
        b.bell(t, 88.0, 0.05);
        b.bell(t + 0.9, 88.0, 0.02);
    }
    b.finish(0.6, 0.6)
}

/// Kaldholm: a lilting 6/8 folk tune (D mixolydian) on plucked strings over a bagpipe-like
/// drone, a frame drum, and the crackle of the longhouse fire.
fn hearth(rng: &mut Rng) -> Vec<f32> {
    let bpm = 100.0;
    let eighth = 60.0 / bpm / 2.0;
    let bars = 16;
    let bar_len = 6.0 * eighth;
    let mut b = Buf::new(bars as f32 * bar_len);
    b.drone(38.0, 0.03); // D2
    b.drone(45.0, 0.018); // A2
    // Fire crackle: sparse soft clicks.
    let mut t = 0.2;
    while t < bars as f32 * bar_len {
        b.drum(t, rng.rf(0.02, 0.05), rng.rf(900.0, 1600.0), rng);
        t += rng.rf(0.08, 0.6);
    }
    let chords: [(f32, [f32; 3]); 4] = [(50.0, [0.0, 4.0, 7.0]), (48.0, [0.0, 4.0, 7.0]), (50.0, [0.0, 4.0, 7.0]), (45.0, [0.0, 3.0, 7.0])];
    // Two four-bar phrases (in eighths), the second answering the first.
    let tune: [[f32; 6]; 8] = [
        [62.0, 66.0, 69.0, 71.0, 69.0, 66.0],
        [64.0, 67.0, 72.0, 71.0, 69.0, 67.0],
        [66.0, 69.0, 74.0, 72.0, 71.0, 69.0],
        [67.0, 66.0, 64.0, 62.0, 64.0, 66.0],
        [69.0, 71.0, 72.0, 74.0, 72.0, 71.0],
        [72.0, 71.0, 69.0, 67.0, 69.0, 72.0],
        [74.0, 72.0, 71.0, 69.0, 67.0, 66.0],
        [64.0, 66.0, 64.0, 62.0, 62.0, 62.0],
    ];
    for bar in 0..bars {
        let t0 = bar as f32 * bar_len + 0.05;
        let (root, iv) = chords[bar % 4];
        // Frame drum on 1 and 4.
        b.drum(t0, 0.22, 62.0, rng);
        b.drum(t0 + 3.0 * eighth, 0.12, 70.0, rng);
        // Strummed chord on the beats.
        for k in [0usize, 3] {
            for (q, i) in iv.iter().enumerate() {
                b.pluck(t0 + k as f32 * eighth + q as f32 * 0.012, root - 12.0 + i, 0.12, 0.45, 0.995, rng);
            }
        }
        b.pad(t0, bar_len, &[root - 12.0, root - 5.0], 0.05, 400.0);
        // The melody after the first time round: the fiddle comes in.
        if bar >= 4 {
            let phrase = tune[(bar - 4) % 8];
            for (k, m) in phrase.iter().enumerate() {
                if k > 0 && *m == phrase[k - 1] {
                    continue;
                }
                let g = if k == 0 || k == 3 { 0.3 } else { 0.22 };
                b.pluck(t0 + k as f32 * eighth, *m, g, 0.75, 0.9965, rng);
            }
        }
    }
    b.finish(0.4, 0.62)
}

/// Mournhold: a slow lute lament (A minor) by candlelight, a soft organ, and a bell
/// tolling far off every few bars.
fn vigil(rng: &mut Rng) -> Vec<f32> {
    let bpm = 60.0;
    let beat = 60.0 / bpm;
    let bars = 12;
    let mut b = Buf::new(bars as f32 * 4.0 * beat);
    b.wind(0.1, rng);
    b.drone(33.0, 0.03); // A1
    let prog: [(f32, [f32; 4]); 6] = [
        (45.0, [0.0, 7.0, 12.0, 15.0]), // Am
        (41.0, [0.0, 7.0, 12.0, 16.0]), // F
        (43.0, [0.0, 7.0, 12.0, 16.0]), // G
        (40.0, [0.0, 7.0, 12.0, 16.0]), // E
        (45.0, [0.0, 7.0, 12.0, 15.0]), // Am
        (38.0, [0.0, 7.0, 12.0, 15.0]), // Dm
    ];
    let tune = [72.0, 71.0, 69.0, 68.0, 69.0, 72.0, 76.0, 74.0, 72.0, 71.0, 69.0, 64.0];
    for bar in 0..bars {
        let (root, iv) = prog[bar % prog.len()];
        let t0 = bar as f32 * 4.0 * beat;
        // Fingerpicked lute: slow, rolling.
        for (k, p) in [0usize, 1, 2, 3, 2, 1].iter().enumerate() {
            b.pluck(t0 + k as f32 * beat * 0.66, root + iv[*p], if k == 0 { 0.32 } else { 0.18 }, 0.5, 0.9965, rng);
        }
        b.pad(t0, 4.0 * beat, &[root, root + 7.0, root + iv[3] - 12.0], 0.06, 450.0);
        // A falling melody on the second half.
        if bar >= 6 {
            let m = tune[(bar - 6) * 2 % tune.len()];
            let n = tune[((bar - 6) * 2 + 1) % tune.len()];
            b.pluck(t0 + 0.02, m, 0.26, 0.7, 0.998, rng);
            b.pluck(t0 + 2.0 * beat, n, 0.22, 0.7, 0.998, rng);
        }
        if bar % 4 == 0 {
            b.bell(t0 + 0.5, 57.0, 0.05);
        }
    }
    b.finish(0.6, 0.55)
}

/// The Last Escapement: a gentle music-box waltz (F major) with a soft tick, warm pads and a
/// little countermelody, the one warm place in the Dominion.
fn refuge(rng: &mut Rng) -> Vec<f32> {
    let bpm = 92.0;
    let beat = 60.0 / bpm;
    let bars = 16;
    let mut b = Buf::new(bars as f32 * 3.0 * beat);
    let chords: [(f32, [f32; 3]); 4] = [(53.0, [0.0, 4.0, 7.0]), (50.0, [0.0, 3.0, 7.0]), (46.0, [0.0, 4.0, 7.0]), (48.0, [0.0, 4.0, 7.0])];
    let tune = [
        [77.0, 76.0, 74.0],
        [72.0, 74.0, 77.0],
        [74.0, 72.0, 70.0],
        [69.0, 70.0, 72.0],
        [77.0, 79.0, 81.0],
        [79.0, 77.0, 74.0],
        [74.0, 76.0, 77.0],
        [76.0, 72.0, 72.0],
    ];
    for bar in 0..bars {
        let (root, iv) = chords[bar % 4];
        // A hair late, so nothing starts exactly on the loop seam.
        let t0 = bar as f32 * 3.0 * beat + 0.05;
        // The escapement's soft tick on every beat.
        for k in 0..3 {
            b.drum(t0 + k as f32 * beat, 0.035, 1200.0, rng);
        }
        // Waltz bass and chord.
        b.pluck(t0, root - 12.0, 0.26, 0.35, 0.996, rng);
        for k in 1..3 {
            for (q, i) in iv.iter().enumerate().skip(1) {
                b.pluck(t0 + k as f32 * beat + q as f32 * 0.01, root + i, 0.08, 0.5, 0.995, rng);
            }
        }
        b.pad(t0, 3.0 * beat, &[root - 12.0, root + iv[1] - 12.0, root + iv[2] - 12.0], 0.05, 420.0);
        // The music box tune.
        let phrase = tune[bar % 8];
        for (k, m) in phrase.iter().enumerate() {
            if k > 0 && *m == phrase[k - 1] {
                continue;
            }
            b.bell(t0 + k as f32 * beat + 0.01, *m, if k == 0 { 0.07 } else { 0.05 });
        }
        // Second time through: a countermelody a sixth below.
        if bar >= 8 && bar % 2 == 0 {
            b.bell(t0 + 1.5 * beat, phrase[0] - 9.0, 0.035);
        }
    }
    b.finish(0.5, 0.58)
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

/// The Mistwood: a slow, lopsided waltz on a music box over fog and a low drone (D harmonic minor).
fn mist(rng: &mut Rng) -> Vec<f32> {
    let bpm = 84.0;
    let beat = 60.0 / bpm;
    let bars = 16;
    let mut b = Buf::new(bars as f32 * 3.0 * beat);
    b.wind(0.18, rng);
    b.drone(38.0, 0.04);
    let chords: [(f32, [f32; 3]); 4] = [(50.0, [0.0, 3.0, 7.0]), (46.0, [0.0, 4.0, 7.0]), (43.0, [0.0, 3.0, 7.0]), (45.0, [0.0, 4.0, 7.0])];
    let tune = [74.0, 73.0, 74.0, 77.0, 76.0, 74.0, 73.0, 69.0, 70.0, 69.0, 67.0, 65.0, 64.0, 65.0, 67.0, 69.0];
    for bar in 0..bars {
        let (root, iv) = chords[bar % 4];
        let t0 = bar as f32 * 3.0 * beat;
        // Oom-pah-pah, softly.
        b.pluck(t0, root - 12.0, 0.35, 0.3, 0.996, rng);
        b.pluck(t0 + beat, root + iv[1], 0.16, 0.5, 0.995, rng);
        b.pluck(t0 + 2.0 * beat, root + iv[2], 0.16, 0.5, 0.995, rng);
        b.pad(t0, 3.0 * beat, &[root, root + iv[1], root + iv[2]], 0.05, 380.0);
        // The music box melody (bell-like, slightly late: haunted).
        let m = tune[bar % tune.len()];
        b.bell(t0 + 0.04, m, 0.06);
        if bar % 2 == 1 {
            b.bell(t0 + 1.5 * beat, m - 5.0, 0.04);
        }
    }
    b.finish(0.55, 0.55)
}

/// Act 3's dungeons and Castle Vardak: a dark cathedral organ with a slow pulse.
fn crypt(rng: &mut Rng) -> Vec<f32> {
    let secs = 48.0;
    let mut b = Buf::new(secs);
    b.drone(26.0, 0.07);
    let chords = [(38.0, 41.0, 45.0), (37.0, 41.0, 44.0), (34.0, 38.0, 41.0), (33.0, 37.0, 40.0)];
    for (k, &(a, c, e)) in chords.iter().enumerate() {
        b.pad(k as f32 * 12.0, 11.5, &[a, c, e, a + 12.0], 0.13, 600.0);
    }
    let mut t = 1.0;
    while t < secs {
        b.drum(t, 0.18, 42.0, rng);
        t += 3.0;
    }
    for (k, m) in [62.0, 61.0, 58.0, 57.0].iter().enumerate() {
        b.bell(4.0 + k as f32 * 12.0, *m + 12.0, 0.05);
    }
    b.finish(0.6, 0.6)
}

/// The Dominion: a clock ticks under a running harpsichord figure (A minor), with an organ swell.
fn gears(rng: &mut Rng) -> Vec<f32> {
    let bpm = 112.0;
    let beat = 60.0 / bpm;
    let bars = 16;
    let mut b = Buf::new(bars as f32 * 4.0 * beat);
    b.drone(33.0, 0.035);
    let chords: [f32; 4] = [57.0, 53.0, 55.0, 52.0];
    for bar in 0..bars {
        let t0 = bar as f32 * 4.0 * beat;
        let root = chords[bar % 4];
        let minor = if bar % 4 == 3 { 4.0 } else { 3.0 };
        // Tick, tock.
        for k in 0..4 {
            b.drum(t0 + k as f32 * beat, if k % 2 == 0 { 0.08 } else { 0.05 }, if k % 2 == 0 { 96.0 } else { 88.0 }, rng);
        }
        // An arpeggio in sixteenths, like clockwork.
        let arp = [0.0, minor, 7.0, 12.0, 7.0, minor, 0.0, minor];
        for k in 0..16 {
            let n = root + arp[k % 8] + if bar >= 8 && k % 4 == 0 { 12.0 } else { 0.0 };
            b.pluck(t0 + k as f32 * beat * 0.25, n, 0.1, 0.8, 0.993, rng);
        }
        b.pad(t0, 4.0 * beat, &[root - 12.0, root - 12.0 + minor, root - 5.0], 0.06, 500.0);
        if bar % 4 == 0 {
            b.bell(t0, root + 24.0, 0.04);
        }
    }
    b.finish(0.55, 0.55)
}

/// Act 4's works: a heavy engine pulse, hissing steam and grinding metal over a low organ.
fn engine(rng: &mut Rng) -> Vec<f32> {
    let secs = 48.0;
    let mut b = Buf::new(secs);
    b.drone(28.0, 0.07);
    b.wind(0.08, rng);
    let chords = [(40.0, 43.0, 47.0), (38.0, 41.0, 45.0), (36.0, 40.0, 43.0), (35.0, 38.0, 42.0)];
    for (k, &(a, c, e)) in chords.iter().enumerate() {
        b.pad(k as f32 * 12.0, 11.5, &[a, c, e], 0.1, 450.0);
    }
    let mut t = 0.5;
    let mut k = 0;
    while t < secs {
        // Piston: thump - clank.
        b.drum(t, 0.2, 40.0, rng);
        b.drum(t + 0.375, 0.07, 92.0, rng);
        if k % 4 == 3 {
            b.drum(t + 0.56, 0.05, 100.0, rng);
        }
        t += 0.75;
        k += 1;
    }
    b.finish(0.6, 0.6)
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

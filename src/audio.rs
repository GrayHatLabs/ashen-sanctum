//! Synthesised sound effects and music loops mixed in an SDL audio callback. No asset files.
use crate::game::Sfx;
use crate::music::{self, Track};
use crate::rng::Rng;
use sdl2::audio::{AudioCallback, AudioDevice, AudioSpecDesired};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

const RATE: i32 = music::RATE;
const MUSIC_GAIN: f32 = 0.32;
/// Crossfade between tracks, in samples.
const FADE: f32 = RATE as f32 * 2.5;

struct Song {
    buf: Arc<Vec<f32>>,
    pos: usize,
    /// 0..1 fade level, and which way it's heading.
    level: f32,
    rising: bool,
}

#[derive(Default)]
struct Music {
    songs: Vec<Song>,
    on: bool,
}

struct Voice {
    buf: Arc<Vec<f32>>,
    pos: usize,
    gain: f32,
}

pub struct Mixer {
    voices: Arc<Mutex<Vec<Voice>>>,
    music: Arc<Mutex<Music>>,
}

impl AudioCallback for Mixer {
    type Channel = f32;
    fn callback(&mut self, out: &mut [f32]) {
        out.fill(0.0);
        {
            let mut m = self.music.lock().unwrap();
            let on = m.on;
            for s in m.songs.iter_mut() {
                for o in out.iter_mut() {
                    let target = if s.rising && on { 1.0 } else { 0.0 };
                    if s.level < target {
                        s.level = (s.level + 1.0 / FADE).min(1.0);
                    } else if s.level > target {
                        s.level = (s.level - 1.0 / FADE).max(0.0);
                    }
                    *o += s.buf[s.pos] * s.level * MUSIC_GAIN;
                    s.pos = (s.pos + 1) % s.buf.len();
                }
            }
            m.songs.retain(|s| s.rising || s.level > 0.0);
        }
        let mut vs = self.voices.lock().unwrap();
        for v in vs.iter_mut() {
            for o in out.iter_mut() {
                if v.pos >= v.buf.len() {
                    break;
                }
                *o += v.buf[v.pos] * v.gain;
                v.pos += 1;
            }
        }
        vs.retain(|v| v.pos < v.buf.len());
        for o in out.iter_mut() {
            *o = o.clamp(-1.0, 1.0);
        }
    }
}

pub struct Audio {
    _dev: AudioDevice<Mixer>,
    voices: Arc<Mutex<Vec<Voice>>>,
    music: Arc<Mutex<Music>>,
    /// Rendered loops (filled by a background thread).
    tracks: Arc<Mutex<HashMap<Track, Arc<Vec<f32>>>>>,
    playing: Option<Track>,
    bank: Vec<(Sfx, Vec<Arc<Vec<f32>>>)>,
    rng: Rng,
}

impl Audio {
    pub fn open(sys: &sdl2::AudioSubsystem) -> Option<Audio> {
        let voices = Arc::new(Mutex::new(Vec::new()));
        let spec = AudioSpecDesired { freq: Some(RATE), channels: Some(1), samples: Some(512) };
        let v2 = voices.clone();
        let music = Arc::new(Mutex::new(Music { songs: vec![], on: true }));
        let m2 = music.clone();
        let dev = sys.open_playback(None, &spec, |_| Mixer { voices: v2, music: m2 }).ok()?;
        dev.resume();
        let mut rng = Rng::new(99);
        let bank = [Sfx::Cast, Sfx::Boom, Sfx::Hit, Sfx::Hurt, Sfx::Die, Sfx::Swing, Sfx::Pickup, Sfx::Drink, Sfx::Descend, Sfx::Eat]
            .into_iter()
            .map(|s| (s, (0..3).map(|_| Arc::new(synth(s, &mut rng))).collect()))
            .collect();
        // Render the music off the main thread; the first track wanted is done first.
        let tracks: Arc<Mutex<HashMap<Track, Arc<Vec<f32>>>>> = Arc::new(Mutex::new(HashMap::new()));
        let t2 = tracks.clone();
        std::thread::spawn(move || {
            for t in music::TRACKS {
                let buf = Arc::new(music::render(t));
                t2.lock().unwrap().insert(t, buf);
            }
        });
        Some(Audio { _dev: dev, voices, music, tracks, playing: None, bank, rng })
    }

    /// Crossfades to this track (once it has been rendered).
    pub fn set_music(&mut self, t: Track) {
        if self.playing == Some(t) {
            return;
        }
        let Some(buf) = self.tracks.lock().unwrap().get(&t).cloned() else { return };
        self.playing = Some(t);
        let mut m = self.music.lock().unwrap();
        for s in m.songs.iter_mut() {
            s.rising = false;
        }
        m.songs.push(Song { buf, pos: 0, level: 0.0, rising: true });
    }

    /// Music on / off (N). Returns the new state.
    pub fn toggle_music(&mut self) -> bool {
        let mut m = self.music.lock().unwrap();
        m.on = !m.on;
        m.on
    }

    pub fn play(&mut self, s: Sfx) {
        let Some((_, vars)) = self.bank.iter().find(|b| b.0 == s) else { return };
        let buf = vars[self.rng.range(0, vars.len() as i32) as usize].clone();
        let mut vs = self.voices.lock().unwrap();
        if vs.len() < 24 {
            vs.push(Voice { buf, pos: 0, gain: 0.5 });
        }
    }
}

fn synth(s: Sfx, rng: &mut Rng) -> Vec<f32> {
    let secs = match s {
        Sfx::Cast => 0.35,
        Sfx::Boom => 0.7,
        Sfx::Hit => 0.12,
        Sfx::Hurt => 0.2,
        Sfx::Die => 0.45,
        Sfx::Swing => 0.18,
        Sfx::Pickup => 0.2,
        Sfx::Drink => 0.35,
        Sfx::Descend => 1.2,
        Sfx::Eat => 0.3,
    };
    let n = (secs * RATE as f32) as usize;
    let mut out = vec![0.0f32; n];
    let mut lp = 0.0f32;
    let mut phase = 0.0f32;
    let detune = rng.rf(0.92, 1.08);
    for (i, o) in out.iter_mut().enumerate() {
        let t = i as f32 / RATE as f32;
        let k = t / secs; // 0..1
        let noise = rng.f() * 2.0 - 1.0;
        let v = match s {
            Sfx::Cast => {
                // Rising filtered whoosh.
                let cut = 0.05 + 0.25 * k;
                lp += (noise - lp) * cut;
                lp * (k * 8.0).min(1.0) * (1.0 - k).powi(2) * 1.6
            }
            Sfx::Boom => {
                let cut = 0.25 * (1.0 - k) + 0.02;
                lp += (noise - lp) * cut;
                phase += (70.0 - 40.0 * k) * detune / RATE as f32;
                (lp * 1.4 + (phase * std::f32::consts::TAU).sin() * 0.6) * (1.0 - k).powi(3)
            }
            Sfx::Hit => {
                lp += (noise - lp) * 0.4;
                lp * (1.0 - k).powi(2)
            }
            Sfx::Hurt => {
                phase += (180.0 - 80.0 * k) * detune / RATE as f32;
                let sq = if phase.fract() < 0.5 { 0.4 } else { -0.4 };
                (sq + noise * 0.2) * (1.0 - k)
            }
            Sfx::Die => {
                phase += (140.0 * (1.0 - 0.6 * k)) * detune / RATE as f32;
                lp += (noise - lp) * 0.1;
                ((phase * std::f32::consts::TAU).sin() * 0.4 + lp * 0.8) * (1.0 - k).powi(2)
            }
            Sfx::Swing => {
                lp += (noise - lp) * (0.05 + 0.2 * (1.0 - (k - 0.5).abs() * 2.0));
                lp * (1.0 - (k - 0.4).abs() * 2.0).max(0.0) * 1.3
            }
            Sfx::Pickup => {
                let f = if k < 0.5 { 880.0 } else { 1320.0 };
                phase += f / RATE as f32;
                (phase * std::f32::consts::TAU).sin() * 0.25 * (1.0 - k)
            }
            Sfx::Drink => {
                phase += (300.0 + 200.0 * (t * 30.0).sin().abs()) / RATE as f32;
                (phase * std::f32::consts::TAU).sin() * 0.25 * (1.0 - k)
            }
            Sfx::Eat => {
                // Three quick crunches.
                let c = (k * 3.0).fract();
                lp += (noise - lp) * 0.5;
                lp * (1.0 - c).powi(3) * 0.9
            }
            Sfx::Descend => {
                phase += (110.0 - 50.0 * k) / RATE as f32;
                lp += (noise - lp) * 0.02;
                ((phase * std::f32::consts::TAU).sin() * 0.35 + lp * 2.0) * (k * 4.0).min(1.0) * (1.0 - k)
            }
        };
        *o = v;
    }
    out
}

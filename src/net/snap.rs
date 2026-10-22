//! What the host sends a joiner: the level when it changes (`LevelData`), then many times a second a
//! snapshot of what's near the joiner's hero. Both are compressed; a joiner inflates them with a hard
//! limit and checks the level's shape before using it, so a bad host can't make it allocate wildly or
//! index out of bounds.
//!
//! The game's own types travel (heroes, monsters, missiles...), so a joiner's game holds the same things
//! as the host's and draws them with the same code.
#![forbid(unsafe_code)]

use crate::game::{Floater, Game, Pickup, Sfx};
use crate::mobs::{Hazard, Mob, Shot};
use crate::party::{Ally, HeroState};
use crate::story::{Npc, Quest};
use crate::world::{LevelId, Portal, Prop, Theme};
use serde::{Deserialize, Serialize};

/// How far around a joiner's hero the world is sent (tiles; the screen is about 20 across).
pub const VIEW: f32 = 28.0;
/// The most a level or a snapshot may inflate to.
const MAX_LEVEL: usize = 8 * 1024 * 1024;
const MAX_SNAP: usize = 4 * 1024 * 1024;
/// The biggest map side accepted (the largest the game makes is well under this).
const MAX_SIDE: i32 = 1024;

/// Everything about the place itself (sent on arrival).
#[derive(Serialize)]
struct LevelOut<'a> {
    id: LevelId,
    name: &'a str,
    theme: Theme,
    tier: f32,
    d: &'a crate::dungeon::Dungeon,
    props: &'a [Prop],
    npcs: &'a [Npc],
    safe: Option<(f32, f32, f32, f32)>,
    start: (f32, f32),
    waypoint: (f32, f32),
}

#[derive(Deserialize)]
struct LevelIn {
    id: LevelId,
    name: String,
    theme: Theme,
    tier: f32,
    d: crate::dungeon::Dungeon,
    props: Vec<Prop>,
    npcs: Vec<Npc>,
    safe: Option<(f32, f32, f32, f32)>,
    start: (f32, f32),
    waypoint: (f32, f32),
}

/// Where a person in town stands and faces (they stroll; the rest of them came with the level).
#[derive(Serialize, Deserialize, Clone, Copy)]
struct NpcAt {
    x: f32,
    y: f32,
    dir: usize,
    moving: bool,
    anim_t: f32,
}

/// One moment of the world near a joiner.
#[derive(Serialize)]
struct SnapOut<'a> {
    tick: u32,
    heroes: Vec<(u8, &'a HeroState)>,
    mobs: Vec<&'a Mob>,
    pickups: Vec<&'a Pickup>,
    shots: Vec<&'a Shot>,
    hazards: Vec<&'a Hazard>,
    portals: &'a [Portal],
    npcs: Vec<NpcAt>,
    quest: &'a Quest,
    sfx: &'a [Sfx],
    floaters: &'a [Floater],
}

#[derive(Deserialize)]
struct SnapIn {
    tick: u32,
    heroes: Vec<(u8, HeroState)>,
    mobs: Vec<Mob>,
    pickups: Vec<Pickup>,
    shots: Vec<Shot>,
    hazards: Vec<Hazard>,
    portals: Vec<Portal>,
    npcs: Vec<NpcAt>,
    quest: Quest,
    sfx: Vec<Sfx>,
    floaters: Vec<Floater>,
}

fn pack<T: Serialize>(m: &T) -> Option<Vec<u8>> {
    let raw = postcard::to_stdvec(m).ok()?;
    Some(miniz_oxide::deflate::compress_to_vec(&raw, 3))
}

/// Compresses raw (already encoded) bytes the way `pack` does (the fuzz test mangles them in between).
#[cfg(test)]
fn deflate(raw: &[u8]) -> Vec<u8> {
    miniz_oxide::deflate::compress_to_vec(raw, 3)
}

fn unpack<T: for<'de> Deserialize<'de>>(bytes: &[u8], max: usize) -> Option<T> {
    let raw = miniz_oxide::inflate::decompress_to_vec_with_limit(bytes, max).ok()?;
    super::proto::decode::<T>(&raw)
}

/// A pickup the game can show: the quest things and meals name one of three, items are real ones.
fn drop_ok(d: &crate::game::Drop) -> bool {
    use crate::game::Drop::*;
    match d {
        Food(i) | Seal(i) | Rune(i) | Sigil(i) | Key(i) | Pearl(i) | Shard(i) | AKey(i) => *i < 3,
        Word(i) => (*i as usize) < crate::churnside::WORDS.len(),
        Item(it) => crate::online::item_ok(it),
        _ => true,
    }
}

fn finite(x: f32, y: f32) -> bool {
    x.is_finite() && y.is_finite()
}

impl Game {
    /// The level, packed for joiners.
    pub fn net_level(&self) -> Option<Vec<u8>> {
        pack(&LevelOut {
            id: self.level,
            name: &self.level_name,
            theme: self.theme,
            tier: self.tier,
            d: &self.d,
            props: &self.props,
            npcs: &self.npcs,
            safe: self.safe,
            start: self.start,
            waypoint: self.waypoint,
        })
    }

    /// What hero `hero` (party numbering: 0 the host) can see, packed, with the sounds and floating
    /// numbers since its last one.
    pub fn net_snapshot(&mut self, hero: u8, sfx: &[Sfx], floaters: &[Floater]) -> Option<Vec<u8>> {
        self.snapshot_bytes(hero, sfx, floaters, true)
    }

    /// The snapshot, compressed (to send) or raw (the fuzz test).
    fn snapshot_bytes(&mut self, hero: u8, sfx: &[Sfx], floaters: &[Floater], compress: bool) -> Option<Vec<u8>> {
        // The host's own hero, as a HeroState like the others (put back straight after).
        let mut lead = HeroState::new();
        self.swap_with(&mut lead);
        let (hx, hy) = match hero {
            0 => (lead.p.x, lead.p.y),
            k => self.allies.get(k as usize - 1).map_or((lead.p.x, lead.p.y), |a| (a.st.p.x, a.st.p.y)),
        };
        let near = |x: f32, y: f32| (x - hx).powi(2) + (y - hy).powi(2) < VIEW * VIEW;
        // Everyone else sees a public view of each hero: no bag, stash, waypoints or records (review 2026-10-22).
        let mut hidden: Vec<(u8, Vec<Option<crate::items::Item>>, Vec<Option<crate::items::Item>>, Vec<crate::world::LevelId>, crate::extras::Extras)> = vec![];
        {
            let mut hide = |k: u8, st: &mut HeroState| {
                if k != hero {
                    let bag = std::mem::replace(&mut st.p.gear.bag, vec![None; crate::items::BAG]);
                    let stash = std::mem::replace(&mut st.p.gear.stash, vec![None; crate::items::STASH]);
                    hidden.push((k, bag, stash, std::mem::take(&mut st.waypoints), std::mem::take(&mut st.ex)));
                }
            };
            hide(0, &mut lead);
            for (k, a) in self.allies.iter_mut().enumerate() {
                hide(k as u8 + 1, &mut a.st);
            }
        }
        let mut heroes = vec![(0u8, &lead)];
        heroes.extend(self.allies.iter().enumerate().map(|(k, a)| (k as u8 + 1, &a.st)));
        let out = SnapOut {
            tick: self.tick,
            heroes,
            mobs: self.mobs.iter().filter(|m| near(m.x, m.y)).collect(),
            pickups: self.pickups.iter().filter(|k| k.for_hero(hero as usize) && near(k.x, k.y)).collect(),
            shots: self.shots.iter().filter(|s| near(s.x, s.y)).collect(),
            hazards: self.hazards.iter().filter(|h| near(h.x, h.y)).collect(),
            portals: &self.portals,
            npcs: self.npcs.iter().map(|n| NpcAt { x: n.x, y: n.y, dir: n.dir, moving: n.moving, anim_t: n.anim_t }).collect(),
            quest: &self.quest,
            sfx,
            floaters,
        };
        let packed = if compress { pack(&out) } else { postcard::to_stdvec(&out).ok() };
        drop(out);
        for (k, bag, stash, wps, ex) in hidden {
            let st = if k == 0 { &mut lead } else { &mut self.allies[k as usize - 1].st };
            (st.p.gear.bag, st.p.gear.stash, st.waypoints, st.ex) = (bag, stash, wps, ex);
        }
        self.swap_with(&mut lead);
        packed
    }

    /// A joiner arrives somewhere: the host's level replaces its own. False if it's malformed.
    pub fn apply_level(&mut self, bytes: &[u8]) -> bool {
        let Some(lv) = unpack::<LevelIn>(bytes, MAX_LEVEL) else { return false };
        let d = &lv.d;
        let n = (d.w as i64 * d.h as i64).max(0) as usize;
        let shaped = d.w > 0 && d.h > 0 && d.w <= MAX_SIDE && d.h <= MAX_SIDE && d.tiles.len() == n && d.var.len() == n && d.ground.len() == n && d.liquid.len() == n;
        if !shaped || !crate::world::level_ok(lv.id) || !lv.tier.is_finite() || !finite(lv.start.0, lv.start.1) || !finite(lv.waypoint.0, lv.waypoint.1) {
            return false;
        }
        if lv.props.iter().any(|p| !finite(p.x, p.y) || !p.depth.is_finite()) || lv.npcs.iter().any(|p| !finite(p.x, p.y)) {
            return false;
        }
        self.level = lv.id;
        self.level_name = crate::net::proto::clean_chat(&lv.name).unwrap_or_default();
        self.theme = lv.theme;
        self.tier = lv.tier;
        self.explored = vec![false; n];
        self.d = lv.d;
        self.props = lv.props;
        self.npcs = lv.npcs;
        self.safe = lv.safe;
        self.start = lv.start;
        self.waypoint = lv.waypoint;
        self.mobs.clear();
        self.pickups.clear();
        self.shots.clear();
        self.hazards.clear();
        self.parts.clear();
        self.floaters.clear();
        self.decals.clear();
        self.portals.clear();
        self.light_ready = false;
        self.banner_t = 3.0;
        self.party_level = self.level;
        true
    }

    /// A joiner's world catches up with the host's: its own hero (number `me`) in the active slot, the
    /// others as allies, and what's near. False if it's malformed (nothing is changed then).
    pub fn apply_snapshot(&mut self, bytes: &[u8], me: u8) -> bool {
        let Some(mut s) = unpack::<SnapIn>(bytes, MAX_SNAP) else { return false };
        if s.heroes.len() > 8 || !s.heroes.iter().any(|(k, _)| *k == me) || s.heroes.iter().any(|(_, h)| !finite(h.p.x, h.p.y)) {
            return false;
        }
        // Things somewhere impossible are dropped rather than drawn.
        s.mobs.retain(|m| finite(m.x, m.y) && m.hp.is_finite() && m.max_hp.is_finite() && (m.superu as usize) <= crate::side::SUPERS.len());
        s.pickups.retain(|k| finite(k.x, k.y) && drop_ok(&k.kind));
        s.shots.retain(|k| finite(k.x, k.y));
        s.hazards.retain(|k| finite(k.x, k.y) && k.r.is_finite());
        let here = self.level;
        s.portals.retain(|k| finite(k.x, k.y) && crate::world::portal_ok(here, k.kind));
        for (_, h) in s.heroes.iter_mut() {
            h.waypoints.retain(|w| crate::world::level_ok(*w));
            if h.tp_return.map_or(false, |(l, x, y)| !crate::world::level_ok(l) || !finite(x, y)) {
                h.tp_return = None;
            }
        }
        let _ = s.tick;
        self.allies.clear();
        for (k, st) in s.heroes {
            if k == me {
                let mut st = st;
                self.swap_with(&mut st);
            } else {
                self.allies.push(Ally { st, inp: Default::default(), target: None, respawn: 0.0 });
            }
        }
        self.mobs = s.mobs;
        self.pickups = s.pickups;
        self.shots = s.shots;
        self.hazards = s.hazards;
        self.portals = s.portals;
        for (n, at) in self.npcs.iter_mut().zip(s.npcs) {
            if finite(at.x, at.y) {
                (n.x, n.y, n.dir, n.moving, n.anim_t) = (at.x, at.y, at.dir.min(7), at.moving, at.anim_t);
            }
        }
        self.quest = s.quest;
        // The host's sounds and numbers (a handful: more than that is noise, or mischief).
        self.sfx.extend(s.sfx.into_iter().take(24));
        for mut f in s.floaters.into_iter().take(48) {
            if finite(f.x, f.y) {
                f.text = crate::net::proto::clean_chat(&f.text).unwrap_or_default();
                f.t = 0.0;
                self.floaters.push(f);
            }
        }
        // Seen from here, you're the one in the active slot.
        self.cur_hero = 0;
        true
    }
}

#[cfg(test)]
mod tests {
    use crate::game::Game;
    use crate::world::LevelId;

    #[test]
    fn a_joiner_sees_what_the_host_sees_near_it() {
        let mut host = Game::new(7, crate::gfx::SH_WIDE);
        host.set_class(crate::skills::Class::Sorceress);
        host.debug_goto(LevelId::Area(0, 1));
        let k = host.add_ally(crate::skills::Class::Valkyrie, "BRYNJA");
        host.party_level = host.level;
        for _ in 0..30 {
            host.update(&Default::default());
        }
        let level = host.net_level().expect("packs");
        let snap = host.net_snapshot(k as u8 + 1, &[], &[]).expect("packs");
        println!("level {} bytes, snapshot {} bytes, {} monsters", level.len(), snap.len(), host.mobs.len());
        assert!(snap.len() < 64 * 1024, "a snapshot is small enough to send 20 times a second");
        let mut joiner = Game::new(99, crate::gfx::SH_WIDE);
        assert!(joiner.apply_level(&level));
        assert_eq!(joiner.level, host.level);
        assert_eq!(joiner.d.tiles, host.d.tiles);
        assert!(joiner.apply_snapshot(&snap, k as u8 + 1));
        // You are the ally; the host is one of your allies.
        let ally = &host.allies[k].st.p;
        assert_eq!((joiner.p.x, joiner.p.y), (ally.x, ally.y));
        assert_eq!(joiner.p.skills.class, crate::skills::Class::Valkyrie);
        assert_eq!(joiner.hero_name, "BRYNJA");
        assert_eq!(joiner.allies.len(), 1);
        assert_eq!(joiner.allies[0].st.p.skills.class, crate::skills::Class::Sorceress);
        // Only what's near came, and every monster is the host's.
        assert!(joiner.mobs.iter().all(|m| host.mobs.iter().any(|h| h.uid == m.uid)));
        // The host's own state is untouched by building it.
        assert_eq!(host.cur_hero, 0);
        assert_eq!(host.p.skills.class, crate::skills::Class::Sorceress);
        // It draws.
        let mut scr = crate::gfx::Screen::new(crate::gfx::SH_WIDE);
        joiner.draw(&mut scr);
    }

    #[test]
    fn mangled_snapshots_never_crash_a_joiner() {
        // A host where every class is fighting, so a snapshot carries every kind of missile, summon and effect.
        let mut host = Game::new(7, crate::gfx::SH_WIDE);
        host.set_class(crate::skills::Class::Sorceress);
        host.debug_goto(LevelId::Area(0, 1));
        for c in crate::skills::ALL_CLASSES.iter().copied().skip(1) {
            let k = host.add_ally(c, "ALLY");
            host.as_hero(k, |g| {
                g.act_gear(3);
                for s in c.tree() {
                    g.p.skills.rank[*s as usize] = 3;
                }
            });
        }
        host.party_level = host.level;
        for t in 0..240u32 {
            for a in host.allies.iter_mut() {
                a.inp = crate::game::Input { cast: t % 3 != 0, cast2: t % 3 == 0, cycle: t % 40 == 0, ..Default::default() };
            }
            let inp = crate::game::Input { cast: t % 2 == 0, cast2: t % 2 == 1, ..Default::default() };
            host.update(&inp);
        }
        let level = host.net_level().unwrap();
        let raw = host.snapshot_bytes(1, &[], &[], false).unwrap();
        let mut rng = crate::rng::Rng::new(std::env::var("FUZZ_SEED").ok().and_then(|v| v.parse().ok()).unwrap_or(1234));
        let mut panics: Vec<String> = vec![];
        let mut applied = 0;
        let mut scr = crate::gfx::Screen::new(crate::gfx::SH_WIDE);
        let mut joiner = Game::new(99, crate::gfx::SH_WIDE);
        assert!(joiner.apply_level(&level));
        let hook = std::panic::take_hook();
        static WHERE: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());
        std::panic::set_hook(Box::new(|info| {
            if let Some(l) = info.location() {
                WHERE.lock().unwrap_or_else(|e| e.into_inner()).push(format!("{}:{}", l.file(), l.line()));
            }
        }));
        for _ in 0..3000 {
            let mut b = raw.clone();
            for _ in 0..rng.range(1, 9) {
                let i = rng.range(0, b.len() as i32) as usize;
                b[i] = if rng.chance(0.3) { 0xff } else { rng.range(0, 256) as u8 };
            }
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                if joiner.apply_snapshot(&super::deflate(&b), 1) {
                    joiner.draw(&mut scr);
                    true
                } else {
                    false
                }
            }));
            match r {
                Ok(true) => applied += 1,
                Ok(false) => {}
                Err(e) => {
                    let m = e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default();
                    let at = WHERE.lock().unwrap_or_else(|e| e.into_inner()).pop().unwrap_or_default();
                    let m = format!("{m} at {at}");
                    if !panics.contains(&m) {
                        panics.push(m);
                    }
                    joiner = Game::new(99, crate::gfx::SH_WIDE);
                    assert!(joiner.apply_level(&level));
                }
            }
        }
        std::panic::set_hook(hook);
        println!("{applied} mangled snapshots applied and drawn; panics: {panics:#?}");
        assert!(applied > 300, "enough of them decode to test the drawing");
        assert!(panics.is_empty(), "a joiner must not crash on what a host sends: {panics:#?}");
    }

    #[test]
    fn a_bad_level_or_snapshot_is_refused() {
        let mut g = Game::new(7, crate::gfx::SH_WIDE);
        for junk in [vec![], vec![0u8; 10], vec![0x78, 0x9c, 1, 2, 3], vec![0xff; 1000]] {
            assert!(!g.apply_level(&junk));
            assert!(!g.apply_snapshot(&junk, 1));
        }
        // A level whose tiles don't match its size.
        let mut lv = Game::new(7, crate::gfx::SH_WIDE);
        lv.d.tiles.pop();
        let bytes = lv.net_level().unwrap();
        assert!(!g.apply_level(&bytes), "short tiles refused");
        // A snapshot that doesn't include you.
        let mut h = Game::new(7, crate::gfx::SH_WIDE);
        let s = h.net_snapshot(0, &[], &[]).unwrap();
        assert!(!g.apply_snapshot(&s, 3));
        assert!(g.apply_snapshot(&s, 0));
    }
}

//! What the host sends a joiner: the level when it changes (`LevelData`), then many times a second a
//! snapshot of what's near the joiner's hero. Both are compressed; a joiner inflates them with a hard
//! limit and checks the level's shape before using it, so a bad host can't make it allocate wildly or
//! index out of bounds.
//!
//! The game's own types travel (heroes, monsters, missiles...), so a joiner's game holds the same things
//! as the host's and draws them with the same code.
#![forbid(unsafe_code)]

use crate::game::{Game, Pickup};
use crate::mobs::{Hazard, Mob, Shot};
use crate::party::{Ally, HeroState};
use crate::story::{Npc, Quest};
use crate::world::{LevelId, Portal, Prop, Theme};
use serde::{Deserialize, Serialize};

/// How far around a joiner's hero the world is sent (tiles; the screen is about 20 across).
pub const VIEW: f32 = 28.0;
/// The most a level or a snapshot may inflate to.
const MAX_LEVEL: usize = 32 * 1024 * 1024;
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
}

fn pack<T: Serialize>(m: &T) -> Option<Vec<u8>> {
    let raw = postcard::to_stdvec(m).ok()?;
    Some(miniz_oxide::deflate::compress_to_vec(&raw, 3))
}

fn unpack<T: for<'de> Deserialize<'de>>(bytes: &[u8], max: usize) -> Option<T> {
    let raw = miniz_oxide::inflate::decompress_to_vec_with_limit(bytes, max).ok()?;
    super::proto::decode::<T>(&raw)
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

    /// What hero `hero` (party numbering: 0 the host) can see, packed.
    pub fn net_snapshot(&mut self, hero: u8) -> Option<Vec<u8>> {
        // The host's own hero, as a HeroState like the others (put back straight after).
        let mut lead = HeroState::new();
        self.swap_with(&mut lead);
        let (hx, hy) = match hero {
            0 => (lead.p.x, lead.p.y),
            k => self.allies.get(k as usize - 1).map_or((lead.p.x, lead.p.y), |a| (a.st.p.x, a.st.p.y)),
        };
        let near = |x: f32, y: f32| (x - hx).powi(2) + (y - hy).powi(2) < VIEW * VIEW;
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
        };
        let packed = pack(&out);
        drop(out);
        self.swap_with(&mut lead);
        packed
    }

    /// A joiner arrives somewhere: the host's level replaces its own. False if it's malformed.
    pub fn apply_level(&mut self, bytes: &[u8]) -> bool {
        let Some(lv) = unpack::<LevelIn>(bytes, MAX_LEVEL) else { return false };
        let d = &lv.d;
        let n = (d.w as i64 * d.h as i64).max(0) as usize;
        let shaped = d.w > 0 && d.h > 0 && d.w <= MAX_SIDE && d.h <= MAX_SIDE && d.tiles.len() == n && d.var.len() == n && d.ground.len() == n && d.liquid.len() == n;
        if !shaped || !lv.tier.is_finite() || !finite(lv.start.0, lv.start.1) || !finite(lv.waypoint.0, lv.waypoint.1) {
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
        s.mobs.retain(|m| finite(m.x, m.y) && m.hp.is_finite() && m.max_hp.is_finite());
        s.pickups.retain(|k| finite(k.x, k.y));
        s.shots.retain(|k| finite(k.x, k.y));
        s.hazards.retain(|k| finite(k.x, k.y) && k.r.is_finite());
        s.portals.retain(|k| finite(k.x, k.y));
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
        let snap = host.net_snapshot(k as u8 + 1).expect("packs");
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
        let s = h.net_snapshot(0).unwrap();
        assert!(!g.apply_snapshot(&s, 3));
        assert!(g.apply_snapshot(&s, 0));
    }
}

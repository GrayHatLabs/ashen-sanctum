//! Outdoor areas (docs/AREAS_PLAN.md): Diablo II-style acts made of a town map and a chain of wild areas
//! you walk between, each with its own waypoint, a dungeon door or none, side branches, and the pass to the
//! next act in the last one.
//!
//! `LevelId::Area(act, n)` is area `n` (from 1) of an act; the act's town map keeps its old id
//! (`LevelId::Overworld` for Hollowmere). `PortalKind::Exit(n)` is a road off the edge of the map into
//! area `n` of the same act (0 = the town).
use crate::dungeon::{Dungeon, Tile};
use crate::game::{Drop, Pickup};
use crate::mobs::{Kind, Mob};
use crate::rng::Rng;
use crate::story::{Npc, Role};
use crate::world::{empty_level, Level, LevelId, Noise, Portal, PortalKind, Prop, PropKind, Theme, DUNGEONS, TOWN};

/// A map edge.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Side {
    N,
    E,
    S,
    W,
}

/// How an area looks: tree cover (noise threshold; lower is denser), the share of dead trees, ash and dirt
/// (noise threshold; lower is more), and loose rocks (chance per tile).
#[derive(Clone, Copy, Debug)]
pub struct Style {
    pub forest: f32,
    pub dead: f32,
    pub dirt: f32,
    pub rocks: f32,
}

pub struct AreaDef {
    pub act: u8,
    pub n: u8,
    /// Level file name.
    pub slug: &'static str,
    pub name: &'static str,
    pub tier: f32,
    pub size: (i32, i32),
    /// Roads off the map: (edge, where along it 0..1, the area it leads to; 0 = the town). The first one
    /// leads back toward town: you arrive there and its waypoint stands near it.
    pub exits: &'static [(Side, f32, u8)],
    /// Dungeon doors: (dungeon, the door tile).
    pub doors: &'static [(usize, (i32, i32))],
    /// The pass to the next act: (the act it leads to, the door tile).
    pub pass: Option<(usize, (i32, i32))>,
    pub monsters: &'static [Kind],
    /// Roaming packs.
    pub packs: usize,
    pub style: Style,
}

const FIELDS: Style = Style { forest: 0.6, dead: 0.2, dirt: 0.3, rocks: 0.03 };

pub const AREAS: &[AreaDef] = &[
    // ---- Act 1: the Ashlands ----
    AreaDef {
        act: 0,
        n: 1,
        slug: "the_ashlands",
        name: "THE ASHLANDS",
        tier: 1.0,
        size: (96, 96),
        exits: &[(Side::S, 0.5, 0), (Side::N, 0.55, 2), (Side::W, 0.45, 7)],
        doors: &[(crate::world::CHARNEL, (68, 62))],
        pass: None,
        monsters: &[Kind::Wolf, Kind::Goblin],
        packs: 16,
        style: FIELDS,
    },
    AreaDef {
        act: 0,
        n: 2,
        slug: "the_barrow_fields",
        name: "THE BARROW FIELDS",
        tier: 1.3,
        size: (96, 96),
        exits: &[(Side::S, 0.55, 1), (Side::N, 0.4, 3), (Side::E, 0.5, 8)],
        doors: &[(0, (26, 34))],
        pass: None,
        monsters: &[Kind::Zombie, Kind::Skeleton, Kind::Wolf],
        packs: 20,
        style: Style { forest: 0.66, dead: 0.45, dirt: 0.32, rocks: 0.04 },
    },
    AreaDef {
        act: 0,
        n: 3,
        slug: "the_rotwood",
        name: "THE ROTWOOD",
        tier: 1.7,
        size: (96, 96),
        exits: &[(Side::S, 0.4, 2), (Side::N, 0.6, 4)],
        doors: &[(1, (70, 40))],
        pass: None,
        monsters: &[Kind::Zombie, Kind::Goblin, Kind::Wolf],
        packs: 20,
        style: Style { forest: 0.48, dead: 0.25, dirt: 0.25, rocks: 0.02 },
    },
    AreaDef {
        act: 0,
        n: 4,
        slug: "the_cinder_hills",
        name: "THE CINDER HILLS",
        tier: 2.1,
        size: (96, 96),
        exits: &[(Side::S, 0.6, 3), (Side::E, 0.4, 5)],
        doors: &[(2, (30, 30))],
        pass: None,
        monsters: &[Kind::Skeleton, Kind::Archer, Kind::Goblin],
        packs: 22,
        style: Style { forest: 0.7, dead: 0.6, dirt: 0.4, rocks: 0.08 },
    },
    AreaDef {
        act: 0,
        n: 5,
        slug: "the_cinder_waste",
        name: "THE CINDER WASTE",
        tier: 2.6,
        size: (112, 80),
        exits: &[(Side::W, 0.5, 4), (Side::N, 0.7, 6)],
        doors: &[],
        pass: None,
        monsters: &[Kind::Zombie, Kind::Skeleton, Kind::Archer, Kind::Goblin],
        packs: 28,
        style: Style { forest: 0.78, dead: 0.85, dirt: 0.55, rocks: 0.05 },
    },
    AreaDef {
        act: 0,
        n: 6,
        slug: "the_ashen_steppe",
        name: "THE ASHEN STEPPE",
        tier: 3.0,
        size: (96, 96),
        exits: &[(Side::S, 0.5, 5)],
        doors: &[(3, (30, 44))],
        pass: Some((1, (60, 8))),
        monsters: &[Kind::Skeleton, Kind::Archer, Kind::Zombie, Kind::Goblin],
        packs: 22,
        style: Style { forest: 0.72, dead: 0.7, dirt: 0.45, rocks: 0.05 },
    },
    AreaDef {
        act: 0,
        n: 7,
        slug: "shepherds_vale",
        name: "SHEPHERD'S VALE",
        tier: 1.2,
        size: (64, 64),
        exits: &[(Side::E, 0.5, 1)],
        doors: &[],
        pass: None,
        monsters: &[Kind::Zombie, Kind::Wolf],
        packs: 7,
        style: Style { forest: 0.62, dead: 0.3, dirt: 0.35, rocks: 0.02 },
    },
    AreaDef {
        act: 0,
        n: 8,
        slug: "skrats_gulch",
        name: "SKRAT'S GULCH",
        tier: 1.5,
        size: (64, 64),
        exits: &[(Side::W, 0.5, 2)],
        doors: &[],
        pass: None,
        monsters: &[Kind::Goblin, Kind::Wolf],
        packs: 8,
        style: Style { forest: 0.58, dead: 0.3, dirt: 0.4, rocks: 0.06 },
    },
];

/// The town maps' roads out: (act, edge, where along it, the area it leads to).
pub const TOWN_EXITS: &[(u8, Side, f32, u8)] = &[(0, Side::N, 0.69, 1)];

pub fn def(act: u8, n: u8) -> &'static AreaDef {
    AREAS.iter().find(|a| a.act == act && a.n == n).expect("area")
}

/// The area number of an outdoor level (0 for a town map).
pub fn number(id: LevelId) -> u8 {
    match id {
        LevelId::Area(_, n) => n,
        _ => 0,
    }
}

/// Area `n` of an act (0: the act's town map).
pub fn level(act: usize, n: u8) -> LevelId {
    if n == 0 {
        LevelId::land(act)
    } else {
        LevelId::Area(act as u8, n)
    }
}

/// Where a dungeon's door stands: its area, or the act's overland.
pub fn dungeon_home(k: usize) -> LevelId {
    AREAS
        .iter()
        .find(|a| a.doors.iter().any(|d| d.0 == k))
        .map(|a| LevelId::Area(a.act, a.n))
        .unwrap_or_else(|| LevelId::land(DUNGEONS[k].act))
}

/// The outdoor level of `act` that holds the pass toward act `to` (coming back from the next act lands you
/// there, not in town).
pub fn pass_home(act: usize, to: usize) -> LevelId {
    AREAS
        .iter()
        .find(|a| a.act as usize == act && a.pass.map_or(false, |p| p.0 == to))
        .map(|a| LevelId::Area(a.act, a.n))
        .unwrap_or_else(|| LevelId::land(act))
}

/// The areas next to this outdoor level.
pub fn neighbours(id: LevelId) -> Vec<LevelId> {
    let act = id.act();
    match id {
        LevelId::Area(a, n) => def(a, n).exits.iter().map(|e| level(act, e.2)).collect(),
        _ => TOWN_EXITS.iter().filter(|e| e.0 as usize == act).map(|e| level(act, e.3)).collect(),
    }
}

/// The next area on the way from `from` to `to` (both outdoor levels of one act), for the test bot.
pub fn route(from: LevelId, to: LevelId) -> Option<LevelId> {
    if from == to {
        return None;
    }
    let mut prev: Vec<(LevelId, LevelId)> = vec![(from, from)];
    let mut queue = std::collections::VecDeque::from([from]);
    while let Some(cur) = queue.pop_front() {
        if cur == to {
            let mut step = cur;
            while let Some(&(s, p)) = prev.iter().find(|(s, _)| *s == step) {
                if p == from {
                    return Some(s);
                }
                step = p;
            }
            return None;
        }
        for nb in neighbours(cur) {
            if !prev.iter().any(|(s, _)| *s == nb) {
                prev.push((nb, cur));
                queue.push_back(nb);
            }
        }
    }
    None
}

/// How many areas apart two outdoor levels of one act are (None: not connected).
pub fn steps(from: LevelId, to: LevelId) -> Option<usize> {
    let mut at = from;
    for n in 0..32 {
        if at == to {
            return Some(n);
        }
        at = route(at, to)?;
    }
    None
}

/// The tile a road leaves the map at: two tiles in from the edge.
fn exit_tile(side: Side, at: f32, (w, h): (i32, i32)) -> (i32, i32) {
    let fx = ((w as f32 * at) as i32).clamp(4, w - 5);
    let fy = ((h as f32 * at) as i32).clamp(4, h - 5);
    match side {
        Side::N => (fx, 2),
        Side::S => (fx, h - 3),
        Side::W => (2, fy),
        Side::E => (w - 3, fy),
    }
}

/// A few tiles inward from an exit (where you arrive).
fn inward(side: Side, (x, y): (i32, i32), k: i32) -> (i32, i32) {
    match side {
        Side::N => (x, y + k),
        Side::S => (x, y - k),
        Side::W => (x + k, y),
        Side::E => (x - k, y),
    }
}

/// The theme of an act's outdoors.
fn theme_of(act: u8) -> Theme {
    match act {
        1 => Theme::Tundra,
        2 => Theme::Mistwood,
        3 => Theme::Mechanus,
        4 => Theme::Deep,
        5 => Theme::Heavens,
        _ => Theme::Overworld,
    }
}

/// Map-building helpers shared by the town map and the areas.
struct Builder {
    w: i32,
    h: i32,
    d: Dungeon,
    keep: Vec<bool>,
}

impl Builder {
    fn new(w: i32, h: i32, rng: &mut Rng) -> Self {
        let mut d = Dungeon::blank(w, h, Tile::Floor);
        for v in d.var.iter_mut() {
            *v = rng.range(0, 100) as u8;
        }
        Builder { w, h, d, keep: vec![false; (w * h) as usize] }
    }

    /// Marks tiles that must stay clear of trees and rocks.
    fn clear(&mut self, x: i32, y: i32, r: i32) {
        for yy in y - r..=y + r {
            for xx in x - r..=x + r {
                if xx >= 0 && yy >= 0 && xx < self.w && yy < self.h {
                    self.keep[(yy * self.w + xx) as usize] = true;
                }
            }
        }
    }

    /// A wandering road, two tiles wide.
    fn road(&mut self, (ax, ay): (i32, i32), (bx, by): (i32, i32), salt: f32) {
        let (mut x, mut y) = (ax as f32, ay as f32);
        let mut guard = 0;
        while ((x - bx as f32).abs() > 0.8 || (y - by as f32).abs() > 0.8) && guard < 600 {
            guard += 1;
            let (dx, dy) = (bx as f32 - x, by as f32 - y);
            let l = (dx * dx + dy * dy).sqrt();
            let wob = (guard as f32 * 0.17 + salt).sin() * 0.6;
            x += dx / l + (-dy / l) * wob * 0.5;
            y += dy / l + (dx / l) * wob * 0.5;
            for (ox, oy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                let (rx, ry) = (x as i32 + ox, y as i32 + oy);
                if self.d.get(rx, ry) == Tile::Floor {
                    self.d.set_ground(rx, ry, 2);
                }
            }
            self.clear(x as i32, y as i32, 2);
        }
    }

    fn prop(&mut self, lv: &mut Level, kind: PropKind, x0: i32, y0: i32, fw: i32, fh: i32) {
        for y in y0..y0 + fh {
            for x in x0..x0 + fw {
                self.d.set(x, y, Tile::Prop);
            }
        }
        lv.props.push(Prop::on(kind, x0, y0, fw, fh));
    }

    /// Trees, rocks and bushes everywhere not kept clear (a thick wall of trees at the border), and patches of
    /// ash and dirt.
    fn wilds(&mut self, lv: &mut Level, rng: &mut Rng, st: Style) {
        let forest = Noise::new(rng, 16, 9.0);
        let dirt = Noise::new(rng, 16, 7.0);
        for y in 0..self.h {
            for x in 0..self.w {
                if self.d.get(x, y) != Tile::Floor {
                    continue;
                }
                if dirt.at(x as f32, y as f32) < st.dirt && self.d.ground_at(x, y) == 0 {
                    self.d.set_ground(x, y, 1);
                }
                if self.keep[(y * self.w + x) as usize] {
                    continue;
                }
                let border = x < 3 || y < 3 || x >= self.w - 3 || y >= self.h - 3;
                let f = forest.at(x as f32, y as f32);
                let r = rng.f();
                let tree = if border { r < 0.9 } else { f > st.forest && r < 0.5 || r < 0.012 };
                if tree {
                    let kind = if rng.chance(st.dead) {
                        PropKind::TreeDead
                    } else if f > st.forest + 0.15 {
                        PropKind::TreePine
                    } else {
                        PropKind::TreeOak
                    };
                    self.prop(lv, kind, x, y, 1, 1);
                } else if r < st.rocks {
                    self.prop(lv, PropKind::Rock, x, y, 1, 1);
                } else if r < st.rocks + 0.012 {
                    self.prop(lv, PropKind::Bush, x, y, 1, 1);
                }
            }
        }
    }

    /// The road off the edge and its exit portal.
    fn exit(&mut self, lv: &mut Level, side: Side, at: f32, to: u8, center: (i32, i32), salt: f32) -> (i32, i32) {
        let t = exit_tile(side, at, (self.w, self.h));
        let edge = match side {
            Side::N => (t.0, 0),
            Side::S => (t.0, self.h - 1),
            Side::W => (0, t.1),
            Side::E => (self.w - 1, t.1),
        };
        self.road(center, t, salt);
        self.road(t, edge, salt);
        self.clear(t.0, t.1, 3);
        lv.portals.push(Portal { x: t.0 as f32 + 0.5, y: t.1 as f32 + 0.5, kind: PortalKind::Exit(to) });
        t
    }
}

/// An area of the wilds.
pub fn area(def: &AreaDef, seed: u64) -> Level {
    let id = LevelId::Area(def.act, def.n);
    let mut rng = Rng::new(seed ^ 0xA4EA_0000 ^ (def.act as u64 * 131 + def.n as u64 * 7919));
    let (w, h) = def.size;
    let mut b = Builder::new(w, h, &mut rng);
    let mut lv = empty_level(id, def.name.into(), theme_of(def.act), def.tier, Dungeon::blank(1, 1, Tile::Void));
    let center = (w / 2 + rng.range(-6, 7), h / 2 + rng.range(-6, 7));
    b.clear(center.0, center.1, 3);
    // ---- the roads off the map, to the doors and the pass ----
    let mut arrive = (center.0, center.1);
    for (i, &(side, at, to)) in def.exits.iter().enumerate() {
        let t = b.exit(&mut lv, side, at, to, center, i as f32 * 3.1);
        if i == 0 {
            arrive = inward(side, t, 4);
        }
    }
    for (i, &(k, (ex, ey))) in def.doors.iter().enumerate() {
        b.road(center, (ex, ey), 5.0 + i as f32);
        b.clear(ex, ey, 5);
        b.prop(&mut lv, PropKind::Entrance(k), ex - 1, ey - 3, 3, 3);
        lv.portals.push(Portal { x: ex as f32 + 0.5, y: ey as f32 + 0.5, kind: PortalKind::Entrance(k) });
    }
    if let Some((to, (px, py))) = def.pass {
        b.road(center, (px, py), 8.0);
        b.clear(px, py, 4);
        b.prop(&mut lv, PropKind::Pass, px - 1, py - 3, 3, 2);
        lv.portals.push(Portal { x: px as f32 + 0.5, y: py as f32 + 0.5, kind: PortalKind::Pass(to) });
    }
    b.clear(arrive.0, arrive.1, 4);
    b.wilds(&mut lv, &mut rng, def.style);
    // ---- roaming packs (none near where you arrive, the doors or the exits) ----
    let mut packs = 0;
    let (ax, ay) = (arrive.0 as f32 + 0.5, arrive.1 as f32 + 0.5);
    for _ in 0..800 {
        if packs >= def.packs {
            break;
        }
        let x = rng.range(5, w - 5) as f32 + 0.5;
        let y = rng.range(5, h - 5) as f32 + 0.5;
        let near = |px: f32, py: f32, r: f32| (px - x).powi(2) + (py - y).powi(2) < r * r;
        if near(ax, ay, 14.0) || b.d.blocked(x, y, 0.4) || lv.portals.iter().any(|p| near(p.x, p.y, 7.0)) {
            continue;
        }
        let kind = def.monsters[rng.range(0, def.monsters.len() as i32) as usize];
        // (Rounded, so a level file keeps it exactly.)
        let tier = (def.tier * rng.rf(0.9, 1.1) * 1000.0).round() / 1000.0;
        for _ in 0..rng.range(3, 6) {
            for _try in 0..10 {
                let (mx, my) = (x + rng.rf(-2.0, 2.0), y + rng.rf(-2.0, 2.0));
                if !b.d.blocked(mx, my, 0.35) {
                    lv.mobs.push(Mob::new(kind, mx, my, tier, &mut rng));
                    break;
                }
            }
        }
        packs += 1;
    }
    // ---- a little wild food ----
    let mut food = 0;
    for _ in 0..300 {
        if food >= 6 {
            break;
        }
        let (x, y) = (rng.range(5, w - 5) as f32 + 0.5, rng.range(5, h - 5) as f32 + 0.5);
        if !b.d.blocked(x, y, 0.3) {
            lv.pickups.push(Pickup { x, y, kind: Drop::Food(if rng.chance(0.75) { 0 } else { 1 }), t: 1.0 });
            food += 1;
        }
    }
    lv.explored = vec![false; (w * h) as usize];
    lv.d = b.d;
    lv.start = (ax, ay);
    lv
}

/// Hollowmere and the fields around it (Act 1's town map): the village, its people, and the north road out
/// into the Ashlands.
pub fn hollowmere(seed: u64) -> Level {
    let mut rng = Rng::new(seed ^ 0x0F0F_1234);
    let (w, h) = (80, 80);
    let mut b = Builder::new(w, h, &mut rng);
    let mut lv = empty_level(LevelId::Overworld, "HOLLOWMERE".into(), Theme::Overworld, 1.0, Dungeon::blank(1, 1, Tile::Void));
    let (tx0, ty0, tx1, ty1) = TOWN;
    for y in ty0..=ty1 {
        for x in tx0..=tx1 {
            b.d.set_ground(x, y, 1);
            let edge = x == tx0 || x == tx1 || y == ty0 || y == ty1;
            let gate = (y == ty0 || y == ty1) && (54..=57).contains(&x) || (x == tx0 || x == tx1) && (59..=62).contains(&y);
            if edge && !gate {
                b.d.set(x, y, Tile::Wall);
            }
        }
    }
    b.clear(56, 60, 16);
    for x in tx0..=tx1 {
        for y in 60..=61 {
            b.d.set_ground(x, y, 2);
        }
    }
    for y in ty0..=ty1 {
        for x in 55..=56 {
            b.d.set_ground(x, y, 2);
        }
    }
    b.prop(&mut lv, PropKind::House1, 46, 52, 4, 4);
    b.prop(&mut lv, PropKind::House2, 62, 52, 4, 4);
    b.prop(&mut lv, PropKind::House2, 46, 65, 4, 3);
    b.prop(&mut lv, PropKind::House1, 62, 64, 4, 4);
    b.prop(&mut lv, PropKind::Stall, 59, 56, 3, 2);
    b.prop(&mut lv, PropKind::Well, 52, 56, 1, 1);
    b.prop(&mut lv, PropKind::Campfire, 58, 63, 1, 1);
    lv.safe = Some((tx0 as f32 - 1.0, ty0 as f32 - 1.0, tx1 as f32 + 2.0, ty1 as f32 + 2.0));
    lv.npcs = vec![
        Npc::new("ELDER MAREN", Role::Elder, "npc_elder", 57.5, 64.5, 6),
        Npc::new("GERTA", Role::Merchant, "npc_merchant", 60.5, 58.8, 0),
        Npc::new("BROTHER ALDRIC", Role::Healer, "npc_healer", 50.5, 58.5, 2),
        Npc::new("CAPTAIN ROLF", Role::Guard, "npc_guard", 57.8, 52.0, 0),
        Npc::new("VILLAGER", Role::Villager(0), "npc_villager", 53.5, 63.5, 1),
        Npc::new("FARMER", Role::Villager(1), "npc_villager", 61.5, 61.0, 7),
        Npc::new("VILLAGER", Role::Villager(2), "npc_villager", 49.5, 62.0, 2),
        Npc::new("FARMER", Role::Villager(3), "npc_villager", 64.0, 58.5, 5),
        Npc::new("MASTER ODO", Role::Jeweler(0), "npc_jeweler0", 55.5, 59.5, 1),
    ];
    // The north road out (from the north gate), and short lanes from the other gates into the fields.
    for &(act, side, at, to) in TOWN_EXITS.iter().filter(|e| e.0 == 0) {
        let _ = act;
        b.exit(&mut lv, side, at, to, (55, ty0 - 1), 2.0);
    }
    b.road((55, ty1 + 1), (55, ty1 + 6), 1.0);
    b.road((tx1 + 1, 60), (tx1 + 6, 60), 1.5);
    b.road((tx0 - 1, 60), (tx0 - 6, 60), 2.5);
    b.wilds(&mut lv, &mut rng, Style { forest: 0.62, dead: 0.2, dirt: 0.25, rocks: 0.02 });
    let mut food = 0;
    for _ in 0..200 {
        if food >= 5 {
            break;
        }
        let (x, y) = (rng.range(5, w - 5) as f32 + 0.5, rng.range(5, h - 5) as f32 + 0.5);
        let far = ((x - 56.5).powi(2) + (y - 61.5).powi(2)).sqrt();
        if far > 16.0 && !b.d.blocked(x, y, 0.3) {
            lv.pickups.push(Pickup { x, y, kind: Drop::Food(0), t: 1.0 });
            food += 1;
        }
    }
    lv.explored = vec![false; (w * h) as usize];
    lv.d = b.d;
    lv.start = crate::world::town_center();
    lv
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn act_one_is_a_chain_of_areas_you_can_walk() {
        // Every area's exits lead to areas that lead back.
        for a in AREAS {
            let here = LevelId::Area(a.act, a.n);
            for nb in neighbours(here) {
                assert!(neighbours(nb).contains(&here), "{} <-> {:?}", a.name, nb);
            }
        }
        // From town to the Ashen Steppe, one area at a time.
        let mut at = LevelId::Overworld;
        let goal = dungeon_home(crate::world::SANCTUM);
        assert_eq!(goal, LevelId::Area(0, 6));
        let mut steps = 0;
        while let Some(next) = route(at, goal) {
            at = next;
            steps += 1;
            assert!(steps < 10);
        }
        assert_eq!((at, steps), (goal, 6));
        assert_eq!(pass_home(0, 1), LevelId::Area(0, 6));
        assert_eq!(dungeon_home(crate::world::CHARNEL), LevelId::Area(0, 1));
    }

    #[test]
    fn every_area_connects_its_exits_doors_and_pass() {
        for a in AREAS {
            let lv = area(a, 7);
            let s = (lv.start.0 as i32, lv.start.1 as i32);
            assert!(lv.d.walkable(s.0, s.1), "{}: start", a.name);
            assert_eq!(lv.portals.iter().filter(|p| matches!(p.kind, PortalKind::Exit(_))).count(), a.exits.len());
            for p in &lv.portals {
                assert!(lv.d.path(s, (p.x as i32, p.y as i32), 200_000).is_some(), "{}: can't reach {:?}", a.name, p.kind);
            }
            assert!(lv.mobs.len() >= a.packs * 2, "{}: monsters", a.name);
        }
        let town = hollowmere(7);
        let exit = town.portals.iter().find(|p| p.kind == PortalKind::Exit(1)).expect("the north road");
        let (cx, cy) = crate::world::town_center();
        assert!(town.d.path((cx as i32, cy as i32), (exit.x as i32, exit.y as i32), 200_000).is_some());
        assert!(town.mobs.is_empty(), "no monsters around the village");
    }
}

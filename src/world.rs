//! The world: the overworld with the town of Hollowmere, and the four dungeons.
//! Each map is a `Level`; levels you leave are parked and come back as you left them.
use crate::dungeon::{Dungeon, Room, Tile};
use crate::game::{Decal, Drop, Pickup};
use crate::mobs::{Kind, Mob};
use crate::rng::Rng;
use crate::story::{Npc, Role};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum LevelId {
    Overworld,
    /// (dungeon index into DUNGEONS, floor from 0)
    Dungeon(usize, usize),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Theme {
    Overworld,
    Crypt,
    Warrens,
    Catacombs,
    Sanctum,
}

impl Theme {
    pub const ALL: [Theme; 5] = [Theme::Overworld, Theme::Crypt, Theme::Warrens, Theme::Catacombs, Theme::Sanctum];

    pub fn index(self) -> usize {
        self as usize
    }

    /// (light radius in pixels, ambient light 0..1)
    pub fn light(self) -> (f32, f32) {
        match self {
            Theme::Overworld => (360.0, 0.5),
            Theme::Sanctum => (230.0, 0.08),
            _ => (250.0, 0.10),
        }
    }
}

pub struct DungeonDef {
    pub name: &'static str,
    pub floors: usize,
    pub theme: Theme,
    pub boss: Kind,
    pub monsters: &'static [Kind],
    pub tier: f32,
    /// Overworld tile in front of the entrance.
    pub entrance: (i32, i32),
}

pub const DUNGEONS: [DungeonDef; 4] = [
    DungeonDef {
        name: "THE BONE CRYPT",
        floors: 2,
        theme: Theme::Crypt,
        boss: Kind::BoneWarden,
        monsters: &[Kind::Skeleton, Kind::Zombie, Kind::Archer],
        tier: 1.0,
        entrance: (20, 92),
    },
    DungeonDef {
        name: "THE ROTTING WARRENS",
        floors: 2,
        theme: Theme::Warrens,
        boss: Kind::PlagueWarden,
        monsters: &[Kind::Zombie, Kind::Imp, Kind::Wolf],
        tier: 1.6,
        entrance: (94, 92),
    },
    DungeonDef {
        name: "THE HEXED CATACOMBS",
        floors: 3,
        theme: Theme::Catacombs,
        boss: Kind::HexWarden,
        monsters: &[Kind::Archer, Kind::Skeleton, Kind::Imp],
        tier: 2.3,
        entrance: (92, 22),
    },
    DungeonDef {
        name: "THE ASHEN SANCTUM",
        floors: 3,
        theme: Theme::Sanctum,
        boss: Kind::AshKing,
        monsters: &[Kind::Imp, Kind::Skeleton, Kind::Archer, Kind::Zombie],
        tier: 3.2,
        entrance: (22, 20),
    },
];

/// The Ashen Sanctum (needs all three seals).
pub const SANCTUM: usize = 3;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PortalKind {
    /// Overworld door into a dungeon's first floor.
    Entrance(usize),
    /// Stairs up (to the floor above, or out to the overworld from the first floor).
    Up,
    Down,
    /// Opens where a boss dies: straight back to Hollowmere.
    TownPortal,
}

pub struct Portal {
    pub x: f32,
    pub y: f32,
    pub kind: PortalKind,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PropKind {
    TreeOak,
    TreePine,
    TreeDead,
    Rock,
    Bush,
    House1,
    House2,
    Stall,
    Campfire,
    Well,
    Entrance(usize),
    StairsDown,
    StairsUp,
}

impl PropKind {
    pub fn art(self) -> &'static str {
        match self {
            PropKind::TreeOak => "tree_oak",
            PropKind::TreePine => "tree_pine",
            PropKind::TreeDead => "tree_dead",
            PropKind::Rock => "rock1",
            PropKind::Bush => "bush1",
            PropKind::House1 => "house1",
            PropKind::House2 => "house2",
            PropKind::Stall => "tent1",
            PropKind::Campfire => "campfire",
            PropKind::Well => "well",
            PropKind::Entrance(0) => "ent_crypt",
            PropKind::Entrance(1) => "ent_warrens",
            PropKind::Entrance(2) => "ent_catacombs",
            PropKind::Entrance(_) => "ent_sanctum",
            PropKind::StairsDown => "stairs_down",
            PropKind::StairsUp => "stairs_up",
        }
    }

    /// Flat props are drawn with the floor (you walk over them), not depth-sorted.
    pub fn flat(self) -> bool {
        matches!(self, PropKind::StairsDown)
    }
}

pub struct Prop {
    /// World point the sprite's bottom centre sits on.
    pub x: f32,
    pub y: f32,
    /// Depth for sorting against actors (x + y of the footprint centre).
    pub depth: f32,
    pub kind: PropKind,
    /// Blocked tiles: x0, y0, width, height.
    pub foot: (i32, i32, i32, i32),
}

impl Prop {
    /// A prop standing on a rectangle of blocked tiles.
    pub fn on(kind: PropKind, x0: i32, y0: i32, fw: i32, fh: i32) -> Self {
        Prop {
            x: (x0 + fw) as f32 - 0.5,
            y: (y0 + fh) as f32 - 0.5,
            depth: x0 as f32 + y0 as f32 + (fw + fh) as f32 * 0.5,
            kind,
            foot: (x0, y0, fw, fh),
        }
    }

    /// Stairs sprite drawn over a portal tile (walkable, not blocking).
    pub fn stairs(kind: PropKind, tx: i32, ty: i32) -> Self {
        let depth = (tx + ty) as f32 + if kind == PropKind::StairsUp { 0.9 } else { 0.0 };
        Prop { x: tx as f32 + 0.5, y: ty as f32 + 0.5, depth, kind, foot: (tx, ty, 0, 0) }
    }
}

pub struct Level {
    pub id: LevelId,
    pub name: String,
    pub theme: Theme,
    pub tier: f32,
    pub d: Dungeon,
    pub mobs: Vec<Mob>,
    pub pickups: Vec<Pickup>,
    pub decals: Vec<Decal>,
    pub explored: Vec<bool>,
    pub props: Vec<Prop>,
    pub portals: Vec<Portal>,
    pub npcs: Vec<Npc>,
    /// Safe zone (town): x0, y0, x1, y1.
    pub safe: Option<(f32, f32, f32, f32)>,
    /// Where you wake up / arrive when there's no matching portal (town square on the overworld).
    pub start: (f32, f32),
}

impl Level {
    fn new(id: LevelId, name: String, theme: Theme, tier: f32, d: Dungeon) -> Self {
        let n = (d.w * d.h) as usize;
        Level {
            id,
            name,
            theme,
            tier,
            d,
            mobs: vec![],
            pickups: vec![],
            decals: vec![],
            explored: vec![false; n],
            props: vec![],
            portals: vec![],
            npcs: vec![],
            safe: None,
            start: (0.0, 0.0),
        }
    }

    #[cfg(test)]
    pub fn portal(&self, kind: PortalKind) -> Option<&Portal> {
        self.portals.iter().find(|p| p.kind == kind)
    }
}

/// An empty level to fill in (used by the level loader).
pub fn empty_level(id: LevelId, name: String, theme: Theme, tier: f32, d: Dungeon) -> Level {
    Level::new(id, name, theme, tier, d)
}

/// The procedural version of a level.
pub fn generate(id: LevelId, seed: u64) -> Level {
    match id {
        LevelId::Overworld => overworld(seed),
        LevelId::Dungeon(k, f) => dungeon_floor(k, f, seed),
    }
}

/// A level as the game plays it: the hand-made file if there is one, else generated.
pub fn build(id: LevelId, seed: u64) -> Level {
    crate::levels::load(id, seed).unwrap_or_else(|| generate(id, seed))
}

pub const WORLD_W: i32 = 112;
pub const WORLD_H: i32 = 112;
/// Hollowmere: palisade rectangle (inclusive tile bounds).
pub const TOWN: (i32, i32, i32, i32) = (44, 50, 68, 70);

pub fn town_center() -> (f32, f32) {
    (56.5, 61.5)
}

/// Smooth value noise in 0..1 from a seeded lattice.
struct Noise {
    grid: Vec<f32>,
    n: i32,
    cell: f32,
}

impl Noise {
    fn new(rng: &mut Rng, n: i32, cell: f32) -> Self {
        Noise { grid: (0..n * n).map(|_| rng.f()).collect(), n, cell }
    }
    fn at(&self, x: f32, y: f32) -> f32 {
        let (gx, gy) = (x / self.cell, y / self.cell);
        let (x0, y0) = (gx.floor() as i32, gy.floor() as i32);
        let (fx, fy) = (gx - x0 as f32, gy - y0 as f32);
        let g = |x: i32, y: i32| self.grid[(y.rem_euclid(self.n) * self.n + x.rem_euclid(self.n)) as usize];
        let s = |t: f32| t * t * (3.0 - 2.0 * t);
        let a = g(x0, y0) + (g(x0 + 1, y0) - g(x0, y0)) * s(fx);
        let b = g(x0, y0 + 1) + (g(x0 + 1, y0 + 1) - g(x0, y0 + 1)) * s(fx);
        a + (b - a) * s(fy)
    }
}

/// Builds the overworld: Hollowmere in the middle, roads out to the four dungeons,
/// forests, rocks, roaming packs and a little wild food.
pub fn overworld(seed: u64) -> Level {
    let mut rng = Rng::new(seed ^ 0x0F0F_1234);
    let (w, h) = (WORLD_W, WORLD_H);
    let mut d = Dungeon::blank(w, h, Tile::Floor);
    for v in d.var.iter_mut() {
        *v = rng.range(0, 100) as u8;
    }
    let mut lv = Level::new(LevelId::Overworld, "THE ASHLANDS".into(), Theme::Overworld, 1.0, Dungeon::blank(1, 1, Tile::Void));
    // Tiles that must stay clear of trees and rocks.
    let mut keep = vec![false; (w * h) as usize];
    let clear = |keep: &mut Vec<bool>, x: i32, y: i32, r: i32| {
        for yy in y - r..=y + r {
            for xx in x - r..=x + r {
                if xx >= 0 && yy >= 0 && xx < w && yy < h {
                    keep[(yy * w + xx) as usize] = true;
                }
            }
        }
    };

    // ---- Hollowmere ----
    let (tx0, ty0, tx1, ty1) = TOWN;
    for y in ty0..=ty1 {
        for x in tx0..=tx1 {
            d.set_ground(x, y, 1);
            let edge = x == tx0 || x == tx1 || y == ty0 || y == ty1;
            let gate = (y == ty0 || y == ty1) && (54..=57).contains(&x) || (x == tx0 || x == tx1) && (59..=62).contains(&y);
            if edge && !gate {
                d.set(x, y, Tile::Wall);
            }
        }
    }
    clear(&mut keep, 56, 60, 16);
    // Roads through town (plus).
    for x in tx0..=tx1 {
        for y in 60..=61 {
            d.set_ground(x, y, 2);
        }
    }
    for y in ty0..=ty1 {
        for x in 55..=56 {
            d.set_ground(x, y, 2);
        }
    }
    let prop = |lv: &mut Level, d: &mut Dungeon, kind: PropKind, x0: i32, y0: i32, fw: i32, fh: i32| {
        for y in y0..y0 + fh {
            for x in x0..x0 + fw {
                d.set(x, y, Tile::Prop);
            }
        }
        lv.props.push(Prop::on(kind, x0, y0, fw, fh));
    };
    prop(&mut lv, &mut d, PropKind::House1, 46, 52, 4, 4);
    prop(&mut lv, &mut d, PropKind::House2, 62, 52, 4, 4);
    prop(&mut lv, &mut d, PropKind::House2, 46, 65, 4, 3);
    prop(&mut lv, &mut d, PropKind::House1, 62, 64, 4, 4);
    prop(&mut lv, &mut d, PropKind::Stall, 59, 56, 3, 2);
    prop(&mut lv, &mut d, PropKind::Well, 52, 56, 1, 1);
    prop(&mut lv, &mut d, PropKind::Campfire, 58, 63, 1, 1);
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
    ];

    // ---- roads to each dungeon ----
    let gates = [(55, ty1 + 1), (tx1 + 1, 60), (55, ty0 - 1), (tx0 - 1, 60)];
    for (k, def) in DUNGEONS.iter().enumerate() {
        let (ex, ey) = def.entrance;
        // Leave by the nearest gate, then wander toward the entrance.
        let &(gx, gy) = gates.iter().min_by_key(|(gx, gy)| (gx - ex).pow(2) + (gy - ey).pow(2)).unwrap();
        let (mut x, mut y) = (gx as f32, gy as f32);
        let mut guard = 0;
        while ((x - ex as f32).abs() > 0.8 || (y - ey as f32).abs() > 0.8) && guard < 400 {
            guard += 1;
            let (dx, dy) = (ex as f32 - x, ey as f32 - y);
            let l = (dx * dx + dy * dy).sqrt();
            let wob = (guard as f32 * 0.21 + k as f32).sin() * 0.6;
            x += dx / l + (-dy / l) * wob * 0.5;
            y += dy / l + (dx / l) * wob * 0.5;
            for (ox, oy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                let (rx, ry) = (x as i32 + ox, y as i32 + oy);
                if d.get(rx, ry) == Tile::Floor {
                    d.set_ground(rx, ry, 2);
                }
            }
            clear(&mut keep, x as i32, y as i32, 2);
        }
        clear(&mut keep, ex, ey, 5);
        // Entrance building behind the door tile.
        prop(&mut lv, &mut d, PropKind::Entrance(k), ex - 1, ey - 3, 3, 3);
        lv.portals.push(Portal { x: ex as f32 + 0.5, y: ey as f32 + 0.5, kind: PortalKind::Entrance(k) });
    }

    // ---- forests, rocks and bushes ----
    let forest = Noise::new(&mut rng, 16, 9.0);
    for y in 0..h {
        for x in 0..w {
            if d.get(x, y) != Tile::Floor || keep[(y * w + x) as usize] {
                continue;
            }
            let border = x < 4 || y < 4 || x >= w - 4 || y >= h - 4;
            let f = forest.at(x as f32, y as f32);
            let r = rng.f();
            let tree = if border { r < 0.85 } else { f > 0.58 && r < 0.5 || r < 0.015 };
            if tree {
                let kind = if f > 0.75 { PropKind::TreePine } else if rng.chance(0.2) { PropKind::TreeDead } else { PropKind::TreeOak };
                prop(&mut lv, &mut d, kind, x, y, 1, 1);
            } else if r < 0.03 {
                prop(&mut lv, &mut d, PropKind::Rock, x, y, 1, 1);
            } else if r < 0.045 {
                prop(&mut lv, &mut d, PropKind::Bush, x, y, 1, 1);
            }
            if f < 0.3 && rng.chance(0.4) {
                d.set_ground(x, y, 1);
            }
        }
    }

    // ---- roaming packs and wild food ----
    let (cx, cy) = town_center();
    let mut packs = 0;
    for _ in 0..600 {
        if packs >= 30 {
            break;
        }
        let x = rng.range(6, w - 6) as f32 + 0.5;
        let y = rng.range(6, h - 6) as f32 + 0.5;
        let far = ((x - cx).powi(2) + (y - cy).powi(2)).sqrt();
        if far < 20.0 || d.blocked(x, y, 0.4) || DUNGEONS.iter().any(|def| (def.entrance.0 as f32 - x).abs() + (def.entrance.1 as f32 - y).abs() < 6.0) {
            continue;
        }
        let tier = if far < 34.0 { 0.8 } else { 1.1 };
        let kinds: &[Kind] = if far < 34.0 { &[Kind::Wolf, Kind::Imp] } else { &[Kind::Wolf, Kind::Imp, Kind::Zombie, Kind::Skeleton] };
        let kind = kinds[rng.range(0, kinds.len() as i32) as usize];
        let n = rng.range(3, 6);
        for _ in 0..n {
            for _try in 0..10 {
                let (mx, my) = (x + rng.rf(-2.0, 2.0), y + rng.rf(-2.0, 2.0));
                if !d.blocked(mx, my, 0.35) {
                    lv.mobs.push(Mob::new(kind, mx, my, tier, &mut rng));
                    break;
                }
            }
        }
        packs += 1;
    }
    let mut food = 0;
    for _ in 0..400 {
        if food >= 12 {
            break;
        }
        let x = rng.range(6, w - 6) as f32 + 0.5;
        let y = rng.range(6, h - 6) as f32 + 0.5;
        if ((x - cx).powi(2) + (y - cy).powi(2)).sqrt() > 16.0 && !d.blocked(x, y, 0.3) {
            lv.pickups.push(Pickup { x, y, kind: Drop::Food(if rng.chance(0.75) { 0 } else { 1 }), t: 1.0 });
            food += 1;
        }
    }
    lv.explored = vec![false; (w * h) as usize];
    lv.d = d;
    lv.start = town_center();
    lv
}

/// One floor of a dungeon: stairs up in the first room, stairs down (or the boss)
/// in the last, monster packs in between.
pub fn dungeon_floor(k: usize, floor: usize, seed: u64) -> Level {
    let def = &DUNGEONS[k];
    let mut rng = Rng::new(seed ^ ((k as u64 + 1) * 7919 + floor as u64 * 104_729));
    let d = Dungeon::generate(&mut rng, 72, 72);
    let tier = def.tier + floor as f32 * 0.25;
    let last = floor + 1 == def.floors;
    let name = format!("{} - LEVEL {}", def.name, floor + 1);
    let mut lv = Level::new(LevelId::Dungeon(k, floor), name, def.theme, tier, d);
    let rooms: Vec<Room> = lv.d.rooms.clone();
    let (sx, sy) = rooms[0].center();
    lv.start = (sx as f32 + 1.5, sy as f32 + 0.5);
    lv.portals.push(Portal { x: sx as f32 + 0.5, y: sy as f32 + 0.5, kind: PortalKind::Up });
    lv.props.push(Prop::stairs(PropKind::StairsUp, sx, sy));
    let end = *rooms.last().unwrap();
    let (ex, ey) = end.center();
    if last {
        let mut boss = Mob::new(def.boss, ex as f32 + 0.5, ey as f32 + 0.5, tier * 0.5 + 0.5, &mut rng);
        boss.home = (-1000.0, -2000.0); // bosses don't wander home
        lv.mobs.push(boss);
    } else {
        lv.portals.push(Portal { x: ex as f32 + 0.5, y: ey as f32 + 0.5, kind: PortalKind::Down });
        lv.props.push(Prop::stairs(PropKind::StairsDown, ex, ey));
    }
    let rooms_with_mobs = rooms.len() - if last { 1 } else { 0 };
    for r in rooms.iter().take(rooms_with_mobs).skip(1) {
        let n = rng.range(2, 5) + floor as i32;
        let main = def.monsters[rng.range(0, def.monsters.len() as i32) as usize];
        for _ in 0..n {
            for _try in 0..20 {
                let x = rng.range(r.x + 1, r.x + r.w - 1) as f32 + 0.5;
                let y = rng.range(r.y + 1, r.y + r.h - 1) as f32 + 0.5;
                if lv.d.blocked(x, y, 0.35) || lv.mobs.iter().any(|m| (m.x - x).abs() + (m.y - y).abs() < 1.0) {
                    continue;
                }
                let kind = if rng.chance(0.25) { def.monsters[rng.range(0, def.monsters.len() as i32) as usize] } else { main };
                lv.mobs.push(Mob::new(kind, x, y, tier, &mut rng));
                break;
            }
        }
    }
    let n_food = 3 + rng.range(0, 3);
    for _ in 0..n_food {
        let r = rooms[rng.range(1, rooms.len() as i32) as usize];
        for _try in 0..20 {
            let x = rng.range(r.x + 1, r.x + r.w - 1) as f32 + 0.5;
            let y = rng.range(r.y + 1, r.y + r.h - 1) as f32 + 0.5;
            if lv.d.blocked(x, y, 0.3) {
                continue;
            }
            let roll = rng.f();
            let kind = if roll < 0.5 { 0 } else if roll < 0.85 { 1 } else { 2 };
            lv.pickups.push(Pickup { x, y, kind: Drop::Food(kind), t: 1.0 });
            break;
        }
    }
    lv
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_entrance_is_reachable_from_town() {
        let lv = overworld(7);
        let (cx, cy) = town_center();
        assert!(lv.d.walkable(cx as i32, cy as i32));
        for p in &lv.portals {
            assert!(lv.d.walkable(p.x as i32, p.y as i32), "portal {:?} blocked", p.kind);
            assert!(lv.d.path((cx as i32, cy as i32), (p.x as i32, p.y as i32), 100_000).is_some(), "no road to {:?}", p.kind);
        }
        for n in &lv.npcs {
            assert!(lv.d.walkable(n.x as i32, n.y as i32), "{} stands in a wall", n.name);
        }
        assert!(lv.mobs.len() > 60, "overworld has {} monsters", lv.mobs.len());
        let (x0, y0, x1, y1) = lv.safe.unwrap();
        assert!(lv.mobs.iter().all(|m| !(m.x > x0 && m.x < x1 && m.y > y0 && m.y < y1)), "monsters spawned in town");
    }

    #[test]
    fn dungeon_floors_have_stairs_and_a_boss_at_the_bottom() {
        for (k, def) in DUNGEONS.iter().enumerate() {
            for f in 0..def.floors {
                let lv = dungeon_floor(k, f, 99);
                let up = lv.portal(PortalKind::Up).expect("stairs up");
                assert!(lv.d.walkable(up.x as i32, up.y as i32));
                let last = f + 1 == def.floors;
                assert_eq!(lv.portal(PortalKind::Down).is_some(), !last);
                assert_eq!(lv.mobs.iter().filter(|m| m.boss).count(), if last { 1 } else { 0 });
                if last {
                    let b = lv.mobs.iter().find(|m| m.boss).unwrap();
                    assert_eq!(b.kind, def.boss);
                    assert!(lv.d.path((up.x as i32, up.y as i32), (b.x as i32, b.y as i32), 20_000).is_some());
                }
            }
        }
    }
}

//! Procedural dungeon: rooms joined by two-wide corridors, walls wrapped around
//! every floor tile, plus A* pathfinding and line-of-sight on the grid.
use crate::rng::Rng;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tile {
    Void,
    Floor,
    Wall,
    /// Blocked by a prop (tree, house, rock); drawn by the prop, not as a wall.
    Prop,
}

#[derive(Clone, Copy)]
pub struct Room {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Room {
    pub fn center(&self) -> (i32, i32) {
        (self.x + self.w / 2, self.y + self.h / 2)
    }
    fn overlaps(&self, o: &Room, pad: i32) -> bool {
        self.x - pad < o.x + o.w && o.x - pad < self.x + self.w && self.y - pad < o.y + o.h && o.y - pad < self.y + self.h
    }
}

pub struct Dungeon {
    pub w: i32,
    pub h: i32,
    pub tiles: Vec<Tile>,
    /// Floor variant per tile (for picking floor art).
    pub var: Vec<u8>,
    /// Ground type per tile on the overworld: 0 grass, 1 dirt, 2 road (always 0 in dungeons).
    pub ground: Vec<u8>,
    pub rooms: Vec<Room>,
}

impl Dungeon {
    pub fn generate(rng: &mut Rng, w: i32, h: i32) -> Self {
        let mut d = Dungeon::blank(w, h, Tile::Void);
        for _ in 0..200 {
            if d.rooms.len() >= 14 {
                break;
            }
            let rw = rng.range(6, 13);
            let rh = rng.range(6, 13);
            let r = Room { x: rng.range(2, w - rw - 2), y: rng.range(2, h - rh - 2), w: rw, h: rh };
            if d.rooms.iter().any(|o| r.overlaps(o, 3)) {
                continue;
            }
            d.rooms.push(r);
        }
        // Order rooms by a nearest-neighbour walk so corridors stay short.
        let mut order = vec![d.rooms.remove(0)];
        while !d.rooms.is_empty() {
            let (lx, ly) = order.last().unwrap().center();
            let (i, _) = d
                .rooms
                .iter()
                .enumerate()
                .map(|(i, r)| {
                    let (cx, cy) = r.center();
                    (i, (cx - lx).pow(2) + (cy - ly).pow(2))
                })
                .min_by_key(|v| v.1)
                .unwrap();
            order.push(d.rooms.remove(i));
        }
        d.rooms = order;
        for r in d.rooms.clone() {
            for y in r.y..r.y + r.h {
                for x in r.x..r.x + r.w {
                    d.set(x, y, Tile::Floor);
                }
            }
            // Pillars in bigger rooms.
            if r.w >= 10 && r.h >= 10 && rng.chance(0.6) {
                for (px, py) in [(r.x + 2, r.y + 2), (r.x + r.w - 3, r.y + 2), (r.x + 2, r.y + r.h - 3), (r.x + r.w - 3, r.y + r.h - 3)] {
                    d.set(px, py, Tile::Wall);
                }
            }
        }
        for i in 1..d.rooms.len() {
            let (ax, ay) = d.rooms[i - 1].center();
            let (bx, by) = d.rooms[i].center();
            d.corridor(ax, ay, bx, by, rng.chance(0.5));
        }
        // A couple of loops so the layout isn't a single chain.
        for _ in 0..2 {
            let n = d.rooms.len() as i32;
            let (a, b) = (rng.range(0, n), rng.range(0, n));
            if (a - b).abs() > 2 {
                let (ax, ay) = d.rooms[a as usize].center();
                let (bx, by) = d.rooms[b as usize].center();
                d.corridor(ax, ay, bx, by, rng.chance(0.5));
            }
        }
        // Wrap floors with walls.
        for y in 0..h {
            for x in 0..w {
                if d.get(x, y) != Tile::Void {
                    continue;
                }
                let near = (-1..=1).any(|dy| (-1..=1).any(|dx| d.get(x + dx, y + dy) == Tile::Floor));
                if near {
                    d.set(x, y, Tile::Wall);
                }
            }
        }
        for v in d.var.iter_mut() {
            *v = rng.range(0, 100) as u8;
        }
        d
    }

    /// An empty map filled with one tile type (the overworld starts all floor).
    pub fn blank(w: i32, h: i32, fill: Tile) -> Self {
        let n = (w * h) as usize;
        Dungeon { w, h, tiles: vec![fill; n], var: vec![0; n], ground: vec![0; n], rooms: vec![] }
    }

    pub fn set_ground(&mut self, x: i32, y: i32, g: u8) {
        if x >= 0 && y >= 0 && x < self.w && y < self.h {
            self.ground[(y * self.w + x) as usize] = g;
        }
    }

    pub fn ground_at(&self, x: i32, y: i32) -> u8 {
        if x >= 0 && y >= 0 && x < self.w && y < self.h {
            self.ground[(y * self.w + x) as usize]
        } else {
            0
        }
    }

    fn corridor(&mut self, ax: i32, ay: i32, bx: i32, by: i32, x_first: bool) {
        let carve = |d: &mut Dungeon, x: i32, y: i32| {
            for (ox, oy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                if d.get(x + ox, y + oy) != Tile::Floor {
                    d.set(x + ox, y + oy, Tile::Floor);
                }
            }
        };
        let (mx, my) = if x_first { (bx, ay) } else { (ax, by) };
        for (sx, sy, ex, ey) in [(ax, ay, mx, my), (mx, my, bx, by)] {
            let (mut x, mut y) = (sx, sy);
            loop {
                carve(self, x, y);
                if x == ex && y == ey {
                    break;
                }
                x += (ex - x).signum();
                y += (ey - y).signum();
            }
        }
    }

    pub fn get(&self, x: i32, y: i32) -> Tile {
        if x < 0 || y < 0 || x >= self.w || y >= self.h {
            Tile::Void
        } else {
            self.tiles[(y * self.w + x) as usize]
        }
    }

    pub fn set(&mut self, x: i32, y: i32, t: Tile) {
        if x >= 0 && y >= 0 && x < self.w && y < self.h {
            self.tiles[(y * self.w + x) as usize] = t;
        }
    }

    pub fn walkable(&self, x: i32, y: i32) -> bool {
        self.get(x, y) == Tile::Floor
    }

    /// Does a circle of radius `r` at (x, y) overlap any blocking tile?
    pub fn blocked(&self, x: f32, y: f32, r: f32) -> bool {
        for ty in (y - r).floor() as i32..=(y + r).floor() as i32 {
            for tx in (x - r).floor() as i32..=(x + r).floor() as i32 {
                if self.walkable(tx, ty) {
                    continue;
                }
                let cx = x.clamp(tx as f32, tx as f32 + 1.0);
                let cy = y.clamp(ty as f32, ty as f32 + 1.0);
                if (cx - x).powi(2) + (cy - y).powi(2) < r * r {
                    return true;
                }
            }
        }
        false
    }

    /// Grid line of sight between two world points.
    pub fn los(&self, x0: f32, y0: f32, x1: f32, y1: f32) -> bool {
        let d = ((x1 - x0).powi(2) + (y1 - y0).powi(2)).sqrt();
        let n = (d * 3.0).ceil() as i32;
        (0..=n).all(|i| {
            let t = i as f32 / n.max(1) as f32;
            self.walkable((x0 + (x1 - x0) * t).floor() as i32, (y0 + (y1 - y0) * t).floor() as i32)
        })
    }

    /// 8-connected A* (no corner cutting). Returns tile centres from start (exclusive) to goal.
    pub fn path(&self, from: (i32, i32), to: (i32, i32), max_nodes: usize) -> Option<Vec<(f32, f32)>> {
        if !self.walkable(to.0, to.1) || !self.walkable(from.0, from.1) {
            return None;
        }
        let idx = |x: i32, y: i32| (y * self.w + x) as usize;
        let n = (self.w * self.h) as usize;
        let mut g = vec![f32::INFINITY; n];
        let mut came = vec![u32::MAX; n];
        let mut closed = vec![false; n];
        let mut open = std::collections::BinaryHeap::new();
        let hfn = |x: i32, y: i32| {
            let (dx, dy) = ((x - to.0).abs() as f32, (y - to.1).abs() as f32);
            dx.max(dy) + 0.414 * dx.min(dy)
        };
        g[idx(from.0, from.1)] = 0.0;
        open.push(Node { f: hfn(from.0, from.1), i: idx(from.0, from.1) as u32 });
        let mut expanded = 0;
        while let Some(Node { i, .. }) = open.pop() {
            let i = i as usize;
            if closed[i] {
                continue;
            }
            closed[i] = true;
            let (x, y) = ((i as i32) % self.w, (i as i32) / self.w);
            if (x, y) == to {
                let mut out = vec![];
                let mut c = i;
                while c != idx(from.0, from.1) {
                    out.push(((c as i32 % self.w) as f32 + 0.5, (c as i32 / self.w) as f32 + 0.5));
                    c = came[c] as usize;
                }
                out.reverse();
                return Some(out);
            }
            expanded += 1;
            if expanded > max_nodes {
                return None;
            }
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)] {
                let (nx, ny) = (x + dx, y + dy);
                if !self.walkable(nx, ny) {
                    continue;
                }
                if dx != 0 && dy != 0 && (!self.walkable(x + dx, y) || !self.walkable(x, y + dy)) {
                    continue;
                }
                let ni = idx(nx, ny);
                let ng = g[i] + if dx != 0 && dy != 0 { 1.414 } else { 1.0 };
                if ng < g[ni] {
                    g[ni] = ng;
                    came[ni] = i as u32;
                    open.push(Node { f: ng + hfn(nx, ny), i: ni as u32 });
                }
            }
        }
        None
    }
}

struct Node {
    f: f32,
    i: u32,
}
impl PartialEq for Node {
    fn eq(&self, o: &Self) -> bool {
        self.f == o.f
    }
}
impl Eq for Node {}
impl PartialOrd for Node {
    fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Node {
    fn cmp(&self, o: &Self) -> std::cmp::Ordering {
        o.f.partial_cmp(&self.f).unwrap_or(std::cmp::Ordering::Equal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_floor_tile_is_reachable() {
        for seed in 1..40 {
            let d = Dungeon::generate(&mut Rng::new(seed), 72, 72);
            assert!(d.rooms.len() >= 6, "seed {seed}: only {} rooms", d.rooms.len());
            let (sx, sy) = d.rooms[0].center();
            let mut seen = vec![false; (d.w * d.h) as usize];
            let mut stack = vec![(sx, sy)];
            while let Some((x, y)) = stack.pop() {
                if !d.walkable(x, y) || seen[(y * d.w + x) as usize] {
                    continue;
                }
                seen[(y * d.w + x) as usize] = true;
                stack.extend([(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)]);
            }
            for y in 0..d.h {
                for x in 0..d.w {
                    if d.walkable(x, y) {
                        assert!(seen[(y * d.w + x) as usize], "seed {seed}: floor {x},{y} unreachable");
                    }
                }
            }
            // Floors are always wrapped by walls, never by the void.
            for y in 0..d.h {
                for x in 0..d.w {
                    if d.walkable(x, y) {
                        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                            assert_ne!(d.get(x + dx, y + dy), Tile::Void, "seed {seed}: floor {x},{y} touches void");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn path_reaches_every_room() {
        let d = Dungeon::generate(&mut Rng::new(3), 72, 72);
        let start = d.rooms[0].center();
        for r in &d.rooms[1..] {
            assert!(d.path(start, r.center(), 20000).is_some());
        }
    }
}

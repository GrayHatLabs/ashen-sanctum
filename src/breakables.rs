//! Breakables (docs/BREAKABLES_PLAN.md): crates, barrels and urns to smash, themed by act. They are
//! monsters with no mind (`Kind::Crate` / `Barrel` / `Urn`, the act in `Mob::form`), so every skill already
//! hits them; the AI, XP, kill hooks, allies, corpses and the test bot all skip them. Breaking one leaves
//! debris and rolls a D2-style loot table.
use crate::game::{Decal, Drop, Game, PKind, Particle, Pickup, Sfx};
use crate::gfx::{rgb, Screen};
use crate::mobs::{Kind, Mob, MobState};
use crate::rng::Rng;
use crate::world::{Level, LevelId, PortalKind};

pub fn is_prop(k: Kind) -> bool {
    matches!(k, Kind::Crate | Kind::Barrel | Kind::Urn)
}

/// The name of each breakable, by act (0-3).
pub fn label(k: Kind, act: u8) -> &'static str {
    match (k, act) {
        (Kind::Crate, 2) => "COFFIN",
        (Kind::Crate, 3) => "BRASS CRATE",
        (Kind::Crate, 4) => "BARNACLED CRATE",
        (Kind::Crate, 5) => "GILDED CHEST",
        (Kind::Crate, _) => "CRATE",
        (Kind::Barrel, 2) => "ROTTEN BARREL",
        (Kind::Barrel, 3) => "OIL DRUM",
        (Kind::Barrel, 4) => "SEALED AMPHORA",
        (Kind::Barrel, 5) => "SUNLIT BARREL",
        (Kind::Barrel, _) => "BARREL",
        (Kind::Urn, 2) => "BONE URN",
        (Kind::Urn, 3) => "CLOCKWORK BOX",
        (Kind::Urn, 4) => "GIANT CLAM",
        (Kind::Urn, 5) => "MARBLE URN",
        _ => "URN",
    }
}

/// Art name for the generated sprite (code-drawn until it exists): brk_<kind>_<act>, plus _broken.
pub fn art_name(k: Kind, act: u8) -> String {
    let n = match k {
        Kind::Crate => "crate",
        Kind::Barrel => "barrel",
        _ => "urn",
    };
    format!("brk_{n}_{act}")
}

// ------------------------------------------------------------------ loot

/// What a break gives (D2 style: often nothing, sometimes supplies, rarely something good).
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Loot {
    Nothing,
    Gold,
    Potion,
    Food,
    Gem,
    Item,
}

pub fn roll_loot(rng: &mut Rng) -> Loot {
    let r = rng.f();
    if r < 0.55 {
        Loot::Nothing
    } else if r < 0.75 {
        Loot::Gold
    } else if r < 0.85 {
        Loot::Potion
    } else if r < 0.93 {
        Loot::Food
    } else if r < 0.97 {
        Loot::Gem
    } else {
        Loot::Item
    }
}

// ------------------------------------------------------------------ placement

fn new_prop(kind: Kind, x: f32, y: f32, act: u8, rng: &mut Rng) -> Mob {
    let mut m = Mob::new(kind, x, y, 1.0, rng);
    m.form = act;
    m.max_hp = 4.0;
    m.hp = 4.0;
    m.xp = 0.0;
    m.r = 0.34;
    m.home = (-1000.0, -2000.0);
    m
}

fn pick_kind(rng: &mut Rng) -> Kind {
    let r = rng.f();
    if r < 0.4 {
        Kind::Crate
    } else if r < 0.75 {
        Kind::Barrel
    } else {
        Kind::Urn
    }
}

/// Puts breakables on a level: clusters along the walls of about a third of a dungeon's rooms, and a few by each
/// dungeon entrance outdoors. Never in town, on portals or right at the start. Same places for the same seed.
pub fn place(lv: &mut Level, seed: u64) {
    let mut rng = Rng::new(seed ^ 0xb4ea_cab1e ^ format!("{:?}", lv.id).bytes().fold(7u64, |a, b| a.wrapping_mul(31).wrapping_add(b as u64)));
    let act = lv.id.act().min(5) as u8;
    let mut spots: Vec<(f32, f32)> = vec![];
    let free = |lv: &Level, spots: &[(f32, f32)], x: f32, y: f32| {
        !lv.d.blocked(x, y, 0.4)
            && (lv.start.0 - x).powi(2) + (lv.start.1 - y).powi(2) > 9.0
            && lv.portals.iter().all(|p| (p.x - x).powi(2) + (p.y - y).powi(2) > 4.0)
            && lv.safe.map_or(true, |(x0, y0, x1, y1)| x < x0 - 3.0 || x > x1 + 3.0 || y < y0 - 3.0 || y > y1 + 3.0)
            && spots.iter().all(|&(sx, sy)| (sx - x).powi(2) + (sy - y).powi(2) > 0.8)
            && lv.mobs.iter().all(|m| (m.x - x).powi(2) + (m.y - y).powi(2) > 1.0)
    };
    match lv.id {
        LevelId::Dungeon(..) => {
            let rooms = lv.d.rooms.clone();
            for r in rooms.iter().skip(1) {
                if !rng.chance(0.35) {
                    continue;
                }
                // A spot along one wall of the room, then a few more around it.
                let (cx, cy) = match rng.range(0, 4) {
                    0 => (rng.range(r.x + 1, r.x + r.w - 1) as f32 + 0.5, r.y as f32 + 1.5),
                    1 => (rng.range(r.x + 1, r.x + r.w - 1) as f32 + 0.5, (r.y + r.h) as f32 - 1.5),
                    2 => (r.x as f32 + 1.5, rng.range(r.y + 1, r.y + r.h - 1) as f32 + 0.5),
                    _ => ((r.x + r.w) as f32 - 1.5, rng.range(r.y + 1, r.y + r.h - 1) as f32 + 0.5),
                };
                let n = rng.range(2, 6);
                let mut placed = 0;
                for _ in 0..n * 6 {
                    if placed >= n {
                        break;
                    }
                    let (x, y) = (cx + rng.rf(-1.4, 1.4), cy + rng.rf(-1.4, 1.4));
                    if free(lv, &spots, x, y) {
                        spots.push((x, y));
                        placed += 1;
                    }
                }
            }
        }
        _ if lv.id.overland() => {
            let entrances: Vec<(f32, f32)> = lv.portals.iter().filter(|p| matches!(p.kind, PortalKind::Entrance(_))).map(|p| (p.x, p.y)).collect();
            for (ex, ey) in entrances {
                let n = rng.range(1, 4);
                let mut placed = 0;
                for _ in 0..40 {
                    if placed >= n {
                        break;
                    }
                    let a = rng.rf(0.0, std::f32::consts::TAU);
                    let d = rng.rf(2.5, 5.0);
                    let (x, y) = (ex + a.cos() * d, ey + a.sin() * d);
                    if free(lv, &spots, x, y) {
                        spots.push((x, y));
                        placed += 1;
                    }
                }
            }
        }
        _ => {}
    }
    for (x, y) in spots {
        let k = pick_kind(&mut rng);
        lv.mobs.push(new_prop(k, x, y, act, &mut rng));
    }
}

// ------------------------------------------------------------------ breaking

impl Game {
    /// A breakable smashed: debris, splinters themed by act, a sound, and the loot roll. No XP, no kill hooks.
    pub(crate) fn break_prop(&mut self, i: usize) {
        let m = &mut self.mobs[i];
        let (x, y, act) = (m.x, m.y, m.form);
        m.state = MobState::Dead(0.0);
        m.hp = 0.0;
        self.sfx.push(Sfx::Swing);
        let (pk, debris) = match act {
            1 => (PKind::Frost, rgb(0x6a7a88)),
            2 => (PKind::Bone, rgb(0x2a2018)),
            3 => (PKind::Fire, rgb(0x4a3a20)),
            4 => (PKind::Frost, rgb(0x2a4a50)),
            5 => (PKind::Holy, rgb(0xb0a888)),
            _ => (PKind::Bone, rgb(0x4a3018)),
        };
        for k in 0..16 {
            let a = k as f32 / 16.0 * std::f32::consts::TAU;
            self.parts.push(Particle { x, y, z: 10.0, vx: a.cos() * 2.4, vy: a.sin() * 2.4, vz: 40.0 + (k % 4) as f32 * 10.0, life: 0.5, max: 0.5, kind: pk });
        }
        for _ in 0..4 {
            self.spray_at(x, y, PKind::Smoke, 6.0);
        }
        self.decals.push(Decal { x, y, r: 0.45, col: debris, a: 0.7 });
        let ilvl = crate::items::ilvl_for(self.tier);
        let loot = roll_loot(&mut self.rng);
        let kind = match loot {
            Loot::Nothing => return,
            Loot::Gold => Drop::Gold(self.rng.range(3, 12) * (1 + ilvl as i32 / 4)),
            Loot::Potion => {
                if self.rng.chance(0.5) {
                    Drop::Health
                } else {
                    Drop::Mana
                }
            }
            Loot::Food => Drop::Food(self.rng.range(0, 3) as usize),
            Loot::Gem => Drop::Item(Box::new(crate::items::gem_item(crate::items::roll_gem(ilvl, &mut self.rng)))),
            Loot::Item => {
                let mf = self.p.bonus.get(crate::items::Stat::Magic);
                let mut it = crate::items::drop(ilvl, mf, false, &mut self.rng);
                if it.rarity == crate::items::Rarity::Normal {
                    it = crate::items::roll(ilvl, crate::items::Rarity::Magic, &mut self.rng);
                }
                Drop::Item(Box::new(it))
            }
        };
        self.pickups.push(Pickup { x, y, kind, t: 0.0 });
    }

    /// The code-drawn stand-in for a breakable (generated art is used instead once it exists).
    pub(crate) fn draw_prop(&self, scr: &mut Screen, i: usize, sx: i32, sy: i32) {
        let m = &self.mobs[i];
        if !m.alive() {
            return;
        }
        if let Some(s) = self.art.item(&art_name(m.kind, m.form)) {
            let fx = crate::gfx::Fx { tint: rgb(0xffffff), tint_a: if m.flash > 0.0 { 0.6 } else { 0.0 }, ..Default::default() };
            scr.blit(s, sx, sy + 2, fx);
            return;
        }
        crate::render::blend_ellipse(scr, sx, sy, 8, 3, rgb(0x000000), 0.4);
        let flash = m.flash > 0.0;
        let c = |v: u32| if flash { rgb(0xffffff) } else { rgb(v) };
        match (m.kind, m.form) {
            (Kind::Crate, 2) => {
                // An upright coffin.
                scr.fill(sx - 5, sy - 22, 10, 22, c(0x2a1c14));
                scr.fill(sx - 4, sy - 21, 8, 20, c(0x4a3020));
                scr.fill(sx - 1, sy - 18, 2, 8, c(0x8a7a60));
                scr.fill(sx - 3, sy - 15, 6, 2, c(0x8a7a60));
            }
            (Kind::Crate, act) => {
                let (dark, mid, top) = match act {
                    1 => (0x3a4450, 0x6a7a88, 0xdaeaf4),
                    3 => (0x4a3418, 0xa07830, 0xd0a850),
                    4 => (0x1a2a2a, 0x3a5a50, 0x8ab0a0),
                    5 => (0x8a7a50, 0xe8e0d0, 0xffe080),
                    _ => (0x3a2410, 0x7a5530, 0xa07848),
                };
                scr.fill(sx - 7, sy - 12, 14, 12, c(dark));
                scr.fill(sx - 6, sy - 11, 12, 10, c(mid));
                scr.fill(sx - 7, sy - 14, 14, 3, c(top));
                scr.fill(sx - 6, sy - 7, 12, 1, c(dark));
                if act == 3 {
                    for (rx, ry) in [(-5, -10), (4, -10), (-5, -3), (4, -3)] {
                        scr.pset(sx + rx, sy + ry, c(0xf0d080));
                    }
                }
            }
            (Kind::Barrel, act) => {
                let (body, band) = match act {
                    1 => (0x7a8a9a, 0xdaeaf4),
                    2 => (0x3a3a24, 0x5a4a30),
                    3 => (0x2a2a30, 0xb08840),
                    4 => (0x8a6a50, 0x3a5a50),
                    5 => (0xe8e0d0, 0xd0a840),
                    _ => (0x6a4424, 0x3a3030),
                };
                for yy in 0..14 {
                    let w = 6 - ((yy as i32 - 7).abs() / 4);
                    scr.fill(sx - w, sy - 14 + yy, w * 2, 1, c(body));
                }
                scr.fill(sx - 6, sy - 12, 12, 1, c(band));
                scr.fill(sx - 6, sy - 3, 12, 1, c(band));
                scr.fill(sx - 5, sy - 15, 10, 2, c(band));
            }
            (_, act) => {
                if act == 3 {
                    // A clockwork box: brass, a gear on the lid.
                    scr.fill(sx - 5, sy - 9, 10, 9, c(0x8a6428));
                    scr.disc(sx, sy - 11, 3, c(0xd0a850));
                    scr.pset(sx, sy - 11, c(0x2a1c10));
                    return;
                }
                let body = match act {
                    1 => 0x9ab8d0,
                    2 => 0xd8d0b0,
                    4 => 0xd8e8e0,
                    5 => 0xf0ece0,
                    _ => 0xa05a30,
                };
                scr.disc(sx, sy - 6, 5, c(body));
                scr.fill(sx - 2, sy - 14, 4, 4, c(body));
                scr.fill(sx - 3, sy - 15, 6, 1, c(0x2a1c10));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{Game, Input};
    use crate::skills::Class;

    fn game_with_crate() -> (Game, usize) {
        let mut g = Game::new(5, crate::gfx::SH_WIDE);
        g.debug_goto(LevelId::Dungeon(0, 0));
        g.mobs.clear();
        let (x, y) = (g.p.x + 2.0, g.p.y);
        let mut rng = Rng::new(3);
        g.mobs.push(new_prop(Kind::Crate, x, y, 0, &mut rng));
        (g, 0)
    }

    #[test]
    fn dungeons_have_breakables_and_towns_dont() {
        let mut g = Game::new(5, crate::gfx::SH_WIDE);
        g.debug_goto(LevelId::Dungeon(0, 0));
        let n = g.mobs.iter().filter(|m| is_prop(m.kind)).count();
        assert!(n >= 3, "a dungeon floor has breakables: {n}");
        g.debug_goto(LevelId::Overworld);
        for m in g.mobs.iter().filter(|m| is_prop(m.kind)) {
            assert!(!g.safe_contains(m.x, m.y, 2.0), "none in town");
        }
        // Same places on every visit.
        g.debug_goto(LevelId::Dungeon(1, 0));
        let a: Vec<(f32, f32)> = g.mobs.iter().filter(|m| is_prop(m.kind)).map(|m| (m.x, m.y)).collect();
        let lv = crate::world::build_at(LevelId::Dungeon(1, 0), g.world_seed(), 0);
        let b: Vec<(f32, f32)> = lv.mobs.iter().filter(|m| is_prop(m.kind)).map(|m| (m.x, m.y)).collect();
        assert_eq!(a, b);
    }

    #[test]
    fn breakables_never_act_give_no_xp_and_dont_count_as_foes() {
        let (mut g, i) = game_with_crate();
        let (x, y) = (g.mobs[i].x, g.mobs[i].y);
        let (xp, kills) = (g.p.xp, g.kills);
        for _ in 0..120 {
            g.update(&Input::default());
        }
        assert_eq!((g.mobs[i].x, g.mobs[i].y), (x, y), "it never moves");
        assert_eq!(g.alive_mobs(), 0, "not counted as a foe");
        assert!(g.bot_target().is_none());
        g.hit_mob(i, 50.0, 0.0, 0.0, None, false);
        assert!(!g.mobs[i].alive(), "one hit breaks it");
        assert_eq!(g.p.xp, xp);
        assert_eq!(g.kills, kills);
    }

    #[test]
    fn any_hit_breaks_them() {
        // A sorceress's fireball.
        let (mut g, i) = game_with_crate();
        let (x, y) = (g.mobs[i].x, g.mobs[i].y);
        g.cast_skill(crate::skills::Skill::Fireball, x, y);
        for _ in 0..60 {
            g.update(&Input::default());
        }
        assert!(!g.mobs[i].alive(), "a fireball smashes it");
        // A berserker's cleave.
        let (mut g, i) = game_with_crate();
        g.set_class(Class::Berserker);
        (g.mobs[i].x, g.mobs[i].y) = (g.p.x + 1.0, g.p.y);
        let (x, y) = (g.mobs[i].x, g.mobs[i].y);
        g.cleave(x, y);
        assert!(!g.mobs[i].alive(), "an axe smashes it");
    }

    #[test]
    fn loot_is_a_d2_style_mix() {
        let mut rng = Rng::new(11);
        let n = 20000;
        let mut c = [0usize; 6];
        for _ in 0..n {
            c[roll_loot(&mut rng) as usize] += 1;
        }
        let f = |k: usize| c[k] as f32 / n as f32;
        assert!((f(0) - 0.55).abs() < 0.02, "nothing {}", f(0));
        assert!((f(1) - 0.20).abs() < 0.02, "gold {}", f(1));
        assert!(f(5) > 0.015 && f(5) < 0.045, "items are rare {}", f(5));
        // Breaking many drops some of each kind.
        let (mut g, _) = game_with_crate();
        let mut rng = Rng::new(4);
        for k in 0..300 {
            g.mobs.push(new_prop(Kind::Barrel, g.p.x + (k % 10) as f32 * 0.1, g.p.y + 3.0, 0, &mut rng));
            let j = g.mobs.len() - 1;
            g.break_prop(j);
        }
        assert!(g.pickups.iter().any(|p| matches!(p.kind, Drop::Gold(_))));
        assert!(g.pickups.iter().any(|p| matches!(p.kind, Drop::Item(_))));
    }
}

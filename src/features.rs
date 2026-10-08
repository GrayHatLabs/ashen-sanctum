//! Set pieces in the areas (docs/SIDE_CONTENT_PLAN.md 2c), the user's picks, act by act.
//!
//! Act 1: the **Burning Barn** (the Barrow Fields: pull three villagers out before the roof falls), the **Goblin
//! Market** (Skrat's Gulch: a neutral camp trading stolen goods, until you attack it) and the **Ash Storm**
//! (random weather: the light chokes and ash elementals roam until it passes).
//!
//! Act 2: **Thin Ice** (the White Waste: a frozen lake that cracks under fighting and heavy feet), the **Raider
//! Longhall** (the Raiders' Fjord: steal Jarl Hrogar's war horn, or challenge him to a duel), the **Frozen
//! Merchant** (the Rime Woods: thaw him and he sells rare goods once) and the **Yeti Cub** (the Howling Tundra:
//! lead it home and the area's yetis leave you be). Also the frozen scouts of Brenna's Lost Patrol (side.rs).
//!
//! Shared pieces: **neutral** monsters (`Mob::neutral`, a group: they stand by until one of them is hurt),
//! NPCs that **follow** you, and **traders** with a stock of items.
use crate::game::{Decal, Drop, Game, Light, PKind, Pickup, Sfx, DT};
use crate::gfx::rgb;
use crate::items::Item;
use crate::mobs::{Kind, Mob, MobState, Rank};
use crate::story::{Act, Dialog, Npc, Role};
use crate::world::{LevelId, Prop, PropKind};

/// Neutral groups (`Mob::neutral`).
pub const G_MARKET: u8 = 1;
pub const G_RAIDERS: u8 = 2;
pub const G_YETIS: u8 = 9;

/// The ice block's `Mob::form`: the frozen merchant, or one of Brenna's scouts.
pub const ICE_MERCHANT: u8 = 0;
pub const ICE_SCOUT: u8 = 1;

/// Where each set piece is (the scouts are in three areas).
pub const BARN: LevelId = LevelId::Area(0, 2);
pub const MARKET: LevelId = LevelId::Area(0, 8);
pub const LAKE: LevelId = LevelId::Area(1, 4);
pub const HALL: LevelId = LevelId::Area(1, 6);
pub const MERCHANT: LevelId = LevelId::Area(1, 3);
pub const CUB: LevelId = LevelId::Area(1, 2);
pub const SCOUTS: [LevelId; 3] = [LevelId::Area(1, 1), LevelId::Area(1, 3), LevelId::Area(1, 4)];

/// The burning barn's fire lasts this long once you're near.
pub const BARN_TIME: f32 = 75.0;
/// Ash storms: how long, and how long between them (a range).
pub const STORM_TIME: f32 = 60.0;
pub const STORM_GAP: (f32, f32) = (180.0, 360.0);

/// The thin ice of the White Waste: which tiles are lake, how cracked each is, and the holes.
#[derive(Clone, Debug, Default)]
pub struct Ice {
    pub w: i32,
    pub lake: Vec<bool>,
    pub stress: Vec<f32>,
    pub holes: Vec<(i32, i32)>,
    pub safe: (f32, f32),
}

impl Ice {
    pub fn on(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && (y * self.w + x) < self.lake.len() as i32 && x < self.w && self.lake[(y * self.w + x) as usize]
    }

    pub fn stress_at(&self, x: i32, y: i32) -> f32 {
        if self.on(x, y) {
            self.stress[(y * self.w + x) as usize]
        } else {
            0.0
        }
    }
}

/// Everything the set pieces remember (per difficulty; reset with the quests).
#[derive(Clone, Debug, Default)]
pub struct Feats {
    pub placed: Vec<LevelId>,
    /// The barn: villagers saved, seconds burning (once you're near), over.
    pub barn_saved: u8,
    pub barn_t: f32,
    pub barn_over: bool,
    pub barn_spot: (f32, f32),
    pub market_hostile: bool,
    /// Ash storm: seconds left (0: none), and until the next.
    pub storm: f32,
    pub storm_cd: f32,
    pub ice: Ice,
    /// The Jarl: 0 at his hall, 1 dueling, 2 beaten, 3 robbed (and after you).
    pub jarl: u8,
    pub horn_spot: (f32, f32),
    pub merchant_free: bool,
    /// The cub: 0 lost, 1 following, 2 home. Its den.
    pub cub: u8,
    pub den: (f32, f32),
    /// Brenna's scouts thawed.
    pub scouts: u8,
    /// Traders' wares: (who, items; None once sold).
    pub stock: Vec<(Role, Vec<Option<Item>>)>,
    /// Whose wares the open conversation shows.
    pub trading: Option<Role>,
    // ---- Act 3 (mist.rs) ----
    pub mist_placed: Vec<LevelId>,
    /// The wolf moon: seconds left, and until the next.
    pub moon: f32,
    pub moon_cd: f32,
    /// The shade: 0 not met, 1 haunting, 2 at rest; until it comes back; whether you know where its grave is.
    pub shade: u8,
    pub shade_cd: f32,
    pub shade_known: bool,
    pub tomb_level: Option<LevelId>,
    pub tomb_spot: (f32, f32),
    /// Bells rung, brides slain (bits), Lady Elspeth's keepsakes found.
    pub bells: u8,
    pub bell_spot: [(f32, f32); 3],
    pub brides: u8,
    pub keepsakes: u8,
    // ---- Act 4 (gears.rs) ----
    pub gear_placed: Vec<LevelId>,
    /// Gears found (bits), and whether the automaton is yours.
    pub gears: u8,
    pub automaton: bool,
    /// The court: 0 not ruled, 1 writ (bribed or acquitted), 2 fought; whether it's called you yet; its place.
    pub court: u8,
    pub court_called: bool,
    pub court_spot: (f32, f32),
    pub writ: bool,
    /// The Timeless Vault's door tile, and whether it's been emptied.
    pub vault_door: (i32, i32),
    pub vault_looted: bool,
    // ---- Act 5 (reef.rs) ----
    pub reef_placed: Vec<LevelId>,
    pub choir_spot: (f32, f32),
    pub whirl: (f32, f32),
    pub choir_done: bool,
    pub song_t: f32,
    pub kraken_spot: (f32, f32),
    pub kraken_done: bool,
    /// Bottles read (0-3), 4 once the cache is dug up; where the cache lies.
    pub bottle: u8,
    pub cache: (f32, f32),
    pub cache_level: Option<LevelId>,
    /// Ysolde's crew laid to rest (bits).
    pub ghosts: u8,
    // ---- Act 6 (isles.rs) ----
    pub isles_placed: Vec<LevelId>,
    pub stones: Vec<crate::isles::Stone>,
    /// The storm relic: 0 on its pedestal, 1 carried, 2 charged, 3 returned.
    pub relic: u8,
    pub relic_spot: (f32, f32),
    pub charge: f32,
    pub zap_t: f32,
    /// Storm cells over the Stormfields: where each is, and where it's drifting to.
    pub cells: Vec<(f32, f32, f32, f32)>,
    pub star_level: Option<LevelId>,
    pub star_spot: (f32, f32),
    pub star_done: bool,
    /// The Last Choir's singers found (bits); the sanctum: 0 sealed, 1 guarded, 2 open.
    pub singers: u8,
    pub sanctum: u8,
    pub sanctum_spot: (f32, f32),
    /// Bram's crates found, the Weeping Seraph's tears (bits).
    pub cargo: u8,
    pub tears: u8,
    // ---- Act 7 (chaos.rs) ----
    /// Seconds to the next surge, and the warning before it; surges so far.
    pub surge_t: f32,
    pub surge_warn: f32,
    pub surges: u32,
    /// The stillness meter (0-100).
    pub stillness: f32,
    pub chaos_placed: Vec<LevelId>,
    pub anchors: Vec<crate::chaos::Anchor>,
    /// The roads that never close, for this level.
    pub lifeline: (Option<LevelId>, Vec<bool>),
}

impl Game {
    /// Entering a level: put its set pieces in place (once per difficulty).
    pub(crate) fn features_enter(&mut self) {
        let here = self.level;
        if self.feats.placed.contains(&here) {
            return;
        }
        self.feats.placed.push(here);
        if here == BARN {
            self.place_barn();
        }
        if here == MARKET {
            self.place_market();
        }
        if here == LAKE {
            self.place_lake();
        }
        if here == HALL {
            self.place_hall();
        }
        if here == MERCHANT && !self.feats.merchant_free {
            self.place_ice_block(ICE_MERCHANT, 22.0);
        }
        if here == CUB && self.feats.cub < 2 {
            self.place_cub();
        }
        if let Some(k) = SCOUTS.iter().position(|l| *l == here) {
            if self.feats.scouts < 3 && (self.feats.scouts as usize) <= k + 1 {
                self.place_ice_block(ICE_SCOUT, 18.0);
            }
        }
    }

    /// A random open spot far from the hero that the hero can walk to (`min` tiles or more).
    fn feat_spot(&mut self, min: f32, room: f32) -> Option<(f32, f32)> {
        let (sx, sy) = (self.p.x, self.p.y);
        for _ in 0..400 {
            let x = self.rng.range(6, self.d.w - 6) as f32 + 0.5;
            let y = self.rng.range(6, self.d.h - 6) as f32 + 0.5;
            if ((x - sx).powi(2) + (y - sy).powi(2)).sqrt() < min || self.d.blocked(x, y, room) || self.in_safe(x, y) {
                continue;
            }
            if self.portals.iter().any(|p| (p.x - x).powi(2) + (p.y - y).powi(2) < 64.0) {
                continue;
            }
            if self.d.path((sx as i32, sy as i32), (x as i32, y as i32), 40_000).is_some() {
                return Some((x, y));
            }
        }
        None
    }

    /// Clears props from a rectangle of tiles (to make room for a set piece).
    fn clear_props(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) {
        let hit = |p: &Prop| p.foot.0 < x1 && p.foot.0 + p.foot.2 > x0 && p.foot.1 < y1 && p.foot.1 + p.foot.3 > y0;
        let gone: Vec<Prop> = self.props.iter().filter(|p| hit(p) && !matches!(p.kind, PropKind::Entrance(_) | PropKind::StairsUp | PropKind::StairsDown | PropKind::Shrine(_))).map(|p| Prop { ..*p }).collect();
        for p in &gone {
            for y in p.foot.1..p.foot.1 + p.foot.3 {
                for x in p.foot.0..p.foot.0 + p.foot.2 {
                    self.d.set(x, y, crate::dungeon::Tile::Floor);
                }
            }
        }
        self.props.retain(|p| !hit(p) || matches!(p.kind, PropKind::Entrance(_) | PropKind::StairsUp | PropKind::StairsDown | PropKind::Shrine(_)));
    }

    pub(crate) fn set_prop(&mut self, kind: PropKind, x0: i32, y0: i32, w: i32, h: i32) {
        self.clear_props(x0 - 1, y0 - 1, x0 + w + 1, y0 + h + 1);
        for y in y0..y0 + h {
            for x in x0..x0 + w {
                self.d.set(x, y, crate::dungeon::Tile::Prop);
            }
        }
        self.props.push(Prop::on(kind, x0, y0, w, h));
    }

    fn neutral_pack(&mut self, kind: Kind, (x, y): (f32, f32), n: usize, group: u8, r: f32) {
        for k in 0..n {
            let a = k as f32 / n as f32 * std::f32::consts::TAU;
            let (mx, my) = (x + a.cos() * r, y + a.sin() * r);
            if self.d.blocked(mx, my, 0.35) {
                continue;
            }
            let mut m = Mob::new(kind, mx, my, self.tier, &mut self.rng);
            m.neutral = group;
            m.dir = crate::iso::dir8(x - mx, y - my);
            self.mobs.push(m);
        }
    }

    pub(crate) fn stock(&mut self, who: Role, n: usize, rare: bool) {
        let ilvl = crate::items::ilvl_for(self.tier) + 3;
        let items = (0..n)
            .map(|_| {
                let r = if rare || self.rng.chance(0.4) { crate::items::Rarity::Rare } else { crate::items::Rarity::Magic };
                Some(crate::items::roll(ilvl, r, &mut self.rng))
            })
            .collect();
        self.feats.stock.retain(|s| s.0 != who);
        self.feats.stock.push((who, items));
    }

    // ------------------------------------------------------------------ Act 1

    fn place_barn(&mut self) {
        if self.feats.barn_over {
            return;
        }
        let Some((x, y)) = self.feat_spot(24.0, 2.5) else { return };
        let (bx, by) = (x as i32 - 2, y as i32 - 2);
        self.set_prop(PropKind::House2, bx, by, 4, 4);
        self.feats.barn_spot = (bx as f32 + 2.0, by as f32 + 2.0);
        // Three villagers trapped by the flames around it.
        let names = ["FARMHAND", "MILKMAID", "OLD TOMAS"];
        for (k, (dx, dy)) in [(-1.6, 2.5), (5.4, 1.0), (2.0, 5.4)].into_iter().enumerate() {
            let (vx, vy) = (bx as f32 + dx, by as f32 + dy);
            if !self.d.blocked(vx, vy, 0.3) {
                self.npcs.push(Npc::new(names[k], Role::Rescue(k as u8), "npc_villager", vx, vy, 1));
            }
        }
        self.feats.barn_t = 0.0;
    }

    fn place_market(&mut self) {
        if self.feats.market_hostile {
            return;
        }
        let Some((x, y)) = self.feat_spot(16.0, 2.0) else { return };
        let (tx, ty) = (x as i32 - 1, y as i32 - 2);
        self.set_prop(PropKind::Stall, tx, ty, 3, 2);
        self.npcs.push(Npc::new("GRUBNIK THE FENCE", Role::GoblinTrader, "goblin", x, y + 1.0, 2));
        self.neutral_pack(Kind::Goblin, (x, y + 1.0), 5, G_MARKET, 2.6);
        self.stock(Role::GoblinTrader, 3, false);
        self.say("A GOBLIN CAMP... THEY'RE NOT ATTACKING. ONE OF THEM IS WAVING YOU OVER".into());
    }

    // ------------------------------------------------------------------ Act 2

    fn place_lake(&mut self) {
        let Some((cx, cy)) = self.feat_spot(18.0, 1.0) else { return };
        let (w, h) = (self.d.w, self.d.h);
        let (rx, ry) = (12.0f32, 9.0f32);
        let mut lake = vec![false; (w * h) as usize];
        let (x0, y0, x1, y1) = ((cx - rx) as i32 - 1, (cy - ry) as i32 - 1, (cx + rx) as i32 + 2, (cy + ry) as i32 + 2);
        let inside = |x: i32, y: i32| ((x as f32 + 0.5 - cx) / rx).powi(2) + ((y as f32 + 0.5 - cy) / ry).powi(2) < 1.0;
        let gone: Vec<(i32, i32, i32, i32)> = self.props.iter().filter(|p| (p.foot.1..p.foot.1 + p.foot.3).any(|y| (p.foot.0..p.foot.0 + p.foot.2).any(|x| inside(x, y)))).filter(|p| !matches!(p.kind, PropKind::Entrance(_))).map(|p| p.foot).collect();
        for &(fx, fy, fw, fh) in &gone {
            self.clear_props(fx, fy, fx + fw, fy + fh);
        }
        for y in y0.max(1)..y1.min(h - 1) {
            for x in x0.max(1)..x1.min(w - 1) {
                if inside(x, y) && self.d.get(x, y) == crate::dungeon::Tile::Floor {
                    lake[(y * w + x) as usize] = true;
                    self.d.set_ground(x, y, 1);
                }
            }
        }
        self.feats.ice = Ice { w, lake, stress: vec![0.0; (w * h) as usize], holes: vec![], safe: (self.p.x, self.p.y) };
        // Heavy feet on the ice.
        self.neutral_pack(Kind::Yeti, (cx, cy), 3, 0, 3.0);
        self.neutral_pack(Kind::IceTroll, (cx + 4.0, cy - 2.0), 2, 0, 1.5);
    }

    fn place_hall(&mut self) {
        if self.feats.jarl >= 2 {
            return;
        }
        let Some((x, y)) = self.feat_spot(20.0, 3.5) else { return };
        let (hx, hy) = (x as i32 - 2, y as i32 - 3);
        self.set_prop(PropKind::Longhouse1, hx, hy, 5, 4);
        let front = (hx as f32 + 2.5, hy as f32 + 5.6);
        self.npcs.push(Npc::new("JARL HROGAR", Role::Jarl, "raider", front.0, front.1, 2));
        self.neutral_pack(Kind::Raider, (front.0, front.1 + 1.0), 7, G_RAIDERS, 3.2);
        // His war horn hangs by the hall's back wall.
        let horn = (hx as f32 + 5.8, hy as f32 + 0.6);
        let horn = if self.d.blocked(horn.0, horn.1, 0.3) { (hx as f32 - 0.8, hy as f32 + 0.6) } else { horn };
        self.feats.horn_spot = horn;
        if let Some(u) = crate::items::boss_unique("jarl") {
            self.pickups.push(Pickup { x: horn.0, y: horn.1, kind: Drop::Item(Box::new(u)), t: 1.0 });
        }
        self.say("A RAIDERS' LONGHALL. THEY EYE YOU, BUT NOBODY DRAWS A BLADE. YET".into());
    }

    fn place_ice_block(&mut self, form: u8, min: f32) {
        let Some(spot) = self.feat_spot(min, 1.0) else { return };
        let mut m = Mob::new(Kind::IceBlock, spot.0, spot.1, self.tier, &mut self.rng);
        m.form = form;
        self.mobs.push(m);
        if form == ICE_SCOUT {
            // Ice wraiths keep the scout frozen.
            let mut k = 0;
            while k < 3 {
                let a = k as f32 * 2.1;
                let (x, y) = (spot.0 + a.cos() * 2.5, spot.1 + a.sin() * 2.5);
                if !self.d.blocked(x, y, 0.35) {
                    self.mobs.push(Mob::new(Kind::IceWraith, x, y, self.tier, &mut self.rng));
                }
                k += 1;
            }
        }
    }

    fn place_cub(&mut self) {
        let Some(a) = self.feat_spot(16.0, 0.8) else { return };
        self.npcs.push(Npc::new("A LOST YETI CUB", Role::YetiCub, "yeti_cub", a.0, a.1, 2));
        for _ in 0..10 {
            if let Some(b) = self.feat_spot(16.0, 1.5) {
                if (b.0 - a.0).powi(2) + (b.1 - a.1).powi(2) > 400.0 {
                    self.feats.den = b;
                    self.decals.push(Decal { x: b.0, y: b.1, r: 1.4, col: rgb(0x203040), a: 0.55 });
                    break;
                }
            }
        }
        self.feats.cub = 0;
    }

    // ------------------------------------------------------------------ every tick

    pub(crate) fn update_features(&mut self) {
        self.update_neutrals();
        self.update_storm();
        let here = self.level;
        if here == BARN {
            self.update_barn();
        }
        if here == LAKE && !self.feats.ice.lake.is_empty() {
            self.update_ice();
        }
        if here == HALL {
            self.update_hall();
        }
        if here == CUB {
            self.update_cub();
        }
    }

    /// Neutral monsters stand by until one of their group is hurt; then the whole group turns on you.
    fn update_neutrals(&mut self) {
        let hurt: Vec<u8> = self.mobs.iter().filter(|m| m.neutral > 0 && (m.hp < m.max_hp || !m.alive())).map(|m| m.neutral).collect();
        for g in hurt {
            // The yetis of a cub's home never hold a grudge for long; the others do.
            for m in self.mobs.iter_mut().filter(|m| m.neutral == g) {
                m.neutral = 0;
                if m.alive() {
                    m.state = MobState::Chase;
                }
            }
            match g {
                G_MARKET if !self.feats.market_hostile => {
                    self.feats.market_hostile = true;
                    if let Some(i) = self.npcs.iter().position(|n| n.role == Role::GoblinTrader) {
                        let (x, y) = (self.npcs[i].x, self.npcs[i].y);
                        self.floater(x, y, "TRAITOR! GET THEM!".into(), rgb(0xff6040));
                        self.npcs.remove(i);
                    }
                }
                G_RAIDERS if self.feats.jarl < 2 => {
                    self.jarl_attacks("YOU DARE?! SHIELDS UP!");
                }
                _ => {}
            }
        }
    }

    fn update_storm(&mut self) {
        let ashlands = self.level.act() == 0 && matches!(self.level, LevelId::Area(..));
        if self.feats.storm > 0.0 {
            self.feats.storm -= DT;
            if !ashlands || self.feats.storm <= 0.0 {
                self.feats.storm = 0.0;
                self.light_ready = false;
                // The ash elementals crumble as it passes.
                let gone: Vec<usize> = (0..self.mobs.len()).filter(|&i| self.mobs[i].kind == Kind::AshElemental && self.mobs[i].alive()).collect();
                for i in gone {
                    let (x, y) = (self.mobs[i].x, self.mobs[i].y);
                    self.mobs[i].state = MobState::Dead(10.0);
                    for _ in 0..8 {
                        self.spray_at(x, y, PKind::Smoke, 10.0);
                    }
                }
                if ashlands {
                    self.say("THE ASH STORM PASSES".into());
                }
                return;
            }
            // Ash on the wind.
            if self.tick % 3 == 0 {
                let (x, y) = (self.p.x + self.rng.rf(-9.0, 9.0), self.p.y + self.rng.rf(-9.0, 9.0));
                self.spray_at(x, y, PKind::Smoke, 30.0);
            }
            return;
        }
        if !ashlands || self.in_safe(self.p.x, self.p.y) {
            return;
        }
        if self.feats.storm_cd <= 0.0 {
            self.feats.storm_cd = self.rng.rf(STORM_GAP.0, STORM_GAP.1);
        }
        self.feats.storm_cd -= DT;
        if self.feats.storm_cd <= 0.0 {
            self.start_storm();
        }
    }

    pub(crate) fn start_storm(&mut self) {
        self.feats.storm = STORM_TIME;
        self.feats.storm_cd = self.rng.rf(STORM_GAP.0, STORM_GAP.1);
        self.light_ready = false;
        self.say("AN ASH STORM ROLLS IN. SOMETHING MOVES IN THE GREY".into());
        for k in 0..4 {
            let a = k as f32 * 1.57 + self.rng.f();
            let (x, y) = (self.p.x + a.cos() * 8.0, self.p.y + a.sin() * 8.0);
            if !self.d.blocked(x, y, 0.4) {
                let mut m = Mob::new(Kind::AshElemental, x, y, self.tier, &mut self.rng);
                m.state = MobState::Chase;
                self.mobs.push(m);
            }
        }
    }

    /// The light in an ash storm (1 = clear).
    pub fn storm_light(&self) -> f32 {
        if self.feats.storm > 0.0 {
            0.45
        } else {
            1.0
        }
    }

    fn update_barn(&mut self) {
        if self.feats.barn_over || self.feats.barn_spot == (0.0, 0.0) {
            return;
        }
        let (bx, by) = self.feats.barn_spot;
        let near = ((bx - self.p.x).powi(2) + (by - self.p.y).powi(2)).sqrt();
        if self.feats.barn_t == 0.0 && near > 16.0 {
            return;
        }
        if self.feats.barn_t == 0.0 {
            self.say("A BARN IS BURNING! PULL THE VILLAGERS OUT BEFORE THE ROOF FALLS!".into());
        }
        self.feats.barn_t += DT;
        // Flames lick out of the barn.
        if self.tick % 2 == 0 {
            let (x, y) = (bx + self.rng.rf(-2.2, 2.2), by + self.rng.rf(-2.2, 2.2));
            self.spray_at(x, y, PKind::Fire, 40.0);
        }
        if self.tick % 20 == 0 {
            self.lights.push(Light { x: bx, y: by, r: 140.0, s: 1.0, life: 0.4, max: 0.4 });
        }
        if self.tick % 120 == 0 {
            let a = self.rng.f() * std::f32::consts::TAU;
            let (x, y) = (bx + a.cos() * 3.2, by + a.sin() * 3.2);
            if !self.d.blocked(x, y, 0.2) {
                self.hazards.push(crate::mobs::Hazard { x, y, r: 0.9, warn: 0.8, live: 3.0, dps: 4.0 * self.tier, burst: 0.0, t: 0.0, fired: false, kind: crate::mobs::HazardKind::Slag });
            }
        }
        // Pull a villager out: walk up to them.
        let (px, py) = (self.p.x, self.p.y);
        if let Some(i) = self.npcs.iter().position(|n| matches!(n.role, Role::Rescue(_)) && (n.x - px).powi(2) + (n.y - py).powi(2) < 1.6 * 1.6) {
            let (x, y, name) = (self.npcs[i].x, self.npcs[i].y, self.npcs[i].name);
            self.npcs.remove(i);
            self.feats.barn_saved += 1;
            self.floater(x, y, format!("{name}: SAVED!"), rgb(0x80ff80));
            self.sfx.push(Sfx::Pickup);
            let gold = (20.0 + 10.0 * self.tier) as i32;
            self.pickups.push(Pickup { x, y, kind: Drop::Gold(gold), t: 0.0 });
            self.gain_xp(30.0 * self.tier);
        }
        let left = self.npcs.iter().filter(|n| matches!(n.role, Role::Rescue(_))).count();
        if left == 0 || self.feats.barn_t > BARN_TIME {
            self.feats.barn_over = true;
            self.npcs.retain(|n| !matches!(n.role, Role::Rescue(_)));
            self.shake = 0.8;
            self.sfx.push(Sfx::Boom);
            for _ in 0..40 {
                self.spray_at(bx, by, PKind::Fire, 50.0);
            }
            let n = self.feats.barn_saved;
            if n == 3 {
                let ilvl = crate::items::ilvl_for(self.tier) + 3;
                self.pickups.push(Pickup { x: px, y: py + 0.5, kind: Drop::Item(Box::new(crate::items::roll(ilvl, crate::items::Rarity::Rare, &mut self.rng))), t: 0.0 });
                self.say("THE ROOF FALLS IN, BUT EVERYONE GOT OUT. THE FARMER PRESSES A GIFT ON YOU".into());
            } else {
                self.say(format!("THE ROOF FALLS IN. YOU SAVED {n} OF 3"));
            }
            self.save_due = true;
        }
    }

    fn update_ice(&mut self) {
        let ice = &mut self.feats.ice;
        let (px, py) = (self.p.x, self.p.y);
        let (tx, ty) = (px as i32, py as i32);
        let on_ice = ice.on(tx, ty);
        if !on_ice && !self.d.blocked(px, py, 0.3) {
            ice.safe = (px, py);
        }
        let w = ice.w;
        let add = |ice: &mut Ice, x: i32, y: i32, v: f32| {
            if ice.on(x, y) {
                ice.stress[(y * w + x) as usize] += v;
            }
        };
        // Fighting on the ice: every cast cracks it around you; running too.
        if on_ice {
            let casts = self.stats.casts.saturating_sub(self.ice_casts);
            for dy in -1..=1 {
                for dx in -1..=1 {
                    add(ice, tx + dx, ty + dy, casts as f32 * 0.12);
                }
            }
            if self.p.moving && self.p.running {
                add(ice, tx, ty, 0.05 * DT);
            }
        }
        self.ice_casts = self.stats.casts;
        // Heavy feet: yetis and trolls.
        let heavy: Vec<(i32, i32)> = self.mobs.iter().filter(|m| m.alive() && m.moving && matches!(m.kind, Kind::Yeti | Kind::IceTroll | Kind::YetiMatriarch | Kind::FrostGiant)).map(|m| (m.x as i32, m.y as i32)).collect();
        for (x, y) in heavy {
            add(&mut self.feats.ice, x, y, 0.25 * DT);
        }
        // Breaking through.
        let mut new_holes = vec![];
        let ice = &mut self.feats.ice;
        for (i, s) in ice.stress.iter_mut().enumerate() {
            if *s >= 1.0 && ice.lake[i] {
                *s = -100.0;
                new_holes.push(((i as i32) % w, (i as i32) / w));
            }
        }
        for (x, y) in new_holes {
            self.feats.ice.holes.push((x, y));
            self.d.set(x, y, crate::dungeon::Tile::Void);
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                add(&mut self.feats.ice, x + dx, y + dy, 0.3);
            }
            self.sfx.push(Sfx::Hit);
            for _ in 0..6 {
                self.spray_at(x as f32 + 0.5, y as f32 + 0.5, PKind::Magic, 6.0);
            }
            // Monsters standing there go under.
            let under: Vec<usize> = (0..self.mobs.len()).filter(|&i| self.mobs[i].alive() && self.mobs[i].x as i32 == x && self.mobs[i].y as i32 == y && !self.mobs[i].boss).collect();
            for i in under {
                let (mx, my) = (self.mobs[i].x, self.mobs[i].y);
                self.floater(mx, my, "THROUGH THE ICE!".into(), rgb(0x80c8ff));
                self.kill(i);
            }
            // So do you.
            if (self.p.x as i32, self.p.y as i32) == (x, y) {
                let dmg = self.p.max_hp * 0.15;
                self.p.hp = (self.p.hp - dmg).max(1.0);
                self.p.chill = self.p.chill.max(3.0);
                (self.p.x, self.p.y) = self.feats.ice.safe;
                self.say("YOU FALL THROUGH THE ICE! FREEZING WATER...".into());
                self.shake = 0.6;
            }
        }
    }

    fn jarl_attacks(&mut self, line: &str) {
        if let Some(i) = self.npcs.iter().position(|n| n.role == Role::Jarl) {
            let (x, y) = (self.npcs[i].x, self.npcs[i].y);
            self.npcs.remove(i);
            self.floater(x, y, line.into(), rgb(0xff8040));
            let mut m = Mob::new(Kind::Raider, x, y, self.tier * 1.3, &mut self.rng);
            let s = crate::side::JARL;
            let def = &crate::side::SUPERS[s];
            m.promote(Rank::Elite, def.mods, Some(def.name.into()));
            m.max_hp *= 2.5;
            m.hp = m.max_hp;
            m.superu = s as u8 + 1;
            m.state = MobState::Chase;
            self.mobs.push(m);
        }
    }

    fn update_hall(&mut self) {
        // The horn taken while the Jarl still sits at his hall: thief!
        if self.feats.jarl == 0 && self.feats.horn_spot != (0.0, 0.0) {
            let there = self.pickups.iter().any(|k| (k.x - self.feats.horn_spot.0).abs() < 0.2 && (k.y - self.feats.horn_spot.1).abs() < 0.2);
            if !there {
                self.feats.jarl = 3;
                self.jarl_attacks("THIEF! THE HORN! AFTER THEM!");
                for m in self.mobs.iter_mut().filter(|m| m.neutral == G_RAIDERS) {
                    m.neutral = 0;
                    m.state = MobState::Chase;
                }
                self.save_due = true;
            }
        }
        // The duel: the Jarl falls, his raiders salute you.
        if self.feats.jarl == 1 && !self.mobs.iter().any(|m| m.superu as usize == crate::side::JARL + 1 && m.alive()) {
            self.feats.jarl = 2;
            self.say("THE JARL FALLS. HIS RAIDERS BEAT THEIR SHIELDS: SKAL! THE HORN IS YOURS BY RIGHT".into());
            self.save_due = true;
        }
    }

    fn update_cub(&mut self) {
        let Some(i) = self.npcs.iter().position(|n| n.role == Role::YetiCub) else { return };
        let (px, py) = (self.p.x, self.p.y);
        let (cx, cy) = (self.npcs[i].x, self.npcs[i].y);
        let d = ((px - cx).powi(2) + (py - cy).powi(2)).sqrt();
        match self.feats.cub {
            0 if d < 1.6 => {
                self.feats.cub = 1;
                self.floater(cx, cy, "MRRR?".into(), rgb(0xe0f0ff));
                self.say("THE CUB FOLLOWS YOU. TAKE IT HOME TO ITS DEN (SEE YOUR MAP)".into());
            }
            1 => {
                if d > 1.8 {
                    let (dx, dy) = ((px - cx) / d, (py - cy) / d);
                    let sp = if d > 6.0 { 6.5 } else { 4.4 };
                    let (mut x, mut y) = (cx, cy);
                    crate::game::move_circle(&self.d, &mut x, &mut y, dx * sp * DT, dy * sp * DT, 0.3);
                    if d > 14.0 {
                        (x, y) = (px - dx * 1.5, py - dy * 1.5);
                    }
                    let n = &mut self.npcs[i];
                    (n.x, n.y) = (x, y);
                    n.dir = crate::iso::dir8(dx, dy);
                    n.moving = true;
                    n.anim_t += DT;
                }
                let (dx, dy) = self.feats.den;
                if (self.npcs[i].x - dx).powi(2) + (self.npcs[i].y - dy).powi(2) < 2.5 * 2.5 {
                    self.feats.cub = 2;
                    self.npcs.remove(i);
                    for m in self.mobs.iter_mut().filter(|m| matches!(m.kind, Kind::Yeti | Kind::YetiMatriarch) && m.alive()) {
                        m.neutral = G_YETIS;
                        m.state = MobState::Idle;
                    }
                    self.floater(dx, dy, "THE CUB IS HOME".into(), rgb(0xe0f0ff));
                    self.say("A HUGE YETI SNIFFS YOU AND GRUNTS. THE YETIS HERE WILL LEAVE YOU BE".into());
                    self.gain_xp(80.0 * self.tier);
                    self.pickups.push(Pickup { x: px, y: py + 0.5, kind: Drop::Gold((40.0 + 20.0 * self.tier) as i32), t: 0.0 });
                    self.save_due = true;
                }
            }
            _ => {}
        }
    }

    /// An ice block broke: the frozen merchant or a scout is free.
    pub(crate) fn ice_broken(&mut self, form: u8, x: f32, y: f32) {
        if form == ICE_MERCHANT {
            self.feats.merchant_free = true;
            self.npcs.push(Npc::new("HALVARD THE THAWED", Role::FrozenMerchant, "npc_trader", x, y, 2));
            self.stock(Role::FrozenMerchant, 3, true);
            self.floater(x, y, "BY THE GODS, I'M WARM! COME, LOOK AT MY WARES!".into(), rgb(0xffe0a0));
        } else {
            self.feats.scouts += 1;
            let n = self.feats.scouts;
            self.floater(x, y, "I... I CAN MOVE! THANK YOU!".into(), rgb(0x80ff80));
            self.say(format!("ONE OF BRENNA'S SCOUTS IS FREE ({n}/3)"));
            if n >= 3 {
                self.side_progress(crate::side::Goal::Scouts);
            }
        }
        self.save_due = true;
    }

    // ------------------------------------------------------------------ talking

    /// What the set pieces' people say (None: not theirs).
    pub(crate) fn feature_dialog(&mut self, role: Role) -> Option<Dialog> {
        match role {
            Role::GoblinTrader | Role::FrozenMerchant | Role::Caravan => {
                self.feats.trading = Some(role);
                let pitch = match role {
                    Role::GoblinTrader => "PSST. SHINY THINGS, FELL OFF A CART. CHEAP, CHEAP. NO QUESTIONS!",
                    Role::FrozenMerchant => "FROZE SOLID IN THAT ICE FOR A WINTER, I DID. YOU SAVED ME: TAKE YOUR PICK, ONE TIME ONLY.",
                    _ if self.ex.caravan_ambush == 2 => "YOU SAVED MY CARAVAN! FOR YOU, A THIRD OFF EVERYTHING ON THE WAGON.",
                    _ => "GOODS FROM EVERY ROAD BETWEEN HERE AND THE EDGE OF THE WORLD. NOT CHEAP, BUT YOU WON'T FIND BETTER IN TOWN.",
                };
                let who = match role {
                    Role::GoblinTrader => "GRUBNIK THE FENCE",
                    Role::FrozenMerchant => "HALVARD THE THAWED",
                    _ => self.ex.caravan_name,
                };
                let mut d = Dialog::new(who, &[pitch]);
                let stock = self.feats.stock.iter().find(|s| s.0 == role).map(|s| s.1.clone()).unwrap_or_default();
                let mul = self.trade_mul(role);
                d.options = stock
                    .iter()
                    .enumerate()
                    .filter_map(|(k, it)| it.as_ref().map(|it| (format!("{}  {} GOLD", it.name, it.price() * mul), Act::BuyStock(k as u8))))
                    .collect();
                d.options.push(("FAREWELL".into(), Act::Close));
                Some(d)
            }
            Role::Jarl => {
                let mut d = Dialog::new("JARL HROGAR", &["HAH! A SOUTHERNER IN MY HALL. YOU WANT MY WAR HORN? EVERYONE WANTS MY HORN. WIN IT, LIKE A RAIDER: STEEL AGAINST STEEL, YOU AND ME. MY BOYS WON'T INTERFERE... MUCH."]);
                d.options = vec![("CHALLENGE THE JARL TO A DUEL".into(), Act::Duel), ("FAREWELL".into(), Act::Close)];
                Some(d)
            }
            Role::BogWitch => Some(self.witch_dialog()),
            Role::Magistrate => Some(if self.feats.court == 0 { self.court_dialog() } else { Dialog::new("MAGISTRATE KORVEL", &["THE COURT HAS RULED. MOVE ALONG, CITIZEN."]) }),
            Role::YetiCub => Some(Dialog::new("A LOST YETI CUB", &["MRRR. (IT SNIFFS YOUR HAND AND WON'T LEAVE YOUR SIDE.)"])),
            Role::Rescue(_) => Some(Dialog::new("TRAPPED VILLAGER", &["HELP! THE FIRE! GET ME OUT OF HERE!"])),
            // The all-act systems (extras.rs).
            Role::Rival => Some(self.rival_dialog()),
            Role::ArenaMaster => Some(self.arena_dialog()),
            Role::CaravanGuard => Some(Dialog::new("CARAVAN GUARD", &["KEEP YOUR HANDS WHERE I CAN SEE THEM, FRIEND. THE MERCHANT'S OVER BY THE WAGON."])),
            _ => None,
        }
    }

    /// A trader's markup over an item's price.
    fn trade_mul(&self, role: Role) -> i32 {
        match role {
            Role::GoblinTrader => 3,
            Role::Caravan if self.ex.caravan_ambush == 2 => 2,
            Role::Caravan => 3,
            _ => 4,
        }
    }

    pub(crate) fn buy_stock(&mut self, k: usize) {
        let Some(role) = self.feats.trading else { return };
        let mul = self.trade_mul(role);
        let Some(slot) = self.feats.stock.iter_mut().find(|s| s.0 == role).and_then(|s| s.1.get_mut(k)) else { return };
        let Some(it) = slot.as_ref() else { return };
        let price = it.price() * mul;
        if self.p.gold < price {
            self.say("NOT ENOUGH GOLD".into());
            return;
        }
        if self.p.gear.free() == 0 {
            self.say("YOUR BAG IS FULL".into());
            return;
        }
        let it = slot.take().unwrap();
        self.p.gold -= price;
        self.sfx.push(Sfx::Pickup);
        self.say(format!("BOUGHT {} FOR {price} GOLD", it.name));
        let _ = self.p.gear.add(it);
        self.save_due = true;
        // Talk again: the list without it.
        self.dialog = self.feature_dialog(role);
    }

    pub(crate) fn start_duel(&mut self) {
        self.dialog = None;
        if self.feats.jarl != 0 {
            return;
        }
        self.feats.jarl = 1;
        self.jarl_attacks("TO THE DEATH, THEN! HAH!");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Game;

    fn at(id: LevelId) -> Game {
        let mut g = Game::new(7, 360);
        g.p.base_hp = 5000.0;
        g.p.recalc();
        g.p.hp = g.p.max_hp;
        g.debug_goto(id);
        g
    }

    #[test]
    fn the_barn_burns_and_villagers_can_be_saved() {
        let mut g = at(BARN);
        assert_eq!(g.npcs.iter().filter(|n| matches!(n.role, Role::Rescue(_))).count(), 3);
        let spots: Vec<(f32, f32)> = g.npcs.iter().filter(|n| matches!(n.role, Role::Rescue(_))).map(|n| (n.x, n.y)).collect();
        for s in spots {
            (g.p.x, g.p.y) = s;
            g.update_barn();
        }
        assert_eq!(g.feats.barn_saved, 3);
        assert!(g.feats.barn_over);
    }

    #[test]
    fn the_goblin_market_trades_until_you_attack() {
        let mut g = at(MARKET);
        assert!(g.npcs.iter().any(|n| n.role == Role::GoblinTrader));
        let d = g.feature_dialog(Role::GoblinTrader).unwrap();
        assert!(d.options.iter().any(|o| matches!(o.1, Act::BuyStock(_))));
        g.p.gold = 100_000;
        let bag = g.p.gear.free();
        g.buy_stock(0);
        assert_eq!(g.p.gear.free(), bag - 1, "bought");
        let i = g.mobs.iter().position(|m| m.neutral == G_MARKET).unwrap();
        g.mobs[i].hp -= 1.0;
        g.update_neutrals();
        assert!(g.feats.market_hostile && !g.npcs.iter().any(|n| n.role == Role::GoblinTrader), "the market turns on you");
        assert!(g.mobs.iter().all(|m| m.neutral != G_MARKET));
    }

    #[test]
    fn ash_storms_darken_the_ashlands_and_pass() {
        let mut g = at(LevelId::Area(0, 1));
        g.start_storm();
        assert!(g.storm_light() < 1.0);
        assert!(g.mobs.iter().any(|m| m.kind == Kind::AshElemental));
        g.feats.storm = 0.01;
        g.update_storm();
        assert_eq!(g.storm_light(), 1.0);
        assert!(!g.mobs.iter().any(|m| m.kind == Kind::AshElemental && m.alive()), "they crumble");
    }

    #[test]
    fn thin_ice_cracks_and_drops_you_in() {
        let mut g = at(LAKE);
        let ice = g.feats.ice.clone();
        let (i, _) = ice.lake.iter().enumerate().find(|(_, l)| **l).expect("a lake");
        let (x, y) = ((i as i32 % ice.w) as f32 + 0.5, (i as i32 / ice.w) as f32 + 0.5);
        (g.p.x, g.p.y) = (x, y);
        let hp = g.p.hp;
        for _ in 0..12 {
            g.stats.casts += 1;
            g.update_ice();
        }
        assert!(!g.feats.ice.holes.is_empty(), "it broke");
        assert!(g.p.hp < hp && g.p.chill > 0.0, "and you went in");
    }

    #[test]
    fn the_jarl_can_be_dueled_or_robbed() {
        let mut g = at(HALL);
        assert!(g.npcs.iter().any(|n| n.role == Role::Jarl));
        g.start_duel();
        assert_eq!(g.feats.jarl, 1);
        assert!(g.mobs.iter().all(|m| m.neutral == G_RAIDERS || m.neutral == 0));
        let j = g.mobs.iter().position(|m| m.superu as usize == crate::side::JARL + 1).unwrap();
        g.kill(j);
        g.update_hall();
        assert_eq!(g.feats.jarl, 2);
        assert!(g.mobs.iter().any(|m| m.neutral == G_RAIDERS), "the raiders stand by");
        // Robbing instead.
        let mut g = at(HALL);
        let (hx, hy) = g.feats.horn_spot;
        g.pickups.retain(|k| (k.x - hx).abs() > 0.2 || (k.y - hy).abs() > 0.2);
        g.update_hall();
        assert_eq!(g.feats.jarl, 3);
        assert!(g.mobs.iter().any(|m| m.superu as usize == crate::side::JARL + 1));
    }

    #[test]
    fn the_frozen_merchant_thaws_and_trades() {
        let mut g = at(MERCHANT);
        let i = g.mobs.iter().position(|m| m.kind == Kind::IceBlock).expect("frozen in ice");
        g.kill(i);
        assert!(g.feats.merchant_free && g.npcs.iter().any(|n| n.role == Role::FrozenMerchant));
    }

    #[test]
    fn the_cub_follows_you_home() {
        let mut g = at(CUB);
        let i = g.npcs.iter().position(|n| n.role == Role::YetiCub).unwrap();
        (g.p.x, g.p.y) = (g.npcs[i].x + 1.0, g.npcs[i].y);
        g.update_cub();
        assert_eq!(g.feats.cub, 1);
        (g.p.x, g.p.y) = g.feats.den;
        for _ in 0..600 {
            g.update_cub();
            if g.feats.cub == 2 {
                break;
            }
        }
        assert_eq!(g.feats.cub, 2, "home");
        assert!(g.mobs.iter().filter(|m| m.kind == Kind::Yeti && m.alive()).all(|m| m.neutral == G_YETIS));
    }
}

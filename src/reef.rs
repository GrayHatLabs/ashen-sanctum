//! Act 5's set pieces (docs/SIDE_CONTENT_PLAN.md 2c), the user's picks:
//!
//! - **Low Tide Treasure**: wrecked chests lie out on the tide flats of the Sunken Reach's areas. At high tide
//!   they're under water and can't be reached: grab them while the sea is out.
//! - **The Siren's Song** (the Sunken Galleon): a choir of three sirens sings you toward a whirlpool. Fight the
//!   pull; silence all three and their pearl is yours.
//! - **The Kraken's Arm** (the Bone Reef): a huge tentacle at the trench's edge grabs, drags and slams. Sever it.
//! - **Message in a Bottle**: three bottles, each pointing to the next area (a different trail every
//!   playthrough), then an X where a pirate buried his cache.
//!
//! The Pearl Grotto (Act 5's optional dungeon) only opens at low tide.
use crate::game::{Drop, Game, PKind, Pickup, Sfx, DT};
use crate::gfx::rgb;
use crate::mobs::{Kind, Mob, Rank};
use crate::rng::Rng;
use crate::story::Dialog;
use crate::world::LevelId;

pub const CHOIR: LevelId = LevelId::Area(4, 6);
pub const KRAKEN: LevelId = LevelId::Area(4, 3);
/// The choir's song pulls you this often (seconds), while you're this close.
pub const SONG_GAP: f32 = 5.0;
pub const SONG_R: f32 = 16.0;

/// The bottles' trail (this world, this difficulty): the first lies in the Kelp Shallows, the next two and the
/// cache in three of the other main areas.
pub fn bottle_trail(world_seed: u64, difficulty: u8) -> [u8; 4] {
    let mut rng = Rng::new(world_seed ^ 0xB0771E ^ difficulty as u64 * 7919);
    let mut a = vec![2u8, 3, 4, 5];
    for i in (1..a.len()).rev() {
        a.swap(i, rng.range(0, i as i32 + 1) as usize);
    }
    [1, a[0], a[1], a[2]]
}

pub const NOTES: [&str; 3] = [
    "IF YOU READ THIS, I'M DEAD AND YOU'RE LUCKY. MY CACHE IS BURIED FAR FROM HERE. MY NEXT NOTE IS IN {AREA}. - CAPTAIN SALT",
    "STILL ALIVE? GOOD. THE LAST BOTTLE IS IN {AREA}. DON'T TELL YSOLDE. SHE'S BEEN AFTER MY GOLD SINCE BEFORE THE SEA WAS DARK.",
    "HERE'S THE TRUTH OF IT: I BURIED EVERYTHING IN {AREA}. LOOK FOR THE X. SPEND IT ON SOMETHING STUPID, FOR MY SAKE.",
];

impl Game {
    /// Act 5 set pieces as you arrive.
    pub(crate) fn reef_enter(&mut self) {
        let here = self.level;
        let LevelId::Area(4, n) = here else { return };
        if self.feats.reef_placed.contains(&here) {
            return;
        }
        self.feats.reef_placed.push(here);
        if n <= 5 {
            self.place_tide_chests();
        }
        if here == CHOIR && !self.feats.choir_done {
            self.place_choir();
        }
        if here == KRAKEN && !self.feats.kraken_done {
            self.place_kraken();
        }
        let trail = bottle_trail(self.world_seed, self.quest.difficulty);
        let stage = self.feats.bottle as usize;
        if stage < 3 && trail[stage] == n {
            if let Some((x, y)) = self.reef_spot(14.0, 0.6, false) {
                self.pickups.push(Pickup { x, y, kind: Drop::Bottle(stage as u8), t: 1.0 });
            }
        }
        if stage == 3 && trail[3] == n {
            self.place_cache();
        }
    }

    /// A spot far from the hero (on the tide flats, if `flats`).
    fn reef_spot(&mut self, min: f32, room: f32, flats: bool) -> Option<(f32, f32)> {
        let (sx, sy) = (self.p.x, self.p.y);
        for _ in 0..600 {
            let x = self.rng.range(6, self.d.w - 6) as f32 + 0.5;
            let y = self.rng.range(6, self.d.h - 6) as f32 + 0.5;
            if ((x - sx).powi(2) + (y - sy).powi(2)).sqrt() < min || self.d.blocked(x, y, room) {
                continue;
            }
            if flats && self.d.ground_at(x as i32, y as i32) != 1 {
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

    fn place_tide_chests(&mut self) {
        for _ in 0..2 {
            if let Some((x, y)) = self.reef_spot(12.0, 0.6, true) {
                self.pickups.push(Pickup { x, y, kind: Drop::TideChest, t: 1.0 });
            }
        }
    }

    fn place_choir(&mut self) {
        let Some((x, y)) = self.reef_spot(18.0, 2.0, false) else { return };
        self.feats.choir_spot = (x, y);
        // The whirlpool the song drags you toward, off to one side.
        let a = self.rng.f() * std::f32::consts::TAU;
        let w = (x + a.cos() * 6.0, y + a.sin() * 6.0);
        self.feats.whirl = if self.d.blocked(w.0, w.1, 0.5) { (x - 5.0, y) } else { w };
        for (k, s) in crate::side::CHOIR.iter().enumerate() {
            let def = &crate::side::SUPERS[*s];
            let b = k as f32 * 2.1;
            let (mx, my) = (x + b.cos() * 1.8, y + b.sin() * 1.8);
            let (mx, my) = if self.d.blocked(mx, my, 0.4) { (x, y) } else { (mx, my) };
            let mut m = Mob::new(Kind::Siren, mx, my, self.tier * 1.1, &mut self.rng);
            m.promote(Rank::Elite, def.mods, Some(def.name.into()));
            m.max_hp *= 1.4;
            m.hp = m.max_hp;
            m.superu = *s as u8 + 1;
            self.mobs.push(m);
        }
    }

    fn place_kraken(&mut self) {
        let Some((x, y)) = self.reef_spot(18.0, 1.4, false) else { return };
        let mut m = Mob::new(Kind::KrakenArm, x, y, self.tier, &mut self.rng);
        m.promote(Rank::Elite, crate::mobs::M_STRONG, Some("THE KRAKEN'S ARM".into()));
        m.max_hp *= 3.0;
        m.hp = m.max_hp;
        self.mobs.push(m);
        self.feats.kraken_spot = (x, y);
    }

    fn place_cache(&mut self) {
        if let Some(s) = self.reef_spot(20.0, 0.8, false) {
            self.feats.cache = s;
            self.feats.cache_level = Some(self.level);
        }
    }

    // ------------------------------------------------------------------ every tick

    pub(crate) fn update_reef(&mut self) {
        if !matches!(self.level, LevelId::Area(4, _)) {
            return;
        }
        if self.level == CHOIR && !self.feats.choir_done {
            self.update_choir();
        }
        if self.level == KRAKEN && !self.feats.kraken_done {
            self.update_kraken();
        }
        if self.feats.cache_level == Some(self.level) && self.feats.bottle == 3 {
            let (cx, cy) = self.feats.cache;
            if (cx - self.p.x).powi(2) + (cy - self.p.y).powi(2) < 1.3 * 1.3 {
                self.dig_cache();
            }
        }
    }

    fn update_choir(&mut self) {
        let alive = self.mobs.iter().filter(|m| m.alive() && crate::side::CHOIR.iter().any(|s| *s as u8 + 1 == m.superu)).count();
        if alive == 0 {
            if self.feats.choir_spot != (0.0, 0.0) {
                self.feats.choir_done = true;
                self.save_due = true;
                let (x, y) = (self.p.x, self.p.y);
                if let Some(u) = crate::items::boss_unique("choir") {
                    self.pickups.push(Pickup { x, y: y + 0.6, kind: Drop::Item(Box::new(u)), t: 0.0 });
                }
                self.say("THE CHOIR FALLS SILENT. THEIR GREAT PEARL ROLLS TO YOUR FEET".into());
            }
            return;
        }
        let (x, y) = self.feats.choir_spot;
        let near = (x - self.p.x).powi(2) + (y - self.p.y).powi(2) < SONG_R * SONG_R;
        // The whirlpool churns whether you're near or not; it hurts to stand in.
        let (wx, wy) = self.feats.whirl;
        if (wx - self.p.x).powi(2) + (wy - self.p.y).powi(2) < 1.6 * 1.6 {
            self.hurt_player(6.0 * self.tier * DT);
        }
        if !near {
            return;
        }
        self.feats.song_t += DT;
        if self.feats.song_t > SONG_GAP {
            self.feats.song_t = 0.0;
            self.pull = (wx, wy, 1.8);
            self.floater(self.p.x, self.p.y, "THE SONG DRAGS YOU TOWARD THE WHIRLPOOL...".into(), rgb(0x60e0d0));
        }
    }

    fn update_kraken(&mut self) {
        let Some(i) = self.mobs.iter().position(|m| m.kind == Kind::KrakenArm && m.alive()) else { return };
        let (x, y) = (self.mobs[i].x, self.mobs[i].y);
        let d = ((x - self.p.x).powi(2) + (y - self.p.y).powi(2)).sqrt();
        self.mobs[i].special += DT;
        if d < 9.0 && self.mobs[i].special > 6.0 {
            self.mobs[i].special = 0.0;
            // It grabs and drags you in, then slams down where it stands.
            self.pull = (x, y, 1.2);
            self.floater(self.p.x, self.p.y, "THE ARM COILS ROUND YOU!".into(), rgb(0x60c0a0));
            let burst = 22.0 * self.tier.powf(0.8);
            self.hazards.push(crate::mobs::Hazard { x, y, r: 2.6, warn: 1.4, live: 0.0, dps: 0.0, burst, t: 0.0, fired: false, kind: crate::mobs::HazardKind::Quake });
        }
    }

    /// The arm is severed.
    pub(crate) fn kraken_killed(&mut self, x: f32, y: f32) {
        self.feats.kraken_done = true;
        self.save_due = true;
        if let Some(u) = crate::items::boss_unique("kraken") {
            self.pickups.push(Pickup { x, y, kind: Drop::Item(Box::new(u)), t: 0.0 });
        }
        self.pickups.push(Pickup { x: x + 0.6, y, kind: Drop::Gold((80.0 + 40.0 * self.tier) as i32), t: 0.0 });
        self.shake = 1.0;
        self.sfx.push(Sfx::Boom);
        self.say("THE ARM IS SEVERED! SOMETHING HUGE SHUDDERS DOWN IN THE TRENCH".into());
    }

    /// Picked up a chest from the flats (only at low tide: the pickup won't happen under water).
    pub(crate) fn open_tide_chest(&mut self) {
        let (x, y) = (self.p.x, self.p.y);
        let ilvl = crate::items::ilvl_for(self.tier) + 2;
        self.pickups.push(Pickup { x, y: y + 0.4, kind: Drop::Gold((40.0 + 25.0 * self.tier) as i32), t: 0.0 });
        if self.rng.chance(0.6) {
            let r = if self.rng.chance(0.35) { crate::items::Rarity::Rare } else { crate::items::Rarity::Magic };
            self.pickups.push(Pickup { x: x + 0.5, y, kind: Drop::Item(Box::new(crate::items::roll(ilvl, r, &mut self.rng))), t: 0.0 });
        }
        if self.rng.chance(0.3) {
            self.pickups.push(Pickup { x: x - 0.5, y, kind: Drop::Item(Box::new(crate::items::gem_item(crate::items::roll_gem(ilvl, &mut self.rng)))), t: 0.0 });
        }
        self.floater(x, y, "A WRECKED CHEST, DRAGGED OUT OF THE SAND".into(), rgb(0xe0c080));
    }

    /// Read a bottle's note: it points to the next area.
    pub(crate) fn read_bottle(&mut self, k: u8) {
        let trail = bottle_trail(self.world_seed, self.quest.difficulty);
        let next = Game::waypoint_name(LevelId::Area(4, trail[k as usize + 1]));
        self.feats.bottle = self.feats.bottle.max(k + 1);
        self.save_due = true;
        let note = NOTES[k as usize % 3].replace("{AREA}", &next);
        let mut d = Dialog::new("A MESSAGE IN A BOTTLE", &[&note]);
        d.refresh_options();
        self.dialog = Some(d);
        if k == 2 {
            // The cache is in the area named; if that's here, it's laid now.
            if LevelId::Area(4, trail[3]) == self.level {
                self.place_cache();
            }
        }
    }

    fn dig_cache(&mut self) {
        self.feats.bottle = 4;
        self.save_due = true;
        let (x, y) = self.feats.cache;
        let ilvl = crate::items::ilvl_for(self.tier) + 4;
        for k in 0..6 {
            let a = k as f32;
            self.pickups.push(Pickup { x: x + a.cos(), y: y + a.sin(), kind: Drop::Gold((60.0 + 30.0 * self.tier) as i32), t: 0.0 });
        }
        for dx in [-0.8f32, 0.8] {
            let it = crate::items::roll(ilvl, crate::items::Rarity::Rare, &mut self.rng);
            self.pickups.push(Pickup { x: x + dx, y: y + 0.8, kind: Drop::Item(Box::new(it)), t: 0.0 });
        }
        for _ in 0..16 {
            self.spray_at(x, y, PKind::Smoke, 8.0);
        }
        self.sfx.push(Sfx::Boom);
        self.say("CAPTAIN SALT'S CACHE! HE WASN'T LYING".into());
    }

    /// The X of the pirate's cache, on this level.
    pub fn cache_x(&self) -> Option<(f32, f32)> {
        (self.feats.cache_level == Some(self.level) && self.feats.bottle == 3).then_some(self.feats.cache)
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
    fn tide_chests_wait_out_on_the_flats() {
        let mut g = at(LevelId::Area(4, 1));
        let i = g.pickups.iter().position(|k| matches!(k.kind, Drop::TideChest)).expect("a chest on the flats");
        let (x, y) = (g.pickups[i].x, g.pickups[i].y);
        assert_eq!(g.d.ground_at(x as i32, y as i32), 1);
        // Under water: no.
        g.tide = 1.0;
        (g.p.x, g.p.y) = (x, y);
        g.pickups[i].t = 1.0;
        let n = g.pickups.len();
        g.collect_pickups();
        assert_eq!(g.pickups.len(), n, "under water");
        g.tide = 0.0;
        g.collect_pickups();
        assert!(!g.pickups.iter().any(|k| matches!(k.kind, Drop::TideChest) && (k.x - x).abs() < 0.1 && (k.y - y).abs() < 0.1), "grabbed at low tide");
    }

    #[test]
    fn the_choir_sings_you_to_the_whirlpool_until_silenced() {
        let mut g = at(CHOIR);
        assert_eq!(g.mobs.iter().filter(|m| m.kind == Kind::Siren && m.superu > 0).count(), 3);
        (g.p.x, g.p.y) = (g.feats.choir_spot.0 + 3.0, g.feats.choir_spot.1);
        g.feats.song_t = SONG_GAP;
        g.update_choir();
        assert!(g.pull.2 > 0.0, "pulled");
        for m in g.mobs.iter_mut().filter(|m| m.kind == Kind::Siren && m.superu > 0) {
            m.state = crate::mobs::MobState::Dead(0.0);
        }
        g.update_choir();
        assert!(g.feats.choir_done);
    }

    #[test]
    fn the_kraken_arm_grabs_and_can_be_severed() {
        let mut g = at(KRAKEN);
        let i = g.mobs.iter().position(|m| m.kind == Kind::KrakenArm).expect("the arm");
        (g.p.x, g.p.y) = (g.mobs[i].x + 4.0, g.mobs[i].y);
        g.mobs[i].special = 6.5;
        g.update_kraken();
        assert!(g.pull.2 > 0.0);
        g.kill(i);
        assert!(g.feats.kraken_done);
    }

    #[test]
    fn three_bottles_lead_to_a_pirate_cache() {
        let trail = bottle_trail(7, 0);
        assert_eq!(trail[0], 1);
        let mut g = at(LevelId::Area(4, 1));
        assert!(g.pickups.iter().any(|k| matches!(k.kind, Drop::Bottle(0))));
        g.read_bottle(0);
        g.read_bottle(1);
        g.read_bottle(2);
        g.debug_goto(LevelId::Area(4, trail[3]));
        let (x, y) = g.cache_x().expect("an X");
        (g.p.x, g.p.y) = (x, y);
        g.update_reef();
        assert_eq!(g.feats.bottle, 4);
    }
}

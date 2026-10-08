//! Act 3's set pieces (docs/SIDE_CONTENT_PLAN.md 2c), the user's picks:
//!
//! - **The Witch of the Bog** (the Witch's Bog): three pacts, each with an upside and a downside, for good.
//! - **Wolf Moon**: now and then the moon turns red over the Mistwood's areas; werewolf packs hunt, and every
//!   werewolf pays a pelt bounty while it lasts.
//! - **The Haunted Manor** (the Hollow Wood): Lady Elspeth's ghost can't be hurt; find her three keepsakes in
//!   the manor and she can be laid to rest.
//! - **Vardak's Brides**: three vampire brides hide in three of the Mistwood's areas (different every
//!   playthrough); kill all three before the Count falls and the last drops his wedding band.
//! - **The Wailing Shade** (the user's idea): a ghost haunts you through the Mistwood. Wound it and it flees;
//!   it can only be put to rest at its tomb, in a random area. Follow it there.
//! - **The Bells of Mournhold** (Father Lucian's side quest): ring three bells, once their guards are dead.
use crate::game::{Decal, Drop, Game, PKind, Pickup, Sfx, DT};
use crate::gfx::rgb;
use crate::mobs::{Kind, Mob, MobState, Rank};
use crate::rng::Rng;
use crate::story::{Act, Dialog, Npc, Role};
use crate::world::{Level, LevelId, Prop, PropKind};

pub const WITCH: LevelId = LevelId::Area(2, 6);
/// The bells: one each in the Blighted Fields, the Gallows Moor and the Barrow Hills.
pub const BELLS: [LevelId; 3] = [LevelId::Area(2, 1), LevelId::Area(2, 2), LevelId::Area(2, 4)];
/// The Mistwood's areas (brides and the shade's tomb pick from these).
pub const MIST_AREAS: [u8; 6] = [1, 2, 3, 4, 5, 6];

/// The witch's pacts (bits of `Player::pacts`).
pub const PACT_WOLF: u8 = 1;
pub const PACT_CROW: u8 = 2;
pub const PACT_TOAD: u8 = 4;
pub const PACTS: [(u8, &str, &str); 3] = [
    (PACT_WOLF, "THE WOLF'S HEART", "+15% DAMAGE, BUT -10% LIFE"),
    (PACT_CROW, "THE CROW'S EYE", "+1 TO ALL SKILLS, BUT YOU HUNGER FASTER"),
    (PACT_TOAD, "THE TOAD'S LUCK", "1500 GOLD NOW, BUT -20% GOLD FOUND"),
];

pub const MOON_TIME: f32 = 90.0;
pub const MOON_GAP: (f32, f32) = (240.0, 420.0);
/// The shade comes back this often (a range), and flees at this share of its life.
pub const SHADE_GAP: (f32, f32) = (70.0, 110.0);
pub const SHADE_FLEE: f32 = 0.45;

/// Where Vardak's brides hide, and where the shade's tomb is (this world, this difficulty).
pub fn bride_areas(world_seed: u64, difficulty: u8) -> [u8; 3] {
    let mut rng = Rng::new(world_seed ^ 0xB21D_E500 ^ difficulty as u64 * 7919);
    let mut a = MIST_AREAS.to_vec();
    for i in (1..a.len()).rev() {
        a.swap(i, rng.range(0, i as i32 + 1) as usize);
    }
    [a[0], a[1], a[2]]
}

pub fn tomb_area(world_seed: u64, difficulty: u8) -> LevelId {
    let mut rng = Rng::new(world_seed ^ 0x7033_5ADE ^ difficulty as u64 * 131);
    // Not the first area: it should be a chase.
    LevelId::Area(2, MIST_AREAS[1 + rng.range(0, MIST_AREAS.len() as i32 - 1) as usize])
}

/// The manor's keepsakes, lying in its far rooms (world::build_at).
pub fn place(lv: &mut Level, seed: u64) {
    if lv.id != LevelId::Dungeon(crate::world::MANOR, 0) {
        return;
    }
    let mut rng = Rng::new(seed ^ 0x6EE9_5A4E);
    let rooms = lv.d.rooms.clone();
    let mut picked: Vec<usize> = (1..rooms.len()).collect();
    for i in (1..picked.len()).rev() {
        picked.swap(i, rng.range(0, i as i32 + 1) as usize);
    }
    for (k, &r) in picked.iter().take(3).enumerate() {
        let (x, y) = rooms[r].center();
        lv.pickups.push(Pickup { x: x as f32 + 0.5, y: y as f32 + 0.5, kind: Drop::Keepsake(k as u8), t: 1.0 });
    }
}

pub const KEEPSAKES: [&str; 3] = ["A CHILD'S PORTRAIT", "A PRESSED ROSE", "A LOVE LETTER"];

impl Game {
    /// Act 3 set pieces as you arrive.
    pub(crate) fn mist_enter(&mut self) {
        let here = self.level;
        if here.act() != 2 || !matches!(here, LevelId::Area(..)) {
            return;
        }
        // The shade finds you the first time you walk the Mistwood's wilds.
        if self.feats.shade == 0 {
            self.feats.shade = 1;
            self.feats.shade_cd = 20.0;
            self.say("A COLD BREATH ON YOUR NECK... SOMETHING FOLLOWS YOU THROUGH THE MIST".into());
        }
        if self.feats.mist_placed.contains(&here) {
            return;
        }
        self.feats.mist_placed.push(here);
        if here == WITCH {
            self.place_witch();
        }
        if let Some(k) = BELLS.iter().position(|b| *b == here) {
            if self.feats.bells & (1 << k) == 0 {
                self.place_bell(k);
            }
        }
        let LevelId::Area(_, n) = here else { return };
        let brides = bride_areas(self.world_seed, self.quest.difficulty);
        if let Some(b) = brides.iter().position(|&a| a == n) {
            if self.feats.brides & (1 << b) == 0 {
                self.place_bride(b);
            }
        }
        if here == tomb_area(self.world_seed, self.quest.difficulty) && self.feats.shade == 1 {
            self.place_tomb();
        }
    }

    fn mist_spot(&mut self, min: f32) -> Option<(f32, f32)> {
        let (sx, sy) = (self.p.x, self.p.y);
        for _ in 0..400 {
            let x = self.rng.range(6, self.d.w - 6) as f32 + 0.5;
            let y = self.rng.range(6, self.d.h - 6) as f32 + 0.5;
            if ((x - sx).powi(2) + (y - sy).powi(2)).sqrt() < min || self.d.blocked(x, y, 1.6) {
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

    fn mist_prop(&mut self, kind: PropKind, x: i32, y: i32, w: i32, h: i32) {
        for yy in y..y + h {
            for xx in x..x + w {
                self.d.set(xx, yy, crate::dungeon::Tile::Prop);
            }
        }
        self.props.push(Prop::on(kind, x, y, w, h));
    }

    fn place_witch(&mut self) {
        let Some((x, y)) = self.mist_spot(14.0) else { return };
        self.npcs.push(Npc::new("THE BOG WITCH", Role::BogWitch, "npc_bogwitch", x, y, 2));
        self.decals.push(Decal { x, y: y + 1.0, r: 1.2, col: rgb(0x203018), a: 0.5 });
        self.mist_prop(PropKind::GlowShrooms, x as i32 + 2, y as i32 - 1, 1, 1);
    }

    fn place_bell(&mut self, k: usize) {
        let Some((x, y)) = self.mist_spot(18.0) else { return };
        self.mist_prop(PropKind::Bell, x as i32, y as i32, 1, 1);
        self.feats.bell_spot[k] = (x + 0.5, y + 1.5);
        for j in 0..4 {
            let a = j as f32 * 1.6;
            let (mx, my) = (x + 0.5 + a.cos() * 2.6, y + 0.5 + a.sin() * 2.6);
            if !self.d.blocked(mx, my, 0.35) {
                let kind = if j % 2 == 0 { Kind::Ghoul } else { Kind::Banshee };
                let mut m = Mob::new(kind, mx, my, self.tier, &mut self.rng);
                if j == 0 {
                    let mods = crate::mobs::roll_mods(1, &mut self.rng);
                    m.promote(Rank::Champion, mods, None);
                }
                self.mobs.push(m);
            }
        }
    }

    fn place_bride(&mut self, b: usize) {
        let Some((x, y)) = self.mist_spot(20.0) else { return };
        let s = crate::side::BRIDES[b];
        let def = &crate::side::SUPERS[s];
        let mut m = Mob::new(Kind::Bride, x, y, self.tier * 1.2, &mut self.rng);
        m.promote(Rank::Elite, def.mods, Some(def.name.into()));
        m.max_hp *= 1.6;
        m.hp = m.max_hp;
        m.superu = s as u8 + 1;
        self.mobs.push(m);
        for j in 0..3 {
            let a = j as f32 * 2.1;
            let (bx, by) = (x + a.cos() * 1.6, y + a.sin() * 1.6);
            if !self.d.blocked(bx, by, 0.35) {
                self.mobs.push(Mob::new(Kind::Cultist, bx, by, self.tier, &mut self.rng));
            }
        }
    }

    fn place_tomb(&mut self) {
        if self.feats.tomb_spot != (0.0, 0.0) && self.feats.tomb_level == Some(self.level) {
            return;
        }
        let Some((x, y)) = self.mist_spot(20.0) else { return };
        self.mist_prop(PropKind::Tomb, x as i32 - 1, y as i32 - 1, 2, 2);
        self.feats.tomb_spot = (x + 0.5, y + 1.8);
        self.feats.tomb_level = Some(self.level);
        self.decals.push(Decal { x: x + 0.5, y: y + 0.5, r: 2.2, col: rgb(0x102030), a: 0.45 });
        // The shade waits at its grave, whole and angry.
        let (sx, sy) = self.feats.tomb_spot;
        let mut m = Mob::new(Kind::Shade, sx, sy + 0.8, self.tier * 1.5, &mut self.rng);
        m.max_hp *= 2.0;
        m.hp = m.max_hp;
        m.errand = 4;
        self.mobs.push(m);
    }

    // ------------------------------------------------------------------ every tick

    pub(crate) fn update_mist(&mut self) {
        self.update_moon();
        self.update_shade();
        self.update_bells();
        self.update_manor();
    }

    fn mist_wilds(&self) -> bool {
        self.level.act() == 2 && matches!(self.level, LevelId::Area(..)) && !self.in_safe(self.p.x, self.p.y)
    }

    fn update_moon(&mut self) {
        let wilds = self.mist_wilds();
        if self.feats.moon > 0.0 {
            self.feats.moon -= DT;
            if !wilds || self.feats.moon <= 0.0 {
                self.feats.moon = 0.0;
                self.light_ready = false;
                if wilds {
                    self.say("THE RED MOON SETS. THE WOLVES SLINK AWAY".into());
                }
            }
            return;
        }
        if !wilds || matches!(self.state, crate::game::State::Dead(_)) {
            return;
        }
        if self.feats.moon_cd <= 0.0 {
            self.feats.moon_cd = self.rng.rf(MOON_GAP.0, MOON_GAP.1);
        }
        self.feats.moon_cd -= DT;
        if self.feats.moon_cd <= 0.0 {
            self.start_moon();
        }
    }

    pub(crate) fn start_moon(&mut self) {
        self.feats.moon = MOON_TIME;
        self.feats.moon_cd = self.rng.rf(MOON_GAP.0, MOON_GAP.1);
        self.light_ready = false;
        self.say("THE WOLF MOON RISES! WEREWOLVES HUNT, AND EVERY PELT PAYS DOUBLE".into());
        for pack in 0..2 {
            let a = pack as f32 * 3.1 + self.rng.f();
            let (x, y) = (self.p.x + a.cos() * 10.0, self.p.y + a.sin() * 10.0);
            for k in 0..3 {
                let (mx, my) = (x + k as f32 * 0.7, y + (k % 2) as f32 * 0.7);
                if !self.d.blocked(mx, my, 0.4) {
                    let mut m = Mob::new(Kind::Werewolf, mx, my, self.tier, &mut self.rng);
                    m.state = MobState::Chase;
                    self.mobs.push(m);
                }
            }
        }
    }

    /// The light under the wolf moon (1 = none).
    pub fn moon_light(&self) -> f32 {
        if self.feats.moon > 0.0 {
            0.6
        } else {
            1.0
        }
    }

    fn update_shade(&mut self) {
        if self.feats.shade != 1 || self.level.act() != 2 || !matches!(self.level, LevelId::Area(..)) {
            return;
        }
        let at_tomb = self.feats.tomb_level == Some(self.level);
        let alive = self.mobs.iter().position(|m| m.kind == Kind::Shade && m.alive());
        match alive {
            Some(i) if !at_tomb => {
                // Wounded, it flees toward its grave.
                let m = &self.mobs[i];
                let (x, y) = (m.x, m.y);
                let far = (x - self.p.x).powi(2) + (y - self.p.y).powi(2) > 30.0 * 30.0;
                if m.hp < m.max_hp * SHADE_FLEE || far {
                    self.mobs[i].state = MobState::Dead(10.0);
                    for _ in 0..20 {
                        self.spray_at(x, y, PKind::Magic, 20.0);
                    }
                    if !far {
                        let tomb = tomb_area(self.world_seed, self.quest.difficulty);
                        let name = Game::waypoint_name(tomb);
                        self.feats.shade_known = true;
                        self.say(format!("THE SHADE SHRIEKS AND FLEES TOWARD {name}. ITS GRAVE MUST BE THERE"));
                    }
                    self.feats.shade_cd = self.rng.rf(SHADE_GAP.0, SHADE_GAP.1);
                }
            }
            None if !at_tomb => {
                if self.in_safe(self.p.x, self.p.y) || matches!(self.state, crate::game::State::Dead(_)) {
                    return;
                }
                self.feats.shade_cd -= DT;
                if self.feats.shade_cd <= 0.0 {
                    self.feats.shade_cd = self.rng.rf(SHADE_GAP.0, SHADE_GAP.1);
                    let a0 = self.rng.f() * std::f32::consts::TAU;
                    let spot = (0..12).map(|k| a0 + k as f32 * 0.52).flat_map(|a| [5.0f32, 7.0, 3.5].map(|r| (self.p.x + a.cos() * r, self.p.y + a.sin() * r))).find(|&(x, y)| !self.d.blocked(x, y, 0.4));
                    if let Some((x, y)) = spot {
                        let mut m = Mob::new(Kind::Shade, x, y, self.tier, &mut self.rng);
                        m.state = MobState::Chase;
                        self.mobs.push(m);
                        self.light_ready = false;
                        self.floater(x, y, "WHY... DID YOU... LEAVE ME...".into(), rgb(0xc0d8ff));
                    }
                }
            }
            _ => {}
        }
    }

    /// The shade died: at its grave it's at rest; anywhere else it only flees.
    pub(crate) fn shade_killed(&mut self, x: f32, y: f32) {
        if self.feats.tomb_level == Some(self.level) && self.feats.shade == 1 {
            self.feats.shade = 2;
            self.save_due = true;
            self.say("THE SHADE SINKS INTO ITS GRAVE WITH A SIGH. IT IS AT REST".into());
            if let Some(u) = crate::items::boss_unique("shade") {
                self.pickups.push(Pickup { x, y, kind: Drop::Item(Box::new(u)), t: 0.0 });
            }
            self.p.skills.points += 1;
            self.floater(x, y, "+1 SKILL POINT".into(), rgb(0xffe080));
            self.gain_xp(200.0 * self.tier);
        }
    }

    fn update_bells(&mut self) {
        let Some(k) = BELLS.iter().position(|b| *b == self.level) else { return };
        if self.feats.bells & (1 << k) != 0 || self.feats.bell_spot[k] == (0.0, 0.0) {
            return;
        }
        let (bx, by) = self.feats.bell_spot[k];
        if (bx - self.p.x).powi(2) + (by - self.p.y).powi(2) > 1.8 * 1.8 {
            return;
        }
        let guarded = self.mobs.iter().any(|m| m.alive() && m.charm <= 0.0 && m.neutral == 0 && !crate::breakables::is_prop(m.kind) && (m.x - bx).powi(2) + (m.y - by).powi(2) < 64.0);
        if guarded {
            if self.tick % 120 == 0 {
                self.say("THE DEAD CROWD THE BELL. CLEAR THEM FIRST".into());
            }
            return;
        }
        self.feats.bells |= 1 << k;
        self.shake = 0.5;
        self.sfx.push(Sfx::Boom);
        for _ in 0..24 {
            self.spray_at(bx, by - 1.0, PKind::Magic, 30.0);
        }
        let n = self.feats.bells.count_ones();
        self.floater(bx, by - 1.5, "DONNNG...".into(), rgb(0xffe0a0));
        self.say(format!("THE BELL RINGS OUT OVER THE MIST ({n}/3)"));
        if n >= 3 {
            self.side_progress(crate::side::Goal::Bells);
        }
        self.save_due = true;
    }

    fn update_manor(&mut self) {
        if self.level != LevelId::Dungeon(crate::world::MANOR, 0) {
            return;
        }
        let Some(i) = self.mobs.iter().position(|m| m.kind == Kind::Elspeth && m.alive()) else { return };
        if self.feats.keepsakes < 3 {
            // Untouchable while her keepsakes are lost.
            self.mobs[i].invuln = 0.5;
        } else {
            let (x, y) = (self.mobs[i].x, self.mobs[i].y);
            self.mobs[i].invuln = 0.0;
            self.mobs[i].stun = self.mobs[i].stun.max(0.3);
            if (x - self.p.x).powi(2) + (y - self.p.y).powi(2) < 2.6 * 2.6 {
                self.floater(x, y - 1.0, "MY THINGS... MY LITTLE ONE... THANK YOU".into(), rgb(0xd0e8ff));
                self.say("LADY ELSPETH TAKES HER KEEPSAKES AND FADES. THE MANOR FALLS SILENT".into());
                self.kill(i);
            }
        }
    }

    /// Picked up one of Lady Elspeth's keepsakes.
    pub(crate) fn pick_keepsake(&mut self, k: u8) {
        self.feats.keepsakes = (self.feats.keepsakes + 1).min(3);
        let (x, y) = (self.p.x, self.p.y);
        self.floater(x, y, KEEPSAKES[k as usize % 3].into(), rgb(0xd0e8ff));
        if self.feats.keepsakes >= 3 {
            self.say("ALL THREE KEEPSAKES. BRING THEM TO LADY ELSPETH'S GHOST".into());
        } else {
            self.say(format!("ONE OF LADY ELSPETH'S KEEPSAKES ({}/3)", self.feats.keepsakes));
        }
    }

    /// A bride fell: with all three gone before the Count, his wedding band.
    pub(crate) fn bride_killed(&mut self, superu: u8, x: f32, y: f32) {
        let Some(b) = crate::side::BRIDES.iter().position(|&s| s + 1 == superu as usize) else { return };
        self.feats.brides |= 1 << b;
        self.save_due = true;
        let n = self.feats.brides.count_ones();
        if n >= 3 && self.quest.stage3 < 3 {
            if let Some(u) = crate::items::boss_unique("vardakring") {
                self.pickups.push(Pickup { x, y, kind: Drop::Item(Box::new(u)), t: 0.0 });
            }
            self.say("THE LAST OF VARDAK'S BRIDES. HER HAND STILL WEARS HIS WEDDING BAND".into());
        } else if n < 3 {
            self.say(format!("ONE OF COUNT VARDAK'S BRIDES IS DEAD ({n}/3)"));
        }
    }

    /// The witch's pacts.
    pub(crate) fn witch_dialog(&self) -> Dialog {
        let mut d = Dialog::new(
            "THE BOG WITCH",
            &["WELL, WELL. A HERO, KNEE-DEEP IN MY BOG. I DEAL IN BARGAINS, DEARIE: SOMETHING FOR YOU, SOMETHING FOR ME. EVERY BARGAIN HAS A PRICE, AND EVERY PRICE IS FOREVER. CHOOSE."],
        );
        d.options = PACTS
            .iter()
            .enumerate()
            .filter(|(_, p)| self.p.pacts & p.0 == 0)
            .map(|(k, p)| (format!("{}: {}", p.1, p.2), Act::Pact(k as u8)))
            .collect();
        d.options.push(("NOT TODAY, WITCH".into(), Act::Close));
        d
    }

    pub(crate) fn make_pact(&mut self, k: usize) {
        let Some(&(bit, name, _)) = PACTS.get(k) else { return };
        if self.p.pacts & bit != 0 {
            return;
        }
        self.p.pacts |= bit;
        if bit == PACT_TOAD {
            self.p.gold += 1500;
        }
        self.p.recalc();
        self.p.hp = self.p.hp.min(self.p.max_hp);
        self.sfx.push(Sfx::Descend);
        self.say(format!("THE PACT IS STRUCK: {name}. THE WITCH CACKLES"));
        self.save_due = true;
        self.dialog = Some(self.witch_dialog());
    }

    /// Pacts and the wolf moon on damage dealt, life, hunger and gold.
    pub fn pact_damage(&self) -> f32 {
        if self.p.pacts & PACT_WOLF != 0 {
            1.15
        } else {
            1.0
        }
    }

    pub fn pact_gold(&self) -> f32 {
        if self.p.pacts & PACT_TOAD != 0 {
            0.8
        } else {
            1.0
        }
    }

    pub fn pact_hunger(&self) -> f32 {
        if self.p.pacts & PACT_CROW != 0 {
            1.6
        } else {
            1.0
        }
    }

    /// A werewolf fell under the wolf moon: its pelt's bounty.
    pub(crate) fn moon_kill(&mut self, kind: Kind, x: f32, y: f32) {
        if self.feats.moon > 0.0 && kind == Kind::Werewolf {
            let g = (12.0 + 8.0 * self.tier) as i32 * 2;
            self.pickups.push(Pickup { x, y: y + 0.3, kind: Drop::Gold(g), t: 0.0 });
        }
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
    fn the_bog_witch_strikes_pacts_with_a_price() {
        let mut g = at(WITCH);
        assert!(g.npcs.iter().any(|n| n.role == Role::BogWitch));
        let hp = g.p.max_hp;
        g.make_pact(0);
        assert!(g.pact_damage() > 1.0 && g.p.max_hp < hp, "the wolf's heart");
        let gold = g.p.gold;
        g.make_pact(2);
        assert_eq!(g.p.gold, gold + 1500);
        assert!(g.pact_gold() < 1.0);
        let skills = g.p.skills.bonus;
        g.make_pact(1);
        assert!(g.p.skills.bonus > skills && g.pact_hunger() > 1.0);
        assert_eq!(g.witch_dialog().options.len(), 1, "nothing left to bargain");
    }

    #[test]
    fn the_wolf_moon_rises_and_pays_for_pelts() {
        let mut g = at(LevelId::Area(2, 1));
        g.start_moon();
        assert!(g.moon_light() < 1.0);
        assert!(g.mobs.iter().filter(|m| m.kind == Kind::Werewolf && m.state == MobState::Chase).count() >= 3);
        let n = g.pickups.len();
        g.moon_kill(Kind::Werewolf, g.p.x, g.p.y);
        assert!(g.pickups.len() > n);
    }

    #[test]
    fn the_brides_and_the_shades_grave_move_between_playthroughs() {
        let a = bride_areas(7, 0);
        let b = bride_areas(8, 0);
        let c = bride_areas(7, 1);
        assert!(a != b || a != c);
        assert!(a.iter().all(|n| MIST_AREAS.contains(n)) && a[0] != a[1] && a[1] != a[2] && a[0] != a[2]);
        assert_ne!(tomb_area(7, 0), LevelId::Area(2, 1));
    }

    #[test]
    fn three_brides_before_the_count_give_his_ring() {
        let mut g = at(LevelId::Area(2, 1));
        for b in 0..3 {
            g.bride_killed(crate::side::BRIDES[b] as u8 + 1, g.p.x, g.p.y);
        }
        assert_eq!(g.feats.brides, 7);
        assert!(g.pickups.iter().any(|k| matches!(&k.kind, Drop::Item(it) if it.name == "VARDAK'S WEDDING BAND")));
    }

    #[test]
    fn the_shade_haunts_you_flees_and_rests_only_at_its_grave() {
        let mut g = at(LevelId::Area(2, 1));
        assert_eq!(g.feats.shade, 1, "it found you");
        g.feats.shade_cd = 0.0;
        g.update_shade();
        let i = g.mobs.iter().position(|m| m.kind == Kind::Shade && m.alive()).expect("it appears");
        g.mobs[i].hp = g.mobs[i].max_hp * 0.3;
        g.update_shade();
        assert!(!g.mobs[i].alive() && g.feats.shade_known, "it fled");
        let tomb = tomb_area(g.world_seed, 0);
        g.debug_goto(tomb);
        let s = g.mobs.iter().position(|m| m.kind == Kind::Shade && m.alive()).expect("waiting at its grave");
        let pts = g.p.skills.points;
        g.kill(s);
        assert_eq!(g.feats.shade, 2);
        assert!(g.p.skills.points > pts, "a skill point (and the experience levels you up besides)");
    }

    #[test]
    fn bells_ring_once_their_guards_are_dead() {
        let mut g = at(BELLS[0]);
        let (bx, by) = g.feats.bell_spot[0];
        assert!(bx > 0.0);
        g.mobs.retain(|m| (m.x - bx).powi(2) + (m.y - by).powi(2) > 64.0);
        (g.p.x, g.p.y) = (bx, by);
        g.update_bells();
        assert_eq!(g.feats.bells, 1);
    }

    #[test]
    fn lady_elspeth_rests_once_her_keepsakes_are_found() {
        let mut g = at(LevelId::Dungeon(crate::world::MANOR, 0));
        assert_eq!(g.pickups.iter().filter(|k| matches!(k.kind, Drop::Keepsake(_))).count(), 3);
        let i = g.mobs.iter().position(|m| m.kind == Kind::Elspeth).expect("her ghost");
        g.update_manor();
        assert!(g.mobs[i].invuln > 0.0, "untouchable");
        for k in 0..3 {
            g.pick_keepsake(k);
        }
        (g.p.x, g.p.y) = (g.mobs[i].x + 1.0, g.mobs[i].y);
        g.update_manor();
        assert!(!g.mobs[i].alive(), "at rest");
    }
}

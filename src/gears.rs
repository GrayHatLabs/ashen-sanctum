//! Act 4's set pieces (docs/SIDE_CONTENT_PLAN.md 2c), the user's picks:
//!
//! - **The Broken Automaton**: five gears lie in the Dominion's five main areas. Find them all and a brass
//!   automaton knight whirs to life and fights beside you for the rest of the act (it walks with you between
//!   Act 4's levels, and is rebuilt if it falls).
//! - **The Unlawful Court** (the Piston Flats): a clockwork magistrate tries you for unlicensed heroism. Fight
//!   the court, bribe it, or argue your case (three questions). Bribed or acquitted, you get a writ: the
//!   laws of the Dominion no longer bind your blows.
//! - **The Scrapyard Lottery** (the Scrapyard): smash scrap heaps for whatever's inside: gold, gems, gear,
//!   rust... or a junk golem.
//! - **The Timeless Vault** (the Archive Stacks): a sealed vault whose door only opens while time is stopped:
//!   strike the clock by it and slip in; strike the one inside to get out again.
//!
//! The Junk Golem (the Scrapyard's lottery and the Scrapheap Labyrinth's boss) rebuilds itself from scrap twice.
use crate::clockwork::TimeClock;
use crate::dungeon::Tile;
use crate::game::{Drop, Game, PKind, Pickup, Sfx};
use crate::gfx::rgb;
use crate::mobs::{Kind, Mob, MobState, Rank};
use crate::story::{Act, Dialog, Npc, Role};
use crate::world::LevelId;

pub const COURT: LevelId = LevelId::Area(3, 2);
pub const SCRAPYARD: LevelId = LevelId::Area(3, 6);
pub const VAULT: LevelId = LevelId::Area(3, 4);
/// The automaton's gears: one in each of the Dominion's five main areas.
pub const GEAR_AREAS: [u8; 5] = [1, 2, 3, 4, 5];
/// The neutral group of the court's guards.
pub const G_COURT: u8 = 3;

/// The court's questions: (the charge, the answers; the first is the lawful one: they're shuffled in the talk).
pub const CHARGES: [(&str, [&str; 3]); 3] = [
    (
        "THE ACCUSED STRUCK DOWN A CITIZEN OF THE DOMINION. DOES THE ACCUSED DENY IT?",
        ["A SCARAB IS NO CITIZEN. YOUR OWN LEDGER LISTS IT AS PROPERTY.", "I DENY EVERYTHING.", "IT HAD IT COMING."],
    ),
    (
        "THE ACCUSED ENTERED THE DOMINION WITHOUT A PERMIT. WHAT SAYS THE ACCUSED?",
        ["THE GEAR GATE OPENED BY YOUR OWN LAW WHEN THE COUNT FELL. I WAS SUMMONED.", "PERMITS ARE FOR COWARDS.", "I'LL GET ONE LATER."],
    ),
    (
        "THE ACCUSED BROKE THE LAW OF STILLNESS. HOW DOES THE ACCUSED PLEAD?",
        ["THE LAW BINDS THE STRUCK, NOT THE STRIKER. READ CLAUSE NINE.", "I WAS STANDING PERFECTLY STILL!", "WHAT LAW?"],
    ),
];

impl Game {
    /// Act 4 set pieces as you arrive (and the automaton, if it's yours).
    pub(crate) fn gears_enter(&mut self) {
        let here = self.level;
        if here.act() != 3 {
            return;
        }
        // The vault's clocks come back every time (the level's clocks are rebuilt on arrival).
        if here == VAULT && self.feats.vault_door != (0, 0) {
            self.vault_clocks();
        }
        if self.feats.automaton && !self.mobs.iter().any(|m| m.kind == Kind::Automaton && m.alive()) {
            self.spawn_automaton();
        }
        if !matches!(here, LevelId::Area(..)) || self.feats.gear_placed.contains(&here) {
            return;
        }
        self.feats.gear_placed.push(here);
        if let LevelId::Area(_, n) = here {
            if let Some(k) = GEAR_AREAS.iter().position(|&a| a == n) {
                if self.feats.gears & (1 << k) == 0 && !self.feats.automaton {
                    self.place_gear(k);
                }
            }
        }
        if here == COURT && self.feats.court == 0 {
            self.place_court();
        }
        if here == SCRAPYARD {
            self.place_scrap();
        }
        if here == VAULT && !self.feats.vault_looted {
            self.place_vault();
        }
    }

    fn gear_spot(&mut self, min: f32, room: f32) -> Option<(f32, f32)> {
        let (sx, sy) = (self.p.x, self.p.y);
        for _ in 0..500 {
            let x = self.rng.range(8, self.d.w - 8) as f32 + 0.5;
            let y = self.rng.range(8, self.d.h - 8) as f32 + 0.5;
            if ((x - sx).powi(2) + (y - sy).powi(2)).sqrt() < min || self.d.blocked(x, y, room) {
                continue;
            }
            if self.portals.iter().any(|p| (p.x - x).powi(2) + (p.y - y).powi(2) < 100.0) {
                continue;
            }
            if self.d.path((sx as i32, sy as i32), (x as i32, y as i32), 40_000).is_some() {
                return Some((x, y));
            }
        }
        None
    }

    fn place_gear(&mut self, k: usize) {
        let Some((x, y)) = self.gear_spot(16.0, 0.8) else { return };
        self.pickups.push(Pickup { x, y, kind: Drop::Gear(k as u8), t: 1.0 });
        // A few scarabs nest on it.
        for j in 0..3 {
            let a = j as f32 * 2.1;
            let (mx, my) = (x + a.cos() * 1.6, y + a.sin() * 1.6);
            if !self.d.blocked(mx, my, 0.35) {
                self.mobs.push(Mob::new(Kind::Scarab, mx, my, self.tier, &mut self.rng));
            }
        }
    }

    /// Picked up one of the automaton's gears.
    pub(crate) fn pick_gear(&mut self, k: u8) {
        self.feats.gears |= 1 << k;
        let n = self.feats.gears.count_ones();
        let (x, y) = (self.p.x, self.p.y);
        self.floater(x, y, format!("AN AUTOMATON GEAR ({n}/5)"), rgb(0xe0c060));
        self.save_due = true;
        if n >= 5 && !self.feats.automaton {
            self.feats.automaton = true;
            self.say("THE FIFTH GEAR CLICKS HOME. A BRASS KNIGHT WHIRS TO LIFE AND BOWS TO YOU".into());
            self.sfx.push(Sfx::Descend);
            self.spawn_automaton();
        } else if n < 5 {
            self.say(format!("A GEAR FROM SOME GREAT MACHINE. {} MORE LIE IN THE DOMINION'S FIELDS", 5 - n));
        }
    }

    fn spawn_automaton(&mut self) {
        let (px, py) = (self.p.x, self.p.y);
        let spot = [(1.2, 0.0), (0.0, 1.2), (-1.2, 0.0), (0.0, -1.2)].into_iter().map(|(dx, dy)| (px + dx, py + dy)).find(|&(x, y)| !self.d.blocked(x, y, 0.4)).unwrap_or((px, py));
        let mut m = Mob::new(Kind::Automaton, spot.0, spot.1, self.tier.max(7.5), &mut self.rng);
        m.charm = 1.0e9;
        m.name = Some("THE BRASS KNIGHT".into());
        self.mobs.push(m);
        for _ in 0..12 {
            self.spray_at(spot.0, spot.1, PKind::Magic, 14.0);
        }
    }

    fn place_court(&mut self) {
        let Some((x, y)) = self.gear_spot(18.0, 3.0) else { return };
        self.npcs.push(Npc::new("MAGISTRATE KORVEL", Role::Magistrate, "npc_magistrate", x, y, 2));
        for j in 0..6 {
            let a = j as f32 / 6.0 * std::f32::consts::TAU;
            let (mx, my) = (x + a.cos() * 3.0, y + a.sin() * 3.0);
            if !self.d.blocked(mx, my, 0.35) {
                let kind = if j % 3 == 0 { Kind::Prism } else { Kind::Ordinal };
                let mut m = Mob::new(kind, mx, my, self.tier, &mut self.rng);
                m.neutral = G_COURT;
                self.mobs.push(m);
            }
        }
        self.laws.push(crate::clockwork::LawZone { x, y, law: crate::clockwork::Law::Stillness });
        self.feats.court_spot = (x, y);
    }

    fn place_scrap(&mut self) {
        for _ in 0..9 {
            if let Some((x, y)) = self.gear_spot(8.0, 0.6) {
                let m = Mob::new(Kind::ScrapPile, x, y, self.tier, &mut self.rng);
                self.mobs.push(m);
            }
        }
        self.say("A SCRAPYARD. SOMETHING VALUABLE MIGHT BE BURIED IN THOSE HEAPS... OR SOMETHING ELSE".into());
    }

    /// A sealed vault: walls round a treasure room, a door that only opens while one of its clocks is stopped.
    fn place_vault(&mut self) {
        let Some((x, y)) = self.gear_spot(20.0, 2.0).or_else(|| self.gear_spot(12.0, 1.0)) else { return };
        let (cx, cy) = (x as i32, y as i32);
        let (x0, y0, x1, y1) = (cx - 4, cy - 4, cx + 4, cy + 3);
        self.clear_props_in(x0 - 1, y0 - 1, x1 + 2, y1 + 4);
        for yy in y0..=y1 {
            for xx in x0..=x1 {
                let edge = xx == x0 || xx == x1 || yy == y0 || yy == y1;
                self.d.set(xx, yy, if edge { Tile::Wall } else { Tile::Floor });
            }
        }
        let door = (cx, y1);
        self.feats.vault_door = door;
        self.vault_clocks();
        // The treasure, and two wardens that never left.
        for (k, (dx, dy)) in [(-2.0, -2.0), (2.0, -2.0), (0.0, -2.5), (-2.5, 0.0), (2.5, 0.0)].into_iter().enumerate() {
            let g = ((60.0 + 40.0 * self.tier) * if k == 2 { 3.0 } else { 1.0 }) as i32;
            self.pickups.push(Pickup { x: cx as f32 + 0.5 + dx, y: cy as f32 + 0.5 + dy, kind: Drop::Gold(g), t: 1.0 });
        }
        let ilvl = crate::items::ilvl_for(self.tier) + 4;
        for dx in [-1.5f32, 1.5] {
            let it = crate::items::roll(ilvl, crate::items::Rarity::Rare, &mut self.rng);
            self.pickups.push(Pickup { x: cx as f32 + 0.5 + dx, y: cy as f32 - 0.5, kind: Drop::Item(Box::new(it)), t: 1.0 });
        }
        for dx in [-2.0f32, 2.0] {
            let mut m = Mob::new(Kind::Marshal, cx as f32 + 0.5 + dx, cy as f32 + 1.0, self.tier * 1.2, &mut self.rng);
            let mods = crate::mobs::roll_mods(1, &mut self.rng);
            m.promote(Rank::Champion, mods, None);
            self.mobs.push(m);
        }
    }

    fn clear_props_in(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) {
        let hit = |p: &crate::world::Prop| p.foot.0 < x1 && p.foot.0 + p.foot.2 > x0 && p.foot.1 < y1 && p.foot.1 + p.foot.3 > y0;
        let keep = |p: &crate::world::Prop| matches!(p.kind, crate::world::PropKind::Entrance(_) | crate::world::PropKind::Shrine(_));
        let gone: Vec<(i32, i32, i32, i32)> = self.props.iter().filter(|p| hit(p) && !keep(p)).map(|p| p.foot).collect();
        for (fx, fy, fw, fh) in gone {
            for yy in fy..fy + fh {
                for xx in fx..fx + fw {
                    self.d.set(xx, yy, Tile::Floor);
                }
            }
        }
        self.props.retain(|p| !hit(p) || keep(p));
    }

    fn vault_clocks(&mut self) {
        let (dx, dy) = self.feats.vault_door;
        for (x, y) in [(dx as f32 + 0.5, dy as f32 + 2.5), (dx as f32 + 0.5, dy as f32 - 1.6)] {
            if !self.clocks.iter().any(|c| (c.x - x).abs() < 0.1 && (c.y - y).abs() < 0.1) {
                self.clocks.push(TimeClock { x, y, field: 0.0, cool: 0.0 });
            }
        }
    }

    // ------------------------------------------------------------------ every tick

    pub(crate) fn update_gears(&mut self) {
        if self.level.act() != 3 {
            return;
        }
        // The automaton never tires.
        for m in self.mobs.iter_mut().filter(|m| m.kind == Kind::Automaton) {
            m.charm = 1.0e9;
        }
        if self.level == VAULT && self.feats.vault_door != (0, 0) {
            self.update_vault();
        }
        if self.level == COURT && self.feats.court == 0 && self.feats.court_spot != (0.0, 0.0) {
            let (x, y) = self.feats.court_spot;
            if (x - self.p.x).powi(2) + (y - self.p.y).powi(2) < 5.0 * 5.0 && self.dialog.is_none() && !self.feats.court_called {
                self.feats.court_called = true;
                self.dialog = Some(self.court_dialog());
            }
            // Hurting a guard is contempt of court.
            if self.mobs.iter().any(|m| m.neutral == 0 && m.kind != Kind::Automaton && matches!(m.kind, Kind::Ordinal | Kind::Prism) && m.hp < m.max_hp && (m.x - x).powi(2) + (m.y - y).powi(2) < 36.0) {
                self.court_fight("CONTEMPT OF COURT!");
            }
        }
    }

    fn update_vault(&mut self) {
        let (dx, dy) = self.feats.vault_door;
        let time_stopped = self.clocks.iter().any(|c| c.field > 0.0 && ((c.x - dx as f32 - 0.5).abs() < 0.2) && ((c.y - dy as f32 - 2.5).abs() < 0.2 || (c.y - dy as f32 + 1.6).abs() < 0.2));
        let on_door = (self.p.x - dx as f32 - 0.5).abs() < 0.9 && (self.p.y - dy as f32 - 0.5).abs() < 0.9;
        let open = time_stopped || on_door;
        let tile = if open { Tile::Floor } else { Tile::Wall };
        if self.d.get(dx, dy) != tile {
            self.d.set(dx, dy, tile);
            self.light_ready = false;
            if open {
                self.say("TIME STANDS STILL... AND THE VAULT DOOR SLIDES OPEN".into());
            }
        }
        // Inside: once the gold is gone, it's been looted.
        let (cx, cy) = (dx as f32 + 0.5, dy as f32 - 3.5);
        if !self.feats.vault_looted && (self.p.x - cx).abs() < 4.0 && (self.p.y - cy).abs() < 3.5 {
            let left = self.pickups.iter().any(|k| matches!(k.kind, Drop::Gold(_)) && (k.x - cx).abs() < 4.0 && (k.y - cy).abs() < 3.5);
            if !left {
                self.feats.vault_looted = true;
                self.save_due = true;
                self.say("THE TIMELESS VAULT IS EMPTY. STRIKE THE INNER CLOCK TO LEAVE".into());
            }
        }
    }

    // ------------------------------------------------------------------ the court

    pub(crate) fn court_dialog(&self) -> Dialog {
        let mut d = Dialog::new(
            "MAGISTRATE KORVEL",
            &["ORDER! THE ACCUSED WILL APPROACH THE BENCH. YOU STAND CHARGED WITH UNLICENSED HEROISM IN THE CLOCKWORK DOMINION. HOW DOES THE ACCUSED WISH TO PROCEED?"],
        );
        let fine = self.court_fine();
        d.options = vec![
            ("ARGUE MY CASE".into(), Act::Court(2)),
            (format!("PAY THE COURT'S FEES  {fine} GOLD"), Act::Court(1)),
            ("I DON'T RECOGNISE THIS COURT (FIGHT)".into(), Act::Court(0)),
        ];
        d
    }

    fn court_fine(&self) -> i32 {
        (200.0 + 60.0 * self.tier) as i32
    }

    fn charge_dialog(&self, q: usize) -> Dialog {
        let (charge, answers) = CHARGES[q];
        let mut d = Dialog::new("MAGISTRATE KORVEL", &[charge]);
        // The answers come in a different order for each charge (the lawful one isn't always first).
        let order: [usize; 3] = match (self.world_seed as usize + q) % 3 {
            0 => [1, 0, 2],
            1 => [2, 1, 0],
            _ => [0, 2, 1],
        };
        d.options = order.iter().map(|&a| (answers[a].to_string(), Act::Court(10 + q as u8 * 3 + a as u8))).collect();
        d
    }

    pub(crate) fn court_act(&mut self, code: u8) {
        match code {
            0 => {
                self.dialog = None;
                self.court_fight("THEN THE SENTENCE IS DEATH!");
            }
            1 => {
                let fine = self.court_fine();
                if self.p.gold < fine {
                    self.say("NOT ENOUGH GOLD. THE COURT IS NOT AMUSED".into());
                    return;
                }
                self.p.gold -= fine;
                self.court_writ("FEES PAID. THE COURT GRANTS YOU A WRIT OF PASSAGE. BE ABOUT YOUR BUSINESS");
            }
            2 => self.dialog = Some(self.charge_dialog(0)),
            _ => {
                let (q, a) = (((code - 10) / 3) as usize, (code - 10) % 3);
                if a != 0 {
                    self.dialog = None;
                    self.court_fight("GUILTY! GUARDS!");
                } else if q + 1 < CHARGES.len() {
                    self.dialog = Some(self.charge_dialog(q + 1));
                } else {
                    self.p.gold += 500;
                    self.court_writ("...THE COURT FINDS THE ACCUSED NOT GUILTY. COSTS TO THE CROWN: 500 GOLD. AND A WRIT OF PASSAGE");
                }
            }
        }
    }

    fn court_writ(&mut self, line: &str) {
        self.feats.court = 1;
        self.feats.writ = true;
        self.save_due = true;
        self.sfx.push(Sfx::Descend);
        let mut d = Dialog::new("MAGISTRATE KORVEL", &[line, "(WITH THE WRIT, THE LAWS OF THE DOMINION NO LONGER BIND YOUR BLOWS.)"]);
        d.refresh_options();
        self.dialog = Some(d);
    }

    fn court_fight(&mut self, line: &str) {
        if self.feats.court != 0 {
            return;
        }
        self.feats.court = 2;
        self.save_due = true;
        for m in self.mobs.iter_mut().filter(|m| m.neutral == G_COURT) {
            m.neutral = 0;
            m.state = MobState::Chase;
        }
        if let Some(i) = self.npcs.iter().position(|n| n.role == Role::Magistrate) {
            let (x, y) = (self.npcs[i].x, self.npcs[i].y);
            self.npcs.remove(i);
            self.floater(x, y, line.into(), rgb(0xffa040));
            let mut m = Mob::new(Kind::Inquisitor, x, y, self.tier * 1.4, &mut self.rng);
            let mods = crate::mobs::roll_mods(2, &mut self.rng);
            m.promote(Rank::Elite, mods, Some("MAGISTRATE KORVEL".into()));
            m.max_hp *= 2.0;
            m.hp = m.max_hp;
            m.state = MobState::Chase;
            self.mobs.push(m);
        }
    }

    // ------------------------------------------------------------------ scrap

    /// A scrap heap smashed: the lottery.
    pub(crate) fn scrap_lottery(&mut self, x: f32, y: f32) {
        let r = self.rng.f();
        let ilvl = crate::items::ilvl_for(self.tier) + 2;
        if r < 0.1 {
            let mut m = Mob::new(Kind::JunkGolem, x, y, self.tier, &mut self.rng);
            m.boss = false;
            m.max_hp *= 0.5;
            m.hp = m.max_hp;
            m.state = MobState::Chase;
            self.mobs.push(m);
            self.floater(x, y, "THE SCRAP STANDS UP!".into(), rgb(0xff8040));
            self.shake = 0.5;
        } else if r < 0.2 {
            self.floater(x, y, "JUST RUST".into(), rgb(0x8a6a50));
        } else if r < 0.6 {
            self.pickups.push(Pickup { x, y, kind: Drop::Gold((15.0 + 12.0 * self.tier) as i32), t: 0.0 });
        } else if r < 0.8 {
            let gem = crate::items::gem_item(crate::items::roll_gem(ilvl, &mut self.rng));
            self.pickups.push(Pickup { x, y, kind: Drop::Item(Box::new(gem)), t: 0.0 });
        } else {
            let rar = if self.rng.chance(0.35) { crate::items::Rarity::Rare } else { crate::items::Rarity::Magic };
            self.pickups.push(Pickup { x, y, kind: Drop::Item(Box::new(crate::items::roll(ilvl, rar, &mut self.rng))), t: 0.0 });
        }
        for _ in 0..10 {
            self.spray_at(x, y, PKind::Smoke, 12.0);
        }
    }

    /// The Junk Golem doesn't stay down: twice it pulls itself back together. Returns true if it rebuilt.
    pub(crate) fn golem_rebuilds(&mut self, i: usize) -> bool {
        let m = &mut self.mobs[i];
        if m.kind != Kind::JunkGolem || m.form >= 2 {
            return false;
        }
        m.form += 1;
        m.hp = m.max_hp * 0.6;
        m.state = MobState::Chase;
        m.stun = 1.2;
        let (x, y) = (m.x, m.y);
        self.floater(x, y, "THE SCRAP REASSEMBLES!".into(), rgb(0xffa040));
        self.shake = 0.6;
        self.sfx.push(Sfx::Boom);
        for _ in 0..20 {
            self.spray_at(x, y, PKind::Smoke, 20.0);
        }
        true
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
    fn five_gears_build_a_brass_knight_that_follows_you() {
        let mut g = at(LevelId::Area(3, 1));
        assert!(g.pickups.iter().any(|k| matches!(k.kind, Drop::Gear(0))), "a gear to find");
        for k in 0..5 {
            g.pick_gear(k);
        }
        assert!(g.feats.automaton);
        assert!(g.mobs.iter().any(|m| m.kind == Kind::Automaton && m.charm > 0.0), "on your side");
        g.debug_goto(LevelId::Area(3, 2));
        assert!(g.mobs.iter().any(|m| m.kind == Kind::Automaton), "it comes along");
    }

    #[test]
    fn the_court_can_be_argued_bribed_or_fought() {
        let mut g = at(COURT);
        assert!(g.npcs.iter().any(|n| n.role == Role::Magistrate));
        g.court_act(2);
        for q in 0..3u8 {
            g.court_act(10 + q * 3);
        }
        assert!(g.feats.writ, "acquitted");
        let mut g = at(COURT);
        g.p.gold = 100_000;
        g.court_act(1);
        assert!(g.feats.writ, "bribed");
        let mut g = at(COURT);
        g.court_act(2);
        g.court_act(11);
        assert_eq!(g.feats.court, 2, "a wrong answer: guilty");
        assert!(g.mobs.iter().any(|m| m.name.as_deref() == Some("MAGISTRATE KORVEL")));
    }

    #[test]
    fn the_writ_lifts_the_laws() {
        let mut g = at(LevelId::Area(3, 1));
        let i = g.mobs.iter().position(|m| m.alive()).unwrap();
        let (x, y) = (g.mobs[i].x, g.mobs[i].y);
        g.laws.push(crate::clockwork::LawZone { x, y, law: crate::clockwork::Law::Arrow });
        (g.p.x, g.p.y) = (x + 1.0, y);
        assert!(g.law_scale(i) < 1.0);
        g.feats.writ = true;
        assert_eq!(g.law_scale(i), 1.0);
    }

    #[test]
    fn scrap_heaps_pay_out_and_the_golem_rebuilds_twice() {
        let mut g = at(SCRAPYARD);
        assert!(g.mobs.iter().filter(|m| m.kind == Kind::ScrapPile).count() >= 5);
        let n = g.pickups.len() + g.mobs.len();
        for _ in 0..20 {
            let (x, y) = (g.p.x, g.p.y);
            g.scrap_lottery(x, y);
        }
        assert!(g.pickups.len() + g.mobs.len() > n);
        let mut m = Mob::new(Kind::JunkGolem, g.p.x + 3.0, g.p.y, 8.0, &mut g.rng);
        m.state = MobState::Chase;
        g.mobs.push(m);
        let i = g.mobs.len() - 1;
        g.kill(i);
        assert!(g.mobs[i].alive(), "back up once");
        g.kill(i);
        assert!(g.mobs[i].alive(), "twice");
        g.kill(i);
        assert!(!g.mobs[i].alive(), "then it stays down");
    }

    #[test]
    fn the_vault_opens_only_while_time_stands_still() {
        let mut g = at(VAULT);
        let (dx, dy) = g.feats.vault_door;
        assert!(dx > 0);
        g.update_gears();
        assert_eq!(g.d.get(dx, dy), Tile::Wall, "sealed");
        let c = g.clocks.iter().position(|c| (c.y - dy as f32 - 2.5).abs() < 0.2).unwrap();
        g.clocks[c].field = 5.0;
        g.update_gears();
        assert_eq!(g.d.get(dx, dy), Tile::Floor, "open while the clock is stopped");
        g.clocks[c].field = 0.0;
        g.update_gears();
        assert_eq!(g.d.get(dx, dy), Tile::Wall, "and shut again");
    }
}

//! Co-op, phase 1 (docs/ONLINE_PLAN.md): several heroes in one world. `Game` keeps the *active* hero in its
//! own fields (`p`, the bag, the missiles in flight, the class's summons...), so all the game code that
//! says `self.p` keeps working. The other heroes wait here in `HeroState`s; to run one, `swap_hero` trades
//! its state into `Game` and back out again. The world (the map, monsters, quests) is shared and never swaps.
//!
//! Heroes are numbered: 0 is the leader (the local player, or the host), 1.. are `Game::allies`. The leader
//! decides where the party goes; allies follow through every portal.
use crate::game::{Fireball, Game, Input, Player, State, Stats, DT};
use crate::world::LevelId;

/// Everything about one hero that isn't the shared world.
pub struct HeroState {
    pub p: Player,
    pub hero_name: String,
    pub waypoints: Vec<LevelId>,
    pub tp_return: Option<(LevelId, f32, f32)>,
    pub ex: crate::extras::Extras,
    pub stats: Stats,
    pub state: State,
    pub hardcore: bool,
    pub fallen: bool,
    pub level_up_t: f32,
    pub portal_cd: f32,
    pub portal_armed: bool,
    pub prev: Input,
    pub second_wind_t: f32,
    pub whip_combo: u8,
    pub whip_combo_t: f32,
    pub hurt_cursed: bool,
    pub blind_t: f32,
    pub pull: (f32, f32, f32),
    pub ice_casts: u32,
    pub last_safe: (f32, f32),
    pub law_here: Option<usize>,
    pub trail: std::collections::VecDeque<(f32, f32)>,
    // What this hero has in flight.
    pub balls: Vec<Fireball>,
    pub novas: Vec<crate::skills::Nova>,
    pub fire_walls: Vec<crate::skills::FireWallFx>,
    pub patches: Vec<crate::skills::FirePatch>,
    pub meteors: Vec<crate::skills::MeteorFx>,
    pub hydras: Vec<crate::skills::HydraFx>,
    pub bats: Vec<crate::vampire::BatFx>,
    pub ravens: Vec<crate::valkyrie::RavenFx>,
    pub javelins: Vec<crate::valkyrie::JavelinFx>,
    pub axes: Vec<crate::berserker::AxeFx>,
    pub souls: Vec<crate::reaper::SoulFx>,
    pub lanterns: Vec<crate::reaper::LanternFx>,
    pub chains_fx: Vec<crate::reaper::ChainsFx>,
    pub glasses: Vec<crate::reaper::GlassFx>,
    pub sweeps: Vec<crate::reaper::SweepFx>,
    pub clouds: Vec<crate::druid::CloudFx>,
    pub fungi: Vec<crate::druid::FungusFx>,
    pub vines: Vec<crate::druid::VineFx>,
    pub fields: Vec<crate::vampire::BloodField>,
    pub bombs: Vec<crate::inventor::BombFx>,
    pub arcs: Vec<crate::inventor::ArcFx>,
    pub turrets: Vec<crate::inventor::TurretFx>,
    pub spiders: Vec<crate::inventor::SpiderFx>,
    pub airships: Vec<crate::inventor::AirshipFx>,
    pub thrusts: Vec<(f32, f32, f32, f32, f32)>,
    pub links: Vec<crate::inquisitor::ChainLinkFx>,
    pub whips: Vec<crate::inquisitor::Whip>,
    pub binds: Vec<crate::inquisitor::BindFx>,
}

impl HeroState {
    /// A fresh hero, as `Game::new` starts one.
    pub fn new() -> Self {
        HeroState {
            p: Player::new(),
            hero_name: "HERO".into(),
            waypoints: vec![LevelId::Overworld],
            tp_return: None,
            ex: Default::default(),
            stats: Stats::default(),
            state: State::Playing,
            hardcore: false,
            fallen: false,
            level_up_t: 0.0,
            portal_cd: 0.0,
            portal_armed: true,
            prev: Input::default(),
            second_wind_t: 0.0,
            whip_combo: 0,
            whip_combo_t: 0.0,
            hurt_cursed: false,
            blind_t: 0.0,
            pull: (0.0, 0.0, 0.0),
            ice_casts: 0,
            last_safe: (0.0, 0.0),
            law_here: None,
            trail: Default::default(),
            balls: vec![],
            novas: vec![],
            fire_walls: vec![],
            patches: vec![],
            meteors: vec![],
            hydras: vec![],
            bats: vec![],
            ravens: vec![],
            javelins: vec![],
            axes: vec![],
            souls: vec![],
            lanterns: vec![],
            chains_fx: vec![],
            glasses: vec![],
            sweeps: vec![],
            clouds: vec![],
            fungi: vec![],
            vines: vec![],
            fields: vec![],
            bombs: vec![],
            arcs: vec![],
            turrets: vec![],
            spiders: vec![],
            airships: vec![],
            thrusts: vec![],
            links: vec![],
            whips: vec![],
            binds: vec![],
        }
    }
}

/// One hero beside the leader, and what it's doing this tick.
pub struct Ally {
    pub st: HeroState,
    /// This tick's input (from the network, or from a bot).
    pub inp: Input,
    /// Seconds until a fallen ally gets up again beside the leader.
    pub respawn: f32,
}

/// How near a kill a hero must be to share its experience and get their own drops (tiles).
pub const SHARE_R: f32 = 25.0;

/// How long a fallen ally lies there before getting up beside the leader (softcore).
pub const ALLY_RESPAWN: f32 = 6.0;

impl Game {
    /// Trades ally `k`'s hero into the active slot (or back out: the same call undoes it).
    pub fn swap_hero(&mut self, k: usize) {
        use std::mem::swap;
        let a = &mut self.allies[k].st;
        swap(&mut self.p, &mut a.p);
        swap(&mut self.hero_name, &mut a.hero_name);
        swap(&mut self.waypoints, &mut a.waypoints);
        swap(&mut self.tp_return, &mut a.tp_return);
        swap(&mut self.ex, &mut a.ex);
        swap(&mut self.stats, &mut a.stats);
        swap(&mut self.state, &mut a.state);
        swap(&mut self.hardcore, &mut a.hardcore);
        swap(&mut self.fallen, &mut a.fallen);
        swap(&mut self.level_up_t, &mut a.level_up_t);
        swap(&mut self.portal_cd, &mut a.portal_cd);
        swap(&mut self.portal_armed, &mut a.portal_armed);
        swap(&mut self.prev, &mut a.prev);
        swap(&mut self.second_wind_t, &mut a.second_wind_t);
        swap(&mut self.whip_combo, &mut a.whip_combo);
        swap(&mut self.whip_combo_t, &mut a.whip_combo_t);
        swap(&mut self.hurt_cursed, &mut a.hurt_cursed);
        swap(&mut self.blind_t, &mut a.blind_t);
        swap(&mut self.pull, &mut a.pull);
        swap(&mut self.ice_casts, &mut a.ice_casts);
        swap(&mut self.last_safe, &mut a.last_safe);
        swap(&mut self.law_here, &mut a.law_here);
        swap(&mut self.trail, &mut a.trail);
        swap(&mut self.balls, &mut a.balls);
        swap(&mut self.novas, &mut a.novas);
        swap(&mut self.fire_walls, &mut a.fire_walls);
        swap(&mut self.patches, &mut a.patches);
        swap(&mut self.meteors, &mut a.meteors);
        swap(&mut self.hydras, &mut a.hydras);
        swap(&mut self.bats, &mut a.bats);
        swap(&mut self.ravens, &mut a.ravens);
        swap(&mut self.javelins, &mut a.javelins);
        swap(&mut self.axes, &mut a.axes);
        swap(&mut self.souls, &mut a.souls);
        swap(&mut self.lanterns, &mut a.lanterns);
        swap(&mut self.chains_fx, &mut a.chains_fx);
        swap(&mut self.glasses, &mut a.glasses);
        swap(&mut self.sweeps, &mut a.sweeps);
        swap(&mut self.clouds, &mut a.clouds);
        swap(&mut self.fungi, &mut a.fungi);
        swap(&mut self.vines, &mut a.vines);
        swap(&mut self.fields, &mut a.fields);
        swap(&mut self.bombs, &mut a.bombs);
        swap(&mut self.arcs, &mut a.arcs);
        swap(&mut self.turrets, &mut a.turrets);
        swap(&mut self.spiders, &mut a.spiders);
        swap(&mut self.airships, &mut a.airships);
        swap(&mut self.thrusts, &mut a.thrusts);
        swap(&mut self.links, &mut a.links);
        swap(&mut self.whips, &mut a.whips);
        swap(&mut self.binds, &mut a.binds);
        self.cur_hero = if self.cur_hero == 0 { k + 1 } else { 0 };
    }

    /// Runs `f` with ally `k` as the active hero.
    pub fn as_hero<R>(&mut self, k: usize, f: impl FnOnce(&mut Game) -> R) -> R {
        self.swap_hero(k);
        let r = f(self);
        self.swap_hero(k);
        r
    }

    /// A new ally of this class, standing beside the leader. Returns its index in `allies`.
    pub fn add_ally(&mut self, class: crate::skills::Class, name: &str) -> usize {
        let (lx, ly) = (self.p.x, self.p.y);
        self.allies.push(Ally { st: HeroState::new(), inp: Input::default(), respawn: 0.0 });
        let k = self.allies.len() - 1;
        let spot = self.ally_spot(lx, ly, k);
        self.as_hero(k, |g| {
            g.set_class(class);
            g.hero_name = name.to_string();
            (g.p.x, g.p.y) = spot;
        });
        k
    }

    /// Open ground near (x, y) for ally `k` (they fan out round the leader).
    fn ally_spot(&self, x: f32, y: f32, k: usize) -> (f32, f32) {
        for r in [1.2f32, 2.0, 3.0, 4.5] {
            for j in 0..8 {
                let a = (k as f32 * 0.9 + j as f32) / 8.0 * std::f32::consts::TAU;
                let (sx, sy) = (x + a.cos() * r, y + a.sin() * r);
                if !self.d.blocked(sx, sy, crate::game::PLAYER_R + 0.05) {
                    return (sx, sy);
                }
            }
        }
        (x, y)
    }

    /// Runs `f` for every hero but the active one (each in turn in the active slot), then puts things back.
    pub fn for_other_heroes(&mut self, f: impl Fn(&mut Game)) {
        let cur = self.cur_hero;
        if cur != 0 {
            // The leader back in the active slot first.
            self.swap_hero(cur - 1);
            f(self);
        }
        for k in 0..self.allies.len() {
            if k + 1 != cur {
                self.as_hero(k, &f);
            }
        }
        if cur != 0 {
            self.swap_hero(cur - 1);
        }
    }

    /// Players the game pretends are here beyond the real ones (the Players setting less the party), for the
    /// drop chance: real heroes get drops of their own instead.
    pub fn pretend_players(&self) -> u8 {
        crate::binds::players().saturating_sub(self.allies.len() as u8).max(1)
    }

    /// Where every hero stands: (x, y, alive). Index 0 is the active hero.
    pub fn hero_spots(&self) -> Vec<(f32, f32, bool)> {
        let mut v = vec![(self.p.x, self.p.y, matches!(self.state, State::Playing | State::Victory(_)))];
        v.extend(self.allies.iter().map(|a| (a.st.p.x, a.st.p.y, matches!(a.st.state, State::Playing | State::Victory(_)))));
        v
    }

    /// One tick for every ally, after the leader's (Game::update): each runs its own input, skills and
    /// missiles against the shared world. Allies don't travel (the leader does, and brings them), and their
    /// words don't cover the leader's screen.
    pub fn update_allies(&mut self) {
        for k in 0..self.allies.len() {
            let inp = std::mem::take(&mut self.allies[k].inp);
            let (lx, ly) = (self.p.x, self.p.y);
            let keep = (self.message.take(), self.notice.take(), self.pending_travel.take(), self.dialog.take());
            let spot = self.ally_spot(lx, ly, k);
            let mut respawn = self.allies[k].respawn;
            self.as_hero(k, |g| {
                g.level_up_t = (g.level_up_t - DT).max(0.0);
                g.portal_cd = (g.portal_cd - DT).max(0.0);
                g.second_wind_t = (g.second_wind_t - DT).max(0.0);
                match g.state {
                    State::Dead(t) => {
                        g.state = State::Dead(t + DT);
                        respawn -= DT;
                        if respawn <= 0.0 && !g.hardcore {
                            // Up again beside the leader, as fresh as a town wake.
                            g.state = State::Playing;
                            (g.p.x, g.p.y) = spot;
                            g.p.hp = g.p.max_hp;
                            g.p.path.clear();
                        }
                    }
                    _ => {
                        respawn = ALLY_RESPAWN;
                        let pressed = |now: bool, before: bool| now && !before;
                        let (p_cast, p_lmb) = (pressed(inp.cast, g.prev.cast), pressed(inp.lmb, g.prev.lmb));
                        g.update_player(&inp, p_cast, p_lmb);
                    }
                }
                g.trail.push_back((g.p.x, g.p.y));
                if g.trail.len() > 180 {
                    g.trail.pop_front();
                }
                g.update_hero_fx();
                g.dialog = None;
                g.pending_travel = None;
                g.prev = inp;
            });
            self.allies[k].respawn = respawn;
            (self.message, self.notice, self.pending_travel, self.dialog) = keep;
        }
    }

    /// For drawing: every ally's missiles and effects join the active hero's lists (so one pass draws them all);
    /// `split_ally_fx` hands them back. Returns where each ally's began.
    pub(crate) fn merge_ally_fx(&mut self) -> Vec<Vec<usize>> {
        let mut all = vec![];
        for k in 0..self.allies.len() {
            let mut lens = vec![];
            let a = &mut self.allies[k].st;
            lens.push(self.balls.len());
            self.balls.append(&mut a.balls);
            lens.push(self.novas.len());
            self.novas.append(&mut a.novas);
            lens.push(self.fire_walls.len());
            self.fire_walls.append(&mut a.fire_walls);
            lens.push(self.patches.len());
            self.patches.append(&mut a.patches);
            lens.push(self.meteors.len());
            self.meteors.append(&mut a.meteors);
            lens.push(self.hydras.len());
            self.hydras.append(&mut a.hydras);
            lens.push(self.bats.len());
            self.bats.append(&mut a.bats);
            lens.push(self.ravens.len());
            self.ravens.append(&mut a.ravens);
            lens.push(self.javelins.len());
            self.javelins.append(&mut a.javelins);
            lens.push(self.axes.len());
            self.axes.append(&mut a.axes);
            lens.push(self.souls.len());
            self.souls.append(&mut a.souls);
            lens.push(self.lanterns.len());
            self.lanterns.append(&mut a.lanterns);
            lens.push(self.chains_fx.len());
            self.chains_fx.append(&mut a.chains_fx);
            lens.push(self.glasses.len());
            self.glasses.append(&mut a.glasses);
            lens.push(self.sweeps.len());
            self.sweeps.append(&mut a.sweeps);
            lens.push(self.clouds.len());
            self.clouds.append(&mut a.clouds);
            lens.push(self.fungi.len());
            self.fungi.append(&mut a.fungi);
            lens.push(self.vines.len());
            self.vines.append(&mut a.vines);
            lens.push(self.fields.len());
            self.fields.append(&mut a.fields);
            lens.push(self.bombs.len());
            self.bombs.append(&mut a.bombs);
            lens.push(self.arcs.len());
            self.arcs.append(&mut a.arcs);
            lens.push(self.turrets.len());
            self.turrets.append(&mut a.turrets);
            lens.push(self.spiders.len());
            self.spiders.append(&mut a.spiders);
            lens.push(self.airships.len());
            self.airships.append(&mut a.airships);
            lens.push(self.thrusts.len());
            self.thrusts.append(&mut a.thrusts);
            lens.push(self.links.len());
            self.links.append(&mut a.links);
            lens.push(self.whips.len());
            self.whips.append(&mut a.whips);
            lens.push(self.binds.len());
            self.binds.append(&mut a.binds);
            all.push(lens);
        }
        all
    }

    pub(crate) fn split_ally_fx(&mut self, all: Vec<Vec<usize>>) {
        for (k, lens) in all.into_iter().enumerate().rev() {
            let a = &mut self.allies[k].st;
            a.balls = self.balls.split_off(lens[0]);
            a.novas = self.novas.split_off(lens[1]);
            a.fire_walls = self.fire_walls.split_off(lens[2]);
            a.patches = self.patches.split_off(lens[3]);
            a.meteors = self.meteors.split_off(lens[4]);
            a.hydras = self.hydras.split_off(lens[5]);
            a.bats = self.bats.split_off(lens[6]);
            a.ravens = self.ravens.split_off(lens[7]);
            a.javelins = self.javelins.split_off(lens[8]);
            a.axes = self.axes.split_off(lens[9]);
            a.souls = self.souls.split_off(lens[10]);
            a.lanterns = self.lanterns.split_off(lens[11]);
            a.chains_fx = self.chains_fx.split_off(lens[12]);
            a.glasses = self.glasses.split_off(lens[13]);
            a.sweeps = self.sweeps.split_off(lens[14]);
            a.clouds = self.clouds.split_off(lens[15]);
            a.fungi = self.fungi.split_off(lens[16]);
            a.vines = self.vines.split_off(lens[17]);
            a.fields = self.fields.split_off(lens[18]);
            a.bombs = self.bombs.split_off(lens[19]);
            a.arcs = self.arcs.split_off(lens[20]);
            a.turrets = self.turrets.split_off(lens[21]);
            a.spiders = self.spiders.split_off(lens[22]);
            a.airships = self.airships.split_off(lens[23]);
            a.thrusts = self.thrusts.split_off(lens[24]);
            a.links = self.links.split_off(lens[25]);
            a.whips = self.whips.split_off(lens[26]);
            a.binds = self.binds.split_off(lens[27]);
        }
    }

    /// What the active hero had in flight goes (it stays on the old map); the spider follows.
    pub(crate) fn clear_hero_fx(&mut self) {
        self.balls.clear();
        self.novas.clear();
        self.fire_walls.clear();
        self.patches.clear();
        self.meteors.clear();
        self.hydras.clear();
        self.bats.clear();
        self.ravens.clear();
        self.javelins.clear();
        self.axes.clear();
        self.souls.clear();
        self.lanterns.clear();
        self.chains_fx.clear();
        self.glasses.clear();
        self.sweeps.clear();
        self.clouds.clear();
        self.fungi.clear();
        self.vines.clear();
        self.fields.clear();
        self.bombs.clear();
        self.arcs.clear();
        self.turrets.clear();
        self.airships.clear();
        self.thrusts.clear();
        self.links.clear();
        self.whips.clear();
        self.binds.clear();
        let (x, y) = (self.p.x, self.p.y);
        for s in self.spiders.iter_mut() {
            (s.x, s.y) = (x, y);
        }
    }

    /// After the leader arrives somewhere new: the allies come too, round the leader.
    pub fn bring_allies(&mut self) {
        let (lx, ly) = (self.p.x, self.p.y);
        for k in 0..self.allies.len() {
            let spot = self.ally_spot(lx, ly, k);
            self.as_hero(k, |g| {
                (g.p.x, g.p.y) = spot;
                g.p.path.clear();
                g.trail.clear();
                // What was in flight stays behind on the old map.
                g.clear_hero_fx();
            });
        }
    }
}

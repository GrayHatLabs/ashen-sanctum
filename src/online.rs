//! Online play in the game (docs/ONLINE_PLAN.md, phase 2): a hosting session (the world runs here; each
//! joiner is an ally whose input comes over the network) and a joining session (the world comes from the
//! host; your controls go to it). The connection code is in `net`.
use crate::game::{Game, Input, State};
use crate::net::client::Link;
use crate::net::host::{Event, Host};
use crate::net::proto::{NetInput, ToClient, ToHost};

/// Snapshots go out every this many ticks (20 a second).
const SNAP_EVERY: u32 = 3;
/// Joiners get their hero back (for their own save) this often (ticks).
const SAVE_EVERY: u32 = 60 * 20;
/// Bad inputs tolerated from one joiner before it's dropped.
const MAX_STRIKES: u32 = 30;

/// Is this item one the game knows how to show? (Items arrive over the network too.)
pub fn item_ok(it: &crate::items::Item) -> bool {
    it.base < crate::items::BASES.len()
        && it.stats.len() <= 16
        && it.name.len() <= 48
        && it.sockets <= 6
        && it.gems.len() <= it.sockets as usize
        && it.gems.iter().all(|g| g.kind <= 6 && (1..=crate::items::TOP_GRADE).contains(&g.grade))
}

/// Is this hero one the game can safely hold: sane numbers, a bag and stash of the right size, known items?
pub fn hero_ok(p: &crate::game::Player) -> bool {
    let nums = [p.x, p.y, p.hp, p.max_hp, p.mana, p.max_mana, p.base_hp, p.base_mana, p.power, p.xp, p.food];
    nums.iter().all(|v| v.is_finite())
        && (1..=99).contains(&p.clvl)
        && p.power > 0.0
        && p.power < 2000.0
        && p.base_hp < 5000.0
        && p.base_mana < 5000.0
        && p.gear.bag.len() == crate::items::BAG
        && p.gear.stash.len() == crate::items::STASH
        && p.gear.worn.iter().chain(p.gear.bag.iter()).chain(p.gear.stash.iter()).flatten().all(item_ok)
}

/// Hosting: who's joined, as which ally.
pub struct Hosting {
    pub host: Host,
    /// The connection behind each ally (same order as `Game::allies`), its name and its bad inputs.
    peers: Vec<(u32, String, u32)>,
    sent_level: Option<crate::world::LevelId>,
    /// The Players setting before anyone joined (put back when hosting ends).
    players_before: u8,
}

impl Hosting {
    pub fn start(port: u16, password: String) -> std::io::Result<Hosting> {
        let host = Host::start(port, password, &crate::save::dir())?;
        Ok(Hosting { host, peers: vec![], sent_level: None, players_before: crate::binds::players() })
    }

    fn slot_of(&self, peer: u32) -> Option<usize> {
        self.peers.iter().position(|p| p.0 == peer)
    }

    /// Everyone but `except` hears it.
    fn tell_all(&self, m: &ToClient, except: Option<u32>) {
        for p in &self.peers {
            if Some(p.0) != except {
                self.host.send(p.0, m);
            }
        }
    }

    /// The party's size sets how tough the monsters are (at least the Players setting).
    fn rescale(&self) {
        crate::binds::set_players_quiet(self.players_before.max(1 + self.peers.len() as u8));
    }

    /// Before the game's tick: what the joiners did.
    pub fn before(&mut self, g: &mut Game) {
        for e in self.host.poll() {
            match e {
                Event::Hello { peer, name, hero } => match g.add_ally_from_save(&hero, &name) {
                    Ok(k) => {
                        debug_assert_eq!(k, self.peers.len());
                        self.peers.push((peer, name.clone(), 0));
                        self.host.send(peer, &ToClient::Welcome { you: k as u8 + 1, host_name: g.hero_name.clone() });
                        if let Some(lv) = g.net_level() {
                            self.host.send(peer, &ToClient::Level(lv));
                        }
                        self.tell_all(&ToClient::Chat { from: String::new(), text: format!("{name} JOINED") }, Some(peer));
                        g.say(format!("{name} JOINED THE GAME"));
                        self.rescale();
                    }
                    Err(why) => self.host.kick(peer, why),
                },
                Event::Input { peer, input } => {
                    let Some(k) = self.slot_of(peer) else { continue };
                    let hero = (g.allies[k].st.p.x, g.allies[k].st.p.y);
                    match input.check(hero) {
                        Ok((inp, target)) => {
                            g.allies[k].inp = inp;
                            g.allies[k].target = target;
                        }
                        Err(_) => {
                            self.peers[k].2 += 1;
                            if self.peers[k].2 > MAX_STRIKES {
                                self.host.kick(peer, "BAD INPUT");
                            }
                        }
                    }
                }
                Event::Chat { peer, text } => {
                    let Some(k) = self.slot_of(peer) else { continue };
                    let from = self.peers[k].1.clone();
                    self.tell_all(&ToClient::Chat { from: from.clone(), text: text.clone() }, None);
                    g.say(format!("{from}: {text}"));
                }
                Event::Gone { peer, why } => {
                    let Some(k) = self.slot_of(peer) else { continue };
                    let (_, name, _) = self.peers.remove(k);
                    g.remove_ally(k);
                    // The others' numbers shift down past the one who left.
                    for (j, p) in self.peers.iter().enumerate().skip(k) {
                        self.host.send(p.0, &ToClient::Welcome { you: j as u8 + 1, host_name: g.hero_name.clone() });
                    }
                    self.tell_all(&ToClient::Chat { from: String::new(), text: format!("{name} {why}") }, None);
                    g.say(format!("{name} {why}"));
                    self.rescale();
                }
            }
        }
    }

    /// After the game's tick: the level (when it changed), snapshots, and now and then each joiner's hero.
    pub fn after(&mut self, g: &mut Game) {
        if self.peers.is_empty() {
            return;
        }
        if self.sent_level != Some(g.level) {
            self.sent_level = Some(g.level);
            if let Some(lv) = g.net_level() {
                self.tell_all(&ToClient::Level(lv), None);
            }
        }
        if g.tick % SNAP_EVERY == 0 {
            for (k, p) in self.peers.iter().enumerate() {
                if let Some(s) = g.net_snapshot(k as u8 + 1) {
                    self.host.send(p.0, &ToClient::Snap(s));
                }
            }
        }
        if g.tick % SAVE_EVERY == 0 {
            for (k, p) in self.peers.iter().enumerate() {
                let text = g.as_hero(k, |g| crate::save::to_text(g));
                self.host.send(p.0, &ToClient::Hero(text));
            }
        }
    }
}

impl Drop for Hosting {
    fn drop(&mut self) {
        crate::binds::set_players_quiet(self.players_before);
    }
}

/// Joined to someone else's game.
pub struct Joined {
    pub link: Link,
    pub me: u8,
    seq: u32,
    has_level: bool,
    /// The latest copy of your hero the host sent back (merged into your save when you leave).
    pub hero: Option<String>,
}

impl Joined {
    pub fn new(link: Link) -> Joined {
        let me = link.you;
        Joined { link, me, seq: 0, has_level: false, hero: None }
    }

    /// One frame: take in what the host sent, show it, and send your controls. Returns why it ended, if it did.
    pub fn frame(&mut self, g: &mut Game, inp: &Input) -> Option<String> {
        for m in self.link.poll() {
            match m {
                ToClient::Level(b) => {
                    if !g.apply_level(&b) {
                        return Some("THE HOST SENT A BAD LEVEL".into());
                    }
                    self.has_level = true;
                }
                ToClient::Snap(b) => {
                    if self.has_level && !g.apply_snapshot(&b, self.me) {
                        return Some("THE HOST SENT BAD DATA".into());
                    }
                    if !hero_ok(&g.p) || g.allies.iter().any(|a| !hero_ok(&a.st.p)) || g.pickups.iter().any(|k| matches!(&k.kind, crate::game::Drop::Item(it) if !item_ok(it))) {
                        return Some("THE HOST SENT BAD DATA".into());
                    }
                }
                ToClient::Welcome { you, .. } if (1..=7).contains(&you) => self.me = you,
                ToClient::Chat { from, text } => {
                    let text = crate::net::proto::clean_chat(&text).unwrap_or_default();
                    let from = crate::net::proto::clean_name(&from);
                    g.say(match from {
                        Some(f) => format!("{f}: {text}"),
                        None => text,
                    });
                }
                ToClient::Hero(text) if text.len() <= crate::net::proto::MAX_HERO => self.hero = Some(text),
                ToClient::Bye(why) | ToClient::Refused(why) => return Some(crate::net::proto::clean_chat(&why).unwrap_or_else(|| "DISCONNECTED".into())),
                _ => {}
            }
        }
        if let Some(why) = self.link.ended() {
            return Some(why);
        }
        // Your controls (the world point under the mouse, not the screen point).
        self.seq = self.seq.wrapping_add(1);
        let target = inp.mouse.map(|m| g.mouse_world(m));
        let _ = self.link.send(&ToHost::Input(NetInput::from_input(self.seq, inp, target)));
        // The world doesn't run here, but sparks fly and words fade.
        g.tick = g.tick.wrapping_add(1);
        g.client_frame();
        None
    }
}

/// Your save, with your hero as the host last had it: level, experience, gold, gear and skills come from the
/// host; everything about your own world (quests, waypoints, seed, name) stays yours. None if the host's copy
/// is unusable (wrong class, malformed) — then your save is left alone.
pub fn merge_hero(own: &str, from_host: &str) -> Option<String> {
    const HERO: [&str; 28] = [
        "clvl", "xp", "max_hp", "max_mana", "power", "gold", "hp_pots", "mp_pots", "food", "running", "belt_lv", "pack_lv", "vial", "food_bag", "rekindles", "rift_best", "rift_runs", "embers", "ember_points",
        "pages", "supers", "shrines_used", "ward", "pacts", "skills", "points", "primary", "secondary",
    ];
    let key = |l: &str| l.split_once('=').map(|(k, _)| k.trim().to_string());
    let hero_key = |k: &str| HERO.contains(&k) || ["worn", "bag", "stash"].iter().any(|p| k.strip_prefix(p).map_or(false, |n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit())));
    let get = |text: &str, k: &str| text.lines().find_map(|l| l.strip_prefix(k).and_then(|r| r.strip_prefix('='))).map(str::trim).map(str::to_string);
    if from_host.len() > crate::net::proto::MAX_HERO || from_host.lines().count() > 400 || get(own, "class") != get(from_host, "class") {
        return None;
    }
    let mut out: String = own.lines().filter(|l| key(l).map_or(true, |k| !hero_key(&k))).map(|l| format!("{l}\n")).collect();
    for l in from_host.lines() {
        if key(l).map_or(false, |k| hero_key(&k)) && !l.contains(|c: char| c.is_control()) {
            out += l;
            out.push('\n');
        }
    }
    Some(out)
}

impl Game {
    /// A joiner's hero (from its save) joins the party. Refused if it can't be read, isn't sane, or doesn't
    /// match the game (hardcore heroes only in hardcore games, and the other way round).
    pub fn add_ally_from_save(&mut self, text: &str, name: &str) -> Result<usize, &'static str> {
        if self.allies.len() >= crate::net::host::MAX_JOINERS {
            return Err("THE GAME IS FULL");
        }
        let mut scratch = Game::new(self.world_seed(), self.view_h);
        if !crate::save::apply(&mut scratch, text) {
            return Err("YOUR HERO COULDN'T BE READ");
        }
        if !hero_ok(&scratch.p) {
            return Err("YOUR HERO DOESN'T CHECK OUT");
        }
        if scratch.fallen {
            return Err("THAT HERO HAS FALLEN");
        }
        if scratch.hardcore != self.hardcore {
            return Err(if self.hardcore { "THIS IS A HARDCORE GAME" } else { "HARDCORE HEROES PLAY IN HARDCORE GAMES" });
        }
        let k = self.add_ally(scratch.p.skills.class, name);
        let (lx, ly) = (self.allies[k].st.p.x, self.allies[k].st.p.y);
        let st = &mut self.allies[k].st;
        std::mem::swap(&mut st.p, &mut scratch.p);
        st.p.x = lx;
        st.p.y = ly;
        st.p.path.clear();
        st.p.hp = st.p.hp.min(st.p.max_hp).max(1.0);
        st.hardcore = scratch.hardcore;
        st.hero_name = name.to_string();
        Ok(k)
    }

    /// An ally leaves: its loot goes, and everyone after it moves up a number.
    pub fn remove_ally(&mut self, k: usize) {
        if k >= self.allies.len() {
            return;
        }
        self.allies.remove(k);
        let gone = (k + 1) as u8;
        self.pickups.retain(|p| p.owner != gone);
        for p in self.pickups.iter_mut() {
            if p.owner != crate::game::ANY && p.owner > gone {
                p.owner -= 1;
            }
        }
        for m in self.mobs.iter_mut() {
            if m.target == gone {
                m.target = 0;
            } else if m.target > gone {
                m.target -= 1;
            }
        }
    }

    /// A joiner's frame without the world running: timers, sparks and floating numbers.
    pub fn client_frame(&mut self) {
        let dt = crate::game::DT;
        self.banner_t = (self.banner_t - dt).max(0.0);
        if let Some((_, t)) = self.message.as_mut() {
            *t -= dt;
            if *t <= 0.0 {
                self.message = None;
            }
        }
        if let Some((_, _, t)) = self.notice.as_mut() {
            *t -= dt;
            if *t <= 0.0 {
                self.notice = None;
            }
        }
        if self.state == State::Playing {
            self.explore();
        }
        self.update_world();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn your_save_takes_only_your_hero_from_the_host() {
        let own = "name=ROWAN\nseed=5\nclass=druid\nclvl=10\ngold=100\nstage=2\nbag0=a|1|2|3|x|\nwaypoints=overworld\n";
        let host = "name=EVIL\nseed=9\nclass=druid\nclvl=12\ngold=900\nstage=3\nbag0=b|1|2|3|y|\nbag1=c|1|2|3|z|\nwaypoints=everywhere\n";
        let m = merge_hero(own, host).unwrap();
        assert!(m.contains("clvl=12") && m.contains("gold=900") && m.contains("bag1=c"));
        assert!(m.contains("name=ROWAN") && m.contains("seed=5") && m.contains("stage=2") && m.contains("waypoints=overworld"));
        assert!(!m.contains("EVIL") && !m.contains("everywhere") && !m.contains("bag0=a"));
        // Another class: not your hero, nothing changes.
        assert_eq!(merge_hero(own, &host.replace("druid", "reaper")), None);
    }

    #[test]
    fn a_joiner_plays_in_the_hosts_world_over_the_network() {
        let dir = std::env::temp_dir().join(format!("ashen_online_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut host = Game::new(7, crate::gfx::SH_WIDE);
        host.set_class(crate::skills::Class::Sorceress);
        host.debug_goto(crate::world::LevelId::Area(0, 1));
        let h = crate::net::host::Host::start(0, String::new(), &dir).unwrap();
        let mut hosting = Hosting { host: h, peers: vec![], sent_level: None, players_before: 1 };
        let addr = format!("127.0.0.1:{}", hosting.host.port);
        crate::net::tls::trust(&dir, &addr, &hosting.host.fingerprint).unwrap();
        // The joiner's hero, as its save.
        let mut mine = Game::new(11, crate::gfx::SH_WIDE);
        mine.set_class(crate::skills::Class::Reaper);
        mine.hero_name = "ROWAN".into();
        let text = crate::save::to_text(&mine);
        let joining = std::thread::spawn({
            let (addr, dir) = (addr.clone(), dir.clone());
            move || crate::net::client::join(&addr, &dir, "ROWAN", &text, "")
        });
        let mut link = None;
        for _ in 0..600 {
            hosting.before(&mut host);
            host.update(&Input::default());
            hosting.after(&mut host);
            if joining.is_finished() {
                link = Some(joining.join().unwrap().expect("joined"));
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let mut joined = Joined::new(link.expect("joined in time"));
        assert_eq!(host.allies.len(), 1);
        assert_eq!(host.allies[0].st.p.skills.class, crate::skills::Class::Reaper);
        // The joiner walks: its world follows the host's, and its hero moves there.
        let mut client = Game::new(3, crate::gfx::SH_WIDE);
        let start = (host.allies[0].st.p.x, host.allies[0].st.p.y);
        let walk = Input { move_x: 1.0, ..Default::default() };
        // At the game's own pace (60 a second: faster than that, the host would call it a flood).
        for _ in 0..150 {
            assert_eq!(joined.frame(&mut client, &walk), None);
            hosting.before(&mut host);
            host.update(&Input::default());
            hosting.after(&mut host);
            std::thread::sleep(std::time::Duration::from_millis(17));
        }
        // Let the last snapshots arrive.
        for _ in 0..20 {
            joined.frame(&mut client, &Input::default());
            std::thread::sleep(std::time::Duration::from_millis(17));
        }
        let now = (host.allies[0].st.p.x, host.allies[0].st.p.y);
        assert!((now.0 - start.0).abs() + (now.1 - start.1).abs() > 1.0, "the joiner's hero moved on the host: {start:?} -> {now:?}");
        assert_eq!(client.level, host.level, "the joiner is in the host's level");
        assert_eq!(client.p.skills.class, crate::skills::Class::Reaper, "and plays its own hero");
        assert!(client.allies.iter().any(|a| a.st.p.skills.class == crate::skills::Class::Sorceress), "beside the host");
        assert!((client.p.x - now.0).abs() < 2.0 && (client.p.y - now.1).abs() < 2.0, "where the host has it");
        // It leaves: the host's party shrinks.
        drop(joined);
        for _ in 0..200 {
            hosting.before(&mut host);
            if host.allies.is_empty() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(host.allies.is_empty(), "gone from the party");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

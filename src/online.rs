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
        && it.req <= 99
        // No stat beyond what the game's own tables can roll (review 2026-10-22: edited saves).
        && it.stats.iter().all(|&(s, v)| crate::items::stat_ok(s, v))
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
        && p.xp >= 0.0
        && p.xp <= crate::game::xp_to_next(p.clvl) * 1.01
        && (0..=50_000_000).contains(&p.gold)
        // Skill points: what the levels and every relic could give, no more.
        && p.skills.rank.iter().map(|&r| r as u32).sum::<u32>() + p.skills.points <= p.clvl + 40
        && p.skills.rank.iter().all(|&r| r <= 20)
}

/// Hosting: who's joined, as which ally.
pub struct Hosting {
    pub host: Host,
    /// The connection behind each ally (same order as `Game::allies`), its name and its bad inputs.
    peers: Vec<(u32, String, u32)>,
    /// Chat allowance per joiner (2 lines a second, 5 at once): a flood isn't relayed (review 2026-10-22).
    chat: Vec<(f32, std::time::Instant)>,
    /// Sounds and floating numbers since each joiner's last snapshot.
    events: Vec<(Vec<crate::game::Sfx>, Vec<crate::game::Floater>)>,
    sent_level: Option<crate::world::LevelId>,
    /// The Players setting before anyone joined (put back when hosting ends).
    players_before: u8,
}

impl Hosting {
    pub fn start(port: u16, password: String) -> std::io::Result<Hosting> {
        let host = Host::start(port, password, &crate::save::dir())?;
        Ok(Hosting { host, peers: vec![], chat: vec![], events: vec![], sent_level: None, players_before: crate::binds::players() })
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
                        self.events.push((vec![], vec![]));
                        self.chat.push((5.0, std::time::Instant::now()));
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
                            // One-shot buttons not yet used stay pressed until they are.
                            let a = &mut g.allies[k];
                            let keep = (a.inp.potion_hp, a.inp.potion_mp, a.inp.eat, a.inp.scroll, a.inp.cycle, a.inp.run_toggle, a.inp.slot);
                            a.inp = inp;
                            a.inp.potion_hp |= keep.0;
                            a.inp.potion_mp |= keep.1;
                            a.inp.eat |= keep.2;
                            a.inp.scroll |= keep.3;
                            a.inp.cycle |= keep.4;
                            a.inp.run_toggle |= keep.5;
                            a.inp.slot = a.inp.slot.or(keep.6);
                            a.target = target;
                            if input.seq > a.seq || a.seq.wrapping_sub(input.seq) > u32::MAX / 2 {
                                a.seq = input.seq;
                            }
                        }
                        Err(_) => {
                            self.peers[k].2 += 1;
                            if self.peers[k].2 > MAX_STRIKES {
                                self.host.kick(peer, "BAD INPUT");
                            }
                        }
                    }
                }
                Event::Act { peer, act } => {
                    let Some(k) = self.slot_of(peer) else { continue };
                    let out = g.as_hero(k, |g| g.remote_act(&act));
                    if !out.ok {
                        self.peers[k].2 += 1;
                        if self.peers[k].2 > MAX_STRIKES {
                            self.host.kick(peer, "BAD INPUT");
                        }
                    }
                    if let Some((shop, jewel)) = out.opened {
                        self.host.send(peer, &ToClient::OpenBag { shop, jewel });
                    }
                    if let Some(s) = out.said {
                        self.host.send(peer, &ToClient::Say(s));
                    }
                }
                Event::Chat { peer, text } => {
                    let Some(k) = self.slot_of(peer) else { continue };
                    let (tokens, last) = &mut self.chat[k];
                    *tokens = (*tokens + last.elapsed().as_secs_f32() * 2.0).min(5.0);
                    *last = std::time::Instant::now();
                    if *tokens < 1.0 {
                        continue;
                    }
                    *tokens -= 1.0;
                    let from = self.peers[k].1.clone();
                    self.tell_all(&ToClient::Chat { from: from.clone(), text: text.clone() }, None);
                    g.chat_line(format!("{from}: {text}"));
                }
                Event::Gone { peer, why } => {
                    let Some(k) = self.slot_of(peer) else { continue };
                    let (_, name, _) = self.peers.remove(k);
                    self.events.remove(k);
                    self.chat.remove(k);
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
        // This tick's sounds and new floating numbers, for everyone's next snapshot.
        let fresh: Vec<crate::game::Floater> = g.floaters.iter().filter(|f| f.t <= crate::game::DT * 1.5).cloned().collect();
        for (sfx, fl) in self.events.iter_mut() {
            if sfx.len() < 32 {
                sfx.extend(g.sfx.iter().copied());
            }
            if fl.len() < 64 {
                fl.extend(fresh.iter().cloned());
            }
        }
        // What each joiner's hero was told.
        for (k, p) in self.peers.iter().enumerate() {
            for s in std::mem::take(&mut g.allies[k].said) {
                self.host.send(p.0, &ToClient::Say(s));
            }
        }
        if g.tick % SNAP_EVERY == 0 {
            for (k, p) in self.peers.iter().enumerate() {
                let (sfx, fl) = std::mem::take(&mut self.events[k]);
                if let Some(s) = g.net_snapshot(k as u8 + 1, &sfx, &fl) {
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
        crate::binds::clear_players_here();
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
    /// Where things are heading (the latest snapshot); they glide there between snapshots.
    mob_goals: std::collections::HashMap<u32, (f32, f32)>,
    /// When the last few levels arrived: a host can't send them endlessly (review 2026-10-22).
    levels_at: Vec<std::time::Instant>,
    /// Your own hero moves on your screen at once (keys or stick); the host confirms. `pred`: each input's
    /// predicted step not yet confirmed (its number, how far). `own`: where your hero is drawn. `corr`: the
    /// difference from the host still being worked off.
    pred: std::collections::VecDeque<(u32, f32, f32)>,
    own: Option<(f32, f32)>,
    corr: (f32, f32),
    hero_goals: Vec<(f32, f32)>,
}

impl Joined {
    pub fn new(link: Link) -> Joined {
        let me = link.you;
        Joined { link, me, seq: 0, has_level: false, hero: None, mob_goals: Default::default(), hero_goals: vec![], levels_at: vec![], pred: Default::default(), own: None, corr: (0.0, 0.0) }
    }

    /// One frame: take in what the host sent, show it, and send your controls. Returns why it ended, if it did.
    pub fn frame(&mut self, g: &mut Game, inp: &Input) -> Option<String> {
        // Only the newest snapshot matters; older ones waiting behind it are skipped.
        let mut msgs = self.link.poll();
        if let Some(last) = msgs.iter().rposition(|m| matches!(m, ToClient::Snap(_))) {
            let mut i = 0;
            msgs.retain(|m| {
                i += 1;
                !matches!(m, ToClient::Snap(_)) || i - 1 == last
            });
        }
        for m in msgs {
            match m {
                ToClient::Level(b) => {
                    self.levels_at.retain(|t| t.elapsed() < std::time::Duration::from_secs(2));
                    self.levels_at.push(std::time::Instant::now());
                    if self.levels_at.len() > 4 {
                        return Some("THE HOST SENT TOO MANY LEVELS".into());
                    }
                    if !g.apply_level(&b) {
                        return Some("THE HOST SENT A BAD LEVEL".into());
                    }
                    // A new place: start over from where the host puts you.
                    (self.own, self.corr) = (None, (0.0, 0.0));
                    self.pred.clear();
                    self.has_level = true;
                }
                ToClient::Snap(b) => {
                    if !self.has_level {
                        continue;
                    }
                    // Where everything is drawn now, so it can glide to where the host says it is.
                    let was: std::collections::HashMap<u32, (f32, f32)> = g.mobs.iter().map(|m| (m.uid, (m.x, m.y))).collect();
                    let heroes_was: Vec<(f32, f32)> = std::iter::once((g.p.x, g.p.y)).chain(g.allies.iter().map(|a| (a.st.p.x, a.st.p.y))).collect();
                    // Your pick in a conversation is yours; the host's copy only says which conversation.
                    let picked = g.dialog.as_ref().map(|d| (d.name, d.page, d.sel));
                    if !g.apply_snapshot(&b, self.me) {
                        return Some("THE HOST SENT BAD DATA".into());
                    }
                    if let (Some((name, page, sel)), Some(d)) = (picked, g.dialog.as_mut()) {
                        if d.name == name && d.page == page {
                            d.sel = sel.min(d.options.len().saturating_sub(1));
                        }
                    }
                    // While the bag is open on a trader, the host's conversation is over.
                    if g.inv.is_some() {
                        g.dialog = None;
                    }
                    self.mob_goals.clear();
                    for m in g.mobs.iter_mut() {
                        if let Some(&(x, y)) = was.get(&m.uid) {
                            if (x - m.x).powi(2) + (y - m.y).powi(2) < 9.0 {
                                self.mob_goals.insert(m.uid, (m.x, m.y));
                                (m.x, m.y) = (x, y);
                            }
                        }
                    }
                    let now: Vec<(f32, f32)> = std::iter::once((g.p.x, g.p.y)).chain(g.allies.iter().map(|a| (a.st.p.x, a.st.p.y))).collect();
                    self.hero_goals = now.clone();
                    if heroes_was.len() == now.len() {
                        for (k, (&(x, y), &(nx, ny))) in heroes_was.iter().zip(now.iter()).enumerate().skip(1) {
                            if (x - nx).powi(2) + (y - ny).powi(2) < 9.0 {
                                (g.allies[k - 1].st.p.x, g.allies[k - 1].st.p.y) = (x, y);
                            }
                        }
                    }
                    // Your own hero: where the host has it, plus the steps you've taken that it hasn't had yet.
                    let ack = g.net_ack;
                    while self.pred.front().map_or(false, |&(s, ..)| s <= ack) {
                        self.pred.pop_front();
                    }
                    let (sx, sy) = self.pred.iter().fold((0.0, 0.0), |a, &(_, dx, dy)| (a.0 + dx, a.1 + dy));
                    let target = (g.p.x + sx, g.p.y + sy);
                    match self.own {
                        Some((ox, oy)) if (target.0 - ox).powi(2) + (target.1 - oy).powi(2) < 6.25 => self.corr = (target.0 - ox, target.1 - oy),
                        _ => {
                            self.own = Some(target);
                            self.corr = (0.0, 0.0);
                        }
                    }
                    if !hero_ok(&g.p) || g.allies.iter().any(|a| !hero_ok(&a.st.p)) || g.pickups.iter().any(|k| matches!(&k.kind, crate::game::Drop::Item(it) if !item_ok(it))) {
                        return Some("THE HOST SENT BAD DATA".into());
                    }
                }
                ToClient::Welcome { you, .. } if you <= 7 => self.me = you,
                ToClient::OpenBag { shop, jewel } => {
                    g.dialog = None;
                    g.open_inventory();
                    if let Some(ui) = g.inv.as_mut() {
                        (ui.shop, ui.jewel) = (shop, jewel);
                    }
                }
                ToClient::Say(text) => {
                    if let Some(t) = crate::net::proto::clean_chat(&text) {
                        g.say(t);
                    }
                }
                ToClient::Chat { from, text } => {
                    let text = crate::net::proto::clean_chat(&text).unwrap_or_default();
                    let from = crate::net::proto::clean_name(&from);
                    g.chat_line(match from {
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
        // The bag and the skill tree open here; what they change is sent to the host.
        if g.remote.is_none() {
            g.remote = Some(vec![]);
        }
        let pressed = |now: bool, before: bool| now && !before;
        if g.dialog.is_some() && g.inv.is_none() {
            // Talking: pick here, the host does it.
            let choose = pressed(inp.confirm, g.prev.confirm) || pressed(inp.cast, g.prev.cast);
            let click = pressed(inp.lmb, g.prev.lmb);
            let back = Input { back: inp.back || inp.cancel, ..inp.clone() };
            match g.dialog_input(&back, choose, click) {
                Some((Some(i), _)) if i < 256 => {
                    let _ = self.link.send(&ToHost::Talk { pick: i as u8 });
                }
                Some((None, _)) => {
                    let _ = self.link.send(&ToHost::TalkClose);
                    g.dialog = None;
                }
                _ => {}
            }
        } else if g.inv.is_some() {
            let (pc, pl) = (pressed(inp.confirm, g.prev.confirm) || pressed(inp.cast, g.prev.cast), pressed(inp.lmb, g.prev.lmb));
            g.update_inventory(inp, pc, pl);
        } else if g.tree.is_some() {
            let (pc, pl) = (pressed(inp.confirm, g.prev.confirm) || pressed(inp.cast, g.prev.cast), pressed(inp.lmb, g.prev.lmb));
            g.update_tree(inp, pc, pl);
        } else if inp.inv && !g.prev.inv {
            g.open_inventory();
        } else if inp.skills && !g.prev.skills {
            g.open_tree();
        }
        let busy = g.inv.is_some() || g.tree.is_some() || g.dialog.is_some();
        for m in g.remote.as_mut().map(std::mem::take).unwrap_or_default() {
            let _ = self.link.send(&m);
        }
        g.prev = inp.clone();
        // Your controls (the world point under the mouse, not the screen point); none while in the bag or tree.
        let inp = &if busy { Input { mouse: inp.mouse, ..Default::default() } } else { inp.clone() };
        self.seq = self.seq.wrapping_add(1);
        let target = inp.mouse.map(|m| g.mouse_world(m));
        let _ = self.link.send(&ToHost::Input(NetInput::from_input(self.seq, inp, target)));
        // Your own step, taken at once (keys or stick: the host walks the same way). Clicked walks follow the
        // host's path, so they aren't predicted.
        if let Some((mut x, mut y)) = self.own {
            let key = inp.move_x * inp.move_x + inp.move_y * inp.move_y;
            let free = g.state == State::Playing && g.p.charge.is_none() && g.p.whirl_t <= 0.0;
            if key > 0.04 && free {
                let (dx, dy) = crate::iso::screen_dir_to_world(inp.move_x, inp.move_y);
                let mag = key.sqrt().min(1.0);
                let run = g.p.running && !g.p.winded;
                let speed = g.hero_speed(run, false);
                let (x0, y0) = (x, y);
                crate::game::move_circle(&g.d, &mut x, &mut y, dx * mag * speed * crate::game::DT, dy * mag * speed * crate::game::DT, crate::game::PLAYER_R);
                self.pred.push_back((self.seq, x - x0, y - y0));
                // A long way ahead of the host means it's not hearing us: stop guessing.
                while self.pred.len() > 90 {
                    self.pred.pop_front();
                }
                g.p.dir = crate::iso::dir8(dx, dy);
                g.p.moving = true;
            }
            // Work off the difference from the host a little each frame.
            let (cx, cy) = (self.corr.0 * 0.2, self.corr.1 * 0.2);
            self.corr = (self.corr.0 - cx, self.corr.1 - cy);
            self.own = Some((x + cx, y + cy));
            if let Some((ox, oy)) = self.own {
                (g.p.x, g.p.y) = (ox, oy);
            }
        }
        // Glide toward the latest snapshot (it comes 20 times a second; frames come 60).
        const GLIDE: f32 = 0.4;
        for m in g.mobs.iter_mut() {
            if let Some(&(x, y)) = self.mob_goals.get(&m.uid) {
                m.x += (x - m.x) * GLIDE;
                m.y += (y - m.y) * GLIDE;
            }
        }
        for (k, &(x, y)) in self.hero_goals.iter().enumerate().skip(1) {
            let p = g.allies.get_mut(k - 1).map(|a| &mut a.st.p);
            if let Some(p) = p {
                p.x += (x - p.x) * GLIDE;
                p.y += (y - p.y) * GLIDE;
            }
        }
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
    // A hero only grows in someone else's game: a copy that's lower level than yours isn't taken (review 2026-10-22).
    let lvl = |t: &str| get(t, "clvl").and_then(|v| v.parse::<u32>().ok()).unwrap_or(0);
    if lvl(from_host) < lvl(own) {
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
    /// For the tests: just whether the action was well-formed.
    #[cfg(test)]
    fn remote_act_ok(&mut self, m: &ToHost) -> bool {
        self.remote_act(m).ok
    }
}

/// What a joiner's action came to: well-formed or not, a trader or jeweler opened (shop, jeweler), and
/// what the hero was told.
pub struct ActOut {
    pub ok: bool,
    pub opened: Option<(bool, bool)>,
    pub said: Option<String>,
}

/// What a leader on the login server may choose besides: going places (the whole party goes).
fn leader_may(a: crate::story::Act) -> bool {
    use crate::story::Act::*;
    matches!(a, Travel(_) | OpenRift(_) | Arena(_) | Duel | NextDifficulty)
}

/// Conversation choices a joiner may make. Not: travelling (the host leads the party), quitting or changing the
/// host's game or settings, rifts and arenas (they move everyone).
fn joiner_may(a: crate::story::Act) -> bool {
    use crate::story::Act::*;
    matches!(a, Next | Close | Buy(_) | Respec(_) | Shop | Jewel | Combine | Ember(_) | Side(_) | BuyStock(_) | Pact(_) | Court(_) | Rival(_) | Rekindle(_))
}

impl Game {
    /// A joiner's bag or skill-tree action, for the active hero (as_hero): checked, then done just as the
    /// hero's own screen would. False if it was malformed (counted against the joiner).
    pub fn remote_act(&mut self, m: &ToHost) -> ActOut {
        self.remote_act_as(m, false)
    }

    /// The same, for the party's leader on the login server (who may also travel: everyone goes).
    pub fn remote_act_as(&mut self, m: &ToHost, leader: bool) -> ActOut {
        use crate::inventory::{Cell, InvAct, InvUi};
        use crate::net::proto::*;
        // The host's own open bag, words and journey are set aside while the joiner's action runs.
        let keep_inv = self.inv.take();
        let keep_tree = self.tree.take();
        let keep_msg = self.message.take();
        let keep_travel = self.pending_travel.take();
        let in_town = self.safe.map_or(false, |(x0, y0, x1, y1)| self.p.x >= x0 && self.p.x <= x1 && self.p.y >= y0 && self.p.y <= y1);
        // The trader or jeweler counts only near where it was opened.
        let near_shop = self.hero_shop.filter(|&(x, y, _)| (self.p.x - x).powi(2) + (self.p.y - y).powi(2) <= 16.0);
        let shelf = near_shop.filter(|s| !s.2).map(|_| self.shop_stock.len());
        let mut opened = None;
        let ok = match *m {
            ToHost::Bag { op: BAG_CLOSE, .. } => {
                self.hero_shop = None;
                true
            }
            ToHost::Bag { op: BAG_JEWEL, cell } => match cell_in(cell, None) {
                Some(c) if near_shop.map_or(false, |s| s.2) && matches!(c, Cell::Bag(_) | Cell::Worn(_)) => {
                    let mut ui = InvUi::at(c);
                    ui.jewel = true;
                    self.inv = Some(ui);
                    self.inv_apply(Some(InvAct::Use), c, true, None);
                    true
                }
                Some(_) => true,
                None => false,
            },
            ToHost::Bag { op, cell } => match (cell_in(cell, shelf), op) {
                (Some(c), BAG_USE | BAG_TOSS | BAG_STORE | BAG_SORT) => {
                    // The stash is only there in town.
                    if (op == BAG_STORE || matches!(c, Cell::Stash(_))) && !in_town {
                        true
                    } else {
                        let act = match op {
                            BAG_USE => InvAct::Use,
                            BAG_TOSS => InvAct::Toss,
                            BAG_STORE => InvAct::Store,
                            _ => InvAct::Sort,
                        };
                        let mut ui = InvUi::at(c);
                        ui.shop = shelf.is_some();
                        self.inv = Some(ui);
                        self.inv_apply(Some(act), c, false, None);
                        true
                    }
                }
                _ => false,
            },
            ToHost::Gem { from, to } => match cell_in(to, None) {
                Some(c) if (from as usize) < crate::items::BAG && matches!(c, Cell::Bag(_) | Cell::Worn(_)) => {
                    self.inv = Some(InvUi::at(c));
                    self.inv_apply(Some(InvAct::Select(c)), c, false, Some(from as usize));
                    true
                }
                _ => false,
            },
            ToHost::Skill { op, skill } => {
                let s = crate::skills::ALL.get(skill as usize).copied();
                match s {
                    Some(s) if self.p.skills.class.tree().contains(&s) && op <= SKILL_SECONDARY => {
                        let act = [crate::skills::TreeAct::Learn, crate::skills::TreeAct::Primary, crate::skills::TreeAct::Secondary][op as usize];
                        self.tree_apply(Some(act), s);
                        true
                    }
                    _ => false,
                }
            }
            ToHost::Talk { pick } => match self.dialog.as_ref().and_then(|d| d.options.get(pick as usize)).map(|o| o.1) {
                Some(act) if joiner_may(act) || (leader && leader_may(act)) => {
                    if let Some(d) = self.dialog.as_mut() {
                        d.sel = pick as usize;
                    }
                    self.dialog_act(Some(act));
                    // A trader's shelf or the jeweler's bench opened: on the joiner's screen, not here.
                    if let Some(ui) = self.inv.take() {
                        self.hero_shop = Some((self.p.x, self.p.y, ui.jewel));
                        opened = Some((ui.shop, ui.jewel));
                    }
                    true
                }
                Some(_) => {
                    self.say("THE HOST LEADS THE PARTY: ASK THEM".into());
                    true
                }
                // No such option, or no conversation: malformed.
                None => false,
            },
            ToHost::TalkClose => {
                if self.dialog.is_some() {
                    self.dialog_act(Some(crate::story::Act::Close));
                }
                true
            }
            _ => false,
        };
        let said = self.message.take().map(|m| m.0);
        self.inv = keep_inv;
        self.tree = keep_tree;
        self.message = keep_msg;
        // A joiner's choice never moves the party (the host leads); the leader's does.
        if !leader {
            self.pending_travel = keep_travel;
        }
        ActOut { ok, opened, said }
    }

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
    fn a_joiners_bag_and_skill_actions_are_checked_then_done() {
        use crate::net::proto::*;
        let mut g = Game::new(7, crate::gfx::SH_WIDE);
        g.set_class(crate::skills::Class::Sorceress);
        g.open_inventory();
        let k = g.add_ally(crate::skills::Class::Valkyrie, "BRYNJA");
        // An item in the joiner's bag: using it wears it (the bag cell empties or swaps).
        let mut rng = crate::rng::Rng::new(5);
        let it = crate::items::roll(1, crate::items::Rarity::Normal, &mut rng);
        let name = it.name.clone();
        g.allies[k].st.p.gear.bag = vec![None; crate::items::BAG];
        g.allies[k].st.p.gear.bag[4] = Some(it);
        assert!(g.as_hero(k, |g| g.remote_act_ok(&ToHost::Bag { op: BAG_USE, cell: (CELL_BAG, 4) })));
        assert!(g.allies[k].st.p.gear.worn.iter().flatten().any(|w| w.name == name), "worn now");
        assert!(g.inv.is_some(), "the host's own bag is still open");
        // Learning a skill of the joiner's class.
        let s = crate::skills::Class::Valkyrie.tree()[0];
        let idx = crate::skills::ALL.iter().position(|x| *x == s).unwrap() as u16;
        g.allies[k].st.p.skills.points = 3;
        let before = g.allies[k].st.p.skills.rank(s);
        assert!(g.as_hero(k, |g| g.remote_act_ok(&ToHost::Skill { op: SKILL_LEARN, skill: idx })));
        assert!(g.allies[k].st.p.skills.rank(s) > before || g.allies[k].st.p.skills.points < 3, "learned (or refused by the game's own rules)");
        // Nonsense is refused: other classes' skills, cells that don't exist, unknown actions, the shop.
        let other = crate::skills::ALL.iter().position(|x| *x == crate::skills::Class::Sorceress.tree()[0]).unwrap() as u16;
        for m in [
            ToHost::Skill { op: SKILL_LEARN, skill: other },
            ToHost::Skill { op: SKILL_LEARN, skill: 9999 },
            ToHost::Skill { op: 7, skill: idx },
            ToHost::Bag { op: BAG_USE, cell: (CELL_BAG, 999) },
            ToHost::Bag { op: 9, cell: (CELL_BAG, 0) },
            ToHost::Bag { op: BAG_USE, cell: (7, 0) },
            ToHost::Gem { from: 999, to: (CELL_BAG, 0) },
            ToHost::Chat("x".into()),
        ] {
            assert!(!g.as_hero(k, |g| g.remote_act_ok(&m)), "{m:?} refused");
        }
        // The stash is only there in town.
        g.allies[k].st.p.x = -500.0;
        assert!(g.as_hero(k, |g| g.remote_act_ok(&ToHost::Bag { op: BAG_STORE, cell: (CELL_BAG, 0) })), "well-formed, but nothing happens out of town");
    }

    #[test]
    fn every_item_the_game_makes_passes_and_edited_ones_dont() {
        use crate::items::*;
        let mut rng = crate::rng::Rng::new(9);
        let mut made = vec![];
        for ilvl in [1u8, 10, 30, 60, 90] {
            for _ in 0..400 {
                made.push(drop(ilvl, 500, true, &mut rng));
                made.push(roll(ilvl, Rarity::Rare, &mut rng));
            }
        }
        for i in 0..UNIQUES.len() {
            made.push(unique(i));
        }
        for c in crate::skills::ALL_CLASSES {
            for _ in 0..50 {
                made.extend(roll_ancient(c, &mut rng));
            }
        }
        for it in &made {
            assert!(item_ok(it), "the game's own item refused: {} {:?}", it.name, it.stats);
        }
        // An edited one: two billion damage.
        let mut evil = made[0].clone();
        evil.stats.push((evil.stats.first().map_or(Stat::Life, |s| s.0), 2_000_000_000));
        assert!(!item_ok(&evil));
        // A hero with every skill maxed at level 1.
        let mut g = Game::new(1, crate::gfx::SH_WIDE);
        g.set_class(crate::skills::Class::Druid);
        assert!(hero_ok(&g.p));
        for r in g.p.skills.rank.iter_mut() {
            *r = 20;
        }
        assert!(!hero_ok(&g.p), "impossible skill points refused");
    }

    #[test]
    fn a_lower_level_copy_from_the_host_isnt_taken() {
        let own = "class=druid
clvl=30
gold=5
";
        assert_eq!(merge_hero(own, "class=druid
clvl=1
gold=0
"), None);
        assert!(merge_hero(own, "class=druid
clvl=31
gold=9
").unwrap().contains("clvl=31"));
    }

    #[test]
    fn enter_opens_a_chat_line_and_sends_it_without_moving_you() {
        let dir = std::env::temp_dir().join(format!("ashen_chat_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let h = crate::net::host::Host::start(0, String::new(), &dir).unwrap();
        let mut session = Some(Session::Host(Hosting { host: h, peers: vec![], chat: vec![], events: vec![], sent_level: None, players_before: 1 }));
        let mut g = Game::new(7, crate::gfx::SH_WIDE);
        g.hero_name = "ISOLDE".into();
        let mut line = None;
        let out = chat_keys(&mut line, &mut session, &mut g, &Input { confirm: true, ..Default::default() });
        assert_eq!(line.as_deref(), Some(""), "open");
        assert!(!out.confirm, "the game doesn't see that Enter");
        // Typing W A S D doesn't walk.
        let out = chat_keys(&mut line, &mut session, &mut g, &Input { typed: "wasd hi".into(), move_x: 1.0, ..Default::default() });
        assert_eq!(out.move_x, 0.0);
        let _ = chat_keys(&mut line, &mut session, &mut g, &Input { backspace: true, ..Default::default() });
        let _ = chat_keys(&mut line, &mut session, &mut g, &Input { confirm: true, ..Default::default() });
        assert_eq!(line, None, "sent");
        assert_eq!(g.chat_log.last().map(|l| l.0.as_str()), Some("ISOLDE: WASD H"));
        // Esc cancels without sending.
        let _ = chat_keys(&mut line, &mut session, &mut g, &Input { confirm: true, ..Default::default() });
        let _ = chat_keys(&mut line, &mut session, &mut g, &Input { typed: "nope".into(), ..Default::default() });
        let _ = chat_keys(&mut line, &mut session, &mut g, &Input { cancel: true, ..Default::default() });
        assert_eq!(line, None);
        assert_eq!(g.chat_log.len(), 1);
        // Offline, Enter is the game's as before.
        let mut none = None;
        let out = chat_keys(&mut line, &mut none, &mut g, &Input { confirm: true, ..Default::default() });
        assert!(out.confirm && line.is_none());
        drop(session);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_joiner_talks_trades_and_cant_steer_the_hosts_game() {
        use crate::net::proto::*;
        use crate::story::{Act, Dialog};
        let mut g = Game::new(7, crate::gfx::SH_WIDE);
        g.set_class(crate::skills::Class::Sorceress);
        let k = g.add_ally(crate::skills::Class::Valkyrie, "BRYNJA");
        g.allies[k].st.p.gold = 100_000;
        // The trader's conversation, for the joiner's hero.
        let mut d = Dialog::new("GERTA", &["WARES?"]);
        d.last_options = vec![("TRADE".into(), Act::Shop), ("QUIT THE HOST'S GAME".into(), Act::QuitGame), ("TRAVEL".into(), Act::Travel(crate::world::LevelId::Frostmarch)), ("PLAYERS".into(), Act::CyclePlayers)];
        d.refresh_options();
        g.allies[k].st.dialog = Some(d);
        let players = crate::binds::players();
        // Not allowed: quitting, travelling, settings. Nothing happens to the host, the conversation stays open.
        for pick in [1u8, 2, 3] {
            let out = g.as_hero(k, |g| g.remote_act(&ToHost::Talk { pick }));
            assert!(out.ok && out.opened.is_none());
        }
        assert!(!g.exit && !g.quit && g.pending_travel.is_none() && crate::binds::players() == players, "the host's game is untouched");
        assert!(g.as_hero(k, |g| !g.remote_act(&ToHost::Talk { pick: 40 }).ok), "no such option");
        // Trading: the shelf opens for the joiner, not on the host's screen.
        let out = g.as_hero(k, |g| g.remote_act(&ToHost::Talk { pick: 0 }));
        assert_eq!(out.opened, Some((true, false)));
        assert!(g.inv.is_none(), "the host's screen stays as it was");
        assert!(!g.shop_stock.is_empty());
        // Buying from the shelf, near the trader.
        let bag_before = g.allies[k].st.p.gear.bag.iter().flatten().count();
        let gold_before = g.allies[k].st.p.gold;
        assert!(g.as_hero(k, |g| g.remote_act(&ToHost::Bag { op: BAG_USE, cell: (CELL_SHOP, 0) }).ok));
        assert!(g.allies[k].st.p.gear.bag.iter().flatten().count() > bag_before && g.allies[k].st.p.gold < gold_before, "bought");
        // Walk off and the shelf is gone: the shop cell isn't a real one any more.
        g.allies[k].st.p.x += 10.0;
        assert!(g.as_hero(k, |g| !g.remote_act(&ToHost::Bag { op: BAG_USE, cell: (CELL_SHOP, 0) }).ok));
        // No jeweler's bench was opened: jeweler actions do nothing.
        assert!(g.as_hero(k, |g| g.remote_act(&ToHost::Bag { op: BAG_JEWEL, cell: (CELL_BAG, 0) }).ok));
        // Closing the bag ends the trade.
        assert!(g.as_hero(k, |g| g.remote_act(&ToHost::Bag { op: BAG_CLOSE, cell: (0, 0) }).ok));
        assert!(g.allies[k].st.shop.is_none());
    }

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
        let mut hosting = Hosting { host: h, peers: vec![], chat: vec![], events: vec![], sent_level: None, players_before: 1 };
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
        // First the level and a snapshot arrive...
        for _ in 0..30 {
            joined.frame(&mut client, &Input::default());
            hosting.before(&mut host);
            host.update(&Input::default());
            hosting.after(&mut host);
            std::thread::sleep(std::time::Duration::from_millis(17));
        }
        // ...then the joiner's own hero moves on its screen at once, before the host has answered at all.
        let here = (client.p.x, client.p.y);
        for _ in 0..12 {
            joined.frame(&mut client, &walk);
        }
        assert!((client.p.x - here.0).abs() + (client.p.y - here.1).abs() > 0.3, "predicted: {here:?} -> {:?}", (client.p.x, client.p.y));
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

/// What's running: hosting, or joined to someone (with your own game set aside until you leave).
pub enum Session {
    Host(Hosting),
    Join { joined: Joined, home: Box<Game> },
    /// In a game on a login server (lobby_ui.rs): the hero is the server's, nothing to save here.
    Server { joined: Joined },
}

/// Online play asked for on the command line: `--host [port]`, `--join ADDRESS`, `--password WORD`,
/// `--trust` (trust a host met for the first time).
#[derive(Default, Clone, PartialEq)]
pub struct Ask {
    pub host: Option<u16>,
    pub join: Option<String>,
    pub password: String,
    /// `--trust KEY`: trust a host met for the first time, but only if its key is this one (as the game shows
    /// it, spaces optional).
    pub trust: Option<String>,
}

/// Never prints the password.
impl std::fmt::Debug for Ask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Ask").field("host", &self.host).field("join", &self.join).field("password", &if self.password.is_empty() { "" } else { "(set)" }).field("trust", &self.trust.is_some()).finish()
    }
}

impl Ask {
    pub fn from_args(args: &[String]) -> Ask {
        let after = |flag: &str| args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).filter(|v| !v.starts_with("--")).cloned();
        Ask {
            host: args.iter().any(|a| a == "--host").then(|| after("--host").and_then(|p| p.parse().ok()).unwrap_or(crate::net::proto::DEFAULT_PORT)),
            join: after("--join"),
            password: after("--password").unwrap_or_default(),
            trust: after("--trust").map(|k| k.replace(' ', "").to_ascii_uppercase()),
        }
    }

    pub fn wanted(&self) -> bool {
        self.host.is_some() || self.join.is_some()
    }
}

/// Starts what was asked for, once a hero is in play. Says how it went in the game's message line.
pub fn start(ask: &mut Ask, game: &mut Game) -> Option<Session> {
    if let Some(port) = ask.host.take() {
        return match Hosting::start(port, ask.password.clone()) {
            Ok(h) => {
                let fp = crate::net::tls::show(&h.host.fingerprint);
                game.say(format!("HOSTING ON PORT {}. KEY {fp}", h.host.port));
                Some(Session::Host(h))
            }
            Err(e) => {
                game.say(format!("COULDN'T HOST: {e}").to_uppercase());
                None
            }
        };
    }
    let addr = ask.join.take()?;
    if game.hardcore && game.fallen {
        game.say("A FALLEN HERO CAN'T JOIN".into());
        return None;
    }
    let dir = crate::save::dir();
    let hero = crate::save::to_text(game);
    let mut tries = 0;
    loop {
        tries += 1;
        match crate::net::client::join(&addr, &dir, &game.hero_name, &hero, &ask.password) {
            Ok(link) => {
                let host = link.host_name.clone();
                let mirror = Game::new(game.world_seed(), game.view_h);
                let home = std::mem::replace(game, mirror);
                game.say(format!("JOINED {host}'S GAME"));
                return Some(Session::Join { joined: Joined::new(link), home: Box::new(home) });
            }
            Err(crate::net::client::JoinError::Unknown(fp)) if tries == 1 && ask.trust.as_deref() == Some(&crate::net::tls::show(&fp).replace(' ', "")) => {
                let _ = crate::net::tls::trust(&dir, &crate::net::tls::host_key(&addr), &fp);
            }
            Err(crate::net::client::JoinError::Unknown(fp)) => {
                let key = crate::net::tls::show(&fp);
                game.say(format!("NEW HOST, KEY {key}. IF THAT'S THEIRS, JOIN WITH --trust {}", key.replace(' ', "")));
                return None;
            }
            Err(crate::net::client::JoinError::Changed(fp)) => {
                game.say(format!("THE HOST'S KEY CHANGED ({}). NOT JOINING", crate::net::tls::show(&fp)));
                return None;
            }
            Err(crate::net::client::JoinError::Refused(why)) | Err(crate::net::client::JoinError::Unreachable(why)) => {
                game.say(format!("COULDN'T JOIN: {why}"));
                return None;
            }
        }
    }
}

impl Hosting {
    /// The host says something: everyone hears it.
    pub fn say_all(&self, g: &mut Game, text: &str) {
        if let Some(text) = crate::net::proto::clean_chat(text) {
            let from = crate::net::proto::clean_name(&g.hero_name).unwrap_or_else(|| "HOST".into());
            self.tell_all(&ToClient::Chat { from: from.clone(), text: text.clone() }, None);
            g.chat_line(format!("{from}: {text}"));
        }
    }
}

/// How long a chat line stays up (seconds), and how many are kept.
const CHAT_SHOWN: f32 = 12.0;
const CHAT_LINES: usize = 6;

impl Game {
    /// A line in the chat log (and the terminal with ASHEN_NETLOG).
    pub fn chat_line(&mut self, text: String) {
        if std::env::var_os("ASHEN_NETLOG").is_some() {
            eprintln!("[{}] chat: {text}", self.hero_name);
        }
        self.chat_log.push((text, CHAT_SHOWN));
        if self.chat_log.len() > CHAT_LINES {
            self.chat_log.remove(0);
        }
    }
}

/// Typing a chat line, online: Enter opens it (out of menus and conversations), Enter sends, Esc cancels.
/// While it's open the game gets no keys (so typing doesn't walk or cast). Returns the input for the game.
pub fn chat_keys(line: &mut Option<String>, session: &mut Option<Session>, game: &mut Game, inp: &Input) -> Input {
    for (_, t) in game.chat_log.iter_mut() {
        *t -= crate::game::DT;
    }
    game.chat_log.retain(|(_, t)| *t > 0.0);
    if session.is_none() {
        *line = None;
        return inp.clone();
    }
    match line {
        None => {
            let free = game.dialog.is_none() && game.inv.is_none() && game.tree.is_none() && game.state == State::Playing;
            if inp.confirm && free {
                *line = Some(String::new());
                return Input { mouse: inp.mouse, ..Default::default() };
            }
            inp.clone()
        }
        Some(text) => {
            for c in inp.typed.chars() {
                if text.chars().count() < crate::net::proto::MAX_CHAT && (c.is_ascii_graphic() || c == ' ') {
                    text.push(c.to_ascii_uppercase());
                }
            }
            if inp.backspace {
                text.pop();
            }
            if inp.confirm {
                let said = std::mem::take(text);
                *line = None;
                match session {
                    Some(Session::Host(h)) => h.say_all(game, &said),
                    Some(Session::Join { joined, .. }) | Some(Session::Server { joined }) => {
                        if crate::net::proto::clean_chat(&said).is_some() {
                            let _ = joined.link.send(&ToHost::Chat(said));
                        }
                    }
                    None => {}
                }
            } else if inp.cancel || inp.back {
                *line = None;
            }
            Input { mouse: inp.mouse, ..Default::default() }
        }
    }
}

/// The chat log over the bottom bar, and the line being typed.
pub fn draw_chat(scr: &mut crate::gfx::Screen, game: &Game, line: &Option<String>) {
    use crate::gfx::{rgb, Align, BLACK};
    let bottom = game.view_h - crate::game::HUD_H - 22;
    // Just right of the life globe.
    let x = 132;
    let mut y = bottom - if line.is_some() { 14 } else { 0 };
    for (text, t) in game.chat_log.iter().rev() {
        y -= 11;
        let a = (*t / 1.5).min(1.0);
        let w = crate::gfx::text_width(text, 1);
        scr.blend(x - 4, y - 2, w + 8, 11, BLACK, 0.5 * a);
        scr.text(text, x, y, crate::gfx::mix(BLACK, rgb(0xe0e8ff), a), Align::Left, 1);
    }
    if let Some(text) = line {
        let caret = if (game.tick / 20) % 2 == 0 { "_" } else { " " };
        let shown = format!("SAY: {text}{caret}");
        let w = crate::gfx::text_width(&shown, 1).max(200);
        scr.blend(x - 4, bottom - 4, w + 8, 13, BLACK, 0.75);
        scr.text(&shown, x, bottom - 2, rgb(0xffe0a0), Align::Left, 1);
    }
}

/// One tick of the game, online or not.
pub fn step(session: &mut Option<Session>, game: &mut Game, inp: &Input) {
    match session {
        None => game.update(inp),
        Some(Session::Host(h)) => {
            h.before(game);
            game.update(inp);
            h.after(game);
        }
        Some(Session::Join { joined, .. }) | Some(Session::Server { joined }) => {
            // Esc leaves the game (out of conversations and the bag: there it closes them). Anything the host sent
            // that the game can't take ends the session, not the game.
            let open = game.dialog.is_some() || game.inv.is_some() || game.tree.is_some();
            let why = if (inp.cancel && !open) || inp.quit_ask {
                Some("LEFT THE GAME".to_string())
            } else {
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| joined.frame(game, inp))).unwrap_or_else(|_| Some("THE HOST SENT SOMETHING THE GAME COULDN'T TAKE".into()))
            };
            if let Some(why) = why {
                leave(session, game, &why);
            }
        }
    }
}

/// Is the game on screen yours to save? (Not while joined: that's the host's world.)
pub fn may_save(session: &Option<Session>) -> bool {
    !matches!(session, Some(Session::Join { .. }) | Some(Session::Server { .. }))
}

/// Ends the session. Joined: back to your own game, with your hero as the host last had it (your save is
/// copied to .bak first).
pub fn leave(session: &mut Option<Session>, game: &mut Game, why: &str) {
    match session.take() {
        Some(Session::Join { joined, home }) => {
            let hero = joined.hero.clone();
            drop(joined);
            let mut home = *home;
            if let Some(h) = hero {
                let own = crate::save::to_text(&home);
                if let Some(merged) = merge_hero(&own, &h) {
                    let mut g = Game::new(home.world_seed(), home.view_h);
                    g.defer_travel = true;
                    if crate::save::apply(&mut g, &merged) && hero_ok(&g.p) {
                        // The last five saves before each online game are kept (save.bak1 newest ... save.bak5).
                        let p = crate::save::path();
                        for k in (1..5).rev() {
                            let _ = std::fs::rename(p.with_extension(format!("bak{k}")), p.with_extension(format!("bak{}", k + 1)));
                        }
                        let _ = std::fs::copy(&p, p.with_extension("bak1"));
                        let _ = std::fs::write(&p, &merged);
                        home = g;
                    }
                }
            }
            *game = home;
            game.say(why.to_string());
        }
        Some(Session::Server { joined }) => {
            // The server keeps the hero; back to a blank world (the server's screens come back up).
            drop(joined);
            *game = Game::new(game.world_seed(), game.view_h);
            game.say(why.to_string());
        }
        Some(Session::Host(h)) => drop(h),
        None => {}
    }
}

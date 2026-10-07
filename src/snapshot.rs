//! Headless verification. `--snapshot <dir>` plays a scripted bot, writes BMP frames, then
//! stages one frame per area (town, wilds, each dungeon, each boss, dialogue, victory);
//! `--selftest` lets the bot play for six minutes and checks invariants.
use crate::game::{Game, Input, State};
use crate::gfx::{Screen, SH_TALL, SH_WIDE};
use crate::story::Role;
use crate::world::{LevelId, DUNGEONS};

fn write_bmp(path: &str, scr: &Screen) -> std::io::Result<()> {
    let (w, h) = (scr.w as u32, scr.h as u32);
    let row = w * 3;
    let pad = (4 - row % 4) % 4;
    let size = 54 + (row + pad) * h;
    let mut b = Vec::with_capacity(size as usize);
    b.extend_from_slice(b"BM");
    b.extend_from_slice(&size.to_le_bytes());
    b.extend_from_slice(&[0; 4]);
    b.extend_from_slice(&54u32.to_le_bytes());
    b.extend_from_slice(&40u32.to_le_bytes());
    b.extend_from_slice(&(w as i32).to_le_bytes());
    b.extend_from_slice(&(h as i32).to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&24u16.to_le_bytes());
    b.extend_from_slice(&[0; 24]);
    for y in (0..h).rev() {
        for x in 0..w {
            let c = scr.px[(y * w + x) as usize];
            b.extend_from_slice(&[(c & 0xFF) as u8, ((c >> 8) & 0xFF) as u8, ((c >> 16) & 0xFF) as u8]);
        }
        b.extend(std::iter::repeat(0).take(pad as usize));
    }
    std::fs::write(path, b)
}

/// A bot that plays the game: talks to the elder, fights what it sees, eats when hungry,
/// and otherwise heads for the next stairs or dungeon. Paths are cached between ticks.
struct Bot {
    path: Vec<(f32, f32)>,
    goal: (f32, f32),
    repath: u32,
}

impl Bot {
    fn steer(&mut self, g: &Game, t: u32, goal: (f32, f32), inp: &mut Input) {
        let moved = (goal.0 - self.goal.0).abs() + (goal.1 - self.goal.1).abs() > 1.5;
        if moved || t >= self.repath || self.path.is_empty() {
            self.path = g.bot_path(goal.0, goal.1).unwrap_or_default();
            self.goal = goal;
            self.repath = t + 45;
        }
        while let Some(&(nx, ny)) = self.path.first() {
            if (nx - g.p.x).powi(2) + (ny - g.p.y).powi(2) < 0.2 {
                self.path.remove(0);
            } else {
                break;
            }
        }
        let (nx, ny) = self.path.first().copied().unwrap_or(goal);
        let (sx, sy) = crate::iso::to_screen(nx - g.p.x, ny - g.p.y);
        let l = (sx * sx + sy * sy).sqrt().max(0.01);
        inp.move_x = sx / l;
        inp.move_y = sy / l;
    }

    fn act(&mut self, g: &Game, t: u32) -> Input {
        let mut inp = Input::default();
        let hp = g.p.hp / g.p.max_hp;
        inp.potion_hp = hp < 0.4 && t % 30 == 0;
        inp.potion_mp = g.p.mana < 6.0 && t % 30 == 0;
        if matches!(g.state, State::Dead(_) | State::Victory(_)) {
            inp.confirm = t % 60 == 0;
            return inp;
        }
        if let Some(d) = g.dialog.as_ref() {
            // Waypoints: the bot walks everywhere (cancel the travel menu).
            if d.name == "WAYPOINT" {
                inp.cancel = t % 20 == 0;
            } else {
                inp.confirm = t % 20 == 0;
            }
            return inp;
        }
        // The act's story-giver first, and again with all three herald tokens.
        if let Some(role) = g.bot_story_npc() {
            if let Some((ex, ey)) = g.bot_npc(role) {
                // Step right up, so it's them (not a villager beside them) you talk to.
                let d2 = (ex - g.p.x).powi(2) + (ey - g.p.y).powi(2);
                if d2 < 2.0 && g.nearest_npc_role(1.8) == Some(role) {
                    inp.confirm = t % 10 == 0;
                } else {
                    self.steer(g, t, (ex, ey), &mut inp);
                }
                return inp;
            }
        }
        if let Some(tk) = g.bot_token() {
            // A boss's token first.
            self.steer(g, t, tk, &mut inp);
            return inp;
        }
        if g.p.food < 40.0 {
            if let Some(f) = g.bot_food() {
                self.steer(g, t, f, &mut inp);
                return inp;
            }
        }
        if let Some((mx, my, dist, visible)) = g.bot_target() {
            if visible && dist < 9.0 && !g.in_safe(g.p.x, g.p.y) {
                // Every third press uses the secondary skill, and it cycles through all she knows.
                if t % 3 == 0 {
                    inp.cast2 = true;
                } else {
                    inp.cast = true;
                }
                inp.cycle = t % 150 == 0;
                // Back off when something is in melee range.
                if dist < 1.6 {
                    let (sx, sy) = crate::iso::to_screen(g.p.x - mx, g.p.y - my);
                    let l = (sx * sx + sy * sy).sqrt().max(0.01);
                    inp.cast = t % 40 < 20;
                    inp.move_x = sx / l;
                    inp.move_y = sy / l;
                }
                return inp;
            }
            // Bosses are worth walking to; everything else only if it's right there.
            let boss_near = g.boss_alive_near(14.0);
            if (dist < 6.0 || boss_near) && !g.level.overland() {
                self.steer(g, t, (mx, my), &mut inp);
                return inp;
            }
        }
        if let Some(p) = g.bot_portal() {
            self.steer(g, t, p, &mut inp);
        } else if let Some(b) = g.bot_boss() {
            // The last floor: go and find the boss.
            self.steer(g, t, b, &mut inp);
        }
        inp
    }
}

pub fn run(dir: Option<&str>, tall: bool) -> i32 {
    let h = if tall { SH_TALL } else { SH_WIDE };
    let mut g = Game::new(7, h);
    // ASHEN_CLASS=druid / ASHEN_ACT=4: let the bot play any hero, from any act (crash hunting).
    if let Ok(c) = std::env::var("ASHEN_CLASS") {
        let class = crate::skills::ALL_CLASSES.iter().copied().find(|k| k.key() == c).unwrap_or(crate::skills::Class::Sorceress);
        g.set_class(class);
        // Every skill learned, so the bot uses them all.
        for s in class.tree() {
            g.p.skills.rank[*s as usize] = 3;
        }
    }
    if let Some(act) = std::env::var("ASHEN_ACT").ok().and_then(|a| a.parse::<usize>().ok()).filter(|a| *a > 1) {
        let skills = g.p.skills.clone();
        g.act_start(act - 1);
        let points = g.p.skills.points;
        g.p.skills = skills;
        g.p.skills.points = points;
    }
    let mut scr = Screen::new(h);
    if let Some(d) = dir {
        std::fs::create_dir_all(d).ok();
    }
    let shots = [2u32, 400, 1200, 2400, 3600];
    // ASHEN_MINUTES=40: a longer run (whole acts); ASHEN_GOD=1 keeps the bot alive so it reaches the bosses.
    let minutes = std::env::var("ASHEN_MINUTES").ok().and_then(|m| m.parse::<u32>().ok()).unwrap_or(6);
    let god = std::env::var("ASHEN_GOD").map_or(false, |v| v == "1");
    let total = if dir.is_some() { 3601 } else { 60 * 60 * minutes };
    let t0 = std::time::Instant::now();
    let mut draw_time = std::time::Duration::ZERO;
    let mut draws = 0;
    let mut bot = Bot { path: vec![], goal: (0.0, 0.0), repath: 0 };
    let mut visited = std::collections::HashSet::new();
    let (mut next_action, mut action_shots) = (0u32, 0);
    let mut last_bosses = 0;
    let mut last_dialog = None;
    for t in 0..total {
        let inp = bot.act(&g, t);
        g.update(&inp);
        g.sfx.clear();
        if god && g.p.hp < g.p.max_hp * 0.5 {
            g.p.hp = g.p.max_hp;
        }
        if dir.is_none() && minutes > 6 && g.stats.bosses != last_bosses {
            last_bosses = g.stats.bosses;
            let tokens = g.bot_token().map(|(x, y)| format!("{x:.1},{y:.1} walkable={} path={}", !g.d.blocked(x, y, 0.3), g.bot_path(x, y).is_some()));
            println!("  [{:>3} min] boss down in {} at {:.1},{:.1} (token: {tokens:?}) quest={}", t / 3600, g.level_name, g.p.x, g.p.y, g.quest_log());
        }
        if visited.insert(g.level) && dir.is_none() && minutes > 6 {
            println!("  [{:>3} min] entered {}  quest={}", t / 3600, g.level_name, g.quest_log());
        }
        let dname = g.dialog.as_ref().map(|d| (d.name, d.page, d.pages.len(), d.options.iter().map(|o| o.0.clone()).collect::<Vec<_>>()));
        if dir.is_none() && std::env::var("ASHEN_TRACE").is_ok() && dname != last_dialog {
            println!("    t={t} dialog {:?}", dname);
            last_dialog = dname;
        }
        if dir.is_none() && std::env::var("ASHEN_TRACE").is_ok() && t % 600 == 0 {
            println!(
                "    t={t} {} at {:.1},{:.1} token={:?} dialog={:?} target={:?} hp={:.0} state={:?}",
                g.level_name,
                g.p.x,
                g.p.y,
                g.bot_token(),
                g.dialog.as_ref().map(|d| d.name),
                g.bot_target().map(|t| (t.0 as i32, t.1 as i32, t.2 as i32, t.3)),
                g.p.hp,
                g.state
            );
        }
        if dir.is_none() && minutes > 6 && t > 0 && t % (3600 * 5) == 0 {
            println!("  [{:>3} min] {} at {:.0},{:.0} clvl={} kills={} bosses={} quest={}", t / 3600, g.level_name, g.p.x, g.p.y, g.p.clvl, g.kills, g.stats.bosses, g.quest_log());
        }
        if let Some(d) = dir {
            let action = g.fire_active() && t >= next_action && action_shots < 3;
            if action {
                action_shots += 1;
                next_action = t + 600;
            }
            if shots.contains(&t) || action {
                let t1 = std::time::Instant::now();
                g.draw(&mut scr);
                draw_time += t1.elapsed();
                draws += 1;
                let path = format!("{d}/frame_{t:05}.bmp");
                write_bmp(&path, &scr).expect("write snapshot");
                println!("wrote {path}  {} hp={:.0} foes={} quest={}", g.level_name, g.p.hp, g.alive_mobs(), g.quest.log());
            }
        } else if t % 4 == 0 {
            let t1 = std::time::Instant::now();
            g.draw(&mut scr);
            draw_time += t1.elapsed();
            draws += 1;
        }
        assert!(g.p.x.is_finite() && g.p.y.is_finite(), "player position went non-finite");
        assert!(g.d.walkable(g.p.x.floor() as i32, g.p.y.floor() as i32), "player inside a wall at {:.2},{:.2} in {}", g.p.x, g.p.y, g.level_name);
    }
    println!(
        "selftest ok: {} ticks in {:.2?}, avg draw {:.2?}, kills={} clvl={} seals={} quest={:?} acts2-4={}/{}/{} tokens={}/{}/{} levels visited={} stats={:?}",
        total,
        t0.elapsed(),
        draw_time / draws.max(1),
        g.kills,
        g.p.clvl,
        g.quest.seal_count(),
        g.quest.stage,
        g.quest.stage2,
        g.quest.stage3,
        g.quest.stage4,
        g.quest.rune_count(),
        g.quest.sigil_count(),
        g.quest.key_count(),
        visited.len(),
        g.stats
    );
    if let Some(d) = dir {
        staged(d, h, &mut scr);
    }
    0
}

/// One frame per area and feature, for eyeballing art and layout.
fn staged(d: &str, h: i32, scr: &mut Screen) {
    let save = |g: &mut Game, scr: &mut Screen, name: &str| {
        g.draw(scr);
        write_bmp(&format!("{d}/{name}.bmp"), scr).expect("write snapshot");
        println!("staged {name}: {}", g.level_name);
    };
    let idle = |g: &mut Game, n: u32| {
        for _ in 0..n {
            g.update(&Input::default());
            g.sfx.clear();
        }
    };
    // Town, with the title banner, then a conversation.
    let mut g = Game::new(7, h);
    idle(&mut g, 30);
    save(&mut g, scr, "town");
    idle(&mut g, 400);
    g.debug_talk(Role::Elder);
    save(&mut g, scr, "dialog");
    g.dialog = None;
    g.debug_talk(Role::Merchant);
    save(&mut g, scr, "shop");
    g.dialog = None;
    // The wilds next to a pack.
    if let Some((x, y)) = g.bot_target().map(|t| (t.0, t.1)) {
        g.p.x = x + 4.0;
        g.p.y = y + 4.0;
        if g.d.blocked(g.p.x, g.p.y, 0.3) {
            g.p.x = x;
            g.p.y = y + 2.0;
        }
        idle(&mut g, 40);
        save(&mut g, scr, "wilds");
    }
    // Each dungeon's first floor, and its boss.
    for k in 0..DUNGEONS.len() {
        g.debug_goto(LevelId::Dungeon(k, 0));
        idle(&mut g, 20);
        save(&mut g, scr, &format!("dungeon{k}"));
        g.debug_goto(LevelId::Dungeon(k, DUNGEONS[k].floors - 1));
        g.p.max_hp = 9999.0;
        g.p.hp = 9999.0;
        if g.debug_near_boss() {
            for t in 0..150 {
                g.update(&Input { cast: t % 30 < 10, ..Input::default() });
                g.sfx.clear();
            }
            save(&mut g, scr, &format!("boss{k}"));
        }
    }
    // The ending.
    g.debug_kill_boss();
    idle(&mut g, 60 * 6);
    save(&mut g, scr, "victory");
    // Skills: the tree, Inferno into a pack, Fire Nova.
    {
        use crate::skills::Skill;
        let mut g = Game::new(7, h);
        g.debug_goto(LevelId::Dungeon(0, 0));
        idle(&mut g, 200);
        g.p.clvl = 6;
        g.p.skills.points = 3;
        g.update(&Input { skills: true, ..Input::default() });
        g.tree.as_mut().unwrap().sel = Skill::Inferno as usize;
        save(&mut g, scr, "skills_tree");
        g.tree = None;
        g.p.skills.rank[Skill::Inferno as usize] = 3;
        g.p.skills.rank[Skill::FireNova as usize] = 2;
        g.p.skills.primary = Skill::Inferno;
        g.p.skills.secondary = Skill::FireNova;
        g.p.mana = 50.0;
        if let Some((x, y)) = g.bot_target().map(|t| (t.0, t.1)) {
            let (px, py) = (g.p.x, g.p.y);
            let _ = (px, py);
            g.debug_place_near(x, y, 2.2);
        }
        for _ in 0..30 {
            let (sx, sy) = g.bot_target().map(|t| crate::iso::to_screen(t.0 - g.p.x, t.1 - g.p.y)).unwrap_or((1.0, 0.0));
            let l = (sx * sx + sy * sy).sqrt().max(0.01);
            g.update(&Input { cast: true, aim_x: sx / l, aim_y: sy / l, ..Input::default() });
            g.sfx.clear();
        }
        save(&mut g, scr, "skills_inferno");
        g.p.mana = 50.0;
        g.update(&Input { cast2: true, ..Input::default() });
        idle(&mut g, 10);
        save(&mut g, scr, "skills_nova");
        // Step 2: Fire Wall across the nearest pack, then Combust; Blaze trail while running.
        g.p.clvl = 12;
        g.p.skills.rank[Skill::FireWall as usize] = 3;
        g.p.skills.rank[Skill::Warmth as usize] = 1;
        g.p.skills.rank[Skill::Blaze as usize] = 2;
        g.p.skills.rank[Skill::Combust as usize] = 2;
        g.p.mana = 80.0;
        g.p.max_mana = 80.0;
        if let Some((x, y)) = g.bot_target().map(|t| (t.0, t.1)) {
            g.debug_place_near(x, y, 3.5);
            g.p.cast_cd = 0.0;
            g.cast_skill(Skill::FireWall, x, y);
        }
        idle(&mut g, 45);
        save(&mut g, scr, "skills_firewall");
        g.p.cast_cd = 0.0;
        g.cast_skill(Skill::Combust, g.p.x, g.p.y);
        idle(&mut g, 6);
        save(&mut g, scr, "skills_combust");
        g.p.cast_cd = 0.0;
        g.cast_skill(Skill::Blaze, g.p.x, g.p.y);
        for t in 0..50 {
            g.update(&Input { move_x: if t < 25 { 1.0 } else { 0.3 }, move_y: if t < 25 { 0.2 } else { 1.0 }, ..Input::default() });
            g.sfx.clear();
        }
        save(&mut g, scr, "skills_blaze");
        g.p.skills.points = 2;
        g.update(&Input { skills: true, ..Input::default() });
        g.tree.as_mut().unwrap().sel = Skill::Combust as usize;
        save(&mut g, scr, "skills_tree2");
        g.tree = None;
        // Step 3: a meteor on its way down, a hydra, Ash Phoenix wings, and the full tree.
        g.p.clvl = 18;
        for sk in [Skill::Meteor, Skill::Mastery, Skill::Hydra, Skill::Phoenix] {
            g.p.skills.rank[sk as usize] = 2;
        }
        g.p.mana = 200.0;
        g.p.max_mana = 200.0;
        let target = g.bot_target().map(|t| (t.0, t.1)).unwrap_or((g.p.x + 3.0, g.p.y));
        g.debug_place_near(target.0, target.1, 4.0);
        g.p.cast_cd = 0.0;
        g.cast_skill(Skill::Meteor, target.0, target.1);
        idle(&mut g, 35);
        save(&mut g, scr, "skills_meteor");
        idle(&mut g, 30);
        g.p.cast_cd = 0.0;
        g.cast_skill(Skill::Hydra, target.0, target.1);
        idle(&mut g, 70);
        save(&mut g, scr, "skills_hydra");
        g.p.cast_cd = 0.0;
        g.cast_skill(Skill::Phoenix, g.p.x, g.p.y);
        for _ in 0..20 {
            g.update(&Input { move_x: 0.6, move_y: 0.4, ..Input::default() });
            g.sfx.clear();
        }
        save(&mut g, scr, "skills_phoenix");
        g.p.skills.points = 1;
        g.update(&Input { skills: true, ..Input::default() });
        g.tree.as_mut().unwrap().sel = Skill::Phoenix as usize;
        save(&mut g, scr, "skills_tree3");
        g.tree = None;
    }
    // The front end: title, character select (with a delete prompt), class select, naming, options.
    {
        use crate::menu::{Menu, Stage};
        use crate::save::HeroInfo;
        use crate::skills::Class;
        let g = Game::new(7, h);
        let heroes = vec![
            HeroInfo { slot: "morwen".into(), name: "MORWEN".into(), class: Class::Sorceress, clvl: 21, act: 1, difficulty: 0 },
            HeroInfo { slot: "carmilla".into(), name: "CARMILLA".into(), class: Class::Vampire, clvl: 9, act: 0, difficulty: 0 },
            HeroInfo { slot: "isolde".into(), name: "ISOLDE".into(), class: Class::Sorceress, clvl: 34, act: 1, difficulty: 1 },
        ];
        let mut m = Menu::new(heroes);
        let shot = |m: &mut Menu, scr: &mut Screen, name: &str| {
            for _ in 0..90 {
                m.update(&Input::default());
            }
            m.draw(scr, &g.art);
            write_bmp(&format!("{d}/{name}.bmp"), scr).expect("write snapshot");
            println!("staged {name}");
        };
        shot(&mut m, scr, "menu_title");
        m.stage = Stage::Heroes;
        m.sel = 1;
        shot(&mut m, scr, "menu_heroes");
        {
            // A full roster: the list scrolls and the buttons stay on screen.
            let classes = crate::skills::ALL_CLASSES;
            let many: Vec<HeroInfo> = (0..10)
                .map(|k| HeroInfo { slot: format!("h{k}"), name: format!("HERO {}", k + 1), class: classes[k % classes.len()], clvl: 5 + k as u32 * 3, act: k % 4, difficulty: 0 })
                .collect();
            let mut mm = Menu::new(many);
            mm.stage = Stage::Heroes;
            mm.sel = 9;
            shot(&mut mm, scr, "menu_heroes_many");
        }
        m.stage = Stage::Delete(0);
        m.sel = 1;
        shot(&mut m, scr, "menu_delete");
        m.stage = Stage::Create;
        m.sel = 1;
        shot(&mut m, scr, "menu_create");
        m.stage = Stage::Name;
        m.class_sel = 1;
        m.name = "CARMILLA".into();
        m.sel = 0;
        shot(&mut m, scr, "menu_name");
        m.stage = Stage::Options;
        m.sel = 0;
        shot(&mut m, scr, "menu_options");
    }
    // The class select screen, then the vampire at work in the crypt.
    {
        use crate::skills::{Class, Skill};
        let mut g = Game::new(7, h);
        g.choose = Some(1);
        save(&mut g, scr, "choose_class");
        g.choose = None;
        g.set_class(Class::Vampire);
        g.p.clvl = 18;
        for s in crate::skills::VAMPIRE {
            g.p.skills.rank[s as usize] = 3;
        }
        g.p.base_mana = 400.0;
        g.p.base_hp = 9000.0;
        g.p.recalc();
        g.p.mana = 400.0;
        g.p.hp = 9000.0;
        g.p.skills.primary = Skill::BloodLance;
        g.p.skills.secondary = Skill::BatSwarm;
        g.debug_goto(LevelId::Dungeon(1, 0));
        g.banner_t = 0.0;
        idle(&mut g, 100);
        let target = g.bot_target().map(|t| (t.0, t.1)).unwrap_or((g.p.x + 3.0, g.p.y));
        g.debug_place_near(target.0, target.1, 3.0);
        idle(&mut g, 10);
        save(&mut g, scr, "vampire_idle");
        g.p.cast_cd = 0.0;
        g.cast_skill(Skill::BatSwarm, target.0, target.1);
        for _ in 0..6 {
            g.p.cast_cd = 0.0;
            g.cast_skill(Skill::BloodLance, target.0, target.1);
            idle(&mut g, 4);
        }
        save(&mut g, scr, "vampire_bats");
        g.p.cast_cd = 0.0;
        g.cast_skill(Skill::Mesmerize, target.0, target.1);
        g.p.cast_cd = 0.0;
        g.p.skills.cooldown = [0.0; crate::skills::ALL.len()];
        g.cast_skill(Skill::BloodMoon, target.0, target.1);
        idle(&mut g, 40);
        save(&mut g, scr, "vampire_moon");
        g.p.cast_cd = 0.0;
        g.cast_skill(Skill::Embrace, g.p.x, g.p.y);
        idle(&mut g, 20);
        g.p.cast_cd = 0.0;
        g.cast_skill(Skill::CrimsonNova, g.p.x, g.p.y);
        idle(&mut g, 8);
        save(&mut g, scr, "vampire_embrace");
        g.p.skills.points = 2;
        g.update(&Input { skills: true, ..Input::default() });
        save(&mut g, scr, "vampire_tree");
        g.tree = None;
    }
    // The inventor: gadgets everywhere.
    {
        use crate::skills::{Class, Skill};
        let mut g = Game::new(7, h);
        g.set_class(Class::Inventor);
        g.p.clvl = 18;
        for s in crate::skills::INVENTOR {
            g.p.skills.rank[s as usize] = 3;
        }
        g.p.base_hp = 9000.0;
        g.p.recalc();
        g.p.hp = 9000.0;
        g.p.skills.primary = Skill::RayPistol;
        g.p.skills.secondary = Skill::ClockBomb;
        g.debug_goto(LevelId::Dungeon(0, 0));
        g.banner_t = 0.0;
        idle(&mut g, 100);
        let target = g.bot_target().map(|t| (t.0, t.1)).unwrap_or((g.p.x + 3.0, g.p.y));
        g.debug_place_near(target.0, target.1, 4.0);
        let cast = |g: &mut Game, s: Skill, x: f32, y: f32| {
            g.p.mana = g.p.max_mana * 0.5;
            g.p.overheat = 0.0;
            g.p.cast_cd = 0.0;
            g.p.skills.cooldown = [0.0; crate::skills::ALL.len()];
            g.cast_skill(s, x, y);
        };
        cast(&mut g, Skill::Turret, target.0, target.1);
        cast(&mut g, Skill::Spider, target.0, target.1);
        idle(&mut g, 30);
        cast(&mut g, Skill::ClockBomb, target.0, target.1);
        idle(&mut g, 30);
        cast(&mut g, Skill::ArcCoil, target.0, target.1);
        cast(&mut g, Skill::RayPistol, target.0, target.1);
        idle(&mut g, 3);
        g.p.mana = g.p.max_mana * 0.2;
        save(&mut g, scr, "inventor_gadgets");
        cast(&mut g, Skill::AirshipStrike, target.0, target.1);
        cast(&mut g, Skill::TeslaField, target.0, target.1);
        idle(&mut g, 25);
        save(&mut g, scr, "inventor_airship");
        let (px, py) = (g.p.x, g.p.y);
        cast(&mut g, Skill::SteamSuit, px, py);
        idle(&mut g, 10);
        save(&mut g, scr, "inventor_suit");
        g.p.skills.points = 2;
        g.update(&Input { skills: true, ..Input::default() });
        save(&mut g, scr, "inventor_tree");
        g.tree = None;
    }
    // The Valkyrie: spear and frost, a frozen foe, her wings, the warhorse, the blizzard.
    {
        use crate::skills::{Class, Skill};
        let mut g = Game::new(7, h);
        g.set_class(Class::Valkyrie);
        g.p.clvl = 18;
        for s in crate::skills::VALKYRIE {
            g.p.skills.rank[s as usize] = 3;
        }
        g.p.base_hp = 9000.0;
        g.p.recalc();
        g.p.hp = 9000.0;
        g.p.skills.primary = Skill::RuneSpear;
        g.p.skills.secondary = Skill::RimeSweep;
        g.debug_goto(LevelId::Dungeon(0, 0));
        g.banner_t = 0.0;
        idle(&mut g, 100);
        let target = g.bot_target().map(|t| (t.0, t.1)).unwrap_or((g.p.x + 3.0, g.p.y));
        g.debug_place_near(target.0, target.1, 2.0);
        let cast = |g: &mut Game, s: Skill, x: f32, y: f32| {
            g.p.mana = g.p.max_mana * 0.6;
            g.p.cast_cd = 0.0;
            g.p.skills.cooldown = [0.0; crate::skills::ALL.len()];
            g.cast_skill(s, x, y);
        };
        cast(&mut g, Skill::RimeSweep, target.0, target.1);
        idle(&mut g, 4);
        save(&mut g, scr, "valkyrie_sweep");
        // Freeze whoever is nearest.
        if let Some(i) = g.mobs.iter().position(|m| m.alive() && (m.x - g.p.x).powi(2) + (m.y - g.p.y).powi(2) < 25.0) {
            g.add_frost(i, 1.0);
        }
        cast(&mut g, Skill::RavenStrike, target.0, target.1);
        idle(&mut g, 12);
        save(&mut g, scr, "valkyrie_frozen");
        cast(&mut g, Skill::GlacierLeap, target.0 + 1.0, target.1 + 1.0);
        idle(&mut g, 4);
        save(&mut g, scr, "valkyrie_leap");
        let (px, py) = (g.p.x, g.p.y);
        cast(&mut g, Skill::Einherjar, px, py);
        cast(&mut g, Skill::Fimbulwinter, px, py);
        idle(&mut g, 40);
        save(&mut g, scr, "valkyrie_fimbulwinter");
        g.p.fimbul_t = 0.0;
        g.p.wings_t = 0.0;
        cast(&mut g, Skill::ValkyrieRide, target.0, target.1);
        idle(&mut g, 8);
        save(&mut g, scr, "valkyrie_ride");
        idle(&mut g, 60);
        g.p.mana = g.p.max_mana;
        idle(&mut g, 2);
        save(&mut g, scr, "valkyrie_valor");
        g.p.skills.points = 2;
        g.update(&Input { skills: true, ..Input::default() });
        save(&mut g, scr, "valkyrie_tree");
        g.tree = None;
    }
    // The Berserker: cleave and blood, her wolf, whirlwind, the red mist.
    {
        use crate::skills::{Class, Skill};
        let mut g = Game::new(7, h);
        g.set_class(Class::Berserker);
        g.p.clvl = 18;
        for s in crate::skills::BERSERKER {
            g.p.skills.rank[s as usize] = 3;
        }
        g.p.base_hp = 9000.0;
        g.p.recalc();
        g.p.hp = 9000.0;
        g.p.skills.primary = Skill::Cleave;
        g.p.skills.secondary = Skill::Whirlwind;
        g.debug_goto(LevelId::Dungeon(0, 0));
        g.banner_t = 0.0;
        idle(&mut g, 100);
        let target = g.bot_target().map(|t| (t.0, t.1)).unwrap_or((g.p.x + 3.0, g.p.y));
        g.debug_place_near(target.0, target.1, 1.6);
        idle(&mut g, 10);
        let cast = |g: &mut Game, s: Skill, x: f32, y: f32| {
            g.p.mana = g.p.max_mana * 0.6;
            g.p.cast_cd = 0.0;
            g.p.skills.cooldown = [0.0; crate::skills::ALL.len()];
            g.cast_skill(s, x, y);
        };
        cast(&mut g, Skill::Rend, target.0, target.1);
        idle(&mut g, 6);
        save(&mut g, scr, "berserker_rend");
        cast(&mut g, Skill::Whirlwind, target.0, target.1);
        idle(&mut g, 30);
        save(&mut g, scr, "berserker_whirlwind");
        idle(&mut g, 120);
        let (px, py) = (g.p.x, g.p.y);
        cast(&mut g, Skill::WarCry, px, py);
        cast(&mut g, Skill::Berserk, px, py);
        idle(&mut g, 8);
        save(&mut g, scr, "berserker_berserk");
        cast(&mut g, Skill::HurlAxe, target.0, target.1);
        idle(&mut g, 10);
        save(&mut g, scr, "berserker_axe");
        g.p.skills.points = 2;
        g.update(&Input { skills: true, ..Input::default() });
        save(&mut g, scr, "berserker_tree");
        g.tree = None;
    }
    // The Inquisitor: the censer on its chain, brands, the lash, binding chains, the sweep, Final Judgment.
    {
        use crate::skills::{Class, Skill};
        let mut g = Game::new(7, h);
        g.set_class(Class::Inquisitor);
        g.p.clvl = 18;
        for s in crate::skills::INQUISITOR {
            g.p.skills.rank[s as usize] = 3;
        }
        g.p.base_hp = 9000.0;
        g.p.recalc();
        g.p.hp = 9000.0;
        g.p.skills.primary = Skill::CenserStrike;
        g.p.skills.secondary = Skill::BrandOfJudgment;
        g.debug_goto(LevelId::Dungeon(0, 0));
        g.banner_t = 0.0;
        idle(&mut g, 100);
        let target = g.bot_target().map(|t| (t.0, t.1)).unwrap_or((g.p.x + 3.0, g.p.y));
        g.debug_place_near(target.0, target.1, 2.4);
        idle(&mut g, 10);
        let cast = |g: &mut Game, s: Skill, x: f32, y: f32| {
            g.p.mana = g.p.max_mana * 0.8;
            g.p.cast_cd = 0.0;
            g.p.skills.cooldown = [0.0; crate::skills::ALL.len()];
            g.cast_skill(s, x, y);
        };
        let target = g.bot_target().map(|t| (t.0, t.1)).unwrap_or(target);
        cast(&mut g, Skill::BrandOfJudgment, target.0, target.1);
        idle(&mut g, 4);
        cast(&mut g, Skill::CenserStrike, target.0, target.1);
        idle(&mut g, 3);
        save(&mut g, scr, "inquisitor_strike");
        idle(&mut g, 7);
        save(&mut g, scr, "inquisitor_strike_crack");
        let target = g.bot_target().map(|t| (t.0, t.1)).unwrap_or(target);
        cast(&mut g, Skill::ChainLash, target.0, target.1);
        idle(&mut g, 4);
        save(&mut g, scr, "inquisitor_lash");
        let target = g.bot_target().map(|t| (t.0, t.1)).unwrap_or(target);
        cast(&mut g, Skill::BindingChains, target.0, target.1);
        idle(&mut g, 10);
        save(&mut g, scr, "inquisitor_bind");
        let (px, py) = (g.p.x, g.p.y);
        cast(&mut g, Skill::CenserSweep, px, py);
        idle(&mut g, 20);
        save(&mut g, scr, "inquisitor_sweep");
        idle(&mut g, 120);
        let (px, py) = (g.p.x, g.p.y);
        cast(&mut g, Skill::Purification, px, py);
        cast(&mut g, Skill::FinalJudgment, px, py);
        idle(&mut g, 8);
        save(&mut g, scr, "inquisitor_judgment");
        // Walking toward the camera (the user saw her legs float here).
        g.p.judge_t = 0.0;
        for _ in 0..14 {
            g.update(&Input { move_y: 1.0, ..Input::default() });
            g.sfx.clear();
        }
        save(&mut g, scr, "inquisitor_walk");
        g.p.skills.points = 2;
        g.update(&Input { skills: true, ..Input::default() });
        save(&mut g, scr, "inquisitor_tree");
        g.tree = None;
    }
    // The Reaper: the scythe and its runes, souls, chains, the hourglass, the open Ledger.
    {
        use crate::skills::{Class, Skill};
        let mut g = Game::new(7, h);
        g.set_class(Class::Reaper);
        g.p.clvl = 18;
        for s in crate::skills::REAPER {
            g.p.skills.rank[s as usize] = 3;
        }
        g.p.base_hp = 9000.0;
        g.p.recalc();
        g.p.hp = 9000.0;
        g.p.skills.primary = Skill::ReapingScythe;
        g.p.skills.secondary = Skill::ChainsOfArchive;
        g.debug_goto(LevelId::Dungeon(0, 0));
        g.banner_t = 0.0;
        idle(&mut g, 100);
        let target = g.bot_target().map(|t| (t.0, t.1)).unwrap_or((g.p.x + 3.0, g.p.y));
        g.debug_place_near(target.0, target.1, 2.0);
        idle(&mut g, 6);
        let cast = |g: &mut Game, s: Skill, x: f32, y: f32| {
            g.p.mana = g.soul_cap() * 0.7;
            g.p.cast_cd = 0.0;
            g.p.skills.cooldown = [0.0; crate::skills::ALL.len()];
            g.cast_skill(s, x, y);
        };
        save(&mut g, scr, "reaper_idle");
        for _ in 0..4 {
            cast(&mut g, Skill::ReapingScythe, target.0, target.1);
            idle(&mut g, 2);
        }
        save(&mut g, scr, "reaper_swing");
        cast(&mut g, Skill::ChainsOfArchive, target.0, target.1);
        cast(&mut g, Skill::Hourglass, target.0, target.1);
        cast(&mut g, Skill::SpiritLantern, target.0, target.1);
        idle(&mut g, 8);
        save(&mut g, scr, "reaper_chains");
        let (px, py) = (g.p.x, g.p.y);
        cast(&mut g, Skill::OpenLedger, px, py);
        cast(&mut g, Skill::ScholarSpirits, px, py);
        cast(&mut g, Skill::SoulHarvest, px, py);
        idle(&mut g, 10);
        save(&mut g, scr, "reaper_ledger");
        g.p.skills.points = 2;
        g.update(&Input { skills: true, ..Input::default() });
        save(&mut g, scr, "reaper_tree");
        g.tree = None;
    }
    // The Druid: spores and thorns, her creatures, the balance bar.
    {
        use crate::skills::{Class, Skill};
        let mut g = Game::new(7, h);
        g.set_class(Class::Druid);
        g.p.clvl = 18;
        g.p.base_mana = 150.0;
        for s in crate::skills::DRUID {
            g.p.skills.rank[s as usize] = 3;
        }
        g.p.base_hp = 9000.0;
        g.p.recalc();
        g.p.hp = 9000.0;
        g.p.skills.primary = Skill::SporeCloud;
        g.p.skills.secondary = Skill::RatSwarm;
        g.debug_goto(LevelId::Dungeon(0, 0));
        g.banner_t = 0.0;
        idle(&mut g, 100);
        let target = g.bot_target().map(|t| (t.0, t.1)).unwrap_or((g.p.x + 3.0, g.p.y));
        g.debug_place_near(target.0, target.1, 3.0);
        idle(&mut g, 4);
        let cast = |g: &mut Game, s: Skill, x: f32, y: f32| {
            g.p.mana = g.p.max_mana;
            g.p.cast_cd = 0.0;
            g.p.skills.cooldown = [0.0; crate::skills::ALL.len()];
            g.cast_skill(s, x, y);
        };
        let (px, py) = (g.p.x, g.p.y);
        cast(&mut g, Skill::MossWolf, px, py);
        cast(&mut g, Skill::RatSwarm, px, py);
        cast(&mut g, Skill::SporeCloud, target.0, target.1);
        cast(&mut g, Skill::ThornLash, target.0, target.1);
        cast(&mut g, Skill::FungalBloom, target.0 + 1.0, target.1);
        idle(&mut g, 30);
        save(&mut g, scr, "druid_plague");
        cast(&mut g, Skill::ThornWarden, px, py);
        cast(&mut g, Skill::Pestilence, px, py);
        idle(&mut g, 20);
        save(&mut g, scr, "druid_warden");
        g.p.skills.points = 2;
        g.update(&Input { skills: true, ..Input::default() });
        save(&mut g, scr, "druid_tree");
        g.tree = None;
    }
    // Act 2: Kaldholm, the Frostmarch, each ice dungeon and its herald.
    {
        let mut g = Game::new(7, h);
        g.quest.stage = 3;
        g.debug_goto(LevelId::Frostmarch);
        (g.p.x, g.p.y) = g.start;
        g.banner_t = 0.0;
        idle(&mut g, 60);
        save(&mut g, scr, "act2_town");
        g.debug_talk(Role::Captain);
        save(&mut g, scr, "act2_dialog");
        g.dialog = None;
        if let Some((x, y)) = g.bot_target().map(|t| (t.0, t.1)) {
            g.debug_place_near(x, y, 4.0);
            idle(&mut g, 40);
            save(&mut g, scr, "act2_wilds");
        }
        for k in 4..8 {
            g.debug_goto(LevelId::Dungeon(k, 0));
            g.banner_t = 0.0;
            idle(&mut g, 20);
            save(&mut g, scr, &format!("dungeon{k}"));
            g.debug_goto(LevelId::Dungeon(k, DUNGEONS[k].floors - 1));
            g.p.base_hp = 9999.0;
            g.p.recalc();
            g.p.hp = 9999.0;
            if g.debug_near_boss() {
                for t in 0..170 {
                    g.update(&Input { cast: t % 30 < 10, ..Input::default() });
                    g.sfx.clear();
                }
                save(&mut g, scr, &format!("boss{k}"));
            }
        }
    }
    // Act 3: Mournhold, the Mistwood, and Count Vardak's mist and bat forms.
    {
        let mut g = Game::new(7, h);
        g.quest.stage = 3;
        g.quest.stage2 = 3;
        g.debug_goto(LevelId::Mistwood);
        (g.p.x, g.p.y) = g.start;
        g.banner_t = 0.0;
        idle(&mut g, 60);
        save(&mut g, scr, "act3_town");
        g.debug_talk(Role::Hunter);
        save(&mut g, scr, "act3_dialog");
        g.dialog = None;
        if let Some((x, y)) = g.bot_target().map(|t| (t.0, t.1)) {
            g.debug_place_near(x, y, 4.0);
            idle(&mut g, 40);
            save(&mut g, scr, "act3_wilds");
        }
        // (The dungeons and heralds are staged with the others; here, the count's later forms.)
        let k = crate::world::CASTLE;
        g.debug_goto(LevelId::Dungeon(k, DUNGEONS[k].floors - 1));
        g.p.base_hp = 9999.0;
        g.p.recalc();
        g.p.hp = 9999.0;
        g.debug_near_boss();
        idle(&mut g, 20);
        // The count's mist and bat forms.
        if let Some(i) = g.mobs.iter().position(|m| m.kind == crate::mobs::Kind::Vardak) {
            g.mobs[i].hp = g.mobs[i].max_hp * 0.5;
            g.mobs[i].special2 = 0.0;
            for _ in 0..30 {
                g.p.hp = 9999.0;
                g.update(&Input::default());
                g.sfx.clear();
            }
            save(&mut g, scr, "vardak_mist");
            g.mobs[i].hp = g.mobs[i].max_hp * 0.2;
            for _ in 0..60 {
                g.p.hp = 9999.0;
                g.update(&Input::default());
                g.sfx.clear();
            }
            save(&mut g, scr, "vardak_bat");
        }
    }
    // Act 4: the Last Escapement, the Grinding Fields, and the Clockmaker's duel and engine.
    {
        let mut g = Game::new(7, h);
        g.quest.stage = 3;
        g.quest.stage2 = 3;
        g.quest.stage3 = 3;
        g.debug_goto(LevelId::Mechanus);
        (g.p.x, g.p.y) = g.start;
        g.banner_t = 0.0;
        idle(&mut g, 60);
        save(&mut g, scr, "act4_town");
        g.debug_talk(Role::Tally);
        save(&mut g, scr, "act4_dialog");
        g.dialog = None;
        if let Some((x, y)) = g.bot_target().map(|t| (t.0, t.1)) {
            g.debug_place_near(x, y, 4.0);
            idle(&mut g, 40);
            save(&mut g, scr, "act4_wilds");
        }
        // Time mechanics: a struck stop-clock beside a law zone, foes crawling in its field.
        if !g.laws.is_empty() && !g.clocks.is_empty() {
            let (zx, zy) = (g.laws[0].x, g.laws[0].y);
            let spot = [(2.5f32, 1.0f32), (-2.5, 1.0), (1.0, 2.5), (1.0, -2.5)].into_iter().map(|(dx, dy)| (zx + dx, zy + dy)).find(|&(x, y)| !g.d.blocked(x, y, 0.6));
            if let Some((cx, cy)) = spot {
                (g.clocks[0].x, g.clocks[0].y) = (cx, cy);
                g.mobs.retain(|m| (m.x - zx).powi(2) + (m.y - zy).powi(2) > 400.0);
                for k in 0..4 {
                    let a = k as f32 * 1.6;
                    let (mx, my) = (zx + a.cos() * 2.0, zy + a.sin() * 2.0);
                    if !g.d.blocked(mx, my, 0.4) {
                        let m = crate::mobs::Mob::new(crate::mobs::Kind::Scarab, mx, my, 7.0, &mut g.rng);
                        g.mobs.push(m);
                    }
                }
                (g.p.x, g.p.y) = (cx, cy + 0.5);
                g.p.base_hp = 9999.0;
                g.p.recalc();
                g.p.hp = 9999.0;
                idle(&mut g, 2);
                // Step back so the clock shows.
                (g.p.x, g.p.y) = (cx + 1.5, cy + 1.5);
                idle(&mut g, 30);
                save(&mut g, scr, "act4_clock_law");
            }
        }
        // An ordinal squad marching in ranks behind its marshal, crows wheeling in.
        if let Some(mi) = g.mobs.iter().position(|m| m.kind == crate::mobs::Kind::Marshal && m.alive()) {
            let (mx, my) = (g.mobs[mi].x, g.mobs[mi].y);
            let spot = [(5.0f32, 0.0f32), (0.0, 5.0), (-5.0, 0.0), (0.0, -5.0), (4.0, 3.0)].into_iter().map(|(dx, dy)| (mx + dx, my + dy)).find(|&(x, y)| !g.d.blocked(x, y, 0.4));
            if let Some((x, y)) = spot {
                (g.p.x, g.p.y) = (x, y);
                g.clocks.clear();
                g.laws.clear();
                for k in 0..6 {
                    let (cx, cy) = (x + 3.0 + (k % 3) as f32 * 0.6, y - 2.0 + (k / 3) as f32 * 0.7);
                    if !g.d.blocked(cx, cy, 0.3) {
                        let c = crate::mobs::Mob::new(crate::mobs::Kind::ClockCrow, cx, cy, 7.0, &mut g.rng);
                        g.mobs.push(c);
                    }
                }
                for _ in 0..50 {
                    g.p.hp = g.p.max_hp;
                    g.update(&Input::default());
                    g.sfx.clear();
                }
                save(&mut g, scr, "act4_ordinals");
            }
        }
        let k = crate::world::HEART;
        g.debug_goto(LevelId::Dungeon(k, DUNGEONS[k].floors - 1));
        g.p.base_hp = 9999.0;
        g.p.recalc();
        g.p.hp = 9999.0;
        g.debug_near_boss();
        idle(&mut g, 20);
        if let Some(i) = g.mobs.iter().position(|m| m.kind == crate::mobs::Kind::Clockmaker) {
            g.mobs[i].special = 0.0;
            for _ in 0..12 {
                g.p.hp = 9999.0;
                g.update(&Input::default());
                g.sfx.clear();
            }
            save(&mut g, scr, "clockmaker_duel");
            g.mobs[i].hp = g.mobs[i].max_hp * 0.4;
            for _ in 0..90 {
                g.p.hp = 9999.0;
                g.update(&Input::default());
                g.sfx.clear();
            }
            save(&mut g, scr, "clockmaker_engine");
        }
    }
    // A waypoint in a dungeon, with the travel menu open.
    {
        let mut g = Game::new(7, h);
        g.waypoints.push(LevelId::Dungeon(0, 0));
        g.waypoints.push(LevelId::Dungeon(2, 1));
        g.debug_goto(LevelId::Dungeon(0, 1));
        g.banner_t = 0.0;
        let (wx, wy) = g.waypoint;
        g.p.x = wx + 1.3;
        g.p.y = wy + 0.4;
        idle(&mut g, 30);
        g.p.x = wx;
        g.p.y = wy;
        g.update(&Input::default());
        g.sfx.clear();
        idle(&mut g, 20);
        save(&mut g, scr, "waypoint");
        g.dialog = None;
    }
    // An elite pack: gold-named leader with minions, its name and modifiers on the bar.
    {
        let mut g = Game::new(7, h);
        g.debug_goto(LevelId::Dungeon(0, 1));
        g.p.max_hp = 9999.0;
        g.p.hp = 9999.0;
        let e = g.mobs.iter().position(|m| m.rank == crate::mobs::Rank::Elite);
        if let Some(i) = e {
            let (x, y) = (g.mobs[i].x, g.mobs[i].y);
            g.debug_place_near(x, y, 3.0);
            g.banner_t = 0.0;
            for _ in 0..40 {
                g.update(&Input::default());
                g.sfx.clear();
            }
            g.focus = Some(i);
            g.focus_t = 3.0;
            save(&mut g, scr, "elite");
        }
    }
    // Equipment: loot on the floor, then the inventory with gear worn and a full-ish bag.
    {
        use crate::items::{self, Rarity};
        let mut g = Game::new(11, h);
        g.debug_goto(LevelId::Dungeon(1, 0));
        idle(&mut g, 60);
        let (px, py) = (g.p.x, g.p.y);
        let mut rng = crate::rng::Rng::new(4);
        let loot = [Rarity::Normal, Rarity::Magic, Rarity::Rare, Rarity::Magic, Rarity::Unique];
        for (k, r) in loot.iter().enumerate() {
            let it = if *r == Rarity::Unique { items::unique(5) } else { items::roll(8, *r, &mut rng) };
            let a = k as f32 * 1.25;
            let (x, y) = (px + a.cos() * 1.6, py + a.sin() * 1.6);
            if !g.d.blocked(x, y, 0.2) {
                g.pickups.push(crate::game::Pickup { x, y, kind: crate::game::Drop::Item(Box::new(it)), t: 1.0 });
            }
        }
        g.banner_t = 0.0;
        idle(&mut g, 2);
        save(&mut g, scr, "loot");
        g.pickups.retain(|k| !matches!(k.kind, crate::game::Drop::Item(_)));
        g.p.clvl = 14;
        for (w, r) in [(0, Rarity::Rare), (1, Rarity::Magic), (2, Rarity::Magic), (4, Rarity::Rare), (6, Rarity::Magic)] {
            let slot = items::WORN[w];
            let base = (0..items::BASES.len()).rev().find(|&b| items::BASES[b].slot == slot && items::BASES[b].lvl <= 10 && !items::BASES[b].unique_only).unwrap();
            g.p.gear.worn[w] = Some(items::roll_base(base, 10, r, &mut rng));
        }
        g.p.gear.worn[8] = Some(items::unique(5));
        for i in 0..14 {
            let r = [Rarity::Normal, Rarity::Magic, Rarity::Magic, Rarity::Rare][i % 4];
            g.p.gear.bag[i] = Some(items::roll(3 + i as u8, r, &mut rng));
        }
        g.p.gear.bag[14] = Some(items::unique(2));
        g.p.recalc();
        g.update(&Input { inv: true, ..Input::default() });
        if let Some(ui) = g.inv.as_mut() {
            ui.sel = crate::inventory::Cell::Bag(3);
        }
        save(&mut g, scr, "inventory");
        g.inv.as_mut().unwrap().sel = crate::inventory::Cell::Bag(14);
        save(&mut g, scr, "inventory_unique");
        g.inv = None;
        // The stash, in town.
        g.debug_goto(LevelId::Overworld);
        (g.p.x, g.p.y) = g.start;
        for i in 0..7 {
            g.p.gear.stash[i] = Some(items::roll(6 + i as u8, [Rarity::Rare, Rarity::Magic][i % 2], &mut rng));
        }
        g.p.gear.stash[7] = Some(items::unique(3));
        g.open_inventory();
        g.inv.as_mut().unwrap().sel = crate::inventory::Cell::Stash(7);
        save(&mut g, scr, "stash");
        g.inv = None;
        // Trading with Gerta.
        g.p.gold = 900;
        g.restock();
        g.open_inventory();
        let ui = g.inv.as_mut().unwrap();
        ui.shop = true;
        ui.sel = crate::inventory::Cell::Shop(2);
        save(&mut g, scr, "shop_gear");
        g.inv = None;
        // Sets and gems: a set worn, socketed gear, loose gems, and the jeweler.
        let si = items::SETS.iter().position(|d| d.name == "EMBERWEAVE").unwrap();
        g.p.clvl = 20;
        g.p.gear = items::Gear::default();
        for (p, w) in [(0, 0), (1, 1), (2, 2)] {
            g.p.gear.worn[w] = Some(items::set_item(si, p));
        }
        let mut boots = items::roll_base(items::base_by_key("iboots").unwrap(), 10, Rarity::Normal, &mut rng);
        boots.sockets = 1;
        boots.gems = vec![items::Gem { kind: 3, grade: 4 }];
        g.p.gear.worn[4] = Some(boots);
        let mut staff = items::roll_base(items::base_by_key("ember").unwrap(), 16, Rarity::Normal, &mut rng);
        staff.sockets = 3;
        staff.gems = vec![items::Gem { kind: 0, grade: 5 }, items::Gem { kind: 6, grade: 3 }];
        g.p.gear.bag[0] = Some(staff);
        for k in 0..7u8 {
            g.p.gear.bag[2 + k as usize] = Some(items::gem_item(items::Gem { kind: k, grade: 1 + k % 5 }));
        }
        g.p.gear.bag[10] = Some(items::set_item(si, 3));
        g.p.gear.bag[11] = Some(items::set_item(3, 0));
        g.p.recalc();
        g.open_inventory();
        g.inv.as_mut().unwrap().sel = crate::inventory::Cell::Bag(10);
        save(&mut g, scr, "inventory_set");
        g.inv.as_mut().unwrap().sel = crate::inventory::Cell::Bag(0);
        save(&mut g, scr, "inventory_sockets");
        g.inv.as_mut().unwrap().sel = crate::inventory::Cell::Bag(2);
        g.inv.as_mut().unwrap().holding = Some(2);
        save(&mut g, scr, "inventory_gem");
        g.inv = None;
        g.debug_talk(crate::story::Role::Jeweler(0));
        save(&mut g, scr, "jeweler_talk");
        g.dialog = None;
        g.open_inventory();
        let ui = g.inv.as_mut().unwrap();
        ui.jewel = true;
        ui.sel = crate::inventory::Cell::Bag(0);
        save(&mut g, scr, "jeweler_bench");
        g.inv = None;
    }
    // Breakables: a cluster in each act's first dungeon, then one shattering.
    for (act, k) in [(0usize, 0usize), (1, 4), (2, 8), (3, 12)] {
        let mut g = Game::new(7, h);
        g.debug_goto(LevelId::Dungeon(k, 0));
        g.banner_t = 0.0;
        g.p.base_hp = 9999.0;
        g.p.recalc();
        g.p.hp = 9999.0;
        g.mobs.retain(|m| crate::breakables::is_prop(m.kind));
        let Some(i) = (0..g.mobs.len()).min_by(|&a, &b| {
            let da = (g.mobs[a].x - g.start.0).powi(2) + (g.mobs[a].y - g.start.1).powi(2);
            let db = (g.mobs[b].x - g.start.0).powi(2) + (g.mobs[b].y - g.start.1).powi(2);
            da.partial_cmp(&db).unwrap()
        }) else {
            continue;
        };
        let (x, y) = (g.mobs[i].x, g.mobs[i].y);
        g.debug_place_near(x, y, 2.0);
        idle(&mut g, 4);
        save(&mut g, scr, &format!("breakables_act{}", act + 1));
        if act == 0 {
            g.hit_mob(i, 50.0, 0.0, 0.0, None, false);
            idle(&mut g, 4);
            save(&mut g, scr, "breakables_shatter");
        }
    }
    // HUD details: out of mana (EMBER), hungry, low stamina, food on the floor.
    let mut g = Game::new(7, h);
    g.debug_goto(LevelId::Dungeon(0, 0));
    idle(&mut g, 200);
    g.p.mana = 2.0;
    g.p.food = 18.0;
    g.p.stamina = 40.0;
    g.debug_food_nearby();
    save(&mut g, scr, "hud");
}

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
        if g.dialog.is_some() {
            inp.confirm = t % 20 == 0;
            return inp;
        }
        // The elder first, and again with three seals.
        if g.quest.elder_has_news() && g.level == LevelId::Overworld {
            if let Some((ex, ey)) = g.bot_npc(Role::Elder) {
                if (ex - g.p.x).powi(2) + (ey - g.p.y).powi(2) < 2.0 {
                    inp.confirm = t % 10 == 0;
                } else {
                    self.steer(g, t, (ex, ey), &mut inp);
                }
                return inp;
            }
        }
        if g.p.food < 40.0 || g.boss_dead_with_loot() {
            if let Some(f) = g.bot_food() {
                self.steer(g, t, f, &mut inp);
                return inp;
            }
        }
        if let Some((mx, my, dist, visible)) = g.bot_target() {
            if visible && dist < 9.0 && !g.in_safe(g.p.x, g.p.y) {
                inp.cast = true;
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
            if (dist < 6.0 || boss_near) && g.level != LevelId::Overworld {
                self.steer(g, t, (mx, my), &mut inp);
                return inp;
            }
        }
        if let Some(p) = g.bot_portal() {
            self.steer(g, t, p, &mut inp);
        }
        inp
    }
}

pub fn run(dir: Option<&str>, tall: bool) -> i32 {
    let h = if tall { SH_TALL } else { SH_WIDE };
    let mut g = Game::new(7, h);
    let mut scr = Screen::new(h);
    if let Some(d) = dir {
        std::fs::create_dir_all(d).ok();
    }
    let shots = [2u32, 400, 1200, 2400, 3600];
    let total = if dir.is_some() { 3601 } else { 60 * 60 * 6 };
    let t0 = std::time::Instant::now();
    let mut draw_time = std::time::Duration::ZERO;
    let mut draws = 0;
    let mut bot = Bot { path: vec![], goal: (0.0, 0.0), repath: 0 };
    let mut visited = std::collections::HashSet::new();
    let (mut next_action, mut action_shots) = (0u32, 0);
    for t in 0..total {
        let inp = bot.act(&g, t);
        g.update(&inp);
        g.sfx.clear();
        visited.insert(g.level);
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
        "selftest ok: {} ticks in {:.2?}, avg draw {:.2?}, kills={} clvl={} seals={} quest={:?} levels visited={} stats={:?}",
        total,
        t0.elapsed(),
        draw_time / draws.max(1),
        g.kills,
        g.p.clvl,
        g.quest.seal_count(),
        g.quest.stage,
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

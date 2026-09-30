//! Headless verification. `--snapshot <dir>` plays a scripted bot and writes BMP
//! frames; `--selftest` plays several levels with the bot and checks invariants.
use crate::game::{Game, Input, State};
use crate::gfx::{Screen, SH_TALL, SH_WIDE};

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

/// A simple bot: walk toward the nearest foe or unexplored room, cast when a foe is visible,
/// drink potions when low.
fn bot(g: &Game, t: u32) -> Input {
    let mut inp = Input::default();
    let hp = g.p.hp / g.p.max_hp;
    inp.potion_hp = hp < 0.35 && t % 30 == 0;
    inp.potion_mp = g.p.mana < 6.0 && t % 30 == 0;
    inp.confirm = matches!(g.state, State::Cleared | State::Dead(_)) && t % 60 == 0;
    if let Some((mx, my, dist, visible)) = g.bot_target() {
        if visible && dist < 9.0 {
            inp.cast = true;
            // Back off when something is in melee range.
            if dist < 1.6 {
                let (dx, dy) = (g.p.x - mx, g.p.y - my);
                let (sx, sy) = crate::iso::to_screen(dx, dy);
                let l = (sx * sx + sy * sy).sqrt().max(0.01);
                inp.cast = t % 40 < 20;
                inp.move_x = sx / l;
                inp.move_y = sy / l;
            }
        } else {
            // Walk the A* path (click-to-move emulation) toward it.
            if let Some((nx, ny)) = g.bot_step(mx, my) {
                let (sx, sy) = crate::iso::to_screen(nx - g.p.x, ny - g.p.y);
                let l = (sx * sx + sy * sy).sqrt().max(0.01);
                inp.move_x = sx / l;
                inp.move_y = sy / l;
            }
        }
    }
    inp
}

pub fn run(dir: Option<&str>, tall: bool) -> i32 {
    let h = if tall { SH_TALL } else { SH_WIDE };
    let mut g = Game::new(7, h);
    let mut scr = Screen::new(h);
    if let Some(d) = dir {
        std::fs::create_dir_all(d).ok();
    }
    let shots = [2u32, 200, 420, 700, 1000, 1400, 2000, 2800, 3600];
    let total = if dir.is_some() { 3601 } else { 60 * 60 * 6 };
    let t0 = std::time::Instant::now();
    let mut draw_time = std::time::Duration::ZERO;
    let mut max_depth = 1;
    let (mut next_action, mut action_shots) = (0u32, 0);
    for t in 0..total {
        let mut inp = bot(&g, t);
        // Show the automap in one snapshot.
        inp.map = t == 1399 || t == 1401;
        g.update(&inp);
        g.sfx.clear();
        max_depth = max_depth.max(g.depth);
        if let Some(d) = dir {
            let action = g.fire_active() && t >= next_action && action_shots < 4;
            if action {
                action_shots += 1;
                next_action = t + 500;
            }
            if shots.contains(&t) || action {
                let t1 = std::time::Instant::now();
                g.draw(&mut scr);
                draw_time += t1.elapsed();
                let path = format!("{d}/frame_{t:05}.bmp");
                write_bmp(&path, &scr).expect("write snapshot");
                println!("wrote {path}  depth={} hp={:.0} mana={:.0} foes={} state={:?}", g.depth, g.p.hp, g.p.mana, g.alive_mobs(), g.state);
            }
        } else if t % 4 == 0 {
            // Exercise the renderer too.
            let t1 = std::time::Instant::now();
            g.draw(&mut scr);
            draw_time += t1.elapsed();
        }
        assert!(g.p.x.is_finite() && g.p.y.is_finite(), "player position went non-finite");
        assert!(g.d.walkable(g.p.x.floor() as i32, g.p.y.floor() as i32), "player inside a wall at {:.2},{:.2}", g.p.x, g.p.y);
    }
    let draws = if dir.is_some() { shots.len() as u32 } else { total / 4 };
    println!(
        "selftest ok: {} ticks in {:.2?}, avg draw {:.2?}, kills={} max_depth={} stats={:?}",
        total,
        t0.elapsed(),
        draw_time / draws,
        g.kills,
        max_depth,
        g.stats
    );
    0
}

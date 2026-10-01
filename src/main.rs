#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
//! ASHEN SANCTUM - SDL2 front end. Renders a 640x360 (desktop) or 640x480
//! (RG35XX H) software framebuffer, scaled to the window.
mod art;
mod art_gen;
mod audio;
mod dungeon;
mod game;
mod gfx;
mod iso;
mod levels;
mod mobs;
mod render;
mod rng;
mod save;
mod snapshot;
mod sprites;
mod story;
mod world;

use game::{Game, Input};
use sdl2::controller::{Axis, Button, GameController};
use sdl2::event::{Event, WindowEvent};
use sdl2::keyboard::Scancode;
use sdl2::mouse::MouseButton;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::Rect;
use std::time::{Duration, Instant};

#[derive(Default)]
struct Keys {
    up: bool,
    down: bool,
    left: bool,
    right: bool,
    cast: bool,
    shift: bool,
}

#[derive(Default)]
struct Pad {
    lx: f32,
    ly: f32,
    rx: f32,
    ry: f32,
    dup: bool,
    ddown: bool,
    dleft: bool,
    dright: bool,
    cast: bool,
    rt: bool,
}

fn dead(v: i16) -> f32 {
    let f = v as f32 / 32767.0;
    if f.abs() < 0.2 {
        0.0
    } else {
        f
    }
}

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let handheld = cfg!(target_arch = "aarch64");
    let tall = args.iter().any(|a| a == "--tall") || (handheld && !args.iter().any(|a| a == "--wide"));
    let seed_arg = args.iter().position(|a| a == "--seed").and_then(|i| args.get(i + 1)).and_then(|s| s.parse::<u64>().ok());
    if let Some(i) = args.iter().position(|a| a == "--export-levels") {
        // Writes every level as JSON for the level editor (generated, or the hand-made one).
        let dir = args.get(i + 1).map(String::as_str).filter(|d| !d.starts_with("--")).unwrap_or("levels");
        let seed = seed_arg.or_else(|| save::read().as_deref().and_then(save::seed_of)).unwrap_or(7);
        match levels::export_all(dir, seed) {
            Ok(n) => println!("wrote {n} levels to {dir}/ (seed {seed})"),
            Err(e) => {
                eprintln!("export failed: {e}");
                std::process::exit(1);
            }
        }
        return Ok(());
    }
    if let Some(i) = args.iter().position(|a| a == "--snapshot" || a == "--selftest") {
        let dir = args.get(i + 1).map(String::as_str).filter(|d| !d.starts_with("--"));
        let dir = if args[i] == "--snapshot" { Some(dir.unwrap_or("snapshots")) } else { None };
        std::process::exit(snapshot::run(dir, tall));
    }
    let view_h = if tall { gfx::SH_TALL } else { gfx::SH_WIDE };

    let sdl = sdl2::init()?;
    let video = sdl.video()?;
    let controllers = sdl.game_controller().ok();
    let audio_sys = sdl.audio().ok();
    sdl2::hint::set("SDL_RENDER_SCALE_QUALITY", "0");

    let fullscreen = args.iter().any(|a| a == "--fullscreen") || (handheld && !args.iter().any(|a| a == "--windowed"));
    let mut wb = video.window("Ashen Sanctum", gfx::SW as u32 * 2, view_h as u32 * 2);
    wb.position_centered().resizable();
    if fullscreen {
        wb.fullscreen_desktop();
    }
    let window = wb.build().map_err(|e| e.to_string())?;
    let mut canvas = window.into_canvas().present_vsync().build().map_err(|e| e.to_string())?;
    let tc = canvas.texture_creator();
    let mut tex = tc
        .create_texture_streaming(PixelFormatEnum::ARGB8888, gfx::SW as u32, view_h as u32)
        .map_err(|e| e.to_string())?;
    // The game draws its own cursor.
    sdl.mouse().show_cursor(false);

    let mut pads: Vec<GameController> = Vec::new();
    let mut audio = audio_sys.as_ref().and_then(audio::Audio::open);
    let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(1);
    // Continue the saved character (fresh world from the same seed), unless --new.
    let saved = if args.iter().any(|a| a == "--new") { None } else { save::read() };
    let seed = seed_arg.or_else(|| saved.as_deref().and_then(save::seed_of)).unwrap_or(seed);
    let mut game = Game::new(seed, view_h);
    if let Some(text) = saved.as_deref() {
        if save::apply(&mut game, text) {
            game.welcome_back();
        }
    }
    // --level <name>: jump straight into a level to test it (e.g. bone_crypt_floor1).
    if let Some(name) = args.iter().position(|a| a == "--level").and_then(|i| args.get(i + 1)) {
        match levels::id_from_name(name) {
            Some(id) => game.debug_goto(id),
            None => eprintln!("--level: unknown level {name:?} (try overworld, bone_crypt_floor1, ...)"),
        }
    }
    let mut scr = gfx::Screen::new(view_h);
    let mut keys = Keys::default();
    let mut pad = Pad::default();
    let mut inp = Input::default();
    let mut dst = Rect::new(0, 0, 1, 1);
    let mut events = sdl.event_pump()?;
    let step = Duration::from_micros(16_667);
    let mut last = Instant::now();
    let mut acc = Duration::ZERO;
    let mut back_held = false;
    // One-shot presses, cleared after each simulated tick.
    let (mut confirm, mut pot_hp, mut pot_mp, mut map, mut run_toggle, mut cancel) = (false, false, false, false, false, false);

    'main: loop {
        for ev in events.poll_iter() {
            match ev {
                Event::Quit { .. } => break 'main,
                Event::KeyDown { scancode: Some(sc), repeat, .. } => {
                    use Scancode::*;
                    match sc {
                        W | Up => keys.up = true,
                        S | Down => keys.down = true,
                        A | Left => keys.left = true,
                        D | Right => keys.right = true,
                        F | Space => keys.cast = true,
                        R if !repeat => run_toggle = true,
                        LShift | RShift => keys.shift = true,
                        Return | KpEnter if !repeat => confirm = true,
                        Q | Num1 if !repeat => pot_hp = true,
                        E | Num2 if !repeat => pot_mp = true,
                        Tab | M if !repeat => map = true,
                        Escape if !repeat => cancel = true,
                        _ => {}
                    }
                }
                Event::KeyUp { scancode: Some(sc), .. } => {
                    use Scancode::*;
                    match sc {
                        W | Up => keys.up = false,
                        S | Down => keys.down = false,
                        A | Left => keys.left = false,
                        D | Right => keys.right = false,
                        F | Space => keys.cast = false,
                        LShift | RShift => keys.shift = false,
                        _ => {}
                    }
                }
                Event::MouseMotion { x, y, .. } => {
                    if !handheld {
                        inp.mouse = Some(to_fb(x, y, dst, view_h, &canvas));
                    }
                }
                Event::MouseButtonDown { mouse_btn, x, y, .. } => {
                    inp.mouse = Some(to_fb(x, y, dst, view_h, &canvas));
                    match mouse_btn {
                        MouseButton::Left => inp.lmb = true,
                        MouseButton::Right => inp.rmb = true,
                        _ => {}
                    }
                }
                Event::MouseButtonUp { mouse_btn, .. } => match mouse_btn {
                    MouseButton::Left => inp.lmb = false,
                    MouseButton::Right => inp.rmb = false,
                    _ => {}
                },
                Event::ControllerDeviceAdded { which, .. } => {
                    if let Some(c) = controllers.as_ref().and_then(|g| g.open(which).ok()) {
                        pads.push(c);
                    }
                }
                Event::ControllerButtonDown { button, .. } => {
                    match button {
                        Button::Back => back_held = true,
                        // SELECT + START quits (the usual handheld hotkey).
                        Button::Start if back_held => break 'main,
                        Button::Start => confirm = true,
                        Button::A | Button::X | Button::RightShoulder => pad.cast = true,
                        Button::B => run_toggle = true,
                        Button::LeftShoulder => pot_hp = true,
                        Button::Y => pot_mp = true,
                        Button::DPadUp => pad.dup = true,
                        Button::DPadDown => pad.ddown = true,
                        Button::DPadLeft => pad.dleft = true,
                        Button::DPadRight => pad.dright = true,
                        _ => {}
                    }
                    if !handheld {
                        inp.mouse = None; // pad in use: stop steering by the mouse cursor
                    }
                }
                Event::ControllerButtonUp { button, .. } => match button {
                    // SELECT on its own toggles the map (SELECT + START quits above).
                    Button::Back => {
                        back_held = false;
                        map = true;
                    }
                    Button::A | Button::X | Button::RightShoulder => pad.cast = false,
                    Button::DPadUp => pad.dup = false,
                    Button::DPadDown => pad.ddown = false,
                    Button::DPadLeft => pad.dleft = false,
                    Button::DPadRight => pad.dright = false,
                    _ => {}
                },
                Event::ControllerAxisMotion { axis, value, .. } => match axis {
                    Axis::LeftX => pad.lx = dead(value),
                    Axis::LeftY => pad.ly = dead(value),
                    Axis::RightX => pad.rx = dead(value),
                    Axis::RightY => pad.ry = dead(value),
                    Axis::TriggerRight => pad.rt = value > 12000,
                    Axis::TriggerLeft => {
                        if value > 12000 {
                            pot_hp = true;
                        }
                    }
                },
                Event::Window { win_event: WindowEvent::FocusLost, .. } => {
                    keys = Keys::default();
                    pad = Pad::default();
                    inp.lmb = false;
                    inp.rmb = false;
                }
                _ => {}
            }
        }

        let b = |v: bool| if v { 1.0 } else { 0.0 };
        let kx = b(keys.right || pad.dright) - b(keys.left || pad.dleft);
        let ky = b(keys.down || pad.ddown) - b(keys.up || pad.dup);
        inp.move_x = if pad.lx != 0.0 || pad.ly != 0.0 { pad.lx } else { kx };
        inp.move_y = if pad.lx != 0.0 || pad.ly != 0.0 { pad.ly } else { ky };
        inp.aim_x = pad.rx;
        inp.aim_y = pad.ry;
        // Twin-stick: pushing the right stick far casts in that direction.
        let aim_cast = pad.rx * pad.rx + pad.ry * pad.ry > 0.5;
        inp.cast = keys.cast || pad.cast || pad.rt || aim_cast;
        inp.stand = keys.shift;

        let now = Instant::now();
        acc += (now - last).min(Duration::from_millis(100));
        last = now;
        while acc >= step {
            inp.confirm = confirm;
            inp.potion_hp = pot_hp;
            inp.potion_mp = pot_mp;
            inp.map = map;
            inp.run_toggle = run_toggle;
            inp.cancel = cancel;
            game.update(&inp);
            confirm = false;
            pot_hp = false;
            pot_mp = false;
            map = false;
            run_toggle = false;
            cancel = false;
            for s in game.sfx.drain(..) {
                if let Some(a) = audio.as_mut() {
                    a.play(s);
                }
            }
            acc -= step;
        }

        if game.save_due {
            game.save_due = false;
            save::write(&game);
        }
        if game.quit {
            break 'main;
        }
        game.draw(&mut scr);
        if let (Some(m), false) = (inp.mouse, handheld) {
            draw_cursor(&mut scr, m.0, m.1);
        }
        // SAFETY: a Vec<u32> is contiguous; ARGB8888 is a native-endian packed u32 format.
        let bytes = unsafe { std::slice::from_raw_parts(scr.px.as_ptr() as *const u8, scr.px.len() * 4) };
        tex.update(None, bytes, gfx::SW as usize * 4).map_err(|e| e.to_string())?;
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();
        let (ww, wh) = canvas.output_size()?;
        let (gw, gh) = (gfx::SW as u32, view_h as u32);
        let s = (ww / gw).min(wh / gh);
        let (dw, dh) = if s >= 1 {
            (gw * s, gh * s)
        } else {
            let f = (ww as f32 / gw as f32).min(wh as f32 / gh as f32);
            ((gw as f32 * f) as u32, (gh as f32 * f) as u32)
        };
        dst = Rect::new(((ww - dw) / 2) as i32, ((wh - dh) / 2) as i32, dw.max(1), dh.max(1));
        canvas.copy(&tex, None, Some(dst))?;
        canvas.present();
        if acc < step {
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    save::write(&game);
    Ok(())
}

/// Window coordinates -> framebuffer pixels (accounts for HiDPI output scaling).
fn to_fb(x: i32, y: i32, dst: Rect, view_h: i32, canvas: &sdl2::render::Canvas<sdl2::video::Window>) -> (i32, i32) {
    let (ww, _) = canvas.window().size();
    let (ow, _) = canvas.output_size().unwrap_or((ww, 0));
    let k = ow as f32 / ww.max(1) as f32;
    let (x, y) = (x as f32 * k, y as f32 * k);
    let fx = (x - dst.x() as f32) / dst.width() as f32 * gfx::SW as f32;
    let fy = (y - dst.y() as f32) / dst.height() as f32 * view_h as f32;
    (fx as i32, fy as i32)
}

fn draw_cursor(scr: &mut gfx::Screen, x: i32, y: i32) {
    // Small gauntlet-style arrow.
    for i in 0..9 {
        scr.fill(x, y + i, i.min(6) + 1, 1, gfx::BLACK);
    }
    for i in 1..7 {
        scr.fill(x + 1, y + i + 1, (i - 1).min(4), 1, gfx::rgb(0xd8b878));
    }
}

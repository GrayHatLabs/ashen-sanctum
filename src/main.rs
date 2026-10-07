#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
//! ASHEN SANCTUM - SDL2 front end. Renders a 640x360 (desktop) or 640x480
//! (RG35XX H) software framebuffer, scaled to the window.
mod art;
mod art_gen;
mod audio;
mod clockwork;
mod dungeon;
mod game;
mod gfx;
mod inventory;
mod inventor;
mod iso;
mod items;
mod levels;
mod menu;
mod mobs;
mod music;
mod render;
mod rng;
mod save;
mod skills;
mod berserker;
mod breakables;
mod reaper;
mod druid;
mod inquisitor;
mod valkyrie;
mod vampire;
mod snapshot;
mod sprites;
mod story;
mod tides;
mod sky;
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
    cast2: bool,
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
    cast2: bool,
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
    // --export-music [dir]: write the music loops as WAV files (to listen to them outside the game).
    if let Some(i) = args.iter().position(|a| a == "--export-music") {
        let dir = args.get(i + 1).map(String::as_str).filter(|d| !d.starts_with("--")).unwrap_or("music");
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        for t in music::TRACKS {
            let s = music::render(t);
            let path = format!("{dir}/{}.wav", format!("{t:?}").to_lowercase());
            std::fs::write(&path, music::wav(&s)).map_err(|e| e.to_string())?;
            println!("wrote {path} ({:.0} s)", s.len() as f32 / music::RATE as f32);
        }
        return Ok(());
    }
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
    let sdl = sdl2::init()?;
    let video = sdl.video()?;
    // Handhelds: fit the screen's shape. 4:3 screens (RG35XX: 640x480) get the tall view; 16:9 ones
    // (AYN Odin 2: 1920x1080, Odin 2 Mini: 1280x720) the wide one, which scales up a whole 3x / 2x.
    let forced = args.iter().any(|a| a == "--tall" || a == "--wide");
    let tall = if handheld && !forced {
        match video.desktop_display_mode(0) {
            Ok(m) if m.w > 0 && m.h > 0 => {
                let (w, h) = (m.w.max(m.h), m.w.min(m.h));
                println!("display {}x{}", m.w, m.h);
                (w as f32 / h as f32) < 1.55
            }
            _ => tall,
        }
    } else {
        tall
    };
    let view_h = if tall { gfx::SH_TALL } else { gfx::SH_WIDE };
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
    // --act2: a ready-made level 18 character in Kaldholm, with its own save file
    // (save_act2.txt) so your real character is never touched. --act2 --new starts it over.
    // --act3 / --act4 / --act5 likewise: level 26 in Mournhold, 34 in the Last Escapement, 42 in Brinehollow.
    let test_act = [("--act2", 1), ("--act3", 2), ("--act4", 3), ("--act5", 4), ("--act6", 5)].iter().find(|(f, _)| args.iter().any(|a| a == f)).map(|&(_, n)| n);
    let act2 = test_act.is_some();
    if let Some(n) = test_act {
        save::use_file(&format!("save_act{}.txt", n + 1));
    }
    // The title screen and character select (heroes in heroes/), unless a test flag skips them:
    // --new / --act2 / --vampire / --sorceress / --level use the old single save.txt flow.
    let skip_menu = ["--new", "--act2", "--act3", "--act4", "--act5", "--act6", "--vampire", "--sorceress", "--inventor", "--valkyrie", "--berserker", "--reaper", "--druid", "--inquisitor", "--level"].iter().any(|f| args.iter().any(|a| a == f));
    let mut menu: Option<menu::Menu> = if skip_menu { None } else { Some(menu::Menu::new(save::list_heroes())) };
    // Continue the saved character (fresh world from the same seed), unless --new.
    let saved = if menu.is_some() || args.iter().any(|a| a == "--new") { None } else { save::read() };
    let seed = seed_arg.or_else(|| saved.as_deref().and_then(save::seed_of)).unwrap_or(seed);
    let mut game = Game::new(seed, view_h);
    let mut loaded = false;
    if let Some(text) = saved.as_deref() {
        if save::apply(&mut game, text) {
            game.welcome_back();
            loaded = true;
        }
    }
    // A new character: pick a class (or --vampire / --sorceress on the command line).
    let class_flag = if args.iter().any(|a| a == "--vampire") {
        Some(skills::Class::Vampire)
    } else if args.iter().any(|a| a == "--inventor") {
        Some(skills::Class::Inventor)
    } else if args.iter().any(|a| a == "--valkyrie") {
        Some(skills::Class::Valkyrie)
    } else if args.iter().any(|a| a == "--berserker") {
        Some(skills::Class::Berserker)
    } else if args.iter().any(|a| a == "--reaper") {
        Some(skills::Class::Reaper)
    } else if args.iter().any(|a| a == "--druid") {
        Some(skills::Class::Druid)
    } else if args.iter().any(|a| a == "--inquisitor") {
        Some(skills::Class::Inquisitor)
    } else if args.iter().any(|a| a == "--sorceress") {
        Some(skills::Class::Sorceress)
    } else {
        None
    };
    if !loaded && menu.is_none() {
        match class_flag {
            Some(c) => game.set_class(c),
            None if !act2 => game.choose = Some(0),
            None => {}
        }
    }
    video.text_input().start();
    let (mut typed, mut backspace) = (String::new(), false);
    let mut wheel = 0i32;
    let mut menu_out: Option<menu::MenuOut> = None;
    if let (Some(n), false) = (test_act, loaded) {
        game.act_start(n);
        save::write(&game);
    }
    // --level <name>: jump straight into a level to test it (e.g. bone_crypt_floor1).
    if let Some(name) = args.iter().position(|a| a == "--level").and_then(|i| args.get(i + 1)) {
        match levels::id_from_name(name) {
            Some(id) => game.debug_goto(id),
            None => eprintln!("--level: unknown level {name:?} (try overworld, bone_crypt_floor1, ...)"),
        }
    }
    if args.iter().any(|a| a == "--cheats") {
        game.say("CHEATS ON: F9 = CHAR LEVEL, F10 = LOOT".into());
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
    let (mut skills_key, mut cycle, mut slot): (bool, bool, Option<u8>) = (false, false, None);
    let mut inv_key = false;
    let cheats = args.iter().any(|a| a == "--cheats");
    let (mut cheat_level, mut cheat_loot) = (false, false);
    let mut back_down: Option<Instant> = None;

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
                        F => keys.cast = true,
                        Space => keys.cast2 = true,
                        K if !repeat => skills_key = true,
                        I | C if !repeat => inv_key = true,
                        N if !repeat => {
                            if let Some(a) = audio.as_mut() {
                                let on = a.toggle_music();
                                game.say(if on { "MUSIC ON (N)".into() } else { "MUSIC OFF (N)".into() });
                            }
                        }
                        F9 if cheats => cheat_level = true,
                        F10 if cheats && !repeat => cheat_loot = true,
                        Num1 | Num2 | Num3 | Num4 if !repeat => {
                            slot = Some(match sc {
                                Num1 => 0,
                                Num2 => 1,
                                Num3 => 2,
                                _ => 3,
                            })
                        }
                        R if !repeat => run_toggle = true,
                        LShift | RShift => keys.shift = true,
                        Return | KpEnter if !repeat => confirm = true,
                        Q if !repeat => pot_hp = true,
                        E if !repeat => pot_mp = true,
                        Tab | M if !repeat => map = true,
                        Escape if !repeat => cancel = true,
                        Backspace | Delete => backspace = true,
                        _ => {}
                    }
                }
                Event::TextInput { text, .. } => typed.push_str(&text),
                Event::KeyUp { scancode: Some(sc), .. } => {
                    use Scancode::*;
                    match sc {
                        W | Up => keys.up = false,
                        S | Down => keys.down = false,
                        A | Left => keys.left = false,
                        D | Right => keys.right = false,
                        F => keys.cast = false,
                        Space => keys.cast2 = false,
                        LShift | RShift => keys.shift = false,
                        _ => {}
                    }
                }
                Event::MouseWheel { y, .. } => wheel += y,
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
                        Button::Back => {
                            back_held = true;
                            back_down = Some(Instant::now());
                        }
                        // SELECT + START quits (the usual handheld hotkey).
                        Button::Start if back_held => break 'main,
                        // START confirms in conversations / after death, and opens the inventory otherwise.
                        Button::Start => {
                            confirm = true;
                            inv_key = true;
                        }
                        Button::A => pad.cast = true,
                        Button::X => pad.cast2 = true,
                        Button::RightShoulder => cycle = true,
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
                    // SELECT tapped: map; held: skill tree (SELECT + START quits above).
                    Button::Back => {
                        back_held = false;
                        if back_down.map_or(false, |t| t.elapsed() > Duration::from_millis(400)) {
                            skills_key = true;
                        } else {
                            map = true;
                        }
                    }
                    Button::A => pad.cast = false,
                    Button::X => pad.cast2 = false,
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
        inp.cast2 = keys.cast2 || pad.cast2;
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
            inp.skills = skills_key;
            inp.inv = inv_key;
            inp.typed = std::mem::take(&mut typed);
            inp.backspace = backspace;
            inp.wheel = wheel;
            inp.cheat_level = cheat_level;
            inp.cheat_loot = cheat_loot;
            inp.cycle = cycle;
            inp.slot = slot;
            match menu.as_mut() {
                Some(m) => {
                    if let Some(out) = m.update(&inp) {
                        menu_out = Some(out);
                    }
                }
                None => game.update(&inp),
            }
            backspace = false;
            wheel = 0;
            confirm = false;
            pot_hp = false;
            pot_mp = false;
            map = false;
            run_toggle = false;
            cancel = false;
            skills_key = false;
            inv_key = false;
            cheat_level = false;
            cheat_loot = false;
            cycle = false;
            slot = None;
            for s in game.sfx.drain(..) {
                if let Some(a) = audio.as_mut() {
                    a.play(s);
                }
            }
            acc -= step;
        }

        // What the front end asked for: continue a hero, start a new one, music, quit.
        match menu_out.take() {
            Some(menu::MenuOut::Load(slot)) => {
                save::use_hero(&slot);
                let text = save::read().unwrap_or_default();
                let seed = save::seed_of(&text).unwrap_or(seed);
                game = Game::new(seed, view_h);
                if save::apply(&mut game, &text) {
                    game.welcome_back();
                }
                menu = None;
            }
            Some(menu::MenuOut::New(class, name)) => {
                let slot = save::new_slot(&name);
                save::use_hero(&slot);
                let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(1);
                game = Game::new(seed, view_h);
                game.set_class(class);
                game.hero_name = name;
                save::write(&game);
                menu = None;
            }
            Some(menu::MenuOut::ToggleMusic) => {
                if let Some(a) = audio.as_mut() {
                    a.toggle_music();
                }
            }
            Some(menu::MenuOut::Quit) => break 'main,
            None => {}
        }
        if let Some(a) = audio.as_mut() {
            a.set_music(if menu.is_some() { music::Track::Town } else { game.music_track() });
        }
        if game.save_due && menu.is_none() {
            game.save_due = false;
            save::write(&game);
        }
        if game.quit {
            if skip_menu {
                break 'main;
            }
            // Esc in game: save, and back to the character select screen.
            save::write(&game);
            game.quit = false;
            menu = Some(menu::Menu::at_heroes(save::list_heroes()));
        }
        match menu.as_mut() {
            Some(m) => m.draw(&mut scr, &game.art),
            None => game.draw(&mut scr),
        }
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
    if menu.is_none() {
        save::write(&game);
    }
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

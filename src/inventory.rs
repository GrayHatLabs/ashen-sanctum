//! The inventory screen (I / START): worn gear on a D2-style paper doll, a 10x3 bag, item
//! tooltips with a comparison against what you're wearing, and your totals. The world pauses
//! while it's open. In town you sell to Gerta from here; outside you drop things on the floor.
use crate::art::Art;
use crate::game::{Drop, Game, Input, Pickup, Sfx, HUD_H};
use crate::gfx::{rgb, Align, Fx, Screen, BLACK};
use crate::items::{self, Item, Rarity, Slot, Stat, BAG, BAG_COLS, SHOP, STASH, WORN};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cell {
    Worn(usize),
    Bag(usize),
    /// Gerta's shelf.
    Shop(usize),
    /// The stash chest (in town).
    Stash(usize),
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum InvAct {
    Select(Cell),
    Use,
    Toss,
    /// Bag <-> stash.
    Store,
    Close,
}

pub struct InvUi {
    pub sel: Cell,
    /// Clickable rectangles from the last draw.
    pub rects: Vec<(i32, i32, i32, i32, InvAct)>,
    /// Cell centres from the last draw (for stick / arrow navigation).
    pub centres: Vec<(Cell, i32, i32)>,
    /// Trading with Gerta: her shelf shows under the bag.
    pub shop: bool,
    /// At the jeweler's bench: using an item adds sockets or takes its gems out.
    pub jewel: bool,
    /// The bag cell of a gem you're about to set in something.
    pub holding: Option<usize>,
}

/// Paper-doll grid position of each worn slot (column, row).
const DOLL: [(i32, i32); WORN.len()] = [(0, 1), (1, 0), (1, 1), (0, 3), (2, 3), (1, 2), (0, 2), (2, 2), (2, 0)];
const CELL: i32 = 26;

impl Game {
    pub(crate) fn open_inventory(&mut self) {
        let sel = if self.p.gear.bag.iter().any(Option::is_some) {
            Cell::Bag(self.p.gear.bag.iter().position(Option::is_some).unwrap())
        } else {
            Cell::Worn(0)
        };
        self.inv = Some(InvUi { sel, rects: vec![], centres: vec![], shop: false, jewel: false, holding: None });
        self.dialog = None;
    }

    fn sel_item(&self, c: Cell) -> Option<&Item> {
        match c {
            Cell::Worn(i) => self.p.gear.worn[i].as_ref(),
            Cell::Bag(i) => self.p.gear.bag[i].as_ref(),
            Cell::Shop(i) => self.shop_stock.get(i).and_then(Option::as_ref),
            Cell::Stash(i) => self.p.gear.stash[i].as_ref(),
        }
    }

    /// Moves an item between the bag (or your body) and the stash chest.
    fn store(&mut self, c: Cell) {
        let gear = &mut self.p.gear;
        let r: Result<(), &str> = match c {
            Cell::Stash(i) => match (gear.stash[i].take(), gear.bag.iter().position(Option::is_none)) {
                (Some(it), Some(j)) => {
                    gear.bag[j] = Some(it);
                    Ok(())
                }
                (Some(it), None) => {
                    gear.stash[i] = Some(it);
                    Err("YOUR BAG IS FULL")
                }
                (None, _) => Err(""),
            },
            Cell::Bag(_) | Cell::Worn(_) => match gear.stash.iter().position(Option::is_none) {
                None => Err("THE STASH IS FULL"),
                Some(j) => {
                    let it = match c {
                        Cell::Bag(i) => gear.bag[i].take(),
                        Cell::Worn(i) => gear.worn[i].take(),
                        _ => None,
                    };
                    match it {
                        Some(it) => {
                            gear.stash[j] = Some(it);
                            Ok(())
                        }
                        None => Err(""),
                    }
                }
            },
            Cell::Shop(_) => Err(""),
        };
        match r {
            Ok(()) => {
                self.sfx.push(Sfx::Pickup);
                self.p.recalc();
                self.save_due = true;
            }
            Err(why) if !why.is_empty() => self.say(why.into()),
            Err(_) => {}
        }
    }

    /// Sets the held gem into the item in cell `c`.
    fn set_gem(&mut self, gem: usize, c: Cell) {
        let r = match c {
            Cell::Bag(i) => self.p.gear.socket(gem, false, i),
            Cell::Worn(w) => self.p.gear.socket(gem, true, w),
            _ => Err("GEMS GO IN THINGS YOU CARRY OR WEAR".into()),
        };
        match r {
            Ok(msg) => {
                self.sfx.push(Sfx::Pickup);
                self.say(msg);
                self.p.recalc();
                self.save_due = true;
                let ui = self.inv.as_mut().unwrap();
                ui.holding = None;
                ui.sel = c;
            }
            Err(why) if !why.is_empty() => self.say(why),
            Err(_) => {}
        }
    }

    /// At the jeweler's bench: empty a socketed item, or cut sockets in a plain one.
    fn jewel_use(&mut self, c: Cell) {
        let (worn, i) = match c {
            Cell::Bag(i) => (false, i),
            Cell::Worn(w) => (true, w),
            _ => return,
        };
        let Some(it) = self.sel_item(c).cloned() else { return };
        if it.gem().is_some() {
            return self.say("CHOOSE THE ITEM, NOT THE GEM".into());
        }
        if !it.gems.is_empty() {
            let cost = items::unsocket_cost(&it);
            if self.p.gold < cost {
                return self.say(format!("TAKING THEM OUT COSTS {cost} GOLD"));
            }
            match self.p.gear.unsocket(worn, i) {
                Ok(n) => {
                    self.p.gold -= cost;
                    self.sfx.push(Sfx::Pickup);
                    self.say(format!("{n} GEM{} TAKEN OUT FOR {cost} GOLD", if n == 1 { "" } else { "S" }));
                    self.p.recalc();
                    self.save_due = true;
                }
                Err(why) => self.say(why),
            }
            return;
        }
        let max = items::max_sockets(it.slot());
        if it.rarity != Rarity::Normal || it.sockets > 0 || max == 0 {
            let why = if it.sockets > 0 { "IT HAS SOCKETS ALREADY" } else { "I ONLY CUT SOCKETS IN PLAIN (WHITE) GEAR" };
            return self.say(why.into());
        }
        let cost = items::socket_cost(&it);
        if self.p.gold < cost {
            return self.say(format!("SOCKETS COST {cost} GOLD"));
        }
        let n = self.rng.range(1, max as i32 + 1) as u8;
        let target = if worn { self.p.gear.worn[i].as_mut() } else { self.p.gear.bag[i].as_mut() };
        if let Some(t) = target {
            t.sockets = n;
            self.p.gold -= cost;
            self.sfx.push(Sfx::Pickup);
            self.say(format!("CUT {n} SOCKET{} FOR {cost} GOLD", if n == 1 { "" } else { "S" }));
            self.save_due = true;
        }
    }

    /// Buys the item on Gerta's shelf.
    fn buy_gear(&mut self, i: usize) {
        let Some(it) = self.shop_stock.get(i).and_then(Option::as_ref) else { return };
        let cost = it.cost();
        if self.p.gold < cost {
            self.say("NOT ENOUGH GOLD".into());
        } else if self.p.gear.free() == 0 {
            self.say("YOUR BAG IS FULL".into());
        } else {
            let it = self.shop_stock[i].take().unwrap();
            self.p.gold -= cost;
            self.sfx.push(Sfx::Pickup);
            self.say(format!("BOUGHT {}", it.name));
            let _ = self.p.gear.add(it);
            self.save_due = true;
        }
    }

    /// Inventory controls (the world is paused while it's open).
    pub(crate) fn update_inventory(&mut self, inp: &Input, pressed_confirm: bool, click: bool) {
        let prev = self.prev.clone();
        let edge = |now: f32, before: f32, neg: bool| if neg { now < -0.5 && before >= -0.5 } else { now > 0.5 && before <= 0.5 };
        let Some(ui) = self.inv.as_mut() else { return };
        let mut act: Option<InvAct> = None;
        let dir = if edge(inp.move_x, prev.move_x, false) {
            Some((1, 0))
        } else if edge(inp.move_x, prev.move_x, true) {
            Some((-1, 0))
        } else if edge(inp.move_y, prev.move_y, false) {
            Some((0, 1))
        } else if edge(inp.move_y, prev.move_y, true) {
            Some((0, -1))
        } else {
            None
        };
        if let (Some((dx, dy)), Some(&(_, sx, sy))) = (dir, ui.centres.iter().find(|c| c.0 == ui.sel)) {
            // Nearest cell in that direction (sideways distance counts double).
            let best = ui
                .centres
                .iter()
                .filter(|&&(_, x, y)| (x - sx) * dx + (y - sy) * dy > 4)
                .min_by_key(|&&(_, x, y)| {
                    let along = (x - sx) * dx + (y - sy) * dy;
                    let side = ((x - sx) * dy - (y - sy) * dx).abs();
                    along + side * 3
                })
                .map(|c| c.0);
            if let Some(c) = best {
                ui.sel = c;
            }
        }
        let mut right_click = false;
        if let Some((mx, my)) = inp.mouse {
            let hit = ui.rects.iter().find(|&&(x, y, w, h, _)| mx >= x && mx < x + w && my >= y && my < y + h).map(|r| r.4);
            if let (true, Some(a)) = (click, hit) {
                act = Some(match a {
                    // Clicking the selected item again wears / takes it off.
                    InvAct::Select(c) if c == ui.sel => InvAct::Use,
                    a => a,
                });
            }
            if inp.rmb && !prev.rmb {
                if let Some(InvAct::Select(c)) = hit {
                    ui.sel = c;
                    right_click = true;
                }
            }
        }
        if pressed_confirm {
            act = Some(InvAct::Use);
        }
        if (inp.cast2 && !prev.cast2) || right_click {
            act = Some(InvAct::Toss);
        }
        let stash_open = !ui.shop && self.safe.map_or(false, |(x0, y0, x1, y1)| self.p.x >= x0 && self.p.x <= x1 && self.p.y >= y0 && self.p.y <= y1);
        if inp.potion_mp && stash_open {
            act = Some(InvAct::Store);
        }
        if inp.inv || inp.cancel || inp.run_toggle || inp.skills {
            act = Some(InvAct::Close);
        }
        let sel = ui.sel;
        let (jewel, holding) = (ui.jewel, ui.holding);
        // A held gem goes into whatever you pick next.
        if let (Some(g), Some(InvAct::Select(_) | InvAct::Use)) = (holding, act) {
            let c = if let Some(InvAct::Select(c)) = act { c } else { sel };
            if c == Cell::Bag(g) {
                self.inv.as_mut().unwrap().holding = None;
                self.inv.as_mut().unwrap().sel = c;
            } else if matches!(c, Cell::Bag(_) | Cell::Worn(_)) && self.sel_item(c).is_some() {
                self.set_gem(g, c);
            } else {
                self.inv.as_mut().unwrap().sel = c;
            }
            return;
        }
        if act.is_some() && !matches!(act, Some(InvAct::Select(_))) {
            self.inv.as_mut().unwrap().holding = None;
        }
        match act {
            Some(InvAct::Select(c)) => self.inv.as_mut().unwrap().sel = c,
            Some(InvAct::Use) if jewel && matches!(sel, Cell::Bag(_) | Cell::Worn(_)) => self.jewel_use(sel),
            Some(InvAct::Use) if matches!(sel, Cell::Bag(_)) && self.sel_item(sel).map_or(false, |it| it.gem().is_some()) => {
                let Cell::Bag(i) = sel else { return };
                self.inv.as_mut().unwrap().holding = Some(i);
                self.say("PICK AN ITEM WITH A FREE SOCKET".into());
            }
            Some(InvAct::Use) => {
                let r = match sel {
                    Cell::Bag(i) => self.p.gear.equip(i, self.p.clvl),
                    Cell::Worn(w) => self.p.gear.unequip(w),
                    Cell::Shop(i) => {
                        self.buy_gear(i);
                        Err(String::new())
                    }
                    Cell::Stash(_) => {
                        self.store(sel);
                        Err(String::new())
                    }
                };
                match r {
                    Ok(()) => {
                        self.sfx.push(Sfx::Pickup);
                        self.p.recalc();
                        self.save_due = true;
                    }
                    Err(why) if !why.is_empty() => self.say(why),
                    Err(_) => {}
                }
            }
            Some(InvAct::Toss) => self.toss(sel),
            Some(InvAct::Store) => self.store(sel),
            Some(InvAct::Close) => self.inv = None,
            None => {}
        }
    }

    /// Sells (in town) or drops (outside) the selected item.
    fn toss(&mut self, c: Cell) {
        let it = match c {
            Cell::Worn(i) => self.p.gear.worn[i].take(),
            Cell::Bag(i) => self.p.gear.bag[i].take(),
            Cell::Shop(i) => return self.buy_gear(i),
            Cell::Stash(i) => self.p.gear.stash[i].take(),
        };
        let Some(it) = it else { return };
        if matches!(c, Cell::Worn(_)) {
            self.p.recalc();
        }
        if self.in_safe(self.p.x, self.p.y) {
            let price = it.price();
            self.p.gold += price;
            self.sfx.push(Sfx::Pickup);
            self.say(format!("SOLD {} FOR {price} GOLD", it.name));
        } else {
            self.sfx.push(Sfx::Swing);
            // A step away, so you don't pick it straight back up.
            let (dx, dy) = crate::game::dir_vec(self.p.dir);
            let (mut x, mut y) = (self.p.x + dx, self.p.y + dy);
            if self.d.blocked(x, y, 0.2) {
                (x, y) = (self.p.x - dx, self.p.y - dy);
            }
            self.pickups.push(Pickup { x, y, kind: Drop::Item(Box::new(it)), t: 0.0 });
        }
        self.save_due = true;
    }

    pub(crate) fn draw_inventory(&mut self, scr: &mut Screen) {
        let Some(ui) = self.inv.as_ref() else { return };
        let sel = ui.sel;
        let top = self.view_h - HUD_H;
        let (pw, ph) = (620, (top - 12).min(310));
        let (x0, y0) = (scr.w / 2 - pw / 2, (top - ph) / 2);
        scr.blend(0, 0, scr.w, top, BLACK, 0.45);
        scr.blend(x0, y0, pw, ph, rgb(0x0c0a08), 0.94);
        for (x, y, w, h) in [(x0, y0, pw, 1), (x0, y0 + ph - 1, pw, 1), (x0, y0, 1, ph), (x0 + pw - 1, y0, 1, ph)] {
            scr.fill(x, y, w, h, rgb(0x8a7050));
        }
        let in_town = self.in_safe(self.p.x, self.p.y);
        let shop = ui.shop;
        let (jewel, holding) = (ui.jewel, ui.holding);
        let title = if shop {
            "INVENTORY - TRADING WITH GERTA"
        } else if jewel {
            "INVENTORY - AT THE JEWELER'S BENCH"
        } else {
            "INVENTORY"
        };
        scr.text(title, x0 + 10, y0 + 8, rgb(0xffd080), Align::Left, 1);
        scr.text(&format!("GOLD {}", self.p.gold), x0 + 400, y0 + 8, rgb(0xe8c050), Align::Right, 1);
        let mut rects = vec![];
        let mut centres = vec![];
        let gear = &self.p.gear;

        // ---- paper doll ----
        let (dx0, dy0) = (x0 + 12, y0 + 24);
        let dcell = 30;
        for (w, &(c, r)) in DOLL.iter().enumerate() {
            let (cx, cy) = (dx0 + c * (dcell + 4), dy0 + r * (dcell + 4));
            let it = gear.worn[w].as_ref();
            cell_box(scr, cx, cy, dcell, sel == Cell::Worn(w), it);
            match it {
                Some(it) => draw_icon(scr, &self.art, it, cx + dcell / 2, cy + dcell / 2, 1.0),
                None => {
                    let label = &items::slot_name(WORN[w])[..2];
                    scr.text(label, cx + dcell / 2, cy + dcell / 2 - 3, rgb(0x4a3e30), Align::Center, 1);
                }
            }
            rects.push((cx, cy, dcell, dcell, InvAct::Select(Cell::Worn(w))));
            centres.push((Cell::Worn(w), cx + dcell / 2, cy + dcell / 2));
        }

        // ---- your totals, under the doll ----
        let b = self.p.bonus;
        let reduce = 100.0 - self.p.armored(100.0);
        let mut sy = dy0 + 4 * (dcell + 4) + 4;
        let totals = [
            format!("LIFE {}", self.p.max_hp as i32),
            format!("MANA {}", self.p.max_mana as i32),
            format!("FIRE DMG +{}%", ((self.p.skills.fire_mult() - 1.0) * 100.0).round() as i32),
            format!("ARMOR {} (-{:.0}%)", b.get(Stat::Armor), reduce),
            format!("CAST +{}%  MOVE +{}%", b.get(Stat::Cast), b.get(Stat::Move)),
            format!("MAGIC FIND +{}%", b.get(Stat::Magic)),
        ];
        for t in totals {
            scr.text(&t, dx0, sy, rgb(0xb8a890), Align::Left, 1);
            sy += 10;
        }
        if b.get(Stat::Skills) > 0 {
            scr.text(&format!("+{} TO FIRE SKILLS", b.get(Stat::Skills)), dx0, sy, rgb(0x7090ff), Align::Left, 1);
            sy += 10;
        }
        for (si, n) in gear.sets_worn() {
            let d = &items::SETS[si];
            scr.text(&format!("{} {n}/{}", d.name, d.pieces.len()), dx0, sy, rgb(items::rarity_col(Rarity::Set)), Align::Left, 1);
            sy += 10;
        }

        // ---- the bag ----
        let (bx0, by0) = (x0 + 132, y0 + 24);
        for i in 0..BAG {
            let (c, r) = ((i % BAG_COLS) as i32, (i / BAG_COLS) as i32);
            let (cx, cy) = (bx0 + c * CELL, by0 + r * CELL);
            let it = gear.bag[i].as_ref();
            cell_box(scr, cx, cy, CELL - 2, sel == Cell::Bag(i), it);
            if let Some(it) = it {
                draw_icon(scr, &self.art, it, cx + CELL / 2 - 1, cy + CELL / 2 - 1, 0.85);
                if it.req > self.p.clvl {
                    scr.blend(cx + 1, cy + 1, CELL - 4, CELL - 4, rgb(0xa01010), 0.3);
                }
            }
            if holding == Some(i) {
                // The gem in your hand pulses.
                let a = 0.25 + 0.2 * ((self.tick as f32) * 0.2).sin();
                scr.blend(cx + 1, cy + 1, CELL - 4, CELL - 4, rgb(0xfff0a0), a);
            }
            rects.push((cx, cy, CELL - 2, CELL - 2, InvAct::Select(Cell::Bag(i))));
            centres.push((Cell::Bag(i), cx + CELL / 2, cy + CELL / 2));
        }
        let free = gear.free();
        match holding.and_then(|g| gear.bag[g].as_ref()) {
            // Setting a gem: say so where the free count goes.
            Some(g) => scr.text(&format!("SETTING {} - PICK AN ITEM", g.name), bx0, by0 + 3 * CELL + 2, rgb(0xfff0a0), Align::Left, 1),
            None => scr.text(&format!("{free} FREE"), bx0 + BAG_COLS as i32 * CELL - 2, by0 + 3 * CELL + 2, rgb(0x7a6a5a), Align::Right, 1),
        }

        // ---- tooltip: selected item, then what it would replace ----
        let (tx, mut ty) = (x0 + 410, y0 + 24);
        let tw = pw - (tx - x0) - 10;
        let wrap_at = (tw / 6) as usize;
        match self.sel_item(sel) {
            Some(it) => {
                ty = tooltip(scr, it, tx, ty, wrap_at, self.p.clvl);
                if let Cell::Shop(_) = sel {
                    let col = if it.cost() > self.p.gold { rgb(0xe04040) } else { rgb(0xe8c050) };
                    scr.text(&format!("COSTS {} GOLD", it.cost()), tx, ty, col, Align::Left, 1);
                    ty += 10;
                }
                if let Cell::Bag(_) | Cell::Shop(_) = sel {
                    if let Some(w) = gear.target(it).and_then(|t| gear.worn[t].as_ref()) {
                        ty += 6;
                        scr.text("YOU ARE WEARING:", tx, ty, rgb(0x7a6a5a), Align::Left, 1);
                        ty += 11;
                        tooltip(scr, w, tx, ty, wrap_at, self.p.clvl);
                    }
                }
            }
            None => {
                let what = match sel {
                    Cell::Worn(w) => format!("NO {} WORN", items::slot_name(WORN[w])),
                    Cell::Bag(_) | Cell::Shop(_) | Cell::Stash(_) => "EMPTY".into(),
                };
                scr.text(&what, tx, ty, rgb(0x7a6a5a), Align::Left, 1);
            }
        }

        // ---- under the bag: Gerta's shelf, or hints ----
        let mut hy = by0 + 3 * CELL + 16;
        if shop {
            scr.text("GERTA'S GEAR", bx0, hy, rgb(0xd8b878), Align::Left, 1);
            let sy0 = hy + 12;
            for i in 0..SHOP {
                let (c, r) = ((i % BAG_COLS) as i32, (i / BAG_COLS) as i32);
                let (cx, cy) = (bx0 + c * CELL, sy0 + r * CELL);
                let it = self.shop_stock.get(i).and_then(Option::as_ref);
                cell_box(scr, cx, cy, CELL - 2, sel == Cell::Shop(i), it);
                if let Some(it) = it {
                    draw_icon(scr, &self.art, it, cx + CELL / 2 - 1, cy + CELL / 2 - 1, 0.85);
                    if it.cost() > self.p.gold {
                        scr.blend(cx + 1, cy + 1, CELL - 4, CELL - 4, BLACK, 0.45);
                    }
                }
                rects.push((cx, cy, CELL - 2, CELL - 2, InvAct::Select(Cell::Shop(i))));
                centres.push((Cell::Shop(i), cx + CELL / 2, cy + CELL / 2));
            }
            hy = sy0 + 2 * CELL + 4;
            scr.text("ENTER/A BUYS FROM GERTA. X SELLS YOURS.", bx0, hy, rgb(0x6a5a4a), Align::Left, 1);
            hy = 10_000;
        } else if in_town && !jewel {
            scr.text("STASH", bx0, hy, rgb(0xd8b878), Align::Left, 1);
            scr.text("Y / E: BAG <-> STASH", bx0 + BAG_COLS as i32 * CELL - 2, hy, rgb(0x6a5a4a), Align::Right, 1);
            let sy0 = hy + 12;
            for i in 0..STASH {
                let (c, r) = ((i % BAG_COLS) as i32, (i / BAG_COLS) as i32);
                let (cx, cy) = (bx0 + c * CELL, sy0 + r * CELL);
                let it = self.p.gear.stash[i].as_ref();
                cell_box(scr, cx, cy, CELL - 2, sel == Cell::Stash(i), it);
                if let Some(it) = it {
                    draw_icon(scr, &self.art, it, cx + CELL / 2 - 1, cy + CELL / 2 - 1, 0.85);
                }
                rects.push((cx, cy, CELL - 2, CELL - 2, InvAct::Select(Cell::Stash(i))));
                centres.push((Cell::Stash(i), cx + CELL / 2, cy + CELL / 2));
            }
            hy = 10_000;
        }
        if jewel {
            let lines = [
                "CHOOSE AN ITEM:".to_string(),
                "WITH GEMS: I TAKE THEM OUT, UNHARMED.".to_string(),
                "PLAIN WHITE GEAR: I CUT 1-3 SOCKETS IN IT.".to_string(),
                match self.sel_item(sel) {
                    Some(it) if !it.gems.is_empty() => format!("THIS: {} GOLD TO TAKE THE GEMS OUT", items::unsocket_cost(it)),
                    Some(it) if it.rarity == Rarity::Normal && it.sockets == 0 && items::max_sockets(it.slot()) > 0 => {
                        format!("THIS: {} GOLD FOR SOCKETS", items::socket_cost(it))
                    }
                    _ => String::new(),
                },
            ];
            for (k, l) in lines.iter().enumerate() {
                let col = if k == 3 { rgb(0xe8c050) } else { rgb(0x9a8a78) };
                scr.text(l, bx0, hy + k as i32 * 10, col, Align::Left, 1);
            }
            hy = 10_000;
        }
        let hints = [
            "WALK OVER GEAR TO PICK IT UP. ENTER/A ON A GEM, THEN AN ITEM, SETS IT.",
            "BLUE: MAGIC  YELLOW: RARE  GREEN: SET  GOLD: UNIQUE",
            if in_town { "X / RIGHT CLICK SELLS IT." } else { "X / RIGHT CLICK DROPS. SELL IN TOWN." },
        ];
        for h in hints {
            if hy > y0 + ph {
                break;
            }
            scr.text(h, bx0, hy, rgb(0x6a5a4a), Align::Left, 1);
            hy += 10;
        }

        // ---- buttons ----
        let by = y0 + ph - 22;
        let mut bx = x0 + 10;
        let has = self.sel_item(sel).is_some();
        let is_gem = self.sel_item(sel).map_or(false, |it| it.gem().is_some());
        let use_label = match sel {
            Cell::Bag(_) | Cell::Worn(_) if holding.is_some() => "SET GEM HERE (ENTER/A)",
            Cell::Bag(_) | Cell::Worn(_) if jewel => "JEWELER (ENTER/A)",
            Cell::Bag(_) if is_gem => "SET IN GEAR (ENTER/A)",
            Cell::Worn(_) => "TAKE OFF (ENTER/A)",
            Cell::Bag(_) => "WEAR (ENTER/A)",
            Cell::Shop(_) => "BUY (ENTER/A)",
            Cell::Stash(_) => "TAKE (ENTER/A)",
        };
        let toss_label = if matches!(sel, Cell::Shop(_)) { "BUY (X)" } else if in_town { "SELL (X)" } else { "DROP (X)" };
        let mut buttons: Vec<(&str, InvAct, bool)> = vec![(use_label, InvAct::Use, has), (toss_label, InvAct::Toss, has)];
        if in_town && !shop && !jewel && !matches!(sel, Cell::Stash(_)) {
            buttons.push(("STASH (Y/E)", InvAct::Store, has));
        }
        buttons.push(("CLOSE (I/START)", InvAct::Close, true));
        for (label, a, on) in buttons {
            let w = crate::gfx::text_width(label, 1) + 12;
            scr.fill(bx, by, w, 15, if on { rgb(0x5a3a10) } else { rgb(0x221c16) });
            scr.text(label, bx + 6, by + 4, if on { rgb(0xffe0a0) } else { rgb(0x6a5a4a) }, Align::Left, 1);
            rects.push((bx, by, w, 15, a));
            bx += w + 6;
        }
        let ui = self.inv.as_mut().unwrap();
        ui.rects = rects;
        ui.centres = centres;
    }
}

fn cell_box(scr: &mut Screen, x: i32, y: i32, s: i32, selected: bool, it: Option<&Item>) {
    let bg = match it.map(|i| i.rarity) {
        Some(Rarity::Magic) => rgb(0x141a2e),
        Some(Rarity::Rare) => rgb(0x26240e),
        Some(Rarity::Unique) => rgb(0x2a1e0e),
        Some(Rarity::Set) => rgb(0x0e2612),
        _ => rgb(0x161210),
    };
    scr.fill(x, y, s, s, bg);
    let edge = if selected { rgb(0xffcf70) } else { rgb(0x3a3026) };
    for (ex, ey, ew, eh) in [(x, y, s, 1), (x, y + s - 1, s, 1), (x, y, 1, s), (x + s - 1, y, 1, s)] {
        scr.fill(ex, ey, ew, eh, edge);
    }
}

/// Draws an item's name and stats; returns the y below the last line.
fn tooltip(scr: &mut Screen, it: &Item, x: i32, mut y: i32, wrap_at: usize, clvl: u32) -> i32 {
    for l in crate::story::wrap(&it.name, wrap_at) {
        scr.text(&l, x, y, rgb(it.col()), Align::Left, 1);
        y += 10;
    }
    for line in it.lines() {
        let col = if line.starts_with("REQUIRED") {
            if it.req > clvl {
                rgb(0xe04040)
            } else {
                rgb(0x9a8a78)
            }
        } else if line == it.base().name || line.starts_with("  ") {
            rgb(0x9a8a78)
        } else if line.starts_with('(') || line.contains(" SET (") {
            rgb(items::rarity_col(Rarity::Set))
        } else {
            rgb(0x8098ff)
        };
        for l in crate::story::wrap(&line, wrap_at) {
            scr.text(&l, x, y, col, Align::Left, 1);
            y += 10;
        }
    }
    scr.text(&format!("SELLS FOR {}", it.price()), x, y, rgb(0x7a6a5a), Align::Left, 1);
    y + 10
}

/// An item's icon centred on (cx, cy): generated art when it exists, a drawn stand-in otherwise.
pub fn draw_icon(scr: &mut Screen, art: &Art, it: &Item, cx: i32, cy: i32, scale: f32) {
    let tint = match it.rarity {
        Rarity::Unique => Some(rgb(0xffb040)),
        _ => None,
    };
    let fx = Fx { tint: tint.unwrap_or(BLACK), tint_a: if tint.is_some() { 0.12 } else { 0.0 }, ..Fx::default() };
    // One icon per gem kind: better grades draw bigger.
    let scale = it.gem().map_or(scale, |g| scale * (0.55 + g.grade as f32 * 0.1));
    if let Some(s) = art.item(it.base().icon) {
        let (w, h) = ((s.w as f32 * scale) as i32, (s.h as f32 * scale) as i32);
        // Icons are anchored at their feet: offset so they sit centred.
        scr.blit_scaled(s, cx, cy + h / 2, scale, fx);
        let _ = w;
    } else if let Some(g) = it.gem() {
        gem_icon(scr, g, cx, cy, scale);
    } else {
        fallback_icon(scr, it.slot(), cx, cy, scale, it.rarity);
    }
    // Sockets: a row of dots along the bottom, filled with their gems' colours.
    let n = it.sockets as i32;
    for k in 0..n {
        let x = cx - (n - 1) * 3 + k * 6;
        let y = cy + (9.0 * scale) as i32;
        scr.disc(x, y, 2, rgb(0x080606));
        match it.gems.get(k as usize) {
            Some(g) => scr.disc(x, y, 1, rgb(items::gem_col(g.kind))),
            None => scr.pset(x, y, rgb(0x5a4a3a)),
        }
    }
}

/// A cut gem: bigger and brighter the better its grade (a skull for skulls).
fn gem_icon(scr: &mut Screen, g: items::Gem, cx: i32, cy: i32, scale: f32) {
    let r = ((3.0 + g.grade as f32 * 1.2) * scale).round() as i32;
    let col = items::gem_col(g.kind);
    if g.kind == 6 {
        scr.disc(cx, cy - 1, r, rgb(col));
        scr.fill(cx - r / 2, cy + r - 2, r, r / 2 + 1, rgb(col));
        scr.disc(cx - r / 3 - 1, cy - 1, (r / 4).max(1), rgb(0x100808));
        scr.disc(cx + r / 3 + 1, cy - 1, (r / 4).max(1), rgb(0x100808));
        return;
    }
    // A diamond shape with a lit upper facet.
    for dy in -r..=r {
        let w = r - dy.abs();
        scr.fill(cx - w, cy + dy, w * 2 + 1, 1, rgb(col));
    }
    for dy in -r..0 {
        let w = (r - dy.abs()) / 2;
        scr.fill(cx - w - r / 3, cy + dy, w.max(1), 1, rgb(0xffffff));
    }
    scr.pset(cx + r / 3, cy + r / 3, rgb(0x101010));
}

fn fallback_icon(scr: &mut Screen, slot: Slot, cx: i32, cy: i32, scale: f32, rarity: Rarity) {
    let k = |v: i32| (v as f32 * scale).round() as i32;
    let gem = match rarity {
        Rarity::Normal => rgb(0xa09080),
        Rarity::Magic => rgb(0x5070ff),
        Rarity::Rare => rgb(0xf0e060),
        Rarity::Unique => rgb(0xffa030),
        Rarity::Set => rgb(0x40d040),
    };
    match slot {
        Slot::Gem => scr.disc(cx, cy, k(5), gem),
        Slot::Weapon => {
            for i in -k(9)..=k(9) {
                scr.fill(cx + i, cy - i, 2, 2, rgb(0x6a4424));
            }
            scr.disc(cx + k(9), cy - k(9), k(3).max(1), gem);
        }
        Slot::Helm => {
            scr.disc(cx, cy, k(7), rgb(0x585048));
            scr.fill(cx - k(7), cy, k(14), k(5), rgb(0x161210));
            scr.fill(cx - k(8), cy, k(16), k(2), rgb(0x787068));
            scr.disc(cx, cy - k(3), k(1).max(1), gem);
        }
        Slot::Armor => {
            scr.fill(cx - k(7), cy - k(7), k(14), k(15), rgb(0x6a2a20));
            scr.fill(cx - k(10), cy - k(7), k(3), k(8), rgb(0x6a2a20));
            scr.fill(cx + k(7), cy - k(7), k(3), k(8), rgb(0x6a2a20));
            scr.fill(cx - 1, cy - k(7), 2, k(15), gem);
        }
        Slot::Gloves => {
            for s in [-1, 1] {
                scr.fill(cx + s * k(5) - k(3), cy - k(5), k(6), k(10), rgb(0x5a4030));
                scr.fill(cx + s * k(5) - k(3), cy + k(3), k(6), k(2), gem);
            }
        }
        Slot::Boots => {
            for s in [-1, 1] {
                let x = cx + s * k(5) - k(3);
                scr.fill(x, cy - k(7), k(5), k(11), rgb(0x4a3424));
                scr.fill(x, cy + k(3), k(8), k(4), rgb(0x4a3424));
                scr.fill(x, cy - k(7), k(5), k(2), gem);
            }
        }
        Slot::Belt => {
            scr.fill(cx - k(10), cy - k(2), k(20), k(5), rgb(0x5a4030));
            scr.fill(cx - k(2), cy - k(3), k(5), k(7), gem);
        }
        Slot::Ring => {
            scr.disc(cx, cy + k(1), k(5), rgb(0xc8a040));
            scr.disc(cx, cy + k(1), k(3), rgb(0x161210));
            scr.disc(cx, cy - k(4), k(2).max(1), gem);
        }
        Slot::Amulet => {
            for i in -k(6)..=k(6) {
                scr.pset(cx + i, cy - k(6) + i.abs() / 2, rgb(0xa08050));
            }
            scr.disc(cx, cy + k(2), k(4), rgb(0xa08050));
            scr.disc(cx, cy + k(2), k(2).max(1), gem);
        }
    }
}

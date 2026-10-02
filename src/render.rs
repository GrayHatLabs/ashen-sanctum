//! Drawing: the isometric world, lighting, effects, HUD, dialogue and overlays.
use crate::art::{Art, CharArt};
use crate::dungeon::Tile;
use crate::game::{
    xp_to_next, Drop, Game, PKind, Pickup, State, CAST_TIME, FIREBALL_COST, HUD_H, MAX_FOOD, MAX_STAMINA,
};
use crate::gfx::{add, mix, rgb, Align, Fx, Screen, Sprite, BLACK, WHITE};
use crate::iso;
use crate::mobs::{def, HazardKind, MobState, ShotKind};
use crate::story::{self, Role};
use crate::world::{LevelId, PortalKind, PropKind, SANCTUM};

/// Face height (pixels) of cut-down front walls.
const LOW_WALL: i32 = 8;

impl Game {
    pub fn draw(&mut self, scr: &mut Screen) {
        let (ox, oy) = self.cam_origin();
        if !self.light_ready {
            let (radius, ambient) = self.theme.light();
            scr.build_base_light(ox as i32, oy as i32 - 14, radius, ambient);
            self.light_ready = true;
        }
        let sh = if self.shake > 0.0 { ((self.tick as f32 * 1.7).sin() * self.shake * 4.0) as i32 } else { 0 };
        scr.shake = (sh, (sh as f32 * 0.5) as i32);
        scr.clear(BLACK);
        let (px, py) = (self.p.x, self.p.y);
        let to_scr = |x: f32, y: f32| -> (i32, i32) {
            let (sx, sy) = iso::to_screen(x - px, y - py);
            ((sx + ox).round() as i32, (sy + oy).round() as i32)
        };
        let view_h = self.view_h;
        let theme = self.theme;
        let overworld = self.level.overland();

        // Visible tile range: invert the four screen corners.
        let corners = [(0.0, -60.0), (scr.w as f32, -60.0), (0.0, view_h as f32 + 120.0), (scr.w as f32, view_h as f32 + 120.0)];
        let (mut x0, mut y0, mut x1, mut y1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
        for (cx, cy) in corners {
            let (wx, wy) = iso::to_world(cx - ox, cy - oy);
            let (wx, wy) = (wx + px, wy + py);
            x0 = x0.min(wx.floor() as i32 - 1);
            y0 = y0.min(wy.floor() as i32 - 1);
            x1 = x1.max(wx.ceil() as i32 + 1);
            y1 = y1.max(wy.ceil() as i32 + 4);
        }
        x0 = x0.max(0);
        y0 = y0.max(0);
        x1 = x1.min(self.d.w - 1);
        y1 = y1.min(self.d.h - 1);
        let in_view = |x: f32, y: f32| x >= x0 as f32 - 4.0 && x <= x1 as f32 + 4.0 && y >= y0 as f32 - 4.0 && y <= y1 as f32 + 4.0;

        // 1. Floors.
        for ty in y0..=y1 {
            for tx in x0..=x1 {
                if self.d.get(tx, ty) == Tile::Void {
                    continue;
                }
                let (sx, sy) = to_scr(tx as f32 + 0.5, ty as f32 + 0.5);
                if sx < -20 || sx > scr.w + 20 || sy < -12 || sy > view_h + 12 {
                    continue;
                }
                let v = self.d.var[(ty * self.d.w + tx) as usize] as usize;
                let f = self.art.floor(theme, self.d.ground_at(tx, ty), v);
                scr.blit(f, sx, sy, Fx::default());
            }
        }
        // Scorch marks and blood.
        for dc in &self.decals {
            let (sx, sy) = to_scr(dc.x, dc.y);
            let r = dc.r * iso::TW * 0.5;
            for yy in -(r as i32 / 2)..=(r as i32 / 2) {
                for xx in -(r as i32)..=(r as i32) {
                    let d = (xx as f32 / r).powi(2) + (yy as f32 * 2.0 / r).powi(2);
                    if d < 1.0 && ((xx * 7 + yy * 13) & 3) != 0 {
                        let (x, y) = (sx + xx + scr.shake.0, sy + yy + scr.shake.1);
                        if x >= 0 && y >= 0 && x < scr.w && y < view_h {
                            let i = (y * scr.w + x) as usize;
                            scr.px[i] = mix(scr.px[i], dc.col, dc.a * (1.0 - d));
                        }
                    }
                }
            }
        }
        // Flat props (stairs down), hazards and pickups sit on the floor.
        for pr in self.props.iter().filter(|p| p.kind.flat()) {
            let (sx, sy) = to_scr(pr.x, pr.y);
            scr.blit(self.art.prop(pr.kind.art()), sx, sy + 8, Fx::default());
        }
        for m in &self.meteors {
            let (sx, sy) = to_scr(m.x, m.y);
            let k = m.t / crate::skills::METEOR_DELAY;
            let rx = (m.r * iso::TW * 0.5 * (0.3 + 0.7 * k)) as i32;
            blend_ellipse(scr, sx, sy, rx, rx / 2, BLACK, 0.25 + 0.35 * k);
        }
        for h in &self.hazards {
            let (sx, sy) = to_scr(h.x, h.y);
            draw_hazard(scr, h.kind, sx, sy, h.r, h.t, h.warn, h.live, self.tick);
        }
        // The waypoint: a rune circle in the floor, lit blue once activated.
        {
            let (sx, sy) = to_scr(self.waypoint.0, self.waypoint.1);
            let on = self.waypoints.contains(&self.level);
            let pulse = ((self.tick as f32) * 0.06).sin() * 0.5 + 0.5;
            blend_ellipse(scr, sx, sy, 22, 9, rgb(0x141418), 0.85);
            let c = if on { mix(rgb(0x4060c0), rgb(0x90b0ff), pulse) } else { rgb(0x585060) };
            ring(scr, sx, sy, 20, 8, c);
            ring(scr, sx, sy, 14, 5, c);
            for k in 0..8 {
                let a = k as f32 * std::f32::consts::FRAC_PI_4 + self.tick as f32 * if on { 0.01 } else { 0.0 };
                let (rx, ry) = ((a.cos() * 17.0) as i32, (a.sin() * 6.5) as i32);
                scr.fill(sx + rx - 1, sy + ry, 3, 1, c);
            }
            if on {
                blend_ellipse(scr, sx, sy, 12, 4, rgb(0x6080ff), 0.25 + 0.2 * pulse);
            }
        }
        for k in &self.pickups {
            let (sx, sy) = to_scr(k.x, k.y);
            draw_pickup(scr, k, sx, sy, self.tick, &self.art);
        }

        // 2. Depth-sorted walls, props and actors.
        enum D {
            Wall(i32, i32),
            Prop(usize),
            Mob(usize),
            Npc(usize),
            Player,
            Ball(usize),
            Shot(usize),
        }
        let mut list: Vec<(f32, D)> = vec![];
        for ty in y0..=y1 {
            for tx in x0..=x1 {
                if self.d.get(tx, ty) == Tile::Wall {
                    list.push((tx as f32 + ty as f32 + 1.0, D::Wall(tx, ty)));
                }
            }
        }
        for (i, pr) in self.props.iter().enumerate() {
            if !pr.kind.flat() && in_view(pr.x, pr.y) {
                list.push((pr.depth, D::Prop(i)));
            }
        }
        for (i, m) in self.mobs.iter().enumerate() {
            let depth = m.x + m.y - if m.alive() { 0.0 } else { 0.6 };
            if in_view(m.x, m.y) {
                list.push((depth, D::Mob(i)));
            }
        }
        for (i, n) in self.npcs.iter().enumerate() {
            if in_view(n.x, n.y) {
                list.push((n.x + n.y, D::Npc(i)));
            }
        }
        list.push((px + py, D::Player));
        for (i, b) in self.balls.iter().enumerate() {
            list.push((b.x + b.y, D::Ball(i)));
        }
        for (i, s) in self.shots.iter().enumerate() {
            list.push((s.x + s.y, D::Shot(i)));
        }
        list.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        let (psx, psy) = to_scr(px, py);
        let player_depth = px + py;
        for (depth, item) in &list {
            match *item {
                D::Wall(tx, ty) => {
                    let (sx, sy) = to_scr(tx as f32 + 0.5, ty as f32 + 0.5);
                    let w = self.art.wall(theme);
                    let top = sy - w.ay;
                    if sx < -20 || sx > scr.w + 20 || top > view_h || sy + 10 < 0 {
                        continue;
                    }
                    // Walls with floor behind them (the camera side of a room) are cut down
                    // to a low ledge so they never hide the room, like D2's front walls.
                    // On the overworld only the town palisade's camera side is cut (town behind it).
                    let behind = |dx: i32, dy: i32| {
                        let (bx, by) = (tx + dx, ty + dy);
                        self.d.get(bx, by) == Tile::Floor && (!overworld || self.safe_contains(bx as f32 + 0.5, by as f32 + 0.5, -1.5))
                    };
                    let low = [(-1, 0), (0, -1), (-1, -1)].iter().any(|&(dx, dy)| behind(dx, dy));
                    if low {
                        let drop = (w.ay - 8 - LOW_WALL).max(0);
                        scr.blit(w, sx, sy + drop, Fx { cut: w.h - drop, ..Fx::default() });
                        continue;
                    }
                    let front = *depth > player_depth + 0.3;
                    let over = (sx - psx).abs() < 26 && psy - 44 < sy + 8 && psy > top;
                    scr.blit(w, sx, sy, Fx { dither: front && over, ..Fx::default() });
                }
                D::Prop(i) => {
                    let pr = &self.props[i];
                    let (sx, sy) = to_scr(pr.x, pr.y);
                    let s = self.art.prop(pr.kind.art());
                    let top = sy - s.ay;
                    if sx + s.w / 2 < 0 || sx - s.w / 2 > scr.w || top > view_h || sy + 16 < 0 {
                        continue;
                    }
                    // Trees and houses between the camera and the player turn see-through.
                    let front = *depth > player_depth + 0.2;
                    let over = (sx - psx).abs() < s.w / 2 + 6 && psy - 44 < sy + 6 && psy > top + 4;
                    scr.blit(s, sx, sy + 8, Fx { dither: front && over, ..Fx::default() });
                    if pr.kind == PropKind::Entrance(SANCTUM) && self.quest.stage < 2 {
                        // The ash barrier.
                        let k = ((self.tick as f32) * 0.08).sin() * 0.15 + 0.45;
                        scr.blend(sx - 26, sy - 40, 52, 44, rgb(0x401010), k * 0.5);
                    }
                }
                D::Mob(i) => self.draw_mob(scr, i, to_scr(self.mobs[i].x, self.mobs[i].y)),
                D::Npc(i) => self.draw_npc(scr, i, to_scr(self.npcs[i].x, self.npcs[i].y)),
                D::Player => self.draw_player(scr, (psx, psy)),
                D::Ball(i) => {
                    let b = &self.balls[i];
                    let (sx, sy) = to_scr(b.x, b.y);
                    blend_ellipse(scr, sx, sy, 5, 2, BLACK, 0.45);
                }
                D::Shot(i) => {
                    let s = &self.shots[i];
                    let (sx, sy) = to_scr(s.x, s.y);
                    if s.kind == ShotKind::Arrow {
                        let l = (s.vx * s.vx + s.vy * s.vy).sqrt().max(0.01);
                        let (ex, ey) = iso::to_screen(s.vx / l * 0.5, s.vy / l * 0.5);
                        let (ax, ay) = (sx + scr.shake.0, sy - 18 + scr.shake.1);
                        line(scr, ax - ex as i32, ay - ey as i32, ax + ex as i32, ay + ey as i32, rgb(0xd8c8a0));
                        scr.pset(ax + ex as i32, ay + ey as i32, rgb(0xffffff));
                    }
                    blend_ellipse(scr, sx, sy, 3, 1, BLACK, 0.4);
                }
            }
        }

        // 3. Lighting.
        scr.begin_light();
        for b in &self.balls {
            let (sx, sy) = to_scr(b.x, b.y);
            scr.add_light(sx, sy, if b.ember { 60.0 } else { 110.0 }, if b.ember { 0.6 } else { 0.9 });
        }
        for l in &self.lights {
            let (sx, sy) = to_scr(l.x, l.y);
            scr.add_light(sx, sy, l.r, l.s * (l.life / l.max));
        }
        if self.p.cast_t > 0.0 {
            scr.add_light(psx, psy - 20, 70.0, 0.4 * self.p.cast_t / self.p.cast_len);
        }
        for m in &self.mobs {
            if m.burn > 0.0 && m.alive() {
                let (sx, sy) = to_scr(m.x, m.y);
                scr.add_light(sx, sy - 10, 50.0, 0.35);
            }
        }
        for s in &self.shots {
            if s.kind != ShotKind::Arrow {
                let (sx, sy) = to_scr(s.x, s.y);
                scr.add_light(sx, sy, 50.0, 0.5);
            }
        }
        for pr in &self.props {
            if pr.kind == PropKind::Campfire && in_view(pr.x, pr.y) {
                let (sx, sy) = to_scr(pr.x, pr.y);
                let fl = ((self.tick as f32) * 0.3).sin() * 6.0;
                scr.add_light(sx, sy, 150.0 + fl, 0.9);
            }
        }
        for p in &self.portals {
            if p.kind == PortalKind::TownPortal {
                let (sx, sy) = to_scr(p.x, p.y);
                scr.add_light(sx, sy - 10, 90.0, 0.7);
            }
        }
        if self.waypoints.contains(&self.level) && in_view(self.waypoint.0, self.waypoint.1) {
            let (sx, sy) = to_scr(self.waypoint.0, self.waypoint.1);
            scr.add_light(sx, sy, 70.0, 0.5);
        }
        for hy in &self.hydras {
            let (sx, sy) = to_scr(hy.x, hy.y);
            scr.add_light(sx, sy - 10, 110.0, 0.7);
        }
        for m in &self.meteors {
            let (sx, sy) = to_scr(m.x, m.y);
            scr.add_light(sx, sy, 90.0, 0.4 * m.t / crate::skills::METEOR_DELAY);
        }
        if self.p.phoenix_t > 0.0 {
            scr.add_light(psx, psy - 20, 130.0, 0.6);
        }
        for w in &self.fire_walls {
            let fade = ((w.life - w.t) / 0.6).min(1.0);
            for (k, &(x, y)) in w.segs.iter().enumerate() {
                if k % 2 == 0 && in_view(x, y) {
                    let (sx, sy) = to_scr(x, y);
                    scr.add_light(sx, sy - 8, 90.0, 0.55 * fade);
                }
            }
        }
        for p in &self.patches {
            if in_view(p.x, p.y) {
                let (sx, sy) = to_scr(p.x, p.y);
                scr.add_light(sx, sy, 50.0, 0.35 * (1.0 - p.t.max(0.0) / crate::skills::PATCH_TIME));
            }
        }
        scr.apply_light(view_h);

        // 4. Unlit, additive fire and magic on top.
        for b in &self.balls {
            let (sx, sy) = to_scr(b.x, b.y);
            let fl = ((self.tick as f32) * 0.9).sin() * 1.5;
            if b.ember {
                scr.glow(sx, sy - 22, 12.0 + fl * 0.5, rgb(0xff4008), 0.9);
                scr.disc(sx + scr.shake.0, sy - 22 + scr.shake.1, 1, rgb(0xffd080));
                continue;
            }
            scr.glow(sx, sy - 22, 26.0 + fl, rgb(0xff5010), 0.9);
            scr.glow(sx, sy - 22, 12.0, rgb(0xffd060), 1.0);
            scr.disc(sx + scr.shake.0, sy - 22 + scr.shake.1, 3, rgb(0xfff4c0));
        }
        for s in &self.shots {
            let (sx, sy) = to_scr(s.x, s.y);
            match s.kind {
                ShotKind::Hex => {
                    scr.glow(sx, sy - 18, 16.0, rgb(0xa040ff), 1.0);
                    scr.disc(sx + scr.shake.0, sy - 18 + scr.shake.1, 2, rgb(0xf0d0ff));
                }
                ShotKind::Ash => {
                    scr.glow(sx, sy - 20, 18.0, rgb(0xff3010), 1.0);
                    scr.disc(sx + scr.shake.0, sy - 20 + scr.shake.1, 2, rgb(0x301008));
                }
                ShotKind::Ice => {
                    scr.glow(sx, sy - 18, 14.0, rgb(0x60b0ff), 0.9);
                    scr.disc(sx + scr.shake.0, sy - 18 + scr.shake.1, 2, rgb(0xf0faff));
                }
                ShotKind::Boulder => {
                    scr.glow(sx, sy - 22, 14.0, rgb(0x6090c0), 0.5);
                    scr.disc(sx + scr.shake.0, sy - 22 + scr.shake.1, 6, rgb(0x1a2a3a));
                    scr.disc(sx + scr.shake.0, sy - 22 + scr.shake.1, 5, rgb(0x9ac8e8));
                    scr.disc(sx - 2 + scr.shake.0, sy - 24 + scr.shake.1, 2, rgb(0xe8f6ff));
                }
                ShotKind::Arrow => {}
            }
        }
        for w in &self.fire_walls {
            let fade = ((w.life - w.t) / 0.6).min(1.0) * (w.t / 0.2).min(1.0);
            let fl = ((self.tick as f32) * 0.4).sin() * 2.0;
            for &(x, y) in &w.segs {
                let (sx, sy) = to_scr(x, y);
                scr.glow(sx, sy - 10, 18.0 + fl, rgb(0xff4a08), 0.8 * fade);
                scr.glow(sx, sy - 6, 8.0, rgb(0xffd060), 0.7 * fade);
            }
        }
        for m in &self.meteors {
            // The meteor itself, falling from above.
            let (sx, sy) = to_scr(m.x, m.y);
            let k = m.t / crate::skills::METEOR_DELAY;
            let h = ((1.0 - k) * 220.0) as i32;
            let drift = ((1.0 - k) * 80.0) as i32;
            scr.glow(sx - drift, sy - 20 - h, 26.0, rgb(0xff4010), 1.0);
            scr.glow(sx - drift, sy - 20 - h, 12.0, rgb(0xffe080), 1.0);
            for t in 1..6 {
                let (tx, ty) = (sx - drift - t * 7, sy - 20 - h - t * 18);
                scr.glow(tx, ty, 14.0 - t as f32 * 2.0, rgb(0xff6010), 0.6 - t as f32 * 0.09);
            }
        }
        for hy in &self.hydras {
            let (sx, sy) = to_scr(hy.x, hy.y);
            let fade = ((crate::skills::HYDRA_TIME - hy.t) / 0.6).min(1.0);
            let w = ((self.tick as f32) * 0.25).sin() * 3.0;
            scr.glow(sx, sy - 8, 22.0, rgb(0xff4a08), 0.9 * fade);
            for (k, dx) in [-8.0f32, 0.0, 8.0].iter().enumerate() {
                let bob = ((self.tick as f32) * 0.2 + k as f32 * 2.1).sin() * 3.0;
                let (hx, hy2) = (sx + *dx as i32 + w as i32, sy - 26 - bob as i32 - if k == 1 { 6 } else { 0 });
                scr.glow(hx, hy2, 11.0, rgb(0xff7020), 1.0 * fade);
                scr.glow(hx, hy2, 5.0, rgb(0xfff0a0), 1.0 * fade);
            }
        }
        if self.p.phoenix_t > 0.0 {
            // Wings of flame behind the mage.
            let (psx, psy) = to_scr(self.p.x, self.p.y);
            let flap = ((self.tick as f32) * 0.3).sin() * 4.0;
            for side in [-1.0f32, 1.0] {
                for k in 0..7 {
                    let t = k as f32 / 6.0;
                    let x = psx as f32 + side * (8.0 + t * 26.0);
                    let y = psy as f32 - 34.0 - (t * 3.14).sin() * 10.0 - flap * t;
                    scr.glow(x as i32, y as i32, 12.0 - t * 4.0, rgb(0xff5010), 0.9);
                }
            }
        }
        for p in &self.patches {
            let (sx, sy) = to_scr(p.x, p.y);
            let k = 1.0 - p.t.max(0.0) / crate::skills::PATCH_TIME;
            scr.glow(sx, sy - 2, 12.0, rgb(0xff5010), 0.6 * k);
        }
        for n in &self.novas {
            let k = (n.t / crate::skills::NOVA_TIME).min(1.0);
            let fade = 1.0 - ((n.t - crate::skills::NOVA_TIME * 0.7) / 0.3).clamp(0.0, 1.0);
            let r = n.r * (0.25 + 0.75 * k);
            let steps = (r * 18.0) as i32 + 12;
            for i in 0..steps {
                let a = i as f32 / steps as f32 * std::f32::consts::TAU;
                let (sx, sy) = to_scr(n.x + a.cos() * r, n.y + a.sin() * r);
                scr.glow(sx, sy - 6, 10.0, rgb(0xff6010), 0.9 * fade);
            }
        }
        for l in &self.lights {
            let (sx, sy) = to_scr(l.x, l.y);
            let k = l.life / l.max;
            scr.glow(sx, sy - 16, 50.0 * (1.2 - k * 0.5), rgb(0xff6010), 1.2 * k);
            scr.glow(sx, sy - 16, 22.0 * (1.3 - k * 0.5), rgb(0xffe080), 1.2 * k);
        }
        for p in &self.portals {
            if p.kind == PortalKind::TownPortal {
                let (sx, sy) = to_scr(p.x, p.y);
                let k = ((self.tick as f32) * 0.1).sin() * 0.15 + 0.85;
                scr.glow(sx, sy - 20, 26.0 * k, rgb(0x4060ff), 1.1);
                scr.glow(sx, sy - 20, 12.0, rgb(0xc0e0ff), 1.0);
            }
        }
        for p in &self.parts {
            let (sx, sy) = to_scr(p.x, p.y);
            let (sx, sy) = (sx + scr.shake.0, sy - p.z as i32 + scr.shake.1);
            if sx < 0 || sy < 0 || sx >= scr.w || sy >= view_h {
                continue;
            }
            let k = p.life / p.max;
            match p.kind {
                PKind::Fire => {
                    let c = if k > 0.6 { rgb(0xffe890) } else if k > 0.3 { rgb(0xff9030) } else { rgb(0xc03010) };
                    let i = (sy * scr.w + sx) as usize;
                    scr.px[i] = add(scr.px[i], c, 0.5 + k);
                    if k > 0.5 {
                        scr.pset(sx + 1, sy, mix(scr.px[i], c, 0.6));
                    }
                }
                PKind::Magic => {
                    let i = (sy * scr.w + sx) as usize;
                    scr.px[i] = add(scr.px[i], rgb(0xb080ff), 0.6 + k);
                }
                PKind::Smoke => {
                    let r = (4.0 + (1.0 - k) * 6.0) as i32;
                    blend_ellipse(scr, sx - scr.shake.0, sy - scr.shake.1, r, r * 2 / 3, rgb(0x201c18), 0.3 * k);
                }
                PKind::Bone => scr.fill(sx, sy, 2, 1, rgb(0xb0a888)),
                PKind::Blood => scr.fill(sx, sy, 1, 1, rgb(0x801010)),
                PKind::Frost => {
                    let i = (sy * scr.w + sx) as usize;
                    scr.px[i] = add(scr.px[i], rgb(0xc0e8ff), 0.4 + k);
                    scr.pset(sx + 1, sy, rgb(0xf0faff));
                }
            }
        }
        scr.shake = (0, 0);

        // Weather: snowfall over the Frostmarch (gusting), frost motes in the ice dungeons.
        if self.theme.cold() {
            let open = self.theme.open();
            let (cx, cy) = iso::to_screen(self.p.x, self.p.y);
            let t = self.tick as f32 / 60.0;
            let gust = if open { 0.6 + 0.4 * (t * 0.13).sin() } else { 0.25 };
            let n = if open { (170.0 * gust) as i32 } else { 40 };
            let (w, hgt) = (scr.w as f32, view_h as f32);
            for i in 0..n {
                let h1 = ((i as u32).wrapping_mul(2654435761) >> 8) as f32 / 16_777_216.0;
                let h2 = ((i as u32).wrapping_mul(40503).wrapping_add(977) % 1000) as f32 / 1000.0;
                let speed = if open { 30.0 + h2 * 40.0 } else { 6.0 + h2 * 6.0 };
                let drift = if open { 25.0 + gust * 35.0 } else { 3.0 };
                let x = (h1 * w * 3.0 - cx + t * drift + (t * 1.3 + h2 * 9.0).sin() * 6.0).rem_euclid(w);
                let y = (h2 * hgt * 3.0 - cy + t * speed).rem_euclid(hgt);
                let (xi, yi) = (x as i32, y as i32);
                if yi >= view_h - crate::game::HUD_H {
                    continue;
                }
                let c = if open { rgb(0xf4f8ff) } else { rgb(0x9ad4ff) };
                if open && h2 > 0.7 {
                    scr.fill(xi, yi, 2, 2, c);
                } else {
                    let idx = (yi * scr.w + xi) as usize;
                    if idx < scr.px.len() {
                        scr.px[idx] = mix(scr.px[idx], c, if open { 0.85 } else { 0.5 + 0.5 * (t * 3.0 + h1 * 20.0).sin().abs() });
                    }
                }
            }
        }

        // Health bars over wounded monsters, names over people.
        for m in &self.mobs {
            if !m.alive() || m.hp >= m.max_hp || m.boss || !in_view(m.x, m.y) {
                continue;
            }
            let (sx, sy) = to_scr(m.x, m.y);
            let (art, scale, ..) = self.art.char_art(def(m.kind).art);
            let h = (art.height as f32 * scale) as i32;
            let w = 22;
            let f = ((m.hp / m.max_hp) * w as f32).ceil() as i32;
            scr.fill(sx - w / 2 - 1, sy - h - 6, w + 2, 4, BLACK);
            scr.fill(sx - w / 2, sy - h - 5, f, 2, rgb(0xc02020));
        }
        for (i, n) in self.npcs.iter().enumerate() {
            let (sx, sy) = to_scr(n.x, n.y);
            let near = (n.x - px).powi(2) + (n.y - py).powi(2) < 9.0;
            if self.hover_npc == Some(i) || near {
                scr.text(n.name, sx, sy - 62, rgb(0xe0d0a0), Align::Center, 1);
            }
            if (n.role == Role::Elder && self.quest.elder_has_news()) || (n.role == Role::Captain && self.quest.captain_has_news()) {
                let bob = (((self.tick as f32) * 0.12).sin() * 2.0) as i32;
                scr.text("!", sx - 2, sy - 66 + bob, rgb(0xffd040), Align::Center, 2);
            }
        }
        // Item names on the floor (D2 labels) for gear near you, stacked so they don't overlap.
        let mut labels: Vec<(i32, i32, &crate::items::Item)> = vec![];
        for k in &self.pickups {
            if let Drop::Item(it) = &k.kind {
                let d2 = (k.x - self.p.x).powi(2) + (k.y - self.p.y).powi(2);
                if d2 < 49.0 || (it.rarity >= crate::items::Rarity::Rare && d2 < 200.0) {
                    let (sx, sy) = to_scr(k.x, k.y);
                    labels.push((sx, sy, it));
                }
            }
        }
        labels.sort_by_key(|l| -l.1);
        let mut placed: Vec<(i32, i32, i32)> = vec![];
        for (sx, sy, it) in labels {
            let w = crate::gfx::text_width(&it.name, 1) + 4;
            let mut y = sy - 16;
            for _ in 0..8 {
                if placed.iter().any(|&(px, py, pw)| (y - py).abs() < 11 && (sx - px).abs() * 2 < w + pw) {
                    y -= 11;
                } else {
                    break;
                }
            }
            placed.push((sx, y, w));
            scr.blend(sx - w / 2, y - 1, w, 10, BLACK, 0.65);
            scr.text(&it.name, sx, y, it.col(), Align::Center, 1);
        }
        for f in &self.floaters {
            let (sx, sy) = to_scr(f.x, f.y);
            let rise = (f.t * 30.0) as i32;
            let col = if f.t > 0.7 { mix(f.col, BLACK, (f.t - 0.7) / 0.3) } else { f.col };
            scr.text(&f.text, sx, sy - 58 - rise, col, Align::Center, 1);
        }

        if self.show_map {
            self.draw_map(scr);
        }
        (self.hud_skill_rects, self.hud_bag) = self.draw_hud(scr);
        self.dlg_rects.clear();
        if self.dialog.is_some() {
            self.draw_dialog(scr);
        }
        self.draw_tree(scr);
        self.draw_inventory(scr);
        self.draw_overlays(scr);
    }

    /// D2-style automap overlay: explored walls projected isometrically, centred on the player.
    fn draw_map(&self, scr: &mut Screen) {
        let (cx, cy) = (scr.w / 2, (self.view_h - HUD_H) / 2);
        let (px, py) = (self.p.x, self.p.y);
        let k = if self.level.overland() { 2 } else { 3 };
        let proj = |x: f32, y: f32| -> (i32, i32) { (cx + ((x - px) - (y - py)) as i32 * k, cy + ((x - px) + (y - py)) as i32 * k / 2) };
        let wall = rgb(0xc8b088);
        let floor = rgb(0x3a3024);
        let tree = rgb(0x2a5a2a);
        for ty in 0..self.d.h {
            for tx in 0..self.d.w {
                let i = (ty * self.d.w + tx) as usize;
                if !self.explored[i] {
                    continue;
                }
                let (sx, sy) = proj(tx as f32, ty as f32);
                if sy >= self.view_h - HUD_H || sy < 0 || sx < 0 || sx >= scr.w {
                    continue;
                }
                match self.d.get(tx, ty) {
                    Tile::Wall => scr.fill(sx, sy, k, 2, wall),
                    Tile::Prop => scr.fill(sx, sy, 2, 1, tree),
                    Tile::Floor => {
                        let road = self.d.ground_at(tx, ty) == 2;
                        let j = (sy * scr.w + sx) as usize;
                        scr.px[j] = mix(scr.px[j], if road { rgb(0x8a7050) } else { floor }, 0.8);
                    }
                    Tile::Void => {}
                }
            }
        }
        for p in &self.portals {
            let (sx, sy) = proj(p.x, p.y);
            let col = match p.kind {
                PortalKind::Entrance(SANCTUM) if self.quest.stage < 2 => rgb(0x802020),
                PortalKind::TownPortal => rgb(0x6080ff),
                _ => rgb(0xffd040),
            };
            scr.fill(sx - 2, sy - 2, 5, 5, col);
        }
        {
            let (sx, sy) = proj(self.waypoint.0, self.waypoint.1);
            let col = if self.waypoints.contains(&self.level) { rgb(0x80a0ff) } else { rgb(0x707080) };
            scr.fill(sx - 3, sy, 7, 1, col);
            scr.fill(sx, sy - 3, 1, 7, col);
        }
        for n in &self.npcs {
            let (sx, sy) = proj(n.x, n.y);
            scr.fill(sx - 1, sy - 1, 2, 2, rgb(0x60d060));
        }
        let few = self.alive_mobs() <= 5 && !self.level.overland();
        for m in &self.mobs {
            if !m.alive() {
                continue;
            }
            let seen = self.explored[(m.y as i32 * self.d.w + m.x as i32) as usize];
            if seen || few || m.boss {
                let (sx, sy) = proj(m.x, m.y);
                let s = if m.boss { 5 } else { 3 };
                scr.fill(sx - s / 2, sy - s / 2, s, s, rgb(0xe02020));
            }
        }
        let (sx, sy) = proj(px, py);
        scr.fill(sx - 1, sy - 2, 3, 4, WHITE);
        scr.text("MAP", scr.w - 30, 8, rgb(0xc8b088), Align::Center, 1);
    }

    /// Draws a character sprite, using another sheet scaled and tinted while its own art is missing.
    fn blit_char(&self, scr: &mut Screen, art_name: &str, anim: CharFrame, (sx, sy): (i32, i32), mut fx: Fx, bob: bool) {
        let (art, scale, tint, tint_a) = self.art.char_art(art_name);
        let spr = anim.pick(art);
        if tint_a > 0.0 && fx.tint_a == 0.0 {
            fx.tint = tint;
            fx.tint_a = tint_a;
        }
        // No walk cycle yet: bob a little so movement still reads.
        let dy = if bob && !art.has("walk") { (((self.tick as f32) * 0.5).sin().abs() * -2.0) as i32 } else { 0 };
        if (scale - 1.0).abs() < 0.01 {
            scr.blit(spr, sx, sy + dy, fx);
        } else {
            scr.blit_scaled(spr, sx, sy + dy, scale, fx);
        }
    }

    fn draw_player(&self, scr: &mut Screen, (sx, sy): (i32, i32)) {
        blend_ellipse(scr, sx, sy, 11, 4, BLACK, 0.5);
        let art = self.art.char("mage");
        let anim = if self.p.cast_t > 0.0 && art.has("cast") {
            CharFrame::At("cast", self.p.dir, 1.0 - self.p.cast_t / self.p.cast_len)
        } else if self.p.moving {
            CharFrame::Loop("walk", self.p.dir, self.p.anim_t)
        } else {
            CharFrame::Loop("idle", self.p.dir, 0.0)
        };
        let mut fx = Fx::default();
        if self.p.flash > 0.0 {
            fx.tint = rgb(0xff2020);
            fx.tint_a = 0.5;
        }
        if let State::Dead(t) = self.state {
            let spr = anim.pick(art);
            fx.tint = rgb(0x400000);
            fx.tint_a = (t * 0.8).min(0.7);
            fx.cut = (spr.h as f32 * (1.0 - (t * 0.6).min(0.6))) as i32;
            scr.blit(spr, sx, sy + (t.min(1.0) * 14.0) as i32, fx);
            return;
        }
        let _ = CAST_TIME;
        if self.p.chill > 0.0 && fx.tint_a == 0.0 {
            fx.tint = rgb(0x80c8ff);
            fx.tint_a = 0.35;
        }
        self.blit_char(scr, "mage", anim, (sx, sy), fx, self.p.moving);
    }

    fn draw_npc(&self, scr: &mut Screen, i: usize, (sx, sy): (i32, i32)) {
        let n = &self.npcs[i];
        blend_ellipse(scr, sx, sy, 10, 4, BLACK, 0.45);
        let anim = if n.moving { CharFrame::Loop("walk", n.dir, n.anim_t) } else { CharFrame::Loop("idle", n.dir, 0.0) };
        let mut fx = Fx::default();
        if self.hover_npc == Some(i) {
            fx.tint = rgb(0xfff0c0);
            fx.tint_a = 0.2;
        }
        self.blit_char(scr, n.art, anim, (sx, sy), fx, n.moving);
    }

    fn draw_mob(&self, scr: &mut Screen, i: usize, (sx, sy): (i32, i32)) {
        let m = &self.mobs[i];
        let name = def(m.kind).art;
        let (art, scale, ..) = self.art.char_art(name);
        let mut fx = Fx::default();
        let anim = match m.state {
            MobState::Dead(t) => {
                // Collapse: flash, then sink into the floor and fade.
                let dur = if m.boss { 3.0 } else { 1.6 };
                if t > dur {
                    return;
                }
                fx.tint = if matches!(m.kind, crate::mobs::Kind::Zombie | crate::mobs::Kind::PlagueWarden) { rgb(0x300808) } else { rgb(0x202020) };
                fx.tint_a = (t * 1.5).min(0.8);
                let s = art.frame("idle", m.dir, 0.0);
                fx.cut = (s.ay as f32 - (t / dur) * s.ay as f32 * 0.9) as i32;
                let sink = (t / dur * art.height as f32 * 0.9 * scale) as i32;
                if (scale - 1.0).abs() < 0.01 {
                    scr.blit(s, sx, sy + sink, fx);
                } else {
                    scr.blit_scaled(s, sx, sy + sink, scale, fx);
                }
                return;
            }
            MobState::Attack(t) => CharFrame::At("attack", m.dir, 1.0 - t / m.windup),
            _ if m.moving => CharFrame::Loop("walk", m.dir, m.anim_t),
            _ => CharFrame::Loop("idle", m.dir, 0.0),
        };
        let shadow = if m.boss { 20 } else { 10 };
        blend_ellipse(scr, sx, sy, shadow, shadow * 2 / 5, BLACK, 0.45);
        if m.flash > 0.0 {
            fx.tint = WHITE;
            fx.tint_a = 0.7;
        } else if m.burn > 0.0 && (self.tick / 4) % 2 == 0 {
            fx.tint = rgb(0xff6020);
            fx.tint_a = 0.25;
        } else if m.flee > 0.0 {
            fx.tint = rgb(0xffffff);
            fx.tint_a = 0.12;
        }
        if self.hover == Some(i) && fx.tint_a == 0.0 {
            fx.tint = rgb(0xffe0a0);
            fx.tint_a = 0.15;
        }
        if m.enraged && fx.tint_a == 0.0 {
            fx.tint = rgb(0xff2010);
            fx.tint_a = 0.18;
        }
        // Champions are tinted blue; elites stand in a golden glow (D2).
        match m.rank {
            crate::mobs::Rank::Champion if fx.tint_a == 0.0 => {
                fx.tint = rgb(0x4060ff);
                fx.tint_a = 0.22;
            }
            crate::mobs::Rank::Elite => blend_ellipse(scr, sx, sy, 16, 6, rgb(0xd8a040), 0.35),
            _ => {}
        }
        if m.mods & crate::mobs::M_FIERY != 0 && (self.tick / 6 + i as u32) % 5 == 0 {
            scr.glow(sx, sy - 14, 10.0, rgb(0xff6020), 0.5);
        }
        self.blit_char(scr, name, anim, (sx, sy), fx, m.moving);
    }

    /// Draws the HUD; returns the clickable skill button rectangles.
    fn draw_hud(&self, scr: &mut Screen) -> (Vec<(i32, i32, i32, i32)>, (i32, i32, i32, i32)) {
        let (w, h) = (scr.w, self.view_h);
        let top = h - HUD_H;
        for y in top..h {
            let k = (y - top) as f32 / HUD_H as f32;
            scr.fill(0, y, w, 1, mix(rgb(0x2a2520), rgb(0x141210), k));
        }
        scr.fill(0, top, w, 1, rgb(0x5a4a38));
        scr.fill(0, top + 1, w, 1, rgb(0x0a0806));
        // Experience bar along the top of the panel.
        let xp = (self.p.xp / xp_to_next(self.p.clvl)).clamp(0.0, 1.0);
        scr.fill(70, top + 2, ((w - 140) as f32 * xp) as i32, 1, rgb(0xc8a040));
        // Globes.
        let gy = h - 28;
        globe(scr, 34, gy, 26, self.p.hp / self.p.max_hp, rgb(0xb01818), rgb(0xff6050));
        globe(scr, w - 34, gy, 26, self.p.mana / self.p.max_mana, rgb(0x1830b0), rgb(0x6090ff));
        scr.text(&format!("{}/{}", self.p.hp.ceil() as i32, self.p.max_hp as i32), 34, gy - 4, WHITE, Align::Center, 1);
        scr.text(&format!("{}/{}", self.p.mana.floor() as i32, self.p.max_mana as i32), w - 34, gy - 4, WHITE, Align::Center, 1);
        // Skill slots: primary (left click / A) and secondary (right click / X), like D2.
        let ix = w / 2 - 12;
        let iy = top + 8;
        let mut skill_rects = vec![];
        for (k, s) in [self.p.skills.primary, self.p.skills.secondary].into_iter().enumerate() {
            let sx = ix - 24 + k as i32 * 32;
            scr.fill(sx - 2, iy - 2, 28, 28, rgb(0x5a4a38));
            scr.blit(&crate::sprites::skill_icon(s), sx, iy, Fx::default());
            let r = self.p.skills.rank(s);
            if self.p.mana < crate::skills::mana_cost(s, r) && self.p.phoenix_t <= 0.0 {
                scr.blend(sx, iy, 24, 24, rgb(0x000040), 0.6);
            }
            let cd = self.p.skills.cooldown[s as usize];
            if cd > 0.0 {
                let full = crate::skills::cooldown_of(s).max(0.01);
                scr.blend(sx, iy, 24, (24.0 * cd / full) as i32, BLACK, 0.7);
            }
            scr.text(["L", "R"][k], sx + 1, iy + 1, rgb(0xd8c090), Align::Left, 1);
            skill_rects.push((sx - 2, iy - 2, 28, 28));
        }
        let out = self.p.mana < crate::skills::mana_cost(self.p.skills.primary, self.p.skills.rank(self.p.skills.primary));
        if out {
            scr.text("EMBER", ix + 4, iy + 28, rgb(0xff9050), Align::Center, 1);
        }
        // Unspent skill points: a pulsing button (opens the tree, like D2's level-up button).
        if self.p.skills.points > 0 {
            let bx = ix - 46;
            let pulse = (self.tick / 15) % 2 == 0;
            scr.fill(bx, iy + 2, 18, 18, if pulse { rgb(0xd8a048) } else { rgb(0x8a6020) });
            scr.text("+", bx + 6, iy + 7, BLACK, Align::Left, 1);
            skill_rects.push((bx, iy + 2, 18, 18));
        }
        // Run / walk button (D2 style).
        let rx = ix + 40;
        let _ = FIREBALL_COST;
        scr.fill(rx - 2, iy - 2, 28, 28, rgb(0x5a4a38));
        let lit = self.p.running && !self.p.winded;
        scr.fill(rx, iy, 24, 24, if lit { rgb(0x5a3a10) } else { rgb(0x1a1410) });
        let col = if self.p.winded {
            rgb(0xc05030)
        } else if self.p.running {
            rgb(0xffd070)
        } else {
            rgb(0x908070)
        };
        scr.text(if self.p.running { "RUN" } else { "WALK" }, rx + 12, iy + 8, col, Align::Center, 1);
        let label = if self.p.winded { "TIRED" } else { "R/B" };
        scr.text(label, rx + 12, iy + 28, if self.p.winded { col } else { rgb(0x908070) }, Align::Center, 1);
        // Bag button (inventory).
        let gx = rx + 32;
        scr.fill(gx - 2, iy - 2, 28, 28, rgb(0x5a4a38));
        scr.fill(gx, iy, 24, 24, rgb(0x1a1410));
        scr.text("BAG", gx + 12, iy + 8, rgb(0xd8c090), Align::Center, 1);
        scr.text("I/ST", gx + 12, iy + 28, rgb(0x908070), Align::Center, 1);
        let bag = (gx - 2, iy - 2, 28, 28);
        // Stamina and food bars.
        let (bar_x, bar_w) = (156, 100);
        let st = self.p.stamina / MAX_STAMINA;
        let st_col = if self.p.winded { rgb(0xa03020) } else { rgb(0xd8b020) };
        scr.text("STAMINA", bar_x, top + 7, rgb(0xb0a090), Align::Left, 1);
        bar(scr, bar_x, top + 17, bar_w, st, st_col);
        let fd = self.p.food / MAX_FOOD;
        let starving = self.p.food <= 0.0;
        let fd_col = if fd < 0.25 { rgb(0xc04020) } else { rgb(0xb07030) };
        let flabel = if starving { "FOOD - STARVING!" } else if fd < 0.25 { "FOOD - HUNGRY" } else { "FOOD" };
        let fcol = if starving && (self.tick / 20) % 2 == 0 { rgb(0xff5030) } else { rgb(0xb0a090) };
        scr.text(flabel, bar_x, top + 27, fcol, Align::Left, 1);
        bar(scr, bar_x, top + 37, bar_w, fd, fd_col);
        // Potions.
        let bx = 70;
        potion(scr, bx, top + 12, rgb(0xc02020));
        scr.text(&format!("X{}", self.p.hp_pots), bx + 14, top + 16, WHITE, Align::Left, 1);
        scr.text("Q", bx + 2, top + 32, rgb(0x908070), Align::Left, 1);
        potion(scr, bx + 44, top + 12, rgb(0x2040c0));
        scr.text(&format!("X{}", self.p.mp_pots), bx + 58, top + 16, WHITE, Align::Left, 1);
        scr.text("E", bx + 46, top + 32, rgb(0x908070), Align::Left, 1);
        scr.text(&format!("GOLD {}", self.p.gold), w - 80, top + 8, rgb(0xe8c050), Align::Right, 1);
        scr.text(&format!("CHAR LEVEL {}", self.p.clvl), w - 80, top + 20, rgb(0xd8c090), Align::Right, 1);
        scr.text(&format!("SEALS {}/3", self.quest.seal_count()), w - 80, top + 32, rgb(0xc8a0ff), Align::Right, 1);

        // Area name and quest log (top left).
        scr.text(&self.level_name, 6, 6, rgb(0xd8b878), Align::Left, 1);
        let log = if self.level.act() == 1 { self.quest.log2() } else { self.quest.log() };
        scr.text(&log, 6, 17, rgb(0x9a8a78), Align::Left, 1);

        // Boss bar (big, top centre) while a boss is fighting you; else the hovered monster.
        let boss = self.mobs.iter().position(|m| m.boss && m.alive() && m.state != MobState::Idle);
        if let Some(i) = boss {
            let m = &self.mobs[i];
            let bw = 260;
            let f = (m.hp / m.max_hp * bw as f32) as i32;
            scr.fill(w / 2 - bw / 2 - 2, 26, bw + 4, 14, rgb(0x5a4a38));
            scr.fill(w / 2 - bw / 2, 28, bw, 10, BLACK);
            scr.fill(w / 2 - bw / 2, 28, f, 10, if m.enraged { rgb(0xc02010) } else { rgb(0x901010) });
            scr.text(def(m.kind).label, w / 2, 16, rgb(0xffd0a0), Align::Center, 1);
        } else if let Some(i) = self.hover.or(self.focus) {
            let m = &self.mobs[i];
            if m.alive() {
                let bw = 150;
                let f = (m.hp / m.max_hp * bw as f32) as i32;
                scr.fill(w / 2 - bw / 2 - 1, 30, bw + 2, 14, BLACK);
                scr.fill(w / 2 - bw / 2, 31, f, 12, rgb(0x801010));
                scr.text(&m.label(), w / 2, 33, rgb(m.name_col()), Align::Center, 1);
                if m.mods != 0 {
                    let t = crate::mobs::mod_text(m.mods);
                    let tw = crate::gfx::text_width(&t, 1);
                    scr.blend(w / 2 - tw / 2 - 2, 45, tw + 4, 10, BLACK, 0.6);
                    scr.text(&t, w / 2, 46, rgb(0x8098ff), Align::Center, 1);
                }
            }
        }
        if let Some((text, t)) = &self.message {
            let c = if *t < 0.5 { mix(BLACK, rgb(0xe8d0a0), *t * 2.0) } else { rgb(0xe8d0a0) };
            scr.text(text, w / 2, top - 70, c, Align::Center, 1);
        }
        if self.level_up_t > 0.0 {
            let a = self.level_up_t.min(1.0);
            scr.text("LEVEL UP!", w / 2, top / 2 - 70, mix(BLACK, rgb(0xffe080), a), Align::Center, 2);
        }
        if self.banner_t > 0.0 && self.dialog.is_none() {
            let a = (self.banner_t.min(1.0)).clamp(0.0, 1.0);
            let c = mix(BLACK, rgb(0xd8a048), a);
            let first = self.stats.levels_entered == 0;
            let title = if first { "ASHEN SANCTUM".to_string() } else { self.level_name.clone() };
            let sc = if first { 3 } else { 2 };
            scr.text(&title, w / 2, top / 2 - 60, c, Align::Center, sc);
            if first {
                scr.text("HOLLOWMERE, ON THE EDGE OF THE ASHLANDS", w / 2, top / 2 - 30, mix(BLACK, rgb(0xb0a090), a), Align::Center, 1);
                let hint = mix(BLACK, rgb(0x8a7a68), a);
                scr.text("CLICK PEOPLE (OR PRESS A / F NEAR THEM) TO TALK. FIND ELDER MAREN BY THE FIRE.", w / 2, top - 46, hint, Align::Center, 1);
                scr.text("LEFT CLICK: MOVE   RIGHT CLICK: SKILL   K: SKILLS   I: BAG   R: RUN   Q / E: POTIONS   TAB: MAP", w / 2, top - 34, hint, Align::Center, 1);
                scr.text("PAD: STICK MOVE   A / X: SKILLS   B: RUN   START: BAG   SELECT: MAP (HOLD: SKILLS)", w / 2, top - 22, hint, Align::Center, 1);
            }
        }
        (skill_rects, bag)
    }

    fn draw_dialog(&mut self, scr: &mut Screen) {
        let Some(d) = self.dialog.as_ref() else { return };
        let top = self.view_h - HUD_H;
        let (bw, x) = (440, scr.w / 2 - 220);
        let text = &d.pages[d.page.min(d.pages.len() - 1)];
        let lines = story::wrap(text, 70);
        let bh = 28 + lines.len() as i32 * 11 + d.options.len() as i32 * 12 + 8;
        let y = top - bh - 8;
        scr.blend(x, y, bw, bh, rgb(0x0c0a08), 0.88);
        scr.fill(x, y, bw, 1, rgb(0x8a7050));
        scr.fill(x, y + bh - 1, bw, 1, rgb(0x8a7050));
        scr.fill(x, y, 1, bh, rgb(0x8a7050));
        scr.fill(x + bw - 1, y, 1, bh, rgb(0x8a7050));
        scr.text(d.name, x + 10, y + 8, rgb(0xffd080), Align::Left, 1);
        for (i, l) in lines.iter().enumerate() {
            scr.text(l, x + 10, y + 22 + i as i32 * 11, rgb(0xe0d8c8), Align::Left, 1);
        }
        let oy = y + 26 + lines.len() as i32 * 11;
        let mut rects = vec![];
        for (i, (label, _)) in d.options.iter().enumerate() {
            let ry = oy + i as i32 * 12;
            let sel = i == d.sel;
            if sel {
                scr.blend(x + 6, ry - 2, bw - 12, 11, rgb(0x5a3a10), 0.8);
            }
            let c = if sel { rgb(0xffe0a0) } else { rgb(0xa09080) };
            scr.text(&format!("{} {}", if sel { ">" } else { " " }, label), x + 10, ry, c, Align::Left, 1);
            rects.push((x + 6, ry - 2, bw - 12, 11));
        }
        scr.text(&format!("GOLD {}", self.p.gold), x + bw - 10, y + 8, rgb(0xe8c050), Align::Right, 1);
        self.dlg_rects = rects;
    }

    fn draw_overlays(&self, scr: &mut Screen) {
        let (w, top) = (scr.w, self.view_h - HUD_H);
        match self.state {
            State::Dead(t) => {
                scr.blend(0, 0, w, top, BLACK, (t * 0.4).min(0.5));
                scr.text("YOU HAVE DIED", w / 2, top / 2 - 20, rgb(0xc02020), Align::Center, 3);
                if t > 1.5 {
                    let town = Game::waypoint_name(LevelId::land(self.level.act()));
                    scr.text(&format!("PRESS ENTER OR START TO WAKE IN {town}"), w / 2, top / 2 + 14, rgb(0xb0a090), Align::Center, 1);
                }
            }
            State::Victory(t) => {
                scr.blend(0, 0, w, top, BLACK, (t * 0.3).min(0.7));
                let epilogue = if self.quest.stage2 >= 3 { story::EPILOGUE2 } else { story::EPILOGUE };
                for (i, line) in epilogue.iter().enumerate() {
                    let a = ((t - i as f32 * 1.2) * 0.8).clamp(0.0, 1.0);
                    if a <= 0.0 {
                        continue;
                    }
                    let (sc, col) = if i == 0 { (3, rgb(0xffc060)) } else { (1, rgb(0xd8c8b0)) };
                    let y = top / 2 - 60 + i as i32 * 26 + if i > 0 { 14 } else { 0 };
                    for (j, l) in story::wrap(line, 80).iter().enumerate() {
                        scr.text(l, w / 2, y + j as i32 * 10, mix(BLACK, col, a), Align::Center, sc);
                    }
                }
                if t > 3.0 {
                    scr.text("PRESS ENTER TO CONTINUE EXPLORING", w / 2, top - 24, rgb(0x9a8a78), Align::Center, 1);
                }
            }
            State::Playing => {}
        }
    }
}

/// Which frame of a character sheet to draw.
#[derive(Clone, Copy)]
enum CharFrame {
    /// (anim, dir, time) looping.
    Loop(&'static str, usize, f32),
    /// (anim, dir, progress 0..1) one-shot.
    At(&'static str, usize, f32),
}

impl CharFrame {
    fn pick(self, art: &CharArt) -> &Sprite {
        match self {
            CharFrame::Loop(a, d, t) => art.frame(a, d, t),
            CharFrame::At(a, d, p) => art.frame_at(a, d, p),
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_hazard(scr: &mut Screen, kind: HazardKind, sx: i32, sy: i32, r: f32, t: f32, warn: f32, live: f32, tick: u32) {
    let rx = (r * iso::TW * 0.5) as i32;
    let ry = rx / 2;
    if t < warn {
        // Telegraph: a growing marker and a pulsing ring.
        let k = t / warn;
        let col = match kind {
            HazardKind::Poison => rgb(0x60c020),
            HazardKind::Frost | HazardKind::Icicle | HazardKind::Quake => rgb(0x60b0ff),
            HazardKind::Nova => rgb(0xff3010),
        };
        blend_ellipse(scr, sx, sy, (rx as f32 * k) as i32, (ry as f32 * k) as i32, col, 0.35);
        ring(scr, sx, sy, rx, ry, if (tick / 4) % 2 == 0 { col } else { rgb(0xffffff) });
    } else if kind == HazardKind::Frost {
        let fade = (1.0 - (t - warn) / live.max(0.01)).clamp(0.0, 1.0);
        blend_ellipse(scr, sx, sy, rx, ry, rgb(0xb0e0ff), 0.5 * fade + 0.1);
        for k in 0..6 {
            let a = k as f32 * 1.05 + tick as f32 * 0.02;
            let (dx, dy) = ((a.cos() * rx as f32 * 0.7) as i32, (a.sin() * ry as f32 * 0.7) as i32);
            scr.fill(sx + dx - 1, sy + dy, 3, 1, rgb(0xf0faff));
        }
    } else if matches!(kind, HazardKind::Icicle | HazardKind::Quake) && t < warn + 0.3 {
        // The impact: a burst of white.
        let k = ((t - warn) / 0.3).clamp(0.0, 1.0);
        blend_ellipse(scr, sx, sy, rx, ry, rgb(0xe8f4ff), 0.6 * (1.0 - k));
        ring(scr, sx, sy, (rx as f32 * (0.6 + k * 0.5)) as i32, (ry as f32 * (0.6 + k * 0.5)) as i32, rgb(0xffffff));
    } else if kind == HazardKind::Poison {
        let fade = (1.0 - (t - warn) / live.max(0.01)).clamp(0.0, 1.0);
        blend_ellipse(scr, sx, sy, rx, ry, rgb(0x40a018), 0.45 * fade + 0.1);
        for k in 0..5 {
            let a = (tick as f32 * 0.05 + k as f32 * 1.3).sin();
            scr.disc(sx + (a * rx as f32 * 0.6) as i32, sy + ((k as f32 * 2.1).cos() * ry as f32 * 0.5) as i32, 1, rgb(0xa0ff60));
        }
    }
}

fn ring(scr: &mut Screen, cx: i32, cy: i32, rx: i32, ry: i32, c: u32) {
    let n = (rx * 4).max(12);
    for i in 0..n {
        let a = i as f32 / n as f32 * std::f32::consts::TAU;
        scr.pset(cx + (a.cos() * rx as f32) as i32, cy + (a.sin() * ry as f32) as i32, c);
    }
}

fn line(scr: &mut Screen, x0: i32, y0: i32, x1: i32, y1: i32, c: u32) {
    let n = (x1 - x0).abs().max((y1 - y0).abs()).max(1);
    for i in 0..=n {
        scr.pset(x0 + (x1 - x0) * i / n, y0 + (y1 - y0) * i / n, c);
    }
}

pub(crate) fn blend_ellipse(scr: &mut Screen, cx: i32, cy: i32, rx: i32, ry: i32, c: u32, a: f32) {
    let (cx, cy) = (cx + scr.shake.0, cy + scr.shake.1);
    let (rx, ry) = (rx.max(1), ry.max(1));
    for y in -ry..=ry {
        for x in -rx..=rx {
            let d = (x as f32 / rx as f32).powi(2) + (y as f32 / ry as f32).powi(2);
            if d <= 1.0 {
                let (px, py) = (cx + x, cy + y);
                if px >= 0 && py >= 0 && px < scr.w && py < scr.h {
                    let i = (py * scr.w + px) as usize;
                    scr.px[i] = mix(scr.px[i], c, a * (1.0 - d * 0.5));
                }
            }
        }
    }
}

fn globe(scr: &mut Screen, cx: i32, cy: i32, r: i32, frac: f32, col: u32, hi: u32) {
    let level = cy + r - (2.0 * r as f32 * frac.clamp(0.0, 1.0)) as i32;
    scr.disc(cx, cy, r + 2, rgb(0x5a4a38));
    scr.disc(cx, cy, r + 1, BLACK);
    for y in -r..=r {
        for x in -r..=r {
            if x * x + y * y > r * r {
                continue;
            }
            let (px, py) = (cx + x, cy + y);
            let c = if py >= level {
                let shade = 1.0 - ((x + r / 3) as f32).hypot((y + r / 3) as f32) / (r as f32 * 1.6);
                mix(mix(col, BLACK, 0.5), hi, shade.clamp(0.0, 1.0) * 0.6)
            } else {
                rgb(0x0c0a0a)
            };
            scr.pset(px, py, c);
        }
    }
    scr.disc(cx - r / 3, cy - r / 2, 3, mix(rgb(0xffffff), col, 0.5));
}

fn potion(scr: &mut Screen, x: i32, y: i32, col: u32) {
    scr.fill(x + 3, y, 4, 3, rgb(0x806040));
    scr.disc(x + 5, y + 9, 5, BLACK);
    scr.disc(x + 5, y + 9, 4, col);
    scr.pset(x + 3, y + 7, rgb(0xffffff));
}

fn bar(scr: &mut Screen, x: i32, y: i32, w: i32, frac: f32, col: u32) {
    scr.fill(x - 1, y - 1, w + 2, 7, BLACK);
    scr.fill(x, y, w, 5, rgb(0x201a14));
    let f = (frac.clamp(0.0, 1.0) * w as f32) as i32;
    scr.fill(x, y, f, 5, col);
    scr.fill(x, y, f, 1, mix(col, WHITE, 0.35));
}

/// Code-drawn food for when no generated sprite exists.
fn draw_food_fallback(scr: &mut Screen, i: usize, sx: i32, sy: i32) {
    match i {
        0 => {
            scr.disc(sx, sy, 4, BLACK);
            scr.disc(sx, sy, 3, rgb(0xc02020));
            scr.pset(sx - 1, sy - 2, rgb(0xff8080));
            scr.fill(sx, sy - 5, 1, 2, rgb(0x5a3a1c));
            scr.fill(sx + 1, sy - 5, 2, 1, rgb(0x40a030));
        }
        1 => {
            scr.fill(sx - 6, sy - 3, 12, 7, BLACK);
            scr.fill(sx - 5, sy - 2, 10, 5, rgb(0xb07830));
            scr.fill(sx - 4, sy - 2, 8, 1, rgb(0xe0b060));
        }
        _ => {
            scr.disc(sx - 1, sy, 4, BLACK);
            scr.disc(sx - 1, sy, 3, rgb(0x904818));
            scr.fill(sx + 2, sy - 1, 5, 2, rgb(0xe8e0c8));
            scr.pset(sx - 2, sy - 1, rgb(0xd08040));
        }
    }
}

fn draw_pickup(scr: &mut Screen, k: &Pickup, sx: i32, sy: i32, tick: u32, art: &Art) {
    let bob = (((tick as f32) * 0.1 + k.x).sin() * 1.5) as i32;
    let pop = if k.t < 0.3 { ((0.3 - k.t) * 40.0) as i32 } else { 0 };
    blend_ellipse(scr, sx, sy, 5, 2, BLACK, 0.5);
    match &k.kind {
        Drop::Item(it) => {
            if it.rarity >= crate::items::Rarity::Rare {
                scr.glow(sx, sy - 4 - pop, 14.0, it.col(), 0.35);
            }
            crate::inventory::draw_icon(scr, art, it, sx, sy - 5 - pop, 0.5);
        }
        Drop::Health => potion(scr, sx - 5, sy - 16 - pop + bob, rgb(0xc02020)),
        Drop::Mana => potion(scr, sx - 5, sy - 16 - pop + bob, rgb(0x2040c0)),
        &Drop::Gold(_) => {
            for (dx, dy) in [(-3, 0), (2, -1), (0, -3), (-1, 1)] {
                scr.fill(sx + dx, sy - 3 + dy - pop, 3, 2, rgb(0xe8c050));
                scr.pset(sx + dx, sy - 3 + dy - pop, rgb(0xfff0a0));
            }
        }
        &Drop::Food(i) => {
            let name = ["food_apple", "food_bread", "food_roast"][i];
            match art.item(name) {
                Some(s) => scr.blit(s, sx, sy + 1 - pop, Fx::default()),
                None => draw_food_fallback(scr, i, sx, sy - 4 - pop),
            }
        }
        &Drop::Rune(i) => {
            let tint = [rgb(0x6090c0), rgb(0xf0f8ff), rgb(0x80e0ff)][i];
            let y = sy - 10 - pop + bob;
            scr.glow(sx, y, 22.0, rgb(0x80c0ff), 0.8);
            match art.item("seal") {
                Some(s) => scr.blit(s, sx, y + 7, Fx { tint, tint_a: 0.55, ..Fx::default() }),
                None => {
                    scr.disc(sx, y, 5, BLACK);
                    scr.disc(sx, y, 4, tint);
                }
            }
        }
        &Drop::Seal(i) => {
            let tint = [rgb(0xe0d8c0), rgb(0x80d040), rgb(0xb070ff)][i];
            let y = sy - 10 - pop + bob;
            scr.glow(sx, y, 20.0, tint, 0.8);
            match art.item("seal") {
                Some(s) => scr.blit(s, sx, y + 7, Fx { tint, tint_a: 0.35, ..Fx::default() }),
                None => {
                    scr.disc(sx, y, 5, BLACK);
                    scr.disc(sx, y, 4, tint);
                    scr.disc(sx, y, 1, WHITE);
                }
            }
        }
    }
}


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
            // Ink in your eyes, or the Angler's dark (tides.rs), shrinks the light.
            let k = self.light_scale();
            scr.build_base_light(ox as i32, oy as i32 - 14, radius * k, ambient * k);
            self.light_ready = true;
        }
        let sh = if self.shake > 0.0 { ((self.tick as f32 * 1.7).sin() * self.shake * 4.0) as i32 } else { 0 };
        scr.shake = (sh, (sh as f32 * 0.5) as i32);
        scr.clear(BLACK);
        // The Skyreach: the islands float over a sunset sea of cloud (the open sky between them).
        if self.theme == crate::world::Theme::Heavens {
            self.draw_cloud_sea(scr);
        }
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
                let ground = self.d.ground_at(tx, ty);
                let f = self.art.floor(theme, ground, v);
                scr.blit(f, sx, sy, Fx::default());
                // The tide over the flats: dark water rising, rippling with the swell.
                if ground == 1 && self.tide > 0.0 && crate::tides::tidal(self.level) {
                    let ripple = ((self.tick as f32 * 0.08 + tx as f32 * 0.7 + ty as f32 * 0.4).sin() * 0.5 + 0.5) * 0.12;
                    let a = (self.tide * 0.72 + ripple * self.tide).min(0.85);
                    for row in -8i32..=8 {
                        let half = 16 - row.abs() * 2;
                        scr.blend(sx - half + scr.shake.0, sy + row + scr.shake.1, half * 2, 1, rgb(0x1e6a84), a);
                    }
                    // Glints moving across the water.
                    if self.tide > 0.5 {
                        let k = (self.tick / 6) as i32 + tx * 3 + ty * 5;
                        let ox = (k % 11) - 5;
                        let oy = (k / 11 % 5) - 2;
                        scr.fill(sx + ox - 2 + scr.shake.0, sy + oy + scr.shake.1, 4, 1, rgb(0x90e8f8));
                        if v % 3 == 0 {
                            scr.fill(sx - ox - 1 + scr.shake.0, sy - oy + 3 + scr.shake.1, 3, 1, rgb(0x60c0d8));
                        }
                    }
                }
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
        self.draw_clockwork_floor(scr, &to_scr);
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
        // The White Waste's thin ice: cracks where it's stressed, and black water where it broke.
        if self.level == crate::features::LAKE && !self.feats.ice.lake.is_empty() {
            for &(hx, hy) in &self.feats.ice.holes {
                let (sx, sy) = to_scr(hx as f32 + 0.5, hy as f32 + 0.5);
                blend_ellipse(scr, sx, sy, 16, 8, rgb(0x0a1a28), 0.85);
                blend_ellipse(scr, sx, sy, 10, 4, rgb(0x305878), 0.4);
            }
            for ty in y0..=y1 {
                for tx in x0..=x1 {
                    let s = self.feats.ice.stress_at(tx, ty);
                    if s > 0.3 {
                        let (sx, sy) = to_scr(tx as f32 + 0.5, ty as f32 + 0.5);
                        let c = mix(rgb(0x6890b0), rgb(0x0a1828), s.min(1.0));
                        let k = (tx * 7 + ty * 13) & 3;
                        for j in -7..=7 {
                            scr.pset(sx + j, sy + (j * (k - 1)) / 4, c);
                            scr.pset(sx + j, sy + 1 + (j * (k - 1)) / 4, c);
                        }
                        if s > 0.6 {
                            for j in -4..=4 {
                                scr.pset(sx + j / 2 + 2, sy + j, c);
                            }
                        }
                    }
                }
            }
        }
        // Act 6: the storm cells, the fallen star's crater, the sanctum's light.
        if self.level == crate::isles::STORMFIELDS && self.feats.relic < 3 {
            for c in self.feats.cells.iter() {
                let (sx, sy) = to_scr(c.0, c.1);
                // A dark cloud hanging over its patch of ground, raining.
                blend_ellipse(scr, sx, sy, 46, 18, rgb(0x101828), 0.3);
                ring(scr, sx, sy, 46, 18, rgb(0x6080c0));

            }
        }
        if let Some((x, y)) = self.star_here() {
            let (sx, sy) = to_scr(x, y);
            blend_ellipse(scr, sx, sy, 40, 16, rgb(0x100818), 0.6);
            ring(scr, sx, sy, 40, 16, rgb(0x5a4060));
            if !self.feats.star_done {
                scr.glow(sx, sy, 34.0, rgb(0xa080ff), 0.3);
            }
        }
        if self.level == crate::isles::HALO && self.feats.sanctum >= 1 {
            let (x, y) = self.feats.sanctum_spot;
            let (sx, sy) = to_scr(x, y);
            let t = (self.tick as f32 * 0.05).sin();
            scr.glow(sx, sy - 20, 40.0, rgb(0xfff0c0), if self.feats.sanctum == 1 { 0.3 + 0.08 * t } else { 0.15 });
            ring(scr, sx, sy, 24, 9, rgb(0xffe8a0));
        }
        // The siren choir's whirlpool, and Captain Salt's X.
        if self.level == crate::reef::CHOIR && !self.feats.choir_done && self.feats.whirl != (0.0, 0.0) {
            let (sx, sy) = to_scr(self.feats.whirl.0, self.feats.whirl.1);
            for k in 0..3 {
                let t = (self.tick as f32) * 0.08 + k as f32 * 2.1;
                ring(scr, sx + (t.cos() * 2.0) as i32, sy, 20 - k * 6, 8 - k * 2, mix(rgb(0x206070), rgb(0x80e0f0), 0.3 + 0.2 * k as f32));
            }
            blend_ellipse(scr, sx, sy, 8, 3, rgb(0x081820), 0.8);
        }
        if let Some((x, y)) = self.cache_x() {
            let (sx, sy) = to_scr(x, y);
            for k in -5..=5 {
                scr.fill(sx + k * 2, sy + k, 2, 1, rgb(0xc03020));
                scr.fill(sx + k * 2, sy - k, 2, 1, rgb(0xc03020));
            }
        }
        // The yeti cub's den.
        if self.level == crate::features::CUB && self.feats.cub < 2 && self.feats.den != (0.0, 0.0) {
            let (sx, sy) = to_scr(self.feats.den.0, self.feats.den.1);
            ring(scr, sx, sy, 24, 10, rgb(0xa0c8e0));
        }
        if let Some((kind, (x, y), open)) = self.errand_marker() {
            let (sx, sy) = to_scr(x, y);
            let pulse = 0.5 + 0.5 * ((self.tick as f32) * 0.08).sin();
            if kind == crate::errands::ErrandKind::Siege {
                blend_ellipse(scr, sx, sy, 22, 9, rgb(0x101820), 0.8);
                let c = if open { mix(rgb(0x4080c0), rgb(0xa0e0ff), pulse) } else { rgb(0x506070) };
                ring(scr, sx, sy, 20, 8, c);
                scr.fill(sx - 3, sy - 22, 7, 22, rgb(0x707880));
                scr.fill(sx - 2, sy - 20, 5, 2, c);
            } else {
                let c = mix(rgb(0xa02010), rgb(0xff4020), pulse);
                for k in -5..=5 {
                    scr.fill(sx + k * 2, sy + k, 2, 1, c);
                    scr.fill(sx + k * 2, sy - k, 2, 1, c);
                }
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
            Clock(usize),
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
        for (i, c) in self.clocks.iter().enumerate() {
            if in_view(c.x, c.y) {
                list.push((c.x + c.y, D::Clock(i)));
            }
        }
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
                    // A shrine glows in its blessing's colour until it's used; then it goes dark.
                    let shrine = match pr.kind {
                        PropKind::Shrine(_) => self.shrines.iter().find(|k| (k.x, k.y) == (pr.foot.0, pr.foot.1)),
                        _ => None,
                    };
                    let dark = shrine.map_or(false, |k| k.used);
                    let fx = if dark { Fx { dither: front && over, tint: rgb(0x282830), tint_a: 0.55, ..Fx::default() } } else { Fx { dither: front && over, ..Fx::default() } };
                    scr.blit(s, sx, sy + 8, fx);
                    if let Some(k) = shrine.filter(|k| !k.used) {
                        let pulse = 0.45 + 0.2 * ((self.tick as f32) * 0.07 + pr.x).sin();
                        scr.glow(sx, sy - s.ay / 2 + 4, 18.0, rgb(k.kind.col()), pulse);
                    }
                    if pr.kind == PropKind::Entrance(SANCTUM) && self.quest.stage < 2 {
                        // The ash barrier.
                        let k = ((self.tick as f32) * 0.08).sin() * 0.15 + 0.45;
                        scr.blend(sx - 26, sy - 40, 52, 44, rgb(0x401010), k * 0.5);
                    }
                }
                D::Mob(i) => self.draw_mob(scr, i, to_scr(self.mobs[i].x, self.mobs[i].y)),
                D::Npc(i) => self.draw_npc(scr, i, to_scr(self.npcs[i].x, self.npcs[i].y)),
                D::Player => self.draw_player(scr, (psx, psy)),
                D::Clock(i) => {
                    let (sx, sy) = to_scr(self.clocks[i].x, self.clocks[i].y);
                    self.draw_clock(scr, i, sx, sy);
                }
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
        let vampire = self.p.skills.class == crate::skills::Class::Vampire;
        let inventor = self.p.skills.class == crate::skills::Class::Inventor;
        for b in &self.balls {
            let (sx, sy) = to_scr(b.x, b.y);
            let fl = ((self.tick as f32) * 0.9).sin() * 1.5;
            // The vampire's bolts are blood, the inventor's aether; the sorceress's fire.
            let (outer, inner, core) = if vampire {
                (rgb(0xb00828), rgb(0xff4060), rgb(0xffd0d8))
            } else if inventor {
                (rgb(0x109080), rgb(0x60f0e0), rgb(0xe0fffa))
            } else {
                (rgb(0xff5010), rgb(0xffd060), rgb(0xfff4c0))
            };
            if inventor {
                // A short streak of aether.
                let l = (b.vx * b.vx + b.vy * b.vy).sqrt().max(0.01);
                let (ex, ey) = iso::to_screen(b.vx / l * 0.35, b.vy / l * 0.35);
                let (ax, ay) = (sx + scr.shake.0, sy - 20 + scr.shake.1);
                scr.glow(ax, ay, 9.0, outer, 0.9);
                line(scr, ax - ex as i32, ay - ey as i32, ax + ex as i32, ay + ey as i32, inner);
                scr.pset(ax + ex as i32, ay + ey as i32, core);
                continue;
            }
            if b.ember {
                scr.glow(sx, sy - 22, 12.0 + fl * 0.5, outer, 0.9);
                scr.disc(sx + scr.shake.0, sy - 22 + scr.shake.1, 1, core);
                continue;
            }
            scr.glow(sx, sy - 22, 24.0 + fl, outer, 0.9);
            scr.glow(sx, sy - 22, 11.0, inner, 1.0);
            scr.disc(sx + scr.shake.0, sy - 22 + scr.shake.1, 3, core);
        }
        // Blood Moon fields: a red moon over a bleeding circle.
        for f in &self.fields {
            let (sx, sy) = to_scr(f.x, f.y);
            let k = (f.t / 0.5).min(1.0) * ((crate::vampire::MOON_TIME - f.t) / 0.6).clamp(0.0, 1.0);
            let rx = (crate::vampire::MOON_RADIUS * iso::TW * 0.5) as i32;
            blend_ellipse(scr, sx, sy, rx, rx / 2, rgb(0x600010), 0.35 * k);
            ring(scr, sx, sy, rx, rx / 2, rgb(0xc01030));
            scr.glow(sx, sy - 90, 30.0, rgb(0xc01020), 0.9 * k);
            scr.disc(sx + scr.shake.0, sy - 90 + scr.shake.1, 9, mix(rgb(0x300008), rgb(0xd02030), k));
            for d in 0..5 {
                let ph = ((self.tick as f32) * 0.04 + d as f32 * 0.37).fract();
                let dx = ((d as f32 * 2.3).sin() * rx as f32 * 0.6) as i32;
                scr.fill(sx + dx, sy - 70 + (ph * 66.0) as i32, 1, 3, rgb(0xd02030));
            }
        }
        // The inventor's gadgets.
        for b in &self.bombs {
            let k = (b.t / crate::inventor::BOMB_FLIGHT).min(1.0);
            let (x, y) = (b.x0 + (b.x - b.x0) * k, b.y0 + (b.y - b.y0) * k);
            let (sx, sy) = to_scr(x, y);
            let arc = if k < 1.0 { ((k * std::f32::consts::PI).sin() * 30.0) as i32 } else { 0 };
            let (bx, by) = (sx + scr.shake.0, sy - 6 - arc + scr.shake.1);
            match self.art.item("gadget_bomb") {
                Some(s) => scr.blit(s, bx, by + 6, Fx::default()),
                None => {
                    scr.disc(bx, by, 4, rgb(0x2a2018));
                    scr.disc(bx, by, 3, rgb(0xb08840));
                }
            }
            if k >= 1.0 && (self.tick / 5) % 2 == 0 {
                scr.glow(bx, by - 5, 7.0, rgb(0xff6020), 0.9);
                let rx = (crate::inventor::BOMB_RADIUS * iso::TW * 0.5) as i32;
                ring(scr, sx, sy, rx, rx / 2, rgb(0xd09040));
            }
        }
        for a in &self.arcs {
            let fade = 1.0 - a.t / 0.25;
            for w in a.pts.windows(2) {
                let (x0, y0) = to_scr(w[0].0, w[0].1);
                let (x1, y1) = to_scr(w[1].0, w[1].1);
                let (x0, y0, x1, y1) = (x0, y0 - 20, x1, y1 - 20);
                // Jagged: a few kinked segments.
                let mut prev = (x0, y0);
                for k in 1..=5 {
                    let t = k as f32 / 5.0;
                    let j = if k == 5 { 0 } else { (hash3(self.tick as i32 / 2, k, x0) % 9) as i32 - 4 };
                    let p = (x0 + ((x1 - x0) as f32 * t) as i32 + j, y0 + ((y1 - y0) as f32 * t) as i32 - j);
                    line(scr, prev.0 + scr.shake.0, prev.1 + scr.shake.1, p.0 + scr.shake.0, p.1 + scr.shake.1, mix(rgb(0x40c0b0), rgb(0xe0fffa), fade));
                    scr.glow(p.0, p.1, 6.0, rgb(0x20a0a0), 0.6 * fade);
                    prev = p;
                }
            }
        }
        for tu in &self.turrets {
            let (sx, sy) = to_scr(tu.x, tu.y);
            match self.art.item("gadget_turret") {
                Some(s) => scr.blit(s, sx + scr.shake.0, sy + 2 + scr.shake.1, Fx::default()),
                None => {
                    scr.fill(sx - 5 + scr.shake.0, sy - 14 + scr.shake.1, 10, 10, rgb(0x8a6a30));
                    scr.fill(sx - 1 + scr.shake.0, sy - 20 + scr.shake.1, 2, 6, rgb(0x504030));
                }
            }
            scr.glow(sx, sy - 16, 8.0, rgb(0x40e0d0), 0.6 + 0.3 * ((self.tick as f32) * 0.2).sin());
        }
        for sp in &self.spiders {
            let (sx, sy) = to_scr(sp.x, sp.y);
            let bob = if sp.moving { ((self.tick as f32) * 0.8).sin().abs() as i32 * 2 } else { 0 };
            match self.art.item("gadget_spider") {
                Some(s) => scr.blit(s, sx + scr.shake.0, sy + 2 - bob + scr.shake.1, Fx::default()),
                None => {
                    scr.disc(sx + scr.shake.0, sy - 5 - bob + scr.shake.1, 4, rgb(0x8a6a30));
                    for k in [-6, -3, 3, 6] {
                        scr.fill(sx + k + scr.shake.0, sy - 3 + scr.shake.1, 1, 3, rgb(0x504030));
                    }
                }
            }
            scr.pset(sx - 1 + scr.shake.0, sy - 7 - bob + scr.shake.1, rgb(0x60f0e0));
            scr.pset(sx + 1 + scr.shake.0, sy - 7 - bob + scr.shake.1, rgb(0x60f0e0));
        }
        for a in &self.airships {
            let (sx, sy) = to_scr(a.x, a.y);
            let (x, y) = (sx + scr.shake.0, sy - 110 + scr.shake.1);
            blend_ellipse(scr, sx, sy, 22, 8, BLACK, 0.35);
            match self.art.item("gadget_airship") {
                Some(s) => scr.blit(s, x, y + s.h / 2, Fx::default()),
                None => {
                    blend_ellipse(scr, x, y - 8, 24, 9, rgb(0x8a6a40), 1.0);
                    scr.fill(x - 8, y, 16, 5, rgb(0x504030));
                }
            }
        }
        if self.p.tesla_t > 0.0 {
            let (psx, psy) = to_scr(self.p.x, self.p.y);
            let rx = (crate::inventor::TESLA_RADIUS * iso::TW * 0.5) as i32;
            ring(scr, psx, psy, rx, rx / 2, rgb(0x40c0b0));
            for k in 0..6 {
                let a = k as f32 * 1.05 + self.tick as f32 * 0.15;
                let (dx, dy) = ((a.cos() * rx as f32) as i32, (a.sin() * rx as f32 * 0.5) as i32);
                scr.glow(psx + dx, psy + dy - 4, 5.0, rgb(0x60f0e0), 0.8);
            }
        }
        // Bats.
        for r in &self.ravens {
            let (sx, sy) = to_scr(r.x, r.y);
            let flap = if (self.tick / 3) % 2 == 0 { 3 } else { -2 };
            let (x, y) = (sx + scr.shake.0, sy - 30 + scr.shake.1);
            scr.fill(x - 2, y - 1, 5, 3, rgb(0x0c0c14));
            for k in 1..=6 {
                scr.pset(x - 2 - k, y - flap * k / 6, rgb(0x181828));
                scr.pset(x + 2 + k, y - flap * k / 6, rgb(0x181828));
            }
            scr.pset(x - 1, y - 1, rgb(0x80e0ff));
            scr.glow(x, y, 6.0, rgb(0x60c0ff), 0.4);
        }
        // The druid: spore clouds, mushrooms about to burst, thorn vines.
        for c in &self.clouds {
            let (cx, cy) = to_scr(c.x, c.y);
            let rx = (c.r * iso::TW * 0.5) as i32;
            let fade = (c.t / 1.0).min(1.0);
            blend_ellipse(scr, cx + scr.shake.0, cy - 4 + scr.shake.1, rx, rx / 2, rgb(0x5a8a20), 0.28 * fade);
            for k in 0..8 {
                let h = ((k as u32).wrapping_mul(2654435761) >> 8) as f32 / 16_777_216.0;
                let a = h * std::f32::consts::TAU + self.tick as f32 * 0.02;
                let (x, y) = (cx + (a.cos() * rx as f32 * 0.7) as i32, cy - 6 - ((self.tick as f32 * 0.3 + h * 40.0) % 18.0) as i32 + (a.sin() * rx as f32 * 0.3) as i32);
                scr.glow(x, y, 3.0, rgb(0xa0f050), 0.4 * fade);
            }
        }
        for f in &self.fungi {
            let (sx, sy) = to_scr(f.x, f.y);
            let grow = (f.t / 0.5).min(1.0);
            let pulse = if f.t > 1.0 { ((self.tick as f32) * 0.6).sin().abs() } else { 0.0 };
            let r = (2.0 + 3.0 * grow + pulse) as i32;
            let (x, y) = (sx + scr.shake.0, sy + scr.shake.1);
            scr.fill(x - 1, y - r, 2, r, rgb(0xd8d0b0));
            blend_ellipse(scr, x, y - r, r + 1, (r + 1) / 2 + 1, rgb(0x7a3a80), 0.95);
            scr.pset(x - 1, y - r - 1, rgb(0xe0f080));
            scr.pset(x + 1, y - r, rgb(0xe0f080));
            scr.glow(x, y - r, 5.0 + pulse * 4.0, rgb(0x90e040), 0.4 + 0.3 * pulse);
        }
        for v in &self.vines {
            let n = 24;
            for k in 0..n {
                let t = k as f32 / n as f32;
                let (wx, wy) = (v.x0 + (v.x1 - v.x0) * t, v.y0 + (v.y1 - v.y0) * t);
                let (x, y) = to_scr(wx, wy);
                let wave = ((t * 18.0 + self.tick as f32 * 0.2).sin() * 2.0) as i32;
                let (x, y) = (x + scr.shake.0, y - 3 + wave + scr.shake.1);
                scr.fill(x - 1, y - 1, 3, 2, rgb(0x1a1a10));
                if k % 3 == 0 {
                    // Thorns.
                    scr.pset(x, y - 3, rgb(0x2a2a18));
                    scr.pset(x + 1, y - 4, rgb(0x2a2a18));
                }
                if k % 5 == 0 {
                    scr.pset(x, y, rgb(0x70b030));
                }
            }
        }
        // The reaper: souls drifting home, the lantern's flame, chains and hourglass sand.
        for s in &self.souls {
            let (sx, sy) = to_scr(s.x, s.y);
            let (x, y) = (sx + scr.shake.0, sy - 24 + scr.shake.1 + ((s.t * 9.0).sin() * 3.0) as i32);
            scr.glow(x, y, 9.0, rgb(0x80c8ff), 0.8);
            scr.disc(x, y, 2, rgb(0xe0f4ff));
            scr.pset(x, y + 3, rgb(0x80c8ff));
        }
        for f in &self.lanterns {
            let (sx, sy) = to_scr(f.x, f.y);
            let (x, y) = (sx + scr.shake.0, sy - 22 + scr.shake.1);
            let fl = ((self.tick as f32) * 0.7).sin();
            scr.glow(x, y, 12.0 + fl, rgb(0x60b0ff), 0.9);
            scr.disc(x, y, 2, rgb(0xd8f0ff));
            scr.pset(x, y - 3, rgb(0xa0d8ff));
        }
        // The inquisitor's chain: dark iron links, glinting, from her hand to the censer.
        for c in &self.links {
            let (x0, y0) = to_scr(c.x0, c.y0);
            let (x1, y1) = to_scr(c.x1, c.y1);
            let fade = 1.0 - (c.t / c.max).min(1.0);
            let (sx, sy) = scr.shake;
            let hand = (x0 as f32, (y0 - 22) as f32);
            let end = (x1 as f32, (y1 - 14) as f32);
            let (dx, dy) = (end.0 - hand.0, end.1 - hand.1);
            let len = (dx * dx + dy * dy).sqrt().max(1.0);
            let (nx, ny) = (-dy / len, dx / len);
            // The chain at lash progress p (0..1): the points from her hand to the censer.
            let chain = |p: f32| -> Vec<(f32, f32)> {
                let n = (len * p / 3.0) as i32 + 2;
                (0..=n)
                    .map(|k| {
                        let u = k as f32 / n as f32; // along the unrolled part
                        let a = u * p;
                        // A curl that travels out and straightens as it goes; an overhead lash arcs high and comes down.
                        let curl = (u * std::f32::consts::PI).sin() * (1.0 - p) * 16.0 * c.side + (u * 9.0 - p * 12.0).sin() * (1.0 - p) * 3.0;
                        let arc = (u * std::f32::consts::PI).sin() * (1.0 - p) * 34.0 * c.lift;
                        (hand.0 + dx * a + nx * curl, hand.1 + dy * a + ny * curl - arc)
                    })
                    .collect()
            };
            // Heavy links: a dark outline, then iron and darkened gold in turn, catching the light.
            let link = |scr: &mut Screen, pts: &[(f32, f32)], dim: f32| {
                if dim > 0.0 {
                    for &(x, y) in pts {
                        scr.blend(x as i32 - 1 + sx, y as i32 - 1 + sy, 3, 3, rgb(0xd8b060), dim);
                    }
                    return;
                }
                for &(x, y) in pts {
                    scr.fill(x as i32 - 2 + sx, y as i32 - 2 + sy, 4, 4, rgb(0x120e0a));
                }
                for (k, &(x, y)) in pts.iter().enumerate() {
                    let (col, hi) = if k % 2 == 0 { (rgb(0x6a625a), rgb(0xa8a098)) } else { (rgb(0xa88440), rgb(0xf0d080)) };
                    scr.fill(x as i32 - 1 + sx, y as i32 - 1 + sy, 2, 2, col);
                    scr.pset(x as i32 - 1 + sx, y as i32 - 1 + sy, hi);
                }
            };
            let censer = |scr: &mut Screen, (x, y): (f32, f32), glow: f32| {
                let (ex, ey) = (x as i32 + sx, y as i32 + sy);
                scr.glow(ex, ey, 10.0, rgb(0xffd060), glow);
                scr.disc(ex, ey, 3, rgb(0x8a6a28));
                scr.pset(ex, ey, rgb(0xfff6d0));
            };
            if !c.whip {
                let pts = chain(1.0);
                link(scr, &pts, 0.0);
                censer(scr, *pts.last().unwrap(), 0.6 * fade + 0.25);
                continue;
            }
            if c.t < c.wind {
                // The wind-up: the chain swung back over her shoulder.
                let k = c.t / c.wind.max(0.001);
                let (bx, by) = (-dx / len, -dy / len);
                let pts: Vec<(f32, f32)> = (0..=8)
                    .map(|i| {
                        let u = i as f32 / 8.0;
                        let r = u * 20.0 * k;
                        (hand.0 + bx * r + nx * c.side * u * 6.0, hand.1 + by * r - (u * std::f32::consts::PI * 0.8).sin() * 18.0 * k - 10.0 * c.lift * u * k)
                    })
                    .collect();
                link(scr, &pts, 0.0);
                censer(scr, *pts.last().unwrap(), 0.5);
                continue;
            }
            let p = ((c.t - c.wind) / c.lash.max(0.001)).min(1.0);
            // Motion trails: where the chain was a moment ago.
            for (j, back) in [(1, 0.14f32), (2, 0.28)] {
                if p - back > 0.05 && p < 1.0 {
                    let ghost = chain(p - back);
                    link(scr, &ghost, 0.35 / j as f32);
                }
            }
            let pts = chain(p);
            link(scr, &pts, 0.0);
            let tip = *pts.last().unwrap();
            censer(scr, tip, 0.6 * fade + 0.3);
            // The crack: a white-gold burst as the whip snaps straight.
            if p >= 1.0 {
                let after = (c.t - c.wind - c.lash) / (c.max - c.wind - c.lash).max(0.001);
                let k = 1.0 - after.clamp(0.0, 1.0);
                let (ex, ey) = (tip.0 as i32 + sx, tip.1 as i32 + sy);
                let big = 1.0 + c.lift;
                scr.glow(ex, ey, (18.0 * k + 4.0) * big, rgb(0xfff0b0), 0.8 * k);
                for a in 0..10 {
                    let ang = a as f32 / 10.0 * std::f32::consts::TAU;
                    let r = (4.0 + 10.0 * (1.0 - k)) * big;
                    scr.pset(ex + (ang.cos() * r) as i32, ey + (ang.sin() * r * 0.6) as i32, rgb(0xfff6d0));
                }
            }
        }
        // Binding chains: links from the floor to every foe held, under a ring of iron.
        for b in &self.binds {
            let (cx, cy) = to_scr(b.x, b.y);
            scr.glow(cx, cy, 18.0, rgb(0xc08030), 0.35);
            for &i in &b.held {
                if let Some(m) = self.mobs.get(i).filter(|m| m.alive()) {
                    let (mx, my) = to_scr(m.x, m.y);
                    for side in [-6, 6] {
                        let n = 8;
                        for k in 0..n {
                            let t = k as f32 / n as f32;
                            let x = mx + side + ((-side) as f32 * t * 0.6) as i32 + scr.shake.0;
                            let y = my + 4 - (t * 18.0) as i32 + scr.shake.1;
                            scr.fill(x - 1, y - 1, 2, 2, if k % 2 == 0 { rgb(0x4a4038) } else { rgb(0x9a8a70) });
                        }
                    }
                }
            }
        }
        // Brands of Judgment: a burning seal under each branded foe, and a sigil over its head.
        for m in self.mobs.iter().filter(|m| m.brand_t > 0.0 && m.alive()) {
            let (mx, my) = to_scr(m.x, m.y);
            let pulse = 0.5 + 0.5 * ((self.tick as f32) * 0.15 + m.x).sin();
            let a = (m.brand_t * 2.0).min(1.0);
            blend_ellipse(scr, mx, my, 12, 5, rgb(0xffa030), 0.12 * a + 0.08 * pulse * a);
            for k in 0..8 {
                let ang = k as f32 / 8.0 * std::f32::consts::TAU + self.tick as f32 * 0.03;
                scr.pset(mx + (ang.cos() * 11.0) as i32, my + (ang.sin() * 4.5) as i32, rgb(0xffd080));
            }
            let (sx, sy) = (mx + scr.shake.0, my - 52 + scr.shake.1);
            scr.glow(sx, sy, 6.0, rgb(0xffa030), 0.6 * a);
            scr.disc(sx, sy, 3, rgb(0xffb040));
            scr.disc(sx, sy, 1, rgb(0x402010));
        }
        for c in &self.chains_fx {
            let (cx, cy) = to_scr(c.x, c.y);
            for &i in &c.held {
                if let Some(m) = self.mobs.get(i).filter(|m| m.alive()) {
                    let (mx, my) = to_scr(m.x, m.y);
                    // A chain from the floor at the circle's heart to the bound foe, link by link.
                    let n = 10;
                    for k in 0..n {
                        let t = k as f32 / n as f32;
                        let x = cx + ((mx - cx) as f32 * t) as i32 + scr.shake.0;
                        let y = cy + ((my - 12 - cy) as f32 * t) as i32 + scr.shake.1;
                        scr.fill(x - 1, y - 1, 2, 2, if k % 2 == 0 { rgb(0x7a8a9a) } else { rgb(0xb0d0f0) });
                    }
                }
            }
            scr.glow(cx, cy, 16.0, rgb(0x6090c0), 0.4);
        }
        // Scythe sweeps: a crescent that races from one end of the cut to the other and fades.
        for a in &self.sweeps {
            let k = (a.t / crate::reaper::ARC_TIME).min(1.0);
            let fade = 1.0 - k;
            let steps = (a.half * 2.0 * a.reach * 9.0) as i32 + 8;
            let lead = -a.half + 2.0 * a.half * (k * 1.6).min(1.0);
            for i in 0..=steps {
                let ang = -a.half + 2.0 * a.half * i as f32 / steps as f32;
                if ang > lead {
                    break;
                }
                // Brighter near the leading edge of the swing.
                let near = 1.0 - ((lead - ang) / (2.0 * a.half)).min(1.0);
                for (rr, w) in [(a.reach, 1.0f32), (a.reach - 0.25, 0.6), (a.reach - 0.5, 0.3)] {
                    let (x, y) = to_scr(a.x + (a.a0 + ang).cos() * rr, a.y + (a.a0 + ang).sin() * rr);
                    let (x, y) = (x + scr.shake.0, y - 16 + scr.shake.1);
                    let col = if a.blaze { rgb(0xe8f8ff) } else { rgb(0x9ad8ff) };
                    scr.glow(x, y, if a.blaze { 7.0 } else { 5.0 }, rgb(0x60b0ff), 0.5 * fade * w * (0.4 + 0.6 * near));
                    if w >= 1.0 {
                        scr.pset(x, y, col);
                    }
                }
            }
        }
        for g in &self.glasses {
            let steps = 48;
            for i in 0..steps {
                let a = i as f32 / steps as f32 * std::f32::consts::TAU;
                let (x, y) = to_scr(g.x + a.cos() * crate::reaper::GLASS_RADIUS, g.y + a.sin() * crate::reaper::GLASS_RADIUS);
                // The ring fades as the sand runs out.
                scr.glow(x, y - 2, 5.0, rgb(0xd0a050), 0.15 + 0.4 * (g.t / g.max).min(1.0));
            }
            for k in 0..14 {
                let h1 = ((k as u32).wrapping_mul(2654435761) >> 8) as f32 / 16_777_216.0;
                let fall = (self.tick as f32 * 0.02 + h1).fract();
                let a = h1 * std::f32::consts::TAU * 3.0;
                let rr = (h1 * 7.0).fract() * crate::reaper::GLASS_RADIUS;
                let (x, y) = to_scr(g.x + a.cos() * rr, g.y + a.sin() * rr);
                scr.pset(x, y - 30 + (fall * 30.0) as i32, rgb(0xe8c070));
            }
        }
        for a in &self.axes {
            // A spinning axe: a dark iron head on a short haft, turning.
            let (sx, sy) = to_scr(a.x, a.y);
            let (ax, ay) = (sx + scr.shake.0, sy - 22 + scr.shake.1);
            let (c, s) = (a.spin.cos(), a.spin.sin());
            line(scr, ax - (c * 6.0) as i32, ay - (s * 3.0) as i32, ax + (c * 6.0) as i32, ay + (s * 3.0) as i32, rgb(0x6a4a2a));
            scr.disc(ax + (c * 6.0) as i32, ay + (s * 3.0) as i32, 3, rgb(0x404448));
            scr.pset(ax + (c * 7.0) as i32, ay + (s * 3.5) as i32, rgb(0xc8ccd0));
        }
        for j in &self.javelins {
            let (sx, sy) = to_scr(j.x, j.y);
            let (ex, ey) = iso::to_screen(j.ux * 0.7, j.uy * 0.7);
            let (ax, ay) = (sx + scr.shake.0, sy - 22 + scr.shake.1);
            scr.glow(ax, ay, 12.0, rgb(0x60c0ff), 0.8);
            line(scr, ax - ex as i32, ay - ey as i32, ax + ex as i32, ay + ey as i32, rgb(0xc0f0ff));
            scr.pset(ax + ex as i32, ay + ey as i32, rgb(0xffffff));
        }
        for m in &self.mobs {
            if m.marked > 0.0 && m.alive() {
                let (sx, sy) = to_scr(m.x, m.y);
                let pulse = 0.5 + 0.5 * (self.tick as f32 * 0.2).sin();
                let (x, y) = (sx + scr.shake.0, sy - 50 + scr.shake.1);
                scr.glow(x, y, 7.0, rgb(0x60c0ff), 0.5 + 0.3 * pulse);
                // A small rune: a vertical stroke with two twigs.
                scr.fill(x, y - 3, 1, 7, rgb(0xd0f4ff));
                scr.pset(x - 1, y - 2, rgb(0xd0f4ff));
                scr.pset(x + 1, y - 2, rgb(0xd0f4ff));
            }
        }
        for b in &self.bats {
            let (sx, sy) = to_scr(b.x, b.y);
            let flap = if (self.tick / 4 + (b.x * 7.0) as u32) % 2 == 0 { 2 } else { -1 };
            let (x, y) = (sx + scr.shake.0, sy - 26 + scr.shake.1);
            scr.fill(x - 1, y - 1, 3, 3, rgb(0x100810));
            for k in 1..=4 {
                scr.pset(x - 1 - k, y - flap * k / 4, rgb(0x201028));
                scr.pset(x + 1 + k, y - flap * k / 4, rgb(0x201028));
            }
            scr.pset(x - 1, y - 1, rgb(0xff2030));
            scr.pset(x + 1, y - 1, rgb(0xff2030));
        }
        if self.p.embrace_t > 0.0 {
            // Countess's Embrace: great bat wings and a violet aura.
            let (psx, psy) = to_scr(self.p.x, self.p.y);
            let flap = ((self.tick as f32) * 0.35).sin() * 6.0;
            scr.glow(psx, psy - 30, 34.0, rgb(0x6020a0), 0.6);
            for side in [-1i32, 1] {
                for k in 0..30 {
                    let t = k as f32 / 29.0;
                    let x = psx + side * (6 + (t * 30.0) as i32);
                    let top = psy - 40 - (t * 3.14).sin() as i32 * 10 - (flap * t) as i32;
                    let h = (16.0 * (1.0 - t * 0.7)) as i32;
                    scr.fill(x, top, 1, h, rgb(0x180c20));
                    scr.pset(x, top, rgb(0x6a3090));
                }
            }
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
                ShotKind::Necro => {
                    scr.glow(sx, sy - 18, 14.0, rgb(0x30e060), 0.9);
                    scr.disc(sx + scr.shake.0, sy - 18 + scr.shake.1, 2, rgb(0xd0ffd8));
                }
                ShotKind::Bone => {
                    let l = (s.vx * s.vx + s.vy * s.vy).sqrt().max(0.01);
                    let (ex, ey) = iso::to_screen(s.vx / l * 0.6, s.vy / l * 0.6);
                    let (ax, ay) = (sx + scr.shake.0, sy - 20 + scr.shake.1);
                    line(scr, ax - ex as i32, ay - ey as i32, ax + ex as i32, ay + ey as i32, rgb(0xe8e0c8));
                    scr.pset(ax + ex as i32, ay + ey as i32, rgb(0xffffff));
                }
                ShotKind::Steam => {
                    let k = (self.tick as f32 * 0.3 + s.x * 3.0).sin() * 0.5 + 0.5;
                    blend_ellipse(scr, sx + scr.shake.0, sy - 16 + scr.shake.1, 5, 4, rgb(0xe8e8e0), 0.45 + 0.2 * k);
                    scr.disc(sx + scr.shake.0, sy - 16 + scr.shake.1, 1, rgb(0xffffff));
                }
                ShotKind::Gear => {
                    scr.glow(sx, sy - 18, 12.0, rgb(0xffc040), 0.7);
                    let a = self.tick as f32 * 0.5;
                    let (ax, ay) = (sx + scr.shake.0, sy - 18 + scr.shake.1);
                    scr.disc(ax, ay, 3, rgb(0xa07820));
                    for k in 0..4 {
                        let b = a + k as f32 * std::f32::consts::FRAC_PI_2;
                        scr.pset(ax + (b.cos() * 4.0) as i32, ay + (b.sin() * 4.0) as i32, rgb(0xffe080));
                    }
                    scr.pset(ax, ay, rgb(0x302010));
                }
                ShotKind::Spark => {
                    scr.glow(sx, sy - 18, 13.0, rgb(0x40a0ff), 0.9);
                    let j = (self.tick as i32 * 7 + s.x as i32) % 3 - 1;
                    scr.disc(sx + scr.shake.0, sy - 18 + scr.shake.1, 2, rgb(0xd0f0ff));
                    scr.pset(sx + scr.shake.0 + 3, sy - 18 + j + scr.shake.1, rgb(0x80c0ff));
                    scr.pset(sx + scr.shake.0 - 3, sy - 18 - j + scr.shake.1, rgb(0x80c0ff));
                }
                ShotKind::Blood => {
                    scr.glow(sx, sy - 18, 14.0, rgb(0xc01020), 0.9);
                    scr.disc(sx + scr.shake.0, sy - 18 + scr.shake.1, 2, rgb(0xff8090));
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
                ShotKind::Song => {
                    let bob = ((self.tick as f32 * 0.4 + s.x * 5.0).sin() * 2.0) as i32;
                    let (x, y) = (sx + scr.shake.0, sy - 20 + bob + scr.shake.1);
                    scr.glow(x, y, 13.0, rgb(0x40e0d0), 0.8);
                    // A note: a head and a stem.
                    scr.disc(x, y + 2, 2, rgb(0xd0fff8));
                    scr.fill(x + 2, y - 4, 1, 6, rgb(0xd0fff8));
                    scr.pset(x + 3, y - 4, rgb(0xd0fff8));
                }
                ShotKind::Ink => {
                    blend_ellipse(scr, sx + scr.shake.0, sy - 16 + scr.shake.1, 5, 4, rgb(0x100818), 0.85);
                    scr.pset(sx + scr.shake.0 - 1, sy - 17 + scr.shake.1, rgb(0x8060c0));
                }
                ShotKind::Cannon => {
                    scr.disc(sx + scr.shake.0, sy - 20 + scr.shake.1, 4, rgb(0x0c0c10));
                    scr.disc(sx + scr.shake.0, sy - 20 + scr.shake.1, 3, rgb(0x3a3a44));
                    scr.pset(sx - 1 + scr.shake.0, sy - 22 + scr.shake.1, rgb(0x9090a0));
                }
                ShotKind::Tide => {
                    scr.glow(sx, sy - 22, 14.0, rgb(0x30c0e0), 0.9);
                    scr.disc(sx + scr.shake.0, sy - 22 + scr.shake.1, 3, rgb(0x80e8ff));
                    scr.pset(sx + scr.shake.0, sy - 23 + scr.shake.1, rgb(0xffffff));
                }
                ShotKind::Light => {
                    let l = (s.vx * s.vx + s.vy * s.vy).sqrt().max(0.01);
                    let (ex, ey) = iso::to_screen(s.vx / l * 0.5, s.vy / l * 0.5);
                    let (ax, ay) = (sx + scr.shake.0, sy - 22 + scr.shake.1);
                    scr.glow(ax, ay, 14.0, rgb(0xffe8a0), 0.9);
                    line(scr, ax - ex as i32, ay - ey as i32, ax + ex as i32, ay + ey as i32, rgb(0xfff8e0));
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
            let col = if n.frost {
                rgb(0x60c8ff)
            } else if n.blood {
                rgb(0xc01030)
            } else if self.is_druid() {
                // The druid's waves are plague and growth: sickly green.
                rgb(0x80d030)
            } else {
                rgb(0xff6010)
            };
            let k = (n.t / crate::skills::NOVA_TIME).min(1.0);
            let fade = 1.0 - ((n.t - crate::skills::NOVA_TIME * 0.7) / 0.3).clamp(0.0, 1.0);
            let r = n.r * (0.25 + 0.75 * k);
            let steps = (r * 18.0) as i32 + 12;
            for i in 0..steps {
                let a = i as f32 / steps as f32 * std::f32::consts::TAU;
                let (sx, sy) = to_scr(n.x + a.cos() * r, n.y + a.sin() * r);
                scr.glow(sx, sy - 6, 10.0, col, 0.9 * fade);
            }
        }
        for l in &self.lights {
            let (sx, sy) = to_scr(l.x, l.y);
            let k = l.life / l.max;
            let (outer, inner) = if vampire { (rgb(0xa00820), rgb(0xff6070)) } else { (rgb(0xff6010), rgb(0xffe080)) };
            scr.glow(sx, sy - 16, 50.0 * (1.2 - k * 0.5), outer, 1.2 * k);
            scr.glow(sx, sy - 16, 22.0 * (1.3 - k * 0.5), inner, 1.2 * k);
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
                PKind::Spore => {
                    let i = (sy * scr.w + sx) as usize;
                    scr.px[i] = add(scr.px[i], rgb(0x90e040), 0.5 + k);
                }
                PKind::Holy => {
                    // White at the heart, gold as it rises, never the red of ordinary fire.
                    let c = if k > 0.6 { rgb(0xfffbe8) } else if k > 0.3 { rgb(0xffd870) } else { rgb(0xd09a30) };
                    let i = (sy * scr.w + sx) as usize;
                    scr.px[i] = add(scr.px[i], c, 0.6 + k);
                    scr.pset(sx, sy - 1, mix(scr.px[i], c, 0.5 * k));
                    if k > 0.5 {
                        scr.pset(sx + 1, sy, mix(scr.px[i], c, 0.6));
                        scr.pset(sx - 1, sy, mix(scr.px[i], c, 0.4));
                    }
                }
            }
        }
        scr.shake = (0, 0);

        // The Stormfields' storm cells (isles.rs): dark clouds over their patch of ground, raining.
        if self.level == crate::isles::STORMFIELDS && self.feats.relic < 3 {
            for (k, c) in self.feats.cells.iter().enumerate() {
                let (sx, sy) = to_scr(c.0, c.1);
                for r in 0..14 {
                    let rx = sx - 38 + ((r * 29 + k * 11) % 76) as i32;
                    let ry = sy - 50 + ((self.tick as usize * 3 + r * 17) % 48) as i32;
                    scr.fill(rx, ry, 1, 5, rgb(0x8098c0));
                }
                for (ox, oy, rx, ry) in [(0, -62, 40, 14), (-22, -56, 24, 10), (22, -58, 26, 11), (0, -70, 26, 10)] {
                    blend_ellipse(scr, sx + ox, sy + oy, rx, ry, rgb(0x2a3040), 0.7);
                }
                if (self.tick as usize + k * 7) % 23 < 3 {
                    let ox = ((self.tick as usize * 13 + k * 31) % 60) as i32 - 30;
                    for st in 0..6 {
                        scr.fill(sx + ox + (st % 2) * 3 - 1, sy - 56 + st * 9, 2, 10, rgb(0xd0e8ff));
                    }
                    scr.glow(sx + ox, sy, 20.0, rgb(0xa0c8ff), 0.5);
                }
            }
        }
        // Weather: the heavens' ash drifting upward, and the wind's streaks (warning, then the gust).
        if self.theme.sky() {
            let (cx, cy) = iso::to_screen(self.p.x, self.p.y);
            let t = self.tick as f32 / 60.0;
            let hh = view_h - crate::game::HUD_H;
            for i in 0..34 {
                let h1 = ((i as u32).wrapping_mul(2654435761) >> 8) as f32 / 16_777_216.0;
                let h2 = ((i as u32).wrapping_mul(40503).wrapping_add(71) % 1000) as f32 / 1000.0;
                let x = (h1 * scr.w as f32 * 3.0 - cx * 0.95 + (t * 0.6 + h2 * 9.0).sin() * 5.0).rem_euclid(scr.w as f32) as i32;
                let y = (h2 * hh as f32 * 3.0 - cy * 0.95 - t * (8.0 + h1 * 8.0)).rem_euclid(hh as f32) as i32;
                scr.pset(x, y, if i % 4 == 0 { rgb(0xffd8a0) } else { rgb(0x9a8a80) });
            }
            if self.wind_warn > 0.0 || self.wind_gust > 0.0 {
                let (wx, wy) = self.wind_dir;
                let (sx, sy) = iso::to_screen(wx, wy);
                let l = (sx * sx + sy * sy).sqrt().max(0.01);
                let (ux, uy) = (sx / l, sy / l);
                let strength = if self.wind_gust > 0.0 { 1.0 } else { 0.45 };
                let n = if self.wind_gust > 0.0 { 26 } else { 12 };
                for i in 0..n {
                    let h1 = ((i as u32).wrapping_mul(2246822519) >> 8) as f32 / 16_777_216.0;
                    let h2 = ((i as u32).wrapping_mul(97).wrapping_add(13) % 1000) as f32 / 1000.0;
                    let run = (t * 260.0 * strength + h1 * 900.0) % 900.0;
                    let x0 = (h2 * scr.w as f32 - ux * 300.0 + ux * run).rem_euclid(scr.w as f32);
                    let y0 = (h1 * hh as f32 - uy * 300.0 + uy * run).rem_euclid(hh as f32);
                    let len = 10.0 + 14.0 * strength;
                    for k in 0..len as i32 {
                        let (x, y) = ((x0 + ux * k as f32) as i32, (y0 + uy * k as f32) as i32);
                        scr.blend(x, y, 1, 1, rgb(0xf0f0ff), 0.35 * strength);
                    }
                }
            }
        }
        // Weather: the deep's rising bubbles and drifting sea snow, and the lure in the dark.
        if self.theme.drowned() {
            let (cx, cy) = iso::to_screen(self.p.x, self.p.y);
            let t = self.tick as f32 / 60.0;
            let hh = view_h - crate::game::HUD_H;
            for i in 0..40 {
                // Sea snow: pale motes drifting down and sideways.
                let h1 = ((i as u32).wrapping_mul(2654435761) >> 8) as f32 / 16_777_216.0;
                let h2 = ((i as u32).wrapping_mul(40503).wrapping_add(331) % 1000) as f32 / 1000.0;
                let x = (h1 * scr.w as f32 * 3.0 - cx * 0.95 + (t * 0.7 + h2 * 9.0).sin() * 6.0).rem_euclid(scr.w as f32) as i32;
                let y = (h2 * hh as f32 * 3.0 - cy * 0.95 + t * (6.0 + h1 * 6.0)).rem_euclid(hh as f32) as i32;
                scr.pset(x, y, if i % 3 == 0 { rgb(0xa8d8e0) } else { rgb(0x5a8a98) });
            }
            for i in 0..18 {
                // Bubbles rising and wobbling.
                let h1 = ((i as u32).wrapping_mul(2246822519) >> 8) as f32 / 16_777_216.0;
                let x = (h1 * scr.w as f32 * 3.0 - cx + (t * 3.0 + h1 * 20.0).sin() * 3.0).rem_euclid(scr.w as f32) as i32;
                let y = ((h1 * 13.0).fract() * hh as f32 * 3.0 - cy - t * (24.0 + h1 * 20.0)).rem_euclid(hh as f32) as i32;
                let r = 1 + (i % 3) as i32;
                scr.blend(x - r, y - r, r * 2, r * 2, rgb(0x60c0d0), 0.25);
                scr.pset(x - r / 2, y - r / 2, rgb(0xd0f8ff));
            }
        }
        // Solanthos always smoulders: in his dark, his cracks still glow.
        for m in self.mobs.iter().filter(|m| m.alive() && m.kind == crate::mobs::Kind::Solanthos) {
            let (sx, sy) = to_scr(m.x, m.y);
            let pulse = 0.6 + 0.4 * (self.tick as f32 * 0.09).sin();
            scr.glow(sx + scr.shake.0, sy - 40 + scr.shake.1, 46.0, rgb(0xff7020), 0.55 * pulse);
            scr.glow(sx + scr.shake.0, sy - 80 + scr.shake.1, 20.0, rgb(0xffd060), 0.7 * pulse);
        }
        // In the Angler's dark, only lures shine.
        if self.dark_t > 0.0 || self.theme == crate::world::Theme::Trench {
            for m in self.mobs.iter().filter(|m| m.alive() && matches!(m.kind, crate::mobs::Kind::Angler | crate::mobs::Kind::Anglerlurk)) {
                let (sx, sy) = to_scr(m.x, m.y);
                // The lure: over the matriarch's head; out in front of the lurker's face (it faces right or left).
                let (up, ahead) = if m.kind == crate::mobs::Kind::Angler { (70, 0) } else { (34, if m.dir >= 4 { -16 } else { 16 }) };
                let pulse = 0.6 + 0.4 * (self.tick as f32 * 0.12 + m.x).sin();
                scr.glow(sx + ahead + scr.shake.0, sy - up + scr.shake.1, 18.0, rgb(0x60f0e0), 0.9 * pulse);
                scr.disc(sx + ahead + scr.shake.0, sy - up + scr.shake.1, 2, rgb(0xe0fffa));
            }
        }
        // Weather: Mechanus's drifting steam and rising brass sparks.
        if self.theme.clockwork() {
            let (cx, cy) = iso::to_screen(self.p.x, self.p.y);
            let t = self.tick as f32 / 60.0;
            let open = self.theme.open();
            let hh = view_h - crate::game::HUD_H;
            for i in 0..(if open { 14 } else { 8 }) {
                // Steam puffs drifting up and fading.
                let h1 = ((i as u32).wrapping_mul(2654435761) >> 8) as f32 / 16_777_216.0;
                let life = (t * 0.25 + h1).fract();
                let x = (h1 * scr.w as f32 * 3.0 - cx * 0.9 + life * 20.0).rem_euclid(scr.w as f32) as i32;
                let y = ((h1 * 7.0).fract() * hh as f32 * 2.0 - cy * 0.9 - life * 40.0).rem_euclid(hh as f32) as i32;
                let r = 6 + (life * 14.0) as i32;
                blend_ellipse(scr, x, y, r, r * 2 / 3, rgb(0xc8c4b8), 0.12 * (1.0 - life));
            }
            for i in 0..(if open { 30 } else { 16 }) {
                // Sparks rising from the works.
                let h1 = ((i as u32).wrapping_mul(40503).wrapping_add(977) % 1000) as f32 / 1000.0;
                let h2 = ((i as u32).wrapping_mul(2246822519) >> 8) as f32 / 16_777_216.0;
                let x = (h1 * scr.w as f32 * 3.0 - cx + (t * 2.0 + h2 * 9.0).sin() * 4.0).rem_euclid(scr.w as f32) as i32;
                let y = (h2 * hh as f32 * 3.0 - cy - t * (18.0 + h1 * 20.0)).rem_euclid(hh as f32) as i32;
                let flick = 0.5 + 0.5 * (t * 9.0 + h1 * 30.0).sin();
                scr.pset(x, y, if flick > 0.6 { rgb(0xffe080) } else { rgb(0xff8020) });
                scr.glow(x, y, 2.5, rgb(0xff9030), 0.3 * flick);
            }
        }
        // Weather: the Mistwood's fog banks and drifting wisp motes.
        if self.theme.misty() {
            let (cx, cy) = iso::to_screen(self.p.x, self.p.y);
            let t = self.tick as f32 / 60.0;
            let open = self.theme.open();
            let hh = view_h - crate::game::HUD_H;
            // Slow bands of fog.
            // Wide soft banks (smoothstep falloff), drifting sideways in patches rather than full-width stripes.
            for k in 0..(if open { 5 } else { 3 }) {
                let span = hh as f32 + 160.0;
                let fy = ((k as f32 * 97.0 - cy * 0.5 + t * 2.0).rem_euclid(span) - 80.0) as i32;
                let a = (if open { 0.07 } else { 0.05 }) + 0.03 * ((t * 0.25 + k as f32 * 1.7).sin() * 0.5 + 0.5);
                let x0 = ((k as f32 * 211.0 - cx * 0.6 + t * 5.0).rem_euclid(scr.w as f32 + 300.0) - 150.0) as i32;
                let bw = scr.w as i32 * 2 / 3;
                for row in 0..64 {
                    let u = 1.0 - ((row as f32 - 32.0).abs() / 32.0);
                    let fade = u * u * (3.0 - 2.0 * u);
                    for seg in 0..4 {
                        // Taper the bank's ends horizontally in four steps.
                        let f = [0.35, 1.0, 1.0, 0.35][seg];
                        let sx0 = x0 + seg as i32 * bw / 4;
                        scr.blend(sx0, fy + row, bw / 4, 1, rgb(0x9aa8b8), a * fade * f);
                    }
                }
            }
            // Neon green motes.
            for i in 0..(if open { 40 } else { 18 }) {
                let h1 = ((i as u32).wrapping_mul(2654435761) >> 8) as f32 / 16_777_216.0;
                let h2 = ((i as u32).wrapping_mul(40503).wrapping_add(977) % 1000) as f32 / 1000.0;
                let x = (h1 * scr.w as f32 * 3.0 - cx + (t * 0.5 + h2 * 9.0).sin() * 14.0).rem_euclid(scr.w as f32) as i32;
                let y = (h2 * hh as f32 * 3.0 - cy - t * (4.0 + h1 * 6.0)).rem_euclid(hh as f32) as i32;
                let pulse = 0.5 + 0.5 * (t * 2.0 + h1 * 20.0).sin();
                scr.glow(x, y, 3.0 + pulse * 2.0, rgb(0x40ff70), 0.5 * pulse + 0.2);
            }
        }
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
            if (n.role == Role::Elder && self.quest.elder_has_news())
                || (n.role == Role::Captain && self.quest.captain_has_news())
                || (n.role == Role::Hunter && self.quest.hunter_has_news())
                || (n.role == Role::Tally && self.quest.tally_has_news())
                || (n.role == Role::Ysolde && self.quest.ysolde_has_news())
                || (n.role == Role::Seraphine && self.quest.seraphine_has_news())
            {
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
        // Roads off the map (areas.rs): where they lead, once you're close.
        for p in &self.portals {
            if let PortalKind::Exit(n) = p.kind {
                let d2 = (p.x - self.p.x).powi(2) + (p.y - self.p.y).powi(2);
                if d2 < 64.0 {
                    let (sx, sy) = to_scr(p.x, p.y);
                    let name = Game::waypoint_name(crate::areas::level(self.level.act(), n));
                    let a = (1.0 - d2.sqrt() / 8.0).clamp(0.0, 1.0);
                    scr.text(&format!("TO {name}"), sx, sy + 6, mix(BLACK, rgb(0xe8d090), a), Align::Center, 1);
                    blend_ellipse(scr, sx, sy, 14, 6, rgb(0xffe0a0), 0.18 * a);
                }
            }
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
        self.draw_choose(scr);
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
        for s in &self.shrines {
            let seen = self.explored.get((s.y * self.d.w + s.x) as usize).copied().unwrap_or(false);
            if seen {
                let (sx, sy) = proj(s.x as f32 + 0.5, s.y as f32 + 0.5);
                let col = if s.used { rgb(0x505060) } else { rgb(s.kind.col()) };
                scr.fill(sx - 1, sy - 3, 3, 6, col);
            }
        }
        for m in self.mobs.iter().filter(|m| m.superu > 0 && m.alive()) {
            let seen = self.explored.get((m.y as i32 * self.d.w + m.x as i32) as usize).copied().unwrap_or(false);
            if seen {
                let (sx, sy) = proj(m.x, m.y);
                scr.fill(sx - 2, sy - 2, 5, 5, rgb(0xffb040));
            }
        }
        if let Some((x, y)) = self.cache_x() {
            let (sx, sy) = proj(x, y);
            for k in -3..=3 {
                scr.fill(sx + k, sy + k, 1, 1, rgb(0xff3020));
                scr.fill(sx + k, sy - k, 1, 1, rgb(0xff3020));
            }
        }
        if self.feats.tomb_level == Some(self.level) && self.feats.shade == 1 {
            let (sx, sy) = proj(self.feats.tomb_spot.0, self.feats.tomb_spot.1);
            scr.fill(sx - 2, sy - 3, 5, 6, rgb(0xa0c0ff));
        }
        if self.level == crate::features::CUB && self.feats.cub == 1 {
            let (sx, sy) = proj(self.feats.den.0, self.feats.den.1);
            scr.fill(sx - 2, sy - 2, 5, 5, rgb(0xa0e0ff));
        }
        if let Some((x, y)) = self.errand_x() {
            let (sx, sy) = proj(x, y);
            for k in -3..=3 {
                scr.fill(sx + k, sy + k, 1, 1, rgb(0xff3020));
                scr.fill(sx + k, sy - k, 1, 1, rgb(0xff3020));
            }
        }
        let (sx, sy) = proj(px, py);
        scr.fill(sx - 1, sy - 2, 3, 4, WHITE);
        scr.text("MAP", scr.w - 30, 8, rgb(0xc8b088), Align::Center, 1);
        // The journal (side.rs): this act's lore pages, side quests, super uniques and shrines.
        let lines = self.journal();
        let (jw, lh) = (196, 10);
        let (jx, jy) = (8, 40);
        let jh = lines.len() as i32 * lh + 22;
        scr.blend(jx - 4, jy - 4, jw, jh, rgb(0x0c0a08), 0.72);
        scr.text("JOURNAL", jx, jy, rgb(0xffd080), Align::Left, 1);
        for (k, (t, col)) in lines.iter().enumerate() {
            if !t.is_empty() {
                scr.text(t, jx, jy + 14 + k as i32 * lh, *col, Align::Left, 1);
            }
        }
    }

    /// The class select screen: the hero carousel (shared with the menu).
    fn draw_choose(&mut self, scr: &mut Screen) {
        let Some(sel) = self.choose else { return };
        let (w, h) = (scr.w, self.view_h);
        scr.fill(0, 0, w, h, rgb(0x08060a));
        scr.text("CHOOSE YOUR HERO", w / 2, 10, rgb(0xffd080), Align::Center, 2);
        let rects = crate::menu::draw_carousel(scr, &self.art, w, h, sel, self.tick as f32 / 60.0);
        scr.text("LEFT / RIGHT TO TURN, ENTER / A TO BEGIN (OR CLICK THE MIDDLE CARD)", w / 2, h - 14, rgb(0x8a7a68), Align::Center, 1);
        self.choose_rects = rects;
    }

    /// Draws a character sprite, using another sheet scaled and tinted while its own art is missing.
    fn blit_char(&self, scr: &mut Screen, art_name: &str, anim: CharFrame, pos: (i32, i32), fx: Fx, bob: bool) {
        self.blit_char_x(scr, art_name, anim, pos, fx, bob, 1.0);
    }

    /// `blit_char`, drawn `extra` times bigger (super uniques).
    #[allow(clippy::too_many_arguments)]
    fn blit_char_x(&self, scr: &mut Screen, art_name: &str, anim: CharFrame, (sx, sy): (i32, i32), mut fx: Fx, bob: bool, extra: f32) {
        let (art, scale, tint, tint_a) = self.art.char_art(art_name);
        let scale = scale * extra;
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
        let vampire = self.p.skills.class == crate::skills::Class::Vampire;
        let sheet = match self.p.skills.class {
            crate::skills::Class::Vampire => "vampire",
            crate::skills::Class::Inventor if self.p.suit_t > 0.0 => "steam_suit",
            crate::skills::Class::Inventor => "inventor",
            crate::skills::Class::Valkyrie if self.p.charge.is_some() => "valkyrie_horse",
            crate::skills::Class::Valkyrie => "valkyrie",
            crate::skills::Class::Berserker => "berserker",
            crate::skills::Class::Reaper => "reaper",
            crate::skills::Class::Druid => "druid",
            crate::skills::Class::Inquisitor => "inquisitor_hero",
            crate::skills::Class::Sorceress => "mage",
        };
        let art = self.art.char_art(sheet).0;
        // The melee heroes pick their own attack poses (thrust, sweep, whirl, throw, cast).
        let valkyrie = matches!(
            self.p.skills.class,
            crate::skills::Class::Valkyrie
                | crate::skills::Class::Berserker
                | crate::skills::Class::Reaper
                | crate::skills::Class::Druid
                | crate::skills::Class::Inquisitor
        );
        // Rake is a claw slash (cast pose of exactly 0.3 s).
        let claw = vampire && (self.p.cast_len - 0.3).abs() < 0.001 && art.has("attack");
        let anim = if self.p.cast_t > 0.0 && claw {
            CharFrame::At("attack", self.p.dir, 1.0 - self.p.cast_t / self.p.cast_len)
        } else if self.p.cast_t > 0.0 && valkyrie && art.has(self.p.pose) {
            CharFrame::At(self.p.pose, self.p.dir, 1.0 - self.p.cast_t / self.p.cast_len)
        } else if self.p.charge.is_some() && art.has("walk") {
            CharFrame::Loop("walk", self.p.dir, self.p.anim_t * 2.0 + self.tick as f32 / 30.0)
        } else if self.p.cast_t > 0.0 && self.p.throwing && art.has("throw") {
            CharFrame::At("throw", self.p.dir, 1.0 - self.p.cast_t / self.p.cast_len)
        } else if self.p.cast_t > 0.0 && art.has("cast") {
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
        if self.p.mist > 0.0 {
            // Mist Step: half there.
            fx.dither = true;
            fx.tint = rgb(0xc8c0d8);
            fx.tint_a = 0.5;
        }
        // The berserker in the red mist.
        if self.p.berserk_t > 0.0 && fx.tint_a == 0.0 {
            fx.tint = rgb(0xc02010);
            fx.tint_a = 0.3 + 0.1 * ((self.tick as f32) * 0.3).sin();
        }
        // The valkyrie's wings: raven feathers turning to ice, spread behind her.
        if self.p.skills.class == crate::skills::Class::Valkyrie && self.p.wings_t > 0.0 {
            draw_wings(scr, sx, sy - 30, self.tick, (self.p.wings_t * 3.0).min(1.0));
        }
        // The inquisitor's Iron Halo ignites in Final Judgment: spikes of light behind her head.
        if self.p.skills.class == crate::skills::Class::Inquisitor && self.p.judge_t > 0.0 {
            let (hx, hy) = (sx, sy - 44);
            scr.glow(hx, hy, 22.0, rgb(0xffb040), 0.55 + 0.15 * ((self.tick as f32) * 0.2).sin());
            for k in 0..12 {
                let a = k as f32 / 12.0 * std::f32::consts::TAU + self.tick as f32 * 0.01;
                for d in 9..15 {
                    scr.pset(hx + (a.cos() * d as f32) as i32, hy + (a.sin() * d as f32) as i32, rgb(0xffe0a0));
                }
            }
        }
        self.blit_char(scr, sheet, anim, (sx, sy), fx, self.p.moving);
        // Until her own sprite exists, draw the scythe in her hands (resting, or swinging with the cut).
        if self.p.skills.class == crate::skills::Class::Reaper && !self.art.has_char("reaper") && !matches!(self.state, State::Dead(_)) {
            let swing = self.sweeps.last().filter(|a| a.t < crate::reaper::ARC_TIME);
            let ang = match swing {
                Some(a) => a.a0 - a.half + 2.0 * a.half * (a.t / crate::reaper::ARC_TIME * 1.6).min(1.0),
                None => -1.9,
            };
            draw_scythe(scr, sx, sy - 18, ang, swing.is_some(), self.p.runes, self.tick);
        }
        // The Ledger of the Forgotten floats open beside her.
        if self.p.ledger_t > 0.0 {
            let bob = ((self.tick as f32) * 0.08).sin() * 2.0;
            let (bx, by) = (sx + 14, sy - 46 + bob as i32);
            scr.fill(bx - 7, by - 4, 6, 8, rgb(0xe8dcc0));
            scr.fill(bx + 1, by - 4, 6, 8, rgb(0xe8dcc0));
            scr.fill(bx - 1, by - 5, 2, 10, rgb(0x2a1a10));
            for k in 0..3 {
                scr.fill(bx - 6, by - 2 + k * 2, 4, 1, rgb(0x6a8ab0));
                scr.fill(bx + 2, by - 2 + k * 2, 4, 1, rgb(0x6a8ab0));
            }
        }
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
        if crate::breakables::is_prop(m.kind) {
            return self.draw_prop(scr, i, sx, sy);
        }
        if m.kind == crate::mobs::Kind::StarMetal {
            if !m.alive() {
                return;
            }
            let s = self.art.prop("star_metal");
            let fx = if m.flash > 0.0 { Fx { tint: WHITE, tint_a: 0.6, ..Fx::default() } } else if self.hover == Some(i) { Fx { tint: rgb(0xe0c0ff), tint_a: 0.2, ..Fx::default() } } else { Fx::default() };
            scr.glow(sx, sy - 8, 16.0, rgb(0xa080ff), 0.35);
            scr.blit_scaled(s, sx, sy + 6, 1.3, fx);
            return;
        }
        if m.kind == crate::mobs::Kind::KrakenArm {
            if !m.alive() {
                return;
            }
            // A great coil of tentacle out of the reef, swaying.
            let s = self.art.prop("kraken_arm");
            let sway = ((self.tick as f32) * 0.05 + i as f32).sin();
            let mut fx = Fx::default();
            if m.flash > 0.0 {
                fx.tint = WHITE;
                fx.tint_a = 0.6;
            }
            blend_ellipse(scr, sx, sy, 26, 9, BLACK, 0.45);
            scr.blit_scaled(s, sx + (sway * 3.0) as i32, sy + 8, 1.6 + 0.04 * sway, fx);
            return;
        }
        if m.kind == crate::mobs::Kind::ScrapPile {
            if !m.alive() {
                return;
            }
            let s = self.art.prop("cog_pile");
            let fx = if m.flash > 0.0 { Fx { tint: WHITE, tint_a: 0.6, ..Fx::default() } } else if self.hover == Some(i) { Fx { tint: rgb(0xffe0a0), tint_a: 0.2, ..Fx::default() } } else { Fx::default() };
            scr.blit_scaled(s, sx, sy + 6, 1.4, fx);
            return;
        }
        if m.kind == crate::mobs::Kind::IceBlock {
            if !m.alive() {
                return;
            }
            // Someone frozen in a block of ice: their figure, pale blue, and the ice around them.
            let who = if m.form == crate::features::ICE_SCOUT { "npc_guard" } else { "npc_trader" };
            let mut fx = Fx { tint: rgb(0xa8d8ff), tint_a: 0.55, ..Fx::default() };
            if m.flash > 0.0 {
                fx.tint = WHITE;
                fx.tint_a = 0.8;
            }
            self.blit_char(scr, who, CharFrame::Loop("idle", 2, 0.0), (sx, sy), fx, false);
            scr.blend(sx - 14, sy - 52, 28, 54, rgb(0x9ad0ff), 0.32);
            scr.fill(sx - 14, sy - 52, 28, 1, rgb(0xe0f4ff));
            scr.fill(sx - 14, sy - 52, 1, 54, rgb(0xc8ecff));
            scr.fill(sx + 13, sy - 52, 1, 54, rgb(0x80b8e0));
            return;
        }
        if m.kind == crate::mobs::Kind::Totem {
            if !m.alive() {
                return;
            }
            blend_ellipse(scr, sx, sy, 10, 4, BLACK, 0.45);
            let pulse = 0.4 + 0.25 * ((self.tick as f32) * 0.1).sin();
            scr.glow(sx, sy - 22, 16.0, rgb(0x80ff60), pulse);
            let bone = if m.flash > 0.0 { WHITE } else { rgb(0xd8ccb0) };
            scr.fill(sx - 1, sy - 26, 3, 26, bone);
            scr.fill(sx - 6, sy - 18, 13, 2, bone);
            scr.disc(sx, sy - 29, 5, bone);
            scr.fill(sx - 2, sy - 30, 1, 2, rgb(0x60ff40));
            scr.fill(sx + 2, sy - 30, 1, 2, rgb(0x60ff40));
            scr.fill(sx - 4, sy - 14, 2, 4, rgb(0x8a7a60));
            scr.fill(sx + 3, sy - 12, 2, 4, rgb(0x8a7a60));
            return;
        }
        // Count Vardak's last form is a giant bat.
        let name = match (m.kind, m.form) {
            (crate::mobs::Kind::Vardak, 1) => "boss_vardak_bat",
            // The Clockmaker's great engine.
            (crate::mobs::Kind::Clockmaker, 1) => "boss_clockmaker_engine",
            _ => def(m.kind).art,
        };
        let (art, scale, ..) = self.art.char_art(name);
        let mut fx = Fx::default();
        if m.poison_t > 0.0 && m.frozen <= 0.0 {
            fx.tint = rgb(0x70c030);
            fx.tint_a = 0.3;
        }
        if m.frozen > 0.0 {
            // Frozen solid.
            fx.tint = rgb(0xb8ecff);
            fx.tint_a = 0.65;
        } else if m.frost > 0.05 {
            fx.tint = rgb(0x80c8ff);
            fx.tint_a = 0.4 * m.frost;
        }
        if m.invuln > 0.0 {
            // Mist form: barely there.
            fx.dither = true;
            fx.tint = rgb(0xc0c8d0);
            fx.tint_a = 0.55;
        }
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
            // The Ember Wyrm curled up on its hoard (a slow breathing loop).
            // (Its "sleep" clip came out standing; a still idle frame reads better.)
            _ if m.asleep => CharFrame::Loop("idle", m.dir, 0.0),
            _ if m.moving => CharFrame::Loop("walk", m.dir, m.anim_t),
            _ => CharFrame::Loop("idle", m.dir, 0.0),
        };
        use crate::mobs::Kind;
        let crow = m.kind == Kind::ClockCrow;
        // Fliers drawn above their shadow, bobbing: crows with each wingbeat, the wheels of eyes and the
        // thunderbird (single images) slowly, so they never look frozen.
        let lift = match m.kind {
            Kind::ClockCrow => 16,
            Kind::Ophanim => 14,
            Kind::OphanPrime => 20,
            Kind::Thunderbird => 22,
            // The anglerlurk (a single image too) drifts just off the sand.
            Kind::Anglerlurk => 3,
            _ => 0,
        };
        let shadow = if m.boss { 20 } else if crow { 6 } else { 10 };
        blend_ellipse(scr, sx, sy, shadow, shadow * 2 / 5, BLACK, if lift > 0 { 0.3 } else { 0.45 });
        let (speed, amp) = if crow { (0.25, 3.0) } else { (0.08, 4.0) };
        let sy = if lift > 0 { sy - lift - ((self.tick as f32 * speed + i as f32 * 1.3).sin() * amp) as i32 } else { sy };
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
        // Asleep (the Ember Wyrm): dim and still.
        if m.asleep && fx.tint_a == 0.0 {
            fx.tint = rgb(0x10080c);
            fx.tint_a = 0.35;
        }
        if m.enraged && fx.tint_a == 0.0 {
            fx.tint = rgb(0xff2010);
            fx.tint_a = 0.18;
        }
        // Mesmerized foes and thralls glow violet; the valkyrie's Einherjar and the reaper's scholars are pale ghosts.
        if matches!(m.kind, crate::mobs::Kind::Einherjar | crate::mobs::Kind::Scholar) {
            fx.tint = rgb(0x9ad8ff);
            fx.tint_a = 0.6;
            fx.dither = (self.tick / 2) % 3 == 0;
        } else if m.charm > 0.0 && !matches!(m.kind, crate::mobs::Kind::DireWolf | crate::mobs::Kind::Rat | crate::mobs::Kind::MossWolf | crate::mobs::Kind::ThornWarden | crate::mobs::Kind::Automaton) {
            fx.tint = rgb(0xa040e0);
            fx.tint_a = 0.35;
        }
        // Champions are tinted blue; elites stand in a golden glow (D2).
        match m.rank {
            crate::mobs::Rank::Champion if fx.tint_a == 0.0 => {
                fx.tint = rgb(0x4060ff);
                fx.tint_a = 0.22;
            }
            crate::mobs::Rank::Elite if m.superu > 0 => {
                let tint = crate::side::SUPERS[m.superu as usize - 1].tint;
                blend_ellipse(scr, sx, sy, 20, 8, rgb(tint), 0.4);
                if fx.tint_a == 0.0 {
                    fx.tint = rgb(tint);
                    fx.tint_a = 0.28;
                }
            }
            crate::mobs::Rank::Elite => blend_ellipse(scr, sx, sy, 16, 6, rgb(0xd8a040), 0.35),
            _ => {}
        }
        // A gold-thief glitters.
        if m.kind == crate::mobs::Kind::Hoarder && (self.tick / 5 + i as u32) % 4 == 0 {
            scr.glow(sx, sy - 10, 9.0, rgb(0xffd040), 0.6);
        }
        if m.mods & crate::mobs::M_FIERY != 0 && (self.tick / 6 + i as u32) % 5 == 0 {
            scr.glow(sx, sy - 14, 10.0, rgb(0xff6020), 0.5);
        }
        let extra = if m.superu > 0 { crate::side::SUPER_SCALE } else { 1.0 };
        self.blit_char_x(scr, name, anim, (sx, sy), fx, m.moving, extra);
    }

    /// Draws the HUD; returns the clickable skill button rectangles.
    /// The Skyreach's backdrop: a sunset gradient over banks of cloud, drifting slowly with the camera.
    fn draw_cloud_sea(&self, scr: &mut Screen) {
        let (w, h) = (scr.w, self.view_h);
        for y in 0..h {
            let k = y as f32 / h as f32;
            let c = if k < 0.5 { mix(rgb(0x3a2850), rgb(0xd07850), k * 2.0) } else { mix(rgb(0xd07850), rgb(0xf0c890), (k - 0.5) * 2.0) };
            scr.fill(0, y, w, 1, c);
        }
        // Cloud banks: soft ellipses in three layers, the near ones moving more with the camera.
        let (cx, cy) = crate::iso::to_screen(self.p.x, self.p.y);
        let t = self.tick as f32 / 60.0;
        for (layer, (par, col, a, n)) in [(0.15f32, rgb(0x8a6878), 0.5f32, 9), (0.3, rgb(0xe8b0a0), 0.45, 11), (0.5, rgb(0xfff0e0), 0.5, 13)].into_iter().enumerate() {
            for i in 0..n {
                let h1 = ((i as u32 + layer as u32 * 31).wrapping_mul(2654435761) >> 8) as f32 / 16_777_216.0;
                let h2 = ((i as u32 + layer as u32 * 17).wrapping_mul(40503).wrapping_add(97) % 1000) as f32 / 1000.0;
                let span = w as f32 + 200.0;
                let x = (h1 * span * 3.0 - cx * par + t * (3.0 + layer as f32 * 2.0)).rem_euclid(span) as i32 - 100;
                let y = (h2 * (h as f32 + 80.0) * 2.0 - cy * par).rem_euclid(h as f32 + 80.0) as i32 - 40;
                let rw = 40 + (h1 * 50.0) as i32 + layer as i32 * 10;
                blend_ellipse(scr, x, y, rw, rw / 3, col, a);
                blend_ellipse(scr, x + rw / 3, y - rw / 8, rw * 2 / 3, rw / 4, col, a * 0.8);
            }
        }
    }

    fn draw_hud(&self, scr: &mut Screen) -> (Vec<(i32, i32, i32, i32)>, (i32, i32, i32, i32)) {
        let (w, h) = (scr.w, self.view_h);
        let top = h - HUD_H;
        // The HUD doesn't shake with the world.
        let shake = std::mem::replace(&mut scr.shake, (0, 0));
        // The carved panel (OpenAI art, tools/oai_hud.py + hud_pack.py), else the plain code-drawn bar.
        if let Some(s) = self.art.item("hud_panel") {
            scr.blit(s, w / 2 - s.w / 2 + s.ax, h - s.h + s.ay, Fx::default());
        } else {
            for y in top..h {
                let k = (y - top) as f32 / HUD_H as f32;
                scr.fill(0, y, w, 1, mix(rgb(0x2a2520), rgb(0x141210), k));
            }
            scr.fill(0, top, w, 1, rgb(0x5a4a38));
            scr.fill(0, top + 1, w, 1, rgb(0x0a0806));
            for k in 0..HUD_SOCKETS {
                let x = HUD_SOCKET0 + k * HUD_MODULE;
                scr.fill(x - 8, top + 2, 6, HUD_H - 2, rgb(0x3a3028));
            }
        }
        // Experience: a thin gold line along the bottom band, under the sockets.
        let xp = (self.p.xp / xp_to_next(self.p.clvl)).clamp(0.0, 1.0);
        let (xx, xw) = (HUD_SOCKET0, HUD_MODULE * HUD_SOCKETS - 8);
        scr.fill(xx - 1, h - 5, xw + 2, 4, rgb(0x0a0806));
        scr.fill(xx, h - 4, (xw as f32 * xp) as i32, 2, rgb(0xc8a040));
        scr.fill(xx, h - 4, (xw as f32 * xp) as i32, 1, rgb(0xffe080));
        // Globes, in their housings (the angel holds life, the gargoyle the other).
        let (lx, gy) = (HUD_ORB_L.2, h - HUD_ORB_L.1 + HUD_ORB_DROP + HUD_ORB_L.3);
        let (rx, ry) = (w - HUD_ORB_R.0 + HUD_ORB_R.2, h - HUD_ORB_R.1 + HUD_ORB_DROP + HUD_ORB_R.3);
        let t = self.tick as f32 / 60.0;
        let r = HUD_ORB_R_PX;
        globe(scr, lx, gy, r, self.p.hp / self.p.max_hp, rgb(0xb01818), rgb(0xff6050), t);
        // The right globe: the class's resource, drawn below at (rx, ry).
        let (w, gy) = (rx + 34, ry);
        let globe = |scr: &mut Screen, _x: i32, gy: i32, _r: i32, frac: f32, dark: u32, hi: u32| globe(scr, rx, gy, r, frac, dark, hi, t + 1.7);
        if self.is_inventor() {
            // Heat: an orange gauge that fills as she fires (it's her mana, upside down).
            let heat = self.heat();
            let (dark, hi) = if self.p.overheat > 0.0 && (self.tick / 6) % 2 == 0 { (rgb(0xe0e0e0), rgb(0xffffff)) } else { (rgb(0xb05010), rgb(0xffa040)) };
            globe(scr, w - 34, gy, 26, heat, dark, hi);
        } else if self.is_reaper() {
            // Souls: a pale spirit-blue lantern.
            let (dark, hi) = if self.p.ledger_t > 0.0 && (self.tick / 8) % 2 == 0 { (rgb(0xb09040), rgb(0xffe0a0)) } else { (rgb(0x2a4a70), rgb(0xa0d8ff)) };
            globe(scr, w - 34, gy, 26, self.p.mana / self.soul_cap(), dark, hi);
        } else if self.is_berserker() {
            // Rage: dark crimson, pulsing in the red mist.
            let (dark, hi) = if self.p.berserk_t > 0.0 && (self.tick / 6) % 2 == 0 { (rgb(0xe02010), rgb(0xffa080)) } else { (rgb(0x701010), rgb(0xd04030)) };
            globe(scr, w - 34, gy, 26, self.p.mana / self.p.max_mana, dark, hi);
        } else if self.is_inquisitor() {
            // Judgment: oxblood, burning gold in Final Judgment.
            let (dark, hi) = if self.p.judge_t > 0.0 && (self.tick / 6) % 2 == 0 { (rgb(0xc08020), rgb(0xfff0a0)) } else { (rgb(0x5a1418), rgb(0xe09040)) };
            globe(scr, w - 34, gy, 26, self.p.mana / self.p.max_mana, dark, hi);
        } else if self.is_valkyrie() {
            // Valor: icy blue, blazing white when full.
            let (dark, hi) = if self.blazing() && (self.tick / 8) % 2 == 0 { (rgb(0x90c0e0), rgb(0xffffff)) } else { (rgb(0x2a5a90), rgb(0x90d8ff)) };
            globe(scr, w - 34, gy, 26, self.p.mana / self.p.max_mana, dark, hi);
        } else {
            globe(scr, w - 34, gy, 26, self.p.mana / self.p.max_mana, rgb(0x1830b0), rgb(0x6090ff));
        }
        // The housings over the globes.
        for (name, x, y) in [("hud_orb_l", 0, h - HUD_ORB_L.1 + HUD_ORB_DROP), ("hud_orb_r", scr.w - HUD_ORB_R.0, h - HUD_ORB_R.1 + HUD_ORB_DROP)] {
            if let Some(s) = self.art.item(name) {
                scr.blit(s, x + s.ax, y + s.ay, Fx::default());
            }
        }
        scr.text(&format!("{}/{}", self.p.hp.ceil() as i32, self.p.max_hp as i32), lx, gy - 4, WHITE, Align::Center, 1);
        if self.is_inventor() {
            let label = if self.p.overheat > 0.0 { "HOT!".to_string() } else { format!("{}%", (self.heat() * 100.0).round() as i32) };
            scr.text(&label, w - 34, gy - 4, WHITE, Align::Center, 1);
            let vent = if self.p.vent_cd > 0.0 { format!("VENT {:.0}S", self.p.vent_cd.ceil()) } else { "VENT: E/Y".into() };
            scr.text(&vent, w - 34, gy - 54, rgb(0xd8b080), Align::Center, 1);
        } else if self.is_druid() {
            scr.text(&format!("{}/{}", self.p.mana.floor() as i32, self.p.max_mana as i32), w - 34, gy - 4, WHITE, Align::Center, 1);
            // Decay (left, sickly yellow-green) ... Bloom (right, fresh green), with a marker.
            let (bx, by, bw) = (w - 82, gy + 18, 48);
            scr.fill(bx - 1, by - 1, bw + 2, 6, rgb(0x100c08));
            scr.fill(bx, by, bw / 2, 4, rgb(0x6a6a18));
            scr.fill(bx + bw / 2, by, bw / 2, 4, rgb(0x2a7a2a));
            let mx = bx + bw / 2 + (self.p.balance * (bw / 2) as f32) as i32;
            scr.fill(mx - 1, by - 2, 3, 8, rgb(0xf0f0d0));
            let label = if self.p.balance < -0.3 {
                "DECAY"
            } else if self.p.balance > 0.3 {
                "BLOOM"
            } else {
                "BALANCE"
            };
            scr.text(label, w - 34, gy - 54, rgb(0xa0d870), Align::Center, 1);
        } else if self.is_reaper() {
            let souls = (self.p.mana / crate::reaper::SOUL).floor() as i32;
            scr.text(&format!("{souls}"), w - 34, gy - 4, WHITE, Align::Center, 1);
            scr.text("SOULS", w - 34, gy - 54, rgb(0xa0d8ff), Align::Center, 1);
            // The scythe's seven runes.
            let blaze = self.p.runes >= crate::reaper::RUNES;
            for k in 0..crate::reaper::RUNES as i32 {
                let lit = (k as u8) < self.p.runes;
                let c = if blaze && (self.tick / 5) % 2 == 0 { rgb(0xffffff) } else if lit { rgb(0x80d0ff) } else { rgb(0x3a4450) };
                let x = w - 70 - (crate::reaper::RUNES as i32 - 1 - k) * 6;
                scr.fill(x, gy + 18, 3, 5, c);
            }
        } else if self.is_berserker() {
            scr.text(&format!("{}", self.p.mana.floor() as i32), w - 34, gy - 4, WHITE, Align::Center, 1);
            let label = if self.p.berserk_t > 0.0 {
                "BERSERK!"
            } else if self.p.exhaust_t > 0.0 {
                "SPENT"
            } else {
                "RAGE"
            };
            scr.text(label, w - 34, gy - 54, rgb(0xe08060), Align::Center, 1);
        } else if self.is_inquisitor() {
            scr.text(&format!("{}", self.p.mana.floor() as i32), w - 34, gy - 4, WHITE, Align::Center, 1);
            scr.text(if self.p.judge_t > 0.0 { "JUDGMENT!" } else { "JUDGMENT" }, w - 34, gy - 54, rgb(0xe8b060), Align::Center, 1);
        } else if self.is_valkyrie() {
            scr.text(&format!("{}", self.p.mana.floor() as i32), w - 34, gy - 4, WHITE, Align::Center, 1);
            scr.text(if self.blazing() { "VALOR!" } else { "VALOR" }, w - 34, gy - 54, rgb(0xa0d8ff), Align::Center, 1);
        } else {
            scr.text(&format!("{}/{}", self.p.mana.floor() as i32, self.p.max_mana as i32), w - 34, gy - 4, WHITE, Align::Center, 1);
        }
        // Skill slots: primary (left click / A) and secondary (right click / X), like D2.
        let w = scr.w;
        let iy = top + 12;
        let sock = |k: i32| HUD_SOCKET0 + k * HUD_MODULE;
        let ix = sock(2) + 38;
        let mut skill_rects = vec![];
        for (k, s) in [self.p.skills.primary, self.p.skills.secondary].into_iter().enumerate() {
            let sx = ix - 10 + k as i32 * 32;
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
        let out = self.p.mana < crate::skills::mana_cost(self.p.skills.primary, self.p.skills.rank(self.p.skills.primary)) || self.p.overheat > 0.0;
        if out {
            let (label, col) = match self.p.skills.class {
                crate::skills::Class::Vampire => ("BITE", rgb(0xd04060)),
                crate::skills::Class::Inventor => ("WEAK", rgb(0x60d0c0)),
                crate::skills::Class::Valkyrie => ("SPEAR", rgb(0x90d8ff)),
                crate::skills::Class::Berserker => ("CLEAVE", rgb(0xe08060)),
                crate::skills::Class::Reaper => ("SCYTHE", rgb(0xa0d8ff)),
                crate::skills::Class::Druid => ("SPORE", rgb(0x90d050)),
                crate::skills::Class::Inquisitor => ("CENSER", rgb(0xe8b060)),
                crate::skills::Class::Sorceress => ("EMBER", rgb(0xff9050)),
            };
            scr.text(label, ix + 18, top + 39, col, Align::Center, 1);
        }
        // Unspent skill points: a pulsing button (opens the tree, like D2's level-up button).
        if self.p.skills.points > 0 {
            let bx = sock(2) + 4;
            let pulse = (self.tick / 15) % 2 == 0;
            scr.fill(bx, iy + 2, 18, 18, if pulse { rgb(0xd8a048) } else { rgb(0x8a6020) });
            scr.text("+", bx + 6, iy + 7, BLACK, Align::Left, 1);
            skill_rects.push((bx, iy + 2, 18, 18));
        }
        // Run / walk button (D2 style).
        let rx = sock(3) + 14;
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
        scr.text(label, rx + 12, top + 39, if self.p.winded { col } else { rgb(0x908070) }, Align::Center, 1);
        // Bag button (inventory).
        let gx = rx + 32;
        scr.fill(gx - 2, iy - 2, 28, 28, rgb(0x5a4a38));
        scr.fill(gx, iy, 24, 24, rgb(0x1a1410));
        scr.text("BAG", gx + 12, iy + 8, rgb(0xd8c090), Align::Center, 1);
        scr.text("I/ST", gx + 12, top + 39, rgb(0x908070), Align::Center, 1);
        let bag = (gx - 2, iy - 2, 28, 28);
        // Stamina and food bars.
        let (bar_x, bar_w) = (sock(1) + 4, HUD_MODULE - 17);
        let st = self.p.stamina / MAX_STAMINA;
        let st_col = if self.p.winded { rgb(0xa03020) } else { rgb(0xd8b020) };
        scr.text("STAMINA", bar_x, top + 4, rgb(0xb0a090), Align::Left, 1);
        bar(scr, bar_x, top + 14, bar_w, st, st_col);
        let fd = self.p.food / MAX_FOOD;
        let starving = self.p.food <= 0.0;
        let vampire = self.p.skills.class == crate::skills::Class::Vampire;
        let fd_col = if vampire {
            if fd < 0.25 {
                rgb(0x701020)
            } else {
                rgb(0xb01830)
            }
        } else if fd < 0.25 {
            rgb(0xc04020)
        } else {
            rgb(0xb07030)
        };
        let flabel = match (vampire, starving, fd < 0.25) {
            (true, true, _) => "BLOOD - BLOODTHIRSTY!",
            (true, false, true) => "BLOOD - THIRSTY",
            (true, ..) => "BLOOD",
            (false, true, _) => "FOOD - STARVING!",
            (false, false, true) => "FOOD - HUNGRY",
            _ => "FOOD",
        };
        let fcol = if starving && (self.tick / 20) % 2 == 0 { rgb(0xff5030) } else { rgb(0xb0a090) };
        let flabel = flabel.replace("BLOODTHIRSTY", "THIRST").replace(" - ", " ");
        scr.text(&flabel, bar_x, top + 23, fcol, Align::Left, 1);
        bar(scr, bar_x, top + 33, bar_w, fd, fd_col);
        // Potions.
        let bx = sock(0) + 8;
        potion(scr, bx, top + 14, rgb(0xc02020));
        scr.text(&format!("X{}", self.p.hp_pots), bx + 14, top + 18, WHITE, Align::Left, 1);
        scr.text("Q", bx + 2, top + 34, rgb(0x908070), Align::Left, 1);
        potion(scr, bx + 42, top + 14, rgb(0x2040c0));
        scr.text(&format!("X{}", self.p.mp_pots), bx + 56, top + 18, WHITE, Align::Left, 1);
        scr.text("E", bx + 44, top + 34, rgb(0x908070), Align::Left, 1);
        // Gold, level and the act's relics: the top right corner, across from the area name.
        scr.text(&format!("GOLD {}", self.p.gold), w - 6, 6, rgb(0xe8c050), Align::Right, 1);
        scr.text(&format!("CHAR LEVEL {}", self.p.clvl), w - 6, 17, rgb(0xd8c090), Align::Right, 1);
        let (relic, col) = match self.level.act() {
            5 => (format!("SHARDS {}/3", self.quest.shard_count()), rgb(0xffe080)),
            4 => (format!("PEARLS {}/3", self.quest.pearl_count()), rgb(0x80e8e0)),
            3 => (format!("KEYS {}/3", self.quest.key_count()), rgb(0xe0b040)),
            2 => (format!("SIGILS {}/3", self.quest.sigil_count()), rgb(0x60f080)),
            1 => (format!("RUNES {}/3", self.quest.rune_count()), rgb(0x90d0ff)),
            _ => (format!("SEALS {}/3", self.quest.seal_count()), rgb(0xc8a0ff)),
        };
        scr.text(&relic, w - 6, 28, col, Align::Right, 1);
        // The tide gauge (the Sunken Reach).
        if let Some((label, level)) = self.tide_gauge() {
            let warn = label == "THE TIDE BELL!" && (self.tick / 15) % 2 == 0;
            let col = if warn { rgb(0xffe080) } else { rgb(0x80d0e0) };
            scr.text(label, w - 6, 39, col, Align::Right, 1);
            let (bx, bw) = (w - 66, 60);
            scr.fill(bx - 1, 49, bw + 2, 5, rgb(0x0a1418));
            scr.fill(bx, 50, (bw as f32 * level) as i32, 3, rgb(0x2a90b0));
        }
        scr.shake = shake;

        // An Ash Rift's bar, clock and modifiers.
        crate::endgame::draw_rift_hud(self, scr);
        // Area name and quest log (top left).
        scr.text(&self.level_name, 6, 6, rgb(0xd8b878), Align::Left, 1);
        let log = self.quest_log();
        scr.text(&log, 6, 17, rgb(0x9a8a78), Align::Left, 1);
        if let Some(side) = self.side_log() {
            scr.text(&side, 6, 27, rgb(0x7a8a68), Align::Left, 1);
        }
        if let Some((t, col)) = self.errand_log() {
            let y = if self.side_log().is_some() { 38 } else { 27 };
            scr.text(&t, 6, y, col, Align::Left, 1);
        }
        // In the Ember Wyrm's lair: how close it is to waking, and the sack.
        if let Some((noise, awake, sack)) = self.wyrm_hud() {
            let (bw, bx) = (160, w / 2 - 80);
            let label = if awake { "VAURATH IS AWAKE: RUN!".to_string() } else { format!("THE WYRM SLEEPS   SACK: {sack} GOLD") };
            let col = if awake { if (self.tick / 10) % 2 == 0 { rgb(0xff4020) } else { rgb(0xffa040) } } else { rgb(0xe8c070) };
            scr.text(&label, w / 2, 46, col, Align::Center, 1);
            if !awake {
                scr.fill(bx - 1, 57, bw + 2, 6, rgb(0x1a0c06));
                let c = if noise > 0.7 { rgb(0xff4020) } else if noise > 0.4 { rgb(0xffa030) } else { rgb(0x80a040) };
                scr.fill(bx, 58, (bw as f32 * noise) as i32, 4, c);
            } else if sack > 0 {
                scr.text(&format!("SACK: {sack} GOLD"), w / 2, 57, rgb(0xffd040), Align::Center, 1);
            }
        }
        if let Some((b, t)) = self.blessing() {
            let label = format!("{}  {}", b.name(), t.ceil() as i32);
            let col = if t < 10.0 && (self.tick / 15) % 2 == 0 { rgb(0x806050) } else { rgb(b.col()) };
            scr.text(&label, 6, 49, col, Align::Left, 1);
        }

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
                let epilogue = if self.quest.stage6 >= 3 {
                    story::EPILOGUE6
                } else if self.quest.stage5 >= 3 {
                    story::EPILOGUE5
                } else if self.quest.stage4 >= 3 {
                    story::EPILOGUE4
                } else if self.quest.stage3 >= 3 {
                    story::EPILOGUE3
                } else if self.quest.stage2 >= 3 {
                    story::EPILOGUE2
                } else {
                    story::EPILOGUE
                };
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
            HazardKind::Slag => rgb(0xff8020),
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
    } else if kind == HazardKind::Slag {
        let fade = (1.0 - (t - warn) / live.max(0.01)).clamp(0.0, 1.0);
        blend_ellipse(scr, sx, sy, rx, ry, rgb(0xc04010), 0.5 * fade + 0.15);
        blend_ellipse(scr, sx, sy, rx * 2 / 3, ry * 2 / 3, rgb(0xffa030), 0.35 * fade + 0.1);
        for k in 0..4 {
            let a = (tick as f32 * 0.04 + k as f32 * 1.7).sin();
            scr.disc(sx + (a * rx as f32 * 0.5) as i32, sy + ((k as f32 * 2.3).cos() * ry as f32 * 0.4) as i32, 1, rgb(0xffe080));
        }
    } else if kind == HazardKind::Poison {
        let fade = (1.0 - (t - warn) / live.max(0.01)).clamp(0.0, 1.0);
        blend_ellipse(scr, sx, sy, rx, ry, rgb(0x40a018), 0.45 * fade + 0.1);
        for k in 0..5 {
            let a = (tick as f32 * 0.05 + k as f32 * 1.3).sin();
            scr.disc(sx + (a * rx as f32 * 0.6) as i32, sy + ((k as f32 * 2.1).cos() * ry as f32 * 0.5) as i32, 1, rgb(0xa0ff60));
        }
    }
}

pub(crate) fn ring(scr: &mut Screen, cx: i32, cy: i32, rx: i32, ry: i32, c: u32) {
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

// The bottom HUD's art layout (tools/hud_pack.py in the art repo writes the matching images):
// the housings' (width, height, globe centre x, globe centre y), how far their pedestals run off the screen,
// the globes' radius, and the panel's sockets (one strap + recess each).
const HUD_ORB_L: (i32, i32, i32, i32) = (126, 126, 72, 45);
const HUD_ORB_R: (i32, i32, i32, i32) = (121, 121, 48, 43);
const HUD_ORB_DROP: i32 = 16;
const HUD_ORB_R_PX: i32 = 24;
pub(crate) const HUD_SOCKET0: i32 = 132;
pub(crate) const HUD_MODULE: i32 = 97;
const HUD_SOCKETS: i32 = 4;

/// The housings rise above the panel: true if (x, y) is on one (clicks there aren't walking orders).
pub(crate) fn on_hud_orb(w: i32, h: i32, x: i32, y: i32) -> bool {
    let (lx, ly) = (HUD_ORB_L.2, h - HUD_ORB_L.1 + HUD_ORB_DROP + HUD_ORB_L.3);
    let (rx, ry) = (w - HUD_ORB_R.0 + HUD_ORB_R.2, h - HUD_ORB_R.1 + HUD_ORB_DROP + HUD_ORB_R.3);
    let r = HUD_ORB_R_PX + 14;
    (x - lx).pow(2) + (y - ly).pow(2) < r * r || (x - rx).pow(2) + (y - ry).pow(2) < r * r
}

/// A life or mana globe: a glass sphere of swirling liquid, the surface rippling, bubbles rising, a glint on the glass.
fn globe(scr: &mut Screen, cx: i32, cy: i32, r: i32, frac: f32, col: u32, hi: u32, t: f32) {
    let frac = frac.clamp(0.0, 1.0);
    let level = cy as f32 + r as f32 - 2.0 * r as f32 * frac;
    let rf = r as f32;
    // A few bubbles rising through the liquid.
    let bubbles: Vec<(i32, i32)> = (0..6)
        .map(|i| {
            let fi = i as f32;
            let bx = ((fi * 7.3).sin() * 0.6 * rf + (t * 2.0 + fi).sin() * 1.5) as i32;
            let by = r - ((t * (6.0 + fi * 1.3) + fi * 9.0) % (2.0 * rf)) as i32;
            (bx, by)
        })
        .collect();
    for y in -r..=r {
        for x in -r..=r {
            let d2 = x * x + y * y;
            if d2 > r * r {
                continue;
            }
            let (fx, fy) = (x as f32, y as f32);
            let d = (d2 as f32).sqrt() / rf;
            // The surface ripples (still when empty or full).
            let wave = if frac > 0.02 && frac < 0.98 { (fx * 0.32 + t * 3.1).sin() * 1.1 + (fx * 0.71 - t * 2.3).sin() * 0.5 } else { 0.0 };
            let surf = level + wave;
            let py = cy as f32 + fy;
            let mut c = if py >= surf {
                // Liquid: lit from the upper left, with slow swirling bands.
                let shade = (1.0 - (fx + rf / 3.0).hypot(fy + rf / 3.0) / (rf * 1.6)).clamp(0.0, 1.0);
                let swirl = ((fx * 0.22 + fy * 0.16 + t * 1.3 + (fy * 0.21 - t * 0.9).sin() * 2.2).sin() * 0.5 + 0.5).powi(3);
                let mut c = mix(mix(col, BLACK, 0.55), hi, shade * 0.6 + swirl * 0.22);
                if py < surf + 1.5 {
                    c = mix(c, hi, 0.55); // the bright surface line
                }
                if bubbles.iter().any(|&(bx, by)| bx == x && by == y && (by as f32 + cy as f32) > surf + 2.0) {
                    c = mix(c, rgb(0xffffff), 0.55);
                }
                c
            } else {
                // Empty glass: near black, a faint tint of the colour.
                mix(rgb(0x0a0809), col, 0.06 + (1.0 - d) * 0.05)
            };
            // Shadow in the glass's rim.
            if d > 0.86 {
                c = mix(c, BLACK, (d - 0.86) * 3.5);
            }
            // The glint: a curved highlight on the upper left of the glass.
            let ang = fy.atan2(fx);
            if d > 0.62 && d < 0.78 && ang > -2.65 && ang < -1.75 {
                c = mix(c, rgb(0xffffff), 0.35);
            }
            scr.pset(cx + x, cy + y, c);
        }
    }
    scr.disc(cx - r / 3, cy - r / 2, 1, mix(rgb(0xffffff), hi, 0.3));
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
        &Drop::Shard(i) => {
            let tint = [rgb(0xfff0c0), rgb(0xc0a0ff), rgb(0xffe080)][i];
            let y = sy - 10 - pop + bob;
            scr.glow(sx, y, 26.0, rgb(0xffc040), 0.9);
            // A shard of sunlight: a bright diamond.
            for k in 0..6 {
                scr.fill(sx - (5 - k), y - 6 + k, (5 - k) * 2 + 1, 1, tint);
                scr.fill(sx - (5 - k), y + 6 - k, (5 - k) * 2 + 1, 1, tint);
            }
            scr.pset(sx - 1, y - 2, rgb(0xffffff));
        }
        &Drop::Pearl(i) => {
            let tint = [rgb(0xd0e8f0), rgb(0xf0d8f0), rgb(0x80f0e0)][i];
            let y = sy - 10 - pop + bob;
            scr.glow(sx, y, 24.0, rgb(0x60e0e0), 0.8);
            // A great pearl, catching the light.
            scr.disc(sx, y, 5, rgb(0x203038));
            scr.disc(sx, y, 4, tint);
            scr.disc(sx - 1, y - 1, 1, rgb(0xffffff));
        }
        &Drop::Hoard(_) => {
            // A heap of the Ember Wyrm's gold, glinting.
            let y = sy - 4;
            scr.glow(sx, y - 4, 16.0, rgb(0xffc040), 0.35 + 0.15 * ((tick as f32) * 0.05 + sx as f32).sin());
            match art.item("hoard_gold") {
                Some(s) => scr.blit(s, sx, y + 6, Fx::default()),
                None => {
                    for k in 0..4 {
                        scr.fill(sx - 7 + k * 2, y - k * 2, 14 - k * 4, 2, if k % 2 == 0 { rgb(0xe0a020) } else { rgb(0xffd860) });
                    }
                }
            }
        }
        &Drop::TideChest => {
            // A wrecked chest half buried in the flats.
            let y = sy - 4;
            blend_ellipse(scr, sx, y + 3, 16, 5, BLACK, 0.4);
            scr.glow(sx, y - 6, 22.0, rgb(0xffd060), 0.35);
            scr.fill(sx - 13, y - 13, 26, 16, rgb(0x2a1808));
            scr.fill(sx - 12, y - 12, 24, 14, rgb(0x6a4a24));
            scr.fill(sx - 12, y - 12, 24, 4, rgb(0x8a6232));
            scr.fill(sx - 13, y - 7, 26, 2, rgb(0x9a8a50));
            scr.fill(sx - 9, y - 12, 2, 14, rgb(0x9a8a50));
            scr.fill(sx + 7, y - 12, 2, 14, rgb(0x9a8a50));
            scr.fill(sx - 2, y - 8, 4, 5, rgb(0xd8b040));
            scr.fill(sx - 12, y, 24, 2, rgb(0x40706a));
        }
        &Drop::StormRelic => {
            // A glass sphere with a dead spark, on a stone stub.
            let y = sy - 8 + bob;
            scr.fill(sx - 4, sy - 4, 8, 5, rgb(0x6a6a7a));
            scr.glow(sx, y, 14.0, rgb(0x80b0ff), 0.45);
            ring(scr, sx, y, 5, 5, rgb(0xc0d8ff));
            scr.fill(sx - 1, y - 1, 2, 2, rgb(0xffffff));
        }
        &Drop::Singer(_) => {
            // A lost singer: a pale robed figure in a shaft of light.
            let t = (tick as f32 * 0.06).sin();
            scr.glow(sx, sy - 14, 22.0, rgb(0xfff0c0), 0.35 + 0.1 * t);
            scr.fill(sx - 1, sy - 40, 2, 40, mix(rgb(0xfff0c0), BLACK, 0.4));
            scr.fill(sx - 4, sy - 16, 8, 14, rgb(0xe8e0d0));
            scr.fill(sx - 3, sy - 21, 6, 5, rgb(0xf0d8c0));
            scr.fill(sx - 5, sy - 3, 10, 2, rgb(0xb8b0a0));
            let ny = sy - 26 - ((tick / 4) % 10) as i32;
            scr.fill(sx + 4, ny, 2, 2, rgb(0xfff0a0));
            scr.fill(sx + 6, ny - 4, 1, 5, rgb(0xfff0a0));
        }
        &Drop::Cargo(_) => {
            // A sky-pirate's crate, Bram's mark on the side.
            let y = sy - 4;
            blend_ellipse(scr, sx, y + 4, 12, 4, BLACK, 0.4);
            scr.fill(sx - 9, y - 14, 18, 16, rgb(0x3a2a14));
            scr.fill(sx - 8, y - 13, 16, 14, rgb(0x8a6a3a));
            scr.fill(sx - 8, y - 7, 16, 2, rgb(0x5a4020));
            scr.fill(sx - 2, y - 13, 2, 14, rgb(0x5a4020));
            scr.fill(sx + 2, y - 11, 4, 4, rgb(0xc03020));
            scr.glow(sx, y - 6, 14.0, rgb(0xffe0a0), 0.2);
        }
        &Drop::Bottle(_) => {
            let y = sy - 6 - pop + bob;
            scr.glow(sx, y, 12.0, rgb(0x80e0c0), 0.5);
            scr.fill(sx - 2, y - 5, 4, 8, rgb(0x3a8a6a));
            scr.fill(sx - 1, y - 7, 2, 2, rgb(0x8a6a40));
            scr.fill(sx - 1, y - 3, 2, 3, rgb(0xe8d8b0));
        }
        &Drop::Gear(_) => {
            let y = sy - 8 - pop + bob;
            scr.glow(sx, y, 18.0, rgb(0xffd060), 0.7);
            scr.disc(sx, y, 5, rgb(0x6a4a18));
            scr.disc(sx, y, 4, rgb(0xd8a840));
            for k in 0..8 {
                let a = k as f32 * std::f32::consts::FRAC_PI_4 + tick as f32 * 0.03;
                scr.fill(sx + (a.cos() * 5.5) as i32, y + (a.sin() * 5.5) as i32, 2, 2, rgb(0xd8a840));
            }
            scr.disc(sx, y, 1, rgb(0x3a2808));
        }
        &Drop::Keepsake(_) => {
            let y = sy - 8 - pop + bob;
            scr.glow(sx, y, 18.0, rgb(0xc0d8ff), 0.7);
            scr.disc(sx, y, 4, rgb(0xc8c0e0));
            scr.disc(sx, y, 2, rgb(0x6050a0));
            scr.fill(sx, y - 6, 1, 3, rgb(0xd8d0b0));
        }
        &Drop::Herb => {
            // A glowing herb.
            let y = sy - 6 + bob;
            scr.glow(sx, y, 14.0, rgb(0x80ff80), 0.55);
            scr.fill(sx, y - 6, 1, 8, rgb(0x2a7a2a));
            scr.fill(sx - 3, y - 3, 3, 2, rgb(0x60c060));
            scr.fill(sx + 1, y - 5, 3, 2, rgb(0x60c060));
            scr.disc(sx, y - 7, 2, rgb(0xd0ff80));
        }
        &Drop::Heirloom => {
            let y = sy - 8 - pop + bob;
            scr.glow(sx, y, 18.0, rgb(0xd0a0ff), 0.7);
            scr.disc(sx, y, 4, rgb(0xe8c040));
            scr.disc(sx, y, 2, rgb(0x402008));
            scr.pset(sx, y - 4, rgb(0xc080ff));
        }
        &Drop::Clue => {
            let y = sy - 8 - pop + bob;
            scr.glow(sx, y, 16.0, rgb(0xffe0a0), 0.6);
            scr.fill(sx - 6, y - 4, 12, 9, rgb(0xd8c090));
            scr.fill(sx - 4, y - 2, 3, 1, rgb(0x806040));
            scr.fill(sx + 1, y + 1, 3, 1, rgb(0x806040));
            scr.fill(sx + 2, y - 2, 1, 1, rgb(0xc02020));
            scr.fill(sx + 3, y - 1, 1, 1, rgb(0xc02020));
        }
        &Drop::Page(_) => {
            let y = sy - 8 - pop + bob;
            scr.glow(sx, y, 18.0, rgb(0xffe8b0), 0.6);
            scr.fill(sx - 5, y - 4, 10, 8, rgb(0x3a2a18));
            scr.fill(sx - 4, y - 3, 8, 6, rgb(0xe8d8a8));
            for k in 0..3 {
                scr.fill(sx - 3, y - 2 + k * 2, 6, 1, rgb(0x8a7050));
            }
            scr.fill(sx - 5, y - 5, 2, 10, rgb(0xc8a870));
            scr.fill(sx + 4, y - 5, 2, 10, rgb(0xc8a870));
        }
        &Drop::Key(i) => {
            let tint = [rgb(0xff9040), rgb(0xe0c060), rgb(0x80b0ff)][i];
            let y = sy - 10 - pop + bob;
            scr.glow(sx, y, 22.0, rgb(0xffc040), 0.8);
            // A big brass winding key.
            scr.fill(sx - 1, y - 5, 3, 9, rgb(0xb08020));
            scr.disc(sx, y - 6, 3, tint);
            scr.disc(sx, y - 6, 1, rgb(0x302010));
            scr.fill(sx + 1, y + 2, 3, 1, rgb(0xe0b040));
            scr.fill(sx + 1, y, 2, 1, rgb(0xe0b040));
        }
        &Drop::Sigil(i) => {
            let tint = [rgb(0xd0d0b0), rgb(0x60f080), rgb(0x8090a0)][i];
            let y = sy - 10 - pop + bob;
            scr.glow(sx, y, 22.0, rgb(0x40e070), 0.8);
            match art.item("seal") {
                Some(s) => scr.blit(s, sx, y + 7, Fx { tint, tint_a: 0.6, ..Fx::default() }),
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


/// Small integer hash (lightning jitter).
fn hash3(a: i32, b: i32, c: i32) -> u32 {
    let mut h = (a as u32).wrapping_mul(374_761_393) ^ (b as u32).wrapping_mul(668_265_263) ^ (c as u32).wrapping_mul(2_246_822_519);
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    h ^ (h >> 16)
}

/// The valkyrie's wings: two fans of black raven feathers whose tips turn to ice, `k` = how spread.
fn draw_wings(scr: &mut Screen, cx: i32, cy: i32, tick: u32, k: f32) {
    let beat = ((tick as f32) * 0.15).sin() * 0.08;
    for side in [-1.0f32, 1.0] {
        for f in 0..9 {
            let a = (0.15 + f as f32 * 0.14 + beat) * k;
            let len = (16.0 + f as f32 * 2.2) * k;
            let (ux, uy) = (side * a.cos(), -a.sin() * 0.9 + 0.35);
            let n = len as i32;
            for t in 0..n {
                let x = cx + (ux * t as f32) as i32 + side as i32 * 3;
                let y = cy + (uy * t as f32) as i32;
                // Black near the body, icy blue at the tips.
                let ice = t as f32 / len.max(1.0);
                let c = if ice > 0.7 { rgb(0xb0e8ff) } else if ice > 0.5 { rgb(0x4a7aa0) } else { rgb(0x101018) };
                scr.pset(x, y, c);
                scr.pset(x, y + 1, if ice > 0.7 { rgb(0x80c8f0) } else { rgb(0x181824) });
            }
        }
    }
    scr.glow(cx, cy, 26.0 * k, rgb(0x60b0ff), 0.25);
}

/// The reaper's scythe (a stand-in until her sprite exists): a dark shaft from her hands and a curved
/// black blade with rune lights. `ang` is the world angle the blade points (swinging, it sweeps round).
fn draw_scythe(scr: &mut Screen, cx: i32, cy: i32, ang: f32, swinging: bool, runes: u8, tick: u32) {
    // World direction -> screen (isometric squash), then a long shaft from her grip.
    let (dx, dy) = iso::to_screen(ang.cos(), ang.sin());
    let l = (dx * dx + dy * dy).sqrt().max(0.01);
    let (ux, uy) = (dx / l, dy / l);
    let len = if swinging { 30.0 } else { 26.0 };
    // Resting, the scythe stands upright at her side; swinging, it reaches out along the cut.
    let (tx, ty) = if swinging { (cx as f32 + ux * len, cy as f32 + uy * len * 0.8 - 6.0) } else { (cx as f32 + 7.0, cy as f32 - 30.0) };
    let (bx, by) = if swinging { (cx as f32 - ux * 6.0, cy as f32 - uy * 4.0 + 6.0) } else { (cx as f32 + 5.0, cy as f32 + 14.0) };
    let (sx, sy) = (scr.shake.0 as f32, scr.shake.1 as f32);
    line(scr, (bx + sx) as i32, (by + sy) as i32, (tx + sx) as i32, (ty + sy) as i32, rgb(0x2a2220));
    line(scr, (bx + sx) as i32 + 1, (by + sy) as i32, (tx + sx) as i32 + 1, (ty + sy) as i32, rgb(0x3a302a));
    // The blade: a crescent leaving the top of the shaft, curving back.
    let (px, py) = if swinging { (-uy, ux) } else { (1.0, 0.25) };
    for k in 0..14 {
        let t = k as f32 / 13.0;
        let bend = (t * std::f32::consts::PI * 0.9).sin() * 7.0;
        let x = tx + px * t * 16.0 - (if swinging { ux } else { 0.0 }) * bend;
        let y = ty + py * t * 16.0 - (if swinging { uy } else { 1.0 }) * bend;
        let (x, y) = ((x + sx) as i32, (y + sy) as i32);
        scr.pset(x, y, rgb(0x30343c));
        scr.pset(x, y + 1, rgb(0x1a1c22));
        // The runes along the edge: lit ones glow.
        if k % 2 == 1 {
            let lit = (k / 2) < runes as i32;
            if lit {
                scr.glow(x, y - 1, 3.0, rgb(0x60c0ff), 0.6);
            }
            scr.pset(x, y - 1, if lit { rgb(0xd8f4ff) } else { rgb(0x4a6a8a) });
        }
    }
    // The spirit lantern swinging under the blade.
    let sway = ((tick as f32) * 0.12).sin() * 1.5;
    let (lx, ly) = ((tx + px * 5.0 + sway + sx) as i32, (ty + py * 5.0 + 6.0 + sy) as i32);
    scr.fill(lx - 1, ly - 1, 3, 4, rgb(0x202026));
    scr.pset(lx, ly, rgb(0xb8e8ff));
    scr.glow(lx, ly, 6.0, rgb(0x60b0ff), 0.5);
}

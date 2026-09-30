//! Software framebuffer, sprites, lighting and the built-in bitmap font.
//!
//! The framebuffer is 640 pixels wide; its height is 360 on desktop (16:9) and
//! 480 on the RG35XX H (4:3), chosen at start-up. Everything is drawn in screen
//! pixels; the isometric projection lives in `iso.rs`.

pub const SW: i32 = 640;
pub const SH_WIDE: i32 = 360;
pub const SH_TALL: i32 = 480;
pub const BLACK: u32 = 0xFF00_0000;
pub const WHITE: u32 = 0xFFF0_ECE0;

pub const fn rgb(v: u32) -> u32 {
    0xFF00_0000 | v
}

pub fn mix(dst: u32, src: u32, a: f32) -> u32 {
    let a = a.clamp(0.0, 1.0);
    let ch = |s: u32| {
        let d = ((dst >> s) & 0xFF) as f32;
        let c = ((src >> s) & 0xFF) as f32;
        (d + (c - d) * a) as u32
    };
    0xFF00_0000 | (ch(16) << 16) | (ch(8) << 8) | ch(0)
}

/// Saturating additive blend of `src * a` onto `dst`.
pub fn add(dst: u32, src: u32, a: f32) -> u32 {
    let ch = |s: u32| {
        let d = (dst >> s) & 0xFF;
        let c = (((src >> s) & 0xFF) as f32 * a) as u32;
        (d + c).min(255)
    };
    0xFF00_0000 | (ch(16) << 16) | (ch(8) << 8) | ch(0)
}

/// An image; a pixel value of 0 is transparent.
#[derive(Clone)]
pub struct Sprite {
    pub w: i32,
    pub h: i32,
    pub px: Vec<u32>,
    /// Anchor (feet / tile centre) in sprite pixels.
    pub ax: i32,
    pub ay: i32,
}

impl Sprite {
    pub fn new(w: i32, h: i32) -> Self {
        Sprite { w, h, px: vec![0; (w * h).max(0) as usize], ax: w / 2, ay: h - 1 }
    }

    pub fn set(&mut self, x: i32, y: i32, c: u32) {
        if x >= 0 && y >= 0 && x < self.w && y < self.h {
            self.px[(y * self.w + x) as usize] = c;
        }
    }

    pub fn get(&self, x: i32, y: i32) -> u32 {
        if x >= 0 && y >= 0 && x < self.w && y < self.h {
            self.px[(y * self.w + x) as usize]
        } else {
            0
        }
    }

    pub fn fill(&mut self, x: i32, y: i32, w: i32, h: i32, c: u32) {
        for yy in y..y + h {
            for xx in x..x + w {
                self.set(xx, yy, c);
            }
        }
    }

    pub fn ellipse(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, c: u32) {
        for y in (cy - ry).floor() as i32..=(cy + ry).ceil() as i32 {
            for x in (cx - rx).floor() as i32..=(cx + rx).ceil() as i32 {
                let (dx, dy) = ((x as f32 + 0.5 - cx) / rx, (y as f32 + 0.5 - cy) / ry);
                if dx * dx + dy * dy <= 1.0 {
                    self.set(x, y, c);
                }
            }
        }
    }

    /// Adds a 1-pixel black outline around every opaque pixel.
    pub fn outline(&mut self, c: u32) {
        let src = self.px.clone();
        for y in 0..self.h {
            for x in 0..self.w {
                if src[(y * self.w + x) as usize] != 0 {
                    continue;
                }
                let hit = [(-1, 0), (1, 0), (0, -1), (0, 1)].iter().any(|(dx, dy)| {
                    let (sx, sy) = (x + dx, y + dy);
                    sx >= 0 && sy >= 0 && sx < self.w && sy < self.h && src[(sy * self.w + sx) as usize] != 0
                });
                if hit {
                    self.px[(y * self.w + x) as usize] = c;
                }
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum Align {
    Left,
    Center,
    Right,
}

/// Per-blit effects.
#[derive(Clone, Copy, Default)]
pub struct Fx {
    /// Blend every pixel toward this colour by `tint_a`.
    pub tint: u32,
    pub tint_a: f32,
    /// Checkerboard dither (half of the pixels skipped) for see-through walls.
    pub dither: bool,
    /// Global opacity (0 = default opaque).
    pub alpha: f32,
    /// Only draw rows above this sprite row (for sinking corpses).
    pub cut: i32,
}

pub struct Screen {
    pub w: i32,
    pub h: i32,
    pub px: Vec<u32>,
    /// Light level per pixel, 0..=255.
    light: Vec<u16>,
    /// Pre-computed player light (static because the camera follows the player).
    base_light: Vec<u16>,
    pub shake: (i32, i32),
}

impl Screen {
    pub fn new(h: i32) -> Self {
        let n = (SW * h) as usize;
        Screen { w: SW, h, px: vec![BLACK; n], light: vec![0; n], base_light: vec![0; n], shake: (0, 0) }
    }

    /// Radial light centred on the player's screen position (in pixels).
    pub fn build_base_light(&mut self, cx: i32, cy: i32, radius: f32, ambient: f32) {
        for y in 0..self.h {
            for x in 0..self.w {
                // Isometric floor: the light pool is an ellipse twice as wide as it is tall.
                let dx = (x - cx) as f32 / radius;
                let dy = (y - cy) as f32 * 2.0 / radius;
                let d = (dx * dx + dy * dy).sqrt();
                let l = (1.0 - d).clamp(0.0, 1.0);
                let l = ambient + (1.0 - ambient) * (l * l * (3.0 - 2.0 * l));
                self.base_light[(y * self.w + x) as usize] = (l * 256.0) as u16;
            }
        }
    }

    pub fn clear(&mut self, c: u32) {
        self.px.fill(c);
    }

    #[inline]
    pub fn pset(&mut self, x: i32, y: i32, c: u32) {
        if x >= 0 && y >= 0 && x < self.w && y < self.h {
            self.px[(y * self.w + x) as usize] = c;
        }
    }

    pub fn fill(&mut self, x: i32, y: i32, w: i32, h: i32, c: u32) {
        let (x0, y0) = (x.max(0), y.max(0));
        let (x1, y1) = ((x + w).min(self.w), (y + h).min(self.h));
        if x0 >= x1 {
            return;
        }
        for yy in y0..y1 {
            let row = (yy * self.w) as usize;
            self.px[row + x0 as usize..row + x1 as usize].fill(c);
        }
    }

    pub fn blend(&mut self, x: i32, y: i32, w: i32, h: i32, c: u32, a: f32) {
        for yy in y.max(0)..(y + h).min(self.h) {
            for xx in x.max(0)..(x + w).min(self.w) {
                let i = (yy * self.w + xx) as usize;
                self.px[i] = mix(self.px[i], c, a);
            }
        }
    }

    /// Draws a sprite so that its anchor lands on (x, y).
    pub fn blit(&mut self, s: &Sprite, x: i32, y: i32, fx: Fx) {
        let (x, y) = (x + self.shake.0, y + self.shake.1);
        let (dx0, dy0) = (x - s.ax, y - s.ay);
        let ys = (-dy0).max(0);
        let ye = s.h.min(self.h - dy0).min(if fx.cut > 0 { fx.cut } else { s.h });
        let xs = (-dx0).max(0);
        let xe = s.w.min(self.w - dx0);
        for sy in ys..ye {
            let drow = ((dy0 + sy) * self.w + dx0) as isize;
            let srow = (sy * s.w) as usize;
            for sx in xs..xe {
                let c = s.px[srow + sx as usize];
                if c == 0 {
                    continue;
                }
                if fx.dither && ((sx + dx0 + sy + dy0) & 1) == 0 {
                    continue;
                }
                let di = (drow + sx as isize) as usize;
                let mut c = c;
                if fx.tint_a > 0.0 {
                    c = mix(c, fx.tint, fx.tint_a);
                }
                self.px[di] = if fx.alpha > 0.0 { mix(self.px[di], c, fx.alpha) } else { c };
            }
        }
    }

    /// Soft additive glow (used for fire). Does not respect lighting: call after `apply_light`
    /// or before it for lit/unlit variants.
    pub fn glow(&mut self, cx: i32, cy: i32, r: f32, c: u32, a: f32) {
        let (cx, cy) = (cx + self.shake.0, cy + self.shake.1);
        let ri = r.ceil() as i32;
        for y in (cy - ri / 2).max(0)..(cy + ri / 2 + 1).min(self.h) {
            for x in (cx - ri).max(0)..(cx + ri + 1).min(self.w) {
                let dx = (x - cx) as f32 / r;
                let dy = (y - cy) as f32 * 2.0 / r;
                let d = dx * dx + dy * dy;
                if d < 1.0 {
                    let i = (y * self.w + x) as usize;
                    self.px[i] = add(self.px[i], c, a * (1.0 - d) * (1.0 - d));
                }
            }
        }
    }

    pub fn disc(&mut self, cx: i32, cy: i32, r: i32, c: u32) {
        for y in -r..=r {
            for x in -r..=r {
                if x * x + y * y <= r * r + r {
                    self.pset(cx + x, cy + y, c);
                }
            }
        }
    }

    // ---- lighting ----

    pub fn begin_light(&mut self) {
        self.light.copy_from_slice(&self.base_light);
    }

    /// Adds a point light (screen pixels). `strength` 1.0 = full brightness at the centre.
    pub fn add_light(&mut self, cx: i32, cy: i32, r: f32, strength: f32) {
        let ri = r.ceil() as i32;
        for y in (cy - ri / 2).max(0)..(cy + ri / 2 + 1).min(self.h) {
            for x in (cx - ri).max(0)..(cx + ri + 1).min(self.w) {
                let dx = (x - cx) as f32 / r;
                let dy = (y - cy) as f32 * 2.0 / r;
                let d = 1.0 - (dx * dx + dy * dy);
                if d > 0.0 {
                    let i = (y * self.w + x) as usize;
                    self.light[i] = (self.light[i] + (d * strength * 256.0) as u16).min(320);
                }
            }
        }
    }

    /// Multiplies the framebuffer by the light buffer. Light above 256 over-brightens a little.
    pub fn apply_light(&mut self, y_end: i32) {
        let n = (y_end.min(self.h) * self.w) as usize;
        for i in 0..n {
            let l = self.light[i] as u32;
            if l == 256 {
                continue;
            }
            let c = self.px[i];
            let ch = |s: u32| ((((c >> s) & 0xFF) * l) >> 8).min(255);
            self.px[i] = 0xFF00_0000 | (ch(16) << 16) | (ch(8) << 8) | ch(0);
        }
    }

    // ---- text ----

    /// Text with a drop shadow. `sc` is the pixel scale (1 = 6x8 cells).
    pub fn text(&mut self, s: &str, x: i32, y: i32, col: u32, align: Align, sc: i32) {
        let tw = text_width(s, sc);
        let x = match align {
            Align::Left => x,
            Align::Center => x - tw / 2,
            Align::Right => x - tw,
        };
        self.text_raw(s, x + sc, y + sc, BLACK, sc);
        self.text_raw(s, x, y, col, sc);
    }

    fn text_raw(&mut self, s: &str, x: i32, y: i32, col: u32, sc: i32) {
        for (i, ch) in s.chars().enumerate() {
            let Some(g) = glyph(ch) else { continue };
            let cx = x + i as i32 * 6 * sc;
            for (gy, row) in g.iter().enumerate() {
                for (gx, b) in row.bytes().enumerate() {
                    if b == b'#' {
                        self.fill(cx + gx as i32 * sc, y + gy as i32 * sc, sc, sc, col);
                    }
                }
            }
        }
    }
}

pub fn text_width(s: &str, sc: i32) -> i32 {
    s.chars().count() as i32 * 6 * sc
}

fn glyph(c: char) -> Option<&'static [&'static str; 7]> {
    Some(match c.to_ascii_uppercase() {
        'A' => &[" ### ", "#   #", "#   #", "#####", "#   #", "#   #", "#   #"],
        'B' => &["#### ", "#   #", "#   #", "#### ", "#   #", "#   #", "#### "],
        'C' => &[" ### ", "#   #", "#    ", "#    ", "#    ", "#   #", " ### "],
        'D' => &["#### ", "#   #", "#   #", "#   #", "#   #", "#   #", "#### "],
        'E' => &["#####", "#    ", "#    ", "#### ", "#    ", "#    ", "#####"],
        'F' => &["#####", "#    ", "#    ", "#### ", "#    ", "#    ", "#    "],
        'G' => &[" ### ", "#   #", "#    ", "# ###", "#   #", "#   #", " ####"],
        'H' => &["#   #", "#   #", "#   #", "#####", "#   #", "#   #", "#   #"],
        'I' => &[" ### ", "  #  ", "  #  ", "  #  ", "  #  ", "  #  ", " ### "],
        'J' => &["  ###", "   # ", "   # ", "   # ", "   # ", "#  # ", " ##  "],
        'K' => &["#   #", "#  # ", "# #  ", "##   ", "# #  ", "#  # ", "#   #"],
        'L' => &["#    ", "#    ", "#    ", "#    ", "#    ", "#    ", "#####"],
        'M' => &["#   #", "## ##", "# # #", "# # #", "#   #", "#   #", "#   #"],
        'N' => &["#   #", "##  #", "# # #", "#  ##", "#   #", "#   #", "#   #"],
        'O' => &[" ### ", "#   #", "#   #", "#   #", "#   #", "#   #", " ### "],
        'P' => &["#### ", "#   #", "#   #", "#### ", "#    ", "#    ", "#    "],
        'Q' => &[" ### ", "#   #", "#   #", "#   #", "# # #", "#  # ", " ## #"],
        'R' => &["#### ", "#   #", "#   #", "#### ", "# #  ", "#  # ", "#   #"],
        'S' => &[" ####", "#    ", "#    ", " ### ", "    #", "    #", "#### "],
        'T' => &["#####", "  #  ", "  #  ", "  #  ", "  #  ", "  #  ", "  #  "],
        'U' => &["#   #", "#   #", "#   #", "#   #", "#   #", "#   #", " ### "],
        'V' => &["#   #", "#   #", "#   #", "#   #", "#   #", " # # ", "  #  "],
        'W' => &["#   #", "#   #", "#   #", "# # #", "# # #", "## ##", "#   #"],
        'X' => &["#   #", "#   #", " # # ", "  #  ", " # # ", "#   #", "#   #"],
        'Y' => &["#   #", "#   #", " # # ", "  #  ", "  #  ", "  #  ", "  #  "],
        'Z' => &["#####", "    #", "   # ", "  #  ", " #   ", "#    ", "#####"],
        '0' => &[" ### ", "#   #", "#  ##", "# # #", "##  #", "#   #", " ### "],
        '1' => &["  #  ", " ##  ", "  #  ", "  #  ", "  #  ", "  #  ", " ### "],
        '2' => &[" ### ", "#   #", "    #", "   # ", "  #  ", " #   ", "#####"],
        '3' => &["#####", "   # ", "  #  ", "   # ", "    #", "#   #", " ### "],
        '4' => &["   # ", "  ## ", " # # ", "#  # ", "#####", "   # ", "   # "],
        '5' => &["#####", "#    ", "#### ", "    #", "    #", "#   #", " ### "],
        '6' => &["  ## ", " #   ", "#    ", "#### ", "#   #", "#   #", " ### "],
        '7' => &["#####", "    #", "   # ", "  #  ", " #   ", " #   ", " #   "],
        '8' => &[" ### ", "#   #", "#   #", " ### ", "#   #", "#   #", " ### "],
        '9' => &[" ### ", "#   #", "#   #", " ####", "    #", "   # ", " ##  "],
        ':' => &["     ", "  #  ", "  #  ", "     ", "  #  ", "  #  ", "     "],
        '-' => &["     ", "     ", "     ", " ### ", "     ", "     ", "     "],
        '.' => &["     ", "     ", "     ", "     ", "     ", "  ## ", "  ## "],
        ',' => &["     ", "     ", "     ", "     ", "  ## ", "   # ", "  #  "],
        '!' => &["  #  ", "  #  ", "  #  ", "  #  ", "  #  ", "     ", "  #  "],
        '?' => &[" ### ", "#   #", "    #", "   # ", "  #  ", "     ", "  #  "],
        '/' => &["    #", "    #", "   # ", "  #  ", " #   ", "#    ", "#    "],
        '(' => &["   # ", "  #  ", " #   ", " #   ", " #   ", "  #  ", "   # "],
        ')' => &[" #   ", "  #  ", "   # ", "   # ", "   # ", "  #  ", " #   "],
        '>' => &[" #   ", "  #  ", "   # ", "    #", "   # ", "  #  ", " #   "],
        '<' => &["   # ", "  #  ", " #   ", "#    ", " #   ", "  #  ", "   # "],
        '+' => &["     ", "  #  ", "  #  ", "#####", "  #  ", "  #  ", "     "],
        '\'' => &["  #  ", "  #  ", "     ", "     ", "     ", "     ", "     "],
        _ => return None,
    })
}

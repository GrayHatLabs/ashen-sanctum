//! Isometric projection. World coordinates are in tiles (f32); a tile is a 32x16
//! diamond on screen. World +x runs down-right on screen, world +y runs down-left.

pub const TW: f32 = 32.0;
pub const TH: f32 = 16.0;

/// World -> screen offset relative to the camera origin (pixels).
pub fn to_screen(x: f32, y: f32) -> (f32, f32) {
    ((x - y) * TW * 0.5, (x + y) * TH * 0.5)
}

/// Screen offset relative to the camera origin -> world.
pub fn to_world(sx: f32, sy: f32) -> (f32, f32) {
    let a = sx / (TW * 0.5);
    let b = sy / (TH * 0.5);
    ((a + b) * 0.5, (b - a) * 0.5)
}

/// A screen-space direction (e.g. a stick) converted to a world-space direction.
pub fn screen_dir_to_world(ux: f32, uy: f32) -> (f32, f32) {
    // (ux, uy) is a direction in screen pixels; pushing the stick right moves right on screen.
    let (x, y) = to_world(ux, uy);
    let l = (x * x + y * y).sqrt();
    if l < 1e-6 {
        (0.0, 0.0)
    } else {
        (x / l, y / l)
    }
}

/// One of the 8 sprite directions for a world-space facing vector, in PixelLab order:
/// 0 south, 1 south-east, 2 east, 3 north-east, 4 north, 5 north-west, 6 west, 7 south-west
/// (south = towards the viewer / down the screen).
pub fn dir8(dx: f32, dy: f32) -> usize {
    let (sx, sy) = to_screen(dx, dy);
    let (sx, sy) = (sx, sy * 2.0); // undo the squash so diagonals split evenly
    if sx.abs() + sy.abs() < 1e-6 {
        return 0;
    }
    let a = sy.atan2(sx).to_degrees(); // 90 = down
    let k = ((90.0 - a) / 45.0).round() as i32;
    k.rem_euclid(8) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projection_round_trips() {
        for (x, y) in [(0.0, 0.0), (3.5, -2.0), (10.25, 7.75)] {
            let (sx, sy) = to_screen(x, y);
            let (wx, wy) = to_world(sx, sy);
            assert!((wx - x).abs() < 1e-4 && (wy - y).abs() < 1e-4);
        }
    }

    #[test]
    fn screen_directions_map_to_sprite_directions() {
        // Stick/screen direction -> world -> sprite direction should match the screen direction.
        let cases = [((0.0, 1.0), 0), ((1.0, 0.0), 2), ((0.0, -1.0), 4), ((-1.0, 0.0), 6)];
        for ((ux, uy), want) in cases {
            let (dx, dy) = screen_dir_to_world(ux, uy);
            assert_eq!(dir8(dx, dy), want, "screen dir {ux},{uy}");
            let (sx, sy) = to_screen(dx, dy);
            assert!(sx * ux + sy * uy > 0.0, "moves the wrong way on screen");
        }
        // World axes are the diagonals.
        assert_eq!(dir8(1.0, 0.0), 1); // +x runs down-right: south-east
        assert_eq!(dir8(0.0, 1.0), 7); // +y runs down-left: south-west
    }
}

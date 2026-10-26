// A braille dot grid.
//
// Each terminal cell holds a 2x4 block of dots (U+2800 + a bitmask), so a
// modest 48x10 cell panel is a 96x40 pixel canvas — enough for a face that
// actually emotes rather than a pile of ASCII punctuation. This is the whole
// reason PARKER.md wanted a custom TUI instead of composing existing ones.
//
// Hand-rolled rather than using ratatui's Canvas: the shapes here are filled
// blobs in pixel space, and a `set(x, y)` grid expresses that more directly
// than a plotting API with its own coordinate system. ratatui arrives when
// there is an interactive layout to manage, which is not yet.

/// Dot bit for a given position within a cell. Braille numbers its dots
///   1 4
///   2 5
///   3 6
///   7 8
/// which is why the fourth row is not simply the next two bits.
const BITS: [[u8; 2]; 4] = [[0x01, 0x08], [0x02, 0x10], [0x04, 0x20], [0x40, 0x80]];

pub struct Grid {
    /// Width and height in *dots*, not cells.
    pub w: i32,
    pub h: i32,
    dots: Vec<bool>,
    /// Which ink each dot belongs to. A terminal cell can only carry one
    /// colour, so this is resolved per cell at render time by majority — at
    /// 2x4 dots that is accurate enough, and it means an accent element
    /// (a red pupil, a green check) can sit inside an otherwise plain face
    /// without a second drawing pass.
    inks: Vec<u8>,
    ink: u8,
}

impl Grid {
    /// Sized in terminal cells; the dot canvas is 2x wide and 4x tall.
    pub fn new(cols: i32, rows: i32) -> Grid {
        let (w, h) = (cols * 2, rows * 4);
        Grid {
            w,
            h,
            dots: vec![false; (w * h) as usize],
            inks: vec![0u8; (w * h) as usize],
            ink: 0,
        }
    }

    /// Everything drawn from here on belongs to this ink.
    pub fn use_ink(&mut self, ink: u8) {
        self.ink = ink;
    }

    /// Per-cell ink, row-major, chosen by majority of the lit dots in each
    /// cell. Cells with nothing lit report 0.
    pub fn cell_inks(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(((self.w / 2) * (self.h / 4)) as usize);
        for row in 0..self.h / 4 {
            for col in 0..self.w / 2 {
                let mut tally = [0u32; 8];
                for dy in 0..4 {
                    for dx in 0..2 {
                        let (x, y) = (col * 2 + dx, row * 4 + dy);
                        if self.get(x, y) {
                            tally[(self.inks[(y * self.w + x) as usize] as usize).min(7)] += 1;
                        }
                    }
                }
                let best = tally
                    .iter()
                    .enumerate()
                    .max_by_key(|(_, n)| **n)
                    .map(|(i, n)| if *n == 0 { 0 } else { i as u8 })
                    .unwrap_or(0);
                out.push(best);
            }
        }
        out
    }

    /// Out-of-bounds writes are dropped rather than clamped. Clamping would
    /// smear a shape along the edge and quietly ruin the drawing; dropping
    /// lets a face be drawn larger than the panel and simply be cropped.
    pub fn set(&mut self, x: i32, y: i32) {
        if x >= 0 && y >= 0 && x < self.w && y < self.h {
            let i = (y * self.w + x) as usize;
            self.dots[i] = true;
            self.inks[i] = self.ink;
        }
    }

    /// A thick straight stroke, for glyphs — a check, a cross, a bar. Drawn as
    /// a swept disc so thickness is uniform at any angle, unlike a Bresenham
    /// line thickened per step.
    pub fn stroke(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, weight: f64) {
        if weight <= 0.0 {
            return;
        }
        let (dx, dy) = (x1 - x0, y1 - y0);
        let len = (dx * dx + dy * dy).sqrt();
        let steps = (len.ceil() as i32 * 2).max(1);
        let r = (weight / 2.0).max(0.5);
        for i in 0..=steps {
            let t = i as f64 / steps as f64;
            self.ellipse(
                (x0 + dx * t).round() as i32,
                (y0 + dy * t).round() as i32,
                r.round().max(1.0) as i32,
                r.round().max(1.0) as i32,
            );
        }
    }

    pub fn get(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < self.w && y < self.h && self.dots[(y * self.w + x) as usize]
    }

    /// Filled ellipse centred on (cx, cy) with the given radii.
    pub fn ellipse(&mut self, cx: i32, cy: i32, rx: i32, ry: i32) {
        if rx <= 0 || ry <= 0 {
            return;
        }
        for y in -ry..=ry {
            for x in -rx..=rx {
                let (fx, fy) = (x as f64 / rx as f64, y as f64 / ry as f64);
                if fx * fx + fy * fy <= 1.0 {
                    self.set(cx + x, cy + y);
                }
            }
        }
    }

    /// Ellipse outline, `thickness` dots wide, drawn inward from the edge.
    pub fn ring(&mut self, cx: i32, cy: i32, rx: i32, ry: i32, thickness: i32) {
        if rx <= 0 || ry <= 0 {
            return;
        }
        let (ix, iy) = ((rx - thickness).max(0), (ry - thickness).max(0));
        for y in -ry..=ry {
            for x in -rx..=rx {
                let outside = {
                    let (fx, fy) = (x as f64 / rx as f64, y as f64 / ry as f64);
                    fx * fx + fy * fy <= 1.0
                };
                let inside = ix > 0 && iy > 0 && {
                    let (fx, fy) = (x as f64 / ix as f64, y as f64 / iy as f64);
                    fx * fx + fy * fy <= 1.0
                };
                if outside && !inside {
                    self.set(cx + x, cy + y);
                }
            }
        }
    }

    /// Erase a filled ellipse. Needed because a pupil is a *hole* — the place
    /// the eye is not — and drawing one as a dot on top would read as a mole.
    pub fn clear_ellipse(&mut self, cx: f64, cy: f64, rx: f64, ry: f64) {
        if rx <= 0.0 || ry <= 0.0 {
            return;
        }
        let (rxi, ryi) = (rx.ceil() as i32, ry.ceil() as i32);
        for y in -ryi..=ryi {
            for x in -rxi..=rxi {
                let (fx, fy) = (x as f64 / rx, y as f64 / ry);
                if fx * fx + fy * fy <= 1.0 {
                    let (px, py) = ((cx.round() as i32) + x, (cy.round() as i32) + y);
                    if px >= 0 && py >= 0 && px < self.w && py < self.h {
                        self.dots[(py * self.w + px) as usize] = false;
                    }
                }
            }
        }
    }

    pub fn to_lines(&self) -> Vec<String> {
        (0..self.h / 4)
            .map(|row| {
                (0..self.w / 2)
                    .map(|col| {
                        let mut mask = 0u8;
                        for (dy, bits) in BITS.iter().enumerate() {
                            for (dx, bit) in bits.iter().enumerate() {
                                if self.get(col * 2 + dx as i32, row * 4 + dy as i32) {
                                    mask |= bit;
                                }
                            }
                        }
                        // U+2800 is blank braille; a real space collapses in
                        // some terminals and breaks alignment.
                        char::from_u32(0x2800 + mask as u32).unwrap_or(' ')
                    })
                    .collect()
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_grid_is_dots_not_cells() {
        let g = Grid::new(10, 3);
        assert_eq!((g.w, g.h), (20, 12));
        assert_eq!(g.to_lines().len(), 3);
        assert_eq!(g.to_lines()[0].chars().count(), 10);
    }

    #[test]
    fn an_empty_grid_is_blank_braille_not_spaces() {
        // U+2800 keeps column alignment where a space can collapse.
        let g = Grid::new(3, 1);
        assert_eq!(g.to_lines()[0], "\u{2800}\u{2800}\u{2800}");
    }

    #[test]
    fn every_dot_position_maps_to_a_distinct_bit() {
        let mut seen = std::collections::HashSet::new();
        for y in 0..4 {
            for x in 0..2 {
                let mut g = Grid::new(1, 1);
                g.set(x, y);
                let ch = g.to_lines()[0].chars().next().unwrap();
                assert!(seen.insert(ch), "dot ({x},{y}) collided with another");
                assert!(('\u{2800}'..='\u{28FF}').contains(&ch));
            }
        }
        assert_eq!(seen.len(), 8);
    }

    #[test]
    fn all_eight_dots_set_is_the_full_cell() {
        let mut g = Grid::new(1, 1);
        for y in 0..4 {
            for x in 0..2 {
                g.set(x, y);
            }
        }
        assert_eq!(g.to_lines()[0], "\u{28FF}");
    }

    #[test]
    fn out_of_bounds_is_dropped_not_clamped() {
        // Clamping would smear a shape along the edge.
        let mut g = Grid::new(2, 1);
        g.set(-5, 0);
        g.set(0, -5);
        g.set(999, 0);
        assert_eq!(g.to_lines()[0], "\u{2800}\u{2800}");
    }

    #[test]
    fn an_ellipse_is_filled_and_centred() {
        let mut g = Grid::new(20, 5);
        g.ellipse(20, 10, 6, 4);
        assert!(g.get(20, 10), "centre");
        assert!(g.get(20, 6) && g.get(20, 14), "vertical extremes");
        assert!(g.get(14, 10) && g.get(26, 10), "horizontal extremes");
        assert!(!g.get(27, 10), "should stop at the radius");
    }

    #[test]
    fn a_ring_is_hollow() {
        let mut g = Grid::new(20, 6);
        g.ring(20, 12, 10, 10, 2);
        assert!(!g.get(20, 12), "centre must be empty");
        assert!(g.get(20, 2), "top edge drawn");
    }

    #[test]
    fn clearing_carves_a_hole_without_touching_its_surroundings() {
        // How a pupil works: the eye is drawn, then unset.
        let mut g = Grid::new(20, 6);
        g.ellipse(20, 12, 10, 10);
        g.clear_ellipse(20.0, 12.0, 3.0, 3.0);
        assert!(!g.get(20, 12), "centre carved out");
        assert!(g.get(20, 5), "the rest of the shape survives");
    }
}

use crate::grayscott::Grid;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Palette {
    Light,
    Dark,
}

impl Palette {
    /// (background, foreground) as RGB, taken from the site's design tokens
    /// so the demo belongs to the page rather than sitting on it.
    fn ends(self) -> ([u8; 3], [u8; 3]) {
        match self {
            // paper #FBFAF8 → accent #A8431E
            Palette::Light => ([0xFB, 0xFA, 0xF8], [0xA8, 0x43, 0x1E]),
            // paper #0E0F11 → accent #E0764A
            Palette::Dark => ([0x0E, 0x0F, 0x11], [0xE0, 0x76, 0x4A]),
        }
    }
}

fn lerp(a: u8, b: u8, t: f32) -> u8 {
    (f32::from(a) + (f32::from(b) - f32::from(a)) * t).round() as u8
}

/// Paint B concentration into an RGBA buffer laid out for `ImageData`.
pub fn paint(grid: &Grid, palette: Palette, out: &mut [u8]) {
    let (bg, fg) = palette.ends();
    for y in 0..grid.height() {
        for x in 0..grid.width() {
            // B rarely exceeds ~0.4, so scale before clamping or the image
            // stays nearly background-coloured.
            let t = (grid.b_at(x, y) * 2.5).clamp(0.0, 1.0);
            let i = (y * grid.width() + x) * 4;
            out[i] = lerp(bg[0], fg[0], t);
            out[i + 1] = lerp(bg[1], fg[1], t);
            out[i + 2] = lerp(bg[2], fg[2], t);
            out[i + 3] = 255;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paint_fills_every_pixel_opaque() {
        let g = Grid::new(8, 8);
        let mut buf = vec![0u8; 8 * 8 * 4];
        paint(&g, Palette::Light, &mut buf);
        for px in buf.chunks_exact(4) {
            assert_eq!(px[3], 255, "alpha must be opaque");
        }
    }

    #[test]
    fn empty_grid_paints_the_background_colour() {
        let g = Grid::new(4, 4);
        let mut buf = vec![0u8; 4 * 4 * 4];
        paint(&g, Palette::Light, &mut buf);
        // B = 0 everywhere, so every pixel is the light paper colour #FBFAF8.
        assert_eq!(&buf[0..3], &[0xFB, 0xFA, 0xF8]);
    }

    #[test]
    fn seeded_cells_differ_from_background() {
        let mut g = Grid::new(8, 8);
        g.seed_rect(4, 4, 0);
        let mut buf = vec![0u8; 8 * 8 * 4];
        paint(&g, Palette::Light, &mut buf);
        let bg = &buf[0..3];
        let seeded = &buf[(4 * 8 + 4) * 4..(4 * 8 + 4) * 4 + 3];
        assert_ne!(bg, seeded, "seeded cell rendered identically to background");
    }

    #[test]
    fn palettes_differ() {
        let g = Grid::new(4, 4);
        let mut light = vec![0u8; 4 * 4 * 4];
        let mut dark = vec![0u8; 4 * 4 * 4];
        paint(&g, Palette::Light, &mut light);
        paint(&g, Palette::Dark, &mut dark);
        assert_ne!(light, dark, "light and dark rendered identically");
    }
}

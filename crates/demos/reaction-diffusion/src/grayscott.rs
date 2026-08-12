//! Gray-Scott reaction-diffusion, using Karl Sims' formulation.
//!
//! Deliberately free of any WebAssembly dependency so the physics can be
//! tested natively rather than through a headless browser.

const DA: f32 = 1.0;
const DB: f32 = 0.5;
const DT: f32 = 1.0;

/// Two chemical concentration fields on a toroidal grid.
pub struct Grid {
    width: usize,
    height: usize,
    a: Vec<f32>,
    b: Vec<f32>,
    a_next: Vec<f32>,
    b_next: Vec<f32>,
}

impl Grid {
    pub fn new(width: usize, height: usize) -> Self {
        let n = width * height;
        Self {
            width,
            height,
            a: vec![1.0; n],
            b: vec![0.0; n],
            a_next: vec![1.0; n],
            b_next: vec![0.0; n],
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn b_at(&self, x: usize, y: usize) -> f32 {
        self.b[y * self.width + x]
    }

    pub fn reset(&mut self) {
        self.a.fill(1.0);
        self.b.fill(0.0);
    }

    /// Fill a square of B, the disturbance the patterns grow from.
    pub fn seed_rect(&mut self, cx: usize, cy: usize, half: usize) {
        for dy in -(half as isize)..=(half as isize) {
            for dx in -(half as isize)..=(half as isize) {
                let x = (cx as isize + dx).rem_euclid(self.width as isize) as usize;
                let y = (cy as isize + dy).rem_euclid(self.height as isize) as usize;
                self.b[y * self.width + x] = 1.0;
            }
        }
    }

    /// Weighted 3×3 Laplacian with wrap-around, per Karl Sims: centre −1,
    /// edge-adjacent 0.2, diagonal 0.05.
    fn laplace(&self, field: &[f32], x: usize, y: usize) -> f32 {
        let w = self.width as isize;
        let h = self.height as isize;
        let at = |dx: isize, dy: isize| -> f32 {
            let nx = (x as isize + dx).rem_euclid(w) as usize;
            let ny = (y as isize + dy).rem_euclid(h) as usize;
            field[ny * self.width + nx]
        };
        -field[y * self.width + x]
            + 0.2 * (at(-1, 0) + at(1, 0) + at(0, -1) + at(0, 1))
            + 0.05 * (at(-1, -1) + at(1, -1) + at(-1, 1) + at(1, 1))
    }

    pub fn step(&mut self, feed: f32, kill: f32) {
        for y in 0..self.height {
            for x in 0..self.width {
                let i = y * self.width + x;
                let a = self.a[i];
                let b = self.b[i];
                let abb = a * b * b;
                self.a_next[i] =
                    (a + (DA * self.laplace(&self.a, x, y) - abb + feed * (1.0 - a)) * DT)
                        .clamp(0.0, 1.0);
                self.b_next[i] =
                    (b + (DB * self.laplace(&self.b, x, y) + abb - (kill + feed) * b) * DT)
                        .clamp(0.0, 1.0);
            }
        }
        std::mem::swap(&mut self.a, &mut self.a_next);
        std::mem::swap(&mut self.b, &mut self.b_next);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A = 1, B = 0 everywhere is an exact fixed point: the reaction term
    /// A·B² is zero, feed f·(1 − A) is zero, kill (k + f)·B is zero, and the
    /// Laplacian of a uniform field is zero. If stepping perturbs it, the
    /// update rule is wrong.
    #[test]
    fn the_empty_state_is_a_fixed_point() {
        let mut g = Grid::new(16, 16);
        for _ in 0..50 {
            g.step(0.0545, 0.0620);
        }
        for y in 0..g.height() {
            for x in 0..g.width() {
                assert!(g.b_at(x, y).abs() < 1e-6, "B drifted at ({x},{y})");
            }
        }
    }

    #[test]
    fn seeding_introduces_b_and_it_spreads() {
        let mut g = Grid::new(32, 32);
        g.seed_rect(16, 16, 2);
        assert!(g.b_at(16, 16) > 0.4, "seed did not take");
        assert_eq!(g.b_at(0, 0), 0.0, "seed leaked across the grid");

        for _ in 0..200 {
            g.step(0.0545, 0.0620);
        }
        let spread = (10..22)
            .flat_map(|y| (10..22).map(move |x| (x, y)))
            .filter(|&(x, y)| g.b_at(x, y) > 0.01)
            .count();
        assert!(spread > 16, "B did not diffuse outward, only {spread} cells");
    }

    #[test]
    fn values_stay_within_bounds() {
        let mut g = Grid::new(24, 24);
        g.seed_rect(12, 12, 3);
        for _ in 0..500 {
            g.step(0.0545, 0.0620);
        }
        for y in 0..g.height() {
            for x in 0..g.width() {
                let b = g.b_at(x, y);
                assert!((0.0..=1.0).contains(&b), "B out of range at ({x},{y}): {b}");
                assert!(b.is_finite(), "B diverged at ({x},{y})");
            }
        }
    }

    #[test]
    fn the_grid_wraps_rather_than_clamping() {
        // Seeding at the right edge must influence the left edge, which only
        // happens if the Laplacian is toroidal.
        let mut g = Grid::new(16, 16);
        g.seed_rect(15, 8, 1);
        for _ in 0..100 {
            g.step(0.0545, 0.0620);
        }
        assert!(g.b_at(0, 8) > 0.001, "no wrap-around diffusion");
    }

    #[test]
    fn simulation_is_deterministic() {
        let run = || {
            let mut g = Grid::new(16, 16);
            g.seed_rect(8, 8, 2);
            for _ in 0..100 {
                g.step(0.0545, 0.0620);
            }
            (0..16).map(|i| g.b_at(i, i)).collect::<Vec<_>>()
        };
        assert_eq!(run(), run(), "same input produced different output");
    }

    #[test]
    fn reset_restores_the_initial_state() {
        let mut g = Grid::new(16, 16);
        g.seed_rect(8, 8, 2);
        g.step(0.0545, 0.0620);
        g.reset();
        for y in 0..16 {
            for x in 0..16 {
                assert_eq!(g.b_at(x, y), 0.0, "reset left B behind at ({x},{y})");
            }
        }
    }
}

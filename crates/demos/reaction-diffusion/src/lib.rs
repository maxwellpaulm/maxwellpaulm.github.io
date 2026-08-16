mod grayscott;
mod render;

use grayscott::Grid;
use render::{paint, Palette};
use wasm_bindgen::prelude::*;

/// The simulation as JavaScript sees it.
///
/// Pixels live in wasm linear memory; JS reads them through a view rather
/// than copying, so a frame costs one `putImageData` and no marshalling.
#[wasm_bindgen]
pub struct Simulation {
    grid: Grid,
    pixels: Vec<u8>,
}

#[wasm_bindgen]
impl Simulation {
    #[wasm_bindgen(constructor)]
    pub fn new(width: usize, height: usize) -> Simulation {
        Simulation { grid: Grid::new(width, height), pixels: vec![0; width * height * 4] }
    }

    /// Grid width in cells.
    pub fn width(&self) -> usize {
        self.grid.width()
    }

    /// Grid height in cells.
    pub fn height(&self) -> usize {
        self.grid.height()
    }

    /// Advance `substeps` iterations. More substeps per frame means faster
    /// pattern evolution without a higher frame rate.
    pub fn step(&mut self, feed: f32, kill: f32, substeps: u32) {
        for _ in 0..substeps {
            self.grid.step(feed, kill);
        }
    }

    pub fn render(&mut self, dark: bool) {
        let palette = if dark { Palette::Dark } else { Palette::Light };
        paint(&self.grid, palette, &mut self.pixels);
    }

    /// Seed a square of reagent B centered at `(x, y)` with half-width `half`.
    pub fn seed(&mut self, x: usize, y: usize, half: usize) {
        self.grid.seed_rect(x, y, half);
    }

    /// Clear the grid back to its initial (unseeded) state.
    pub fn reset(&mut self) {
        self.grid.reset();
    }

    /// Pointer to the RGBA pixel buffer in wasm linear memory.
    ///
    /// The caller MUST rebuild its typed-array view from this pointer on
    /// every frame and never cache it: if wasm memory grows, the backing
    /// `ArrayBuffer` is detached and replaced, and a retained view reads
    /// garbage or throws. `pixels` is allocated once and never resized, so
    /// growth is unlikely here — but the rule costs nothing to follow and
    /// the failure mode is silent.
    pub fn pixels_ptr(&self) -> *const u8 {
        self.pixels.as_ptr()
    }

    /// Length in bytes of the buffer at `pixels_ptr`: `width * height * 4`.
    pub fn pixels_len(&self) -> usize {
        self.pixels.len()
    }
}

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
        Simulation {
            grid: Grid::new(width, height),
            pixels: vec![0; width * height * 4],
        }
    }

    pub fn width(&self) -> usize {
        self.grid.width()
    }

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

    pub fn seed(&mut self, x: usize, y: usize, half: usize) {
        self.grid.seed_rect(x, y, half);
    }

    pub fn reset(&mut self) {
        self.grid.reset();
    }

    pub fn pixels_ptr(&self) -> *const u8 {
        self.pixels.as_ptr()
    }

    pub fn pixels_len(&self) -> usize {
        self.pixels.len()
    }
}

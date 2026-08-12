# Bucket 4: Reaction-Diffusion WASM Demo — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship the first WebAssembly on paul-maxwell.com — an interactive Gray-Scott reaction-diffusion simulation running at 60fps on a canvas, loaded only on its own page.

**Architecture:** The simulation is a pure Rust crate with no WebAssembly dependency, so the physics is unit-tested natively with `cargo test` on the host. A thin `#[wasm_bindgen]` wrapper exposes it to the browser, and a small hand-written JS module loads the wasm and drives the animation loop. The generator gains a `Demos` route and one demo page; every other page ships zero WebAssembly.

**Tech Stack:** Rust 1.92, `wasm-bindgen` 0.2.127, `js-sys` 0.3.104, `web-sys` 0.3.104, `maud` 0.27. No bundler, no npm, no framework.

## Global Constraints

Every task's requirements implicitly include this section.

- **No threads.** GitHub Pages cannot set `Cross-Origin-Opener-Policy` / `Cross-Origin-Embedder-Policy`, so `SharedArrayBuffer` is unavailable and `rayon` cannot be used. The simulation must be single-threaded.
- **No external hosts.** No CDN, no npm, no remote fonts or scripts. Everything is self-hosted.
- **WASM loads only on the demo page.** Reading the bio, About, Projects, or Resume must download zero WebAssembly bytes. The loader is lazy and page-scoped.
- **Zero `#[allow(dead_code)]` anywhere in the tree.** There are none today and none may be added.
- **CI gates deploy on `cargo clippy --all-targets -- -D warnings` and `cargo test --all`.** A red build must never publish.
- **Light is the default theme** for all visitors; dark is opt-in via `data-theme` + `localStorage`. The demo must be legible in both.
- **The inline theme scripts use double-quoted JS deliberately** so their un-escaped tests are falsifying. Do not convert them to single quotes.
- **`content/site.toml` prose is approved and fixed.** Do not reword existing copy.
- **Motion is capped at 180ms and `prefers-reduced-motion` is honoured.** The demo animation must not auto-run for a visitor who has asked for reduced motion — see Task 5.
- **Colour tokens:** light paper `#FBFAF8`, ink `#14161A`, muted `#6E7076`, rule `#E5E2DC`, accent `#A8431E`; dark paper `#0E0F11`, surface `#16181B`, ink `#E9E7E2`, muted `#94989F`, rule `#25282D`, accent `#E0764A`.

**Simulation parameters** (Karl Sims' formulation, the standard reference):

- Laplacian kernel: centre `-1.0`, edge-adjacent `0.2`, diagonal `0.05`
- Diffusion rates: `dA = 1.0`, `dB = 0.5`; timestep `dt = 1.0`
- Presets (feed `f`, kill `k`): coral `0.0545`/`0.0620`; mitosis `0.0367`/`0.0649`; solitons `0.0300`/`0.0620`; worms `0.0780`/`0.0610`
- Grid wraps at the edges (toroidal), so there is no boundary special case

**Deviation from the original spec, flagged:** spec §5.1 anticipated `crates/demos/<name>/`. This plan uses `crates/demos/reaction-diffusion/` with the simulation split into a dependency-free module, which the spec did not anticipate but which is what makes the physics testable without a headless browser.

---

### Task 1: The simulation core, with no WebAssembly in sight

**Files:**
- Create: `crates/demos/reaction-diffusion/Cargo.toml`, `crates/demos/reaction-diffusion/src/grayscott.rs`, `crates/demos/reaction-diffusion/src/lib.rs`
- Modify: `Cargo.toml` (workspace members)

**Interfaces:**
- Consumes: nothing.
- Produces: `grayscott::Grid` with `Grid::new(width: usize, height: usize) -> Grid`, `Grid::step(&mut self, feed: f32, kill: f32)`, `Grid::seed_rect(&mut self, cx: usize, cy: usize, half: usize)`, `Grid::reset(&mut self)`, `Grid::b_at(&self, x: usize, y: usize) -> f32`, `Grid::width()`, `Grid::height()`. Tasks 2 and 3 build on these.

This task deliberately contains no `wasm-bindgen`. Keeping the physics in a plain Rust module is what lets `cargo test` verify it on the host — testing it through WebAssembly would need a headless browser and would make failures far harder to read.

- [ ] **Step 1: Add the crate to the workspace**

In the root `Cargo.toml`, change the members list to:

```toml
[workspace]
members = ["crates/site", "crates/demos/reaction-diffusion"]
resolver = "2"
```

Create `crates/demos/reaction-diffusion/Cargo.toml`:

```toml
[package]
name = "reaction-diffusion"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
wasm-bindgen = "0.2.127"
```

`cdylib` produces the `.wasm`; `rlib` is what lets the host test binary link the same code.

- [ ] **Step 2: Write the failing tests**

Create `crates/demos/reaction-diffusion/src/grayscott.rs`:

```rust
//! Gray-Scott reaction-diffusion, using Karl Sims' formulation.
//!
//! Deliberately free of any WebAssembly dependency so the physics can be
//! tested natively rather than through a headless browser.

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
```

Create `crates/demos/reaction-diffusion/src/lib.rs`:

```rust
mod grayscott;
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test -p reaction-diffusion`
Expected: FAIL — `cannot find type 'Grid' in this scope`.

- [ ] **Step 4: Implement the grid**

Add above the `tests` module in `grayscott.rs`:

```rust
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
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p reaction-diffusion`
Expected: PASS, 6 tests.

Then `cargo clippy --all-targets -- -D warnings` — expected clean.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock crates/demos
git commit -m "feat: add Gray-Scott reaction-diffusion simulation core"
```

---

### Task 2: Rendering to a pixel buffer

**Files:**
- Create: `crates/demos/reaction-diffusion/src/render.rs`
- Modify: `crates/demos/reaction-diffusion/src/lib.rs`

**Interfaces:**
- Consumes: `grayscott::Grid` (Task 1).
- Produces: `render::Palette` (enum with variants `Light`, `Dark`), `render::paint(grid: &Grid, palette: Palette, out: &mut [u8])`. Task 3 calls `paint` into a buffer it shares with JavaScript.

The buffer is RGBA, four bytes per cell, matching the browser's `ImageData` layout so the JS side can hand it straight to the canvas with no per-pixel work.

- [ ] **Step 1: Write the failing tests**

Create `crates/demos/reaction-diffusion/src/render.rs`:

```rust
use crate::grayscott::Grid;

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
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p reaction-diffusion render`
Expected: FAIL — `cannot find function 'paint'`.

- [ ] **Step 3: Implement rendering**

Add above the `tests` module in `render.rs`:

```rust
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
```

- [ ] **Step 4: Register the module and run the tests**

`crates/demos/reaction-diffusion/src/lib.rs`:

```rust
mod grayscott;
mod render;
```

Run: `cargo test -p reaction-diffusion` — expected PASS, 10 tests.
Run: `cargo clippy --all-targets -- -D warnings` — expected clean.

- [ ] **Step 5: Commit**

```bash
git add crates/demos/reaction-diffusion/src
git commit -m "feat: render reaction-diffusion state into an RGBA buffer"
```

---

### Task 3: The WebAssembly boundary and build script

**Files:**
- Modify: `crates/demos/reaction-diffusion/src/lib.rs`
- Create: `scripts/build-wasm.sh`

**Interfaces:**
- Consumes: `grayscott::Grid`, `render::{paint, Palette}`.
- Produces: a `Simulation` type exported to JavaScript with `new Simulation(width, height)`, `.step(feed, kill, substeps)`, `.render(dark)`, `.seed(x, y, half)`, `.reset()`, `.pixels_ptr()`, `.pixels_len()`, `.width()`, `.height()`. Task 5's JS module calls exactly these.
- Produces: `scripts/build-wasm.sh`, which Task 7 calls from CI.

- [ ] **Step 1: Install the target and CLI locally**

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.127
```

The CLI version must match the `wasm-bindgen` crate version exactly — a mismatch produces a confusing "schema version mismatch" error at build time rather than at runtime.

- [ ] **Step 2: Write the wasm wrapper**

Replace `crates/demos/reaction-diffusion/src/lib.rs`:

```rust
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
```

- [ ] **Step 3: Write the build script**

Create `scripts/build-wasm.sh` and `chmod +x` it:

```bash
#!/bin/bash
# Builds the reaction-diffusion demo to WebAssembly and emits the JS bindings.
# Run before `cargo run -p site` so the generator can copy the artifacts.
set -euo pipefail

OUT=static/demos/reaction-diffusion

cargo build -p reaction-diffusion --target wasm32-unknown-unknown --release

mkdir -p "$OUT"
wasm-bindgen \
  --target web \
  --no-typescript \
  --out-dir "$OUT" \
  target/wasm32-unknown-unknown/release/reaction_diffusion.wasm

echo "wasm artifacts in $OUT:"
ls -la "$OUT"
```

`--target web` emits an ES module usable directly from a `<script type="module">` with no bundler. `--no-typescript` skips `.d.ts` files we have no use for.

Output lands in `static/`, which the site generator already copies wholesale into `dist/` — so the wasm needs no special handling in the build.

- [ ] **Step 4: Build and verify**

```bash
./scripts/build-wasm.sh
ls -la static/demos/reaction-diffusion/
du -h static/demos/reaction-diffusion/reaction_diffusion_bg.wasm
```

Expected: `reaction_diffusion.js` and `reaction_diffusion_bg.wasm` exist. Record the `.wasm` size in your report — it is the number that decides whether this was worth shipping.

- [ ] **Step 5: Keep build output out of git**

The generated wasm and JS are build artifacts, not source. Append to `.gitignore`:

```
/static/demos/reaction-diffusion/
```

Verify with `git status --short` that nothing under that path is staged.

- [ ] **Step 6: Commit**

```bash
git add crates/demos/reaction-diffusion/src/lib.rs scripts/build-wasm.sh .gitignore
git commit -m "feat: expose the simulation to WebAssembly with a build script"
```

---

### Task 4: The Demos route

**Files:**
- Modify: `crates/site/src/route.rs`, `crates/site/src/components/rail.rs`, `crates/site/src/checks.rs`, `crates/site/src/build.rs`
- Create: `crates/site/src/pages/demos.rs`
- Modify: `crates/site/src/pages/mod.rs`

**Interfaces:**
- Consumes: `Route`, `shell::layout`, `content::Site`.
- Produces: `Route::Demos` (fifth variant, `path()` = `/demos/`, `output_path()` = `demos/index.html`, `label()` = `"Demos"`), and `pages::demos::render(site: &Site) -> Markup`.

This is where the bucket-1 constraint "Demos must not exist" is deliberately lifted. Four places assert it today and all four must change:

- `route.rs:10` — `ALL: [Route; 4]`
- `route.rs:46` — `assert_eq!(Route::ALL.len(), 4, "Demos must not exist until bucket 4")`
- `rail.rs:69` — `assert!(!out.contains("Demos"), "Demos must not be linked in bucket 1")`
- `checks.rs:129` — uses `/demos/` as its *dead link* fixture

That last one is subtle. The test scaffolds its own temp directory, so `/demos/` is still genuinely absent there and the test still passes — but using a path that now exists on the real site as the example of a broken link is actively misleading. Change the fixture to `/nonexistent-page/` and keep the assertion identical.

- [ ] **Step 1: Update the route tests**

In `route.rs`, change the count assertion to:

```rust
        assert_eq!(Route::ALL.len(), 5);
```

and add to the same test module:

```rust
    #[test]
    fn demos_is_routed() {
        assert_eq!(Route::Demos.path(), "/demos/");
        assert_eq!(Route::Demos.output_path(), "demos/index.html");
        assert_eq!(Route::Demos.label(), "Demos");
    }
```

In `rail.rs`, replace the `!out.contains("Demos")` assertion with:

```rust
        assert!(out.contains("Demos"), "Demos must be linked from bucket 4 on");
```

In `checks.rs`, change the dead-link fixture from `/demos/` to `/nonexistent-page/` in both the scaffolded HTML and the assertion that the error names it.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p site`
Expected: FAIL — no `Route::Demos` variant, and the rail assertion fails.

- [ ] **Step 3: Add the route**

In `route.rs`, add `Demos` to the enum after `Resume`, extend `ALL` to `[Route; 5]` including `Route::Demos`, and add match arms: `path()` → `"/demos/"`, `output_path()` → `"demos/index.html"`, `label()` → `"Demos"`.

- [ ] **Step 4: Write the failing page test**

Create `crates/site/src/pages/demos.rs`:

```rust
use crate::components::shell;
use crate::content::Site;
use crate::route::Route;
use maud::{html, Markup};

/// Path of the reaction-diffusion demo page. Task 5 renders it.
pub const REACTION_DIFFUSION: &str = "/demos/reaction-diffusion/";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demos_index_links_to_the_reaction_diffusion_demo() {
        let out = render(&crate::content::fixture_site()).into_string();
        assert!(out.contains(REACTION_DIFFUSION), "demo link missing");
        assert!(out.contains("Reaction-Diffusion"));
        assert!(out.contains(r#"href="/demos/" aria-current="page""#));
    }
}
```

- [ ] **Step 5: Run it to verify it fails, then implement**

Run: `cargo test -p site demos` — expected FAIL, `cannot find function 'render'`.

Add above the `tests` module:

```rust
pub fn render(site: &Site) -> Markup {
    let main = html! {
        h1 { "Demos" }
        p .prose { "Small things built in Rust, compiled to WebAssembly, running in your browser." }
        div .section-head {
            span .mono { "Demos" }
            span .mono { "01" }
        }
        div .item {
            div {
                h3 { a href=(REACTION_DIFFUSION) { "Reaction-Diffusion" } }
                p { "A Gray-Scott simulation: two chemicals, one feeding on the other, painting patterns that look uncannily biological. Every pixel is computed in Rust, sixty times a second." }
            }
            div .mono .year { "2026" }
        }
    };
    shell::layout(site, Route::Demos, "Demos", main)
}
```

- [ ] **Step 6: Wire the route into the build**

Register `pub mod demos;` in `pages/mod.rs`, and add the `Route::Demos => pages::demos::render(&site)` arm to the match in `build.rs`.

Run: `cargo test -p site` and `cargo clippy --all-targets -- -D warnings` — both expected clean. Then `cargo run -p site -- --strict` and confirm `dist/demos/index.html` exists and `dist/sitemap.xml` now lists five URLs.

- [ ] **Step 7: Commit**

```bash
git add crates/site/src
git commit -m "feat: add the Demos route and index page"
```

---

### Task 5: The demo page and its loader

**Files:**
- Create: `crates/site/src/pages/demo_reaction_diffusion.rs`, `static/demos/loader.js`
- Modify: `crates/site/src/pages/mod.rs`, `crates/site/src/build.rs`, `crates/site/src/theme.rs`

**Interfaces:**
- Consumes: `Route::Demos`, `shell::layout`, and the `Simulation` API from Task 3.
- Produces: `pages::demo_reaction_diffusion::render(site: &Site) -> Markup`, written to `demos/reaction-diffusion/index.html`.

The demo page is not a `Route` variant — `Route::ALL` drives the nav, and a second Demos-family entry there would duplicate the nav item. It is written directly, exactly as the 404 page is.

- [ ] **Step 1: Write the loader**

Create `static/demos/loader.js`:

```js
// Lazy loader for the reaction-diffusion demo. Only this page imports it,
// so no other page on the site downloads any WebAssembly.
import init, { Simulation } from "./reaction-diffusion/reaction_diffusion.js";

const PRESETS = {
  coral:     { feed: 0.0545, kill: 0.0620 },
  mitosis:   { feed: 0.0367, kill: 0.0649 },
  solitons:  { feed: 0.0300, kill: 0.0620 },
  worms:     { feed: 0.0780, kill: 0.0610 },
};

async function main() {
  const canvas = document.getElementById("rd-canvas");
  const status = document.getElementById("rd-status");
  if (!canvas) return;

  const wasm = await init();
  const sim = new Simulation(220, 140);
  const w = sim.width(), h = sim.height();
  canvas.width = w;
  canvas.height = h;

  const ctx = canvas.getContext("2d");
  const image = ctx.createImageData(w, h);

  let preset = PRESETS.coral;
  let running = false;
  let frames = 0, fpsAt = performance.now();

  const isDark = () => document.documentElement.dataset.theme === "dark";

  function seedCentre() {
    sim.reset();
    for (let i = 0; i < 12; i++) {
      const x = Math.floor(w / 2 + (Math.random() - 0.5) * 40);
      const y = Math.floor(h / 2 + (Math.random() - 0.5) * 40);
      sim.seed(x, y, 3);
    }
  }

  function draw() {
    sim.render(isDark());
    const px = new Uint8ClampedArray(wasm.memory.buffer, sim.pixels_ptr(), sim.pixels_len());
    image.data.set(px);
    ctx.putImageData(image, 0, 0);
  }

  function frame() {
    if (!running) return;
    sim.step(preset.feed, preset.kill, 8);
    draw();
    frames++;
    const now = performance.now();
    if (now - fpsAt >= 1000) {
      status.textContent = frames + " fps";
      frames = 0;
      fpsAt = now;
    }
    requestAnimationFrame(frame);
  }

  function start() {
    if (running) return;
    running = true;
    status.textContent = "running";
    requestAnimationFrame(frame);
  }

  function stop() {
    running = false;
    status.textContent = "paused";
  }

  canvas.addEventListener("pointerdown", (e) => {
    const r = canvas.getBoundingClientRect();
    const x = Math.floor((e.clientX - r.left) / r.width * w);
    const y = Math.floor((e.clientY - r.top) / r.height * h);
    sim.seed(x, y, 4);
    if (!running) draw();
  });

  document.getElementById("rd-toggle").addEventListener("click", () => {
    running ? stop() : start();
  });
  document.getElementById("rd-reset").addEventListener("click", () => {
    seedCentre();
    draw();
  });
  document.querySelectorAll("[data-preset]").forEach((b) => {
    b.addEventListener("click", () => {
      preset = PRESETS[b.dataset.preset];
      seedCentre();
      if (!running) draw();
    });
  });

  seedCentre();
  draw();

  // Respect a reduced-motion preference: render one frame and wait to be
  // asked rather than animating unbidden.
  if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
    status.textContent = "paused — reduced motion";
  } else {
    start();
  }
}

main();
```

- [ ] **Step 2: Write the failing page test**

Create `crates/site/src/pages/demo_reaction_diffusion.rs`:

```rust
use crate::components::shell;
use crate::content::Site;
use crate::route::Route;
use maud::{html, Markup};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_page_has_a_canvas_and_loads_the_module_lazily() {
        let out = render(&crate::content::fixture_site()).into_string();
        assert!(out.contains(r#"id="rd-canvas""#), "canvas missing");
        assert!(out.contains(r#"type="module""#), "module script missing");
        assert!(out.contains("/demos/loader.js"), "loader not referenced");
        assert!(out.contains(r#"data-preset="coral""#), "presets missing");
        assert!(out.contains("noscript"), "no fallback for JS-disabled visitors");
    }

    #[test]
    fn demo_page_marks_demos_as_the_current_nav_item() {
        let out = render(&crate::content::fixture_site()).into_string();
        assert!(out.contains(r#"href="/demos/" aria-current="page""#));
    }
}
```

- [ ] **Step 3: Run it to verify it fails, then implement**

Run: `cargo test -p site demo_page` — expected FAIL.

Add above the `tests` module:

```rust
pub fn render(site: &Site) -> Markup {
    let main = html! {
        h1 { "Reaction-Diffusion" }
        p .prose {
            "Two chemicals diffuse across a grid at different rates; one converts the other on contact. "
            "That single rule, iterated, produces patterns that look like coral, cell division, or fingerprints. "
            "The whole simulation runs in Rust compiled to WebAssembly — click the canvas to disturb it."
        }

        canvas #rd-canvas .rd-canvas {}

        div .rd-controls {
            button #rd-toggle .theme-toggle type="button" { "Pause" }
            button #rd-reset .theme-toggle type="button" { "Reset" }
            @for (id, label) in [("coral", "Coral"), ("mitosis", "Mitosis"), ("solitons", "Solitons"), ("worms", "Worms")] {
                button .theme-toggle type="button" data-preset=(id) { (label) }
            }
            span #rd-status .mono { "loading" }
        }

        noscript {
            p .prose { "This demo needs JavaScript and WebAssembly. The rest of the site works without either." }
        }

        script type="module" src="/demos/loader.js" {}
    };
    shell::layout(site, Route::Demos, "Reaction-Diffusion", main)
}
```

- [ ] **Step 4: Style the canvas**

In `theme::stylesheet()`, add inside the `format!` raw string (remember: literal braces doubled):

```css
.rd-canvas {{
  width: 100%;
  max-width: 660px;
  aspect-ratio: 220 / 140;
  display: block;
  border: 1px solid var(--rule);
  image-rendering: pixelated;
  cursor: crosshair;
  touch-action: none;
  margin: calc(var(--space) * 3) 0;
}}
.rd-controls {{
  display: flex;
  flex-wrap: wrap;
  gap: var(--space);
  align-items: center;
  max-width: 660px;
}}
```

`image-rendering: pixelated` keeps the upscaled grid crisp rather than blurred, and `touch-action: none` stops a drag on the canvas from scrolling the page on mobile.

- [ ] **Step 5: Write the page in the build**

In `build.rs`, alongside the existing `404.html` write, add a write of `pages::demo_reaction_diffusion::render(&site)` to `demos/reaction-diffusion/index.html`. Register `pub mod demo_reaction_diffusion;` in `pages/mod.rs`.

Add a build test asserting `dist/demos/reaction-diffusion/index.html` exists, and that `sitemap.xml` does **not** contain `reaction-diffusion` — the demo page is not a `Route`, so it should not appear there.

- [ ] **Step 6: Verify end to end**

```bash
./scripts/build-wasm.sh
cargo test --all
cargo clippy --all-targets -- -D warnings
cargo run -p site -- --strict
python3 -m http.server 8137 -d dist
```

Open `http://localhost:8137/demos/reaction-diffusion/`. Confirm: patterns appear and evolve; clicking the canvas seeds new growth; the preset buttons change the pattern; pause/reset work; the fps readout is sensible; and toggling the site theme changes the demo's colours on the next frame.

Then confirm the isolation constraint — no other page pulls wasm:

```bash
grep -l 'loader.js' dist/**/*.html
```

Expected: only `dist/demos/reaction-diffusion/index.html`.

- [ ] **Step 7: Commit**

```bash
git add crates/site/src static/demos/loader.js
git commit -m "feat: add the reaction-diffusion demo page and lazy loader"
```

---

### Task 6: Measure it before claiming it

**Files:**
- Create: `docs/superpowers/wasm-benchmark.md`

**Interfaces:**
- Consumes: the built demo.
- Produces: a written measurement. No code ships from this task unless the numbers justify it.

The pitch for this demo is that WebAssembly does work JavaScript would struggle with. That is an assumption. A modern JIT on a tight loop over a typed array can come closer than people expect, and if it does, a side-by-side comparison would undercut the demo rather than sell it. Measure first, decide second.

- [ ] **Step 1: Write a JavaScript reference implementation**

Create a scratch file (not committed to `static/`) implementing the identical Gray-Scott update over `Float32Array`s — same kernel weights, same constants, same grid size, same substep count as the Rust version. Correctness matters: an accidentally cheaper JS version would produce a flattering, meaningless result.

- [ ] **Step 2: Time both**

In the browser console on the demo page, time 300 substeps of each at 220×140, five runs, and record the median milliseconds per substep for both. Use `performance.now()`, and discard the first run — JIT warm-up would otherwise make JS look worse than it is.

- [ ] **Step 3: Record the result and decide**

Write `docs/superpowers/wasm-benchmark.md` with: browser and version, machine, grid size, substep count, the raw medians, and the ratio.

Then apply this rule, and state which branch you took:

- **Ratio ≥ 2×:** the claim holds. Note that a comparison toggle would be a worthwhile follow-up, and record the number — a specific "3.4× faster than the equivalent JavaScript" is a far better portfolio line than a vague assertion.
- **Ratio < 2×:** the claim does not hold on this workload. Say so plainly in the document, and do **not** build a comparison UI that would advertise a weak result. The demo still stands on its own as an interactive simulation; it just should not be sold on a speed claim.

Do not adjust the JS implementation to make the ratio look better. A benchmark you tuned toward a conclusion is worth nothing.

- [ ] **Step 4: Commit**

```bash
git add docs/superpowers/wasm-benchmark.md
git commit -m "docs: measure WebAssembly against a JavaScript reference"
```

---

### Task 7: CI builds the WebAssembly

**Files:**
- Modify: `.github/workflows/deploy.yml`

**Interfaces:**
- Consumes: `scripts/build-wasm.sh`.
- Produces: a deployment including the wasm artifacts.

The wasm is gitignored build output, so without this task CI would publish a demo page whose module 404s. Nothing in the existing test suite would catch that, because the artifacts exist locally.

- [ ] **Step 1: Add the wasm toolchain to the build job**

In `.github/workflows/deploy.yml`, change the toolchain step to install the target:

```yaml
    - uses: dtolnay/rust-toolchain@stable
      with:
        components: clippy
        targets: wasm32-unknown-unknown
```

- [ ] **Step 2: Install the CLI and build the wasm**

After the `Swatinem/rust-cache@v2` step and before `Build site`, add:

```yaml
    - name: Install wasm-bindgen-cli
      run: cargo install wasm-bindgen-cli --version 0.2.127 --locked

    - name: Build WebAssembly
      run: ./scripts/build-wasm.sh
```

The version must match the `wasm-bindgen` dependency exactly. `--locked` keeps a transitive update from silently changing the tool mid-flight. The rust-cache step covers `~/.cargo/bin`, so the install is paid once and restored thereafter.

Order matters: this must run before `Build site`, because the generator copies `static/` into `dist/` and the artifacts have to exist by then.

- [ ] **Step 3: Fail the build if the artifacts are missing**

Add immediately after `Build site`:

```yaml
    - name: Verify wasm shipped
      run: |
        test -f dist/demos/reaction-diffusion/reaction_diffusion_bg.wasm
        test -f dist/demos/reaction-diffusion/reaction_diffusion.js
        ls -la dist/demos/reaction-diffusion/
```

This is the guard that makes the failure loud. A missing artifact would otherwise produce a page that looks fine to every test and is broken for every visitor.

- [ ] **Step 4: Commit**

```bash
git add .github/workflows/deploy.yml
git commit -m "ci: build WebAssembly before the site and verify it ships"
```

---

## Definition of Done

- The demo runs at `/demos/reaction-diffusion/` on the live site, animating smoothly, responding to clicks and preset changes.
- `Demos` appears in the nav; `/demos/` lists the demo; the demo page itself is not in `sitemap.xml`.
- No page other than the demo downloads any WebAssembly.
- With JavaScript disabled, the demo page renders its explanation and a `noscript` note; every other page is unaffected.
- A visitor with `prefers-reduced-motion` gets a still first frame rather than an unbidden animation.
- The demo's colours follow the site theme in both light and dark.
- CI builds the wasm, verifies both artifacts land in `dist/`, and fails the build if either is missing.
- `cargo test --all` and `cargo clippy --all-targets -- -D warnings` pass; zero `#[allow(dead_code)]` in the tree.
- `docs/superpowers/wasm-benchmark.md` records a real measurement and the decision it drove.

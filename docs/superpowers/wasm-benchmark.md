# WebAssembly vs. JavaScript: measuring the reaction-diffusion demo

The demo's pitch is that WebAssembly does work JavaScript would struggle
with. That is an assumption, not a given — a modern JIT on a tight loop over
`Float32Array`s can come closer than people expect. This document measures
it directly, before any comparison claim ships.

## Setup

- **Browser**: Headless Chrome 151.0.7922.109 (V8 15.1.206.16), launched
  with `--headless=new` against an isolated, freshly created
  `--user-data-dir` (not the user's real profile), driven via the Chrome
  DevTools Protocol (`Runtime.evaluate`) — the same approach Task 5 used to
  verify the demo end-to-end.
- **Machine**: Apple M4, macOS 26.6.1 (build 25G76), 10 cores.
- **Page**: the built demo served from `dist/demos/reaction-diffusion/` via
  a local `python3 -m http.server` on `localhost:8137`.
- **Grid size**: 220×140 (same as the live demo).
- **Substep count**: 300 per timed call, matching `Simulation::step`'s own
  substep-looping semantics (one call runs the full batch internally, same
  as the JS reference's `stepN`).
- **Preset**: coral (`feed = 0.0545`, `kill = 0.0620`), the demo's default.
- **Wasm binary size**: 27,640 bytes raw (measured directly, matches the
  brief's stated figure exactly), 12,318 bytes gzipped (`gzip -9 -n`,
  content only, no filename/mtime header). This is for context on load
  cost — not part of the runtime-speed question this document answers. The
  brief cited 12,350 gzipped; the ~30-byte difference is consistent with
  gzip header metadata (e.g. an embedded filename) rather than a different
  file, and doesn't affect anything below.

## The JavaScript reference

Written from scratch as scratch code at
`/private/tmp/.../scratchpad/js-reference.js` (session-local scratchpad, not
committed, not under `static/`, not referenced by any page). It is a
line-for-line port of `crates/demos/reaction-diffusion/src/grayscott.rs`:
same kernel weights (centre `-1.0`, edge-adjacent `0.2`, diagonal `0.05`),
same constants (`DA = 1.0`, `DB = 0.5`, `DT = 1.0`), same toroidal wrap via
double-modulo (JS has no `rem_euclid`), same double-buffered `a`/`b`/`aNext`/
`bNext` as `Float32Array`s, same `[0, 1]` clamp. No `Array` of objects, no
hand-tuning, no attempt to make either implementation look better — the
per-cell 3×3 Laplacian is a small closure called from the hot loop in both
languages, mirroring the Rust source's own `at` closure rather than being
manually unrolled for speed in either direction.

## Equivalence verification

Three independent checks, run before any timing:

1. **Closed-form pin (JS-only, mirrors the Rust unit tests).** A single
   seeded cell (`seed_rect(4, 4, 0)` on an 8×8 grid) stepped once with
   `A = 1, B = 0` everywhere else has closed-form neighbour values,
   independent of the reaction/kill terms (both are zero when the
   neighbour's own B is zero). The Rust test `laplacian_weights_are_exact`
   asserts an edge-adjacent neighbour of `0.1` and a diagonal neighbour of
   `0.025`. The JS port reproduced:
   - edge-adjacent: `0.10000000149011612`
   - diagonal: `0.02500000037252903`

   Both match `0.1` and `0.025` to `f32` precision (the trailing digits are
   exactly `f32`'s representation error for these values). This pins `DB`,
   the edge weight, and the diagonal weight in the JS port independent of
   any comparison against the WASM build.
2. **Wrap-around pin (JS-only, mirrors `wrap_is_immediate_across_the_edge`).**
   A cell seeded at the right edge of a 16×16 grid (`seed_rect(15, 8, 0)`),
   stepped once, produced `0.10000000149011612` at its wrap-around neighbour
   `(0, 8)` — confirming the toroidal wrap is immediate, not clamping.
3. **Cross-implementation comparison against the real WASM build.** A fresh
   `Simulation` (WASM) and a fresh `JSGrid` (JS reference) at 220×140 were
   seeded identically at five fixed points (`[110,70,3]`, `[60,40,2]`,
   `[160,100,4]`, `[30,110,2]`, `[200,20,3]`) and stepped through checkpoints
   at 1, 5, 10, 25, and 50 total steps with the coral preset. At each
   checkpoint, the WASM field was rendered via `Simulation.render()` (the
   only public accessor for the field — it exposes the B channel, quantized
   to `u8`) and compared byte-for-byte against a JS port of `render.rs`'s
   `paint()` applied to the JS field. Result: **`maxDiff = 0` at every
   checkpoint** (all 30,800 pixels × 4 channels, at all five checkpoints) —
   the two implementations are pixel-identical, not merely close. This is
   strong evidence of true equivalence: a reaction-diffusion system is
   sensitive enough that a wrong constant or kernel weight would show
   visible divergence well before 50 steps.

Equivalence is verified. The timing comparison below is against a genuinely
equivalent implementation.

## Method

For each implementation: 5 runs of 300 substeps at 220×140, timed with
`performance.now()` around a single call that loops all 300 substeps
internally (`Simulation.step(feed, kill, 300)` for WASM,
`JSGrid.stepN(feed, kill, 300)` for JS). Runs were interleaved (wasm, js,
wasm, js, ...) across the same page load to spread any thermal/scheduling
drift evenly across both implementations rather than let it fall unevenly on
whichever ran second. The first run of each implementation was discarded to
exclude V8 JIT warm-up (WASM is ahead-of-time compiled and has no analogous
warm-up cost, but was measured under the identical discard-first protocol
for symmetry). Median taken over the remaining four runs.

## Raw results (primary run)

**WASM** (ms per 300-substep call):

| run | ms |
|---|---|
| 1 (discarded — warm-up) | 101.00 |
| 2 | 97.40 |
| 3 | 97.90 |
| 4 | 98.20 |
| 5 | 98.60 |

Median of runs 2–5: **98.05 ms** (0.327 ms/substep)

**JavaScript** (ms per 300-substep call):

| run | ms |
|---|---|
| 1 (discarded — warm-up) | 194.10 |
| 2 | 193.20 |
| 3 | 190.20 |
| 4 | 188.00 |
| 5 | 188.80 |

Median of runs 2–5: **189.50 ms** (0.632 ms/substep)

**Ratio (JS median ÷ WASM median): 1.93×**

### Stability check

The full 5-run protocol was repeated three additional times (fresh page
loads) to confirm the primary run wasn't a fluke:

| run set | WASM raw (ms) | JS raw (ms) |
|---|---|---|
| check 1 | 98.8, 100.5, 99.7, 100.2, 98.6 | 190.2, 190.5, 191.1, 190.8, 190.6 |
| check 2 | 100.6, 100.5, 102.2, 97.5, 98.1 | 191.4, 191.6, 192.4, 193.1, 191.8 |
| check 3 | 99.0, 99.6, 99.4, 98.1, 99.6 | 191.0, 190.7, 191.7, 190.4, 190.9 |

All three land in the same ~98–100 ms (WASM) / ~190–193 ms (JS) band as the
primary run, giving a ratio consistently in the **1.9×–1.95× range** — not a
one-off measurement artifact. The equivalence check (`maxDiff = 0` at every
checkpoint) was re-verified on each of these runs as well.

## Decision

**Branch taken: Ratio < 2× — the claim does not hold on this workload.**

The measured ratio is **1.93×**, consistently reproduced across four
independent runs. This is meaningfully faster than JavaScript, but it falls
short of the 2× bar this task set for "the claim holds," and it is far
short of the kind of multiple (5–10×+) that would make a strong headline
speed claim. A modern JIT (V8, in this case) on a tight loop over
`Float32Array`s at this grid size closes most of the gap to ahead-of-time
compiled WASM.

**Recommendation: do not build a comparison UI (a JS-vs-WASM toggle) for
this demo.** Shipping a side-by-side comparison advertising "~2× faster"
would undercut the demo rather than sell it — it invites the reader to focus
on a modest number instead of the demo itself. The reaction-diffusion demo
stands on its own as an interactive simulation (the pattern generation,
presets, and click-to-seed interaction are the actual product); it should
not be sold on a speed claim at this grid size and substep count.

If a future task increases the grid size substantially, or the workload
gains more arithmetic intensity per cell, this ratio should be re-measured
before revisiting the decision — the two implementations may separate more
at a larger scale where memory-layout and compilation advantages compound,
but that is a new measurement, not an extrapolation from this one.

## Reasons to trust these numbers

- Equivalence is exact (pixel-identical output across all five checkpoints,
  and the JS port independently reproduces the Rust unit tests' closed-form
  values), so this is a comparison between genuinely equivalent
  implementations, not an accidentally-cheaper JS version.
- The ratio was stable across four independent full runs (primary run + 3
  stability checks), all landing in a narrow band, not a single noisy
  sample.
- Discard-first-run protocol was followed for both implementations.
- No production code changed; the JS reference lived only in the session
  scratchpad and was never imported by any page.

## Reasons for caution

- Headless Chrome, not a windowed browser — CPU scheduling and any
  compositor/vsync-related overhead differ from a real display, though
  neither implementation renders during the timed window (rendering is a
  separate, untimed call), so this shouldn't materially affect the compute
  comparison.
- Single machine (Apple M4). Ratios on other CPU microarchitectures
  (particularly ones with weaker JIT tiers, e.g. lower-end mobile devices)
  could differ in either direction.
- `performance.now()` resolution and OS scheduling noise are inherent to
  any browser-based timing; the four-run stability check was performed
  specifically to bound this rather than trust a single sample.

# Aho–Corasick Visualizer Demo — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A second WebAssembly demo at `/demos/aho-corasick/`: build an Aho–Corasick automaton from user-supplied patterns, draw its trie and failure links, and stream text through it with matches lighting up — the algorithm behind the owner's "hours to 90 seconds" Ampla work, visible.

**Architecture:** A pure Rust crate module builds the automaton (trie, BFS failure links, merged outputs) and computes a tidy-tree layout — all natively tested. A thin `#[wasm_bindgen]` wrapper exposes structure and step events as flat arrays in linear memory. A hand-written JS module draws the graph on canvas and renders the scanned text as DOM spans, so match highlighting is real, selectable text.

**Tech Stack:** Rust 1.92, `wasm-bindgen` 0.2.127, `maud` 0.27. No bundler, no npm, no other crates — building the automaton ourselves is the point; the crates.io `aho-corasick` crate must not appear.

## Global Constraints

Every task's requirements implicitly include this section.

- **No threads** (GitHub Pages cannot serve COOP/COEP). Single-threaded only.
- **No external hosts.** Everything self-hosted.
- **WASM loads only on its own demo page.** The reaction-diffusion demo's isolation guard pattern extends to this one.
- **Zero `#[allow(dead_code)]` anywhere in the tree.**
- CI gates deploy on `cargo clippy --all-targets -- -D warnings` and `cargo test --all`.
- Light is the default theme; dark is opt-in via `data-theme`. The demo must be legible in both, using only existing colour tokens.
- `prefers-reduced-motion`: the automaton renders built and still; streaming never auto-plays for those visitors.
- Inline theme scripts in `shell.rs`/`rail.rs` are untouched; their double-quoted JS keeps existing tests falsifying.
- **If a closed-form expected value exists, assert it.** Seven tests on this project could not fail; every fix was an exact-value assertion. The `{he, she, his, hers}` / `"ushers"` textbook fixture has exact known matches and failure links — use them.
- **Input caps** (legibility, not security — it all runs client-side): at most 8 patterns, each 1–10 chars, lowercase `a–z` only; scan text at most 200 chars. Uppercase folds to lowercase; other characters are stripped by the Rust side, and the page states the restriction.
- Crate name is `aho-corasick-demo` (a local crate named `aho-corasick` would collide with the well-known crates.io name in every conversation about it). Artifacts live under `static/demos/aho-corasick/`.

**The canonical fixture, used throughout:** patterns `["he", "she", "his", "hers"]`, text `"ushers"`. Matches (pattern, end-position-exclusive): `("she", 4)`, `("he", 4)`, `("hers", 6)`. The automaton has 10 states; the state for `"she"` fails to the state for `"he"` (suffix `he`), `"hers"`'s `r`-state fails to `"r"`… which doesn't exist, so to root — the exact link set is asserted in Task 1.

---

### Task 1: The automaton, natively tested

**Files:**
- Create: `crates/demos/aho-corasick-demo/Cargo.toml`, `crates/demos/aho-corasick-demo/src/automaton.rs`, `crates/demos/aho-corasick-demo/src/lib.rs`
- Modify: `Cargo.toml` (workspace members)

**Interfaces:**
- Produces: `automaton::Automaton` with `build(patterns: &[&str]) -> Result<Automaton, BuildError>`, `node_count()`, `label(state) -> u8` (the byte on the incoming edge; 0 for root), `parent(state) -> usize`, `fail(state) -> usize`, `depth(state) -> usize`, `outputs(state) -> &[usize]` (pattern indices ending here, including via suffix links), and a streaming cursor: `Cursor::new(&Automaton)`, `cursor.step(byte) -> StepEvent` where `StepEvent { hops: Vec<usize>, state: usize, matches: Vec<(usize, usize)> }` (failure states traversed this step, the landing state, and `(pattern_index, end_pos)` matches). Tasks 2–3 consume all of these.

- [ ] **Step 1: Workspace and manifest**

Root `Cargo.toml` members become `["crates/site", "crates/demos/reaction-diffusion", "crates/demos/aho-corasick-demo"]`.

`crates/demos/aho-corasick-demo/Cargo.toml`:

```toml
[package]
name = "aho-corasick-demo"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
wasm-bindgen = "0.2.127"
```

- [ ] **Step 2: Write the failing tests**

`src/automaton.rs`, tests module (implementation comes after):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn textbook() -> Automaton {
        Automaton::build(&["he", "she", "his", "hers"]).unwrap()
    }

    /// Walks the trie from the root along `word`, returning the state.
    fn state_for(a: &Automaton, word: &str) -> usize {
        let mut s = 0;
        for b in word.bytes() {
            s = (0..a.node_count())
                .find(|&n| a.parent(n) == s && a.label(n) == b)
                .expect("path exists");
        }
        s
    }

    #[test]
    fn textbook_trie_has_exactly_ten_states() {
        // root + h,he + s,sh,she + hi,his + her,hers — the classic figure.
        assert_eq!(textbook().node_count(), 10);
    }

    #[test]
    fn failure_links_match_the_textbook_exactly() {
        let a = textbook();
        // she → he (longest proper suffix that is a trie prefix)
        assert_eq!(a.fail(state_for(&a, "she")), state_for(&a, "he"));
        // sh → h ; her → root, since neither "er" nor "r" is a trie prefix
        assert_eq!(a.fail(state_for(&a, "sh")), state_for(&a, "h"));
        assert_eq!(a.fail(state_for(&a, "her")), 0);
        // hi → root (no "i" child of root); his → s
        assert_eq!(a.fail(state_for(&a, "hi")), 0);
        assert_eq!(a.fail(state_for(&a, "his")), state_for(&a, "s"));
        // depth-1 states always fail to root
        assert_eq!(a.fail(state_for(&a, "h")), 0);
        assert_eq!(a.fail(state_for(&a, "s")), 0);
    }

    #[test]
    fn ushers_produces_the_three_textbook_matches_in_order() {
        let a = textbook();
        let mut c = Cursor::new(&a);
        let mut found = Vec::new();
        for (i, b) in "ushers".bytes().enumerate() {
            for (pat, end) in c.step(b).matches {
                found.push((pat, end));
                assert_eq!(end, i + 1, "end must be the position after this byte");
            }
        }
        // pattern indices: 0=he, 1=she, 2=his, 3=hers
        assert_eq!(found, vec![(1, 4), (0, 4), (3, 6)]);
    }

    #[test]
    fn suffix_outputs_are_merged_not_rediscovered() {
        // "she" ends at position 4 and simultaneously ends "he" via the
        // suffix link — both must be reported from the SAME landing state.
        let a = textbook();
        assert_eq!(a.outputs(state_for(&a, "she")), &[1, 0]);
    }

    #[test]
    fn failure_hops_are_reported_for_the_visualizer() {
        // After "ushe", stepping 'r': "sher" is not in the trie, so the
        // cursor hops she→he before taking he→her. Exactly one hop,
        // landing on "her".
        let a = textbook();
        let mut c = Cursor::new(&a);
        for b in "ushe".bytes() {
            c.step(b);
        }
        let ev = c.step(b'r');
        assert_eq!(ev.hops, vec![state_for(&a, "he")]);
        assert_eq!(ev.state, state_for(&a, "her"));
    }

    #[test]
    fn build_rejects_bad_input_loudly() {
        assert!(Automaton::build(&[]).is_err(), "no patterns");
        assert!(Automaton::build(&["ok", ""]).is_err(), "empty pattern");
        assert!(Automaton::build(&["abcdefghijk"]).is_err(), "over 10 chars");
        let nine = ["a", "b", "c", "d", "e", "f", "g", "h", "i"];
        assert!(Automaton::build(&nine).is_err(), "over 8 patterns");
        assert!(Automaton::build(&["He"]).is_ok(), "uppercase folds, not errors");
    }

    #[test]
    fn overlapping_patterns_all_match() {
        let a = Automaton::build(&["aa", "aaa"]).unwrap();
        let mut c = Cursor::new(&a);
        let mut found = Vec::new();
        for b in "aaaa".bytes() {
            found.extend(c.step(b).matches);
        }
        // aa ends at 2,3,4; aaa ends at 3,4.
        assert_eq!(found, vec![(0, 2), (1, 3), (0, 3), (1, 4), (0, 4)]);
    }
}
```

- [ ] **Step 3: Run to verify failure**

`cargo test -p aho-corasick-demo` — FAIL, `Automaton` not found.

- [ ] **Step 4: Implement**

Above the tests:

```rust
//! Aho–Corasick automaton, built from scratch for the visualizer.
//! Deliberately free of wasm so the algorithm is tested natively.

#[derive(Debug, PartialEq)]
pub enum BuildError {
    NoPatterns,
    TooManyPatterns,
    EmptyPattern,
    PatternTooLong,
}

const MAX_PATTERNS: usize = 8;
const MAX_PATTERN_LEN: usize = 10;

struct Node {
    label: u8,
    parent: usize,
    depth: usize,
    children: Vec<usize>, // indices into nodes; labels are unique per parent
    fail: usize,
    outputs: Vec<usize>,
}

pub struct Automaton {
    nodes: Vec<Node>,
}

impl Automaton {
    pub fn build(patterns: &[&str]) -> Result<Self, BuildError> {
        if patterns.is_empty() {
            return Err(BuildError::NoPatterns);
        }
        if patterns.len() > MAX_PATTERNS {
            return Err(BuildError::TooManyPatterns);
        }
        let mut nodes = vec![Node {
            label: 0,
            parent: 0,
            depth: 0,
            children: Vec::new(),
            fail: 0,
            outputs: Vec::new(),
        }];

        for (pi, pat) in patterns.iter().enumerate() {
            let folded: Vec<u8> = pat
                .bytes()
                .filter(|b| b.is_ascii_alphabetic())
                .map(|b| b.to_ascii_lowercase())
                .collect();
            if folded.is_empty() {
                return Err(BuildError::EmptyPattern);
            }
            if folded.len() > MAX_PATTERN_LEN {
                return Err(BuildError::PatternTooLong);
            }
            let mut s = 0usize;
            for &b in &folded {
                s = match nodes[s].children.iter().copied().find(|&c| nodes[c].label == b) {
                    Some(c) => c,
                    None => {
                        let id = nodes.len();
                        let depth = nodes[s].depth + 1;
                        nodes.push(Node {
                            label: b,
                            parent: s,
                            depth,
                            children: Vec::new(),
                            fail: 0,
                            outputs: Vec::new(),
                        });
                        nodes[s].children.push(id);
                        id
                    }
                };
            }
            nodes[s].outputs.push(pi);
        }

        // BFS failure links; merge suffix outputs as we go.
        let mut queue: std::collections::VecDeque<usize> = nodes[0].children.clone().into();
        while let Some(u) = queue.pop_front() {
            for c in nodes[u].children.clone() {
                queue.push_back(c);
                let label = nodes[c].label;
                let mut f = nodes[u].fail;
                let fail_of_c = loop {
                    if let Some(t) = nodes[f].children.iter().copied().find(|&t| nodes[t].label == label) {
                        if t != c {
                            break t;
                        }
                    }
                    if f == 0 {
                        break 0;
                    }
                    f = nodes[f].fail;
                };
                nodes[c].fail = fail_of_c;
                let inherited = nodes[fail_of_c].outputs.clone();
                nodes[c].outputs.extend(inherited);
            }
        }
        Ok(Automaton { nodes })
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
    pub fn label(&self, s: usize) -> u8 {
        self.nodes[s].label
    }
    pub fn parent(&self, s: usize) -> usize {
        self.nodes[s].parent
    }
    pub fn fail(&self, s: usize) -> usize {
        self.nodes[s].fail
    }
    pub fn depth(&self, s: usize) -> usize {
        self.nodes[s].depth
    }
    pub fn outputs(&self, s: usize) -> &[usize] {
        &self.nodes[s].outputs
    }

    fn child(&self, s: usize, b: u8) -> Option<usize> {
        self.nodes[s].children.iter().copied().find(|&c| self.nodes[c].label == b)
    }
}

pub struct StepEvent {
    pub hops: Vec<usize>,
    pub state: usize,
    pub matches: Vec<(usize, usize)>,
}

pub struct Cursor<'a> {
    automaton: &'a Automaton,
    state: usize,
    pos: usize,
}

impl<'a> Cursor<'a> {
    pub fn new(automaton: &'a Automaton) -> Self {
        Cursor { automaton, state: 0, pos: 0 }
    }

    /// Advance one byte. Non-alphabetic bytes reset to root (word boundary).
    pub fn step(&mut self, byte: u8) -> StepEvent {
        self.pos += 1;
        if !byte.is_ascii_alphabetic() {
            self.state = 0;
            return StepEvent { hops: Vec::new(), state: 0, matches: Vec::new() };
        }
        let b = byte.to_ascii_lowercase();
        let mut hops = Vec::new();
        while self.automaton.child(self.state, b).is_none() && self.state != 0 {
            self.state = self.automaton.fail(self.state);
            hops.push(self.state);
        }
        if let Some(next) = self.automaton.child(self.state, b) {
            self.state = next;
        }
        let matches = self
            .automaton
            .outputs(self.state)
            .iter()
            .map(|&pi| (pi, self.pos))
            .collect();
        StepEvent { hops, state: self.state, matches }
    }
}
```

`src/lib.rs`: `pub mod automaton;` (the Task 3 wrapper will consume it and visibility gets revisited then, as in bucket 4).

- [ ] **Step 5: Verify and commit**

`cargo test -p aho-corasick-demo` — 7 tests PASS. `cargo clippy --all-targets -- -D warnings` clean. If clippy flags dead code on the unconsumed API, follow the bucket-4 precedent (`pub mod` until the wasm wrapper consumes it) — never `#[allow(dead_code)]`.

```bash
git add Cargo.toml Cargo.lock crates/demos/aho-corasick-demo
git commit -m "feat: add Aho-Corasick automaton with textbook closed-form tests"
```

---

### Task 2: Tidy-tree layout

**Files:**
- Create: `crates/demos/aho-corasick-demo/src/layout.rs`
- Modify: `src/lib.rs`

**Interfaces:**
- Consumes: `Automaton` (parent/depth/child structure).
- Produces: `layout::layout(a: &Automaton) -> Vec<(f32, f32)>` — one `(x, y)` per state in a unit-free coordinate space: `y = depth as f32`, leaves at consecutive integer `x` in trie insertion order, internal nodes centred over their children, root centred over everything. Task 3 ships these to JS, which scales them to the canvas.

- [ ] **Step 1: Failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::automaton::Automaton;

    #[test]
    fn ys_equal_depths_exactly() {
        let a = Automaton::build(&["he", "she", "his", "hers"]).unwrap();
        let pos = layout(&a);
        for s in 0..a.node_count() {
            assert_eq!(pos[s].1, a.depth(s) as f32, "state {s}");
        }
    }

    #[test]
    fn a_lone_chain_is_a_vertical_line_at_x_zero() {
        let a = Automaton::build(&["abc"]).unwrap();
        let pos = layout(&a);
        assert_eq!(pos, vec![(0.0, 0.0), (0.0, 1.0), (0.0, 2.0), (0.0, 3.0)]);
    }

    #[test]
    fn two_disjoint_chains_split_and_root_centres() {
        // patterns "ab", "cd": leaves b at x=0, d at x=1 (insertion order),
        // a over b, c over d, root centred at 0.5.
        let a = Automaton::build(&["ab", "cd"]).unwrap();
        let pos = layout(&a);
        assert_eq!(pos[0].0, 0.5, "root centres over both subtrees");
        let xs: Vec<f32> = (1..5).map(|s| pos[s].0).collect();
        assert_eq!(xs, vec![0.0, 0.0, 1.0, 1.0]);
    }

    #[test]
    fn parents_centre_over_their_children() {
        let a = Automaton::build(&["he", "hi"]).unwrap();
        let pos = layout(&a);
        // h has children e (x=0) and i (x=1) → h at 0.5; root over h at 0.5.
        let h = (0..a.node_count()).find(|&s| a.label(s) == b'h').unwrap();
        assert_eq!(pos[h].0, 0.5);
        assert_eq!(pos[0].0, 0.5);
    }
}
```

- [ ] **Step 2: Verify failure, implement**

Post-order walk: leaves take the next free integer slot left-to-right (children in stored insertion order); internal nodes take the mean of their children's x. Root included. ~30 lines; no recursion limits at depth ≤ 11.

- [ ] **Step 3: Verify and commit**

`cargo test -p aho-corasick-demo` — 11 PASS; clippy clean.

```bash
git add crates/demos/aho-corasick-demo/src
git commit -m "feat: add deterministic tidy-tree layout for the automaton"
```

---

### Task 3: WASM boundary and build script

**Files:**
- Modify: `crates/demos/aho-corasick-demo/src/lib.rs`, `scripts/build-wasm.sh`, `.gitignore`

**Interfaces:**
- Produces for JS (all names verbatim — the loader calls exactly these): `new Visualizer(patterns: &str, text: &str)` (patterns comma-separated; constructor errors become a JS exception with a readable message), `node_count()`, `xs_ptr()`, `ys_ptr()` (f32 arrays, one per state), `labels_ptr()` (u8 per state), `parents_ptr()`, `fails_ptr()` (u32 per state), `terminal_ptr()` (u8 per state, 1 if the state has outputs), `text_len()`, `reset()`, `step() -> bool` (advance one char; false when exhausted), then post-step accessors `current_state()`, `hops_ptr()`/`hops_len()`, `match_starts_ptr()`/`match_ends_ptr()`/`match_len()` (u32 arrays; this step's matches as text ranges, start inclusive, end exclusive — the wrapper converts `(pattern_idx, end)` to ranges using pattern lengths), and `pos()` (chars consumed).
- Every `_ptr` carries the established contract comment: rebuild the typed-array view every read; never cache it across wasm calls.

- [ ] **Step 1:** Implement the wrapper: build `Automaton` + `layout` in the constructor, flatten into owned `Vec`s, hold a rebuilt-per-`reset` cursor by index (store `state: usize` and re-drive the automaton — the borrow-holding `Cursor<'a>` cannot live in a wasm struct; give `automaton.rs` a small owned-cursor variant or inline the stepping loop in the wrapper, whichever is cleaner, without duplicating the failure-hop logic — expose it from `automaton.rs` as a free function if needed).

- [ ] **Step 2:** Extend `scripts/build-wasm.sh` to build **both** crates — factor the three-line build-and-bindgen sequence into a shell function called once per crate (`reaction_diffusion` → `static/demos/reaction-diffusion`, `aho_corasick_demo` → `static/demos/aho-corasick`). Append `/static/demos/aho-corasick/` to `.gitignore`.

- [ ] **Step 3:** Build; verify both artifact sets exist; record the new `.wasm` size (raw and gzipped) in the report. `cargo test --all` and clippy clean.

```bash
git add crates/demos/aho-corasick-demo/src/lib.rs scripts/build-wasm.sh .gitignore
git commit -m "feat: expose the automaton visualizer to WebAssembly"
```

---

### Task 4: Demo page, loader, and demos index entry

**Files:**
- Create: `crates/site/src/pages/demo_aho_corasick.rs`, `static/demos/aho-loader.js`
- Modify: `crates/site/src/pages/demos.rs`, `crates/site/src/build.rs`, `crates/site/src/theme.rs`, `crates/site/src/pages/mod.rs`

**Interfaces:**
- Produces: `pages::demos::AHO_CORASICK: &str = "/demos/aho-corasick/"`, `pages::demo_aho_corasick::render(site) -> Markup`, written by `build()` to `demos/aho-corasick/index.html` (not a `Route`; not in the nav; **in the sitemap** via the extra-paths argument, exactly like the reaction-diffusion page).

Page structure (follow `demo_reaction_diffusion.rs`'s shape — `sub_page` for the canonical, `noscript`, module script):

- `h1 "Aho–Corasick"`, one `.prose` paragraph: *"The string-matching automaton behind the transaction-tagging rewrite on the projects page. Patterns build a trie; failure links let a single pass over the text find every match. Edit the patterns, then step or play the scan."* (owner-voice: system facts, one pointer, no self-praise)
- Controls row: pattern input (`input .demo-input #ac-patterns`, default `he, she, his, hers`), text input (`#ac-text`, default `ushers say she sells seashells`), Rebuild / Play–Pause / Step / Reset buttons (`.theme-toggle` styling), `#ac-status .mono`
- `canvas #ac-canvas .ac-canvas aria-label="Aho-Corasick automaton graph"` — the graph
- `#ac-scan` — the scanned text as DOM spans (real text; the loader marks consumed chars, the current char, and matched ranges with classes)
- Legend line in `.mono`: solid = trie edge, dashed = failure link, filled = pattern end

Loader (`aho-loader.js`, ES module, only this page references it):

- Guard all element lookups as a group (bucket-4 lesson); `.catch` on `main()` setting a visible "failed to load" status
- Rebuild constructs a new `Visualizer` inside try/catch; a build error shows the message in `#ac-status`, keeping the previous automaton live
- Canvas draw: scale unit coords to the canvas with padding; trie edges as lines; failure links as dashed quadratic arcs (skip arcs whose target is the root — every shallow state fails to root and drawing them is pure clutter; state this in a comment); nodes as circles, letter labels, terminal states filled with `--accent`, current state ring-highlighted, hop states flashed
- Play loop: generation-token pattern **copied from `loader.js`** (the double-rAF lesson); speed fixed at ~3 steps/second — a speed slider is YAGNI until someone asks
- Colours read from `getComputedStyle` custom properties at draw time, and the existing `data-theme` MutationObserver pattern redraws a paused canvas on theme change (bucket-4 lesson I2)
- Reduced-motion: build and draw the automaton, never auto-play; status "paused — reduced motion"
- Text spans: `.consumed`, `.current`, `.matched` classes; matched ranges accumulate

CSS additions (braces doubled): `.ac-canvas` (responsive, `aspect-ratio: 16 / 9`, hairline border, white background in both themes like `.resume-page`? No — use `var(--surface)` so it participates in the theme), `.demo-input` (mono, surface background, rule border, focus ring inherited), `.ac-scan` classes (`.consumed` muted, `.current` inverse ring, `.matched` accent underline/background at AA contrast — use the existing tokens only), plus stylesheet-tying test entries.

Demos index (`pages/demos.rs`): second `.item` — title "Aho–Corasick", year "2026", link via `AHO_CORASICK`, summary *"Build the automaton, watch failure links form, and stream text through it."*, count `02`.

Build tests: page exists at `dist/demos/aho-corasick/index.html`; its canonical and `og:url` are its own URL; sitemap contains it; **loader isolation guard extended** — loop `Route::ALL` + `404.html` asserting neither `loader.js` nor `aho-loader.js` appears, demo pages each reference exactly their own loader.

Manual verification: build wasm, strict build, serve, and in a browser: automaton renders; Rebuild with changed patterns works; bad input (empty, 9 patterns, long pattern) shows an error without killing the page; Step walks "ushers" showing the she→he hop; matches highlight `she`/`he`/`hers` overlapping correctly; theme toggle recolours a paused canvas; reduced-motion stays still. Report observations.

```bash
git add crates/site/src static/demos/aho-loader.js
git commit -m "feat: add the Aho-Corasick visualizer demo page and loader"
```

---

### Task 5: CI

**Files:**
- Modify: `.github/workflows/deploy.yml`

`Build WebAssembly` already runs `scripts/build-wasm.sh`, which now builds both crates — no new build step. Extend `Verify wasm shipped`:

```yaml
        test -f dist/demos/aho-corasick/aho_corasick_demo_bg.wasm
        test -f dist/demos/aho-corasick/aho_corasick_demo.js
        grep -q 'aho-loader.js' dist/demos/aho-corasick/index.html
```

(the bucket-4 lesson: artifact assertions live in the workflow step where artifacts exist, and the grep proves the page references them). Validate YAML parses and step order is unchanged; `deploy` job untouched.

```bash
git add .github/workflows/deploy.yml
git commit -m "ci: verify the Aho-Corasick demo ships"
```

---

## Definition of Done

- `/demos/aho-corasick/` live: automaton drawn with failure links, editable patterns with loud-but-survivable input errors, step/play scan with overlapping matches highlighted in real DOM text.
- The `"ushers"` walk visibly shows a failure hop (she→he on the `r`).
- `/demos/` lists two demos; the new page is self-canonical and in the sitemap; nav unchanged.
- No page outside `/demos/aho-corasick/` references its loader or wasm; reaction-diffusion's isolation is preserved verbatim.
- Reduced-motion visitors get a still, built automaton. Theme changes recolour a paused canvas.
- CI builds both wasm crates and fails if either demo's artifacts or page references are missing.
- `cargo test --all` and `cargo clippy --all-targets -- -D warnings` green; zero `#[allow(dead_code)]`; every new physics/layout/stepping assertion is a closed-form exact value.

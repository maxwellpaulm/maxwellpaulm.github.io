# Bucket 1: Rust Generator + Design System — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the four hand-written HTML files with a Rust static site generator that renders them in composition A with a real design system, deploying `dist/` to GitHub Pages with CI gating on tests and lints.

**Architecture:** A Cargo workspace with one binary crate, `crates/site`. It owns design tokens (emitted as CSS custom properties), a `Route` enum that makes internal links unforgeable, `maud` components for the shell and left rail, one module per page, and a post-render integrity check that fails the build on dead internal links or missing assets. Output goes to `dist/`, which becomes the published artifact. No WebAssembly in this bucket.

**Tech Stack:** Rust 1.92, `maud` 0.27 (compile-time HTML), `serde` 1.0 + `toml` 1.1 (content), `anyhow` 1.0 (errors). No JavaScript, no CDN, no npm.

## Global Constraints

Every task's requirements implicitly include this section.

- **Rust edition 2021.** In maud templates on this edition, `#id` shorthand must be preceded by a space, and void elements terminate with `;`.
- **Colour tokens, light:** paper `#FBFAF8`, surface `#FFFFFF`, ink `#14161A`, muted `#6E7076`, rule `#E5E2DC`, accent `#A8431E`.
- **Colour tokens, dark:** paper `#0E0F11`, surface `#16181B`, ink `#E9E7E2`, muted `#94989F`, rule `#25282D`, accent `#E0764A`.
- **Accent is used only** for links, focus rings, active nav state, and section markers.
- **All token pairs must meet WCAG AA (≥ 4.5:1).** Task 2 enforces this with a test. The tightest pair is light muted-on-paper at ≈ 4.74:1 — do not darken the paper or lighten the muted token without re-running that test.
- **Spacing is an 8px scale.** Motion is capped at 180ms and must respect `prefers-reduced-motion`.
- **Prose measure is 56–68 characters.**
- **Fonts are self-hosted.** No CDN request may appear in any emitted page.
- **All content deserialisation uses `#[serde(deny_unknown_fields)]`.**
- **Navigation has exactly four entries in this bucket:** Index, About, Projects, Resume. Demos does not exist until bucket 4 and must not be linked — Task 8's checker will fail the build if it is.
- **`dist/` is the publish root.** It is generated output and must be git-ignored.
- **CI gates deploy on `cargo test` and `cargo clippy -- -D warnings`.**
- **Bio copy is fixed** (see Task 3 fixture): first person, NSA Codebreaker as a standalone credential, CFA on About only.

**Deviations from the spec, deliberate and flagged:**
1. Spec §5.1 lists a `content-model` crate. This bucket keeps content types inside `crates/site` because nothing else consumes them yet. Extract the crate in bucket 3, when the projects island needs to share them.
2. Spec §5.4 says fonts are subset at build time. This bucket self-hosts pre-subset Latin `woff2` files instead, which achieves the same goals (no CDN, no layout shift, small payload) without adding a build-time font toolchain. Revisit only if payload measurably matters.
3. Spec §7 says "snapshot tests on rendered components". This plan uses targeted assertions on rendered output instead. Snapshot tooling (`insta`) requires an interactive `cargo insta review` step to accept changes, which an agentic executor cannot perform, and a snapshot that is blindly accepted tests nothing. Targeted assertions state the actual invariant.

---

### Task 1: Workspace scaffold and page shell

**Files:**
- Create: `Cargo.toml`, `crates/site/Cargo.toml`, `crates/site/src/main.rs`, `crates/site/src/components/mod.rs`, `crates/site/src/components/shell.rs`
- Modify: `.gitignore`

**Interfaces:**
- Consumes: nothing.
- Produces: `components::shell::page(title: &str, body: Markup) -> Markup` — wraps content in the full document. Every page module in Tasks 5 and 6 calls it.

- [ ] **Step 1: Create the workspace manifest**

`Cargo.toml`:

```toml
[workspace]
members = ["crates/site"]
resolver = "2"

[workspace.package]
edition = "2021"
rust-version = "1.92"
```

`crates/site/Cargo.toml`:

```toml
[package]
name = "site"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true

[dependencies]
maud = "0.27"
serde = { version = "1.0", features = ["derive"] }
toml = "1.1"
anyhow = "1.0"
```

- [ ] **Step 2: Add `/dist` to `.gitignore`**

Append the line `/dist` to `.gitignore`. The file already contains `/target`, `set_env.sh`, `/assets/paul_maxwell_resume.pdf`, and `.superpowers/`.

- [ ] **Step 3: Write the failing test**

Create `crates/site/src/components/shell.rs`:

```rust
use maud::{html, Markup, DOCTYPE};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_emits_a_complete_document() {
        let out = page("About", html! { p { "hello" } }).into_string();
        assert!(out.starts_with("<!DOCTYPE html>"), "missing doctype: {out}");
        assert!(out.contains(r#"<html lang="en">"#), "missing lang attribute");
        assert!(out.contains("<title>About · Paul Maxwell</title>"));
        assert!(out.contains(r#"<meta name="viewport""#), "missing viewport meta");
        assert!(out.contains(r#"<link rel="stylesheet" href="/style.css">"#));
        assert!(out.contains("<p>hello</p>"), "body content not rendered");
    }

    #[test]
    fn page_makes_no_external_requests() {
        let out = page("Index", html! { p { "x" } }).into_string();
        for host in ["http://", "https://fonts.", "cdn.", "googleapis"] {
            assert!(!out.contains(host), "external reference {host} found in output");
        }
    }
}
```

Create `crates/site/src/components/mod.rs`:

```rust
pub mod shell;
```

Replace `crates/site/src/main.rs` with:

```rust
mod components;

fn main() {
    println!("site generator");
}
```

- [ ] **Step 4: Run the test to verify it fails**

Run: `cargo test -p site`
Expected: FAIL — `cannot find function 'page' in this scope`.

- [ ] **Step 5: Implement the shell**

Add to the top of `crates/site/src/components/shell.rs`, above the `tests` module:

```rust
/// Wraps page content in the full HTML document.
pub fn page(title: &str, body: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { (title) " · Paul Maxwell" }
                link rel="stylesheet" href="/style.css";
            }
            body {
                (body)
            }
        }
    }
}
```

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo test -p site`
Expected: PASS, 2 tests.

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml crates/site .gitignore
git commit -m "feat: add cargo workspace and page shell component"
```

---

### Task 2: Design tokens and stylesheet emission

**Files:**
- Create: `crates/site/src/theme.rs`
- Modify: `crates/site/src/main.rs`

**Interfaces:**
- Consumes: nothing.
- Produces: `theme::stylesheet() -> String` (the complete CSS, called by Task 7's build), `theme::LIGHT` and `theme::DARK` as `theme::Palette` values, and `theme::contrast_ratio(fg: &str, bg: &str) -> f64` used by the accessibility test.

- [ ] **Step 1: Write the failing test**

Create `crates/site/src/theme.rs` containing only this test module for now:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contrast_ratio_matches_known_values() {
        // Black on white is the reference maximum, 21:1.
        assert!((contrast_ratio("#000000", "#FFFFFF") - 21.0).abs() < 0.01);
        assert!((contrast_ratio("#FFFFFF", "#FFFFFF") - 1.0).abs() < 0.01);
    }

    #[test]
    fn every_token_pair_meets_wcag_aa() {
        for (name, p) in [("light", LIGHT), ("dark", DARK)] {
            for (label, fg) in [("ink", p.ink), ("muted", p.muted), ("accent", p.accent)] {
                let ratio = contrast_ratio(fg, p.paper);
                assert!(
                    ratio >= 4.5,
                    "{name}/{label} on paper is {ratio:.2}:1, below WCAG AA 4.5:1"
                );
            }
        }
    }

    #[test]
    fn stylesheet_defines_both_themes_and_respects_reduced_motion() {
        let css = stylesheet();
        assert!(css.contains("--paper: #FBFAF8"), "light tokens missing");
        assert!(css.contains("--paper: #0E0F11"), "dark tokens missing");
        assert!(css.contains("prefers-color-scheme: dark"));
        assert!(css.contains("prefers-reduced-motion: reduce"));
        assert!(css.contains(":focus-visible"), "focus ring missing");
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p site theme`
Expected: FAIL — `contrast_ratio`, `LIGHT`, `DARK`, `stylesheet` not found.

- [ ] **Step 3: Implement the palette and contrast maths**

Add above the `tests` module in `crates/site/src/theme.rs`:

```rust
#[derive(Clone, Copy)]
pub struct Palette {
    pub paper: &'static str,
    pub surface: &'static str,
    pub ink: &'static str,
    pub muted: &'static str,
    pub rule: &'static str,
    pub accent: &'static str,
}

pub const LIGHT: Palette = Palette {
    paper: "#FBFAF8",
    surface: "#FFFFFF",
    ink: "#14161A",
    muted: "#6E7076",
    rule: "#E5E2DC",
    accent: "#A8431E",
};

pub const DARK: Palette = Palette {
    paper: "#0E0F11",
    surface: "#16181B",
    ink: "#E9E7E2",
    muted: "#94989F",
    rule: "#25282D",
    accent: "#E0764A",
};

fn channel_luminance(byte: u8) -> f64 {
    let c = f64::from(byte) / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn relative_luminance(hex: &str) -> f64 {
    let h = hex.trim_start_matches('#');
    let parse = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).expect("valid hex colour");
    0.2126 * channel_luminance(parse(0))
        + 0.7152 * channel_luminance(parse(2))
        + 0.0722 * channel_luminance(parse(4))
}

/// WCAG 2.1 contrast ratio between two hex colours, from 1.0 to 21.0.
pub fn contrast_ratio(fg: &str, bg: &str) -> f64 {
    let (a, b) = (relative_luminance(fg), relative_luminance(bg));
    let (hi, lo) = if a > b { (a, b) } else { (b, a) };
    (hi + 0.05) / (lo + 0.05)
}
```

- [ ] **Step 4: Implement stylesheet emission**

Append to `crates/site/src/theme.rs`, above the `tests` module:

```rust
fn tokens(p: Palette) -> String {
    format!(
        "  --paper: {};\n  --surface: {};\n  --ink: {};\n  --muted: {};\n  --rule: {};\n  --accent: {};\n",
        p.paper, p.surface, p.ink, p.muted, p.rule, p.accent
    )
}

/// The complete stylesheet, emitted from the tokens above so light and dark
/// stay one system rather than two hand-maintained sheets.
pub fn stylesheet() -> String {
    format!(
        r#":root {{
{light}
  --space: 8px;
  --measure: 62ch;
  --font-sans: "Inter", "Helvetica Neue", system-ui, sans-serif;
  --font-mono: "JetBrains Mono", ui-monospace, Menlo, monospace;
  --motion: 180ms;
}}

@media (prefers-color-scheme: dark) {{
  :root {{
{dark}
  }}
}}

*, *::before, *::after {{ box-sizing: border-box; }}

body {{
  margin: 0;
  background: var(--paper);
  color: var(--ink);
  font-family: var(--font-sans);
  font-size: 16px;
  line-height: 1.6;
  font-weight: 350;
  -webkit-font-smoothing: antialiased;
}}

a {{ color: var(--accent); text-decoration: none; }}
a:hover {{ text-decoration: underline; }}

:focus-visible {{
  outline: 2px solid var(--accent);
  outline-offset: 3px;
  border-radius: 2px;
}}

.mono {{
  font-family: var(--font-mono);
  font-size: 10px;
  letter-spacing: 0.14em;
  text-transform: uppercase;
  color: var(--muted);
}}

/* Composition A: persistent left rail, content off-centre. */
.layout {{ display: grid; grid-template-columns: 132px 1fr; min-height: 100vh; }}
.rail {{
  border-right: 1px solid var(--rule);
  padding: calc(var(--space) * 4) calc(var(--space) * 2.5);
  display: flex;
  flex-direction: column;
  gap: calc(var(--space) * 3.5);
}}
.rail nav {{ display: flex; flex-direction: column; gap: 9px; }}
.rail nav a {{ font-size: 13px; color: var(--muted); transition: color var(--motion) ease; }}
.rail nav a[aria-current="page"] {{ color: var(--ink); font-weight: 500; }}
.rail nav a[aria-current="page"]::before {{ content: "— "; color: var(--accent); }}
.rail-foot {{ margin-top: auto; }}

main {{ padding: calc(var(--space) * 6.5) calc(var(--space) * 7) calc(var(--space) * 5); }}
h1 {{ font-size: 54px; line-height: 1.02; letter-spacing: -0.035em; font-weight: 600; margin: 0 0 calc(var(--space) * 3.25); }}
h2 {{ font-size: 24px; letter-spacing: -0.02em; font-weight: 600; }}
.lede {{ font-size: 17px; line-height: 1.62; max-width: var(--measure); margin: 0 0 calc(var(--space) * 1.75); }}
.prose {{ font-size: 14.5px; line-height: 1.65; max-width: var(--measure); color: var(--muted); }}

.section-head {{
  display: flex; justify-content: space-between; align-items: baseline;
  padding-bottom: 11px; border-bottom: 1px solid var(--ink);
  margin: calc(var(--space) * 5.5) 0 0;
}}
.section-head .mono {{ color: var(--ink); }}

.item {{
  display: grid; grid-template-columns: 1fr 74px; gap: calc(var(--space) * 2.25);
  padding: calc(var(--space) * 2.125) 0; border-bottom: 1px solid var(--rule);
  align-items: start;
}}
.item h3 {{ margin: 0 0 5px; font-size: 15px; font-weight: 500; letter-spacing: -0.01em; }}
.item p {{ margin: 0; font-size: 13.5px; line-height: 1.55; color: var(--muted); max-width: var(--measure); }}
.item .year {{ text-align: right; }}
.org {{ color: var(--accent); }}

/* The rail becomes a top bar before the grid gets cramped. */
@media (max-width: 640px) {{
  .layout {{ grid-template-columns: 1fr; }}
  .rail {{
    border-right: 0; border-bottom: 1px solid var(--rule);
    flex-direction: row; align-items: center; justify-content: space-between;
    gap: calc(var(--space) * 2); padding: calc(var(--space) * 2);
  }}
  .rail nav {{ flex-direction: row; gap: calc(var(--space) * 2); flex-wrap: wrap; }}
  .rail-foot {{ margin-top: 0; }}
  main {{ padding: calc(var(--space) * 4) calc(var(--space) * 2.5); }}
  h1 {{ font-size: 38px; }}
  .item {{ grid-template-columns: 1fr; gap: 4px; }}
  .item .year {{ text-align: left; }}
}}

@media (prefers-reduced-motion: reduce) {{
  *, *::before, *::after {{
    animation-duration: 0.01ms !important;
    transition-duration: 0.01ms !important;
  }}
}}
"#,
        light = tokens(LIGHT),
        dark = tokens(DARK),
    )
}
```

- [ ] **Step 5: Register the module**

In `crates/site/src/main.rs`, add `mod theme;` below `mod components;`.

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo test -p site`
Expected: PASS, 5 tests.

- [ ] **Step 7: Commit**

```bash
git add crates/site/src/theme.rs crates/site/src/main.rs
git commit -m "feat: add design tokens with WCAG AA contrast enforcement"
```

---

### Task 3: Content model and site content

**Files:**
- Create: `crates/site/src/content.rs`, `content/site.toml`

**Interfaces:**
- Consumes: nothing.
- Produces: `content::Site` with fields `name: String`, `location: String`, `role: String`, `lede: String`, `bio: String`, `credential: String`, `about: Vec<String>`, `work: Vec<content::Work>`; `content::Work` with `title: String`, `org: String`, `year: String`, `summary: String`; and `content::Site::load(path: &Path) -> anyhow::Result<Site>`.

- [ ] **Step 1: Write the failing test**

Create `crates/site/src/content.rs`:

```rust
use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_the_real_site_content() {
        let site = Site::load(Path::new("../../content/site.toml")).expect("site.toml loads");
        assert_eq!(site.name, "Paul Maxwell");
        assert_eq!(site.location, "Washington, DC");
        assert!(!site.work.is_empty(), "expected selected work entries");
        assert!(site.lede.len() < 160, "lede should stay a single sentence");
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let toml = r#"
name = "X"
location = "Y"
role = "Z"
lede = "L"
bio = "B"
credential = "C"
about = []
surprise = "should not parse"
"#;
        let err = toml::from_str::<Site>(toml).expect_err("unknown field must fail");
        assert!(err.to_string().contains("surprise"), "got: {err}");
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p site content`
Expected: FAIL — `cannot find type 'Site' in this scope`.

- [ ] **Step 3: Implement the content types**

Add above the `tests` module in `crates/site/src/content.rs`:

```rust
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Site {
    pub name: String,
    pub location: String,
    pub role: String,
    /// One sentence. The only line most visitors will read.
    pub lede: String,
    pub bio: String,
    pub credential: String,
    pub about: Vec<String>,
    #[serde(default)]
    pub work: Vec<Work>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Work {
    pub title: String,
    pub org: String,
    pub year: String,
    pub summary: String,
}

impl Site {
    pub fn load(path: &Path) -> Result<Self> {
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("reading {}", path.display()))?;
        toml::from_str(&raw).with_context(|| format!("parsing {}", path.display()))
    }
}
```

- [ ] **Step 4: Write the real content**

Create `content/site.toml`. This copy is the approved bio from the spec — do not paraphrase it:

```toml
name = "Paul Maxwell"
location = "Washington, DC"
role = "Lead Software Engineer / P-1 AI / 2025 —"
lede = "I build the infrastructure that gets AI systems safely into production — deployment, observability, and the trust boundaries in between."
bio = "Currently leading platform work at P-1 AI, where our engineering agent runs inside customers' own cloud environments so their data never has to leave. Before that: fraud prevention at Amazon, credit underwriting infrastructure at Ampla, distributed portfolio construction at BlackRock."
credential = "NSA Codebreaker Challenge — one of 24 finishers out of more than 3,300 participants, 2023."

about = [
  "I work at the layer where a system meets production and has to be trusted. That has meant bring-your-own-cloud deployment for an AI engineering agent, zero-trust gateways that keep real credentials out of agent sandboxes, real-time fraud detection, and underwriting controls that have to be auditable by people who are not engineers.",
  "I hold a Master's in Computer Science from Georgia Tech and a BSE from the University of Michigan, and I am a CFA charterholder — an unusual pairing that turns out to be useful anywhere software meets capital.",
  "Outside of work I do security challenges; I was one of 24 finishers of the NSA Codebreaker Challenge out of more than 3,300 participants in 2023.",
]

[[work]]
title = "Archie BYOC Platform"
org = "P-1 AI"
year = "2025"
summary = "Deploys an engineering AI agent into customer-controlled cloud; each install assembles from independently versioned components."

[[work]]
title = "Zero-Trust Agent Gateway"
org = "P-1 AI"
year = "2025"
summary = "A phantom-token pattern — agent code in ephemeral sandboxes holds only opaque, request-scoped tokens; the gateway keeps every real credential."

[[work]]
title = "Duplicate-Invoice Detection"
org = "Amazon"
year = "2024"
summary = "Real-time service bursting to 500 TPS with pluggable rule and ML engines, inside a suite blocking $1B+ in fraud annually."

[[work]]
title = "EU Compliance Ingestion"
org = "Amazon"
year = "2024"
summary = "Processes roughly one billion book titles per year, extracting required metadata and storing compliance artifacts at 500 TPS."

[[work]]
title = "Transaction Tagging Engine"
org = "Ampla"
year = "2022"
summary = "Re-implemented with Aho–Corasick; runtime went from hours to 90 seconds."
```

- [ ] **Step 5: Register the module and run the tests**

Add `mod content;` to `crates/site/src/main.rs`.

Run: `cargo test -p site`
Expected: PASS, 7 tests.

- [ ] **Step 6: Commit**

```bash
git add crates/site/src/content.rs crates/site/src/main.rs content/site.toml
git commit -m "feat: add site content model with strict deserialisation"
```

---

### Task 4: Routes and the left rail

**Files:**
- Create: `crates/site/src/route.rs`, `crates/site/src/components/rail.rs`
- Modify: `crates/site/src/components/mod.rs`, `crates/site/src/main.rs`

**Interfaces:**
- Consumes: `content::Site` (Task 3).
- Produces: `route::Route` (enum with variants `Index`, `About`, `Projects`, `Resume`), `Route::ALL: [Route; 4]`, `Route::path(&self) -> &'static str`, `Route::label(&self) -> &'static str`, `Route::output_path(&self) -> &'static str`, and `components::rail::rail(site: &Site, current: Route) -> Markup`.

Routing through this enum is what makes internal links unforgeable: pages never write an `href` by hand.

- [ ] **Step 1: Write the failing test**

Create `crates/site/src/route.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_route_has_a_rooted_path_and_html_output() {
        assert_eq!(Route::ALL.len(), 4, "Demos must not exist until bucket 4");
        for r in Route::ALL {
            assert!(r.path().starts_with('/'), "{:?} path must be rooted", r);
            assert!(r.output_path().ends_with(".html"), "{:?} bad output", r);
            assert!(!r.label().is_empty());
        }
    }

    #[test]
    fn index_is_served_from_the_root() {
        assert_eq!(Route::Index.path(), "/");
        assert_eq!(Route::Index.output_path(), "index.html");
        assert_eq!(Route::About.path(), "/about/");
        assert_eq!(Route::About.output_path(), "about/index.html");
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p site route`
Expected: FAIL — `cannot find type 'Route'`.

- [ ] **Step 3: Implement the routes**

Add above the `tests` module in `crates/site/src/route.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    Index,
    About,
    Projects,
    Resume,
}

impl Route {
    pub const ALL: [Route; 4] = [Route::Index, Route::About, Route::Projects, Route::Resume];

    pub fn path(&self) -> &'static str {
        match self {
            Route::Index => "/",
            Route::About => "/about/",
            Route::Projects => "/projects/",
            Route::Resume => "/resume/",
        }
    }

    pub fn output_path(&self) -> &'static str {
        match self {
            Route::Index => "index.html",
            Route::About => "about/index.html",
            Route::Projects => "projects/index.html",
            Route::Resume => "resume/index.html",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Route::Index => "Index",
            Route::About => "About",
            Route::Projects => "Projects",
            Route::Resume => "Resume",
        }
    }
}
```

- [ ] **Step 4: Write the failing rail test**

Create `crates/site/src/components/rail.rs`:

```rust
use crate::content::Site;
use crate::route::Route;
use maud::{html, Markup};

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn site() -> Site {
        Site::load(Path::new("../../content/site.toml")).unwrap()
    }

    #[test]
    fn rail_lists_every_route_and_marks_the_current_one() {
        let out = rail(&site(), Route::About).into_string();
        for r in Route::ALL {
            assert!(out.contains(r.label()), "missing nav entry {}", r.label());
        }
        assert!(
            out.contains(r#"href="/about/" aria-current="page""#),
            "current page not marked: {out}"
        );
        assert!(!out.contains("Demos"), "Demos must not be linked in bucket 1");
    }

    #[test]
    fn rail_shows_location() {
        let out = rail(&site(), Route::Index).into_string();
        assert!(out.contains("Washington, DC"));
    }
}
```

- [ ] **Step 5: Run it to verify it fails**

Run: `cargo test -p site rail`
Expected: FAIL — `cannot find function 'rail'`.

- [ ] **Step 6: Implement the rail**

Add above the `tests` module in `crates/site/src/components/rail.rs`:

```rust
/// The persistent left rail from composition A. Collapses to a top bar
/// under 640px via the stylesheet.
pub fn rail(site: &Site, current: Route) -> Markup {
    html! {
        div .rail {
            div .mono { "PM" }
            nav aria-label="Primary" {
                @for route in Route::ALL {
                    @if route == current {
                        a href=(route.path()) aria-current="page" { (route.label()) }
                    } @else {
                        a href=(route.path()) { (route.label()) }
                    }
                }
            }
            div .rail-foot {
                div .mono { (site.location) }
            }
        }
    }
}
```

- [ ] **Step 7: Register modules and run tests**

Add `pub mod rail;` to `crates/site/src/components/mod.rs`, and `mod route;` to `crates/site/src/main.rs`.

Run: `cargo test -p site`
Expected: PASS, 11 tests.

- [ ] **Step 8: Commit**

```bash
git add crates/site/src/route.rs crates/site/src/components crates/site/src/main.rs
git commit -m "feat: add route enum and left rail navigation"
```

---

### Task 5: Work list component and the index page

**Files:**
- Create: `crates/site/src/components/work.rs`, `crates/site/src/pages/mod.rs`, `crates/site/src/pages/index.rs`
- Modify: `crates/site/src/components/mod.rs`, `crates/site/src/components/shell.rs`, `crates/site/src/main.rs`

**Interfaces:**
- Consumes: `shell::page` (Task 1), `content::Site`/`Work` (Task 3), `Route` and `rail` (Task 4).
- Produces: `components::work::work_list(items: &[Work]) -> Markup`, `components::shell::layout(site: &Site, current: Route, title: &str, main: Markup) -> Markup`, and `pages::index::render(site: &Site) -> Markup`. Tasks 6 and 7 call `layout` and `render`.

- [ ] **Step 1: Add the `layout` helper test to `shell.rs`**

Add to the `tests` module in `crates/site/src/components/shell.rs`:

```rust
    #[test]
    fn layout_wraps_main_content_beside_the_rail() {
        use crate::content::Site;
        use crate::route::Route;
        use std::path::Path;

        let site = Site::load(Path::new("../../content/site.toml")).unwrap();
        let out = layout(&site, Route::Index, "Index", html! { p { "body" } }).into_string();
        assert!(out.contains(r#"class="layout""#));
        assert!(out.contains(r#"class="rail""#));
        assert!(out.contains("<main>"));
        assert!(out.contains("<p>body</p>"));
    }
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p site layout_wraps`
Expected: FAIL — `cannot find function 'layout'`.

- [ ] **Step 3: Implement `layout`**

Add to `crates/site/src/components/shell.rs`, and extend its imports to `use crate::content::Site; use crate::route::Route; use crate::components::rail::rail;`:

```rust
/// Composition A: rail plus main column, wrapped in the document shell.
pub fn layout(site: &Site, current: Route, title: &str, main: Markup) -> Markup {
    page(
        title,
        html! {
            div .layout {
                (rail(site, current))
                main { (main) }
            }
        },
    )
}
```

- [ ] **Step 4: Write the failing work-list test**

Create `crates/site/src/components/work.rs`:

```rust
use crate::content::Work;
use maud::{html, Markup};

#[cfg(test)]
mod tests {
    use super::*;

    fn items() -> Vec<Work> {
        vec![Work {
            title: "Zero-Trust Agent Gateway".into(),
            org: "P-1 AI".into(),
            year: "2025".into(),
            summary: "Phantom-token pattern.".into(),
        }]
    }

    #[test]
    fn renders_one_item_per_entry_with_org_and_year() {
        let out = work_list(&items()).into_string();
        assert!(out.contains("Zero-Trust Agent Gateway"));
        assert!(out.contains(r#"class="org""#));
        assert!(out.contains("P-1 AI"));
        assert!(out.contains("2025"));
        assert!(out.contains("Phantom-token pattern."));
        assert_eq!(out.matches(r#"class="item""#).count(), 1);
    }

    #[test]
    fn empty_input_renders_nothing_rather_than_an_empty_shell() {
        assert_eq!(work_list(&[]).into_string(), "");
    }
}
```

- [ ] **Step 5: Run it to verify it fails**

Run: `cargo test -p site work`
Expected: FAIL — `cannot find function 'work_list'`.

- [ ] **Step 6: Implement the work list**

Add above the `tests` module in `crates/site/src/components/work.rs`:

```rust
pub fn work_list(items: &[Work]) -> Markup {
    html! {
        @for item in items {
            div .item {
                div {
                    h3 { (item.title) " " span .org { "· " (item.org) } }
                    p { (item.summary) }
                }
                div .mono .year { (item.year) }
            }
        }
    }
}
```

- [ ] **Step 7: Write the failing index page test**

Create `crates/site/src/pages/index.rs`:

```rust
use crate::components::{shell, work::work_list};
use crate::content::Site;
use crate::route::Route;
use maud::{html, Markup};

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn index_shows_name_lede_and_selected_work() {
        let site = Site::load(Path::new("../../content/site.toml")).unwrap();
        let out = render(&site).into_string();
        assert!(out.contains("Paul"), "name missing");
        assert!(out.contains("trust boundaries in between"), "lede missing");
        assert!(out.contains("Selected Work"));
        assert!(out.contains("Archie BYOC Platform"));
        assert!(out.contains("3,300"), "credential line missing");
        assert!(out.contains(r#"aria-current="page""#));
    }
}
```

Create `crates/site/src/pages/mod.rs`:

```rust
pub mod index;
```

- [ ] **Step 8: Run it to verify it fails**

Run: `cargo test -p site index_shows`
Expected: FAIL — `cannot find function 'render'`.

- [ ] **Step 9: Implement the index page**

Add above the `tests` module in `crates/site/src/pages/index.rs`:

```rust
pub fn render(site: &Site) -> Markup {
    let main = html! {
        // Rendered from `site.name` rather than hardcoded, so the field is
        // actually read — an unread struct field fails `clippy -D warnings`.
        h1 {
            @for (i, part) in site.name.split(' ').enumerate() {
                @if i > 0 { br; }
                (part)
            }
        }
        p .lede { (site.lede) }
        p .prose { (site.bio) }
        p .mono style="margin-top:2rem" { (site.role) }

        div .section-head {
            span .mono { "Selected Work" }
            span .mono { (format!("{:02}", site.work.len())) }
        }
        (work_list(&site.work))

        p .prose style="margin-top:2rem" { (site.credential) }
    };
    shell::layout(site, Route::Index, "Index", main)
}
```

- [ ] **Step 10: Register modules and run all tests**

Add `pub mod work;` to `crates/site/src/components/mod.rs` and `mod pages;` to `crates/site/src/main.rs`.

Run: `cargo test -p site`
Expected: PASS, 15 tests.

- [ ] **Step 11: Commit**

```bash
git add crates/site/src
git commit -m "feat: add work list component and index page"
```

---

### Task 6: About, Projects, and Resume pages

**Files:**
- Create: `crates/site/src/pages/about.rs`, `crates/site/src/pages/projects.rs`, `crates/site/src/pages/resume.rs`
- Modify: `crates/site/src/pages/mod.rs`

**Interfaces:**
- Consumes: `shell::layout`, `content::Site`, `Route`, `work_list`.
- Produces: `pages::about::render(site: &Site) -> Markup`, `pages::projects::render(site: &Site) -> Markup`, `pages::resume::render(site: &Site) -> Markup`. Task 7 calls all three.

The resume page keeps the PDF embed and download link in this bucket. Bucket 2 replaces the embed with HTML rendered from `resume.json`; the download link stays permanently.

- [ ] **Step 1: Write the three failing tests**

Create `crates/site/src/pages/about.rs`:

```rust
use crate::components::shell;
use crate::content::Site;
use crate::route::Route;
use maud::{html, Markup};

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn about_renders_every_paragraph_and_mentions_the_cfa() {
        let site = Site::load(Path::new("../../content/site.toml")).unwrap();
        let out = render(&site).into_string();
        assert_eq!(out.matches("<p class=\"prose\">").count(), site.about.len());
        assert!(out.contains("CFA charterholder"), "CFA belongs on About");
        assert!(out.contains(r#"href="/about/" aria-current="page""#));
    }
}
```

Create `crates/site/src/pages/projects.rs`:

```rust
use crate::components::{shell, work::work_list};
use crate::content::Site;
use crate::route::Route;
use maud::{html, Markup};

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn projects_lists_the_work_entries_without_placeholder_copy() {
        let site = Site::load(Path::new("../../content/site.toml")).unwrap();
        let out = render(&site).into_string();
        assert!(out.contains("Transaction Tagging Engine"));
        for banned in ["Project 1", "Description of your", "goes here", "Lorem"] {
            assert!(!out.contains(banned), "placeholder copy found: {banned}");
        }
    }
}
```

Create `crates/site/src/pages/resume.rs`:

```rust
use crate::components::shell;
use crate::content::Site;
use crate::route::Route;
use maud::{html, Markup};

/// Published location of the PDF fetched from the private release.
pub const RESUME_PDF: &str = "/assets/paul_maxwell_resume.pdf";

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn resume_offers_a_download_and_embeds_the_pdf() {
        let site = Site::load(Path::new("../../content/site.toml")).unwrap();
        let out = render(&site).into_string();
        assert!(out.contains(&format!(r#"href="{RESUME_PDF}""#)), "download link missing");
        assert!(out.contains("download"), "download attribute missing");
        assert!(out.contains(RESUME_PDF), "embed source missing");
    }
}
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test -p site pages`
Expected: FAIL — `cannot find function 'render'` in three modules.

- [ ] **Step 3: Implement the About page**

Add above the `tests` module in `crates/site/src/pages/about.rs`:

```rust
pub fn render(site: &Site) -> Markup {
    let main = html! {
        h1 { "About" }
        @for paragraph in &site.about {
            p .prose { (paragraph) }
        }
    };
    shell::layout(site, Route::About, "About", main)
}
```

- [ ] **Step 4: Implement the Projects page**

Add above the `tests` module in `crates/site/src/pages/projects.rs`:

```rust
pub fn render(site: &Site) -> Markup {
    let main = html! {
        h1 { "Projects" }
        p .prose { "Selected work, most recent first. Longer write-ups and interactive demos are on the way." }
        div .section-head {
            span .mono { "Selected Work" }
            span .mono { (format!("{:02}", site.work.len())) }
        }
        (work_list(&site.work))
    };
    shell::layout(site, Route::Projects, "Projects", main)
}
```

- [ ] **Step 5: Implement the Resume page**

Add above the `tests` module in `crates/site/src/pages/resume.rs`:

```rust
pub fn render(site: &Site) -> Markup {
    let main = html! {
        h1 { "Resume" }
        p .prose {
            a href=(RESUME_PDF) download { "Download PDF" }
        }
        object data=(RESUME_PDF) type="application/pdf"
            style="width:100%;height:80vh;border:1px solid var(--rule);margin-top:1.5rem" {
            p .prose {
                "Your browser cannot display the embedded PDF. "
                a href=(RESUME_PDF) download { "Download it instead." }
            }
        }
    };
    shell::layout(site, Route::Resume, "Resume", main)
}
```

`<object>` replaces the old `<embed>` because it provides fallback content for browsers that cannot render PDFs inline, which `<embed>` cannot.

- [ ] **Step 6: Register the modules and run tests**

`crates/site/src/pages/mod.rs`:

```rust
pub mod about;
pub mod index;
pub mod projects;
pub mod resume;
```

Run: `cargo test -p site`
Expected: PASS, 18 tests.

- [ ] **Step 7: Commit**

```bash
git add crates/site/src/pages
git commit -m "feat: add about, projects, and resume pages"
```

---

### Task 7: Build orchestration and static assets

**Files:**
- Create: `crates/site/src/build.rs`, `static/fonts/README.md`
- Modify: `crates/site/src/main.rs`, `crates/site/src/theme.rs`

**Interfaces:**
- Consumes: every `pages::*::render`, `theme::stylesheet`, `Route::output_path`.
- Produces: `build::build(root: &Path, out: &Path, strict: bool) -> anyhow::Result<Vec<PathBuf>>`, returning every file written. Task 8's checker consumes the output directory. `root` is passed explicitly rather than read from `current_dir()` because tests run with the working directory set to the package root (`crates/site`), not the repo root.

- [ ] **Step 1: Fetch the fonts**

Download the two self-hosted webfonts into `static/fonts/`:

```bash
mkdir -p static/fonts
# Inter (OFL) — variable Latin subset
curl -fL -o static/fonts/InterVariable.woff2 \
  "https://github.com/rsms/inter/raw/master/docs/font-files/InterVariable.woff2"
# JetBrains Mono (OFL) — variable
curl -fL -o static/fonts/JetBrainsMono.woff2 \
  "https://github.com/JetBrains/JetBrainsMono/raw/master/fonts/webfonts/JetBrainsMono-Regular.woff2"
ls -la static/fonts/
```

Both files must be non-empty. If either URL 404s, fetch the equivalent `woff2` from the project's latest GitHub release instead — the filenames above are what the CSS expects, so rename accordingly. Record the source URL and licence in `static/fonts/README.md`.

- [ ] **Step 2: Add the `@font-face` rules**

At the very top of the returned string in `theme::stylesheet()` (before `:root`), insert:

```css
@font-face {
  font-family: "Inter";
  src: url("/fonts/InterVariable.woff2") format("woff2");
  font-weight: 100 900;
  font-display: swap;
}
@font-face {
  font-family: "JetBrains Mono";
  src: url("/fonts/JetBrainsMono.woff2") format("woff2");
  font-weight: 400;
  font-display: swap;
}
```

Remember this is inside a `format!` block: literal braces must be doubled to `{{` and `}}`.

- [ ] **Step 3: Write the failing build test**

Create `crates/site/src/build.rs`:

```rust
use crate::content::Site;
use crate::{pages, route::Route, theme};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_writes_every_route_plus_the_stylesheet() {
        let tmp = std::env::temp_dir().join("site-build-test");
        let _ = std::fs::remove_dir_all(&tmp);

        // Repo root, relative to the package root that `cargo test` runs in.
        let written = build(Path::new("../.."), &tmp, false).expect("build succeeds");

        for route in Route::ALL {
            let p = tmp.join(route.output_path());
            assert!(p.exists(), "missing output {}", p.display());
            let html = std::fs::read_to_string(&p).unwrap();
            assert!(html.starts_with("<!DOCTYPE html>"));
        }
        assert!(tmp.join("style.css").exists(), "stylesheet not emitted");
        assert!(tmp.join("fonts/InterVariable.woff2").exists(), "fonts not copied");
        assert!(written.len() >= 5);

        std::fs::remove_dir_all(&tmp).ok();
    }
}
```

- [ ] **Step 4: Run it to verify it fails**

Run: `cargo test -p site build`
Expected: FAIL — `cannot find function 'build'`.

- [ ] **Step 5: Implement the build**

Add above the `tests` module in `crates/site/src/build.rs`:

```rust
const CONTENT: &str = "content/site.toml";

fn write(path: &Path, body: &str, written: &mut Vec<PathBuf>) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    std::fs::write(path, body).with_context(|| format!("writing {}", path.display()))?;
    written.push(path.to_path_buf());
    Ok(())
}

fn copy_tree(from: &Path, to: &Path, written: &mut Vec<PathBuf>) -> Result<()> {
    if !from.exists() {
        return Ok(());
    }
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &target, written)?;
        } else {
            std::fs::create_dir_all(to)?;
            std::fs::copy(entry.path(), &target)
                .with_context(|| format!("copying {}", entry.path().display()))?;
            written.push(target);
        }
    }
    Ok(())
}

/// Renders the whole site into `out`. With `strict`, a missing resume PDF is a
/// hard error; without it, a warning — so local builds work without the token
/// that CI uses to fetch the private release.
pub fn build(root: &Path, out: &Path, strict: bool) -> Result<Vec<PathBuf>> {
    let site = Site::load(&root.join(CONTENT))?;
    let mut written = Vec::new();

    for route in Route::ALL {
        let markup = match route {
            Route::Index => pages::index::render(&site),
            Route::About => pages::about::render(&site),
            Route::Projects => pages::projects::render(&site),
            Route::Resume => pages::resume::render(&site),
        };
        write(&out.join(route.output_path()), &markup.into_string(), &mut written)?;
    }

    write(&out.join("style.css"), &theme::stylesheet(), &mut written)?;
    copy_tree(&root.join("static"), out, &mut written)?;
    copy_tree(&root.join("assets"), &out.join("assets"), &mut written)?;

    let pdf = out.join("assets/paul_maxwell_resume.pdf");
    if !pdf.exists() {
        let msg = "resume PDF missing — run the release fetch before building";
        if strict {
            anyhow::bail!("{msg}");
        }
        eprintln!("warning: {msg} (non-strict build, continuing)");
    }

    if let Ok(cname) = std::fs::read_to_string(root.join("CNAME")) {
        write(&out.join("CNAME"), &cname, &mut written)?;
    }

    Ok(written)
}
```

- [ ] **Step 6: Wire up `main.rs`**

Replace `crates/site/src/main.rs` with:

```rust
mod build;
mod components;
mod content;
mod pages;
mod route;
mod theme;

use anyhow::Result;
use std::path::{Path, PathBuf};

fn main() -> Result<()> {
    let strict = std::env::args().any(|a| a == "--strict");
    let out = PathBuf::from("dist");

    if out.exists() {
        std::fs::remove_dir_all(&out)?;
    }
    let written = build::build(Path::new("."), &out, strict)?;
    println!("wrote {} files to {}", written.len(), out.display());
    Ok(())
}
```

- [ ] **Step 7: Run the tests and a real build**

```bash
cargo test -p site
cargo run -p site
ls -R dist | head -30
```

Expected: 19 tests pass; `dist/` contains `index.html`, `about/index.html`, `projects/index.html`, `resume/index.html`, `style.css`, `fonts/`, and a warning about the missing PDF if you have not fetched it.

- [ ] **Step 8: Commit**

```bash
git add crates/site/src static/fonts
git commit -m "feat: add build orchestration, static assets, and self-hosted fonts"
```

---

### Task 8: Build integrity checks

**Files:**
- Create: `crates/site/src/checks.rs`
- Modify: `crates/site/src/build.rs`, `crates/site/src/main.rs`

**Interfaces:**
- Consumes: the output of `build::build`.
- Produces: `checks::verify(out: &Path, strict: bool) -> anyhow::Result<()>`, called at the end of `build::build`. It always runs, so local builds catch dead links too; `strict` only controls whether `/assets/` references (fetched from the private release in CI) are required to exist.

This is the payoff of a generator that knows its own routes: shipping a broken internal link becomes impossible rather than something discovered later.

- [ ] **Step 1: Write the failing test**

Create `crates/site/src/checks.rs`:

```rust
use anyhow::{bail, Result};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests {
    use super::*;

    fn scaffold(name: &str, html: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(name);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("index.html"), html).unwrap();
        std::fs::write(dir.join("style.css"), "body{}").unwrap();
        dir
    }

    #[test]
    fn accepts_links_that_resolve() {
        let dir = scaffold(
            "checks-ok",
            r#"<a href="/style.css">css</a><a href="https://example.com">ext</a>"#,
        );
        assert!(verify(&dir, true).is_ok());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rejects_a_dead_internal_link() {
        let dir = scaffold("checks-dead", r#"<a href="/demos/">Demos</a>"#);
        let err = verify(&dir, true).expect_err("dead link must fail the build");
        assert!(err.to_string().contains("/demos/"), "got: {err}");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rejects_a_missing_asset() {
        let dir = scaffold("checks-asset", r#"<img src="/logo.svg">"#);
        let err = verify(&dir, true).expect_err("missing asset must fail the build");
        assert!(err.to_string().contains("/logo.svg"), "got: {err}");
        std::fs::remove_dir_all(&dir).ok();
    }
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p site checks`
Expected: FAIL — `cannot find function 'verify'`.

- [ ] **Step 3: Implement the checker**

Add above the `tests` module in `crates/site/src/checks.rs`:

```rust
/// Pulls every `href="..."` and `src="..."` value out of the HTML.
fn references(html: &str) -> Vec<String> {
    let mut found = Vec::new();
    for attr in [r#"href=""#, r#"src=""#, r#"data=""#] {
        let mut rest = html;
        while let Some(start) = rest.find(attr) {
            rest = &rest[start + attr.len()..];
            if let Some(end) = rest.find('"') {
                found.push(rest[..end].to_string());
                rest = &rest[end..];
            }
        }
    }
    found
}

/// Maps a rooted site path to the file that must exist in `out`.
fn target(out: &Path, link: &str) -> PathBuf {
    let rel = link.trim_start_matches('/');
    if link.ends_with('/') || link.is_empty() {
        out.join(rel).join("index.html")
    } else {
        out.join(rel)
    }
}

fn html_files(dir: &Path, acc: &mut Vec<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            html_files(&path, acc)?;
        } else if path.extension().is_some_and(|e| e == "html") {
            acc.push(path);
        }
    }
    Ok(())
}

/// Fails the build if any internal link or asset reference does not resolve to
/// a file in the output directory.
pub fn verify(out: &Path, strict: bool) -> Result<()> {
    let mut pages = Vec::new();
    html_files(out, &mut pages)?;

    let mut broken = BTreeSet::new();
    for page in &pages {
        let html = std::fs::read_to_string(page)?;
        for link in references(&html) {
            if !link.starts_with('/') {
                continue; // external, anchor, or relative — out of scope
            }
            if !strict && link.starts_with("/assets/") {
                continue; // fetched from the private release; absent locally
            }
            let path = target(out, &link);
            if !path.exists() {
                broken.insert(format!("{} → {}", page.display(), link));
            }
        }
    }

    if !broken.is_empty() {
        bail!(
            "{} unresolved reference(s):\n  {}",
            broken.len(),
            broken.into_iter().collect::<Vec<_>>().join("\n  ")
        );
    }
    Ok(())
}
```

- [ ] **Step 4: Call it from the build**

In `crates/site/src/build.rs`, add `use crate::checks;` to the imports, and insert this immediately before `Ok(written)` in `build`:

```rust
    checks::verify(out, strict)?;
```

The check always runs, so a dead internal link fails a local build too. `strict` only relaxes `/assets/` references, which are absent locally because the resume PDF comes from the private release.

- [ ] **Step 5: Register the module and verify both modes**

Add `mod checks;` to `crates/site/src/main.rs`.

```bash
cargo test -p site
cargo run -p site                      # non-strict: warns, succeeds
cargo run -p site -- --strict          # fails: resume PDF absent locally
```

Expected: 22 tests pass; the non-strict build succeeds; the strict build exits non-zero complaining about the missing PDF. That failure is correct — it proves the gate works.

- [ ] **Step 6: Commit**

```bash
git add crates/site/src
git commit -m "feat: fail the build on dead internal links and missing assets"
```

---

### Task 9: CI workflow and deployment

**Files:**
- Create: `scripts/fetch-resume.sh`
- Modify: `.github/workflows/deploy.yml`
- Delete: `build.sh`, `index.html`, `about.html`, `projects.html`, `resume.html`

**Interfaces:**
- Consumes: `cargo run -p site -- --strict`.
- Produces: a deployed site. Nothing downstream depends on this task.

- [ ] **Step 1: Extract the resume fetch into its own script**

Create `scripts/fetch-resume.sh` — this is the existing `build.sh` logic, unchanged apart from doing only one job:

```bash
#!/bin/bash
# Downloads the resume PDF from the latest release of the private resume repo.
set -euo pipefail

if [ -z "${GITHUB_TOKEN:-}" ]; then
    echo "Error: GITHUB_TOKEN is required for private repo access" >&2
    exit 1
fi

mkdir -p assets

ASSET_URL=$(curl -sf -H "Authorization: token $GITHUB_TOKEN" \
  https://api.github.com/repos/maxwellpaulm/resume/releases/latest | \
  jq -r '.assets[] | select(.name == "paul_maxwell_resume.pdf") | .url')

if [ -z "$ASSET_URL" ] || [ "$ASSET_URL" = "null" ]; then
    echo "Error: could not find paul_maxwell_resume.pdf in the latest release" >&2
    exit 1
fi

curl -fL -H "Authorization: token $GITHUB_TOKEN" \
  -H "Accept: application/octet-stream" \
  "$ASSET_URL" -o assets/paul_maxwell_resume.pdf

echo "Resume downloaded to assets/paul_maxwell_resume.pdf"
```

Then `chmod +x scripts/fetch-resume.sh`.

- [ ] **Step 2: Replace the workflow**

`.github/workflows/deploy.yml`:

```yaml
name: Deploy Website

on:
  push:
    branches: [ master ]
  pull_request:
  workflow_dispatch:

permissions:
  contents: read
  pages: write
  id-token: write

concurrency:
  group: "pages"
  cancel-in-progress: false

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4

    - uses: dtolnay/rust-toolchain@stable
      with:
        components: clippy

    - uses: Swatinem/rust-cache@v2

    - name: Lint
      run: cargo clippy --all-targets -- -D warnings

    - name: Test
      run: cargo test --all

    - name: Fetch resume
      env:
        GITHUB_TOKEN: ${{ secrets.RESUME_GITHUB_TOKEN }}
      run: ./scripts/fetch-resume.sh

    - name: Build site
      run: cargo run -p site -- --strict

    - uses: actions/upload-pages-artifact@v3
      with:
        path: 'dist'

  deploy:
    needs: build
    if: github.ref == 'refs/heads/master'
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    runs-on: ubuntu-latest
    steps:
    - id: deployment
      uses: actions/deploy-pages@v4
```

Three deliberate changes from the current workflow: the artifact path is `dist` rather than `.` (so source and repo metadata are no longer published), lint and test gate the build, and pull requests are built but not deployed.

- [ ] **Step 3: Delete the files the generator replaces**

```bash
git rm build.sh index.html about.html projects.html resume.html
```

`CNAME` stays at the repo root — `build::build` copies it into `dist/`, which is required for the custom domain to keep working.

- [ ] **Step 4: Verify the full pipeline locally**

```bash
cargo clippy --all-targets -- -D warnings
cargo test --all
GITHUB_TOKEN=<your token> ./scripts/fetch-resume.sh
cargo run -p site -- --strict
test -f dist/CNAME && echo "CNAME present"
python3 -m http.server -d dist 8000
```

Open `http://localhost:8000` and confirm: all four pages render in composition A, the rail marks the current page, the resume PDF loads and downloads, dark mode follows your system setting, and the layout collapses to a top bar below 640px wide.

- [ ] **Step 5: Commit**

```bash
git add scripts/fetch-resume.sh .github/workflows/deploy.yml
git commit -m "ci: build with cargo, publish dist/, gate deploy on tests"
```

- [ ] **Step 6: Deploy**

Push the branch and open a PR. Confirm the build job passes on the PR before merging. **Ask before pushing to `master`** — it is a protected branch.

After merge, verify against production:

```bash
curl -sI https://paul-maxwell.com/ | grep -i 'HTTP/'
curl -sI https://paul-maxwell.com/style.css | grep -iE 'HTTP/|content-type'
curl -sI https://paul-maxwell.com/assets/paul_maxwell_resume.pdf | grep -iE 'HTTP/|content-type'
curl -s https://paul-maxwell.com/ | grep -c 'Maxwell'
```

Expected: `200` for all three, `text/css` for the stylesheet, `application/pdf` for the resume.

---

## Definition of Done

Bucket 1 is complete when all of the following hold:

- All four pages render from Rust in composition A, deployed to `paul-maxwell.com`.
- Light and dark themes both work from one set of tokens, and every token pair passes the WCAG AA test.
- The resume PDF still downloads from the latest private release.
- No placeholder copy remains on any page.
- `cargo clippy -- -D warnings` and `cargo test --all` pass, and CI blocks deploy if they do not.
- A dead internal link fails the build (verified by the Task 8 tests).
- `dist/` is the published artifact; repo source is no longer uploaded to Pages.

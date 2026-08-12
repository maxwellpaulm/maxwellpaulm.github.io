# Rust + WASM Rebuild of maxwellpaulm.github.io

**Date:** 2026-08-12
**Status:** Approved design, ready for implementation planning

## 1. Context

The site today is four hand-written HTML files (`index`, `about`, `projects`, `resume`), a
`build.sh` that downloads a resume PDF from a private release, and a GitHub Actions workflow
that publishes the repo root to GitHub Pages. `about.html` and `projects.html` contain
placeholder copy. The resume page embeds the PDF in an `<embed>` tag.

The site is served at `paul-maxwell.com`, which resolves to Cloudflare (`104.21.88.4`,
`172.67.149.222`) proxying GitHub Pages.

## 2. Goals

In priority order, all confirmed with the author:

1. **Learn and showcase Rust.** The site is a portfolio; building it in Rust is itself part
   of the deliverable.
2. **A real authoring workflow.** Replace four copies of the same `<nav>` with templates and
   components.
3. **Interactive WASM features** that are load-bearing, not decorative.
4. **A site that looks finished.** Professional and slick, with real content replacing
   placeholders.

## 3. Verified Constraints

WebAssembly on GitHub Pages was confirmed empirically before design, not assumed. A live
Rust-WASM app on a Pages site with an identical setup (github.io repo plus custom-domain
CNAME) serves:

```
GET https://www.egui.rs/egui_demo_app_bg.wasm
  HTTP/2 200
  server: GitHub.com
  content-type: application/wasm
  content-encoding: gzip        # 11.8 MB → 4.6 MB
```

Correct MIME type (so `WebAssembly.instantiateStreaming` works) and automatic compression,
on an 11.8 MB binary. Our budget is one to two orders of magnitude smaller.

Constraints that follow:

- **No server.** Everything "backend" happens at build time in Actions.
- **No WASM threads by default.** Pages cannot set `Cross-Origin-Opener-Policy` /
  `Cross-Origin-Embedder-Policy`, so no `SharedArrayBuffer`, so no `rayon`. All demos must be
  single-threaded. Recoverable later if ever needed: the site sits behind Cloudflare, whose
  Transform Rules can inject those headers.
- **Pages limits:** 100 MB per file, 1 GB per site, ~10 builds/hour. Not binding at this scale.
- **Cache headers are fixed** at `max-age=600` by Pages; Cloudflare can override if needed.

## 4. Decisions

| Decision | Choice | Rationale |
|---|---|---|
| Architecture | Static shell + WASM islands | Real HTML at build time; wasm loads only on pages that need it. A full Rust SPA was rejected: blank page until wasm loads, nothing for crawlers, and every visitor downloads demo code to read a bio. |
| Generator | Hand-rolled Rust with `maud` | Compile-time-checked HTML, full control over markup (which the visual design needs), and maximum learning value. Zola was rejected because it means writing Tera and TOML, not Rust. |
| Resume content | `resume.json` upstream, rendered at build time | Build-time rendering strictly dominates WASM rendering here: zero payload, selectable and crawlable text, correct printing. Parsing the `.tex` was rejected as brittle; client-side PDF rendering was rejected as heavy and worse on mobile. |
| PDF | Download button retained | It is what recruiters actually want. |
| Visual direction | Technical editorial on a Swiss grid | Editorial restraint in type and colour, strict grid with deliberate asymmetry. Chosen over terminal-modern (too common in dev portfolios) and pure Swiss minimal (unforgiving with thin content). |
| Homepage composition | **A** — asymmetric, left rail | Selected from side-by-side mockups. More distinctive than a centered layout, gives the bio room to be a real paragraph, and the rail is a consistent spine across pages. |
| Publish root | `dist/` | Today the workflow publishes `.`, uploading source and repo metadata along with the site. |

Later upgrade path: individual pages can move to Leptos/Dioxus SSG-plus-hydration without
discarding work, since the component code is what survives.

## 5. Architecture

### 5.1 Repo layout

```
Cargo.toml                    workspace
crates/
  site/                       generator binary → dist/
    src/main.rs               build orchestration
    src/pages/                index, about, projects, resume, demos
    src/components/           rail nav, work item, resume sections
    src/theme.rs              design tokens; single source for emitted CSS
  content-model/              serde types shared by build and wasm
  island-projects/            cdylib → projects browser wasm
  demos/<name>/               one cdylib per demo, own .wasm
content/
  site.toml                   bio, links, metadata
  projects.yaml               project entries
static/                       fonts (subset at build), favicon, OG image
dist/                         build output; the published artifact
```

### 5.2 Build pipeline

Replaces `build.sh`, runs in the existing Actions job:

1. Fetch `resume.json` and the PDF from the private release, using the current
   `RESUME_GITHUB_TOKEN` flow unchanged.
2. `cargo run -p site` — renders every page into `dist/`, emits one stylesheet from
   `theme.rs`, subsets fonts to glyphs actually used.
3. Each wasm crate builds `--release`, is `wasm-opt`'d, and lands in `dist/wasm/<name>/`.
4. `upload-pages-artifact` publishes `dist/`.

CI runs `cargo test` and `cargo clippy -D warnings` before deploy; a red build never
publishes.

### 5.3 Island model

The projects list renders as real HTML at build time. A small loader (~1 KB) scans for
`data-island="projects"` and fetches that wasm only if the element is present; the island
then hydrates the existing list with filter and search. With JS disabled, wasm blocked, or a
slow connection, the list still renders and the page still works.

Each demo lives on its own page with its own binary. Reading the bio downloads zero wasm, and
adding a fifth demo costs the other four nothing.

### 5.4 Design system

Defined once in `theme.rs` and emitted as CSS custom properties, so light and dark are one
system rather than two stylesheets.

- **Palette (light):** paper `#FBFAF8`, ink `#14161A`, muted `#6E7076`, rule `#E5E2DC`,
  accent `#A8431E`.
- **Palette (dark):** paper `#0E0F11`, surface `#16181B`, ink `#E9E7E2`, muted `#94989F`,
  rule `#25282D`, accent `#E0764A`.
- **Accent usage:** links, focus rings, active nav state, section markers, and the
  organisation label in work-item rows. Nothing else.
- **Type:** grotesk for headings and body, monospace for labels and metadata (dates, tech
  tags, section markers). Display sizes carry negative tracking (about `-0.035em`). Prose is
  set to roughly a 56–68 character measure.
- **Spacing:** 8px scale, on a consistent baseline rhythm.
- **Structure:** hairline rules instead of boxes and cards.
- **Motion:** capped at ~180ms; `prefers-reduced-motion` honoured.
- **Fonts:** self-hosted and subset at build time. No CDN, no layout shift.
- **Accessibility:** all token pairs must meet WCAG AA contrast; visible focus rings; the
  rail collapses to a top bar on narrow viewports.

### 5.5 Content model

`content/site.toml` holds bio, links, and metadata. `content/projects.yaml` holds project
entries. `resume.json` follows the JSON Resume schema, restricted to the fields the resume
actually uses: `basics`, `work[]`, `education[]`, `skills[]`, `certificates[]` (CFA), and
`awards[]` (NSA Codebreaker) — the standard JSON Resume field names, not invented ones. All three are
deserialised through `content-model` with `deny_unknown_fields`.

### 5.6 Copy decisions

Bio is first person. The lede is:

> I build the infrastructure that gets AI systems safely into production — deployment,
> observability, and the trust boundaries in between.

The framing follows the resume's actual through-line: repeatedly owning the layer where a
system meets production and has to be trusted (BYOC deployment, zero-trust gateways, fraud
prevention, underwriting controls). The NSA Codebreaker result (one of 24 finishers out of
3,300+) is promoted out of Selected Work to a standalone credentials line. The CFA charter
appears on the About page, not the hero.

## 6. Error Handling

The build fails loudly rather than degrading. Missing `resume.json`, unknown fields in any
content file, a dead internal link, or a referenced asset that does not exist are all hard
errors. Dead internal links are the notable one: a generator that knows all of its own routes
can prove they resolve, making broken internal links unshippable rather than something
discovered later.

## 7. Testing

- Snapshot tests on rendered components.
- Serde round-trip tests against `resume.json` and `projects.yaml` fixtures.
- A build invariant test: every route reachable, every internal link resolves, every
  referenced asset exists.
- `wasm-bindgen-test` for island logic, with DOM glue kept thin.
- `cargo test` and `cargo clippy -D warnings` gate deployment in CI.

## 8. Delivery Buckets

Each bucket ends deployed and working. The site is never half-migrated. Review checkpoint
after each.

**Bucket 1 — Generator and design system.**
The existing four pages rendered by the Rust generator in composition A, with the real bio and
the design system from 5.4. PDF flow preserved exactly as today. No wasm.
*Done when:* the deployed site renders all four pages from Rust, light and dark both work, the
resume PDF still downloads, and CI gates on tests and clippy.

**Bucket 2 — Resume pipeline.**
*Cross-repo prerequisite:* the private LaTeX repo changes so `resume.json` is the source of
truth, the Makefile generates the `.tex` from it, and the release publishes `resume.json`
alongside the PDF.
*Done when:* the resume page renders as real HTML from `resume.json`, the Download PDF button
still serves the release PDF, and the PDF's own content is unchanged.

**Bucket 3 — Projects.**
Real project copy, plus the projects browser island. First WASM in production.
*Done when:* the projects list renders statically and works with JS disabled, and the island
adds filter and search when it loads.

**Bucket 4 — Demos.**
The demo harness plus the first demo. The specific demo is deliberately unchosen; the author
expressed interest in compute/visual toys, client-side tools, and infra simulations, and the
harness supports all three. Buckets 1–3 do not depend on this choice.
*Done when:* a demo page loads its own wasm binary independently, and no other page's payload
grows as a result.

## 9. Out of Scope

- Server-side rendering at request time (impossible on Pages).
- WASM threads and `SharedArrayBuffer`.
- A blog, CMS, analytics, or comments.
- Client-side PDF rendering.
- Migrating to Leptos/Dioxus hydration now. Explicitly deferred, not precluded.

## 10. Housekeeping Found During Design

`.gitignore` line 3 was `set_env.sh/assets/paul_maxwell_resume.pdf` — two entries collapsed
onto one line, matching a path that does not exist. Neither `set_env.sh` nor the resume PDF
was actually ignored. Split into separate entries, with `.superpowers/` added.

Checked for fallout: `git log --all -- set_env.sh` returns nothing and the file is absent from
the working tree, so no secrets were exposed by the broken rule. The resume PDF was left
untracked by luck rather than by the ignore rule; it is build output and is now correctly
ignored.

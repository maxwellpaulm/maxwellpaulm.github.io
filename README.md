# paul-maxwell.com

My personal site. It's a static site generator written in Rust — no Jekyll, no
Next.js, no npm — that renders a handful of pages, compiles two interactive
demos to WebAssembly, and deploys itself to GitHub Pages.

Live at **[paul-maxwell.com](https://paul-maxwell.com)**.

## Why it's built this way

A personal site is small enough that a framework is mostly overhead, and static
enough that a build step can enforce real invariants. So the generator is a
plain Rust binary: content in TOML, markup in [maud](https://maud.lambda.xyz/)
templates, one stylesheet emitted from a shared palette, and a `checks` pass
that fails the build rather than shipping something broken.

Concretely, the build refuses to produce a site where:

- an internal link, asset reference, or CSS `url()` points at a file that
  doesn't exist (`crates/site/src/checks.rs`),
- a colour pair in either theme falls below the WCAG AA contrast ratio
  (`crates/site/src/theme.rs` computes the ratios in a test),
- a page claims the wrong canonical URL, or the sitemap drifts from the real
  route set — both are generated from `Route::ALL`, so they can't disagree,
- an inline script changes without the Content-Security-Policy being updated to
  match (see [Security](#security)).

164 tests across four crates. Most of them exist to pin an invariant that would
otherwise be silently breakable — including cross-file ones that assert a CSS
class the JavaScript writes is actually defined in the stylesheet.

## Layout

```
content/         site.toml — all copy, work history, and metadata
crates/site/     the generator: routes, pages, components, theme, checks
crates/demos/    reaction-diffusion and aho-corasick, compiled to wasm
crates/ask/      a BM25 search engine (parked — see below)
static/          fonts, icons, hand-written JS, wasm build output
scripts/         resume fetch/render, wasm build, CSP hash gate
security/        the CSP and edge headers, with rationale per directive
docs/            design specs and implementation plans
```

## Build

```sh
./scripts/build-wasm.sh      # compile the demo crates to wasm (needs wasm-bindgen 0.2.127)
cargo run -p site            # render the site into dist/
cargo test --all             # 164 tests
cargo fmt --all              # rustfmt.toml keeps the codebase's compact style
```

`cargo run -p site -- --strict` is what CI runs: it additionally fails if the
resume artifacts are missing, which is intentional — a local build without the
private resume PDF should still work, a deploy without it should not.

The resume itself is fetched from a private repo's latest release
(`scripts/fetch-resume.sh`) and rasterised to one SVG per page
(`scripts/render-resume.sh`), so `/resume/` displays the real document inline
rather than an embedded PDF viewer.

## Demos

Both are Rust compiled to WebAssembly, rendered to a canvas, with the
simulation state living entirely on the Rust side:

- **[Reaction–diffusion](https://paul-maxwell.com/demos/reaction-diffusion/)** —
  a Gray–Scott simulation you can paint into. `docs/superpowers/wasm-benchmark.md` measures
  it against a JavaScript implementation, because "wasm is faster" deserved a
  number rather than an assumption.
- **[Aho–Corasick](https://paul-maxwell.com/demos/aho-corasick/)** — the string
  matching automaton behind a rewrite on the projects page, drawn as a live
  graph: watch the trie build, then step the scan and see failure links fire.

## Easter eggs

Two, documented plainly here on the theory that anyone reading the source has
earned them.

**CRT mode.** Type the Konami code — `↑ ↑ ↓ ↓ ← → ← → B A` — anywhere on the
site. The page snaps into a green-phosphor CRT: scanlines, phosphor glow, red/
blue colour fringing, a slow flicker, and a glitch jitter on entry. `Esc` or
re-entering the code exits. It's implemented as a third palette in
`theme.rs`, so it recolours the whole site through the same design tokens light
and dark mode use — and passes the same WCAG contrast test they do.
(`static/crt.js`)

**Asteroids.** Click the `PM` monogram in the nav rail. A wireframe ship spawns
over the live page: `←`/`→` rotate, `↑` thrusts with momentum, `space` fires.
Bullets destroy individual words of the page, which explode into particles;
touching a surviving word ends the run with your word count. `Esc` restores the
page exactly as it was. Shot words hide via `visibility` rather than `display`
so the layout — and every remaining hitbox — stays put mid-game.
(`static/ship.js`)

They compose: enter CRT mode first and you fly in green phosphor.

## Parked: the ask terminal

`crates/ask` is a BM25 retrieval engine — tokenizer, stopwords, stemmer, and
ranking — that powered an `/ask/` page: you asked a question in plain English
and it returned a passage I'd actually written, ranked in the browser, with a
link to its source page. No LLM, deliberately, so the site couldn't invent
claims about my career.

It works and it's fully tested, but a search box over ~12 passages is a less
interesting artifact than it sounds, so the route is parked rather than
deleted. `Route::ALL` documents the four commented-out lines that bring it
back, and a test (`parked_ask_terminal_publishes_nothing`) fails loudly if it's
ever only half-restored.

## Security

The site makes no external requests — fonts, styles, images, and wasm are all
same-origin — which makes a strict Content-Security-Policy practical:

```
default-src 'none'; script-src 'self' <2 sha256 hashes> 'wasm-unsafe-eval';
style-src 'self'; img-src 'self'; font-src 'self'; connect-src 'self';
base-uri 'none'; form-action 'none'; frame-ancestors 'none'
```

The two hashes cover the site's only inline scripts (theme restore and the
theme toggle). Because a CSP that silently drifts from the code is worse than
none, `scripts/check-csp-hashes.sh` recomputes the hash of every inline script
actually shipped in `dist/` and fails CI unless the set matches
`security/csp-hashes.txt` exactly — in both directions, so a stale allowance is
as much of an error as a missing one. `security/cloudflare-headers.md` is the
paste-ready reference, with a rationale for every directive.

CI also runs `cargo deny` (licences and advisories), `zizmor` (GitHub Actions
misconfiguration), and `gitleaks` on a schedule, and every third-party action
is pinned to a full commit SHA rather than a tag.

## Deploy

Push to `master`. The workflow checks formatting, lints, tests, fetches and renders the resume,
builds the wasm, generates the site, verifies the CSP hashes and that the
expected artifacts actually shipped, then publishes to GitHub Pages. Cloudflare
sits in front for the edge headers.

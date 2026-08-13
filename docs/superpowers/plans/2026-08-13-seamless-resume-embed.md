# Seamless Resume Embed Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the browser's PDF viewer on `/resume/` with the resume rendered inline as vector SVG, plus a hidden text layer, so it looks like part of the page on every device and is readable by screen readers and crawlers.

**Architecture:** A build-time script converts the release PDF to one SVG per page and extracts its text with poppler. The generator discovers those artifacts and renders them as images with a visually-hidden text block. The PDF download link stays. No change to the upstream LaTeX repository.

**Tech Stack:** poppler (`pdftocairo`, `pdftotext`, `pdfinfo`), Rust 1.92, `maud` 0.27.

## Why vector rather than raster

Measured on the real resume (1 page, US Letter):

| Output | Size | Verdict |
|---|---|---|
| PNG 110 dpi | 329 KB | soft on retina displays |
| PNG grayscale 150 dpi | 199 KB | best raster, still degrades when zoomed |
| JPEG q88 150 dpi | 398 KB | larger *and* ringing artifacts on text |
| **SVG (vector)** | **592 KB raw, 69 KB gzipped** | crisp at any zoom |

GitHub Pages serves SVG with `content-encoding: gzip` (verified against the live `favicon.svg`), so 69 KB is what a visitor downloads. `pdftocairo -svg` outlines glyphs as paths, so the SVG carries no selectable text — which is exactly what the hidden text layer is for.

## Global Constraints

Every task's requirements implicitly include this section.

- **Zero `#[allow(dead_code)]` anywhere in the tree.** There are none today.
- CI gates deploy on `cargo clippy --all-targets -- -D warnings` and `cargo test --all`. A red build must never publish.
- No new Rust dependencies, no npm, no bundler.
- **No external hosts.** Everything self-hosted.
- The generated SVG and text are **build artifacts, not source** — git-ignored, never committed, exactly like the wasm.
- **The PDF download link must remain**, and must keep pointing at `/assets/paul_maxwell_resume.pdf`. It is what recruiters actually want.
- The resume page must remain legible in both themes; light is the site default.
- `content/site.toml` prose is approved and fixed — do not reword existing copy.
- The inline theme scripts in `shell.rs`/`rail.rs` use double-quoted JS deliberately, so their un-escaped tests are falsifying. Do not touch them.
- Existing behaviour that must not regress: `checks::verify` fails the build on any dead internal link or missing asset; the demo page keeps its own canonical; wasm loads only on the demo page.

**A note on tests, carried from earlier buckets:** this project has produced five tests that could not fail, every one because the input made the assertion trivially true. **If a closed-form expected value exists, assert it.** Prefer equality over `assert_ne!` and over threshold comparisons.

---

### Task 1: Render the PDF to SVG and text

**Files:**
- Create: `scripts/render-resume.sh`
- Modify: `.gitignore`

**Interfaces:**
- Consumes: `assets/paul_maxwell_resume.pdf` (fetched from the private release by `scripts/fetch-resume.sh`).
- Produces: `static/resume/page-N.svg` for each page, one-indexed and zero-padded to two digits (`page-01.svg`), plus `static/resume/resume.txt`. Task 2 discovers these; Task 3 builds them in CI.

- [ ] **Step 1: Write the script**

Create `scripts/render-resume.sh` and `chmod +x` it:

```bash
#!/bin/bash
# Renders the resume PDF to one SVG per page plus a plain-text extraction,
# for inline display on /resume/. Run after scripts/fetch-resume.sh and
# before `cargo run -p site`.
#
# Vector rather than raster: the SVG is ~69 KB gzipped and stays crisp at
# any zoom, where a 150 dpi PNG is ~199 KB and goes soft.
set -euo pipefail

PDF=assets/paul_maxwell_resume.pdf
OUT=static/resume

if [ ! -f "$PDF" ]; then
    echo "Error: $PDF not found — run scripts/fetch-resume.sh first" >&2
    exit 1
fi

PAGES=$(pdfinfo "$PDF" | awk '/^Pages:/ { print $2 }')
if [ -z "$PAGES" ] || [ "$PAGES" -lt 1 ]; then
    echo "Error: could not determine page count of $PDF" >&2
    exit 1
fi

rm -rf "$OUT"
mkdir -p "$OUT"

for p in $(seq 1 "$PAGES"); do
    printf -v name "page-%02d.svg" "$p"
    pdftocairo -svg -f "$p" -l "$p" "$PDF" "$OUT/$name"
done

pdftotext -layout "$PDF" "$OUT/resume.txt"

echo "Rendered $PAGES page(s) to $OUT:"
ls -la "$OUT"
```

`-f`/`-l` bound the page range, because `pdftocairo -svg` writes a single file and would otherwise emit only the first page.

- [ ] **Step 2: Ignore the artifacts**

Append to `.gitignore`:

```
/static/resume/
```

Confirm with `git status --short` that nothing under `static/resume/` is staged. The `.gitignore` already carries `/static/demos/reaction-diffusion/` for the wasm; this is the same pattern.

- [ ] **Step 3: Run it and verify**

```bash
./scripts/render-resume.sh
ls -la static/resume/
head -3 static/resume/resume.txt
gzip -c static/resume/page-01.svg | wc -c
```

Expected: one `page-01.svg`, a `resume.txt` beginning with "Paul Maxwell", and a gzipped SVG in the region of 69 KB. Record the actual sizes in your report.

- [ ] **Step 4: Commit**

```bash
git add scripts/render-resume.sh .gitignore
git commit -m "feat: render the resume PDF to vector SVG and text"
```

---

### Task 2: Render the resume page from those artifacts

**Files:**
- Modify: `crates/site/src/pages/resume.rs`, `crates/site/src/build.rs`, `crates/site/src/theme.rs`

**Interfaces:**
- Consumes: `static/resume/page-*.svg` and `static/resume/resume.txt` (Task 1).
- Produces: `pages::resume::render(site: &Site, pages: &[String], text: &str) -> Markup`, where `pages` holds rooted URLs such as `/resume/page-01.svg` in page order, and `text` is the extracted plain text. `build.rs` discovers the artifacts and passes them in.

The page module stays a pure function of its inputs — filesystem discovery belongs in `build.rs`, which already owns that responsibility.

- [ ] **Step 1: Write the failing tests**

Replace the test module in `crates/site/src/pages/resume.rs` with:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn pages() -> Vec<String> {
        vec!["/resume/page-01.svg".to_string(), "/resume/page-02.svg".to_string()]
    }

    #[test]
    fn resume_shows_one_image_per_page_in_order() {
        let out = render(&crate::content::fixture_site(), &pages(), "Paul Maxwell").into_string();
        let first = out.find("/resume/page-01.svg").expect("page 1 missing");
        let second = out.find("/resume/page-02.svg").expect("page 2 missing");
        assert!(first < second, "pages rendered out of order");
        assert_eq!(out.matches("<img").count(), 2, "expected exactly one img per page");
    }

    #[test]
    fn resume_keeps_the_pdf_download() {
        let out = render(&crate::content::fixture_site(), &pages(), "text").into_string();
        assert!(out.contains(&format!(r#"href="{RESUME_PDF}""#)), "download link missing");
        assert!(out.contains("download"), "download attribute missing");
    }

    #[test]
    fn resume_embeds_the_extracted_text_for_screen_readers() {
        let out = render(&crate::content::fixture_site(), &pages(), "Aho-Corasick").into_string();
        assert!(out.contains("visually-hidden"), "hidden text container missing");
        assert!(out.contains("Aho-Corasick"), "extracted text not embedded");
    }

    #[test]
    fn resume_without_rendered_pages_still_offers_the_download() {
        let out = render(&crate::content::fixture_site(), &[], "").into_string();
        assert!(out.contains(&format!(r#"href="{RESUME_PDF}""#)), "download link missing");
        assert_eq!(out.matches("<img").count(), 0, "no pages should render no images");
    }

    #[test]
    fn extracted_text_is_escaped_not_injected() {
        let out = render(&crate::content::fixture_site(), &pages(), "a <script>x</script> b").into_string();
        assert!(!out.contains("<script>x</script>"), "raw markup leaked from the PDF text");
        assert!(out.contains("&lt;script&gt;"), "text should be HTML-escaped");
    }
}
```

That last test matters: the text comes from a PDF, and `maud` escapes interpolated values by default — this pins that nobody later "fixes" it with `PreEscaped`.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p site resume`
Expected: FAIL — `render` takes the wrong number of arguments.

- [ ] **Step 3: Implement the page**

Replace the body of `crates/site/src/pages/resume.rs` above the tests:

```rust
use crate::components::shell;
use crate::content::Site;
use crate::route::Route;
use maud::{html, Markup};

/// Published location of the PDF fetched from the private release.
pub const RESUME_PDF: &str = "/assets/paul_maxwell_resume.pdf";

/// Renders the resume as inline vector pages with a hidden text layer.
///
/// `pages` holds rooted URLs in page order; `text` is the plain-text
/// extraction. With no pages — a local build that has not run
/// `scripts/render-resume.sh` — the page degrades to the download link.
pub fn render(site: &Site, pages: &[String], text: &str) -> Markup {
    let main = html! {
        h1 { "Resume" }
        p .prose {
            a href=(RESUME_PDF) download { "Download PDF" }
        }

        @for (i, page) in pages.iter().enumerate() {
            img .resume-page src=(page) alt=(format!("Resume page {} of {}", i + 1, pages.len()));
        }

        @if !text.is_empty() {
            div .visually-hidden { (text) }
        }
    };
    shell::layout(site, Route::Resume, "Resume", main)
}
```

- [ ] **Step 4: Style the pages**

In `theme::stylesheet()`, replace nothing and add — remembering that every literal brace is doubled inside the `format!` raw string:

```css
.resume-page {{
  display: block;
  width: 100%;
  max-width: 780px;
  height: auto;
  margin: calc(var(--space) * 3) 0;
  border: 1px solid var(--rule);
  background: #FFFFFF;
}}
.visually-hidden {{
  position: absolute;
  width: 1px;
  height: 1px;
  margin: -1px;
  padding: 0;
  overflow: hidden;
  clip: rect(0 0 0 0);
  white-space: nowrap;
  border: 0;
}}
```

The explicit white background is deliberate: the resume is a document with its own white page, and it must not become transparent over the dark theme's near-black paper.

- [ ] **Step 5: Discover the artifacts in the build**

In `build.rs`, before rendering routes, read the artifacts from `root`:

```rust
/// Finds the rendered resume pages and extracted text, if they exist.
/// Returns rooted URLs in page order.
fn resume_artifacts(root: &Path) -> Result<(Vec<String>, String)> {
    let dir = root.join("static/resume");
    if !dir.exists() {
        return Ok((Vec::new(), String::new()));
    }
    let mut pages: Vec<String> = std::fs::read_dir(&dir)?
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| n.starts_with("page-") && n.ends_with(".svg"))
        .map(|n| format!("/resume/{n}"))
        .collect();
    pages.sort();
    let text = std::fs::read_to_string(dir.join("resume.txt")).unwrap_or_default();
    Ok((pages, text))
}
```

Sorting the zero-padded names gives page order, which is why Task 1 pads them.

Then pass them into the `Route::Resume` arm, and add the strict check alongside the existing resume-PDF one:

```rust
    let (resume_pages, resume_text) = resume_artifacts(root)?;
    if resume_pages.is_empty() {
        let msg = "resume pages not rendered — run scripts/render-resume.sh";
        if strict {
            anyhow::bail!("{msg}");
        }
        eprintln!("warning: {msg} (non-strict build, continuing)");
    }
```

- [ ] **Step 6: Add build tests**

```rust
    #[test]
    fn build_embeds_the_rendered_resume_pages() {
        let tmp = std::env::temp_dir().join("site-resume-test");
        let _ = std::fs::remove_dir_all(&tmp);
        build(Path::new("../.."), &tmp, false).expect("build succeeds");

        let html = std::fs::read_to_string(tmp.join("resume/index.html")).unwrap();
        let (pages, _) = resume_artifacts(Path::new("../..")).unwrap();
        for page in &pages {
            assert!(html.contains(page.as_str()), "resume page {page} not referenced");
        }
        assert!(
            html.contains("/assets/paul_maxwell_resume.pdf"),
            "download link missing from the built page"
        );
        assert!(!html.contains("<object"), "the old PDF object embed should be gone");
        std::fs::remove_dir_all(&tmp).ok();
    }
```

- [ ] **Step 7: Verify**

```bash
./scripts/render-resume.sh
cargo test --all
cargo clippy --all-targets -- -D warnings
cargo run -p site -- --strict
grep -o '<img[^>]*resume-page[^>]*>' dist/resume/index.html
grep -c 'visually-hidden' dist/resume/index.html
grep -c '<object' dist/resume/index.html      # expect 0
ls -la dist/resume/
```

Then serve `dist/` and open `/resume/` in a browser. Confirm: the resume renders inline at full width with a hairline border, no browser PDF toolbar, the download link works, and it is legible in both themes. Report what you saw.

- [ ] **Step 8: Commit**

```bash
git add crates/site/src
git commit -m "feat: embed the resume as inline vector pages with a hidden text layer"
```

---

### Task 3: Build the artifacts in CI

**Files:**
- Modify: `.github/workflows/deploy.yml`

**Interfaces:**
- Consumes: `scripts/render-resume.sh`.
- Produces: a deployment containing the rendered resume.

Without this, CI publishes a resume page whose images 404 — and no test catches it, because every test runs where the artifacts already exist locally. This is the same trap the wasm build had, and it gets the same explicit verification step.

- [ ] **Step 1: Install poppler and render**

In `.github/workflows/deploy.yml`, after the `Fetch resume` step and before `Build site`:

```yaml
    - name: Install poppler
      run: sudo apt-get update && sudo apt-get install -y poppler-utils

    - name: Render resume
      run: ./scripts/render-resume.sh
```

It must follow `Fetch resume`, since the script needs the downloaded PDF, and precede `Build site`, since the generator copies `static/` into `dist/`.

- [ ] **Step 2: Verify the artifacts shipped**

Extend the existing `Verify wasm shipped` step, or add alongside it:

```yaml
    - name: Verify resume shipped
      run: |
        test -f dist/resume/page-01.svg
        test -f dist/assets/paul_maxwell_resume.pdf
        ls -la dist/resume/
```

- [ ] **Step 3: Validate and commit**

```bash
python3 -c "import yaml; d=yaml.safe_load(open('.github/workflows/deploy.yml')); [print(f'{i+1}. ' + (s.get('name') or s.get('uses'))) for i,s in enumerate(d['jobs']['build']['steps'])]"
cargo test --all
cargo clippy --all-targets -- -D warnings
```

Confirm by eye that `Fetch resume` → `Render resume` → `Build site` → `Verify resume shipped` appear in that order, and that the `deploy` job is unchanged.

```bash
git add .github/workflows/deploy.yml
git commit -m "ci: render the resume to SVG before building the site"
```

---

## Definition of Done

- `/resume/` shows the resume inline as crisp vector pages, with no browser PDF viewer chrome, on desktop and mobile alike.
- The PDF download link still works and still points at the release asset.
- The extracted text ships in a visually-hidden block, so screen readers and crawlers get real text where they previously got none.
- The page is legible in both themes, with the document's white page preserved.
- A local build without rendered artifacts warns and degrades to the download link; a strict build fails.
- CI installs poppler, renders the pages, and fails the build if they are missing.
- `cargo test --all` and `cargo clippy --all-targets -- -D warnings` pass; zero `#[allow(dead_code)]`.

use crate::checks;
use crate::content::Site;
use crate::pages::resume::ResumePage;
use crate::{pages, route::Route, theme};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

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

/// Build artifacts that must never be published: licence notes meant for
/// contributors reading the repo, and placeholder dotfiles that keep empty
/// directories tracked in git.
fn is_publish_excluded(name: &std::ffi::OsStr) -> bool {
    name == "README.md" || name.to_string_lossy().starts_with('.')
}

fn copy_tree(from: &Path, to: &Path, written: &mut Vec<PathBuf>) -> Result<()> {
    if !from.exists() {
        return Ok(());
    }
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        if is_publish_excluded(&entry.file_name()) {
            continue;
        }
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

/// Escapes XML metacharacters for use in text/attribute content. `&` must be
/// replaced first, or the entities introduced for `<`/`>` would themselves
/// be escaped again.
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// `User-agent: *\nAllow: /\n\nSitemap: {url}/sitemap.xml\n`
///
/// Plain text, not XML, so `site.url` is interpolated as-is here.
fn robots_txt(site: &Site) -> String {
    format!("User-agent: *\nAllow: /\n\nSitemap: {}/sitemap.xml\n", site.url)
}

/// A standard sitemap with one `<url>` per route — generated from
/// `Route::ALL` so it cannot drift from the real route set — plus `extra`
/// absolute paths for real, linkable pages that aren't a top-level `Route`
/// (the reaction-diffusion demo, nested under `/demos/`).
fn sitemap_xml(site: &Site, extra: &[&str]) -> String {
    let mut urls = String::new();
    for route in Route::ALL {
        urls.push_str(&format!(
            "  <url><loc>{}{}</loc></url>\n",
            escape_xml(&site.url),
            escape_xml(route.path())
        ));
    }
    for path in extra {
        urls.push_str(&format!(
            "  <url><loc>{}{}</loc></url>\n",
            escape_xml(&site.url),
            escape_xml(path)
        ));
    }
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n\
         {urls}\
         </urlset>\n"
    )
}

/// Parses the leading numeric value out of an SVG length attribute value
/// such as `612pt` or `792` — the unit is ignored, since only the ratio
/// between width and height matters here (the CSS governs actual display
/// size).
fn parse_svg_length(value: &str) -> Option<u32> {
    let numeric: String = value.chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
    if numeric.is_empty() {
        return None;
    }
    numeric.parse::<f64>().ok().map(|n| n.round() as u32)
}

/// Extracts `attr="value"` from an SVG's opening `<svg ...>` tag and parses
/// its leading numeric length.
fn svg_attr_length(open_tag: &str, attr: &str) -> Option<u32> {
    let marker = format!("{attr}=\"");
    let start = open_tag.find(&marker)? + marker.len();
    let value = &open_tag[start..];
    let end = value.find('"')?;
    parse_svg_length(&value[..end])
}

/// Reads the `width`/`height` declared on an SVG's root element. These
/// files are produced by our own `pdftocairo` render step, so if they
/// aren't there, something about the render is genuinely broken — fail
/// loudly rather than silently shipping an image with no reserved space.
fn svg_dimensions(svg: &str) -> Result<(u32, u32)> {
    // Locate the `<svg` tag rather than assuming it opens the file: an XML
    // declaration (`<?xml version="1.0"?>`) usually precedes it, and that
    // itself contains a `>` — the first in the document — so scanning for
    // the plain first `>` would truncate the tag before it even starts.
    let tag_start = svg.find("<svg").context("no <svg> root element found")?;
    let tag_end = svg[tag_start..].find('>').map(|i| tag_start + i + 1).unwrap_or(svg.len());
    let open_tag = &svg[tag_start..tag_end];
    let width = svg_attr_length(open_tag, "width").context("no parsable width on <svg> root")?;
    let height = svg_attr_length(open_tag, "height").context("no parsable height on <svg> root")?;
    Ok((width, height))
}

/// Finds the rendered resume pages, if any exist, in page order.
fn resume_artifacts(root: &Path) -> Result<Vec<ResumePage>> {
    let dir = root.join("static/resume");
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut names: Vec<String> = std::fs::read_dir(&dir)?
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| n.starts_with("page-") && n.ends_with(".svg"))
        .collect();
    names.sort();

    let mut pages = Vec::with_capacity(names.len());
    for name in names {
        let path = dir.join(&name);
        let svg = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        let (width, height) = svg_dimensions(&svg)
            .with_context(|| format!("{} has no parsable svg dimensions", path.display()))?;
        pages.push(ResumePage { url: format!("/resume/{name}"), width, height });
    }

    Ok(pages)
}

/// Renders the whole site into `out`. With `strict`, a missing resume PDF is a
/// hard error; without it, a warning — so local builds work without the token
/// that CI uses to fetch the private release.
pub fn build(root: &Path, out: &Path, strict: bool) -> Result<Vec<PathBuf>> {
    let site = Site::load(&root.join(CONTENT))?;
    let mut written = Vec::new();

    let resume_pages = resume_artifacts(root)?;
    if resume_pages.is_empty() {
        let msg = "resume pages not rendered — run scripts/render-resume.sh";
        if strict {
            anyhow::bail!("{msg}");
        }
        eprintln!("warning: {msg} (non-strict build, continuing)");
    }

    for route in Route::ALL {
        let markup = match route {
            Route::Index => pages::index::render(&site),
            Route::About => pages::about::render(&site),
            Route::Projects => pages::projects::render(&site),
            Route::Resume => pages::resume::render(&site, &resume_pages),
            Route::Demos => pages::demos::render(&site),
        };
        write(&out.join(route.output_path()), &markup.into_string(), &mut written)?;
    }

    write(&out.join("style.css"), &theme::stylesheet(), &mut written)?;
    write(&out.join("robots.txt"), &robots_txt(&site), &mut written)?;
    write(
        &out.join("sitemap.xml"),
        &sitemap_xml(&site, &[pages::demos::REACTION_DIFFUSION]),
        &mut written,
    )?;
    write(
        &out.join("404.html"),
        &pages::not_found::render(&site).into_string(),
        &mut written,
    )?;
    write(
        &out.join("demos/reaction-diffusion/index.html"),
        &pages::demo_reaction_diffusion::render(&site).into_string(),
        &mut written,
    )?;
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

    let cname = std::fs::read_to_string(root.join("CNAME"))
        .context("reading CNAME — required for the custom domain")?;
    write(&out.join("CNAME"), &cname, &mut written)?;

    checks::verify(out, strict)?;

    Ok(written)
}

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
        assert!(tmp.join("CNAME").exists(), "CNAME missing — custom domain would break");
        assert!(!tmp.join("fonts/README.md").exists(), "licence notes must not be published");
        assert!(!tmp.join("assets/.gitkeep").exists(), "dotfiles must not be published");
        assert!(written.len() >= 5);

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn build_writes_robots_and_sitemap_covering_every_route() {
        let tmp = std::env::temp_dir().join("site-build-test-seo");
        let _ = std::fs::remove_dir_all(&tmp);

        build(Path::new("../.."), &tmp, false).expect("build succeeds");

        let site = Site::load(Path::new("../../content/site.toml")).unwrap();

        let robots = std::fs::read_to_string(tmp.join("robots.txt")).expect("robots.txt written");
        assert!(robots.contains("User-agent: *"));
        assert!(robots.contains("Allow: /"));
        assert!(robots.contains(&format!("Sitemap: {}/sitemap.xml", site.url)));

        let sitemap =
            std::fs::read_to_string(tmp.join("sitemap.xml")).expect("sitemap.xml written");
        assert!(sitemap.contains(r#"xmlns="http://www.sitemaps.org/schemas/sitemap/0.9""#));
        for route in Route::ALL {
            let loc = format!("<loc>{}{}</loc>", site.url, route.path());
            assert!(sitemap.contains(&loc), "sitemap missing {loc}: {sitemap}");
        }
        assert!(sitemap.contains("/demos/"), "demos must be in the sitemap from bucket 4 on");

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn sitemap_escapes_xml_metacharacters_in_the_url() {
        let site = Site {
            name: "Test".to_string(),
            location: "Nowhere".to_string(),
            role: "Tester".to_string(),
            lede: "Lede".to_string(),
            bio: "Bio".to_string(),
            credential: "Credential".to_string(),
            description: "Description".to_string(),
            url: "https://example.com/?a=1&b=2".to_string(),
            projects_intro: "Intro".to_string(),
            about: vec![],
            work: vec![],
        };

        let sitemap = sitemap_xml(&site, &[]);
        assert!(
            !sitemap.contains("&b=2"),
            "raw unescaped ampersand leaked into sitemap: {sitemap}"
        );
        assert!(
            sitemap.contains("https://example.com/?a=1&amp;b=2"),
            "expected escaped ampersand in <loc>: {sitemap}"
        );
    }

    #[test]
    fn build_writes_a_404_page_that_stays_out_of_the_sitemap() {
        let tmp = std::env::temp_dir().join("site-build-test-404");
        let _ = std::fs::remove_dir_all(&tmp);

        build(Path::new("../.."), &tmp, false).expect("build succeeds");

        let html = std::fs::read_to_string(tmp.join("404.html")).expect("404.html written");
        assert!(html.starts_with("<!DOCTYPE html>"));
        assert!(
            html.contains(&format!(r#"href="{}""#, Route::Index.path())),
            "404 page must link back to the index: {html}"
        );
        assert!(
            !html.contains(r#"aria-current="page""#),
            "404 rail must mark nothing current: {html}"
        );
        assert!(
            !html.contains(r#"rel="canonical""#),
            "404 page has no single stable URL, so it must not claim a canonical: {html}"
        );
        assert!(!html.contains("og:url"), "404 page must not emit og:url: {html}");
        assert!(
            html.contains(r#"<meta name="robots" content="noindex">"#),
            "404 page must be noindexed: {html}"
        );

        let sitemap = std::fs::read_to_string(tmp.join("sitemap.xml")).expect("sitemap written");
        assert!(!sitemap.contains("404"), "404 must not appear in the sitemap: {sitemap}");

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn build_writes_the_reaction_diffusion_demo_page_and_lists_it_in_the_sitemap() {
        let tmp = std::env::temp_dir().join("site-build-test-rd-demo");
        let _ = std::fs::remove_dir_all(&tmp);

        build(Path::new("../.."), &tmp, false).expect("build succeeds");
        let site = Site::load(Path::new("../../content/site.toml")).unwrap();

        assert!(
            tmp.join("demos/reaction-diffusion/index.html").exists(),
            "reaction-diffusion demo page not written"
        );

        // Not a `Route`, but a real, stable, linkable URL and the flagship
        // page of this bucket — it must be discoverable, not hidden because
        // of an implementation detail of the generator.
        let sitemap = std::fs::read_to_string(tmp.join("sitemap.xml")).expect("sitemap written");
        assert!(
            sitemap.contains(&format!(
                "<loc>{}{}</loc>",
                site.url,
                pages::demos::REACTION_DIFFUSION
            )),
            "demo page missing from the sitemap: {sitemap}"
        );

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn build_gives_every_real_route_its_own_canonical_and_no_noindex() {
        let tmp = std::env::temp_dir().join("site-build-test-canonical");
        let _ = std::fs::remove_dir_all(&tmp);

        build(Path::new("../.."), &tmp, false).expect("build succeeds");
        let site = Site::load(Path::new("../../content/site.toml")).unwrap();

        for route in Route::ALL {
            let html = std::fs::read_to_string(tmp.join(route.output_path())).unwrap();
            let canonical = format!(r#"<link rel="canonical" href="{}{}">"#, site.url, route.path());
            assert!(html.contains(&canonical), "{:?} missing its canonical: {html}", route);
            assert!(
                !html.contains("noindex"),
                "{:?} must not be noindexed: {html}",
                route
            );
        }

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn demo_page_claims_its_own_canonical_not_its_parent_routes() {
        let tmp = std::env::temp_dir().join("site-build-test-demo-canonical");
        let _ = std::fs::remove_dir_all(&tmp);

        build(Path::new("../.."), &tmp, false).expect("build succeeds");
        let site = Site::load(Path::new("../../content/site.toml")).unwrap();

        // `build_gives_every_real_route_its_own_canonical_and_no_noindex`
        // loops `Route::ALL` and so cannot see this page — it is nested
        // under `/demos/` but is not itself a `Route`.
        let html = std::fs::read_to_string(tmp.join("demos/reaction-diffusion/index.html"))
            .expect("demo page written");
        let expected_url = format!("{}{}", site.url, pages::demos::REACTION_DIFFUSION);
        assert!(
            html.contains(&format!(r#"<link rel="canonical" href="{expected_url}">"#)),
            "demo page canonical must be its own URL, not /demos/: {html}"
        );
        assert!(
            html.contains(&format!(r#"<meta property="og:url" content="{expected_url}">"#)),
            "demo page og:url must be its own URL, not /demos/: {html}"
        );
        assert!(!html.contains("noindex"), "demo page must not be noindexed: {html}");

        // Every real Route, plus the demo page above, still claims a
        // distinct, correct canonical of its own.
        for route in Route::ALL {
            let html = std::fs::read_to_string(tmp.join(route.output_path())).unwrap();
            let canonical = format!(r#"<link rel="canonical" href="{}{}">"#, site.url, route.path());
            assert!(html.contains(&canonical), "{:?} missing its canonical: {html}", route);
        }

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn wasm_loader_is_referenced_only_by_the_demo_page() {
        let tmp = std::env::temp_dir().join("site-build-test-wasm-scope");
        let _ = std::fs::remove_dir_all(&tmp);

        build(Path::new("../.."), &tmp, false).expect("build succeeds");

        for route in Route::ALL {
            let html = std::fs::read_to_string(tmp.join(route.output_path())).unwrap();
            assert!(
                !html.contains("loader.js"),
                "{:?} must not download the wasm demo loader: {html}",
                route
            );
        }
        let not_found = std::fs::read_to_string(tmp.join("404.html")).unwrap();
        assert!(
            !not_found.contains("loader.js"),
            "404 page must not download the wasm demo loader: {not_found}"
        );

        let demo_html = std::fs::read_to_string(tmp.join("demos/reaction-diffusion/index.html"))
            .expect("demo page written");
        assert!(
            demo_html.contains("loader.js"),
            "demo page must load the wasm demo: {demo_html}"
        );

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn build_embeds_the_rendered_resume_pages() {
        let tmp = std::env::temp_dir().join("site-resume-test");
        let _ = std::fs::remove_dir_all(&tmp);
        build(Path::new("../.."), &tmp, false).expect("build succeeds");

        let html = std::fs::read_to_string(tmp.join("resume/index.html")).unwrap();
        let pages = resume_artifacts(Path::new("../..")).unwrap();
        for page in &pages {
            assert!(html.contains(page.url.as_str()), "resume page {} not referenced", page.url);
        }
        // No count assertion here: `cargo test` runs before the artifacts
        // exist in CI (see .github/workflows/deploy.yml), so both sides of
        // any such count would read zero and the assertion would be inert.
        // The equivalent check that actually runs against real artifacts
        // lives in the "Verify resume shipped" CI step.
        assert!(
            html.contains("/assets/paul_maxwell_resume.pdf"),
            "download link missing from the built page"
        );
        assert!(!html.contains("<object"), "the old PDF object embed should be gone");
        std::fs::remove_dir_all(&tmp).ok();
    }

    fn write_svg(path: &Path, width: &str, height: &str) {
        std::fs::write(
            path,
            format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 612 792">
<rect width="10" height="10"/>
</svg>"#
            ),
        )
        .unwrap();
    }

    #[test]
    fn resume_artifacts_returns_pages_in_order_regardless_of_directory_order() {
        let dir = std::env::temp_dir().join("resume-artifacts-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("static/resume")).unwrap();
        // Created deliberately out of order — read_dir order is not guaranteed.
        for n in ["page-03.svg", "page-01.svg", "page-02.svg"] {
            write_svg(&dir.join("static/resume").join(n), "612pt", "792pt");
        }

        let pages = resume_artifacts(&dir).unwrap();
        let urls: Vec<&str> = pages.iter().map(|p| p.url.as_str()).collect();
        assert_eq!(
            urls,
            vec!["/resume/page-01.svg", "/resume/page-02.svg", "/resume/page-03.svg"],
            "pages must come back sorted by page number"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn resume_artifacts_is_empty_when_nothing_has_been_rendered() {
        let dir = std::env::temp_dir().join("resume-artifacts-empty-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let pages = resume_artifacts(&dir).unwrap();
        assert!(pages.is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn resume_artifacts_parses_each_pages_intrinsic_dimensions() {
        let dir = std::env::temp_dir().join("resume-artifacts-dimensions-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("static/resume")).unwrap();
        write_svg(&dir.join("static/resume/page-01.svg"), "612pt", "792pt");

        let pages = resume_artifacts(&dir).unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].width, 612, "width must be parsed from the svg root, ignoring the unit");
        assert_eq!(pages[0].height, 792, "height must be parsed from the svg root, ignoring the unit");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn resume_artifacts_fails_loudly_on_a_page_with_no_parsable_dimensions() {
        let dir = std::env::temp_dir().join("resume-artifacts-malformed-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("static/resume")).unwrap();
        std::fs::write(
            dir.join("static/resume/page-01.svg"),
            r#"<svg xmlns="http://www.w3.org/2000/svg"><rect/></svg>"#,
        )
        .unwrap();

        let err = resume_artifacts(&dir).expect_err("missing svg dimensions must error, not skip");
        assert!(
            err.to_string().contains("page-01.svg"),
            "error should name the offending file: {err}"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn build_copies_favicon_and_social_images() {
        let tmp = std::env::temp_dir().join("site-build-test-icons");
        let _ = std::fs::remove_dir_all(&tmp);

        build(Path::new("../.."), &tmp, false).expect("build succeeds");

        for asset in ["favicon.svg", "apple-touch-icon.png", "og-image.png"] {
            assert!(tmp.join(asset).exists(), "missing {asset} in build output");
        }

        std::fs::remove_dir_all(&tmp).ok();
    }
}

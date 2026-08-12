use anyhow::{bail, Result};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Pulls every value that follows `marker` up to the next `"` out of `text`.
fn extract_quoted(text: &str, marker: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find(marker) {
        rest = &rest[start + marker.len()..];
        if let Some(end) = rest.find('"') {
            found.push(rest[..end].to_string());
            rest = &rest[end..];
        }
    }
    found
}

/// Pulls every `href="..."`, `src="..."`, and `data="..."` value out of the HTML.
fn html_references(html: &str) -> Vec<String> {
    [r#"href=""#, r#"src=""#, r#"data=""#]
        .into_iter()
        .flat_map(|marker| extract_quoted(html, marker))
        .collect()
}

/// Pulls every `url("...")` value out of the CSS.
fn css_references(css: &str) -> Vec<String> {
    extract_quoted(css, r#"url(""#)
}

/// Maps a rooted site path to the file that must exist in `out`.
fn target(out: &Path, link: &str) -> PathBuf {
    let link = link
        .split(['#', '?'])
        .next()
        .expect("split always yields at least one element");
    let rel = link.trim_start_matches('/');
    if link.ends_with('/') || link.is_empty() {
        out.join(rel).join("index.html")
    } else {
        out.join(rel)
    }
}

fn files_with_extension(dir: &Path, ext: &str, acc: &mut Vec<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            files_with_extension(&path, ext, acc)?;
        } else if path.extension().is_some_and(|e| e == ext) {
            acc.push(path);
        }
    }
    Ok(())
}

/// Checks each rooted link's resolved target exists in `out`, recording any
/// miss (attributed to `source`) in `broken`.
fn check_links(source: &Path, links: &[String], out: &Path, strict: bool, broken: &mut BTreeSet<String>) {
    for link in links {
        if !link.starts_with('/') {
            continue; // external, anchor, or relative — out of scope
        }
        if !strict && link.starts_with("/assets/") {
            continue; // fetched from the private release; absent locally
        }
        let path = target(out, link);
        if !path.exists() {
            broken.insert(format!("{} → {}", source.display(), link));
        }
    }
}

/// Fails the build if any internal link, asset reference, or CSS `url()`
/// reference does not resolve to a file in the output directory.
pub fn verify(out: &Path, strict: bool) -> Result<()> {
    let mut pages = Vec::new();
    files_with_extension(out, "html", &mut pages)?;
    let mut stylesheets = Vec::new();
    files_with_extension(out, "css", &mut stylesheets)?;

    let mut broken = BTreeSet::new();
    for page in &pages {
        let html = std::fs::read_to_string(page)?;
        check_links(page, &html_references(&html), out, strict, &mut broken);
    }
    for sheet in &stylesheets {
        let css = std::fs::read_to_string(sheet)?;
        check_links(sheet, &css_references(&css), out, strict, &mut broken);
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
        let dir = scaffold("checks-dead", r#"<a href="/nonexistent-page/">Nonexistent</a>"#);
        let err = verify(&dir, true).expect_err("dead link must fail the build");
        assert!(err.to_string().contains("/nonexistent-page/"), "got: {err}");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rejects_a_missing_asset() {
        let dir = scaffold("checks-asset", r#"<img src="/logo.svg">"#);
        let err = verify(&dir, true).expect_err("missing asset must fail the build");
        assert!(err.to_string().contains("/logo.svg"), "got: {err}");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rejects_a_missing_css_asset() {
        let dir = scaffold("checks-css-missing", "<html></html>");
        std::fs::write(
            dir.join("style.css"),
            r#"@font-face { src: url("/fonts/Missing.woff2") format("woff2"); }"#,
        )
        .unwrap();

        let err = verify(&dir, true).expect_err("missing css asset must fail the build");
        assert!(err.to_string().contains("/fonts/Missing.woff2"), "got: {err}");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn accepts_a_css_reference_that_resolves() {
        let dir = scaffold("checks-css-ok", "<html></html>");
        std::fs::create_dir_all(dir.join("fonts")).unwrap();
        std::fs::write(dir.join("fonts/Inter.woff2"), b"").unwrap();
        std::fs::write(
            dir.join("style.css"),
            r#"@font-face { src: url("/fonts/Inter.woff2") format("woff2"); }"#,
        )
        .unwrap();

        assert!(verify(&dir, true).is_ok());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn strips_fragment_before_resolving() {
        let dir = scaffold(
            "checks-fragment",
            r#"<a href="/resume/#experience">Experience</a>"#,
        );
        std::fs::create_dir_all(dir.join("resume")).unwrap();
        std::fs::write(dir.join("resume/index.html"), "<!DOCTYPE html>").unwrap();

        assert_eq!(target(&dir, "/resume/#experience"), dir.join("resume/index.html"));
        assert!(verify(&dir, true).is_ok());
        std::fs::remove_dir_all(&dir).ok();
    }
}

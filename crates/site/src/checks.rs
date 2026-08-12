use anyhow::{bail, Result};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

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

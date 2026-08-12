use crate::content::Site;
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

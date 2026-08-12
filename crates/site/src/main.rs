mod build;
mod checks;
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

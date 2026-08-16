mod ask_index;
mod build;
mod checks;
mod components;
mod content;
mod pages;
mod route;
mod theme;

use anyhow::Result;
use std::path::Path;

fn main() -> Result<()> {
    let strict = std::env::args().any(|a| a == "--strict");
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = root.join("dist");

    if out.exists() {
        std::fs::remove_dir_all(&out)?;
    }
    let written = build::build(&root, &out, strict)?;
    println!("wrote {} files to {}", written.len(), out.display());
    Ok(())
}

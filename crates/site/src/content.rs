use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

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
    // TODO(task-7): remove once build.rs loads content/site.toml.
    #[allow(dead_code)]
    pub fn load(path: &Path) -> Result<Self> {
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("reading {}", path.display()))?;
        toml::from_str(&raw).with_context(|| format!("parsing {}", path.display()))
    }
}

#[cfg(test)]
pub fn fixture_site() -> Site {
    Site::load(std::path::Path::new("../../content/site.toml")).expect("content/site.toml loads")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_the_real_site_content() {
        let site = fixture_site();
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

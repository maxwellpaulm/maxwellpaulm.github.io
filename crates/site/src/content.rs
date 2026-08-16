use anyhow::{bail, Context, Result};
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
    /// Meta/OG description.
    pub description: String,
    /// Canonical base URL, no trailing slash.
    pub url: String,
    pub github: String,
    pub linkedin: String,
    pub projects_intro: String,
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
    #[serde(default)]
    pub detail: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AskContent {
    /// Real corpus vocabulary offered by the miss message ("try: …").
    pub suggest: Vec<String>,
    #[serde(default)]
    pub intent: Vec<Intent>,
    #[serde(default)]
    pub note: Vec<Note>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Intent {
    #[serde(rename = "match")]
    pub match_: Vec<String>,
    pub answer: String,
    pub source: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Note {
    pub q: String,
    pub a: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub source: String,
}

impl AskContent {
    pub fn load(path: &Path) -> Result<Self> {
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("reading {}", path.display()))?;
        Self::parse(&raw, path)
    }

    pub fn parse(raw: &str, path: &Path) -> Result<Self> {
        let ask: AskContent =
            toml::from_str(raw).with_context(|| format!("parsing {}", path.display()))?;
        ask.validate(path)?;
        Ok(ask)
    }

    /// Empty match lists or suggest terms would ship an intent that can
    /// never fire or a miss message with nothing to suggest — content
    /// mistakes that should fail the build loudly.
    fn validate(&self, path: &Path) -> Result<()> {
        if self.suggest.is_empty() {
            bail!("{}: `suggest` must not be empty", path.display());
        }
        for intent in &self.intent {
            if intent.match_.is_empty() {
                bail!("{}: every [[intent]] needs a non-empty `match` list", path.display());
            }
        }
        Ok(())
    }
}

impl Site {
    pub fn load(path: &Path) -> Result<Self> {
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("reading {}", path.display()))?;
        let site: Site =
            toml::from_str(&raw).with_context(|| format!("parsing {}", path.display()))?;
        site.validate(path)?;
        Ok(site)
    }

    /// Rejects content that would render a broken-looking page. Empty `work`
    /// or `about` produce a section header over nothing, or a bare heading —
    /// a content mistake should fail the build loudly rather than ship.
    fn validate(&self, path: &Path) -> Result<()> {
        if self.about.is_empty() {
            bail!("{}: `about` must not be empty", path.display());
        }
        if self.work.is_empty() {
            bail!("{}: `work` must not be empty", path.display());
        }
        Ok(())
    }
}

#[cfg(test)]
pub fn fixture_site() -> Site {
    Site::load(std::path::Path::new("../../content/site.toml")).expect("content/site.toml loads")
}

#[cfg(test)]
pub fn fixture_ask() -> AskContent {
    AskContent::load(std::path::Path::new("../../content/ask.toml")).expect("content/ask.toml loads")
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
        assert!(!site.github.is_empty(), "github must be set");
        assert!(!site.linkedin.is_empty(), "linkedin must be set");
        assert!(
            site.github.starts_with("https://"),
            "github should be a full URL: {}",
            site.github
        );
        assert!(
            site.linkedin.starts_with("https://"),
            "linkedin should be a full URL: {}",
            site.linkedin
        );
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let toml = r#"
name = "X"
location = "Y"
role = "Z"
lede = "L"
bio = "B"
projects_intro = "P"
about = []
surprise = "should not parse"
"#;
        let err = toml::from_str::<Site>(toml).expect_err("unknown field must fail");
        assert!(err.to_string().contains("surprise"), "got: {err}");
    }

    #[test]
    fn empty_work_is_rejected() {
        let mut site = fixture_site();
        site.work = vec![];
        let err = site
            .validate(Path::new("content/site.toml"))
            .expect_err("empty work must be rejected");
        assert!(err.to_string().contains("work"), "got: {err}");
        assert!(err.to_string().contains("site.toml"), "got: {err}");
    }

    #[test]
    fn empty_about_is_rejected() {
        let mut site = fixture_site();
        site.about = vec![];
        let err = site
            .validate(Path::new("content/site.toml"))
            .expect_err("empty about must be rejected");
        assert!(err.to_string().contains("about"), "got: {err}");
        assert!(err.to_string().contains("site.toml"), "got: {err}");
    }

    #[test]
    fn load_rejects_empty_about_end_to_end() {
        let tmp = std::env::temp_dir().join("site-content-empty-about.toml");
        std::fs::write(
            &tmp,
            r#"
name = "X"
location = "Y"
role = "Z"
lede = "L"
bio = "B"
description = "D"
url = "https://example.com"
github = "https://github.com/example"
linkedin = "https://www.linkedin.com/in/example"
projects_intro = "P"
about = []

[[work]]
title = "T"
org = "O"
year = "2020"
summary = "S"
"#,
        )
        .unwrap();

        let err = Site::load(&tmp).expect_err("empty about must fail Site::load");
        assert!(err.to_string().contains("about"), "got: {err}");

        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn ask_content_loads_and_exposes_intents_and_notes() {
        let ask = AskContent::load(Path::new("../../content/ask.toml")).unwrap();
        assert!(!ask.suggest.is_empty(), "suggest terms drive the miss message");
        assert!(!ask.intent.is_empty(), "starter content must include intents");
        assert!(!ask.note.is_empty(), "starter content must include notes");
        let who = ask.intent.iter().find(|i| i.match_.iter().any(|m| m == "who are you"));
        assert!(who.is_some(), "the canonical who-are-you intent is missing");
    }

    #[test]
    fn ask_content_rejects_an_empty_intent_match_list() {
        let toml = r#"
suggest = ["amazon"]
[[intent]]
match = []
answer = "x"
source = "/"
"#;
        let err = AskContent::parse(toml, Path::new("test.toml")).unwrap_err().to_string();
        assert!(err.contains("match"), "error should name the empty field: {err}");
    }
}

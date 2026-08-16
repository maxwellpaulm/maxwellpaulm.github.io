# Ask Terminal Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A first-class `/ask/` page where visitors type natural questions about Paul's work and get instant retrieval-only answers, ranked by a Rust BM25 searcher compiled to WASM.

**Architecture:** The site generator gains an `ask.toml` content file and an indexing step that emits `dist/ask/index.json` (the corpus: passages auto-derived from `site.toml` plus authored notes/intents). A new workspace crate `crates/ask` (package `ask-terminal`) implements tokenize→stopword→stem→BM25 plus intent matching, unit-tested natively and exposed to the page through wasm-bindgen. A hand-written module script `static/ask/terminal.js` fetches the corpus, instantiates the WASM, and renders a terminal UI.

**Tech Stack:** Rust 2021 (workspace, rust-version 1.92), maud, serde/toml/serde_json, wasm-bindgen 0.2.127 (pinned — CI installs exactly this CLI version), vanilla JS module.

**Spec:** `docs/superpowers/specs/2026-08-16-ask-terminal-design.md`

## Global Constraints

- CSP must not change: no inline `<script>` bodies beyond the two existing hashed ones, no `style=""` attributes anywhere (CI greps for them), no external requests. `scripts/check-csp-hashes.sh` must pass after every task that touches markup.
- The existing test `build::tests::wasm_loaders_are_referenced_only_by_their_own_demo_page` asserts **no route page's HTML contains the substring `loader.js`** — the ask page's script is therefore `static/ask/terminal.js`, never `loader.js`.
- `content/*.toml` structs use `#[serde(deny_unknown_fields)]` and validate content that would render broken, failing the build loudly (see `content.rs::Site::validate`).
- Comments only where they add value; match the repo's existing comment density and idiom.
- wasm-bindgen dependency version in `crates/ask` must be `0.2.127` to match the CLI version CI installs.
- Run `cargo test -p site` and (for `crates/ask` tasks) `cargo test -p ask-terminal` before every commit. Commit at the end of every task. NEVER push — pushing to master requires the user's explicit go-ahead.
- Full verification loop after the last task: `./scripts/build-wasm.sh && cargo run -p site 2>/dev/null; ./scripts/check-csp-hashes.sh` (non-strict local build tolerates the missing resume PDF; CI runs strict).

---

### Task 1: `content/ask.toml` schema, loader, and starter content

**Files:**
- Create: `content/ask.toml`
- Modify: `crates/site/src/content.rs` (append after `Work` struct and extend tests)

**Interfaces:**
- Produces: `content::AskContent { suggest: Vec<String>, intent: Vec<Intent>, note: Vec<Note> }`, `content::Intent { match_: Vec<String>, answer: String, source: String }` (TOML key `match`), `content::Note { q: String, a: String, aliases: Vec<String>, source: String }`, `AskContent::load(path: &Path) -> Result<AskContent>`, and `content::fixture_ask()` (test-only, `#[cfg(test)]`, mirroring the existing `fixture_site()`).

- [ ] **Step 1: Write the failing tests** — append to the `#[cfg(test)] mod tests` in `crates/site/src/content.rs`:

```rust
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
```

- [ ] **Step 2: Run to verify failure** — `cargo test -p site ask_content` — expected: compile error, `AskContent` not found.

- [ ] **Step 3: Implement.** In `content.rs`, after the `Work` struct:

```rust
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
```

Add a `fixture_ask()` beside the existing `fixture_site()` at `content.rs:61`, matching its exact style (loads the real content file, so fixtures never drift from what ships):

```rust
#[cfg(test)]
pub fn fixture_ask() -> AskContent {
    AskContent::load(std::path::Path::new("../../content/ask.toml")).expect("content/ask.toml loads")
}
```

Note this makes Task 2's tests depend on the starter `ask.toml` written in Step 4 below — in particular the note titled `What's Paul's education?` with alias `degree`.

- [ ] **Step 4: Write `content/ask.toml`** (starter content — every claim already appears in `site.toml`, keeping it truthful by construction; the user extends it later):

```toml
suggest = ["amazon", "fraud detection", "aho-corasick", "p-1 ai", "georgia tech"]

[[intent]]
match = ["who are you", "who is paul", "about", "about paul", "tell me about yourself"]
answer = "I'm Paul Maxwell — a software engineer in Washington, DC, currently at P-1 AI building the platform behind Archie, an AI engineering agent. Before that: fraud detection at Amazon, credit underwriting at Ampla, portfolio analytics at BlackRock."
source = "/about/"

[[intent]]
match = ["contact", "email", "how do i contact you", "how can i reach you", "get in touch"]
answer = "GitHub and LinkedIn are linked in the rail on every page — LinkedIn is the fastest way to reach me."
source = "/"

[[intent]]
match = ["what do you do", "where do you work", "current job", "what is your job"]
answer = "Lead Software Engineer at P-1 AI (2025—), working on the platform behind Archie, an AI engineering agent — everything from how it deploys into customer clouds to the plugin and dataset infrastructure it runs on."
source = "/about/"

[[note]]
q = "What's Paul's education?"
a = "A master's in computer science from Georgia Tech, a BSE from Michigan, and a CFA charter."
aliases = ["degree", "school", "university", "studied", "masters", "cfa", "michigan"]
source = "/about/"

[[note]]
q = "What happened in the NSA Codebreaker Challenge?"
a = "In 2023 Paul was one of 24 finishers (of about 3,300) in the NSA Codebreaker Challenge."
aliases = ["nsa", "codebreaker", "ctf", "security", "challenge"]
source = "/about/"
```

- [ ] **Step 5: Run to verify pass** — `cargo test -p site ask_content` — expected: 2 passed. Then `cargo test -p site` — all pass.

- [ ] **Step 6: Commit** — `git add content/ask.toml crates/site/src/content.rs && git commit -m "Ask terminal: ask.toml schema, loader, and starter content"`

---

### Task 2: corpus builder — `crates/site/src/ask_index.rs`

**Files:**
- Create: `crates/site/src/ask_index.rs`
- Modify: `crates/site/src/main.rs` (add `mod ask_index;` beside the other module declarations)
- Modify: `crates/site/Cargo.toml` (add `serde_json = "1"` to `[dependencies]` — a normal dependency, not dev-only: `ask_index` serializes with it at build time and the tests parse with it)

**Interfaces:**
- Consumes: `content::{Site, AskContent}` (Task 1), `content::{fixture_site, fixture_ask}`.
- Produces: `ask_index::index_json(site: &Site, ask: &AskContent) -> anyhow::Result<String>` emitting exactly the JSON contract `crates/ask` parses in Task 3: `{"passages":[{"title","text","source","boost":[…strings…]}],"intents":[{"match":[…],"answer","source"}],"suggest":[…]}`. `boost` carries a note's `q` + `aliases` (empty for derived passages).

- [ ] **Step 1: Write the failing tests** in `ask_index.rs`'s own `#[cfg(test)] mod tests`:

```rust
#[test]
fn corpus_covers_every_work_entry_and_about_paragraph() {
    let site = crate::content::fixture_site();
    let ask = crate::content::fixture_ask();
    let json = index_json(&site, &ask).unwrap();
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    let texts: Vec<String> = v["passages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| format!("{} {}", p["title"].as_str().unwrap(), p["text"].as_str().unwrap()))
        .collect();
    for work in &site.work {
        assert!(
            texts.iter().any(|t| t.contains(&work.title)),
            "work entry {} missing from corpus",
            work.title
        );
    }
    for para in &site.about {
        assert!(texts.iter().any(|t| t.contains(para.as_str())), "about paragraph missing");
    }
}

#[test]
fn notes_carry_their_question_and_aliases_as_boost_terms() {
    let site = crate::content::fixture_site();
    let ask = crate::content::fixture_ask();
    let json = index_json(&site, &ask).unwrap();
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    let note = v["passages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["title"] == "What's Paul's education?")
        .expect("note passage present");
    let boost: Vec<&str> =
        note["boost"].as_array().unwrap().iter().map(|b| b.as_str().unwrap()).collect();
    assert!(boost.contains(&"degree"), "aliases must land in boost: {boost:?}");
}
```

- [ ] **Step 2: Run to verify failure** — `cargo test -p site ask_index` — expected: compile error, module missing.

- [ ] **Step 3: Implement `ask_index.rs`:**

```rust
use crate::content::{AskContent, Site};
use anyhow::Result;
use serde::Serialize;

#[derive(Serialize)]
struct Passage<'a> {
    title: &'a str,
    text: String,
    source: &'a str,
    boost: Vec<&'a str>,
}

#[derive(Serialize)]
struct IntentOut<'a> {
    #[serde(rename = "match")]
    match_: &'a [String],
    answer: &'a str,
    source: &'a str,
}

#[derive(Serialize)]
struct Index<'a> {
    passages: Vec<Passage<'a>>,
    intents: Vec<IntentOut<'a>>,
    suggest: &'a [String],
}

/// The `/ask/` corpus: one passage per work entry, per about paragraph,
/// one "now" passage from the lede/bio/role lines, plus every authored
/// note. Notes carry their question and aliases as `boost` terms the
/// searcher indexes at extra weight.
pub fn index_json(site: &Site, ask: &AskContent) -> Result<String> {
    let mut passages = Vec::new();
    passages.push(Passage {
        title: "Now",
        text: format!("{} {} {}", site.role, site.lede, site.bio),
        source: "/",
        boost: Vec::new(),
    });
    for para in &site.about {
        passages.push(Passage { title: "About", text: para.clone(), source: "/about/", boost: Vec::new() });
    }
    for work in &site.work {
        passages.push(Passage {
            title: &work.title,
            text: format!("{} · {} · {} — {} {}", work.title, work.org, work.year, work.summary, work.detail),
            source: "/projects/",
            boost: Vec::new(),
        });
    }
    for note in &ask.note {
        let mut boost: Vec<&str> = vec![note.q.as_str()];
        boost.extend(note.aliases.iter().map(String::as_str));
        passages.push(Passage { title: &note.q, text: note.a.clone(), source: &note.source, boost });
    }
    let intents = ask
        .intent
        .iter()
        .map(|i| IntentOut { match_: &i.match_, answer: &i.answer, source: &i.source })
        .collect();
    Ok(serde_json::to_string(&Index { passages, intents, suggest: &ask.suggest })?)
}
```

- [ ] **Step 4: Run to verify pass** — `cargo test -p site ask_index` then `cargo test -p site` — all pass.

- [ ] **Step 5: Commit** — `git add crates/site/src/ask_index.rs crates/site/src/main.rs crates/site/Cargo.toml Cargo.lock && git commit -m "Ask terminal: build-time corpus emitter"`

---

### Task 3: `crates/ask` — tokenizer, stopwords, stemmer

**Files:**
- Create: `crates/ask/Cargo.toml`, `crates/ask/src/lib.rs`, `crates/ask/src/text.rs`
- Modify: `Cargo.toml` (workspace `members` += `"crates/ask"`)

**Interfaces:**
- Produces: `text::tokenize(s: &str) -> Vec<String>` (lowercase → split on non-alphanumeric → drop stopwords → stem) and `text::normalize(s: &str) -> String` (lowercase, punctuation → space, whitespace-collapsed — the intent-equality form). Task 4 consumes both.

- [ ] **Step 1: Create the crate manifest** `crates/ask/Cargo.toml` (mirrors the demo crates), and add `"crates/ask"` to the root workspace `members`:

```toml
[package]
name = "ask-terminal"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
publish = false

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
wasm-bindgen = "0.2.127"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

`src/lib.rs` starts as just `pub mod text;`.

- [ ] **Step 2: Write the failing tests** in `crates/ask/src/text.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn natural_questions_reduce_to_content_words() {
        // The stopword list is what makes questions work as queries.
        assert_eq!(tokenize("What did Paul build at Amazon?"), vec!["paul", "build", "amazon"]);
    }

    #[test]
    fn stemming_folds_light_suffixes() {
        assert_eq!(tokenize("underwriting systems"), vec!["underwrit", "system"]);
        // Short words must not be stemmed into stubs.
        assert_eq!(tokenize("gas red"), vec!["gas", "red"]);
    }

    #[test]
    fn normalize_strips_punctuation_and_case_only() {
        assert_eq!(normalize("Who are you?"), "who are you");
        assert_eq!(normalize("  Who,ARE   you!  "), "who are you");
    }
}
```

- [ ] **Step 3: Run to verify failure** — `cargo test -p ask-terminal` — expected: compile error, functions missing.

- [ ] **Step 4: Implement `text.rs`:**

```rust
/// English stopwords: the words a natural question is mostly made of.
/// Dropping them is what turns "what did paul build at amazon?" into
/// {paul, build, amazon}.
const STOPWORDS: &[&str] = &[
    "a", "about", "after", "again", "all", "also", "am", "an", "and", "any", "are", "as", "at",
    "be", "because", "been", "before", "being", "between", "both", "but", "by", "can", "could",
    "did", "do", "does", "doing", "down", "during", "each", "few", "for", "from", "further",
    "get", "had", "has", "have", "having", "he", "her", "here", "hers", "him", "his", "how", "i",
    "if", "in", "into", "is", "it", "its", "just", "like", "me", "more", "most", "my", "no",
    "nor", "not", "now", "of", "off", "on", "once", "only", "or", "other", "our", "out", "over",
    "own", "same", "she", "should", "so", "some", "such", "than", "that", "the", "their", "them",
    "then", "there", "these", "they", "this", "those", "through", "to", "too", "under", "until",
    "up", "us", "very", "was", "we", "were", "what", "when", "where", "which", "while", "who",
    "whom", "why", "will", "with", "would", "you", "your", "yours",
];

fn stem(word: &str) -> String {
    for suffix in ["ing", "ed", "es", "s"] {
        if word.len() > suffix.len() + 2 && word.ends_with(suffix) {
            return word[..word.len() - suffix.len()].to_string();
        }
    }
    word.to_string()
}

pub fn tokenize(s: &str) -> Vec<String> {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty() && !STOPWORDS.contains(w))
        .map(stem)
        .collect()
}

/// Intent-matching form: lowercase, punctuation flattened to spaces,
/// whitespace collapsed. Intents match by equality on this — stopwords
/// stay, because intent phrases ("who are you") are made of them.
pub fn normalize(s: &str) -> String {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}
```

- [ ] **Step 5: Run to verify pass** — `cargo test -p ask-terminal` — 3 passed. Also `cargo test -p site` still green.

- [ ] **Step 6: Commit** — `git add Cargo.toml Cargo.lock crates/ask && git commit -m "Ask terminal: ask crate with tokenizer, stopwords, stemmer"`

---

### Task 4: `crates/ask` — BM25 engine, intents, wasm surface

**Files:**
- Create: `crates/ask/src/engine.rs`
- Modify: `crates/ask/src/lib.rs`

**Interfaces:**
- Consumes: `text::{tokenize, normalize}` (Task 3); the index JSON contract from Task 2.
- Produces: `engine::Engine::new(index_json: &str) -> Result<Engine, String>`, `Engine::ask(&self, query: &str) -> Response` where `Response` serializes to the JSON `terminal.js` (Task 7) parses: `{"kind":"answer","title":str,"text":str,"source":str,"also":[{"title":str,"source":str}]}` or `{"kind":"miss","suggest":[str]}`. Also the wasm-bindgen `Terminal` class: `new Terminal(indexJson)` + `terminal.ask(query) -> String` (JSON).

- [ ] **Step 1: Write the failing tests** in `engine.rs` (`#[cfg(test)]` uses a small inline index JSON literal — three passages titled "Duplicate-Invoice Detection" (`text` containing "amazon fraud detection invoice", source `/projects/`), "Transaction Tagging Engine" (`text` containing "ampla aho corasick string matching", `boost: ["aho-corasick"]`, source `/projects/`), "About" (`text` containing "software engineer washington", source `/about/`); one intent `{"match":["who are you"],"answer":"I'm Paul.","source":"/about/"}`; `"suggest":["amazon","aho-corasick"]`):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    const INDEX: &str = r#"{
      "passages": [
        {"title":"Duplicate-Invoice Detection","text":"amazon fraud detection invoice real-time","source":"/projects/","boost":[]},
        {"title":"Transaction Tagging Engine","text":"ampla transaction tagging string matching runtime","source":"/projects/","boost":["aho-corasick"]},
        {"title":"About","text":"software engineer washington dc","source":"/about/","boost":[]}
      ],
      "intents": [{"match":["who are you"],"answer":"I'm Paul.","source":"/about/"}],
      "suggest": ["amazon","aho-corasick"]
    }"#;

    fn engine() -> Engine {
        Engine::new(INDEX).unwrap()
    }

    #[test]
    fn amazon_question_ranks_the_amazon_passage_first() {
        match engine().ask("what fraud work happened at amazon?") {
            Response::Answer { title, source, .. } => {
                assert_eq!(title, "Duplicate-Invoice Detection");
                assert_eq!(source, "/projects/");
            }
            other => panic!("expected an answer, got {other:?}"),
        }
    }

    #[test]
    fn alias_terms_find_their_note() {
        match engine().ask("aho corasick") {
            Response::Answer { title, .. } => assert_eq!(title, "Transaction Tagging Engine"),
            other => panic!("expected an answer, got {other:?}"),
        }
    }

    #[test]
    fn intent_phrases_match_exactly_with_punctuation_ignored() {
        match engine().ask("  Who ARE you?! ") {
            Response::Answer { text, title, .. } => {
                assert_eq!(text, "I'm Paul.");
                assert_eq!(title, "", "intent answers carry no passage title");
            }
            other => panic!("expected the intent answer, got {other:?}"),
        }
    }

    #[test]
    fn unknown_and_empty_queries_miss_with_suggestions() {
        for q in ["quantum blockchain golf", "", "the of and"] {
            match engine().ask(q) {
                Response::Miss { suggest } => assert_eq!(suggest, vec!["amazon", "aho-corasick"]),
                other => panic!("expected a miss for {q:?}, got {other:?}"),
            }
        }
    }

    #[test]
    fn runner_ups_are_reported_as_also() {
        match engine().ask("amazon string matching") {
            Response::Answer { also, .. } => assert!(!also.is_empty(), "expected runner-ups"),
            other => panic!("expected an answer, got {other:?}"),
        }
    }
}
```

- [ ] **Step 2: Run to verify failure** — `cargo test -p ask-terminal engine` — expected: compile error.

- [ ] **Step 3: Implement `engine.rs`:**

```rust
use crate::text::{normalize, tokenize};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize)]
struct PassageIn {
    title: String,
    text: String,
    source: String,
    #[serde(default)]
    boost: Vec<String>,
}

#[derive(Deserialize)]
struct IntentIn {
    #[serde(rename = "match")]
    match_: Vec<String>,
    answer: String,
    source: String,
}

#[derive(Deserialize)]
struct IndexIn {
    passages: Vec<PassageIn>,
    #[serde(default)]
    intents: Vec<IntentIn>,
    suggest: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Response {
    Answer { title: String, text: String, source: String, also: Vec<Also> },
    Miss { suggest: Vec<String> },
}

#[derive(Debug, Serialize)]
pub struct Also {
    pub title: String,
    pub source: String,
}

/// How many extra times a passage's `boost` terms (a note's question and
/// aliases) are counted, so an alias hit outranks an incidental body match.
const BOOST_WEIGHT: usize = 3;
const K1: f64 = 1.2;
const B: f64 = 0.75;
const TOP_ALSO: usize = 4;

pub struct Engine {
    passages: Vec<PassageIn>,
    docs: Vec<HashMap<String, usize>>, // term -> tf per passage
    lens: Vec<usize>,
    avg_len: f64,
    df: HashMap<String, usize>,
    intents: Vec<(Vec<String>, String, String)>, // (normalized phrases, answer, source)
    suggest: Vec<String>,
}

impl Engine {
    pub fn new(index_json: &str) -> Result<Engine, String> {
        let index: IndexIn = serde_json::from_str(index_json).map_err(|e| e.to_string())?;
        let mut docs = Vec::new();
        let mut lens = Vec::new();
        let mut df: HashMap<String, usize> = HashMap::new();
        for p in &index.passages {
            let mut terms = tokenize(&format!("{} {}", p.title, p.text));
            for b in &p.boost {
                for _ in 0..BOOST_WEIGHT {
                    terms.extend(tokenize(b));
                }
            }
            let mut tf: HashMap<String, usize> = HashMap::new();
            for t in &terms {
                *tf.entry(t.clone()).or_insert(0) += 1;
            }
            for term in tf.keys() {
                *df.entry(term.clone()).or_insert(0) += 1;
            }
            lens.push(terms.len());
            docs.push(tf);
        }
        let avg_len = if lens.is_empty() {
            0.0
        } else {
            lens.iter().sum::<usize>() as f64 / lens.len() as f64
        };
        let intents = index
            .intents
            .into_iter()
            .map(|i| (i.match_.iter().map(|m| normalize(m)).collect(), i.answer, i.source))
            .collect();
        Ok(Engine { passages: index.passages, docs, lens, avg_len, df, intents, suggest: index.suggest })
    }

    pub fn ask(&self, query: &str) -> Response {
        let norm = normalize(query);
        for (phrases, answer, source) in &self.intents {
            if phrases.iter().any(|p| *p == norm) {
                return Response::Answer {
                    title: String::new(),
                    text: answer.clone(),
                    source: source.clone(),
                    also: Vec::new(),
                };
            }
        }

        let terms = tokenize(query);
        let n = self.passages.len() as f64;
        let mut scored: Vec<(f64, usize)> = self
            .docs
            .iter()
            .enumerate()
            .map(|(i, tf)| {
                let len = self.lens[i] as f64;
                let score: f64 = terms
                    .iter()
                    .map(|t| {
                        let tf_t = *tf.get(t).unwrap_or(&0) as f64;
                        if tf_t == 0.0 {
                            return 0.0;
                        }
                        let df_t = *self.df.get(t).unwrap_or(&0) as f64;
                        let idf = (1.0 + (n - df_t + 0.5) / (df_t + 0.5)).ln();
                        idf * (tf_t * (K1 + 1.0)) / (tf_t + K1 * (1.0 - B + B * len / self.avg_len))
                    })
                    .sum();
                (score, i)
            })
            .filter(|(score, _)| *score > 0.0)
            .collect();
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        match scored.first() {
            None => Response::Miss { suggest: self.suggest.clone() },
            Some(&(_, best)) => {
                let p = &self.passages[best];
                let also = scored
                    .iter()
                    .skip(1)
                    .take(TOP_ALSO)
                    .map(|&(_, i)| Also {
                        title: self.passages[i].title.clone(),
                        source: self.passages[i].source.clone(),
                    })
                    .collect();
                Response::Answer {
                    title: p.title.clone(),
                    text: p.text.clone(),
                    source: p.source.clone(),
                    also,
                }
            }
        }
    }
}
```

- [ ] **Step 4: Wasm surface** — replace `crates/ask/src/lib.rs` with:

```rust
pub mod engine;
pub mod text;

use wasm_bindgen::prelude::*;

/// The `/ask/` page's handle: constructed once from the fetched
/// `/ask/index.json`, then queried per keystroke-free submission.
#[wasm_bindgen]
pub struct Terminal {
    engine: engine::Engine,
}

#[wasm_bindgen]
impl Terminal {
    #[wasm_bindgen(constructor)]
    pub fn new(index_json: &str) -> Result<Terminal, JsError> {
        let engine = engine::Engine::new(index_json).map_err(|e| JsError::new(&e))?;
        Ok(Terminal { engine })
    }

    /// Returns the response as JSON — rendering is terminal.js's job.
    pub fn ask(&self, query: &str) -> String {
        serde_json::to_string(&self.engine.ask(query)).unwrap_or_else(|_| {
            r#"{"kind":"miss","suggest":[]}"#.to_string()
        })
    }
}
```

- [ ] **Step 5: Run to verify pass** — `cargo test -p ask-terminal` — all pass (native target; wasm-bindgen attrs compile to no-ops off-wasm).

- [ ] **Step 6: Commit** — `git add crates/ask && git commit -m "Ask terminal: BM25 engine, intent matching, wasm surface"`

---

### Task 5: `Route::Ask`, the `/ask/` page, and theme styles

**Files:**
- Modify: `crates/site/src/route.rs` (add variant to enum, `ALL` becomes `[Route; 6]`, arms in `path()`, `output_path()`, `label()`)
- Create: `crates/site/src/pages/ask.rs`
- Modify: `crates/site/src/pages/mod.rs` (add `pub mod ask;`)
- Modify: `crates/site/src/build.rs` (add `Route::Ask => pages::ask::render(&site)` to the match in `build()`)
- Modify: `crates/site/src/theme.rs` (`.ask-*` styles + tests)

**Interfaces:**
- Consumes: `shell::layout` (existing), theme tokens.
- Produces: `pages::ask::render(site: &Site) -> Markup`; markup ids/classes the loader (Task 7) uses: `#ask-log`, `#ask-form`, `#ask-input`; classes the loader writes: `.ask-card`, `.ask-q`, `.ask-src`, `.ask-also`.

- [ ] **Step 1: Write the failing tests.** In `pages/ask.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::fixture_site;

    #[test]
    fn ask_page_wires_the_terminal() {
        let out = render(&fixture_site()).into_string();
        assert!(out.contains(r#"id="ask-log""#), "missing scrollback container: {out}");
        assert!(out.contains(r#"id="ask-input""#), "missing query input: {out}");
        assert!(
            out.contains(r#"<script type="module" src="/ask/terminal.js">"#),
            "missing module script (must be terminal.js, never loader.js): {out}"
        );
        assert!(out.contains("noscript"), "no fallback for JS-disabled visitors: {out}");
        assert!(!out.contains("loader.js"), "route pages must not reference any loader.js");
    }
}
```

In `theme.rs` tests:

```rust
#[test]
fn ask_terminal_class_names_agree_with_the_script_that_writes_them() {
    // static/ask/terminal.js renders .ask-card/.ask-q/.ask-src/.ask-also
    // into #ask-log; renaming either side alone should break the build.
    let css = stylesheet();
    const TERMINAL_JS: &str = include_str!("../../../static/ask/terminal.js");
    for class in [".ask-log {", ".ask-card {", ".ask-src {", ".ask-also {", ".ask-form {"] {
        assert!(css.contains(class), "stylesheet missing {class}: {css}");
    }
    for name in ["ask-log", "ask-card", "ask-q", "ask-src", "ask-also", "ask-input"] {
        assert!(TERMINAL_JS.contains(name), "static/ask/terminal.js no longer mentions {name}");
    }
}
```

Create an empty `static/ask/terminal.js` (`touch static/ask/terminal.js`) so `include_str!` compiles and the test fails on assertions; Task 7 fills it.

- [ ] **Step 2: Run to verify failure** — `cargo test -p site ask_page ask_terminal_class` — expected: compile errors (module/route missing), then assertion failures.

- [ ] **Step 3: Implement.** `route.rs`: add `Ask` after `Resume` (nav order: Index, About, Projects, Resume, Ask, Demos — `ALL` length 6), `path()` → `"/ask/"`, `output_path()` → `"ask/index.html"`, `label()` → `"Ask"`. `pages/ask.rs`:

```rust
use crate::components::shell;
use crate::content::Site;
use crate::route::Route;
use maud::{html, Markup};

pub fn render(site: &Site) -> Markup {
    let main = html! {
        h1 { "Ask" }
        p .prose {
            "Ask about my work. Answers come straight from this site's content, "
            "ranked in your browser by a Rust BM25 searcher compiled to WebAssembly — "
            "no server, no model, no tracking."
        }

        div #ask-log .ask-log {}

        form #ask-form .ask-form {
            span .ask-prompt aria-hidden="true" { ">" }
            input #ask-input .demo-input type="text" autocomplete="off"
                aria-label="Ask a question about Paul's work";
            button .theme-toggle type="submit" { "Ask" }
        }

        noscript {
            p .prose {
                "The ask terminal needs JavaScript and WebAssembly. Everything it "
                "knows is already on the about and projects pages."
            }
        }

        script type="module" src="/ask/terminal.js" {}
    };
    shell::layout(site, Route::Ask, "Ask", main)
}
```

`theme.rs` stylesheet additions (before the `@media (max-width: 640px)` block):

```css
.ask-log {
  font-family: var(--font-mono);
  font-size: 13px;
  line-height: 1.7;
  max-width: 660px;
  display: flex;
  flex-direction: column;
  gap: calc(var(--space) * 2);
  margin: calc(var(--space) * 3) 0;
}
.ask-card { border-left: 2px solid var(--rule); padding-left: calc(var(--space) * 1.5); }
.ask-q { color: var(--muted); }
.ask-src { color: var(--muted); font-size: 11px; }
.ask-src a { color: var(--accent); }
.ask-also { color: var(--muted); font-size: 12px; }
.ask-form {
  display: flex;
  gap: var(--space);
  align-items: center;
  max-width: 660px;
}
.ask-form input {{ flex: 1; }}
.ask-prompt { font-family: var(--font-mono); color: var(--accent); }
```

(In `theme.rs` the block is inside `format!` — double every `{`/`}` as `{{`/`}}` like the surrounding rules.)

- [ ] **Step 4: Run to verify pass** — `cargo test -p site` — the two new tests pass; existing route-driven tests (nav, sitemap, canonicals) pass automatically because they iterate `Route::ALL`. If any existing test hardcodes the route count or list, update it to match the enum.

- [ ] **Step 5: Commit** — `git add crates/site/src/route.rs crates/site/src/pages crates/site/src/theme.rs crates/site/src/build.rs static/ask/terminal.js && git commit -m "Ask terminal: /ask/ route, page, and styles"`

---

### Task 6: build wiring — emit `index.json`, build the WASM, CI ships-check

**Files:**
- Modify: `crates/site/src/build.rs` (load `AskContent`, emit `ask/index.json`)
- Modify: `scripts/build-wasm.sh` (add the ask crate)
- Modify: `.github/workflows/deploy.yml` ("Verify wasm shipped" step += ask artifact)

**Interfaces:**
- Consumes: `content::AskContent::load`, `ask_index::index_json` (Tasks 1–2).
- Produces: `dist/ask/index.json` beside `dist/ask/index.html`; `static/ask/ask_terminal.js` + `static/ask/ask_terminal_bg.wasm` (wasm-bindgen artifacts, committed like the demo crates' artifacts).

- [ ] **Step 1: Write the failing test** in `build.rs`'s test module (copy the setup style of the neighboring build tests, which build into a temp dir):

```rust
#[test]
fn build_emits_a_parsable_ask_corpus() {
    let tmp = std::env::temp_dir().join("site-build-test-ask-index");
    let _ = std::fs::remove_dir_all(&tmp);
    build(Path::new("../.."), &tmp, false).expect("build succeeds");
    let json = std::fs::read_to_string(tmp.join("ask/index.json")).expect("index.json emitted");
    let v: serde_json::Value = serde_json::from_str(&json).expect("index.json parses");
    assert!(!v["passages"].as_array().unwrap().is_empty(), "corpus must not be empty");
    assert!(!v["suggest"].as_array().unwrap().is_empty(), "suggest terms must ship");
}
```

- [ ] **Step 2: Run to verify failure** — `cargo test -p site build_emits_a_parsable_ask_corpus` — FAIL: `ask/index.json` missing.

- [ ] **Step 3: Implement.** In `build()` after `let site = Site::load(...)`: `let ask = crate::content::AskContent::load(&root.join("content/ask.toml"))?;` and, beside the other `write(...)` calls: `write(&out.join("ask/index.json"), &crate::ask_index::index_json(&site, &ask)?, &mut written)?;`. In `build-wasm.sh`, after the existing two lines: `build_wasm_crate ask-terminal static/ask ask_terminal`. In `deploy.yml`'s "Verify wasm shipped" step, add `test -f dist/ask/ask_terminal_bg.wasm`.

- [ ] **Step 4: Build the WASM and verify everything passes:**

Run: `rustup target add wasm32-unknown-unknown 2>/dev/null; ./scripts/build-wasm.sh` (needs `wasm-bindgen` CLI 0.2.127 — `cargo install wasm-bindgen-cli --version 0.2.127 --locked` if absent), then `cargo test -p site` — all pass, including the new one.

- [ ] **Step 5: Commit** — `git add crates/site/src/build.rs scripts/build-wasm.sh .github/workflows/deploy.yml static/ask && git commit -m "Ask terminal: emit corpus, build wasm, verify it ships"`

---

### Task 7: `static/ask/terminal.js` — the terminal UI

**Files:**
- Modify: `static/ask/terminal.js` (created empty in Task 5)

**Interfaces:**
- Consumes: `static/ask/ask_terminal.js` wasm-bindgen module (`init`, `Terminal`), `/ask/index.json`, page ids `#ask-log` / `#ask-form` / `#ask-input`, response JSON from Task 4.
- Produces: rendered `.ask-card` / `.ask-q` / `.ask-src` / `.ask-also` DOM in `#ask-log`.

- [ ] **Step 1: Implement** (no JS harness exists in this repo — coverage comes from Task 5's `include_str!` cross-check and Task 8's live browser verification):

```javascript
// The /ask/ terminal. Fetches the corpus, hands it to the Rust BM25
// searcher (ask_terminal wasm), and renders answers into #ask-log.
// All content shown is authored text from the corpus — this file only
// formats it. Class names are pinned by theme.rs tests.
import init, { Terminal } from "./ask_terminal.js";

const EXAMPLES = [
  "what did paul build at amazon?",
  "aho-corasick",
  "who are you?",
];

function el(tag, className, text) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text) node.textContent = text;
  return node;
}

function sourceLine(source, title) {
  const src = el("div", "ask-src");
  src.appendChild(document.createTextNode("— "));
  const link = el("a", "", source);
  link.href = source;
  src.appendChild(link);
  if (title) src.appendChild(document.createTextNode(" · " + title));
  return src;
}

async function main() {
  const log = document.getElementById("ask-log");
  const form = document.getElementById("ask-form");
  const input = document.getElementById("ask-input");
  if (!log || !form || !input) return;

  const card = (children) => {
    const c = el("div", "ask-card");
    children.forEach((ch) => c.appendChild(ch));
    log.appendChild(c);
    c.scrollIntoView({ block: "nearest" });
    return c;
  };

  let terminal = null;
  try {
    const [, indexJson] = await Promise.all([
      init(),
      fetch("/ask/index.json").then((r) => {
        if (!r.ok) throw new Error("index fetch failed: " + r.status);
        return r.text();
      }),
    ]);
    terminal = new Terminal(indexJson);
  } catch (err) {
    card([el("div", "", "the terminal failed to load — everything it knows is on /about/ and /projects/.")]);
    return;
  }

  card([
    el("div", "", "ask about my work — e.g. " + EXAMPLES.map((e) => '"' + e + '"').join(", ")),
  ]);

  const history = [];
  let historyAt = -1;

  form.addEventListener("submit", (event) => {
    event.preventDefault();
    const query = input.value.trim();
    if (!query) return;
    history.push(query);
    historyAt = history.length;
    input.value = "";

    const result = JSON.parse(terminal.ask(query));
    const children = [el("div", "ask-q", "> " + query)];
    if (result.kind === "answer") {
      if (result.title) children.push(el("div", "", result.title));
      children.push(el("div", "", result.text));
      children.push(sourceLine(result.source, result.title));
      if (result.also.length) {
        children.push(el("div", "ask-also", "also: " + result.also.map((a) => a.title).join(" · ")));
      }
    } else {
      children.push(el("div", "", "no matches — try: " + result.suggest.join(", ")));
    }
    card(children);
  });

  input.addEventListener("keydown", (event) => {
    if (event.key === "ArrowUp" && historyAt > 0) {
      historyAt -= 1;
      input.value = history[historyAt];
      event.preventDefault();
    } else if (event.key === "ArrowDown" && historyAt < history.length - 1) {
      historyAt += 1;
      input.value = history[historyAt];
      event.preventDefault();
    }
  });
}

main();
```

- [ ] **Step 2: Verify the cross-check test now passes for real content** — `cargo test -p site ask_terminal_class` — PASS.

- [ ] **Step 3: Commit** — `git add static/ask/terminal.js && git commit -m "Ask terminal: terminal UI"`

---

### Task 8: full build, CSP gate, live browser verification

**Files:**
- Create (scratchpad only, not committed): a Playwright check script.

**Interfaces:**
- Consumes: everything above; the scratchpad Playwright setup from earlier sessions (`npm install playwright` in the session scratchpad if absent; Chromium is cached in `~/Library/Caches/ms-playwright`).

- [ ] **Step 1: Full build + gates** — `./scripts/build-wasm.sh && cargo test -p site && cargo test -p ask-terminal && cargo run -p site && ./scripts/check-csp-hashes.sh` — expected: all tests pass, build writes dist/, CSP gate reports the same 2 inline hashes (no drift; the module script and WASM are covered by `script-src 'self'` + `'wasm-unsafe-eval'`, the `index.json` fetch by `connect-src 'self'`).

- [ ] **Step 2: Serve and drive.** `cd dist && python3 -m http.server 8931 &`, then a Playwright script that: loads `/ask/`, waits for the greeting card, types `what did paul build at amazon?` + Enter and asserts the answer card's source line links to `/projects/`, types `who are you?` and asserts the intent answer text appears, types `quantum blockchain golf` and asserts the miss message lists the suggest terms, presses ArrowUp and asserts the input recalls the last query, screenshots, and checks zero console errors. Look at the screenshot — a blank terminal is a failure.

- [ ] **Step 3: Nav sanity** — in the same session, load `/` and assert the rail contains an `Ask` link to `/ask/`; fetch `/sitemap.xml` and assert it contains `/ask/`.

- [ ] **Step 4: Kill the server, report results** — with screenshots and the exact assertion outcomes. Do NOT push; pushing to master needs the user's explicit approval.

---

## Self-review notes

- Spec coverage: schema→Task 1, corpus artifact→Tasks 2+6, search pipeline/intents/miss→Tasks 3–4, page/UX/route/styles→Task 5, loader→Task 7, CSP/testing/live-verify→Tasks 6+8, non-goals honored (no resume PDF, no embeddings). One deliberate refinement over the spec: intents match by **normalized-string equality per phrase** rather than token overlap, because intent phrases ("who are you") are made entirely of stopwords and token-subset matching would both empty them and over-trigger (e.g. the phrase "about paul" reducing to `{paul}` would hijack every question naming Paul). The spec's intent-behavior contract (canned answer, short-circuits ranking) is unchanged.
- Type consistency: `index_json` output fields (`passages[].title/text/source/boost`, `intents[].match/answer/source`, `suggest`) match `engine.rs`'s `PassageIn`/`IntentIn`/`IndexIn` deserializers; `Response` JSON (`kind`/`title`/`text`/`source`/`also`/`suggest`) matches `terminal.js`'s rendering; page ids `ask-log`/`ask-form`/`ask-input` match between `pages/ask.rs`, `terminal.js`, and the theme cross-check test; artifact name `ask_terminal` matches crate name `ask-terminal` through `build-wasm.sh`'s hyphen→underscore convention and the CI check.

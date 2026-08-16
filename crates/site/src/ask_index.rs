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

#[cfg(test)]
mod tests {
    use super::*;

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

    /// Builds the engine from the REAL corpus (real site.toml + ask.toml,
    /// not the synthetic fixture in engine.rs's own tests) and checks that
    /// the obvious questions land on the right passage. This is the only
    /// place that would have caught the "Now" passage outranking the
    /// Amazon work entries for "what did paul build at amazon?".
    mod real_corpus {
        use super::*;
        use ask_terminal::engine::{Engine, Response};

        fn real_engine() -> Engine {
            let site = crate::content::fixture_site();
            let ask = crate::content::fixture_ask();
            let json = index_json(&site, &ask).unwrap();
            Engine::new(&json).unwrap()
        }

        #[test]
        fn amazon_question_lands_on_amazon_work_not_the_now_passage() {
            match real_engine().ask("what did paul build at amazon?") {
                Response::Answer { title, source, .. } => {
                    assert_eq!(source, "/projects/", "expected an Amazon work/project passage, got title {title:?}");
                    assert_ne!(title, "Now", "regression: the generic bio passage outranked the Amazon work");
                }
                other => panic!("expected an answer, got {other:?}"),
            }
        }

        #[test]
        fn aho_corasick_finds_the_transaction_tagging_engine() {
            match real_engine().ask("aho corasick") {
                Response::Answer { title, .. } => {
                    assert_eq!(title, "Transaction Tagging Engine");
                }
                other => panic!("expected an answer, got {other:?}"),
            }
        }

        #[test]
        fn who_are_you_hits_the_intent_answer() {
            match real_engine().ask("who are you?") {
                Response::Answer { title, .. } => {
                    assert_eq!(title, "", "intent answers carry no passage title");
                }
                other => panic!("expected an answer, got {other:?}"),
            }
        }

        #[test]
        fn nonsense_query_misses_with_suggestions() {
            match real_engine().ask("quantum blockchain golf") {
                Response::Miss { suggest } => assert!(!suggest.is_empty(), "expected suggest terms"),
                other => panic!("expected a miss, got {other:?}"),
            }
        }
    }
}

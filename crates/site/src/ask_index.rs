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
    examples: &'a [String],
}

/// The `/ask/` corpus: one passage per work entry, per about paragraph,
/// one "now" passage from the lede/bio/role lines (plus location — the
/// rail displays it, so the corpus must contain it too), plus every
/// authored note. Notes carry their question and aliases as `boost`
/// terms the searcher indexes at extra weight.
pub fn index_json(site: &Site, ask: &AskContent) -> Result<String> {
    let mut passages = Vec::new();
    passages.push(Passage {
        title: "Now",
        text: format!("{} {} {} {}", site.location, site.role, site.lede, site.bio),
        source: "/",
        boost: Vec::new(),
    });
    for para in &site.about {
        passages.push(Passage {
            title: "About",
            text: para.clone(),
            source: "/about/",
            boost: Vec::new(),
        });
    }
    for work in &site.work {
        passages.push(Passage {
            title: &work.title,
            text: format!(
                "{} · {} · {} — {} {}",
                work.title, work.org, work.year, work.summary, work.detail
            ),
            source: "/projects/",
            boost: Vec::new(),
        });
    }
    for note in &ask.note {
        let mut boost: Vec<&str> = vec![note.q.as_str()];
        boost.extend(note.aliases.iter().map(String::as_str));
        passages.push(Passage {
            title: &note.q,
            text: note.a.clone(),
            source: &note.source,
            boost,
        });
    }
    let intents = ask
        .intent
        .iter()
        .map(|i| IntentOut { match_: &i.match_, answer: &i.answer, source: &i.source })
        .collect();
    Ok(serde_json::to_string(&Index {
        passages,
        intents,
        suggest: &ask.suggest,
        examples: &ask.examples,
    })?)
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
            // Derived from site.toml rather than hardcoded, so routine prose
            // edits there don't make this brittle — but it still pins the
            // answer to something actually Amazon-relevant, not just any
            // /projects/ passage (Zero-Trust Agent Gateway and Archie BYOC
            // are also /projects/ and would wrongly satisfy a looser check).
            let site = crate::content::fixture_site();
            let amazon_titles: Vec<&str> =
                site.work.iter().filter(|w| w.org == "Amazon").map(|w| w.title.as_str()).collect();
            match real_engine().ask("what did paul build at amazon?") {
                Response::Answer { title, source, .. } => {
                    assert_eq!(
                        source, "/projects/",
                        "expected an Amazon work/project passage, got title {title:?}"
                    );
                    assert_ne!(
                        title, "Now",
                        "regression: the generic bio passage outranked the Amazon work"
                    );
                    assert!(
                        amazon_titles.contains(&title.as_str()) || title.to_lowercase().contains("amazon"),
                        "expected an Amazon-relevant title (one of {amazon_titles:?}, or containing \"amazon\"), got {title:?}"
                    );
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
                Response::Miss { suggest } => {
                    assert!(!suggest.is_empty(), "expected suggest terms")
                }
                other => panic!("expected a miss, got {other:?}"),
            }
        }

        /// Regression: third-person "where does paul work" used to fall
        /// through to BM25, where {paul, work} was dominated by the Amazon
        /// note's tripled-boost title — his *previous* employer, not his
        /// current one.
        #[test]
        fn third_person_job_questions_hit_the_current_job_intent() {
            for q in ["where does paul work", "where does he work", "what does he do now"] {
                match real_engine().ask(q) {
                    Response::Answer { text, .. } => {
                        assert!(
                            text.contains("P-1 AI"),
                            "query {q:?} did not answer with the current job: {text:?}"
                        )
                    }
                    other => panic!("expected an answer for {q:?}, got {other:?}"),
                }
            }
        }

        /// Regression: "where is he based" used to stem "based" to "bas"
        /// and collide with "rule-based", landing on the fraud-detection
        /// passage instead of the owner's actual location.
        #[test]
        fn location_questions_answer_with_the_owners_location() {
            let site = crate::content::fixture_site();
            for q in ["where is he based", "where does he live"] {
                match real_engine().ask(q) {
                    Response::Answer { text, .. } => {
                        assert!(
                            text.contains(&site.location),
                            "query {q:?} did not mention {:?}: {text:?}",
                            site.location
                        )
                    }
                    other => panic!("expected an answer for {q:?}, got {other:?}"),
                }
            }
        }

        /// Regression: "resume", "cv", and "demos" used to MISS on a site
        /// with real /resume/ and /demos/ routes.
        #[test]
        fn resume_and_demos_questions_point_at_their_routes() {
            match real_engine().ask("resume") {
                Response::Answer { source, .. } => assert_eq!(source, "/resume/"),
                other => panic!("expected an answer, got {other:?}"),
            }
            match real_engine().ask("cv") {
                Response::Answer { source, .. } => assert_eq!(source, "/resume/"),
                other => panic!("expected an answer, got {other:?}"),
            }
            match real_engine().ask("demos") {
                Response::Answer { source, .. } => assert_eq!(source, "/demos/"),
                other => panic!("expected an answer, got {other:?}"),
            }
        }

        /// Regression: apostrophe questions used to yield a bare "s" token
        /// that hijacked the education note via its boosted title's six
        /// "s" occurrences.
        #[test]
        fn apostrophe_questions_no_longer_hijack_the_education_note() {
            match real_engine().ask("what's your email address?") {
                Response::Answer { title, .. } => {
                    assert_ne!(
                        title, "What's Paul's education?",
                        "regression: apostrophe 's' token hijack"
                    )
                }
                Response::Miss { .. } => {} // honest miss is fine; the wrong answer is what regressed before
            }
        }

        /// The terminal's greeting shows `ask.examples` as suggested
        /// queries. If an intent phrase is edited later and an example
        /// stops resolving, that's a silent regression a visitor would
        /// hit immediately — this closes the loop that `suggest` (whose
        /// terms are never run through the engine) only half-closes.
        #[test]
        fn every_example_resolves_to_a_real_answer() {
            let ask = crate::content::fixture_ask();
            let engine = real_engine();
            for example in &ask.examples {
                match engine.ask(example) {
                    Response::Answer { .. } => {}
                    Response::Miss { .. } => {
                        panic!("example {example:?} misses instead of answering")
                    }
                }
            }
        }
    }
}

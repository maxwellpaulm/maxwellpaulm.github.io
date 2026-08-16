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
/// A runner-up must score at least this fraction of the best match to be
/// shown as "also" — otherwise a merely nonzero-scoring, barely-related
/// passage gets presented as if it were relevant.
const ALSO_RELEVANCE_FLOOR: f64 = 0.35;

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
        Ok(Engine {
            passages: index.passages,
            docs,
            lens,
            avg_len,
            df,
            intents,
            suggest: index.suggest,
        })
    }

    pub fn ask(&self, query: &str) -> Response {
        let norm = normalize(query);
        for (phrases, answer, source) in &self.intents {
            if phrases.contains(&norm) {
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
            Some(&(best_score, best)) => {
                let p = &self.passages[best];
                let floor = best_score * ALSO_RELEVANCE_FLOOR;
                let also = scored
                    .iter()
                    .skip(1)
                    .filter(|&&(score, _)| score >= floor)
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

    #[test]
    fn weakly_scoring_runner_ups_are_not_reported_as_also() {
        // "aho corasick" only matches the alias boost on Transaction
        // Tagging Engine — nothing else scores meaningfully, so `also`
        // must not pad itself out with barely-related passages.
        match engine().ask("aho corasick") {
            Response::Answer { also, .. } => {
                assert!(also.is_empty(), "expected no relevant runner-ups, got {also:?}")
            }
            other => panic!("expected an answer, got {other:?}"),
        }
    }
}

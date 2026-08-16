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

/// The site owner's own name. On a single-person site, "paul" and
/// "maxwell" appear in nearly every note and passage title — they carry
/// no discriminative signal for ranking, only noise (e.g. "where does
/// paul work" was dominated by whichever passage repeats "Paul" the
/// most, rather than the one actually about his job). Kept separate
/// from the generic English stopword list because this one is specific
/// to whose site this is, not English grammar.
const NAME_STOPWORDS: &[&str] = &["paul", "maxwell"];

fn stem(word: &str) -> String {
    for suffix in ["ing", "ed", "es", "s"] {
        // Measured in bytes, not chars — fine here since tokens are
        // already lowercased ASCII/alphanumeric by the time this runs.
        if word.len() > suffix.len() + 2 && word.ends_with(suffix) {
            return word[..word.len() - suffix.len()].to_string();
        }
    }
    word.to_string()
}

pub fn tokenize(s: &str) -> Vec<String> {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        // Splitting on the apostrophe in "what's"/"paul's" leaves a bare
        // "s" (and "I'd"/"we'll" leave "d"/"ll"): single-character tokens
        // carry no signal and are dropped along with the stopwords.
        .filter(|w| {
            w.len() > 1 && !STOPWORDS.contains(w) && !NAME_STOPWORDS.contains(w)
        })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn natural_questions_reduce_to_content_words() {
        // The stopword list is what makes questions work as queries.
        // "paul" is dropped too — see NAME_STOPWORDS: the owner's own
        // name carries no discriminative signal on his own site.
        assert_eq!(tokenize("What did Paul build at Amazon?"), vec!["build", "amazon"]);
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

    #[test]
    fn apostrophes_do_not_leave_bare_single_letter_tokens() {
        // "What's Paul's education?" splits on the apostrophe into
        // "what", "s", "paul", "s", "education" — the bare "s" tokens
        // must not survive, and the owner's own name carries no
        // discriminative signal on his own site (see NAME_STOPWORDS).
        assert_eq!(tokenize("What's Paul's education?"), vec!["education"]);
    }

    #[test]
    fn single_character_tokens_are_dropped() {
        assert_eq!(tokenize("I'd love that"), vec!["love"]);
    }

    #[test]
    fn owner_name_is_stopped_out() {
        assert_eq!(tokenize("where does paul work"), vec!["work"]);
        assert_eq!(tokenize("Paul Maxwell"), Vec::<String>::new());
    }
}

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

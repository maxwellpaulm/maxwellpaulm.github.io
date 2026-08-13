//! Normalises text extracted from the resume PDF by `pdftotext`.
//!
//! Kept in Rust rather than the shell script that produces the raw
//! extraction: `pdftotext`'s output has PDF-rendering artefacts (ligatures,
//! split small-caps headings) that need fixing identically on every
//! platform this build runs on, and BSD `sed` (macOS) and GNU `sed` (CI)
//! disagree on `\b`-style word-boundary regexes.

/// Ligatures `pdftotext` emits verbatim, expanded to their ASCII letter
/// sequences so full-text search and screen readers see ordinary words
/// instead of a single unfamiliar glyph.
const LIGATURES: [(char, &str); 5] = [
    ('\u{FB01}', "fi"),
    ('\u{FB02}', "fl"),
    ('\u{FB00}', "ff"),
    ('\u{FB03}', "ffi"),
    ('\u{FB04}', "ffl"),
];

fn expand_ligatures(text: &str) -> String {
    let mut out = text.to_string();
    for (ligature, expansion) in LIGATURES {
        out = out.replace(ligature, expansion);
    }
    out
}

/// The LaTeX resume template renders section headings in small caps: a
/// full-size initial letter followed by a smaller run of capitals.
/// `pdftotext` extracts that as two separate tokens — `E XPERIENCE` — which
/// a screen reader reads as "E, Xperience" and a crawler indexes as
/// "XPERIENCE". This joins a lone uppercase ASCII letter back onto an
/// immediately following uppercase run.
///
/// Deliberately narrow so real text survives untouched: the first token
/// must be *exactly* one uppercase letter (so `P-1` in "P-1 AI" doesn't
/// qualify), and the second must be two or more uppercase letters with
/// nothing else (so a lone `I` used as a job-level suffix, e.g. "Engineer I
/// (Jan", never merges — "(Jan" isn't all-uppercase).
fn join_split_small_caps(line: &str) -> String {
    let is_lone_capital =
        |word: &str| word.len() == 1 && word.chars().next().is_some_and(|c| c.is_ascii_uppercase());
    let is_upper_run =
        |word: &str| word.len() >= 2 && word.chars().all(|c| c.is_ascii_uppercase());

    let tokens: Vec<&str> = line.split(' ').collect();
    let mut out: Vec<String> = Vec::with_capacity(tokens.len());
    let mut i = 0;
    while i < tokens.len() {
        match tokens.get(i + 1) {
            Some(next) if is_lone_capital(tokens[i]) && is_upper_run(next) => {
                out.push(format!("{}{}", tokens[i], next));
                i += 2;
            }
            _ => {
                out.push(tokens[i].to_string());
                i += 1;
            }
        }
    }
    out.join(" ")
}

/// Normalises text extracted from the resume PDF: expands ligatures, joins
/// split small-caps headings, and strips the trailing form-feed page marker
/// `pdftotext` appends after the last page.
pub fn normalize(text: &str) -> String {
    let text = expand_ligatures(text);
    let text = text.replace('\u{c}', "");
    text.lines().map(join_split_small_caps).collect::<Vec<_>>().join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joins_a_split_small_caps_heading() {
        assert_eq!(join_split_small_caps("E XPERIENCE"), "EXPERIENCE");
        assert_eq!(join_split_small_caps("E DUCATION"), "EDUCATION");
    }

    #[test]
    fn joins_multiple_split_headings_on_one_line() {
        assert_eq!(join_split_small_caps("P ROGRAMMING S KILLS"), "PROGRAMMING SKILLS");
        assert_eq!(
            join_split_small_caps("A DDITIONAL C REDENTIALS AND ACCOMPLISHMENTS"),
            "ADDITIONAL CREDENTIALS AND ACCOMPLISHMENTS"
        );
    }

    #[test]
    fn leaves_a_hyphenated_initialism_untouched() {
        assert_eq!(join_split_small_caps("P-1 AI, Remote"), "P-1 AI, Remote");
    }

    #[test]
    fn leaves_a_bare_initialism_untouched() {
        assert_eq!(join_split_small_caps("Computer Science; GPA: 4.0"), "Computer Science; GPA: 4.0");
    }

    #[test]
    fn leaves_a_job_level_suffix_untouched() {
        assert_eq!(
            join_split_small_caps("Senior Software Engineer I (Jan 2023 – Dec 2023)"),
            "Senior Software Engineer I (Jan 2023 – Dec 2023)"
        );
    }

    #[test]
    fn expands_every_ligature_to_its_ascii_letters() {
        assert_eq!(expand_ligatures("One of 24 \u{FB01}nishers"), "One of 24 finishers");
        assert_eq!(expand_ligatures("workflow: \u{FB02}"), "workflow: fl");
        assert_eq!(expand_ligatures("sti\u{FB00}ness"), "stiffness");
        assert_eq!(expand_ligatures("o\u{FB03}ce"), "office");
        assert_eq!(expand_ligatures("wa\u{FB04}e"), "waffle");
    }

    #[test]
    fn normalize_strips_the_trailing_form_feed_page_marker() {
        assert_eq!(normalize("last line\n\u{c}"), "last line");
    }

    #[test]
    fn normalize_fixes_a_realistic_extraction_end_to_end() {
        let raw = "E XPERIENCE\nP-1 AI, Remote\n\u{FB01}nishers and workflow\u{FB02}\n\u{c}";
        assert_eq!(normalize(raw), "EXPERIENCE\nP-1 AI, Remote\nfinishers and workflowfl");
    }
}

//! Aho–Corasick automaton, built from scratch for the visualizer.
//! Deliberately free of wasm so the algorithm is tested natively.

#[derive(Debug, PartialEq)]
pub enum BuildError {
    NoPatterns,
    TooManyPatterns,
    EmptyPattern,
    PatternTooLong,
}

impl std::fmt::Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildError::NoPatterns => write!(f, "no patterns given"),
            BuildError::TooManyPatterns => write!(f, "too many patterns (max {MAX_PATTERNS})"),
            BuildError::EmptyPattern => {
                write!(f, "a pattern has no letters left after removing non-letters")
            }
            BuildError::PatternTooLong => {
                write!(f, "pattern too long (max {MAX_PATTERN_LEN} letters)")
            }
        }
    }
}

const MAX_PATTERNS: usize = 8;
const MAX_PATTERN_LEN: usize = 10;

/// Strips non-alphabetic bytes and lowercases the rest. `Automaton::build`
/// folds every pattern this way before inserting it into the trie, so
/// anything that needs to reason about a pattern's on-trie length (the
/// wasm wrapper, converting match end positions to text ranges) must fold
/// with this exact function or its lengths will disagree with the trie's.
pub fn fold(pattern: &str) -> Vec<u8> {
    pattern.bytes().filter(|b| b.is_ascii_alphabetic()).map(|b| b.to_ascii_lowercase()).collect()
}

struct Node {
    label: u8,
    parent: usize,
    depth: usize,
    children: Vec<usize>, // indices into nodes; labels are unique per parent
    fail: usize,
    outputs: Vec<usize>,
}

pub struct Automaton {
    nodes: Vec<Node>,
}

impl Automaton {
    pub fn build(patterns: &[&str]) -> Result<Self, BuildError> {
        if patterns.is_empty() {
            return Err(BuildError::NoPatterns);
        }
        if patterns.len() > MAX_PATTERNS {
            return Err(BuildError::TooManyPatterns);
        }
        let mut nodes = vec![Node {
            label: 0,
            parent: 0,
            depth: 0,
            children: Vec::new(),
            fail: 0,
            outputs: Vec::new(),
        }];

        for (pi, pat) in patterns.iter().enumerate() {
            let folded = fold(pat);
            if folded.is_empty() {
                return Err(BuildError::EmptyPattern);
            }
            if folded.len() > MAX_PATTERN_LEN {
                return Err(BuildError::PatternTooLong);
            }
            let mut s = 0usize;
            for &b in &folded {
                s = match nodes[s].children.iter().copied().find(|&c| nodes[c].label == b) {
                    Some(c) => c,
                    None => {
                        let id = nodes.len();
                        let depth = nodes[s].depth + 1;
                        nodes.push(Node {
                            label: b,
                            parent: s,
                            depth,
                            children: Vec::new(),
                            fail: 0,
                            outputs: Vec::new(),
                        });
                        nodes[s].children.push(id);
                        id
                    }
                };
            }
            nodes[s].outputs.push(pi);
        }

        // BFS failure links; merge suffix outputs as we go.
        let mut queue: std::collections::VecDeque<usize> = nodes[0].children.clone().into();
        while let Some(u) = queue.pop_front() {
            for c in nodes[u].children.clone() {
                queue.push_back(c);
                let label = nodes[c].label;
                let mut f = nodes[u].fail;
                let fail_of_c = loop {
                    // f is always strictly shallower than u, so t can never be c itself.
                    if let Some(t) =
                        nodes[f].children.iter().copied().find(|&t| nodes[t].label == label)
                    {
                        break t;
                    }
                    if f == 0 {
                        break 0;
                    }
                    f = nodes[f].fail;
                };
                nodes[c].fail = fail_of_c;
                let inherited = nodes[fail_of_c].outputs.clone();
                nodes[c].outputs.extend(inherited);
            }
        }
        Ok(Automaton { nodes })
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
    pub fn label(&self, s: usize) -> u8 {
        self.nodes[s].label
    }
    pub fn parent(&self, s: usize) -> usize {
        self.nodes[s].parent
    }
    pub fn fail(&self, s: usize) -> usize {
        self.nodes[s].fail
    }
    pub fn depth(&self, s: usize) -> usize {
        self.nodes[s].depth
    }
    pub fn outputs(&self, s: usize) -> &[usize] {
        &self.nodes[s].outputs
    }

    fn child(&self, s: usize, b: u8) -> Option<usize> {
        self.nodes[s].children.iter().copied().find(|&c| self.nodes[c].label == b)
    }

    /// Advances from `state` on `byte`, taking as many failure hops as the
    /// trie requires. Non-alphabetic bytes reset to the root (word
    /// boundary) with no hops. Returns the hops taken (each an intermediate
    /// state landed on while backing off) and the resulting state.
    ///
    /// This is the one place the failure-hop stepping logic lives: both
    /// `Cursor::step` (native tests) and the wasm wrapper (which cannot
    /// hold a borrowing `Cursor<'a>` inside a `#[wasm_bindgen]` struct)
    /// drive the automaton through this method instead of duplicating it.
    pub fn advance(&self, state: usize, byte: u8) -> (Vec<usize>, usize) {
        if !byte.is_ascii_alphabetic() {
            return (Vec::new(), 0);
        }
        let b = byte.to_ascii_lowercase();
        let mut state = state;
        let mut hops = Vec::new();
        while self.child(state, b).is_none() && state != 0 {
            state = self.fail(state);
            hops.push(state);
        }
        if let Some(next) = self.child(state, b) {
            state = next;
        }
        (hops, state)
    }
}

// `Cursor` predates `Automaton::advance` (see its doc comment): it now
// exists solely to exercise `advance` and `outputs` natively without the
// wasm wrapper's flattened-array bookkeeping. Nothing outside tests
// constructs one — the wrapper drives the automaton directly — so it is
// `cfg(test)`-only rather than dead weight in the shipped crate.
#[cfg(test)]
pub struct StepEvent {
    pub hops: Vec<usize>,
    pub state: usize,
    pub matches: Vec<(usize, usize)>,
}

#[cfg(test)]
pub struct Cursor<'a> {
    automaton: &'a Automaton,
    state: usize,
    pos: usize,
}

#[cfg(test)]
impl<'a> Cursor<'a> {
    pub fn new(automaton: &'a Automaton) -> Self {
        Cursor { automaton, state: 0, pos: 0 }
    }

    /// Advance one byte. Non-alphabetic bytes reset to root (word boundary).
    pub fn step(&mut self, byte: u8) -> StepEvent {
        self.pos += 1;
        let (hops, state) = self.automaton.advance(self.state, byte);
        self.state = state;
        let matches = self.automaton.outputs(state).iter().map(|&pi| (pi, self.pos)).collect();
        StepEvent { hops, state, matches }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn textbook() -> Automaton {
        Automaton::build(&["he", "she", "his", "hers"]).unwrap()
    }

    /// Walks the trie from the root along `word`, returning the state.
    fn state_for(a: &Automaton, word: &str) -> usize {
        let mut s = 0;
        for b in word.bytes() {
            s = (0..a.node_count())
                .find(|&n| a.parent(n) == s && a.label(n) == b)
                .expect("path exists");
        }
        s
    }

    #[test]
    fn textbook_trie_has_exactly_ten_states() {
        // root + h,he + s,sh,she + hi,his + her,hers — the classic figure.
        assert_eq!(textbook().node_count(), 10);
    }

    #[test]
    fn failure_links_match_the_textbook_exactly() {
        let a = textbook();
        // she → he (longest proper suffix that is a trie prefix)
        assert_eq!(a.fail(state_for(&a, "she")), state_for(&a, "he"));
        // sh → h ; her → root, since neither "er" nor "r" is a trie prefix
        assert_eq!(a.fail(state_for(&a, "sh")), state_for(&a, "h"));
        assert_eq!(a.fail(state_for(&a, "her")), 0);
        // hi → root (no "i" child of root); his → s
        assert_eq!(a.fail(state_for(&a, "hi")), 0);
        assert_eq!(a.fail(state_for(&a, "his")), state_for(&a, "s"));
        // depth-1 states always fail to root
        assert_eq!(a.fail(state_for(&a, "h")), 0);
        assert_eq!(a.fail(state_for(&a, "s")), 0);
    }

    #[test]
    fn ushers_produces_the_three_textbook_matches_in_order() {
        let a = textbook();
        let mut c = Cursor::new(&a);
        let mut found = Vec::new();
        for (i, b) in "ushers".bytes().enumerate() {
            for (pat, end) in c.step(b).matches {
                found.push((pat, end));
                assert_eq!(end, i + 1, "end must be the position after this byte");
            }
        }
        // pattern indices: 0=he, 1=she, 2=his, 3=hers
        assert_eq!(found, vec![(1, 4), (0, 4), (3, 6)]);
    }

    #[test]
    fn suffix_outputs_are_merged_not_rediscovered() {
        // "she" ends at position 4 and simultaneously ends "he" via the
        // suffix link — both must be reported from the SAME landing state.
        let a = textbook();
        assert_eq!(a.outputs(state_for(&a, "she")), &[1, 0]);
    }

    #[test]
    fn failure_hops_are_reported_for_the_visualizer() {
        // After "ushe", stepping 'r': "sher" is not in the trie, so the
        // cursor hops she→he before taking he→her. Exactly one hop,
        // landing on "her".
        let a = textbook();
        let mut c = Cursor::new(&a);
        for b in "ushe".bytes() {
            c.step(b);
        }
        let ev = c.step(b'r');
        assert_eq!(ev.hops, vec![state_for(&a, "he")]);
        assert_eq!(ev.state, state_for(&a, "her"));
    }

    #[test]
    fn build_rejects_bad_input_loudly() {
        assert!(Automaton::build(&[]).is_err(), "no patterns");
        assert!(Automaton::build(&["ok", ""]).is_err(), "empty pattern");
        assert!(Automaton::build(&["abcdefghijk"]).is_err(), "over 10 chars");
        let nine = ["a", "b", "c", "d", "e", "f", "g", "h", "i"];
        assert!(Automaton::build(&nine).is_err(), "over 8 patterns");
        assert!(Automaton::build(&["He"]).is_ok(), "uppercase folds, not errors");
    }

    #[test]
    fn overlapping_patterns_all_match() {
        let a = Automaton::build(&["aa", "aaa"]).unwrap();
        let mut c = Cursor::new(&a);
        let mut found = Vec::new();
        for b in "aaaa".bytes() {
            found.extend(c.step(b).matches);
        }
        // aa ends at 2,3,4; aaa ends at 3,4.
        assert_eq!(found, vec![(0, 2), (1, 3), (0, 3), (1, 4), (0, 4)]);
    }

    #[test]
    fn transitive_suffix_outputs_inherit_through_two_levels() {
        // cba's outputs must inherit through ba, which itself inherits from a.
        // A DFS/stack build order reads ba's outputs before ba has merged
        // from a, yielding [2, 1] — this asserts the BFS invariant.
        let a = Automaton::build(&["a", "ba", "cba"]).unwrap();
        assert_eq!(a.outputs(state_for(&a, "cba")), &[2, 1, 0]);
    }

    #[test]
    fn non_alphabetic_bytes_reset_and_still_count_positions() {
        let a = Automaton::build(&["he"]).unwrap();
        let mut c = Cursor::new(&a);
        let mut found = Vec::new();
        for b in "he he".bytes() {
            let ev = c.step(b);
            if b == b' ' {
                assert_eq!(ev.state, 0, "space must reset to root");
                assert!(ev.matches.is_empty() && ev.hops.is_empty());
            }
            found.extend(ev.matches);
        }
        // End positions count the space: 2 and 5 against the original text.
        assert_eq!(found, vec![(0, 2), (0, 5)]);
    }

    #[test]
    fn patterns_strip_non_alphabetic_rather_than_rejecting() {
        let a = Automaton::build(&["h3-e"]).unwrap(); // folds to "he"
        let mut c = Cursor::new(&a);
        let mut found = Vec::new();
        for b in "he".bytes() {
            found.extend(c.step(b).matches);
        }
        assert_eq!(found, vec![(0, 2)]);
        assert!(matches!(Automaton::build(&["123"]), Err(BuildError::EmptyPattern)));
    }

    #[test]
    fn advance_matches_the_cursor_it_replaced() {
        // `Cursor::step` now delegates its failure-hop logic to
        // `Automaton::advance`; walk both in lockstep over the textbook
        // hop-producing case and check they never diverge.
        let a = textbook();
        let mut cursor = Cursor::new(&a);
        let mut state = 0usize;
        for b in "ushers".bytes() {
            let event = cursor.step(b);
            let (hops, next_state) = a.advance(state, b);
            assert_eq!(hops, event.hops);
            assert_eq!(next_state, event.state);
            state = next_state;
        }
    }

    #[test]
    fn build_error_messages_are_readable() {
        // The wasm wrapper relays `BuildError::to_string()` verbatim as the
        // JS exception message, so its wording is pinned here rather than
        // only through the wrapper (which can't run outside a wasm target;
        // see `lib.rs`'s test module).
        assert_eq!(BuildError::NoPatterns.to_string(), "no patterns given");
        assert_eq!(BuildError::TooManyPatterns.to_string(), "too many patterns (max 8)");
        assert_eq!(BuildError::PatternTooLong.to_string(), "pattern too long (max 10 letters)");
        // `Automaton` has no `Debug` impl, so `Result::unwrap_err` (which
        // requires the `Ok` side to be `Debug`) isn't available here; match
        // instead, as the wrapper's `.map_err` effectively does.
        let nine = ["a", "b", "c", "d", "e", "f", "g", "h", "i"];
        let Err(err) = Automaton::build(&nine) else { panic!("9 patterns should be rejected") };
        assert_eq!(err.to_string(), "too many patterns (max 8)");
        let Err(err) = Automaton::build(&["123"]) else {
            panic!("digits-only pattern should be rejected")
        };
        assert_eq!(err.to_string(), "a pattern has no letters left after removing non-letters");
    }
}

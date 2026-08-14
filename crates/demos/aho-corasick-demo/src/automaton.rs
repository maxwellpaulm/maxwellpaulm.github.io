//! Aho–Corasick automaton, built from scratch for the visualizer.
//! Deliberately free of wasm so the algorithm is tested natively.

#[derive(Debug, PartialEq)]
pub enum BuildError {
    NoPatterns,
    TooManyPatterns,
    EmptyPattern,
    PatternTooLong,
}

const MAX_PATTERNS: usize = 8;
const MAX_PATTERN_LEN: usize = 10;

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
            let folded: Vec<u8> = pat
                .bytes()
                .filter(|b| b.is_ascii_alphabetic())
                .map(|b| b.to_ascii_lowercase())
                .collect();
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
                    if let Some(t) = nodes[f].children.iter().copied().find(|&t| nodes[t].label == label) {
                        if t != c {
                            break t;
                        }
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
}

pub struct StepEvent {
    pub hops: Vec<usize>,
    pub state: usize,
    pub matches: Vec<(usize, usize)>,
}

pub struct Cursor<'a> {
    automaton: &'a Automaton,
    state: usize,
    pos: usize,
}

impl<'a> Cursor<'a> {
    pub fn new(automaton: &'a Automaton) -> Self {
        Cursor { automaton, state: 0, pos: 0 }
    }

    /// Advance one byte. Non-alphabetic bytes reset to root (word boundary).
    pub fn step(&mut self, byte: u8) -> StepEvent {
        self.pos += 1;
        if !byte.is_ascii_alphabetic() {
            self.state = 0;
            return StepEvent { hops: Vec::new(), state: 0, matches: Vec::new() };
        }
        let b = byte.to_ascii_lowercase();
        let mut hops = Vec::new();
        while self.automaton.child(self.state, b).is_none() && self.state != 0 {
            self.state = self.automaton.fail(self.state);
            hops.push(self.state);
        }
        if let Some(next) = self.automaton.child(self.state, b) {
            self.state = next;
        }
        let matches = self
            .automaton
            .outputs(self.state)
            .iter()
            .map(|&pi| (pi, self.pos))
            .collect();
        StepEvent { hops, state: self.state, matches }
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
}
